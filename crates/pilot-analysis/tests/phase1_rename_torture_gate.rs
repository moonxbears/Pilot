// ==============================================================================
// tests/phase1_rename_torture_gate.rs
// Phase 1 & M50.4 Verification Gate: Universal Type-Rename Torture Test
// Verifies that WholeNum, TruthBit, Utf8Run, and RecordContainer are analyzed
// directly by `selfhost/analyzer.pilot` via `analyze_physics` AND that
// `verify_capability_transition`, `compute_field_projection_offset`, and
// target-width `compute_soa_projection` execute 100% in Pilot.
// ==============================================================================

use pilot_analysis::{
    analyze_physics, bootstrap_core_file, eval_file_with_tables, handle_engine_intrinsic,
    CapabilityBits, ComptimeEngine, LayoutId, PhysicsGeometry,
};
use pilot_ast::NodeKind;
use pilot_intern::SymbolId;
use pilot_parser::ParserTables;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

const CORE_PILOT_SRC: &str = include_str!("../../../stdlib/core/core.pilot");
const ANALYZER_PILOT_SRC: &str = include_str!("../../../selfhost/analyzer.pilot");

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

    let mut registry = analyze_physics(&arena, arena.root, None);

    let find_struct_lid = |target_name: &str, reg: &pilot_analysis::PhysicsRegistry| {
        for (idx, node) in arena.nodes.iter().enumerate() {
            if node.node_kind() == NodeKind::StructDecl {
                let name = arena.interner.resolve(SymbolId(node.lhs));
                if name == target_name {
                    return reg.node_layouts[idx]
                        .unwrap_or_else(|| panic!("StructDecl '{}' must have a bound LayoutId from analyzer.pilot", target_name));
                }
            }
        }
        panic!("StructDecl '{}' not found in AST", target_name);
    };

    let whole_num_id = find_struct_lid("WholeNum", &registry);
    let truth_bit_id = find_struct_lid("TruthBit", &registry);
    let utf8_run_id = find_struct_lid("Utf8Run", &registry);
    let record_id = find_struct_lid("RecordContainer", &registry);

    let utf8_layout = registry.layouts.get(utf8_run_id).expect("Utf8Run layout must exist").clone();
    assert_eq!(utf8_layout.byte_size, 16, "Utf8Run size must be 16 bytes (ptr + len)");
    assert_eq!(utf8_layout.byte_align, 8, "Utf8Run alignment must be 8 bytes");

    let record_layout = registry.layouts.get(record_id).expect("RecordContainer layout must exist").clone();
    assert_eq!(record_layout.byte_size, 32, "RecordContainer size with padding must be 32 bytes");
    assert_eq!(record_layout.byte_align, 8, "RecordContainer alignment must be 8 bytes");

    match &record_layout.geometry {
        PhysicsGeometry::Aggregate { fields, .. } => {
            assert_eq!(fields[0].byte_offset, 0);
            assert_eq!(fields[0].layout, whole_num_id);
            assert_eq!(fields[1].byte_offset, 8);
            assert_eq!(fields[1].layout, truth_bit_id);
            assert_eq!(fields[2].byte_offset, 16);
            assert_eq!(fields[2].layout, utf8_run_id);
        }
        other => panic!("Expected Aggregate geometry, got {:?}", other),
    }

    // Verify Pilot-hosted capability transitions, field projections & target-width SOA projections
    let eval_pilot_expr = |reg: &mut pilot_analysis::PhysicsRegistry, call_expr: &str| -> pilot_rt::PilotVal {
        let base_src = ANALYZER_PILOT_SRC.trim_end().strip_suffix("analyze_unit(0);").unwrap_or(ANALYZER_PILOT_SRC);
        let script = format!("{}\n{};\n", base_src, call_expr);
        let p = pilot_parser::parse(&script);
        let bin = BytecodeCompiler::compile(&p, p.root);
        let mut pvm = PilVirtualMachine::new();
        pvm.load_binary(bin);
        let reg_ptr = reg as *mut pilot_analysis::PhysicsRegistry;
        pvm.engine_handler = Some(Box::new(move |op, args| handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, args)));
        pvm.run()
    };

    assert_eq!(eval_pilot_expr(&mut registry, &format!("compute_field_projection_offset({}, 2)", record_id.0)).as_int(), Some(16));
    assert_eq!(
        eval_pilot_expr(&mut registry, &format!("verify_capability_transition({}, {})", CapabilityBits::VALUE.raw(), CapabilityBits::BORROW_REF.raw())).as_bool(),
        Some(true)
    );
    assert_eq!(
        eval_pilot_expr(&mut registry, &format!("verify_capability_transition({}, {})", CapabilityBits::OWNED_OBJ.raw(), CapabilityBits::SCOPED_GUARD.raw())).as_bool(),
        Some(true)
    );
    assert_eq!(
        eval_pilot_expr(&mut registry, &format!("verify_capability_transition({}, {})", CapabilityBits::BORROW_REF.raw(), CapabilityBits::OWNED_OBJ.raw())).as_bool(),
        Some(false)
    );

    // 64-bit target SOA column slice -> 16 bytes, align 8; 32-bit target SOA column slice -> 8 bytes, align 4
    let soa64_lid = LayoutId(eval_pilot_expr(&mut registry, &format!("compute_soa_projection({}, 64)", whole_num_id.0)).as_int().unwrap() as u32);
    let soa32_lid = LayoutId(eval_pilot_expr(&mut registry, &format!("compute_soa_projection({}, 32)", whole_num_id.0)).as_int().unwrap() as u32);
    assert_eq!(registry.layouts.get(soa64_lid).unwrap().byte_size, 16);
    assert_eq!(registry.layouts.get(soa64_lid).unwrap().byte_align, 8);
    assert_eq!(registry.layouts.get(soa32_lid).unwrap().byte_size, 8);
    assert_eq!(registry.layouts.get(soa32_lid).unwrap().byte_align, 4);
}
