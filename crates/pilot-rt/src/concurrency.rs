//! Milestone 34.4: Concurrency Channels, Reactive Signals (!^) & Dry-Run Sandbox (!?).

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use crate::value::PilotVal;

/// Bounded MPMC Channel for message passing between Pilot tasks.
#[derive(Clone)]
pub struct Channel {
    inner: Arc<Mutex<VecDeque<PilotVal>>>,
    capacity: usize,
}

impl Channel {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(capacity.min(1024)))),
            capacity,
        }
    }

    pub fn send(&self, val: PilotVal) -> bool {
        let mut q = self.inner.lock().unwrap();
        if q.len() >= self.capacity {
            return false;
        }
        q.push_back(val);
        true
    }

    pub fn recv(&self) -> Option<PilotVal> {
        let mut q = self.inner.lock().unwrap();
        q.pop_front()
    }
}

/// Reactive Signal Graph node (!^).
pub struct Signal {
    pub id: u32,
    pub value: PilotVal,
    pub is_dirty: bool,
    pub subscribers: Vec<u32>,
}

impl Signal {
    pub fn new(id: u32, initial: PilotVal) -> Self {
        Self {
            id,
            value: initial,
            is_dirty: false,
            subscribers: Vec::new(),
        }
    }

    pub fn set(&mut self, new_val: PilotVal) {
        if self.value != new_val {
            self.value = new_val;
            self.is_dirty = true;
        }
    }

    pub fn get(&self) -> PilotVal {
        self.value
    }
}

/// Copy-on-Write Shadow Effect Context for dry-run transactions (!?).
pub struct ShadowEffectContext {
    original_state: HashMap<String, PilotVal>,
    shadow_writes: HashMap<String, PilotVal>,
    committed: bool,
}

impl Default for ShadowEffectContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ShadowEffectContext {
    pub fn new() -> Self {
        Self {
            original_state: HashMap::new(),
            shadow_writes: HashMap::new(),
            committed: false,
        }
    }

    pub fn read(&self, key: &str) -> Option<PilotVal> {
        if let Some(&val) = self.shadow_writes.get(key) {
            Some(val)
        } else {
            self.original_state.get(key).copied()
        }
    }

    pub fn write(&mut self, key: &str, val: PilotVal) {
        self.shadow_writes.insert(key.to_string(), val);
    }

    pub fn commit(&mut self) {
        for (k, v) in self.shadow_writes.drain() {
            self.original_state.insert(k, v);
        }
        self.committed = true;
    }

    pub fn rollback(&mut self) {
        self.shadow_writes.clear();
        self.committed = false;
    }

    pub fn is_committed(&self) -> bool {
        self.committed
    }
}
