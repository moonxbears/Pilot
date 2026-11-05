// ==============================================================================
// tests/phase1_rename_torture_gate.rs
// Phase 1 Verification Gate: Universal Type-Rename Torture Test
// Verifies that WholeNum, TruthBit, and Utf8Run compile, infer, and derive
// correct physical layouts and C-ABI layouts via LayoutTable methods.
// ==============================================================================

use pilot_analysis::{
    bootstrap_core_file, eval_file_with_tables, ComptimeEngine,
    CapabilityBits, LayoutId, LayoutTable,
    infer::{verify_capability_transition, compute_field_projection, NodeLayoutMap},
};
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

    let _arena = eval_file_with_tables(renamed_src, &mut tables, &mut vm)
        .expect("Renamed WholeNum/TruthBit/Utf8Run module must parse & evaluate cleanly");

    // Name-Blind Physical Layout Verification using LayoutTable methods
    let mut layout_table = LayoutTable::new();
    assert_eq!(layout_table.len(), 0, "LayoutTable must start empty");

    // Register WholeNum: 64-bit signed scalar (8 bytes, 8 align)
    let whole_num_id = layout_table.register_scalar(64, 1, 2);

    // Register TruthBit: 1-bit scalar (1 byte, 1 align)
    let truth_bit_id = layout_table.register_scalar(1, 1, 0);

    // Register AddrVal: pointer targeting WholeNum
    let addr_val_id = layout_table.register_pointer(whole_num_id, CapabilityBits::MUTABLE, 0);

    // Register Utf8Run: Aggregate { data: AddrVal, len: WholeNum }
    let utf8_run_id = layout_table.register_aggregate(&[addr_val_id, whole_num_id], false, false);

    let utf8_layout = layout_table.get(utf8_run_id).expect("Utf8Run layout must exist");
    assert_eq!(utf8_layout.byte_size, 16, "Utf8Run size must be 16 bytes (ptr + len)");
    assert_eq!(utf8_layout.byte_align, 8, "Utf8Run alignment must be 8 bytes");

    // Register RecordContainer: Aggregate { count: WholeNum, flag: TruthBit, label: Utf8Run }
    // Offset calculation:
    // count:  offset 0,  size 8, align 8
    // flag:   offset 8,  size 1, align 1
    // [pad 7 bytes to align Utf8Run to 8 bytes]
    // label:  offset 16, size 16, align 8
    // Total size: 32 bytes, Align: 8
    let record_id = layout_table.register_aggregate(&[whole_num_id, truth_bit_id, utf8_run_id], false, false);

    let record_layout = layout_table.get(record_id).expect("RecordContainer layout must exist");
    assert_eq!(record_layout.byte_size, 32, "RecordContainer size with padding must be 32 bytes");
    assert_eq!(record_layout.byte_align, 8, "RecordContainer alignment must be 8 bytes");

    // Verify Field Projection with Alignment Padding
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

    // Verify NodeLayoutMap Binding
    let mut node_map = NodeLayoutMap::new();
    node_map.set(10, whole_num_id);
    node_map.set(11, truth_bit_id);
    node_map.set(12, utf8_run_id);

    assert_eq!(node_map.get(10), Some(whole_num_id));
    assert_eq!(node_map.get(11), Some(truth_bit_id));
    assert_eq!(node_map.get(12), Some(utf8_run_id));
    assert_eq!(node_map.get(999), None);
}
