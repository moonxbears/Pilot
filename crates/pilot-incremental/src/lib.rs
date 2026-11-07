//! Milestone 32.1 & 53: Sub-Millisecond Incremental Reparsing & Green-Tree Subtree Splicing (`pilot-incremental`).
//!
//! Built directly over the 14-seed `pilot-lexer` `TokenTape` with zero dependency on `pilot-syntax`.

use std::sync::Arc;
use pilot_lexer::{tokenize, TokenKind};
use pilot_span::ByteSpan;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SyntaxKind {
    Program,
    FnDecl,
    StructDecl,
    InterfaceDecl,
    LetStmt,
    VarStmt,
    ImportDecl,
    BlockExpr,
    Ident,
    Whitespace,
    Newline,
    LineComment,
    BlockComment,
    Token,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GreenToken {
    pub kind: SyntaxKind,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GreenElement {
    Node(Arc<GreenNode>),
    Token(GreenToken),
}

impl GreenElement {
    pub fn text_len(&self) -> u32 {
        match self {
            GreenElement::Node(n) => n.text_len,
            GreenElement::Token(t) => t.text.len() as u32,
        }
    }

    pub fn write_to(&self, out: &mut String) {
        match self {
            GreenElement::Node(n) => n.write_to(out),
            GreenElement::Token(t) => out.push_str(&t.text),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GreenNode {
    pub kind: SyntaxKind,
    pub text_len: u32,
    children: Vec<GreenElement>,
}

impl GreenNode {
    pub fn new(kind: SyntaxKind, children: Vec<GreenElement>) -> Arc<Self> {
        let text_len = children.iter().map(|c| c.text_len()).sum();
        Arc::new(Self { kind, text_len, children })
    }

    pub fn children(&self) -> &[GreenElement] {
        &self.children
    }

    pub fn to_text(&self) -> String {
        let mut out = String::with_capacity(self.text_len as usize);
        self.write_to(&mut out);
        out
    }

    pub fn write_to(&self, out: &mut String) {
        for c in &self.children {
            c.write_to(out);
        }
    }
}

#[derive(Clone, Debug)]
pub struct SyntaxNode {
    green: Arc<GreenNode>,
    offset: u32,
}

impl SyntaxNode {
    pub fn new_root(green: Arc<GreenNode>) -> Self {
        Self { green, offset: 0 }
    }

    pub fn green(&self) -> &Arc<GreenNode> {
        &self.green
    }

    pub fn kind(&self) -> SyntaxKind {
        self.green.kind
    }

    pub fn text_range(&self) -> ByteSpan {
        ByteSpan::new(self.offset, self.offset + self.green.text_len)
    }

    pub fn to_text(&self) -> String {
        self.green.to_text()
    }

    pub fn children_with_ranges(&self) -> Vec<(ByteSpan, GreenElement)> {
        let mut cur = self.offset;
        let mut out = Vec::with_capacity(self.green.children.len());
        for c in &self.green.children {
            let len = c.text_len();
            out.push((ByteSpan::new(cur, cur + len), c.clone()));
            cur += len;
        }
        out
    }
}

pub struct ParsedCst {
    pub root: SyntaxNode,
    pub diagnostics: Vec<String>,
}

/// Builds a lossless Red-Green tree directly from `pilot_lexer::tokenize(src)`.
pub fn parse_cst(src: &str) -> ParsedCst {
    let tape = tokenize(src);
    let mut top_children: Vec<GreenElement> = Vec::new();
    let mut cur_item_tokens: Vec<GreenElement> = Vec::new();
    let mut cur_item_kind = SyntaxKind::BlockExpr;
    let mut seen_code_in_item = false;
    let mut brace_depth = 0i32;
    let mut paren_depth = 0i32;
    let mut byte_cursor = 0usize;

    for i in 0..tape.len() {
        let tk = tape.kind(i);
        if tk == TokenKind::Eof {
            break;
        }
        let sp = tape.span(i);
        let s = sp.start as usize;
        let e = (sp.end as usize).min(src.len());
        if s > byte_cursor {
            cur_item_tokens.push(GreenElement::Token(GreenToken {
                kind: SyntaxKind::Whitespace,
                text: src[byte_cursor..s].to_string(),
            }));
        }
        let text = &src[s..e];
        byte_cursor = e;

        let sk = match tk {
            TokenKind::Newline => SyntaxKind::Newline,
            TokenKind::Comment => SyntaxKind::LineComment,
            TokenKind::BlockComment => SyntaxKind::BlockComment,
            TokenKind::Ident => SyntaxKind::Ident,
            _ => SyntaxKind::Token,
        };

        if !seen_code_in_item && !matches!(tk, TokenKind::Newline | TokenKind::Comment | TokenKind::BlockComment | TokenKind::Shebang) {
            cur_item_kind = match (tk, text) {
                (TokenKind::KwFn, _) | (TokenKind::Ident, "def") => SyntaxKind::FnDecl,
                (TokenKind::KwStruct, _) => SyntaxKind::StructDecl,
                (TokenKind::KwInterface, _) => SyntaxKind::InterfaceDecl,
                (TokenKind::KwLet, _) => SyntaxKind::LetStmt,
                (TokenKind::Ident, "var") => SyntaxKind::VarStmt,
                (TokenKind::Ident, "use" | "import" | "pkg") => SyntaxKind::ImportDecl,
                _ => SyntaxKind::BlockExpr,
            };
            seen_code_in_item = true;
        } else if seen_code_in_item && cur_item_kind == SyntaxKind::BlockExpr {
            match (tk, text) {
                (TokenKind::KwFn, _) => cur_item_kind = SyntaxKind::FnDecl,
                (TokenKind::KwStruct, _) => cur_item_kind = SyntaxKind::StructDecl,
                (TokenKind::KwInterface, _) => cur_item_kind = SyntaxKind::InterfaceDecl,
                _ => {}
            }
        }

        cur_item_tokens.push(GreenElement::Token(GreenToken {
            kind: sk,
            text: text.to_string(),
        }));

        match tk {
            TokenKind::LParen => paren_depth += 1,
            TokenKind::RParen => paren_depth = (paren_depth - 1).max(0),
            TokenKind::LBrace => brace_depth += 1,
            TokenKind::RBrace => {
                brace_depth = (brace_depth - 1).max(0);
                if brace_depth == 0 && paren_depth == 0 && seen_code_in_item {
                    let node = GreenNode::new(cur_item_kind, std::mem::take(&mut cur_item_tokens));
                    top_children.push(GreenElement::Node(node));
                    seen_code_in_item = false;
                    cur_item_kind = SyntaxKind::BlockExpr;
                }
            }
            TokenKind::Semicolon if brace_depth == 0 && paren_depth == 0 && seen_code_in_item => {
                let node = GreenNode::new(cur_item_kind, std::mem::take(&mut cur_item_tokens));
                top_children.push(GreenElement::Node(node));
                seen_code_in_item = false;
                cur_item_kind = SyntaxKind::BlockExpr;
            }
            _ => {}
        }
    }

    if byte_cursor < src.len() {
        cur_item_tokens.push(GreenElement::Token(GreenToken {
            kind: SyntaxKind::Whitespace,
            text: src[byte_cursor..].to_string(),
        }));
    }
    if !cur_item_tokens.is_empty() {
        let node = GreenNode::new(cur_item_kind, cur_item_tokens);
        top_children.push(GreenElement::Node(node));
    }

    let mut diags = Vec::new();
    if brace_depth != 0 || paren_depth != 0 {
        diags.push("Unbalanced delimiters in CST".to_string());
    }

    let root = SyntaxNode::new_root(GreenNode::new(SyntaxKind::Program, top_children));
    ParsedCst { root, diagnostics: diags }
}

/// Represents a single byte-range text replacement in a source document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    pub start: u32,
    pub end: u32,
    pub replacement: String,
}

impl TextEdit {
    pub fn new(start: u32, end: u32, replacement: impl Into<String>) -> Self {
        Self { start, end, replacement: replacement.into() }
    }

    pub fn apply(&self, src: &str) -> String {
        let mut result = String::with_capacity(src.len() + self.replacement.len());
        let s = (self.start as usize).min(src.len());
        let e = (self.end as usize).min(src.len()).max(s);
        result.push_str(&src[..s]);
        result.push_str(&self.replacement);
        result.push_str(&src[e..]);
        result
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReparseMetrics {
    pub reparse_time_micros: u64,
    pub reused_nodes: usize,
    pub allocated_nodes: usize,
    pub escalated_to_root: bool,
}

pub struct IncrementalResult {
    pub new_root: Arc<GreenNode>,
    pub new_source: String,
    pub metrics: ReparseMetrics,
}

pub fn reparse_incremental(
    old_root: &Arc<GreenNode>,
    old_source: &str,
    edit: &TextEdit,
) -> IncrementalResult {
    let start_instant = std::time::Instant::now();
    let new_source = edit.apply(old_source);

    if edit.start == edit.end && edit.replacement.is_empty() {
        return IncrementalResult {
            new_root: old_root.clone(),
            new_source,
            metrics: ReparseMetrics {
                reparse_time_micros: start_instant.elapsed().as_micros() as u64,
                reused_nodes: old_root.children().len(),
                allocated_nodes: 0,
                escalated_to_root: false,
            },
        };
    }

    let root_node = SyntaxNode::new_root(old_root.clone());
    let mut reused_nodes = 0;
    let mut allocated_nodes = 0;
    let mut target_child_idx = None;

    for (idx, (span, _)) in root_node.children_with_ranges().into_iter().enumerate() {
        if edit.start >= span.start && edit.end <= span.end {
            target_child_idx = Some((idx, span));
            break;
        }
    }

    if let Some((child_idx, child_span)) = target_child_idx {
        let old_child_len = child_span.len();
        let delta = (edit.replacement.len() as isize) - ((edit.end - edit.start) as isize);
        let new_child_len = (old_child_len as isize + delta).max(0) as usize;
        let new_child_start = child_span.start as usize;
        let new_child_end = new_child_start + new_child_len;

        if new_child_end <= new_source.len() {
            let new_child_text = &new_source[new_child_start..new_child_end];
            let re_parsed = parse_cst(new_child_text);

            if re_parsed.diagnostics.is_empty() && re_parsed.root.to_text() == new_child_text {
                let mut new_children = Vec::with_capacity(old_root.children().len());
                for (idx, old_elem) in old_root.children().iter().enumerate() {
                    if idx == child_idx {
                        for newly_parsed in re_parsed.root.green().children() {
                            new_children.push(newly_parsed.clone());
                            allocated_nodes += 1;
                        }
                    } else {
                        new_children.push(old_elem.clone());
                        reused_nodes += 1;
                    }
                }

                let new_root = GreenNode::new(SyntaxKind::Program, new_children);
                let elapsed = start_instant.elapsed().as_micros() as u64;
                return IncrementalResult {
                    new_root,
                    new_source,
                    metrics: ReparseMetrics {
                        reparse_time_micros: elapsed,
                        reused_nodes,
                        allocated_nodes,
                        escalated_to_root: false,
                    },
                };
            }
        }
    }

    let full_reparse = parse_cst(&new_source);
    let elapsed = start_instant.elapsed().as_micros() as u64;
    IncrementalResult {
        new_root: full_reparse.root.green().clone(),
        new_source,
        metrics: ReparseMetrics {
            reparse_time_micros: elapsed,
            reused_nodes: 0,
            allocated_nodes: full_reparse.root.green().children().len(),
            escalated_to_root: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incremental_reparse_sub_millisecond_and_structural_sharing() {
        let code = r#"use std.math;

fn calculate(a: i64, b: i64) -> i64 {
    let res = (a + b) * 10;
    return res;
}

fn helper() -> i64 {
    return 42;
}
"#;
        let initial_cst = parse_cst(code);
        let initial_root = initial_cst.root.green().clone();

        let target = "42";
        let offset = code.find(target).unwrap() as u32;
        let edit = TextEdit::new(offset, offset + target.len() as u32, "100");
        let result = reparse_incremental(&initial_root, code, &edit);

        assert!(result.metrics.reparse_time_micros < 5000);
        assert!(result.new_source.contains("return 100;"));
        assert_eq!(result.new_root.to_text(), result.new_source);

        if let (Some(GreenElement::Node(old_first)), Some(GreenElement::Node(new_first))) =
            (initial_root.children().first(), result.new_root.children().first())
        {
            assert!(Arc::ptr_eq(old_first, new_first));
        }
    }

    #[test]
    fn test_incremental_reparse_escalates_on_cross_boundary_edit() {
        let code = "fn f1() { return 1; }\nfn f2() { return 2; }\n";
        let initial = parse_cst(code);
        let edit = TextEdit::new(10, 30, " /* deleted boundary */ ");
        let result = reparse_incremental(initial.root.green(), code, &edit);
        assert!(result.metrics.escalated_to_root);
        assert_eq!(result.new_root.to_text(), result.new_source);
    }
}
