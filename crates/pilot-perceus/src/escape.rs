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

    /// 1-cycle `LatticeBits` borrow-escape check for `IReference` (`0b01`) and `IGuard` (`0b11`).
    /// When `(lattice_bits & 0b01) != 0`:
    /// 1. Forbids returning the value from a function unless its lifetime is tied to an input `ref` parameter.
    /// 2. Forbids storing the value inside an `IObject` (`0b10`) heap container.
    /// 3. Forbids sending the value across a `.pchor` `cue` boundary.
    pub fn verify_borrow_escape_by_lattice(
        &self,
        var_name: &str,
        lattice_bits: u8,
        tied_to_input_ref_param: bool,
        stored_in_container_lattice: Option<u8>,
        crosses_cue_boundary: bool,
        span: pilot_span::ByteSpan,
    ) -> Result<(), pilot_analysis::diagnostics::AnalysisDiagnostic> {
        if (lattice_bits & 0b01) == 0 {
            return Ok(());
        }
        if crosses_cue_boundary {
            return Err(pilot_analysis::diagnostics::AnalysisDiagnostic::error(
                "ESCAPE001",
                format!(
                    "Borrow-tracked value `{}` (lattice `{:#04b}`) cannot cross a `.pchor` `cue` or thread boundary",
                    var_name,
                    lattice_bits & 0b11
                ),
                span,
            ));
        }
        if let Some(container_lat) = stored_in_container_lattice {
            if (container_lat & 0b10) != 0 && (container_lat & 0b01) == 0 {
                return Err(pilot_analysis::diagnostics::AnalysisDiagnostic::error(
                    "ESCAPE002",
                    format!(
                        "Borrow-tracked value `{}` (lattice `{:#04b}`) cannot be stored inside an `IObject` (`0b10`) heap container",
                        var_name,
                        lattice_bits & 0b11
                    ),
                    span,
                ));
            }
        }
        if !tied_to_input_ref_param {
            return Err(pilot_analysis::diagnostics::AnalysisDiagnostic::error(
                "ESCAPE003",
                format!(
                    "Borrow-tracked value `{}` (lattice `{:#04b}`) cannot escape function return unless tied to an input `ref` parameter",
                    var_name,
                    lattice_bits & 0b11
                ),
                span,
            ));
        }
        Ok(())
    }
}
