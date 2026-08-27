use std::time::Duration;

use aigw_core::date_format_local;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use time::OffsetDateTime;

use crate::{
    service::{HH_MM_FORMAT, se_task::Task},
    storage::{
        PageRequest, tb_analytics_monitor::TbAnalyticsMonitor,
        tb_analytics_monitor_cluster::TbAnalyticsMonitorCluster,
        tb_analytics_monitor_cluster_hour::TbAnalyticsMonitorClusterHour,
    },
};

#[derive(Serialize, Deserialize)]
pub struct AnalyticsMonitorItem {
    pub time: String,
    pub item: MonitorItem,
}

#[derive(Serialize, Deserialize, Default)]
pub struct MonitorItem {
    pub cpu: f64,
    pub cpu_current_process: f64,
    pub cpu_load_one: f64,
    pub cpu_load_five: f64,
    pub cpu_load_fifteen: f64,
    pub mem: f64,
    pub swap: f64,
    pub disk: f64,
    pub io_read: u64,
    pub io_written: u64,
    pub net_send: u64,
    pub net_received: u64,
    pub rt: u64,
    pub error: u64,
}

pub async fn get_analytics_monitor(
    rb: &MySqlPool,
    cluster_name: &str,
    limit: usize,
) -> anyhow::Result<Vec<AnalyticsMonitorItem>> {
    let items = TbAnalyticsMonitorCluster::select_by_cluster(rb, cluster_name, limit as u64).await?;
    let mut items: Vec<AnalyticsMonitorItem> = items
        .iter()
        .map(|item| {
            let gmt_create = item
                .gmt_create
                .as_ref()
                .and_then(|d| date_format_local(d.unix_timestamp(), HH_MM_FORMAT));
            convert_tb_analytics_monitor_cluster(item, gmt_create)
        })
        .collect();
    items.reverse();
    Ok(items)
}

pub async fn get_analytics_monitor_server(
    rb: &MySqlPool,
    cluster_name: &str,
    ip: &str,
    limit: usize,
) -> anyhow::Result<Vec<AnalyticsMonitorItem>> {
    let items =
        TbAnalyticsMonitor::select_by_cluster_and_ip(rb, cluster_name, ip, limit as u64).await?;
    let mut items: Vec<AnalyticsMonitorItem> = items
        .iter()
        .map(|item| {
            let gmt_create = item
                .gmt_create
                .as_ref()
                .and_then(|d| date_format_local(d.unix_timestamp(), HH_MM_FORMAT));
            convert_tb_analytics_monitor(item, gmt_create)
        })
        .collect();
    items.reverse();
    Ok(items)
}

pub(crate) async fn analytics_monitor_minute(
    rb: &MySqlPool,
    cluster_name: &str,
    task: &Task,
) -> anyhow::Result<(Option<MonitorItem>, usize)> {
    // 处理监控
    let mointor_items = TbAnalyticsMonitorCluster::select_by_cluster_gmt_create(
        rb,
        cluster_name,
        OffsetDateTime::from_unix_timestamp(task.end_time.unix_timestamp())
            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
    )
    .await?;

    let new_end_time = task.end_time + Duration::from_secs(60);

    let mut mointor_size = 0;
    if mointor_items.is_none() {
        let mut page_no = 1;

        let mut mointor_item = MonitorItem::default();
        loop {
            let page_request = PageRequest::new(page_no, 100);
            let r = TbAnalyticsMonitor::select_page_by_cluster_and_time(
                rb,
                &page_request,
                cluster_name,
                OffsetDateTime::from_unix_timestamp(task.end_time.unix_timestamp())
                    .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                OffsetDateTime::from_unix_timestamp(new_end_time.unix_timestamp())
                    .unwrap_or(OffsetDateTime::UNIX_EPOCH),
            )
            .await?;

            if r.records.is_empty() {
                break;
            }
            mointor_size += r.records.len();

            for a in r.records {
                count_tb_analytics_monitor(&mut mointor_item, a);
            }
            page_no += 1;
        }
        Ok((Some(mointor_item), mointor_size))
    } else {
        Ok((None, mointor_size))
    }
}

pub(crate) async fn analytics_monitor_hour(
    rb: &MySqlPool,
    cluster_name: &str,
    task: &Task,
) -> anyhow::Result<(Option<MonitorItem>, usize)> {
    // 处理监控
    let monitor_item = TbAnalyticsMonitorClusterHour::select_by_cluster_gmt_create(
        rb,
        cluster_name,
        OffsetDateTime::from_unix_timestamp(task.end_time.unix_timestamp())
            .unwrap_or(OffsetDateTime::UNIX_EPOCH),
    )
    .await?;

    let new_end_time = task.end_time + Duration::from_secs(3600);

    let mut monitor_size = 0;
    if monitor_item.is_none() {
        let mut page_no = 1;

        let mut item = MonitorItem::default();
        loop {
            let page_request = PageRequest::new(page_no, 100);
            let r = TbAnalyticsMonitorCluster::select_page_by_cluster_and_time(
                rb,
                &page_request,
                cluster_name,
                OffsetDateTime::from_unix_timestamp(task.end_time.unix_timestamp())
                    .unwrap_or(OffsetDateTime::UNIX_EPOCH),
                OffsetDateTime::from_unix_timestamp(new_end_time.unix_timestamp())
                    .unwrap_or(OffsetDateTime::UNIX_EPOCH),
            )
            .await?;

            if r.records.is_empty() {
                break;
            }
            monitor_size += r.records.len();

            for a in r.records {
                count_tb_analytics_monitor_cluster(&mut item, a);
            }
            page_no += 1;
        }
        Ok((Some(item), monitor_size))
    } else {
        Ok((None, monitor_size))
    }
}

fn convert_tb_analytics_monitor_cluster(
    a: &TbAnalyticsMonitorCluster,
    gmt_create: Option<String>,
) -> AnalyticsMonitorItem {
    AnalyticsMonitorItem {
        time: gmt_create.unwrap_or("-".to_string()),
        item: MonitorItem {
            cpu: a.cpu.unwrap_or(0.0),
            cpu_current_process: a.cpu_current_process.unwrap_or(0.0),
            cpu_load_one: a.cpu_load_one.unwrap_or(0.0),
            cpu_load_five: a.cpu_load_five.unwrap_or(0.0),
            cpu_load_fifteen: a.cpu_load_fifteen.unwrap_or(0.0),
            mem: a.mem.unwrap_or(0.0),
            swap: a.swap.unwrap_or(0.0),
            disk: a.disk.unwrap_or(0.0),
            io_read: a.io_read.map_or(0, |i| i as u64),
            io_written: a.io_written.map_or(0, |i| i as u64),
            net_send: a.net_send.map_or(0, |i| i as u64),
            net_received: a.net_received.map_or(0, |i| i as u64),
            rt: a.rt.map_or(0, |i| i as u64),
            error: a.error.map_or(0, |i| i as u64),
        },
    }
}

fn convert_tb_analytics_monitor(
    a: &TbAnalyticsMonitor,
    gmt_create: Option<String>,
) -> AnalyticsMonitorItem {
    let mem_used = a.mem_used.unwrap_or(0);
    let mem_total = a.mem_free.unwrap_or(0) + mem_used;
    let mem = if mem_total == 0 {
        0.0
    } else {
        mem_used as f64 / mem_total as f64
    };

    let swap_used = a.swap_used.unwrap_or(0);
    let swap_total = a.swap_free.unwrap_or(0) + swap_used;
    let swap = if swap_total == 0 {
        0.0
    } else {
        swap_used as f64 / swap_total as f64
    };

    let disk_used = a.disk_used.unwrap_or(0);
    let disk_total = a.disk_free.unwrap_or(0) + disk_used;
    let disk = if disk_total == 0 {
        0.0
    } else {
        disk_used as f64 / disk_total as f64
    };

    AnalyticsMonitorItem {
        time: gmt_create.unwrap_or("-".to_string()),
        item: MonitorItem {
            cpu: a.cpu.unwrap_or(0.0),
            cpu_current_process: a.cpu_current_process.unwrap_or(0.0),
            cpu_load_one: a.cpu_load_one.unwrap_or(0.0),
            cpu_load_five: a.cpu_load_five.unwrap_or(0.0),
            cpu_load_fifteen: a.cpu_load_fifteen.unwrap_or(0.0),
            mem,
            swap,
            disk,
            io_read: a.io_read.map_or(0, |i| i as u64),
            io_written: a.io_written.map_or(0, |i| i as u64),
            net_send: a.net_send.map_or(0, |i| i as u64),
            net_received: a.net_received.map_or(0, |i| i as u64),
            rt: a.rt.map_or(0, |i| i as u64),
            error: a.error.map_or(0, |i| i as u64),
        },
    }
}

fn count_tb_analytics_monitor(item: &mut MonitorItem, a: TbAnalyticsMonitor) {
    let mem_used = a.mem_used.unwrap_or(0);
    let mem_total = a.mem_free.unwrap_or(0) + mem_used;
    let mem = if mem_total == 0 {
        0.0
    } else {
        mem_used as f64 / mem_total as f64
    };

    let swap_used = a.swap_used.unwrap_or(0);
    let swap_total = a.swap_free.unwrap_or(0) + swap_used;
    let swap = if swap_total == 0 {
        0.0
    } else {
        swap_used as f64 / swap_total as f64
    };

    let disk_used = a.disk_used.unwrap_or(0);
    let disk_total = a.disk_free.unwrap_or(0) + disk_used;
    let disk = if disk_total == 0 {
        0.0
    } else {
        disk_used as f64 / disk_total as f64
    };

    item.cpu += a.cpu.unwrap_or(0.0);
    item.cpu_current_process += a.cpu_current_process.unwrap_or(0.0);
    item.cpu_load_one += a.cpu_load_one.unwrap_or(0.0);
    item.cpu_load_five += a.cpu_load_five.unwrap_or(0.0);
    item.cpu_load_fifteen += a.cpu_load_fifteen.unwrap_or(0.0);
    item.mem += mem;
    item.swap += swap;
    item.disk += disk;
    item.io_read += a.io_read.map_or(0, |i| i as u64);
    item.io_written += a.io_written.map_or(0, |i| i as u64);
    item.net_send += a.net_send.map_or(0, |i| i as u64);
    item.net_received += a.net_received.map_or(0, |i| i as u64);
    item.rt += a.rt.map_or(0, |i| i as u64);
    item.error += a.error.map_or(0, |i| i as u64);
}

fn count_tb_analytics_monitor_cluster(item: &mut MonitorItem, a: TbAnalyticsMonitorCluster) {
    item.cpu += a.cpu.unwrap_or(0.0);
    item.cpu_current_process += a.cpu_current_process.unwrap_or(0.0);
    item.cpu_load_one += a.cpu_load_one.unwrap_or(0.0);
    item.cpu_load_five += a.cpu_load_five.unwrap_or(0.0);
    item.cpu_load_fifteen += a.cpu_load_fifteen.unwrap_or(0.0);
    item.mem += a.mem.unwrap_or(0.0);
    item.swap += a.swap.unwrap_or(0.0);
    item.disk += a.disk.unwrap_or(0.0);
    item.io_read += a.io_read.map_or(0, |i| i as u64);
    item.io_written += a.io_written.map_or(0, |i| i as u64);
    item.net_send += a.net_send.map_or(0, |i| i as u64);
    item.net_received += a.net_received.map_or(0, |i| i as u64);
    item.rt += a.rt.map_or(0, |i| i as u64);
    item.error += a.error.map_or(0, |i| i as u64);
}
