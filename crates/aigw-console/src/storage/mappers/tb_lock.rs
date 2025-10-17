use rbatis::{impl_delete, rbdc::DateTime};
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

impl_delete!(TbLock { delete_by_key(key: &str) => "`WHERE lock_key = #{key}`"});
