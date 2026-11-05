//! Milestone 33: Native Static Analysis, Bidirectional Inference, Perceus Escape Analysis & `comptime` (`pilot-analysis`)

pub mod diagnostics;
pub mod types;
pub mod infer;
pub mod patterns;
pub mod loops;
pub mod contracts;
pub mod dimensions;
pub mod comptime;
pub mod pipeline;
pub mod autodiff;
pub mod quoting;
pub mod macros;

pub use diagnostics::{
    verify_core_naming_rules, AnalysisDiagnostic, Diagnostic, DiagnosticSeverity, PackageAst,
};
pub use types::{
    CapabilityBits, CapabilityPhysics, FieldOffset, HwScalarHeader, LayoutId, LayoutTable, PhysicalLayout,
    PhysicsGeometry,
};
pub use infer::{
    analyze_physics, flatten_ast, handle_engine_intrinsic, run_pilot_analyzer,
    compute_field_projection, compute_soa_projection, verify_capability_transition,
    NodeLayoutMap, PhysicsDiagnostic, PhysicsRegistry, RawAstArena, RawAstNode,
};
pub use patterns::{MarangetChecker, PatternCase};
pub use loops::{
    ForLoopAnalyzer, ForLoweringStrategy, LoopAnalyzer, LoopReturnMode, LoopScheduleMode,
};
pub use contracts::{ContractVerifier, Interval};
pub use dimensions::{DimensionChecker, DimensionVector};
pub use comptime::{
    bootstrap_core_file, eval_file_with_tables, BootstrapDiagnostic, BootstrapSnapshot,
    CallFrameLayoutInfo, ComptimeEngine, ComptimeEvaluator, ComptimeFnDef, ComptimeValue,
    InlineHint, MemoryLayoutMode, QuoteKind, RegisterClass, StaticDemotedSlot,
    TypeLayoutSpec,
};
pub use pipeline::{AnalysisPipeline, AnalysisResult, AttributeWeaver};
pub use autodiff::{DualValue, SymbolicDifferentiator, SymbolicExpr};
pub use quoting::{
    AstSpliceResult, CallFrameLayout, CallQuoteProjectionInfo, CallQuotingEngine, EffectDiff,
    EffectPlan, ReactiveSignalNode, ShallowCallDescriptor,
};
pub use macros::{
    desugar_bracket_attributes, expand_syntax_macro, splice_quote_template, DecoratorKind,
    MacroExpander, SynthesizedMember,
};
