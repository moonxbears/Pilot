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
