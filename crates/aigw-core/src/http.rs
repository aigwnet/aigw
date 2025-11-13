use std::{collections::HashMap, str::FromStr};

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
pub fn convert_headers(
    header_values: &Vec<HashMap<String, String>>,
) -> anyhow::Result<Vec<HttpHeader>> {
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
pub fn convert_header(value: &HashMap<String, String>) -> anyhow::Result<Option<HttpHeader>> {
    let name = value.get("name");
    let value = value.get("value");

    if let Some(k) = name
        && let Some(v) = value
    {
        if k.trim().is_empty() || v.trim().is_empty() {
            return Ok(None);
        }
        return Ok(Some((
            HeaderName::from_str(k.trim())?,
            HeaderValue::from_str(v.trim())?,
        )));
    }
    Ok(None)
}

pub fn convert_headers_to_string(
    headers: &Vec<HttpHeader>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    let mut r = vec![];
    for (k, v) in headers {
        let mut map = HashMap::new();
        map.insert("name".to_string(), k.as_str().to_string());
        map.insert("value".to_string(), v.to_str()?.to_string());
        r.push(map);
    }
    Ok(r)
}
