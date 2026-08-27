use std::collections::HashMap;
use std::sync::Arc;

use aigw_core::{Buffer, ChangeLog, DataFrame, LogAction, LogPoint, LogType, build_data};
use time::OffsetDateTime;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::server::connection::Connection;
use crate::service::find_site_by_page;
use crate::storage::PageRequest;
use crate::storage::tb_change_log::{self, TbChangeLog};

/// Asynchronously builds a change log entry.
///
/// This function creates a new change log record in the database with the specified parameters.
/// It supports different log types (Site, Cluster, Acme, IpLayer4) and actions (Create, Update, Delete).
/// For Cluster type logs, only the most recent entry is retained (single entry), while Site type logs
/// can contain multiple entries. The log entry will automatically expire after the specified duration.
///
/// # Parameters
/// - `rb`: Database executor for performing database operations
/// - `cluster_name`: Name of the cluster where the change occurred
/// - `log_type`: Type of log entry (Site, Cluster, Acme, or IpLayer4)
/// - `log_action`: Action performed (Create, Update, or Delete)
/// - `data_id`: Unique identifier of the data being logged
/// - `expire_second`: Expiration time in seconds after which the log entry will be automatically cleaned up
/// - `data`: Optional JSON string containing the actual data content (can be None for delete operations)
///
/// # Returns
/// - `ChangeLog`: The created change log entry with generated ID, timestamps, and other metadata
///
/// # Errors
/// Returns an error if database operations fail or if required parameters are invalid.
pub async fn do_build_change_log(
    conn: &mut sqlx::MySqlConnection,
    cluster_name: String,
    log_type: LogType,
    log_action: LogAction,
    data_id: u64,
    expire_second: u32,
    data: Option<String>,
) -> anyhow::Result<ChangeLog> {
    // 0. delete expired items
    let _r = tb_change_log::TbChangeLog::delete_expired(&mut *conn).await?;

    // 1. delete old change log
    let old_change_log = tb_change_log::TbChangeLog::select_by_data_id_and_type(
        &mut *conn,
        log_type.code() as i32,
        data_id as i64,
    )
    .await?;
    if let Some(old) = old_change_log {
        tb_change_log::TbChangeLog::delete_by_id(&mut *conn, old.id.unwrap()).await?;
    }
    // 2. add change log
    let now = OffsetDateTime::now_utc();
    let mut change_log = TbChangeLog {
        id: None,
        cluster_name: Some(cluster_name),
        log_type: Some(log_type.code() as i32),
        log_action: Some(log_action.code() as i32),
        data_id: Some(data_id as i64),
        data,
        expire_second: Some(expire_second as i32),
        gmt_create: Some(now),
        gmt_modified: Some(now),
    };
    let r = tb_change_log::TbChangeLog::insert(&mut *conn, &change_log).await?;
    change_log.id = Some(r.last_insert_id() as i64);

    Ok(ChangeLog {
        log_id: change_log.id.unwrap() as u64,
        cluster: change_log.cluster_name.unwrap_or_default(),
        log_type,
        log_action,
        data_id: change_log.data_id.unwrap() as u64,
        data: change_log.data.map_or(vec![], |s| s.into_bytes()),
    })
}

/// Asynchronously sends change logs to the AIGW (AI Gateway).
///
/// This function processes a batch of log points and transmits them to the AIGW
/// through the provided connection. It handles the serialization and transmission of
/// change log data, ensuring reliable delivery to the gateway for further processing.
///
/// # Parameters
/// - `connection`: Thread-safe reference to the active connection to the AIGW,
///   wrapped in Arc<Mutex<>> for concurrent access protection
/// - `rb`: Reference to the RBatis instance for any required database operations
///   during the transmission process (e.g., status updates, acknowledgments)
/// - `log_points`: Vector of LogPoint structures containing the change log entries
///   to be transmitted to the AIGW
///
/// # Returns
/// - `Ok(())` on successful transmission of all log points
/// - `Err(anyhow::Error)` if transmission fails, connection errors occur, or data processing encounters issues
///
/// # Errors
/// This function may return errors in cases such as:
/// - Connection to AIGW fails or is interrupted
/// - Serialization of log points fails
/// - Network timeout or communication errors
/// - Database operations during transmission fail
pub async fn send_change_logs_to_aigw(
    connection: &Arc<Mutex<Connection>>,
    rb: &sqlx::MySqlPool,
    log_points: &Vec<LogPoint>,
) -> anyhow::Result<()> {
    info!("Try to send change logs to aigw.");
    let mut map = HashMap::new();
    for p in log_points {
        map.insert(p.log_type, p.log_id);
    }

    for log_type in LogType::all_types() {
        map.entry(log_type).or_insert(0);
    }
    let mut buffer = Buffer::new(32);

    for (log_type, log_id) in map {
        let mut page_no = 1;
        let mut page_request = PageRequest::default();
        page_request = page_request.set_page_size(20);
        loop {
            let mut connection = connection.lock().await;
            if let Some(cluster) = &connection.cluster
                && let Some(crypto) = &connection.crypto
            {
                if log_type == LogType::Site && log_id == 0 {
                    let r = find_site_by_page(rb, &page_request, cluster).await;
                    match r {
                        Ok(page) => {
                            if page.items.is_empty() {
                                break;
                            }

                            let mut logs = vec![];
                            for item in page.items {
                                let json = serde_json::to_string_pretty(&item)?;
                                logs.push(ChangeLog {
                                    log_id: 0,
                                    cluster: item.cluster,
                                    log_type: LogType::Site,
                                    log_action: LogAction::Create,
                                    data_id: item.id.unwrap(),
                                    data: json.into_bytes(),
                                });
                            }

                            let log_id = if let Some(last) = logs.last() {
                                let change_log =
                                    tb_change_log::TbChangeLog::select_by_data_id_and_type(
                                        rb,
                                        log_type.code() as i32,
                                        last.data_id as i64,
                                    )
                                    .await?;
                                change_log.map_or(0, |c| c.id.unwrap_or_default() as u64)
                            } else {
                                0
                            };

                            let data: DataFrame = DataFrame {
                                logs,
                                log_point: Some(LogPoint {
                                    log_id,
                                    log_type: LogType::Site,
                                }),
                            };
                            build_data(&mut buffer, data, crypto)?;
                            connection.write(&buffer).await?;

                            page_no += 1;
                            page_request = page_request.set_page_no(page_no);
                        }
                        Err(e) => {
                            error!("Query error. {:?}", e);
                        }
                    }
                } else {
                    let r = TbChangeLog::select_by_type(
                        rb,
                        &page_request,
                        cluster,
                        log_type.code() as i32,
                        log_id as i64,
                    )
                    .await;
                    match r {
                        Ok(page) => {
                            if page.records.is_empty() {
                                break;
                            }

                            let mut logs = vec![];
                            for item in page.records {
                                logs.push(ChangeLog {
                                    log_id: item.id.unwrap() as u64,
                                    cluster: item.cluster_name.unwrap_or_default(),
                                    log_type,
                                    log_action: (item.log_action.unwrap() as u32).try_into()?,
                                    data_id: item.data_id.unwrap() as u64,
                                    data: item.data.map_or(vec![], |data| data.into_bytes()),
                                });
                            }
                            let log_point = logs.last().map(|last| LogPoint {
                                log_id: last.log_id,
                                log_type,
                            });
                            let data: DataFrame = DataFrame { logs, log_point };
                            build_data(&mut buffer, data, crypto)?;
                            connection.write(&buffer).await?;

                            page_no += 1;
                            page_request = page_request.set_page_no(page_no);
                        }
                        Err(e) => {
                            error!("Query error. {:?}", e);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
