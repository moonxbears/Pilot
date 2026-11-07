use pilot_analysis::{eval_file_with_tables, BootstrapSnapshot};
use pilot_parser::parse;
use pilot_rt::PilotVal;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

fn eval_pilot_expr(module_src: &str, expr: &str) -> PilotVal {
    let script = format!("{}\n{};\n", module_src, expr);
    let arena = parse(&script);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    vm.run()
}

#[test]
fn test_52_1_pilot_preprocessor_conditional_compilation_and_diagnostics_in_vm() {
    let snap = BootstrapSnapshot::global();
    let mut tables = snap.tables.clone();
    let mut vm = snap.vm.clone();

    let preproc_src = include_str!("../../../selfhost/preprocessor.pilot");
    eval_file_with_tables(preproc_src, &mut tables, &mut vm)
        .expect("selfhost/preprocessor.pilot must evaluate cleanly");

    // 1. Test boolean condition evaluation (`DEBUG || (LINUX && !X86)`)
    let cond1 = eval_pilot_expr(
        preproc_src,
        r#"eval_preproc_condition("DEBUG || (LINUX && !X86)", "LINUX,ARM64")"#,
    );
    assert_eq!(cond1.as_bool(), Some(true));

    let cond2 = eval_pilot_expr(
        preproc_src,
        r#"eval_preproc_condition("DEBUG || (LINUX && !X86)", "LINUX,X86")"#,
    );
    assert_eq!(cond2.as_bool(), Some(false));

    // 2. Test conditional source preprocessing (`#define`, `#if`, `#elif`, `#else`, `#endif`)
    let active_out = eval_pilot_expr(
        preproc_src,
        r##"preprocess_source("#define FEATURE_GPU\n#if DEBUG\nlet mode = 1;\n#elif FEATURE_GPU\nlet mode = 2;\n#else\nlet mode = 3;\n#endif\n", "")"##,
    )
    .to_string_repr();
    assert!(active_out.contains("let mode = 2;"));
    assert!(!active_out.contains("let mode = 1;"));
    assert!(!active_out.contains("let mode = 3;"));

    // 3. Test PREPROC001..PREPROC008 diagnostic codes
    let unclosed_if = eval_pilot_expr(
        preproc_src,
        r##"validate_preprocessor_directives("#if DEBUG\nlet x = 1;\n", "DEBUG")"##,
    )
    .to_string_repr();
    assert_eq!(unclosed_if, "PREPROC004");

    let stray_elif = eval_pilot_expr(
        preproc_src,
        r##"validate_preprocessor_directives("#elif DEBUG\n", "")"##,
    )
    .to_string_repr();
    assert_eq!(stray_elif, "PREPROC002");

    let stray_endregion = eval_pilot_expr(
        preproc_src,
        r##"validate_preprocessor_directives("#endregion\n", "")"##,
    )
    .to_string_repr();
    assert_eq!(stray_endregion, "PREPROC008");
}

#[test]
fn test_52_3_pilot_unit_decl_fstring_and_keyword_receptors_in_vm() {
    let unit_src = include_str!("../../../stdlib/core/syntax/unit_decl.pilot");
    let fstr_src = include_str!("../../../stdlib/core/syntax/fstring.pilot");
    let kw_src = include_str!("../../../stdlib/core/keywords.pilot");

    // 1. Unit scale & family lookup in Pilot
    assert_eq!(
        eval_pilot_expr(unit_src, r#"unit_family_for_suffix("ms")"#).as_int(),
        Some(1)
    );
    assert_eq!(
        eval_pilot_expr(unit_src, r#"unit_scale_to_base("KB")"#).as_int(),
        Some(1024)
    );

    // 2. f"..." interpolation hole extraction & desugaring in Pilot
    assert_eq!(
        eval_pilot_expr(fstr_src, r#"count_fstring_holes("hello {user}, score={pts + 1}!")"#).as_int(),
        Some(2)
    );
    assert_eq!(
        eval_pilot_expr(fstr_src, r#"extract_fstring_hole_expr("hello {user}, score={pts + 1}!", 1)"#)
            .to_string_repr(),
        "pts + 1"
    );
    assert_eq!(
        eval_pilot_expr(fstr_src, r#"desugar_fstring_to_concat("hello {user}!")"#).to_string_repr(),
        r#""hello " + (user) + "!""#
    );

    // 3. :ns/kw hierarchical keyword splitting in Pilot
    assert_eq!(
        eval_pilot_expr(kw_src, r#"keyword_namespace(":http/ok")"#).to_string_repr(),
        "http"
    );
    assert_eq!(
        eval_pilot_expr(kw_src, r#"keyword_name(":http/ok")"#).to_string_repr(),
        "ok"
    );
}
