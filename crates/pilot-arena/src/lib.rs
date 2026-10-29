//! Milestone 28.2: Zero-GC Arena Allocation Primitives (`pilot-arena`)
//!
//! Enforces Cross-Milestone Invariant #9:
//! - No heap-allocated pointer graphs (`Box<Expr>`, `Rc`, or self-referential `'ast` lifetimes).
//! - 64KB (`65,536`-byte) page-aligned bump chunks with $O(1)$ watermark reset for thread-local reuse.
//! - Contiguous `TypedArena<T>` indexed by 32-bit `Copy` handles (`u32`).

use std::cell::RefCell;
use std::marker::PhantomData;

/// Default page-aligned chunk capacity: 64 KiB (`65,536` bytes).
pub const DEFAULT_CHUNK_CAPACITY: usize = 64 * 1024;

/// A 64KB chunked bump allocator that retains allocated slabs across `.reset()` calls
/// so subsequent parse/analysis passes perform zero OS/allocator syscalls.
#[derive(Debug)]
pub struct BumpArena {
    chunks: Vec<Vec<u8>>,
    current_chunk: usize,
    offset: usize,
    chunk_capacity: usize,
    total_allocated_bytes: usize,
}

impl Default for BumpArena {
    fn default() -> Self {
        Self::with_chunk_capacity(DEFAULT_CHUNK_CAPACITY)
    }
}

impl BumpArena {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn with_chunk_capacity(chunk_capacity: usize) -> Self {
        let cap = chunk_capacity.max(4096).next_power_of_two();
        let mut first = Vec::with_capacity(cap);
        first.resize(cap, 0);
        Self {
            chunks: vec![first],
            current_chunk: 0,
            offset: 0,
            chunk_capacity: cap,
            total_allocated_bytes: 0,
        }
    }

    /// Allocates `size` bytes aligned to `align` (power of two) inside the current 64KB chunk,
    /// or advances/allocates a new chunk if the current chunk has insufficient space.
    #[inline]
    pub fn alloc_raw(&mut self, size: usize, align: usize) -> &mut [u8] {
        let align = align.max(1);
        debug_assert!(align.is_power_of_two());

        let aligned_offset = (self.offset + (align - 1)) & !(align - 1);
        let chunk_len = self.chunks[self.current_chunk].len();

        if aligned_offset + size <= chunk_len {
            self.offset = aligned_offset + size;
            self.total_allocated_bytes += size;
            return &mut self.chunks[self.current_chunk][aligned_offset..self.offset];
        }

        self.grow_for(size, align)
    }

    #[cold]
    fn grow_for(&mut self, size: usize, align: usize) -> &mut [u8] {
        let required = size.checked_add(align).unwrap_or(size);
        // Reuse an already-allocated subsequent chunk if available and large enough
        if self.current_chunk + 1 < self.chunks.len()
            && self.chunks[self.current_chunk + 1].len() >= required
        {
            self.current_chunk += 1;
            self.offset = size;
            self.total_allocated_bytes += size;
            return &mut self.chunks[self.current_chunk][0..size];
        }

        let new_cap = self.chunk_capacity.max(required.next_power_of_two());
        let mut chunk = Vec::with_capacity(new_cap);
        chunk.resize(new_cap, 0);
        self.chunks.push(chunk);
        self.current_chunk = self.chunks.len() - 1;
        self.offset = size;
        self.total_allocated_bytes += size;
        &mut self.chunks[self.current_chunk][0..size]
    }

    /// Copies a byte slice into the bump arena and returns its slice reference.
    #[inline]
    pub fn alloc_bytes(&mut self, bytes: &[u8]) -> &[u8] {
        if bytes.is_empty() {
            return &[];
        }
        let dst = self.alloc_raw(bytes.len(), 1);
        dst.copy_from_slice(bytes);
        dst
    }

    /// Copies a UTF-8 string into the bump arena and returns its `&str` reference.
    #[inline]
    pub fn alloc_str(&mut self, s: &str) -> &str {
        let bytes = self.alloc_bytes(s.as_bytes());
        // Safety: `s` is valid UTF-8 and we copied its exact bytes.
        unsafe { std::str::from_utf8_unchecked(bytes) }
    }

    /// Resets the bump watermark in $O(1)$ while keeping all allocated 64KB chunks in memory.
    #[inline]
    pub fn reset(&mut self) {
        self.current_chunk = 0;
        self.offset = 0;
        self.total_allocated_bytes = 0;
    }

    #[inline]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    #[inline]
    pub fn total_allocated_bytes(&self) -> usize {
        self.total_allocated_bytes
    }

    #[inline]
    pub fn reserved_capacity_bytes(&self) -> usize {
        self.chunks.iter().map(|c| c.len()).sum()
    }
}

/// Contiguous `Copy`-node arena indexed by a 32-bit handle (`u32`), satisfying `Send + Sync`.
#[derive(Clone, Debug)]
pub struct TypedArena<T: Copy, Id = u32> {
    items: Vec<T>,
    _marker: PhantomData<fn() -> Id>,
}

impl<T: Copy, Id> Default for TypedArena<T, Id> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<T: Copy, Id: From<u32> + Into<u32> + Copy> TypedArena<T, Id> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn alloc(&mut self, item: T) -> Id {
        let idx = self.items.len() as u32;
        self.items.push(item);
        Id::from(idx)
    }

    #[inline]
    pub fn get(&self, id: Id) -> Option<&T> {
        self.items.get(id.into() as usize)
    }

    #[inline]
    pub fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        self.items.get_mut(id.into() as usize)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.items
    }

    #[inline]
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

thread_local! {
    static THREAD_BUMP_ARENA: RefCell<BumpArena> = RefCell::new(BumpArena::new());
}

/// Executes `f` with a resettable thread-local 64KB `BumpArena`, resetting it upon completion.
pub fn with_thread_local_arena<R, F: FnOnce(&mut BumpArena) -> R>(f: F) -> R {
    THREAD_BUMP_ARENA.with(|cell| {
        let mut arena = cell.borrow_mut();
        arena.reset();
        let res = f(&mut arena);
        arena.reset();
        res
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_arena_alloc_and_reset_reuses_64kb_slab() {
        let mut arena = BumpArena::new();
        assert_eq!(arena.chunk_count(), 1);
        assert_eq!(arena.reserved_capacity_bytes(), DEFAULT_CHUNK_CAPACITY);

        let s1 = arena.alloc_str("pilot_zero_gc").to_owned();
        let b2 = arena.alloc_bytes(&[1, 2, 3, 4]).to_vec();
        assert_eq!(s1, "pilot_zero_gc");
        assert_eq!(b2, vec![1, 2, 3, 4]);
        assert!(arena.total_allocated_bytes() >= 17);

        let cap_before = arena.reserved_capacity_bytes();
        arena.reset();
        assert_eq!(arena.total_allocated_bytes(), 0);
        assert_eq!(arena.reserved_capacity_bytes(), cap_before);
    }

    #[test]
    fn typed_arena_u32_handles_roundtrip() {
        let mut arena: TypedArena<u64, u32> = TypedArena::with_capacity(16);
        let id0 = arena.alloc(100);
        let id1 = arena.alloc(200);
        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(*arena.get(id0).unwrap(), 100);
        assert_eq!(*arena.get(id1).unwrap(), 200);
    }
}
