use std::{sync::Arc, time::Duration};

use ahash::AHashMap;
use aigw_core::{ProxyLocation, Site};
use bytes::BytesMut;
use http::StatusCode;

use crate::server::RateLimit;

use super::now_ms;

/// Statistics about response compression operations
pub struct CompressionStat {
    /// Size of the data before compression in bytes
    pub in_bytes: usize,
    /// Size of the data after compression in bytes
    pub out_bytes: usize,
    /// Time taken to perform the compression operation
    pub duration: Duration,
}

impl CompressionStat {
    pub fn ratio(&self) -> f64 {
        (self.in_bytes as f64) / (self.out_bytes as f64)
    }
}

const SECOND: u64 = 1_000;
const MINUTE: u64 = 60 * SECOND;
const HOUR: u64 = 60 * MINUTE;
const ONE_HOUR_MS: u64 = 60 * 60 * 1000;

/// Format the duration in human readable format
fn format_duration(mut buf: BytesMut, ms: u64) -> BytesMut {
    if ms >= HOUR {
        buf.extend(itoa::Buffer::new().format(ms / HOUR).as_bytes());
        let value = ms % HOUR * 10 / HOUR;
        if value != 0 {
            buf.extend(b".");
            buf.extend(itoa::Buffer::new().format(value).as_bytes());
        }
        buf.extend(b"h");
    } else if ms >= MINUTE {
        buf.extend(itoa::Buffer::new().format(ms / MINUTE).as_bytes());
        let value = ms % MINUTE * 10 / MINUTE;
        if value != 0 {
            buf.extend(b".");
            buf.extend(itoa::Buffer::new().format(value).as_bytes());
        }
        buf.extend(b"m");
    } else if ms >= SECOND {
        buf.extend(itoa::Buffer::new().format(ms / SECOND).as_bytes());
        let value = (ms % SECOND) / 100;
        if value != 0 {
            buf.extend(b".");
            buf.extend(itoa::Buffer::new().format(value).as_bytes());
        }
        buf.extend(b"s");
    } else {
        buf.extend(itoa::Buffer::new().format(ms).as_bytes());
        buf.extend(b"ms");
    }
    buf
}

#[derive(Default)]
pub struct AigwCtx {
    /// Unique identifier for the connection, it should be unique among all existing connections of the same type
    pub connection_id: usize,
    /// Number of requests currently processing
    pub processing: i32,
    /// Total number of requests accepted
    pub accepted: u64,
    /// Timestamp when this context was created (in milliseconds)
    pub created_at: u64,
    /// TLS version used by the client connection (e.g., "TLSv1.3")
    pub tls_version: Option<String>,
    /// TLS cipher suite used by the client connection
    pub tls_cipher: Option<String>,
    /// Time taken for TLS handshake with client (in milliseconds)
    pub tls_handshake_time: Option<u64>,
    /// JA4
    pub tls_fingerprint: Option<String>,
    /// HTTP status code of the response
    pub status: Option<StatusCode>,
    /// Total time the connection has been alive (in milliseconds)
    /// May be large for reused connections
    pub connection_time: u64,
    /// Indicates if this connection is reused
    pub connection_reused: bool,
    /// Current site
    pub site: Option<Arc<Site>>,
    /// The location handling request
    pub location: Option<(String, Arc<ProxyLocation>)>,
    /// Number of request body bytes received so far (for client_max_body_size enforcement)
    pub request_body_size: usize,
    /// Address of the upstream server
    pub upstream_address: String,
    /// Client's IP address
    pub client_ip: Option<String>,
    /// Remote connection port
    pub remote_port: Option<u16>,
    /// Remote connection address
    pub remote_addr: Option<String>,
    /// Server's listening port
    pub server_port: Option<u16>,
    /// Server's address
    pub server_addr: Option<String>,
    /// Upgrade Header
    pub http_upgrade: Option<String>,
    /// Unique identifier for the request
    pub request_id: Option<String>,
    /// Time spent looking up cache entries (in milliseconds)
    pub cache_lookup_time: Option<u64>,
    /// Time spent acquiring cache locks (in milliseconds)
    pub cache_lock_time: Option<u64>,
    /// Indicates if the upstream connection is reused
    pub upstream_reused: bool,
    /// Time taken to establish/reuse upstream connection (in milliseconds)
    pub upstream_connect_time: Option<u64>,
    /// Current number of active upstream connections
    pub upstream_connected: Option<i32>,
    /// Time taken for TCP connection to upstream (in milliseconds)
    pub upstream_tcp_connect_time: Option<u64>,
    /// Time taken for TLS handshake with upstream (in milliseconds)
    pub upstream_tls_handshake_time: Option<u64>,
    /// Time taken by upstream server to process request (in milliseconds)
    pub upstream_processing_time: Option<u64>,
    /// Total time taken by upstream server (in milliseconds)
    pub upstream_response_time: Option<u64>,
    /// Total time the upstream connection has been alive (in milliseconds)
    /// May be large for reused connections
    pub upstream_connection_time: Option<u64>,
    /// Statistics about response compression
    pub compression_stat: Option<CompressionStat>,
    /// Custom variables map for request processing
    variables: AHashMap<String, String>,
    pub rate: Option<Arc<RateLimit>>,
}

impl AigwCtx {
    pub fn new() -> Self {
        Self {
            created_at: now_ms(),
            ..Default::default()
        }
    }
    /// Adds a variable to the state's variables map with the given key and value.
    /// The key will be automatically prefixed with '$' before being stored.
    ///
    /// # Arguments
    /// * `key` - The variable name (will be prefixed with '$')
    /// * `value` - The value to store for this variable
    #[inline]
    pub fn add_variable(&mut self, key: &str, value: &str) {
        let key = format!("${key}");
        self.variables.insert(key, value.to_string());
    }

    pub fn get_variable(&self, key: &str) -> Option<&String> {
        let key = format!("${key}");
        self.variables.get(&key)
    }

    pub fn get_variables(&self) -> &AHashMap<String, String> {
        &self.variables
    }

    /// Returns the upstream response time if it's less than one hour, otherwise None.
    /// This helps filter out potentially invalid or stale timing data.
    ///
    /// Returns: Option<u64> representing milliseconds, or None if time exceeds 1 hour
    #[inline]
    pub fn get_upstream_response_time(&self) -> Option<u64> {
        if let Some(value) = self.upstream_response_time
            && value < ONE_HOUR_MS
        {
            return Some(value);
        }
        None
    }

    /// Returns the upstream connect time if it's less than one hour, otherwise None.
    /// This helps filter out potentially invalid or stale timing data.
    ///
    /// Returns: Option<u64> representing milliseconds, or None if time exceeds 1 hour
    #[inline]
    pub fn get_upstream_connect_time(&self) -> Option<u64> {
        if let Some(value) = self.upstream_connect_time
            && value < ONE_HOUR_MS
        {
            return Some(value);
        }
        None
    }

    /// Returns the upstream processing time if it's less than one hour, otherwise None.
    /// This helps filter out potentially invalid or stale timing data.
    ///
    /// Returns: Option<u64> representing milliseconds, or None if time exceeds 1 hour
    #[inline]
    pub fn get_upstream_processing_time(&self) -> Option<u64> {
        if let Some(value) = self.upstream_processing_time
            && value < ONE_HOUR_MS
        {
            return Some(value);
        }
        None
    }

    /// Appends a formatted value to the provided buffer based on the given key.
    /// Handles various metrics including connection info, timing data, and TLS details.
    ///
    /// # Arguments
    /// * `buf` - The BytesMut buffer to append the value to
    /// * `key` - The key identifying which state value to format and append
    ///
    /// Returns: The modified BytesMut buffer
    #[inline]
    pub fn append_value(&self, mut buf: BytesMut, key: &str) -> BytesMut {
        match key {
            "connection_id" => {
                buf.extend(itoa::Buffer::new().format(self.connection_id).as_bytes());
            }
            "upstream_reused" => {
                if self.upstream_reused {
                    buf.extend(b"true");
                } else {
                    buf.extend(b"false");
                }
            }
            "upstream_addr" => buf.extend(self.upstream_address.as_bytes()),
            "processing" => buf.extend(itoa::Buffer::new().format(self.processing).as_bytes()),
            "upstream_connect_time" => {
                if let Some(ms) = self.get_upstream_connect_time() {
                    buf.extend(itoa::Buffer::new().format(ms).as_bytes());
                }
            }
            "upstream_connect_time_human" => {
                if let Some(ms) = self.get_upstream_connect_time() {
                    buf = format_duration(buf, ms);
                }
            }
            "upstream_connected" => {
                if let Some(value) = self.upstream_connected {
                    buf.extend(itoa::Buffer::new().format(value).as_bytes());
                }
            }
            "upstream_processing_time" => {
                if let Some(ms) = self.get_upstream_processing_time() {
                    buf.extend(itoa::Buffer::new().format(ms).as_bytes());
                }
            }
            "upstream_processing_time_human" => {
                if let Some(ms) = self.get_upstream_processing_time() {
                    buf = format_duration(buf, ms);
                }
            }
            "upstream_response_time" => {
                if let Some(ms) = self.get_upstream_response_time() {
                    buf.extend(itoa::Buffer::new().format(ms).as_bytes());
                }
            }
            "upstream_response_time_human" => {
                if let Some(ms) = self.get_upstream_response_time() {
                    buf = format_duration(buf, ms);
                }
            }
            "upstream_tcp_connect_time" => {
                if let Some(ms) = self.upstream_tcp_connect_time {
                    buf.extend(itoa::Buffer::new().format(ms).as_bytes());
                }
            }
            "upstream_tcp_connect_time_human" => {
                if let Some(ms) = self.upstream_tcp_connect_time {
                    buf = format_duration(buf, ms);
                }
            }
            "upstream_tls_handshake_time" => {
                if let Some(ms) = self.upstream_tls_handshake_time {
                    buf.extend(itoa::Buffer::new().format(ms).as_bytes());
                }
            }
            "upstream_tls_handshake_time_human" => {
                if let Some(ms) = self.upstream_tls_handshake_time {
                    buf = format_duration(buf, ms);
                }
            }
            "upstream_connection_time" => {
                if let Some(ms) = self.upstream_connection_time {
                    buf.extend(itoa::Buffer::new().format(ms).as_bytes());
                }
            }
            "upstream_connection_time_human" => {
                if let Some(ms) = self.upstream_connection_time {
                    buf = format_duration(buf, ms);
                }
            }
            "location" => {
                if let Some((location, _)) = &self.location {
                    buf.extend(location.as_bytes())
                }
            }
            "connection_time" => {
                buf.extend(itoa::Buffer::new().format(self.connection_time).as_bytes());
            }
            "connection_time_human" => buf = format_duration(buf, self.connection_time),
            "connection_reused" => {
                if self.connection_reused {
                    buf.extend(b"true");
                } else {
                    buf.extend(b"false");
                }
            }
            "tls_version" => {
                if let Some(value) = &self.tls_version {
                    buf.extend(value.as_bytes());
                }
            }
            "tls_cipher" => {
                if let Some(value) = &self.tls_cipher {
                    buf.extend(value.as_bytes());
                }
            }
            "tls_handshake_time" => {
                if let Some(ms) = self.tls_handshake_time {
                    buf.extend(itoa::Buffer::new().format(ms).as_bytes());
                }
            }
            "tls_handshake_time_human" => {
                if let Some(value) = self.tls_handshake_time {
                    buf = format_duration(buf, value);
                }
            }
            "compression_time" => {
                if let Some(value) = &self.compression_stat {
                    buf.extend(
                        itoa::Buffer::new()
                            .format(value.duration.as_millis() as u64)
                            .as_bytes(),
                    );
                }
            }
            "compression_time_human" => {
                if let Some(value) = &self.compression_stat {
                    buf = format_duration(buf, value.duration.as_millis() as u64);
                }
            }
            "compression_ratio" => {
                if let Some(value) = &self.compression_stat {
                    buf.extend(format!("{:.1}", value.ratio()).as_bytes());
                }
            }
            "cache_lookup_time" => {
                if let Some(ms) = self.cache_lookup_time {
                    buf.extend(itoa::Buffer::new().format(ms).as_bytes());
                }
            }
            "cache_lookup_time_human" => {
                if let Some(ms) = self.cache_lookup_time {
                    buf = format_duration(buf, ms);
                }
            }
            "cache_lock_time" => {
                if let Some(ms) = self.cache_lock_time {
                    buf.extend(itoa::Buffer::new().format(ms).as_bytes());
                }
            }
            "cache_lock_time_human" => {
                if let Some(ms) = self.cache_lock_time {
                    buf = format_duration(buf, ms);
                }
            }
            "service_time" => {
                buf.extend(
                    itoa::Buffer::new()
                        .format(now_ms().saturating_sub(self.created_at))
                        .as_bytes(),
                );
            }
            "service_time_human" => {
                buf = format_duration(buf, now_ms().saturating_sub(self.created_at))
            }
            _ => {}
        }
        buf
    }
}
