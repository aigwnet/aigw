use super::{from_i8_to_bool, serialize_bool_to_i8};
use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql, htmlsql_select_page};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbCluster {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub security_key: Option<String>,
    #[serde(
        deserialize_with = "from_i8_to_bool",
        serialize_with = "serialize_bool_to_i8"
    )]
    pub enable: bool,
    #[serde(
        deserialize_with = "from_i8_to_bool",
        serialize_with = "serialize_bool_to_i8"
    )]
    pub enable_default_site: bool,
    #[serde(
        deserialize_with = "from_i8_to_bool",
        serialize_with = "serialize_bool_to_i8"
    )]
    pub enable_white_list: bool,
    #[serde(
        deserialize_with = "from_i8_to_bool",
        serialize_with = "serialize_bool_to_i8"
    )]
    pub enable_block_list: bool,
    pub description: Option<String>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl TbCluster {
    htmlsql!(insert(rb: &dyn Executor, table: &TbCluster) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_cluster.html");
    htmlsql!(select_all(rb: &dyn Executor) -> Result<Vec<TbCluster>, rbatis::Error> => "src/storage/mappers/html/tb_cluster.html");
    htmlsql!(select_by_name(rb: &dyn Executor, name: &str) -> Result<Option<TbCluster>, rbatis::Error> => "src/storage/mappers/html/tb_cluster.html");
    htmlsql!(select_by_id(rb: &dyn Executor, id: u64) -> Result<Option<TbCluster>, rbatis::Error> => "src/storage/mappers/html/tb_cluster.html");
    htmlsql!(delete_by_id(rb: &dyn Executor, id: u64) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_cluster.html");
    htmlsql!(delete_by_name(rb: &dyn Executor, name: &str) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_cluster.html");
    htmlsql_select_page!(select_page() -> TbCluster => "src/storage/mappers/html/tb_cluster.html");
    htmlsql!(update_by_id(rb: &dyn Executor, table: &TbCluster, id: u64) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_cluster.html");
    htmlsql!(update_by_name(rb: &dyn Executor, table: &TbCluster, name: &str) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_cluster.html");
}
