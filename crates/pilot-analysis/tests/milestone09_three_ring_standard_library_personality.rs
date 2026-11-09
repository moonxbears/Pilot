//! ============================================================================
//! Milestone 9 Verification Suite:
//! Three-Ring Standard Library Personality (`core`, `std`, `pilot`)
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 9,
    test_file_rel: "crates/pilot-analysis/tests/milestone09_three_ring_standard_library_personality.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/loader/ring_verifier.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "ring_name",
                "parse_pkg_ring",
                "verify_ring_dependency",
                "verify_freestanding_source",
                "check_legacy_import_migration",
                "scan_and_migrate_ring_imports",
                "verify_no_stale_stdlib_cache",
            ],
            required_code_markers: &[
                "RingCheckResult",
                "verify_ring_dependency",
                "verify_freestanding_source",
                "check_legacy_import_migration",
                "scan_and_migrate_ring_imports",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_ring(script: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let deps = [
        "selfhost/diagnostics/model.pilot",
        "selfhost/diagnostics/renderer.pilot",
        "selfhost/cst/autofix.pilot",
        "selfhost/loader/ring_verifier.pilot",
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
    vm.run().to_string_repr()
}

// -----------------------------------------------------------------------------
// Test 1: Three-Ring Hierarchy & Illegal Upward Dependency Rejection (RING001)
// -----------------------------------------------------------------------------
#[test]
pub fn test_m9_1_three_ring_hierarchy_and_illegal_upward_dependency_rejection() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m9_1_three_ring_hierarchy_and_illegal_upward_dependency_rejection",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        let r1 = parse_pkg_ring("core.mem");
        let r2 = parse_pkg_ring("std.col");
        let r3 = parse_pkg_ring("pilot.ast");
        let r4 = parse_pkg_ring("Pilot.SelfHost.Analyzer");
        let r0 = parse_pkg_ring("App.Billing.Account");

        // Illegal: Ring 1 (core) -> Ring 2 (std)
        let chk_up1 = verify_ring_dependency("core.ops", "std.io");

        // Illegal: Ring 1 (core) -> Ring 3 (pilot)
        let chk_up2 = verify_ring_dependency("core.simd", "pilot.ir");

        // Illegal: Ring 2 (std) -> SelfHost
        let chk_sh = verify_ring_dependency("std.net", "Pilot.SelfHost.Analyzer");

        // Legal: User -> std, User -> core, std -> core, pilot -> std
        let chk_ok1 = verify_ring_dependency("App.Main", "std.http");
        let chk_ok2 = verify_ring_dependency("std.col", "core.mem");
        let chk_ok3 = verify_ring_dependency("pilot.cst", "std.text");

        ("" + r1 + ":" + r2 + ":" + r3 + ":" + r4 + ":" + r0 + "|" + (if !chk_up1.passed && chk_up1.code == "RING001" { "UP1_REJECTED" } else { "FAIL" }) + "|" + (if !chk_up2.passed && chk_up2.code == "RING001" { "UP2_REJECTED" } else { "FAIL" }) + "|" + (if !chk_sh.passed && chk_sh.code == "RING001" { "SH_REJECTED" } else { "FAIL" }) + "|" + (if chk_ok1.passed && chk_ok2.passed && chk_ok3.passed { "LEGAL_OK" } else { "FAIL" }));
    "#;

    let res = eval_ring(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 5, "Expected 5 parts, got: {}", res);
    assert_eq!(parts[0], "1:2:3:4:0", "Ring classifier must map core=1, std=2, pilot=3, selfhost=4, user=0");
    assert_eq!(parts[1], "UP1_REJECTED", "core -> std must be rejected with RING001");
    assert_eq!(parts[2], "UP2_REJECTED", "core -> pilot must be rejected with RING001");
    assert_eq!(parts[3], "SH_REJECTED", "std -> SelfHost must be rejected with RING001");
    assert_eq!(parts[4], "LEGAL_OK", "Legal downward dependencies must pass");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Freestanding Core Boundary & Host Intrinsic Exclusion (RING002)
// -----------------------------------------------------------------------------
#[test]
pub fn test_m9_2_freestanding_core_boundary_and_host_intrinsic_exclusion() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m9_2_freestanding_core_boundary_and_host_intrinsic_exclusion",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        let bad_src1 = "pub pkg core.bad;\nuse std.net;\npub fn foo() => 0;\n";
        let bad_src2 = "pub pkg core.bad;\npub fn leak() => __engine(\"fs_open\", \"data.bin\");\n";
        let bad_src3 = "pub pkg core.bad;\npub fn socket() => __engine(\"tcp_connect\", \"127.0.0.1\");\n";

        let chk_bad1 = verify_freestanding_source(bad_src1, "core.bad");
        let chk_bad2 = verify_freestanding_source(bad_src2, "core.bad");
        let chk_bad3 = verify_freestanding_source(bad_src3, "core.bad");

        let good_core = "pub pkg core.mem;\npub pure fn add(a: i64, b: i64) -> i64 => a + b;\n";
        let chk_good = verify_freestanding_source(good_core, "core.mem");

        (chk_bad1.code + "|" + chk_bad2.code + "|" + chk_bad3.code + "|" + (if chk_good.passed { "FREESTANDING_OK" } else { "FAIL" }));
    "#;

    let res = eval_ring(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 4, "Expected 4 parts, got: {}", res);
    assert_eq!(parts[0], "RING002", "Importing std in core must trigger RING002");
    assert_eq!(parts[1], "RING002", "Calling fs_open in core must trigger RING002");
    assert_eq!(parts[2], "RING002", "Calling tcp_connect in core must trigger RING002");
    assert_eq!(parts[3], "FREESTANDING_OK", "Freestanding core module must pass");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Retired Python-Clone Import Detection & Lossless Auto-Fix (RING003)
// -----------------------------------------------------------------------------
#[test]
pub fn test_m9_3_retired_python_clone_import_detection_and_lossless_autofix() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m9_3_retired_python_clone_import_detection_and_lossless_autofix",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        let src = "// Header comment\nuse std.unittest;\nuse std.tomllib;\nuse std.pathlib;\nuse std.pickle;\nuse std.urllib;\nuse std.asyncio;\nuse std.itertools;\n// Tail comment\n";

        let diags = scan_and_migrate_ring_imports(src, 1);
        let diag_count = diags.len;

        var fixes: List<FixSuggestion> = [];
        var i = 0;
        while i < diags.len {
            let d = diags[i];
            var j = 0;
            while j < d.fixes.len {
                fixes.push(d.fixes[j]);
                j = j + 1;
            }
            i = i + 1;
        }

        let fix_res = apply_file_fixes_lossless(src, fixes, 3);
        let fixed = fix_res.new_src;

        let has_test = fixed.find("use std.test;") >= 0;
        let has_toml = fixed.find("use std.toml;") >= 0;
        let has_path = fixed.find("use std.path;") >= 0;
        let has_bin  = fixed.find("use std.bin;") >= 0;
        let has_net  = fixed.find("use std.net;") >= 0;
        let has_task = fixed.find("use std.task;") >= 0;
        let has_col  = fixed.find("use std.col;") >= 0;

        let comments_ok = verify_comment_conservation(src, fixed);

        ("" + diag_count + "|" + fix_res.fixes_applied + "|" + (if has_test && has_toml && has_path && has_bin && has_net && has_task && has_col { "ALL_MIGRATED" } else { "MIGRATION_MISSING" }) + "|" + (if comments_ok { "COMMENTS_PRESERVED" } else { "COMMENTS_LOST" }));
    "#;

    let res = eval_ring(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 4, "Expected 4 parts, got: {}", res);
    assert_eq!(parts[0], "7", "Expected 7 retired Python-clone imports detected");
    assert_eq!(parts[1], "7", "Expected 7 lossless fixes applied");
    assert_eq!(parts[2], "ALL_MIGRATED", "All legacy imports must be migrated to canonical std.*");
    assert_eq!(parts[3], "COMMENTS_PRESERVED", "Comments must be preserved verbatim");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Canonical `std.*` Packages & Stale Cache Rejection (RING004)
// -----------------------------------------------------------------------------
#[test]
pub fn test_m9_4_canonical_std_packages_and_stale_cache_rejection() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m9_4_canonical_std_packages_and_stale_cache_rejection",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        // 1. RING004 Stale Cache check
        let bad_cache1 = verify_no_stale_stdlib_cache("stdlib/.pilc-cache/std.core.pilc");
        let bad_cache2 = verify_no_stale_stdlib_cache("stdlib/foo.pilc");
        let good_file = verify_no_stale_stdlib_cache("stdlib/std/col.pilot");

        let chk_r4_ok = (!bad_cache1.passed && bad_cache1.code == "RING004") &&
                        (!bad_cache2.passed && bad_cache2.code == "RING004") &&
                        good_file.passed;

        // 2. Test canonical migration matchers
        let m_pickle = check_legacy_import_migration("pickle");
        let m_unittest = check_legacy_import_migration("std.unittest");
        let m_numpy = check_legacy_import_migration("numpy");
        let m_shutil = check_legacy_import_migration("std.shutil");

        let match_ok = (m_pickle.new_name == "std.bin") &&
                       (m_unittest.new_name == "std.test") &&
                       (m_numpy.new_name == "std.math") &&
                       (m_shutil.new_name == "std.fs");

        (if chk_r4_ok { "RING004_OK" } else { "FAIL" }) + "|" + (if match_ok { "MATCHERS_OK" } else { "FAIL" });
    "#;

    let res = eval_ring(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 2, "Expected 2 parts, got: {}", res);
    assert_eq!(parts[0], "RING004_OK", "Committed .pilc in stdlib/ must be rejected with RING004");
    assert_eq!(parts[1], "MATCHERS_OK", "Canonical Python-clone matchers must resolve accurately");

    guard.finish();
}
