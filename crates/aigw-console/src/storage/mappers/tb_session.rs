use rbatis::{impl_insert, impl_select, impl_update, rbdc::DateTime};
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

impl_insert!(TbSession {});
impl_select!(TbSession { select_by_token(token: &str)  -> Option => "`WHERE token = #{token}`"});
impl_update!(TbSession { update_by_token(token: &str) => "`WHERE token = #{token}`"});
