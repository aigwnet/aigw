mod http;
mod model;
mod protocol;
mod util;

pub use http::HttpHeader;
pub use http::convert_headers;
pub use http::convert_headers_to_string;
pub use model::acme::AcmeToken;
pub use model::cluster::Cluster;
pub use model::location::BanckedProtocol;
pub use model::location::PathSelector;
pub use model::location::ProxyLocation;
pub use model::location::find_matched_location;
pub use model::location::new_path_selector;
pub use model::location::new_rewrite;
pub use model::site::DynamicCert;
pub use model::site::Site;
pub use model::site::TlsPrivateKey;
pub use model::statistics::Statistics;
pub use protocol::Algorithm;
pub use protocol::close::Close;
pub use protocol::data::ChangeLog;
pub use protocol::data::DataAck;
pub use protocol::data::DataFrame;
pub use protocol::data::LogAction;
pub use protocol::data::LogPoint;
pub use protocol::data::LogType;
pub use protocol::frame::Frame;
pub use protocol::handshake::HandshakeInfo;
pub use protocol::handshake::HandshakeRequest;
pub use protocol::handshake::HandshakeResponse;
pub use protocol::heartbeat::Ping;
pub use protocol::heartbeat::Pong;
pub use util::buf::Buffer;
pub use util::crypto::CryptoCore;
pub use util::protocol::*;
pub use util::shutdown::Shutdown;
pub use util::signature::Signature;
pub use util::statistics::statistics;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref LOCAL_IP: String = local_ip_address::local_ip().unwrap().to_string();
}
