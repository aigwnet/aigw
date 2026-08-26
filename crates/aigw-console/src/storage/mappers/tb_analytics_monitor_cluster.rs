use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql, htmlsql_select_page};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbAnalyticsMonitorCluster {
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

impl TbAnalyticsMonitorCluster {
    htmlsql!(insert(rb: &dyn Executor, table: &TbAnalyticsMonitorCluster) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_analytics_monitor_cluster.html");
    htmlsql!(select_by_cluster(rb: &dyn Executor, cluster_name: &str, limit: usize) -> Result<Vec<TbAnalyticsMonitorCluster>, rbatis::Error> => "src/storage/mappers/html/tb_analytics_monitor_cluster.html");
    htmlsql!(select_by_cluster_gmt_create(rb: &dyn Executor, cluster_name: &str, gmt_create: DateTime) -> Result<Option<TbAnalyticsMonitorCluster>, rbatis::Error> => "src/storage/mappers/html/tb_analytics_monitor_cluster.html");
    htmlsql_select_page!(select_page_by_cluster_and_time(cluster_name: &str, start_time: DateTime, end_time: DateTime) -> TbAnalyticsMonitorCluster => "src/storage/mappers/html/tb_analytics_monitor_cluster.html");
}
