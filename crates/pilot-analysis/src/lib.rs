//! Milestone 33: Native Static Analysis, Bidirectional Inference, Perceus Escape Analysis & `comptime` (`pilot-analysis`)

pub mod diagnostics;
pub mod types;
pub mod infer;
pub mod comptime;
pub mod pipeline;

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
pub use comptime::{
    bootstrap_core_file, eval_file_with_tables, BootstrapDiagnostic, BootstrapSnapshot,
    ComptimeEngine, ComptimeEvaluator, ComptimeValue,
};
pub use pipeline::{AnalysisPipeline, AnalysisResult, AttributeWeaver};
