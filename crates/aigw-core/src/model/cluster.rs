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
    /// Trusted proxy IPs or CIDR ranges. X-Forwarded-For / X-Real-IP headers
    /// are only honored when the direct peer matches one of these entries
    /// (nginx `set_real_ip_from` semantics). Empty means never trust them.
    #[serde(default)]
    pub real_ip_from: Vec<String>,
    pub description: Option<String>,
    pub gmt_modified: Option<String>,
}
