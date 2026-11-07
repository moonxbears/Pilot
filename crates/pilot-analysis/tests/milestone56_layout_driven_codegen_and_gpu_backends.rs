//! Milestone 56 Verification Suite:
//! Layout-Driven Native C, LLVM, WASM, ASM & Real GPU Codegen (`pilot-codegen` <= 120 LOC).

use std::path::Path;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[test]
fn test_56_1_layout_driven_c11_backend_executes_on_pilvm() {
    let physics = std::fs::read_to_string("../../selfhost/codegen/physics.pilot").unwrap();
    let c_types = std::fs::read_to_string("../../selfhost/codegen/c_types.pilot").unwrap();
    let c_header = std::fs::read_to_string("../../selfhost/codegen/c_header.pilot").unwrap();
    let c_emitter = std::fs::read_to_string("../../selfhost/codegen/c_emitter.pilot").unwrap();

    let script = format!(
        "{}\n{}\n{}\n{}\n\
        let lay_i64 = create_layout_descriptor(10, reg_gpr_signed(), 64, 1, 0, 0);\n\
        let sz = compute_byte_size(lay_i64.bit_width, lay_i64.repeat_count);\n\
        let al = compute_byte_align(lay_i64.bit_width);\n\
        let c_map = map_layout_to_c11(10, reg_gpr_signed(), 64);\n\
        let view_td = deduce_structural_c_typedef(2, 1);\n\
        let buf_td = deduce_structural_c_typedef(3, 2);\n\
        let hdr = emit_c11_runtime_header();\n\
        let hdr_ok = verify_c11_header_abi(hdr);\n\
        let fn_c = emit_c11_function(c_map.c_type_name, \"compute\", \"int64_t x\", \"  return (x + 1);\");\n\
        let mod_art = build_c11_module(\"math_mod\", fn_c, \"int64_t r = compute(41);\", 1, true);\n\
        if sz == 8 && al == 8 && c_map.c_type_name == \"int64_t\" && hdr_ok && view_td.len() > 20 && buf_td.len() > 20 {{\n\
            mod_art.fn_count + sz + al + c_map.byte_size\n\
        }} else {{\n\
            -1\n\
        }};\n",
        physics, c_types, c_header, c_emitter
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "Unexpected parse diagnostics: {:?}", arena.diagnostics);
    let binary = BytecodeCompiler::compile(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(binary);
    let res = vm.run();
    // 1 + 8 + 8 + 8 = 25
    assert_eq!(res.as_int(), Some(25));
}

#[test]
fn test_56_2_ast_lowering_gpu_backends_execute_on_pilvm() {
    let gpu_c = std::fs::read_to_string("../../selfhost/codegen/gpu_c_dialects.pilot").unwrap();
    let gpu_sh = std::fs::read_to_string("../../selfhost/codegen/gpu_shaders.pilot").unwrap();
    let gpu_ll = std::fs::read_to_string("../../selfhost/codegen/gpu_llvm.pilot").unwrap();

    let script = format!(
        "{}\n{}\n{}\n\
        let cuda_k = emit_cuda_kernel(\"saxpy\", \"float* x, float* y, float a, int64_t n\", \"n\", \"y[idx] = a * x[idx] + y[idx];\");\n\
        let hip_k = emit_hip_kernel(\"saxpy\", \"float* x, float* y, float a, int64_t n\", \"n\", \"y[idx] = a * x[idx] + y[idx];\");\n\
        let cl_k = emit_opencl_kernel(\"saxpy\", \"__global float* x, __global float* y, float a, long n\", \"n\", \"y[idx] = a * x[idx] + y[idx];\");\n\
        let omp_k = emit_openmp_kernel(\"saxpy\", \"float* x, float* y, float a, int64_t n\", \"n\", \"y[idx] = a * x[idx] + y[idx];\");\n\
        let wgsl_k = emit_wgsl_compute_shader(\"saxpy\", \"@group(0) @binding(0) var<storage, read_write> y: array<f32>;\", \"1024\", \"y[gid.x] = y[gid.x] * 2.0;\");\n\
        let spv_k = emit_spirv_compute_module(\"saxpy\", \"%val = OpFMul %float %x %a\\nOpStore %ptr %val\");\n\
        let ptx_k = emit_nvptx_llvm_ir_module(\"saxpy\", \"ptr %x, ptr %y\", \"%v = fadd float %0, %1\");\n\
        let gcn_k = emit_amdgpu_llvm_ir_module(\"saxpy\", \"ptr %x, ptr %y\", \"%v = fadd float %0, %1\");\n\
        if cuda_k.source_code.len() > 80 && hip_k.source_code.len() > 80 && cl_k.source_code.len() > 60 && omp_k.source_code.len() > 80 && wgsl_k.shader_source.len() > 80 && spv_k.shader_source.len() > 80 && ptx_k.llvm_ir.len() > 100 && gcn_k.llvm_ir.len() > 100 {{\n\
            8\n\
        }} else {{\n\
            -1\n\
        }};\n",
        gpu_c, gpu_sh, gpu_ll
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "Unexpected parse diagnostics: {:?}", arena.diagnostics);
    let binary = BytecodeCompiler::compile(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(binary);
    let res = vm.run();
    assert_eq!(res.as_int(), Some(8));
}

#[test]
fn test_56_3_llvm_wasm_and_asm_emitters_execute_on_pilvm() {
    let llvm_em = std::fs::read_to_string("../../selfhost/codegen/llvm_emitter.pilot").unwrap();
    let wasm_em = std::fs::read_to_string("../../selfhost/codegen/wasm_emitter.pilot").unwrap();
    let asm_em = std::fs::read_to_string("../../selfhost/codegen/asm_emitter.pilot").unwrap();

    let script = format!(
        "{}\n{}\n{}\n\
        let ll_ty = llvm_type_for_physics(2, 64);\n\
        let ll_fn = emit_llvm_function(\"add_vals\", ll_ty, \"i64 %a, i64 %b\", \"%r = add i64 %a, %b\\n  ret i64 %r\");\n\
        let ll_mod = build_llvm_module(\"calc\", ll_fn, 1);\n\
        let w_ty = wasm_type_for_physics(4, 64);\n\
        let w_bundle = build_wasm_bundle(\"calc\", \"i64.const 42\");\n\
        let asm_mod = build_asm_module(\"calc\", \"add_vals\", \"vaddps %ymm0, %ymm1, %ymm2\", true);\n\
        if ll_ty == \"i64\" && w_ty == \"f64\" && ll_mod.ir_text.len() > 60 && w_bundle.wat_source.len() > 60 && w_bundle.mjs_bindings.len() > 40 && w_bundle.dts_bindings.len() > 40 && asm_mod.uses_avx2_simd {{\n\
            w_bundle.magic_word\n\
        }} else {{\n\
            -1\n\
        }};\n",
        llvm_em, wasm_em, asm_em
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "Unexpected parse diagnostics: {:?}", arena.diagnostics);
    let binary = BytecodeCompiler::compile(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(binary);
    let res = vm.run();
    assert_eq!(res.as_int(), Some(1836278016));
}

#[test]
fn test_56_4_legacy_rust_codegen_files_deleted_and_line_caps_enforced() {
    for deleted in [
        "../../crates/pilot-codegen/src/asm.rs",
        "../../crates/pilot-codegen/src/c_target.rs",
        "../../crates/pilot-codegen/src/driver.rs",
        "../../crates/pilot-codegen/src/gpu.rs",
        "../../crates/pilot-codegen/src/llvm.rs",
        "../../crates/pilot-codegen/src/physics.rs",
        "../../crates/pilot-codegen/src/wasm.rs",
        "../../selfhost/emitter.pilot",
    ] {
        assert!(!Path::new(deleted).exists(), "Legacy file {} must be deleted in Milestone 56.4", deleted);
    }

    let codegen_lib = std::fs::read_to_string("../../crates/pilot-codegen/src/lib.rs").unwrap();
    let codegen_lines = codegen_lib.lines().count();
    assert!(
        codegen_lines <= 120,
        "crates/pilot-codegen/src/lib.rs ({}) must be <= 120 lines",
        codegen_lines
    );

    let c_abi = std::fs::read_to_string("../../crates/pilot-rt/src/c_abi.rs").unwrap();
    let c_abi_lines = c_abi.lines().count();
    assert!(
        c_abi_lines <= 220,
        "crates/pilot-rt/src/c_abi.rs ({}) must have its 955L hardcoded C11 header purged (<= 220 lines)",
        c_abi_lines
    );
}
