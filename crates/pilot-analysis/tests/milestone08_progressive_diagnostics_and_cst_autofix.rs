//! ============================================================================
//! Milestone 8 Verification Suite:
//! Progressive Diagnostics & Lossless CST Auto-Fix (`pilot check --fix`)
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
    milestone_id: 8,
    test_file_rel: "crates/pilot-analysis/tests/milestone08_progressive_diagnostics_and_cst_autofix.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/diagnostics/model.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "diag_severity_name",
                "fix_applicability_name",
                "diagnostic_with_primary_label",
            ],
            required_code_markers: &[
                "DiagSeverity",
                "FixApplicability",
                "SourceSpan",
                "LabeledSpan",
                "CstTextEdit",
                "FixSuggestion",
                "Diagnostic",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/diagnostics/renderer.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "utf8_char_info",
                "measure_display_column",
                "render_diagnostic_human",
                "render_explain",
            ],
            required_code_markers: &[
                "CharWidthInfo",
                "find_line_col_at",
                "render_diagnostic_human",
                "render_explain",
                "render_sarif_report",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/cst/autofix.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "extract_all_comments",
                "verify_comment_conservation",
                "apply_file_fixes_lossless",
                "scan_and_suggest_fixes",
            ],
            required_code_markers: &[
                "FixApplicationResult",
                "verify_comment_conservation",
                "apply_file_fixes_lossless",
                "generate_unified_diff",
                "scan_and_suggest_fixes",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_diag(script: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let deps = [
        "stdlib/core/file.pilot",
        "stdlib/std/text.pilot",
        "selfhost/diagnostics/model.pilot",
        "selfhost/diagnostics/renderer.pilot",
        "selfhost/cst/autofix.pilot",
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
// Test 1: Transactional Lossless CST Auto-Fix (4 Lexical Defects + 12 Comments + CRLF)
// -----------------------------------------------------------------------------
#[test]
pub fn test_m8_1_transactional_cst_autofix_six_defects_and_comment_conservation() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m8_1_transactional_cst_autofix_six_defects_and_comment_conservation",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        // 12 comments interleaved with 6 defects and CRLF line endings
        let src = "/* comment 1 */\r\nlet counter = 10; // comment 2\r\n/* comment 3 */ counter = 20;\r\n// comment 4\r\nlet factor = .5; /* comment 5 */\r\n// comment 6\r\nlet missing_semi = 42\r\n/* comment 7 */\r\nmut mutable_var = 100;\r\n// comment 8\r\nmatch state_val {\r\n    /* comment 9 */\r\n    case 1: break;\r\n}\r\n// comment 10\r\nupdate_data(ref target_buffer); /* comment 11 */\r\n// comment 12\r\n";

        let diags = scan_and_suggest_fixes(src, 1);
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
        let comments_ok = verify_comment_conservation(src, fix_res.new_src);
        let crlf_ok = fix_res.new_src.find("\r\n") >= 0;

        // Adversarial check: attempt an edit that overwrites comment 1
        var adv_fix = fix_suggestion_new("Adversarial delete comment", fix_machine_applicable());
        let bad_span = source_span_new(1, 0, 15);
        let bad_edit = cst_text_edit_new(bad_span, "");
        adv_fix.edits.push(bad_edit);

        var adv_list: List<FixSuggestion> = [];
        adv_list.push(adv_fix);
        let adv_res = apply_file_fixes_lossless(src, adv_list, 3);

        ("" + diag_count + "|" + fix_res.fixes_applied + "|" + (if fix_res.success { "OK" } else { "FAIL" }) + "|" + (if comments_ok { "CONSERVED" } else { "LOST" }) + "|" + (if crlf_ok { "CRLF" } else { "LF" }) + "|" + adv_res.error_code + "|" + fix_res.new_src);
    "#;

    let res = eval_diag(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts.len(), 7, "Expected 7 parts, got: {}", res);
    // The source holds four lexical defects: `.5`, the unterminated
    // `let missing_semi = 42`, `mut` and `match`. `let counter = 10; // comment 2`
    // ends in a comment, not a missing `;`.
    assert_eq!(parts[0], "4", "Expected 4 lexical diagnostics");
    assert_eq!(parts[1], "4", "Expected 4 fixes applied");
    assert_eq!(parts[2], "OK", "Autofix should report OK");
    assert_eq!(parts[3], "CONSERVED", "All 12 comments must be preserved verbatim");
    assert_eq!(parts[4], "CRLF", "CRLF line endings must be preserved verbatim");
    assert_eq!(parts[5], "FIX001", "Adversarial comment loss must abort with FIX001");
    // Hand-applied edits: `.5` -> `0.5`, `42` -> `42;`, `mut` -> `var`, `match` -> `switch`.
    assert_eq!(
        parts[6],
        "/* comment 1 */\r\nlet counter = 10; // comment 2\r\n/* comment 3 */ counter = 20;\r\n// comment 4\r\nlet factor = 0.5; /* comment 5 */\r\n// comment 6\r\nlet missing_semi = 42;\r\n/* comment 7 */\r\nvar mutable_var = 100;\r\n// comment 8\r\nswitch state_val {\r\n    /* comment 9 */\r\n    case 1: break;\r\n}\r\n// comment 10\r\nupdate_data(ref target_buffer); /* comment 11 */\r\n// comment 12\r\n",
        "Fixed source must differ from the input only at the four edits"
    );

    // DECL004 / BORROW001 need the type checker; that path stops at its seam.
    let semantic = eval_diag(r#"suggest_semantic_fixes("let a = 1;\na = 2;\n", 1);"#);
    assert!(
        semantic.starts_with("FAULT: SEAM001: unconnected seam `autofix.semantic_defects`"),
        "semantic fixes must stop at their seam, got: {}",
        semantic
    );

    // The atomic replace writes the .tmp file, then stops before the rename.
    let dir = std::env::temp_dir().join(format!("pilot_m8_atomic_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("app.pilot");
    let target_s = target.to_string_lossy().replace('\\', "/");
    let atomic = eval_diag(&format!(r#"write_file_atomic("{}", "let x = 1;\n");"#, target_s));
    let tmp_written = std::fs::read_to_string(dir.join("app.pilot.tmp")).ok();
    let target_exists = target.exists();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        atomic.starts_with("FAULT: SEAM001: unconnected seam `autofix.atomic_rename`"),
        "atomic write must stop at its rename seam, got: {}",
        atomic
    );
    assert_eq!(tmp_written.as_deref(), Some("let x = 1;\n"), "the .tmp file holds the new source");
    assert!(!target_exists, "the target is not written before the rename exists");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Progressive `--explain <CODE>` Tutorial and Tier-1 Guidance
// -----------------------------------------------------------------------------
#[test]
pub fn test_m8_2_progressive_explain_tutorial_and_tier1_guidance() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m8_2_progressive_explain_tutorial_and_tier1_guidance",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        let exp_escape = render_explain("ESCAPE001", false);
        let exp_decl = render_explain("DECL004", false);
        let exp_syn = render_explain("SYN001", false);
        let exp_fix = render_explain("FIX001", false);

        let has_why = exp_escape.find("Why This Happens:") >= 0;
        let has_tier1 = exp_escape.find("How to Fix (Tier 1 Guidance):") >= 0;
        let has_bad = exp_escape.find("Incorrect Code Example:") >= 0;
        let has_good = exp_escape.find("Corrected Code Example:") >= 0;
        let has_ref = exp_escape.find("Reference:") >= 0;

        (if has_why && has_tier1 && has_bad && has_good && has_ref { "ALL_SECTIONS_PRESENT" } else { "MISSING_SECTIONS" }) + "|" + (if exp_decl.find("DECL004") >= 0 { "true" } else { "false" }) + "|" + (if exp_syn.find("SYN001") >= 0 { "true" } else { "false" }) + "|" + (if exp_fix.find("FIX001") >= 0 { "true" } else { "false" });
    "#;

    let res = eval_diag(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts[0], "ALL_SECTIONS_PRESENT", "Explain output must include all required tutorial sections");
    assert_eq!(parts[1], "true", "DECL004 explain must reference DECL004");
    assert_eq!(parts[2], "true", "SYN001 explain must reference SYN001");
    assert_eq!(parts[3], "true", "FIX001 explain must reference FIX001");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Unicode Multi-Byte & East-Asian Display-Width Caret Alignment
// -----------------------------------------------------------------------------
#[test]
pub fn test_m8_3_unicode_east_asian_display_width_and_caret_alignment() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m8_3_unicode_east_asian_display_width_and_caret_alignment",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        // 1. Greek math 'Δ' (2 bytes, 1 visual column)
        let greek_src = "let Δ = 42;\n";
        let info_greek = utf8_char_info("Δ", 0);

        // 2. East-Asian CJK character '中' (3 bytes, 2 visual columns)
        let cjk_src = "let 中 = 100;\n";
        let info_cjk = utf8_char_info("中", 0);

        // 3. Emoji '🚀' (4 bytes, 2 visual columns)
        let emoji_src = "let 🚀 = 999;\n";
        let info_emoji = utf8_char_info("🚀", 0);

        // Underline span carets test
        let sp = source_span_new(1, 4, 6); // span over 'Δ' (2 bytes)
        let diag = diagnostic_with_primary_label("SYN001", diag_severity_error(), "Greek variable name test", sp, "check variable name");
        let rendered = render_diagnostic_human(diag, greek_src, "test.pilot", false);

        ("" + info_greek.byte_len + ":" + info_greek.display_width + "|" + info_cjk.byte_len + ":" + info_cjk.display_width + "|" + info_emoji.byte_len + ":" + info_emoji.display_width + "|" + (if rendered.find("^") >= 0 { "HAS_CARET" } else { "NO_CARET" }));
    "#;

    let res = eval_diag(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts[0], "2:1", "Greek 'Δ' is 2 bytes and 1 display width column");
    assert_eq!(parts[1], "3:2", "CJK '中' is 3 bytes and 2 display width columns");
    assert_eq!(parts[2], "4:2", "Emoji '🚀' is 4 bytes and 2 display width columns");
    assert_eq!(parts[3], "HAS_CARET", "Rendered diagnostic must position caret under token");

    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: SARIF 2.1.0 Emitter and Unified Diff Preview
// -----------------------------------------------------------------------------
#[test]
pub fn test_m8_4_sarif_json_emitter_and_unified_diff_preview() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m8_4_sarif_json_emitter_and_unified_diff_preview",
        &ANTICHEAT_SPEC,
    );

    let script = r#"
        let sp = source_span_new(1, 0, 5);
        let diag1 = diagnostic_with_primary_label("TYPE001", diag_severity_error(), "Type Mismatch Error", sp, "mismatched types");
        let diag2 = diagnostic_with_primary_label("SYN001", diag_severity_warning(), "Missing Semicolon Warning", sp, "insert semicolon");

        var diag_list: List<Diagnostic> = [];
        diag_list.push(diag1);
        diag_list.push(diag2);

        let src = "let x = 1\nlet y = 2\n";
        let sarif = render_sarif_report(diag_list, "app.pilot", src);
        let single_json = render_diagnostic_json(diag1, "app.pilot", src);

        let has_schema = sarif.find("sarif-schema-2.1.0.json") >= 0;
        let has_version = sarif.find("\"version\": \"2.1.0\"") >= 0;
        let has_driver = sarif.find("\"name\": \"pilot\"") >= 0;
        let has_results = sarif.find("\"results\": [") >= 0;

        let old_src = "let x = 1;\nlet y = .5;\n";
        let new_src = "var x = 1;\nlet y = 0.5;\n";
        let diff = generate_unified_diff("app.pilot", old_src, new_src);

        let diff_has_old = diff.find("--- a/app.pilot") >= 0;
        let diff_has_new = diff.find("+++ b/app.pilot") >= 0;
        let diff_has_minus = diff.find("-let x = 1;") >= 0;
        let diff_has_plus = diff.find("+var x = 1;") >= 0;

        (if has_schema && has_version && has_driver && has_results { "SARIF_OK" } else { "SARIF_INVALID" }) + "|" + (if single_json.find("\"code\":\"TYPE001\"") >= 0 { "true" } else { "false" }) + "|" + (if diff_has_old && diff_has_new && diff_has_minus && diff_has_plus { "DIFF_OK" } else { "DIFF_INVALID" });
    "#;

    let res = eval_diag(script);
    let parts: Vec<&str> = res.split('|').collect();
    assert_eq!(parts[0], "SARIF_OK", "SARIF report must comply with 2.1.0 schema");
    assert_eq!(parts[1], "true", "Single JSON diagnostic must contain code TYPE001");
    assert_eq!(parts[2], "DIFF_OK", "Unified diff preview must contain unified diff headers and changes");

    guard.finish();
}
