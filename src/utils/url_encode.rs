
pub fn url_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            // Try to decode the next two characters as hex digits.
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(h1), Some(h2)) = (h1, h2) {
                let hex = format!("{}{}", h1, h2);
                match u8::from_str_radix(&hex, 16) {
                    Ok(byte) => {
                        // Convert the decoded byte to char.
                        result.push(byte as char);
                    },
                    Err(_) => {
                        // If decoding fails, push the literal characters.
                        result.push(ch);
                        result.push(h1);
                        result.push(h2);
                    }
                }
            } else {
                // If there aren't enough characters after '%', just append '%'
                result.push(ch);
                if let Some(h1) = h1 { result.push(h1); }
                if let Some(h2) = h2 { result.push(h2); }
            }
        } else {
            // For now, we treat all other characters as literal.
            result.push(ch);
        }
    }
    
    result
}


/// Percent-encodes arbitrary bytes using a minimal set of "unreserved" characters.
/// In many BitTorrent implementations, the `info_hash` and `peer_id` are
/// treated as raw bytes that must be percent-encoded (i.e., not assumed to be UTF-8).
///
/// This will produce uppercase hex (e.g. "%3A" not "%3a").
pub fn url_encode_bytes(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 3);
    for &b in bytes {
        if is_unreserved(b) {
            // "Safe" character: add as-is
            encoded.push(b as char);
        } else {
            // Percent-encode everything else as %XX
            encoded.push_str(&format!("%{:02X}", b));
        }
    }
    encoded
}

/// Defines which characters should remain unencoded. For standard "unreserved"
/// = ALPHA / DIGIT / "-" / "." / "_" / "~"
/// https://datatracker.ietf.org/doc/html/rfc3986
fn is_unreserved(byte: u8) -> bool {
    	   (byte >= b'a' && byte <= b'z')
        || (byte >= b'A' && byte <= b'Z')
        || (byte >= b'0' && byte <= b'9')
        || byte == b'.'
        || byte == b'-'
        || byte == b'_'
        || byte == b'~'
}
