//! Bounded MPMC Concurrency Channel (`Channel`).
//! Reactive Signals (`!^`) and Shadow Effect Dry-Runs (`!?`) live in `stdlib/core/{reactive,effects}.pilot`.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use crate::value::PilotVal;

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
        if q.len() >= self.capacity { return false; }
        q.push_back(val);
        true
    }

    pub fn recv(&self) -> Option<PilotVal> {
        self.inner.lock().unwrap().pop_front()
    }
}
