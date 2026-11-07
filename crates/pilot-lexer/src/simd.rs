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
        if (b >= b'a' && b <= b'z') || (b >= b'A' && b <= b'Z') || (b >= b'0' && b <= b'9') || b == b'_' {
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
        if b == b'\\' { pos += 2; continue; }
        if b == quote || b == b'\n' { return pos; }
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
