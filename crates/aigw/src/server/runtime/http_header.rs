use std::str::FromStr;

use super::{context::AigwCtx, get_hostname};
use bytes::BytesMut;
use http::{HeaderName, HeaderValue};
use once_cell::sync::Lazy;
use pingora_http::RequestHeader;
use pingora_proxy::Session;

pub static HTTP_HEADER_X_FORWARDED_FOR: Lazy<http::HeaderName> =
    Lazy::new(|| HeaderName::from_str("X-Forwarded-For").unwrap());

pub static HTTP_HEADER_X_REAL_IP: Lazy<http::HeaderName> =
    Lazy::new(|| HeaderName::from_str("X-Real-Ip").unwrap());

pub const HOST_NAME_TAG: &[u8] = b"$hostname";
const HOST_TAG: &[u8] = b"$host";
const SCHEME_TAG: &[u8] = b"$scheme";
const REMOTE_ADDR_TAG: &[u8] = b"$remote_addr";
const REMOTE_PORT_TAG: &[u8] = b"$remote_port";
const SERVER_ADDR_TAG: &[u8] = b"$server_addr";
const SERVER_PORT_TAG: &[u8] = b"$server_port";
const PROXY_ADD_FORWARDED_TAG: &[u8] = b"$proxy_add_x_forwarded_for";
const UPSTREAM_ADDR_TAG: &[u8] = b"$upstream_addr";
const HTTP_UPGRADE: &[u8] = b"$http_upgrade";

static SCHEME_HTTPS: HeaderValue = HeaderValue::from_static("https");
static SCHEME_HTTP: HeaderValue = HeaderValue::from_static("http");

/// Get request host in this order of precedence:
/// host name from the request line,
/// or host name from the "Host" request header field
pub fn get_host(header: &RequestHeader) -> Option<&str> {
    if let Some(host) = header.uri.host() {
        return Some(host);
    }
    if let Some(host) = header.headers.get(http::header::HOST)
        && let Ok(value) = host.to_str()
    {
        let value = value.trim();
        // IPv6 literal: "[::1]:8080" -> "::1"
        if let Some(rest) = value.strip_prefix('[') {
            return rest.find(']').map(|end| &rest[..end]);
        }
        // host or host:port
        return value.split(':').next();
    }
    None
}

pub static HTTP_HEADER_NAME_X_REQUEST_ID: Lazy<HeaderName> =
    Lazy::new(|| HeaderName::from_str("X-Request-Id").unwrap());

/// Processes special header values that contain dynamic variables.
/// Supports variables like $host, $scheme, $remote_addr etc.
///
/// # Arguments
/// * `value` - The header value to process
/// * `session` - The HTTP session context
/// * `ctx` - The application state
///
/// # Returns
/// * `Option<HeaderValue>` - The processed header value or None if no special handling needed
#[inline]
pub fn convert_header_value(
    value: &HeaderValue,
    session: &Session,
    ctx: &AigwCtx,
) -> Option<HeaderValue> {
    let buf = value.as_bytes();

    // Early return if not a special header (moved this check earlier)
    if buf.is_empty() || !(buf[0] == b'$' || buf[0] == b':') {
        return None;
    }

    // Helper closure to convert string to HeaderValue
    let to_header_value = |s: &str| HeaderValue::from_str(s).ok();

    match buf {
        HOST_TAG => get_host(session.req_header()).and_then(to_header_value),
        SCHEME_TAG => Some(if ctx.tls_version.is_some() {
            SCHEME_HTTPS.clone()
        } else {
            SCHEME_HTTP.clone()
        }),
        HOST_NAME_TAG => to_header_value(get_hostname()),
        REMOTE_ADDR_TAG => ctx.remote_addr.as_deref().and_then(to_header_value),
        REMOTE_PORT_TAG => ctx
            .remote_port
            .map(|p| p.to_string())
            .and_then(|s| to_header_value(&s)),
        SERVER_ADDR_TAG => ctx.server_addr.as_deref().and_then(to_header_value),
        SERVER_PORT_TAG => ctx
            .server_port
            .map(|p| p.to_string())
            .and_then(|s| to_header_value(&s)),
        UPSTREAM_ADDR_TAG => {
            if !ctx.upstream_address.is_empty() {
                to_header_value(&ctx.upstream_address)
            } else {
                None
            }
        }
        PROXY_ADD_FORWARDED_TAG => ctx.remote_addr.as_deref().and_then(|remote_addr| {
            let value = match session.get_header(HTTP_HEADER_X_FORWARDED_FOR.clone()) {
                Some(existing) => {
                    format!("{}, {}", existing.to_str().unwrap_or_default(), remote_addr)
                }
                None => remote_addr.to_string(),
            };
            to_header_value(&value)
        }),
        HTTP_UPGRADE => {
            if let Some(http_upgrade) = ctx.http_upgrade.as_ref() {
                to_header_value(http_upgrade)
            } else {
                None
            }
        }
        _ => handle_special_headers(buf, session, ctx),
    }
}

const HTTP_HEADER_PREFIX: &[u8] = b"$http_";
const HTTP_HEADER_PREFIX_LEN: usize = HTTP_HEADER_PREFIX.len();

#[inline]
fn handle_special_headers(buf: &[u8], session: &Session, ctx: &AigwCtx) -> Option<HeaderValue> {
    // Handle headers that reference other HTTP headers (e.g., $http_origin)
    if buf.starts_with(HTTP_HEADER_PREFIX) {
        return handle_http_header(buf, session);
    }
    // Handle environment variable references (e.g., $HOME)
    if buf.starts_with(b"$") {
        return handle_env_var(buf);
    }
    // Handle context value references (e.g., :connection_id)
    if buf.starts_with(b":") {
        return handle_context_value(buf, ctx);
    }
    None
}

#[inline]
fn handle_http_header(buf: &[u8], session: &Session) -> Option<HeaderValue> {
    // Skip the "$http_" prefix (6 bytes) and convert remaining bytes to header key
    let key = std::str::from_utf8(&buf[HTTP_HEADER_PREFIX_LEN..]).ok()?;
    // Look up and clone the header value from the session
    session.get_header(key).cloned()
}

#[inline]
fn handle_env_var(buf: &[u8]) -> Option<HeaderValue> {
    // Skip the "$" prefix and convert to environment variable name
    let var_name = std::str::from_utf8(&buf[1..]).ok()?;
    // Look up environment variable and convert to HeaderValue if found
    std::env::var(var_name)
        .ok()
        .and_then(|v| HeaderValue::from_str(&v).ok())
}

#[inline]
fn handle_context_value(buf: &[u8], ctx: &AigwCtx) -> Option<HeaderValue> {
    // Skip the ":" prefix and convert to context key
    let key = std::str::from_utf8(&buf[1..]).ok()?;
    // Pre-allocate buffer for value
    let mut value = BytesMut::with_capacity(20);
    // Append context value to buffer
    value = ctx.append_value(value, key);
    // Convert to HeaderValue if buffer is not empty
    if !value.is_empty() {
        HeaderValue::from_bytes(&value).ok()
    } else {
        None
    }
}

/// Get remote addr from session
pub fn get_remote_addr(session: &Session) -> Option<(String, u16)> {
    session
        .client_addr()
        .and_then(|addr| addr.as_inet())
        .map(|addr| (addr.ip().to_canonical().to_string(), addr.port()))
}

/// Returns true when `ip` matches one of the trusted proxy entries
/// (exact IP or CIDR range, e.g. "10.0.0.1" or "10.0.0.0/8").
fn is_trusted_proxy(ip: &str, trusted_proxies: &[String]) -> bool {
    let Ok(addr) = ip.parse::<std::net::IpAddr>() else {
        return false;
    };
    trusted_proxies.iter().any(|t| {
        let t = t.trim();
        if let Ok(net) = t.parse::<ipnet::IpNet>() {
            net.contains(&addr)
        } else {
            t == ip
        }
    })
}

/// Gets client ip from X-Forwarded-For,
/// If none, get from X-Real-Ip,
/// If none, get remote addr.
///
/// The X-Forwarded-For / X-Real-Ip headers are client-supplied and only
/// trusted when the direct peer matches `trusted_proxies` (exact IP or CIDR).
/// Otherwise the socket peer address is used, so clients cannot forge their
/// IP for hashing, geo stats and rate limiting.
pub fn get_client_ip(session: &Session, trusted_proxies: &[String]) -> String {
    let peer = get_remote_addr(session)
        .map(|(ip, _)| ip)
        .unwrap_or_default();
    if peer.is_empty() || is_trusted_proxy(&peer, trusted_proxies) {
        if let Some(value) = session.get_header(HTTP_HEADER_X_FORWARDED_FOR.clone()) {
            let arr: Vec<&str> = value.to_str().unwrap_or_default().split(',').collect();
            if !arr.is_empty() {
                return arr[0].trim().to_string();
            }
        }
        if let Some(value) = session.get_header(HTTP_HEADER_X_REAL_IP.clone()) {
            return value.to_str().unwrap_or_default().to_string();
        }
    }
    peer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_host_ipv6_and_port() {
        let mut header = RequestHeader::build(http::Method::GET, b"/", None).unwrap();
        header.insert_header("Host", "[::1]:8080").unwrap();
        assert_eq!(get_host(&header), Some("::1"));
        header.insert_header("Host", "Example.COM:8443").unwrap();
        assert_eq!(get_host(&header), Some("Example.COM"));
        header.insert_header("Host", "example.com").unwrap();
        assert_eq!(get_host(&header), Some("example.com"));
    }

    #[test]
    fn test_is_trusted_proxy() {
        let trusted = vec!["10.0.0.0/8".to_string(), "192.168.1.1".to_string()];
        assert!(is_trusted_proxy("10.1.2.3", &trusted));
        assert!(is_trusted_proxy("192.168.1.1", &trusted));
        assert!(!is_trusted_proxy("192.168.1.2", &trusted));
        assert!(!is_trusted_proxy("8.8.8.8", &trusted));
        assert!(!is_trusted_proxy("not-an-ip", &trusted));
        assert!(!is_trusted_proxy("10.1.2.3", &[]));
    }
}
