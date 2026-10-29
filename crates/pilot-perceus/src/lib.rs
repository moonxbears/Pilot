//! Milestone 33.4: Worry-Free Memory Pass — Escape Analysis, Region Placement & Perceus ARC Insertion (`pilot-perceus`)
//!
//! Enforces:
//! - Tier 1 Escape Placement: Lowers non-escaping allocations (strings, lists, maps, closures) to `RegionAlloc` bump arenas with O(1) watermark resets.
//! - Tier 2 Perceus Precise ARC & Drop-Guided Reuse: Inserts minimal last-use `dup`/`drop` and `drop_reuse` in-place memory recycling tokens when `rc == 1`.
//! - `@hard_realtime` & `noalloc` static proof: Guarantees zero Tier 2 heap allocations in real-time blocks (`PILOT104`).

pub mod escape;
pub mod arc;
pub mod realtime;

pub use escape::{AllocationSite, EscapeAnalyzer, EscapeState};
pub use arc::{ArcOp, PerceusArcEngine};
pub use realtime::RealtimeVerifier;

pub struct PerceusReport {
    pub region_allocs: usize,
    pub global_allocs: usize,
    pub stack_allocs: usize,
    pub drop_reuses: usize,
    pub elided_retains: usize,
}

pub struct PerceusOptimizer {
    pub escape: EscapeAnalyzer,
    pub arc: PerceusArcEngine,
    pub realtime: RealtimeVerifier,
}

impl Default for PerceusOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PerceusOptimizer {
    pub fn new() -> Self {
        Self {
            escape: EscapeAnalyzer::new(),
            arc: PerceusArcEngine::new(),
            realtime: RealtimeVerifier::new(),
        }
    }

    pub fn generate_report(&self) -> PerceusReport {
        PerceusReport {
            region_allocs: self.escape.region_alloc_count(),
            global_allocs: self.escape.global_alloc_count(),
            stack_allocs: self.escape.stack_alloc_count(),
            drop_reuses: self.arc.drop_reuse_count(),
            elided_retains: self.arc.elided_retains_count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pilot_span::ByteSpan;

    #[test]
    fn test_tier1_region_alloc_placement_and_escape_escalation() {
        let mut optimizer = PerceusOptimizer::new();

        // 1. Local non-escaping string concat -> RegionAlloc
        let site1 = optimizer.escape.record_allocation("temp_str", "str", EscapeState::Region);
        // 2. Local stack primitive
        let site2 = optimizer.escape.record_allocation("count", "i64", EscapeState::Stack);
        // 3. Escaping object returned from function
        let site3 = optimizer.escape.record_allocation("result_node", "Node", EscapeState::Region);
        optimizer.escape.mark_escaped(site3, EscapeState::Global);

        assert_eq!(optimizer.escape.get_allocation(site1).unwrap().escape_state, EscapeState::Region);
        assert_eq!(optimizer.escape.get_allocation(site2).unwrap().escape_state, EscapeState::Stack);
        assert_eq!(optimizer.escape.get_allocation(site3).unwrap().escape_state, EscapeState::Global);

        let report = optimizer.generate_report();
        assert_eq!(report.region_allocs, 1);
        assert_eq!(report.stack_allocs, 1);
        assert_eq!(report.global_allocs, 1);
    }

    #[test]
    fn test_tier2_perceus_precise_arc_and_drop_reuse_tokens() {
        let mut optimizer = PerceusOptimizer::new();

        // Borrowed arg -> retain elided
        optimizer.arc.elide_borrowed_retain();

        // Object dropped, immediately followed by new allocation of identical layout
        optimizer.arc.insert_drop("old_buffer");
        optimizer.arc.insert_drop_reuse("old_buffer", "new_buffer");

        assert_eq!(optimizer.arc.drop_reuse_count(), 1);
        assert_eq!(optimizer.arc.elided_retains_count(), 1);

        let ops = optimizer.arc.ops();
        assert_eq!(ops.len(), 2);
        assert_eq!(
            ops[1],
            ArcOp::DropReuse {
                dropped_var: "old_buffer".to_string(),
                target_var: "new_buffer".to_string(),
            }
        );
    }

    #[test]
    fn test_hard_realtime_noalloc_verification_catches_heap_leak() {
        let mut optimizer = PerceusOptimizer::new();
        let span = ByteSpan::new(10, 25);

        // Tier 0 / Tier 1 inside noalloc -> allowed
        optimizer.realtime.verify_noalloc_block("dsp_process", EscapeState::Region, span);
        assert!(optimizer.realtime.diagnostics.is_empty());

        // Tier 2 Global heap escape inside noalloc -> PILOT104
        optimizer.realtime.verify_noalloc_block("dsp_process", EscapeState::Global, span);
        assert_eq!(optimizer.realtime.diagnostics.len(), 1);
        assert_eq!(optimizer.realtime.diagnostics[0].code, "PILOT104");
    }
}
