//! Milestone 28.2: Zero-Copy Arena String & `:ns/kw` Keyword Interner (`pilot-intern`)
//!
//! Stores all interned UTF-8 strings in a single contiguous byte buffer (`Vec<u8>`) indexed by
//! `ByteSpan`, using an open-addressed hash table so repeated identifier/keyword lookups allocate
//! **0 bytes** on the heap.

use pilot_span::ByteSpan;

/// 32-bit `Copy` interned identifier/string handle.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolId(pub u32);

impl SymbolId {
    pub const NONE: Self = Self(u32::MAX);

    #[inline]
    pub const fn is_none(self) -> bool {
        self.0 == u32::MAX
    }
}

/// 32-bit `Copy` interned first-class keyword handle (`:kw` or `:namespace/kw`, Milestone 17).
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeywordId(pub u32);

impl KeywordId {
    pub const NONE: Self = Self(u32::MAX);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct KeywordEntry {
    pub raw_symbol: SymbolId,
    pub namespace: SymbolId,
    pub name: SymbolId,
}

#[derive(Clone, Debug)]
pub struct Interner {
    bytes: Vec<u8>,
    spans: Vec<ByteSpan>,
    hashes: Vec<u64>,
    buckets: Vec<u32>, // u32::MAX = empty slot, else SymbolId index
    keywords: Vec<KeywordEntry>,
}

const EMPTY_BUCKET: u32 = u32::MAX;

impl Default for Interner {
    fn default() -> Self {
        Self::with_capacity(256, 4096)
    }
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(expected_symbols: usize, expected_bytes: usize) -> Self {
        let bucket_cap = (expected_symbols * 2).max(64).next_power_of_two();
        let mut interner = Self {
            bytes: Vec::with_capacity(expected_bytes),
            spans: Vec::with_capacity(expected_symbols),
            hashes: Vec::with_capacity(expected_symbols),
            buckets: vec![EMPTY_BUCKET; bucket_cap],
            keywords: Vec::new(),
        };
        // SymbolId(0) is always the empty string ""
        let _ = interner.intern("");
        interner
    }

    /// Interns `s` into the contiguous UTF-8 pool without allocating if already present.
    #[inline]
    pub fn intern(&mut self, s: &str) -> SymbolId {
        let hash = fx_hash_bytes(s.as_bytes());
        let mask = self.buckets.len() - 1;
        let mut slot = (hash as usize) & mask;

        loop {
            let entry = self.buckets[slot];
            if entry == EMPTY_BUCKET {
                break;
            }
            if self.hashes[entry as usize] == hash {
                let span = self.spans[entry as usize];
                let existing = &self.bytes[span.start as usize..span.end as usize];
                if existing == s.as_bytes() {
                    return SymbolId(entry);
                }
            }
            slot = (slot + 1) & mask;
        }

        let sym_idx = self.spans.len() as u32;
        let start = self.bytes.len() as u32;
        self.bytes.extend_from_slice(s.as_bytes());
        let end = self.bytes.len() as u32;

        self.spans.push(ByteSpan::new(start, end));
        self.hashes.push(hash);
        self.buckets[slot] = sym_idx;

        if (self.spans.len() * 10) >= (self.buckets.len() * 7) {
            self.rehash();
        }

        SymbolId(sym_idx)
    }

    /// Interns a Pilot keyword (`:ok` or `:http/ok` or `ok`) into a `KeywordId(u32)`.
    pub fn intern_keyword(&mut self, raw: &str) -> KeywordId {
        let stripped = raw.strip_prefix(':').unwrap_or(raw);
        let raw_symbol = self.intern(stripped);
        let (ns_sym, name_sym) = if let Some((ns, name)) = stripped.split_once('/') {
            (self.intern(ns), self.intern(name))
        } else {
            (SymbolId::NONE, raw_symbol)
        };

        for (idx, kw) in self.keywords.iter().enumerate() {
            if kw.raw_symbol == raw_symbol {
                return KeywordId(idx as u32);
            }
        }

        let id = KeywordId(self.keywords.len() as u32);
        self.keywords.push(KeywordEntry {
            raw_symbol,
            namespace: ns_sym,
            name: name_sym,
        });
        id
    }

    #[inline]
    pub fn resolve(&self, id: SymbolId) -> &str {
        if id.is_none() || (id.0 as usize) >= self.spans.len() {
            return "";
        }
        let span = self.spans[id.0 as usize];
        let slice = &self.bytes[span.start as usize..span.end as usize];
        unsafe { core::str::from_utf8_unchecked(slice) }
    }

    #[inline]
    pub fn resolve_keyword(&self, id: KeywordId) -> Option<KeywordEntry> {
        self.keywords.get(id.0 as usize).copied()
    }

    #[inline]
    pub fn symbol_count(&self) -> usize {
        self.spans.len()
    }

    #[inline]
    pub fn keyword_count(&self) -> usize {
        self.keywords.len()
    }

    fn rehash(&mut self) {
        let new_cap = (self.buckets.len() * 2).next_power_of_two();
        self.buckets.clear();
        self.buckets.resize(new_cap, EMPTY_BUCKET);
        let mask = new_cap - 1;

        for (idx, &hash) in self.hashes.iter().enumerate() {
            let mut slot = (hash as usize) & mask;
            while self.buckets[slot] != EMPTY_BUCKET {
                slot = (slot + 1) & mask;
            }
            self.buckets[slot] = idx as u32;
        }
    }
}

#[inline]
fn fx_hash_bytes(bytes: &[u8]) -> u64 {
    const K: u64 = 0x517c_c1b7_2722_0a95;
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut i = 0;
    while i + 8 <= bytes.len() {
        let word = u64::from_le_bytes(bytes[i..i + 8].try_into().unwrap());
        hash = (hash.rotate_left(5) ^ word).wrapping_mul(K);
        i += 8;
    }
    while i < bytes.len() {
        hash = (hash.rotate_left(5) ^ (bytes[i] as u64)).wrapping_mul(K);
        i += 1;
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_deduplicates_strings_and_hierarchical_keywords() {
        let mut interner = Interner::new();
        let s1 = interner.intern("SliceBuffer");
        let s2 = interner.intern("RangeWindow");
        let s3 = interner.intern("SliceBuffer");
        assert_eq!(s1, s3);
        assert_ne!(s1, s2);
        assert_eq!(interner.resolve(s1), "SliceBuffer");
        assert_eq!(interner.resolve(s2), "RangeWindow");

        let kw1 = interner.intern_keyword(":http/ok");
        let kw2 = interner.intern_keyword("http/ok");
        assert_eq!(kw1, kw2);
        let entry = interner.resolve_keyword(kw1).unwrap();
        assert_eq!(interner.resolve(entry.namespace), "http");
        assert_eq!(interner.resolve(entry.name), "ok");
    }
}
