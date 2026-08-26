use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbLocation {
    pub id: Option<u64>,
    pub site_id: Option<u64>,
    pub location: Option<String>,
    pub proxy: i8,
    pub protocol: Option<String>,
    pub sni: Option<String>,
    pub client_max_body_size: Option<usize>,
    pub connection_timeout: Option<u32>,
    pub read_timeout: Option<u32>,
    pub write_timeout: Option<u32>,
    pub idle_timeout: Option<u32>,
    pub rewrite: Option<String>,
    pub http_version: Option<String>,
    pub proxy_set_headers: Option<String>,
    pub proxy_add_headers: Option<String>,
    pub proxy_remove_headers: Option<String>,
    pub response_set_headers: Option<String>,
    pub response_add_headers: Option<String>,
    pub response_remove_headers: Option<String>,
    pub root_dir: Option<String>,
    pub auto_index: i8,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl TbLocation {
    htmlsql!(insert(rb: &dyn Executor, table: &TbLocation) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_location.html");
    htmlsql!(select_by_site_id(rb: &dyn Executor, site_id: u64) -> Result<Vec<TbLocation>, rbatis::Error> => "src/storage/mappers/html/tb_location.html");
    htmlsql!(select_by_site_id_and_location(rb: &dyn Executor, site_id: u64, location: &str) -> Result<Option<TbLocation>, rbatis::Error> => "src/storage/mappers/html/tb_location.html");
    htmlsql!(select_by_id(rb: &dyn Executor, id: u64) -> Result<Option<TbLocation>, rbatis::Error> => "src/storage/mappers/html/tb_location.html");
    htmlsql!(update_by_id(rb: &dyn Executor, table: &TbLocation, id: u64) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_location.html");
    htmlsql!(delete_by_id(rb: &dyn Executor, id: u64) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_location.html");
}
