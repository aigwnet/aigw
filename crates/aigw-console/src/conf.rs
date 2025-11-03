use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub(crate) struct AigwConsoleConfig {
    pub(crate) database: DatabaseConfig,
    pub(crate) server: ServerConfig,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct DatabaseConfig {
    pub(crate) user: String,
    pub(crate) password: String,
    pub(crate) url: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct ServerConfig {
    pub(crate) tcp: TcpConfig,
    pub(crate) http: HttpConfig,
    pub(crate) boradcast: BoradcastConfig,
    pub(crate) ui: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct TcpConfig {
    pub(crate) port: u16,
    pub(crate) max_connections: usize,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct HttpConfig {
    pub(crate) port: u16,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct BoradcastConfig {
    pub(crate) port: u16,
}