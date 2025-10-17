use rbatis::{impl_delete, impl_insert, impl_select, impl_select_page, rbdc::DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbChangeLog {
    pub id: Option<u64>,
    pub cluster_name: Option<String>,
    pub log_type: Option<u32>,
    pub log_action: Option<u32>,
    pub data_id: Option<u64>,
    pub data: Option<String>,
    pub expire_second: Option<u32>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}
impl_insert!(TbChangeLog {});
impl_delete!(TbChangeLog{delete_by_id(log_id: u64) => "`WHERE id = #{log_id}`"});
impl_delete!(TbChangeLog{delete_by_expired() => "`WHERE expire_second != 0 and TIMESTAMPDIFF(SECOND, gmt_modified, NOW()) > expire_second`"});
impl_delete!(TbChangeLog{delete_by_data_id(data_id: u64) => "`WHERE data_id = #{data_id}`"});
impl_select!(TbChangeLog{select_by_data_id_and_type(log_type: u32, data_id: u64) -> Option => "`WHERE log_type = #{log_type} AND data_id = #{data_id} and (expire_second = 0 or TIMESTAMPDIFF(SECOND, gmt_modified, NOW()) <= expire_second)`"});
impl_select_page!(TbChangeLog{select_by_type(cluster_name: &str, log_type: u32, log_id: u64) => "`WHERE cluster_name=#{cluster_name} AND log_type = #{log_type} AND id > #{log_id}`"});
