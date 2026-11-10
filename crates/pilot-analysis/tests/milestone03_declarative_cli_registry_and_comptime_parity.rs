//! ============================================================================
//! Milestone 3 Verification Suite:
//! First-Class Declarative CLI Command Registry & Minimal Pure-Pilot Launcher
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
    milestone_id: 3,
    test_file_rel: "crates/pilot-analysis/tests/milestone03_declarative_cli_registry_and_comptime_parity.rs",
    expected_test_count: 4,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/cli/spec.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "spec_new_flag",
                "spec_new_positional",
                "spec_new_command",
                "spec_command_add_flag",
                "spec_command_add_positional",
                "spec_registry_new",
                "spec_registry_register",
                "spec_registry_count",
                "spec_registry_get",
                "spec_registry_find_by_name",
            ],
            required_code_markers: &[
                "FlagSpec",
                "PositionalSpec",
                "CommandSpec",
                "CommandRegistry",
                "ParsedArgs",
                "ICliHandler",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/cli/parser.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "cli_tokenize",
                "cli_suggest_typo",
                "cli_parse_args",
            ],
            required_code_markers: &[
                "CLI001",
                "CLI002",
                "CLI003",
                "CLI004",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/cli/help_and_comp.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "render_global_help",
                "render_command_help",
                "emit_shell_completions",
                "export_cli_json_schema",
            ],
            required_code_markers: &[
                "_pilot_completions",
                "bash",
                "zsh",
                "fish",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/cli/main.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "default_command_registry",
                "resolve_cli_subcommand",
                "run_cli_entry",
            ],
            required_code_markers: &[
                "default_command_registry",
            ],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

/// Runs the real `pilot` binary with the whitespace-separated `raw_args`
/// from the workspace root; returns stdout, stderr and the exit code.
fn pilot_cli(raw_args: &str) -> (String, String, i32) {
    let out = std::process::Command::new("cargo")
        .args(["run", "-q", "-p", "pilot-cli", "--"])
        .args(raw_args.split_whitespace())
        .current_dir(anticheat::workspace_root())
        .output()
        .expect("run pilot");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// Everything `pilot <raw_args>` wrote, stdout then stderr.
fn eval_cli(raw_args: &str) -> String {
    let (stdout, stderr, _) = pilot_cli(raw_args);
    stdout + &stderr
}

/// Links the CLI units with `tail` as the last statement and runs it.
fn eval_cli_script(tail: &str) -> String {
    let root = anticheat::workspace_root();
    let deps = [
        "stdlib/core/file.pilot",
        "stdlib/core/env.pilot",
        "stdlib/core/primitives/map.pilot",
        "selfhost/cli/spec.pilot",
        "selfhost/cli/parser.pilot",
        "selfhost/cli/help_and_comp.pilot",
        "selfhost/cli/main.pilot",
    ];
    let mut b = String::new();
    for d in deps {
        b.push_str(&fs::read_to_string(root.join(d)).unwrap_or_else(|e| panic!("Failed to read {d}: {e}")));
        b.push('\n');
    }
    let src = format!("{b}\n{tail}");
    let arena = parse(&src);
    let (flat, syms) = pilot_analysis::flatten_ast(&arena, arena.root);
    let (mut vm, mut phy) = (PilVirtualMachine::new(), PhysicsRegistry::new(flat, syms));
    let reg_ptr = &mut phy as *mut PhysicsRegistry;
    vm.engine_handler = Some(Box::new(move |op, a| pilot_analysis::handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, a)));
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    vm_outcome::outcome(&mut vm)
}

/// Parses `raw_args` with the default registry and returns
/// `command|target|output|release` from the parsed flags.
fn eval_parsed(raw_args: &str) -> String {
    let script = format!(
        "let p_res = cli_parse_args(default_command_registry(), \"{raw_args}\");\n\
         let p: ParsedArgs = cli_map_get(p_res, \"parsed\");\n\
         p.command + \"|\" + parsed_get_str(p, \"target\", \"\") + \"|\" + parsed_get_str(p, \"output\", \"\") + \"|\" + parsed_get_bool(p, \"release\");"
    );
    eval_cli_script(&script)
}

#[test]
pub fn test_m3_1_command_registry_and_typo_suggestions() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m3_1_command_registry_and_typo_suggestions",
        &ANTICHEAT_SPEC,
    );

    // 1. Verify unknown command emits CLI001 with Levenshtein typo suggestion
    let out_bld = eval_cli("bld");
    assert!(
        out_bld.contains("CLI001: Unknown command 'bld'. Did you mean 'build'?"),
        "Expected typo suggestion for 'bld' -> 'build', got: {out_bld}"
    );

    let out_chekc = eval_cli("chekc");
    assert!(
        out_chekc.contains("CLI001: Unknown command 'chekc'. Did you mean 'check'?"),
        "Expected typo suggestion for 'chekc' -> 'check', got: {out_chekc}"
    );

    // 2. Verify unknown flag emits CLI002 with typo suggestion
    let out_trget = eval_cli("build --trget native");
    assert!(
        out_trget.contains("CLI002: Unknown flag '--trget'. Did you mean '--target'?"),
        "Expected flag typo suggestion for '--trget' -> '--target', got: {out_trget}"
    );

    // 3. Verify missing required flag value emits CLI003
    let out_missing = eval_cli("build --target");
    assert!(
        out_missing.contains("CLI003: Flag '--target' requires a value"),
        "Expected CLI003 missing value, got: {out_missing}"
    );

    // 4. Verify invalid enum choice emits CLI004
    let out_choice = eval_cli("build --target invalid_backend");
    assert!(
        out_choice.contains("CLI004: Invalid value 'invalid_backend' for flag '--target'. Allowed: native,c,llvm,wasm,gpu,asm"),
        "Expected CLI004 invalid choice, got: {out_choice}"
    );

    guard.finish();
}

#[test]
pub fn test_m3_2_posix_gnu_argument_parser_and_passthrough() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m3_2_posix_gnu_argument_parser_and_passthrough",
        &ANTICHEAT_SPEC,
    );

    // 1. Verify --flag=val syntax correctly binds value
    let out_eq = eval_parsed("build --target=wasm -o output.wasm");
    assert_eq!(out_eq, "build|wasm|output.wasm|false", "--target=wasm and -o must bind, got: {out_eq}");

    // 2. Verify short flag syntax -t wasm and -r
    let out_short = eval_parsed("build -t wasm -r");
    assert_eq!(out_short, "build|wasm||true", "short flags -t wasm -r must bind, got: {out_short}");

    // A command that parses but has no handler stops at its seam instead of
    // exiting 0 having done nothing.
    let (_, err_build, code_build) = pilot_cli("build --target=wasm -o output.wasm");
    assert!(
        err_build.contains("SEAM001: unconnected seam `cli.command_handler`") && code_build == 70,
        "build has no handler and must stop at the cli.command_handler seam, got {code_build}: {err_build}"
    );

    // 3. Verify -- passthrough forwards tokens verbatim to the script without
    //    parsing them as flags. `script_args.pilot` prints the argument count,
    //    then each argument in brackets.
    let (out_run, err_run, code_run) = pilot_cli("run tests/conformance/script_args.pilot -- --target=native --release");
    assert_eq!(
        (out_run.as_str(), code_run),
        ("2\n[--target=native]\n[--release]\n", 0),
        "run must pass the tokens after -- to the script, stderr: {err_run}"
    );

    // 4. Verify gate command parses --full and --json: a JSON report that
    //    includes the gates only --full runs.
    let (out_gate, err_gate, code_gate) = pilot_cli("gate --full --json");
    assert_eq!(code_gate, 0, "gate --full --json failed: {err_gate}");
    assert!(out_gate.trim_start().starts_with('{'), "--json must print JSON, got: {out_gate}");
    assert!(out_gate.contains("\"id\":\"rename_litmus\""), "--full must run the rename litmus, got: {out_gate}");

    // 5. Verify backward-compatible resolve_cli_subcommand API
    let root = anticheat::workspace_root();
    let deps = [
        "stdlib/core/file.pilot",
        "stdlib/core/env.pilot",
        "stdlib/core/primitives/map.pilot",
        "selfhost/cli/spec.pilot",
        "selfhost/cli/parser.pilot",
        "selfhost/cli/help_and_comp.pilot",
        "selfhost/cli/main.pilot",
    ];
    let mut b = String::new();
    for d in deps {
        b.push_str(&fs::read_to_string(root.join(d)).unwrap());
        b.push('\n');
    }
    let src_resolve = format!("{b}\nlet id = resolve_cli_subcommand(\"build\"); return id;");
    let arena = parse(&src_resolve);
    let (flat, syms) = pilot_analysis::flatten_ast(&arena, arena.root);
    let (mut vm, mut phy) = (PilVirtualMachine::new(), PhysicsRegistry::new(flat, syms));
    let reg_ptr = &mut phy as *mut PhysicsRegistry;
    vm.engine_handler = Some(Box::new(move |op, a| pilot_analysis::handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, a)));
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    let id_val = vm.run().as_int().unwrap_or(0);
    assert!(id_val > 0, "resolve_cli_subcommand(\"build\") must return positive index, got: {id_val}");

    guard.finish();
}

#[test]
pub fn test_m3_3_dynamic_shell_completions_and_schema_json() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m3_3_dynamic_shell_completions_and_schema_json",
        &ANTICHEAT_SPEC,
    );

    // 1. Verify dynamic command registration produces updated shell completions without compiler edits
    let root = anticheat::workspace_root();
    let deps = [
        "stdlib/core/file.pilot",
        "stdlib/core/env.pilot",
        "stdlib/core/primitives/map.pilot",
        "selfhost/cli/spec.pilot",
        "selfhost/cli/parser.pilot",
        "selfhost/cli/help_and_comp.pilot",
        "selfhost/cli/main.pilot",
    ];
    let mut b = String::new();
    for d in deps {
        b.push_str(&fs::read_to_string(root.join(d)).unwrap());
        b.push('\n');
    }

    let dyn_src = format!(
        "{b}\n\
        let reg = default_command_registry();\n\
        let custom_cmd = spec_new_command(\"deploy\", \"\", \"Cloud & Ops\", \"Deploy artifact to cloud provider\", \"pilot deploy --env prod\");\n\
        spec_command_add_flag(custom_cmd, spec_new_flag(\"env\", \"e\", 5, \"dev\", \"\", \"dev,staging,prod\", false, false, \"Target cloud environment\"));\n\
        let _ = spec_registry_register(reg, custom_cmd);\n\
        let bash_comp = emit_shell_completions(reg, \"bash\");\n\
        return bash_comp;\n"
    );

    let arena = parse(&dyn_src);
    let (flat, syms) = pilot_analysis::flatten_ast(&arena, arena.root);
    let (mut vm, mut phy) = (PilVirtualMachine::new(), PhysicsRegistry::new(flat, syms));
    let reg_ptr = &mut phy as *mut PhysicsRegistry;
    vm.engine_handler = Some(Box::new(move |op, a| pilot_analysis::handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, a)));
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    let bash_output = vm_outcome::outcome(&mut vm);

    assert!(
        bash_output.contains("deploy"),
        "Dynamic command 'deploy' must appear in generated bash completions, got: {bash_output}"
    );
    assert!(
        bash_output.contains("--env"),
        "Dynamic flag '--env' must appear in generated bash completions, got: {bash_output}"
    );

    // 2. Verify export_cli_json_schema outputs valid JSON schema structure
    let out_schema = eval_cli("--schema-json");
    assert!(
        out_schema.contains("\"$schema\": \"http://json-schema.org/draft-07/schema#\""),
        "Schema must have draft-07 URI, got: {out_schema}"
    );
    assert!(out_schema.contains("\"title\": \"PilotCLI\""), "Schema must have title PilotCLI");
    assert!(out_schema.contains("\"name\": \"build\""), "Schema must declare 'build' command");
    assert!(out_schema.contains("\"name\": \"target\""), "Schema must declare 'target' flag");

    guard.finish();
}

#[test]
pub fn test_m3_4_zero_stdout_scraping_and_cli_shrink_ceiling() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_m3_4_zero_stdout_scraping_and_cli_shrink_ceiling",
        &ANTICHEAT_SPEC,
    );

    let root = anticheat::workspace_root();

    // 1. Verify crates/pilot-cli/src/main.rs <= 35 LOC
    let main_rs_path = root.join("crates/pilot-cli/src/main.rs");
    let main_rs = fs::read_to_string(&main_rs_path).expect("read crates/pilot-cli/src/main.rs");
    let line_cnt = main_rs.lines().count();
    let main_cap = anticheat::seed_rust_file_loc_cap(&root, 35);
    assert!(
        line_cnt <= main_cap,
        "crates/pilot-cli/src/main.rs must be <= {} LOC (found {} lines)!",
        main_cap,
        line_cnt
    );

    // 2. Verify zero subcommand string names in crates/pilot-cli/src/main.rs
    let banned_subcommands = [
        "\"run\"", "\"build\"", "\"compile\"", "\"check\"", "\"fmt\"",
        "\"test\"", "\"gate\"", "\"pkg\"", "\"deps\"", "\"tree\"",
        "\"venv\"", "\"repl\"", "\"debug\"", "\"lsp\"", "\"perf\"",
        "\"emit\"", "\"from\"", "\"map\"", "\"ide-sync\"", "\"completions\"",
    ];
    for subcmd in banned_subcommands {
        assert!(
            !main_rs.contains(subcmd),
            "crates/pilot-cli/src/main.rs must not hardcode subcommand string {subcmd}!"
        );
    }

    // 3. Verify pkg_cmd.pilot writes files directly via __engine("fs_write_text") without @@WRITE: scraping
    let pkg_cmd_path = root.join("selfhost/cli/pkg_cmd.pilot");
    let pkg_cmd_src = fs::read_to_string(&pkg_cmd_path).expect("read pkg_cmd.pilot");
    assert!(
        !pkg_cmd_src.contains("@@WRITE:"),
        "selfhost/cli/pkg_cmd.pilot must contain zero '@@WRITE:' stdout scraping markers!"
    );
    assert!(
        pkg_cmd_src.contains("fs_write_text"),
        "selfhost/cli/pkg_cmd.pilot must use direct __engine(\"fs_write_text\") file writes!"
    );

    guard.finish();
}
