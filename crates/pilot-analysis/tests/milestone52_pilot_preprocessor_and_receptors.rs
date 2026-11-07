//! ==============================================================================
//! crates/pilot-analysis/tests/milestone52_pilot_preprocessor_and_receptors.rs
//! Milestone 52 (Pilot VM Preprocessor & Receptor Gate) — Uncheatable Suite
//!
//! Required `.pilot` Contracts (all tested simultaneously on randomized inputs):
//!   1. `selfhost/preprocessor.pilot`:
//!      - `pub fn eval_preproc_condition(expr: str, defined_csv: str) -> bool`
//!        Evaluates arbitrary nested `!`, `&&` (higher precedence than `||`), `||`,
//!        `(...)`, `defined(SYM)`, `true`/`false`/`1`/`0`, and exact CSV symbols
//!        (must NOT match substrings of longer symbols).
//!      - `pub fn preprocess_source(src: str, initial_symbols_csv: str) -> str`
//!        Implements a true arbitrary-depth `IfFrame` stack (`#if`/`#elif`/`#else`/`#endif`)
//!        plus `#define`/`#undef`, `#region`/`#endregion`, `#pragma`, and line-1 `#!`:
//!        * `#define` / `#undef` inside ANY inactive branch (including nested inside
//!          an outer inactive `#if`) MUST NOT mutate defined symbols.
//!        * An inner `#endif` inside an inactive outer `#if` MUST NOT re-activate
//!          the outer inactive branch.
//!        * Total newline (`\n`) count in the returned string MUST equal `src`'s
//!          newline count so downstream `ByteSpan` line numbers stay 1-to-1 accurate.
//!      - `pub fn validate_preprocessor_directives(src: str, initial_symbols_csv: str) -> str`
//!        Returns comma-separated `PREPROC001`..`PREPROC008` codes over nested stacks:
//!        * `PREPROC001`: unknown/malformed directive or empty `#define`
//!        * `PREPROC002`: `#elif` or `#else` with no open `#if`
//!        * `PREPROC003`: `#elif` or `#else` after `#else`, or stray `#endif`
//!        * `PREPROC004`: any unclosed `#if` remaining on stack at EOF (e.g. 2 `#if`s, 1 `#endif`)
//!        * `PREPROC005`: `#error` in a currently ACTIVE branch (suppressed when any enclosing `#if` is inactive!)
//!        * `PREPROC006`: `#warning` in a currently ACTIVE branch unless suppressed by `#pragma warning disable`
//!          (and re-enabled after `#pragma warning restore`)
//!        * `PREPROC007`: any unclosed `#region` remaining at EOF (e.g. 2 `#region`s, 1 `#endregion`)
//!        * `PREPROC008`: stray `#endregion` with `region_depth == 0`
//!   2. `stdlib/core/syntax/unit_decl.pilot`:
//!      - `pub pure fn unit_family_for_suffix(suffix: str) -> i64`
//!      - `pub pure fn unit_scale_to_base(suffix: str) -> i64`
//!      - `pub fn split_numeric_and_unit_suffix(raw_lit: str) -> UnitDescriptor`
//!      - `pub fn convert_unit_literal_to_base(raw_lit: str) -> i64`
//!        Parses integer digits (ignoring `_` digit separators) before suffix and
//!        multiplies by `unit_scale_to_base(suffix)` (e.g., `"250_000us"` -> `250000000`).
//!   3. `stdlib/core/syntax/fstring.pilot`:
//!      - `pub fn count_fstring_holes(raw: str) -> i64`
//!      - `pub fn extract_fstring_hole_expr(raw: str, target_idx: i64) -> str`
//!      - `pub fn desugar_fstring_to_concat(raw: str) -> str`
//!        Must handle nested `{ ... }` inside holes, string literals containing `}` inside
//!        holes, and unescape `{{` -> `{` and `}}` -> `}` in literal text segments so the
//!        desugared Pilot expression parses and evaluates to the exact interpolated string.
//!   4. `stdlib/core/keywords.pilot`:
//!      - `pub fn keyword_namespace(raw: str) -> str`
//!      - `pub fn keyword_name(raw: str) -> str`
//!      - `pub fn keyword_split(raw: str) -> KeywordDescriptor`
//!        Splits `:ns/name` and hierarchical `:org/sub/name` at the LAST `/` (`namespace = "org/sub"`, `name = "name"`).
//! ==============================================================================

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_analysis::{eval_file_with_tables, BootstrapSnapshot};
use pilot_parser::parse;
use pilot_rt::PilotVal;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5252_AAAA_BBBB_CCCC);
        let s = nanos ^ ((std::process::id() as u64).rotate_left(21)) ^ salt;
        Self(if s == 0 { 0xCAFE_F00D_1234_5678 } else { s })
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

    fn ident(&mut self, prefix: &str) -> String {
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let mut s = format!("{}_", prefix);
        for _ in 0..6 {
            s.push(alphabet[(self.next_u64() as usize) % alphabet.len()] as char);
        }
        s
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
    let mut out = String::with_capacity(s.len() + 8);
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

fn eval_pilot_expr(module_src: &str, tail_expr: &str) -> PilotVal {
    let script = format!("{}\nlet __res = {};\n__res;\n", module_src, tail_expr);
    let arena = parse(&script);
    assert!(
        arena.diagnostics.is_empty(),
        "Unexpected parse diagnostics in test harness:\nDiagnostics: {:?}\nTail: {}",
        arena.diagnostics,
        tail_expr
    );
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    vm.run()
}

#[test]
fn test_52_1_nested_if_stack_inactive_isolation_and_boolean_algebra() {
    let mut rng = Rng::new(0x5210);
    let snap = BootstrapSnapshot::global();
    let mut tables = snap.tables.clone();
    let mut cvm = snap.vm.clone();

    let preproc_src = read_ws("selfhost/preprocessor.pilot");
    eval_file_with_tables(&preproc_src, &mut tables, &mut cvm)
        .expect("selfhost/preprocessor.pilot must evaluate cleanly");

    // 1. 12 randomized boolean expression trials (`&&` tighter than `||`, `!`, `defined(...)`, substring trap)
    for trial in 0..12 {
        let a_sym = rng.ident("SYM_A");
        let b_sym = rng.ident("SYM_B");
        let c_sym = rng.ident("SYM_C");
        let super_a = format!("{}_EXTENDED", a_sym);

        let a_val = (rng.next_u64() & 1) == 1;
        let b_val = (rng.next_u64() & 1) == 1;
        let c_val = (rng.next_u64() & 1) == 1;

        let mut csv_items = vec![super_a]; // substring trap: `SYM_A_EXTENDED` must NOT make `SYM_A` true!
        if a_val {
            csv_items.push(a_sym.clone());
        }
        if b_val {
            csv_items.push(b_sym.clone());
        }
        if c_val {
            csv_items.push(c_sym.clone());
        }
        let csv = csv_items.join(",");

        let expr = format!(
            "defined({a}) || (!{b} && ({c} || false))",
            a = a_sym,
            b = b_sym,
            c = c_sym
        );
        let expected = a_val || (!b_val && c_val);
        let actual = eval_pilot_expr(
            &preproc_src,
            &format!(
                "eval_preproc_condition({}, {})",
                escape_pilot_str(&expr),
                escape_pilot_str(&csv)
            ),
        )
        .as_bool();
        assert_eq!(
            actual,
            Some(expected),
            "Trial {} failed boolean algebra `{}` with csv `{}`",
            trial,
            expr,
            csv
        );
    }

    // 2. 10 randomized 3-level nested `#if`/`#elif`/`#else`/`#endif` torture trials!
    // Specifically catches flat-scalar `top_active` cheats where an inner `#endif` inside
    // an outer `#if false` leaks subsequent inactive lines or mutates `#define`/`#undef`.
    for trial in 0..10 {
        let outer_sym = rng.ident("OUTER");
        let inner_sym = rng.ident("INNER");
        let poison_sym = rng.ident("POISON");
        let kept_sym = rng.ident("KEPT");

        let v_dead_1 = rng.range(100, 999);
        let v_dead_2 = rng.range(1000, 1999);
        let v_live_elif = rng.range(2000, 2999);
        let v_live_nested = rng.range(3000, 3999);
        let v_dead_else = rng.range(4000, 4999);
        let v_final = rng.range(5000, 5999);

        let raw_program = format!(
            "#!/usr/bin/env pilot\n\
             #define {kept}\n\
             #if {outer}\n\
             let leaked_before = {v_dead_1};\n\
             #if true\n\
             #define {poison}\n\
             #undef {kept}\n\
             let leaked_inner = {v_dead_2};\n\
             #endif\n\
             let leaked_after_inner_endif = {v_dead_1};\n\
             #elif {kept} && !{poison}\n\
             let chosen_branch = {v_live_elif};\n\
             #if !{inner}\n\
             let nested_dead = {v_dead_else};\n\
             #else\n\
             let nested_live = {v_live_nested};\n\
             #endif\n\
             #else\n\
             let dead_else = {v_dead_else};\n\
             #endif\n\
             #if {kept} && !{poison}\n\
             let final_sum = chosen_branch + nested_live + {v_final};\n\
             #endif\n\
             final_sum;\n",
            kept = kept_sym,
            outer = outer_sym,
            inner = inner_sym,
            poison = poison_sym,
            v_dead_1 = v_dead_1,
            v_dead_2 = v_dead_2,
            v_live_elif = v_live_elif,
            v_live_nested = v_live_nested,
            v_dead_else = v_dead_else,
            v_final = v_final,
        );

        let initial_csv = inner_sym.clone(); // `OUTER` is false, `INNER` is true
        let preprocessed = eval_pilot_expr(
            &preproc_src,
            &format!(
                "preprocess_source({}, {})",
                escape_pilot_str(&raw_program),
                escape_pilot_str(&initial_csv)
            ),
        )
        .to_string_repr();

        // Exact newline preservation check (critical for ByteSpan line fidelity!)
        let expected_newlines = raw_program.bytes().filter(|&b| b == b'\n').count();
        let actual_newlines = preprocessed.bytes().filter(|&b| b == b'\n').count();
        assert_eq!(
            actual_newlines, expected_newlines,
            "Trial {}: preprocess_source MUST preserve exact line/newline count ({} vs {}):\n{}",
            trial, actual_newlines, expected_newlines, preprocessed
        );

        assert!(
            !preprocessed.contains("leaked_before")
                && !preprocessed.contains("leaked_inner")
                && !preprocessed.contains("leaked_after_inner_endif")
                && !preprocessed.contains("nested_dead")
                && !preprocessed.contains("dead_else"),
            "Trial {}: Inactive lines leaked through nested #if stack:\n{}",
            trial,
            preprocessed
        );

        // Parse and execute the preprocessed Pilot program on PilVM!
        let arena = parse(&preprocessed);
        assert!(
            arena.diagnostics.is_empty(),
            "Trial {}: Preprocessed program had parse errors: {:?}\n{}",
            trial,
            arena.diagnostics,
            preprocessed
        );
        let mut vm = PilVirtualMachine::new();
        vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
        let result = vm.run().as_int();
        assert_eq!(
            result,
            Some(v_live_elif + v_live_nested + v_final),
            "Trial {}: Preprocessed program evaluated to wrong value!\nProgram:\n{}",
            trial,
            preprocessed
        );
    }
}

#[test]
fn test_52_1_all_eight_preproc_diagnostics_and_pragma_suppression() {
    let mut rng = Rng::new(0x5211);
    let preproc_src = read_ws("selfhost/preprocessor.pilot");

    let check_diag = |src: &str, syms: &str| -> String {
        eval_pilot_expr(
            &preproc_src,
            &format!(
                "validate_preprocessor_directives({}, {})",
                escape_pilot_str(src),
                escape_pilot_str(syms)
            ),
        )
        .to_string_repr()
    };

    // PREPROC001: unknown directive & empty `#define`
    let bad_dir = format!("#unknown_{} 123\n", rng.ident("DIR").to_lowercase());
    assert!(check_diag(&bad_dir, "").contains("PREPROC001"));
    assert!(check_diag("#define   \n", "").contains("PREPROC001"));

    // PREPROC002: `#elif` / `#else` without open `#if` (including after closed `#if..#endif`)
    assert!(check_diag("#if true\n#endif\n#elif true\n", "").contains("PREPROC002"));
    assert!(check_diag("#else\n", "").contains("PREPROC002"));

    // PREPROC003: `#elif` after `#else`, duplicate `#else`, stray `#endif`
    assert!(check_diag("#if true\n#else\n#elif false\n#endif\n", "").contains("PREPROC003"));
    assert!(check_diag("#if true\n#else\n#else\n#endif\n", "").contains("PREPROC003"));
    assert!(check_diag("#if true\n#endif\n#endif\n", "").contains("PREPROC003"));

    // PREPROC004: 2-level nested `#if` with only 1 `#endif` at EOF
    assert!(check_diag("#if true\n#if false\n#endif\nlet x = 1;\n", "").contains("PREPROC004"));

    // PREPROC005: `#error` active vs suppressed inside nested inactive `#if`
    assert!(check_diag("#if true\n#error boom\n#endif\n", "").contains("PREPROC005"));
    assert_eq!(
        check_diag("#if false\n#if true\n#error must_not_fire_inside_inactive_outer\n#endif\n#endif\n", ""),
        "",
        "Nested `#if true` inside inactive `#if false` must NOT trigger `#error` (`PREPROC005`)"
    );

    // PREPROC006: `#warning` active vs `#pragma warning disable` / `#pragma warning restore`
    assert!(check_diag("#warning heads_up\n", "").contains("PREPROC006"));
    assert_eq!(
        check_diag("#pragma warning disable\n#warning suppressed_here\n", ""),
        "",
        "`#pragma warning disable` must suppress subsequent `#warning` (`PREPROC006`)"
    );
    assert!(
        check_diag("#pragma warning disable\n#warning suppressed\n#pragma warning restore\n#warning active_again\n", "")
            .contains("PREPROC006"),
        "`#pragma warning restore` must re-enable `#warning` (`PREPROC006`)"
    );

    // PREPROC007 & PREPROC008: nested `#region` / `#endregion`
    assert!(check_diag("#region Outer\n#region Inner\n#endregion\n", "").contains("PREPROC007"));
    assert!(check_diag("#region A\n#endregion\n#endregion\n", "").contains("PREPROC008"));
}

#[test]
fn test_52_3_unit_receptors_fstring_execution_and_hierarchical_keywords() {
    let mut rng = Rng::new(0x5230);
    let unit_src = read_ws("stdlib/core/syntax/unit_decl.pilot");
    let fstr_src = read_ws("stdlib/core/syntax/fstring.pilot");
    let kw_src = read_ws("stdlib/core/keywords.pilot");

    // 1. Randomized `unit_decl.pilot` conversion trials
    let unit_table: &[(&str, i64, i64)] = &[
        ("ns", 1, 1),
        ("us", 1, 1_000),
        ("ms", 1, 1_000_000),
        ("s", 1, 1_000_000_000),
        ("min", 1, 60_000_000_000),
        ("h", 1, 3_600_000_000_000),
        ("Hz", 2, 1),
        ("kHz", 2, 1_000),
        ("MHz", 2, 1_000_000),
        ("GHz", 2, 1_000_000_000),
        ("B", 3, 1),
        ("KB", 3, 1_024),
        ("MB", 3, 1_048_576),
        ("GB", 3, 1_073_741_824),
        ("TB", 3, 1_099_511_627_776),
    ];
    for &(suf, fam, mult) in unit_table {
        let qty = rng.range(2, 500);
        let formatted_lit = if qty >= 100 {
            format!("{}_{:02}{}", qty / 100, qty % 100, suf)
        } else {
            format!("{}{}", qty, suf)
        };
        let actual_fam = eval_pilot_expr(
            &unit_src,
            &format!("unit_family_for_suffix({})", escape_pilot_str(suf)),
        )
        .as_int();
        assert_eq!(actual_fam, Some(fam));

        let converted = eval_pilot_expr(
            &unit_src,
            &format!(
                "convert_unit_literal_to_base({})",
                escape_pilot_str(&formatted_lit)
            ),
        )
        .as_int();
        assert_eq!(
            converted,
            Some(qty * mult),
            "`convert_unit_literal_to_base(\"{}\")` must return {}",
            formatted_lit,
            qty * mult
        );
    }

    // 2. Randomized `fstring.pilot` hole extraction, escaped `{{`/`}}`, string-in-hole `}`, and VM execution!
    for trial in 0..8 {
        let v_a = rng.range(10, 999);
        let v_b = rng.range(10, 999);
        let tag = rng.ident("TAG");
        let raw_fstr = format!(
            "f\"hdr={{{{k}}}}|{tag}:{{alpha + beta * 2}}|{{echo(\"x}}y\")}}!\""
        );

        let holes = eval_pilot_expr(
            &fstr_src,
            &format!("count_fstring_holes({})", escape_pilot_str(&raw_fstr)),
        )
        .as_int();
        assert_eq!(
            holes,
            Some(2),
            "Trial {}: `{}` must have 2 holes (`{{{{k}}}}` is escaped and `\"x}}y\"` is inside string literal)",
            trial,
            raw_fstr
        );

        let hole1 = eval_pilot_expr(
            &fstr_src,
            &format!("extract_fstring_hole_expr({}, 1)", escape_pilot_str(&raw_fstr)),
        )
        .to_string_repr();
        assert_eq!(hole1, "echo(\"x}y\")");

        let desugared = eval_pilot_expr(
            &fstr_src,
            &format!("desugar_fstring_to_concat({})", escape_pilot_str(&raw_fstr)),
        )
        .to_string_repr();

        let exec_script = format!(
            "fn echo(s: str) -> str {{ return s; }}\n\
             let alpha = {v_a};\n\
             let beta = {v_b};\n\
             let out = {desugared};\n\
             out;\n"
        );
        let arena = parse(&exec_script);
        assert!(
            arena.diagnostics.is_empty(),
            "Trial {}: Desugared f-string failed to parse: `{}` -> {:?}",
            trial,
            desugared,
            arena.diagnostics
        );
        let mut vm = PilVirtualMachine::new();
        vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
        let runtime_str = vm.run().to_string_repr();
        let expected_str = format!("hdr={{k}}|{}:{}|x}}y!", tag, v_a + v_b * 2);
        assert_eq!(
            runtime_str, expected_str,
            "Trial {}: Desugared f-string evaluated to wrong string (`{}`)",
            trial, desugared
        );
    }

    // 3. Randomized `:ns/kw` and multi-segment `:org/sub/kw` keyword splitting
    for _ in 0..8 {
        let seg1 = rng.ident("ns").to_lowercase();
        let seg2 = rng.ident("sub").to_lowercase();
        let name = rng.ident("kw").to_lowercase();

        let simple = format!(":{}/{}", seg1, name);
        assert_eq!(
            eval_pilot_expr(&kw_src, &format!("keyword_namespace({})", escape_pilot_str(&simple)))
                .to_string_repr(),
            seg1
        );
        assert_eq!(
            eval_pilot_expr(&kw_src, &format!("keyword_name({})", escape_pilot_str(&simple)))
                .to_string_repr(),
            name
        );

        let hierarchical = format!(":{}/{}/{}", seg1, seg2, name);
        assert_eq!(
            eval_pilot_expr(
                &kw_src,
                &format!("keyword_namespace({})", escape_pilot_str(&hierarchical))
            )
            .to_string_repr(),
            format!("{}/{}", seg1, seg2),
            "Multi-segment keyword `:a/b/c` must split at the LAST `/` -> namespace `a/b`"
        );
        assert_eq!(
            eval_pilot_expr(
                &kw_src,
                &format!("keyword_name({})", escape_pilot_str(&hierarchical))
            )
            .to_string_repr(),
            name
        );

        let bare = format!(":{}", name);
        assert_eq!(
            eval_pilot_expr(&kw_src, &format!("keyword_namespace({})", escape_pilot_str(&bare)))
                .to_string_repr(),
            ""
        );
        assert_eq!(
            eval_pilot_expr(&kw_src, &format!("keyword_name({})", escape_pilot_str(&bare)))
                .to_string_repr(),
            name
        );
    }
}
