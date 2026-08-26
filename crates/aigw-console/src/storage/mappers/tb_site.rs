use super::{from_i8_to_bool, serialize_bool_to_i8};
use rbatis::rbdc::db::ExecResult;
use rbatis::rbdc::DateTime;
use rbatis::{executor::Executor, htmlsql, htmlsql_select_page};
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

impl TbSite {
    htmlsql!(insert(rb: &dyn Executor, table: &TbSite) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_site.html");
    htmlsql!(select_by_id(rb: &dyn Executor, id: i64) -> Result<Option<TbSite>, rbatis::Error> => "src/storage/mappers/html/tb_site.html");
    htmlsql!(select_by_name(rb: &dyn Executor, name: &str) -> Result<Option<TbSite>, rbatis::Error> => "src/storage/mappers/html/tb_site.html");
    htmlsql!(delete_by_name(rb: &dyn Executor, name: &str) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_site.html");
    htmlsql_select_page!(select_page(cluster_name: &str) -> TbSite => "src/storage/mappers/html/tb_site.html");
    htmlsql_select_page!(select_page_with_acme() -> TbSite => "src/storage/mappers/html/tb_site.html");
    htmlsql!(update_by_name(rb: &dyn Executor, table: &TbSite, name: &str) -> Result<ExecResult, rbatis::Error> => "src/storage/mappers/html/tb_site.html");
    htmlsql_select_page!(select_acme_cert_about_to_expire() -> TbSite => "src/storage/mappers/html/tb_site.html");
}
