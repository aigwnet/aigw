use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql, htmlsql_select_page};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbAigw {
    pub id: Option<u64>,
    pub cluster_name: Option<String>,
    pub ip: Option<String>,
    pub version: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub os_arch: Option<String>,
    pub cpu_name: Option<String>,
    pub cpu_vendor: Option<String>,
    pub cpu_frequency: Option<u64>,
    pub cpu_nums: Option<u32>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl TbAigw {
    htmlsql!(insert(rb: &dyn Executor, table: &TbAigw) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_aigw.html");
    htmlsql!(update_by_id(rb: &dyn Executor, table: &TbAigw, id: u64) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_aigw.html");
    htmlsql_select_page!(select_by_page(cluster_name: &str) -> TbAigw => "src/storage/mappers/html/tb_aigw.html");
    htmlsql!(select_by_cluster_name_and_ip(rb: &dyn Executor, cluster_name: &str, ip: &str) -> Result<Option<TbAigw>, rbatis::Error> => "src/storage/mappers/html/tb_aigw.html");
}
