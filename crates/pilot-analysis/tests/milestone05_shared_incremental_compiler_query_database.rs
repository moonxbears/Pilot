//! ============================================================================
//! Milestone 5 Verification Suite:
//! Shared Incremental Compiler Query Database (PilotDb)
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
        "selfhost/lexer.pilot",
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
    vm_outcome::outcome(&mut vm)
}

// -----------------------------------------------------------------------------
// Test 1: Early-Cutoff Counter Proof
// Build a 25-module chain M0 <- M1 <- ... <- M24, where each module's
// ImportScope reads the ItemSignature of the module it imports. Mutate a
// statement inside M0's fn body: M0's ItemSignature re-executes once, its
// output is unchanged, so no ImportScope re-executes and all 24 are re-verified
// as hits. (DefTypesAndLattice, the stage the spec names, is a seam until F4.)
// -----------------------------------------------------------------------------
#[test]
pub fn test_m5_1_early_cutoff_counter_proof() {
    let guard = anticheat::AntiCheatGuard::arm("test_m5_1_early_cutoff_counter_proof", &ANTICHEAT_SPEC);

    let script = r#"
        let db = pilotdb_new();
        setup_module_chain(db, 25);

        var m = 1;
        while m < 25 {
            let _ = query_import_scope(db, m + 0);
            m = m + 1;
        }

        db_reset_telemetry(db);

        // Mutate only a function body in root module M0 (preserves signature)
        mutate_fn_body(db, 0, "fresh_local");

        m = 1;
        while m < 25 {
            let _ = query_import_scope(db, m + 0);
            m = m + 1;
        }

        let exec_scope = db_query_exec_count(db, stage_import_scope());
        let hits_scope = db_query_hit_count(db, stage_import_scope());
        let exec_sig = db_query_exec_count(db, stage_item_signature());
        exec_scope + "|" + hits_scope + "|" + exec_sig;
    "#;

    let res = eval_db(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 3, "Expected exec|hits|sig_exec, got: {}", res);
    assert_eq!(parts[0], "0", "Early cutoff failed: no ImportScope may re-execute after a body-only edit, got: {}", res);
    assert_eq!(parts[1], "24", "Expected 24 re-verified ImportScope memos (M1..M24), got: {}", res);
    assert_eq!(parts[2], "1", "Only M0's ItemSignature may re-execute, got: {}", res);

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Signature-Change Propagation Proof
// Changing M0's public return type changes its ItemSignature, so the direct
// importer M1 re-executes its ImportScope; M2 imports the unchanged M1 and is
// cut off.
// -----------------------------------------------------------------------------
#[test]
pub fn test_m5_2_signature_change_propagation_proof() {
    let guard = anticheat::AntiCheatGuard::arm("test_m5_2_signature_change_propagation_proof", &ANTICHEAT_SPEC);

    let script = r#"
        let db = pilotdb_new();
        setup_module_chain(db, 3);
        let before = query_import_scope(db, 1);
        let _ = query_import_scope(db, 2);

        db_reset_telemetry(db);

        mutate_fn_signature(db, 0, "opt<str>");

        let after = query_import_scope(db, 1);
        let _ = query_import_scope(db, 2);

        let exec_scope = db_query_exec_count(db, stage_import_scope());
        let exec_sig = db_query_exec_count(db, stage_item_signature());
        exec_scope + "|" + exec_sig + "|" + before + "|" + after;
    "#;

    let res = eval_db(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 4, "Expected exec_scope|exec_sig|before|after, got: {}", res);
    assert_eq!(parts[0], "1", "Only M1's ImportScope may re-execute, got: {}", res);
    assert_eq!(parts[1], "1", "Only M0's ItemSignature may re-execute, got: {}", res);
    // The scope of M1 is M0's body-free signature, before and after the edit.
    assert_eq!(parts[2], "pub fn compute_0(x: i64) -> i64 {}\n", "M1 scope before the edit");
    assert_eq!(parts[3], "pub fn compute_0(x: i64) -> opt<str> {}\n", "M1 scope after the edit");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Identical Cold vs Incremental Oracle
// Compares ImportScope and LexTokens outputs between an incrementally updated
// database and a fresh cold database built from the same end-state files.
// -----------------------------------------------------------------------------
#[test]
pub fn test_m5_3_identical_cold_vs_incremental_oracle() {
    let guard = anticheat::AntiCheatGuard::arm("test_m5_3_identical_cold_vs_incremental_oracle", &ANTICHEAT_SPEC);

    let script = r#"
        let inc_db = pilotdb_new();
        setup_module_chain(inc_db, 5);
        let inc_before = query_import_scope(inc_db, 2) + query_import_scope(inc_db, 3);
        let _ = query_lex_tokens(inc_db, 2);

        mutate_fn_body(inc_db, 2, "v_added");
        mutate_fn_signature(inc_db, 1, "u8");
        let inc_scope = query_import_scope(inc_db, 2) + query_import_scope(inc_db, 3);
        let inc_lex = query_lex_tokens(inc_db, 2);

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
        let cold_scope = query_import_scope(cold_db, 2) + query_import_scope(cold_db, 3);
        let cold_lex = query_lex_tokens(cold_db, 2);

        (if inc_scope == cold_scope { "SCOPE_IDENTICAL" } else { "SCOPE_MISMATCH" })
            + "|" + (if inc_lex == cold_lex { "LEX_IDENTICAL" } else { "LEX_MISMATCH" })
            + "|" + (if inc_before != inc_scope { "EDIT_SEEN" } else { "EDIT_LOST" });
    "#;

    let res = eval_db(script);
    assert_eq!(res, "SCOPE_IDENTICAL|LEX_IDENTICAL|EDIT_SEEN", "Incremental query output did not match cold database oracle: {}", res);

    check_lex_tokens_and_unconnected_stage_seams();

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

/// LexTokens is the real scanner; the stages that need engines that do not
/// exist yet stop at their seams. Run from test 3.
fn check_lex_tokens_and_unconnected_stage_seams() {
    // The token texts, in order, spell the source without its whitespace.
    let src = "pub fn area(w: i64, h: i64) -> i64 { return w * h; }";
    let script = format!(
        r#"
        let db = pilotdb_new();
        pilotdb_set_file_source(db, 7, "{src}", durability_workspace());
        query_lex_tokens(db, 7);
        "#
    );
    let lexed = eval_db(&script);
    assert!(!lexed.starts_with("FAULT"), "LexTokens must run, got: {lexed}");
    let mut texts = Vec::new();
    for line in lexed.lines() {
        let (_, text) = line.split_once(' ').unwrap_or_else(|| panic!("token line `{line}` has no text"));
        texts.push(text);
    }
    let expected: String = src.chars().filter(|c| !c.is_whitespace()).collect();
    assert_eq!(texts.concat(), expected, "token texts must spell the source: {lexed}");

    for (query, seam_id) in [
        ("query_lossless_cst", "db.lossless_cst"),
        ("query_expanded_ast", "db.expanded_ast"),
        ("query_def_types_and_lattice", "db.def_types"),
        ("query_file_diagnostics", "db.file_diagnostics"),
    ] {
        let script = format!(
            r#"
            let db = pilotdb_new();
            pilotdb_set_file_source(db, 7, "{src}", durability_workspace());
            {query}(db, 7);
            "#
        );
        let out = eval_db(&script);
        let want = format!("FAULT: SEAM001: unconnected seam `{seam_id}`");
        assert!(out.starts_with(&want), "{query} must stop at {seam_id}, got: {out}");
    }
}
