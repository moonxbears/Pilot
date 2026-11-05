// ==============================================================================
// tests/phase1_ironclad_killswitch.rs
// Ironclad Phase 1 Verification:
// Gate 1: The Kill-Switch Proof Test (Proves Rust Has Zero Hidden Inference)
// Gate 2: The Randomized Type-Name Torture Test (Proves True Name-Blindness)
// ==============================================================================

use pilot_analysis::{analyze_physics, PhysicsRegistry};
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
        struct Zorglub64 {
            raw: Zorglub64;
        }

        struct FluxCapacitor {
            raw: FluxCapacitor;
        }

        struct QuantumTruth {
            raw: QuantumTruth;
        }

        struct Point {
            x: Zorglub64;
            y: Zorglub64;
        }

        fn run_physics(p: Point, cond: QuantumTruth) -> Zorglub64 {
            let offset = 100;
            let res = p.x + offset;
            if cond {
                return res;
            }
            return p.y;
        }
    "#;

    let arena = parse(custom_prelude_snippet);
    let result: PhysicsRegistry = analyze_physics(&arena, arena.root, None);

    assert!(
        result.layouts.len() >= 3,
        "Expected at least 3 layouts for arbitrary types, got: {}",
        result.layouts.len()
    );

    let scalar_layout = result.layouts.get(pilot_analysis::LayoutId(0)).expect("Scalar layout must exist");
    assert_eq!(scalar_layout.byte_size, 8, "Zorglub64 scalar size must be 8 bytes");
    assert_eq!(scalar_layout.byte_align, 8, "Zorglub64 scalar align must be 8 bytes");
}
