use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbBackend {
    pub id: Option<u64>,
    pub location_id: Option<u64>,
    pub host: Option<String>,
    pub port: Option<u32>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl TbBackend {
    htmlsql!(insert(rb: &dyn Executor, table: &TbBackend) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_backend.html");
    htmlsql!(select_by_location_id(rb: &dyn Executor, location_id: u64) -> Result<Vec<TbBackend>, rbatis::Error> => "src/storage/mappers/html/tb_backend.html");
    htmlsql!(delete_by_id(rb: &dyn Executor, id: u64) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_backend.html");
}
