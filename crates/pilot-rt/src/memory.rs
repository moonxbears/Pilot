//! Milestone 34.2: 3-Tier Deterministic Memory Engine.
//!
//! - Tier 1: Thread-local 64KB slab Bump Region Arenas with O(1) watermark resets.
//! - Tier 2: Perceus Non-Atomic / Atomic ARC Header (`rc: u32`, `type_tag: u16`, `flags: u16`).
//! - Bacon-Rajan Trial-Deletion Cycle Collector hooks running strictly outside real-time zones.

use std::sync::atomic::{AtomicU32, Ordering};

pub const FLAG_ATOMIC: u16 = 1 << 0;
pub const FLAG_CYCLE_BUFFERED: u16 = 1 << 1;
pub const FLAG_PURPLE: u16 = 1 << 2; // Bacon-Rajan candidate

/// Perceus reference counting header prepended to heap-allocated objects.
#[repr(C)]
pub struct ArcHeader {
    pub rc: u32,
    pub type_tag: u16,
    pub flags: u16,
}

impl ArcHeader {
    pub fn new(type_tag: u16, is_atomic: bool) -> Self {
        Self {
            rc: 1,
            type_tag,
            flags: if is_atomic { FLAG_ATOMIC } else { 0 },
        }
    }

    #[inline]
    pub fn is_atomic(&self) -> bool {
        (self.flags & FLAG_ATOMIC) != 0
    }

    #[inline]
    pub fn retain(&mut self) {
        if self.is_atomic() {
            let atomic_rc = unsafe { &*(&self.rc as *const u32 as *const AtomicU32) };
            atomic_rc.fetch_add(1, Ordering::Relaxed);
        } else {
            self.rc = self.rc.wrapping_add(1);
        }
    }

    #[inline]
    pub fn release(&mut self) -> bool {
        if self.is_atomic() {
            let atomic_rc = unsafe { &*(&self.rc as *const u32 as *const AtomicU32) };
            atomic_rc.fetch_sub(1, Ordering::Release) == 1
        } else {
            self.rc = self.rc.saturating_sub(1);
            self.rc == 0
        }
    }

    #[inline]
    pub fn can_drop_reuse(&self) -> bool {
        if self.is_atomic() {
            let atomic_rc = unsafe { &*(&self.rc as *const u32 as *const AtomicU32) };
            atomic_rc.load(Ordering::Relaxed) == 1
        } else {
            self.rc == 1
        }
    }
}

/// 64KB slab bump allocator arena for Tier 1 non-escaping allocations.
pub struct BumpRegion {
    slab: Box<[u8; 65536]>,
    offset: usize,
}

impl Default for BumpRegion {
    fn default() -> Self {
        Self::new()
    }
}

impl BumpRegion {
    pub fn new() -> Self {
        Self {
            slab: Box::new([0u8; 65536]),
            offset: 0,
        }
    }

    /// Allocates an aligned buffer from the 64KB slab.
    pub fn alloc(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        let current = self.offset;
        let aligned = (current + (align - 1)) & !(align - 1);
        if aligned + size > self.slab.len() {
            return None;
        }
        self.offset = aligned + size;
        Some(unsafe { self.slab.as_mut_ptr().add(aligned) })
    }

    /// O(1) watermark reset: releases all Tier 1 memory instantaneously upon scope/loop exit.
    #[inline]
    pub fn reset_watermark(&mut self, watermark: usize) {
        self.offset = watermark.min(self.slab.len());
    }

    #[inline]
    pub fn watermark(&self) -> usize {
        self.offset
    }
}
