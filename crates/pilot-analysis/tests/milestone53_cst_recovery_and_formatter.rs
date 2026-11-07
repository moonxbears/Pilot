use std::path::Path;
use pilot_analysis::{eval_file_with_tables, BootstrapSnapshot};
use pilot_rt::PilotVal;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

fn run_pilot_script(modules: &[&str], main_src: &str) -> PilotVal {
    let mut combined = String::new();
    for m in modules {
        combined.push_str(m);
        combined.push('\n');
    }
    combined.push_str(main_src);
    let arena = pilot_parser::parse(&combined);
    assert!(
        arena.diagnostics.is_empty(),
        "Pilot script parse diagnostics: {:?}",
        arena.diagnostics
    );
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    vm.run()
}

#[test]
fn milestone53_pilot_cst_and_incremental_splicer_execute_on_vm() {
    let cst_src = include_str!("../../../selfhost/cst/cst.pilot");
    let inc_src = include_str!("../../../selfhost/cst/incremental.pilot");

    let snap = BootstrapSnapshot::global();
    for (name, src) in [
        ("selfhost/cst/cst.pilot", cst_src),
        ("selfhost/cst/incremental.pilot", inc_src),
    ] {
        let mut tables = snap.tables.clone();
        let mut vm = snap.vm.clone();
        let res = eval_file_with_tables(src, &mut tables, &mut vm);
        assert!(res.is_ok(), "{} failed analysis: {:?}", name, res.err());
    }

    let val = run_pilot_script(
        &[cst_src, inc_src],
        r#"
fn main() -> i64 {
    let src = "pub fn alpha() -> i64 { return 1; }\npub fn beta() -> i64 { return 2; }\n";
    let top_count = count_cst_top_level_children(src);
    if top_count != 2 { return 100 + top_count; }

    let green = green_node_create(cst_kind_program(), top_count, false, false, src);
    if green_node_to_text(green) != src { return 101; }

    let edited = apply_text_edit(src, 31, 32, "42");
    if !verify_delimiter_convergence(edited) { return 102; }

    let island_status = find_enclosing_island(edited, 24, 34);
    if island_status != "reused" { return 103; }

    return 1;
}
"#,
    );
    assert_eq!(val.as_int(), Some(1));
}

#[test]
fn milestone53_live_keyword_recovery_and_nameof_anchors_execute_on_vm() {
    let rec_src = include_str!("../../../selfhost/cst/cst_recovery.pilot");
    let par_src = include_str!("../../../selfhost/cst/cst_parser.pilot");

    let snap = BootstrapSnapshot::global();
    for (name, src) in [
        ("selfhost/cst/cst_recovery.pilot", rec_src),
        ("selfhost/cst/cst_parser.pilot", par_src),
    ] {
        let mut tables = snap.tables.clone();
        let mut vm = snap.vm.clone();
        let res = eval_file_with_tables(src, &mut tables, &mut vm);
        assert!(res.is_ok(), "{} failed analysis: {:?}", name, res.err());
    }

    let val = run_pilot_script(
        &[rec_src, par_src],
        r#"
fn main() -> i64 {
    // 1. Damerau-Levenshtein typo fix-its across seed & dynamic keywords
    if suggest_keyword_typo("retrun") != nameof(return) { return 10; }
    if suggest_keyword_typo("strcut") != nameof(struct) { return 11; }
    if suggest_keyword_typo("swtich") != nameof(switch) { return 12; }
    if suggest_keyword_typo(nameof(return)) != "" { return 13; }

    // 2. Arbitrary-width statement & declaration sync anchors using nameof()
    if !is_stmt_sync_anchor(nameof(return)) { return 20; }
    if !is_stmt_sync_anchor(nameof(guard)) { return 21; }
    if !is_decl_sync_anchor(nameof(struct)) { return 22; }
    if !is_decl_sync_anchor(nameof(interface)) { return 23; }

    // 3. Missing token recovery markers
    if !cst_detect_missing_ident("pub fn (x: i64) -> i64 { return x; }") { return 30; }
    if cst_detect_missing_ident("pub fn compute(x: i64) -> i64 { return x; }") { return 31; }
    if !cst_detect_missing_rparen("fn main() { if (x > 0 { return 1; } }") { return 32; }
    if !cst_detect_missing_rbrace("fn main() { return 1;") { return 33; }

    return 1;
}
"#,
    );
    assert_eq!(val.as_int(), Some(1));
}

#[test]
fn milestone53_wadler_lindig_formatter_and_legacy_crate_purge() {
    let fmt_src = include_str!("../../../selfhost/fmt/fmt.pilot");
    let snap = BootstrapSnapshot::global();
    let mut tables = snap.tables.clone();
    let mut vm = snap.vm.clone();
    let res = eval_file_with_tables(fmt_src, &mut tables, &mut vm);
    assert!(
        res.is_ok(),
        "selfhost/fmt/fmt.pilot failed analysis: {:?}",
        res.err()
    );

    let val = run_pilot_script(
        &[fmt_src],
        r#"
fn main() -> i64 {
    let messy = "pub fn compute(x: i64) -> i64 {\nlet y = x + 1;\nreturn y;\n}\n";
    let once = format_pilot_source(messy);
    let twice = format_pilot_source(once);
    if once != twice { return 10; }
    if !verify_format_idempotent(messy) { return 11; }
    return 1;
}
"#,
    );
    assert_eq!(val.as_int(), Some(1));

    // Deliverable 53.4: Verify crates/pilot-syntax and crates/pilot-fmt are deleted
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    assert!(
        !root.join("crates/pilot-syntax").exists(),
        "crates/pilot-syntax must be deleted from disk"
    );
    assert!(
        !root.join("crates/pilot-fmt").exists(),
        "crates/pilot-fmt must be deleted from disk"
    );
}
