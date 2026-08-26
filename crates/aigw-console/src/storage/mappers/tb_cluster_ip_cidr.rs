use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql, htmlsql_select_page};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbClusterIpCidr {
    pub id: Option<u64>,
    pub cluster_name: Option<String>,
    pub ip: Option<String>,
    pub prefix_len: Option<u32>,
    pub r#type: Option<u8>,
    pub start_time: Option<DateTime>,
    pub end_time: Option<DateTime>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl TbClusterIpCidr {
    htmlsql!(insert(rb: &dyn Executor, table: &TbClusterIpCidr) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_cluster_ip_cidr.html");
    htmlsql!(select_by_id(rb: &dyn Executor, id: u64) -> Result<Option<TbClusterIpCidr>, rbatis::Error> => "src/storage/mappers/html/tb_cluster_ip_cidr.html");
    htmlsql!(delete_by_id(rb: &dyn Executor, id: u64) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_cluster_ip_cidr.html");
    htmlsql_select_page!(select_page(cluster_name: &str, t: u8) -> TbClusterIpCidr => "src/storage/mappers/html/tb_cluster_ip_cidr.html");
    htmlsql!(update_by_id(rb: &dyn Executor, table: &TbClusterIpCidr, id: u64) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_cluster_ip_cidr.html");
}
