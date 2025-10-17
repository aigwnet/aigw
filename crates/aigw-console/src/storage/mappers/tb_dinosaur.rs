use rbatis::{impl_insert, impl_select, impl_select_page, impl_update, rbdc::DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbDinosaur {
    pub id: Option<u64>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub last_active_time: Option<DateTime>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl_insert!(TbDinosaur {});
impl_update!(TbDinosaur {update_by_id(id: u64) => "`where id = #{id}`"});
impl_select_page!(TbDinosaur{select_by_page() => "`ORDER BY ID DESC`"});
impl_select!(TbDinosaur{select_by_host_port(host: &str, port: u16) -> Option => "`WHERE host = #{host} and port= #{port}`"});
