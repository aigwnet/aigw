use rbatis::{impl_insert, impl_select, rbdc::DateTime};
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

impl_insert!(TbAnalyticsMonitorClusterHour {});
impl_select!(TbAnalyticsMonitorClusterHour { select_by_cluster(cluster_name: &str, limit: usize) => "`WHERE cluster_name = #{cluster_name} ORDER BY ID DESC LIMIT #{limit}`"});
impl_select!(TbAnalyticsMonitorClusterHour { select_by_cluster_gmt_create(cluster_name: &str, gmt_create: DateTime) -> Option => "`WHERE cluster_name = #{cluster_name} AND gmt_create = #{gmt_create}`"});
