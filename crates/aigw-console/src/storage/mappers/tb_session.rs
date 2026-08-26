use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbSession {
    pub id: Option<u64>,
    pub user: Option<String>,
    pub email: Option<String>,
    pub login_ip: Option<String>,
    pub token: Option<String>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl TbSession {
    htmlsql!(insert(rb: &dyn Executor, table: &TbSession) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_session.html");
    htmlsql!(select_by_token(rb: &dyn Executor, token: &str) -> Result<Option<TbSession>, rbatis::Error> => "src/storage/mappers/html/tb_session.html");
    htmlsql!(update_by_token(rb: &dyn Executor, table: &TbSession, token: &str) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_session.html");
}
