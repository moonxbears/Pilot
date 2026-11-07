//! Milestone 57 Verification Suite:
//! 7-Target Bidirectional Transpilers & Foreign Ingesters — Retire `pilot-transpile`.

use std::path::Path;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[test]
fn test_57_1_and_57_2_transpile_ir_and_7_language_emitters_execute_on_pilvm() {
    let ir = std::fs::read_to_string("../../selfhost/transpile/ir.pilot").unwrap();
    let em_rs = std::fs::read_to_string("../../selfhost/transpile/emit_rust.pilot").unwrap();
    let em_c = std::fs::read_to_string("../../selfhost/transpile/emit_c.pilot").unwrap();
    let em_cpp = std::fs::read_to_string("../../selfhost/transpile/emit_cpp.pilot").unwrap();
    let em_cs = std::fs::read_to_string("../../selfhost/transpile/emit_csharp.pilot").unwrap();
    let em_py = std::fs::read_to_string("../../selfhost/transpile/emit_python.pilot").unwrap();
    let em_js = std::fs::read_to_string("../../selfhost/transpile/emit_js.pilot").unwrap();
    let em_lua = std::fs::read_to_string("../../selfhost/transpile/emit_lua.pilot").unwrap();

    let script = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n\
        let ir_cat = classify_ir_physics(2, 64);\n\
        let ir_fn = create_ir_function(\"scale_val\", 2, 64, \"x\", 2, 64, \"x\", \"*\", \"2\");\n\
        let ir_fld = create_ir_struct_field(\"Point\", 0, \"x\", 4, 64);\n\
        let seed_ok = is_transpilable_seed_construct(nameof(struct));\n\
        let out_rs = emit_rust_function(ir_fn.fn_name, ir_fn.param_name, 2, 64, 2, 64, \"x * 2\");\n\
        let out_c = emit_c_function_source(ir_fn.fn_name, ir_fn.param_name, 2, 64, 2, 64, \"x * 2\");\n\
        let out_cpp = emit_cpp_function_source(ir_fn.fn_name, ir_fn.param_name, 2, 64, 2, 64, \"x * 2\");\n\
        let out_cs = emit_csharp_function_source(\"MathMod\", ir_fn.fn_name, ir_fn.param_name, 2, 64, 2, 64, \"x * 2\");\n\
        let out_py = emit_python_function_source(ir_fn.fn_name, ir_fn.param_name, \"x * 2\");\n\
        let out_js = emit_js_function_source(ir_fn.fn_name, ir_fn.param_name, \"x * 2\");\n\
        let out_lua = emit_lua_function_source(ir_fn.fn_name, ir_fn.param_name, \"x * 2\");\n\
        if ir_cat == 1 && ir_fld.bit_width == 64 && seed_ok && out_rs.source_code.len() > 20 && out_c.len() > 20 && out_cpp.len() > 20 && out_cs.len() > 20 && out_py.len() > 15 && out_js.len() > 15 && out_lua.len() > 15 {{\n\
            7\n\
        }} else {{\n\
            -1\n\
        }};\n",
        ir, em_rs, em_c, em_cpp, em_cs, em_py, em_js, em_lua
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "Unexpected parse diagnostics: {:?}", arena.diagnostics);
    let binary = BytecodeCompiler::compile(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(binary);
    let res = vm.run();
    assert_eq!(res.as_int(), Some(7));
}

#[test]
fn test_57_3_canonical_foreign_ingester_and_spdx_verifier_execute_on_pilvm() {
    let ingest = std::fs::read_to_string("../../selfhost/transpile/ingest.pilot").unwrap();
    let licensing = std::fs::read_to_string("../../stdlib/pkg/licensing.pilot").unwrap();

    let script = format!(
        "{}\n{}\n\
        let mit_ok = is_viable_spdx_license(\"MIT\");\n\
        let apache_ok = is_viable_spdx_license(\"Apache-2.0\");\n\
        let gpl_bad = !is_viable_spdx_license(\"GPL-3.0\");\n\
        let ing = ingest_foreign_function(\"python\", \"compute\", \"x: i64\", \"i64\", \"x + 1\");\n\
        let st_code = emit_canonical_pilot_struct(\"Vec2\", \"x\", \"f64\");\n\
        if mit_ok && apache_ok && gpl_bad && ing.is_canonical && ing.pilot_code.len() > 20 && st_code.len() > 30 {{\n\
            ing.pilot_code\n\
        }} else {{\n\
            \"\"\n\
        }};\n",
        ingest, licensing
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "Unexpected parse diagnostics: {:?}", arena.diagnostics);
    let binary = BytecodeCompiler::compile(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(binary);
    let res = vm.run().to_string_repr();
    assert!(res.contains("pub fn compute(x: i64) -> i64"));
    assert!(!res.contains("def "));
    assert!(!res.contains("class "));

    // Verify the ingested Pilot code parses cleanly with zero diagnostics
    let ingested_arena = parse(&res);
    assert!(ingested_arena.diagnostics.is_empty(), "Ingested canonical Pilot must parse cleanly");
}

#[test]
fn test_57_4_pilot_transpile_crate_retired() {
    assert!(
        !Path::new("../../crates/pilot-transpile").exists(),
        "crates/pilot-transpile must be completely removed from disk in Milestone 57.4"
    );
    let root_cargo = std::fs::read_to_string("../../Cargo.toml").unwrap();
    assert!(
        !root_cargo.contains("pilot-transpile"),
        "Workspace Cargo.toml must not reference pilot-transpile"
    );
}
