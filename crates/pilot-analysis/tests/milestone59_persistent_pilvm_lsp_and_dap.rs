//! Milestone 59 Rigorous Uncheatable Verification Suite:
//! Persistent `PilVM` LSP & `VmHook`-Stepped DAP.
//! What runs: `test_59_2` runs contracts 3 and 4 below. Contracts 1, 2 and 5
//! are checked only by the file audit (`ANTICHEAT_SPEC`), not run by any test.
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
//!    - Uses `nameof(...)` for all seed keyword comparisons.
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
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const DAP_BANNED_MOCKS: &[&str] = &["Apples", "production_cluster_db", "7fff2000"];

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 59,
    test_file_rel: "crates/pilot-analysis/tests/milestone59_persistent_pilvm_lsp_and_dap.rs",
    expected_test_count: 1,
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
        .args([
            "-c",
            "import json, sys; obj = json.loads(sys.stdin.read()); assert obj is not None",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("python3 required for strict JSON validation");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(json_text.as_bytes())
        .unwrap();
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
                len_opt = Some(
                    rest.trim()
                        .parse::<usize>()
                        .expect("Invalid Content-Length integer"),
                );
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
    let text_src = std::fs::read_to_string(root.join("stdlib/std/text.pilot")).unwrap();
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
    let body_edit = base_mod.replace(
        &format!("return x + {};", k1),
        &format!("return (x * {}) - 3;", k2),
    );
    // Pair 2: Unbalanced `}` inside string literal in `fn` body -> MUST cutoff (`1`)
    let str_brace_edit = base_mod.replace("\"{brace}\"", "\"}}}still_in_string{{{\"");
    // Pair 3: `fn` return type edit -> MUST NOT cutoff (`0`)
    let fn_ret_edit = base_mod.replace("-> i64 {", "-> f64 {");
    // Pair 4: `struct` field type edit INSIDE `{ ... }` -> MUST NOT cutoff (`0`)
    let struct_field_edit = base_mod.replace("pub gain: i64;", "pub gain: f64;");
    // Pair 5: `struct` field visibility edit INSIDE `{ ... }` -> MUST NOT cutoff (`0`)
    let struct_vis_edit = base_mod.replace("pub gain: i64;", "priv gain: i64;");
    // Pair 6: `interface` method signature edit INSIDE `{ ... }` -> MUST NOT cutoff (`0`)
    let iface_sig_edit = base_mod.replace(
        "fn run(v: i64) -> i64;",
        "fn run(v: i64, scale: i64) -> i64;",
    );

    // 3. Multi-message `Content-Length` persistent LSP stream (`initialize` -> `didOpen` -> `didChange` -> `hover` -> `shutdown`)
    let live_fn_v1 = format!("orbit_v1_{}", rng.ident_suffix(5));
    let live_fn_v2 = format!("orbit_v2_{}", rng.ident_suffix(5));
    let doc_v1 = format!(
        "pub fn {}(mass: i64) -> i64 {{ return mass * 2; }}",
        live_fn_v1
    );
    let doc_v2 = format!(
        "pub fn {}(mass: i64, dt: i64) -> i64 {{ return mass * dt; }}",
        live_fn_v2
    );
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
        "{text_src}\n{ide_db_src}\n{lsp_srv_src}\n{edit_stmts}\n\
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
    assert!(
        arena.diagnostics.is_empty(),
        "ide_db/server parse diagnostics: {:?}",
        arena.diagnostics
    );
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    let out = vm
        .run_checked()
        .unwrap_or_else(|f| panic!("VM fault: {f}"))
        .to_string_repr();
    let segs: Vec<&str> = out.split("@@LSP@@").collect();
    assert_eq!(segs.len(), 3, "Unexpected ide_db/lsp output: {}", out);

    assert_eq!(
        segs[0], ref_doc,
        "8-step incremental span edit diverged from reference document"
    );
    assert_eq!(
        segs[1], "1,1,0,0,0,0",
        "Early-cutoff matrix failed (expected 1,1,0,0,0,0 for [fn_body, str_brace, fn_ret, struct_field, struct_vis, iface_sig], got {})",
        segs[1]
    );

    let frames = parse_content_length_frames(segs[2]);
    assert_eq!(
        frames.len(),
        3,
        "Expected 3 JSON-RPC responses (id 1, 2, 3), got {:?}",
        frames
    );
    for (idx, f) in frames.iter().enumerate() {
        assert_valid_json_via_python(f, &format!("LSP response frame {}", idx));
    }
    assert!(frames[0].contains("\"hoverProvider\":true"));
    assert!(
        frames[1].contains(&live_fn_v2)
            && !frames[1].contains(&live_fn_v1)
            && !frames[1].contains("resident_pilvm_lsp"),
        "LSP hover response did not reflect live didChange symbol '{}':\n{}",
        live_fn_v2,
        frames[1]
    );
    guard.finish();
}
