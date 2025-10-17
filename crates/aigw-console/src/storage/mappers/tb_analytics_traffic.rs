use rbatis::{impl_delete, impl_insert, impl_select_page, rbdc::DateTime};
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

impl_insert!(TbAnalyticsTraffic {});
impl_select_page!(TbAnalyticsTraffic{select_page_by_cluster_and_time(cluster_name: &str, start_time: DateTime, end_time: DateTime) => "`WHERE cluster_name = #{cluster_name} AND gmt_create >=#{start_time} AND gmt_create <#{end_time} ORDER BY ID DESC`"});
impl_delete!(TbAnalyticsTraffic { delete_by_gmt_create(gmt_create: DateTime) => "`WHERE gmt_create < #{gmt_create}`"});
