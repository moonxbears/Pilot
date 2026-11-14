//! ==============================================================================
//! crates/pilot-analysis/tests/milestone53_cst_recovery_and_formatter.rs
//! Milestone 53 — Uncheatable Lossless CST, Live Typo Recovery & Formatter Gate
//!
//! Required `.pilot` Contracts (all tested simultaneously on randomized inputs):
//!   1. `selfhost/cst/cst.pilot` & `selfhost/cst/cst_parser.pilot`:
//!      - `pub fn parse_lossless_cst(src: str) -> i64`
//!        Builds a hierarchical immutable `GreenNode` tree in Pilot memory (`map_new` /
//!        `map_set` / `map_get`) and returns the root `GreenNode` handle (`>= 0`).
//!      - `pub fn cst_node_kind(handle: i64) -> i64`
//!        Returns `cst_kind_program()` (`500`) for root, `cst_kind_fn_decl()` (`501`),
//!        `cst_kind_struct_decl()` (`502`), `cst_kind_interface_decl()` (`503`),
//!        `cst_kind_let_stmt()` (`504`), etc. for declaration children.
//!      - `pub fn cst_child_count(handle: i64) -> i64`
//!      - `pub fn cst_child_at(handle: i64, idx: i64) -> i64`
//!        Every top-level declaration node must itself decompose into `>= 3` child
//!        tokens/nodes (e.g., keyword/header, identifier, params/body/trivia).
//!      - `pub fn cst_node_text(handle: i64) -> str`
//!        Reconstructs lossless source text from the CST subtree (including leading/
//!        trailing trivia, `//` line comments, `/* ... */` block comments, and `#!` shebang).
//!      - `pub fn cst_detect_missing_ident(src: str) -> bool`
//!      - `pub fn cst_detect_missing_rparen(src: str) -> bool`
//!      - `pub fn cst_detect_missing_rbrace(src: str) -> bool`
//!        Must ignore delimiters/keywords inside `"..."` string literals and `//` or `/*...*/` comments!
//!   2. `selfhost/cst/incremental.pilot`:
//!      - `pub fn incremental_reparse_cst(old_root: i64, old_src: str, edit_start: i64, edit_end: i64, replacement: str) -> i64`
//!        Reparses only the affected syntactic island(s) when delimiters converge and
//!        splices a new root `GreenNode` that **reuses the exact unchanged sibling
//!        `GreenNode` handles** (`cst_child_at(new_root, i) == cst_child_at(old_root, i)`
//!        for unaffected top-level declarations `i`), escalating cleanly when an edit
//!        breaks delimiter balance across island boundaries.
//!   3. `selfhost/cst/cst_recovery.pilot`:
//!      - `pub fn suggest_keyword_typo(ident: str) -> str`
//!        Queries live registered keywords in `PhysicsRegistry` via
//!        `__engine("query_registered_symbols", cat)` (note: words are `'|'`-delimited,
//!        supporting core categories `0..=37` and user categories `100..=255` with zero
//!        `u128` bitmask truncation!) to return Damerau-Levenshtein distance-1 fix-its
//!        (insertion, deletion, substitution, adjacent transposition), or `""` if exact
//!        match or distance `> 1`.
//!      - `pub pure fn is_stmt_sync_anchor(tok: str) -> bool` (using `nameof(...)`)
//!      - `pub pure fn is_decl_sync_anchor(tok: str) -> bool` (using `nameof(...)`)
//!   4. `selfhost/fmt/fmt.pilot`:
//!      - `pub fn format_pilot_with_width(src: str, max_width: i64) -> str`
//!      - `pub fn format_pilot_source(src: str) -> str`
//!      - `pub fn verify_format_idempotent(src: str) -> bool`
//!        Implements real Wadler-Lindig `Doc` group layout: expands single-line `{ ... }`
//!        blocks to canonical 4-space indented multi-line blocks, normalizes binary/arrow
//!        operator spacing without corrupting string literals or `//` comments, wraps
//!        long argument/parameter lists when exceeding `max_width` while keeping them
//!        flat when `< max_width`, is strictly idempotent, and preserves program semantics.
//! ==============================================================================

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_analysis::{
    eval_file_with_tables, flatten_ast, handle_engine_intrinsic, BootstrapSnapshot, PhysicsRegistry,
};
use pilot_parser::parse;
use pilot_rt::PilotVal;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const CST_GROUP: &[&str] = &["selfhost/cst/cst.pilot", "selfhost/cst/cst_parser.pilot"];

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 53,
    test_file_rel: "crates/pilot-analysis/tests/milestone53_cst_recovery_and_formatter.rs",
    expected_test_count: 3,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/cst/cst.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/cst/cst_parser.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["parse_lossless_cst"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/cst/incremental.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["incremental_reparse_cst"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/cst/cst_recovery.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "suggest_keyword_typo",
                "is_stmt_sync_anchor",
                "is_decl_sync_anchor",
            ],
            required_code_markers: &["query_registered_symbols", "nameof("],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/fmt/fmt.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "format_pilot_with_width",
                "format_pilot_source",
                "verify_format_idempotent",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[
        ("cst_node_kind", CST_GROUP),
        ("cst_child_count", CST_GROUP),
        ("cst_child_at", CST_GROUP),
        ("cst_node_text", CST_GROUP),
        ("cst_detect_missing_ident", CST_GROUP),
        ("cst_detect_missing_rparen", CST_GROUP),
        ("cst_detect_missing_rbrace", CST_GROUP),
    ],
};

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5353_5353_ABCD_EF01);
        let s = nanos ^ ((std::process::id() as u64).rotate_left(15)) ^ salt;
        Self(if s == 0 { 0x9876_5432_10FE_DCBA } else { s })
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn range(&mut self, lo: i64, hi_inclusive: i64) -> i64 {
        lo + (self.next_u64() % ((hi_inclusive - lo + 1) as u64)) as i64
    }

    fn lower_ident(&mut self, prefix: &str, len: usize) -> String {
        let chars = b"abcdefghijklmnopqrstuvwxyz";
        let mut out = format!("{}_", prefix);
        for _ in 0..len {
            out.push(chars[(self.next_u64() as usize) % chars.len()] as char);
        }
        out
    }

    fn pascal_ident(&mut self, prefix: &str) -> String {
        let chars = b"abcdefghijklmnopqrstuvwxyz";
        let mut out = prefix.to_string();
        for _ in 0..5 {
            out.push(chars[(self.next_u64() as usize) % chars.len()] as char);
        }
        out
    }
}

fn ws_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read_ws(rel: &str) -> String {
    let p = ws_root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("Failed to read {}: {}", p.display(), e))
}

fn escape_pilot_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    out.push('"');
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
    out.push('"');
    out
}

fn run_with_registry(modules: &[&str], tail_src: &str, reg_setup: impl FnOnce(&mut PhysicsRegistry)) -> PilotVal {
    let mut combined = String::new();
    for m in modules {
        combined.push_str(m);
        combined.push('\n');
    }
    combined.push_str(tail_src);
    let arena = parse(&combined);
    assert!(
        arena.diagnostics.is_empty(),
        "Pilot script parse diagnostics: {:?}",
        arena.diagnostics
    );
    let (flat, syms) = flatten_ast(&arena, arena.root);
    let mut registry = PhysicsRegistry::new(flat, syms);
    registry.source_text = combined.clone();
    reg_setup(&mut registry);

    let reg_ptr = &mut registry as *mut PhysicsRegistry;
    let mut vm = PilVirtualMachine::new();
    vm.engine_handler = Some(Box::new(move |op, args| {
        handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, args)
    }));
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    vm.run_checked().unwrap_or_else(|f| panic!("VM fault: {f}"))
}

#[test]
pub fn test_53_1_hierarchical_lossless_cst_and_green_node_incremental_reuse() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_53_1_hierarchical_lossless_cst_and_green_node_incremental_reuse",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5301);
    let cst_src = read_ws("selfhost/cst/cst.pilot");
    let par_src = read_ws("selfhost/cst/cst_parser.pilot");
    let inc_src = read_ws("selfhost/cst/incremental.pilot");

    let snap = BootstrapSnapshot::global();
    for (name, src) in [
        ("selfhost/cst/cst.pilot", &cst_src),
        ("selfhost/cst/cst_parser.pilot", &par_src),
        ("selfhost/cst/incremental.pilot", &inc_src),
    ] {
        let mut tables = snap.tables.clone();
        let mut vm = snap.vm.clone();
        eval_file_with_tables(src, &mut tables, &mut vm)
            .unwrap_or_else(|e| panic!("{} failed bootstrap evaluation: {:?}", name, e));
    }

    for trial in 0..6 {
        let st_name = rng.pascal_ident("Vec");
        let iface_name = rng.pascal_ident("ICompute");
        let fn_edit = rng.lower_ident("target_fn", 5);
        let fn_after = rng.lower_ident("helper_fn", 5);
        let let_var = rng.lower_ident("seed_const", 4);
        let old_lit = rng.range(100, 999);
        let new_lit = rng.range(10000, 99999);

        let d0 = format!("// Header comment {}\npub struct {} {{ pub x: i64; /* inlinetrivia */ }}\n", trial, st_name);
        let d1 = format!("pub interface {} {{ fn step(v: i64) -> i64; }}\n", iface_name);
        let d2 = format!("pub fn {}(a: i64) -> i64 {{\n    let k = {};\n    return a + k;\n}}\n", fn_edit, old_lit);
        let d3 = format!("pub fn {}(b: i64) -> i64 {{ return b * 3; }}\n", fn_after);
        let d4 = format!("let {} = 77;   // trailing trivia\n", let_var);

        let full_src = format!("{}{}{}{}{}", d0, d1, d2, d3, d4);
        let lit_str = old_lit.to_string();
        let edit_start = d0.len() + d1.len() + d2.find(&lit_str).unwrap();
        let edit_end = edit_start + lit_str.len();
        let new_lit_str = new_lit.to_string();
        let expected_edited = format!(
            "{}{}{}",
            &full_src[..edit_start],
            new_lit_str,
            &full_src[edit_end..]
        );

        // Also prepare a brace-breaking edit inside d2 (deleting `}` at end of d2)
        let close_brace_off = d0.len() + d1.len() + d2.rfind('}').unwrap();
        let expected_broken = format!(
            "{}{}",
            &full_src[..close_brace_off],
            &full_src[close_brace_off + 1..]
        );

        let driver = format!(
            "let src = {src};\n\
             let root = parse_lossless_cst(src);\n\
             let root_kind = cst_node_kind(root);\n\
             let top_n = cst_child_count(root);\n\
             let rt_text = cst_node_text(root);\n\
             let c0 = cst_child_at(root, 0);\n\
             let c1 = cst_child_at(root, 1);\n\
             let c2 = cst_child_at(root, 2);\n\
             let c3 = cst_child_at(root, 3);\n\
             let c4 = cst_child_at(root, 4);\n\
             let k0 = cst_node_kind(c0);\n\
             let k1 = cst_node_kind(c1);\n\
             let k2 = cst_node_kind(c2);\n\
             let k3 = cst_node_kind(c3);\n\
             let k4 = cst_node_kind(c4);\n\
             let sub0 = cst_child_count(c0);\n\
             let sub2 = cst_child_count(c2);\n\
             var sub2_concat = \"\";\n\
             var si = 0;\n\
             while si < sub2 {{\n\
                 sub2_concat = sub2_concat + cst_node_text(cst_child_at(c2, si));\n\
                 si = si + 1;\n\
             }}\n\
             let joined_top = cst_node_text(c0) + cst_node_text(c1) + cst_node_text(c2) + cst_node_text(c3) + cst_node_text(c4);\n\
             let new_root = incremental_reparse_cst(root, src, {es}, {ee}, {repl});\n\
             let new_text = cst_node_text(new_root);\n\
             let nc0 = cst_child_at(new_root, 0);\n\
             let nc1 = cst_child_at(new_root, 1);\n\
             let nc2 = cst_child_at(new_root, 2);\n\
             let nc3 = cst_child_at(new_root, 3);\n\
             let nc4 = cst_child_at(new_root, 4);\n\
             let broken_root = incremental_reparse_cst(root, src, {boff}, {boff1}, \"\");\n\
             let broken_text = cst_node_text(broken_root);\n\
             let ok_ lossless = (rt_text == src) && (joined_top == src) && (sub2_concat == cst_node_text(c2));\n\
             let ok_kinds = (root_kind == 500) && (top_n == 5) && (k0 == 502) && (k1 == 503) && (k2 == 501) && (k3 == 501) && (k4 == 504);\n\
             let ok_sub = (sub0 >= 3) && (sub2 >= 3);\n\
             let ok_reuse = (nc0 == c0) && (nc1 == c1) && (nc2 != c2) && (nc3 == c3) && (nc4 == c4);\n\
             let ok_inc_txt = (new_text == {exp_ed}) && (broken_text == {exp_br});\n\
             \"\" + ok_lossless + \"|\" + ok_kinds + \"|\" + ok_sub + \"|\" + ok_reuse + \"|\" + ok_inc_txt;\n",
            src = escape_pilot_str(&full_src),
            es = edit_start,
            ee = edit_end,
            repl = escape_pilot_str(&new_lit_str),
            boff = close_brace_off,
            boff1 = close_brace_off + 1,
            exp_ed = escape_pilot_str(&expected_edited),
            exp_br = escape_pilot_str(&expected_broken),
        );
        // Fix variable name typo `ok_ lossless` -> `ok_lossless` cleanly:
        let driver = driver.replace("ok_ lossless", "ok_lossless");

        let out = run_with_registry(&[&cst_src, &par_src, &inc_src], &driver, |_| {}).to_string_repr();
        assert_eq!(
            out, "true|true|true|true|true",
            "Trial {} failed CST hierarchy / lossless trivia / GreenNode handle reuse check (got `{}`)",
            trial, out
        );
    }
    guard.finish();
}

#[test]
pub fn test_53_2_live_dynamic_keyword_typo_recovery_beyond_u128_and_recovery_markers() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_53_2_live_dynamic_keyword_typo_recovery_beyond_u128_and_recovery_markers",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5302);
    let rec_src = read_ws("selfhost/cst/cst_recovery.pilot");
    let par_src = read_ws("selfhost/cst/cst_parser.pilot");

    // Enforce `nameof(...)` discipline in `cst_recovery.pilot` & `cst_parser.pilot`
    assert!(
        rec_src.contains("nameof(return)") && rec_src.contains("nameof(struct)"),
        "cst_recovery.pilot must use nameof(...) for seed/anchor keywords"
    );

    for trial in 0..6 {
        // Generate brand-new runtime keywords in user categories `100..=255` AND register > 140
        // dummy symbols first so any legacy u128 128-slot bitmask/table truncates!
        let user_cat = rng.range(110, 240);
        let kw_trans = format!("quantum{}x", rng.lower_ident("op", 4)); // length >= 11
        let kw_del = format!("tensor{}z", rng.lower_ident(" flow", 4).replace(' ', ""));
        let kw_sub = format!("chronos{}w", rng.lower_ident("rt", 4));

        // Construct distance-1 typos:
        // 1) Adjacent transposition of chars 2 and 3 in `kw_trans`
        let mut t_bytes = kw_trans.clone().into_bytes();
        t_bytes.swap(2, 3);
        let typo_trans = String::from_utf8(t_bytes).unwrap();

        // 2) Single deletion of char 3 from `kw_del`
        let mut d_chars: Vec<char> = kw_del.chars().collect();
        d_chars.remove(3);
        let typo_del: String = d_chars.into_iter().collect();

        // 3) Single substitution of char 4 in `kw_sub`
        let mut s_bytes = kw_sub.clone().into_bytes();
        s_bytes[4] = if s_bytes[4] == b'q' { b'v' } else { b'q' };
        let typo_sub = String::from_utf8(s_bytes).unwrap();

        // 4) Distance-3 non-match
        let far_word = format!("zzz_{}", kw_trans);

        let driver = format!(
            "let fix_trans = suggest_keyword_typo({t_tr});\n\
             let fix_del = suggest_keyword_typo({t_dl});\n\
             let fix_sub = suggest_keyword_typo({t_sb});\n\
             let exact_self = suggest_keyword_typo({kw_tr});\n\
             let far_none = suggest_keyword_typo({far});\n\
             // Missing-token recovery with string & comment decoy traps:\n\
             let decoy_ok = !cst_detect_missing_ident(\"pub fn ok() {{ let s = \\\"fn (\\\"; // struct {t_tr_word}\\n }}\") \
                         && !cst_detect_missing_rparen(\"pub fn ok() {{ let s = \\\"(\\\"; /* ( */ }}\") \
                         && !cst_detect_missing_rbrace(\"pub fn ok() {{ let s = \\\"{{\\\"; // {{\\n }}\");\n\
             let real_miss = cst_detect_missing_ident(\"pub fn (x: i64) -> i64 {{ return x; }}\") \
                          && cst_detect_missing_rparen(\"pub fn f(x: i64 {{ return x; }}\") \
                          && cst_detect_missing_rbrace(\"pub fn f(x: i64) {{ if x > 0 {{ return x; }}\");\n\
             fix_trans + \"|\" + fix_del + \"|\" + fix_sub + \"|\" + exact_self + \"|\" + far_none + \"|\" + decoy_ok + \"|\" + real_miss;\n",
            t_tr = escape_pilot_str(&typo_trans),
            // The bare word: it sits in a comment inside the decoy source string.
            t_tr_word = typo_trans,
            t_dl = escape_pilot_str(&typo_del),
            t_sb = escape_pilot_str(&typo_sub),
            kw_tr = escape_pilot_str(&kw_trans),
            far = escape_pilot_str(&far_word),
        );

        let kw_t_clone = kw_trans.clone();
        let kw_d_clone = kw_del.clone();
        let kw_s_clone = kw_sub.clone();

        let out = run_with_registry(&[&rec_src, &par_src], &driver, |reg| {
            // Mount core keywords.pilot + 150 filler keywords + our 3 random runtime keywords!
            let _ = handle_engine_intrinsic(
                reg,
                "mount_unit",
                &[PilotVal::from_str(ws_root().join("stdlib/core/keywords.pilot").to_str().unwrap())],
            );
            let _ = handle_engine_intrinsic(
                reg,
                "register_keyword_category",
                &[
                    PilotVal::from_int(user_cat),
                    PilotVal::from_str("keyword.custom.domain.pilot"),
                    PilotVal::from_str("Custom Domain"),
                    PilotVal::from_int(14),
                ],
            );
            for idx in 0..150 {
                let filler = format!("filler_kw_padding_{}", idx);
                let _ = handle_engine_intrinsic(
                    reg,
                    "register_keyword",
                    &[
                        PilotVal::from_int(user_cat),
                        PilotVal::from_str(&filler),
                        PilotVal::from_str("padding beyond 128"),
                    ],
                );
            }
            for w in [&kw_t_clone, &kw_d_clone, &kw_s_clone] {
                let _ = handle_engine_intrinsic(
                    reg,
                    "register_keyword",
                    &[
                        PilotVal::from_int(user_cat),
                        PilotVal::from_str(w),
                        PilotVal::from_str("runtime registered keyword"),
                    ],
                );
            }
        })
        .to_string_repr();

        let expected = format!("{}|{}|{}|||true|true", kw_trans, kw_del, kw_sub);
        assert_eq!(
            out, expected,
            "Trial {}: Live dynamic keyword recovery or decoy-safe missing-token detection failed!",
            trial
        );
    }
    guard.finish();
}

#[test]
pub fn test_53_3_and_53_4_wadler_lindig_width_formatter_semantic_parity_and_crate_purge() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_53_3_and_53_4_wadler_lindig_width_formatter_semantic_parity_and_crate_purge",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5303);
    let fmt_src = read_ws("selfhost/fmt/fmt.pilot");

    for trial in 0..6 {
        let fn_name = rng.lower_ident("compute_step", 5);
        let helper_name = rng.lower_ident("combine_quad", 5);
        let k1 = rng.range(3, 40);
        let k2 = rng.range(5, 60);
        let arg_x = rng.range(7, 50);
        let arg_y = rng.range(2, 30);

        // Ugly single-line + cramped operators + string literal spacing trap + comment
        let messy = format!(
            "// Preserve comment {trial}\n\
             pub   fn   {helper}( alpha_long:i64 , beta_long:i64 , gamma_long:i64 , delta_long:i64 )->i64{{return alpha_long+beta_long+gamma_long+delta_long;}}\n\
             pub  fn  {fn_name}( x:i64,y:i64 )->i64{{let msg=\"  keep   inner   spaces  \";if msg.len()>0{{let sum={helper}(x*{k1},y+{k2},x-y,{k1}+{k2});return sum;}}return 0;}}\n",
            trial = trial,
            helper = helper_name,
            fn_name = fn_name,
            k1 = k1,
            k2 = k2,
        );

        let driver = format!(
            "let messy = {m};\n\
             let fmt_default = format_pilot_source(messy);\n\
             let fmt_twice = format_pilot_source(fmt_default);\n\
             let fmt_wide = format_pilot_with_width(messy, 140);\n\
             let fmt_narrow = format_pilot_with_width(messy, 48);\n\
             let narrow_twice = format_pilot_with_width(fmt_narrow, 48);\n\
             if fmt_default != fmt_twice || fmt_narrow != narrow_twice || !verify_format_idempotent(messy) {{\n\
                 \"IDEMPOTENCY_FAIL\"\n\
             }} else {{\n\
                 fmt_default + \"<<<SEP>>>\" + fmt_wide + \"<<<SEP>>>\" + fmt_narrow\n\
             }};\n",
            m = escape_pilot_str(&messy),
        );

        let res = run_with_registry(&[&fmt_src], &driver, |_| {}).to_string_repr();
        assert_ne!(res, "IDEMPOTENCY_FAIL", "Trial {}: Formatter was not idempotent!", trial);
        let parts: Vec<&str> = res.split("<<<SEP>>>").collect();
        assert_eq!(parts.len(), 3);
        let (fmt_default, fmt_wide, fmt_narrow) = (parts[0], parts[1], parts[2]);

        // 1. Must genuinely reformat crammed `{...}` blocks & spacing (cannot return `src` unchanged!)
        assert_ne!(fmt_default, messy);
        assert!(
            fmt_default.contains(&format!("// Preserve comment {}", trial)),
            "Formatter dropped `//` comment:\n{}",
            fmt_default
        );
        assert!(
            fmt_default.contains("\"  keep   inner   spaces  \""),
            "Formatter corrupted whitespace inside string literal:\n{}",
            fmt_default
        );
        assert!(
            fmt_default.lines().any(|l| l.starts_with("    ")),
            "Formatter must expand single-line `{{ ... }}` blocks with 4-space indentation:\n{}",
            fmt_default
        );
        assert!(
            fmt_default.contains("-> i64"),
            "Formatter must normalize `->i64` to `-> i64`:\n{}",
            fmt_default
        );

        // 2. Wadler-Lindig width sensitivity: `max_width = 48` MUST wrap long parameter/argument
        //    groups onto strictly more lines than `max_width = 140`!
        let wide_lines = fmt_wide.lines().count();
        let narrow_lines = fmt_narrow.lines().count();
        assert!(
            narrow_lines > wide_lines,
            "Trial {}: Wadler-Lindig `format_pilot_with_width` ignored `max_width`! \
             narrow (width=48) produced {} lines vs wide (width=140) {} lines.\nNarrow:\n{}\nWide:\n{}",
            trial, narrow_lines, wide_lines, fmt_narrow, fmt_wide
        );

        // 3. Both `fmt_default` and `fmt_narrow` MUST parse with 0 errors AND evaluate on PilVM
        //    to the exact same runtime integer as `messy`!
        let eval_prog = |code: &str| -> i64 {
            let run_src = format!("{}\nlet __v = {}({}, {});\n__v;\n", code, fn_name, arg_x, arg_y);
            let arena = parse(&run_src);
            assert!(
                arena.diagnostics.is_empty(),
                "Formatted Pilot code failed to parse: {:?}\nCode:\n{}",
                arena.diagnostics,
                code
            );
            let mut vm = PilVirtualMachine::new();
            vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
            vm.run().as_int().expect("Expected int result")
        };

        let expected_val = (arg_x * k1) + (arg_y + k2) + (arg_x - arg_y) + (k1 + k2);
        assert_eq!(eval_prog(&messy), expected_val);
        assert_eq!(eval_prog(fmt_default), expected_val);
        assert_eq!(eval_prog(fmt_narrow), expected_val);
    }

    // Deliverable 53.4: Legacy `pilot-syntax` and `pilot-fmt` completely purged
    let root = ws_root();
    assert!(!root.join("crates/pilot-syntax").exists(), "crates/pilot-syntax must be deleted");
    assert!(!root.join("crates/pilot-fmt").exists(), "crates/pilot-fmt must be deleted");
    let cargo_toml = read_ws("Cargo.toml");
    assert!(!cargo_toml.contains("pilot-syntax") && !cargo_toml.contains("pilot-fmt"));
    guard.finish();
}
