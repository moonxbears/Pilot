//! Milestone 61 Rigorous Uncheatable Verification Suite:
//! Stage-3 Self-Hosted Fixed-Point (`pilotc2 == pilotc3`) & Scorched-Earth
//! Universal Rename Gate.
//!
//! ============================================================================
//! FROZEN-RUST RUNNER CONTRACT (DO NOT MODIFY THIS TEST FILE OR RUST CRATES)
//! ============================================================================
//! All tests in this file must pass simultaneously using ONLY edits to `.pilot`
//! files (`selfhost/{lexer,ast,parser,pilotc}.pilot`, `selfhost/codegen/*.pilot`,
//! `selfhost/lsp/*.pilot`, `selfhost/dap/*.pilot`, `stdlib/crypto/hash.pilot`).
//!
//! Required `.pilot` entrypoints & behavioral contracts:
//!
//! 1. Standalone Self-Hosted Frontend (`selfhost/{lexer,ast,parser}.pilot`, `<= 1,500` LOC each):
//!    - Executes in pure Pilot WITHOUT relying on mounted Rust `AstArena` (`ast_*` hooks are
//!      unmounted during `61.1` tests to prove zero post-Stage-0 dependence on `pilot-lexer`/`pilot-parser`!).
//!    - `selfhost/lexer.pilot`:
//!      - `pub fn sh_tokenize(src: str) -> i64`
//!        Scans full Pilot `TokenTape` (14 seed keywords vs user `Ident`, integer/string/`f"..."` literals,
//!        skipping `//` & `/* ... */` comments so brackets inside comments/strings NEVER corrupt bracket links,
//!        `InfixDot` vs `PrefixDot`, all 6 zero-whitespace `CallQuote` modes vs spaced `! (`, and O(1)
//!        bidirectional `matching_bracket` links across `()`, `[]`, `{}`).
//!      - `pub fn sh_token_count(tape: i64) -> i64`
//!      - `pub fn sh_token_text(tape: i64, idx: i64) -> str`
//!      - `pub fn sh_token_tag(tape: i64, idx: i64) -> str`
//!        Canonical token tag string: `"SeedKw"`, `"Ident"`, `"IntLit"`, `"StrLit"`, `"FStrLit"`,
//!        `"InfixDot"`, `"PrefixDot"`, `"CallQuoteShallow"`, `"CallQuoteDeep"`, `"CallQuoteAbi"`,
//!        `"CallQuoteSignal"`, `"CallQuoteAdjoint"`, `"CallQuoteEffect"`, `"LParen"`, `"RParen"`,
//!        `"LBracket"`, `"RBracket"`, `"LBrace"`, `"RBrace"`, `"Op"`, `"Punct"`, `"Eof"`.
//!      - `pub fn sh_token_matching_bracket(tape: i64, idx: i64) -> i64` (returns paired index, or `-1`)
//!    - `selfhost/ast.pilot` & `selfhost/parser.pilot`:
//!      - `pub fn sh_sizeof_ast_node() -> i64` (`== 16`)
//!      - `pub fn sh_parse_program(src: str) -> i64`
//!        Full standalone Pilot parser supporting `struct`, `interface`, `fn`, `let`, `var`, `if`/`else`,
//!        `while`, `loop`, `return`, `[Attr]`, dynamic Pratt expression precedence (`&&`, `||`, `==`, `!=`,
//!        `<`, `<=`, `>`, `>=`, `+`, `-`, `*`, `/`, `%`, unary `!` / `-`), AND homoiconic
//!        `syntax <name>(<params...>) => quote { ... $<param> ... };` AST splicing!
//!        (Bans fake `"expanded syntax ..."` comment-concatenation cheats.)
//!      - `pub fn sh_ast_root(arena: i64) -> i64`
//!      - `pub fn sh_ast_kind_name(arena: i64, node_id: i64) -> str`
//!        (`"Program"`, `"StructDecl"`, `"InterfaceDecl"`, `"FnDecl"`, `"LetStmt"`, `"VarStmt"`,
//!         `"IfExpr"`, `"WhileStmt"`, `"ReturnStmt"`, `"BlockExpr"`, `"BinaryExpr"`, `"UnaryExpr"`,
//!         `"CallExpr"`, `"IntLit"`, `"IdentExpr"`)
//!      - `pub fn sh_ast_op_text(arena: i64, node_id: i64) -> str` (e.g. `"&&"`, `"=="`, `"+"`, `"*"`, `"-"`, `"!"`)
//!      - `pub fn sh_ast_child_count(arena: i64, node_id: i64) -> i64`
//!      - `pub fn sh_ast_child_at(arena: i64, node_id: i64, idx: i64) -> i64`
//!
//! 2. Stage-3 Reproducible Bootstrap Fixed-Point (`selfhost/pilotc.pilot`, `<= 1,500` LOC):
//!    - `pub fn pilotc_compile_source_to_c11(pilot_src: str) -> str`
//!      Compiles raw Pilot source text end-to-end via `selfhost/{lexer,ast,parser}.pilot` into standalone
//!      compilable C11 source (compiled & executed via `clang -std=c11 -O2` in the test!).
//!    - `pub fn pilotc_bootstrap_stage_c11(stage: i64, compiler_unit_src: str) -> str`
//!      Runs bootstrap stage `1`, `2`, or `3` over `compiler_unit_src`, emitting standalone C11 code.
//!      Must actually compile `compiler_unit_src` (compiled & run via `clang` on two distinct randomized
//!      inputs!) and satisfy bit-for-bit fixed-point `stage2 == stage3` (`sha256(stage2) == sha256(stage3)`).
//!    - `pub fn pilotc_bootstrap_stage_pilc_hex(stage: i64, compiler_unit_src: str) -> str`
//!      Emits lowercase hex `.pilc` v3 binary (`PIL3` magic) for `compiler_unit_src` that loads cleanly
//!      via `PilcBinary::load_from_bytes`, executes on `PilVirtualMachine`, and satisfies `stage2 == stage3`.
//!
//! 3. Automated Contraction & Dual-Pass Universal Rename Gate (`61.3` & `61.4`):
//!    - Passes `gate1_check.py`, `gate2_check.py`, `gate3_check.py`, and `gate4_selfhost_contraction.py`.
//!    - Passes the **Dual-Pass Universal Rename Litmus Test** (rewriting `stdlib/core/*.pilot` both to
//!      `c32`/`window`/`maybe`/`outcome`/`interval` AND to runtime-PRNG names `c32_<tag>`, `window_<tag>`,
//!      `maybe_<tag>`, `outcome_<tag>`, `interval_<tag>`) across `PilVM`, C11 (`clang`), LLVM IR (`clang`),
//!      WASM (`node`), WGSL GPU, LSP (`textDocument/hover`), and DAP (`inspect_struct_fields_json`).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_analysis::{
    AnalysisPipeline, flatten_ast, handle_engine_intrinsic, LayoutId, PhysicsGeometry,
    PhysicsRegistry,
};
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_parser::{parse, parse_and_ingest, parse_with_tables, ParserTables};
use pilot_vm::{BytecodeCompiler, PilVirtualMachine, PilcBinary};

#[path = "common/anticheat.rs"]
mod anticheat;

const SH_AST_GROUP: &[&str] = &["selfhost/ast.pilot", "selfhost/parser.pilot"];

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 61,
    test_file_rel: "crates/pilot-analysis/tests/milestone61_stage3_fixed_point_and_universal_rename_gate.rs",
    expected_test_count: 3,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/lexer.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "sh_tokenize",
                "sh_token_count",
                "sh_token_text",
                "sh_token_tag",
                "sh_token_matching_bracket",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &["__engine(\"ast_node_kind\"", "__engine(\"ast_child_at\""],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/ast.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &["__engine(\"ast_node_kind\"", "__engine(\"ast_child_at\""],
            forbidden_string_substrings: &["expanded syntax "],
        },
        anticheat::FileAuditRule {
            path: "selfhost/parser.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["sh_parse_program"],
            required_code_markers: &[],
            forbidden_code_markers: &["__engine(\"ast_node_kind\"", "__engine(\"ast_child_at\""],
            forbidden_string_substrings: &["expanded syntax "],
        },
        anticheat::FileAuditRule {
            path: "selfhost/pilotc.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "pilotc_compile_source_to_c11",
                "pilotc_bootstrap_stage_c11",
                "pilotc_bootstrap_stage_pilc_hex",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &["system(", "host_spawn_eval"],
            forbidden_string_substrings: &["pilot build", "PILOTC_BINARY["],
        },
    ],
    required_group_pub_fns: &[
        ("sh_ast_root", SH_AST_GROUP),
        ("sh_ast_kind_name", SH_AST_GROUP),
        ("sh_ast_op_text", SH_AST_GROUP),
        ("sh_ast_child_count", SH_AST_GROUP),
        ("sh_ast_child_at", SH_AST_GROUP),
    ],
};

fn eval_with_mounted_physics(driver_src: &str, target_src: &str, physics: &PhysicsRegistry) -> String {
    let drv_arena = parse(driver_src);
    assert!(
        drv_arena.diagnostics.is_empty(),
        "Driver parse diagnostics: {:?}",
        drv_arena.diagnostics
    );
    let mut reg = physics.clone();
    reg.source_text = target_src.to_string();
    let reg_ptr = &mut reg as *mut PhysicsRegistry;
    let mut vm = PilVirtualMachine::new();
    vm.engine_handler = Some(Box::new(move |op, args| {
        handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, args)
    }));
    vm.load_binary(BytecodeCompiler::compile(&drv_arena, drv_arena.root));
    vm.run().to_string_repr()
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn from_entropy() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        let pid = std::process::id() as u64;
        Self {
            state: nanos ^ pid.rotate_left(29) ^ 0xBF58_476D_1CE4_E5B9,
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

    fn ident(&mut self, prefix: &str, len: usize) -> String {
        const CH: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let mut s = String::from(prefix);
        for _ in 0..len {
            s.push(CH[(self.next_u64() as usize) % CH.len()] as char);
        }
        s
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

fn decode_hex(hex_str: &str) -> Vec<u8> {
    let clean = hex_str.trim();
    assert!(clean.len() % 2 == 0, "Odd hex length: {}", clean.len());
    (0..clean.len() / 2)
        .map(|i| u8::from_str_radix(&clean[2 * i..2 * i + 2], 16).expect("Invalid hex byte"))
        .collect()
}

fn python_sha256_bytes(bytes: &[u8]) -> String {
    let mut child = Command::new("python3")
        .args(["-c", "import hashlib, sys; print(hashlib.sha256(sys.stdin.buffer.read()).hexdigest())"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(bytes).unwrap();
    String::from_utf8(child.wait_with_output().unwrap().stdout)
        .unwrap()
        .trim()
        .to_string()
}

fn compile_and_run_c11_i64(c_src: &str, call_expr_c: &str, tag: &str) -> i64 {
    static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let seq = TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("pilot_m61_{}_{}_{}", std::process::id(), seq, tag));
    let c_path = tmp.with_extension("c");
    let bin_path = tmp.with_extension("bin");
    let full_c = format!(
        "{}\n#include <stdio.h>\n#include <inttypes.h>\nint main(void) {{\n    int64_t r = (int64_t)({});\n    printf(\"%\" PRId64 \"\\n\", r);\n    return 0;\n}}\n",
        c_src, call_expr_c
    );
    std::fs::write(&c_path, &full_c).unwrap();
    let comp = Command::new("clang")
        .args(["-std=c11", "-O2", c_path.to_str().unwrap(), "-o", bin_path.to_str().unwrap()])
        .output()
        .expect("clang must be installed");
    assert!(
        comp.status.success(),
        "clang failed on {}:\n{}\nSource:\n{}",
        tag,
        String::from_utf8_lossy(&comp.stderr),
        full_c
    );
    let run = Command::new(&bin_path).output().unwrap();
    let _ = std::fs::remove_file(&c_path);
    let _ = std::fs::remove_file(&bin_path);
    assert!(run.status.success());
    String::from_utf8(run.stdout)
        .unwrap()
        .trim()
        .parse::<i64>()
        .expect("Expected integer stdout from compiled C11 binary")
}

/// Whole-identifier word-boundary renamer: replaces exact standalone identifier tokens
/// without corrupting compound identifiers like `slice_t`, `source_slice`, or `char_t`.
fn rename_identifiers_word_boundary(src: &str, mappings: &[(&str, &str)]) -> String {
    let bytes = src.as_bytes();
    let is_id = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut out = String::with_capacity(src.len() + 64);
    let mut i = 0usize;
    while i < bytes.len() {
        if is_id(bytes[i]) && (i == 0 || !is_id(bytes[i - 1])) {
            let mut j = i + 1;
            while j < bytes.len() && is_id(bytes[j]) {
                j += 1;
            }
            let word = &src[i..j];
            if let Some(&(_, rep)) = mappings.iter().find(|&&(orig, _)| orig == word) {
                out.push_str(rep);
            } else {
                out.push_str(word);
            }
            i = j;
        } else {
            let ch = src[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn collect_type_named_symbols(arena: &AstArena, id: NodeId, out: &mut Vec<String>) {
    if id.is_none() || (id.0 as usize) >= arena.nodes.len() {
        return;
    }
    for (idx, n) in arena.nodes.iter().enumerate() {
        if n.node_kind() == NodeKind::TypeNamed {
            out.push(arena.interner.resolve(SymbolId(n.lhs)).to_string());
        }
        let _ = idx;
    }
}

#[test]
pub fn test_61_1_standalone_selfhost_lexer_16b_ast_pratt_precedence_and_quote_splicing() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_61_1_standalone_selfhost_lexer_16b_ast_pratt_precedence_and_quote_splicing",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();

    let lexer_src = std::fs::read_to_string(root.join("selfhost/lexer.pilot")).unwrap();
    let ast_src = std::fs::read_to_string(root.join("selfhost/ast.pilot")).unwrap();
    let parser_src = std::fs::read_to_string(root.join("selfhost/parser.pilot")).unwrap();

    for (nm, s) in [
        ("lexer.pilot", &lexer_src),
        ("ast.pilot", &ast_src),
        ("parser.pilot", &parser_src),
    ] {
        assert!(s.lines().count() <= 1500, "selfhost/{} exceeds 1,500 LOC", nm);
        assert!(
            !s.contains("expanded syntax "),
            "selfhost/{} must perform real AST splicing, not fake 'expanded syntax' comments!",
            nm
        );
    }

    let user_id = rng.ident("sensor_", 5);
    let macro_unless = rng.ident("unless_", 5);
    let fn_pratt = rng.ident("calc_pratt_", 5);
    let c1 = rng.range_i64(11, 99);
    let c2 = rng.range_i64(101, 299);

    // 1. Adversarial lexical input: comments & strings containing fake `{ [ (` brackets,
    //    `InfixDot` vs `PrefixDot`, all 6 `CallQuote` sigils vs spaced `! (`, and nested brackets.
    let lex_sample = format!(
        "// decoy comment {{ [ (\n\
         /* block decoy ) ] }} */\n\
         struct {user_id} {{ let v = \"str ({{[\"; }}\n\
         fn run() {{ obj.val; .reset(); f!(1); f!!(2); f!&(3); f!^(4); f!@(5); f!?(6); f ! (7); }}\n"
    );

    // 2. Adversarial parser input: custom homoiconic `syntax` macro + multi-level Pratt expression
    //    `a + b * c == d - e && !flag` -> `&&` > (`==` > (`+` > `*`, `-`)) & `!`
    let parse_sample = format!(
        "syntax {macro_unless}(cond: ast, body: ast) => quote {{ if !($cond) {{ $body }} }};\n\
         pub fn {fn_pratt}(a: i64, b: i64, c: i64, d: i64, e: i64, flag: bool) -> i64 {{\n\
             {macro_unless}(a == {c1}, return {c2};);\n\
             if a + b * c == d - e && !flag {{\n\
                 return 1;\n\
             }}\n\
             return 0;\n\
         }}\n"
    );

    // Execute on a bare `PilVirtualMachine` with NO mounted Rust `PhysicsRegistry`!
    let script = format!(
        "{lexer_src}\n{ast_src}\n{parser_src}\n\
         let sz = sh_sizeof_ast_node();\n\
         let tape = sh_tokenize(\"{lex_esc}\");\n\
         let tcnt = sh_token_count(tape);\n\
         var seen_tags = \"\";\n\
         var bracket_bi_ok = true;\n\
         var ti = 0;\n\
         while ti < tcnt {{\n\
             let tg = sh_token_tag(tape, ti);\n\
             let tx = sh_token_text(tape, ti);\n\
             if tx == \"struct\" || tx == \"{user_id}\" || tg == \"InfixDot\" || tg == \"PrefixDot\" || tg == \"CallQuoteShallow\" || tg == \"CallQuoteDeep\" || tg == \"CallQuoteAbi\" || tg == \"CallQuoteSignal\" || tg == \"CallQuoteAdjoint\" || tg == \"CallQuoteEffect\" {{\n\
                 seen_tags = seen_tags + tx + \":\" + tg + \"|\";\n\
             }}\n\
             let mate = sh_token_matching_bracket(tape, ti);\n\
             if mate >= 0 {{\n\
                 if sh_token_matching_bracket(tape, mate) != ti {{\n\
                     bracket_bi_ok = false;\n\
                 }}\n\
             }}\n\
             ti = ti + 1;\n\
         }}\n\
         \n\
         let ast_h = sh_parse_program(\"{parse_esc}\");\n\
         let root = sh_ast_root(ast_h);\n\
         let root_k = sh_ast_kind_name(ast_h, root);\n\
         // Find `FnDecl` child in Program\n\
         var fn_nid = -1;\n\
         var ci = 0;\n\
         let r_ch = sh_ast_child_count(ast_h, root);\n\
         while ci < r_ch {{\n\
             let ch = sh_ast_child_at(ast_h, root, ci);\n\
             if sh_ast_kind_name(ast_h, ch) == \"FnDecl\" {{ fn_nid = ch; }}\n\
             ci = ci + 1;\n\
         }}\n\
         let body = sh_ast_child_at(ast_h, fn_nid, 0);\n\
         // Stmt 0 in body MUST be expanded `IfExpr` from `syntax {macro_unless}`!\n\
         let stmt0 = sh_ast_child_at(ast_h, body, 0);\n\
         let stmt0_k = sh_ast_kind_name(ast_h, stmt0);\n\
         let stmt0_cond = sh_ast_child_at(ast_h, stmt0, 0);\n\
         let stmt0_cond_op = sh_ast_op_text(ast_h, stmt0_cond);\n\
         // Stmt 1 in body is `if a + b * c == d - e && !flag`: verify full Pratt precedence tree!\n\
         let stmt1 = sh_ast_child_at(ast_h, body, 1);\n\
         let top_and = sh_ast_child_at(ast_h, stmt1, 0);\n\
         let eq_lhs = sh_ast_child_at(ast_h, top_and, 0);\n\
         let not_rhs = sh_ast_child_at(ast_h, top_and, 1);\n\
         let add_node = sh_ast_child_at(ast_h, eq_lhs, 0);\n\
         let sub_node = sh_ast_child_at(ast_h, eq_lhs, 1);\n\
         let mul_node = sh_ast_child_at(ast_h, add_node, 1);\n\
         let tree_sig = sh_ast_op_text(ast_h, top_and) + \">(\" + sh_ast_op_text(ast_h, eq_lhs)\n\
             + \">(\" + sh_ast_op_text(ast_h, add_node) + \">\" + sh_ast_op_text(ast_h, mul_node)\n\
             + \",\" + sh_ast_op_text(ast_h, sub_node) + \"),\" + sh_ast_op_text(ast_h, not_rhs) + \")\";\n\
         let __out = \"\" + sz + \"@@FE@@\" + (if bracket_bi_ok {{ \"BR_OK\" }} else {{ \"BR_BAD\" }}) + \"@@FE@@\"\n\
             + seen_tags + \"@@FE@@\" + root_k + \":\" + stmt0_k + \":\" + stmt0_cond_op + \"@@FE@@\" + tree_sig;\n\
         __out;\n",
        lex_esc = escape_pilot_str(&lex_sample),
        parse_esc = escape_pilot_str(&parse_sample),
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "selfhost frontend parse diagnostics: {:?}", arena.diagnostics);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    let out = vm.run().to_string_repr();
    let cols: Vec<&str> = out.split("@@FE@@").collect();
    assert_eq!(cols.len(), 5, "Unexpected selfhost frontend output: {}", out);

    assert_eq!(cols[0], "16", "Self-hosted AST node must be 16 bytes");
    assert_eq!(cols[1], "BR_OK", "O(1) bidirectional bracket matching corrupted by comments/strings");
    for req in [
        "struct:SeedKw",
        &format!("{}:Ident", user_id),
        "InfixDot",
        "PrefixDot",
        "CallQuoteShallow",
        "CallQuoteDeep",
        "CallQuoteAbi",
        "CallQuoteSignal",
        "CallQuoteAdjoint",
        "CallQuoteEffect",
    ] {
        assert!(
            cols[2].contains(req),
            "Self-hosted lexer missing expected token classification '{}' in: {}",
            req,
            cols[2]
        );
    }
    assert_eq!(
        cols[3], "Program:IfExpr:!",
        "Homoiconic `syntax` quote splicing failed to expand macro into `IfExpr(!cond)`"
    );
    assert_eq!(
        cols[4], "&&>(==>(+>*,-),!)",
        "Pratt precedence hierarchy mismatch"
    );
    guard.finish();
}

#[test]
pub fn test_61_2_stage3_bootstrap_fixed_point_clang_native_and_pilc_sha256_parity() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_61_2_stage3_bootstrap_fixed_point_clang_native_and_pilc_sha256_parity",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();

    let hash_src = std::fs::read_to_string(root.join("stdlib/crypto/hash.pilot")).unwrap();
    let lexer_src = std::fs::read_to_string(root.join("selfhost/lexer.pilot")).unwrap();
    let ast_src = std::fs::read_to_string(root.join("selfhost/ast.pilot")).unwrap();
    let parser_src = std::fs::read_to_string(root.join("selfhost/parser.pilot")).unwrap();
    let pilotc_src = std::fs::read_to_string(root.join("selfhost/pilotc.pilot")).unwrap();
    assert!(pilotc_src.lines().count() <= 1500);
    assert!(
        !pilotc_src.contains("PILOTC_BINARY["),
        "pilotc.pilot must actually compile Pilot source, not return static 'PILOTC_BINARY[...]' strings!"
    );

    let fn_entry = rng.ident("stage_kernel_", 6);
    let k_a = rng.range_i64(13, 47);
    let k_b = rng.range_i64(51, 97);
    let bias_a = rng.range_i64(100, 400);
    let bias_b = rng.range_i64(500, 900);
    let input_n = rng.range_i64(6, 14);

    let unit_a = format!(
        "pub fn {fn_entry}(n: i64) -> i64 {{\n\
             var acc = {bias_a};\n\
             var i = 1;\n\
             while i <= n {{\n\
                 acc = acc + (i * {k_a});\n\
                 i = i + 1;\n\
             }}\n\
             return acc;\n\
         }}\n\
         {fn_entry}({input_n});\n"
    );
    let unit_b = format!(
        "pub fn {fn_entry}(n: i64) -> i64 {{\n\
             var acc = {bias_b};\n\
             var i = 1;\n\
             while i <= n {{\n\
                 acc = acc + (i * {k_b});\n\
                 i = i + 1;\n\
             }}\n\
             return acc;\n\
         }}\n\
         {fn_entry}({input_n});\n"
    );

    let sum_1_to_n = input_n * (input_n + 1) / 2;
    let expected_a = bias_a + sum_1_to_n * k_a;
    let expected_b = bias_b + sum_1_to_n * k_b;

    let script = format!(
        "{hash_src}\n{lexer_src}\n{ast_src}\n{parser_src}\n{pilotc_src}\n\
         let direct_c = pilotc_compile_source_to_c11(\"{ua_esc}\");\n\
         let s1_c_a = pilotc_bootstrap_stage_c11(1, \"{ua_esc}\");\n\
         let s2_c_a = pilotc_bootstrap_stage_c11(2, \"{ua_esc}\");\n\
         let s3_c_a = pilotc_bootstrap_stage_c11(3, \"{ua_esc}\");\n\
         let s2_c_b = pilotc_bootstrap_stage_c11(2, \"{ub_esc}\");\n\
         let s2_pilc = pilotc_bootstrap_stage_pilc_hex(2, \"{ua_esc}\");\n\
         let s3_pilc = pilotc_bootstrap_stage_pilc_hex(3, \"{ua_esc}\");\n\
         let sha_s2 = sha256_hex(s2_c_a);\n\
         let sha_s3 = sha256_hex(s3_c_a);\n\
         let __out = direct_c + \"@@BS@@\" + s1_c_a + \"@@BS@@\" + s2_c_a + \"@@BS@@\" + s3_c_a\n\
             + \"@@BS@@\" + s2_c_b + \"@@BS@@\" + s2_pilc + \"@@BS@@\" + s3_pilc + \"@@BS@@\" + sha_s2 + \"|\" + sha_s3;\n\
         __out;\n",
        ua_esc = escape_pilot_str(&unit_a),
        ub_esc = escape_pilot_str(&unit_b),
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "pilotc bootstrap parse diagnostics: {:?}", arena.diagnostics);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    let out = vm.run().to_string_repr();
    let cols: Vec<&str> = out.split("@@BS@@").collect();
    assert_eq!(cols.len(), 8, "Unexpected bootstrap output");

    let (direct_c, _s1_c_a, s2_c_a, s3_c_a, s2_c_b, s2_pilc_hex, s3_pilc_hex, shas) = (
        cols[0], cols[1], cols[2], cols[3], cols[4], cols[5], cols[6], cols[7],
    );

    // 1. Stage-2 and Stage-3 MUST be bit-for-bit identical (C11 AND `.pilc` bytecode!)
    assert_eq!(s2_c_a, s3_c_a, "Stage-2 and Stage-3 C11 emissions diverged (not a fixed point!)");
    assert_eq!(s2_pilc_hex, s3_pilc_hex, "Stage-2 and Stage-3 .pilc binaries diverged!");
    assert_ne!(s2_c_a, s2_c_b, "Stage-2 compiler ignored input program differences!");

    let (sha2, sha3) = shas.split_once('|').unwrap();
    let py_sha = python_sha256_bytes(s3_c_a.as_bytes());
    assert_eq!(sha2, sha3);
    assert_eq!(sha3, py_sha, "Stage-3 SHA-256 mismatch vs python3 hashlib");

    // 2. Compile `direct_c`, `s3_c_a`, and `s2_c_b` with `clang` and execute native binaries!
    let call_expr = format!("{}({})", fn_entry, input_n);
    assert_eq!(compile_and_run_c11_i64(direct_c, &call_expr, "direct"), expected_a);
    assert_eq!(compile_and_run_c11_i64(s3_c_a, &call_expr, "stage3_a"), expected_a);
    assert_eq!(compile_and_run_c11_i64(s2_c_b, &call_expr, "stage2_b"), expected_b);

    // 3. Deserialize Stage-3 `.pilc` binary via `PilcBinary::load_from_bytes` and execute on fresh `PilVirtualMachine`!
    let pilc_bytes = decode_hex(s3_pilc_hex);
    assert!(pilc_bytes.starts_with(b"PIL3"), "Stage-3 .pilc must start with PIL3 magic");
    let loaded = PilcBinary::load_from_bytes(&pilc_bytes).expect("Stage-3 .pilc failed deserialization");
    let mut fresh_vm = PilVirtualMachine::new();
    fresh_vm.load_binary(loaded);
    assert_eq!(
        fresh_vm.run().as_int(),
        Some(expected_a),
        "Stage-3 deserialized .pilc executed to wrong value on PilVirtualMachine"
    );
    guard.finish();
}

#[test]
pub fn test_61_3_and_61_4_all_four_gates_and_dual_pass_universal_rename_litmus_across_7_backends() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_61_3_and_61_4_all_four_gates_and_dual_pass_universal_rename_litmus_across_7_backends",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();

    // 1. Run ALL 4 workspace verification gates via pure-Pilot `pilot gate --full`
    let pilot_bin = root.join("target/debug/pilot");
    if !pilot_bin.exists() {
        let build_st = Command::new("cargo")
            .args(["build", "-q", "-p", "pilot-cli"])
            .current_dir(&root)
            .status()
            .expect("cargo build -p pilot-cli");
        assert!(build_st.success(), "Failed building pilot-cli");
    }
    let out = Command::new(&pilot_bin)
        .args(["gate", "--full"])
        .current_dir(&root)
        .output()
        .expect("Failed to spawn pilot gate --full");
    assert!(
        out.status.success(),
        "pilot gate --full failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // 2. Scan all surviving Rust `crates/*/src/**/*.rs` for zero hardcoded Pilot scalar/composite type strings
    let banned_rust_type_literals = [
        "\"i64\"", "\"u8\"", "\"f64\"", "\"char\"", "\"bool\"", "\"usize\"",
        "\"stride\"", "\"option\"", "\"result\"", "\"range\"",
        "\"SliceBuffer\"", "\"RangeWindow\"", "\"Dual\"", "\"Signal\"", "\"EffectPlan\"",
    ];
    let mut total_src_loc = 0usize;
    for crate_entry in std::fs::read_dir(root.join("crates")).unwrap() {
        let src_dir = crate_entry.unwrap().path().join("src");
        if !src_dir.exists() {
            continue;
        }
        let mut stack = vec![src_dir];
        while let Some(dir) = stack.pop() {
            for ent in std::fs::read_dir(dir).unwrap() {
                let p = ent.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                    let txt = std::fs::read_to_string(&p).unwrap();
                    total_src_loc += txt.lines().count();
                    assert!(!txt.contains("allow(dead_code)"), "#[allow(dead_code)] banned in {:?}", p);
                    for lit in banned_rust_type_literals {
                        assert!(
                            !txt.contains(lit),
                            "Universal Name-Blind Violation: hardcoded type literal {} in {:?}",
                            lit,
                            p
                        );
                    }
                }
            }
        }
    }
    let total_cap = anticheat::seed_rust_crate_loc_cap(&root, 5900);
    assert!(total_src_loc <= total_cap, "Total Rust src LOC ({}) exceeds {}", total_src_loc, total_cap);

    // 3. Dual-Pass Universal Rename Litmus Test:
    //    Pass 0: Canonical AGENTS.md litmus names (`c32`, `window`, `maybe`, `outcome`, `interval`)
    //    Pass 1: Runtime-PRNG unpredictable names (`c32_<tag>`, `window_<tag>`, `maybe_<tag>`, `outcome_<tag>`, `interval_<tag>`)
    //            (Defeats any AI that tries to hardcode `"window"` or `"maybe"` in `selfhost/*.pilot`!)
    let bootstrap_rel_paths = [
        "stdlib/core/core.pilot",
        "stdlib/core/syntax/type_construct.pilot",
        "stdlib/core/syntax/visibility.pilot",
        "stdlib/core/syntax/lifecycle.pilot",
        "stdlib/core/syntax/properties.pilot",
        "stdlib/core/syntax/module_ext.pilot",
        "stdlib/core/syntax/attribute.pilot",
        "stdlib/core/syntax/for.pilot",
        "stdlib/core/syntax/while.pilot",
        "stdlib/core/syntax/until.pilot",
        "stdlib/core/syntax/switch.pilot",
        "stdlib/core/syntax/guard.pilot",
        "stdlib/core/syntax/try_catch.pilot",
        "stdlib/core/syntax/expr_sugar.pilot",
        "stdlib/core/syntax/fstring.pilot",
        "stdlib/core/syntax/unit_decl.pilot",
        "stdlib/db/sql_syntax.pilot",
        "stdlib/polyglot/polyglot_syntax.pilot",
        "stdlib/realtime/rt_syntax.pilot",
        "stdlib/asyncio/select_syntax.pilot",
        "stdlib/contracts/contracts_syntax.pilot",
    ];

    let c_phys = std::fs::read_to_string(root.join("selfhost/codegen/physics.pilot")).unwrap();
    let c_types = std::fs::read_to_string(root.join("selfhost/codegen/c_types.pilot")).unwrap();
    let c_hdr = std::fs::read_to_string(root.join("selfhost/codegen/c_header.pilot")).unwrap();
    let c_emit = std::fs::read_to_string(root.join("selfhost/codegen/c_emitter.pilot")).unwrap();
    let llvm_emit = std::fs::read_to_string(root.join("selfhost/codegen/llvm_emitter.pilot")).unwrap();
    let wasm_emit = std::fs::read_to_string(root.join("selfhost/codegen/wasm_emitter.pilot")).unwrap();
    let gpu_c = std::fs::read_to_string(root.join("selfhost/codegen/gpu_c_dialects.pilot")).unwrap();
    let gpu_sh = std::fs::read_to_string(root.join("selfhost/codegen/gpu_shaders.pilot")).unwrap();
    let gpu_ll = std::fs::read_to_string(root.join("selfhost/codegen/gpu_llvm.pilot")).unwrap();
    let lsp_handlers = std::fs::read_to_string(root.join("selfhost/lsp/handlers.pilot")).unwrap();
    let dap_inspector = std::fs::read_to_string(root.join("selfhost/dap/inspector.pilot")).unwrap();

    let tag = rng.ident("", 5);
    let dyn_c32 = format!("c32_{}", tag);
    let dyn_win = format!("window_{}", tag);
    let dyn_may = format!("maybe_{}", tag);
    let dyn_out = format!("outcome_{}", tag);
    let dyn_int = format!("interval_{}", tag);

    let passes: [[(&str, &str); 5]; 2] = [
        [
            ("char", "c32"),
            ("slice", "window"),
            ("option", "maybe"),
            ("result", "outcome"),
            ("range", "interval"),
        ],
        [
            ("char", dyn_c32.as_str()),
            ("slice", dyn_win.as_str()),
            ("option", dyn_may.as_str()),
            ("result", dyn_out.as_str()),
            ("range", dyn_int.as_str()),
        ],
    ];

    for (pass_idx, mapping) in passes.iter().enumerate() {
        let (_, r_char) = mapping[0];
        let (_, r_slice) = mapping[1];
        let (_, r_opt) = mapping[2];
        let (_, r_res) = mapping[3];
        let (_, r_rng) = mapping[4];

        // Bootstrap fresh `ParserTables` strictly from renamed `stdlib` files!
        let mut renamed_tables = ParserTables::new();
        let mut renamed_core = String::new();
        for rel in bootstrap_rel_paths {
            let raw = std::fs::read_to_string(root.join(rel)).unwrap();
            let renamed = rename_identifiers_word_boundary(&raw, mapping);
            if rel == "stdlib/core/core.pilot" {
                renamed_core = renamed.clone();
            }
            let _ = parse_and_ingest(&renamed, &mut renamed_tables);
        }

        // Verify symbolic type syntax `T[]`, `T?`, `T ! E` now lowers to `r_slice`, `r_opt`, `r_res`!
        let sym_probe = format!(
            "pub struct ProbeTypes {{\n\
                 pub ch: {r_char};\n\
                 pub win: i64[];\n\
                 pub opt: i64?;\n\
                 pub res: i64 ! i32;\n\
                 pub iv: {r_rng}<i64>;\n\
             }}\n"
        );
        let probe_arena = parse_with_tables(&sym_probe, &renamed_tables);
        assert!(
            probe_arena.diagnostics.is_empty(),
            "Renamed symbolic type probe failed parse in pass {}: {:?}",
            pass_idx,
            probe_arena.diagnostics
        );
        let mut type_names = Vec::new();
        collect_type_named_symbols(&probe_arena, probe_arena.root, &mut type_names);
        assert!(
            type_names.contains(&r_slice.to_string())
                && type_names.contains(&r_opt.to_string())
                && type_names.contains(&r_res.to_string()),
            "syntax type_construct did not rewrite T[], T?, T!E to renamed {}, {}, {} (got {:?})",
            r_slice,
            r_opt,
            r_res,
            type_names
        );

        // Build executable kernel using renamed `[Bits]` scalar & renamed struct types across ALL 7 backends!
        let fn_kern = rng.ident("litmus_compute_", 5);
        let gpu_kern = rng.ident("litmus_shader_", 5);
        let arg_x = rng.range_i64(9, 45);
        let mul_m = rng.range_i64(3, 11);
        let add_b = rng.range_i64(17, 89);
        let expected_val = arg_x * mul_m + add_b;

        let kernel_src = format!(
            "[Bits(32, 3)] pub struct {r_char} {{}}\n\
             [Bits(64, 2)] pub struct ScalarI64 {{}}\n\
             pub struct {r_rng} {{ pub start: ScalarI64; pub stop: ScalarI64; }}\n\
             /// Universal Rename Litmus Kernel {pass_idx}\n\
             pub fn {fn_kern}(x: ScalarI64) -> ScalarI64 {{\n\
                 let y = x * {mul_m} + {add_b};\n\
                 return y;\n\
             }}\n\
             pub fn {gpu_kern}(idx: ScalarI64) -> ScalarI64 {{\n\
                 return idx * {mul_m};\n\
             }}\n"
        );
        let _ = renamed_core;

        let k_arena = parse_with_tables(&kernel_src, &renamed_tables);
        assert!(k_arena.diagnostics.is_empty(), "Kernel parse failed: {:?}", k_arena.diagnostics);
        let k_analysis = AnalysisPipeline::analyze(&k_arena, k_arena.root);

        // (A) Backend 1: PilVirtualMachine execution
        let vm_src = format!("{}\nlet __v = {}({});\n__v;\n", kernel_src, fn_kern, arg_x);
        let vm_arena = parse_with_tables(&vm_src, &renamed_tables);
        let mut pil_vm = PilVirtualMachine::new();
        pil_vm.load_binary(BytecodeCompiler::compile(&vm_arena, vm_arena.root));
        assert_eq!(pil_vm.run().as_int(), Some(expected_val), "PilVM failed under rename pass {}", pass_idx);

        // (B) Backends 2, 3, 4, 5: C11, LLVM IR, WASM, and WGSL GPU emitted from mounted renamed AST!
        let cg_driver = format!(
            "{c_phys}\n{c_types}\n{c_hdr}\n{c_emit}\n{llvm_emit}\n{wasm_emit}\n{gpu_c}\n{gpu_sh}\n{gpu_ll}\n\
             let c11 = emit_arena_c11();\n\
             let ll = emit_arena_llvm_ir();\n\
             let wasm_h = emit_arena_wasm_hex();\n\
             let wgsl = emit_arena_gpu_kernel(\"wgsl\", \"{gpu_kern}\");\n\
             let __cg = c11 + \"@@LIT@@\" + ll + \"@@LIT@@\" + wasm_h + \"@@LIT@@\" + wgsl;\n\
             __cg;\n"
        );
        let cg_out = eval_with_mounted_physics(&cg_driver, &kernel_src, &k_analysis.physics);
        let cg_cols: Vec<&str> = cg_out.split("@@LIT@@").collect();
        assert_eq!(cg_cols.len(), 4);

        // Execute C11 via `clang`
        assert_eq!(
            compile_and_run_c11_i64(cg_cols[0], &format!("{}({})", fn_kern, arg_x), &format!("lit_c_{}", pass_idx)),
            expected_val,
            "C11 execution failed under rename pass {}",
            pass_idx
        );

        // Execute LLVM IR via `clang -x ir`
        let nonce = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let tmp_ll = std::env::temp_dir().join(format!("pilot_m61_ll_{}_{}_{}.ll", std::process::id(), nonce, pass_idx));
        let tmp_drv = std::env::temp_dir().join(format!("pilot_m61_ll_{}_{}_{}.c", std::process::id(), nonce, pass_idx));
        let tmp_bin = std::env::temp_dir().join(format!("pilot_m61_ll_{}_{}_{}.bin", std::process::id(), nonce, pass_idx));
        std::fs::write(&tmp_ll, cg_cols[1]).unwrap();
        std::fs::write(
            &tmp_drv,
            format!(
                "#include <stdio.h>\n#include <stdint.h>\n#include <inttypes.h>\nextern int64_t {}(int64_t);\nint main(void){{printf(\"%\" PRId64 \"\\n\", {}({}));return 0;}}\n",
                fn_kern, fn_kern, arg_x
            ),
        )
        .unwrap();
        let ll_comp = Command::new("clang")
            .args([
                "-O2",
                tmp_ll.to_str().unwrap(),
                tmp_drv.to_str().unwrap(),
                "-o",
                tmp_bin.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(ll_comp.status.success(), "clang LLVM IR compile failed:\n{}", String::from_utf8_lossy(&ll_comp.stderr));
        let ll_run = Command::new(&tmp_bin).output().unwrap();
        let _ = std::fs::remove_file(&tmp_ll);
        let _ = std::fs::remove_file(&tmp_drv);
        let _ = std::fs::remove_file(&tmp_bin);
        assert_eq!(
            String::from_utf8(ll_run.stdout).unwrap().trim().parse::<i64>().unwrap(),
            expected_val,
            "LLVM IR execution failed under rename pass {}",
            pass_idx
        );

        // Execute binary WASM via `node` `WebAssembly.Instance`
        let node_wasm = format!(
            "const b = Buffer.from('{}', 'hex');\n\
             const m = new WebAssembly.Module(b);\n\
             const i = new WebAssembly.Instance(m, {{}});\n\
             console.log(Number(i.exports['{}'](BigInt({}))));",
            cg_cols[2].trim(),
            fn_kern,
            arg_x
        );
        let nw_out = Command::new("node").args(["-e", &node_wasm]).output().unwrap();
        assert!(nw_out.status.success(), "WASM execution failed:\n{}", String::from_utf8_lossy(&nw_out.stderr));
        assert_eq!(
            String::from_utf8(nw_out.stdout).unwrap().trim().parse::<i64>().unwrap(),
            expected_val,
            "WASM result mismatch under rename pass {}",
            pass_idx
        );

        // Verify WGSL compute shader contains `@compute` and kernel name
        assert!(
            cg_cols[3].contains("@compute") && cg_cols[3].contains(&gpu_kern),
            "WGSL shader invalid under rename pass {}: {}",
            pass_idx,
            cg_cols[3]
        );

        // (C) Backend 6: LSP `textDocument/hover` on renamed function
        let lsp_combined = format!("{}\n{}", kernel_src, lsp_handlers);
        let lsp_arena = parse_with_tables(&lsp_combined, &renamed_tables);
        let lsp_an = AnalysisPipeline::analyze(&lsp_arena, lsp_arena.root);
        let mut lsp_reg = lsp_an.physics;
        lsp_reg.source_text = lsp_combined.clone();
        lsp_reg.lsp_method = "textDocument/hover".to_string();
        lsp_reg.lsp_offset = (kernel_src.find(&fn_kern).unwrap() + 2) as u32;
        let lsp_ptr = &mut lsp_reg as *mut PhysicsRegistry;
        let mut lsp_vm = PilVirtualMachine::new();
        lsp_vm.engine_handler = Some(Box::new(move |n, a| handle_engine_intrinsic(unsafe { &mut *lsp_ptr }, n, a)));
        lsp_vm.load_binary(BytecodeCompiler::compile(&lsp_arena, lsp_arena.root));
        let _ = lsp_vm.run();
        assert!(
            lsp_reg.lsp_result.contains(&fn_kern),
            "LSP hover failed under rename pass {}: {}",
            pass_idx,
            lsp_reg.lsp_result
        );

        // (D) Backend 7: DAP Name-Blind struct field inspection on renamed `r_rng` (`interval*`)
        let rng_lid = (0..k_analysis.physics.layouts.len())
            .find(|&id| {
                matches!(
                    k_analysis.physics.layouts.get(LayoutId(id as u32)).map(|l| &l.geometry),
                    Some(PhysicsGeometry::Aggregate { fields, .. }) if fields.len() == 2
                )
            })
            .expect("Expected 2-field renamed interval layout");
        let dap_drv = format!("{}\nlet __d = inspect_struct_fields_json({});\n__d;\n", dap_inspector, rng_lid);
        let dap_arena = parse(&dap_drv);
        let (flat, syms) = flatten_ast(&dap_arena, dap_arena.root);
        let mut dap_reg = PhysicsRegistry::new(flat, syms);
        dap_reg.layouts = k_analysis.physics.layouts.clone();
        let dap_ptr = &mut dap_reg as *mut PhysicsRegistry;
        let mut dap_vm = PilVirtualMachine::new();
        dap_vm.engine_handler = Some(Box::new(move |n, a| handle_engine_intrinsic(unsafe { &mut *dap_ptr }, n, a)));
        dap_vm.load_binary(BytecodeCompiler::compile(&dap_arena, dap_arena.root));
        let dap_json = dap_vm.run().to_string_repr();
        assert!(
            dap_json.contains("\"offset\":0") && dap_json.contains("\"offset\":8"),
            "DAP Name-Blind field inspection failed on renamed struct '{}' in pass {}: {}",
            r_rng,
            pass_idx,
            dap_json
        );
    }
    guard.finish();
}
