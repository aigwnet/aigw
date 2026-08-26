use std::{
    cmp::min,
    collections::HashSet,
    fmt::Display,
    io::{ErrorKind, SeekFrom},
    path::{Path, PathBuf},
    str::FromStr,
    time::SystemTime,
};

use bytes::{Bytes, BytesMut};
use http::{Method, StatusCode, header};
use mime_guess::{Mime, mime::FromStrError};
use pingora_core::{Error, ErrorType, modules::http::compression::ResponseCompression};
use pingora_http::ResponseHeader;
use pingora_proxy::Session;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tracing::{debug, error, warn};

use crate::{
    SERVER,
    server::runtime::{
        error_page,
        file::{
            auto_index::build_auto_index,
            path::{path_to_uri, resolve_uri},
        },
    },
};

/// Encapsulates the compression state for the current session.
struct Compression<'a> {
    precompressed: &'a [CompressionAlgorithm],
    precompressed_active: Option<CompressionAlgorithm>,
    dynamic: bool,
}

impl<'a> Compression<'a> {
    /// Creates a new compression state supporting the given compression algorithms for
    /// pre-compressed files. *Note*: Dynamic compression is determined by the Pingora session.
    fn new(session: &Session, precompressed: &'a [CompressionAlgorithm]) -> Self {
        Self {
            precompressed,
            precompressed_active: None,
            // Remember this now, later on request header check might flip this flag
            dynamic: session
                .downstream_modules_ctx
                .get::<ResponseCompression>()
                .is_some_and(|compression| compression.is_enabled()),
        }
    }

    /// Checks whether the given path should be rewritten to a pre-compressed version of the file.
    fn rewrite_path(&mut self, session: &Session, path: &Path) -> Option<PathBuf> {
        if self.precompressed.is_empty() {
            return None;
        }

        let filename = path.file_name()?;
        let requested = session.req_header().headers.get(header::ACCEPT_ENCODING)?;
        let overlap = find_matches(requested.to_str().ok()?, self.precompressed);

        for algorithm in overlap {
            let mut candidate_name = filename.to_os_string();
            candidate_name.push(".");
            candidate_name.push(algorithm.ext());

            let mut candidate_path = path.to_path_buf();
            candidate_path.set_file_name(candidate_name);
            if candidate_path.is_file() {
                self.precompressed_active = Some(algorithm);
                return Some(candidate_path);
            }
        }

        None
    }

    /// Applies the necessary modification to the HTTP response if compression is active. This will
    /// add `Content-Encoding` HTTP header among other thins.
    pub(crate) fn transform_header(
        &mut self,
        _session: &mut Session,
        mut header: Box<ResponseHeader>,
    ) -> Result<Box<ResponseHeader>, Box<Error>> {
        let mut header =
            if header.status != StatusCode::OK && header.status != StatusCode::PARTIAL_CONTENT {
                // No actual content here, so no compression
                header
            } else if let Some(algorithm) = self.precompressed_active {
                // File is pre-compressed, only need to adjust header
                header.insert_header(header::CONTENT_ENCODING, algorithm.name())?;
                header
            } else {
                // Pingora’s dynamic compression will take care of this if necessary
                header
            };

        if !self.precompressed.is_empty() || self.dynamic {
            // If compression is enabled, we might produce different responses based on
            // Accept-Encoding header. Make sure to let the client know regardless of whether
            // compression is active right now.
            //
            // Note: This should not be necessary for dynamic compression. Pingora won't currently
            // do it however, see https://github.com/cloudflare/pingora/issues/233
            header.insert_header(header::VARY, "Accept-Encoding")?;
        }
        Ok(header)
    }
}

/// Represents a compression algorithm choice.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize)]
enum CompressionAlgorithm {
    /// gzip compression
    #[serde(rename = "gz")]
    Gzip,
    /// deflate (zlib) compression
    #[serde(rename = "zz")]
    Deflate,
    /// compress compression
    #[serde(rename = "z")]
    Compress,
    /// Brotli compression
    #[serde(rename = "br")]
    Brotli,
    /// Zstandard compression
    #[serde(rename = "zst")]
    Zstandard,
}

impl CompressionAlgorithm {
    /// Returns the file extension corresponding to the algorithm.
    pub fn ext(&self) -> &'static str {
        match self {
            Self::Gzip => "gz",
            Self::Deflate => "zz",
            Self::Compress => "z",
            Self::Brotli => "br",
            Self::Zstandard => "zst",
        }
    }

    /// Determines the algorithm corresponding to the file extension if any.
    pub fn from_ext(ext: &str) -> Option<Self> {
        match ext {
            "gz" => Some(Self::Gzip),
            "zz" => Some(Self::Deflate),
            "z" => Some(Self::Compress),
            "br" => Some(Self::Brotli),
            "zst" => Some(Self::Zstandard),
            _ => None,
        }
    }

    /// Returns the algorithm name as used in `Accept-Encoding` HTTP header.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Gzip => "gzip",
            Self::Deflate => "deflate",
            Self::Compress => "compress",
            Self::Brotli => "br",
            Self::Zstandard => "zstd",
        }
    }

    /// Determines the algorithm corresponding to a name from `Accept-Encoding` HTTP header.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "gzip" => Some(Self::Gzip),
            "deflate" => Some(Self::Deflate),
            "compress" => Some(Self::Compress),
            "br" => Some(Self::Brotli),
            "zstd" => Some(Self::Zstandard),
            _ => None,
        }
    }
}

impl FromStr for CompressionAlgorithm {
    type Err = UnsupportedCompressionAlgorithm;

    /// Coverts a file extension into a compression algorithm.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CompressionAlgorithm::from_ext(s).ok_or(UnsupportedCompressionAlgorithm(s.to_owned()))
    }
}

impl Display for CompressionAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.name())
    }
}

/// The error type returned by `CompressionAlgorithm::from_str()`
#[derive(Debug, PartialEq, Eq)]
struct UnsupportedCompressionAlgorithm(String);

impl Display for UnsupportedCompressionAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "Unsupported compression algorithm: {}", self.0)
    }
}

/// Parses an encoding specifier from `Accept-Encoding` HTTP header into an
/// algorithm/quality pair.
fn parse_encoding(encoding: &str) -> Option<(&str, u16)> {
    let mut params = encoding.split(';');
    let algorithm = params.next()?.trim();
    let mut quality = 1000;
    for param in params {
        if let Some((name, value)) = param.split_once('=')
            && name.trim() == "q"
            && let Ok(value) = f64::from_str(value.trim())
        {
            quality = (value * 1000.0) as u16;
        }
    }
    Some((algorithm, quality))
}

/// Compares the requested encodings from `Accept-Encoding` HTTP header with a list of supported
/// algorithms and returns any matches, sorted by the respective quality value.
fn find_matches(requested: &str, supported: &[CompressionAlgorithm]) -> Vec<CompressionAlgorithm> {
    let mut requested = requested
        .split(',')
        .filter_map(parse_encoding)
        .collect::<Vec<_>>();
    requested.sort_by_key(|(_, quality)| -(*quality as i32));

    let mut result = Vec::new();
    for (algorithm, _) in requested {
        if algorithm == "*" {
            for algorithm in supported {
                if !result.contains(algorithm) {
                    result.push(*algorithm);
                }
            }
            break;
        } else if let Some(algorithm) = CompressionAlgorithm::from_name(algorithm)
            && supported.contains(&algorithm)
            && !result.contains(&algorithm)
        {
            result.push(algorithm);
        }
    }
    result
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
enum MimeMatch {
    Exact(Mime),
    Type(String),
    Prefix(String),
    Suffix(String),
}

impl TryFrom<&str> for MimeMatch {
    type Error = FromStrError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(if let Some(prefix) = value.strip_suffix('*') {
            if let Some(type_) = prefix.strip_suffix('/') {
                Self::Type(type_.to_owned())
            } else {
                Self::Prefix(prefix.to_owned())
            }
        } else if let Some(suffix) = value.strip_prefix('*') {
            Self::Suffix(suffix.to_owned())
        } else {
            Self::Exact(value.parse()?)
        })
    }
}

impl TryFrom<String> for MimeMatch {
    type Error = FromStrError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MimeMatcher {
    exact: HashSet<Mime>,
    type_: HashSet<String>,
    prefix: Vec<String>,
    suffix: Vec<String>,
}

impl MimeMatcher {
    pub(crate) fn new() -> Self {
        Self {
            exact: HashSet::new(),
            type_: HashSet::new(),
            prefix: Vec::new(),
            suffix: Vec::new(),
        }
    }

    pub(crate) fn add(&mut self, mime: MimeMatch) {
        match mime {
            MimeMatch::Exact(mime) => {
                self.exact.insert(mime);
            }
            MimeMatch::Type(type_) => {
                self.type_.insert(type_);
            }
            MimeMatch::Prefix(prefix) => self.prefix.push(prefix),
            MimeMatch::Suffix(suffix) => self.suffix.push(suffix),
        }
    }

    pub(crate) fn matches(&self, mime: &Mime) -> bool {
        self.exact.contains(mime)
            || self.type_.contains(mime.type_().as_str())
            || self
                .prefix
                .iter()
                .any(|prefix| mime.as_ref().starts_with(prefix))
            || self
                .suffix
                .iter()
                .any(|suffix| mime.as_ref().ends_with(suffix))
    }
}

/// Helper wrapping file metadata information
#[derive(Debug)]
struct Metadata {
    /// Guessed MIME types (if any) for the file
    pub mime: Mime,
    /// File size in bytes
    pub size: u64,
    /// Last modified time of the file in the format `Fri, 15 May 2015 15:34:21 GMT` if the time
    /// can be retrieved
    pub modified: Option<String>,
    /// ETag header for the file, encoding last modified time and file size
    pub etag: String,
}

impl Metadata {
    /// Collects the metadata for a file. If `orig_path` is present, it will be used to determine
    /// the MIME type instead of `path`.
    ///
    /// This method will return any errors produced by [`std::fs::metadata()`]. It will also result
    /// in a [`ErrorKind::InvalidInput`] error if the path given doesn’t point to a regular file.
    pub async fn from_path<P: AsRef<Path> + ?Sized>(
        path: &P,
        orig_path: Option<&P>,
    ) -> Result<Self, std::io::Error> {
        let meta = tokio::fs::metadata(path).await?;

        if !meta.is_file() {
            return Err(ErrorKind::InvalidInput.into());
        }

        let mime = mime_guess::from_path(orig_path.unwrap_or(path)).first_or_octet_stream();
        let size = meta.len();
        let modified = meta.modified().ok().map(httpdate::fmt_http_date);
        let etag = format!(
            "\"{:x}-{:x}\"",
            meta.modified()
                .ok()
                .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map_or(0, |duration| duration.as_secs()),
            meta.len()
        );

        Ok(Self {
            mime,
            size,
            modified,
            etag,
        })
    }

    /// Checks `If-Match` and `If-Unmodified-Since` headers of the request to determine whether
    /// a `412 Precondition Failed` response should be produced.
    pub fn has_failed_precondition(&self, session: &Session) -> bool {
        let headers = &session.req_header().headers;
        if let Some(value) = headers
            .get(header::IF_MATCH)
            .and_then(|value| value.to_str().ok())
        {
            value != "*"
                && value
                    .split(',')
                    .map(str::trim)
                    .all(|value| value != self.etag)
        } else if let Some(value) = headers
            .get(header::IF_UNMODIFIED_SINCE)
            .and_then(|value| value.to_str().ok())
        {
            self.modified
                .as_ref()
                .is_some_and(|modified| modified != value)
        } else {
            false
        }
    }

    /// Checks `If-None-Match` and `If-Modified-Since` headers of the request to determine whether
    /// a `304 Not Modified` response should be produced.
    pub fn is_not_modified(&self, session: &Session) -> bool {
        let headers = &session.req_header().headers;
        if let Some(value) = headers
            .get(header::IF_NONE_MATCH)
            .and_then(|value| value.to_str().ok())
        {
            value == "*"
                || value
                    .split(',')
                    .map(str::trim)
                    .any(|value| value == self.etag)
        } else if let Some(value) = headers
            .get(header::IF_MODIFIED_SINCE)
            .and_then(|value| value.to_str().ok())
        {
            self.modified
                .as_ref()
                .is_some_and(|modified| modified == value)
        } else {
            false
        }
    }

    #[inline(always)]
    fn add_content_type(
        &self,
        header: &mut ResponseHeader,
        charset: Option<&str>,
    ) -> Result<(), Box<Error>> {
        let mime_type = self.mime.as_ref();
        if let Some(charset) = charset {
            header.append_header(
                header::CONTENT_TYPE,
                format!("{};charset={charset}", mime_type),
            )?;
        } else {
            header.append_header(header::CONTENT_TYPE, mime_type)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn add_etag(&self, header: &mut ResponseHeader) -> Result<(), Box<Error>> {
        if let Some(modified) = &self.modified {
            header.append_header(header::LAST_MODIFIED, modified)?;
        }
        header.append_header(header::ETAG, &self.etag)?;
        Ok(())
    }

    /// Produces a `200 OK` response and adds headers according to file metadata.
    pub(crate) fn to_response_header(
        &self,
        charset: Option<&str>,
    ) -> Result<Box<ResponseHeader>, Box<Error>> {
        let mut header = ResponseHeader::build(StatusCode::OK, Some(8))?;
        header.append_header(header::CONTENT_LENGTH, self.size.to_string())?;
        header.append_header(header::ACCEPT_RANGES, "bytes")?;
        self.add_content_type(&mut header, charset)?;
        self.add_etag(&mut header)?;
        Ok(Box::new(header))
    }

    /// Produces a `206 Partial Content` response and adds headers according to file metadata.
    pub(crate) fn to_partial_content_header(
        &self,
        charset: Option<&str>,
        start: u64,
        end: u64,
    ) -> Result<Box<ResponseHeader>, Box<Error>> {
        let mut header = ResponseHeader::build(StatusCode::PARTIAL_CONTENT, Some(8))?;
        header.append_header(header::CONTENT_LENGTH, (end - start + 1).to_string())?;
        header.append_header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{}", self.size),
        )?;
        self.add_content_type(&mut header, charset)?;
        self.add_etag(&mut header)?;
        Ok(Box::new(header))
    }

    /// Produces a `416 Range Not Satisfiable` response and adds headers according to file
    /// metadata.
    pub(crate) fn to_not_satisfiable_header(
        &self,
        charset: Option<&str>,
    ) -> Result<Box<ResponseHeader>, Box<Error>> {
        let mut header = ResponseHeader::build(StatusCode::RANGE_NOT_SATISFIABLE, Some(4))?;
        header.append_header(header::CONTENT_RANGE, format!("bytes */{}", self.size))?;
        self.add_content_type(&mut header, charset)?;
        self.add_etag(&mut header)?;
        Ok(Box::new(header))
    }

    /// Produces a response with specified status code and no response body (all headers added
    /// except `Content-Length``).
    pub(crate) fn to_custom_header(
        &self,
        status: StatusCode,
    ) -> Result<Box<ResponseHeader>, Box<Error>> {
        let mut header = ResponseHeader::build(status, Some(4))?;
        self.add_etag(&mut header)?;
        Ok(Box::new(header))
    }
}

/// Represents the result of parsing the `Range` HTTP header.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Range {
    /// A valid range with the given start and end bounds
    Valid(u64, u64),
    /// A range that is outside of the file’s boundaries
    OutOfBounds,
}

impl Range {
    /// Parses the value of a `Range` HTTP header. The file size is required to resolve ranges
    /// specified relative to the end of file and to recognize out of bounds ranges. Ranges that
    /// cannot be parsed (unexpected format) will result in `None`.
    pub fn parse(range: &str, file_size: u64) -> Option<Self> {
        let (units, range) = range.split_once('=')?;
        if units != "bytes" {
            return None;
        }
        // Any range of an empty file is unsatisfiable (also avoids u64 underflow below)
        if file_size == 0 {
            return Some(Self::OutOfBounds);
        }

        let (start, end) = range.trim().split_once('-')?;
        let (start, end) = if start.is_empty() {
            let len = u64::from_str(end.trim()).ok()?;
            if len > file_size {
                return Some(Self::OutOfBounds);
            }
            (file_size - len, file_size - 1)
        } else if end.is_empty() {
            (u64::from_str(start.trim()).ok()?, file_size - 1)
        } else {
            (
                u64::from_str(start.trim()).ok()?,
                u64::from_str(end.trim()).ok()?,
            )
        };

        if end >= file_size || start > end {
            Some(Self::OutOfBounds)
        } else {
            Some(Self::Valid(start, end))
        }
    }
}

/// This processes the `Range` and `If-Range` request headers to produce the requested byte range
/// if any.
///
/// `Range` header missing, using some unsupported format or overruled by `If-Range` header will
/// all result in `None` being returned.
///
/// Note: Multiple ranges are not supported.
fn extract_range(session: &Session, meta: &Metadata) -> Option<Range> {
    let headers = &session.req_header().headers;
    // If-Range: only honor the Range when the validator matches (strong etag or
    // modification date); otherwise the client must receive the full entity.
    if let Some(value) = headers
        .get(header::IF_RANGE)
        .and_then(|value| value.to_str().ok())
    {
        let matches = value == meta.etag
            || meta
                .modified
                .as_ref()
                .is_some_and(|modified| modified == value);
        if !matches {
            return None;
        }
    }

    let value = headers.get(header::RANGE)?;
    let value = value.to_str().ok()?;

    Range::parse(value, meta.size)
}

/// Static Files handler
pub struct StaticFilesHandler {}

lazy_static::lazy_static! {
     static ref MIME_MTACHER: MimeMatcher = {
        let mut declare_charset_matcher = MimeMatcher::new();
        for mime in DEFAULT_TEXT_TYPES {
            declare_charset_matcher.add((*mime).try_into().unwrap());
        }
        declare_charset_matcher
     };

     static ref PRE_COMPRESSED: Vec<CompressionAlgorithm> = {
        Vec::<CompressionAlgorithm>::new()
     };

}

impl StaticFilesHandler {
    pub async fn handle(
        root: Option<&PathBuf>,
        index_file: &[&str],
        page_404: Option<&String>,
        auto_index: bool,
        session: &mut Session,
    ) -> Result<(), Box<Error>> {
        handle_file(
            session,
            root,
            true,
            index_file,
            page_404,
            &PRE_COMPRESSED,
            "utf-8",
            &MIME_MTACHER,
            auto_index,
        )
        .await
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_file(
    session: &mut Session,
    file_root: Option<&PathBuf>,
    canonicalize_uri: bool,
    index_file: &[&str],
    page_404: Option<&String>,
    precompressed: &[CompressionAlgorithm],
    declare_charset: &str,
    declare_charset_matcher: &MimeMatcher,
    auto_index: bool,
) -> Result<(), Box<Error>> {
    match session.req_header().method {
        Method::GET | Method::HEAD => {
            // Allowed
        }
        _ => {
            warn!("Denying method {}", session.req_header().method);
            html_response(
                session,
                StatusCode::METHOD_NOT_ALLOWED,
                error_page::get_error_page(StatusCode::METHOD_NOT_ALLOWED).into(),
                "text/html;charset=utf-8",
            )
            .await?;
            return Ok(());
        }
    }

    let uri = &session.req_header().uri;

    let root = if let Some(root) = file_root {
        root
    } else {
        debug!("received request but static files handler is not configured, ignoring");
        return Err(pingora_core::Error::new_str("Request Failed"));
    };

    debug!("received URI path {}", uri.path());

    let (mut path, not_found) = match resolve_uri(uri.path(), root) {
        Ok(path) => (path, false),
        Err(err) if err.kind() == ErrorKind::NotFound => {
            debug!("canonicalizing resulted in NotFound error");

            let path = page_404.as_ref().and_then(|page_404| {
                debug!("error page is {page_404}");
                match resolve_uri(page_404, root) {
                    Ok(path) => Some(path),
                    Err(err) => {
                        warn!("Failed resolving error page {page_404}: {err}");
                        None
                    }
                }
            });

            if let Some(path) = path {
                (path, true)
            } else {
                html_response(
                    session,
                    StatusCode::NOT_FOUND,
                    error_page::get_error_page(StatusCode::NOT_FOUND).into(),
                    "text/html;charset=utf-8",
                )
                .await?;
                return Ok(());
            }
        }
        Err(err) => {
            match err.kind() {
                ErrorKind::InvalidInput => {
                    warn!("rejecting invalid path {}", uri.path());
                    html_response(
                        session,
                        StatusCode::BAD_REQUEST,
                        error_page::get_error_page(StatusCode::BAD_REQUEST).into(),
                        "text/html;charset=utf-8",
                    )
                    .await?;
                }
                ErrorKind::InvalidData => {
                    warn!("Requested path outside root directory: {}", uri.path());
                    html_response(
                        session,
                        StatusCode::BAD_REQUEST,
                        error_page::get_error_page(StatusCode::BAD_REQUEST).into(),
                        "text/html;charset=utf-8",
                    )
                    .await?;
                }
                ErrorKind::PermissionDenied => {
                    debug!("canonicalizing resulted in PermissionDenied error");
                    html_response(
                        session,
                        StatusCode::FORBIDDEN,
                        error_page::get_error_page(StatusCode::FORBIDDEN).into(),
                        "text/html;charset=utf-8",
                    )
                    .await?;
                }
                _ => {
                    warn!("failed canonicalizing the path {}: {err}", uri.path());
                    html_response(
                        session,
                        StatusCode::INTERNAL_SERVER_ERROR,
                        error_page::get_error_page(StatusCode::INTERNAL_SERVER_ERROR).into(),
                        "text/html;charset=utf-8",
                    )
                    .await?;
                }
            };
            return Ok(());
        }
    };

    debug!("translated into file path {path:?}");

    if canonicalize_uri
        && !not_found
        && let Some(mut canonical) = path_to_uri(&path, root)
        && canonical != uri.path()
    {
        if let Some(query) = uri.query() {
            canonical.push('?');
            canonical.push_str(query);
        }

        if let Some(prefix) = uri
            .path()
            .strip_suffix(uri.path())
            .filter(|p| !p.is_empty())
        {
            // A prefix has been removed from the original URI, insert it for the
            // redirect.
            canonical.insert_str(0, prefix);
        }
        debug!("redirecting to canonical URI: {canonical}");
        redirect_response(session, StatusCode::PERMANENT_REDIRECT, &canonical).await?;
        return Ok(());
    }

    if path.is_dir() {
        for filename in index_file {
            let candidate = path.join(filename);
            if candidate.is_file() {
                debug!("using directory index file {filename}");
                path = candidate;
            }
        }
    }

    debug!("successfully resolved request path: {path:?}");

    let mut compression = Compression::new(session, precompressed);

    let (path, orig_path) =
        if let Some(precompressed_path) = compression.rewrite_path(session, &path) {
            (precompressed_path, Some(path))
        } else {
            (path, None)
        };

    // list files
    if path.is_dir() && auto_index {
        let s = build_auto_index(&path).await;
        html_response(session, StatusCode::OK, s.into(), "text/html;charset=utf-8").await?;
    } else {
        let meta = match Metadata::from_path(&path, orig_path.as_ref()).await {
            Ok(meta) => meta,
            Err(err) if err.kind() == ErrorKind::InvalidInput => {
                warn!("Path {path:?} is not a regular file, denying access");
                html_response(
                    session,
                    StatusCode::FORBIDDEN,
                    error_page::get_error_page(StatusCode::FORBIDDEN).into(),
                    "text/html;charset=utf-8",
                )
                .await?;
                return Ok(());
            }
            Err(err) => {
                warn!("failed retrieving metadata for path {path:?}: {err}");
                html_response(
                    session,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    error_page::get_error_page(StatusCode::INTERNAL_SERVER_ERROR).into(),
                    "text/html;charset=utf-8",
                )
                .await?;

                return Ok(());
            }
        };

        if meta.has_failed_precondition(session) {
            debug!("If-Match/If-Unmodified-Since precondition failed");
            let header = meta.to_custom_header(StatusCode::PRECONDITION_FAILED)?;
            let header = compression.transform_header(session, header)?;
            session.write_response_header(header, true).await?;
            return Ok(());
        }

        if meta.is_not_modified(session) {
            debug!("If-None-Match/If-Modified-Since check resulted in Not Modified");
            let header = meta.to_custom_header(StatusCode::NOT_MODIFIED)?;
            let header = compression.transform_header(session, header)?;
            session.write_response_header(header, true).await?;
            return Ok(());
        }

        let charset = if declare_charset_matcher.matches(&meta.mime) {
            Some(declare_charset)
        } else {
            None
        };

        let (mut header, start, end) = match extract_range(session, &meta) {
            Some(Range::Valid(start, end)) => {
                debug!("bytes range requested: {start}-{end}");
                let header = meta.to_partial_content_header(charset, start, end)?;
                let header = compression.transform_header(session, header)?;
                (header, start, end)
            }
            Some(Range::OutOfBounds) => {
                debug!("requested bytes range is out of bounds");
                let header = meta.to_not_satisfiable_header(charset)?;
                let header = compression.transform_header(session, header)?;
                session.write_response_header(header, true).await?;
                return Ok(());
            }
            None => {
                // Range is either missing or cannot be parsed, produce the entire file.
                let header = meta.to_response_header(charset)?;
                let header = compression.transform_header(session, header)?;
                (header, 0, if meta.size == 0 { 0 } else { meta.size - 1 })
            }
        };

        if not_found {
            header.set_status(StatusCode::NOT_FOUND)?;
        }

        let send_body = session.req_header().method != Method::HEAD;
        header.append_header(header::SERVER, SERVER)?;

        session.write_response_header(header, !send_body).await?;

        if send_body {
            // sendfile would be nice but not currently possible within pingora-proxy (see
            // https://github.com/cloudflare/pingora/issues/160)
            if meta.size > 0 {
                file_response(session, &path, start, end).await?;
            } else {
                // Empty file: Content-Length: 0 was already sent, just finish the stream
                session.write_response_body(None, true).await?;
            }
        }
    }

    Ok(())
}

const DEFAULT_TEXT_TYPES: &[&str] = &[
    "text/*",
    "*+xml",
    "*+json",
    "application/javascript",
    "application/json",
    "application/json5",
];

const BUFFER_SIZE: usize = 64 * 1024;

/// Writes a chunk of a file as a Pingora session response. The data will be passed through the
/// compression handler first in case dynamic compression is enabled.
async fn file_response(
    session: &mut Session,
    path: &Path,
    start: u64,
    end: u64,
) -> Result<(), Box<Error>> {
    let mut file = tokio::fs::File::open(path).await.map_err(|err| {
        error!("failed opening file {path:?}: {err}");
        Error::new(ErrorType::HTTPStatus(
            StatusCode::INTERNAL_SERVER_ERROR.into(),
        ))
    })?;

    if start != 0 {
        file.seek(SeekFrom::Start(start)).await.map_err(|err| {
            error!("failed seeking in file {path:?}: {err}");
            Error::new(ErrorType::HTTPStatus(
                StatusCode::INTERNAL_SERVER_ERROR.into(),
            ))
        })?;
    }

    let mut remaining = (end - start + 1) as usize;
    while remaining > 0 {
        let mut buf = BytesMut::zeroed(min(remaining, BUFFER_SIZE));
        let len = file.read(buf.as_mut()).await.map_err(|err| {
            error!("failed reading data from {path:?}: {err}");
            Error::new(ErrorType::HTTPStatus(
                StatusCode::INTERNAL_SERVER_ERROR.into(),
            ))
        })?;

        if len == 0 {
            error!("file ended with {remaining} bytes left to be written");
            return Err(Error::new(ErrorType::ReadError));
        }

        buf.truncate(len);
        session.write_response_body(Some(buf.into()), false).await?;
        remaining -= len;
    }

    session.write_response_body(None, true).await?;

    Ok(())
}

async fn html_response(
    session: &mut Session,
    status: StatusCode,
    text: Bytes,
    content_type: &str,
) -> Result<(), Box<Error>> {
    let mut header = ResponseHeader::build(status, Some(4))?;
    header.append_header(header::CONTENT_LENGTH, text.len().to_string())?;
    header.append_header(header::CONTENT_TYPE, content_type)?;
    header.append_header(header::SERVER, SERVER)?;

    let send_body = session.req_header().method != Method::HEAD;
    session
        .write_response_header(Box::new(header), !send_body)
        .await?;

    if send_body {
        session.write_response_body(Some(text), true).await?;
    }

    Ok(())
}

/// Responds with a redirect to the given location.
async fn redirect_response(
    session: &mut Session,
    status: StatusCode,
    location: &str,
) -> Result<(), Box<Error>> {
    let mut header = ResponseHeader::build(status, Some(4))?;
    header.append_header(header::CONTENT_LENGTH, "0".to_string())?;
    header.append_header(header::CONTENT_TYPE, "text/html;charset=utf-8")?;
    header.append_header(header::SERVER, SERVER)?;
    header.append_header(header::LOCATION, location)?;

    session
        .write_response_header(Box::new(header), true)
        .await?;

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::Range;

    #[test]
    fn test_range_parse_empty_file() {
        // No u64 underflow on empty files: any range is unsatisfiable
        assert_eq!(Range::parse("bytes=0-", 0), Some(Range::OutOfBounds));
        assert_eq!(Range::parse("bytes=-0", 0), Some(Range::OutOfBounds));
        assert_eq!(Range::parse("bytes=-5", 0), Some(Range::OutOfBounds));
    }

    #[test]
    fn test_range_parse_normal() {
        assert_eq!(Range::parse("bytes=0-9", 100), Some(Range::Valid(0, 9)));
        assert_eq!(Range::parse("bytes=10-", 100), Some(Range::Valid(10, 99)));
        assert_eq!(Range::parse("bytes=-10", 100), Some(Range::Valid(90, 99)));
        assert_eq!(Range::parse("bytes=95-100", 100), Some(Range::OutOfBounds));
        assert_eq!(Range::parse("items=0-9", 100), None);
    }
}
