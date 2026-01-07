use rbatis::{impl_insert, impl_select, impl_select_page, impl_update, rbdc::DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbAigw {
    pub id: Option<u64>,
    pub cluster_name: Option<String>,
    pub ip: Option<String>,
    pub version: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub os_arch: Option<String>,
    pub cpu_name: Option<String>,
    pub cpu_vendor: Option<String>,
    pub cpu_frequency: Option<u64>,
    pub cpu_nums: Option<u32>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl_insert!(TbAigw {});
impl_update!(TbAigw {update_by_id(id: u64) => "`where id = #{id}`"});
impl_select_page!(TbAigw{select_by_page(cluster_name: &str) => "`WHERE cluster_name = #{cluster_name} ORDER BY ID DESC`"});
impl_select!(TbAigw{select_by_cluster_name_and_ip(cluster_name: &str, ip: &str) -> Option => "`WHERE cluster_name = #{cluster_name} and ip = #{ip}`"});
