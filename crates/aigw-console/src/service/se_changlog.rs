use std::collections::HashMap;
use std::sync::Arc;

use aigw_core::{Buffer, ChangeLog, DataFrame, LogAction, LogPoint, LogType, build_data};
use rbatis::rbdc::DateTime;
use rbatis::{PageRequest, RBatis};
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::server::connection::Connection;
use crate::service::find_site_by_page;
use crate::storage::tb_change_log::{self, TbChangeLog};

pub async fn do_build_change_log(
    rb: &dyn rbatis::executor::Executor,
    cluster_name: String,
    log_type: LogType,
    log_action: LogAction,
    data_id: u64,
    expire_second: u32,
    data: Option<String>,
) -> anyhow::Result<ChangeLog> {
    // 0. delete expired items
    let _r = tb_change_log::TbChangeLog::delete_expired(rb).await?;

    // 1. delete old change log
    let old_change_log =
        tb_change_log::TbChangeLog::select_by_data_id_and_type(rb, log_type.code(), data_id)
            .await?;
    if let Some(old) = old_change_log {
        tb_change_log::TbChangeLog::delete_by_id(rb, old.id.unwrap()).await?;
    }
    // 2. add change log
    let now = DateTime::utc();
    let mut change_log = TbChangeLog {
        id: None,
        cluster_name: Some(cluster_name),
        log_type: Some(log_type.code()),
        log_action: Some(log_action.code()),
        data_id: Some(data_id),
        data,
        expire_second: Some(expire_second),
        gmt_create: Some(now.clone()),
        gmt_modified: Some(now),
    };
    let r = tb_change_log::TbChangeLog::insert(rb, &change_log).await?;
    change_log.id = r.last_insert_id.as_u64();

    Ok(ChangeLog {
        log_id: change_log.id.unwrap(),
        cluster: change_log.cluster_name.unwrap_or_default(),
        log_type,
        log_action,
        data_id: change_log.data_id.unwrap(),
        data: change_log.data.map_or(vec![], |s| s.into_bytes()),
    })
}

pub async fn send_all_sites_to_aigw(
    connection: &Arc<Mutex<Connection>>,
    rb: &RBatis,
) -> anyhow::Result<()> {
    let mut page_no = 1;
    let mut page_request = PageRequest::default();
    page_request = page_request.set_page_size(20);
    let mut buffer = Buffer::new(32);

    loop {
        let mut connection = connection.lock().await;
        if let Some(cluster) = &connection.cluster
            && let Some(crypto) = &connection.crypto
        {
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
                            log_id: item.id.unwrap(),
                            cluster: item.cluster,
                            log_type: LogType::Site,
                            log_action: LogAction::Add,
                            data_id: item.id.unwrap(),
                            data: json.into_bytes(),
                        });
                    }
                    let log_point = logs.last().map(|last| LogPoint {
                        log_id: last.log_id,
                        log_type: LogType::Site,
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
        } else {
            break;
        }
    }

    Ok(())
}

pub async fn send_change_logs_to_aigw(
    connection: &Arc<Mutex<Connection>>,
    rb: &RBatis,
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
                let r = TbChangeLog::select_by_type(
                    rb,
                    &page_request,
                    cluster,
                    log_type.code(),
                    log_id,
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
                                log_id: item.id.unwrap(),
                                cluster: item.cluster_name.unwrap_or_default(),
                                log_type,
                                log_action: item.log_action.unwrap().try_into()?,
                                data_id: item.data_id.unwrap(),
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
            } else {
                break;
            }
        }
    }

    Ok(())
}
