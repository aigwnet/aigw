use rbatis::{impl_delete, impl_insert, impl_select, impl_update, rbdc::DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbLocation {
    pub id: Option<u64>,
    pub site_id: Option<u64>,
    pub location: Option<String>,
    pub proxy: i8,
    pub protocol: Option<String>,
    pub sni: Option<String>,
    pub client_max_body_size: Option<usize>,
    pub connection_timeout: Option<u32>,
    pub read_timeout: Option<u32>,
    pub write_timeout: Option<u32>,
    pub idle_timeout: Option<u32>,
    pub rewrite: Option<String>,
    pub http_version: Option<String>,
    pub set_headers: Option<String>,
    pub add_headers: Option<String>,
    pub remove_headers: Option<String>,
    pub root_dir: Option<String>,
    pub auto_index: i8,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}
impl_insert!(TbLocation {});
impl_select!(TbLocation { select_by_site_id(site_id: u64)  => "`WHERE site_id = #{site_id}`"});
impl_select!(TbLocation { select_by_site_id_and_location(site_id: u64, location: &str)  -> Option => "`WHERE site_id = #{site_id} AND location=#{location}`"});
impl_select!(TbLocation { select_by_id(id: u64)  -> Option => "`WHERE id = #{id}"});
impl_update!(TbLocation { update_by_id(id: u64)  => "`WHERE id = #{id}`"});
impl_delete!(TbLocation { delete_by_id(id: u64) => "`WHERE id = #{id}`"});
