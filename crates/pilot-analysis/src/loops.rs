//! Milestone 33.2 & Self-Hosting Phase 4: Dual-Path 3-Tier `ForExpr` & `IEnumerable<T>` Loop Analyzer.

use crate::diagnostics::AnalysisDiagnostic;
use crate::types::LayoutId;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LoopScheduleMode {
    FixedScalar,
    MutableBounds,
    WorklistStack,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LoopReturnMode {
    StatementVoid,
    CollectionYield { elem_layout: LayoutId },
    ScalarBreak { value_layout: LayoutId, has_then_fallback: bool },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ForLoweringStrategy {
    ThreeTierDynamicSchedule {
        schedule: LoopScheduleMode,
        ret_mode: LoopReturnMode,
        can_preallocate_yield_buffer: bool,
    },
    EnumerableProtocol {
        enumerator_layout: LayoutId,
        item_layout: LayoutId,
    },
}

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
        Self { diagnostics: Vec::new() }
    }
}

pub type ForLoopAnalyzer = LoopAnalyzer;
