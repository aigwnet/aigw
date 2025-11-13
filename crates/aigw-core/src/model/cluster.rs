use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Cluster {
    pub id: Option<u64>,
    pub name: String,
    pub security_key: String,
    pub enable: bool,
    pub enable_default_site: bool,
    pub enable_white_list: bool,
    pub enable_block_list: bool,
    pub description: Option<String>,
    pub gmt_modified: Option<String>,
}
