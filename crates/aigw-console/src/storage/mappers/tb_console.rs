use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql, htmlsql_select_page};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbConsole {
    pub id: Option<u64>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub last_active_time: Option<DateTime>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl TbConsole {
    htmlsql!(insert(rb: &dyn Executor, table: &TbConsole) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_console.html");
    htmlsql!(update_by_id(rb: &dyn Executor, table: &TbConsole, id: u64) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_console.html");
    htmlsql_select_page!(select_by_page() -> TbConsole => "src/storage/mappers/html/tb_console.html");
    htmlsql!(select_by_host_port(rb: &dyn Executor, host: &str, port: u16) -> Result<Option<TbConsole>, rbatis::Error> => "src/storage/mappers/html/tb_console.html");
}
