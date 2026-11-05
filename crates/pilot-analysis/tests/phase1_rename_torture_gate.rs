// ==============================================================================
// tests/phase1_rename_torture_gate.rs
// Phase 1 Verification Gate: Universal Type-Rename Torture Test
// Verifies that WholeNum, TruthBit, Utf8Run, and RecordContainer are analyzed
// directly by `selfhost/analyzer.pilot` via `analyze_physics` with ZERO manual
// layout registration in Rust test code.
// ==============================================================================

use pilot_analysis::{
    analyze_physics, bootstrap_core_file, eval_file_with_tables, ComptimeEngine,
    CapabilityBits,
    infer::{verify_capability_transition, compute_field_projection},
};
use pilot_ast::NodeKind;
use pilot_intern::SymbolId;
use pilot_parser::ParserTables;

const CORE_PILOT_SRC: &str = include_str!("../../../stdlib/core/core.pilot");

#[test]
fn test_type_rename_torture_gate_wholenum_truthbit_utf8run() {
    let mut tables = ParserTables::new();
    let mut vm = ComptimeEngine::new();
    let _ = bootstrap_core_file(CORE_PILOT_SRC, &mut tables, &mut vm)
        .expect("core.pilot must bootstrap cleanly");

    let renamed_src = r#"
        pub partial struct {
            [Bits(64, 2)] WholeNum : IValue {
                pub pure op + (rhs: WholeNum) -> WholeNum => __ssa(1, self, rhs);
                pub pure op - (rhs: WholeNum) -> WholeNum => __ssa(2, self, rhs);
                pub pure op * (rhs: WholeNum) -> WholeNum => __ssa(3, self, rhs);
            }

            [Bits(1, 3)] TruthBit : IValue {
                pub pure op == (rhs: TruthBit) -> TruthBit => __ssa(12, self, rhs);
            }

            [Bits(64, 6)] AddrVal : IReference {}

            [Layout(c_abi)]
            Utf8Run : IReference {
                pub data: AddrVal;
                pub len:  WholeNum;
            }

            RecordContainer : IObject {
                pub count:  WholeNum;
                pub flag:   TruthBit;
                pub label:  Utf8Run;
                ~self() {}
            }
        }
    "#;

    let arena = eval_file_with_tables(renamed_src, &mut tables, &mut vm)
        .expect("Renamed WholeNum/TruthBit/Utf8Run module must parse & evaluate cleanly");

    // Drive physical layout inference strictly through `selfhost/analyzer.pilot`
    let registry = analyze_physics(&arena, arena.root, None);

    let find_struct_lid = |target_name: &str| {
        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == NodeKind::StructDecl {
                let name = arena.interner.resolve(SymbolId(node.lhs));
                if name == target_name {
                    return registry.node_layouts[idx]
                        .unwrap_or_else(|| panic!("StructDecl '{}' must have a bound LayoutId from analyzer.pilot", target_name));
                }
            }
        }
        panic!("StructDecl '{}' not found in AST", target_name);
    };

    let whole_num_id = find_struct_lid("WholeNum");
    let truth_bit_id = find_struct_lid("TruthBit");
    let utf8_run_id = find_struct_lid("Utf8Run");
    let record_id = find_struct_lid("RecordContainer");

    let utf8_layout = registry.layouts.get(utf8_run_id).expect("Utf8Run layout must exist in registry.layouts");
    assert_eq!(utf8_layout.byte_size, 16, "Utf8Run size must be 16 bytes (ptr + len)");
    assert_eq!(utf8_layout.byte_align, 8, "Utf8Run alignment must be 8 bytes");

    let record_layout = registry.layouts.get(record_id).expect("RecordContainer layout must exist in registry.layouts");
    assert_eq!(record_layout.byte_size, 32, "RecordContainer size with padding must be 32 bytes");
    assert_eq!(record_layout.byte_align, 8, "RecordContainer alignment must be 8 bytes");

    // Verify Field Projection with Alignment Padding produced by `selfhost/analyzer.pilot`
    let field_0 = compute_field_projection(record_layout, 0).expect("field 0 exists");
    assert_eq!(field_0.byte_offset, 0, "count byte_offset is 0");
    assert_eq!(field_0.layout, whole_num_id);

    let field_1 = compute_field_projection(record_layout, 1).expect("field 1 exists");
    assert_eq!(field_1.byte_offset, 8, "flag byte_offset is 8");
    assert_eq!(field_1.layout, truth_bit_id);

    let field_2 = compute_field_projection(record_layout, 2).expect("field 2 exists");
    assert_eq!(field_2.byte_offset, 16, "label byte_offset must be padded to 16 for 8-byte alignment");
    assert_eq!(field_2.layout, utf8_run_id);

    // Verify Capability Transitions
    assert!(verify_capability_transition(CapabilityBits::VALUE, CapabilityBits::BORROW_REF));
    assert!(verify_capability_transition(CapabilityBits::OWNED_OBJ, CapabilityBits::BORROW_REF));
    assert!(verify_capability_transition(CapabilityBits::OWNED_OBJ, CapabilityBits::SCOPED_GUARD));
}
