use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql, htmlsql_select_page};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbAnalyticsTraffic {
    pub id: Option<u64>,
    pub cluster_name: Option<String>,
    pub ip: Option<String>,
    pub tls: Option<u64>,
    pub pv: Option<u64>,
    pub http_country: Option<String>,
    pub http_code: Option<String>,
    pub http_source: Option<String>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl TbAnalyticsTraffic {
    htmlsql!(insert(rb: &dyn Executor, table: &TbAnalyticsTraffic) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_analytics_traffic.html");
    htmlsql_select_page!(select_page_by_cluster_and_time(cluster_name: &str, start_time: DateTime, end_time: DateTime) -> TbAnalyticsTraffic => "src/storage/mappers/html/tb_analytics_traffic.html");
    htmlsql!(delete_by_gmt_create(rb: &dyn Executor, gmt_create: DateTime) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_analytics_traffic.html");
}
