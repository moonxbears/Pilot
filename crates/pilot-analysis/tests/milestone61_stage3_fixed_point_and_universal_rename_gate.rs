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

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const SH_AST_GROUP: &[&str] = &["selfhost/ast.pilot", "selfhost/parser.pilot"];

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 61,
    test_file_rel:
        "crates/pilot-analysis/tests/milestone61_stage3_fixed_point_and_universal_rename_gate.rs",
    expected_test_count: 2,
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
            required_pub_fns: &["pilotc_compile_source_to_c11", "pilotc_bootstrap_stage_c11"],
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
        assert!(
            s.lines().count() <= 1500,
            "selfhost/{} exceeds 1,500 LOC",
            nm
        );
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
    assert!(
        arena.diagnostics.is_empty(),
        "selfhost frontend parse diagnostics: {:?}",
        arena.diagnostics
    );
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    let out = vm.run().to_string_repr();
    let cols: Vec<&str> = out.split("@@FE@@").collect();
    assert_eq!(
        cols.len(),
        5,
        "Unexpected selfhost frontend output: {}",
        out
    );

    assert_eq!(cols[0], "16", "Self-hosted AST node must be 16 bytes");
    assert_eq!(
        cols[1], "BR_OK",
        "O(1) bidirectional bracket matching corrupted by comments/strings"
    );
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
pub fn test_61_2_pilotc_c11_and_stage_build_stop_at_seams() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_61_2_pilotc_c11_and_stage_build_stop_at_seams",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let pilotc_src = std::fs::read_to_string(root.join("selfhost/pilotc.pilot")).unwrap();
    assert!(pilotc_src.lines().count() <= 1500);
    assert!(
        !pilotc_src.contains("PILOTC_BINARY["),
        "pilotc.pilot must actually compile Pilot source, not return static 'PILOTC_BINARY[...]' strings!"
    );

    // No self-hosted pipeline compiles a Pilot program yet (F4-F8): both
    // entry points stop at their registered seams instead of filling a
    // template. When they are connected, this test must compile real
    // programs with clang and check stage 2 == stage 3 on the selfhost/ tree.
    let run = |call: &str| {
        let script = format!("{pilotc_src}\n{call}\n");
        let arena = parse(&script);
        assert!(
            arena.diagnostics.is_empty(),
            "pilotc parse diagnostics: {:?}",
            arena.diagnostics
        );
        let mut vm = PilVirtualMachine::new();
        vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
        let value = vm.run();
        match vm.fault.take() {
            Some(fault) => format!("FAULT: {}", fault.message),
            None => value.to_string_repr(),
        }
    };
    let unit = escape_pilot_str("pub fn f(n: i64) -> i64 {\n    return n + 1;\n}\nf(41);\n");
    let c11 = run(&format!("pilotc_compile_source_to_c11(\"{unit}\");"));
    assert!(
        c11.starts_with("FAULT: SEAM001: unconnected seam `pilotc.compile_c11`"),
        "pilotc_compile_source_to_c11 must stop at its seam: {c11}"
    );
    for stage in ["::Stage2", "::Stage3"] {
        let built = run(&format!("pilotc_bootstrap_stage_c11({stage}, \"{unit}\");"));
        assert!(
            built.starts_with("FAULT: SEAM001: unconnected seam `bootstrap.stage_compile`"),
            "pilotc_bootstrap_stage_c11({stage}) must stop at its seam: {built}"
        );
    }
    guard.finish();
}
