//! ==============================================================================
//! crates/pilot-analysis/tests/mega_all_milestones_simultaneous_gate.rs
//! MEGA GATE — All 13 Milestone Suites (44 Tests Across M50–M61) Simultaneously
//!
//! Passing `test_mega_all_milestones_50_through_61_simultaneously` is the ultimate
//! end-to-end verification gate:
//!   1. Arms the 10-pass `AntiCheatGuard` (verifying all 48 frozen Rust/Cargo/Gate
//!      SHA-256 digests, test-suite self-integrity, and `agent_test_readme.md`).
//!   2. Records a cryptographic SHA-256 snapshot of EVERY `.pilot` file across
//!      `stdlib/` and `selfhost/` BEFORE running any milestone test.
//!   3. Executes ALL 44 `#[test]` functions across ALL 13 milestone suites
//!      (`M50` through `M61`) sequentially in-process against the exact same
//!      workspace state — if ANY single test or anti-cheat pass in ANY milestone
//!      fails, this Mega Test fails immediately.
//!   4. Re-hashes EVERY `.pilot` file across `stdlib/` and `selfhost/` AFTER all
//!      44 milestone tests finish and asserts bit-for-bit equality with the
//!      pre-run snapshot, proving zero `.pilot` files were swapped mid-run.
//!   5. Completes `guard.finish()`.
//! ==============================================================================

use std::collections::BTreeMap;
use std::path::Path;

#[path = "common/anticheat.rs"]
mod anticheat;

#[path = "milestone50_lossless_bridge_and_reflection.rs"]
mod m50;

#[path = "../../pilot-parser/tests/milestone51_homoiconic_syntax_and_contraction.rs"]
mod m51;

#[path = "../../pilot-lexer/tests/milestone52_seed_lexer_and_preprocessor.rs"]
mod m52_lexer;

#[path = "milestone52_pilot_preprocessor_and_receptors.rs"]
mod m52_analysis;

#[path = "milestone53_cst_recovery_and_formatter.rs"]
mod m53;

#[path = "milestone54_perceus_ownership_and_escape.rs"]
mod m54;

#[path = "milestone55_selfhost_vm_compiler_and_debug_table.rs"]
mod m55;

#[path = "milestone56_layout_driven_codegen_and_gpu_backends.rs"]
mod m56;

#[path = "milestone57_bidirectional_transpilers_and_ingest.rs"]
mod m57;

#[path = "milestone58_runtime_crypto_wire_db_and_polyglot.rs"]
mod m58;

#[path = "milestone59_persistent_pilvm_lsp_and_dap.rs"]
mod m59;

#[path = "milestone60_selfhost_cli_pkg_perf_and_bench.rs"]
mod m60;

#[path = "milestone61_stage3_fixed_point_and_universal_rename_gate.rs"]
mod m61;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 99,
    test_file_rel: "crates/pilot-analysis/tests/mega_all_milestones_simultaneous_gate.rs",
    expected_test_count: 1,
    files: &[],
    required_group_pub_fns: &[],
};

fn snapshot_all_pilot_sha256(root: &Path) -> BTreeMap<String, String> {
    let mut digests = BTreeMap::new();
    for top in ["stdlib", "selfhost"] {
        let mut stack = vec![root.join(top)];
        while let Some(dir) = stack.pop() {
            if !dir.exists() {
                continue;
            }
            for entry in std::fs::read_dir(&dir).expect("read_dir in pilot snapshot") {
                let p = entry.expect("dir entry").path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|e| e.to_str()) == Some("pilot") {
                    let rel = p
                        .strip_prefix(root)
                        .expect("workspace-relative path")
                        .to_string_lossy()
                        .replace('\\', "/");
                    let bytes = std::fs::read(&p).expect("read .pilot file");
                    digests.insert(rel, anticheat::sha256_hex(&bytes));
                }
            }
        }
    }
    assert!(
        !digests.is_empty(),
        "Expected non-empty .pilot file snapshot across stdlib/ and selfhost/"
    );
    digests
}

#[test]
pub fn test_mega_all_milestones_50_through_61_simultaneously() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_mega_all_milestones_50_through_61_simultaneously",
        &ANTICHEAT_SPEC,
    );
    let root = anticheat::workspace_root();
    let pre_pilot_snapshot = snapshot_all_pilot_sha256(&root);

    // -------------------------------------------------------------------------
    // Milestone 50 (4 tests)
    // -------------------------------------------------------------------------
    m50::test_50_1_lossless_flatten_ast_randomized_quotes_attrs_bases_and_layouts();
    m50::test_50_2_open_keyword_reflection_and_randomized_user_categories();
    m50::test_50_3_bootstrap_manifest_randomized_naming_verifier_and_full_weight_sorted_weaver();
    m50::test_50_4_and_50_5_target_width_projections_pilc_snapshot_and_gate1_gate2_contraction();

    // -------------------------------------------------------------------------
    // Milestone 51 (4 tests)
    // -------------------------------------------------------------------------
    m51::test_51_1_randomized_type_construct_precedence_and_executable_homoiconic_macros();
    m51::test_51_2_surface_syntax_distributive_headers_call_quotes_and_diagnostics();
    m51::test_51_3_domain_syntax_packages_and_strict_nameof_hygiene();
    m51::test_51_4_rust_parser_and_ast_contraction_and_name_blind_purity();

    // -------------------------------------------------------------------------
    // Milestone 52 — Seed Lexer (4 tests) + Pilot Preprocessor & Receptors (3 tests)
    // -------------------------------------------------------------------------
    m52_lexer::test_52_2_strict_14_seed_keywords_and_randomized_ident_boundary();
    m52_lexer::test_52_2_two_way_dot_call_quotes_and_nested_bracket_matching();
    m52_lexer::test_52_3_name_blind_unit_suffixes_fstrings_and_keywords_randomized();
    m52_lexer::test_52_4_contraction_caps_and_legacy_deletion_invariants();

    m52_analysis::test_52_1_nested_if_stack_inactive_isolation_and_boolean_algebra();
    m52_analysis::test_52_1_all_eight_preproc_diagnostics_and_pragma_suppression();
    m52_analysis::test_52_3_unit_receptors_fstring_execution_and_hierarchical_keywords();

    // -------------------------------------------------------------------------
    // Milestone 53 (3 tests)
    // -------------------------------------------------------------------------
    m53::test_53_1_hierarchical_lossless_cst_and_green_node_incremental_reuse();
    m53::test_53_2_live_dynamic_keyword_typo_recovery_beyond_u128_and_recovery_markers();
    m53::test_53_3_and_53_4_wadler_lindig_width_formatter_semantic_parity_and_crate_purge();

    // -------------------------------------------------------------------------
    // Milestone 54 (2 tests)
    // -------------------------------------------------------------------------
    m54::test_54_1_real_ast_perceus_arc_dup_last_use_drop_reuse_and_lifo_guards();
    m54::test_54_2_and_54_3_real_ast_escape_realtime_cue_verifiers_and_dsp_decoy_trap();

    // -------------------------------------------------------------------------
    // Milestone 55 (3 tests)
    // -------------------------------------------------------------------------
    m55::test_55_1_selfhost_compiler_emits_executable_pilc_for_randomized_recursive_and_loop_programs();
    m55::test_55_2_sso_and_heap_string_constants_and_zero_arg_entry_without_main_hardcoding();
    m55::test_55_3_and_55_4_debug_tables_opcode_purification_and_line_caps();

    // -------------------------------------------------------------------------
    // Milestone 56 (3 tests)
    // -------------------------------------------------------------------------
    m56::test_56_1_layout_driven_c11_backend_compiled_and_executed_by_clang();
    m56::test_56_3_llvm_ir_wasm_binary_and_x86_64_asm_executed_by_clang_and_node();
    m56::test_56_2_and_56_4_all_eight_gpu_backends_ast_lowering_cpu_execution_and_legacy_purge();

    // -------------------------------------------------------------------------
    // Milestone 57 (2 tests)
    // -------------------------------------------------------------------------
    m57::test_57_1_and_57_2_seven_target_transpilers_compiled_and_executed_on_host_toolchains();
    m57::test_57_3_and_57_4_six_language_foreign_ingesters_vm_execution_spdx_and_crate_purge();

    // -------------------------------------------------------------------------
    // Milestone 58 (7 tests)
    // -------------------------------------------------------------------------
    m58::test_58_1_ffi_bindgen_clang_c11_header_and_rt_getrandom_dlopen();
    m58::test_58_2a_sha256_sha512_blake3_and_hmac_randomized_boundary_oracles();
    m58::test_58_2b_ed25519_rfc8032_bidirectional_node_oracle_and_malleability_rejection();
    m58::test_58_3_glitch_free_diamond_reactive_cow_effects_and_real_tcp_http11();
    m58::test_58_4a_shm_bounds_json_marshal_and_polyglot_python_node_bridges();
    m58::test_58_4b_relational_sql_sqlite3_oracle_and_postgres_mysql_binary_wire_frames();
    m58::test_58_5_legacy_polyglot_crypto_deleted_and_strict_rust_loc_caps();

    // -------------------------------------------------------------------------
    // Milestone 59 (3 tests)
    // -------------------------------------------------------------------------
    m59::test_59_1_vscode_gen_strict_json_no_fs_bypass_and_full_lsp_handlers_matrix();
    m59::test_59_2_incremental_span_edits_adversarial_early_cutoff_and_persistent_lsp_stream();
    m59::test_59_3_and_59_4_live_vmhook_dap_stepping_watch_eval_inspector_and_5_crate_purge();

    // -------------------------------------------------------------------------
    // Milestone 60 (3 tests)
    // -------------------------------------------------------------------------
    m60::test_60_1_pkg_manifest_parser_real_sha256_lockfile_multifile_pilpkg_and_venv();
    m60::test_60_2_live_vmhook_flamegraph_svg_ast_wcet_and_real_constant_folding_optimizer();
    m60::test_60_3_and_60_4_selfhost_cli_test_runner_diff_test_stateful_repl_and_crate_purge();

    // -------------------------------------------------------------------------
    // Milestone 61 (3 tests)
    // -------------------------------------------------------------------------
    m61::test_61_1_standalone_selfhost_lexer_16b_ast_pratt_precedence_and_quote_splicing();
    m61::test_61_2_stage3_bootstrap_fixed_point_clang_native_and_pilc_sha256_parity();
    m61::test_61_3_and_61_4_all_four_gates_and_dual_pass_universal_rename_litmus_across_7_backends();

    // -------------------------------------------------------------------------
    // Simultaneous Disk Snapshot Verification (Zero Mid-Run `.pilot` Swapping)
    // -------------------------------------------------------------------------
    let post_pilot_snapshot = snapshot_all_pilot_sha256(&root);
    assert_eq!(
        pre_pilot_snapshot, post_pilot_snapshot,
        "ANTI-CHEAT [MEGA_SNAPSHOT_DRIFT]: `.pilot` files on disk changed mid-execution between Milestone 50 and Milestone 61! All 44 milestone tests must pass simultaneously against one static `.pilot` codebase."
    );

    guard.finish();
}
