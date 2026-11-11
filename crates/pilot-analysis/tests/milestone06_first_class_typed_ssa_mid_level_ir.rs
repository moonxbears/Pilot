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
            path: "selfhost/ir/passes.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["pass_register", "default_passes", "run_passes"],
            required_code_markers: &["IPilIrPass", "IR007", "verify_ir_function"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ir/pass_fold.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["fold_function"],
            required_code_markers: &["ConstFoldPass", "op_cond_br", "holds"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ir/pass_dce.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["eliminate_dead_code"],
            required_code_markers: &["DeadCodePass", "op_unreachable", "value_positions"],
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
    ],
    required_group_pub_fns: &[],
};

fn eval_ir(script: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let deps = [
        "stdlib/core/pending.pilot",
        "selfhost/ir/model.pilot",
        "selfhost/ir/verifier.pilot",
        "selfhost/ir/printer.pilot",
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

        let block_cnt = parse_ir_count_blocks(textual_ir);
        let inst_cnt = parse_ir_count_instructions(textual_ir);

        block_cnt + "|" + inst_cnt + "|" + two_blocks + "|" + two_insts;
    "#;

    let res = eval_ir(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 4, "Expected 4 parts, got: {}", res);
    assert_eq!(parts[0], "1", "Expected 1 block");
    assert_eq!(parts[1], "3", "Expected 3 instructions");
    assert_eq!(parts[2], "2", "A jump naming block1 is not a block header");
    assert_eq!(parts[3], "3", "Expected 3 instructions in the two-block function");

    // The real PilIR text (selfhost/ir/text.pilot): printing a lowered
    // program, parsing that text and printing it again gives the same bytes
    // (`pilot ir roundtrip` fails otherwise), for hand-written IR and for
    // programs lowered from source. F5 deleted the text-template lowering
    // (selfhost/ir/lower_ast.pilot) this test used to call.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().to_path_buf();
    let bin = root.join("target/debug/pilot");
    if !bin.exists() {
        let st = std::process::Command::new("cargo")
            .args(["build", "-q", "-p", "pilot-cli"])
            .current_dir(&root)
            .status()
            .expect("cargo build -p pilot-cli");
        assert!(st.success(), "Failed building pilot-cli binary");
    }
    for file in ["tests/ir/loop_params.pir", "tests/conformance/closures.pilot", "tests/conformance/guard_try.pilot", "tests/conformance/defer.pilot"] {
        let out = std::process::Command::new(&bin)
            .args(["ir", "roundtrip", file])
            .current_dir(&root)
            .output()
            .expect("run pilot ir roundtrip");
        assert_eq!(out.status.code(), Some(0), "{file}: {}", String::from_utf8_lossy(&out.stderr));
    }

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Constant Folding & Dead Code Elimination
// -----------------------------------------------------------------------------
// Constant folding and dead-code elimination run on real PilIR
// (selfhost/ir/pass_fold.pilot and pass_dce.pilot, driven by passes.pilot),
// reached through `pilot ir opt` / `pilot ir run`. More cases are in
// crates/pilot-cli/tests/cli_f5.rs.
#[test]
pub fn test_m6_3_constant_folding_and_dead_code_elimination() {
    let guard = anticheat::AntiCheatGuard::arm("test_m6_3_constant_folding_and_dead_code_elimination", &ANTICHEAT_SPEC);

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().to_path_buf();
    let bin = root.join("target/debug/pilot");
    if !bin.exists() {
        let st = std::process::Command::new("cargo")
            .args(["build", "-q", "-p", "pilot-cli"])
            .current_dir(&root)
            .status()
            .expect("cargo build -p pilot-cli");
        assert!(st.success(), "Failed building pilot-cli binary");
    }
    let run = |args: &[&str]| {
        let out = std::process::Command::new(&bin).args(args).current_dir(&root).output().expect("run pilot");
        assert_eq!(out.status.code(), Some(0), "pilot {:?}: {}", args, String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).into_owned()
    };

    // tests/ir/loop_params.pir prints 6 * 7, then sums 0..5 in a loop whose
    // values travel in block parameters. Hand-worked: the product is the
    // pool's new fifth entry, 42; the constant 7 feeds nothing else and goes;
    // the loop, which depends on its parameters, is left alone.
    let opt = run(&["ir", "opt", "tests/ir/loop_params.pir"]);
    assert!(opt.contains("int 4 42\n"), "42 is pooled: {opt}");
    assert!(opt.contains("v2 = const_bits #4 :1 !0\n"), "6 * 7 folds: {opt}");
    assert!(!opt.contains(" mul "), "no multiplication is left: {opt}");
    assert!(!opt.contains("v1 = "), "the unused constant 7 is removed: {opt}");
    assert!(opt.contains("v10 = add v5, v9 :1 !0\n"), "the loop step stays: {opt}");

    // The optimised and unoptimised modules print the same.
    assert_eq!(run(&["ir", "run", "tests/ir/loop_params.pir"]), "42\n15\n");
    assert_eq!(run(&["ir", "run-unopt", "tests/ir/loop_params.pir"]), "42\n15\n");

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
