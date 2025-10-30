use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AigwConfig {
    basic: BasicConfig,
    console: ConsoleConfig,
}

impl AigwConfig {
    pub fn basic(&self) -> &BasicConfig {
        &self.basic
    }

    pub fn console(&self) -> &ConsoleConfig {
        &self.console
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BasicConfig {
    // defualt 80
    http: u32,
    // default 443
    https: u32,

    data_dir: Option<String>,
}

impl BasicConfig {
    pub fn http(&self) -> u32 {
        self.http
    }

    pub fn https(&self) -> u32 {
        self.https
    }

    pub fn data_dir(&self) -> &Option<String> {
        &self.data_dir
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsoleConfig {
    address: String,
    key: String,
    cluster: String,
}

impl ConsoleConfig {
    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn cluster(&self) -> &str {
        &self.cluster
    }
}
