//! Milestone 33.2: 3-Tier For-Loop Static Verification (LOOP001–LOOP004).

use pilot_span::ByteSpan;
use crate::diagnostics::AnalysisDiagnostic;

pub struct LoopAnalyzer {
    pub diagnostics: Vec<AnalysisDiagnostic>,
}

impl Default for LoopAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LoopAnalyzer {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn report_non_iterable(&mut self, ty_name: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "LOOP001",
            format!("Type '{}' is not iterable: must implement iterator protocol, range, or collection", ty_name),
            span,
        ));
    }

    pub fn report_mutation_during_iteration(&mut self, coll_name: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "LOOP002",
            format!("Illegal concurrent modification: cannot mutate collection '{}' while actively iterating over it", coll_name),
            span,
        ));
    }

    pub fn report_zero_step_range(&mut self, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "LOOP003",
            "Range step argument cannot be zero",
            span,
        ));
    }

    pub fn report_yield_outside_comprehension(&mut self, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "LOOP004",
            "'yield' is only permitted within comprehension or generator expressions",
            span,
        ));
    }
}
