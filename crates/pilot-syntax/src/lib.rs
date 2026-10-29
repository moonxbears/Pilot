//! Milestone 31: Lossless Red-Green Concrete Syntax Tree (CST), TreeSink, and Resilient Parsing (`pilot-syntax`).
//!
//! Provides:
//! - Immutable width-relative `GreenNode` / `GreenToken` with global interning.
//! - Lazy `SyntaxNode` Red cursor façade with on-demand absolute span calculation.
//! - Lossless round-trip byte-for-byte fidelity (`cst.to_text() == src`).
//! - Resilient panic-free error recovery with `ErrorNode`, token sets, and Damerau-Levenshtein typo fix-it hints.
//! - IDE completion synthesis (`MissingIdent` on `user.`, `MissingRParen` on `if (x > 0 {`).

pub mod green;
pub mod kind;
pub mod parser;
pub mod recovery;
pub mod red;
pub mod sink;

pub use green::{GreenElement, GreenNode, GreenToken};
pub use kind::SyntaxKind;
pub use parser::{parse_cst, scan_raw_tokens, ParsedCst, RawToken};
pub use recovery::{damerau_levenshtein, find_keyword_typo_candidate, RecoverySets, SyntaxDiagnostic, TokenSet};
pub use red::{SyntaxElement, SyntaxNode, SyntaxToken};
pub use sink::{LosslessCstSink, TreeSink};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lossless_round_trip_invariant() {
        let code = r#"// Leading module comment
#region Initialization
import std.math;

/* Block comment */
def calculate(a: i64, b: i64) -> i64 {
    // Inner computation
    let res = (a + b) * 10;
    return res;
}
#endregion
"#;
        let cst = parse_cst(code);
        let reconstructed = cst.to_text();
        assert_eq!(reconstructed, code, "Round-trip must match byte-for-byte!");
        assert!(cst.diagnostics.is_empty(), "Valid code should have no diagnostics");
    }

    #[test]
    fn test_colon_suite_round_trip() {
        let code = "def greet(name: str):\n    let msg = \"Hello \" + name;\n    return msg;\n";
        let cst = parse_cst(code);
        assert_eq!(cst.to_text(), code);
        assert!(cst.diagnostics.is_empty());
    }

    #[test]
    fn test_missing_ident_synthesis_on_trailing_dot() {
        let code = "let x = user.\nreturn x;\n";
        let cst = parse_cst(code);
        // The original code is STILL preserved 100% losslessly
        assert_eq!(cst.to_text(), code);
        // But a diagnostic for missing identifier was recorded
        assert!(!cst.diagnostics.is_empty());
        let diag = &cst.diagnostics[0];
        assert!(diag.message.contains("identifier"));

        // Verify that the CST tree contains a MissingIdent token
        let has_missing_ident = cst.root.descendants().any(|elem| elem.kind() == SyntaxKind::MissingIdent);
        assert!(has_missing_ident, "CST should contain synthesized MissingIdent for IDE dot-completion");
    }

    #[test]
    fn test_missing_rparen_synthesis_on_unclosed_if() {
        let code = "if (x > 0 {\n    return 1;\n}\n";
        let cst = parse_cst(code);
        assert_eq!(cst.to_text(), code);
        assert!(!cst.diagnostics.is_empty());
        let diag = &cst.diagnostics[0];
        assert!(diag.message.contains("')'"));
        assert_eq!(diag.suggested_fix.as_deref(), Some(")"));

        let has_missing_rparen = cst.root.descendants().any(|elem| elem.kind() == SyntaxKind::MissingRParen);
        assert!(has_missing_rparen, "CST should contain synthesized MissingRParen");
    }

    #[test]
    fn test_keyword_typo_fix_it() {
        let code = "gurd x > 0 else {\n    return 0;\n}\n";
        let cst = parse_cst(code);
        assert_eq!(cst.to_text(), code);
        assert!(!cst.diagnostics.is_empty());
        let diag = &cst.diagnostics[0];
        assert_eq!(diag.suggested_fix.as_deref(), Some("guard"));
    }

    #[test]
    fn test_token_interning_zero_alloc() {
        let tok1 = GreenToken::new(SyntaxKind::KwLet, "let");
        let tok2 = GreenToken::new(SyntaxKind::KwLet, "let");
        assert_eq!(tok1, tok2);
        // Both point to identical Arc address
        assert!(std::sync::Arc::ptr_eq(tok1.text_arc(), tok2.text_arc()));
    }
}
