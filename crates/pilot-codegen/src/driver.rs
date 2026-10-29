//! Milestone 35.1: Unified Native AOT Build Driver (pilot build).

use std::path::PathBuf;
use crate::c_target::CTargetEmitter;
use crate::llvm::LlvmEmitter;
use crate::wasm::WasmEmitter;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BuildTarget {
    Native,
    Llvm,
    Wasm,
    C,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OptLevel {
    O0,
    O2,
    O3,
}

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub target: BuildTarget,
    pub opt_level: OptLevel,
    pub output_path: PathBuf,
    pub pgo_trace: Option<PathBuf>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            target: BuildTarget::Native,
            opt_level: OptLevel::O2,
            output_path: PathBuf::from("a.out"),
            pgo_trace: None,
        }
    }
}

pub struct BuildDriver;

impl BuildDriver {
    pub fn compile_source_to_artifact(
        module_name: &str,
        options: &BuildOptions,
    ) -> Result<String, String> {
        match options.target {
            BuildTarget::Llvm => {
                let mut emitter = LlvmEmitter::new();
                emitter.emit_module_header(module_name);
                emitter.emit_function("main", "i64", &[], &["ret i64 0"]);
                Ok(emitter.finish())
            }
            BuildTarget::C => {
                let mut emitter = CTargetEmitter::new();
                emitter.emit_header();
                emitter.emit_function("int", "main", &[], "  return 0;\n");
                Ok(emitter.finish())
            }
            BuildTarget::Wasm => {
                let mut emitter = WasmEmitter::new();
                Ok(emitter.emit_wat_module(module_name, 0))
            }
            BuildTarget::Native => {
                // Emits native LLVM IR as the intermediary for clang/lld invocation
                let mut emitter = LlvmEmitter::new();
                emitter.emit_module_header(module_name);
                emitter.emit_function("main", "i64", &[], &["ret i64 0"]);
                Ok(emitter.finish())
            }
        }
    }
}
