//! Native Static Analysis & Lossless `__engine` Bridge (`pilot-analysis`)

pub mod diagnostics;
pub mod infer;
pub mod pipeline;
pub mod types;

pub use diagnostics::{AnalysisDiagnostic, Diagnostic, DiagnosticSeverity};
pub use infer::{
    analyze_physics, analyze_physics_with_source, engine_intrinsic, flatten_ast,
    handle_engine_intrinsic, load_or_compile_analyzer_pilc, run_pilot_analyzer, CategoryRecord,
    PhysicsDiagnostic, PhysicsRegistry, RawAstArena, RawAstNode, RawAttr,
};
pub use pipeline::{AnalysisPipeline, AnalysisResult};
pub use types::{
    CapabilityBits, FieldOffset, HwScalarHeader, LayoutId, LayoutTable, PhysicalLayout,
    PhysicsGeometry,
};
