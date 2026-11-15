//! Compact `FileId(u32)` & 8-Byte C-ABI `ByteSpan` (`pilot-span`).
//!
//! Line/column indexing lives in pure Pilot (`stdlib/core/source_lines.pilot`).

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub u32);

impl FileId {
    pub const DUMMY: Self = Self(u32::MAX);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_span_size_merge_and_slice() {
        let a = ByteSpan::new(4, 9);
        let b = ByteSpan::new(10, 15);
        assert_eq!(a.merge(b), ByteSpan::new(4, 15));
        assert!(a.contains(4) && !a.contains(9));
        assert_eq!(a.slice("let hello = 10;"), "hello");
        assert_eq!(FileId::DUMMY, FileId(u32::MAX));
    }
}
