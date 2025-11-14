use pingora_core::tls::ssl_sys::{
    SSL, SSL_get_ex_new_index, SSL_set_ex_data, SSL3_MT_CLIENT_HELLO, SSL3_RT_HANDSHAKE,
};
use tracing::info;

pub static JA4_INDEX: once_cell::sync::Lazy<i32> = once_cell::sync::Lazy::new(|| unsafe {
    SSL_get_ex_new_index(0, std::ptr::null_mut(), std::ptr::null_mut(), None, None)
});

pub unsafe extern "C" fn msg_callback(
    is_write: ::std::os::raw::c_int,
    _version: ::std::os::raw::c_int,
    content_type: ::std::os::raw::c_int,
    buf: *const ::std::os::raw::c_void,
    len: usize,
    ssl: *mut SSL,
    _arg: *mut ::std::os::raw::c_void,
) {
    info!(target: "default", "TLS msg_callback");
    if is_write == 1 && content_type == SSL3_RT_HANDSHAKE {
        let msg: &[u8] = unsafe { std::slice::from_raw_parts(buf as *const u8, len) };

        if len >= 4
            && msg[0] == SSL3_MT_CLIENT_HELLO as u8
            && let Some(data) = ja4(msg)
        {
            info!("ja4: {}", &data);
            let boxed = Box::new(data);
            let _ = unsafe { SSL_set_ex_data(ssl, *JA4_INDEX, Box::into_raw(boxed) as *mut _) };
        }
    }
}

/// Handshake Layer:
/// - handshake_type (1 byte): 0x01 = ClientHello
/// - length (3 bytes): big-endian
///  - legacy_version (2 bytes)
/// - random (32 bytes)
/// - session_id_len (1 byte)
/// - session_id (var)
/// - cipher_suites_len (2 bytes)
/// - cipher_suites (var)
/// - compression_methods_len (1 byte)
/// - compression_methods (var)
/// - extensions_len (2 bytes)
/// - extensions (var)
fn ja4(msg: &[u8]) -> Option<String> {
    let len = msg.len();
    // Parse handshake length (3 bytes, big-endian)
    let hs_len = ((msg[1] as usize) << 16) | ((msg[2] as usize) << 8) | (msg[3] as usize);
    if len != (hs_len + 4) {
        return None;
    }
    let body = &msg[4..4 + hs_len];
    if body.len() < 2 + 32 + 1 {
        return None;
    }

    // legacy_version
    let legacy_version = u16::from_be_bytes([body[0], body[1]]);
    let proto = match legacy_version {
        0x0304 => "13",
        0x0303 => "12",
        0x0302 => "11",
        0x0301 => "10",
        _ => "00",
    };
    let mut offset = 2 + 32; // skip version + random
    let session_id_len = body[offset] as usize;
    offset += 1;
    if offset + session_id_len > body.len() {
        return None;
    }
    offset += session_id_len;

    // Cipher suites
    if offset + 2 > body.len() {
        return None;
    }
    let cipher_suites_len = u16::from_be_bytes([body[offset], body[offset + 1]]) as usize;
    offset += 2;
    if cipher_suites_len % 2 != 0 || offset + cipher_suites_len > body.len() {
        return None;
    }

    let mut ciphers: Vec<u16> = Vec::new();
    let mut i = 0;
    while i < cipher_suites_len {
        let cipher = u16::from_be_bytes([body[offset + i], body[offset + i + 1]]);
        if !is_grease(cipher) {
            ciphers.push(cipher);
        }
        i += 2;
    }
    offset += cipher_suites_len;

    // Compression methods
    if offset >= body.len() {
        return None;
    }
    let compression_len = body[offset] as usize;
    offset += 1 + compression_len;
    if offset >= body.len() {
        return None;
    }

    // Extensions
    if offset + 2 > body.len() {
        return None;
    }
    let extensions_len = u16::from_be_bytes([body[offset], body[offset + 1]]) as usize;
    offset += 2;
    if offset + extensions_len > body.len() {
        return None;
    }

    let mut extensions: Vec<u16> = Vec::new();
    let mut has_sni = false;
    let mut alpn = String::from("00");
    let mut ext_cursor = offset;
    let ext_end = offset + extensions_len;

    while ext_cursor < ext_end {
        if ext_cursor + 4 > ext_end {
            break;
        }
        let ext_type = u16::from_be_bytes([body[ext_cursor], body[ext_cursor + 1]]);
        let ext_data_len =
            u16::from_be_bytes([body[ext_cursor + 2], body[ext_cursor + 3]]) as usize;
        ext_cursor += 4;
        if ext_cursor + ext_data_len > ext_end {
            break;
        }

        // Only collect non-GREASE, non-padding extensions for extension list
        if ext_type != 0x0015 && !is_grease(ext_type) {
            extensions.push(ext_type);
        }

        // Check for SNI (type 0)
        if ext_type == 0x0000 {
            has_sni = true;
        }

        // Parse ALPN (type 16)
        if ext_type == 0x0010 && alpn == "00" {
            let alpn_data = &body[ext_cursor..ext_cursor + ext_data_len];
            if alpn_data.len() >= 2 {
                let list_len = u16::from_be_bytes([alpn_data[0], alpn_data[1]]) as usize;
                if list_len == alpn_data.len() - 2 && list_len > 0 {
                    let first_proto_len = alpn_data[2] as usize;
                    if first_proto_len > 0 && 3 + first_proto_len <= alpn_data.len() {
                        let proto = &alpn_data[3..3 + first_proto_len];
                        if let Ok(s) = std::str::from_utf8(proto) {
                            alpn = s.to_lowercase();
                        }
                    }
                }
            }
        }

        ext_cursor += ext_data_len;
    }

    // Truncate ALPN to 2 chars, or use "00"
    let alpn_label = if alpn == "00" {
        "00".to_string()
    } else {
        if alpn.len() >= 2 {
            alpn[..2].to_string()
        } else {
            format!("{:<2}", alpn).replace(' ', "0")
        }
    };

    // Build extension and cipher strings for hashing
    ciphers.sort_unstable();
    extensions.sort_unstable();

    let cipher_str: String = ciphers
        .iter()
        .map(|c| format!("{:04x}", c))
        .collect::<Vec<_>>()
        .join(",");

    let ext_str: String = extensions
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join(",");

    // JA4 hash input: cipher_str, ext_str, alpn_value (original, not truncated)
    let alpn_for_hash = if alpn == "00" { "" } else { &alpn };
    let hash_input = format!("{},{},{}", cipher_str, ext_str, alpn_for_hash);

    let digest = md5::compute(hash_input.as_bytes());
    let hash_hex: String = digest.iter().map(|b| format!("{:x}", b)).collect();
    let hash_part = &hash_hex[..12];

    // Determine SNI char
    let sni_char = if has_sni { 'd' } else { 'i' };

    // Final JA4: d13050a_h2_abc123def456
    let cipher_count = std::cmp::min(ciphers.len(), 99);
    let ext_count = std::cmp::min(extensions.len(), 99);

    let ja4: String = format!(
        "{}{}{:02}{:02}{}_{:12}",
        sni_char,
        &proto[0..1], // '1' from "13"
        cipher_count,
        ext_count,
        alpn_label,
        hash_part
    );
    Some(ja4)
}

#[inline]
fn is_grease(val: u16) -> bool {
    match val {
        0x0a0a | 0x1a1a | 0x2a2a | 0x3a3a | 0x4a4a | 0x5a5a | 0x6a6a | 0x7a7a | 0x8a8a | 0x9a9a
        | 0xaaaa | 0xbaba | 0xcaca | 0xdada | 0xeaea | 0xfafa => true,
        _ => false,
    }
}
