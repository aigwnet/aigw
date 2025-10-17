use rbatis::{impl_delete, impl_insert, impl_select, rbdc::DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbBackend {
    pub id: Option<u64>,
    pub location_id: Option<u64>,
    pub host: Option<String>,
    pub port: Option<u32>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl_insert!(TbBackend {});
impl_select!(TbBackend { select_by_location_id(location_id: u64)  => "`WHERE location_id = #{location_id}`"});
impl_delete!(TbBackend { delete_by_id(id: u64) => "`WHERE id = #{id}`"});
