use super::{from_i8_to_bool, serialize_bool_to_i8};
use rbatis::{
    impl_delete, impl_insert, impl_select, impl_select_page, impl_update, rbdc::DateTime,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TbSite {
    pub id: Option<u64>,
    pub cluster_name: Option<String>,
    pub name: Option<String>,
    pub alt_names: Option<String>,
    pub root_dir: Option<String>,
    #[serde(
        deserialize_with = "from_i8_to_bool",
        serialize_with = "serialize_bool_to_i8"
    )]
    pub auto_index: bool,
    #[serde(
        deserialize_with = "from_i8_to_bool",
        serialize_with = "serialize_bool_to_i8"
    )]
    pub tls_on: bool,
    #[serde(
        deserialize_with = "from_i8_to_bool",
        serialize_with = "serialize_bool_to_i8"
    )]
    pub tls_enforce: bool,
    #[serde(
        deserialize_with = "from_i8_to_bool",
        serialize_with = "serialize_bool_to_i8"
    )]
    pub acme_on: bool,
    pub tls_cert: Option<String>,
    pub tls_cert_start_date: Option<DateTime>,
    pub tls_cert_end_date: Option<DateTime>,
    pub tls_private_key: Option<String>,
    pub rate_limit: Option<isize>,
    pub rate_limit_unit: Option<u64>,
    pub gmt_create: Option<DateTime>,
    pub gmt_modified: Option<DateTime>,
}

impl_insert!(TbSite {});
impl_select!(TbSite{select_by_id(id: i64) -> Option => "`WHERE id = #{id}`"});
impl_select!(TbSite{select_by_name(name: &str) -> Option => "`WHERE name = #{name}`"});
impl_delete!(TbSite{delete_by_name(name: &str) => "`WHERE name = #{name}`"});
impl_select_page!(TbSite{select_page(cluster_name: &str) => "`WHERE cluster_name = #{cluster_name} ORDER BY ID DESC`"});
impl_select_page!(TbSite{select_page_with_acme() => "`WHERE acme_on=1 ORDER BY ID DESC`"});
impl_update!(TbSite{update_by_name(name: &str) => "`WHERE name = #{name}`"});
impl_select_page!(TbSite{select_acme_cert_about_to_expire() => "`WHERE acme_on=1 AND tls_cert_end_date < DATE_ADD(NOW(), INTERVAL 30 DAY) ORDER BY ID DESC`"});
