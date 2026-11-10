//! Milestone 59 Rigorous Uncheatable Verification Suite:
//! Persistent `PilVM` LSP & `VmHook`-Stepped DAP — Retire `pilot-dap`,
//! `pilot-lsp`, `pilot-lsp-macros`, `pilot-db` & `pilot-incremental`.
//!
//! ============================================================================
//! FROZEN-RUST RUNNER CONTRACT (DO NOT MODIFY THIS TEST FILE OR RUST CRATES)
//! ============================================================================
//! All tests in this file must pass simultaneously using ONLY edits to `.pilot`
//! files (`selfhost/ide/vscode_gen.pilot`, `selfhost/lsp/*.pilot`,
//! `selfhost/dap/*.pilot`).
//!
//! Required `.pilot` entrypoints & behavioral contracts:
//!
//! 1. `selfhost/ide/vscode_gen.pilot` (`<= 1,000` LOC):
//!    - Zero calls to `fs_read_text` (any call trips a hard trap in the test VM).
//!    - `pub fn dumps_tm_language(ctrl_re: str, decl_re: str, mod_re: str, type_re: str, lit_re: str, indent: i64) -> str`
//!      (Must weave non-empty regex arguments into valid TextMate JSON.)
//!    - `pub fn dumps_package_json(version: str, indent: i64) -> str`
//!    - `pub fn dumps_language_configuration(indent: i64) -> str`
//!    - `pub fn dumps_snippets(indent: i64) -> str`
//!    All 4 outputs MUST parse strictly via `python3 json.loads`.
//!
//! 2. `selfhost/lsp/handlers.pilot` (`<= 1,000` LOC):
//!    - Uses `nameof(...)` for all seed keyword comparisons and passes `scripts/gate3_check.py`.
//!    - Responds via `__engine("lsp_query_method")`, `__engine("lsp_query_offset")`,
//!      and `__engine("lsp_set_result", payload)` for:
//!      - `"textDocument/hover"`: includes doc comments and declaration signature.
//!      - `"textDocument/completion"`: after `receiver.`, returns `pub` fields/methods of `receiver`'s struct type while strictly excluding `priv` fields.
//!      - `"textDocument/definition"`: returns JSON containing declaration byte span (`"start"` / `"end"`) of the target symbol.
//!      - `"textDocument/references"`: returns JSON array `[ {"start": ..., "end": ...}, ... ]` of exact symbol occurrences (never prefix-matching longer decoy identifiers).
//!      - `"textDocument/prepareRename"` & `"textDocument/rename:<word>"`: returns `"null"` on all 14 seed keywords and registered stdlib keywords, and returns the identifier/edits on valid user identifiers.
//!
//! 3. `selfhost/lsp/ide_db.pilot` (`<= 1,500` LOC):
//!    - `pub fn apply_doc_span_edit(doc: str, start_byte: i64, end_byte: i64, replacement: str) -> str`
//!    - `pub fn should_early_cutoff(old_source: str, new_source: str) -> i64`
//!      Returns `1` ONLY when top-level item signatures (`fn` headers, `struct` fields & visibility, `interface` method signatures) are unchanged and edits are confined to `fn` executable `{ ... }` bodies (ignoring braces inside string literals and comments). Returns `0` whenever `struct` fields, `interface` signatures, `pub`/`priv` modifiers, or `fn` parameter/return types change!
//!
//! 4. `selfhost/lsp/server.pilot` (`<= 1,500` LOC):
//!    - `pub fn encode_lsp_frame(json_payload: str) -> str`
//!    - `pub fn lsp_session_new() -> i64`
//!    - `pub fn lsp_session_feed_stream(sess: i64, framed_stream: str) -> str`
//!      Parses concatenated `Content-Length: <N>\r\n\r\n<json>` LSP messages (`initialize`, `textDocument/didOpen`, `textDocument/didChange`, `textDocument/hover`, `textDocument/completion`, `shutdown`), updates resident document state in memory, and returns concatenated `Content-Length`-framed JSON-RPC responses reflecting live edited symbols (never hardcoded `"resident_pilvm_lsp"` stubs).
//!
//! 5. `selfhost/dap/{server,vm_controller,inspector,repl_eval,cli_dbg}.pilot` (`<= 1,500` LOC each):
//!    - `vm_controller.pilot`:
//!      - `pub fn should_pause_at_pc(step_mode: i64, cur_line: i64, last_stopped_line: i64, breakpoint_line: i64, cur_call_depth: i64, target_call_depth: i64) -> i64`
//!        (`0` = Continue, `1` = StepIn, `2` = StepOver, `3` = StepOut; ignores `cur_line <= 0`).
//!    - `cli_dbg.pilot`:
//!      - `pub fn parse_cli_dbg_action(cmd_line: str) -> i64`
//!        (`"s"`/`"step"` => `1`, `"n"`/`"next"` => `2`, `"finish"`/`"out"` => `3`, `"c"`/`"continue"` => `4`, `"bt"`/`"backtrace"` => `5`, `"b <line>"`/`"break <line>"` => `6`, `"p <expr>"`/`"print <expr>"` => `7`, `"q"`/`"quit"` => `0`)
//!      - `pub fn parse_cli_dbg_arg(cmd_line: str) -> str` (returns trimmed argument after command verb, e.g. `"42"` for `"break 42"` or `"x * 2 + 1"` for `"p x * 2 + 1"`).
//!    - `inspector.pilot`:
//!      - `pub fn format_register_variable_json(var_name: str, reg_slot: i64, val_str: str, layout_id: i64) -> str`
//!      - `pub fn inspect_struct_fields_json(layout_id: i64) -> str`
//!        Queries `layout_metrics` & `layout_field_offset(layout_id, idx)` Name-Blindly to emit a valid JSON array of `{"index": idx, "offset": byte_off}` descriptors for all fields of `layout_id`.
//!    - `repl_eval.pilot`:
//!      - `pub fn eval_dap_watch_expr(env_map: i64, expr_src: str) -> i64`
//!        Evaluates arithmetic/comparison watch expressions (`+`, `-`, `*`, `/`, `%`, parentheses, integer literals, and live variable names stored in `env_map`) and returns the computed `i64`.
//!      - `pub fn format_dap_evaluate_response(seq: i64, result_repr: str) -> str`
//!    - `dap/server.pilot`:
//!      - `pub fn encode_dap_frame(json_payload: str) -> str`
//!      - `pub fn build_dap_stack_trace_frame(seq: i64, fn_name: str, line: i64, col: i64) -> str`
//!      - `pub fn route_dap_command(command: str, seq: i64) -> str`

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_analysis::{
    AnalysisPipeline, flatten_ast, handle_engine_intrinsic, LayoutId, PhysicsGeometry,
    PhysicsRegistry,
};
use pilot_intern::SymbolId;
use pilot_parser::parse;
use pilot_span::LineIndex;
use pilot_vm::{BytecodeCompiler, Opcode, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const DAP_BANNED_MOCKS: &[&str] = &["Apples", "production_cluster_db", "7fff2000"];

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 59,
    test_file_rel: "crates/pilot-analysis/tests/milestone59_persistent_pilvm_lsp_and_dap.rs",
    expected_test_count: 3,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/ide/vscode_gen.pilot",
            max_lines: 1000,
            name_blind: true,
            required_pub_fns: &[
                "dumps_tm_language",
                "dumps_package_json",
                "dumps_language_configuration",
                "dumps_snippets",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &["fs_read_text"],
            forbidden_string_substrings: &[
                "bool|usize|i8",
                "BUILTIN_DOCS",
                "KEYWORD_DOCS",
                "e:/Pilot",
            ],
        },
        anticheat::FileAuditRule {
            path: "selfhost/lsp/handlers.pilot",
            max_lines: 1000,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &["lsp_query_method", "lsp_set_result", "nameof("],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &["bool|usize|i8", "BUILTIN_DOCS", "STR_METHOD_DOCS"],
        },
        anticheat::FileAuditRule {
            path: "selfhost/lsp/ide_db.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["apply_doc_span_edit", "should_early_cutoff"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/lsp/server.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "encode_lsp_frame",
                "lsp_session_new",
                "lsp_session_feed_stream",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &["resident_pilvm_lsp"],
        },
        anticheat::FileAuditRule {
            path: "selfhost/dap/vm_controller.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["should_pause_at_pc"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: DAP_BANNED_MOCKS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/dap/cli_dbg.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["parse_cli_dbg_action", "parse_cli_dbg_arg"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: DAP_BANNED_MOCKS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/dap/inspector.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "format_register_variable_json",
                "inspect_struct_fields_json",
            ],
            required_code_markers: &["layout_field_offset"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: DAP_BANNED_MOCKS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/dap/repl_eval.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["eval_dap_watch_expr", "format_dap_evaluate_response"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: DAP_BANNED_MOCKS,
        },
        anticheat::FileAuditRule {
            path: "selfhost/dap/server.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "encode_dap_frame",
                "build_dap_stack_trace_frame",
                "route_dap_command",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: DAP_BANNED_MOCKS,
        },
    ],
    required_group_pub_fns: &[],
};

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn from_entropy() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x517C_C1B7_2722_0A95);
        let pid = std::process::id() as u64;
        Self {
            state: nanos ^ pid.rotate_left(23) ^ 0xA076_1D64_78BD_642F,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn range_i64(&mut self, lo: i64, hi_inclusive: i64) -> i64 {
        let span = (hi_inclusive - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }

    fn ident_suffix(&mut self, len: usize) -> String {
        const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let mut out = String::with_capacity(len);
        for _ in 0..len {
            out.push(CHARS[(self.next_u64() as usize) % CHARS.len()] as char);
        }
        out
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn escape_pilot_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn assert_valid_json_via_python(json_text: &str, context: &str) {
    let mut child = Command::new("python3")
        .args(["-c", "import json, sys; obj = json.loads(sys.stdin.read()); assert obj is not None"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("python3 required for strict JSON validation");
    child.stdin.as_mut().unwrap().write_all(json_text.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "Invalid JSON in {}:\n{}\nPayload:\n{}",
        context,
        String::from_utf8_lossy(&out.stderr),
        json_text
    );
}

fn parse_content_length_frames(stream: &str) -> Vec<String> {
    let bytes = stream.as_bytes();
    let mut pos = 0usize;
    let mut out = Vec::new();
    while pos < bytes.len() {
        while pos < bytes.len() && (bytes[pos] == b'\r' || bytes[pos] == b'\n') {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        let rem = &stream[pos..];
        let hdr_end = rem
            .find("\r\n\r\n")
            .unwrap_or_else(|| panic!("Missing \\r\\n\\r\\n in LSP/DAP frame:\n{}", rem));
        let hdr = &rem[..hdr_end];
        let mut len_opt = None;
        for line in hdr.split("\r\n") {
            if let Some(rest) = line.strip_prefix("Content-Length:") {
                len_opt = Some(rest.trim().parse::<usize>().expect("Invalid Content-Length integer"));
            }
        }
        let body_len = len_opt.expect("Missing Content-Length header");
        let body_start = pos + hdr_end + 4;
        let body_end = body_start + body_len;
        assert!(
            body_end <= bytes.len(),
            "Truncated Content-Length frame: expected {} bytes, remaining {}",
            body_len,
            bytes.len() - body_start
        );
        out.push(stream[body_start..body_end].to_string());
        pos = body_end;
    }
    out
}

/// A document linked with the LSP handlers and analyzed once; each query
/// runs the handlers against its own copy of the analysis.
struct LspSession {
    arena: pilot_ast::AstArena,
    analysis: PhysicsRegistry,
    combined: String,
}

fn open_lsp_session(target_doc: &str, handlers_src: &str) -> LspSession {
    let combined = format!("{}\n{}", target_doc, handlers_src);
    let arena = parse(&combined);
    assert!(
        arena.diagnostics.is_empty(),
        "LSP combined parse diagnostics: {:?}",
        arena.diagnostics
    );
    let analysis = AnalysisPipeline::analyze(&arena, arena.root).physics;
    LspSession { arena, analysis, combined }
}

fn run_lsp_handler_query(session: &LspSession, method: &str, offset: usize) -> String {
    let mut registry = session.analysis.clone();
    registry.source_text = session.combined.clone();
    registry.lsp_method = method.to_string();
    registry.lsp_offset = offset as u32;

    let reg_ptr = &mut registry as *mut PhysicsRegistry;
    let mut vm = PilVirtualMachine::new();
    vm.engine_handler = Some(Box::new(move |name, args| {
        let reg = unsafe { &mut *reg_ptr };
        handle_engine_intrinsic(reg, name, args)
    }));
    let bin = BytecodeCompiler::compile(&session.arena, session.arena.root);
    vm.load_binary(bin);
    let _ = vm.run();
    registry.lsp_result
}

#[test]
pub fn test_59_1_vscode_gen_strict_json_no_fs_bypass_and_full_lsp_handlers_matrix() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_59_1_vscode_gen_strict_json_no_fs_bypass_and_full_lsp_handlers_matrix",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();

    assert!(
        !root.join("selfhost/ide_sync.pilot").exists(),
        "Monolithic selfhost/ide_sync.pilot must be deleted in M59"
    );

    // Run pure-Pilot `pilot gate` directly
    let pilot_bin = root.join("target/debug/pilot");
    if !pilot_bin.exists() {
        let build_st = Command::new("cargo")
            .args(["build", "-q", "-p", "pilot-cli"])
            .current_dir(&root)
            .status()
            .expect("cargo build -p pilot-cli");
        assert!(build_st.success(), "Failed building pilot-cli");
    }
    let gate_out = Command::new(&pilot_bin)
        .args(["gate", "--json"])
        .current_dir(&root)
        .output()
        .expect("pilot gate --json must execute");
    assert!(
        gate_out.status.success(),
        "pilot gate failed:\n{}{}",
        String::from_utf8_lossy(&gate_out.stdout),
        String::from_utf8_lossy(&gate_out.stderr)
    );

    let vscode_gen_src = std::fs::read_to_string(root.join("selfhost/ide/vscode_gen.pilot")).unwrap();
    let handlers_src = std::fs::read_to_string(root.join("selfhost/lsp/handlers.pilot")).unwrap();
    assert!(vscode_gen_src.lines().count() <= 1000);
    assert!(handlers_src.lines().count() <= 1000);

    // 1. Test `vscode_gen.pilot` with hard `fs_read_text` trap and runtime-randomized grammar tokens
    let dyn_ctrl = format!("custom_flow_{}", rng.ident_suffix(6));
    let dyn_mod = format!("custom_mod_{}", rng.ident_suffix(6));
    let dyn_ver = format!("3.{}.{}", rng.range_i64(10, 99), rng.range_i64(100, 999));

    let gen_script = format!(
        "{vscode_gen_src}\n\
         let tm = dumps_tm_language(\"{dyn_ctrl}\", \"struct|fn\", \"{dyn_mod}\", \"i64\", \"true|false\", 2);\n\
         let pkg = dumps_package_json(\"{dyn_ver}\", 2);\n\
         let lc = dumps_language_configuration(2);\n\
         let sn = dumps_snippets(2);\n\
         let __out = tm + \"\\n@@SPLIT@@\\n\" + pkg + \"\\n@@SPLIT@@\\n\" + lc + \"\\n@@SPLIT@@\\n\" + sn;\n\
         __out;\n"
    );

    let gen_arena = parse(&gen_script);
    assert!(gen_arena.diagnostics.is_empty(), "vscode_gen diagnostics: {:?}", gen_arena.diagnostics);
    let (flat, syms) = flatten_ast(&gen_arena, gen_arena.root);
    let mut reg = PhysicsRegistry::new(flat, syms);
    let reg_ptr = &mut reg as *mut PhysicsRegistry;
    let fs_read_trapped = Arc::new(AtomicBool::new(false));
    let trap_clone = Arc::clone(&fs_read_trapped);

    let mut vm = PilVirtualMachine::new();
    vm.engine_handler = Some(Box::new(move |name, args| {
        if name == "fs_read_text" {
            trap_clone.store(true, Ordering::SeqCst);
            return Err("FORBIDDEN: fs_read_text pass-through is banned in vscode_gen.pilot".into());
        }
        let r = unsafe { &mut *reg_ptr };
        handle_engine_intrinsic(r, name, args)
    }));
    vm.load_binary(BytecodeCompiler::compile(&gen_arena, gen_arena.root));
    let gen_out = vm.run_checked().unwrap_or_else(|f| panic!("VM fault: {f}")).to_string_repr();
    assert!(
        !fs_read_trapped.load(Ordering::SeqCst),
        "vscode_gen.pilot attempted forbidden fs_read_text pass-through!"
    );

    let artifacts: Vec<&str> = gen_out.split("\n@@SPLIT@@\n").collect();
    assert_eq!(artifacts.len(), 4, "Expected 4 VS Code JSON artifacts");
    assert_valid_json_via_python(artifacts[0], "pilot.tmLanguage.json");
    assert_valid_json_via_python(artifacts[1], "package.json");
    assert_valid_json_via_python(artifacts[2], "language-configuration.json");
    assert_valid_json_via_python(artifacts[3], "snippets/pilot.json");
    assert!(
        artifacts[0].contains(&dyn_ctrl) && artifacts[0].contains(&dyn_mod),
        "dumps_tm_language ignored dynamic keyword regex parameters!"
    );
    assert!(
        artifacts[1].contains(&dyn_ver),
        "dumps_package_json ignored dynamic version parameter!"
    );

    // 2. Test `handlers.pilot`: hover, pub/priv dot-completion, definition, decoy-safe references, rename guard
    let tag = rng.ident_suffix(6);
    let struct_nm = format!("TelemetrySensor_{}", tag);
    let pub_field = format!("reading_{}", tag);
    let priv_field = format!("secret_bias_{}", tag);
    let pub_method = format!("calibrate_{}", tag);
    let helper_fn = format!("amplify_{}", tag);
    let decoy_fn = format!("amplify_{}_decoy", tag);
    let main_fn = format!("evaluate_{}", tag);
    let doc_line = format!("Computes stabilized sensor output {}.", tag);

    let target_doc = format!(
        "pub struct {struct_nm} {{\n\
             pub {pub_field}: i64;\n\
             priv {priv_field}: i64;\n\
             pub fn {pub_method}() -> i64 {{ return 1; }}\n\
         }}\n\
         pub fn {decoy_fn}(z: i64) -> i64 {{ return z + 99; }}\n\
         pub fn {helper_fn}(v: i64) -> i64 {{\n\
             return v * 3;\n\
         }}\n\
         /// {doc_line}\n\
         pub fn {main_fn}(sensor: {struct_nm}) -> i64 {{\n\
             let a = {helper_fn}(sensor.{pub_field});\n\
             let b = {helper_fn}(a) + {decoy_fn}(a);\n\
             let c = sensor.{pub_field};\n\
             return {helper_fn}(b + c);\n\
         }}\n"
    );

    let lsp = open_lsp_session(&target_doc, &handlers_src);

    // 2a. Hover on `main_fn`
    let hover_off = target_doc.find(&format!("pub fn {}", main_fn)).unwrap() + 8;
    let hover_res = run_lsp_handler_query(&lsp, "textDocument/hover", hover_off);
    assert!(
        hover_res.contains(&main_fn) && hover_res.contains(&doc_line),
        "Hover must include function signature and doc comment, got:\n{}",
        hover_res
    );

    // 2b. Dot-completion on `sensor.` inside `main_fn`
    let dot_off = target_doc.find(&format!("let c = sensor.{}", pub_field)).unwrap() + "let c = sensor.".len();
    let comp_res = run_lsp_handler_query(&lsp, "textDocument/completion", dot_off);
    assert!(
        comp_res.contains(&pub_field) && comp_res.contains(&pub_method),
        "Dot-completion on struct receiver must include pub field '{}' and pub method '{}', got:\n{}",
        pub_field,
        pub_method,
        comp_res
    );
    assert!(
        !comp_res.contains(&priv_field),
        "VISIBILITY VIOLATION: Dot-completion leaked `priv` field '{}':\n{}",
        priv_field,
        comp_res
    );

    // 2c. Definition on second call to `helper_fn` must point to `pub fn helper_fn` declaration
    let decl_pos = target_doc.find(&format!("pub fn {}", helper_fn)).unwrap();
    let decl_ident_start = decl_pos + "pub fn ".len();
    let call_pos = target_doc.find(&format!("{}(b + c)", helper_fn)).unwrap() + 2;
    let def_res = run_lsp_handler_query(&lsp, "textDocument/definition", call_pos);
    assert!(
        def_res.contains(&decl_ident_start.to_string()) || def_res.contains(&decl_pos.to_string()),
        "Definition result must reference declaration span ({} / {}), got:\n{}",
        decl_pos,
        decl_ident_start,
        def_res
    );

    // 2d. References on `helper_fn` must find exact 4 occurrences (1 decl + 3 calls) and exclude `decoy_fn`
    let refs_res = run_lsp_handler_query(&lsp, "textDocument/references", call_pos);
    assert_valid_json_via_python(&refs_res, "textDocument/references");
    let decoy_pos = target_doc.find(&decoy_fn).unwrap();
    assert_eq!(
        refs_res.matches("\"start\"").count(),
        4,
        "Expected exactly 4 references (1 decl + 3 calls) for '{}', got:\n{}",
        helper_fn,
        refs_res
    );
    assert!(
        !refs_res.contains(&format!("\"start\":{}", decoy_pos)),
        "References falsely matched prefix decoy identifier '{}':\n{}",
        decoy_fn,
        refs_res
    );

    // 2e. prepareRename & rename guard on seed keywords vs user symbol
    let seed_fn_off = target_doc.find("fn ").unwrap() + 1;
    let prep_seed = run_lsp_handler_query(&lsp, "textDocument/prepareRename", seed_fn_off);
    assert_eq!(prep_seed, "null", "prepareRename must reject seed keyword 'fn'");
    let ren_seed = run_lsp_handler_query(&lsp, "textDocument/rename:struct", call_pos);
    assert_eq!(ren_seed, "null", "rename must reject renaming to seed keyword 'struct'");
    let prep_user = run_lsp_handler_query(&lsp, "textDocument/prepareRename", call_pos);
    assert!(
        prep_user.contains(&helper_fn) && prep_user != "null",
        "prepareRename must accept valid user identifier '{}', got: {}",
        helper_fn,
        prep_user
    );
    guard.finish();
}

#[test]
pub fn test_59_2_incremental_span_edits_adversarial_early_cutoff_and_persistent_lsp_stream() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_59_2_incremental_span_edits_adversarial_early_cutoff_and_persistent_lsp_stream",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();

    let ide_db_src = std::fs::read_to_string(root.join("selfhost/lsp/ide_db.pilot")).unwrap();
    let lsp_srv_src = std::fs::read_to_string(root.join("selfhost/lsp/server.pilot")).unwrap();
    assert!(ide_db_src.lines().count() <= 1500);
    assert!(lsp_srv_src.lines().count() <= 1500);

    // 1. Chain of 8 random byte-span edits (`apply_doc_span_edit`) verified against Rust `replace_range`
    let mut ref_doc = format!(
        "pub struct Point {{ pub x: i64; pub y: i64; }}\npub fn norm(p: Point) -> i64 {{ return p.x + {}; }}\n",
        rng.range_i64(10, 99)
    );
    let mut edit_stmts = format!("var cur_doc = \"{}\";\n", escape_pilot_str(&ref_doc));
    for step in 0..8 {
        let len = ref_doc.len();
        let a = (rng.next_u64() as usize) % (len - 4);
        let b = a + ((rng.next_u64() as usize) % (len - a).min(12));
        let repl_len = (rng.next_u64() as usize % 9) + 1;
        let repl = rng.ident_suffix(repl_len);
        ref_doc.replace_range(a..b, &repl);
        edit_stmts.push_str(&format!(
            "cur_doc = apply_doc_span_edit(cur_doc, {}, {}, \"{}\");\n",
            a,
            b,
            escape_pilot_str(&repl)
        ));
        let _ = step;
    }

    // 2. Six adversarial early-cutoff pairs (defeating naive `in_brace_depth == 0` cheats!)
    let k1 = rng.range_i64(10, 90);
    let k2 = rng.range_i64(100, 900);
    let fn_nm = format!("step_kernel_{}", rng.ident_suffix(5));
    let base_mod = format!(
        "pub struct Config {{\n    pub gain: i64;\n}}\n\
         pub interface IStage {{\n    fn run(v: i64) -> i64;\n}}\n\
         pub fn {fn_nm}(x: i64) -> i64 {{\n    let tag = \"{{brace}}\";\n    return x + {k1};\n}}\n"
    );
    // Pair 1: Edit strictly inside `fn` body -> MUST cutoff (`1`)
    let body_edit = base_mod.replace(&format!("return x + {};", k1), &format!("return (x * {}) - 3;", k2));
    // Pair 2: Unbalanced `}` inside string literal in `fn` body -> MUST cutoff (`1`)
    let str_brace_edit = base_mod.replace("\"{brace}\"", "\"}}}still_in_string{{{\"");
    // Pair 3: `fn` return type edit -> MUST NOT cutoff (`0`)
    let fn_ret_edit = base_mod.replace("-> i64 {", "-> f64 {");
    // Pair 4: `struct` field type edit INSIDE `{ ... }` -> MUST NOT cutoff (`0`)
    let struct_field_edit = base_mod.replace("pub gain: i64;", "pub gain: f64;");
    // Pair 5: `struct` field visibility edit INSIDE `{ ... }` -> MUST NOT cutoff (`0`)
    let struct_vis_edit = base_mod.replace("pub gain: i64;", "priv gain: i64;");
    // Pair 6: `interface` method signature edit INSIDE `{ ... }` -> MUST NOT cutoff (`0`)
    let iface_sig_edit = base_mod.replace("fn run(v: i64) -> i64;", "fn run(v: i64, scale: i64) -> i64;");

    // 3. Multi-message `Content-Length` persistent LSP stream (`initialize` -> `didOpen` -> `didChange` -> `hover` -> `shutdown`)
    let live_fn_v1 = format!("orbit_v1_{}", rng.ident_suffix(5));
    let live_fn_v2 = format!("orbit_v2_{}", rng.ident_suffix(5));
    let doc_v1 = format!("pub fn {}(mass: i64) -> i64 {{ return mass * 2; }}", live_fn_v1);
    let doc_v2 = format!("pub fn {}(mass: i64, dt: i64) -> i64 {{ return mass * dt; }}", live_fn_v2);
    let hover_byte = 8; // inside `orbit_v2_<rand>`

    let frame = |j: &str| format!("Content-Length: {}\r\n\r\n{}", j.len(), j);
    let lsp_stream = format!(
        "{}{}{}{}{}",
        frame("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}"),
        frame(&format!(
            "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{{\"textDocument\":{{\"uri\":\"file:///sim.pilot\",\"version\":1,\"text\":\"{}\"}}}}}}",
            escape_pilot_str(&doc_v1)
        )),
        frame(&format!(
            "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didChange\",\"params\":{{\"textDocument\":{{\"uri\":\"file:///sim.pilot\",\"version\":2}},\"contentChanges\":[{{\"text\":\"{}\"}}]}}}}",
            escape_pilot_str(&doc_v2)
        )),
        frame(&format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"textDocument/hover\",\"params\":{{\"textDocument\":{{\"uri\":\"file:///sim.pilot\"}},\"offset\":{}}}}}",
            hover_byte
        )),
        frame("{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"shutdown\",\"params\":null}")
    );

    let script = format!(
        "{ide_db_src}\n{lsp_srv_src}\n{edit_stmts}\n\
         let c1 = should_early_cutoff(\"{base}\", \"{e1}\");\n\
         let c2 = should_early_cutoff(\"{base}\", \"{e2}\");\n\
         let c3 = should_early_cutoff(\"{base}\", \"{e3}\");\n\
         let c4 = should_early_cutoff(\"{base}\", \"{e4}\");\n\
         let c5 = should_early_cutoff(\"{base}\", \"{e5}\");\n\
         let c6 = should_early_cutoff(\"{base}\", \"{e6}\");\n\
         let sess = lsp_session_new();\n\
         let responses = lsp_session_feed_stream(sess, \"{stream_esc}\");\n\
         let cutoffs = \"\" + c1 + \",\" + c2 + \",\" + c3 + \",\" + c4 + \",\" + c5 + \",\" + c6;\n\
         let __out = cur_doc + \"@@LSP@@\" + cutoffs + \"@@LSP@@\" + responses;\n\
         __out;\n",
        base = escape_pilot_str(&base_mod),
        e1 = escape_pilot_str(&body_edit),
        e2 = escape_pilot_str(&str_brace_edit),
        e3 = escape_pilot_str(&fn_ret_edit),
        e4 = escape_pilot_str(&struct_field_edit),
        e5 = escape_pilot_str(&struct_vis_edit),
        e6 = escape_pilot_str(&iface_sig_edit),
        stream_esc = escape_pilot_str(&lsp_stream),
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "ide_db/server parse diagnostics: {:?}", arena.diagnostics);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    let out = vm.run_checked().unwrap_or_else(|f| panic!("VM fault: {f}")).to_string_repr();
    let segs: Vec<&str> = out.split("@@LSP@@").collect();
    assert_eq!(segs.len(), 3, "Unexpected ide_db/lsp output: {}", out);

    assert_eq!(segs[0], ref_doc, "8-step incremental span edit diverged from reference document");
    assert_eq!(
        segs[1], "1,1,0,0,0,0",
        "Early-cutoff matrix failed (expected 1,1,0,0,0,0 for [fn_body, str_brace, fn_ret, struct_field, struct_vis, iface_sig], got {})",
        segs[1]
    );

    let frames = parse_content_length_frames(segs[2]);
    assert_eq!(frames.len(), 3, "Expected 3 JSON-RPC responses (id 1, 2, 3), got {:?}", frames);
    for (idx, f) in frames.iter().enumerate() {
        assert_valid_json_via_python(f, &format!("LSP response frame {}", idx));
    }
    assert!(frames[0].contains("\"hoverProvider\":true"));
    assert!(
        frames[1].contains(&live_fn_v2) && !frames[1].contains(&live_fn_v1) && !frames[1].contains("resident_pilvm_lsp"),
        "LSP hover response did not reflect live didChange symbol '{}':\n{}",
        live_fn_v2,
        frames[1]
    );
    guard.finish();
}

#[test]
pub fn test_59_3_and_59_4_live_vmhook_dap_stepping_watch_eval_inspector_and_5_crate_purge() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_59_3_and_59_4_live_vmhook_dap_stepping_watch_eval_inspector_and_5_crate_purge",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();

    // 1. Verify all 5 retired Rust crates are deleted from disk & workspace Cargo.toml
    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    for retired in [
        "pilot-dap",
        "pilot-lsp",
        "pilot-lsp-macros",
        "pilot-db",
        "pilot-incremental",
    ] {
        assert!(
            !root.join("crates").join(retired).exists(),
            "Retired crate crates/{} must be deleted from disk",
            retired
        );
        assert!(
            !cargo_toml.contains(&format!("\"crates/{}\"", retired)),
            "Workspace Cargo.toml must not list retired crate {}",
            retired
        );
    }

    let dap_server = std::fs::read_to_string(root.join("selfhost/dap/server.pilot")).unwrap();
    let vm_ctrl = std::fs::read_to_string(root.join("selfhost/dap/vm_controller.pilot")).unwrap();
    let inspector = std::fs::read_to_string(root.join("selfhost/dap/inspector.pilot")).unwrap();
    let repl_eval = std::fs::read_to_string(root.join("selfhost/dap/repl_eval.pilot")).unwrap();
    let cli_dbg = std::fs::read_to_string(root.join("selfhost/dap/cli_dbg.pilot")).unwrap();

    for (name, src) in [
        ("server.pilot", &dap_server),
        ("vm_controller.pilot", &vm_ctrl),
        ("inspector.pilot", &inspector),
        ("repl_eval.pilot", &repl_eval),
        ("cli_dbg.pilot", &cli_dbg),
    ] {
        assert!(src.lines().count() <= 1500, "selfhost/dap/{} exceeds 1,500 LOC", name);
        for banned in ["Apples", "production_cluster_db", "7fff2000"] {
            assert!(
                !src.contains(banned),
                "selfhost/dap/{} contains banned legacy mock string '{}'",
                name,
                banned
            );
        }
    }

    // 2. Compile a real multi-function Pilot debuggee with randomized line padding & constants!
    let pad_top = rng.range_i64(2, 6) as usize;
    let pad_mid = rng.range_i64(1, 5) as usize;
    let helper_nm = format!("callee_{}", rng.ident_suffix(5));
    let entry_nm = format!("driver_{}", rng.ident_suffix(5));
    let seed_x = rng.range_i64(7, 25);
    let mul_k = rng.range_i64(3, 8);
    let add_c = rng.range_i64(11, 49);

    let mut debuggee = String::new();
    for i in 0..pad_top {
        debuggee.push_str(&format!("// header pad {}\n", i));
    }
    debuggee.push_str(&format!("pub fn {helper_nm}(n: i64) -> i64 {{\n"));
    let helper_line_1 = debuggee.lines().count() as i64 + 1;
    debuggee.push_str(&format!("    let t1 = n * {mul_k};\n"));
    debuggee.push_str(&format!("    let t2 = t1 + {add_c};\n"));
    debuggee.push_str("    return t2;\n");
    debuggee.push_str("}\n");
    for i in 0..pad_mid {
        debuggee.push_str(&format!("// mid pad {}\n", i));
    }
    debuggee.push_str(&format!("pub fn {entry_nm}(seed_in: i64) -> i64 {{\n"));
    debuggee.push_str("    let local_x = seed_in + 4;\n");
    let bp_line = debuggee.lines().count() as i64 + 1;
    debuggee.push_str(&format!("    let first_call = {helper_nm}(local_x);\n"));
    let second_call_line = debuggee.lines().count() as i64 + 1;
    debuggee.push_str(&format!("    let second_call = {helper_nm}(first_call);\n"));
    let after_second_call_line = debuggee.lines().count() as i64 + 1;
    debuggee.push_str("    let final_sum = first_call + second_call;\n");
    debuggee.push_str("    return final_sum;\n");
    debuggee.push_str("}\n");
    debuggee.push_str(&format!("let __dbg_res = {entry_nm}({seed_x});\n__dbg_res;\n"));

    let dbg_arena = parse(&debuggee);
    assert!(dbg_arena.diagnostics.is_empty(), "debuggee parse diagnostics: {:?}", dbg_arena.diagnostics);
    let line_index = LineIndex::new(debuggee.as_bytes());
    let dbg_bin = BytecodeCompiler::compile(&dbg_arena, dbg_arena.root);
    let pc_spans = dbg_bin.pc_spans.clone();
    let reg_symbols = dbg_bin.reg_symbols.clone();

    // Collect real instruction execution trace via `PilVirtualMachine::set_hook`:
    // `(line, call_depth, snapshot_of_local_x)`
    let mut local_x_reg: Option<u8> = None;
    for &(reg, sym_id, _) in &reg_symbols {
        if dbg_arena.interner.resolve(SymbolId(sym_id)) == "local_x" {
            local_x_reg = Some(reg);
        }
    }

    let trace: Arc<std::sync::Mutex<Vec<(i64, i64, i64)>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let trace_ref = Arc::clone(&trace);
    let mut cur_depth = 0i64;
    let mut target_vm = PilVirtualMachine::new();
    target_vm.set_hook(Some(Box::new(move |pc, ins, regs| {
        let line = pc_spans
            .get(pc)
            .filter(|s| s.start > 0 || s.end > 0)
            .map(|s| line_index.lookup(s.start).line as i64)
            .unwrap_or(0);
        let lx_val = local_x_reg
            .and_then(|r| regs[r as usize].as_int())
            .unwrap_or(-1);
        if line > 0 {
            trace_ref.lock().unwrap().push((line, cur_depth, lx_val));
        }
        match ins.opcode() {
            Opcode::Call => cur_depth += 1,
            Opcode::Ret => {
                if cur_depth > 0 {
                    cur_depth -= 1;
                }
            }
            _ => {}
        }
    })));
    target_vm.load_binary(dbg_bin);
    let expected_first = (seed_x + 4) * mul_k + add_c;
    let expected_second = expected_first * mul_k + add_c;
    assert_eq!(target_vm.run().as_int(), Some(expected_first + expected_second));

    let recorded = trace.lock().unwrap().clone();
    assert!(!recorded.is_empty(), "VmHook recorded zero spans");

    // 3. Drive `vm_controller`, `cli_dbg`, `inspector`, `repl_eval`, and `dap/server` on `PilVirtualMachine`
    // over the REAL recorded `VmHook` trace AND a real `PhysicsRegistry` struct layout!
    let struct_src = "\
        [Bits(16, 3)] struct U16W {}\n\
        [Bits(64, 2)] struct I64W {}\n\
        [Bits(32, 4)] struct F32W {}\n\
        pub struct FrameHeader { pub tag: U16W; pub pts: I64W; pub gain: F32W; }\n";
    let struct_arena = parse(struct_src);
    let struct_analysis = AnalysisPipeline::analyze(&struct_arena, struct_arena.root);
    let frame_layout_id = (0..struct_analysis.physics.layouts.len())
        .find(|&id| {
            matches!(
                struct_analysis.physics.layouts.get(LayoutId(id as u32)).map(|l| &l.geometry),
                Some(PhysicsGeometry::Aggregate { fields, .. }) if fields.len() == 3
            )
        })
        .expect("Expected 3-field FrameHeader layout in PhysicsRegistry");

    let watch_k = rng.range_i64(4, 12);
    let watch_bias = rng.range_i64(5, 30);
    let watch_expr = format!("(local_x * {}) + {}", watch_k, watch_bias);
    let expected_watch = ((seed_x + 4) * watch_k) + watch_bias;

    // Replay the real trace through `should_pause_at_pc` across:
    //   Phase A: `b <bp_line>` + `continue` -> MUST pause at `bp_line` (depth 1)
    //   Phase B: `step` (`StepIn`) -> MUST pause at `helper_line_1` (depth 2)
    //   Phase C: `finish` (`StepOut`) -> MUST return to depth 1 (`second_call_line`)
    //   Phase D: `next` (`StepOver`) -> MUST step OVER `callee` (skipping depth 2!) and pause at `after_second_call_line` (depth 1)
    let mut trace_pilot = String::from("var step_idx = 0;\n");
    trace_pilot.push_str(&format!(
        "let act_b = parse_cli_dbg_action(\"break {bp_line}\");\n\
         let arg_b = parse_cli_dbg_arg(\"break {bp_line}\");\n\
         let act_s = parse_cli_dbg_action(\"step\");\n\
         let act_o = parse_cli_dbg_action(\"finish\");\n\
         let act_n = parse_cli_dbg_action(\"next\");\n\
         let act_p = parse_cli_dbg_action(\"print {watch_expr}\");\n\
         let arg_p = parse_cli_dbg_arg(\"print {watch_expr}\");\n"
    ));

    // Simulate state machine across recorded `(line, depth, lx)` events in Pilot:
    // phase 0 = Continue to bp_line; phase 1 = StepIn; phase 2 = StepOut; phase 3 = StepOver; phase 4 = Done
    trace_pilot.push_str(&format!(
        "var phase = 0;\n\
         var last_line = -1;\n\
         var target_depth = 1;\n\
         var bp = {bp_line};\n\
         var paused_lines = \"\";\n\
         var paused_lx = 0;\n"
    ));
    for &(ln, dp, lx) in &recorded {
        trace_pilot.push_str(&format!(
            "if phase < 4 {{\n\
                 let mode = if phase == 0 {{ 0 }} else {{ if phase == 1 {{ 1 }} else {{ if phase == 2 {{ 3 }} else {{ 2 }} }} }};\n\
                 let cur_bp = if phase == 0 {{ bp }} else {{ -999 }};\n\
                 if should_pause_at_pc(mode, {ln}, last_line, cur_bp, {dp}, target_depth) == 1 {{\n\
                     if paused_lines != \"\" {{ paused_lines = paused_lines + \",\"; }}\n\
                     paused_lines = paused_lines + {ln};\n\
                     if phase == 0 {{ paused_lx = {lx}; }}\n\
                     last_line = {ln};\n\
                     target_depth = {dp};\n\
                     phase = phase + 1;\n\
                 }}\n\
             }}\n"
        ));
    }

    let dap_driver = format!(
        "{dap_server}\n{vm_ctrl}\n{inspector}\n{repl_eval}\n{cli_dbg}\n{trace_pilot}\n\
         let env = __engine(\"map_new\");\n\
         let _m = __engine(\"map_set\", env, \"local_x\", paused_lx);\n\
         let watch_val = eval_dap_watch_expr(env, arg_p);\n\
         let eval_json = format_dap_evaluate_response(7, \"\" + watch_val);\n\
         let st_frame = encode_dap_frame(build_dap_stack_trace_frame(8, \"{entry_nm}\", {bp_line}, 5));\n\
         let fields_json = inspect_struct_fields_json({frame_layout_id});\n\
         let cli_ok = act_b == 6 && arg_b == \"{bp_line}\" && act_s == 1 && act_o == 3 && act_n == 2 && act_p == 7;\n\
         let __out = (if cli_ok {{ \"CLI_OK\" }} else {{ \"CLI_BAD\" }}) + \"@@DAP@@\"\n\
             + paused_lines + \"@@DAP@@\" + watch_val + \"@@DAP@@\" + eval_json + \"@@DAP@@\" + st_frame + \"@@DAP@@\" + fields_json;\n\
         __out;\n"
    );

    let dap_arena = parse(&dap_driver);
    assert!(dap_arena.diagnostics.is_empty(), "DAP driver diagnostics: {:?}", dap_arena.diagnostics);
    let mut phys = struct_analysis.physics;
    let phys_ptr = &mut phys as *mut PhysicsRegistry;
    let mut dap_vm = PilVirtualMachine::new();
    dap_vm.engine_handler = Some(Box::new(move |name, args| {
        let r = unsafe { &mut *phys_ptr };
        handle_engine_intrinsic(r, name, args)
    }));
    dap_vm.load_binary(BytecodeCompiler::compile(&dap_arena, dap_arena.root));
    let dap_out = dap_vm.run_checked().unwrap_or_else(|f| panic!("VM fault: {f}")).to_string_repr();
    let cols: Vec<&str> = dap_out.split("@@DAP@@").collect();
    assert_eq!(cols.len(), 6, "Unexpected DAP output: {}", dap_out);

    assert_eq!(cols[0], "CLI_OK", "cli_dbg command/argument parser failed");
    let expected_pauses = format!(
        "{},{},{},{}",
        bp_line, helper_line_1, second_call_line, after_second_call_line
    );
    assert_eq!(
        cols[1], expected_pauses,
        "VmHook stepping sequence [Bp -> StepIn -> StepOut -> StepOver] mismatch!"
    );
    assert_eq!(
        cols[2],
        expected_watch.to_string(),
        "DAP REPL live watch expression evaluation mismatch"
    );
    assert_valid_json_via_python(cols[3], "DAP evaluate response");
    let st_frames = parse_content_length_frames(cols[4]);
    assert_eq!(st_frames.len(), 1);
    assert_valid_json_via_python(&st_frames[0], "DAP stackTrace frame");
    assert!(
        st_frames[0].contains(&entry_nm) && st_frames[0].contains(&format!("\"line\":{}", bp_line)),
        "DAP stackTrace must report live function '{}' and line {}, got:\n{}",
        entry_nm,
        bp_line,
        st_frames[0]
    );
    assert_valid_json_via_python(cols[5], "inspector struct fields JSON");
    assert!(
        cols[5].contains("\"offset\":0") && cols[5].contains("\"offset\":8") && cols[5].contains("\"offset\":16"),
        "Name-Blind PhysicsRegistry struct field offset inspection expected offsets 0, 8, 16, got:\n{}",
        cols[5]
    );
    guard.finish();
}
