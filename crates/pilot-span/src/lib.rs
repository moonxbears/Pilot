//! Milestone 28.2: Compact `FileId(u16)`, 8-Byte C-ABI `ByteSpan` & SIMD/SWAR `LineIndex` (`pilot-span`)

/// Compact 16-bit source file identifier (supports up to 65,535 files per workspace).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub u16);

impl FileId {
    pub const DUMMY: Self = Self(u16::MAX);
}

/// 8-byte C-ABI source byte span `[start, end)` within a file's UTF-8 buffer.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ByteSpan {
    pub start: u32,
    pub end: u32,
}

const _: () = assert!(core::mem::size_of::<ByteSpan>() == 8);

impl ByteSpan {
    pub const EMPTY: Self = Self { start: 0, end: 0 };

    #[inline]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    #[inline]
    pub const fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }

    #[inline]
    pub const fn contains(self, offset: u32) -> bool {
        offset >= self.start && offset < self.end
    }

    #[inline]
    pub fn merge(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    #[inline]
    pub fn slice<'a>(self, source: &'a str) -> &'a str {
        let s = (self.start as usize).min(source.len());
        let e = (self.end as usize).min(source.len()).max(s);
        &source[s..e]
    }
}

/// 1-based `(line, col)` source coordinate paired with raw UTF-8 byte offset.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SourceLoc {
    pub line: u32,
    pub col: u32,
    pub byte_offset: u32,
}

/// Fast newline-offset lookup table computed in a single SIMD/SWAR pass over UTF-8 bytes.
#[derive(Clone, Debug)]
pub struct LineIndex {
    /// Byte offset of the start of each line (`line_starts[0] == 0`).
    line_starts: Vec<u32>,
    source_len: u32,
}

impl LineIndex {
    /// Builds a `LineIndex` over `utf8_bytes` using hardware SSE2 16-byte vector comparison
    /// on `x86_64` and a portable 64-bit SWAR word scanner on all other architectures.
    pub fn new(utf8_bytes: &[u8]) -> Self {
        let mut line_starts = Vec::with_capacity((utf8_bytes.len() / 32).max(8));
        line_starts.push(0);

        scan_newlines_simd_or_swar(utf8_bytes, &mut line_starts);

        Self {
            line_starts,
            source_len: utf8_bytes.len() as u32,
        }
    }

    /// Total number of 1-based lines in the file ($\ge 1$).
    #[inline]
    pub fn line_count(&self) -> u32 {
        self.line_starts.len() as u32
    }

    /// Resolves a UTF-8 `byte_offset` into 1-based `SourceLoc { line, col, byte_offset }` in $O(\log N)$.
    #[inline]
    pub fn lookup(&self, byte_offset: u32) -> SourceLoc {
        let clamped = byte_offset.min(self.source_len);
        let line_idx = self
            .line_starts
            .partition_point(|&start| start <= clamped)
            .saturating_sub(1);
        let line_start = self.line_starts[line_idx];
        SourceLoc {
            line: (line_idx as u32) + 1,
            col: (clamped - line_start) + 1,
            byte_offset: clamped,
        }
    }

    #[inline]
    pub fn line_starts(&self) -> &[u32] {
        &self.line_starts
    }
}

#[inline]
fn scan_newlines_simd_or_swar(bytes: &[u8], out_line_starts: &mut Vec<u32>) {
    let len = bytes.len();
    let mut i = 0usize;

    #[cfg(target_arch = "x86_64")]
    {
        // SSE2 is baseline on all x86_64 targets: scan 16 bytes per cycle!
        use core::arch::x86_64::{_mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_set1_epi8};
        unsafe {
            let nl_vec = _mm_set1_epi8(b'\n' as i8);
            while i + 16 <= len {
                let chunk = _mm_loadu_si128(bytes.as_ptr().add(i) as *const _);
                let cmp = _mm_cmpeq_epi8(chunk, nl_vec);
                let mut mask = _mm_movemask_epi8(cmp) as u32;
                while mask != 0 {
                    let bit = mask.trailing_zeros() as usize;
                    out_line_starts.push((i + bit + 1) as u32);
                    mask &= mask - 1;
                }
                i += 16;
            }
        }
    }

    // Portable 64-bit SWAR fast path (8 bytes per iteration)
    const NL_BROADCAST: u64 = 0x0A0A_0A0A_0A0A_0A0A;
    const LO_BITS: u64 = 0x0101_0101_0101_0101;
    const HI_BITS: u64 = 0x8080_8080_8080_8080;

    while i + 8 <= len {
        let word = u64::from_le_bytes(bytes[i..i + 8].try_into().unwrap());
        let xored = word ^ NL_BROADCAST;
        let has_match = xored.wrapping_sub(LO_BITS) & !xored & HI_BITS;
        if has_match != 0 {
            for k in 0..8 {
                if bytes[i + k] == b'\n' {
                    out_line_starts.push((i + k + 1) as u32);
                }
            }
        }
        i += 8;
    }

    while i < len {
        if bytes[i] == b'\n' {
            out_line_starts.push((i + 1) as u32);
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_index_simd_and_swar_lookup_accuracy() {
        let src = "let a = 1;\nlet b = 2;\n// long line with more than sixteen characters for SIMD\nreturn a + b;\n";
        let idx = LineIndex::new(src.as_bytes());
        assert_eq!(idx.line_count(), 5);

        let loc0 = idx.lookup(0);
        assert_eq!((loc0.line, loc0.col), (1, 1));

        let loc_b = idx.lookup(15); // 'b' on line 2
        assert_eq!((loc_b.line, loc_b.col), (2, 5));

        let ret_pos = src.find("return").unwrap() as u32;
        let loc_ret = idx.lookup(ret_pos);
        assert_eq!((loc_ret.line, loc_ret.col), (4, 1));
    }
}
