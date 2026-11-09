//! ============================================================================
//! Milestone 29 Verification Suite:
//! Capability-Sandboxed & Deterministic comptime Execution
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 29,
    test_file_rel: "crates/pilot-analysis/tests/milestone29_capability_sandboxed_comptime_execution.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/comptime/guardian.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "comptime_guardian_new",
                "guardian_check_dependency_grant",
            ],
            required_code_markers: &[
                "CTIME001",
                "CTIME002",
                "CTIME003",
                "CTIME004",
                "CTIME005",
                "CTIME006",
                "ComptimePolicy",
                "ComptimeGuardian",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/comptime/deterministic_env.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "clock_now_virtual",
                "normalize_virtual_path",
            ],
            required_code_markers: &[
                "DetPrng",
                "DetMapEntry",
                "DetMap",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/pkg/manifest.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "parse_manifest_json",
                "parse_comptime_allow_policy",
                "validate_dependency_comptime_needs",
            ],
            required_code_markers: &[
                "ComptimeAllowPolicy",
                "CTIME006",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m29(deps: &[&str], script: &str) -> String {
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
    vm.run().to_string_repr()
}

// -----------------------------------------------------------------------------
// Test 1: Unauthorized Filesystem Access & Path Traversal Blocked (CTIME001)
// -----------------------------------------------------------------------------
#[test]
fn test_m29_1_unauthorized_fs_and_path_traversal_blocked_ctime001() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m29_1_unauthorized_fs_and_path_traversal_blocked_ctime001",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "selfhost/comptime/guardian.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // Case 1: Sandbox with zero capabilities
            var g_zero = comptime_guardian_new(cap_none(), 100000, 1048576);

            // Attempting read without FsRead cap fails with CTIME001
            let d1 = g_zero.guardian_check_fs_read("src/config.json");
            if d1.len() == 0 { return 1; }
            let exp_ctime001 = "CTIME001";
            if d1.slice(0, exp_ctime001.len()) != exp_ctime001 { return 2; }

            // Case 2: Sandbox with FsRead granted
            var g_read = comptime_guardian_new(cap_fs_read(), 100000, 1048576);

            // Attempting path traversal ".." must be blocked by CTIME001!
            let d2 = g_read.guardian_check_fs_read("../../secret.txt");
            if d2.len() == 0 { return 3; }
            if d2.slice(0, exp_ctime001.len()) != exp_ctime001 { return 4; }

            // Attempting absolute host path "/etc/passwd" must be blocked!
            let d3 = g_read.guardian_check_fs_read("/etc/passwd");
            if d3.len() == 0 { return 5; }
            if d3.slice(0, exp_ctime001.len()) != exp_ctime001 { return 6; }

            // Valid relative read succeeds
            let d4 = g_read.guardian_check_fs_read("queries/users.sql");
            if d4.len() != 0 { return 7; }
            if g_read.accessed_files.len() != 1 { return 8; }

            return 42;
        }
    "#;

    let res = eval_m29(&deps, script);
    assert_eq!(res, "42", "Failed CTIME001 sandbox test: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Network, Subprocess & Non-Deterministic Clock Blocked (CTIME002 & CTIME003)
// -----------------------------------------------------------------------------
#[test]
fn test_m29_2_network_subprocess_and_clock_guards_ctime002_003() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m29_2_network_subprocess_and_clock_guards_ctime002_003",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "selfhost/comptime/guardian.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            var g = comptime_guardian_new(cap_fs_read(), 100000, 1048576); // No net, no subprocess

            // 1. Network attempt blocked with CTIME002
            let d_net = g.guardian_check_net("1.1.1.1", 80);
            if d_net.len() == 0 { return 1; }
            let exp_ctime002 = "CTIME002";
            if d_net.slice(0, exp_ctime002.len()) != exp_ctime002 { return 2; }

            // 2. Subprocess attempt blocked with CTIME002
            let d_proc = g.guardian_check_subprocess("curl evil.com");
            if d_proc.len() == 0 { return 3; }
            if d_proc.slice(0, exp_ctime002.len()) != exp_ctime002 { return 4; }

            // 3. Non-deterministic host clock / entropy blocked with CTIME003
            let d_clock = g.guardian_check_clock();
            if d_clock.len() == 0 { return 5; }
            let exp_ctime003 = "CTIME003";
            if d_clock.slice(0, exp_ctime003.len()) != exp_ctime003 { return 6; }

            return 42;
        }
    "#;

    let res = eval_m29(&deps, script);
    assert_eq!(res, "42", "Failed CTIME002/003 guards test: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Instruction Fuel Metering & Infinite Loop Trap (CTIME004)
// -----------------------------------------------------------------------------
#[test]
fn test_m29_3_fuel_exhaustion_infinite_loop_trap_ctime004() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m29_3_fuel_exhaustion_infinite_loop_trap_ctime004",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "selfhost/comptime/guardian.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // Test guardian fuel counter limit
            var g = comptime_guardian_new(cap_all(), 1000, 1048576); // 1000 instruction fuel limit

            // Consuming 500 steps is okay
            let ok_fuel = g.guardian_consume_fuel(500);
            if ok_fuel.len() != 0 { return 1; }

            // Consuming another 600 steps exceeds 1000 limit -> CTIME004!
            let err_fuel = g.guardian_consume_fuel(600);
            if err_fuel.len() == 0 { return 2; }
            let exp_ctime004 = "CTIME004";
            if err_fuel.slice(0, exp_ctime004.len()) != exp_ctime004 { return 3; }

            return 42;
        }
    "#;

    let res = eval_m29(&deps, script);
    assert_eq!(res, "42", "Failed CTIME004 fuel exhaustion test: {}", res);

    // Also verify PilVirtualMachine fuel trap on infinite loop in Rust
    let loop_src = "pub fn main() -> i64 { var x = 0; loop { x = x + 1; } return x; }";
    let arena = parse(loop_src);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    vm.fuel_limit = Some(500); // 500 fuel limit
    let _ = vm.run();
    assert!(vm.fuel_exhausted, "PilVirtualMachine must trap on infinite loop when fuel is exhausted");
    assert!(vm.fuel_used > 500, "PilVirtualMachine must record used fuel");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Deterministic Environment & Transitive Dependency Privilege Block (CTIME006)
// -----------------------------------------------------------------------------
#[test]
fn test_m29_4_deterministic_env_and_transitive_privilege_escalation_ctime006() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m29_4_deterministic_env_and_transitive_privilege_escalation_ctime006",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "selfhost/comptime/guardian.pilot",
        "selfhost/comptime/deterministic_env.pilot",
        "selfhost/pkg/manifest.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // 1. Virtual monotonic clock
            let v_time = clock_now_virtual(100);
            if v_time != 1000 { return 1; }

            // 2. Content-seeded PRNG is 100% deterministic
            var prng1 = DetPrng::from_seed(12345678);
            var prng2 = DetPrng::from_seed(12345678);
            let r1 = prng1.next_u64();
            let r2 = prng2.next_u64();
            if r1 != r2 { return 2; }

            // 3. Insertion-ordered deterministic map
            var m = det_map_new();
            m.det_map_set("alpha", 1);
            m.det_map_set("beta", 2);
            m.det_map_set("gamma", 3);
            let keys = m.det_map_keys_ordered();
            if keys.len() != 3 { return 3; }
            if keys[0] != "alpha" || keys[1] != "beta" || keys[2] != "gamma" { return 4; }

            // 4. Path normalization
            let norm = normalize_virtual_path(".\\src\\nested\\file.sql");
            if norm != "src/nested/file.sql" { return 5; }

            // 5. Transitive dependency privilege escalation block (CTIME006)
            let root_policy = parse_comptime_allow_policy("{\n  \"comptime\": {\n    \"allow\": [\"fs.read\"]\n  }\n}");
            if !root_policy.allow_fs_read || root_policy.allow_net { return 6; }

            // Dependency requests "net" which is not allowed by root policy -> CTIME006!
            var dep_needs: List<str> = [];
            dep_needs.push("net");
            let err_dep = validate_dependency_comptime_needs(root_policy, "evil_macro", dep_needs);
            if err_dep.len() == 0 { return 7; }
            let exp_ctime006 = "CTIME006";
            if err_dep.slice(0, exp_ctime006.len()) != exp_ctime006 { return 8; }

            return 42;
        }
    "#;

    let res = eval_m29(&deps, script);
    assert_eq!(res, "42", "Failed deterministic env and CTIME006 test: {}", res);
    guard.finish();
}
