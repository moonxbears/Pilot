//! Native Static Analysis & Lossless `__engine` Bridge (`pilot-analysis`)

pub mod infer;
pub mod types;

pub use infer::{
    engine_intrinsic, flatten_ast, handle_engine_intrinsic, CategoryRecord, PhysicsDiagnostic,
    PhysicsRegistry, RawAstArena, RawAstNode, RawAttr,
};
pub use types::{
    CapabilityBits, FieldOffset, HwScalarHeader, LayoutId, LayoutTable, PhysicalLayout,
    PhysicsGeometry,
};
