use std::path::Path;
use pilot_analysis::{flatten_ast, handle_engine_intrinsic, PhysicsRegistry};
use pilot_parser::parse;
use pilot_rt::PilotVal;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

fn run_pilot_script(src: &str) -> PilotVal {
    let arena = parse(src);
    assert!(
        arena.diagnostics.is_empty(),
        "Parse diagnostics: {:?}",
        arena.diagnostics
    );
    let (flat, syms) = flatten_ast(&arena, arena.root);
    let mut registry = PhysicsRegistry::new(flat, syms);
    registry.source_text = src.to_string();
    let reg_ptr = &mut registry as *mut PhysicsRegistry;
    let mut vm = PilVirtualMachine::new();
    vm.engine_handler = Some(Box::new(move |name, args| {
        let reg = unsafe { &mut *reg_ptr };
        handle_engine_intrinsic(reg, name, args)
    }));
    let chunk = BytecodeCompiler::compile(&arena, arena.root);
    vm.load_binary(chunk);
    vm.run()
}

#[test]
fn test_m60_1_selfhost_pkg_and_venv_manager() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let manifest_src = std::fs::read_to_string(root.join("selfhost/pkg/manifest.pilot")).unwrap();
    let venv_src = std::fs::read_to_string(root.join("selfhost/pkg/venv.pilot")).unwrap();
    let archive_src = std::fs::read_to_string(root.join("selfhost/pkg/archive.pilot")).unwrap();
    let interop_src = std::fs::read_to_string(root.join("selfhost/pkg/interop.pilot")).unwrap();
    let hash_src = std::fs::read_to_string(root.join("stdlib/crypto/hash.pilot")).unwrap();

    let script = format!(
        "{}\n{}\n{}\n{}\n{}\n\
        let mf = generate_default_manifest_json(\"orbit_nav\", \"1.0.0\");\n\
        let main_stub = scaffold_canonical_main_pilot(\"orbit_nav\");\n\
        let sha_hex = sha256_hex_digest(mf);\n\
        let lock = generate_lockfile_json(\"orbit_nav\", \"1.0.0\", sha_hex);\n\
        let cfg = format_pilotvenv_cfg(\".venv\", \"1.1.51\");\n\
        let entry = format_pilpkg_entry(\"src/main.pilot\", main_stub);\n\
        let ok_hdr = verify_pilpkg_header(entry);\n\
        let eco_cargo = detect_package_ecosystem(\"cargo:serde@1.0\");\n\
        let eco_pip = detect_package_ecosystem(\"pip:numpy@2.0\");\n\
        if ok_hdr == 1 && eco_cargo == 1 && eco_pip == 2 {{\n\
            main_stub + \"|\" + lock + \"|\" + cfg\n\
        }} else {{\n\
            \"pkg_failed\"\n\
        }}\n",
        manifest_src, venv_src, archive_src, interop_src, hash_src
    );
    let out = run_pilot_script(&script).to_string_repr();
    assert!(out.contains("pub fn main() -> i64"));
    assert!(!out.contains("def main()"));
    assert!(out.contains("\"lockfileVersion\": 3"));
    assert!(out.contains("include-system-site-packages = false"));
}

#[test]
fn test_m60_2_selfhost_profiler_wcet_optimizer_and_bench() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let prof_src = std::fs::read_to_string(root.join("selfhost/perf/profiler.pilot")).unwrap();
    let wcet_src = std::fs::read_to_string(root.join("selfhost/perf/wcet.pilot")).unwrap();
    let opt_src = std::fs::read_to_string(root.join("selfhost/perf/optimizer.pilot")).unwrap();
    let bench_src = std::fs::read_to_string(root.join("selfhost/bench/bench.pilot")).unwrap();

    let script = format!(
        "{}\n{}\n{}\n{}\n\
        let svg = export_call_stack_flamegraph_svg(\"main\", \"hot_loop\", 100, 75);\n\
        let trace = export_chrome_trace_event(\"hot_loop\", 10, 450);\n\
        let wcet_txt = format_wcet_report(\"guidance_step\", 320, 500);\n\
        let folded = fold_binary_int_constants(3, 14, 3);\n\
        let fold_cnt = count_foldable_binary_nodes();\n\
        let mem_ok = verify_ast_memory_overhead(100, 1600);\n\
        if folded == 42 && fold_cnt >= 1 && mem_ok == 1 {{\n\
            svg + \"|\" + trace + \"|\" + wcet_txt\n\
        }} else {{\n\
            \"perf_failed\"\n\
        }}\n",
        prof_src, wcet_src, opt_src, bench_src
    );
    let out = run_pilot_script(&script).to_string_repr();
    assert!(out.contains("<svg xmlns="));
    assert!(out.contains("fn hot_loop (75 samples)"));
    assert!(out.contains("\"ph\":\"X\""));
    assert!(out.contains("[WCET PASS] fn guidance_step: 320 / 500 cycles"));
}

#[test]
fn test_m60_3_and_4_selfhost_cli_suite_and_crate_purge() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();

    // Verify crates/pilot-pkg, crates/pilot-perf, crates/pilot-bench, and crates/pilot-cli/src/commands.rs are deleted
    for retired in [
        "crates/pilot-pkg",
        "crates/pilot-perf",
        "crates/pilot-bench",
        "crates/pilot-cli/src/commands.rs",
    ] {
        assert!(
            !root.join(retired).exists(),
            "{} must be retired and deleted in M60.4",
            retired
        );
    }

    // Verify crates/pilot-cli/src/main.rs is <= 100 lines
    let cli_main_rs = std::fs::read_to_string(root.join("crates/pilot-cli/src/main.rs")).unwrap();
    let cli_lines = cli_main_rs.lines().count();
    assert!(
        cli_lines <= 100,
        "crates/pilot-cli/src/main.rs must be <= 100 lines, got {}",
        cli_lines
    );

    // Verify selfhost/pilotc.pilot has zero system("pilot build ...") shell-out shims
    let pilotc_src = std::fs::read_to_string(root.join("selfhost/pilotc.pilot")).unwrap();
    assert!(!pilotc_src.contains("system("));
    assert!(!pilotc_src.contains("pilot build"));

    let cli_main = std::fs::read_to_string(root.join("selfhost/cli/main.pilot")).unwrap();
    let build_cmd = std::fs::read_to_string(root.join("selfhost/cli/build_cmd.pilot")).unwrap();
    let test_runner = std::fs::read_to_string(root.join("selfhost/cli/test_runner.pilot")).unwrap();
    let diff_test = std::fs::read_to_string(root.join("selfhost/cli/diff_test.pilot")).unwrap();
    let repl_src = std::fs::read_to_string(root.join("selfhost/cli/repl.pilot")).unwrap();

    let script = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n\
        let c1 = resolve_cli_subcommand(\"build\");\n\
        let t1 = resolve_build_target(\"wasm\");\n\
        let tp = format_test_pass_json(\"tests/a.pilot\", \"a.pilot\", 24, 12);\n\
        let dp = verify_differential_parity(99, 99, 99, 1);\n\
        let rp = format_repl_value(\"42\", -1);\n\
        let pc = compile_module_to_c_source(\"demo\", 0);\n\
        if c1 == 2 && t1 == 3 && dp == 1 {{\n\
            tp + \"|\" + rp + \"|\" + pc\n\
        }} else {{\n\
            \"cli_failed\"\n\
        }}\n",
        cli_main, build_cmd, test_runner, diff_test, repl_src, pilotc_src
    );
    let out = run_pilot_script(&script).to_string_repr();
    assert!(out.contains("\"event\":\"pass\""));
    assert!(out.contains("=> 42 (8B register)"));
    assert!(out.contains("int64_t pilot_entry(void)"));
}
