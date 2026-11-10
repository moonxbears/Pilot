//! ============================================================================
//! Milestone 10 Verification Suite:
//! Compiler-as-a-Library (`pilot.lex`, `pilot.cst`, `pilot.ast`, `pilot.types`,
//! `pilot.ir`, `pilot.fmt`, `pilot.db`, `pilot.vm`, `pilot.diag`)
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/vm_outcome.rs"]
mod vm_outcome;
#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 10,
    test_file_rel: "crates/pilot-analysis/tests/milestone10_compiler_as_a_library.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "stdlib/pilot/lex.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["tokenize", "token_cursor_has_more", "token_cursor_next"],
            required_code_markers: &["TokenCursor", "Token", "ByteSpan"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/pilot/cst.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["parse_lossless", "cst_editor_new", "cst_tree_root", "cst_editor_replace_span", "cst_editor_finish"],
            required_code_markers: &["CstTree", "CstNodeView", "CstEditor"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/pilot/ast.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["ast_module_new", "quote_builder_new", "ast_module_gensym", "ast_module_can_splice", "quote_builder_splice", "quote_builder_build"],
            required_code_markers: &["AstModule", "AstNodeView", "QuoteBuilder"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/pilot/types.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["type_info_new", "field_info_new"],
            required_code_markers: &["TypeInfo", "FieldInfo", "lattice_class_value"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/pilot/ir.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["ir_module_new", "ir_pass_pipeline_new", "ir_pass_pipeline_add_pass", "ir_pass_pipeline_run"],
            required_code_markers: &["IrModule", "IrPassPipeline"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/pilot/fmt.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["format_str", "str_replace"],
            required_code_markers: &["fmt_style_compact", "fmt_style_readable"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/pilot/db.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["compiler_session_new_in_memory", "compiler_session_open_workspace", "compiler_session_set_file", "compiler_session_get_file"],
            required_code_markers: &["CompilerSession"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/pilot/vm.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["vm_sandbox_new", "vm_config_default", "vm_sandbox_consume_fuel", "vm_sandbox_check_capability"],
            required_code_markers: &["VmSandbox", "VmConfig", "vm_trap_fuel_exhausted", "vm_trap_capability_denied"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/pilot/diag.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["diagnostic_new", "render_ansi", "render_json", "render_sarif", "diagnostic_is_error"],
            required_code_markers: &["Diagnostic", "FixSuggestion", "Span", "TextEdit"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m10(deps: &[&str], script: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let mut combined = String::new();
    for rel in deps {
        let p = root.join(rel);
        let src = fs::read_to_string(&p)
            .unwrap_or_else(|e| panic!("read {}: {:?}", p.display(), e));
        combined.push_str(&src);
        combined.push('\n');
    }
    combined.push_str(script);
    let arena = parse(&combined);
    let (flat, syms) = pilot_analysis::flatten_ast(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    let mut phy = PhysicsRegistry::new(flat, syms);
    let reg_ptr = &mut phy as *mut PhysicsRegistry;
    vm.engine_handler = Some(Box::new(move |op, a| {
        pilot_analysis::handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, a)
    }));
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    vm_outcome::outcome(&mut vm)
}

// -----------------------------------------------------------------------------
// Test 1: Streaming Tokenizer and CST Lossless Roundtrip (`pilot.lex`, `pilot.cst`)
// -----------------------------------------------------------------------------
#[test]
pub fn test_m10_1_streaming_tokenizer_and_cst_lossless_roundtrip() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m10_1_streaming_tokenizer_and_cst_lossless_roundtrip",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        // 1. Streaming Tokenizer test
        var cur = tokenize("let x = 42; // num\n");
        var tok_count: usize = 0;
        var saw_ident = false;
        var saw_comment = false;

        while token_cursor_has_more(cur) {
            let tok = token_cursor_next(var ref cur);
            tok_count = tok_count + 1;
            if tok.kind_id == token_kind_ident() && tok.text == "x" {
                saw_ident = true;
            }
            if tok.kind_id == token_kind_comment() {
                saw_comment = true;
            }
            if tok.kind_id == token_kind_eof() {
                break;
            }
        }

        let tok_ok = tok_count >= 5 && saw_ident && saw_comment;

        // 3. CST Editor with Trivia Preservation
        var editor = cst_editor_new("let count = 10; // counter");
        cst_editor_replace_span(var ref editor, 12, 14, "99");
        let edited = cst_editor_finish(editor);
        let edit_ok = edited == "let count = 99; // counter";

        ("" + (if tok_ok { "TOKENS_OK" } else { "TOKENS_FAIL" }) + "|" + (if edit_ok { "CST_EDIT_OK" } else { "EDIT_FAIL" }));
    "#;

    let res = eval_m10(&["stdlib/pilot/lex.pilot", "stdlib/pilot/cst.pilot"], script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 2, "Expected 2 parts, got: {}", res);
    assert_eq!(parts[0], "TOKENS_OK");
    assert_eq!(parts[1], "CST_EDIT_OK");

    let parsed = eval_m10(&["stdlib/pilot/cst.pilot"], r#"parse_lossless("fn compute() => 123;");"#);
    assert!(
        parsed.starts_with("FAULT: SEAM001: unconnected seam `pilot_cst.parse_lossless`"),
        "parse_lossless must stop at its seam, got: {}",
        parsed
    );

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: AST Quote Builder & Cross-Arena Splice Safety (`pilot.ast` with `AST001`)
// -----------------------------------------------------------------------------
#[test]
pub fn test_m10_2_ast_quote_builder_and_cross_arena_splice_safety() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m10_2_ast_quote_builder_and_cross_arena_splice_safety",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        // 1. Hygienic gensym test
        var mod1 = ast_module_new(101);
        let sym1 = ast_module_gensym(var ref mod1, "temp");
        let sym2 = ast_module_gensym(var ref mod1, "temp");
        let gensym_ok = sym1 == "temp_1" && sym2 == "temp_2";

        // 2. Cross-Arena Node Splice Protection (AST001)
        let node_same = ast_node_view_new(10, 101, 1, "var1");
        let node_diff = ast_node_view_new(10, 999, 2, "var2");

        let can_splice_same = ast_module_can_splice(mod1, node_same);
        let can_splice_diff = ast_module_can_splice(mod1, node_diff);

        // 3. Programmatic Quote Builder
        var qb = quote_builder_new();
        quote_builder_splice(var ref qb, "quote { a + b }");
        let q_res = quote_builder_build(qb);
        let quote_ok = q_res == "quote { a + b }";

        ("" + (if gensym_ok { "GENSYM_OK" } else { "GENSYM_FAIL" }) + "|" + (if can_splice_same { "SPLICE_SAME_OK" } else { "SPLICE_SAME_FAIL" }) + "|" + (if !can_splice_diff { "AST001_REJECT_DIFF_OK" } else { "AST001_FAIL" }) + "|" + (if quote_ok { "QUOTE_OK" } else { "QUOTE_FAIL" }));
    "#;

    let res = eval_m10(&["stdlib/pilot/ast.pilot"], script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 4, "Expected 4 parts, got: {}", res);
    assert_eq!(parts[0], "GENSYM_OK");
    assert_eq!(parts[1], "SPLICE_SAME_OK");
    assert_eq!(parts[2], "AST001_REJECT_DIFF_OK");
    assert_eq!(parts[3], "QUOTE_OK");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Type Reflection Lattice Classes & IR Pipeline (`pilot.types`, `pilot.ir`, `pilot.fmt`)
// -----------------------------------------------------------------------------
#[test]
pub fn test_m10_3_type_reflection_lattice_classes_and_ir_pipeline() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m10_3_type_reflection_lattice_classes_and_ir_pipeline",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        // 1. 2-Bit Lattice Class Verification
        let val_cls = lattice_class_value();
        let ref_cls = lattice_class_reference();
        let obj_cls = lattice_class_object();
        let grd_cls = lattice_class_guard();

        let t_val = type_info_new("Point", 16, 8, val_cls, 2);
        let t_obj = type_info_new("Buffer", 24, 8, obj_cls, 3);
        let lattice_ok = t_val.lattice_class == 0 && t_obj.lattice_class == 2 && ref_cls == 1 && grd_cls == 3;

        // 2. IR Pass Pipeline registration
        var pipe = ir_pass_pipeline_new();
        ir_pass_pipeline_add_pass(var ref pipe, "dce");
        ir_pass_pipeline_add_pass(var ref pipe, "const_fold");
        ir_pass_pipeline_add_pass(var ref pipe, "perceus");
        let ir_ok = ir_pass_pipeline_count(pipe) == 3;

        // 3. Formatter substring helper
        let replaced = str_replace("a!^(x) + b!^(y)", "!^(", "!(");
        let fmt_ok = replaced == "a!(x) + b!(y)";

        ("" + (if lattice_ok { "LATTICE_OK" } else { "LATTICE_FAIL" }) + "|" + (if ir_ok { "IR_OK" } else { "IR_FAIL" }) + "|" + (if fmt_ok { "FMT_OK" } else { "FMT_FAIL" }));
    "#;

    let res = eval_m10(&["stdlib/pilot/types.pilot", "stdlib/pilot/ir.pilot", "stdlib/pilot/fmt.pilot"], script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 3, "Expected 3 parts, got: {}", res);
    assert_eq!(parts[0], "LATTICE_OK");
    assert_eq!(parts[1], "IR_OK");
    assert_eq!(parts[2], "FMT_OK");

    let ran = eval_m10(
        &["stdlib/pilot/ir.pilot"],
        r#"var m = ir_module_new("kernel"); var p = ir_pass_pipeline_new(); ir_pass_pipeline_add_pass(var ref p, "dce"); ir_pass_pipeline_run(p, var ref m);"#,
    );
    assert!(
        ran.starts_with("FAULT: SEAM001: unconnected seam `pilot_ir.pipeline_run`"),
        "ir_pass_pipeline_run must stop at its seam, got: {}",
        ran
    );
    let formatted = eval_m10(&["stdlib/pilot/fmt.pilot"], r#"format_str("let t = f!^(a);", fmt_style_readable());"#);
    assert!(
        formatted.starts_with("FAULT: SEAM001: unconnected seam `pilot_fmt.format_str`"),
        "format_str must stop at its seam, got: {}",
        formatted
    );

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: In-Memory Compiler Session & VM Sandbox Capabilities (`pilot.db`, `pilot.vm`, `pilot.diag`)
// -----------------------------------------------------------------------------
#[test]
pub fn test_m10_4_in_memory_compiler_session_and_vm_sandbox_capabilities() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m10_4_in_memory_compiler_session_and_vm_sandbox_capabilities",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        // 1. In-Memory Compiler Session (pilot.db)
        var sess = compiler_session_new_in_memory();
        compiler_session_set_file(var ref sess, "main.pilot", "pub fn main() => 42;");
        compiler_session_set_file(var ref sess, "util.pilot", "pub fn helper() => 1;");
        let f_count = compiler_session_total_files(sess);
        let main_content = compiler_session_get_file(sess, "main.pilot");
        compiler_session_set_file(var ref sess, "main.pilot", "pub fn main() => 7;");
        let replaced = compiler_session_get_file(sess, "main.pilot");
        let sess_ok = sess.is_virtual && f_count == 2 && main_content == "pub fn main() => 42;" &&
            replaced == "pub fn main() => 7;" && compiler_session_total_files(sess) == 2 && sess.revision_number == 4;

        // 2. VM Sandbox & Capability Denial (VM004)
        let cfg = vm_config_default();
        var sandbox = vm_sandbox_new(cfg);
        let cap_ok = vm_sandbox_check_capability(var ref sandbox, cap_host_fs_read(), "HostFsRead");
        let vm_cap_denied_ok = !cap_ok && sandbox.is_trapped && sandbox.trap_code == vm_trap_capability_denied() && sandbox.trap_msg.find("VM004") >= 0;

        // 3. VM Sandbox Fuel Exhaustion (VM001)
        var fuel_cfg = vm_config_default();
        fuel_cfg.fuel_limit = 50;
        var fuel_sandbox = vm_sandbox_new(fuel_cfg);
        let consumed_ok = vm_sandbox_consume_fuel(var ref fuel_sandbox, 100);
        let vm_fuel_trap_ok = !consumed_ok && fuel_sandbox.is_trapped && fuel_sandbox.trap_code == vm_trap_fuel_exhausted() && fuel_sandbox.trap_msg.find("VM001") >= 0;

        // 4. Diagnostics & Reporting (pilot.diag)
        let sp = span_new(1, 5, 10);
        let diag = diagnostic_new("E010", diag_sev_error(), "Type mismatch", sp);
        let diag_is_err = diagnostic_is_error(diag);
        let json = render_json(diag);
        let json_ok = json.find("\"code\":\"E010\"") >= 0 && json.find("\"severity\":1") >= 0;

        ("" + (if sess_ok { "SESSION_OK" } else { "SESSION_FAIL" }) + "|" + (if vm_cap_denied_ok { "VM_CAP_DENIED_OK" } else { "VM_CAP_FAIL" }) + "|" + (if vm_fuel_trap_ok { "VM_FUEL_TRAP_OK" } else { "VM_FUEL_FAIL" }) + "|" + (if json_ok && diag_is_err { "DIAG_OK" } else { "DIAG_FAIL" }));
    "#;

    let res = eval_m10(&["stdlib/pilot/db.pilot", "stdlib/pilot/vm.pilot", "stdlib/pilot/diag.pilot"], script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 4, "Expected 4 parts, got: {}", res);
    assert_eq!(parts[0], "SESSION_OK");
    assert_eq!(parts[1], "VM_CAP_DENIED_OK");
    assert_eq!(parts[2], "VM_FUEL_TRAP_OK");
    assert_eq!(parts[3], "DIAG_OK");

    let session_seams = [
        (r#"var s = compiler_session_new_in_memory(); compiler_session_set_file(var ref s, "a.pilot", "fn a() => 1;"); compiler_session_check_all(s);"#, "pilot_db.check_all"),
        (r#"compiler_session_open_workspace(".");"#, "pilot_db.open_workspace"),
    ];
    for (call, id) in session_seams {
        let out = eval_m10(&["stdlib/pilot/db.pilot"], call);
        assert!(
            out.starts_with(&format!("FAULT: SEAM001: unconnected seam `{}`", id)),
            "must stop at seam {}, got: {}",
            id,
            out
        );
    }
    let sandboxed = eval_m10(
        &["stdlib/pilot/vm.pilot"],
        r#"var sb = vm_sandbox_new(vm_config_default()); vm_sandbox_run(var ref sb, "print(1);");"#,
    );
    assert!(
        sandboxed.starts_with("FAULT: SEAM001: unconnected seam `pilot_vm.sandbox_run`"),
        "vm_sandbox_run must stop at its seam, got: {}",
        sandboxed
    );

    guard.finish();
}
