//! ============================================================================
//! Milestone 26 Verification Suite:
//! Compile-Time Typestates (state) & Design-by-Contract (requires / ensures / old)
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
    milestone_id: 26,
    test_file_rel: "crates/pilot-analysis/tests/milestone26_typestates_and_design_by_contract.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/types/typestate.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "typestate_decl_new",
                "typestate_check_call",
                "typestate_check_drop",
            ],
            required_code_markers: &[
                "STATE001",
                "STATE002",
                "STATE003",
                "STATE004",
                "STATE005",
                "StateSetMask",
                "TransitionSig",
                "TypestateDecl",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ir/pass_contracts.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "contract_weave_plan_new",
                "weave_contracts",
                "verify_where_refinement_i64",
            ],
            required_code_markers: &[
                "DBC001",
                "DBC002",
                "DBC003",
                "DBC004",
                "DBC005",
                "ContractCheckMode",
                "ContractClause",
                "OldSnapshotDesc",
                "ContractWeavePlan",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m26(deps: &[&str], script: &str) -> String {
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
// Test 1: Zero-Byte Typestate Algebra & Transitions (File@Closed -> File@Open)
// -----------------------------------------------------------------------------
#[test]
fn test_m26_1_zero_byte_typestate_algebra_and_transitions() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m26_1_zero_byte_typestate_algebra_and_transitions",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "selfhost/types/typestate.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            var decl = typestate_decl_new("File");
            let s_closed = decl.typestate_register_state("Closed", true);  // idx 0, terminal
            let s_open   = decl.typestate_register_state("Open", false);   // idx 1, non-terminal

            let mask_closed = StateSetMask::from_index(s_closed);
            let mask_open   = StateSetMask::from_index(s_open);

            // Register transitions
            // open(): Closed -> Open
            decl.typestate_register_transition("open", mask_closed, mask_open, mask_closed, false, true);
            // read(): Open -> Open
            decl.typestate_register_transition("read", mask_open, mask_open, mask_open, false, false);
            // close(): Open -> Closed
            decl.typestate_register_transition("close", mask_open, mask_closed, mask_closed, true, false);

            // 1. Initial state: Closed
            var cur_state = mask_closed;

            // 2. Call open(): Closed -> Open succeeds
            let res_open = typestate_check_call(decl, cur_state, "open");
            if res_open.has_error { return 1; }
            cur_state = res_open.ok_next_state;
            if !cur_state.mask_contains(s_open) { return 2; }

            // 3. Call read(): Open -> Open succeeds
            let res_read = typestate_check_call(decl, cur_state, "read");
            if res_read.has_error { return 3; }
            cur_state = res_read.ok_next_state;

            // 4. Call close(): Open -> Closed succeeds
            let res_close = typestate_check_call(decl, cur_state, "close");
            if res_close.has_error { return 4; }
            cur_state = res_close.ok_next_state;
            if !cur_state.mask_contains(s_closed) { return 5; }

            // 5. Calling read() while in Closed must fail with STATE001!
            let bad_read = typestate_check_call(decl, cur_state, "read");
            if !bad_read.has_error { return 6; }
            if !bad_read.diagnostic.starts_with("STATE001: Invalid method call 'read' on 'File@Closed'; valid states: Open") { return 7; }

            return 42;
        }
    "#;

    let res = eval_m26(&deps, script);
    assert_eq!(res, "42", "Failed typestate algebra test: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Branch Merge Ambiguity (STATE002) & Drop Obligation (STATE004)
// -----------------------------------------------------------------------------
#[test]
fn test_m26_2_branch_merge_ambiguity_and_terminal_drop_obligations() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m26_2_branch_merge_ambiguity_and_terminal_drop_obligations",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "selfhost/types/typestate.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            var decl = typestate_decl_new("Tx");
            let s_open      = decl.typestate_register_state("Open", false);       // idx 0
            let s_committed = decl.typestate_register_state("Committed", true);   // idx 1, terminal
            let s_aborted   = decl.typestate_register_state("Aborted", true);     // idx 2, terminal

            let mask_open      = StateSetMask::from_index(s_open);
            let mask_committed = StateSetMask::from_index(s_committed);
            let mask_aborted   = StateSetMask::from_index(s_aborted);

            decl.typestate_register_transition("exec_sql", mask_open, mask_open, mask_open, false, false);
            decl.typestate_register_transition("commit", mask_open, mask_committed, mask_aborted, true, true);
            decl.typestate_register_transition("rollback", mask_open, mask_aborted, mask_aborted, true, false);

            // 1. Branch merge: then-branch commits, else-branch stays open
            let merged = typestate_merge_branches(mask_committed, mask_open);
            if !merged.mask_contains(s_open) || !merged.mask_contains(s_committed) { return 1; }

            // 2. Calling exec_sql on merged (Open | Committed) must trigger STATE002 ambiguous merge!
            let call_res = typestate_check_call(decl, merged, "exec_sql");
            if !call_res.has_error || !call_res.is_ambiguous { return 2; }
            if !call_res.diagnostic.starts_with("STATE002: Ambiguous merged typestate 'Tx@(Open | Committed)' at call to 'exec_sql'; narrow with a switch branch") { return 3; }

            // 3. Drop obligation: Tx@Open dropped without reaching Committed or Aborted -> STATE004!
            let drop_open_diag = typestate_check_drop(decl, mask_open);
            if drop_open_diag.len() == 0 { return 4; }
            if drop_open_diag != "STATE004: Typestate obligation unfulfilled: 'Tx@Open' dropped without reaching declared terminal state 'Committed | Aborted'" { return 5; }

            // 4. Drop obligation satisfied: Tx@Committed dropped -> no diagnostic
            let drop_term_diag = typestate_check_drop(decl, mask_committed);
            if drop_term_diag.len() != 0 { return 6; }

            return 42;
        }
    "#;

    let res = eval_m26(&deps, script);
    assert_eq!(res, "42", "Failed branch merge & drop obligation test: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Design-by-Contract Hoisted old() Snapshots & Impurity Diagnostics
// -----------------------------------------------------------------------------
#[test]
fn test_m26_3_design_by_contract_hoisted_old_snapshots_and_purity() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m26_3_design_by_contract_hoisted_old_snapshots_and_purity",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "selfhost/ir/pass_contracts.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // Case 1: Valid Account.deposit contract plan
            var plan = contract_weave_plan_new("deposit", ContractCheckMode::All);
            plan.plan_add_requires("amt > 0", true);
            plan.plan_add_old_snapshot(".balance", true, true); // pure, 0b00 IValue
            plan.plan_add_ensures(".balance == __old_0 + amt", true);

            let ok = check_contract_plan(plan);
            if !ok { return 1; }
            if plan.requires_checked != 1 { return 2; }
            if plan.old_snapshots_hoisted != 1 { return 3; }
            if plan.ensures_checked != 1 { return 4; }
            if plan.diagnostics.len() != 0 { return 5; }

            // Case 2: Impure expression in old() must trigger DBC003
            var plan_impure = contract_weave_plan_new("bad_old", ContractCheckMode::All);
            plan_impure.plan_add_old_snapshot("fetch_api()", false, true); // impure!
            let ok2 = weave_contracts(plan_impure);
            if ok2 { return 6; }
            if plan_impure.diagnostics.len() != 1 { return 7; }
            if plan_impure.diagnostics[0] != "DBC003: Impure expression inside old(...) forbidden: 'fetch_api()'" { return 8; }

            // Case 3: Borrowed 0b01 view captured in old() must trigger DBC004
            var plan_borrow = contract_weave_plan_new("bad_borrow", ContractCheckMode::All);
            plan_borrow.plan_add_old_snapshot("borrowed_slice", true, false); // not value type!
            let ok3 = weave_contracts(plan_borrow);
            if ok3 { return 9; }
            if plan_borrow.diagnostics.len() != 1 { return 10; }
            if !plan_borrow.diagnostics[0].starts_with("DBC004: old(...) captures mutated 0b01 borrowed view 'borrowed_slice'") { return 11; }

            return 42;
        }
    "#;

    let res = eval_m26(&deps, script);
    assert_eq!(res, "42", "Failed DBC old() snapshot & purity test: {}", res);

    // A plan that passes its checks is handed to PilIR, which does not exist yet.
    let weave_script = r#"
        pub fn main() -> i64 {
            var plan = contract_weave_plan_new("deposit", ContractCheckMode::All);
            plan.plan_add_requires("amt > 0", true);
            plan.plan_add_ensures(".balance == __old_0 + amt", true);
            if weave_contracts(plan) { return 1; }
            return 2;
        }
    "#;
    let weave = eval_m26(&deps, weave_script);
    assert!(
        weave.starts_with("FAULT: SEAM001: unconnected seam `contracts.weave_ir`"),
        "weave_contracts must stop at its seam: {}",
        weave
    );
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Comptime Precondition Violation, Refinement Types & Invariants
// -----------------------------------------------------------------------------
#[test]
fn test_m26_4_comptime_precondition_refinement_and_invariant() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m26_4_comptime_precondition_refinement_and_invariant",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "selfhost/ir/pass_contracts.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // Case 1: Refinement type Percentage = f64 where self >= 0 && self <= 100
            let ref_valid = verify_where_refinement_i64("Percentage", 75, 0, 100, "self >= 0 && self <= 100");
            if ref_valid.len() != 0 { return 4; }
            let ref_low = verify_where_refinement_i64("Percentage", 0, 0, 100, "self >= 0 && self <= 100");
            if ref_low.len() != 0 { return 9; }

            let ref_invalid = verify_where_refinement_i64("Percentage", 150, 0, 100, "self >= 0 && self <= 100");
            if ref_invalid != "DBC001: Compile-time provable refinement 'Percentage' violation: 'self >= 0 && self <= 100' evaluated to false (value 150 out of bounds [0..100])" { return 6; }
            let ref_neg = verify_where_refinement_i64("Percentage", -1, 0, 100, "self >= 0 && self <= 100");
            if ref_neg.len() == 0 { return 10; }

            // Case 2: Struct invariant checking
            var plan_inv = contract_weave_plan_new("Account", ContractCheckMode::All);
            plan_inv.plan_add_invariant(".balance >= 0", true);
            let ok_inv = check_contract_plan(plan_inv);
            if !ok_inv { return 7; }
            if plan_inv.invariants_woven != 1 { return 8; }

            return 42;
        }
    "#;

    let res = eval_m26(&deps, script);
    assert_eq!(res, "42", "Failed comptime precondition & refinement test: {}", res);

    // Proving `requires(amt > 0)` false for `deposit(-5)` needs a comptime evaluator.
    let prove_script = r#"
        pub fn main() -> i64 {
            var plan = contract_weave_plan_new("deposit", ContractCheckMode::All);
            plan.plan_add_requires("amt > 0", true);
            let args: List<str> = ["-5"];
            if prove_requires_at_call(plan, 0, args) { return 1; }
            return 2;
        }
    "#;
    let prove = eval_m26(&deps, prove_script);
    assert!(
        prove.starts_with("FAULT: SEAM001: unconnected seam `contracts.prove_requires`"),
        "requires proving must stop at its seam: {}",
        prove
    );
    guard.finish();
}
