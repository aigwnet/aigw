use rbatis::{
    impl_delete, impl_insert, impl_select, impl_select_page, impl_update, rbdc::DateTime,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbClusterIpCidr {
    pub id: Option<u64>,
    pub cluster_name: Option<String>,
    pub ip: Option<String>,
    pub prefix_len: Option<u32>,
    pub r#type: Option<u8>,
    pub start_time: Option<DateTime>,
    pub end_time: Option<DateTime>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}
impl_insert!(TbClusterIpCidr {});
impl_select!(TbClusterIpCidr { select_by_id(id: u64)  -> Option => "`WHERE id = #{id}`"});
impl_delete!(TbClusterIpCidr { delete_by_id(id: u64) => "`WHERE id = #{id}`"});
impl_select_page!(TbClusterIpCidr{select_page(cluster_name: &str, t: u8) => "`WHERE cluster_name = #{cluster_name} AND type = #{t} ORDER BY ID DESC`"});
impl_update!(TbClusterIpCidr {update_by_id(id: u64) => "`where id = #{id}`"});
