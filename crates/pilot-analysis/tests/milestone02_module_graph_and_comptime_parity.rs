//! ============================================================================
//! Milestone 2 Verification Suite:
//! Unified CLI Runtime, Transitive Module Graph & comptime Reflection Parity
//! ============================================================================

use std::process::Command;

use pilot_analysis::{ComptimeEngine, PhysicsRegistry};
use pilot_ast::AstArena;
use pilot_parser::{parse, parse_unit};
use pilot_rt::PilotVal;
use pilot_span::{FileId, SourceMap};
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/vm_outcome.rs"]
mod vm_outcome;
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
                "mg_normalize_path",
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

/// The Pilot loader's units, as `pilot run` links them.
const LOADER: [&str; 3] = [
    "stdlib/core/file.pilot",
    "selfhost/diagnostics/model.pilot",
    "selfhost/loader/module_graph.pilot",
];

/// Links the loader with `tail` as the entry unit and runs it with the
/// engine's file intrinsics, returning the VM and the entry's value.
fn run_loader_val(tail: &str) -> (PilVirtualMachine, PilotVal) {
    let root = anticheat::workspace_root();
    let mut arena = AstArena::new();
    let mut base = 0u32;
    let mut sources: Vec<String> = LOADER
        .iter()
        .map(|p| std::fs::read_to_string(root.join(p)).expect("read loader unit"))
        .collect();
    sources.push(tail.to_string());
    for (i, src) in sources.iter().enumerate() {
        arena = parse_unit(arena, src, FileId(i as u32), base);
        base += src.len() as u32 + 1;
    }
    assert!(arena.diagnostics.is_empty(), "loader must parse: {:?}", arena.diagnostics);
    let bin = BytecodeCompiler::compile_linked(&arena);
    let errs: Vec<_> = bin.diagnostics.iter().map(|d| d.message.clone()).collect();
    assert!(errs.is_empty(), "loader must compile: {errs:?}");
    let (flat, syms) = pilot_analysis::flatten_ast(&arena, arena.root);
    let mut phy = PhysicsRegistry::new(flat, syms);
    let mut vm = PilVirtualMachine::new();
    vm.engine_handler = Some(Box::new(move |op, a| {
        pilot_analysis::handle_engine_intrinsic(&mut phy, op, a)
    }));
    vm.load_binary(bin);
    let out = vm.run();
    if let Some(fault) = vm.fault.take() {
        panic!("loader faulted: {}", fault.message);
    }
    (vm, out)
}

fn run_loader(tail: &str) -> Option<i64> {
    run_loader_val(tail).1.as_int()
}

fn run_loader_str(tail: &str) -> Option<String> {
    let (vm, v) = run_loader_val(tail);
    Some(vm.display(v))
}

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

    // 1b. Verify CRLF (\r\n) line endings compute exact coordinates without skew
    let f_crlf = sm.add_file("crlf.pilot", "let a = 1;\r\nlet b = 2;\r\nreturn a + b;\r\n");
    let (p_crlf, loc_crlf) = sm.lookup_loc(f_crlf, 12).expect("lookup in f_crlf");
    assert_eq!(p_crlf, "crlf.pilot");
    assert_eq!(loc_crlf.line, 2);
    assert_eq!(loc_crlf.col, 1);

    // 2. Verify linked units keep their own file identity: every unit is
    //    parsed into one arena at its own base offset, the entry unit starts
    //    at 0, and any node maps back to the file it came from
    let entry_src = "use util;\nfn main() => twice(4);\n";
    let util_src = "pkg util;\npub fn twice(x: i64) -> i64 => x * 2;\n";
    let util_base = entry_src.len() as u32 + 1;
    let arena = parse_unit(AstArena::new(), entry_src, FileId(0), 0);
    let arena = parse_unit(arena, util_src, FileId(1), util_base);
    assert_eq!(arena.units.len(), 2, "Each linked file must be its own unit");
    assert!(arena.diagnostics.is_empty(), "Both units must parse: {:?}", arena.diagnostics);
    let entry = &arena.units[0];
    let util = &arena.units[1];
    assert_eq!(arena.span(entry.root).start, 0, "Entry unit span MUST start at 0");
    assert_eq!(arena.span(util.root).start, util_base, "Second unit MUST start at its own base");
    let last = pilot_ast::NodeId(arena.nodes.len() as u32 - 1);
    assert_eq!(arena.unit_of(last).map(|u| u.file), Some(FileId(1)));
    assert_eq!(arena.unit_of(entry.root).map(|u| u.file), Some(FileId(0)));

    // 3. Verify a span in the second unit maps back to its own line/col
    let twice_at = util_base + util_src.find("twice").unwrap() as u32;
    let mut sm2 = SourceMap::new();
    sm2.add_file("app.pilot", entry_src);
    let fu = sm2.add_file("util.pilot", util_src);
    let (pu, lu) = sm2.lookup_loc(fu, twice_at - util_base).expect("lookup in util");
    assert_eq!((pu, lu.line, lu.col), ("util.pilot", 2, 8));

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

    // 2. Verify the comptime ENV record (obtained through `get_env`; integers are
    //    never record handles) is queryable via map_get at comptime
    let env_arena = parse("let c = __engine(\"map_get\", __engine(\"get_env\"), \"is_comptime\"); return c;\n");
    let env_res = engine.eval_ast_node(&env_arena, env_arena.root).expect("eval is_comptime");
    assert_eq!(env_res.as_int(), Some(1), "is_comptime must be 1 at comptime");

    // 3. Verify multiplatform target physics map aliases ("arch", "pointer_bytes", "endian")
    let arch_arena = parse("let a = __engine(\"map_get\", __engine(\"map_get\", __engine(\"get_env\"), \"target\"), \"arch\"); return a;\n");
    let arch_res = engine.eval_ast_node(&arch_arena, arch_arena.root).expect("eval arch");
    let interner = pilot_intern::Interner::new();
    assert_eq!(arch_res.as_str(&interner), Some(std::env::consts::ARCH));

    let ptr_arena = parse("let p = __engine(\"map_get\", __engine(\"map_get\", __engine(\"get_env\"), \"target\"), \"pointer_bytes\"); return p;\n");
    let ptr_res = engine.eval_ast_node(&ptr_arena, ptr_arena.root).expect("eval pointer_bytes");
    assert_eq!(ptr_res.as_int(), Some(std::mem::size_of::<usize>() as i64));

    let endian_arena = parse("let e = __engine(\"map_get\", __engine(\"map_get\", __engine(\"get_env\"), \"target\"), \"endian\"); return e;\n");
    let endian_res = engine.eval_ast_node(&endian_arena, endian_arena.root).expect("eval endian");
    let expected_endian = if cfg!(target_endian = "little") { "little" } else { "big" };
    assert_eq!(endian_res.as_str(&interner), Some(expected_endian));

    // 3c. `env()` in stdlib/core/reflect.pilot reads the same ENV.target
    //     record at comptime, and stops at a seam when no record exists.
    let reflect_src = std::fs::read_to_string(anticheat::workspace_root().join("stdlib/core/reflect.pilot"))
        .expect("read reflect.pilot");
    let pending_src = std::fs::read_to_string(anticheat::workspace_root().join("stdlib/core/pending.pilot"))
        .expect("read pending.pilot");
    let env_section = format!(
        "{pending_src}\n{}",
        &reflect_src[reflect_src
            .find("pub struct TargetDescriptor")
            .map(|i| reflect_src[..i].rfind("[Layout(c_abi)]").unwrap_or(i))
            .expect("TargetDescriptor in reflect.pilot")..]
    );
    let env_arch_src = format!("{env_section}\nlet e = env();\nreturn e.target.arch + \"|\" + e.target.pointer_bytes + \"|\" + e.is_comptime;\n");
    let env_arch_arena = parse(&env_arch_src);
    assert!(env_arch_arena.diagnostics.is_empty(), "env() section must parse: {:?}", env_arch_arena.diagnostics);
    let env_arch = engine.eval_ast_node(&env_arch_arena, env_arch_arena.root).expect("eval env()");
    let expected_env = format!("{}|{}|true", std::env::consts::ARCH, std::mem::size_of::<usize>());
    assert_eq!(env_arch.as_str(&interner), Some(expected_env.as_str()), "env() must report the host target at comptime");

    // SIMD width, cache line size and opt level are not known without a
    // `--target` descriptor from the compile driver: env() hands them out as
    // pending values, never as invented numbers.
    for (field, pending_id) in [
        ("simd_width_bits", "reflect.target_simd_width"),
        ("cache_line_bytes", "reflect.target_cache_line"),
        ("opt_level", "reflect.target_opt_level"),
    ] {
        let src = format!("{env_section}\nlet e = env();\nreturn e.target.{field}.id;\n");
        let arena = parse(&src);
        let got = engine.eval_ast_node(&arena, arena.root).expect("eval pending target field");
        assert_eq!(got.as_str(&interner), Some(pending_id), "ENV.target.{field} must be pending");
        assert!(
            !engine_target_has(&mut engine, field),
            "the engine must not publish an invented ENV.target.{field}"
        );
    }
    let backend_src = format!("{env_section}\nlet e = env();\nreturn e.target.backend;\n");
    let backend_arena = parse(&backend_src);
    let backend = engine.eval_ast_node(&backend_arena, backend_arena.root).expect("eval backend");
    assert_eq!(backend.as_str(&interner), Some("vm"), "comptime code runs on the seed VM");
    let read_src = format!("{env_section}\nlet t = TargetDescriptor {{ arch: \"\", os: \"\", pointer_bytes: 0, endian: \"\", backend: \"\", simd_width_bits: pending(\"reflect.target_simd_width\", \"x\"), cache_line_bytes: pending(\"reflect.target_cache_line\", \"x\"), opt_level: pending(\"reflect.target_opt_level\", \"x\") }};\nreturn t.simd_width_bits.get();\n");
    let read_arena = parse(&read_src);
    let mut read_vm = PilVirtualMachine::new();
    read_vm.load_binary(BytecodeCompiler::compile(&read_arena, read_arena.root));
    let read_out = vm_outcome::outcome(&mut read_vm);
    assert!(
        read_out.starts_with("FAULT: SEAM001: unconnected seam `reflect.target_simd_width`"),
        "reading a pending target field must stop at SEAM001, got: {read_out}"
    );

    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&env_arch_arena, env_arch_arena.root));
    let run_out = vm_outcome::outcome(&mut vm);
    assert!(
        run_out.starts_with("FAULT: SEAM001: unconnected seam `reflect.target_descriptor`"),
        "env() without an ENV.target record must stop at its seam, got: {run_out}"
    );

    for (call, seam_id) in [
        ("e.lookup_type(\"i64\")", "reflect.lookup_type"),
        ("e.symbols_in_pkg(\"core\")", "reflect.symbols_in_pkg"),
        ("e.ast_root(0)", "reflect.ast_root"),
    ] {
        let src = format!(
            "{env_section}\nlet t = TargetDescriptor {{ arch: \"\", os: \"\", pointer_bytes: 0, endian: \"\", backend: \"\", simd_width_bits: pending(\"reflect.target_simd_width\", \"x\"), cache_line_bytes: pending(\"reflect.target_cache_line\", \"x\"), opt_level: pending(\"reflect.target_opt_level\", \"x\") }};\n\
             let e = CompileEnvView {{ target: t, is_comptime: false, unit_count: 0, symbol_count: 0 }};\nreturn {call};\n"
        );
        let arena = parse(&src);
        let mut vm = PilVirtualMachine::new();
        vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
        let out = vm_outcome::outcome(&mut vm);
        assert!(out.contains(&format!("SEAM001: unconnected seam `{seam_id}`")), "{call} must stop at {seam_id}, got: {out}");
    }

    // 4. Verify string return values are preserved in ComptimeValue
    let str_arena = parse("return \"pilot_env_ok\";\n");
    let str_res = engine.eval_ast_node(&str_arena, str_arena.root).expect("eval str");
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

    // 2. Test the Pure-Pilot module graph on a real project: imports resolve
    //    by convention and through the package index, and the link order puts
    //    the prelude first, dependencies before dependents, the entry last
    let proj = std::env::temp_dir().join(format!("pilot_m2_loader_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&proj);
    std::fs::create_dir_all(proj.join("lib")).expect("create project");
    let write = |rel: &str, text: &str| std::fs::write(proj.join(rel), text).expect("write unit");
    write("app.pilot", "pub lvl(2) pkg Pilot.App.Main;\nuse util;\nuse Pilot.App.Shapes;\n");
    write("util.pilot", "pkg util;\npub fn twice(x: i64) -> i64 => x * 2;\n");
    write("lib/geometry.pilot", "pkg Pilot.App.Shapes;\nuse util;\n");
    let proj_dir = proj.to_str().unwrap().replace('\\', "/");
    let root_dir = root.to_str().unwrap().replace('\\', "/");
    let order = |entry: &str| format!(
        r#"
        fn check() -> i64 {{
            let plan = mg_link_plan("{entry}", "{root_dir}");
            let paths = plan[0];
            let diags = plan[1];
            if diags.len() != 0 {{ return 0; }}
            // The prelude's own imports link before it, then the program.
            var prelude = -1;
            var util = -1;
            var geometry = -1;
            var i = 0;
            for p in paths {{
                if p.ends_with("stdlib/core/prelude.pilot") {{ prelude = i; }}
                if p == "{proj_dir}/util.pilot" {{ util = i; }}
                if p == "{proj_dir}/lib/geometry.pilot" {{ geometry = i; }}
                i = i + 1;
            }}
            let last = paths.len() - 1;
            if prelude < 0 || util < 0 || geometry < 0 {{ return 1; }}
            if !(prelude < util && util < geometry && geometry == last - 1) {{ return 2; }}
            if paths[last] != "{proj_dir}/app.pilot" {{ return 3; }}
            return 200;
        }}
        check();
        "#
    );
    let plan_src = order(&format!("{proj_dir}/app.pilot"));
    assert_eq!(run_loader(&plan_src), Some(200), "Module graph must link the prelude, dependencies, then the entry");

    // 3. Test Pure-Pilot Module Graph Path Normalization (Windows backslashes & relative prefixes)
    let mg_norm_tail = r#"
        fn check() -> i64 {
            let p1 = mg_normalize_path(".\\src\\sub\\mod.pilot");
            let p2 = mg_normalize_path("stdlib\\core\\reflect.pilot");
            let p3 = mg_normalize_path("c:\\projects\\pilot\\main.pilot");
            if p1 == "src/sub/mod.pilot" && p2 == "stdlib/core/reflect.pilot" && p3 == "C:/projects/pilot/main.pilot" {
                return 300;
            }
            return 0;
        }
        check();
    "#;
    assert_eq!(run_loader(mg_norm_tail), Some(300), "Module graph path normalization must convert Windows paths and normalize drive letters");

    // 4. Test a Windows-style entry path links the same files, and an import
    //    cycle and a missing module each fail with their code
    let win_entry = format!("{proj_dir}/app.pilot").replace('/', "\\\\");
    assert_eq!(run_loader(&order(&win_entry)), Some(200), "Module graph must resolve Windows-style backslash paths seamlessly");
    write("util.pilot", "pkg util;\nuse Pilot.App.Main;\n");
    write("missing.pilot", "pkg Pilot.App.Missing;\nuse no.such.module;\n");
    let codes = |entry: &str| format!(
        r#"
        fn check() -> str {{
            let diags = mg_link_plan("{proj_dir}/{entry}", "{root_dir}")[1];
            if diags.len() != 1 {{ return "count"; }}
            return diags[0][0];
        }}
        check();
        "#
    );
    assert_eq!(run_loader_str(&codes("app.pilot")).as_deref(), Some("MOD002"), "An import cycle must report MOD002");
    assert_eq!(run_loader_str(&codes("missing.pilot")).as_deref(), Some("MOD001"), "A missing module must report MOD001");
    let _ = std::fs::remove_dir_all(&proj);

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
    let cli_cap = anticheat::seed_rust_file_loc_cap(&root, 100);
    assert!(cli_lines <= cli_cap, "crates/pilot-cli/src/main.rs LOC ({cli_lines}) must be <= {cli_cap}");

    let span_src = std::fs::read_to_string(root.join("crates/pilot-span/src/lib.rs")).expect("read lib.rs");
    let span_lines = span_src.lines().count();
    let span_cap = anticheat::seed_rust_file_loc_cap(&root, 250);
    assert!(span_lines <= span_cap, "crates/pilot-span/src/lib.rs LOC ({span_lines}) must be <= {span_cap}");

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

/// Whether the comptime engine publishes `field` in its ENV.target record.
fn engine_target_has(engine: &mut ComptimeEngine, field: &str) -> bool {
    let src = format!(
        "let v = __engine(\"map_get\", __engine(\"map_get\", __engine(\"get_env\"), \"target\"), \"{field}\");\nif v == none {{ return 0; }}\nreturn 1;\n"
    );
    let arena = parse(&src);
    engine.eval_ast_node(&arena, arena.root).expect("eval target field probe").as_int() == Some(1)
}
