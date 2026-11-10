//! ============================================================================
//! Milestone 4 Verification Suite:
//! Dual Readable/Symbolic Syntax Pairings & Bidirectional Formatter
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_ast::{CallQuoteKind, NodeId, NodeKind, UNARY_OP_DEEP_FLATTEN, UNARY_OP_DOUBLE_SPLAT, UNARY_OP_SPLAT};
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/vm_outcome.rs"]
mod vm_outcome;
#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 4,
    test_file_rel: "crates/pilot-analysis/tests/milestone04_dual_syntax_pairings_and_bidirectional_formatter.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "stdlib/core/keywords.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[
                "CatCallQuote",
                "CatOpSplatUnpack",
                "call",
                "splice",
                "frame",
                "sig",
                "grad",
                "plan",
                "splat",
                "pairs",
                "flat",
                "opt",
                "res",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/core/syntax/call_quote.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[
                "CallExpr",
                "Signal",
                "Dual",
                "EffectPlan",
                "pub syntax",
                "call(",
                "splice(",
                "frame(",
                "sig(",
                "grad(",
                "plan(",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/core/syntax/expr_sugar.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[
                "pub syntax",
                "splat(",
                "pairs(",
                "flat(",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/core/syntax/type_construct.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[
                "pub syntax type_construct",
                "(opt<T>)",
                "(res<T, E>)",
                "(sig<T>)",
                "(grad<T, G>)",
                "(plan<T>)",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/fmt/fmt.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "format_pilot_with_width",
                "format_pilot_source",
                "format_pilot_with_style",
                "fmt_transform_stream_style",
            ],
            required_code_markers: &[
                "FmtStyle",
                "preserve",
                "readable",
                "compact",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ide/vscode_gen.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "reflect_category_keywords",
                "reflect_paired_symbol_for_keyword",
                "generate_hover_markdown_for_keyword",
            ],
            required_code_markers: &[
                "reflect_category_keywords",
                "reflect_paired_symbol_for_keyword",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_fmt(script: &str) -> String {
    let root = anticheat::workspace_root();
    let deps = [
        "stdlib/core/primitives/map.pilot",
        "selfhost/fmt/fmt.pilot",
    ];
    let mut combined = String::new();
    for rel in &deps {
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

/// Runs `src` on the VM alone and returns its outcome.
fn eval_program(src: &str) -> String {
    let arena = parse(src);
    assert!(arena.diagnostics.is_empty(), "parse diagnostics: {:?}", arena.diagnostics);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    vm_outcome::outcome(&mut vm)
}

// -----------------------------------------------------------------------------
// Test 1: Identifier Non-Collision Proof
// -----------------------------------------------------------------------------
#[test]
pub fn test_m4_1_identifier_non_collision_and_prefix_projection() {
    let guard = anticheat::AntiCheatGuard::arm("test_m4_1_identifier_non_collision_and_prefix_projection", &ANTICHEAT_SPEC);

    // Program defining identifiers named after projection keywords, alongside prefix calls
    let src = r#"
        fn compute(x: i64) -> i64 => x * 2;
        fn run_test() -> i64 {
            let opt = 1;
            var call = 2;
            let sig = 3;
            let grad = 4;
            let plan = 5;
            let flat = 6;
            let s1 = sig compute(opt);
            let s2 = call compute(call);
            let s3 = grad compute(sig);
            let s4 = plan compute(grad);
            return opt + call + sig + grad + plan + flat;
        }
    "#;

    let arena = parse(src);
    assert!(arena.diagnostics.is_empty(), "Expected 0 diagnostics, got: {:?}", arena.diagnostics);

    // Verify s1 is CallExpr with ReactiveSignal (kind 4)
    let s1_call = (0..arena.nodes.len())
        .map(|i| NodeId(i as u32))
        .find(|&id| {
            let n = arena.node(id);
            n.node_kind() == NodeKind::CallExpr && n.flags == CallQuoteKind::ReactiveSignal as u16
        });
    assert!(s1_call.is_some(), "Expected CallExpr with ReactiveSignal flags for `sig compute(opt)`");

    // Verify s2 is CallExpr with ShallowQuote (kind 1)
    let s2_call = (0..arena.nodes.len())
        .map(|i| NodeId(i as u32))
        .find(|&id| {
            let n = arena.node(id);
            n.node_kind() == NodeKind::CallExpr && n.flags == CallQuoteKind::ShallowQuote as u16
        });
    assert!(s2_call.is_some(), "Expected CallExpr with ShallowQuote flags for `call compute(call)`");

    // Verify s3 is CallExpr with SymbolicAdjoint (kind 5)
    let s3_call = (0..arena.nodes.len())
        .map(|i| NodeId(i as u32))
        .find(|&id| {
            let n = arena.node(id);
            n.node_kind() == NodeKind::CallExpr && n.flags == CallQuoteKind::SymbolicAdjoint as u16
        });
    assert!(s3_call.is_some(), "Expected CallExpr with SymbolicAdjoint flags for `grad compute(sig)`");

    // Verify s4 is CallExpr with EffectDryRun (kind 6)
    let s4_call = (0..arena.nodes.len())
        .map(|i| NodeId(i as u32))
        .find(|&id| {
            let n = arena.node(id);
            n.node_kind() == NodeKind::CallExpr && n.flags == CallQuoteKind::EffectDryRun as u16
        });
    assert!(s4_call.is_some(), "Expected CallExpr with EffectDryRun flags for `plan compute(grad)`");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Dual Syntax Equivalence & Identical Codegen
// -----------------------------------------------------------------------------
#[test]
pub fn test_m4_2_dual_syntax_equivalence_and_identical_codegen() {
    let guard = anticheat::AntiCheatGuard::arm("test_m4_2_dual_syntax_equivalence_and_identical_codegen", &ANTICHEAT_SPEC);

    // Compact symbolic syntax
    let compact_src = r#"
        fn compute(x: i64) -> i64 => x + 1;
        fn test_compact(v: i64) {
            let q1 = compute!(v);
            let q2 = compute!!(~v);
            let q3 = compute!&(v);
            let q4 = compute!^(v);
            let q5 = compute!@(v);
            let q6 = compute!?(v);
            let u1 = *v;
            let u2 = **v;
            let u3 = *..v;
        }
    "#;

    // Readable lowercase prefix syntax
    let readable_src = r#"
        fn compute(x: i64) -> i64 => x + 1;
        fn test_compact(v: i64) {
            let q1 = call compute(v);
            let q2 = splice compute(~v);
            let q3 = frame compute(v);
            let q4 = sig compute(v);
            let q5 = grad compute(v);
            let q6 = plan compute(v);
            let u1 = splat(v);
            let u2 = pairs(v);
            let u3 = flat(v);
        }
    "#;

    let arena_compact = parse(compact_src);
    let arena_readable = parse(readable_src);

    assert!(arena_compact.diagnostics.is_empty(), "Compact diag: {:?}", arena_compact.diagnostics);
    assert!(arena_readable.diagnostics.is_empty(), "Readable diag: {:?}", arena_readable.diagnostics);

    // Verify all 6 call quote flags match exactly
    let kinds = [
        CallQuoteKind::ShallowQuote as u16,
        CallQuoteKind::DeepSplice as u16,
        CallQuoteKind::AbiFrame as u16,
        CallQuoteKind::ReactiveSignal as u16,
        CallQuoteKind::SymbolicAdjoint as u16,
        CallQuoteKind::EffectDryRun as u16,
    ];
    for &k in &kinds {
        let count_compact = arena_compact.nodes.iter().filter(|n| n.node_kind() == NodeKind::CallExpr && n.flags == k).count();
        let count_readable = arena_readable.nodes.iter().filter(|n| n.node_kind() == NodeKind::CallExpr && n.flags == k).count();
        assert_eq!(count_compact, 1, "Expected 1 CallExpr with flag {} in compact", k);
        assert_eq!(count_readable, 1, "Expected 1 CallExpr with flag {} in readable", k);
    }

    // Verify unpack operators match exactly
    let un_splat_c = arena_compact.nodes.iter().filter(|n| n.node_kind() == NodeKind::UnaryExpr && n.flags == UNARY_OP_SPLAT).count();
    let un_splat_r = arena_readable.nodes.iter().filter(|n| n.node_kind() == NodeKind::UnaryExpr && n.flags == UNARY_OP_SPLAT).count();
    assert_eq!(un_splat_c, un_splat_r, "Splat unary expr mismatch");

    let un_pairs_c = arena_compact.nodes.iter().filter(|n| n.node_kind() == NodeKind::UnaryExpr && n.flags == UNARY_OP_DOUBLE_SPLAT).count();
    let un_pairs_r = arena_readable.nodes.iter().filter(|n| n.node_kind() == NodeKind::UnaryExpr && n.flags == UNARY_OP_DOUBLE_SPLAT).count();
    assert_eq!(un_pairs_c, un_pairs_r, "Pairs unary expr mismatch");

    let un_flat_c = arena_compact.nodes.iter().filter(|n| n.node_kind() == NodeKind::UnaryExpr && n.flags == UNARY_OP_DEEP_FLATTEN).count();
    let un_flat_r = arena_readable.nodes.iter().filter(|n| n.node_kind() == NodeKind::UnaryExpr && n.flags == UNARY_OP_DEEP_FLATTEN).count();
    assert_eq!(un_flat_c, un_flat_r, "Flat unary expr mismatch");

    // A plain call runs; every call-quote mode, in either spelling, stops at
    // the `vm.call_quote` seam instead of silently running as a plain call.
    let plain = eval_program("fn compute(x: i64) -> i64 => x + 1;\nreturn compute(41);\n");
    assert_eq!(plain, "42", "a plain call must run, got: {plain}");
    let forms = [
        "compute!(41)", "compute!!(41)", "compute!&(41)", "compute!^(41)", "compute!@(41)", "compute!?(41)",
        "call compute(41)", "splice compute(41)", "frame compute(41)", "sig compute(41)", "grad compute(41)",
        "plan compute(41)",
    ];
    for form in forms {
        let out = eval_program(&format!("fn compute(x: i64) -> i64 => x + 1;\nreturn {form};\n"));
        assert!(
            out.starts_with("FAULT: SEAM001: unconnected seam `vm.call_quote`"),
            "`{form}` must stop at the vm.call_quote seam, got: {out}"
        );
    }

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Bidirectional Formatter Styles
// -----------------------------------------------------------------------------
#[test]
pub fn test_m4_3_bidirectional_formatter_styles() {
    let guard = anticheat::AntiCheatGuard::arm("test_m4_3_bidirectional_formatter_styles", &ANTICHEAT_SPEC);

    // 1. Verify --style=readable rewrites compact syntax to readable syntax
    let script_readable = r#"
        let code = "fn demo(x: i64?) -> res<i64, Err> { let s = compute!^(x); let u = *..items; return s; }";
        format_pilot_with_style(code, 100, "readable");
    "#;
    let res_readable = eval_fmt(script_readable);
    assert!(res_readable.contains("opt<i64>"), "Expected opt<i64>, got: {}", res_readable);
    assert!(res_readable.contains("sig compute(x)"), "Expected sig compute(x), got: {}", res_readable);
    assert!(res_readable.contains("flat(items)"), "Expected flat(items), got: {}", res_readable);

    // 2. Verify --style=compact rewrites readable syntax to compact syntax
    let script_compact = r#"
        let code = "fn demo(x: opt<i64>) { let s = sig compute(x); let u = flat(items); return s; }";
        format_pilot_with_style(code, 100, "compact");
    "#;
    let res_compact = eval_fmt(script_compact);
    assert!(res_compact.contains("i64?"), "Expected i64?, got: {}", res_compact);
    assert!(res_compact.contains("compute!^(x)"), "Expected compute!^(x), got: {}", res_compact);
    assert!(res_compact.contains("*..items"), "Expected *..items, got: {}", res_compact);

    // 3. Verify --style=preserve keeps the author's choice untouched
    let script_preserve = r#"
        let code = "fn demo(a: opt<i64>, b: str?) { let s1 = sig f(a); let s2 = g!^(b); }";
        format_pilot_with_style(code, 100, "preserve");
    "#;
    let res_preserve = eval_fmt(script_preserve);
    assert!(res_preserve.contains("opt<i64>"), "Expected opt<i64> preserved in: {}", res_preserve);
    assert!(res_preserve.contains("str?"), "Expected str? preserved in: {}", res_preserve);
    assert!(res_preserve.contains("sig f(a)"), "Expected sig f(a) preserved in: {}", res_preserve);
    assert!(res_preserve.contains("g!^(b)"), "Expected g!^(b) preserved in: {}", res_preserve);

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Universal Rename Litmus and IDE Reflection
// -----------------------------------------------------------------------------
#[test]
pub fn test_m4_4_universal_rename_litmus_and_ide_reflection() {
    let guard = anticheat::AntiCheatGuard::arm("test_m4_4_universal_rename_litmus_and_ide_reflection", &ANTICHEAT_SPEC);

    // 1. Verify dynamic keyword reflection from keywords.pilot via vscode_gen
    let root = anticheat::workspace_root();
    let deps = [
        "stdlib/core/keywords.pilot",
        "selfhost/ide/vscode_gen.pilot",
    ];
    let mut combined = String::new();
    for rel in &deps {
        let p = root.join(rel);
        let src = fs::read_to_string(&p)
            .unwrap_or_else(|e| panic!("read {}: {:?}", p.display(), e));
        combined.push_str(&src);
        combined.push('\n');
    }
    let script = r#"
        let s_sig = reflect_paired_symbol_for_keyword("sig");
        let s_plan = reflect_paired_symbol_for_keyword("plan");
        let s_call = reflect_paired_symbol_for_keyword("call");
        let s_splat = reflect_paired_symbol_for_keyword("splat");
        let h_sig = generate_hover_markdown_for_keyword("sig");
        s_sig + "|" + s_plan + "|" + s_call + "|" + s_splat + "|" + (if h_sig.len() > 0 { "OK" } else { "ERR" });
    "#;
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
    let reflected = vm_outcome::outcome(&mut vm);
    assert!(reflected.contains("!^|!?|!|*|OK"), "Reflected symbols mismatch: {}", reflected);

    guard.finish();
}
