use super::{from_i8_to_bool, serialize_bool_to_i8};
use rbatis::{
    impl_delete, impl_insert, impl_select, impl_select_page, impl_update, rbdc::DateTime,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbCluster {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub security_key: Option<String>,
    #[serde(
        deserialize_with = "from_i8_to_bool",
        serialize_with = "serialize_bool_to_i8"
    )]
    pub enable: bool,
    #[serde(
        deserialize_with = "from_i8_to_bool",
        serialize_with = "serialize_bool_to_i8"
    )]
    pub enable_default_site: bool,
    #[serde(
        deserialize_with = "from_i8_to_bool",
        serialize_with = "serialize_bool_to_i8"
    )]
    pub enable_white_list: bool,
    #[serde(
        deserialize_with = "from_i8_to_bool",
        serialize_with = "serialize_bool_to_i8"
    )]
    pub enable_block_list: bool,
    pub description: Option<String>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}
impl_insert!(TbCluster {});
impl_select!(TbCluster { select_all() => "`ORDER BY ID ASC`"});
impl_select!(TbCluster { select_by_name(name: &str)  -> Option => "`WHERE name = #{name}`"});
impl_select!(TbCluster { select_by_id(id: u64)  -> Option => "`WHERE id = #{id}`"});
impl_delete!(TbCluster { delete_by_id(id: u64) => "`WHERE id = #{id}`"});
impl_delete!(TbCluster { delete_by_name(name: &str) => "`WHERE name = #{name}`"});
impl_select_page!(TbCluster{select_page() => "`ORDER BY ID DESC`"});
impl_update!(TbCluster {update_by_id(id: u64) => "`where id = #{id}`"});
impl_update!(TbCluster {update_by_name(name: &str) => "`where name = #{name}`"});