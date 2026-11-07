//! Milestone 29 & 52: Stage 1 — Zero-Copy SIMD Seed Lexer (`pilot-lexer`).
//!
//! Recognizes strictly the **14 Hardware Seed Keywords** (`KwEngine`..`KwAsm`), raw atomic literals,
//! two-way dot (`InfixDot` vs `PrefixDot`), and 6 homoiconic call-quote projections.

pub mod keywords;
pub mod lexer;
pub mod simd;
pub mod token;

pub use lexer::Lexer;
pub use token::{TokenFlags, TokenKind, TokenTape};

/// Tokenizes `source` into a dense Struct-of-Arrays `TokenTape`.
pub fn tokenize(source: &str) -> TokenTape {
    Lexer::new(source).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_14_seed_tokens_vs_identifiers() {
        let code = "__engine struct interface fn let ref if loop return yield defer comptime quote asm var switch for async select";
        let tape = tokenize(code);
        let kinds: Vec<TokenKind> = tape.kinds.iter().copied().filter(|&k| k != TokenKind::Eof).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::KwEngine,
                TokenKind::KwStruct,
                TokenKind::KwInterface,
                TokenKind::KwFn,
                TokenKind::KwLet,
                TokenKind::KwRef,
                TokenKind::KwIf,
                TokenKind::KwLoop,
                TokenKind::KwReturn,
                TokenKind::KwYield,
                TokenKind::KwDefer,
                TokenKind::KwComptime,
                TokenKind::KwQuote,
                TokenKind::KwAsm,
                TokenKind::Ident, // var
                TokenKind::Ident, // switch
                TokenKind::Ident, // for
                TokenKind::Ident, // async
                TokenKind::Ident, // select
            ]
        );
    }

    #[test]
    fn test_two_way_dot_and_6_call_quoting_modes() {
        let code = "obj.field . implicit_self f!(x) f!!(x) f!&(x) f!^(x) f!@(x) f!?(x)";
        let tape = tokenize(code);
        assert!(tape.kinds.contains(&TokenKind::InfixDot));
        assert!(tape.kinds.contains(&TokenKind::PrefixDot));
        assert!(tape.kinds.contains(&TokenKind::CallQuoteShallow));
        assert!(tape.kinds.contains(&TokenKind::CallQuoteDeep));
        assert!(tape.kinds.contains(&TokenKind::CallQuoteAbi));
        assert!(tape.kinds.contains(&TokenKind::CallQuoteReact));
        assert!(tape.kinds.contains(&TokenKind::CallQuoteAdjoint));
        assert!(tape.kinds.contains(&TokenKind::CallQuoteEffect));
    }

    #[test]
    fn test_name_blind_dimensional_and_interpolated_flags() {
        let code = r#"let dt = 250ms; let bw = 64KB; let custom = 100mol; let s = f"hello {dt}";"#;
        let tape = tokenize(code);
        let mut dim_slices = Vec::new();
        let mut interp_slices = Vec::new();
        for i in 0..tape.len() {
            if tape.flag(i).contains(TokenFlags::IS_DIMENSIONAL) {
                dim_slices.push(tape.slice_str(i, code));
            }
            if tape.flag(i).contains(TokenFlags::IS_INTERPOLATED) {
                interp_slices.push(tape.slice_str(i, code));
            }
        }
        assert_eq!(dim_slices, vec!["250ms", "64KB", "100mol"]);
        assert_eq!(interp_slices, vec![r#"f"hello {dt}""#]);
    }
}
