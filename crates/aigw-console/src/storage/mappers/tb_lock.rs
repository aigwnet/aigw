use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbLock {
    pub id: Option<u64>,
    pub lock_key: Option<String>,
    pub host: Option<String>,
    pub expires_at: Option<DateTime>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl TbLock {
    htmlsql!(delete_by_key(rb: &dyn Executor, key: &str) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_lock.html");
}
