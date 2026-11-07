//! ==============================================================================
//! crates/pilot-lexer/tests/milestone52_seed_lexer_and_preprocessor.rs
//! Milestone 52 (Lexer Crate Gate) — Uncheatable 14-Token Seed Lexer Verification
//!
//! Enforces:
//!   1. Strictly 14 `TokenKind::Kw*` seed keywords (`__engine`..`asm`); every non-seed
//!      identifier, surface/domain keyword, legacy word, and randomized prefix/suffix
//!      variant lexes strictly as `TokenKind::Ident`.
//!   2. Context-sensitive `InfixDot` vs `PrefixDot`, ranges (`..`, `..=`, `..<`, `...`),
//!      and safe navigation (`?.`, `?..`).
//!   3. All 6 homoiconic call-quote tokens (`!(`, `!!(`, `!&(`, `!^(`, `!@(`, `!?(`)
//!      requiring zero whitespace before `(`, vs `Exclamation` / `NotEqual` otherwise.
//!   4. O(1) `TokenTape::matching_bracket` links across randomized nested bracket trees.
//!   5. Name-blind `IS_DIMENSIONAL`, `IS_INTERPOLATED`, and `KeywordLit` (`:ns/kw`) flags.
//!   6. Contraction & legacy purge: `preproc.rs` deleted, `pilot-lexer <= 900` LOC,
//!      `pilot-intern <= 165` LOC, zero `RwLock`/`DYNAMIC_KEYWORDS`, zero hardcoded unit
//!      strings in `lexer.rs`, and zero `#` comments in `selfhost/preprocessor.pilot`.
//! ==============================================================================

use pilot_lexer::{tokenize, TokenFlags, TokenKind};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "../../pilot-analysis/tests/common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 52,
    test_file_rel: "crates/pilot-lexer/tests/milestone52_seed_lexer_and_preprocessor.rs",
    expected_test_count: 4,
    files: &[anticheat::FileAuditRule {
        path: "selfhost/preprocessor.pilot",
        max_lines: 1500,
        name_blind: false,
        required_pub_fns: &[],
        required_code_markers: &["PREPROC001"],
        forbidden_code_markers: &[],
        forbidden_string_substrings: &[],
    }],
    required_group_pub_fns: &[],
};

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5252_5252_CAFE_BABE);
        let s = nanos ^ ((std::process::id() as u64).rotate_left(13)) ^ salt;
        Self(if s == 0 { 0xDEAD_BEEF_1234_5678 } else { s })
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + (self.next_u64() as usize % (hi_inclusive - lo + 1))
    }

    fn alpha_word(&mut self, len: usize) -> String {
        let chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let mut out = String::with_capacity(len);
        for _ in 0..len {
            out.push(chars[self.range(0, chars.len() - 1)] as char);
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

#[test]
fn test_52_2_strict_14_seed_keywords_and_randomized_ident_boundary() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_52_2_strict_14_seed_keywords_and_randomized_ident_boundary",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5201);
    let seeds = [
        ("__engine", TokenKind::KwEngine),
        ("struct", TokenKind::KwStruct),
        ("interface", TokenKind::KwInterface),
        ("fn", TokenKind::KwFn),
        ("let", TokenKind::KwLet),
        ("ref", TokenKind::KwRef),
        ("if", TokenKind::KwIf),
        ("loop", TokenKind::KwLoop),
        ("return", TokenKind::KwReturn),
        ("yield", TokenKind::KwYield),
        ("defer", TokenKind::KwDefer),
        ("comptime", TokenKind::KwComptime),
        ("quote", TokenKind::KwQuote),
        ("asm", TokenKind::KwAsm),
    ];

    for (word, expected_kind) in seeds {
        let tape = tokenize(word);
        assert_eq!(
            tape.kind(0),
            expected_kind,
            "Seed keyword '{}' must lex as {:?}",
            word,
            expected_kind
        );
        assert!(tape.kind(0).is_keyword());

        // Near-miss prefix/suffix variants of every seed keyword must lex as Ident
        let suffixed = format!("{}_{}", word, rng.range(1, 999));
        let prefixed = format!("x_{}", word);
        for near in [suffixed, prefixed] {
            let nt = tokenize(&near);
            assert_eq!(
                nt.kind(0),
                TokenKind::Ident,
                "Seed near-miss '{}' must lex as Ident",
                near
            );
            assert!(!nt.kind(0).is_keyword());
        }
    }

    let non_seed_words = [
        "var", "const", "switch", "case", "default", "for", "in", "while", "until", "then",
        "break", "continue", "guard", "try", "catch", "pub", "priv", "prot", "pkg", "int",
        "file", "open", "export", "lvl", "pure", "static", "iso", "partial", "ext", "get",
        "set", "self", "field", "value", "scope", "enter", "exit", "requires", "ensures",
        "old", "context", "adapter", "each", "coalesce", "with", "without", "as", "async",
        "await", "spawn", "atomic", "sync", "isolate", "select", "timeout", "bounded",
        "noalloc", "region", "deadline", "within", "periodic", "priority", "jitter",
        "database", "transaction", "commit", "rollback", "savepoint", "connect", "from",
        "where", "order", "ascending", "descending", "insert", "into", "values", "update",
        "delete", "extern", "module", "lib", "syntax", "unit", "use", "op", "def", "mut",
        "match", "class", "record", "elif", "lambda", "i64", "u8", "f64", "str", "bool",
        "usize", "slice", "option", "result", "range", "SliceBuffer", "RangeWindow",
    ];
    for word in non_seed_words {
        let tape = tokenize(word);
        assert_eq!(
            tape.kind(0),
            TokenKind::Ident,
            "Non-seed word '{}' must lex strictly as TokenKind::Ident",
            word
        );
        assert!(!tape.kind(0).is_keyword());
    }

    for _ in 0..40 {
        let rand_id = format!("u_{}_{}", rng.alpha_word(6), rng.range(10, 9999));
        let tape = tokenize(&rand_id);
        assert_eq!(tape.kind(0), TokenKind::Ident);
        assert!(!tape.kind(0).is_keyword());
    }
    guard.finish();
}

#[test]
fn test_52_2_two_way_dot_call_quotes_and_nested_bracket_matching() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_52_2_two_way_dot_call_quotes_and_nested_bracket_matching",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5202);

    // 1. Two-way dot (`InfixDot` vs `PrefixDot`)
    let infix_cases = ["alpha.beta", "call().next", "arr[0].len", "{x}.val"];
    for c in infix_cases {
        let tape = tokenize(c);
        assert!(
            tape.kinds.contains(&TokenKind::InfixDot),
            "Expected InfixDot in `{}`: {:?}",
            c,
            tape.kinds
        );
        assert!(
            !tape.kinds.contains(&TokenKind::PrefixDot),
            "Unexpected PrefixDot in `{}`: {:?}",
            c,
            tape.kinds
        );
    }

    let prefix_cases = [
        " .reset()",
        ";.reset()",
        "(.count)",
        "[.first]",
        "{.inner}",
        "a, .second",
        "0.. .limit",
    ];
    for c in prefix_cases {
        let tape = tokenize(c);
        assert!(
            tape.kinds.contains(&TokenKind::PrefixDot),
            "Expected PrefixDot in `{}`: {:?}",
            c,
            tape.kinds
        );
    }

    // 2. 6 Call-Quoting sigils (zero-whitespace `(` lookahead) vs non-quoted `!`
    let fn_name = format!("kern_{}", rng.alpha_word(5));
    let quoted_src = format!(
        "{f}!(1) {f}!!(2) {f}!&(3) {f}!^(4) {f}!@(5) {f}!?(6)",
        f = fn_name
    );
    let qt = tokenize(&quoted_src);
    for expected in [
        TokenKind::CallQuoteShallow,
        TokenKind::CallQuoteDeep,
        TokenKind::CallQuoteAbi,
        TokenKind::CallQuoteReact,
        TokenKind::CallQuoteAdjoint,
        TokenKind::CallQuoteEffect,
    ] {
        assert!(
            qt.kinds.contains(&expected),
            "Missing {:?} in `{}`",
            expected,
            quoted_src
        );
    }

    // With whitespace before `(`, `!` must NOT lex as CallQuoteShallow
    let spaced_bang = format!("{}! (1)", fn_name);
    let st = tokenize(&spaced_bang);
    assert!(
        !st.kinds.contains(&TokenKind::CallQuoteShallow),
        "`f! (1)` with space before `(` must not lex as CallQuoteShallow"
    );
    assert!(st.kinds.contains(&TokenKind::Exclamation));

    // 3. Randomized nested bracket matching O(1) bidirectional links
    for _ in 0..10 {
        let pairs = [('(', ')'), ('[', ']'), ('{', '}')];
        let mut src = String::new();
        let mut stack = Vec::new();
        let steps = rng.range(8, 18);
        for _ in 0..steps {
            let (op, cl) = pairs[rng.range(0, 2)];
            src.push(op);
            src.push_str(&format!(" x_{} ", rng.range(1, 99)));
            stack.push(cl);
            if stack.len() > 1 && rng.range(0, 2) == 0 {
                src.push(stack.pop().unwrap());
                src.push(' ');
            }
        }
        while let Some(cl) = stack.pop() {
            src.push(cl);
        }

        let tape = tokenize(&src);
        let mut open_count = 0usize;
        for i in 0..tape.len() {
            match tape.kind(i) {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                    open_count += 1;
                    let close_idx = tape
                        .matching_bracket(i)
                        .unwrap_or_else(|| panic!("Missing matching close bracket at {} in `{}`", i, src))
                        as usize;
                    assert!(close_idx > i);
                    assert_eq!(
                        tape.matching_bracket(close_idx),
                        Some(i as u32),
                        "Bidirectional bracket link mismatch at {} <-> {}",
                        i,
                        close_idx
                    );
                }
                _ => {}
            }
        }
        assert!(open_count >= 8);
    }
    guard.finish();
}

#[test]
fn test_52_3_name_blind_unit_suffixes_fstrings_and_keywords_randomized() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_52_3_name_blind_unit_suffixes_fstrings_and_keywords_randomized",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5203);
    for _ in 0..8 {
        let u1 = rng.alpha_word(4);
        let u2 = rng.alpha_word(5);
        let n1 = rng.range(10, 9999);
        let ns = rng.alpha_word(5).to_lowercase();
        let kw = rng.alpha_word(6).to_lowercase();
        let src = format!(
            "let a = {n1}{u1}; let b = 3.25{u2}; let s = f\"v={{a}}\"; let k = :{ns}/{kw};"
        );
        let tape = tokenize(&src);

        let mut dim_tokens = Vec::new();
        let mut interp_tokens = Vec::new();
        let mut kw_lits = Vec::new();
        for i in 0..tape.len() {
            if tape.flag(i).contains(TokenFlags::IS_DIMENSIONAL) {
                dim_tokens.push((tape.kind(i), tape.slice_str(i, &src).to_string()));
            }
            if tape.flag(i).contains(TokenFlags::IS_INTERPOLATED) {
                interp_tokens.push((tape.kind(i), tape.slice_str(i, &src).to_string()));
            }
            if tape.kind(i) == TokenKind::KeywordLit {
                kw_lits.push(tape.slice_str(i, &src).to_string());
            }
        }

        assert_eq!(
            dim_tokens,
            vec![
                (TokenKind::IntLit, format!("{}{}", n1, u1)),
                (TokenKind::FloatLit, format!("3.25{}", u2)),
            ]
        );
        assert_eq!(interp_tokens, vec![(TokenKind::StrLit, "f\"v={a}\"".to_string())]);
        assert_eq!(kw_lits, vec![format!(":{}/{}", ns, kw)]);
    }
    guard.finish();
}

#[test]
fn test_52_4_contraction_caps_and_legacy_deletion_invariants() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_52_4_contraction_caps_and_legacy_deletion_invariants",
        &ANTICHEAT_SPEC,
    );
    let workspace_root = ws_root();

    assert!(
        !workspace_root.join("crates/pilot-lexer/src/preproc.rs").exists(),
        "Legacy crates/pilot-lexer/src/preproc.rs must be deleted"
    );

    let lexer_files = [
        "crates/pilot-lexer/src/lib.rs",
        "crates/pilot-lexer/src/token.rs",
        "crates/pilot-lexer/src/keywords.rs",
        "crates/pilot-lexer/src/lexer.rs",
        "crates/pilot-lexer/src/simd.rs",
    ];
    let mut total_lexer_loc = 0usize;
    for rel in lexer_files {
        let content = std::fs::read_to_string(workspace_root.join(rel)).unwrap();
        total_lexer_loc += content.lines().count();
    }
    assert!(
        total_lexer_loc <= 900,
        "crates/pilot-lexer total LOC must be <= 900, got {}",
        total_lexer_loc
    );

    let kw_rs = std::fs::read_to_string(workspace_root.join("crates/pilot-lexer/src/keywords.rs")).unwrap();
    assert!(!kw_rs.contains("RwLock") && !kw_rs.contains("DYNAMIC_KEYWORDS"));

    let lexer_rs = std::fs::read_to_string(workspace_root.join("crates/pilot-lexer/src/lexer.rs")).unwrap();
    for forbidden_unit in ["\"ns\"", "\"us\"", "\"ms\"", "\"GHz\"", "\"KB\"", "\"MB\"", "\"TB\""] {
        assert!(
            !lexer_rs.contains(forbidden_unit),
            "crates/pilot-lexer/src/lexer.rs must not hardcode unit suffix {}",
            forbidden_unit
        );
    }

    let token_rs = std::fs::read_to_string(workspace_root.join("crates/pilot-lexer/src/token.rs")).unwrap();
    let is_id = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let has_whole_ident = |haystack: &str, needle: &str| -> bool {
        let hb = haystack.as_bytes();
        haystack.match_indices(needle).any(|(pos, _)| {
            let before_ok = pos == 0 || !is_id(hb[pos - 1]);
            let after_pos = pos + needle.len();
            let after_ok = after_pos == hb.len() || !is_id(hb[after_pos]);
            before_ok && after_ok
        })
    };
    for forbidden_variant in [
        "KwDef", "KwMut", "KwMatch", "KwClass", "KwRecord", "KwElif", "KwLambda",
        "DurationLit", "FrequencyLit", "MemoryLit", "VirtualLBrace", "VirtualRBrace",
    ] {
        assert!(
            !has_whole_ident(&token_rs, forbidden_variant),
            "crates/pilot-lexer/src/token.rs must not contain legacy variant {}",
            forbidden_variant
        );
    }

    let intern_rs = std::fs::read_to_string(workspace_root.join("crates/pilot-intern/src/lib.rs")).unwrap();
    let intern_loc = intern_rs.lines().count();
    assert!(
        intern_loc <= 165,
        "crates/pilot-intern/src/lib.rs must be <= 165 LOC, got {}",
        intern_loc
    );
    assert!(!intern_rs.contains("KeywordId") && !intern_rs.contains("intern_keyword"));
    assert!(!intern_rs.contains("SliceBuffer") && !intern_rs.contains("RangeWindow"));

    let preproc_pilot = std::fs::read_to_string(workspace_root.join("selfhost/preprocessor.pilot")).unwrap();
    let preproc_loc = preproc_pilot.lines().count();
    assert!(
        (1..=1500).contains(&preproc_loc),
        "selfhost/preprocessor.pilot LOC must be in 1..=1500, got {}",
        preproc_loc
    );
    for (idx, line) in preproc_pilot.lines().enumerate() {
        let trimmed = line.trim_start();
        assert!(
            !trimmed.starts_with('#'),
            "selfhost/preprocessor.pilot line {} must not use prohibited '#' comments: {}",
            idx + 1,
            line
        );
    }
    guard.finish();
}
