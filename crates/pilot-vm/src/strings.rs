//! Strings the VM makes while a program runs.
//!
//! A string value is a tagged pointer (`pilot_rt::PilotVal`), copied freely,
//! with no count of who holds it. Constants and strings made outside the VM
//! are interned for the life of the process. Strings the VM makes at run time
//! (concatenation, slices, file text, process output) are owned here instead,
//! and freed once no value the VM can reach refers to them: otherwise a loop
//! that builds a string one piece at a time keeps every intermediate string,
//! and memory grows with the square of the result's length.
//!
//! The collector (`collect`) frees them with the lists and records nothing
//! reaches. Strings still owned when the VM is dropped are kept for the life
//! of the process, so a result the host reads after that stays valid.

use crate::vm::PilVirtualMachine;
use pilot_rt::PilotVal;
use std::collections::{HashMap, HashSet};

/// The run-time strings of one VM, by the address of their bytes.
#[derive(Default)]
pub struct RuntimeStrings {
    owned: HashMap<usize, Box<str>>,
    /// Bytes owned now.
    bytes: usize,
}

impl RuntimeStrings {
    /// The number of strings owned now.
    pub fn len(&self) -> usize {
        self.owned.len()
    }

    pub fn is_empty(&self) -> bool {
        self.owned.is_empty()
    }

    /// The bytes owned now.
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}

impl Drop for RuntimeStrings {
    fn drop(&mut self) {
        for (_, s) in self.owned.drain() {
            Box::leak(s);
        }
    }
}

impl PilVirtualMachine {
    /// A string value made at run time, freed once nothing refers to it.
    pub fn new_str(&mut self, s: &str) -> PilotVal {
        if let Some(v) = PilotVal::from_str_sso(s) {
            return v;
        }
        let owned: Box<str> = s.into();
        // The bytes stay where they are until the collector drops them,
        // which it does only once no reachable value refers to them.
        let v = unsafe { PilotVal::from_heap_parts(owned.as_ptr(), owned.len()) };
        self.strings.bytes += owned.len();
        self.strings.owned.insert(owned.as_ptr() as usize, owned);
        v
    }

    /// Keeps only the strings whose bytes start at an address in `live`.
    pub(crate) fn retain_strings(&mut self, live: &HashSet<usize>) {
        let strings = &mut self.strings;
        strings.owned.retain(|addr, _| live.contains(addr));
        strings.bytes = strings.owned.values().map(|s| s.len()).sum();
    }
}
