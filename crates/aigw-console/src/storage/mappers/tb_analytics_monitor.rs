use rbatis::{impl_delete, impl_insert, impl_select_page, rbdc::DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbAnalyticsMonitor {
    pub id: Option<u64>,
    pub cluster_name: Option<String>,
    pub ip: Option<String>,
    pub uptime: Option<u64>,
    pub cpu: Option<f64>,
    pub cpu_current_process: Option<f64>,
    pub cpu_load_one: Option<f64>,
    pub cpu_load_five: Option<f64>,
    pub cpu_load_fifteen: Option<f64>,
    pub mem_used: Option<u64>,
    pub mem_free: Option<u64>,
    pub swap_used: Option<u64>,
    pub swap_free: Option<u64>,
    pub disk_used: Option<u64>,
    pub disk_free: Option<u64>,
    pub io_read: Option<u64>,
    pub io_written: Option<u64>,
    pub net_send: Option<u64>,
    pub net_received: Option<u64>,
    pub rt: Option<u64>,
    pub error: Option<u64>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl_insert!(TbAnalyticsMonitor {});
impl_select_page!(TbAnalyticsMonitor{select_page_by_cluster_and_time(cluster_name: &str, start_time: DateTime, end_time: DateTime) => "`WHERE cluster_name = #{cluster_name} AND gmt_create >=#{start_time} AND gmt_create <#{end_time} ORDER BY ID DESC`"});
impl_delete!(TbAnalyticsMonitor { delete_by_gmt_create(gmt_create: DateTime) => "`WHERE gmt_create < #{gmt_create}`"});
