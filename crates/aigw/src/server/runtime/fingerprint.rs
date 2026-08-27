use core::slice;
use pingora_core::tls::ssl_sys::{
    CRYPTO_EX_DATA, OPENSSL_free, SSL, SSL_client_hello_get0_ciphers, SSL_client_hello_get0_ext,
    SSL_client_hello_get0_legacy_version, SSL_client_hello_get1_extensions_present,
    SSL_get_ex_data, SSL_get_ex_new_index, SSL_set_ex_data,
};
use sha::{
    sha256,
    utils::{Digest, DigestExt},
};

/// Free the JA4 box attached to an SSL object. Registered as the ex_data free
/// callback so failed handshakes (which never reach handshake_complete_callback)
/// don't leak it.
unsafe extern "C" fn ja4_ex_data_free(
    _parent: *mut std::ffi::c_void,
    ptr: *mut std::ffi::c_void,
    _ad: *mut CRYPTO_EX_DATA,
    _idx: std::os::raw::c_int,
    _argl: std::os::raw::c_long,
    _argp: *mut std::ffi::c_void,
) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr as *mut (String, String))) };
    }
}

pub static JA4_INDEX: once_cell::sync::Lazy<i32> = once_cell::sync::Lazy::new(|| unsafe {
    SSL_get_ex_new_index(
        0,
        std::ptr::null_mut(),
        None,
        None,
        Some(ja4_ex_data_free),
    )
});

pub unsafe extern "C" fn client_hello_cb(
    ssl: *mut SSL,
    _al: *mut std::os::raw::c_int,
    _arg: *mut ::std::os::raw::c_void,
) -> std::os::raw::c_int {
    unsafe {
        // The callback can run twice (HelloRetryRequest); free the previous box
        // before overwriting the pointer.
        let old = SSL_get_ex_data(ssl, *JA4_INDEX);
        if !old.is_null() {
            drop(Box::from_raw(old as *mut (String, String)));
        }
        let (ja4_hash, ja4_origin) = ja4(ssl);
        let boxed = Box::new((ja4_hash, ja4_origin));
        SSL_set_ex_data(ssl, *JA4_INDEX, Box::into_raw(boxed) as *mut _);
    }

    1
}

///
/// https://github.com/FoxIO-LLC/ja4/blob/main/technical_details/JA4.md
///
/// (QUIC=”q”, DTLS="d", or TLS over TCP=”t”)
/// (2 character TLS version)
/// (SNI=”d” or no SNI=”i”)
/// (2 character count of ciphers)
/// (2 character count of extensions)
/// (first and last characters of first ALPN extension value)
/// _
/// (sha256 hash of the list of cipher hex codes sorted in hex order, truncated to 12 characters)
/// _
/// (sha256 hash of (the list of extension hex codes sorted in hex order)_(the list of signature algorithms), truncated to 12 characters)
///
/// The end result is a fingerprint that looks like:
/// t13d1516h2_8daaf6152771_b186095e22b6
unsafe fn ja4(ssl: *mut SSL) -> (String, String) {
    unsafe {
        let mut fingerprint = String::from("t");

        let version = get_version(ssl);

        fingerprint += match version {
            0x0304 => "13",
            0x0303 => "12",
            0x0302 => "11",
            0x0301 => "10",
            0x0300 => "s2",
            0x0002 => "s1",
            0xfeff => "d1",
            0xfefd => "d2",
            0xfefc => "d3",
            _ => "00",
        };
        // has_sni ? 'd' : 'i';
        let has_sni = has_sni(ssl);
        fingerprint += if has_sni { "d" } else { "i" };

        let (mut ciphers_len, ciphers, ciphers_hash) = get_ciphers(ssl);
        ciphers_len = 99.min(ciphers_len);
        let (extensions_len, extensions, extensions_hash) = get_extensions_hash(ssl);

        let alpn_list = get_alpn(ssl);
        // A “00” here denotes the lack of ALPN.
        let alpn = alpn_list.first().map_or("00", |s| s);

        fingerprint += &format!("{:02}", ciphers_len);
        fingerprint += &format!("{:02}", extensions_len);

        let fingerprint_origin = fingerprint.clone() + alpn + "_" + &ciphers + "_" + &extensions;
        fingerprint = fingerprint + alpn + "_" + &ciphers_hash + "_" + &extensions_hash;

        //info!(target: "test", "Ciphers: {}, Ciphers Hash: {}", &ciphers, &ciphers_hash);
        //info!(target: "test", "Extensions: {}, Extensions Hash:  {}", &extensions, extensions_hash);

        (fingerprint, fingerprint_origin)
    }
}

/// In TLS 1.3, for compatibility with middleboxes, the legacy_version field in the ClientHello
/// is fixed to 0x0303 (which corresponds to TLS 1.2), even if the client supports TLS 1.3.
/// The actual negotiated version is communicated via the supported_versions extension.
/// Thus, legacy_version no longer reflects the true protocol version the client intends to use.
unsafe fn get_version(ssl: *mut SSL) -> u16 {
    unsafe {
        let legacy_version = SSL_client_hello_get0_legacy_version(ssl) as u16;
        // Check for the supported_versions extension (0x002b) in the ClientHello
        let mut supported_versions_ext = std::ptr::null();
        let mut supported_versions_ext_len = 0;
        let mut highest_supported_tls_client_version = 0;

        if SSL_client_hello_get0_ext(
            ssl,
            0x002b,
            &mut supported_versions_ext,
            &mut supported_versions_ext_len,
        ) == 1
        {
            if supported_versions_ext.is_null() || supported_versions_ext_len < 3 {
                return legacy_version;
            }
            // Example: [10, 106, 106, 3, 4, 3, 3, 3, 2, 3, 1]
            let data = slice::from_raw_parts(supported_versions_ext, supported_versions_ext_len);
            // info!(target: "test", "Read supported_versions: {:?}", data);
            let list_len = data[0] as usize;
            if (list_len + 1) as usize > supported_versions_ext_len {
                return legacy_version;
            }
            let supported_versions = &data[1..];

            let mut i = 0;
            while i + 1 < list_len && i + 1 < supported_versions.len() {
                let version =
                    u16::from_be_bytes([supported_versions[i], supported_versions[i + 1]]);
                if !is_grease(version) && version > highest_supported_tls_client_version {
                    highest_supported_tls_client_version = version;
                }
                i += 2;
            }
        }

        if highest_supported_tls_client_version > 0 {
            highest_supported_tls_client_version
        } else {
            legacy_version
        }
    }
}

unsafe fn has_sni(ssl: *mut SSL) -> bool {
    // Determine if SNI is present or not
    unsafe {
        let mut sni = std::ptr::null();
        let mut sni_len = 0;

        if SSL_client_hello_get0_ext(ssl, 0x0000, &mut sni, &mut sni_len) == 1 {
            if sni.is_null() || sni_len < 5 {
                return false;
            }

            let data = slice::from_raw_parts(sni, sni_len);

            let list_len = u16::from_be_bytes([data[0], data[1]]) as usize;
            if list_len == 0 || list_len + 2 != sni_len {
                return false;
            }
            let name_type = data[2];
            if name_type != 0 {
                return false;
            }
            let name_len = u16::from_be_bytes([data[3], data[4]]) as usize;
            if name_len > 0 {
                let _name = &data[5..];
                return true;
            }
        }
        false
    }
}

///
/// The first and last alphanumeric characters of the ALPN (Application-Layer Protocol Negotiation) first value.
/// List of possible ALPN Values (scroll down): https://www.iana.org/assignments/tls-extensiontype-values/tls-extensiontype-values.xhtml
///
/// In the above example, the first ALPN value is h2 so the first and last characters to use in the fingerprint are “h2”.
/// If the first ALPN listed was http/1.1 then the first and last characters to use in the fingerprint would be “h1”.
///
/// In Wireshark this field is located under tls.handshake.extensions_alpn_str
///
/// If there is no ALPN extension, no ALPN values, or the first ALPN value is empty, then we print "00" as the value in the fingerprint.
/// If the first ALPN value is only a single character, then that character is treated as both the first and last character.
///
/// If the first or last byte of the first ALPN is non-alphanumeric (meaning not 0x30-0x39, 0x41-0x5A, or 0x61-0x7A),
/// then we print the first and last characters of the hex representation of the first ALPN instead. For example:
///
/// 0xAB would be printed as "ab"
/// 0xAB 0xCD would be printed as "ad"
/// 0x30 0xAB would be printed as "3b"
/// 0x30 0x31 0xAB 0xCD would be printed as "3d"
/// 0x30 0xAB 0xCD 0x31 would be printed as "01"
///
///
///
unsafe fn get_alpn(ssl: *mut SSL) -> Vec<String> {
    let mut protocols = Vec::new();
    unsafe {
        let mut alpn_data = std::ptr::null();
        let mut alpn_len = 0;

        if SSL_client_hello_get0_ext(ssl, 0x0010, &mut alpn_data, &mut alpn_len) == 1
            && !alpn_data.is_null()
            && alpn_len >= 2
        {
            // Example: [0, 12, 2, 104, 50, 8, 104, 116, 116, 112, 47, 49, 46, 49]
            let data = slice::from_raw_parts(alpn_data, alpn_len);
            let list_len = u16::from_be_bytes([data[0], data[1]]) as usize;
            if list_len == 0 || list_len + 2 != alpn_len {
                return protocols;
            }
            let payload = &data[2..];

            let mut offset = 0;
            while offset < payload.len() {
                let name_len = payload[offset] as usize;
                offset += 1;

                if offset + name_len > payload.len() {
                    break;
                }
                let name = &payload[offset..offset + name_len];

                // Check if first and last bytes are alphanumeric
                let first_byte = name[0];
                let last_byte = name[name.len() - 1];

                if is_alphanumeric(first_byte) && is_alphanumeric(last_byte) {
                    // Alphanumeric path
                    if name.len() == 1 {
                        // Single char: repeat it
                        let c = first_byte as char;
                        protocols.push(format!("{}{}", c, c));
                    } else {
                        let first_char = first_byte as char;
                        let last_char = last_byte as char;
                        protocols.push(format!("{}{}", first_char, last_char));
                    }
                } else {
                    // Non-alphanumeric: use hex representation of the raw bytes
                    let hex_str: String = name.iter().map(|b| format!("{:02x}", b)).collect();
                    // Take first and last character of the hex string
                    let first_hex_char = hex_str.chars().next().unwrap_or('0');
                    let last_hex_char = hex_str.chars().last().unwrap_or('0');
                    protocols.push(format!("{}{}", first_hex_char, last_hex_char));
                }

                offset += name_len;
            }

            // info!(target: "test", "ALPN: {:?}", protocols);
        }
    }
    protocols
}

///
/// Extract and format cipher suites from ClientHello for JA4.
///
/// Number of Ciphers:
///
/// 2 character number of cipher suites, so if there’s 6 cipher suites in the hello packet,
/// then the value should be “06”. If there’s > 99, which there should never be, then output “99”.
/// Remember, ignore GREASE values. They don’t count. Do, however, count other non-cipher values
/// such as SCSV (0x00FF, 0x5600) and Experimental/Reserved values (0xFE00-0xFEFF).
///
/// Cipher hash:
/// A 12 character truncated sha256 hash of the list of ciphers sorted in hex order, first 12 characters.
/// The list is created using the 4 character hex values of the ciphers, lower case, comma delimited,
/// ignoring GREASE yet still including other non-cipher values such as SCSV (0x00FF, 0x5600) and Experimental/Reserved values (0xFE00-0xFEFF).
/// Example:
///
/// 1301,1302,1303,c02b,c02f,c02c,c030,cca9,cca8,c013,c014,009c,009d,002f,0035
/// Is sorted to:
///
/// 002f,0035,009c,009d,1301,1302,1303,c013,c014,c02b,c02c,c02f,c030,cca8,cca9 = 8daaf6152771
/// If there are no ciphers in the sorted cipher list, then the value of JA4_b is set to 000000000000
/// We do this rather than running a sha256 hash of nothing as this makes it clear to the user when a field has no values.
///
///
unsafe fn get_ciphers(ssl: *mut SSL) -> (u8, String, String) {
    unsafe {
        let mut ptr = std::ptr::null();
        let len = SSL_client_hello_get0_ciphers(ssl, &mut ptr);
        if len.is_multiple_of(2) {
            let data = slice::from_raw_parts(ptr, len);

            let mut ciphers = data
                .as_chunks::<2>().0.iter()
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .filter(|i| !is_grease(*i))
                .map(|c| format!("{:04x}", c)) // lowercase hex, 4 digits
                .collect::<Vec<_>>();

            ciphers.sort_unstable();
            let ciphers_len = ciphers.len();

            let ciphers = ciphers.join(",");
            let ciphers_hash = hash12(&ciphers);

            return (ciphers_len as u8, ciphers, ciphers_hash);
        }
        (0, "".to_string(), "000000000000".to_string())
    }
}

///
///
/// Number of Extensions:
/// Same as counting ciphers. Ignore GREASE. Include SNI and ALPN.
///
/// Extension hash:
/// A 12 character truncated sha256 hash of the list of extensions, sorted by hex value,
/// followed by the list of signature algorithms, in the order that they appear (not sorted).
///
/// The extension list is created using the 4 character hex values of the extensions, lower case,
/// comma delimited, sorted (not in the order they appear). Ignore the SNI extension (0000) and the ALPN extension (0010)
/// as we’ve already captured them in the a section of the fingerprint.
/// These values are omitted so that the same application would have the same c section of the fingerprint
/// regardless of if it were going to a domain, IP, or changing ALPNs.
///
/// For example:
///
/// 001b,0000,0033,0010,4469,0017,002d,000d,0005,0023,0012,002b,ff01,000b,000a,0015
/// Is sorted to:
///
/// 0005,000a,000b,000d,0012,0015,0017,001b,0023,002b,002d,0033,4469,ff01
/// (notice 0000 and 0010 is removed)
///
/// The signature algorithm hex values are then added to the end of the list in the order that they appear
/// (not sorted) with an underscore delimiting the two lists.
/// For example the signature algorithms:
///
/// 0403,0804,0401,0503,0805,0501,0806,0601
/// Are added to the end of the previous string to create:
///
/// 0005,000a,000b,000d,0012,0015,0017,001b,0023,002b,002d,0033,4469,ff01_0403,0804,0401,0503,0805,0501,0806,0601
/// Hashed to:
///
/// e5627efa2ab19723084c1033a96c694a45826ab5a460d2d3fd5ffcfe97161c95
/// Truncated to first 12 characters:
///
/// e5627efa2ab1
/// If there are no signature algorithms in the hello packet, then the string ends without an underscore and is hashed.
/// For example:
///
/// 0005,000a,000b,000d,0012,0015,0017,001b,0023,002b,002d,0033,4469,ff01 = 6d807ffa2a79
/// If there are no extensions in the sorted extensions list, then the value of JA4_c is set to 000000000000
/// We do this rather than running a sha256 hash of nothing as this makes it clear to the user when a field has no values.
///
unsafe fn get_extensions_hash(ssl: *mut SSL) -> (u8, String, String) {
    let (mut extensions_len, extensions) = unsafe {
        let mut ext_ptr = std::ptr::null_mut();
        let mut ext_len = 0;
        if SSL_client_hello_get1_extensions_present(ssl, &mut ext_ptr, &mut ext_len) == 1 {
            if ext_ptr.is_null() {
                (0, "".to_string())
            } else {
                let exts: Vec<u16> = slice::from_raw_parts(ext_ptr, ext_len)
                    .iter()
                    .map(|i| *i as u16)
                    .filter(|i| !is_grease(*i))
                    .collect();

                let len = exts.len();

                let mut exts_ignored = exts
                    .iter()
                    .copied()
                    .filter(|i| !is_ignore(*i))
                    .map(|c| format!("{:04x}", c))
                    .collect::<Vec<_>>();
                exts_ignored.sort_unstable();

                OPENSSL_free(ext_ptr as *mut _);

                (len as u8, exts_ignored.join(","))
            }
        } else {
            (0, "".to_string())
        }
    };
    extensions_len = 99.min(extensions_len);

    // SignatureScheme signature_algorithms<2..2^16-2>;
    let signature_algorithms = unsafe {
        let mut sa_data = std::ptr::null();
        let mut sa_len = 0;

        if SSL_client_hello_get0_ext(ssl, 0x000d, &mut sa_data, &mut sa_len) == 1
            && !sa_data.is_null()
            && sa_len >= 2
        {
            let data = slice::from_raw_parts(sa_data, sa_len);

            let list_len = u16::from_be_bytes([data[0], data[1]]) as usize;
            if list_len == 0 || !list_len.is_multiple_of(2) || list_len + 2 != sa_len {
                "".to_string()
            } else {
                let data = &data[2..];
                data.as_chunks::<2>().0.iter()
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .map(|c| format!("{:04x}", c)) // lowercase hex, 4 digits
                    .collect::<Vec<_>>()
                    .join(",")
            }
        } else {
            "".to_string()
        }
    };

    let extensions = if extensions.is_empty() || signature_algorithms.is_empty() {
        extensions
    } else {
        extensions + "_" + &signature_algorithms
    };

    let extensions_hash = hash12(&extensions);
    (extensions_len, extensions, extensions_hash)
}

#[inline]
fn is_grease(val: u16) -> bool {
    matches!(
        val,
        0x0a0a
            | 0x1a1a
            | 0x2a2a
            | 0x3a3a
            | 0x4a4a
            | 0x5a5a
            | 0x6a6a
            | 0x7a7a
            | 0x8a8a
            | 0x9a9a
            | 0xaaaa
            | 0xbaba
            | 0xcaca
            | 0xdada
            | 0xeaea
            | 0xfafa
    )
}

fn is_ignore(val: u16) -> bool {
    match val {
        // ALPN IGNORE
        // SNI IGNORE
        0x0010 | 0x0000 => true,
        _ => false,
    }
}

fn hash12(s: impl AsRef<str>) -> String {
    let s = s.as_ref();
    if s.is_empty() {
        "000000000000".to_owned()
    } else {
        let mut sha = sha256::Sha256::default();
        sha.digest(s.as_bytes());
        let sha256 = hex::encode(sha.to_bytes());
        sha256[..12].into()
    }
}

fn is_alphanumeric(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z')
}
