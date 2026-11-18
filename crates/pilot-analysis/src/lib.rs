//! Legacy `RawAstArena` flattening and the `ast_*` / `layout_*` / `sym_*`
//! `__engine` hooks that `pilot-cli` serves to Pilot tools (`pilot-analysis`).
//! No analysis happens here any more; the checker is `selfhost/check/`. The
//! crate is deleted by `PROP-004` (roadmap D0).

pub mod infer;
pub mod types;

pub use infer::{
    engine_intrinsic, flatten_ast, handle_engine_intrinsic, CategoryRecord, PhysicsRegistry,
    RawAstArena, RawAstNode, RawAttr,
};
pub use types::{
    CapabilityBits, FieldOffset, HwScalarHeader, LayoutId, LayoutTable, PhysicalLayout,
    PhysicsGeometry,
};
