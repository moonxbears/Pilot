//! ============================================================================
//! Milestone 6 Verification Suite:
//! First-Class Typed SSA Mid-Level IR (PilIR)
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
    milestone_id: 6,
    test_file_rel: "crates/pilot-analysis/tests/milestone06_first_class_typed_ssa_mid_level_ir.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/ir/model.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "reg_class_name",
                "lattice_name",
                "target_descriptor_new",
                "target_max_gpr_args",
            ],
            required_code_markers: &[
                "IrTypePhysics",
                "IrInst",
                "TargetDescriptor",
                "reg_class_gpr_signed",
                "op_const_bits",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ir/verifier.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "ir_diag_new",
                "ir_diag_to_string",
                "block_dominates",
                "verify_ssa_dominance",
            ],
            required_code_markers: &[
                "IR001",
                "IR002",
                "IR003",
                "block_dominates",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ir/pass_manager.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "pass_registry_add",
                "pass_registry_count",
                "pass_registry_get",
                "check_iteration_cap",
            ],
            required_code_markers: &[
                "PassResult",
                "max_fixed_point_iterations",
                "IR007",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ir/passes_core.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "fold_binary_const",
                "simplify_algebraic_identity",
                "can_eliminate_dead_instruction",
                "can_fold_branch",
            ],
            required_code_markers: &[
                "ConstPropAndFold",
                "DeadCodeElim",
                "CfgSimplify",
                "IDENTITY_ARG1",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ir/printer.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "print_physics",
                "print_instruction",
                "print_block_header",
                "print_function_header",
            ],
            required_code_markers: &[
                "print_physics",
                "print_instruction",
                "parse_ir_count_blocks",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ir/lower_ast.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "scope_cleanup_push",
                "scope_cleanup_count",
                "scope_cleanup_unwind_to_depth",
                "lower_short_circuit_and",
            ],
            required_code_markers: &[
                "lower_short_circuit_and",
                "lower_short_circuit_or",
                "lower_loop_header",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_ir(script: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let deps = [
        "stdlib/core/pending.pilot",
        "selfhost/ir/model.pilot",
        "selfhost/ir/verifier.pilot",
        "selfhost/ir/pass_manager.pilot",
        "selfhost/ir/passes_core.pilot",
        "selfhost/ir/printer.pilot",
        "selfhost/ir/lower_ast.pilot",
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

// -----------------------------------------------------------------------------
// Test 1: SSA Dominance & Block Parameter Lowering
// -----------------------------------------------------------------------------
#[test]
pub fn test_m6_1_ssa_dominance_and_block_parameter_lowering() {
    let guard = anticheat::AntiCheatGuard::arm("test_m6_1_ssa_dominance_and_block_parameter_lowering", &ANTICHEAT_SPEC);

    let script = r#"
        // 1. Build a 4-block dominator tree:
        // Block 0 (entry) dominates Block 1 and Block 2.
        // Block 3 is a join block dominated by Block 0.
        var idoms = encode_idom_table(4);
        idoms = set_idom(idoms, 1, 0); // Block 1 idom is 0
        idoms = set_idom(idoms, 2, 0); // Block 2 idom is 0
        idoms = set_idom(idoms, 3, 0); // Block 3 idom is 0

        // Test valid dominance: Block 0 dominates Block 1
        let dom_ok = verify_ssa_dominance(0, 1, 0, 0, idoms, 4);

        // Test invalid dominance: Block 1 does NOT dominate Block 2
        let dom_err = verify_ssa_dominance(1, 2, 0, 0, idoms, 4);

        // Test intra-block dominance: use before def in same block
        let intra_err = verify_ssa_dominance(1, 1, 5, 2, idoms, 4);

        // Test block terminator completeness
        let term_ok = verify_block_terminator("8|9|44|"); // add, sub, return
        let term_missing = verify_block_terminator("8|9|"); // missing terminator
        let term_dead = verify_block_terminator("44|8|"); // dead instruction after return

        // Test branch arity matching
        let arity_ok = verify_branch_arity(2, 2, 3);
        let arity_mismatch = verify_branch_arity(1, 2, 3);

        dom_ok + "|" + (if dom_err.slice(0, 5) == "IR001" { "IR001_OK" } else { "FAIL" }) + "|" +
        (if intra_err.slice(0, 5) == "IR001" { "IR001_INTRA_OK" } else { "FAIL" }) + "|" +
        term_ok + "|" + (if term_missing.slice(0, 5) == "IR003" { "IR003_MISSING_OK" } else { "FAIL" }) + "|" +
        (if term_dead.slice(0, 5) == "IR003" { "IR003_DEAD_OK" } else { "FAIL" }) + "|" +
        arity_ok + "|" + (if arity_mismatch.slice(0, 5) == "IR002" { "IR002_OK" } else { "FAIL" });
    "#;

    let res = eval_ir(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 8, "Expected 8 parts, got: {}", res);
    assert_eq!(parts[0], "OK", "Valid dominance failed");
    assert_eq!(parts[1], "IR001_OK", "Invalid dominance should trigger IR001");
    assert_eq!(parts[2], "IR001_INTRA_OK", "Use-before-def should trigger IR001");
    assert_eq!(parts[3], "OK", "Valid terminator check failed");
    assert_eq!(parts[4], "IR003_MISSING_OK", "Missing terminator should trigger IR003");
    assert_eq!(parts[5], "IR003_DEAD_OK", "Dead instructions after terminator should trigger IR003");
    assert_eq!(parts[6], "OK", "Branch arity match failed");
    assert_eq!(parts[7], "IR002_OK", "Branch arity mismatch should trigger IR002");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: IR Textual Printer & Round-Trip Parser
// -----------------------------------------------------------------------------
#[test]
pub fn test_m6_2_ir_printer_and_parser_round_trip() {
    let guard = anticheat::AntiCheatGuard::arm("test_m6_2_ir_printer_and_parser_round_trip", &ANTICHEAT_SPEC);

    let script = r#"
        let phy_i64 = ir_physics_i64();
        let phy_bool = ir_physics_bool();

        let inst1 = ir_inst_new(0, op_const_bits(), 0, phy_i64, 0, 0, 0);
        let inst2 = ir_inst_new(1, op_add(), 0, phy_i64, 0, 2, 1);
        let inst3 = ir_inst_new(2, op_return(), 0, phy_i64, 0, 1, 2);

        let s_fn = print_function_header("compute", "v0: gpr_signed[64:IValue]", phy_i64, true, false);
        let s_b0 = print_block_header(0, "v0: gpr_signed[64:IValue]");
        let s_i1 = print_instruction(inst1, "42");
        let s_i2 = print_instruction(inst2, "v0, v1");
        let s_i3 = print_instruction(inst3, "v2");

        let textual_ir = s_fn + "\n" + s_b0 + "\n" + s_i1 + "\n" + s_i2 + "\n" + s_i3 + "\n}";

        // Two blocks; the jump names `block1` without being a block header.
        let inst_j = ir_inst_new(3, op_jump(), 0, phy_i64, 0, 1, 3);
        let two_block_ir = s_fn + "\n" + s_b0 + "\n" + s_i1 + "\n" + print_instruction(inst_j, "block1(v0)") + "\n" +
            print_block_header(1, "v3: gpr_signed[64:IValue]") + "\n" + s_i3 + "\n}";
        let two_blocks = parse_ir_count_blocks(two_block_ir);
        let two_insts = parse_ir_count_instructions(two_block_ir);

        let header = lower_loop_header(loop_tier_fixed_scalar(), "i", phy_i64, phy_bool, "v0", "v1", 3);

        let block_cnt = parse_ir_count_blocks(textual_ir);
        let inst_cnt = parse_ir_count_instructions(textual_ir);

        // Scope cleanup unwinding test
        var clean_stack = scope_cleanup_new();
        clean_stack = scope_cleanup_push(clean_stack, "lease_a");
        clean_stack = scope_cleanup_push(clean_stack, "lease_b");
        let unwound = scope_cleanup_unwind_to_depth(clean_stack, 0);

        block_cnt + "|" + inst_cnt + "|" + unwound + "|" + two_blocks + "|" + two_insts + "|" + header;
    "#;

    let res = eval_ir(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 6, "Expected 6 parts, got: {}", res);
    assert_eq!(parts[0], "1", "Expected 1 block");
    assert_eq!(parts[1], "3", "Expected 3 instructions");
    // Entries pushed a then b unwind b then a.
    assert_eq!(parts[2], "lease_b;lease_a;", "LIFO scope unwinding failed");
    assert_eq!(parts[3], "2", "A jump naming block1 is not a block header");
    assert_eq!(parts[4], "3", "Expected 3 instructions in the two-block function");
    // The loop header carries physics, never a source type name.
    assert_eq!(
        parts[5],
        "block3(i: gpr_signed[64:IValue] = v0, i_end: gpr_signed[64:IValue] = v1):",
        "Fixed-scalar loop header"
    );

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Constant Folding & Dead Code Elimination
// -----------------------------------------------------------------------------
#[test]
pub fn test_m6_3_constant_folding_and_dead_code_elimination() {
    let guard = anticheat::AntiCheatGuard::arm("test_m6_3_constant_folding_and_dead_code_elimination", &ANTICHEAT_SPEC);

    let script = r#"
        // 1. Algebraic Identities:
        // x + 0 -> x
        let id_add0 = simplify_algebraic_identity(op_add(), false, 0, true, 0);
        // 0 + x -> x
        let id_0add = simplify_algebraic_identity(op_add(), true, 0, false, 0);
        // x * 1 -> x
        let id_mul1 = simplify_algebraic_identity(op_mul(), false, 0, true, 1);
        // x * 0 -> 0
        let id_mul0 = simplify_algebraic_identity(op_mul(), false, 0, true, 0);
        // x - 0 -> x
        let id_sub0 = simplify_algebraic_identity(op_sub(), false, 0, true, 0);

        // 2. Constant Arithmetic Folding:
        let fold_add = fold_binary_const(op_add(), 15, 27);
        let fold_mul = fold_binary_const(op_mul(), 7, 8);
        let fold_sub = fold_binary_const(op_sub(), 50, 18);

        // 3. Dead Code Elimination:
        let dce_unused_pure = can_eliminate_dead_instruction(op_add(), 0);
        let dce_used_pure = can_eliminate_dead_instruction(op_add(), 2);
        let dce_store = can_eliminate_dead_instruction(op_store(), 0); // stores have side effects!
        let dce_ret = can_eliminate_dead_instruction(op_return(), 0); // terminators have side effects!

        // 4. Fixed-Point Iteration Cap (IR007):
        let cap_ok = check_iteration_cap(12);
        let cap_exceeded = check_iteration_cap(65);

        id_add0 + "|" + id_0add + "|" + id_mul1 + "|" + id_mul0 + "|" + id_sub0 + "|" +
        fold_add + "|" + fold_mul + "|" + fold_sub + "|" +
        (if dce_unused_pure { "DCE_OK" } else { "FAIL" }) + "|" +
        (if !dce_used_pure { "RETAIN_USED" } else { "FAIL" }) + "|" +
        (if !dce_store { "RETAIN_STORE" } else { "FAIL" }) + "|" +
        (if !dce_ret { "RETAIN_TERM" } else { "FAIL" }) + "|" +
        cap_ok + "|" + (if cap_exceeded.slice(0, 5) == "IR007" { "IR007_OK" } else { "FAIL" });
    "#;

    let res = eval_ir(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 14, "Expected 14 parts, got: {}", res);
    assert_eq!(parts[0], "IDENTITY_ARG1", "x + 0 -> x identity failed");
    assert_eq!(parts[1], "IDENTITY_ARG2", "0 + x -> x identity failed");
    assert_eq!(parts[2], "IDENTITY_ARG1", "x * 1 -> x identity failed");
    assert_eq!(parts[3], "ZERO", "x * 0 -> 0 identity failed");
    assert_eq!(parts[4], "IDENTITY_ARG1", "x - 0 -> x identity failed");
    assert_eq!(parts[5], "42", "15 + 27 constant fold failed");
    assert_eq!(parts[6], "56", "7 * 8 constant fold failed");
    assert_eq!(parts[7], "32", "50 - 18 constant fold failed");
    assert_eq!(parts[8], "DCE_OK", "Unused arithmetic should be eliminated");
    assert_eq!(parts[9], "RETAIN_USED", "Used arithmetic must be retained");
    assert_eq!(parts[10], "RETAIN_STORE", "Memory store has side effects and must be retained");
    assert_eq!(parts[11], "RETAIN_TERM", "Terminators must be retained");
    assert_eq!(parts[12], "OK", "Iteration cap <= 64 should be OK");
    assert_eq!(parts[13], "IR007_OK", "Iteration cap > 64 should trigger IR007");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Name-Blind Physics & Multiplatform Target Invariance
// -----------------------------------------------------------------------------
#[test]
pub fn test_m6_4_name_blind_physics_and_multiplatform_target_invariance() {
    let guard = anticheat::AntiCheatGuard::arm("test_m6_4_name_blind_physics_and_multiplatform_target_invariance", &ANTICHEAT_SPEC);

    let script = r#"
        // 1. Name-Blind Type Physics & 2-Bit Lattice:
        let p_val = ir_physics_new(10, reg_class_gpr_signed(), 64, 3, lattice_value());
        let p_ref = ir_physics_new(11, reg_class_raw_ptr(), 64, 3, lattice_reference());
        let p_obj = ir_physics_new(12, reg_class_raw_ptr(), 64, 3, lattice_object());
        let p_guard = ir_physics_new(13, reg_class_raw_ptr(), 64, 3, lattice_guard());

        // Verifier: 2-bit lattice return boundary escape (IR004)
        let ret_val_ok = verify_return_lattice(p_val);
        let ret_ref_escape = verify_return_lattice(p_ref);
        let ret_guard_escape = verify_return_lattice(p_guard);

        // Verifier: noalloc function heap allocation check (IR006)
        let noalloc_pure_ok = verify_noalloc_op(true, op_add());
        let noalloc_alloc_err = verify_noalloc_op(true, op_arena_alloc());

        // 2. Multiplatform Target Invariance:
        let t_win = target_descriptor_new("windows", "x86_64");
        let t_linux = target_descriptor_new("linux", "x86_64");
        let t_arm = target_descriptor_new("darwin", "aarch64");
        let t_wasm = target_descriptor_new("wasi", "wasm32");

        let win_gprs = target_max_gpr_args(t_win);
        let win_shadow = target_caller_shadow_bytes(t_win);

        let linux_gprs = target_max_gpr_args(t_linux);
        let linux_shadow = target_caller_shadow_bytes(t_linux);

        let arm_gprs = target_max_gpr_args(t_arm);
        let wasm_gprs = target_max_gpr_args(t_wasm);

        ret_val_ok + "|" + (if ret_ref_escape.slice(0, 5) == "IR004" { "IR004_REF_OK" } else { "FAIL" }) + "|" +
        (if ret_guard_escape.slice(0, 5) == "IR004" { "IR004_GUARD_OK" } else { "FAIL" }) + "|" +
        noalloc_pure_ok + "|" + (if noalloc_alloc_err.slice(0, 5) == "IR006" { "IR006_OK" } else { "FAIL" }) + "|" +
        win_gprs + "|" + win_shadow + "|" + linux_gprs + "|" + linux_shadow + "|" + arm_gprs + "|" + wasm_gprs;
    "#;

    let res = eval_ir(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 11, "Expected 11 parts, got: {}", res);
    assert_eq!(parts[0], "OK", "IValue return should be OK");
    assert_eq!(parts[1], "IR004_REF_OK", "IReference return escape should trigger IR004");
    assert_eq!(parts[2], "IR004_GUARD_OK", "IGuard return escape should trigger IR004");
    assert_eq!(parts[3], "OK", "Pure op in noalloc should be OK");
    assert_eq!(parts[4], "IR006_OK", "Heap allocation in noalloc should trigger IR006");
    assert_eq!(parts[5], "4", "Win64 max GPR args should be 4");
    assert_eq!(parts[6], "32", "Win64 shadow bytes should be 32");
    assert_eq!(parts[7], "6", "SysV AMD64 max GPR args should be 6");
    assert_eq!(parts[8], "0", "SysV AMD64 shadow bytes should be 0");
    assert_eq!(parts[9], "8", "AAPCS64 max GPR args should be 8");
    assert_eq!(parts[10], "16", "WASM virtual local args should be 16");

    guard.finish();
}
