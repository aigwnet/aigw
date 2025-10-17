use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Cluster {
    pub id: Option<u64>,
    pub name: String,
    pub description: Option<String>,
    pub gmt_create: Option<String>,
    pub gmt_modified: Option<String>,
}
