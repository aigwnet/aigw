use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbAnalyticsMonitorClusterHour {
    pub id: Option<u64>,
    pub cluster_name: Option<String>,
    pub cpu: Option<f64>,
    pub cpu_current_process: Option<f64>,
    pub cpu_load_one: Option<f64>,
    pub cpu_load_five: Option<f64>,
    pub cpu_load_fifteen: Option<f64>,
    pub mem: Option<f64>,
    pub swap: Option<f64>,
    pub disk: Option<f64>,
    pub io_read: Option<u64>,
    pub io_written: Option<u64>,
    pub net_send: Option<u64>,
    pub net_received: Option<u64>,
    pub rt: Option<u64>,
    pub error: Option<u64>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl TbAnalyticsMonitorClusterHour {
    htmlsql!(insert(rb: &dyn Executor, table: &TbAnalyticsMonitorClusterHour) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_analytics_monitor_cluster_hour.html");
    htmlsql!(select_by_cluster(rb: &dyn Executor, cluster_name: &str, limit: usize) -> Result<Vec<TbAnalyticsMonitorClusterHour>, rbatis::Error> => "src/storage/mappers/html/tb_analytics_monitor_cluster_hour.html");
    htmlsql!(select_by_cluster_gmt_create(rb: &dyn Executor, cluster_name: &str, gmt_create: DateTime) -> Result<Option<TbAnalyticsMonitorClusterHour>, rbatis::Error> => "src/storage/mappers/html/tb_analytics_monitor_cluster_hour.html");
}
