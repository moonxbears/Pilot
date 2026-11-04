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

    /// Verifies that a type with `lattice_bits` and optional `iso` capability can transfer across
    /// a `.pchor` cue or realtime arena boundary with zero allocations or deep copies.
    /// Allows `IValue` (`0b00`) unconditionally and `iso`-qualified `IObject` (`0b10`).
    pub fn verify_cue_transfer(
        &mut self,
        cue_name: &str,
        lattice_bits: u8,
        is_iso_qualified: bool,
        span: ByteSpan,
    ) -> bool {
        let bits = lattice_bits & 0b11;
        if bits == 0b00 || (bits == 0b10 && is_iso_qualified) {
            return true;
        }
        self.diagnostics.push(AnalysisDiagnostic::error(
            "PILOT105",
            format!(
                "Type with lattice `{:#04b}` (iso={}) cannot cross `.pchor` cue or realtime arena boundary `{}` without `IValue` (`0b00`) or `iso IObject` (`0b10`) capability",
                bits, is_iso_qualified, cue_name
            ),
            span,
        ));
        false
    }
}
