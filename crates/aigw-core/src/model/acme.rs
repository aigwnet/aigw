use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct AcmeToken {
    pub host: String,
    pub token: String,
    pub proof: String,
}
