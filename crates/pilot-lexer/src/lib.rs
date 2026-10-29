//! Milestone 29: Stage 1 — Zero-Copy SIMD Preprocessor, Structural Indexer & Modal Lexer (`pilot-lexer`).

pub mod simd;
pub mod token;
pub mod preproc;
pub mod keywords;
pub mod lexer;

pub use token::{TokenKind, TokenFlags, TokenTape};
pub use preproc::{Preprocessor, PreprocessorOptions, PreprocessorResult, PreprocessorDiagnostic, DiagnosticSeverity, CodeRegion, WarningSuppression};
pub use lexer::Lexer;

/// Tokenizes `source` with default preprocessor options into a dense SoA `TokenTape`.
pub fn tokenize(source: &str) -> TokenTape {
    Lexer::new(source, PreprocessorOptions::default()).tokenize()
}

/// Tokenizes `source` with custom `PreprocessorOptions`.
pub fn tokenize_with_options(source: &str, options: PreprocessorOptions) -> TokenTape {
    Lexer::new(source, options).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_token_tape_basic_and_balanced_bracket_links() {
        let code = "def add(x: i64, y: i64) -> i64 { return x + y; }";
        let tape = tokenize(code);

        assert!(!tape.is_empty());
        assert_eq!(tape.kind(0), TokenKind::KwDef);

        // Find '(' and '{'
        let mut lparen_idx = None;
        let mut rparen_idx = None;
        let mut lbrace_idx = None;
        let mut rbrace_idx = None;

        for i in 0..tape.len() {
            match tape.kind(i) {
                TokenKind::LParen => lparen_idx = Some(i),
                TokenKind::RParen => rparen_idx = Some(i),
                TokenKind::LBrace => lbrace_idx = Some(i),
                TokenKind::RBrace => rbrace_idx = Some(i),
                _ => {}
            }
        }

        assert!(lparen_idx.is_some() && rparen_idx.is_some());
        assert_eq!(tape.matching_bracket(lparen_idx.unwrap()), Some(rparen_idx.unwrap() as u32));
        assert_eq!(tape.matching_bracket(rparen_idx.unwrap()), Some(lparen_idx.unwrap() as u32));

        assert!(lbrace_idx.is_some() && rbrace_idx.is_some());
        assert_eq!(tape.matching_bracket(lbrace_idx.unwrap()), Some(rbrace_idx.unwrap() as u32));
        assert_eq!(tape.matching_bracket(rbrace_idx.unwrap()), Some(lbrace_idx.unwrap() as u32));
    }

    #[test]
    fn test_pythonic_indentation_virtual_braces() {
        let code = "if true:\n    let x = 10\n    return x\nlet y = 20";
        let tape = tokenize(code);

        let mut has_virtual_lbrace = false;
        let mut has_virtual_rbrace = false;

        for i in 0..tape.len() {
            if tape.kind(i) == TokenKind::VirtualLBrace {
                has_virtual_lbrace = true;
            }
            if tape.kind(i) == TokenKind::VirtualRBrace {
                has_virtual_rbrace = true;
            }
        }

        assert!(has_virtual_lbrace, "Expected VirtualLBrace from indented block after colon.");
        assert!(has_virtual_rbrace, "Expected VirtualRBrace from dedented block.");
    }

    #[test]
    fn test_call_quoting_maximal_munch_invariant() {
        // fn!(a, b) -> CallQuoteShallow
        // fn!!(a, b) -> CallQuoteDeep
        // fn!&(a, b) -> CallQuoteAbi
        // fn!^(a, b) -> CallQuoteReact
        // fn!@(a, b) -> CallQuoteAdjoint
        // fn!?(a, b) -> CallQuoteEffect
        // BUT if not followed immediately by '(':
        // x != y -> NotEqual
        // !flag -> Exclamation
        let code = "fn!(a) + fn!!(b) + fn!&(c) + fn!^(d) + fn!@(e) + fn!?(f) + (x != y) + !flag";
        let tape = tokenize(code);

        let kinds: Vec<TokenKind> = (0..tape.len()).map(|i| tape.kind(i)).collect();

        assert!(kinds.contains(&TokenKind::CallQuoteShallow));
        assert!(kinds.contains(&TokenKind::CallQuoteDeep));
        assert!(kinds.contains(&TokenKind::CallQuoteAbi));
        assert!(kinds.contains(&TokenKind::CallQuoteReact));
        assert!(kinds.contains(&TokenKind::CallQuoteAdjoint));
        assert!(kinds.contains(&TokenKind::CallQuoteEffect));
        assert!(kinds.contains(&TokenKind::NotEqual));
        assert!(kinds.contains(&TokenKind::Exclamation));
    }

    #[test]
    fn test_dimensional_literals() {
        let code = "let t = 250ms; let f = 2.4GHz; let m = 64KB;";
        let tape = tokenize(code);

        let mut durations = 0;
        let mut freqs = 0;
        let mut mems = 0;

        for i in 0..tape.len() {
            match tape.kind(i) {
                TokenKind::DurationLit => durations += 1,
                TokenKind::FrequencyLit => freqs += 1,
                TokenKind::MemoryLit => mems += 1,
                _ => {}
            }
        }

        assert_eq!(durations, 1);
        assert_eq!(freqs, 1);
        assert_eq!(mems, 1);
    }

    #[test]
    fn test_preprocessor_zero_copy_conditional_compilation() {
        let code = r#"
#if DEBUG
let mode = "debug"
#else
let mode = "release"
#endif
"#;

        let mut init = HashSet::new();
        init.insert("DEBUG".to_string());
        let tape_debug = tokenize_with_options(code, PreprocessorOptions { initial_symbols: init });

        let debug_text: Vec<&str> = (0..tape_debug.len())
            .filter(|&i| tape_debug.kind(i) == TokenKind::StrLit)
            .map(|i| tape_debug.text(i, code))
            .collect();
        assert_eq!(debug_text, vec!["\"debug\""]);

        let tape_release = tokenize_with_options(code, PreprocessorOptions::default());
        let release_text: Vec<&str> = (0..tape_release.len())
            .filter(|&i| tape_release.kind(i) == TokenKind::StrLit)
            .map(|i| tape_release.text(i, code))
            .collect();
        assert_eq!(release_text, vec!["\"release\""]);
    }
}
