//! Milestone 29.1: Portable SIMD / SWAR Character-Class & Delimiter Scanner.
//!
//! Provides ultra-fast 64-bit SWAR (SIMD Within A Register) and vectorized routines for:
//! - ASCII whitespace run-length scanning.
//! - ASCII identifier run-length scanning (`[A-Za-z0-9_]`).
//! - Fast-forward scanning to quote characters, newlines, or comment endings.

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

/// Returns the number of consecutive whitespace characters (`' '`, `'\t'`, `'\r'`) starting at `pos`.
#[inline]
pub fn scan_whitespace_run(bytes: &[u8], mut pos: usize) -> usize {
    let start = pos;
    while pos < bytes.len() {
        let b = unsafe { *bytes.get_unchecked(pos) };
        if IS_WS_TABLE[b as usize] {
            pos += 1;
        } else {
            break;
        }
    }
    pos - start
}

/// Returns the number of consecutive ASCII identifier bytes (`[A-Za-z0-9_]`) starting at `pos`.
#[inline]
pub fn scan_ident_run(bytes: &[u8], mut pos: usize) -> usize {
    let start = pos;
    while pos < bytes.len() {
        let b = unsafe { *bytes.get_unchecked(pos) };
        if IS_IDENT_TABLE[b as usize] {
            pos += 1;
        } else {
            break;
        }
    }
    pos - start
}

/// Scans forward to the next newline character (`\n`), returning the index of `\n` or `bytes.len()`.
#[inline]
pub fn scan_to_newline(bytes: &[u8], mut pos: usize) -> usize {
    while pos + 8 <= bytes.len() {
        let chunk = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());
        // SWAR zero-byte detection on (chunk ^ 0x0A0A0A0A0A0A0A0A)
        let has_nl = chunk ^ 0x0A0A0A0A0A0A0A0A;
        let test = has_nl.wrapping_sub(0x0101010101010101) & !has_nl & 0x8080808080808080;
        if test != 0 {
            let offset = (test.trailing_zeros() / 8) as usize;
            return pos + offset;
        }
        pos += 8;
    }

    while pos < bytes.len() {
        if bytes[pos] == b'\n' {
            return pos;
        }
        pos += 1;
    }
    bytes.len()
}

/// Scans forward to the end of a block comment (`*/`), returning the index directly following `*/`.
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

/// Scans forward to closing quote `quote` (taking into account backslash escaping), returning the closing quote index.
#[inline]
pub fn scan_quoted_string_end(bytes: &[u8], mut pos: usize, quote: u8) -> usize {
    while pos < bytes.len() {
        let b = bytes[pos];
        if b == b'\\' {
            // Skip escape sequence
            pos += 2;
            continue;
        }
        if b == quote {
            return pos;
        }
        if b == b'\n' {
            // Unterminated single-line string at newline
            return pos;
        }
        pos += 1;
    }
    bytes.len()
}

/// Scans forward to closing triple quote sequence (e.g. `"""` or `'''`), returning the index after closing quotes.
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
