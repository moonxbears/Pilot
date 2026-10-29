//! Milestone 33.4: @hard_realtime & noalloc Static Verification (PILOT104).

use pilot_analysis::diagnostics::AnalysisDiagnostic;
use pilot_span::ByteSpan;
use crate::escape::EscapeState;

pub struct RealtimeVerifier {
    pub diagnostics: Vec<AnalysisDiagnostic>,
}

impl Default for RealtimeVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl RealtimeVerifier {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn verify_noalloc_block(
        &mut self,
        block_name: &str,
        escape_state: EscapeState,
        span: ByteSpan,
    ) {
        if escape_state == EscapeState::Global {
            self.diagnostics.push(AnalysisDiagnostic::error(
                "PILOT104",
                format!(
                    "Illegal heap allocation in '@hard_realtime' / 'noalloc' context '{}': must execute strictly within Tier 0 registers or pre-allocated region buffers",
                    block_name
                ),
                span,
            ));
        }
    }
}
