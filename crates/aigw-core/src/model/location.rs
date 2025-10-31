use ahash::AHashMap;
use pingora_http::RequestHeader;
use pingora_load_balancing::{LoadBalancer, selection::Consistent};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, path::PathBuf, sync::Arc};
use substring::Substring;
use thiserror::Error;
use tracing::error;

use crate::{HttpHeader, http::convert_headers, util::regex::RegexCapture};

#[derive(Error, Debug)]
pub enum LocationError {
    #[error("Request Entity Too Large, max:{0}")]
    BodyTooLarge(usize),
}

#[derive(Debug)]
pub struct RegexPath {
    value: RegexCapture,
}

#[derive(Debug)]
pub struct PrefixPath {
    value: String,
}

#[derive(Debug)]
pub struct EqualPath {
    value: String,
}

// PathSelector enum represents different ways to match request paths:
// - RegexPath: Uses regex pattern matching
// - PrefixPath: Matches if path starts with prefix
// - EqualPath: Matches exact path
// - Empty: Matches all paths
#[derive(Debug)]
pub enum PathSelector {
    RegexPath(String, RegexPath),
    PrefixPath(String, PrefixPath),
    EqualPath(String, EqualPath),
    Empty,
}

impl PathSelector {
    pub fn as_str(&self) -> &str {
        match self {
            PathSelector::RegexPath(p, _) => p,
            PathSelector::PrefixPath(p, _) => p,
            PathSelector::EqualPath(p, _) => p,
            PathSelector::Empty => "",
        }
    }
}

/// Creates a new path selector based on the input path string.
///
/// # Arguments
/// * `path` - The path pattern string to parse
///
/// # Returns
/// * `Result<PathSelector>` - The parsed path selector or error
///
/// # Path Format
/// - Empty string: Matches all paths
/// - Starting with "~": Regex pattern matching
/// - Starting with "=": Exact path matching  
/// - Otherwise: Prefix path matching
pub fn new_path_selector(path: &str) -> anyhow::Result<PathSelector> {
    let path = path.trim();
    if path.is_empty() {
        return Ok(PathSelector::Empty);
    }
    let first = path.chars().next().unwrap_or_default();
    let last = path.substring(1, path.len()).trim();
    let se = match first {
        '~' => {
            let re = RegexCapture::new(last).map_err(|e| anyhow::anyhow!(e))?;
            PathSelector::RegexPath(path.to_owned(), RegexPath { value: re })
        }
        '=' => PathSelector::EqualPath(
            path.to_owned(),
            EqualPath {
                value: last.to_string(),
            },
        ),
        _ => {
            // trim
            PathSelector::PrefixPath(
                path.to_owned(),
                PrefixPath {
                    value: path.to_string(),
                },
            )
        }
    };

    Ok(se)
}

pub fn new_rewrite(rewrite: Option<&str>) -> anyhow::Result<Option<(Regex, String)>> {
    if let Some(rewrite) = rewrite {
        let mut arr: Vec<&str> = rewrite.split(' ').collect();
        if arr.len() == 1 && arr[0].contains("$") {
            arr.push(arr[0]);
            arr[0] = ".*";
        }

        let value = if arr.len() == 2 { arr[1] } else { "" };
        let re = Regex::new(arr[0])?;
        Ok(Some((re, value.to_string())))
    } else {
        Ok(None)
    }
}

/// Get the content length from http request header.
fn get_content_length(header: &RequestHeader) -> Option<usize> {
    if let Some(content_length) = header.headers.get(http::header::CONTENT_LENGTH) {
        if let Ok(size) = content_length.to_str().unwrap_or_default().parse::<usize>() {
            return Some(size);
        }
    }
    None
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub enum BanckedProtocol {
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "https")]
    Https,
}

impl Display for BanckedProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            BanckedProtocol::Http => write!(f, "http"),
            BanckedProtocol::Https => write!(f, "https"),
        }
    }
}

impl TryFrom<&str> for BanckedProtocol {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "http" => Ok(BanckedProtocol::Http),
            "https" => Ok(BanckedProtocol::Https),
            _ => Err(anyhow::anyhow!("Protocol not supported.")),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ProxyLocation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(
        serialize_with = "serialize_path",
        deserialize_with = "deserialize_path"
    )]
    pub path: Arc<PathSelector>,
    pub proxy: bool,
    pub protocol: BanckedProtocol,
    #[serde(serialize_with = "serialize_lb", deserialize_with = "deserialize_lb")]
    pub lb: Arc<LoadBalancer<Consistent>>,
    pub upstream: Vec<String>,
    pub connection_timeout: u32,
    pub read_timeout: u32,
    pub write_timeout: u32,
    pub idle_timeout: u32,
    /// Server Name Indication value for TLS connections
    /// Special value "$host" means use the request's Host header
    pub sni: String,
    /// Maximum allowed size of client request body in bytes
    /// Zero means unlimited. Requests exceeding this limit receive 413 error
    pub client_max_body_size: usize,
    /// Optional URL rewriting rule consisting of:
    /// - regex pattern to match against request path
    /// - replacement string with optional capture group references
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_rewrite",
        deserialize_with = "deserialize_rewrite"
    )]
    pub rewrite: Option<(Regex, String)>,

    /// Additional headers to append to proxied requests
    /// These are added without removing existing headers
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_http_headers",
        deserialize_with = "deserialize_http_headers"
    )]
    pub proxy_add_headers: Option<Vec<HttpHeader>>,

    /// Headers to set on proxied requests
    /// These override any existing headers with the same name
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_http_headers",
        deserialize_with = "deserialize_http_headers"
    )]
    pub proxy_set_headers: Option<Vec<HttpHeader>>,
    pub auto_index: bool,
    // root dir
    pub root_dir: Option<PathBuf>,
}

impl ProxyLocation {
    /// Validates that the request's Content-Length header does not exceed the configured maximum
    ///
    /// # Arguments
    /// * `header` - The HTTP request header to validate
    ///
    /// # Returns
    /// * `Result<()>` - Ok if validation passes, Error::BodyTooLarge if content length exceeds limit
    ///
    /// # Notes
    /// - Returns Ok if client_max_body_size is 0 (unlimited)
    /// - Uses get_content_length() helper to parse the Content-Length header
    #[inline]
    pub fn validate_content_length(&self, header: &RequestHeader) -> anyhow::Result<()> {
        if self.client_max_body_size == 0 {
            return Ok(());
        }
        if get_content_length(header).unwrap_or_default() > self.client_max_body_size {
            return Err(LocationError::BodyTooLarge(self.client_max_body_size).into());
        }

        Ok(())
    }

    /// Applies URL rewriting rules if configured for this location.
    ///
    /// This method performs path rewriting based on regex patterns and replacement rules.
    /// It supports variable interpolation from captured values in the host matching.
    ///
    /// # Arguments
    /// * `header` - Mutable reference to the request header containing the URI to rewrite
    /// * `variables` - Optional map of variables captured from host matching that can be interpolated
    ///   into the replacement value
    ///
    /// # Returns
    /// * `bool` - Returns true if the path was rewritten, false if no rewriting was performed
    ///
    /// # Examples
    /// ```
    /// // Configuration example:
    /// // rewrite: "^/users/(.*)$ /api/users/$1"
    /// // This would rewrite "/users/123" to "/api/users/123"
    /// ```
    ///
    /// # Notes
    /// - Preserves query parameters when rewriting the path
    /// - Logs debug information about path rewrites
    /// - Logs errors if the new path cannot be parsed as a valid URI
    #[inline]
    pub fn rewrite(
        &self,
        header: &mut RequestHeader,
        variables: Option<&AHashMap<String, String>>,
    ) -> bool {
        if let Some((re, value)) = &self.rewrite {
            let mut replace_value = value.to_string();
            // replace variables for rewrite value
            if let Some(variables) = variables {
                for (k, v) in variables.iter() {
                    replace_value = replace_value.replace(k, v);
                }
            }
            let path = header.uri.path();
            let mut new_path = if re.to_string() == ".*" {
                replace_value
            } else {
                re.replace(path, replace_value).to_string()
            };
            if path == new_path {
                return false;
            }
            // preserve query parameters
            if let Some(query) = header.uri.query() {
                new_path = format!("{new_path}?{query}");
            }
            // set new uri
            if let Err(e) = new_path.parse::<http::Uri>().map(|uri| header.set_uri(uri)) {
                error!("new path parse fail, {:?}", e);
            }
            return true;
        }
        false
    }
}

fn serialize_path<S>(value: &Arc<PathSelector>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match &**value {
        PathSelector::RegexPath(path, _) => serializer.serialize_str(path),
        PathSelector::PrefixPath(path, _) => serializer.serialize_str(path),
        PathSelector::EqualPath(path, _) => serializer.serialize_str(path),
        PathSelector::Empty => serializer.serialize_str(""),
    }
}

fn deserialize_path<'de, D>(deserializer: D) -> Result<Arc<PathSelector>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let path = String::deserialize(deserializer)?;
    let path = new_path_selector(&path)
        .map_err(|_| serde::de::Error::custom("PathSelector decode error"))?;
    Ok(Arc::new(path))
}

fn serialize_lb<S>(value: &Arc<LoadBalancer<Consistent>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let backends: Vec<String> = value
        .backends()
        .get_backend()
        .iter()
        .map(|b| b.addr.to_string())
        .collect();
    serializer.collect_seq(backends)
}

fn deserialize_lb<'de, D>(deserializer: D) -> Result<Arc<LoadBalancer<Consistent>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let backends: Vec<String> = Vec::deserialize(deserializer)?;
    let lb: LoadBalancer<Consistent> = LoadBalancer::try_from_iter(backends.iter())
        .map_err(|_| serde::de::Error::custom("LoadBalancer decode error"))?;

    Ok(Arc::new(lb))
}

fn serialize_rewrite<S>(value: &Option<(Regex, String)>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    if let Some(value) = value {
        let s = "".to_owned() + value.0.as_str() + " " + value.1.as_str();
        serializer.serialize_str(s.as_str().trim())
    } else {
        serializer.serialize_none()
    }
}

fn deserialize_rewrite<'de, D>(deserializer: D) -> Result<Option<(Regex, String)>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // rewrite: "^/users/(.*)$ /api/users/$1"
    let rewrite = String::deserialize(deserializer)?;
    new_rewrite(Some(&rewrite)).map_err(|_| serde::de::Error::custom("Regex compile error"))
}

fn serialize_http_headers<S>(
    value: &Option<Vec<HttpHeader>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut headers = vec![];
    if let Some(h) = value {
        for (k, v) in h {
            let mut map = HashMap::new();
            map.insert("name", k.as_str());
            map.insert(
                "value",
                v.to_str()
                    .map_err(|_| serde::ser::Error::custom("HeaderValue error"))?,
            );

            headers.push(map);
        }
    }

    serializer.collect_seq(headers)
}

fn deserialize_http_headers<'de, D>(deserializer: D) -> Result<Option<Vec<HttpHeader>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let headers: Vec<HashMap<String, String>> = Vec::deserialize(deserializer)?;
    if headers.is_empty() {
        Ok(None)
    } else {
        let r = convert_headers(&headers)
            .map_err(|_| serde::de::Error::custom("Convert headers error"))?;
        Ok(Some(r))
    }
}

pub fn find_matched_location(
    locations: &[Arc<ProxyLocation>],
    path: &str,
) -> Option<(Arc<ProxyLocation>, Vec<(String, String)>)> {
    // Stage 1: Exact match (=) — highest priority
    for location in locations {
        match &*location.path {
            PathSelector::EqualPath(_, EqualPath { value }) => {
                if value == path {
                    return Some((location.clone(), vec![]));
                }
            }
            _ => {}
        }
    }

    // Stage 2: Non-exact matches
    let mut best_prefix: Option<(&Arc<ProxyLocation>, usize)> = None;
    let mut empty_match: Option<&Arc<ProxyLocation>> = None;

    for location in locations {
        match &*location.path {
            // For exact path matching, compare path strings directly
            PathSelector::EqualPath(_, _) => {
                continue;
            }
            // For regex path matching, use regex is_match
            PathSelector::RegexPath(_, RegexPath { value }) => {
                let (matched, captures) = value.captures(path);
                if matched {
                    // Assuming captures is Some(_) when matched; if not, use unwrap_or_default()
                    return Some((location.clone(), captures.unwrap_or_default()));
                }
            }
            // For prefix path matching, check if path starts with prefix
            PathSelector::PrefixPath(_, PrefixPath { value }) => {
                if path.starts_with(value) {
                    let len = value.len();
                    if best_prefix.as_ref().map_or(true, |&(_, l)| len > l) {
                        best_prefix = Some((location, len));
                    }
                }
            }
            PathSelector::Empty => {
                // Empty matches everything, but lowest priority
                if empty_match.is_none() {
                    empty_match = Some(location);
                }
            }
        }
    }

    // Stage 3: Return longest prefix or empty
    if let Some((location, _)) = best_prefix {
        return Some((location.clone(), vec![]));
    }

    if let Some(location) = empty_match {
        return Some((location.clone(), vec![]));
    }
    None
}
