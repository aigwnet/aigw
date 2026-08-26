use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql, htmlsql_select_page};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbAnalyticsTrafficCluster {
    pub id: Option<u64>,
    pub cluster_name: Option<String>,
    pub tls: Option<u64>,
    pub pv: Option<u64>,
    pub http_country: Option<String>,
    pub http_code: Option<String>,
    pub http_source: Option<String>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl TbAnalyticsTrafficCluster {
    htmlsql!(insert(rb: &dyn Executor, table: &TbAnalyticsTrafficCluster) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_analytics_traffic_cluster.html");
    htmlsql!(select_by_cluster(rb: &dyn Executor, cluster_name: &str, limit: usize) -> Result<Vec<TbAnalyticsTrafficCluster>, rbatis::Error> => "src/storage/mappers/html/tb_analytics_traffic_cluster.html");
    htmlsql!(select_by_cluster_gmt_create(rb: &dyn Executor, cluster_name: &str, gmt_create: DateTime) -> Result<Option<TbAnalyticsTrafficCluster>, rbatis::Error> => "src/storage/mappers/html/tb_analytics_traffic_cluster.html");
    htmlsql_select_page!(select_page_by_cluster_and_time(cluster_name: &str, start_time: DateTime, end_time: DateTime) -> TbAnalyticsTrafficCluster => "src/storage/mappers/html/tb_analytics_traffic_cluster.html");
}
