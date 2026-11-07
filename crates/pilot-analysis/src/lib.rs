//! Native Static Analysis, Lossless `__engine` Bridge & `comptime` (`pilot-analysis`)

pub mod diagnostics;
pub mod types;
pub mod infer;
pub mod comptime;
pub mod pipeline;

pub use diagnostics::{AnalysisDiagnostic, Diagnostic, DiagnosticSeverity};
pub use types::{CapabilityBits, CapabilityPhysics, FieldOffset, HwScalarHeader, LayoutId, LayoutTable, PhysicalLayout, PhysicsGeometry};
pub use infer::{
    analyze_physics, analyze_physics_with_source, flatten_ast, handle_engine_intrinsic, load_or_compile_analyzer_pilc,
    run_pilot_analyzer, CategoryRecord, PhysicsDiagnostic, PhysicsRegistry, RawAstArena, RawAstNode, RawAttr,
};
pub use comptime::{bootstrap_core_file, eval_file_with_tables, BootstrapDiagnostic, BootstrapSnapshot, ComptimeEngine, ComptimeEvaluator, ComptimeValue};
pub use pipeline::{AnalysisPipeline, AnalysisResult};
