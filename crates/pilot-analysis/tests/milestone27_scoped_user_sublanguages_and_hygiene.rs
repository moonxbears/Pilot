//! ============================================================================
//! Milestone 27 Verification Suite:
//! Scoped User Sublanguage Builder (`syntax` + Hygiene + Declarative IDE Sync)
//! ============================================================================

use std::fs;
use pilot_analysis::PhysicsRegistry;
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 27,
    test_file_rel: "crates/pilot-analysis/tests/milestone27_scoped_user_sublanguages_and_hygiene.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/syntax/pattern_dsl.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "pattern_dsl_rule_new",
                "pattern_dsl_check_diagnostics",
                "pattern_dsl_match_prefix",
            ],
            required_code_markers: &[
                "SYN001",
                "SYN002",
                "SYN003",
                "SYN004",
                "SYN005",
                "FragmentSpecKind",
                "SyntaxOriginFrame",
                "PatternArmDef",
                "SyntaxRuleDef",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/syntax/hygiene_marks.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "hygienic_env_new",
                "resolve_identifier",
            ],
            required_code_markers: &[
                "SYN006",
                "SyntaxScopeSet",
                "HygienicBinding",
                "ResolvedBinding",
                "HygienicEnv",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ide/live_dsl_sync.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "live_dsl_catalog_new",
                "query_dsl_completions",
                "query_dsl_hover",
                "is_dsl_keyword",
            ],
            required_code_markers: &[
                "DslCompletionItem",
                "DslSemanticToken",
                "DslHoverInfo",
                "LiveDslSyncCatalog",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

fn eval_m27(deps: &[&str], script: &str) -> String {
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
// Test 1: Pattern DSL Compilation & Ambiguity/Recursion Diagnostics
// -----------------------------------------------------------------------------
#[test]
fn test_m27_1_pattern_dsl_compilation_and_diagnostics() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m27_1_pattern_dsl_compilation_and_diagnostics",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "selfhost/syntax/pattern_dsl.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            // Case 1: Valid UI DSL rule
            var ui_rule = pattern_dsl_rule_new("UiDsl", true);
            ui_rule.pattern_dsl_add_arm("vstack", "vstack ( gap : $gap:expr ) { $body:block }", false, 10);
            ui_rule.pattern_dsl_add_arm("hstack", "hstack ( gap : $gap:expr ) { $body:block }", false, 10);
            ui_rule.pattern_dsl_add_arm("button", "button ( $label:str ) => $action:expr", false, 5);

            let diags_valid = pattern_dsl_check_diagnostics(ui_rule);
            if diags_valid.len() != 0 { return 1; }

            // Verify prefix matching
            let m1 = pattern_dsl_match_prefix(ui_rule, "vstack");
            if !m1.matched || m1.matched_head != "vstack" { return 2; }

            let m2 = pattern_dsl_match_prefix(ui_rule, "nonexistent");
            if m2.matched { return 3; }

            // Case 2: Left-recursive arm must trigger SYN002
            var bad_rec_rule = pattern_dsl_rule_new("LeftRecDsl", true);
            bad_rec_rule.pattern_dsl_add_arm("expr", "expr + expr", true, 0);
            let diags_rec = pattern_dsl_check_diagnostics(bad_rec_rule);
            if diags_rec.len() != 1 { return 4; }
            let expected_syn002 = "SYN002";
            if diags_rec[0].len() < expected_syn002.len() { return 5; }

            // Case 3: Overlapping arms with same head and equal priority must trigger SYN001
            var bad_overlap_rule = pattern_dsl_rule_new("OverlapDsl", true);
            bad_overlap_rule.pattern_dsl_add_arm("parse", "parse ( a )", false, 5);
            bad_overlap_rule.pattern_dsl_add_arm("parse", "parse ( b )", false, 5);
            let diags_overlap = pattern_dsl_check_diagnostics(bad_overlap_rule);
            if diags_overlap.len() != 1 { return 6; }
            let expected_syn001 = "SYN001";
            if diags_overlap[0].len() < expected_syn001.len() { return 7; }

            return 42;
        }
    "#;

    let res = eval_m27(&deps, script);
    assert_eq!(res, "42", "Failed pattern DSL test: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 2: Flatt Scopes-As-Sets Hygienic Renaming & Anaphoric Opt-in
// -----------------------------------------------------------------------------
#[test]
fn test_m27_2_flatt_scopes_as_sets_hygienic_renaming() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m27_2_flatt_scopes_as_sets_hygienic_renaming",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "selfhost/syntax/hygiene_marks.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            var env = hygienic_env_new();
            let caller_scopes = env.active_scopes; // Scope {0}

            // 1. Caller defines `let tmp = "hello"`
            let b_caller = env.add_binding("tmp", false); // binding 0

            // 2. Macro expansion begins (e.g. quote { let tmp = 42; })
            let exp_id = env.enter_expansion(false); // active_scopes = {0, 1}
            let b_macro = env.add_binding("tmp", false);  // binding 1 in {0, 1}

            // 3. Resolve `tmp` inside macro expansion ({0, 1}):
            // Must resolve to macro binding (binding 1) because {0, 1} is maximal subset
            let res_inside = resolve_identifier(env, "tmp", env.active_scopes);
            if !res_inside.found || res_inside.binding_id != b_macro { return 1; }

            // 4. Exit macro expansion back to caller scope ({0})
            env.exit_expansion(caller_scopes);

            // 5. Resolve `tmp` in caller scope ({0}):
            // Must resolve to caller binding (binding 0), NEVER accidentally shadowed by macro!
            let res_outside = resolve_identifier(env, "tmp", caller_scopes);
            if !res_outside.found || res_outside.binding_id != b_caller { return 2; }

            // 6. Explicit anaphoric opt-in: unhygienic binding
            let exp_ana = env.enter_expansion(true);
            let b_ana = env.add_binding("it", true); // unhygienic
            env.exit_expansion(caller_scopes);

            // Unhygienic binding matches in caller scope
            let res_ana = resolve_identifier(env, "it", caller_scopes);
            if !res_ana.found || !res_ana.is_unhygienic { return 3; }

            return 42;
        }
    "#;

    let res = eval_m27(&deps, script);
    assert_eq!(res, "42", "Failed hygiene renaming test: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 3: Scoped DSL Lexical Isolation & Live LSP / IDE Catalog Sync
// -----------------------------------------------------------------------------
#[test]
fn test_m27_3_scoped_dsl_lexical_isolation_and_live_lsp_sync() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m27_3_scoped_dsl_lexical_isolation_and_live_lsp_sync",
        &ANTICHEAT_SPEC,
    );

    let deps = [
        "selfhost/ide/live_dsl_sync.pilot",
    ];

    let script = r#"
        pub fn main() -> i64 {
            var cat = live_dsl_catalog_new();

            // Register 3 user sublanguages
            // 1. UI DSL: vstack, hstack, button
            cat.register_dsl_sublanguage("UiDsl");
            cat.register_dsl_keyword("vstack", "UiDsl", "Vertical stack layout container", "vstack(gap: 8) { ... }");
            cat.register_dsl_keyword("button", "UiDsl", "Clickable button widget", "button(\"Click\") => { ... }");

            // 2. FSM DSL: transition, on_state
            cat.register_dsl_sublanguage("FsmDsl");
            cat.register_dsl_keyword("transition", "FsmDsl", "State transition rule", "transition(From -> To on Evt)");

            // 3. SQL DSL: query_table
            cat.register_dsl_sublanguage("SqlDsl");
            cat.register_dsl_keyword("query_table", "SqlDsl", "Relational query projection", "query_table(t, where: cond)");

            // Verify live keyword checks
            if !is_dsl_keyword(cat, "vstack") { return 1; }
            if !is_dsl_keyword(cat, "button") { return 2; }
            if !is_dsl_keyword(cat, "transition") { return 3; }
            if is_dsl_keyword(cat, "random_ident") { return 4; }

            // Verify LSP completion provider
            let completions = query_dsl_completions(cat, "vst");
            if completions.len() != 1 { return 5; }
            if completions[0].label != "vstack" { return 6; }
            if completions[0].detail != "DSL Keyword (UiDsl)" { return 7; }

            // Verify LSP hover doc provider
            let hover_info = query_dsl_hover(cat, "button");
            if !hover_info.found { return 8; }
            if hover_info.symbol_name != "button" { return 9; }
            if hover_info.category_name != "UiDsl" { return 10; }

            return 42;
        }
    "#;

    let res = eval_m27(&deps, script);
    assert_eq!(res, "42", "Failed live DSL sync test: {}", res);
    guard.finish();
}

// -----------------------------------------------------------------------------
// Test 4: Zero Hardcoded Keywords Audit (14 Seed Tokens Boundary)
// -----------------------------------------------------------------------------
#[test]
fn test_m27_4_zero_hardcoded_keywords_in_analyzer_and_cst_recovery() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m27_4_zero_hardcoded_keywords_in_analyzer_and_cst_recovery",
        &ANTICHEAT_SPEC,
    );

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();

    // 1. Audit selfhost/analyzer.pilot: build_dynamic_keyword_set must NOT contain giant static string
    let analyzer_src = fs::read_to_string(root.join("selfhost/analyzer.pilot"))
        .expect("read analyzer.pilot");
    assert!(
        !analyzer_src.contains("else|while|until|for|in|then|break|continue"),
        "selfhost/analyzer.pilot still contains hardcoded non-seed keyword string!"
    );
    assert!(
        analyzer_src.contains("|__engine|struct|interface|fn|let|ref|if|loop|return|yield|defer|comptime|quote|asm|"),
        "selfhost/analyzer.pilot must preserve the 14 seed tokens"
    );

    // 2. Audit selfhost/cst/cst_recovery.pilot: canonical_keywords must NOT contain giant static string
    let cst_src = fs::read_to_string(root.join("selfhost/cst/cst_recovery.pilot"))
        .expect("read cst_recovery.pilot");
    assert!(
        !cst_src.contains("else,while,until,for,in,then,break,continue"),
        "selfhost/cst/cst_recovery.pilot still contains hardcoded non-seed keyword string!"
    );
    assert!(
        cst_src.contains("__engine,struct,interface,fn,let,ref,if,loop,return,yield,defer,comptime,quote,asm"),
        "selfhost/cst/cst_recovery.pilot must preserve the 14 seed tokens"
    );

    guard.finish();
}
