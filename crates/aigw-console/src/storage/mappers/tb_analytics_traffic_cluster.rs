use rbatis::{impl_insert, impl_select, impl_select_page, rbdc::DateTime};
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

impl_insert!(TbAnalyticsTrafficCluster {});
impl_select!(TbAnalyticsTrafficCluster { select_by_cluster(cluster_name: &str, limit: usize) => "`WHERE cluster_name = #{cluster_name} ORDER BY ID DESC LIMIT #{limit}`"});
impl_select!(TbAnalyticsTrafficCluster { select_by_cluster_gmt_create(cluster_name: &str, gmt_create: DateTime) -> Option => "`WHERE cluster_name = #{cluster_name} AND gmt_create = #{gmt_create}`"});
impl_select_page!(TbAnalyticsTrafficCluster{ select_page_by_cluster_and_time(cluster_name: &str, start_time: DateTime, end_time: DateTime) => "`WHERE cluster_name = #{cluster_name} AND gmt_create >=#{start_time} AND gmt_create <#{end_time} ORDER BY ID DESC`"});
