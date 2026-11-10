// ==============================================================================
// tests/phase1_ironclad_killswitch.rs
// Ironclad Phase 1 Verification:
// Gate 1: The Kill-Switch Proof Test (Proves Rust Has Zero Hidden Inference)
// Gate 2: The Anti-Hardcoding Structural Proof Test (1B, 8B, 24B, 32B)
// ==============================================================================

use pilot_analysis::{analyze_physics, PhysicsRegistry};
use pilot_ast::NodeKind;
use pilot_intern::SymbolId;
use pilot_parser::parse;

#[test]
fn test_killswitch_proof_part_a_and_part_b() {
    let snippet = r#"
        struct Vector2 {
            x: i64;
            y: i64;
        }

        fn compute(v: Vector2) -> i64 {
            let base = 42;
            let sum = base + 100;
            if true {
                return sum;
            }
            return v.x;
        }
    "#;

    let arena = parse(snippet);

    // =========================================================================
    // Gate 1, Part A: No-Op Analyzer Kill-Switch
    // Overriding the analyzer script with a no-op proves Rust does ZERO inference.
    // =========================================================================
    let no_op_script = "fn analyze_unit(unit_id) { return 0; }";
    let killswitch_result: PhysicsRegistry = analyze_physics(&arena, arena.root, Some(no_op_script));

    assert_eq!(
        killswitch_result.layouts.len(),
        0,
        "KILL-SWITCH FAILED: LayoutTable must be strictly empty (len == 0) when analyzer is a no-op!"
    );

    assert!(
        killswitch_result.node_layouts.iter().all(|l| l.is_none()),
        "KILL-SWITCH FAILED: All node layouts must be None when analyzer is a no-op!"
    );

    // =========================================================================
    // Gate 1, Part B: Real selfhost/analyzer.pilot Execution
    // The exact same snippet analyzed by selfhost/analyzer.pilot populates layouts.
    // =========================================================================
    let real_result: PhysicsRegistry = analyze_physics(&arena, arena.root, None);

    assert!(
        real_result.layouts.len() >= 3,
        "REAL ANALYZER FAILED: Expected at least 3 layouts registered by selfhost/analyzer.pilot, got: {}",
        real_result.layouts.len()
    );

    let bound_count = real_result.node_layouts.iter().filter(|l| l.is_some()).count();
    assert!(
        bound_count > 0,
        "REAL ANALYZER FAILED: Expected bound node layouts from selfhost/analyzer.pilot, got 0"
    );

    for (node_id, bound) in real_result.node_layouts.iter().enumerate() {
        if let Some(lid) = bound {
            let layout = real_result.layouts.get(*lid).expect("Bound LayoutId must exist in LayoutTable");
            assert!(
                layout.byte_size > 0 || layout.byte_align > 0,
                "Node {} layout {:?} must have non-zero size/align",
                node_id,
                layout
            );
        }
    }
}

#[test]
fn test_arbitrary_prelude_type_names() {
    let custom_prelude_snippet = r#"
        [Bits(64, 2)]
        struct Zorglub64 {
            raw: Zorglub64;
        }

        [Bits(8, 3)]
        struct QuantumTruth {
            raw: QuantumTruth;
        }

        struct Vec3 {
            x: Zorglub64;
            y: Zorglub64;
            z: Zorglub64;
        }

        struct Composite {
            flag: QuantumTruth;
            vec: Vec3;
        }

        fn run_physics(v: Vec3, cond: QuantumTruth) -> Zorglub64 {
            let flag = true;
            let offset = 100;
            let res = v.x + offset;
            if flag {
                return res;
            }
            return v.z;
        }
    "#;

    let arena = parse(custom_prelude_snippet);
    let result: PhysicsRegistry = analyze_physics(&arena, arena.root, None);

    let find_struct_layout = |target_name: &str| {
        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == NodeKind::StructDecl {
                let name = arena.interner.resolve(SymbolId(node.lhs));
                if name == target_name {
                    let lid = result.node_layouts[idx]
                        .unwrap_or_else(|| panic!("StructDecl '{}' must have a bound LayoutId", target_name));
                    return result.layouts.get(lid).expect("Layout must exist in LayoutTable").clone();
                }
            }
        }
        panic!("StructDecl '{}' not found in AST", target_name);
    };

    let find_let_layout = |target_name: &str| {
        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == NodeKind::LetStmt {
                let name = arena.interner.resolve(SymbolId(node.lhs));
                if name == target_name {
                    let lid = result.node_layouts[idx]
                        .unwrap_or_else(|| panic!("LetStmt '{}' must have a bound LayoutId", target_name));
                    return result.layouts.get(lid).expect("Layout must exist in LayoutTable").clone();
                }
            }
        }
        panic!("LetStmt '{}' not found in AST", target_name);
    };

    // 1. Zorglub64 must be 8 bytes, 8-byte aligned
    let zorglub_layout = find_struct_layout("Zorglub64");
    assert_eq!(zorglub_layout.byte_size, 8, "Zorglub64 scalar size must be 8 bytes");
    assert_eq!(zorglub_layout.byte_align, 8, "Zorglub64 scalar align must be 8 bytes");

    // 2. QuantumTruth must be 1 byte, 1-byte aligned
    let quantum_layout = find_struct_layout("QuantumTruth");
    assert_eq!(quantum_layout.byte_size, 1, "QuantumTruth scalar size must be 1 byte");
    assert_eq!(quantum_layout.byte_align, 1, "QuantumTruth scalar align must be 1 byte");

    // 3. Vec3 (3 x Zorglub64) must be 24 bytes (NOT 8 or 16)
    let vec3_layout = find_struct_layout("Vec3");
    assert_eq!(vec3_layout.byte_size, 24, "Vec3 struct size must be 24 bytes (3 x 8B)");
    assert_eq!(vec3_layout.byte_align, 8, "Vec3 struct align must be 8 bytes");

    // 4. Composite { flag: QuantumTruth (1B + 7B pad), vec: Vec3 (24B) } must be 32 bytes
    let composite_layout = find_struct_layout("Composite");
    assert_eq!(composite_layout.byte_size, 32, "Composite struct size with padding must be 32 bytes (1B + 7B pad + 24B)");
    assert_eq!(composite_layout.byte_align, 8, "Composite struct align must be 8 bytes");

    // 5. `let flag = true;` must be bound to a 1-byte layout (NOT 8)
    let flag_let_layout = find_let_layout("flag");
    assert_eq!(flag_let_layout.byte_size, 1, "`let flag = true;` must be inferred as 1-byte layout, not 8-byte");

    // 6. `let res = v.x + offset;` must be bound to an 8-byte layout via MemberExpr + BinaryExpr inference
    let res_let_layout = find_let_layout("res");
    assert_eq!(res_let_layout.byte_size, 8, "`let res = v.x + offset;` must be inferred as 8-byte layout");
}

#[test]
fn test_pilot_hosted_div_by_zero_and_comptime_persistence() {
    // 1. Static Division-by-Zero Verification via AnalysisPipeline::analyze
    let div_zero_src = r#"
        fn bad_div(x: i64) -> i64 {
            return x / 0;
        }
    "#;
    let arena = parse(div_zero_src);
    let result = pilot_analysis::AnalysisPipeline::analyze(&arena, arena.root);
    assert!(!result.diagnostics.is_empty(), "Division by zero must emit a diagnostic");
    assert!(
        result.diagnostics.iter().any(|d| d.message.contains("DIV001")),
        "Expected DIV001 diagnostic, got: {:?}",
        result.diagnostics
    );

    // 2. ComptimeEngine state persistence
    let comptime_snippet = r#"
        __engine("layout_hw_header", 10);
    "#;
    let comptime_arena = parse(comptime_snippet);
    let mut comptime_engine = pilot_analysis::ComptimeEngine::new();
    assert_eq!(comptime_engine.layout_table.len(), 0);
    let eval_res = comptime_engine.eval_ast_node(&comptime_arena, comptime_arena.root);
    assert!(eval_res.is_ok(), "eval_ast_node must succeed: {:?}", eval_res);
    assert_eq!(
        comptime_engine.layout_table.len(),
        1,
        "eval_ast_node must persist registered layout back into self.layout_table"
    );
}

fn check_src(src: &str) -> pilot_analysis::AnalysisResult {
    let a = parse(src);
    pilot_analysis::AnalysisPipeline::analyze(&a, a.root)
}

#[test]
fn test_semantic_null_safety_diagnostics() {
    // 1. NULL001: Assigning 'none' to non-nullable type
    let res = check_src("fn test_bad() { let x: i64 = none; }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("NULL001")), "Expected NULL001 on bad assign: {:?}", res.diagnostics);

    // NULL001: Postfix '!' non-null assertion forbidden
    let res = check_src("fn test_bang(x: i64?) -> i64 { return x!; }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("NULL001")), "Expected NULL001 on postfix '!': {:?}", res.diagnostics);

    // Valid: Assigning 'none' to optional type
    let res = check_src("fn test_good() { let x: i64? = none; }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("NULL001")), "Unexpected NULL001 on optional assign: {:?}", res.diagnostics);

    // 2. NULL002: Guard statement without else block
    let res = check_src("fn test_g(x: i64) { guard x > 0; }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("NULL002")), "Expected NULL002 on guard without else: {:?}", res.diagnostics);

    // NULL002: Guard statement with non-diverging else block
    let res = check_src("fn test_g2(x: i64) { guard x > 0 else { let y = 1; } }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("NULL002")), "Expected NULL002 on guard fallthrough: {:?}", res.diagnostics);

    // Valid: Guard statement with diverging else block (return)
    let res = check_src("fn test_g3(x: i64) -> i64 { guard x > 0 else { return 0; } return x; }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("NULL002")), "Unexpected NULL002 on valid guard return: {:?}", res.diagnostics);

    // Valid: Guard statement with diverging else block (panic)
    let res = check_src("fn test_g4(x: i64) -> i64 { guard x > 0 else { panic(0); } return x; }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("NULL002")), "Unexpected NULL002 on valid guard panic: {:?}", res.diagnostics);

    // 3. NULL003: Returning 'none' from function with non-nullable return type
    let res = check_src("fn test_ret() -> i64 { return none; }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("NULL003")), "Expected NULL003 on return none: {:?}", res.diagnostics);

    // Valid: Returning 'none' from function with optional return type
    let res = check_src("fn test_ret_opt() -> i64? { return none; }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("NULL003")), "Unexpected NULL003 on optional return: {:?}", res.diagnostics);

    // 4. NULL004: Passing 'none' to non-nullable parameter
    let res = check_src("fn helper(x: i64) -> i64 { return x; } fn caller() { helper(none); }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("NULL004")), "Expected NULL004 on arg none: {:?}", res.diagnostics);

    // Valid: Passing 'none' to optional parameter
    let res = check_src("fn helper_opt(x: i64?) -> i64 { return 0; } fn caller_opt() { helper_opt(none); }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("NULL004")), "Unexpected NULL004 on optional arg: {:?}", res.diagnostics);

    // 5. NULL005: Index access on optional type requires safe navigation or unwrapping
    let res = check_src("fn test_opt_idx(arr: i64[]?) { let v = arr[0]; }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("NULL005")), "Expected NULL005 on optional indexing: {:?}", res.diagnostics);

    // Valid: Index access on non-optional collection
    let res = check_src("fn test_good_idx(arr: i64[]) { let v = arr[0]; }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("NULL005")), "Unexpected NULL005 on valid indexing: {:?}", res.diagnostics);
}

#[test]
fn test_semantic_values_first_error_and_loop_diagnostics() {
    // 1. ERR001: Statement-level call to fallible function discarded
    let res = check_src("fn try_op() -> i64 ! i64 { return 1; } fn caller() { try_op(); }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("ERR001")), "Expected ERR001 on discarded fallible call: {:?}", res.diagnostics);

    // Valid: Fallible call assigned to a let binding
    let res = check_src("fn try_op() -> i64 ! i64 { return 1; } fn caller() { let x = try_op(); }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("ERR001")), "Unexpected ERR001 on bound call: {:?}", res.diagnostics);

    // Valid: Fallible call handled with catch recovery
    let res = check_src("fn try_op() -> i64 ! i64 { return 1; } fn caller() { try_op() catch err => 0; }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("ERR001")), "Unexpected ERR001 on catch call: {:?}", res.diagnostics);

    // 2. ERR005: Legacy try/catch in native Pilot code emits warning
    let res = check_src("fn test_try() { try { let a = 1; } catch e { let b = 2; } }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("ERR005")), "Expected ERR005 on native try/catch: {:?}", res.diagnostics);

    // 3. LOOP001: Cannot mix 'break <expr>' and 'yield <expr>' in same for loop
    let res = check_src("fn test_loop() { for i in 0 .. 10 { yield i; break i; } }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("LOOP001")), "Expected LOOP001 on mixed break/yield: {:?}", res.diagnostics);

    // 4. LOOP002: Cannot mix valueless 'break;' and valued 'break <expr>;'
    let res = check_src("fn test_break() { for i in 0 .. 10 { if i == 1 { break; } break i; } }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("LOOP002")), "Expected LOOP002 on mixed break void/value: {:?}", res.diagnostics);

    // 5. LOOP003: Loop completion 'then' branch only valid on ScalarBreak
    let res = check_src("fn test_then() { for i in 0 .. 10 { let x = i; } then 0; }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("LOOP003")), "Expected LOOP003 on then without valued break: {:?}", res.diagnostics);

    // Valid: ScalarBreak loop with break <expr> and then branch
    let res = check_src("fn test_good_then() -> i64 { return for i in 0 .. 10 { if i == 5 { break i; } } then 0; }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("LOOP003")), "Unexpected LOOP003 on valid then: {:?}", res.diagnostics);

    // 6. LOOP004: Cannot yield inside statement-position for in non-generator function
    let res = check_src("fn test_stmt_yield() -> i64 { for i in 0 .. 10 { yield i; } return 0; }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("LOOP004")), "Expected LOOP004 on statement yield: {:?}", res.diagnostics);

    // Valid: Expression-position for loop with yield
    let res = check_src("fn test_expr_yield() { let items = for i in 0 .. 10 { yield i; }; }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("LOOP004")), "Unexpected LOOP004 on expression yield: {:?}", res.diagnostics);
}

#[test]
fn test_semantic_pattern_matching_diagnostics() {
    // 1. MATCH002: Unreachable pattern after wildcard arm
    let res = check_src("fn test_sw(x: i64) -> i64 { return switch x { case _: 0, case 1: 1 }; }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("MATCH002")), "Expected MATCH002 on unreachable arm: {:?}", res.diagnostics);

    // Valid: Wildcard arm at the end
    let res = check_src("fn test_good_sw(x: i64) -> i64 { return switch x { case 1: 1, case _: 0 }; }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("MATCH002")), "Unexpected MATCH002 on valid arms: {:?}", res.diagnostics);
}

#[test]
fn test_semantic_warning_and_unimplemented_diagnostics() {
    // 1. UNREACH001: Unreachable code after return
    let res = check_src("fn test_unreach() -> i64 { return 42; let dead = 10; return dead; }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("UNREACH001")), "Expected UNREACH001 on dead code: {:?}", res.diagnostics);
    let d = res.diagnostics.iter().find(|d| d.message.contains("UNREACH001")).unwrap();
    assert_eq!(d.severity, pilot_analysis::DiagnosticSeverity::Warning);

    // Valid: reachable code
    let res = check_src("fn test_reach() -> i64 { let alive = 10; return alive; }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("UNREACH001")), "Unexpected UNREACH001: {:?}", res.diagnostics);

    // 2. LOOP005: Potential infinite loop
    let res = check_src("fn test_inf() { while true { let x = 1; } }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("LOOP005")), "Expected LOOP005 on infinite loop: {:?}", res.diagnostics);
    let d = res.diagnostics.iter().find(|d| d.message.contains("LOOP005")).unwrap();
    assert_eq!(d.severity, pilot_analysis::DiagnosticSeverity::Warning);

    // Valid: loop with break
    let res = check_src("fn test_finite() { while true { break; } }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("LOOP005")), "Unexpected LOOP005 on break loop: {:?}", res.diagnostics);

    // 3. COND001: Compile-time constant condition
    let res = check_src("fn test_cond(x: i64) { if false { let y = 1; } }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("COND001")), "Expected COND001 on const cond: {:?}", res.diagnostics);

    // 4. COND002: Empty branch in condition
    let res = check_src("fn test_empty_if(x: i64) { if x > 0 {} }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("COND002")), "Expected COND002 on empty branch: {:?}", res.diagnostics);

    // 5. UNIMP001: Call to todo() or unimplemented()
    let res = check_src("fn test_todo() { todo(); }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("UNIMP001")), "Expected UNIMP001 on todo(): {:?}", res.diagnostics);
    let res = check_src("fn test_unimp() { unimplemented(); }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("UNIMP001")), "Expected UNIMP001 on unimplemented(): {:?}", res.diagnostics);

    // 6. UNIMP002: Function declaring return type with empty body
    let res = check_src("fn test_empty_body() -> i64 {}");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("UNIMP002")), "Expected UNIMP002 on empty fn body: {:?}", res.diagnostics);

    // Valid: non-empty body
    let res = check_src("fn test_valid_body() -> i64 { return 1; }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("UNIMP002")), "Unexpected UNIMP002 on valid fn: {:?}", res.diagnostics);

    // 7. UNIMP003: Struct missing interface method
    let res = check_src("interface IWorker { fn perform() -> i64; } struct BadWorker : IWorker { pub id: i64; }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("UNIMP003")), "Expected UNIMP003 on missing iface method: {:?}", res.diagnostics);

    // Valid: struct implementing interface method
    let res = check_src("interface IWorker { fn perform() -> i64; } struct GoodWorker : IWorker { pub id: i64; pub fn perform() -> i64 { return 1; } }");
    assert!(!res.diagnostics.iter().any(|d| d.message.contains("UNIMP003")), "Unexpected UNIMP003 on implemented iface: {:?}", res.diagnostics);

    // 8. UNIMP004: [Unimplemented] attribute
    let res = check_src("[Unimplemented] struct FutureFeature { pub x: i64; }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("UNIMP004")), "Expected UNIMP004 on [Unimplemented]: {:?}", res.diagnostics);
}

#[test]
fn test_semantic_info_and_optimization_diagnostics() {
    // 1. OPT001: Constant binary expression can be folded at compile time
    let res = check_src("fn test_fold() -> i64 { return 10 + 20; }");
    assert!(
        res.diagnostics.iter().any(|d| d.code == "OPT001" && d.severity == pilot_analysis::DiagnosticSeverity::Info),
        "Expected OPT001 info on constant addition: {:?}",
        res.diagnostics
    );

    // Non-constant binary expr does not trigger OPT001
    let res = check_src("fn test_non_const(x: i64) -> i64 { return x + 1; }");
    assert!(
        !res.diagnostics.iter().any(|d| d.code == "OPT001"),
        "Unexpected OPT001 on non-constant expr: {:?}",
        res.diagnostics
    );

    // 2. OPT002: Bounded loop unrolling candidate
    let res = check_src("fn test_unroll() { for i in 0 .. 4 { let y = i; } }");
    assert!(
        res.diagnostics.iter().any(|d| d.code == "OPT002" && d.severity == pilot_analysis::DiagnosticSeverity::Info),
        "Expected OPT002 info on bounded loop: {:?}",
        res.diagnostics
    );
}

/// Codes of the diagnostics `res` reports, failing if the analyzer faulted.
fn codes_of(res: &pilot_analysis::AnalysisResult) -> Vec<String> {
    assert!(res.physics.analyzer_error.is_none(), "analyzer faulted: {:?}", res.physics.analyzer_error);
    res.diagnostics.iter().map(|d| d.code.clone()).collect()
}

fn count_code(res: &pilot_analysis::AnalysisResult, code: &str) -> usize {
    codes_of(res).iter().filter(|c| c.as_str() == code).count()
}

/// Source text of every `code` diagnostic in `src`, in report order.
fn flagged_text(src: &str, code: &str) -> Vec<String> {
    let res = check_src(src);
    let _ = codes_of(&res);
    res.diagnostics
        .iter()
        .filter(|d| d.code == code)
        .map(|d| src[d.span.start as usize..d.span.end as usize].to_string())
        .collect()
}

#[test]
fn test_tier_1_1_flow_and_declaration_diagnostics() {
    // FLOW002: Constructor return disallowed
    let res = check_src("struct S { self() { return 1; } }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("FLOW002")), "Expected FLOW002 on constructor return: {:?}", res.diagnostics);

    // FLOW003: no loop or switch around `break` in its function.
    let res = check_src("fn quit_early() { break; }");
    assert_eq!(count_code(&res, "FLOW003"), 1, "{:?}", res.diagnostics);
    let res = check_src("fn quit_early() { while true { break; } }");
    assert_eq!(count_code(&res, "FLOW003"), 0, "{:?}", res.diagnostics);
    let res = check_src("fn quit_early() { loop { break; } }");
    assert_eq!(count_code(&res, "FLOW003"), 0, "{:?}", res.diagnostics);
    // A loop outside a closure does not enclose a `break` inside the closure.
    let res = check_src("fn outer() { while true { let stop = () => { break; }; stop(); break; } }");
    assert_eq!(count_code(&res, "FLOW003"), 1, "{:?}", res.diagnostics);

    // FLOW004: no loop around `continue` in its function.
    let res = check_src("fn skip_ahead() { continue; }");
    assert_eq!(count_code(&res, "FLOW004"), 1, "{:?}", res.diagnostics);
    let res = check_src("fn skip_ahead() { for i in 0 .. 3 { continue; } }");
    assert_eq!(count_code(&res, "FLOW004"), 0, "{:?}", res.diagnostics);

    // FLOW005: Non-exhaustive pattern match in switch
    let res = check_src("fn bad_sw(x: i64) -> i64 { return switch x { case 1: 10, case 2: 20 }; }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("FLOW005")), "Expected FLOW005 on switch without wildcard: {:?}", res.diagnostics);

    // DECL002: Duplicate member declaration in struct scope
    let res = check_src("struct DupS { pub x: i64; pub x: i64; }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("DECL002")), "Expected DECL002 on duplicate struct field: {:?}", res.diagnostics);

    // DECL004: Reassignment to immutable let binding
    let res = check_src("fn test_immut() { let immut = 10; immut = 20; }");
    assert!(res.diagnostics.iter().any(|d| d.message.contains("DECL004")), "Expected DECL004 on immutable let reassignment: {:?}", res.diagnostics);
}

#[test]
fn test_tier_1_2_unused_bindings_and_shadowing() {
    // `scratch` is never read and `spare` is never read; every other binding is.
    // A name that merely starts with "unused" but is read is not reported.
    let src = "fn tally(limit: i64, spare: i64) -> i64 {\n    let total = limit * 2;\n    let scratch = 7;\n    var count = 0;\n    count = count + total;\n    let unused_name = 1;\n    return count + unused_name;\n}\n";
    let unused_locals = flagged_text(src, "UNUSED001");
    assert_eq!(unused_locals.len(), 1, "{:?}", unused_locals);
    assert!(unused_locals[0].starts_with("let scratch"), "{:?}", unused_locals);
    let unused_params = flagged_text(src, "UNUSED004");
    assert_eq!(unused_params.len(), 1, "{:?}", unused_params);
    assert!(unused_params[0].starts_with("spare"), "{:?}", unused_params);
    assert!(flagged_text(src, "SHADOW001").is_empty());

    // A parameter read only inside a closure is used; a closure parameter
    // that is never read is reported.
    let src = "fn lift(base: i64) -> i64 {\n    let add = (v, w) => v + base;\n    return add(1, 2);\n}\n";
    assert!(flagged_text(src, "UNUSED001").is_empty());
    let params = flagged_text(src, "UNUSED004");
    assert_eq!(params.len(), 1, "{:?}", params);
    assert!(params[0].starts_with('w'), "{:?}", params);

    // Bodiless interface methods and `_` names are not reported.
    let src = "interface IShape { fn area(scale: i64) -> i64; }\nfn keep(_ignored: i64) -> i64 { let _tmp = 1; return 1; }\n";
    assert!(flagged_text(src, "UNUSED004").is_empty());
    assert!(flagged_text(src, "UNUSED001").is_empty());

    // A `let` in a nested block hides a parameter of the function.
    let src = "fn scale(x: i64) -> i64 {\n    if x > 0 {\n        let x = 2;\n        return x;\n    }\n    return x;\n}\n";
    let shadows = flagged_text(src, "SHADOW001");
    assert_eq!(shadows.len(), 1, "{:?}", shadows);
    assert!(shadows[0].starts_with("let x = 2"), "{:?}", shadows);
}

#[test]
fn test_tier_1_3_shadowing_conformance_program() {
    // tests/conformance/shadowing.pilot: the three inner `let x` hide the
    // outer `x`; `let y = y + 1;` re-declares in the same block, which is not
    // shadowing an enclosing scope. Every binding is read.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let src = std::fs::read_to_string(root.join("tests/conformance/shadowing.pilot")).expect("read shadowing.pilot");
    let shadows = flagged_text(&src, "SHADOW001");
    assert_eq!(shadows.len(), 3, "{:?}", shadows);
    assert!(shadows[0].starts_with("let x = i * 10"), "{:?}", shadows);
    assert!(shadows[1].starts_with("let x = \"inner\""), "{:?}", shadows);
    assert!(shadows[2].starts_with("let x = x + 1"), "{:?}", shadows);
    assert!(flagged_text(&src, "UNUSED001").is_empty());
    assert!(flagged_text(&src, "UNUSED004").is_empty());
}

#[test]
fn test_diagnostic_catalogs_parsed_cleanly() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    for catalog_rel in [
        "selfhost/diagnostics/errors.pilot",
        "selfhost/diagnostics/warnings.pilot",
        "selfhost/diagnostics/infos.pilot",
    ] {
        let p = root.join(catalog_rel);
        assert!(p.exists(), "Diagnostic catalog file must exist: {:?}", p);
        let src = std::fs::read_to_string(&p).expect("read catalog");
        let parsed = parse(&src);
        assert!(
            parsed.diagnostics.is_empty(),
            "Diagnostic catalog {:?} must parse with 0 diagnostics, got: {:?}",
            catalog_rel,
            parsed.diagnostics
        );
    }
}

#[test]
fn test_engine_keyword_banned_outside_core_and_mapped_classes() {
    // 1. Direct __engine call outside core must emit ENGINE001
    let banned_user_src = r#"
        pkg MyApp;
        fn write_data() {
            let res = __engine("fs_write_text", "out.txt", "hello");
        }
    "#;
    let res = check_src(banned_user_src);
    assert!(
        res.diagnostics.iter().any(|d| d.message.contains("ENGINE001")),
        "Direct __engine call in user code outside core must emit ENGINE001: {:?}",
        res.diagnostics
    );

    // 2. Direct __engine call in script without package must also emit ENGINE001
    let banned_script_src = r#"
        fn main() {
            let m = __engine("map_new");
        }
    "#;
    let res2 = check_src(banned_script_src);
    assert!(
        res2.diagnostics.iter().any(|d| d.message.contains("ENGINE001")),
        "Direct __engine call in script without package must emit ENGINE001: {:?}",
        res2.diagnostics
    );

    // 3. __engine call inside Pilot.Core.* package is valid (no ENGINE001)
    let valid_core_src = r#"
        pub pkg Pilot.Core.Tests;
        fn core_action() {
            let h = __engine("map_new");
        }
    "#;
    let res3 = check_src(valid_core_src);
    assert!(
        !res3.diagnostics.iter().any(|d| d.message.contains("ENGINE001")),
        "__engine call inside Pilot.Core.* must be permitted: {:?}",
        res3.diagnostics
    );

    // 4. Calling properly mapped File struct methods outside core emits zero ENGINE001 errors
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let file_src = std::fs::read_to_string(root.join("stdlib/core/file.pilot")).expect("read file.pilot");
    let mapped_user_src = format!(
        "{}\npkg MyApp;\nfn write_data() {{\n    let res = File.write_text(\"out.txt\", \"hello\");\n}}\n",
        file_src
    );
    let res4 = check_src(&mapped_user_src);
    assert!(
        !res4.diagnostics.iter().any(|d| d.message.contains("ENGINE001")),
        "Calling mapped File.write_text outside core must have zero ENGINE001 errors: {:?}",
        res4.diagnostics
    );

    // 5. Verify selfhost/cli/ core files contain zero direct __engine calls
    for rel in [
        "selfhost/cli/pkg_cmd.pilot",
        "selfhost/cli/spec.pilot",
        "selfhost/cli/parser.pilot",
        "selfhost/cli/main.pilot",
    ] {
        let p = root.join(rel);
        let src = std::fs::read_to_string(&p).expect("read file");
        for line in src.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("//") {
                assert!(
                    !trimmed.contains("__engine("),
                    "File '{}' contains banned direct __engine call: {}",
                    rel,
                    trimmed
                );
            }
        }
    }
}


