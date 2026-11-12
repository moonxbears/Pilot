//! The seed VM's collector: frees the lists, records, closure environments
//! and run-time strings no live value can reach.
//!
//! Values are tagged handles (`pilot_rt::PilotVal`) copied freely, with no
//! count of who holds them, so the VM finds what is live by tracing. The
//! roots are every value the program can still read:
//!
//! - the registers of the running frame and the saved registers of every
//!   frame below it (a block's locals are cleared when the block ends, so a
//!   value whose scope has closed is no longer held there);
//! - module globals, the constants, and the registers of generator frames;
//! - record slot 0 (the environment record) and every slot the host made
//!   before the program started (`pin_host_slots`).
//!
//! From the roots the collector follows list elements, record values,
//! closure environments and record type tags. Every slot it did not reach is
//! emptied and marked dead; a dead slot is reused by the next allocation.
//! Reading a dead slot is an internal error (`list_slot`, `map_slot`), never
//! a silent read of another value.
//!
//! Collection runs only between instructions (`collect_if_due`), after an
//! intrinsic or a string concatenation has written its result, when every
//! value is in one of the roots. The host never keeps a VM value across
//! instructions: host operations receive copies (`HostValue`) and the engine
//! handler stores no values.
//!
//! With `PILOT_HEAP_VERIFY` set, the VM collects four times as often and
//! never reuses a slot, so a value the roots miss is read from a dead slot
//! and faults instead of silently reading whatever reused the slot.

use crate::vm::PilVirtualMachine;
use pilot_rt::PilotVal;
use std::collections::HashSet;

/// Allocations (lists, records, closures) between collections, at least.
const MIN_ALLOCATIONS_BETWEEN: usize = 64 * 1024;
/// Run-time string bytes made between collections, at least.
const MIN_STRING_BYTES_BETWEEN: usize = 8 << 20;
/// Allocations between collections in verify mode, at least; at most a
/// quarter of what was live after the last one.
const VERIFY_ALLOCATIONS_BETWEEN: usize = 1024;
/// The environment variable that turns verify mode on.
pub const VERIFY_ENV: &str = "PILOT_HEAP_VERIFY";

/// What the collector knows about the heap.
#[derive(Default)]
pub struct HeapState {
    /// Dead list slots (lists and closure environments), by slot.
    list_dead: Vec<bool>,
    /// Dead record slots, by slot.
    map_dead: Vec<bool>,
    /// Dead slots ready to be reused.
    free_lists: Vec<usize>,
    free_maps: Vec<usize>,
    /// Slots below these were made by the host before the program ran.
    pinned_lists: usize,
    pinned_maps: usize,
    pinned: bool,
    /// Allocations and string bytes since the last collection.
    allocations: usize,
    string_bytes_at_last: usize,
    /// Live slots after the last collection.
    live_after_last: usize,
    live_string_bytes_after_last: usize,
    /// Collections run so far.
    collections: u64,
    /// Collect often and never reuse a slot (`VERIFY_ENV`).
    verify: bool,
}

impl HeapState {
    pub fn new() -> Self {
        Self { verify: std::env::var_os(VERIFY_ENV).is_some(), ..Self::default() }
    }

    /// Collections run so far.
    pub fn collections(&self) -> u64 {
        self.collections
    }

    /// Forgets every slot: the VM's heap has just been emptied.
    pub(crate) fn reset(&mut self) {
        let verify = self.verify;
        *self = Self { verify, ..Self::default() };
    }
}

impl PilVirtualMachine {
    /// A slot for a new list, reusing a dead one when there is one.
    pub(crate) fn alloc_list_slot(&mut self, items: Vec<PilotVal>) -> usize {
        self.heap.allocations += 1;
        if let Some(slot) = self.heap.free_lists.pop() {
            self.heap_lists[slot] = items;
            self.heap.list_dead[slot] = false;
            return slot;
        }
        self.heap_lists.push(items);
        self.heap_lists.len() - 1
    }

    /// A slot for a new record, reusing a dead one when there is one.
    pub(crate) fn alloc_map_slot(&mut self, m: std::collections::HashMap<String, PilotVal>) -> usize {
        self.heap.allocations += 1;
        if let Some(slot) = self.heap.free_maps.pop() {
            self.heap_maps[slot] = m;
            self.heap.map_dead[slot] = false;
            return slot;
        }
        self.heap_maps.push(m);
        self.heap_maps.len() - 1
    }

    /// Whether list slot `slot` was freed.
    #[inline]
    pub(crate) fn list_is_dead(&self, slot: usize) -> bool {
        self.heap.list_dead.get(slot).copied().unwrap_or(false)
    }

    /// Whether record slot `slot` was freed.
    #[inline]
    pub(crate) fn map_is_dead(&self, slot: usize) -> bool {
        self.heap.map_dead.get(slot).copied().unwrap_or(false)
    }

    /// Keeps the slots the host made before the program started (the
    /// comptime environment's records) for as long as the program runs.
    pub(crate) fn pin_host_slots(&mut self) {
        if !self.heap.pinned {
            self.heap.pinned = true;
            self.heap.pinned_lists = self.heap_lists.len();
            self.heap.pinned_maps = self.heap_maps.len();
        }
    }

    /// Collects once enough has been allocated since the last collection.
    /// Called only between instructions.
    #[inline]
    pub(crate) fn collect_if_due(&mut self) {
        let h = &self.heap;
        let new_bytes = self.strings.bytes().saturating_sub(h.string_bytes_at_last);
        let due = if h.verify {
            h.allocations >= (h.live_after_last / 4).max(VERIFY_ALLOCATIONS_BETWEEN)
        } else {
            h.allocations >= h.live_after_last.max(MIN_ALLOCATIONS_BETWEEN)
                || new_bytes >= h.live_string_bytes_after_last.max(MIN_STRING_BYTES_BETWEEN)
        };
        if due {
            self.collect();
        }
    }

    /// Frees every list, record, closure environment and run-time string no
    /// root reaches.
    pub fn collect(&mut self) {
        let (lists, maps) = (self.heap_lists.len(), self.heap_maps.len());
        self.heap.list_dead.resize(lists, false);
        self.heap.map_dead.resize(maps, false);
        let mut list_marks = vec![false; lists];
        let mut map_marks = vec![false; maps];
        let mut strings: HashSet<usize> = HashSet::new();
        let mut work: Vec<PilotVal> = Vec::new();
        work.extend_from_slice(&self.registers[..self.reg_top]);
        work.extend_from_slice(&self.reg_stack);
        work.extend_from_slice(&self.constants);
        work.extend(self.globals.values().copied());
        work.extend(self.generator_values().copied());
        work.extend((0..self.heap.pinned_lists.min(lists)).map(PilotVal::from_list_handle));
        work.extend((0..self.heap.pinned_maps.min(maps).max(maps.min(1))).map(PilotVal::from_map_handle));
        while let Some(v) = work.pop() {
            if let Some((ptr, _)) = v.heap_parts() {
                strings.insert(ptr as usize);
            } else if let Some(slot) = v.as_list_handle().or_else(|| v.as_fn_handle()) {
                if slot < lists && !list_marks[slot] {
                    list_marks[slot] = true;
                    work.extend_from_slice(&self.heap_lists[slot]);
                }
            } else if let Some(slot) = v.as_map_handle() {
                if slot < maps && !map_marks[slot] {
                    map_marks[slot] = true;
                    work.extend(self.heap_maps[slot].values().copied());
                    if let Some(ty) = self.record_types.get(&slot) {
                        work.push(*ty);
                    }
                }
            }
        }
        let verify = self.heap.verify;
        let mut live = 0;
        for slot in 0..lists {
            if list_marks[slot] {
                live += 1;
            } else if !self.heap.list_dead[slot] {
                self.heap_lists[slot] = Vec::new();
                self.heap.list_dead[slot] = true;
                if !verify {
                    self.heap.free_lists.push(slot);
                }
            }
        }
        for slot in 0..maps {
            if map_marks[slot] {
                live += 1;
            } else if !self.heap.map_dead[slot] {
                self.heap_maps[slot] = Default::default();
                self.heap.map_dead[slot] = true;
                self.record_types.remove(&slot);
                if !verify {
                    self.heap.free_maps.push(slot);
                }
            }
        }
        self.retain_strings(&strings);
        let h = &mut self.heap;
        h.allocations = 0;
        h.live_after_last = live;
        h.string_bytes_at_last = self.strings.bytes();
        h.live_string_bytes_after_last = self.strings.bytes();
        h.collections += 1;
    }
}
