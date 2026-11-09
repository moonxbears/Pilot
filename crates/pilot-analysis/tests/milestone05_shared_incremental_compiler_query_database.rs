//! ============================================================================
//! Milestone 5 Verification Suite:
//! Shared Incremental Compiler Query Database (PilotDb)
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 5,
    test_file_rel: "crates/pilot-analysis/tests/milestone05_shared_incremental_compiler_query_database.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/db/revision.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "fingerprint_from_hex",
                "fingerprint_compute",
                "query_key_new",
                "query_key_encode",
            ],
            required_code_markers: &[
                "Fingerprint128",
                "QueryKey",
                "MemoHeader",
                "durability_volatile",
                "stage_source_text",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/db/engine.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "pilotdb_new",
                "pilotdb_current_rev",
                "pilotdb_set_file_source",
                "pilotdb_query_with_fn",
            ],
            required_code_markers: &[
                "pilotdb_new",
                "pilotdb_query_with_fn",
                "DB001",
                "fingerprint_equals",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/db/queries.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "query_source_text",
                "query_item_signature",
                "query_def_types_and_lattice",
                "query_file_diagnostics",
            ],
            required_code_markers: &[
                "query_source_text",
                "query_item_signature",
                "query_def_types_and_lattice",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/db/cache_io.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "db_canonicalize_uri",
                "db_serialize_cache",
                "db_deserialize_cache",
            ],
            required_code_markers: &[
                "PDB1",
                "db_canonicalize_uri",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/lsp/ide_db.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "extract_item_tree_signature",
                "apply_doc_span_edit",
                "should_early_cutoff",
                "lsp_db_new",
                "lsp_db_update_document",
            ],
            required_code_markers: &[
                "extract_item_tree_signature",
                "lsp_db_new",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_db(script: &str) -> String {
    let root = anticheat::workspace_root();
    let deps = [
        "stdlib/core/file.pilot",
        "selfhost/db/revision.pilot",
        "selfhost/db/engine.pilot",
        "selfhost/db/queries.pilot",
        "selfhost/db/cache_io.pilot",
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
// Test 1: Early-Cutoff Counter Proof
// Build 25-module chain M0 <- M1 <- ... <- M24.
// Mutate statement inside M0 fn body: assert DefTypesAndLattice runs 1 time
// (M0) and 0 times for M1..M24!
// -----------------------------------------------------------------------------
#[test]
pub fn test_m5_1_early_cutoff_counter_proof() {
    let guard = anticheat::AntiCheatGuard::arm("test_m5_1_early_cutoff_counter_proof", &ANTICHEAT_SPEC);

    let script = r#"
        let db = pilotdb_new();
        // 1. Setup 25-module dependency chain
        setup_module_chain(db, 25);

        // 2. Initial warm-up: evaluate target module M24 (walks all 25 upstream modules)
        let _ = query_def_types_and_lattice(db, 24);

        // 3. Reset telemetry counters
        db_reset_telemetry(db);

        // 4. Mutate only a function body in root module M0 (preserves signature)
        mutate_fn_body(db, 0, "fresh_local");

        // 5. Query M24 again
        let _ = query_def_types_and_lattice(db, 24);

        let exec_stage7 = db_query_exec_count(db, stage_def_types_and_lattice());
        let hits_stage7 = db_query_hit_count(db, stage_def_types_and_lattice());
        exec_stage7 + "|" + hits_stage7;
    "#;

    let res = eval_db(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 2, "Expected exec|hits, got: {}", res);
    let exec_count: i64 = parts[0].parse().unwrap_or(-1);
    let hit_count: i64 = parts[1].parse().unwrap_or(-1);

    assert_eq!(exec_count, 1, "Early-cutoff failed! DefTypesAndLattice should only recompute M0 (1 time), got: {}", exec_count);
    assert!(hit_count >= 24, "Expected >= 24 fast-path hits across M1..M24, got: {}", hit_count);

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Signature-Change Propagation Proof
// Modifying a public function return type invalidates direct callers and
// re-executes type checking across dependent modules.
// -----------------------------------------------------------------------------
#[test]
pub fn test_m5_2_signature_change_propagation_proof() {
    let guard = anticheat::AntiCheatGuard::arm("test_m5_2_signature_change_propagation_proof", &ANTICHEAT_SPEC);

    let script = r#"
        let db = pilotdb_new();
        setup_module_chain(db, 3);

        // Initial warm-up
        let _ = query_def_types_and_lattice(db, 2);

        db_reset_telemetry(db);

        // Mutate M0 return type signature (breaks early-cutoff barrier)
        mutate_fn_signature(db, 0, "opt<str>");

        // Query M2 again
        let _ = query_def_types_and_lattice(db, 2);

        let exec_stage7 = db_query_exec_count(db, stage_def_types_and_lattice());
        let exec_sig = db_query_exec_count(db, stage_item_signature());
        exec_stage7 + "|" + exec_sig;
    "#;

    let res = eval_db(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 2, "Expected exec7|exec_sig, got: {}", res);
    let exec_stage7: i64 = parts[0].parse().unwrap_or(-1);
    assert!(exec_stage7 >= 2, "Signature change must propagate! Expected >= 2 recomputations, got: {}", exec_stage7);

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Identical Cold vs Incremental Oracle
// Compares query outputs between an incrementally updated database and a
// fresh cold database built from scratch.
// -----------------------------------------------------------------------------
#[test]
pub fn test_m5_3_identical_cold_vs_incremental_oracle() {
    let guard = anticheat::AntiCheatGuard::arm("test_m5_3_identical_cold_vs_incremental_oracle", &ANTICHEAT_SPEC);

    let script = r#"
        // 1. Build incremental database with edits
        let inc_db = pilotdb_new();
        setup_module_chain(inc_db, 5);
        let _ = query_file_diagnostics(inc_db, 4);

        mutate_fn_body(inc_db, 2, "v_added");
        let inc_out = query_file_diagnostics(inc_db, 4);

        // 2. Build cold database with exact same end-state files
        let cold_db = pilotdb_new();
        var i = 0;
        while i < 5 {
            let src = pilotdb_get_file_source(inc_db, i + 0);
            pilotdb_set_file_source(cold_db, i + 0, src, durability_workspace());
            if i > 0 {
                pilotdb_register_module_import(cold_db, i + 0, (i - 1) + 0);
            }
            i = i + 1;
        }
        let cold_out = query_file_diagnostics(cold_db, 4);

        if inc_out == cold_out { "BIT_IDENTICAL" } else { "MISMATCH" };
    "#;

    let res = eval_db(script);
    assert_eq!(res, "BIT_IDENTICAL", "Incremental query output did not match cold database oracle!");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Multiplatform URI Normalization & Persistent Cache I/O
// Verifies Windows/POSIX/file:// path normalization and .pilc-cache serialization.
// -----------------------------------------------------------------------------
#[test]
pub fn test_m5_4_multiplatform_uri_normalization_and_persistent_cache_io() {
    let guard = anticheat::AntiCheatGuard::arm("test_m5_4_multiplatform_uri_normalization_and_persistent_cache_io", &ANTICHEAT_SPEC);

    let script = r#"
        // 1. Multiplatform URI normalization tests
        let u1 = db_canonicalize_uri("file:///C:/Users/dev/project/src/main.pilot");
        let u2 = db_canonicalize_uri("C:\\Users\\dev\\project\\src\\main.pilot");
        let u3 = db_canonicalize_uri("file:///home/user/project/src/main.pilot");
        let u4 = db_canonicalize_uri("file://server/share/project/src/main.pilot");

        // 2. Cache serialization & deserialization
        let db1 = pilotdb_new();
        pilotdb_set_file_source(db1, 101, "pub fn hello() -> str => \"hi\";", durability_workspace());
        let _ = query_item_signature(db1, 101);

        let cached_data = db_serialize_cache(db1, "COMPILER_BUILD_V1");

        let db2 = pilotdb_new();
        let load_ok = db_deserialize_cache(db2, cached_data, "COMPILER_BUILD_V1");
        let mismatch_ok = db_deserialize_cache(db2, cached_data, "COMPILER_BUILD_V2_MISMATCH");

        let loaded_src = pilotdb_get_file_source(db2, 101);

        u1 + "|" + u2 + "|" + u3 + "|" + u4 + "|" + (if load_ok { "LOAD_OK" } else { "ERR" }) + "|" + (if !mismatch_ok { "REJECTED_MISMATCH" } else { "ERR" }) + "|" + (if loaded_src.len() > 0 { "SRC_OK" } else { "ERR" });
    "#;

    let res = eval_db(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 7, "Expected 7 result tokens, got: {}", res);

    assert_eq!(parts[0], "c:/Users/dev/project/src/main.pilot", "URI 1 normalization mismatch");
    assert_eq!(parts[1], "c:/Users/dev/project/src/main.pilot", "URI 2 Windows backslash mismatch");
    assert_eq!(parts[2], "/home/user/project/src/main.pilot", "URI 3 POSIX file:/// mismatch");
    assert_eq!(parts[3], "server/share/project/src/main.pilot", "URI 4 UNC file:// mismatch");
    assert_eq!(parts[4], "LOAD_OK", "Cache deserialization failed");
    assert_eq!(parts[5], "REJECTED_MISMATCH", "Version mismatch should reject cache");
    assert_eq!(parts[6], "SRC_OK", "Loaded source text empty");

    guard.finish();
}
