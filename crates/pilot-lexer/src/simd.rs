//! Milestone 29.1 & 52: Portable SIMD / SWAR Character-Class & Delimiter Scanner.

const IS_WS_TABLE: [bool; 256] = {
    let mut table = [false; 256];
    table[b' ' as usize] = true;
    table[b'\t' as usize] = true;
    table[b'\r' as usize] = true;
    table
};

const IS_IDENT_TABLE: [bool; 256] = {
    let mut table = [false; 256];
    let mut i = 0;
    while i < 256 {
        let b = i as u8;
        if (b >= b'a' && b <= b'z')
            || (b >= b'A' && b <= b'Z')
            || (b >= b'0' && b <= b'9')
            || b == b'_'
        {
            table[i] = true;
        }
        i += 1;
    }
    table
};

#[inline]
pub fn scan_whitespace_run(bytes: &[u8], mut pos: usize) -> usize {
    let start = pos;
    while pos < bytes.len() && IS_WS_TABLE[bytes[pos] as usize] {
        pos += 1;
    }
    pos - start
}

#[inline]
pub fn scan_ident_run(bytes: &[u8], mut pos: usize) -> usize {
    let start = pos;
    while pos < bytes.len() && IS_IDENT_TABLE[bytes[pos] as usize] {
        pos += 1;
    }
    pos - start
}

#[inline]
pub fn scan_to_newline(bytes: &[u8], mut pos: usize) -> usize {
    while pos + 8 <= bytes.len() {
        let chunk = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());
        let has_nl = chunk ^ 0x0A0A0A0A0A0A0A0A;
        let test = has_nl.wrapping_sub(0x0101010101010101) & !has_nl & 0x8080808080808080;
        if test != 0 {
            return pos + (test.trailing_zeros() / 8) as usize;
        }
        pos += 8;
    }
    while pos < bytes.len() && bytes[pos] != b'\n' {
        pos += 1;
    }
    pos
}

#[inline]
pub fn scan_to_block_comment_end(bytes: &[u8], mut pos: usize) -> usize {
    while pos + 1 < bytes.len() {
        if bytes[pos] == b'*' && bytes[pos + 1] == b'/' {
            return pos + 2;
        }
        pos += 1;
    }
    bytes.len()
}

#[inline]
pub fn scan_quoted_string_end(bytes: &[u8], mut pos: usize, quote: u8) -> usize {
    while pos < bytes.len() {
        let b = bytes[pos];
        if b == b'\\' {
            pos += 2;
            continue;
        }
        if b == quote || b == b'\n' {
            return pos;
        }
        pos += 1;
    }
    bytes.len()
}

#[inline]
pub fn scan_triple_string_end(bytes: &[u8], mut pos: usize, quote: u8) -> usize {
    while pos + 2 < bytes.len() {
        if bytes[pos] == quote && bytes[pos + 1] == quote && bytes[pos + 2] == quote {
            return pos + 3;
        }
        pos += 1;
    }
    bytes.len()
}

fn string_prefix_flags(bytes: &[u8], quote_pos: usize) -> (bool, bool) {
    if quote_pos == 0 {
        return (false, false);
    }
    let b1 = bytes[quote_pos - 1].to_ascii_lowercase();
    let bound1 = quote_pos == 1
        || !(bytes[quote_pos - 2].is_ascii_alphanumeric() || bytes[quote_pos - 2] == b'_');
    if bound1 {
        return (b1 == b'f', b1 == b'r');
    }
    let b2 = bytes[quote_pos - 2].to_ascii_lowercase();
    let bound2 = quote_pos == 2
        || !(bytes[quote_pos - 3].is_ascii_alphanumeric() || bytes[quote_pos - 3] == b'_');
    if bound2 && ((b2 == b'r' && b1 == b'f') || (b2 == b'f' && b1 == b'r')) {
        return (true, true);
    }
    (false, false)
}

fn scan_interp_hole_end(bytes: &[u8], mut pos: usize, allow_newline: bool) -> usize {
    let mut depth = 1usize;
    while pos < bytes.len() {
        let b = bytes[pos];
        if !allow_newline && b == b'\n' {
            return pos;
        }
        if b == b'\\' {
            pos = (pos + 2).min(bytes.len());
            continue;
        }
        if b == b'{' {
            depth += 1;
            pos += 1;
        } else if b == b'}' {
            depth -= 1;
            pos += 1;
            if depth == 0 {
                return pos;
            }
        } else if b == b'"' || b == b'\'' {
            let triple = pos + 2 < bytes.len() && bytes[pos + 1] == b && bytes[pos + 2] == b;
            let (is_f, is_raw) = string_prefix_flags(bytes, pos);
            if is_f {
                pos = scan_interpolated_string_end(
                    bytes,
                    pos + if triple { 3 } else { 1 },
                    b,
                    is_raw,
                    triple,
                );
                if !triple && pos < bytes.len() && bytes[pos] == b {
                    pos += 1;
                }
            } else if triple {
                pos = scan_triple_string_end(bytes, pos + 3, b);
            } else {
                let end = scan_quoted_string_end(bytes, pos + 1, b);
                pos = if end < bytes.len() && bytes[end] == b {
                    end + 1
                } else {
                    end
                };
            }
        } else if b == b'/' && bytes.get(pos + 1) == Some(&b'/') {
            pos = scan_to_newline(bytes, pos + 2);
        } else if b == b'/' && bytes.get(pos + 1) == Some(&b'*') {
            pos = scan_to_block_comment_end(bytes, pos + 2);
        } else {
            pos += 1;
        }
    }
    bytes.len()
}

/// Scans an interpolated string literal from just after its opening quote(s)
/// to its closing quote (or end of `"""` when `is_triple` is true), tracking
/// `{ ... }` interpolation holes and any nested strings inside them.
pub fn scan_interpolated_string_end(
    bytes: &[u8],
    mut pos: usize,
    quote: u8,
    is_raw: bool,
    is_triple: bool,
) -> usize {
    while pos < bytes.len() {
        let b = bytes[pos];
        if !is_raw && b == b'\\' {
            pos = (pos + 2).min(bytes.len());
            continue;
        }
        if is_triple {
            if pos + 2 < bytes.len()
                && b == quote
                && bytes[pos + 1] == quote
                && bytes[pos + 2] == quote
            {
                return pos + 3;
            }
        } else if b == quote || b == b'\n' {
            return pos;
        }
        if b == b'{' {
            if bytes.get(pos + 1) == Some(&b'{') {
                pos += 2;
                continue;
            }
            pos = scan_interp_hole_end(bytes, pos + 1, is_triple);
            continue;
        }
        if b == b'}' && bytes.get(pos + 1) == Some(&b'}') {
            pos += 2;
            continue;
        }
        pos += 1;
    }
    bytes.len()
}
