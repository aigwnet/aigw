use rbatis::{impl_insert, impl_select, impl_update, rbdc::DateTime};
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

impl_insert!(TbUser {});
impl_select!(TbUser { select_by_name(name: &str)  -> Option => "`WHERE name = #{name}`"});
impl_select!(TbUser { select_by_email(email: &str)  -> Option => "`WHERE email = #{email}`"});
impl_update!(TbUser { update_by_name(name: &str)  => "`WHERE name = #{name}`"});
impl_update!(TbUser { update_by_email(email: &str)  => "`WHERE email = #{email}`"});
impl_select!(TbUser { select_default_user()  -> Option => "`WHERE email IS NOT NULL ORDER BY ID ASC LIMIT 1`"});
