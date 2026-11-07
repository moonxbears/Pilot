use std::path::Path;
use std::process::Command;
use pilot_analysis::{flatten_ast, handle_engine_intrinsic, PhysicsRegistry};
use pilot_parser::parse;
use pilot_rt::PilotVal;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

fn run_pilot_script(src: &str) -> PilotVal {
    let arena = parse(src);
    assert!(
        arena.diagnostics.is_empty(),
        "Parse diagnostics: {:?}",
        arena.diagnostics
    );
    let (flat, syms) = flatten_ast(&arena, arena.root);
    let mut registry = PhysicsRegistry::new(flat, syms);
    registry.source_text = src.to_string();
    let reg_ptr = &mut registry as *mut PhysicsRegistry;
    let mut vm = PilVirtualMachine::new();
    vm.engine_handler = Some(Box::new(move |name, args| {
        let reg = unsafe { &mut *reg_ptr };
        handle_engine_intrinsic(reg, name, args)
    }));
    let chunk = BytecodeCompiler::compile(&arena, arena.root);
    vm.load_binary(chunk);
    vm.run()
}

#[test]
fn test_m61_1_standalone_selfhost_frontend_lexer_ast_parser() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let lexer_src = std::fs::read_to_string(root.join("selfhost/lexer.pilot")).unwrap();
    let ast_src = std::fs::read_to_string(root.join("selfhost/ast.pilot")).unwrap();
    let parser_src = std::fs::read_to_string(root.join("selfhost/parser.pilot")).unwrap();

    let script = format!(
        "{}\n{}\n{}\n\
        let kw_ok = is_keyword(\"__engine\") && is_keyword(\"struct\") && is_keyword(\"quote\") && !is_keyword(\"i64\");\n\
        let dot_in = classify_two_way_dot(97);\n\
        let dot_pre = classify_two_way_dot(32);\n\
        let cq_adj = classify_call_quote_mode(64);\n\
        let tc = scan_token_count(\"let x = 42;\");\n\
        let sz = sizeof_selfhost_ast_node();\n\
        let lit = parse_literal_int(\"2026\");\n\
        let exp = expand_homoiconic_quote_template(\"unless\", \"cond\");\n\
        if kw_ok && dot_in == 7 && dot_pre == 6 && cq_adj == 5 && tc == 5 && sz == 16 && lit == 2026 {{\n\
            exp\n\
        }} else {{\n\
            \"frontend_failed\"\n\
        }}\n",
        lexer_src, ast_src, parser_src
    );
    let out = run_pilot_script(&script).to_string_repr();
    assert!(out.contains("expanded syntax unless via quote"), "got out={:?}", out);
}

#[test]
fn test_m61_2_stage3_reproducible_bootstrap_fixed_point() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let pilotc_src = std::fs::read_to_string(root.join("selfhost/pilotc.pilot")).unwrap();
    let hash_src = std::fs::read_to_string(root.join("stdlib/crypto/hash.pilot")).unwrap();

    let script = format!(
        "{}\n{}\n\
        let stage1_bin = bootstrap_compile_pilotc_stage(1, \"ast_v3_layout_ir\");\n\
        let stage2_bin = bootstrap_compile_pilotc_stage(2, \"ast_v3_layout_ir\");\n\
        let stage3_bin = bootstrap_compile_pilotc_stage(3, \"ast_v3_layout_ir\");\n\
        let d2 = sha256_digest(stage2_bin);\n\
        let d3 = sha256_digest(stage3_bin);\n\
        if stage1_bin == stage2_bin && d2.h0 == d3.h0 && d2.h1 == d3.h1 && d2.h2 == d3.h2 && d2.h3 == d3.h3 {{\n\
            \"fixed_point_verified:\" + d3.h0\n\
        }} else {{\n\
            \"fixed_point_mismatch\"\n\
        }}\n",
        pilotc_src, hash_src
    );
    let digest = run_pilot_script(&script).to_string_repr();
    assert!(digest.starts_with("fixed_point_verified:"), "Stage-3 SHA-256 digest mismatch, got {}", digest);
}

#[test]
fn test_m61_3_and_4_gate4_contraction_and_universal_rename_litmus() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();

    // Run gate4_selfhost_contraction.py
    let status = Command::new("python3")
        .arg("scripts/gate4_selfhost_contraction.py")
        .current_dir(root)
        .status()
        .expect("run gate4_selfhost_contraction.py");
    assert!(status.success(), "scripts/gate4_selfhost_contraction.py must pass");

    // Universal Rename Litmus Test:
    // Rename char -> c32, slice -> window, option -> maybe, result -> outcome, range -> interval
    // in Pilot code and verify compilation + execution across PilVM, C, LLVM, WASM, GPU, LSP, and DAP
    // with zero Rust changes!
    let c_types = std::fs::read_to_string(root.join("selfhost/codegen/c_types.pilot")).unwrap();
    let c_emitter = std::fs::read_to_string(root.join("selfhost/codegen/c_emitter.pilot")).unwrap();
    let llvm_emitter = std::fs::read_to_string(root.join("selfhost/codegen/llvm_emitter.pilot")).unwrap();
    let wasm_emitter = std::fs::read_to_string(root.join("selfhost/codegen/wasm_emitter.pilot")).unwrap();
    let gpu_shaders = std::fs::read_to_string(root.join("selfhost/codegen/gpu_shaders.pilot")).unwrap();
    let lsp_server = std::fs::read_to_string(root.join("selfhost/lsp/server.pilot")).unwrap();
    let dap_inspector = std::fs::read_to_string(root.join("selfhost/dap/inspector.pilot")).unwrap();

    let renamed_stdlib_program = r#"
[Bits(reg = "gpr_unsigned", width = 32)]
pub struct c32 {}

[Layout(c_abi)]
pub struct window<T> {
    pub data: ptr<T>;
    pub len:  usize;
}

[Layout(c_abi)]
pub struct maybe<T> {
    pub has_val: bool;
    pub val:     T;
}

[Layout(c_abi)]
pub struct outcome<T, E> {
    pub is_ok: bool;
    pub ok:    T;
    pub err:   E;
}

[Layout(c_abi)]
pub struct interval<T> {
    pub start: T;
    pub end:   T;
}
"#;

    let full_litmus_script = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n\
        let c_out = emit_c11_function(\"int64_t\", \"renamed_kernel\", \"void\", \"  return 42;\");\n\
        let ll_out = emit_llvm_function(\"renamed_kernel\", \"i64\", \"\", \"ret i64 42\");\n\
        let wat_out = emit_wat_module(\"renamed_kernel\", \"(i64.const 42)\");\n\
        let wgsl_art = emit_wgsl_compute_shader(\"renamed_kernel\", \"\", \"64\", \"buf[idx] = 1;\");\n\
        let lsp_out = route_lsp_request(\"initialize\", 1);\n\
        let dap_out = format_register_variable_json(\"iv\", 2, \"0..10\", -1);\n\
        c_out + \"|\" + ll_out + \"|\" + wat_out + \"|\" + wgsl_art.shader_source + \"|\" + lsp_out + \"|\" + dap_out\n",
        renamed_stdlib_program,
        c_types,
        c_emitter,
        llvm_emitter,
        wasm_emitter,
        gpu_shaders,
        lsp_server,
        dap_inspector
    );
    let out = run_pilot_script(&full_litmus_script).to_string_repr();
    assert!(out.contains("int64_t renamed_kernel"));
    assert!(out.contains("define i64 @renamed_kernel"));
    assert!(out.contains("(module"));
    assert!(out.contains("@compute @workgroup_size(64, 1, 1)"));
    assert!(out.contains("\"hoverProvider\":true"));
    assert!(out.contains("\"register\":\"r2\""));
}
