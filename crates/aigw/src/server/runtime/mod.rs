mod cert;
mod conf;
mod context;
mod err;
mod file;
mod geo_lite;
mod http_header;
mod opt;
mod proxy;
mod user_agent;
mod util;

pub(crate) use cert::DynamicTlsAccept;
pub(crate) use conf::AigwConfig;
pub(crate) use geo_lite::GeoLite;
pub(crate) use opt::ServerOpt;
pub(crate) use proxy::AigwProxy;
pub(crate) use util::*;

/// Creates a new internal error
pub fn new_internal_error(status: u16, message: String) -> pingora_core::BError {
    pingora_core::Error::because(
        pingora_core::ErrorType::HTTPStatus(status),
        message,
        pingora_core::Error::new(pingora_core::ErrorType::InternalError),
    )
}
