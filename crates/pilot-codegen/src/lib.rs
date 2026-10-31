//! Milestone 35.1 & Milestone 47: Unified Native & GPU Codegen Backends (`pilot-codegen`).
//!
//! Emits optimized LLVM IR, C11/C23, WebAssembly + WASI modules, and multi-approach GPU kernels (CUDA, ROCm/HIP, Vulkan/SPIR-V, WGSL).

pub mod llvm;
pub mod c_target;
pub mod wasm;
pub mod asm;
pub mod driver;
pub mod gpu;

pub use llvm::LlvmEmitter;
pub use c_target::CTargetEmitter;
pub use wasm::WasmEmitter;
pub use asm::AsmEmitter;
pub use driver::{BuildDriver, BuildOptions, BuildTarget, OptLevel};
pub use gpu::{GpuBackend, GpuEmitter, GpuVendor};

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
    fn test_c_target_builtin_member_methods() {
        let src = r#"
let s = "hello world";
let parts = s.split(" ");
let joined = parts.join("-");
let rep = s.replace("world", "pilot");
let t = s.trim();
let up = s.to_upper();
let low = s.to_lower();
let sw = s.starts_with("he");
let ew = s.ends_with("ld");
let c = s.contains("ll");
let idx = s.index_of("w");
let sub = s.substring(0, 5);
let rep_s = s.repeat(2);
let is_d = s.is_digit();
let is_a = s.is_alpha();

let items = ["a", "b", "c"];
items.push("d");
let popped = items.pop();
items.reverse();
let f = items.first();
let l = items.last();

let num = -42;
let a_val = num.abs();
let cl = num.clamp(0, 100);
let str_val = num.to_string();

let fl = 3.7;
let fl_floor = fl.floor();
let fl_ceil = fl.ceil();
let fl_round = fl.round();
"#;
        let arena = pilot_parser::parse(src);
        let analysis = pilot_analysis::pipeline::AnalysisPipeline::analyze(&arena, arena.root);
        let c_code = CTargetEmitter::emit_ast_module(&arena, &analysis, src);

        assert!(c_code.contains("pilot_rt_str_split"));
        assert!(c_code.contains("pilot_rt_list_join"));
        assert!(c_code.contains("pilot_rt_str_replace"));
        assert!(c_code.contains("pilot_rt_str_trim"));
        assert!(c_code.contains("pilot_rt_str_to_upper"));
        assert!(c_code.contains("pilot_rt_str_to_lower"));
        assert!(c_code.contains("pilot_rt_str_starts_with"));
        assert!(c_code.contains("pilot_rt_str_ends_with"));
        assert!(c_code.contains("pilot_rt_str_contains"));
        assert!(c_code.contains("pilot_rt_str_index_of"));
        assert!(c_code.contains("pilot_rt_str_substring"));
        assert!(c_code.contains("pilot_rt_str_repeat"));
        assert!(c_code.contains("pilot_rt_str_is_digit"));
        assert!(c_code.contains("pilot_rt_str_is_alpha"));
        assert!(c_code.contains("pilot_rt_list_push"));
        assert!(c_code.contains("pilot_rt_list_pop"));
        assert!(c_code.contains("pilot_rt_list_reverse"));
        assert!(c_code.contains("pilot_rt_list_first"));
        assert!(c_code.contains("pilot_rt_list_last"));
        assert!(c_code.contains("pilot_rt_int_abs"));
        assert!(c_code.contains("pilot_rt_int_clamp"));
        assert!(c_code.contains("pilot_rt_int_to_str"));
        assert!(c_code.contains("floor"));
        assert!(c_code.contains("ceil"));
        assert!(c_code.contains("round"));
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
        assert!(asm.contains("calc"));

        let synth = AsmEmitter::synthesize_vector_assembly("define i64 @calc(i64 %a, i64 %b)", "calc_mod", 3, true);
        assert!(synth.contains("AVX"));
        assert!(synth.contains(".file\t\"calc_mod.pilot\""));
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

    #[test]
    fn test_gpu_emitters_all_three_approaches() {
        let arena = pilot_parser::parse("
@kernel
def vec_add(a: *float, b: *float, c: *float, n: int) {
    let idx = gpu_global_id()
    if idx < n {
        c[idx] = a[idx] + b[idx]
    }
}
");
        // Approach A: CUDA, HIP, OpenCL, OpenMP
        let cuda_code = GpuEmitter::emit_cuda(&arena, "vec_add");
        assert!(cuda_code.contains("__global__ void vec_add"));
        assert!(cuda_code.contains("blockIdx.x * blockDim.x + threadIdx.x"));
        assert!(cuda_code.contains("cudaMalloc"));

        let hip_code = GpuEmitter::emit_hip(&arena, "vec_add");
        assert!(hip_code.contains("__global__ void vec_add"));
        assert!(hip_code.contains("hipBlockIdx_x * hipBlockDim_x + hipThreadIdx_x"));
        assert!(hip_code.contains("hipMalloc"));

        let cl_code = GpuEmitter::emit_opencl(&arena, "vec_add");
        assert!(cl_code.contains("__kernel void vec_add"));
        assert!(cl_code.contains("get_global_id(0)"));

        let omp_code = GpuEmitter::emit_openmp(&arena, "vec_add");
        assert!(omp_code.contains("#pragma omp target teams distribute parallel for simd"));

        // Approach B: WebGPU WGSL & Vulkan SPIR-V
        let wgsl_code = GpuEmitter::emit_wgsl(&arena, "vec_add");
        assert!(wgsl_code.contains("@compute @workgroup_size(64, 1, 1)"));
        assert!(wgsl_code.contains("fn vec_add(@builtin(global_invocation_id)"));

        let spirv_code = GpuEmitter::emit_spirv(&arena, "vec_add");
        assert!(spirv_code.contains("OpCapability Shader"));
        assert!(spirv_code.contains("OpEntryPoint GLCompute"));

        // Approach C: Direct LLVM IR for NVPTX and AMDGPU
        let nvptx_ir = GpuEmitter::emit_nvptx_llvm_ir(&arena, "vec_add");
        assert!(nvptx_ir.contains("target triple = \"nvptx64-nvidia-cuda\""));
        assert!(nvptx_ir.contains("!nvvm.annotations"));
        assert!(nvptx_ir.contains("@llvm.nvvm.read.ptx.sreg.tid.x()"));

        let amdgpu_ir = GpuEmitter::emit_amdgpu_llvm_ir(&arena, "vec_add");
        assert!(amdgpu_ir.contains("target triple = \"amdgcn-amd-amdhsa\""));
        assert!(amdgpu_ir.contains("define amdgpu_kernel void @vec_add"));
        assert!(amdgpu_ir.contains("@llvm.amdgcn.workitem.id.x()"));
    }
}
