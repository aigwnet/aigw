use rbatis::{impl_insert, impl_select, impl_update, rbdc::DateTime};
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

impl_insert!(TbTask {});
impl_select!(TbTask { select_by_name_and_type(name: &str, t: u32)  -> Option => "`WHERE name = #{name} AND type = #{t}`"});
impl_update!(TbTask { update_by_name_and_type(name: &str, t: u32) => "`WHERE name = #{name} AND type = #{t}`"});
