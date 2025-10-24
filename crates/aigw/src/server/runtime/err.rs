use bytes::Bytes;
use http::header;
use pingora_http::ResponseHeader;

use crate::SERVER;

const BAD_REQUEST: &[u8] = b"Bad Request";
const FORBIDDEN: &[u8] = b"Forbidden";
const INTERNAL_SERVER_ERROR: &[u8] = b"internal server error";
const BAD_GATEWAY: &[u8] = b"Bad Gateway";
const SERVICE_UNAVAILABLE: &[u8] = b"Service Unavailable";

pub fn generate_error(code: u16) -> (ResponseHeader, Bytes) {
    let body = match code {
        400 => Bytes::copy_from_slice(BAD_REQUEST),
        403 => Bytes::copy_from_slice(FORBIDDEN),
        500 => Bytes::copy_from_slice(INTERNAL_SERVER_ERROR),
        502 => Bytes::copy_from_slice(BAD_GATEWAY),
        503 => Bytes::copy_from_slice(SERVICE_UNAVAILABLE),
        _ => Bytes::default(),
    };
    let length = body.len();
    let mut resp = ResponseHeader::build(code, Some(3)).unwrap();
    resp.insert_header(header::SERVER, SERVER).unwrap();
    resp.insert_header(header::CONTENT_LENGTH, &length.to_string())
        .unwrap();
    resp.insert_header(header::CACHE_CONTROL, "private, no-store")
        .unwrap();
    (resp, body)
}
