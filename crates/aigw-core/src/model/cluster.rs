use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Cluster {
    pub id: Option<u64>,
    pub name: String,
    pub key: String,
    pub enable: bool,
    pub default_site_enable: bool,
    pub description: Option<String>,
    pub gmt_modified: Option<String>,
}
