//! Milestone 33.4: Intra-procedural and inter-procedural Escape Analysis.

use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EscapeState {
    /// Allocation does not escape current basic block/statement; eligible for Tier 0 stack register.
    Stack,
    /// Allocation escapes local statement but does not escape enclosing function or loop;
    /// eligible for Tier 1 `RegionAlloc` bump arena with O(1) watermark reset.
    Region,
    /// Allocation escapes function return, thread boundary, or is stored into escaping object;
    /// requires Tier 2 Perceus Precise ARC.
    Global,
}

#[derive(Clone, Debug)]
pub struct AllocationSite {
    pub site_id: u32,
    pub var_name: String,
    pub escape_state: EscapeState,
    pub type_desc: String,
}

pub struct EscapeAnalyzer {
    allocations: HashMap<u32, AllocationSite>,
    next_site_id: u32,
}

impl Default for EscapeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl EscapeAnalyzer {
    pub fn new() -> Self {
        Self {
            allocations: HashMap::new(),
            next_site_id: 1,
        }
    }

    pub fn record_allocation(&mut self, var_name: &str, type_desc: &str, state: EscapeState) -> u32 {
        let id = self.next_site_id;
        self.next_site_id += 1;
        self.allocations.insert(
            id,
            AllocationSite {
                site_id: id,
                var_name: var_name.to_string(),
                escape_state: state,
                type_desc: type_desc.to_string(),
            },
        );
        id
    }

    pub fn mark_escaped(&mut self, site_id: u32, new_state: EscapeState) {
        if let Some(site) = self.allocations.get_mut(&site_id) {
            if new_state > site.escape_state {
                site.escape_state = new_state;
            }
        }
    }

    pub fn get_allocation(&self, site_id: u32) -> Option<&AllocationSite> {
        self.allocations.get(&site_id)
    }

    pub fn region_alloc_count(&self) -> usize {
        self.allocations
            .values()
            .filter(|a| a.escape_state == EscapeState::Region)
            .count()
    }

    pub fn global_alloc_count(&self) -> usize {
        self.allocations
            .values()
            .filter(|a| a.escape_state == EscapeState::Global)
            .count()
    }

    pub fn stack_alloc_count(&self) -> usize {
        self.allocations
            .values()
            .filter(|a| a.escape_state == EscapeState::Stack)
            .count()
    }
}
