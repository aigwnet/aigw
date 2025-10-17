use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;

static HOST_NAME: Lazy<String> = Lazy::new(|| {
    hostname::get()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
        .to_string()
});

/// Returns the system hostname.
///
/// Returns:
/// * `&'static str` - The system's hostname as a string slice
pub fn get_hostname() -> &'static str {
    HOST_NAME.as_str()
}

#[inline]
pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
