use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicI32, AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};

use aigw_core::{BanckedProtocol, HttpHeader, convert_headers, find_matched_location};
use async_trait::async_trait;
use bytes::Bytes;
use http::{
    HeaderName, HeaderValue, StatusCode,
    header::{self, UPGRADE, USER_AGENT},
};
use once_cell::sync::Lazy;
use pingora_core::{
    Error, ErrorSource,
    ErrorType::{self, ConnectionClosed, HTTPStatus, ReadError, WriteError},
    Result,
    modules::http::{
        HttpModules,
        compression::ResponseCompressionBuilder,
        grpc_web::{GrpcWeb, GrpcWebBridge},
    },
    prelude::HttpPeer,
    protocols::{ALPN, Digest, TcpKeepalive, TimingDigest},
};
use pingora_http::{RequestHeader, ResponseHeader};
use pingora_proxy::{FailToProxy, ProxyHttp, Session};
use simple_useragent::UserAgentParser;
use substring::Substring;
use tracing::{debug, error, info};

use crate::{
    AigwConfig, SERVER,
    server::{
        acme::Http01Handler,
        runtime::{
            GeoLite, error_page,
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

use super::{context::AigwCtx, now_ms};

static DEFAULT_PROXY_SET_HEADERS: Lazy<Vec<HttpHeader>> = Lazy::new(|| {
    let mut headers = vec![];
    let mut map = HashMap::new();
    map.insert("name".to_owned(), "X-Real-IP".to_owned());
    map.insert("value".to_owned(), "$remote_addr".to_owned());
    headers.push(map);

    let mut map = HashMap::new();
    map.insert("name".to_owned(), "X-Forwarded-For".to_owned());
    map.insert("value".to_owned(), "$proxy_add_x_forwarded_for".to_owned());
    headers.push(map);

    let mut map = HashMap::new();
    map.insert("name".to_owned(), "X-Forwarded-Proto".to_owned());
    map.insert("value".to_owned(), "$scheme".to_owned());
    headers.push(map);

    let mut map = HashMap::new();
    map.insert("name".to_owned(), "X-Forwarded-Host".to_owned());
    map.insert("value".to_owned(), "$host".to_owned());
    headers.push(map);

    let mut map = HashMap::new();
    map.insert("name".to_owned(), "X-Forwarded-Port".to_owned());
    map.insert("value".to_owned(), "$server_port".to_owned());
    headers.push(map);

    convert_headers(&headers).unwrap()
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

pub struct AigwProxy {
    config: Arc<AigwConfig>,
    /// Counter tracking total number of accepted connections since server start
    accepted: AtomicU64,
    /// Counter tracking number of currently active request processing operations
    processing: AtomicI32,
    storage: Arc<Storage>,
    http01_handler: Http01Handler,
    user_agent_parser: Arc<UserAgentParser>,
    geo_lite: Arc<GeoLite>,
}

impl AigwProxy {
    pub fn new(config: Arc<AigwConfig>, storage: Arc<Storage>, geo_lite: Arc<GeoLite>) -> Self {
        Self {
            config,
            accepted: AtomicU64::new(0),
            processing: AtomicI32::new(0),
            storage: storage.clone(),
            http01_handler: Http01Handler::new(storage),
            user_agent_parser: Arc::new(simple_useragent::UserAgentParser::new()),
            geo_lite,
        }
    }
}

#[async_trait]
impl ProxyHttp for AigwProxy {
    type CTX = AigwCtx;

    fn new_ctx(&self) -> Self::CTX {
        AigwCtx::new()
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
        ctx.http_upgrade = header
            .headers
            .get(UPGRADE)
            .and_then(|v| v.to_str().map_or(None, |s| Some(s.to_owned())));
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

        let site = self
            .storage
            .find_site(host)
            .map_or(self.storage.find_default_tls_site(), Some);

        let Some(site) = site else {
            let (mut header, body) = error_page::generate_error(StatusCode::NOT_FOUND);
            header.insert_header(http::header::CONNECTION, "close")?;
            session
                .write_error_response(header, body)
                .await
                .unwrap_or_else(|e| {
                    error!("failed to send error response to downstream: {e}");
                });

            return Ok(());
        };
        ctx.site = Some(site.clone());
        ctx.rate = self.storage.find_rate(host);
        let path = header.uri.path();

        if ctx.tls_version.is_none()
            && site.tls_on
            && site.tls_enforce
            && !path.starts_with(ACME_PATH)
        {
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

        if let Some((location, variables)) = find_matched_location(&site.locations, path) {
            ctx.location = Some((path.to_string(), location.clone()));
            for (key, value) in variables.iter() {
                ctx.add_variable(key, value);
            }
        }

        ctx.add_variable("hostname", get_hostname());
        debug!("variables: {:?}", ctx.variables);

        if let Some((_, location)) = &ctx.location {
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
        if let Some(rate) = &ctx.rate
            && rate.max_request > 0
        {
            let curr_window_requests = rate.rate.observe(b"global", 1);
            if curr_window_requests > rate.max_request {
                let (mut header, body) = error_page::generate_error(StatusCode::TOO_MANY_REQUESTS);
                header.insert_header(http::header::CONNECTION, "close")?;
                session
                    .write_error_response(header, body)
                    .await
                    .unwrap_or_else(|e| {
                        error!("failed to send error response to downstream: {e}");
                    });

                return Ok(true);
            }
        }
        let path = session.req_header().uri.path();
        if path.starts_with(ACME_PATH) {
            let host = get_host(session.req_header());
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

                    let (mut header, body) = error_page::generate_error(StatusCode::NOT_FOUND);
                    header.insert_header(http::header::CONNECTION, "close")?;
                    session
                        .write_error_response(header, body)
                        .await
                        .unwrap_or_else(|e| {
                            error!("failed to send error response to downstream: {e}");
                        });
                }
                return Ok(true);
            } else {
                error!("Host is empty: {}", session.req_header().uri);
            }
        }
        //
        let Some(site) = &ctx.site else {
            return Ok(true);
        };

        let Some((_, location)) = &ctx.location else {
            StaticFilesHandler::handle(
                site.root_dir.as_ref(),
                &["index.html", "default.html"],
                None,
                site.auto_index,
                session,
            )
            .await?;

            return Ok(true);
        };
        if !location.proxy {
            StaticFilesHandler::handle(
                if location.root_dir.is_none() {
                    site.root_dir.as_ref()
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
        if let Some((_, location)) = &ctx.location
            && let Some(client_ip) = &ctx.client_ip
            && let Some(b) = location.lb.select(client_ip.as_bytes(), 5)
        {
            let sni = {
                if location.sni.is_empty() || location.sni.eq("$host") {
                    get_host(session.req_header()).map_or("", |h| h).to_owned()
                } else {
                    location.sni.clone()
                }
            };
            let mut peer = HttpPeer::new(b.addr, location.protocol == BanckedProtocol::Https, sni);

            peer.options.connection_timeout =
                Some(Duration::from_secs(location.connection_timeout.into()));
            peer.options.write_timeout = Some(Duration::from_secs(location.write_timeout.into()));
            peer.options.read_timeout = Some(Duration::from_secs(location.read_timeout.into()));
            peer.options.idle_timeout = Some(Duration::from_secs(location.idle_timeout.into()));
            peer.options.tcp_keepalive = Some(TcpKeepalive {
                idle: Duration::from_secs(location.idle_timeout.into()),
                interval: Duration::from_secs(5),
                count: 5,
                #[cfg(target_os = "linux")]
                user_timeout: Duration::from_millis(30000),
            });
            peer.options.verify_hostname = true;

            if let Some(http_version) = &location.http_version {
                match http_version {
                    aigw_core::HttpVersion::H1 => peer.options.alpn = ALPN::H1,
                    aigw_core::HttpVersion::H2 => peer.options.alpn = ALPN::H2,
                    aigw_core::HttpVersion::H2H1 => peer.options.alpn = ALPN::H2H1,
                }
            } else if session.is_upgrade_req() {
                peer.options.alpn = ALPN::H1;
            } else {
                peer.options.alpn = ALPN::H2H1;
            }

            debug!("peer: {:?}", &peer);

            return Ok(Box::new(peer));
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
        let _ = upstream_response.insert_header(header::SERVER, SERVER);

        if session.cache.enabled() {
            // ignore insert header error
            let cache_status = session.cache.phase().as_str();
            let _ = upstream_response.insert_header("X-Cache-Status", cache_status);
        }
        let code = upstream_response.status.as_u16();
        if (100..200).contains(&code) {
            self.storage.http_code_1xx();
        } else if (200..300).contains(&code) {
            self.storage.http_code_2xx();
        } else if (300..400).contains(&code) {
            self.storage.http_code_3xx();
        } else if (400..500).contains(&code) {
            self.storage.http_code_4xx();
        } else if (500..600).contains(&code) {
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

    async fn fail_to_proxy(
        &self,
        session: &mut Session,
        e: &Error,
        _ctx: &mut Self::CTX,
    ) -> FailToProxy
    where
        Self::CTX: Send + Sync,
    {
        let code = match e.etype() {
            HTTPStatus(code) => *code,
            _ => {
                match e.esource() {
                    ErrorSource::Upstream => 502,
                    ErrorSource::Downstream => {
                        match e.etype() {
                            WriteError | ReadError | ConnectionClosed => {
                                /* conn already dead */
                                0
                            }
                            _ => 400,
                        }
                    }
                    ErrorSource::Internal | ErrorSource::Unset => 500,
                }
            }
        };
        if code > 0 {
            let (header, body) = error_page::generate_error(
                StatusCode::from_u16(code).map_or(StatusCode::INTERNAL_SERVER_ERROR, |s| s),
            );
            session
                .write_error_response(header, body)
                .await
                .unwrap_or_else(|e| {
                    error!("failed to send error response to downstream: {e}");
                });
        }

        FailToProxy {
            error_code: code,
            // default to no reuse, which is safest
            can_reuse_downstream: false,
        }
    }

    async fn logging(&self, session: &mut Session, e: Option<&Error>, ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        if e.is_some() {
            self.storage.error();
        }
        // Record rt
        let rt = now_ms() - ctx.created_at;
        self.storage.rt(rt);

        let code = session
            .response_written()
            .map_or("-", |r| r.status.as_str());

        let content_length = session.response_written().map_or("-", |r| {
            r.headers
                .get(header::CONTENT_LENGTH)
                .map_or("-", |s| s.to_str().map_or("-", |s| s))
        });

        let host = get_host(session.req_header()).map_or("-", |s| s);
        let ua = session
            .req_header()
            .headers
            .get(USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let path = match session.req_header().uri.query() {
            Some(q) => session.req_header().uri.path().to_string() + "?" + q,
            None => session.req_header().uri.path().to_string(),
        };
        info!(target: "access", "{:<17} - {} {:<4} {:<8} \"{:<7} {}\" {} \"{}\"", ctx.client_ip.as_ref().map_or("", |s|s), code ,rt, content_length,
            session.req_header().method, 
            path, host, ua);
    }
}
