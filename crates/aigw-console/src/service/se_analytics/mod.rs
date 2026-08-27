mod monitor;
mod traffic;

pub use monitor::AnalyticsMonitorItem;
pub use monitor::get_analytics_monitor;
pub use monitor::get_analytics_monitor_server;
pub use traffic::AnalyticsTrafficItem;
pub use traffic::ExtInfo;
pub use traffic::get_analytics_traffic;
pub use traffic::get_analytics_traffic_1day;
pub use traffic::get_analytics_traffic_1month;
pub use traffic::get_analytics_traffic_ext_info_1month;

use crate::{
    service::{
        se_analytics::{
            monitor::{MonitorItem, analytics_monitor_hour, analytics_monitor_minute},
            traffic::{TrafficItem, analytics_traffic_hour, analytics_traffic_minute},
        },
        se_lock,
        se_task::{self, Task},
    },
    storage::{
        db::DatabaseClient, tb_analytics_monitor::TbAnalyticsMonitor,
        tb_analytics_monitor_cluster::TbAnalyticsMonitorCluster,
        tb_analytics_monitor_cluster_hour::TbAnalyticsMonitorClusterHour,
        tb_analytics_traffic::TbAnalyticsTraffic,
        tb_analytics_traffic_cluster::TbAnalyticsTrafficCluster,
        tb_analytics_traffic_cluster_hour::TbAnalyticsTrafficClusterHour, tb_cluster::TbCluster,
    },
};
use aigw_core::{LOCAL_IP, Ping};
use sqlx::{MySql, MySqlPool, Transaction};
use std::{sync::Arc, time::Duration};
use time::OffsetDateTime;
use tokio::time::interval;
use tracing::{debug, error};

pub async fn save_ping(
    rb: &MySqlPool,
    cluster_name: String,
    ip: String,
    ping: Ping,
) -> anyhow::Result<()> {
    let now = OffsetDateTime::now_utc();

    TbAnalyticsMonitor::insert(
        rb,
        &TbAnalyticsMonitor {
            id: None,
            cluster_name: Some(cluster_name.clone()),
            ip: Some(ip.clone()),
            uptime: Some(ping.statistics.uptime as i64),
            cpu: Some(ping.statistics.cpu),
            cpu_current_process: Some(ping.statistics.cpu_current_process),
            cpu_load_one: Some(ping.statistics.cpu_load_one),
            cpu_load_five: Some(ping.statistics.cpu_load_five),
            cpu_load_fifteen: Some(ping.statistics.cpu_load_fifteen),
            mem_used: Some(ping.statistics.mem_used as i64),
            mem_free: Some(ping.statistics.mem_free as i64),
            swap_used: Some(ping.statistics.swap_used as i64),
            swap_free: Some(ping.statistics.swap_free as i64),
            disk_used: Some(ping.statistics.disk_used as i64),
            disk_free: Some(ping.statistics.disk_free as i64),
            io_read: Some(ping.statistics.io_read as i64),
            io_written: Some(ping.statistics.io_written as i64),
            net_send: Some(ping.statistics.net_send as i64),
            net_received: Some(ping.statistics.net_received as i64),
            rt: Some(ping.statistics.rt as i64),
            error: Some(ping.statistics.error as i64),
            gmt_create: Some(now),
            gmt_modified: Some(now),
        },
    )
    .await?;

    let ext_info: Result<ExtInfo, serde_json::Error> =
        serde_json::from_str(&ping.statistics.ext_info);
    let (http_country, http_code, http_source) = match ext_info {
        Ok(e) => (
            serde_json::to_string(&e.http_country).ok(),
            serde_json::to_string(&e.http_code).ok(),
            serde_json::to_string(&e.http_source).ok(),
        ),
        Err(e) => {
            debug!("Ext info parse error: {:?}", e);
            (None, None, None)
        }
    };

    TbAnalyticsTraffic::insert(
        rb,
        &TbAnalyticsTraffic {
            id: None,
            cluster_name: Some(cluster_name),
            ip: Some(ip),
            tls: Some(ping.statistics.tls as i64),
            pv: Some(ping.statistics.pv as i64),
            http_country,
            http_code,
            http_source,
            gmt_create: Some(now),
            gmt_modified: Some(now),
        },
    )
    .await?;
    Ok(())
}

pub async fn start_analytics_minute(databse_client: Arc<DatabaseClient>) {
    let mut interval = interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        match do_start_analytics_minute(&databse_client.rb).await {
            Ok(_) => {}
            Err(err) => {
                error!("Analytics minute error: {:?}", err);
            }
        }
    }
}

pub async fn start_analytics_hour(databse_client: Arc<DatabaseClient>) {
    let mut interval = interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        match do_start_analytics_hour(&databse_client.rb).await {
            Ok(_) => {}
            Err(err) => {
                error!("Analytics hour error: {:?}", err);
            }
        }
    }
}

async fn do_start_analytics_minute(rb: &MySqlPool) -> anyhow::Result<()> {
    let clusters = TbCluster::select_all(rb).await?;

    for cluster in clusters {
        let cluster_name = cluster
            .name
            .ok_or(anyhow::anyhow!("Cluster name is empty"))?;

        let lock_key = "analytics_minute_".to_string() + &cluster_name;

        let host = &LOCAL_IP;
        // 乐观锁
        let r = se_lock::try_acquire_lock(rb, &lock_key, host, 120).await;
        if !r {
            continue;
        }

        let task_name = "analytics_minute_".to_string() + &cluster_name;
        let mut task = se_task::find_task(rb, &task_name, 1).await?;

        let mut end_time = task.end_time;
        let mut new_end_time = task.end_time + Duration::from_secs(60);

        while new_end_time.unix_timestamp() < OffsetDateTime::now_utc().unix_timestamp() {
            let (monitor_item, monitor_size) =
                analytics_monitor_minute(rb, &cluster_name, &task).await?;
            let traffic_item = analytics_traffic_minute(rb, &cluster_name, &task).await?;

            task.end_time = new_end_time;
            let mut tx = rb.begin().await?;
            match do_save_cluster_minute(
                &mut tx,
                monitor_item,
                monitor_size,
                traffic_item,
                cluster_name.clone(),
                &task,
                end_time,
            )
            .await
            {
                Ok(_) => {
                    let _ = tx.commit().await;
                }
                Err(e) => {
                    let _ = tx.rollback().await;
                    error!("Save analytics minute error. {:?}", e);
                }
            }

            end_time = task.end_time;
            new_end_time = task.end_time + Duration::from_secs(60);
        }

        // Release lock
        se_lock::release_lock(rb, &lock_key).await;
    }

    let one_mounth_ago = OffsetDateTime::now_utc() - time::Duration::seconds(2592000);
    // Clean up records older than one month.
    let _ = TbAnalyticsMonitor::delete_by_gmt_create(rb, one_mounth_ago).await;
    let _ = TbAnalyticsTraffic::delete_by_gmt_create(rb, one_mounth_ago).await;
    Ok(())
}

async fn do_start_analytics_hour(rb: &MySqlPool) -> anyhow::Result<()> {
    let clusters = TbCluster::select_all(rb).await?;

    for cluster in clusters {
        let cluster_name = cluster
            .name
            .ok_or(anyhow::anyhow!("Cluster name is empty"))?;

        let lock_key = "analytics_hour_".to_string() + &cluster_name;

        let host = &LOCAL_IP;
        // Optimistic Locking
        let r = se_lock::try_acquire_lock(rb, &lock_key, host, 120).await;
        if !r {
            continue;
        }

        let task_name = "analytics_hour_".to_string() + &cluster_name;
        let mut task = se_task::find_task(rb, &task_name, 2).await?;

        let mut end_time = task.end_time;
        let mut new_end_time = task.end_time + Duration::from_secs(3600);

        while new_end_time.unix_timestamp() < OffsetDateTime::now_utc().unix_timestamp() {
            let (monitor_item, monitor_size) =
                analytics_monitor_hour(rb, &cluster_name, &task).await?;
            let traffic_item = analytics_traffic_hour(rb, &cluster_name, &task).await?;

            task.end_time = new_end_time;
            let mut tx = rb.begin().await?;
            match do_save_cluster_hour(
                &mut tx,
                monitor_item,
                monitor_size,
                traffic_item,
                cluster_name.clone(),
                &task,
                end_time,
            )
            .await
            {
                Ok(_) => {
                    let _ = tx.commit().await;
                }
                Err(e) => {
                    let _ = tx.rollback().await;
                    error!("Save analytics hour error. {:?}", e);
                }
            }

            end_time = task.end_time;
            new_end_time = task.end_time + Duration::from_secs(3600);
        }

        // 删除锁
        se_lock::release_lock(rb, &lock_key).await;
    }
    Ok(())
}

async fn do_save_cluster_minute(
    tx: &mut Transaction<'_, MySql>,
    monitor_item: Option<MonitorItem>,
    monitor_size: usize,
    traffic_item: Option<TrafficItem>,
    cluster_name: String,
    task: &Task,
    end_time: OffsetDateTime,
) -> anyhow::Result<()> {
    if let Some(item) = monitor_item
        && monitor_size > 0
    {
        TbAnalyticsMonitorCluster::insert(
            &mut **tx,
            &TbAnalyticsMonitorCluster {
                id: None,
                cluster_name: Some(cluster_name.clone()),
                cpu: Some(item.cpu / monitor_size as f64),
                cpu_current_process: Some(item.cpu_current_process / monitor_size as f64),
                cpu_load_one: Some(item.cpu_load_one / monitor_size as f64),
                cpu_load_five: Some(item.cpu_load_five / monitor_size as f64),
                cpu_load_fifteen: Some(item.cpu_load_fifteen / monitor_size as f64),
                mem: Some(item.mem / monitor_size as f64),
                swap: Some(item.swap / monitor_size as f64),
                disk: Some(item.disk / monitor_size as f64),
                io_read: Some((item.io_read / monitor_size as u64) as i64),
                io_written: Some((item.io_written / monitor_size as u64) as i64),
                net_send: Some((item.net_send / monitor_size as u64) as i64),
                net_received: Some((item.net_received / monitor_size as u64) as i64),
                rt: Some((item.rt / monitor_size as u64) as i64),
                error: Some(item.error as i64),
                gmt_create: Some(
                    OffsetDateTime::from_unix_timestamp(end_time.unix_timestamp())
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                ),
                gmt_modified: Some(
                    OffsetDateTime::from_unix_timestamp(end_time.unix_timestamp())
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                ),
            },
        )
        .await?;
    }

    if let Some(item) = traffic_item {
        TbAnalyticsTrafficCluster::insert(
            &mut **tx,
            &TbAnalyticsTrafficCluster {
                id: None,
                cluster_name: Some(cluster_name),
                tls: Some(item.tls as i64),
                pv: Some(item.pv as i64),
                http_country: serde_json::to_string(&item.ext_info.http_country).ok(),
                http_code: serde_json::to_string(&item.ext_info.http_code).ok(),
                http_source: serde_json::to_string(&item.ext_info.http_source).ok(),
                gmt_create: Some(
                    OffsetDateTime::from_unix_timestamp(end_time.unix_timestamp())
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                ),
                gmt_modified: Some(
                    OffsetDateTime::from_unix_timestamp(end_time.unix_timestamp())
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                ),
            },
        )
        .await?;
    }

    se_task::update_task(&mut **tx, task).await?;
    Ok(())
}

async fn do_save_cluster_hour(
    tx: &mut Transaction<'_, MySql>,
    monitor_item: Option<MonitorItem>,
    monitor_size: usize,
    traffic_item: Option<TrafficItem>,
    cluster_name: String,
    task: &Task,
    end_time: OffsetDateTime,
) -> anyhow::Result<()> {
    if monitor_size == 0 {
        return Ok(());
    }
    if let Some(item) = monitor_item
        && monitor_size > 0
    {
        TbAnalyticsMonitorClusterHour::insert(
            &mut **tx,
            &TbAnalyticsMonitorClusterHour {
                id: None,
                cluster_name: Some(cluster_name.clone()),
                cpu: Some(item.cpu / monitor_size as f64),
                cpu_current_process: Some(item.cpu_current_process / monitor_size as f64),
                cpu_load_one: Some(item.cpu_load_one / monitor_size as f64),
                cpu_load_five: Some(item.cpu_load_five / monitor_size as f64),
                cpu_load_fifteen: Some(item.cpu_load_fifteen / monitor_size as f64),
                mem: Some(item.mem / monitor_size as f64),
                swap: Some(item.swap / monitor_size as f64),
                disk: Some(item.disk / monitor_size as f64),
                io_read: Some((item.io_read / monitor_size as u64) as i64),
                io_written: Some((item.io_written / monitor_size as u64) as i64),
                net_send: Some((item.net_send / monitor_size as u64) as i64),
                net_received: Some((item.net_received / monitor_size as u64) as i64),
                rt: Some((item.rt / monitor_size as u64) as i64),
                error: Some(item.error as i64),
                gmt_create: Some(
                    OffsetDateTime::from_unix_timestamp(end_time.unix_timestamp())
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                ),
                gmt_modified: Some(
                    OffsetDateTime::from_unix_timestamp(end_time.unix_timestamp())
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                ),
            },
        )
        .await?;
    }

    if let Some(item) = traffic_item {
        TbAnalyticsTrafficClusterHour::insert(
            &mut **tx,
            &TbAnalyticsTrafficClusterHour {
                id: None,
                cluster_name: Some(cluster_name),
                tls: Some(item.tls as i64),
                pv: Some(item.pv as i64),
                http_country: serde_json::to_string(&item.ext_info.http_country).ok(),
                http_code: serde_json::to_string(&item.ext_info.http_code).ok(),
                http_source: serde_json::to_string(&item.ext_info.http_source).ok(),
                gmt_create: Some(
                    OffsetDateTime::from_unix_timestamp(end_time.unix_timestamp())
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                ),
                gmt_modified: Some(
                    OffsetDateTime::from_unix_timestamp(end_time.unix_timestamp())
                        .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                ),
            },
        )
        .await?;
    }
    se_task::update_task(&mut **tx, task).await?;
    Ok(())
}
