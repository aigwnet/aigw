use std::{
    sync::{
        Arc,
        atomic::{AtomicI32, AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};

use aigw_core::{BanckedProtocol, HttpHeader, Site, convert_headers};
use async_trait::async_trait;
use bytes::Bytes;
use http::{
    HeaderName, HeaderValue, StatusCode,
    header::{self, USER_AGENT},
};
use log::{debug, error};
use once_cell::sync::Lazy;
use pingora_core::{
    Error, ErrorType, Result,
    modules::http::{
        HttpModules,
        compression::ResponseCompressionBuilder,
        grpc_web::{GrpcWeb, GrpcWebBridge},
    },
    prelude::HttpPeer,
    protocols::{ALPN, Digest, TcpKeepalive, TimingDigest},
};
use pingora_http::{RequestHeader, ResponseHeader};
use pingora_proxy::{ProxyHttp, Session};
use simple_useragent::UserAgentParser;
use substring::Substring;

use crate::{
    LoongConfig,
    server::{
        acme::Http01Handler,
        runtime::{
            GeoLite,
            file::StaticFilesHandler,
            get_hostname,
            http_header::{
                HTTP_HEADER_NAME_X_REQUEST_ID, convert_header_value, get_client_ip, get_host,
                get_remote_addr,
            },
            new_internal_error,
            user_agent::{UserAgentType, classify_user_agent},
        },
        storage::Storage,
    },
};

use super::{context::LoongCtx, now_ms};

static DEFAULT_PROXY_SET_HEADERS: Lazy<Vec<HttpHeader>> = Lazy::new(|| {
    convert_headers(&[
        "X-Real-IP:$remote_addr".to_string(),
        "X-Forwarded-For:$proxy_add_x_forwarded_for".to_string(),
        "X-Forwarded-Proto:$scheme".to_string(),
        "X-Forwarded-Host:$host".to_string(),
        "X-Forwarded-Port:$server_port".to_string(),
    ])
    .unwrap()
});

static ACME_PATH: &str = "/.well-known/acme-challenge/";

/// Helper struct to store connection timing and TLS details
#[derive(Debug, Default)]
struct DigestDetail {
    /// Whether the connection was reused from pool
    connection_reused: bool,
    /// Total connection time in milliseconds
    connection_time: u64,
    /// Timestamp when TCP connection was established
    tcp_established: u64,
    /// Timestamp when TLS handshake completed
    tls_established: u64,
    /// TLS protocol version if using HTTPS
    tls_version: Option<String>,
    /// TLS cipher suite in use if using HTTPS
    tls_cipher: Option<String>,
}

/// Extracts timing and TLS information from connection digest.
/// Used for metrics and logging connection details.
#[inline]
fn get_digest_detail(digest: &Digest) -> DigestDetail {
    let get_established = |value: Option<&Option<TimingDigest>>| -> u64 {
        value
            .map(|item| {
                if let Some(item) = item {
                    item.established_ts
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64
                } else {
                    0
                }
            })
            .unwrap_or_default()
    };

    let tcp_established = get_established(digest.timing_digest.first());
    let mut connection_time = 0;
    if tcp_established > 0 {
        connection_time = now_ms() - tcp_established;
    }
    let connection_reused = connection_time > 100;

    let Some(ssl_digest) = &digest.ssl_digest else {
        return DigestDetail {
            connection_reused,
            tcp_established,
            connection_time,
            ..Default::default()
        };
    };

    DigestDetail {
        connection_reused,
        tcp_established,
        connection_time,
        tls_established: get_established(digest.timing_digest.last()),
        tls_version: Some(ssl_digest.version.to_string()),
        tls_cipher: Some(ssl_digest.cipher.to_string()),
    }
}

pub struct LoongProxy {
    config: Arc<LoongConfig>,
    /// Counter tracking total number of accepted connections since server start
    accepted: AtomicU64,
    /// Counter tracking number of currently active request processing operations
    processing: AtomicI32,
    storage: Arc<Storage>,
    default_site: Arc<Site>,
    http01_handler: Http01Handler,
    user_agent_parser: Arc<UserAgentParser>,
    geo_lite: Arc<GeoLite>,
}

impl LoongProxy {
    pub fn new(
        config: Arc<LoongConfig>,
        storage: Arc<Storage>,
        geo_lite: Arc<GeoLite>,
        default_site: Arc<Site>,
    ) -> Self {
        Self {
            config,
            accepted: AtomicU64::new(0),
            processing: AtomicI32::new(0),
            storage: storage.clone(),
            default_site,
            http01_handler: Http01Handler::new(storage),
            user_agent_parser: Arc::new(simple_useragent::UserAgentParser::new()),
            geo_lite,
        }
    }
}

#[async_trait]
impl ProxyHttp for LoongProxy {
    type CTX = LoongCtx;

    fn new_ctx(&self) -> Self::CTX {
        LoongCtx::new()
    }

    fn init_downstream_modules(&self, modules: &mut HttpModules) {
        debug!("init downstream modules");
        // Add disabled downstream compression module by default
        modules.add_module(ResponseCompressionBuilder::enable(0));
        modules.add_module(Box::new(GrpcWeb));
    }

    /// Handles early request processing before main request handling.
    /// Key responsibilities:
    /// - Sets up connection tracking and metrics
    /// - Records timing information
    /// - Initializes OpenTelemetry tracing
    /// - Matches request to location configuration
    /// - Validates request parameters
    /// - Initializes compression and gRPC modules if needed
    async fn early_request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
        self.storage.pv();
        debug!("early request filter");

        if let Some(stream) = session.stream() {
            ctx.connection_id = stream.id() as usize;
        }
        // get digest of timing and tls
        if let Some(digest) = session.digest() {
            let digest_detail = get_digest_detail(digest);
            ctx.connection_time = digest_detail.connection_time;
            ctx.connection_reused = digest_detail.connection_reused;

            if !ctx.connection_reused
                && digest_detail.tls_established >= digest_detail.tcp_established
            {
                ctx.tls_handshake_time =
                    Some(digest_detail.tls_established - digest_detail.tcp_established);
            }
            ctx.tls_cipher = digest_detail.tls_cipher;
            ctx.tls_version = digest_detail.tls_version;
        }

        let ip = get_client_ip(session);
        let address = ip.as_str().parse();
        if let Ok(address) = address {
            let country = self.geo_lite.country(address);
            if let Ok(country) = country {
                self.storage.country(&country);
                ctx.add_variable("country", &country);
            }
        }
        ctx.client_ip = Some(ip.clone());

        ctx.processing = self.processing.fetch_add(1, Ordering::Relaxed) + 1;
        ctx.accepted = self.accepted.fetch_add(1, Ordering::Relaxed) + 1;
        if let Some((remote_addr, remote_port)) = get_remote_addr(session) {
            ctx.remote_addr = Some(remote_addr);
            ctx.remote_port = Some(remote_port);
        }
        if let Some(addr) = session.server_addr().and_then(|addr| addr.as_inet()) {
            ctx.server_addr = Some(addr.ip().to_string());
            ctx.server_port = Some(addr.port());
        }

        let header = session.req_header();

        // 统计user-agent
        let user_agent = header
            .headers
            .get(USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let user_agent = self.user_agent_parser.parse(user_agent);
        match classify_user_agent(&user_agent) {
            UserAgentType::PC => {
                self.storage.http_source_pc();
            }
            UserAgentType::Pad => {
                self.storage.http_source_pad();
            }
            UserAgentType::Mobile => {
                self.storage.http_source_mobile();
            }
            UserAgentType::Bot => {
                self.storage.http_source_bot();
            }
            UserAgentType::Other => {
                self.storage.http_source_unknown();
            }
        }

        let host = get_host(header).unwrap_or_default();

        let Some(server) = self.storage.find_site(host) else {
            return Ok(());
        };
        let path = header.uri.path();

        if ctx.tls_version.is_none() && server.tls_on && !path.starts_with(ACME_PATH) {
            let mut uri = format!("https://{host}");
            let port = self.config.basic().https();
            if port != 443 {
                uri = format!("{uri}:{port}");
            }
            uri = format!("{uri}{path}");
            if let Some(query) = header.uri.query() {
                uri = format!("{uri}?{query}");
            }

            let mut header = ResponseHeader::build(StatusCode::PERMANENT_REDIRECT, Some(2))?;
            header.insert_header(http::header::LOCATION, uri)?;
            header.insert_header(header::CONTENT_LENGTH, 0.to_string())?;
            session
                .write_response_header(Box::new(header), false)
                .await?;
            return Ok(());
        }

        let mut current_location = None;
        for location in server.locations.iter() {
            current_location = Some(location.clone());
            let (matched, variables) = location.match_host_path(path);
            if matched {
                ctx.location = Some((path.to_string(), location.clone()));
                if let Some(variables) = variables {
                    for (key, value) in variables.iter() {
                        ctx.add_variable(key, value);
                    }
                };
                break;
            }
        }
        ctx.add_variable("hostname", get_hostname());
        debug!("variables: {:?}", ctx.variables);

        if let Some(location) = current_location {
            location
                .validate_content_length(header)
                .map_err(|e| new_internal_error(413, e.to_string()))?;

            //
            // Initialize grpc web module for this request
            let grpc_web = session
                .downstream_modules_ctx
                .get_mut::<GrpcWebBridge>()
                .ok_or_else(|| {
                    new_internal_error(500, "grpc web bridge module should be added".to_string())
                })?;
            grpc_web.init();
        }
        Ok(())
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool>
    where
        Self::CTX: Send + Sync,
    {
        let path = session.req_header().uri.path();
        if path.starts_with(ACME_PATH) {
            let host = {
                let mut host = session.req_header().uri.host();
                if host.is_none() {
                    host = session
                        .req_header()
                        .headers
                        .get("Host")
                        .and_then(|h| h.to_str().ok())
                        .map(|h| h.split(':').next().unwrap_or(h));
                }
                host
            };

            if let Some(host) = host {
                let token = path.substring(ACME_PATH.len(), path.len());
                let r = self.http01_handler.handle(host, token);
                if let Some(r) = r {
                    debug!(
                        "Acme http challenge: {:?}, token: {:?}, {:?}",
                        host, &r.token, &r.proof
                    );

                    let b = Bytes::from(r.proof.bytes().collect::<Vec<u8>>());
                    let mut header = ResponseHeader::build(StatusCode::OK, Some(2))?;
                    header.insert_header(header::CONTENT_TYPE, "application/octet-stream")?;
                    header.insert_header(header::CONTENT_LENGTH, b.len())?;
                    let _ = session.write_response_header(Box::new(header), false).await;
                    let body = Some(b);
                    let _ = session.write_response_body(body, true).await;
                } else {
                    error!("Acme validate error: {},{} not found.", host, token);

                    let b = Bytes::from("Not Found".bytes().collect::<Vec<u8>>());
                    let mut header = ResponseHeader::build(StatusCode::NOT_FOUND, Some(2))?;
                    header.insert_header(header::CONTENT_TYPE, "text/plain")?;
                    header.insert_header(header::CONTENT_LENGTH, b.len())?;
                    let _ = session.write_response_header(Box::new(header), false).await;
                    let body = Some(b);
                    let _ = session.write_response_body(body, true).await;
                }
                return Ok(true);
            } else {
                error!("Host is empty: {}", session.req_header().uri);
            }
        }
        //

        let Some((_, location)) = &ctx.location else {
            StaticFilesHandler::handle(
                self.default_site.root_dir.as_ref(),
                &["index.html", "default.html"],
                None,
                self.default_site.auto_index,
                session,
            )
            .await?;

            return Ok(true);
        };
        if !location.proxy {
            StaticFilesHandler::handle(
                if location.root_dir.is_none() {
                    self.default_site.root_dir.as_ref()
                } else {
                    location.root_dir.as_ref()
                },
                &["index.html", "default.html"],
                None,
                location.auto_index,
                session,
            )
            .await?;

            return Ok(true);
        }

        let header = session.req_header_mut();
        location.rewrite(header, ctx.variables.as_ref());
        Ok(false)
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        if let Some((_, location)) = &ctx.location {
            if let Some(client_ip) = &ctx.client_ip {
                if let Some(b) = location.lb.select(client_ip.as_bytes(), 5) {
                    let sni = {
                        if location.sni.is_empty() || location.sni.eq("$host") {
                            get_host(session.req_header()).map_or("", |h| h).to_owned()
                        } else {
                            location.sni.clone()
                        }
                    };
                    let mut peer =
                        HttpPeer::new(b.addr, location.protocol == BanckedProtocol::Https, sni);

                    peer.options.connection_timeout =
                        Some(Duration::from_secs(location.connection_timeout.into()));
                    peer.options.write_timeout =
                        Some(Duration::from_secs(location.write_timeout.into()));
                    peer.options.read_timeout =
                        Some(Duration::from_secs(location.read_timeout.into()));
                    peer.options.idle_timeout =
                        Some(Duration::from_secs(location.idle_timeout.into()));
                    peer.options.tcp_keepalive = Some(TcpKeepalive {
                        idle: Duration::from_secs(location.idle_timeout.into()),
                        interval: Duration::from_secs(5),
                        count: 5,
                        #[cfg(target_os = "linux")]
                        user_timeout: Duration::from_millis(30000),
                    });
                    peer.options.verify_hostname = true;

                    if session.is_upgrade_req() {
                        peer.options.alpn = ALPN::H1;
                    } else {
                        peer.options.alpn = ALPN::H2H1;
                    }
                    debug!("peer: {:?}", &peer);

                    return Ok(Box::new(peer));
                }
            }
        };
        Err(Error::new(ErrorType::new("Host not found.")))
    }

    /// Called when connection is established to upstream.
    /// Records timing metrics and TLS details.
    async fn connected_to_upstream(
        &self,
        _session: &mut Session,
        reused: bool,
        _peer: &HttpPeer,
        #[cfg(unix)] _fd: std::os::unix::io::RawFd,
        #[cfg(windows)] _sock: std::os::windows::io::RawSocket,
        digest: Option<&Digest>,
        ctx: &mut Self::CTX,
    ) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
        debug!("connected to upstream");
        if let Some(digest) = digest {
            let detail = get_digest_detail(digest);
            if !reused {
                let upstream_connect_time = ctx.upstream_connect_time.unwrap_or_default();
                if upstream_connect_time > 0 && detail.tcp_established > upstream_connect_time {
                    ctx.upstream_tcp_connect_time =
                        Some(detail.tcp_established - upstream_connect_time);
                }
                if detail.tls_established > detail.tcp_established {
                    ctx.upstream_tls_handshake_time =
                        Some(detail.tls_established - detail.tcp_established);
                }
            }
            ctx.upstream_connection_time = Some(detail.connection_time);
        }

        ctx.upstream_reused = reused;
        Ok(())
    }

    /// Filters upstream request before sending.
    /// Adds proxy headers and performs any request modifications.
    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        header: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
        debug!("upstream request filter");

        if let Some((_, location)) = &ctx.location {
            let host = {
                if location.sni.is_empty() || location.sni.eq("$host") {
                    get_host(session.req_header()).map_or("", |h| h).to_owned()
                } else {
                    location.sni.clone()
                }
            };
            let _ = header.insert_header("Host", host);

            // Helper closure to avoid code duplication
            let mut set_header = |k: &HeaderName, v: &HeaderValue, append: bool| {
                let value = convert_header_value(v, session, ctx).unwrap_or_else(|| v.clone());
                // v validate for HeaderValue, so always no error
                if append {
                    let _ = header.append_header(k, value);
                } else {
                    let _ = header.insert_header(k, value);
                };
            };

            // Set default reverse proxy headers if enabled
            DEFAULT_PROXY_SET_HEADERS
                .iter()
                .for_each(|(k, v)| set_header(k, v, false));

            // Set custom proxy headers
            if let Some(arr) = &location.proxy_set_headers {
                arr.iter().for_each(|(k, v)| set_header(k, v, false));
            }

            // Append custom proxy headers
            if let Some(arr) = &location.proxy_add_headers {
                arr.iter().for_each(|(k, v)| set_header(k, v, true));
            }
        }

        Ok(())
    }

    async fn response_filter(
        &self,
        session: &mut Session,
        upstream_response: &mut ResponseHeader,
        _ctx: &mut Self::CTX,
    ) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
        debug!("response filter");
        if session.cache.enabled() {
            // ignore insert header error
            let cache_status = session.cache.phase().as_str();
            let _ = upstream_response.insert_header("X-Cache-Status", cache_status);
        }

        let code = upstream_response.status.as_u16();
        if code >= 100 && code < 200 {
            self.storage.http_code_1xx();
        } else if code >= 200 && code < 300 {
            self.storage.http_code_2xx();
        } else if code >= 300 && code < 400 {
            self.storage.http_code_3xx();
        } else if code >= 400 && code < 500 {
            self.storage.http_code_4xx();
        } else if code >= 500 && code < 5600 {
            self.storage.http_code_5xx();
        }

        Ok(())
    }

    fn upstream_response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        debug!("upstream response filter");

        if ctx.status.is_none() {
            ctx.status = Some(upstream_response.status);
        }
        if let Some(id) = &ctx.request_id {
            let _ = upstream_response.insert_header(HTTP_HEADER_NAME_X_REQUEST_ID.clone(), id);
        }
        Ok(())
    }

    async fn logging(&self, _session: &mut Session, e: Option<&Error>, ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        if let Some(_) = e {
            self.storage.error();
        }
        // Record rt
        let rt = now_ms() - ctx.created_at;
        self.storage.rt(rt);
    }
}
