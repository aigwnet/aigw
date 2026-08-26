use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql, htmlsql_select_page};
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

impl TbAnalyticsMonitor {
    htmlsql!(insert(rb: &dyn Executor, table: &TbAnalyticsMonitor) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_analytics_monitor.html");
    htmlsql!(select_by_cluster_and_ip(rb: &dyn Executor, cluster_name: &str, ip: &str, limit: usize) -> Result<Vec<TbAnalyticsMonitor>, rbatis::Error> => "src/storage/mappers/html/tb_analytics_monitor.html");
    htmlsql_select_page!(select_page_by_cluster_and_time(cluster_name: &str, start_time: DateTime, end_time: DateTime) -> TbAnalyticsMonitor => "src/storage/mappers/html/tb_analytics_monitor.html");
    htmlsql!(delete_by_gmt_create(rb: &dyn Executor, gmt_create: DateTime) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_analytics_monitor.html");
}
