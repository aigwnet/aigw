use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbTask {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub r#type: Option<u32>,
    pub last_time: Option<DateTime>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl TbTask {
    htmlsql!(insert(rb: &dyn Executor, table: &TbTask) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_task.html");
    htmlsql!(select_by_name_and_type(rb: &dyn Executor, name: &str, t: u32) -> Result<Option<TbTask>, rbatis::Error> => "src/storage/mappers/html/tb_task.html");
    htmlsql!(update_by_name_and_type(rb: &dyn Executor, table: &TbTask, name: &str, t: u32) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_task.html");
}
