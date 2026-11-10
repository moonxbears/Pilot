//! ============================================================================
//! Milestone 28 Verification Suite:
//! 4-Tier Progressive Learning Profiles & Friendly Compiler Guidance
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
    milestone_id: 28,
    test_file_rel: "crates/pilot-analysis/tests/milestone28_progressive_learning_profiles_and_coach.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "stdlib/pilot/profile.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "learning_tier_name",
                "profile_for_tier",
            ],
            required_code_markers: &[
                "LearningTier",
                "ClearanceRing",
                "ProfileDescriptor",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/diag/progressive_coach.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "adapt_escape001",
                "analyze_struct_padding",
            ],
            required_code_markers: &[
                "COACH001",
                "COACH002",
                "PERF001",
                "PERF002",
                "PERF003",
                "AdaptedDiagnostic",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/types/clearance_flow.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "clearance_solver_new",
                "check_clearance_access",
            ],
            required_code_markers: &[
                "LVL001",
                "ClearanceFrame",
                "ClearanceSolver",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m28(deps: &[&str], script: &str) -> String {
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
// Test 1: Progressive Diagnostic Adaptation Across All 4 Tiers (ESCAPE001)
// -----------------------------------------------------------------------------
#[test]
fn test_m28_1_progressive_diagnostic_adaptation_across_tiers() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m28_1_progressive_diagnostic_adaptation_across_tiers",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "stdlib/pilot/profile.pilot",
        "selfhost/diag/progressive_coach.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // Case 1: Tier 1 Everyday -> Plain everyday English, no bitcodes
            let d1 = adapt_escape001(LearningTier::Tier1Everyday, "names", "get_names", 42);
            if d1.error_code != "ESCAPE001" { return 1; }
            if d1.fix_suggestion != "return 'names' directly" { return 2; }
            // Verify title is friendly
            let exp_t1 = "Cannot return a slice of 'names'";
            if d1.title.slice(0, exp_t1.len()) != exp_t1 { return 3; }

            // Case 2: Tier 2 Domain -> Architectural phrasing
            let d2 = adapt_escape001(LearningTier::Tier2Domain, "names", "get_names", 42);
            if d2.error_code != "ESCAPE001" { return 4; }
            let exp_t2 = "Borrowed slice<T> escapes lexical scope";
            if d2.title.slice(0, exp_t2.len()) != exp_t2 { return 5; }

            // Case 3: Tier 3 Systems -> Full hardware & lattice precision
            let d3 = adapt_escape001(LearningTier::Tier3Systems, "names", "get_names", 42);
            if d3.error_code != "ESCAPE001" { return 6; }
            let exp_t3 = "ESCAPE001: 0b01 borrowed view escapes lexical owner span";
            if d3.title != exp_t3 { return 7; }
            if d3.explanation != "Cannot outlive scope of 0b10 local buffer 'names'; Perceus drop occurs at line 42" { return 8; }

            // Case 4: Non-bifurcation verification (same code across all tiers)
            if d1.error_code != d3.error_code { return 9; }

            return 42;
        }
    "#;

    let res = eval_m28(&deps, script);
    assert_eq!(res, "42", "Failed progressive diagnostic adaptation test: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: PerfCoach Struct Field Padding Waste Analysis (PERF002)
// -----------------------------------------------------------------------------
#[test]
fn test_m28_2_perf_coach_struct_padding_waste_perf002() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m28_2_perf_coach_struct_padding_waste_perf002",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "stdlib/pilot/profile.pilot",
        "selfhost/diag/progressive_coach.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // struct BadPad { a: u8; b: i64; c: u8; d: i64; }
            // Natural order:
            // a: 1B + 7B pad -> 8B
            // b: 8B -> 16B
            // c: 1B + 7B pad -> 24B
            // d: 8B -> 32B total (8B waste over 24B optimal!)
            var fields: List<FieldLayoutInfo> = [];
            fields.push(field_layout_info_new("a", 1, 1));
            fields.push(field_layout_info_new("b", 8, 8));
            fields.push(field_layout_info_new("c", 1, 1));
            fields.push(field_layout_info_new("d", 8, 8));

            let diag = analyze_struct_padding("BadPad", fields);
            if diag.len() == 0 { return 1; }

            if diag != "PERF002: Struct 'BadPad' wastes 8 bytes in padding (32B -> 24B); reorder fields by descending alignment" { return 2; }

            // 16-byte alignment (u128 on x86_64 SysV): { u8, u128, u8 } is 48B in
            // declared order and 32B reordered (rustc/clang give the same sizes).
            var wide: List<FieldLayoutInfo> = [];
            wide.push(field_layout_info_new("x", 1, 1));
            wide.push(field_layout_info_new("y", 16, 16));
            wide.push(field_layout_info_new("z", 1, 1));
            let wide_diag = analyze_struct_padding("Wide", wide);
            if wide_diag != "PERF002: Struct 'Wide' wastes 16 bytes in padding (48B -> 32B); reorder fields by descending alignment" { return 4; }

            // Well-ordered struct (no waste): struct GoodPad { b: i64; d: i64; a: u8; c: u8; }
            var good_fields: List<FieldLayoutInfo> = [];
            good_fields.push(field_layout_info_new("b", 8, 8));
            good_fields.push(field_layout_info_new("d", 8, 8));
            good_fields.push(field_layout_info_new("a", 1, 1));
            good_fields.push(field_layout_info_new("c", 1, 1));

            let good_diag = analyze_struct_padding("GoodPad", good_fields);
            if good_diag.len() != 0 { return 3; }

            return 42;
        }
    "#;

    let res = eval_m28(&deps, script);
    assert_eq!(res, "42", "Failed struct padding waste test: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: PerfCoach Loop Clone (PERF001) & Architectural Guidance (COACH001/002)
// -----------------------------------------------------------------------------
#[test]
fn test_m28_3_perf_coach_loop_clone_perf001_and_arch_coaching() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m28_3_perf_coach_loop_clone_perf001_and_arch_coaching",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "stdlib/pilot/profile.pilot",
        "selfhost/diag/progressive_coach.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // Case 1: PERF001 message
            let p1 = format_perf001("items", "List<f64>");
            if p1 != "PERF001: Avoidable 0b10 .clone() of 'items: List<f64>' inside loop; borrow via ref<List<f64>> for zero allocations" { return 1; }

            // Case 2: COACH001 on manual equality boilerplate
            let c1 = format_coach001("UserRecord");
            let exp_coach1 = "COACH001";
            if c1.slice(0, exp_coach1.len()) != exp_coach1 { return 4; }

            // Case 3: COACH002 on repetitive mapping funcs
            let c2 = format_coach002("UserDto", "UserEntity");
            let exp_coach2 = "COACH002";
            if c2.slice(0, exp_coach2.len()) != exp_coach2 { return 5; }

            // Case 4: PERF003 on AoS -> SoA layout speedup
            let p3 = format_perf003("Particle", 16);
            let exp_perf3 = "PERF003";
            if p3.slice(0, exp_perf3.len()) != exp_perf3 { return 6; }

            return 42;
        }
    "#;

    let res = eval_m28(&deps, script);
    assert_eq!(res, "42", "Failed coaching & perf hints test: {}", res);

    // Finding clones inside loops needs PilIR function bodies.
    let loop_script = r#"
        pub fn main() -> i64 {
            let diags = analyze_loop_clones("sum_all");
            return diags.len() as i64;
        }
    "#;
    let loop_res = eval_m28(&deps, loop_script);
    assert!(
        loop_res.starts_with("FAULT: SEAM001: unconnected seam `coach.loop_clones`"),
        "loop clone analysis must stop at its seam: {}",
        loop_res
    );
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Flow-Sensitive Clearance Ring Solver (LVL001 & Audit Trail)
// -----------------------------------------------------------------------------
#[test]
fn test_m28_4_flow_sensitive_clearance_ring_solver() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m28_4_flow_sensitive_clearance_ring_solver",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "selfhost/types/clearance_flow.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // 1. Initial base level: lvl(1)
            var solver = clearance_solver_new(1);
            if solver.current_clearance() != 1 { return 1; }

            // 2. Accessing target requiring lvl(2) fails with LVL001!
            let denied = solver.check_clearance_access(2, "PrivilegedHardwareIo");
            if denied.len() == 0 { return 2; }
            let exp_lvl001 = "LVL001";
            if denied.slice(0, exp_lvl001.len()) != exp_lvl001 { return 3; }

            // 3. Enter elevated scope (e.g. `if lvl >= 2 { ... }`)
            solver.enter_elevated_scope(2, "if lvl >= 2 branch");
            if solver.current_clearance() != 2 { return 4; }

            // 4. Access now permitted
            let allowed = solver.check_clearance_access(2, "PrivilegedHardwareIo");
            if allowed.len() != 0 { return 5; }

            // 5. Exit elevated scope restores previous level
            solver.exit_elevated_scope();
            if solver.current_clearance() != 1 { return 6; }

            // 6. Access denied again
            let denied_again = solver.check_clearance_access(2, "PrivilegedHardwareIo");
            if denied_again.len() == 0 { return 7; }

            // 7. Verify audit trail records transitions
            if solver.audit_trail.len() != 2 { return 8; }
            if solver.audit_trail[0] != "Elevated clearance from lvl1 to lvl2 (if lvl >= 2 branch)" { return 9; }
            if solver.audit_trail[1] != "Restored clearance to lvl1 on exit from 'if lvl >= 2 branch'" { return 10; }

            // 8. A later scope sees its own level, not a stale one from an exited scope
            solver.enter_elevated_scope(3, "first");
            solver.exit_elevated_scope();
            solver.enter_elevated_scope(2, "second");
            if solver.current_clearance() != 2 { return 11; }
            solver.exit_elevated_scope();
            if solver.current_clearance() != 1 { return 12; }

            return 42;
        }
    "#;

    let res = eval_m28(&deps, script);
    assert_eq!(res, "42", "Failed flow-sensitive clearance solver test: {}", res);
    guard.finish();
}
