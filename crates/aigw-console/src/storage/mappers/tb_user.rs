use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbUser {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub real_name: Option<String>,
    pub ext_info: Option<String>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl TbUser {
    htmlsql!(insert(rb: &dyn Executor, table: &TbUser) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_user.html");
    htmlsql!(select_by_name(rb: &dyn Executor, name: &str) -> Result<Option<TbUser>, rbatis::Error> => "src/storage/mappers/html/tb_user.html");
    htmlsql!(select_by_email(rb: &dyn Executor, email: &str) -> Result<Option<TbUser>, rbatis::Error> => "src/storage/mappers/html/tb_user.html");
    htmlsql!(update_by_name(rb: &dyn Executor, table: &TbUser, name: &str) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_user.html");
    htmlsql!(update_by_email(rb: &dyn Executor, table: &TbUser, email: &str) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_user.html");
    htmlsql!(select_default_user(rb: &dyn Executor) -> Result<Option<TbUser>, rbatis::Error> => "src/storage/mappers/html/tb_user.html");
}
