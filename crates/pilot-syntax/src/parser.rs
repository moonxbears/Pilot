//! Milestone 31.1 & 31.2: Resilient Lossless CST Parser, Trivia Attachment, and Error Recovery.

use pilot_lexer::keywords::lookup_keyword;
use pilot_span::ByteSpan;
use crate::kind::SyntaxKind;
use crate::recovery::{find_keyword_typo_candidate, RecoverySets, SyntaxDiagnostic};
use crate::red::SyntaxNode;
use crate::sink::{LosslessCstSink, TreeSink};

/// A raw scanned token covering exactly [span.start..span.end] in the source.
#[derive(Clone, Debug)]
pub struct RawToken<'a> {
    pub kind: SyntaxKind,
    pub text: &'a str,
    pub span: ByteSpan,
}

/// Scans raw source string into a continuous, gap-free stream of tokens covering 100% of bytes.
pub fn scan_raw_tokens<'a>(src: &'a str) -> Vec<RawToken<'a>> {
    let bytes = src.as_bytes();
    let mut tokens = Vec::with_capacity(src.len() / 4 + 8);
    let mut cur = 0;

    while cur < bytes.len() {
        let start = cur;
        let b = bytes[cur];

        // 1. Whitespace (spaces and tabs)
        if b == b' ' || b == b'\t' {
            while cur < bytes.len() && (bytes[cur] == b' ' || bytes[cur] == b'\t') {
                cur += 1;
            }
            tokens.push(RawToken {
                kind: SyntaxKind::Whitespace,
                text: &src[start..cur],
                span: ByteSpan::new(start as u32, cur as u32),
            });
            continue;
        }

        // 2. Newlines (\r\n or \n)
        if b == b'\r' || b == b'\n' {
            if b == b'\r' && cur + 1 < bytes.len() && bytes[cur + 1] == b'\n' {
                cur += 2;
            } else {
                cur += 1;
            }
            tokens.push(RawToken {
                kind: SyntaxKind::Newline,
                text: &src[start..cur],
                span: ByteSpan::new(start as u32, cur as u32),
            });
            continue;
        }

        // 3. Comments & Preprocessor Directives (#region, #endregion, #, //, /* ... */)
        if b == b'#' {
            // Check for #region / #endregion
            let rest = &src[cur..];
            if rest.starts_with("#region") {
                let end = scan_to_newline(bytes, cur);
                tokens.push(RawToken {
                    kind: SyntaxKind::RegionStart,
                    text: &src[start..end],
                    span: ByteSpan::new(start as u32, end as u32),
                });
                cur = end;
                continue;
            } else if rest.starts_with("#endregion") {
                let end = scan_to_newline(bytes, cur);
                tokens.push(RawToken {
                    kind: SyntaxKind::RegionEnd,
                    text: &src[start..end],
                    span: ByteSpan::new(start as u32, end as u32),
                });
                cur = end;
                continue;
            }

            let end = scan_to_newline(bytes, cur);
            tokens.push(RawToken {
                kind: SyntaxKind::LineComment,
                text: &src[start..end],
                span: ByteSpan::new(start as u32, end as u32),
            });
            cur = end;
            continue;
        }

        if b == b'/' && cur + 1 < bytes.len() {
            if bytes[cur + 1] == b'/' {
                let end = scan_to_newline(bytes, cur);
                tokens.push(RawToken {
                    kind: SyntaxKind::LineComment,
                    text: &src[start..end],
                    span: ByteSpan::new(start as u32, end as u32),
                });
                cur = end;
                continue;
            } else if bytes[cur + 1] == b'*' {
                let end = scan_to_block_comment_end(bytes, cur + 2);
                tokens.push(RawToken {
                    kind: SyntaxKind::BlockComment,
                    text: &src[start..end],
                    span: ByteSpan::new(start as u32, end as u32),
                });
                cur = end;
                continue;
            }
        }

        // 4. Raw DSL Blocks: ```lang ... ```
        if b == b'`' && cur + 2 < bytes.len() && bytes[cur + 1] == b'`' && bytes[cur + 2] == b'`' {
            cur += 3;
            while cur + 2 < bytes.len() {
                if bytes[cur] == b'`' && bytes[cur + 1] == b'`' && bytes[cur + 2] == b'`' {
                    cur += 3;
                    break;
                }
                cur += 1;
            }
            tokens.push(RawToken {
                kind: SyntaxKind::RawDslBlock,
                text: &src[start..cur],
                span: ByteSpan::new(start as u32, cur as u32),
            });
            continue;
        }

        // 5. Triple Strings (""" or ''')
        if (b == b'"' || b == b'\'') && cur + 2 < bytes.len() && bytes[cur + 1] == b && bytes[cur + 2] == b {
            cur += 3;
            while cur + 2 < bytes.len() {
                if bytes[cur] == b && bytes[cur + 1] == b && bytes[cur + 2] == b {
                    cur += 3;
                    break;
                }
                cur += 1;
            }
            tokens.push(RawToken {
                kind: SyntaxKind::TripleStrLit,
                text: &src[start..cur],
                span: ByteSpan::new(start as u32, cur as u32),
            });
            continue;
        }

        // 6. Regular Strings: "..." or '...'
        if b == b'"' || b == b'\'' {
            cur += 1;
            let mut escaped = false;
            while cur < bytes.len() {
                let c = bytes[cur];
                if escaped {
                    escaped = false;
                    cur += 1;
                } else if c == b'\\' {
                    escaped = true;
                    cur += 1;
                } else if c == b {
                    cur += 1;
                    break;
                } else if c == b'\n' || c == b'\r' {
                    break;
                } else {
                    cur += 1;
                }
            }
            tokens.push(RawToken {
                kind: SyntaxKind::StrLit,
                text: &src[start..cur],
                span: ByteSpan::new(start as u32, cur as u32),
            });
            continue;
        }

        // 7. Numbers (Int, Hex, Bin, Oct, Float)
        if b >= b'0' && b <= b'9' {
            let mut is_float = false;
            cur += 1;
            if cur < bytes.len() && (bytes[cur] == b'x' || bytes[cur] == b'X' || bytes[cur] == b'b' || bytes[cur] == b'B' || bytes[cur] == b'o' || bytes[cur] == b'O') {
                cur += 1;
                while cur < bytes.len() && (bytes[cur].is_ascii_alphanumeric() || bytes[cur] == b'_') {
                    cur += 1;
                }
            } else {
                while cur < bytes.len() {
                    let c = bytes[cur];
                    if c >= b'0' && c <= b'9' || c == b'_' {
                        cur += 1;
                    } else if c == b'.' && !is_float && cur + 1 < bytes.len() && bytes[cur + 1] >= b'0' && bytes[cur + 1] <= b'9' {
                        is_float = true;
                        cur += 2;
                    } else {
                        break;
                    }
                }
            }
            let kind = if is_float { SyntaxKind::FloatLit } else { SyntaxKind::IntLit };
            tokens.push(RawToken {
                kind,
                text: &src[start..cur],
                span: ByteSpan::new(start as u32, cur as u32),
            });
            continue;
        }

        // 8. Identifiers and Keywords
        if b.is_ascii_alphabetic() || b == b'_' {
            cur += 1;
            while cur < bytes.len() && (bytes[cur].is_ascii_alphanumeric() || bytes[cur] == b'_') {
                cur += 1;
            }
            let ident = &src[start..cur];
            let kind = if let Some((kw, _)) = lookup_keyword(ident) {
                SyntaxKind::from_token_kind(kw)
            } else {
                SyntaxKind::Ident
            };
            tokens.push(RawToken {
                kind,
                text: ident,
                span: ByteSpan::new(start as u32, cur as u32),
            });
            continue;
        }

        // 9. Call Quotes and Multi-character Operators
        let (kind, len) = match b {
            b'!' => {
                if cur + 2 < bytes.len() && bytes[cur + 1] == b'!' && bytes[cur + 2] == b'(' {
                    (SyntaxKind::CallQuoteDeep, 2)
                } else if cur + 2 < bytes.len() && bytes[cur + 1] == b'&' && bytes[cur + 2] == b'(' {
                    (SyntaxKind::CallQuoteAbi, 2)
                } else if cur + 2 < bytes.len() && bytes[cur + 1] == b'^' && bytes[cur + 2] == b'(' {
                    (SyntaxKind::CallQuoteReact, 2)
                } else if cur + 2 < bytes.len() && bytes[cur + 1] == b'@' && bytes[cur + 2] == b'(' {
                    (SyntaxKind::CallQuoteAdjoint, 2)
                } else if cur + 2 < bytes.len() && bytes[cur + 1] == b'?' && bytes[cur + 2] == b'(' {
                    (SyntaxKind::CallQuoteEffect, 2)
                } else if cur + 1 < bytes.len() && bytes[cur + 1] == b'(' {
                    (SyntaxKind::CallQuoteShallow, 1)
                } else if cur + 1 < bytes.len() && bytes[cur + 1] == b'=' {
                    (SyntaxKind::NotEqual, 2)
                } else {
                    (SyntaxKind::Exclamation, 1)
                }
            }
            b'=' => {
                if cur + 1 < bytes.len() && bytes[cur + 1] == b'=' {
                    (SyntaxKind::EqEqual, 2)
                } else if cur + 1 < bytes.len() && bytes[cur + 1] == b'>' {
                    (SyntaxKind::FatArrow, 2)
                } else {
                    (SyntaxKind::Equal, 1)
                }
            }
            b'<' => {
                if cur + 1 < bytes.len() && bytes[cur + 1] == b'=' {
                    (SyntaxKind::LessEqual, 2)
                } else if cur + 1 < bytes.len() && bytes[cur + 1] == b'-' {
                    (SyntaxKind::ArrowLeft, 2)
                } else if cur + 1 < bytes.len() && bytes[cur + 1] == b'|' {
                    (SyntaxKind::PipeBackward, 2)
                } else {
                    (SyntaxKind::Less, 1)
                }
            }
            b'>' => {
                if cur + 1 < bytes.len() && bytes[cur + 1] == b'=' {
                    (SyntaxKind::GreaterEqual, 2)
                } else {
                    (SyntaxKind::Greater, 1)
                }
            }
            b'-' => {
                if cur + 1 < bytes.len() && bytes[cur + 1] == b'>' {
                    (SyntaxKind::Arrow, 2)
                } else {
                    (SyntaxKind::Minus, 1)
                }
            }
            b'|' => {
                if cur + 1 < bytes.len() && bytes[cur + 1] == b'>' {
                    (SyntaxKind::PipeForward, 2)
                } else {
                    (SyntaxKind::Vbar, 1)
                }
            }
            b'.' => {
                if cur + 2 < bytes.len() && bytes[cur + 1] == b'.' && bytes[cur + 2] == b'=' {
                    (SyntaxKind::DotDotEqual, 3)
                } else if cur + 2 < bytes.len() && bytes[cur + 1] == b'.' && bytes[cur + 2] == b'<' {
                    (SyntaxKind::DotDotLess, 3)
                } else if cur + 1 < bytes.len() && bytes[cur + 1] == b'.' {
                    (SyntaxKind::DotDot, 2)
                } else {
                    (SyntaxKind::Dot, 1)
                }
            }
            b'?' => {
                if cur + 1 < bytes.len() && bytes[cur + 1] == b'?' {
                    (SyntaxKind::Coalesce, 2)
                } else if cur + 1 < bytes.len() && bytes[cur + 1] == b'.' {
                    (SyntaxKind::SafeNav, 2)
                } else {
                    (SyntaxKind::Question, 1)
                }
            }
            b':' => {
                if cur + 1 < bytes.len() && bytes[cur + 1] == b'=' {
                    (SyntaxKind::Walrus, 2)
                } else {
                    (SyntaxKind::Colon, 1)
                }
            }
            b'*' => {
                if cur + 1 < bytes.len() && bytes[cur + 1] == b'*' {
                    (SyntaxKind::DoubleStar, 2)
                } else {
                    (SyntaxKind::Star, 1)
                }
            }
            b'/' => {
                if cur + 1 < bytes.len() && bytes[cur + 1] == b'/' {
                    (SyntaxKind::DoubleSlash, 2)
                } else {
                    (SyntaxKind::Slash, 1)
                }
            }
            b'+' => (SyntaxKind::Plus, 1),
            b'%' => (SyntaxKind::Percent, 1),
            b'&' => (SyntaxKind::Ampersand, 1),
            b'^' => (SyntaxKind::Caret, 1),
            b'~' => (SyntaxKind::Tilde, 1),
            b'$' => (SyntaxKind::Dollar, 1),
            b'@' => (SyntaxKind::At, 1),
            b'(' => (SyntaxKind::LParen, 1),
            b')' => (SyntaxKind::RParen, 1),
            b'{' => (SyntaxKind::LBrace, 1),
            b'}' => (SyntaxKind::RBrace, 1),
            b'[' => (SyntaxKind::LBracket, 1),
            b']' => (SyntaxKind::RBracket, 1),
            b';' => (SyntaxKind::Semicolon, 1),
            b',' => (SyntaxKind::Comma, 1),
            _ => (SyntaxKind::ErrorToken, 1),
        };

        cur += len;
        tokens.push(RawToken {
            kind,
            text: &src[start..cur],
            span: ByteSpan::new(start as u32, cur as u32),
        });
    }

    tokens
}

fn scan_to_newline(bytes: &[u8], mut cur: usize) -> usize {
    while cur < bytes.len() && bytes[cur] != b'\n' && bytes[cur] != b'\r' {
        cur += 1;
    }
    cur
}

fn scan_to_block_comment_end(bytes: &[u8], mut cur: usize) -> usize {
    while cur + 1 < bytes.len() {
        if bytes[cur] == b'*' && bytes[cur + 1] == b'/' {
            return cur + 2;
        }
        cur += 1;
    }
    bytes.len()
}

/// The result of parsing raw source code into a Concrete Syntax Tree.
pub struct ParsedCst {
    pub root: SyntaxNode,
    pub diagnostics: Vec<SyntaxDiagnostic>,
}

impl ParsedCst {
    #[inline]
    pub fn to_text(&self) -> String {
        self.root.to_text()
    }
}

/// Parses raw Pilot source code into a lossless Red-Green Concrete Syntax Tree.
pub fn parse_cst(src: &str) -> ParsedCst {
    let tokens = scan_raw_tokens(src);
    let mut parser = CstParser::new(&tokens);
    parser.parse_program()
}

struct CstParser<'a, 't> {
    tokens: &'t [RawToken<'a>],
    pos: usize,
    sink: LosslessCstSink,
    error_cooldown: bool,
}

impl<'a, 't> CstParser<'a, 't> {
    fn new(tokens: &'t [RawToken<'a>]) -> Self {
        Self {
            tokens,
            pos: 0,
            sink: LosslessCstSink::new(),
            error_cooldown: false,
        }
    }

    #[inline]
    fn at_end(&self) -> bool {
        self.skip_trivia_idx(self.pos) >= self.tokens.len()
    }

    fn skip_trivia_idx(&self, mut idx: usize) -> usize {
        while idx < self.tokens.len() && self.tokens[idx].kind.is_trivia() {
            idx += 1;
        }
        idx
    }

    fn peek(&self) -> SyntaxKind {
        let idx = self.skip_trivia_idx(self.pos);
        if idx < self.tokens.len() {
            self.tokens[idx].kind
        } else {
            SyntaxKind::Eof
        }
    }

    fn peek_text(&self) -> &'a str {
        let idx = self.skip_trivia_idx(self.pos);
        if idx < self.tokens.len() {
            self.tokens[idx].text
        } else {
            ""
        }
    }

    fn peek_span(&self) -> ByteSpan {
        let idx = self.skip_trivia_idx(self.pos);
        if idx < self.tokens.len() {
            self.tokens[idx].span
        } else if let Some(last) = self.tokens.last() {
            ByteSpan::new(last.span.end, last.span.end)
        } else {
            ByteSpan::new(0, 0)
        }
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.peek() == kind
    }

    fn emit_trivia(&mut self) {
        while self.pos < self.tokens.len() && self.tokens[self.pos].kind.is_trivia() {
            let tok = &self.tokens[self.pos];
            self.sink.token(tok.kind, tok.text);
            self.pos += 1;
        }
    }

    fn advance(&mut self) {
        self.emit_trivia();
        if self.pos < self.tokens.len() {
            let tok = &self.tokens[self.pos];
            self.sink.token(tok.kind, tok.text);
            self.pos += 1;
            self.error_cooldown = false;
        }
        self.emit_trivia();
    }

    fn parse_program(&mut self) -> ParsedCst {
        self.sink.start_node(SyntaxKind::Program);
        self.emit_trivia();

        while !self.at_end() {
            let start_pos = self.pos;
            self.parse_stmt_or_decl();

            if self.pos == start_pos {
                // Recover from unexpected token
                self.recover_unexpected();
            }
            self.emit_trivia();
        }

        self.emit_trivia();
        self.sink.finish_node(); // Close Program

        let sink = std::mem::replace(&mut self.sink, LosslessCstSink::new());
        let (root, diagnostics) = sink.finish();
        ParsedCst { root, diagnostics }
    }

    fn recover_unexpected(&mut self) {
        self.sink.start_node(SyntaxKind::ErrorNode);
        let span = self.peek_span();
        let txt = self.peek_text();

        if !self.error_cooldown {
            if let Some(fix) = find_keyword_typo_candidate(txt) {
                self.sink.error(SyntaxDiagnostic::with_fix(
                    format!("Unrecognized keyword '{}', did you mean '{}'?", txt, fix),
                    span,
                    fix,
                ));
            } else {
                self.sink.error(SyntaxDiagnostic::new(
                    format!("Unexpected token '{}'", txt),
                    span,
                ));
            }
            self.error_cooldown = true;
        }

        self.advance();
        let recovery = RecoverySets::stmt_recovery();
        while !self.at_end() && !recovery.contains(&self.peek()) {
            self.advance();
        }
        self.sink.finish_node();
    }

    fn parse_stmt_or_decl(&mut self) {
        let k = self.peek();
        match k {
            SyntaxKind::KwImport => self.parse_import_decl(),
            SyntaxKind::KwFrom => self.parse_from_import_decl(),
            SyntaxKind::KwDef | SyntaxKind::KwFn => self.parse_fn_decl(),
            SyntaxKind::KwClass => self.parse_class_decl(),
            SyntaxKind::KwStruct => self.parse_struct_decl(),
            SyntaxKind::KwRecord => self.parse_record_decl(),
            SyntaxKind::KwInterface => self.parse_interface_decl(),
            SyntaxKind::KwExtend => self.parse_extend_decl(),
            SyntaxKind::KwType => self.parse_type_adt_decl(),
            SyntaxKind::KwLet => self.parse_binding_stmt(SyntaxKind::LetStmt),
            SyntaxKind::KwVar => self.parse_binding_stmt(SyntaxKind::VarStmt),
            SyntaxKind::KwConst => self.parse_binding_stmt(SyntaxKind::ConstDecl),
            SyntaxKind::KwReturn => self.parse_return_stmt(),
            SyntaxKind::KwYield => self.parse_yield_stmt(),
            SyntaxKind::KwBreak => self.parse_break_stmt(),
            SyntaxKind::KwContinue => self.parse_continue_stmt(),
            SyntaxKind::KwGuard => self.parse_guard_stmt(),
            SyntaxKind::KwDefer => self.parse_defer_stmt(),
            SyntaxKind::KwWhile => self.parse_while_stmt(),
            SyntaxKind::KwFor => self.parse_for_stmt(),
            SyntaxKind::Ident => {
                let txt = self.peek_text();
                if let Some(candidate) = find_keyword_typo_candidate(txt) {
                    if matches!(candidate, "guard" | "while" | "return" | "yield" | "break" | "continue" | "import" | "from") {
                        let next_idx = self.skip_trivia_idx(self.skip_trivia_idx(self.pos) + 1);
                        if next_idx < self.tokens.len() {
                            let next_k = self.tokens[next_idx].kind;
                            if !matches!(next_k, SyntaxKind::Equal | SyntaxKind::Dot | SyntaxKind::LParen | SyntaxKind::LBracket | SyntaxKind::Question | SyntaxKind::Semicolon) {
                                self.sink.start_node(SyntaxKind::ErrorNode);
                                self.sink.error(SyntaxDiagnostic::with_fix(
                                    format!("Unrecognized keyword '{}', did you mean '{}'?", txt, candidate),
                                    self.peek_span(),
                                    candidate,
                                ));
                                self.advance();
                                self.sink.finish_node();
                                return;
                            }
                        }
                    }
                }
                self.parse_expr_stmt();
            }
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_import_decl(&mut self) {
        self.sink.start_node(SyntaxKind::ImportDecl);
        self.advance(); // 'import'
        while !self.at_end() && !self.at(SyntaxKind::Semicolon) && !self.at(SyntaxKind::Newline) {
            self.advance();
        }
        if self.at(SyntaxKind::Semicolon) {
            self.advance();
        }
        self.sink.finish_node();
    }

    fn parse_from_import_decl(&mut self) {
        self.sink.start_node(SyntaxKind::FromImportDecl);
        self.advance(); // 'from'
        while !self.at_end() && !self.at(SyntaxKind::Semicolon) && !self.at(SyntaxKind::Newline) {
            self.advance();
        }
        if self.at(SyntaxKind::Semicolon) {
            self.advance();
        }
        self.sink.finish_node();
    }

    fn parse_fn_decl(&mut self) {
        self.sink.start_node(SyntaxKind::FnDecl);
        self.advance(); // 'def' or 'fn'

        if self.at(SyntaxKind::Ident) {
            self.advance(); // function name
        } else {
            self.sink.token(SyntaxKind::MissingIdent, "");
            self.sink.error(SyntaxDiagnostic::new("Expected function name", self.peek_span()));
        }

        // Parameters: ( ... )
        if self.at(SyntaxKind::LParen) {
            self.sink.start_node(SyntaxKind::ParamList);
            self.advance(); // '('
            while !self.at_end() && !self.at(SyntaxKind::RParen) && !self.at(SyntaxKind::LBrace) {
                self.advance();
            }
            if self.at(SyntaxKind::RParen) {
                self.advance();
            } else {
                self.sink.token(SyntaxKind::MissingRParen, "");
                self.sink.error(SyntaxDiagnostic::with_fix("Expected ')' closing parameter list", self.peek_span(), ")"));
            }
            self.sink.finish_node(); // ParamList
        }

        // Optional return type: -> Type
        if self.at(SyntaxKind::Arrow) {
            self.advance();
            while !self.at_end() && !self.at(SyntaxKind::LBrace) && !self.at(SyntaxKind::Colon) && !self.at(SyntaxKind::Semicolon) && !self.at(SyntaxKind::Newline) {
                self.advance();
            }
        }

        // Body: block ({ ... } or : ...)
        self.parse_block_or_colon_suite();
        self.sink.finish_node(); // FnDecl
    }

    fn parse_class_decl(&mut self) {
        self.sink.start_node(SyntaxKind::ClassDecl);
        self.advance(); // 'class'
        if self.at(SyntaxKind::Ident) {
            self.advance();
        }
        self.parse_block_or_colon_suite();
        self.sink.finish_node();
    }

    fn parse_struct_decl(&mut self) {
        self.sink.start_node(SyntaxKind::StructDecl);
        self.advance(); // 'struct'
        if self.at(SyntaxKind::Ident) {
            self.advance();
        }
        self.parse_block_or_colon_suite();
        self.sink.finish_node();
    }

    fn parse_record_decl(&mut self) {
        self.sink.start_node(SyntaxKind::RecordDecl);
        self.advance(); // 'record'
        if self.at(SyntaxKind::Ident) {
            self.advance();
        }
        self.parse_block_or_colon_suite();
        self.sink.finish_node();
    }

    fn parse_interface_decl(&mut self) {
        self.sink.start_node(SyntaxKind::InterfaceDecl);
        self.advance(); // 'interface'
        if self.at(SyntaxKind::Ident) {
            self.advance();
        }
        self.parse_block_or_colon_suite();
        self.sink.finish_node();
    }

    fn parse_extend_decl(&mut self) {
        self.sink.start_node(SyntaxKind::ExtendDecl);
        self.advance(); // 'extend'
        if self.at(SyntaxKind::Ident) {
            self.advance();
        }
        self.parse_block_or_colon_suite();
        self.sink.finish_node();
    }

    fn parse_type_adt_decl(&mut self) {
        self.sink.start_node(SyntaxKind::TypeAdtDecl);
        self.advance(); // 'type'
        if self.at(SyntaxKind::Ident) {
            self.advance();
        }
        if self.at(SyntaxKind::Equal) {
            self.advance();
            while !self.at_end() && !self.at(SyntaxKind::Semicolon) && !self.at(SyntaxKind::Newline) {
                self.advance();
            }
        }
        if self.at(SyntaxKind::Semicolon) {
            self.advance();
        }
        self.sink.finish_node();
    }

    fn parse_binding_stmt(&mut self, node_kind: SyntaxKind) {
        self.sink.start_node(node_kind);
        self.advance(); // 'let', 'var', or 'const'

        if self.at(SyntaxKind::Ident) {
            self.advance();
        } else {
            self.sink.token(SyntaxKind::MissingIdent, "");
            self.sink.error(SyntaxDiagnostic::new("Expected variable identifier", self.peek_span()));
        }

        // Type annotation: : Type
        if self.at(SyntaxKind::Colon) {
            self.advance();
            while !self.at_end() && !self.at(SyntaxKind::Equal) && !self.at(SyntaxKind::Semicolon) && !self.at(SyntaxKind::Newline) {
                self.advance();
            }
        }

        if self.at(SyntaxKind::Equal) {
            self.advance();
            self.parse_expr();
        }

        if self.at(SyntaxKind::Semicolon) {
            self.advance();
        }
        self.sink.finish_node();
    }

    fn parse_return_stmt(&mut self) {
        self.sink.start_node(SyntaxKind::ReturnStmt);
        self.advance(); // 'return'
        if !self.at(SyntaxKind::Semicolon) && !self.at(SyntaxKind::Newline) && !self.at_end() {
            self.parse_expr();
        }
        if self.at(SyntaxKind::Semicolon) {
            self.advance();
        }
        self.sink.finish_node();
    }

    fn parse_yield_stmt(&mut self) {
        self.sink.start_node(SyntaxKind::YieldStmt);
        self.advance(); // 'yield'
        if !self.at(SyntaxKind::Semicolon) && !self.at(SyntaxKind::Newline) && !self.at_end() {
            self.parse_expr();
        }
        if self.at(SyntaxKind::Semicolon) {
            self.advance();
        }
        self.sink.finish_node();
    }

    fn parse_break_stmt(&mut self) {
        self.sink.start_node(SyntaxKind::BreakStmt);
        self.advance();
        if self.at(SyntaxKind::Semicolon) {
            self.advance();
        }
        self.sink.finish_node();
    }

    fn parse_continue_stmt(&mut self) {
        self.sink.start_node(SyntaxKind::ContinueStmt);
        self.advance();
        if self.at(SyntaxKind::Semicolon) {
            self.advance();
        }
        self.sink.finish_node();
    }

    fn parse_guard_stmt(&mut self) {
        self.sink.start_node(SyntaxKind::GuardStmt);
        self.advance(); // 'guard'
        self.parse_expr();
        if self.at(SyntaxKind::KwElse) {
            self.advance();
            self.parse_block_or_colon_suite();
        }
        self.sink.finish_node();
    }

    fn parse_defer_stmt(&mut self) {
        self.sink.start_node(SyntaxKind::DeferStmt);
        self.advance(); // 'defer'
        self.parse_block_or_colon_suite();
        self.sink.finish_node();
    }

    fn parse_while_stmt(&mut self) {
        self.sink.start_node(SyntaxKind::WhileStmt);
        self.advance(); // 'while'
        self.parse_expr();
        self.parse_block_or_colon_suite();
        self.sink.finish_node();
    }

    fn parse_for_stmt(&mut self) {
        self.sink.start_node(SyntaxKind::ForStmt);
        self.advance(); // 'for'
        if self.at(SyntaxKind::Ident) {
            self.advance();
        }
        if self.at(SyntaxKind::KwIn) {
            self.advance();
        }
        self.parse_expr();
        self.parse_block_or_colon_suite();
        self.sink.finish_node();
    }

    fn parse_expr_stmt(&mut self) {
        self.sink.start_node(SyntaxKind::ExprStmt);
        self.parse_expr();
        if self.at(SyntaxKind::Semicolon) {
            self.advance();
        }
        self.sink.finish_node();
    }

    fn parse_block_or_colon_suite(&mut self) {
        if self.at(SyntaxKind::LBrace) {
            self.sink.start_node(SyntaxKind::BlockExpr);
            self.advance(); // '{'
            while !self.at_end() && !self.at(SyntaxKind::RBrace) {
                let start_pos = self.pos;
                self.parse_stmt_or_decl();
                if self.pos == start_pos {
                    self.recover_unexpected();
                }
            }
            if self.at(SyntaxKind::RBrace) {
                self.advance();
            } else {
                self.sink.token(SyntaxKind::MissingRBrace, "");
                self.sink.error(SyntaxDiagnostic::with_fix("Expected '}' closing block", self.peek_span(), "}"));
            }
            self.sink.finish_node();
        } else if self.at(SyntaxKind::Colon) {
            self.sink.start_node(SyntaxKind::BlockExpr);
            self.advance(); // ':'
            // In a colon suite, statements follow until dedent or next top-level item
            while !self.at_end() && !self.at(SyntaxKind::RBrace) && !self.at(SyntaxKind::VirtualRBrace) {
                let k = self.peek();
                if matches!(k, SyntaxKind::KwDef | SyntaxKind::KwFn | SyntaxKind::KwClass | SyntaxKind::KwStruct | SyntaxKind::KwRecord | SyntaxKind::KwInterface | SyntaxKind::KwExtend) {
                    break;
                }
                let start_pos = self.pos;
                self.parse_stmt_or_decl();
                if self.pos == start_pos {
                    break;
                }
            }
            self.sink.finish_node();
        }
    }

    pub fn parse_expr(&mut self) {
        self.parse_binary_expr(0);
    }

    fn parse_binary_expr(&mut self, min_prec: u8) {
        self.parse_unary_or_primary();

        while !self.at_end() {
            let k = self.peek();
            let prec = match k {
                SyntaxKind::Plus | SyntaxKind::Minus => 10,
                SyntaxKind::Star | SyntaxKind::Slash | SyntaxKind::Percent | SyntaxKind::DoubleSlash => 20,
                SyntaxKind::DoubleStar => 25,
                SyntaxKind::EqEqual | SyntaxKind::NotEqual | SyntaxKind::Less | SyntaxKind::LessEqual | SyntaxKind::Greater | SyntaxKind::GreaterEqual => 5,
                SyntaxKind::KwAnd | SyntaxKind::Ampersand => 4,
                SyntaxKind::KwOr | SyntaxKind::Vbar => 3,
                SyntaxKind::PipeForward => 2,
                SyntaxKind::Coalesce => 1,
                _ => break,
            };

            if prec < min_prec {
                break;
            }

            self.advance(); // consume operator
            self.parse_binary_expr(prec + 1);
        }
    }

    fn parse_unary_or_primary(&mut self) {
        let k = self.peek();
        match k {
            SyntaxKind::Minus | SyntaxKind::Plus | SyntaxKind::Exclamation | SyntaxKind::Tilde | SyntaxKind::KwNot => {
                self.advance();
                self.parse_unary_or_primary();
            }
            SyntaxKind::Ident => {
                self.advance();
                self.parse_postfix_chain();
            }
            SyntaxKind::IntLit | SyntaxKind::HexIntLit | SyntaxKind::BinIntLit | SyntaxKind::OctIntLit | SyntaxKind::FloatLit | SyntaxKind::StrLit | SyntaxKind::TripleStrLit | SyntaxKind::KwTrue | SyntaxKind::KwFalse | SyntaxKind::KwNone | SyntaxKind::KwNull => {
                self.advance();
                self.parse_postfix_chain();
            }
            SyntaxKind::LParen => {
                self.advance();
                self.parse_expr();
                if self.at(SyntaxKind::RParen) {
                    self.advance();
                } else {
                    self.sink.token(SyntaxKind::MissingRParen, "");
                    self.sink.error(SyntaxDiagnostic::with_fix("Expected ')' closing expression", self.peek_span(), ")"));
                }
                self.parse_postfix_chain();
            }
            SyntaxKind::LBracket => {
                self.advance();
                while !self.at_end() && !self.at(SyntaxKind::RBracket) {
                    self.parse_expr();
                    if self.at(SyntaxKind::Comma) {
                        self.advance();
                    }
                }
                if self.at(SyntaxKind::RBracket) {
                    self.advance();
                } else {
                    self.sink.token(SyntaxKind::MissingRBracket, "");
                    self.sink.error(SyntaxDiagnostic::with_fix("Expected ']' closing list", self.peek_span(), "]"));
                }
                self.parse_postfix_chain();
            }
            SyntaxKind::KwIf => {
                self.parse_if_expr();
            }
            SyntaxKind::LBrace => {
                self.parse_block_or_colon_suite();
            }
            _ => {
                // If not at a recognized primary, synthesize MissingExpr if at semicolon or delimiter
                if self.at(SyntaxKind::Semicolon) || self.at(SyntaxKind::RParen) || self.at(SyntaxKind::RBrace) || self.at(SyntaxKind::Comma) {
                    self.sink.token(SyntaxKind::MissingExpr, "");
                    self.sink.error(SyntaxDiagnostic::new("Expected expression", self.peek_span()));
                }
            }
        }
    }

    fn parse_if_expr(&mut self) {
        self.sink.start_node(SyntaxKind::IfExpr);
        self.advance(); // 'if'

        // Check if there is an unclosed '(' before block: e.g. if (x > 0 {
        if self.at(SyntaxKind::LParen) {
            self.advance();
            self.parse_expr();
            if self.at(SyntaxKind::RParen) {
                self.advance();
            } else if self.at(SyntaxKind::LBrace) || self.at(SyntaxKind::Colon) {
                // Synthesize MissingRParen!
                self.sink.token(SyntaxKind::MissingRParen, "");
                self.sink.error(SyntaxDiagnostic::with_fix("Expected ')' before block", self.peek_span(), ")"));
            }
        } else {
            self.parse_expr();
        }

        self.parse_block_or_colon_suite();

        if self.at(SyntaxKind::KwElse) {
            self.advance();
            if self.at(SyntaxKind::KwIf) {
                self.parse_if_expr();
            } else {
                self.parse_block_or_colon_suite();
            }
        }
        self.sink.finish_node();
    }

    fn parse_postfix_chain(&mut self) {
        loop {
            let k = self.peek();
            if k == SyntaxKind::Dot || k == SyntaxKind::SafeNav {
                self.sink.start_node(SyntaxKind::MemberExpr);
                self.advance(); // consume '.' or '?.'
                if self.at(SyntaxKind::Ident) {
                    self.advance();
                } else {
                    // Milestone 31.2: Synthesize MissingIdent on trailing dot for IDE completion!
                    self.sink.token(SyntaxKind::MissingIdent, "");
                    self.sink.error(SyntaxDiagnostic::new("Expected identifier after '.'", self.peek_span()));
                }
                self.sink.finish_node();
                continue;
            }

            if k == SyntaxKind::LParen || matches!(k, SyntaxKind::CallQuoteShallow | SyntaxKind::CallQuoteDeep | SyntaxKind::CallQuoteAbi | SyntaxKind::CallQuoteReact | SyntaxKind::CallQuoteAdjoint | SyntaxKind::CallQuoteEffect) {
                self.sink.start_node(SyntaxKind::CallExpr);
                if k != SyntaxKind::LParen {
                    self.advance(); // consume CallQuote*
                }
                if self.at(SyntaxKind::LParen) {
                    self.advance(); // '('
                    while !self.at_end() && !self.at(SyntaxKind::RParen) {
                        self.parse_expr();
                        if self.at(SyntaxKind::Comma) {
                            self.advance();
                        }
                    }
                    if self.at(SyntaxKind::RParen) {
                        self.advance();
                    } else {
                        self.sink.token(SyntaxKind::MissingRParen, "");
                        self.sink.error(SyntaxDiagnostic::with_fix("Expected ')' closing arguments", self.peek_span(), ")"));
                    }
                }
                self.sink.finish_node();
                continue;
            }

            if k == SyntaxKind::LBracket {
                self.sink.start_node(SyntaxKind::IndexExpr);
                self.advance(); // '['
                self.parse_expr();
                if self.at(SyntaxKind::RBracket) {
                    self.advance();
                } else {
                    self.sink.token(SyntaxKind::MissingRBracket, "");
                    self.sink.error(SyntaxDiagnostic::with_fix("Expected ']' closing index", self.peek_span(), "]"));
                }
                self.sink.finish_node();
                continue;
            }

            if k == SyntaxKind::Question {
                self.sink.start_node(SyntaxKind::TryPropagateExpr);
                self.advance();
                self.sink.finish_node();
                continue;
            }

            break;
        }
    }
}
