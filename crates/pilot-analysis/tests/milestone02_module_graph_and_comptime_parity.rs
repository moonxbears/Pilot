//! ============================================================================
//! Milestone 2 Verification Suite:
//! Unified CLI Runtime, Transitive Module Graph & comptime Reflection Parity
//! ============================================================================

use std::process::Command;

use pilot_analysis::{AnalysisPipeline, ComptimeEngine};
use pilot_parser::parse;
use pilot_span::{FileId, SourceMap};
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 2,
    test_file_rel: "crates/pilot-analysis/tests/milestone02_module_graph_and_comptime_parity.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/loader/module_graph.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "mg_default_search_roots",
                "mg_resolve_import_path",
                "mg_extract_package_info",
                "mg_build_graph",
            ],
            required_code_markers: &[
                "FileUnitId",
                "SourceSpan",
                "ModuleNode",
                "ModuleGraph",
                "ModuleDiagnostic",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/loader/pragma_filter.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "pragma_solve_intervals",
                "pragma_is_suppressed",
            ],
            required_code_markers: &["PragmaInterval"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/core/reflect.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "env",
            ],
            required_code_markers: &[
                "TargetDescriptor",
                "CompileEnvView",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

#[test]
pub fn test_m2_1_exact_line_col_preservation_without_span_drift() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m2_1_exact_line_col_preservation_without_span_drift",
        &ANTICHEAT_SPEC,
    );

    // 1. Verify SourceMap multi-file mapping & 1-based exact coordinates
    let mut sm = SourceMap::new();
    let f0 = sm.add_file("app.pilot", "use core.mem;\nuse core.reflect;\nlet x = 10;\nreturn x;\n");
    let f1 = sm.add_file("lib.pilot", "pub fn mul(a: i64, b: i64) -> i64 => a * b;\n");
    assert_eq!(sm.file_count(), 2);
    assert_eq!(f0, FileId(0));
    assert_eq!(f1, FileId(1));

    // 'let x = 10;' is on line 3, col 1 of app.pilot (offset 32)
    let (path0, loc0) = sm.lookup_loc(f0, 32).expect("lookup in f0");
    assert_eq!(path0, "app.pilot");
    assert_eq!(loc0.line, 3);
    assert_eq!(loc0.col, 1);

    // 'mul' in lib.pilot is on line 1, col 8 (offset 7)
    let (path1, loc1) = sm.lookup_loc(f1, 7).expect("lookup in f1");
    assert_eq!(path1, "lib.pilot");
    assert_eq!(loc1.line, 1);
    assert_eq!(loc1.col, 8);

    // 2. Verify AnalysisPipeline loads dependencies without shifting entry file spans
    let root = anticheat::workspace_root();
    let (arena, phy) = AnalysisPipeline::load_program(root.join("stdlib/core/bootstrap.pilot").to_str().unwrap());
    assert!(!arena.nodes.is_empty(), "Bootstrap AST nodes must be loaded");
    assert!(!phy.symbol_strings.is_empty(), "Symbols must be mounted");

    // The root node span of bootstrap.pilot MUST start at 0
    let root_span = arena.span(arena.root);
    assert_eq!(root_span.start, 0, "Entry file span MUST start at 0 (no pre-concatenation offset drift)");

    guard.finish();
}

#[test]
pub fn test_m2_2_comptime_and_runtime_env_reflection_parity() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m2_2_comptime_and_runtime_env_reflection_parity",
        &ANTICHEAT_SPEC,
    );

    let mut engine = ComptimeEngine::new();

    // 1. Verify ComptimeEngine evaluates basic arithmetic
    let arena = parse("let x = 42;\nreturn x;\n");
    let res = engine.eval_ast_node(&arena, arena.root).expect("eval basic int");
    assert_eq!(res.as_int(), Some(42));

    // 2. Verify ENV map at index 0 is queryable via map_get at comptime
    let env_arena = parse("let c = __engine(\"map_get\", 0, \"is_comptime\"); return c;\n");
    let env_res = engine.eval_ast_node(&env_arena, env_arena.root).expect("eval is_comptime");
    assert_eq!(env_res.as_int(), Some(1), "is_comptime must be 1 at comptime");

    // 3. Verify target physics map at index 1 is queryable at comptime
    let simd_arena = parse("let s = __engine(\"map_get\", 1, \"simd_width_bits\"); return s;\n");
    let simd_res = engine.eval_ast_node(&simd_arena, simd_arena.root).expect("eval simd_width_bits");
    assert_eq!(simd_res.as_int(), Some(128));

    let cache_arena = parse("let cl = __engine(\"map_get\", 1, \"cache_line_bytes\"); return cl;\n");
    let cache_res = engine.eval_ast_node(&cache_arena, cache_arena.root).expect("eval cache_line_bytes");
    assert_eq!(cache_res.as_int(), Some(64));

    // 4. Verify string return values are preserved in ComptimeValue
    let str_arena = parse("return \"pilot_env_ok\";\n");
    let str_res = engine.eval_ast_node(&str_arena, str_arena.root).expect("eval str");
    let interner = pilot_intern::Interner::new();
    assert_eq!(str_res.as_str(&interner), Some("pilot_env_ok"));

    guard.finish();
}

#[test]
pub fn test_m2_3_pure_pilot_module_graph_and_pragma_suppression() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m2_3_pure_pilot_module_graph_and_pragma_suppression",
        &ANTICHEAT_SPEC,
    );

    let root = anticheat::workspace_root();

    // 1. Test Pure-Pilot Pragma Interval Solver
    let pragma_src = std::fs::read_to_string(root.join("selfhost/loader/pragma_filter.pilot")).expect("read pragma_filter.pilot");
    let test_tail = r#"
        let test_src = "let x = 1;\n#pragma warning disable TYPE001\nlet y = 2;\n#pragma warning restore TYPE001\nlet z = 3;\n";
        let intervals = pragma_solve_intervals(test_src);
        let s1 = pragma_is_suppressed(intervals, 0, "TYPE001", false);
        let s2 = pragma_is_suppressed(intervals, 45, "TYPE001", false);
        let s3 = pragma_is_suppressed(intervals, 95, "TYPE001", false);
        if s1 == false && s2 == true && s3 == false { return 100; }
        return 0;
    "#;
    let full_src = format!("{pragma_src}\n{test_tail}\n");
    let arena = parse(&full_src);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    let res = vm.run().as_int();
    assert_eq!(res, Some(100), "Pragma interval solver must correctly suppress disabled ranges");

    // 2. Test Pure-Pilot Module Graph Package Extraction
    let mg_src = std::fs::read_to_string(root.join("selfhost/loader/module_graph.pilot")).expect("read module_graph.pilot");
    let mg_test_tail = r#"
        let code = "pub lvl(2) pkg Pilot.App.Main;\nuse core.mem;\nuse core.reflect;\n";
        let imports = glist_new();
        let node = mg_extract_package_info(code, imports);
        let imp_count = glist_len(imports);
        if node.pkg_name == "Pilot.App.Main" && node.clearance_lvl == 2 && imp_count == 2 {
            return 200;
        }
        return 0;
    "#;
    let mg_full = format!("{mg_src}\n{mg_test_tail}\n");
    let mg_arena = parse(&mg_full);
    let mut mg_vm = PilVirtualMachine::new();
    mg_vm.load_binary(BytecodeCompiler::compile(&mg_arena, mg_arena.root));
    assert_eq!(mg_vm.run().as_int(), Some(200), "Module graph must accurately extract package name, lvl, and imports");

    guard.finish();
}

#[test]
pub fn test_m2_4_zero_fake_transpile_stubs_and_crate_ceilings() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m2_4_zero_fake_transpile_stubs_and_crate_ceilings",
        &ANTICHEAT_SPEC,
    );

    let root = anticheat::workspace_root();

    // 1. Verify `crates/pilot-cli/src/main.rs` has ZERO static mocks
    let cli_src = std::fs::read_to_string(root.join("crates/pilot-cli/src/main.rs")).expect("read main.rs");
    assert!(!cli_src.contains("def add(a, b):"), "Static python mock 'def add' MUST NOT exist in main.rs");
    assert!(!cli_src.contains("pub fn mul(x: i64"), "Static rust mock 'pub fn mul' MUST NOT exist in main.rs");

    // 2. Verify crate line counts
    let cli_lines = cli_src.lines().count();
    assert!(cli_lines <= 100, "crates/pilot-cli/src/main.rs LOC ({cli_lines}) must be <= 100");

    let span_src = std::fs::read_to_string(root.join("crates/pilot-span/src/lib.rs")).expect("read lib.rs");
    let span_lines = span_src.lines().count();
    assert!(span_lines <= 250, "crates/pilot-span/src/lib.rs LOC ({span_lines}) must be <= 250");

    // 3. Verify real transpile routing works via `pilot emit python`
    let sample_pilot = "pub fn add_nums(a: i64, b: i64) -> i64 => a + b;\n";
    let tmp_dir = std::env::temp_dir().join("pilot_m2_test");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let sample_path = tmp_dir.join("sample.pilot");
    std::fs::write(&sample_path, sample_pilot).expect("write sample.pilot");

    let py_out = Command::new("cargo")
        .args(["run", "-q", "-p", "pilot-cli", "--", "emit", "python", sample_path.to_str().unwrap()])
        .output()
        .expect("run pilot emit python");
    let py_str = String::from_utf8_lossy(&py_out.stdout);
    assert!(py_str.contains("add_nums"), "Real transpile output must contain function name 'add_nums': got {py_str}");

    let _ = std::fs::remove_dir_all(tmp_dir);

    guard.finish();
}
