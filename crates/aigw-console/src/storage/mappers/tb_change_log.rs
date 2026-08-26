use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql, htmlsql_select_page};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbChangeLog {
    pub id: Option<u64>,
    pub cluster_name: Option<String>,
    pub log_type: Option<u32>,
    pub log_action: Option<u32>,
    pub data_id: Option<u64>,
    pub data: Option<String>,
    pub expire_second: Option<u32>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl TbChangeLog {
    htmlsql!(insert(rb: &dyn Executor, table: &TbChangeLog) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_change_log.html");
    htmlsql!(delete_by_id(rb: &dyn Executor, log_id: u64) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_change_log.html");
    htmlsql!(delete_expired(rb: &dyn Executor) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_change_log.html");
    htmlsql!(delete_by_data_id(rb: &dyn Executor, data_id: u64) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_change_log.html");
    htmlsql!(select_by_data_id_and_type(rb: &dyn Executor, log_type: u32, data_id: u64) -> Result<Option<TbChangeLog>, rbatis::Error> => "src/storage/mappers/html/tb_change_log.html");
    htmlsql_select_page!(select_by_type(cluster_name: &str, log_type: u32, log_id: u64) -> TbChangeLog => "src/storage/mappers/html/tb_change_log.html");
}
