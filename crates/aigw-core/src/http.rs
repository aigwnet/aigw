use std::str::FromStr;

use http::{HeaderName, HeaderValue};

pub type HttpHeader = (HeaderName, HeaderValue);

/// Converts a slice of strings into HTTP headers.
/// Each string should be in "name: value" format.
///
/// # Arguments
/// * `header_values` - Slice of strings representing headers
///
/// # Returns
/// * `Result<Vec<HttpHeader>>` - Vector of parsed HTTP headers
pub fn convert_headers(header_values: &[String]) -> anyhow::Result<Vec<HttpHeader>> {
    let mut arr = vec![];
    for item in header_values {
        if let Some(item) = convert_header(item)? {
            arr.push(item);
        }
    }
    Ok(arr)
}

/// Converts a string in "name: value" format into an HTTP header tuple.
/// Returns None if the input string doesn't contain a colon separator.
///
/// # Arguments
/// * `value` - A string in the format "header_name: header_value"
///
/// # Returns
/// * `Result<Option<HttpHeader>>` - The parsed header tuple or None if invalid format
pub fn convert_header(value: &str) -> anyhow::Result<Option<HttpHeader>> {
    value
        .split_once(':')
        .map(|(k, v)| {
            let name = HeaderName::from_str(k.trim())?;
            let value = HeaderValue::from_str(v.trim())?;
            Ok(Some((name, value)))
        })
        .unwrap_or(Ok(None))
}

pub fn convert_headers_to_string(headers: &Vec<HttpHeader>) -> anyhow::Result<Vec<String>> {
    let mut r = vec![];
    for (k, v) in headers {
        let s = "".to_owned() + k.as_str() + ":" + v.to_str()?;
        r.push(s);
    }
    Ok(r)
}
