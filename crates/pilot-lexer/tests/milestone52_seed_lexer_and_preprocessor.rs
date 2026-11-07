use pilot_lexer::{tokenize, TokenFlags, TokenKind};
use std::path::Path;

#[test]
fn test_token_kind_strict_14_seed_keywords_only() {
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
        assert_eq!(tape.kind(0), expected_kind, "Seed keyword '{}' must lex as {:?}", word, expected_kind);
        assert!(tape.kind(0).is_keyword());
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
        "match", "class", "record", "elif", "lambda",
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
}

#[test]
fn test_name_blind_unit_suffixes_fstrings_and_keywords() {
    let src = r#"let a = 250ms; let b = 2.4GHz; let c = 64KB; let d = 100mol; let s = f"val={a}"; let k = :http/ok;"#;
    let tape = tokenize(src);

    let mut dim_tokens = Vec::new();
    let mut interp_tokens = Vec::new();
    let mut kw_lits = Vec::new();

    for i in 0..tape.len() {
        if tape.flag(i).contains(TokenFlags::IS_DIMENSIONAL) {
            dim_tokens.push((tape.kind(i), tape.slice_str(i, src)));
        }
        if tape.flag(i).contains(TokenFlags::IS_INTERPOLATED) {
            interp_tokens.push((tape.kind(i), tape.slice_str(i, src)));
        }
        if tape.kind(i) == TokenKind::KeywordLit {
            kw_lits.push(tape.slice_str(i, src));
        }
    }

    assert_eq!(
        dim_tokens,
        vec![
            (TokenKind::IntLit, "250ms"),
            (TokenKind::FloatLit, "2.4GHz"),
            (TokenKind::IntLit, "64KB"),
            (TokenKind::IntLit, "100mol"),
        ]
    );
    assert_eq!(interp_tokens, vec![(TokenKind::StrLit, r#"f"val={a}""#)]);
    assert_eq!(kw_lits, vec![":http/ok"]);
}

#[test]
fn test_milestone52_line_counts_and_legacy_deletion_invariants() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();

    // 1. Verify crates/pilot-lexer/src/preproc.rs is deleted
    let preproc_rs = workspace_root.join("crates/pilot-lexer/src/preproc.rs");
    assert!(!preproc_rs.exists(), "Legacy crates/pilot-lexer/src/preproc.rs must be deleted");

    // 2. Verify crates/pilot-lexer total <= 900 LOC
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

    // 3. Verify crates/pilot-lexer/src/keywords.rs has zero RwLock or DYNAMIC_KEYWORDS
    let kw_rs = std::fs::read_to_string(workspace_root.join("crates/pilot-lexer/src/keywords.rs")).unwrap();
    assert!(!kw_rs.contains("RwLock") && !kw_rs.contains("DYNAMIC_KEYWORDS"));

    // 4. Verify crates/pilot-lexer/src/lexer.rs has zero hardcoded unit suffixes
    let lexer_rs = std::fs::read_to_string(workspace_root.join("crates/pilot-lexer/src/lexer.rs")).unwrap();
    for forbidden_unit in ["\"ns\"", "\"ms\"", "\"GHz\"", "\"KB\"", "\"TB\""] {
        assert!(
            !lexer_rs.contains(forbidden_unit),
            "crates/pilot-lexer/src/lexer.rs must not hardcode unit suffix {}",
            forbidden_unit
        );
    }

    // 5. Verify crates/pilot-intern/src/lib.rs <= 165 LOC and has zero KeywordId or hardcoded type strings
    let intern_rs = std::fs::read_to_string(workspace_root.join("crates/pilot-intern/src/lib.rs")).unwrap();
    let intern_loc = intern_rs.lines().count();
    assert!(
        intern_loc <= 165,
        "crates/pilot-intern/src/lib.rs must be <= 165 LOC, got {}",
        intern_loc
    );
    assert!(!intern_rs.contains("KeywordId") && !intern_rs.contains("intern_keyword"));
    assert!(!intern_rs.contains("SliceBuffer") && !intern_rs.contains("RangeWindow"));

    // 6. Verify selfhost/preprocessor.pilot has zero prohibited `#` comments and <= 1,500 LOC
    let preproc_pilot = std::fs::read_to_string(workspace_root.join("selfhost/preprocessor.pilot")).unwrap();
    let preproc_loc = preproc_pilot.lines().count();
    assert!(preproc_loc >= 250 && preproc_loc <= 1500);
    for (idx, line) in preproc_pilot.lines().enumerate() {
        let trimmed = line.trim_start();
        assert!(
            !trimmed.starts_with('#'),
            "selfhost/preprocessor.pilot line {} must not use '#' comments: {}",
            idx + 1,
            line
        );
    }
    for code in [
        "PREPROC001", "PREPROC002", "PREPROC003", "PREPROC004",
        "PREPROC005", "PREPROC006", "PREPROC007", "PREPROC008",
    ] {
        assert!(
            preproc_pilot.contains(code),
            "selfhost/preprocessor.pilot must implement diagnostic code {}",
            code
        );
    }
}
