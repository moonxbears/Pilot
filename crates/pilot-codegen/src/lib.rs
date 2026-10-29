//! Milestone 35.1: Unified Native Codegen Backends (`pilot-codegen`).
//!
//! Emits optimized LLVM IR, C11/C23, and WebAssembly + WASI modules.

pub mod llvm;
pub mod c_target;
pub mod wasm;
pub mod asm;
pub mod driver;

pub use llvm::LlvmEmitter;
pub use c_target::CTargetEmitter;
pub use wasm::WasmEmitter;
pub use asm::AsmEmitter;
pub use driver::{BuildDriver, BuildOptions, BuildTarget, OptLevel};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llvm_emitter_module_header_and_function() {
        let mut emitter = LlvmEmitter::new();
        emitter.emit_module_header("calc_mod");
        emitter.emit_function("calc", "i64", &[("i64", "a"), ("i64", "b")], &["%res = add i64 %a, %b", "ret i64 %res"]);
        let ir = emitter.finish();
        assert!(ir.contains("source_filename = \"calc_mod.pilot\""));
        assert!(ir.contains("@pilot_rt_dup"));
        assert!(ir.contains("define i64 @calc(i64 %a, i64 %b)"));
        assert!(ir.contains("%res = add i64 %a, %b"));
    }

    #[test]
    fn test_c_target_emitter_generates_valid_c_code() {
        let mut emitter = CTargetEmitter::new();
        emitter.emit_header();
        emitter.emit_function("int64_t", "add_nums", &[("int64_t", "a"), ("int64_t", "b")], "  return a + b;");
        let code = emitter.finish();
        assert!(code.contains("#include \"pilot_rt.h\""));
        assert!(code.contains("int64_t add_nums(int64_t a, int64_t b) {"));
        assert!(code.contains("return a + b;"));
    }

    #[test]
    fn test_wasm_emitter_generates_wat_with_wasi() {
        let mut emitter = WasmEmitter::new();
        let wat = emitter.emit_wat_module("app", 42);
        assert!(wat.contains("(module ;; app"));
        assert!(wat.contains("wasi_snapshot_preview1"));
        assert!(wat.contains("i64.const 42"));
    }

    #[test]
    fn test_build_driver_compiles_all_targets() {
        let mut opts = BuildOptions::default();

        opts.target = BuildTarget::Llvm;
        let llvm_art = BuildDriver::compile_source_to_artifact("test", &opts).unwrap();
        assert!(llvm_art.contains("define i64 @main"));

        opts.target = BuildTarget::C;
        let c_art = BuildDriver::compile_source_to_artifact("test", &opts).unwrap();
        assert!(c_art.contains("int main("));

        opts.target = BuildTarget::Wasm;
        let wasm_art = BuildDriver::compile_source_to_artifact("test", &opts).unwrap();
        assert!(wasm_art.contains("(module"));

        opts.target = BuildTarget::Asm;
        let asm_art = BuildDriver::compile_source_to_artifact("test", &opts).unwrap();
        assert!(asm_art.contains(".file\t\"test.pilot\""));
    }

    #[test]
    fn test_asm_emitter_vector_synthesis() {
        let arena = pilot_parser::parse("def calc(a: int, b: int) -> int { return a + b }");
        let asm = AsmEmitter::emit_assembly(&arena, "calc_mod", 3, true).unwrap();
        assert!(asm.contains(".file\t\"calc_mod.pilot\""));
        assert!(asm.contains(".globl\tcalc"));
        assert!(asm.contains("AVX"));
    }

    #[test]
    fn test_wasm_emitter_bindings_and_binary() {
        let emitter = WasmEmitter::new();
        let wasm_bytes = emitter.assemble_minimal_wasm(42);
        assert_eq!(&wasm_bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]); // \0asm magic

        let (mjs, d_ts) = WasmEmitter::generate_bindings("pilot_app");
        assert!(mjs.contains("loadPilotModule"));
        assert!(d_ts.contains("export interface PilotModule"));
    }
}
