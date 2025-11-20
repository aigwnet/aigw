use time::{OffsetDateTime, UtcOffset, format_description::BorrowedFormatItem};

pub fn date_format_local(unix_timestamp: i64, format: &[BorrowedFormatItem<'_>]) -> Option<String> {
    OffsetDateTime::from_unix_timestamp(unix_timestamp)
        .ok()
        .and_then(|utc| {
            let local_offset = UtcOffset::local_offset_at(utc).unwrap_or(UtcOffset::UTC);
            let local_time = utc.to_offset(local_offset);
            local_time.format(format).ok()
        })
}

pub fn date_format_local_nanos(
    unix_timestamp_nanos: i128,
    format: &[BorrowedFormatItem<'_>],
) -> Option<String> {
    OffsetDateTime::from_unix_timestamp_nanos(unix_timestamp_nanos)
        .ok()
        .and_then(|utc| {
            let local_offset = UtcOffset::local_offset_at(utc).unwrap_or(UtcOffset::UTC);
            let local_time = utc.to_offset(local_offset);
            local_time.format(format).ok()
        })
}

pub fn date_format_utc(unix_timestamp: i64, format: &[BorrowedFormatItem<'_>]) -> Option<String> {
    OffsetDateTime::from_unix_timestamp(unix_timestamp)
        .ok()
        .and_then(|utc| utc.format(format).ok())
}

pub fn date_format_utc_nanos(
    unix_timestamp_nanos: i128,
    format: &[BorrowedFormatItem<'_>],
) -> Option<String> {
    OffsetDateTime::from_unix_timestamp_nanos(unix_timestamp_nanos)
        .ok()
        .and_then(|utc| utc.format(format).ok())
}
