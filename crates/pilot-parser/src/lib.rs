//! Milestone 30: Stage 2 — Hand-Written Recursive Descent + Pratt (TDOP) Parser (`pilot-parser`).
//!
//! Enforces Cross-Milestone Invariants #1–#9:
//! - Strictly 16-byte `AstNode` arena lowered directly from Stage 1 `TokenTape`.
//! - Pratt Top-Down Operator Precedence (TDOP) expression binding powers.
//! - Pratt Type-Spec binding powers (Invariant #6: `T?` > `&` > `|` > `->` > `!`).
//! - O(1) delimiter jump table disambiguation using `TokenTape::bracket_links`.

use pilot_ast::*;
use pilot_intern::SymbolId;
use pilot_lexer::{tokenize, TokenFlags, TokenKind, TokenTape};
use pilot_span::ByteSpan;

/// Parses a Pilot source string into an `AstArena`.
pub fn parse(src: &str) -> AstArena {
    let tape = tokenize(src);
    let mut parser = Parser::new(src, tape);
    parser.parse_program()
}

/// Milestone 30.4: Parallel module skeleton & chunk parsing using scoped threads.
pub fn parse_parallel(src: &str) -> AstArena {
    if src.len() < 32 * 1024 {
        return parse(src);
    }

    let mid = src.len() / 2;
    let split_pos = match src[mid..].find('\n') {
        Some(idx) => mid + idx + 1,
        None => return parse(src),
    };

    let (part1, part2) = src.split_at(split_pos);
    std::thread::scope(|s| {
        let h1 = s.spawn(|| parse(part1));
        let h2 = s.spawn(|| parse(part2));
        let mut a1 = h1.join().unwrap();
        let a2 = h2.join().unwrap();

        let offset = a1.nodes.len() as u32;
        let root2_children = if a2.root.is_some() {
            let n = a2.node(a2.root);
            a2.children_slice(n.lhs, n.rhs).to_vec()
        } else {
            Vec::new()
        };

        for n in a2.nodes {
            let mut adjusted = n;
            if adjusted.lhs != u32::MAX && adjusted.node_kind() != NodeKind::IdentExpr && adjusted.node_kind() != NodeKind::IntLit {
                adjusted.lhs += offset;
            }
            if adjusted.rhs != u32::MAX && adjusted.rhs != 0 && adjusted.node_kind() == NodeKind::BinaryExpr {
                adjusted.rhs += offset;
            }
            a1.nodes.push(adjusted);
        }
        for sp in a2.spans {
            a1.spans.push(ByteSpan::new(sp.start + split_pos as u32, sp.end + split_pos as u32));
        }

        if a1.root.is_some() {
            for &c in &root2_children {
                a1.extra_children.push(c + offset);
            }
            let root_node = &mut a1.nodes[a1.root.0 as usize];
            root_node.rhs += root2_children.len() as u32;
        }

        a1
    })
}

#[inline]
pub fn is_polyglot_lang(name: &str) -> bool {
    matches!(
        name,
        "python"
            | "py"
            | "csharp"
            | "cs"
            | "rust"
            | "rs"
            | "js"
            | "javascript"
            | "ts"
            | "typescript"
            | "java"
            | "asm"
            | "assembly"
            | "cpp"
            | "c++"
            | "c"
            | "lua"
            | "sql"
            | "postgres"
            | "postgresql"
            | "mysql"
            | "sqlite"
    )
}

pub struct Parser<'a> {
    src: &'a str,
    tape: TokenTape,
    pos: usize,
    arena: AstArena,
    pub errors: Vec<String>,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str, tape: TokenTape) -> Self {
        Self {
            src,
            tape,
            pos: 0,
            arena: AstArena::with_capacity(32),
            errors: Vec::new(),
        }
    }

    #[inline]
    fn at_end(&self) -> bool {
        self.pos >= self.tape.len() || self.tape.kind(self.pos) == TokenKind::Eof
    }

    #[inline]
    fn peek(&self) -> TokenKind {
        if self.pos < self.tape.len() {
            self.tape.kind(self.pos)
        } else {
            TokenKind::Eof
        }
    }

    #[inline]
    fn peek_ahead(&self, offset: usize) -> TokenKind {
        if self.pos + offset < self.tape.len() {
            self.tape.kind(self.pos + offset)
        } else {
            TokenKind::Eof
        }
    }

    #[inline]
    fn at(&self, kind: TokenKind) -> bool {
        self.peek() == kind
    }

    #[inline]
    fn span(&self) -> ByteSpan {
        if self.pos < self.tape.len() {
            self.tape.span(self.pos)
        } else {
            ByteSpan::new(self.src.len() as u32, self.src.len() as u32)
        }
    }

    #[inline]
    fn prev_span(&self) -> ByteSpan {
        if self.pos > 0 && self.pos - 1 < self.tape.len() {
            self.tape.span(self.pos - 1)
        } else {
            self.span()
        }
    }

    #[inline]
    fn text(&self) -> &'a str {
        if self.pos < self.tape.len() {
            self.tape.text(self.pos, self.src)
        } else {
            ""
        }
    }

    #[inline]
    fn is_ident_like(&self) -> bool {
        if self.pos >= self.tape.len() {
            return false;
        }
        let k = self.peek();
        if k == TokenKind::Ident {
            return true;
        }
        if self.tape.flag(self.pos).contains(TokenFlags::IS_CONTEXTUAL_IDENT) {
            !matches!(
                k,
                TokenKind::KwTrue
                    | TokenKind::KwFalse
                    | TokenKind::KwNone
                    | TokenKind::KwNull
            )
        } else {
            false
        }
    }

    #[inline]
    fn advance(&mut self) -> TokenKind {
        let k = self.peek();
        if self.pos < self.tape.len() {
            self.pos += 1;
        }
        self.skip_whitespace_and_comments();
        k
    }

    fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.tape.len() {
            let k = self.tape.kind(self.pos);
            if k == TokenKind::Comment || k == TokenKind::BlockComment || k == TokenKind::Shebang {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn consume_semi_or_newlines(&mut self) {
        while self.pos < self.tape.len() {
            let k = self.tape.kind(self.pos);
            if k == TokenKind::Semicolon || k == TokenKind::Newline {
                self.pos += 1;
                self.skip_whitespace_and_comments();
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, kind: TokenKind) -> bool {
        if self.at(kind) {
            self.advance();
            true
        } else {
            self.errors.push(format!("Expected {:?}, found {:?}", kind, self.peek()));
            false
        }
    }

    fn peek_is_polyglot_block(&self) -> bool {
        if !self.is_ident_like() || !is_polyglot_lang(self.text()) {
            return false;
        }
        let mut lookahead = self.pos + 1;
        while lookahead < self.tape.len() && self.tape.kind(lookahead) == TokenKind::Newline {
            lookahead += 1;
        }
        if lookahead < self.tape.len() {
            let k = self.tape.kind(lookahead);
            k == TokenKind::LBrace
                || k == TokenKind::RawDslBlock
                || k == TokenKind::TripleStrLit
                || k == TokenKind::StrLit
        } else {
            false
        }
    }

    fn parse_polyglot_block(&mut self) -> NodeId {
        let start_span = self.span();
        let lang = self.text().to_string();
        self.advance(); // consume language identifier
        self.skip_whitespace_and_comments();
        while self.at(TokenKind::Newline) {
            self.advance();
        }

        let mut flags = 0u16;
        let code_str: String;
        let end_pos: u32;

        if self.at(TokenKind::LBrace) {
            let open_pos = self.pos;
            let open_span = self.span();
            let close_pos = self.tape.bracket_links[open_pos] as usize;
            if close_pos > open_pos && close_pos < self.tape.len() {
                let close_span = self.tape.span(close_pos);
                let raw_content = &self.src[open_span.end as usize..close_span.start as usize];
                code_str = raw_content.to_string();
                end_pos = close_span.end;
                self.pos = close_pos + 1;
            } else {
                // Fallback: scan to matching RBrace
                self.advance();
                let mut depth = 1;
                let start_idx = open_span.end as usize;
                while !self.at_end() && depth > 0 {
                    if self.at(TokenKind::LBrace) {
                        depth += 1;
                    } else if self.at(TokenKind::RBrace) {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    self.advance();
                }
                let end_idx = if self.at(TokenKind::RBrace) {
                    let s = self.span().start as usize;
                    self.advance();
                    s
                } else {
                    self.src.len()
                };
                code_str = self.src[start_idx..end_idx.min(self.src.len())].to_string();
                end_pos = self.prev_span().end;
            }
        } else if self.at(TokenKind::RawDslBlock) {
            let raw = self.text();
            let trimmed = raw.trim_start_matches('`').trim_end_matches('`');
            let cleaned = if let Some(newline_idx) = trimmed.find('\n') {
                &trimmed[newline_idx + 1..]
            } else {
                trimmed
            };
            code_str = cleaned.to_string();
            end_pos = self.span().end;
            self.advance();
        } else if self.at(TokenKind::TripleStrLit) {
            let raw = self.text();
            let cleaned = if raw.starts_with("\"\"\"") {
                raw.trim_matches('"')
            } else {
                raw.trim_matches('\'')
            };
            code_str = cleaned.to_string();
            end_pos = self.span().end;
            self.advance();
        } else if self.at(TokenKind::StrLit) {
            let raw = self.text();
            let cleaned = raw.trim_matches('"').trim_matches('\'');
            code_str = cleaned.to_string();
            end_pos = self.span().end;
            self.advance();
        } else {
            code_str = String::new();
            end_pos = start_span.end;
        }

        if code_str.contains("yield") {
            flags |= 1;
        }

        let full_span = ByteSpan::new(start_span.start, end_pos);
        self.arena.alloc_polyglot_block(&lang, &code_str, flags, full_span)
    }

    fn parse_extern_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'extern'

        // Optional lang target: "rust" or rust
        if self.at(TokenKind::StrLit) || self.is_ident_like() {
            self.advance();
        }
        // Optional kind: 'lib' | 'assembly' | 'module'
        if self.is_ident_like() {
            let t = self.text();
            if t == "lib" || t == "assembly" || t == "module" {
                self.advance();
                // target path or name
                if self.at(TokenKind::StrLit) || self.is_ident_like() {
                    self.advance();
                }
            }
        }
        if self.at(TokenKind::Colon) {
            self.advance();
        }

        if self.at(TokenKind::LBrace) {
            self.advance();
            let mut members = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RBrace) {
                while self.is_ident_like() {
                    let m = self.text();
                    if m == "public" || m == "private" || m == "protected" || m == "internal" || m == "static" {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let member = self.parse_stmt_or_decl();
                if member.is_some() {
                    members.push(member);
                }
                self.consume_semi_or_newlines();
            }
            self.expect(TokenKind::RBrace);
            self.arena.alloc_list_node(NodeKind::Program, 0, ByteSpan::new(start_span.start, self.prev_span().end), &members)
        } else {
            while self.is_ident_like() {
                let m = self.text();
                if m == "public" || m == "private" || m == "protected" || m == "internal" || m == "static" {
                    self.advance();
                } else {
                    break;
                }
            }
            self.parse_stmt_or_decl()
        }
    }

    /// Entry point: parses an entire Pilot program.
    pub fn parse_program(&mut self) -> AstArena {
        self.skip_whitespace_and_comments();
        let mut top_children = Vec::new();

        while !self.at_end() {
            self.consume_semi_or_newlines();
            if self.at_end() {
                break;
            }
            let start_pos = self.pos;
            let node = self.parse_stmt_or_decl();
            if node.is_some() {
                if self.arena.node(node).node_kind() == NodeKind::Program {
                    let s = self.arena.node(node).lhs;
                    let l = self.arena.node(node).rhs;
                    let sub_children = self.arena.children_slice(s, l).to_vec();
                    for c in sub_children {
                        top_children.push(NodeId(c));
                    }
                } else {
                    top_children.push(node);
                }
            }
            if self.pos == start_pos {
                self.advance();
            }
            self.consume_semi_or_newlines();
        }

        let root_span = ByteSpan::new(0, self.src.len() as u32);
        let root = self.arena.alloc_list_node(NodeKind::Program, 0, root_span, &top_children);
        self.arena.root = root;
        std::mem::take(&mut self.arena)
    }

    fn parse_stmt_or_decl(&mut self) -> NodeId {
        while self.at(TokenKind::At) {
            self.advance();
            if self.is_ident_like() {
                self.advance();
                if self.at(TokenKind::LParen) {
                    self.advance();
                    while !self.at_end() && !self.at(TokenKind::RParen) {
                        self.advance();
                    }
                    if self.at(TokenKind::RParen) {
                        self.advance();
                    }
                }
            }
            self.consume_semi_or_newlines();
        }

        while self.is_ident_like() {
            let txt = self.text();
            if txt == "public" || txt == "private" || txt == "protected" || txt == "internal" || txt == "static" {
                self.advance();
            } else {
                break;
            }
        }

        if self.at(TokenKind::KwExtern) || (self.is_ident_like() && self.text() == "extern") {
            return self.parse_extern_decl();
        }

        if self.peek_is_polyglot_block() {
            return self.parse_polyglot_block();
        }

        // Labeled statement: `outer_loop: for ...`
        if self.is_ident_like() && self.peek_ahead(1) == TokenKind::Colon && self.peek_ahead(2) != TokenKind::Colon {
            let label_name = self.text();
            if label_name != "default" && label_name != "case" {
                self.advance(); // consume ident
                self.advance(); // consume ':'
                return self.parse_stmt_or_decl();
            }
        }

        if self.is_ident_like() {
            let txt = self.text();
            if txt == "async" || txt == "pure" || txt == "atomic" {
                let next_pos = self.pos + 1;
                if next_pos < self.tape.len() && (self.tape.kind(next_pos) == TokenKind::KwDef || self.tape.kind(next_pos) == TokenKind::KwFn) {
                    self.advance();
                    return self.parse_fn_decl();
                }
            } else if txt == "actor" {
                self.advance();
                return self.parse_class_decl();
            } else if txt == "dynamic" {
                self.advance();
                return self.parse_binding_stmt(NodeKind::LetStmt);
            } else if txt == "ref" {
                self.advance();
                return self.parse_binding_stmt(NodeKind::VarStmt);
            } else if txt == "noalloc" || txt == "region" {
                self.advance();
                if self.is_ident_like() {
                    self.advance();
                }
                return self.parse_block_expr();
            }
        }

        let k = self.peek();
        match k {
            TokenKind::KwImport => self.parse_import_stmt(),
            TokenKind::KwFrom => self.parse_from_import_stmt(),
            TokenKind::KwDef | TokenKind::KwFn => self.parse_fn_decl(),
            TokenKind::KwClass | TokenKind::KwActor | TokenKind::KwState => self.parse_class_decl(),
            TokenKind::KwStruct => self.parse_struct_decl(),
            TokenKind::KwRecord => self.parse_record_decl(),
            TokenKind::KwInterface | TokenKind::KwProtocol => self.parse_interface_decl(),
            TokenKind::KwExtend | TokenKind::KwExtension => self.parse_extend_decl(),
            TokenKind::KwType | TokenKind::KwEnum => self.parse_type_adt_decl(),
            TokenKind::KwLet | TokenKind::KwDynamic => self.parse_binding_stmt(NodeKind::LetStmt),
            TokenKind::KwVar | TokenKind::KwRef => self.parse_binding_stmt(NodeKind::VarStmt),
            TokenKind::KwConst => self.parse_binding_stmt(NodeKind::ConstDecl),
            TokenKind::KwReturn => self.parse_return_stmt(),
            TokenKind::KwYield => self.parse_yield_stmt(),
            TokenKind::KwBreak => self.parse_break_stmt(),
            TokenKind::KwContinue => self.parse_continue_stmt(),
            TokenKind::KwGuard => self.parse_guard_stmt(),
            TokenKind::KwDefer => self.parse_defer_stmt(),
            TokenKind::KwWhile => self.parse_while_stmt(),
            TokenKind::KwFor => self.parse_for_stmt(),
            TokenKind::KwIf => self.parse_if_expr(),
            TokenKind::KwMatch => self.parse_match_expr(),
            TokenKind::KwSelect => self.parse_select_stmt(),
            TokenKind::KwTry => self.parse_try_stmt(),
            TokenKind::KwWith => self.parse_with_stmt(),
            TokenKind::KwProperty => self.parse_property_decl(),
            TokenKind::KwOperator => self.parse_operator_decl(),
            TokenKind::KwMacro | TokenKind::KwDefmacro => self.parse_macro_decl(),
            TokenKind::KwAdapter | TokenKind::KwConverter => self.parse_adapter_decl(),
            TokenKind::KwRaise => self.parse_raise_stmt(),
            TokenKind::KwAssert => self.parse_assert_stmt(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_assert_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'assert'
        let has_paren = self.at(TokenKind::LParen);
        if has_paren {
            self.advance();
        }
        let cond = self.parse_expr();
        if has_paren {
            self.expect(TokenKind::RParen);
        }
        let callee = self.arena.alloc_ident("assert", start_span);
        let (arg_start, arg_len) = self.arena.alloc_child_slice(&[callee, cond]);
        let call_node = self.arena.alloc_node(NodeKind::CallExpr, 0, start_span, arg_start, arg_len);
        self.arena.alloc_node(NodeKind::ExprStmt, 0, start_span, call_node.0, 0)
    }

    fn parse_import_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'import'
        let mut path = String::new();
        let mut alias = None;

        while !self.at_end() && !self.at(TokenKind::Semicolon) && !self.at(TokenKind::Newline) {
            if self.at(TokenKind::KwAs) {
                self.advance(); // consume 'as'
                if self.is_ident_like() {
                    alias = Some(self.text().to_string());
                    self.advance();
                }
                break;
            }
            path.push_str(self.text());
            self.advance();
        }

        let path_sym = self.arena.interner.intern(&path);
        let alias_sym = if let Some(ref a) = alias {
            self.arena.interner.intern(a).0
        } else {
            u32::MAX
        };
        self.arena.alloc_node(NodeKind::ImportDecl, 0, ByteSpan::new(start_span.start, self.prev_span().end), path_sym.0, alias_sym)
    }

    fn parse_from_import_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'from'
        let mut path = String::new();
        while !self.at_end() && !self.at(TokenKind::KwImport) {
            path.push_str(self.text());
            self.advance();
        }
        let _path_sym = self.arena.interner.intern(&path);

        let mut imported_symbols = Vec::new();
        if self.at(TokenKind::KwImport) {
            self.advance(); // consume 'import'
            if self.at(TokenKind::LParen) {
                self.advance();
            }
            while !self.at_end() && !self.at(TokenKind::Semicolon) && !self.at(TokenKind::Newline) && !self.at(TokenKind::RParen) {
                if self.is_ident_like() {
                    let mut name = self.text().to_string();
                    self.advance();
                    if self.at(TokenKind::KwAs) {
                        self.advance();
                        if self.is_ident_like() {
                            name = self.text().to_string();
                            self.advance();
                        }
                    }
                    let sym_node = self.arena.alloc_ident(&name, self.prev_span());
                    imported_symbols.push(sym_node);
                } else if self.at(TokenKind::Star) {
                    self.advance();
                } else if self.at(TokenKind::Comma) {
                    self.advance();
                } else {
                    self.advance();
                }
            }
            if self.at(TokenKind::RParen) {
                self.advance();
            }
        }
        self.arena.alloc_list_node(NodeKind::FromImportDecl, 0, ByteSpan::new(start_span.start, self.prev_span().end), &imported_symbols)
    }

    fn parse_fn_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'def' or 'fn'
        let name = if self.is_ident_like() {
            let s = self.text();
            self.advance();
            s
        } else {
            "anon"
        };
        let sym = self.arena.interner.intern(name);

        // Generic type parameters: [T, U]
        if self.at(TokenKind::LBracket) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::RBracket) {
                self.advance();
                if self.at(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBracket);
        }

        // Parameters: (a: i64, b: i64)
        let mut params = Vec::new();
        if self.at(TokenKind::LParen) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::RParen) {
                while self.at(TokenKind::KwVar)
                    || self.at(TokenKind::KwRef)
                    || self.at(TokenKind::KwMut)
                    || (self.is_ident_like() && (self.text() == "context" || self.text() == "var" || self.text() == "ref" || self.text() == "val"))
                {
                    if self.pos + 1 < self.tape.len() {
                        let next_k = self.tape.kind(self.pos + 1);
                        if next_k == TokenKind::Colon || next_k == TokenKind::Comma || next_k == TokenKind::RParen {
                            break;
                        }
                    }
                    self.advance();
                }
                if self.is_ident_like() {
                    let p_name = self.text();
                    self.advance();
                    let p_sym = self.arena.interner.intern(p_name);
                    let mut type_node = NodeId::NONE;
                    if self.at(TokenKind::Colon) {
                        self.advance();
                        type_node = self.parse_type();
                    }
                    let p_node = self.arena.alloc_node(NodeKind::ParamDecl, 0, self.span(), p_sym.0, type_node.0);
                    params.push(p_node);
                } else {
                    self.advance();
                }
                if self.at(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RParen);
        }

        // Return type: -> T
        if self.at(TokenKind::Arrow) {
            self.advance();
            self.parse_type();
        }

        self.consume_semi_or_newlines();

        // Design-by-Contract clauses: requires <expr>, ensures <expr>
        while self.at(TokenKind::KwRequires) || self.at(TokenKind::KwEnsures) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::Colon) && !self.at(TokenKind::KwRequires) && !self.at(TokenKind::KwEnsures) && !self.at(TokenKind::KwDef) && !self.at(TokenKind::KwFn) {
                self.advance();
            }
            self.consume_semi_or_newlines();
        }

        // Body: => expr, { ... }, or : suite
        let body = if self.at(TokenKind::FatArrow) {
            self.advance();
            let ret_expr = self.parse_expr();
            self.arena.alloc_node(NodeKind::ReturnStmt, 0, start_span, ret_expr.0, 0)
        } else if self.at(TokenKind::LBrace) || self.at(TokenKind::VirtualLBrace) {
            self.parse_block_expr()
        } else if self.at(TokenKind::Colon) {
            self.advance();
            self.parse_block_expr()
        } else {
            NodeId::NONE
        };

        let mut all_fn = Vec::with_capacity(1 + params.len());
        all_fn.push(body);
        all_fn.extend(params);
        let (fn_start, fn_len) = self.arena.alloc_child_slice(&all_fn);

        self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, start_span, sym.0, fn_start)
    }

    fn parse_class_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'class', 'actor', or 'state'
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);
        if self.at(TokenKind::LBracket) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::RBracket) {
                self.advance();
                if self.at(TokenKind::Comma) {
                    self.advance();
                }
            }
            if self.at(TokenKind::RBracket) {
                self.advance();
            }
        }
        if self.at(TokenKind::Equal) {
            self.advance(); // consume '='
            let expr = self.parse_expr();
            self.consume_semi_or_newlines();
            return self.arena.alloc_node(NodeKind::ClassDecl, 0, start_span, sym.0, expr.0);
        }
        let body = self.parse_block_expr();
        self.arena.alloc_node(NodeKind::ClassDecl, 0, start_span, sym.0, body.0)
    }

    fn parse_struct_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'struct'
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);
        if !self.at(TokenKind::LBrace) && !self.at(TokenKind::Colon) {
            // C-style struct variable declaration e.g. `struct timespec start, end`
            let mut last_span = self.span();
            while self.is_ident_like() || self.at(TokenKind::Star) {
                last_span = self.span();
                self.advance();
                if self.at(TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            if self.at(TokenKind::Semicolon) {
                self.advance();
            }
            let span = ByteSpan::new(start_span.start, last_span.end);
            return self.arena.alloc_node(NodeKind::StructDecl, 0, span, sym.0, 0);
        }
        let body = self.parse_block_expr();
        self.arena.alloc_node(NodeKind::StructDecl, 0, start_span, sym.0, body.0)
    }

    fn parse_record_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'record'
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);
        // Parameters / fields: (x: i64, y: i64)
        if self.at(TokenKind::LParen) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::RParen) {
                self.advance();
            }
            self.expect(TokenKind::RParen);
        }
        self.arena.alloc_node(NodeKind::RecordDecl, 0, start_span, sym.0, 0)
    }

    fn parse_interface_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'interface' or 'protocol'
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);
        if self.at(TokenKind::LBracket) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::RBracket) {
                self.advance();
                if self.at(TokenKind::Comma) {
                    self.advance();
                }
            }
            if self.at(TokenKind::RBracket) {
                self.advance();
            }
        }
        let body = self.parse_block_expr();
        self.arena.alloc_node(NodeKind::InterfaceDecl, 0, start_span, sym.0, body.0)
    }

    fn parse_extend_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'extend' or 'extension'
        let target = self.text();
        self.advance();
        let sym = self.arena.interner.intern(target);
        while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::Colon) && !self.at(TokenKind::Newline) {
            self.advance();
        }
        let body = self.parse_block_expr();
        self.arena.alloc_node(NodeKind::ExtendDecl, 0, start_span, sym.0, body.0)
    }

    fn parse_type_adt_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'type' or 'enum'
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);

        // Optional generic type parameter: [T]
        if self.at(TokenKind::LBracket) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::RBracket) {
                self.advance();
            }
            if self.at(TokenKind::RBracket) {
                self.advance();
            }
        }

        let mut variants = Vec::new();
        if self.at(TokenKind::Equal) || self.at(TokenKind::LBrace) {
            let is_brace = self.at(TokenKind::LBrace);
            self.advance();
            while self.at(TokenKind::Newline) || self.at(TokenKind::VirtualLBrace) {
                self.advance();
            }

            // Check if this is an ADT or a Type Alias
            let is_adt = is_brace
                || self.at(TokenKind::Vbar)
                || (self.is_ident_like() && self.peek_ahead(1) == TokenKind::LParen);

            if !is_adt {
                let ty_node = self.parse_type();
                if self.at(TokenKind::Semicolon) {
                    self.advance();
                }
                return self.arena.alloc_node(NodeKind::TypeAdtDecl, 0, start_span, sym.0, ty_node.0);
            }

            while !self.at_end() && !self.at(TokenKind::Semicolon) && (!is_brace || !self.at(TokenKind::RBrace)) {
                while self.at(TokenKind::Vbar) || self.at(TokenKind::Comma) || self.at(TokenKind::Newline) || self.at(TokenKind::VirtualLBrace) || self.at(TokenKind::VirtualRBrace) {
                    self.advance();
                }
                if self.at(TokenKind::Semicolon)
                    || (is_brace && self.at(TokenKind::RBrace))
                    || self.at_end()
                    || self.at(TokenKind::KwType)
                    || self.at(TokenKind::KwEnum)
                    || self.at(TokenKind::KwDef)
                    || self.at(TokenKind::KwLet)
                    || self.at(TokenKind::KwVar)
                    || self.at(TokenKind::KwRecord)
                    || self.at(TokenKind::KwAssert)
                {
                    break;
                }
                if self.is_ident_like() {
                    let v_span = self.span();
                    let v_name = self.text();
                    self.advance();
                    let v_sym = self.arena.interner.intern(v_name);

                    // Variant parameters: (radius: f64, ...)
                    let mut v_params = Vec::new();
                    if self.at(TokenKind::LParen) {
                        self.advance();
                        while !self.at_end() && !self.at(TokenKind::RParen) {
                            let p_span = self.span();
                            if self.is_ident_like() {
                                let p_name = self.text();
                                self.advance();
                                let p_sym = self.arena.interner.intern(p_name);
                                let mut ty_node = NodeId::NONE;
                                if self.at(TokenKind::Colon) {
                                    self.advance();
                                    ty_node = self.parse_type();
                                }
                                let p_node = self.arena.alloc_node(NodeKind::ParamDecl, 0, p_span, p_sym.0, ty_node.0);
                                v_params.push(p_node);
                            } else {
                                self.advance();
                            }
                            if self.at(TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::RParen);
                    }
                    let (v_start, v_len) = self.arena.alloc_child_slice(&v_params);
                    let v_node = self.arena.alloc_node(NodeKind::AdtVariant, v_len as u16, v_span, v_sym.0, v_start);
                    variants.push(v_node);
                    if self.at(TokenKind::Comma) {
                        self.advance();
                    }
                } else {
                    break;
                }
            }
            if is_brace {
                while self.at(TokenKind::VirtualRBrace) {
                    self.advance();
                }
                if self.at(TokenKind::RBrace) {
                    self.advance();
                }
            }
        }
        if self.at(TokenKind::Semicolon) {
            self.advance();
        }

        let (var_start, var_len) = self.arena.alloc_child_slice(&variants);
        self.arena.alloc_node(NodeKind::TypeAdtDecl, var_len as u16, start_span, sym.0, var_start)
    }

    fn parse_binding_stmt(&mut self, kind: NodeKind) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'let', 'var', 'const', 'dynamic', 'ref'
        if self.at(TokenKind::LParen) || self.at(TokenKind::LBracket) {
            let pat = self.parse_pattern();
            let init = if self.at(TokenKind::Equal) {
                self.advance();
                self.parse_expr()
            } else {
                NodeId::NONE
            };
            return self.arena.alloc_node(kind, 1, start_span, pat.0, init.0);
        }
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);

        // Optional type annotation: : type_spec
        if self.at(TokenKind::Colon) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::Equal) && !self.at(TokenKind::Semicolon) && !self.at(TokenKind::Newline) {
                self.advance();
            }
        }

        let init = if self.at(TokenKind::Equal) {
            self.advance();
            self.parse_expr()
        } else {
            NodeId::NONE
        };

        self.arena.alloc_node(kind, 0, start_span, sym.0, init.0)
    }

    fn parse_return_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'return'
        let val = if !self.at(TokenKind::Semicolon) && !self.at(TokenKind::Newline) && !self.at_end() {
            self.parse_expr()
        } else {
            NodeId::NONE
        };
        self.arena.alloc_node(NodeKind::ReturnStmt, 0, start_span, val.0, 0)
    }

    fn parse_yield_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'yield'
        let val = if !self.at(TokenKind::Semicolon) && !self.at(TokenKind::Newline) && !self.at_end() {
            self.parse_expr()
        } else {
            NodeId::NONE
        };
        self.arena.alloc_node(NodeKind::YieldStmt, 0, start_span, val.0, 0)
    }

    fn parse_break_stmt(&mut self) -> NodeId {
        let span = self.span();
        self.advance();
        if self.is_ident_like() {
            let name = self.text();
            let sym = self.arena.interner.intern(name);
            self.advance();
            return self.arena.alloc_node(NodeKind::BreakStmt, 0, span, sym.0, 0);
        }
        self.arena.alloc_node(NodeKind::BreakStmt, 0, span, 0, 0)
    }

    fn parse_continue_stmt(&mut self) -> NodeId {
        let span = self.span();
        self.advance();
        if self.is_ident_like() {
            let name = self.text();
            let sym = self.arena.interner.intern(name);
            self.advance();
            return self.arena.alloc_node(NodeKind::ContinueStmt, 0, span, sym.0, 0);
        }
        self.arena.alloc_node(NodeKind::ContinueStmt, 0, span, 0, 0)
    }

    fn parse_try_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'try'
        let try_body = self.parse_block_expr();
        let mut clauses = Vec::new();
        clauses.push(try_body);

        while self.at(TokenKind::KwExcept) || self.at(TokenKind::KwCatch) {
            let clause_span = self.span();
            let is_catch = self.at(TokenKind::KwCatch);
            self.advance(); // consume 'except' or 'catch'

            let mut var_sym = SymbolId(0);
            let mut type_sym = SymbolId(0);

            if is_catch {
                if self.at(TokenKind::LParen) {
                    self.advance();
                    if self.is_ident_like() {
                        var_sym = self.arena.interner.intern(self.text());
                        self.advance();
                    }
                    self.expect(TokenKind::RParen);
                }
            } else {
                // except [ErrorType] [as var_name]
                if self.is_ident_like() && !self.at(TokenKind::KwAs) {
                    type_sym = self.arena.interner.intern(self.text());
                    self.advance();
                }
                if self.at(TokenKind::KwAs) {
                    self.advance(); // consume 'as'
                    if self.is_ident_like() {
                        var_sym = self.arena.interner.intern(self.text());
                        self.advance();
                    }
                }
            }

            let body = self.parse_block_expr();
            let clause_node = self.arena.alloc_node(NodeKind::CatchClause, type_sym.0 as u16, clause_span, var_sym.0, body.0);
            clauses.push(clause_node);
        }

        // Optional else block: `else { ... }`
        if self.at(TokenKind::KwElse) {
            self.advance();
            let else_body = self.parse_block_expr();
            clauses.push(else_body);
        }

        // Optional finally block: `finally { ... }`
        if self.at(TokenKind::KwFinally) {
            self.advance();
            let finally_body = self.parse_block_expr();
            clauses.push(finally_body);
        }

        let (start, len) = self.arena.alloc_child_slice(&clauses);
        self.arena.alloc_node(NodeKind::TryStmt, 0, start_span, start, len)
    }

    fn parse_with_stmt(&mut self) -> NodeId {
        self.advance(); // consume 'with'
        while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::Colon) && !self.at(TokenKind::Newline) {
            self.advance();
        }
        if self.at(TokenKind::Colon) {
            self.advance();
        }
        self.parse_block_expr()
    }

    fn parse_property_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'property'
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);
        if self.at(TokenKind::Colon) {
            self.advance();
            self.parse_type();
        }
        if self.at(TokenKind::LBrace) {
            self.advance();
            let mut stmts = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.consume_semi_or_newlines();
                if self.at(TokenKind::RBrace) || self.at_end() {
                    break;
                }
                let span = self.span();
                if self.text() == "get" {
                    self.advance();
                    let get_body = if self.at(TokenKind::FatArrow) {
                        self.advance();
                        let expr = self.parse_expr();
                        self.arena.alloc_node(NodeKind::ReturnStmt, 0, span, expr.0, 0)
                    } else {
                        self.parse_block_expr()
                    };
                    stmts.push(get_body);
                } else if self.text() == "set" {
                    self.advance();
                    let mut p_sym = self.arena.interner.intern("value");
                    if self.at(TokenKind::LParen) {
                        self.advance();
                        if self.is_ident_like() {
                            let p_name = self.text();
                            self.advance();
                            p_sym = self.arena.interner.intern(p_name);
                        }
                        self.expect(TokenKind::RParen);
                    }
                    let set_body = if self.at(TokenKind::FatArrow) {
                        self.advance();
                        let expr = self.parse_expr();
                        self.arena.alloc_node(NodeKind::ExprStmt, 0, span, expr.0, 0)
                    } else {
                        self.parse_block_expr()
                    };
                    let set_sym = self.arena.interner.intern("set");
                    let p_node = self.arena.alloc_node(NodeKind::ParamDecl, 0, span, p_sym.0, 0);
                    let (fn_start, fn_len) = self.arena.alloc_child_slice(&[set_body, p_node]);
                    let setter_fn = self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, span, set_sym.0, fn_start);
                    stmts.push(setter_fn);
                } else {
                    stmts.push(self.parse_stmt_or_decl());
                }
                self.consume_semi_or_newlines();
            }
            self.expect(TokenKind::RBrace);
            let body = self.arena.alloc_list_node(NodeKind::BlockExpr, 0, start_span, &stmts);
            return self.arena.alloc_node(NodeKind::FnDecl, 0, start_span, sym.0, body.0);
        }
        let body = self.parse_block_expr();
        self.arena.alloc_node(NodeKind::FnDecl, 0, start_span, sym.0, body.0)
    }

    fn parse_operator_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'operator'
        let op_text = self.text();
        self.advance();
        let sym = self.arena.interner.intern(op_text);
        if self.at(TokenKind::LParen) {
            let mut params = Vec::new();
            self.advance();
            while !self.at_end() && !self.at(TokenKind::RParen) {
                if self.is_ident_like() {
                    let p_name = self.text();
                    self.advance();
                    let p_sym = self.arena.interner.intern(p_name);
                    let mut ty_node = NodeId::NONE;
                    if self.at(TokenKind::Colon) {
                        self.advance();
                        ty_node = self.parse_type();
                    }
                    let p_node = self.arena.alloc_node(NodeKind::ParamDecl, 0, self.span(), p_sym.0, ty_node.0);
                    params.push(p_node);
                } else {
                    self.advance();
                }
                if self.at(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RParen);
            if self.at(TokenKind::Arrow) {
                self.advance();
                self.parse_type();
            }
            let body = self.parse_block_expr();
            let mut all_fn = Vec::with_capacity(1 + params.len());
            all_fn.push(body);
            all_fn.extend(params);
            let (fn_start, fn_len) = self.arena.alloc_child_slice(&all_fn);
            return self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, start_span, sym.0, fn_start);
        }
        let body = self.parse_block_expr();
        self.arena.alloc_node(NodeKind::FnDecl, 0, start_span, sym.0, body.0)
    }

    fn parse_macro_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'macro'
        if self.at(TokenKind::Exclamation) {
            self.advance();
        }
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);
        let mut params = Vec::new();
        if self.at(TokenKind::LParen) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::RParen) {
                if self.is_ident_like() {
                    let p_name = self.text();
                    self.advance();
                    let p_sym = self.arena.interner.intern(p_name);
                    let mut ty_node = NodeId::NONE;
                    if self.at(TokenKind::Colon) {
                        self.advance();
                        ty_node = self.parse_type();
                    }
                    let p_node = self.arena.alloc_node(NodeKind::ParamDecl, 0, self.span(), p_sym.0, ty_node.0);
                    params.push(p_node);
                } else {
                    self.advance();
                }
                if self.at(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RParen);
        }
        if self.at(TokenKind::Arrow) {
            self.advance();
            self.parse_type();
        }
        let body = self.parse_block_expr();
        let mut all_fn = Vec::with_capacity(1 + params.len());
        all_fn.push(body);
        all_fn.extend(params);
        let (fn_start, fn_len) = self.arena.alloc_child_slice(&all_fn);
        self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, start_span, sym.0, fn_start)
    }

    fn parse_adapter_decl(&mut self) -> NodeId {
        let _start_span = self.span();
        self.advance(); // consume 'adapter'
        while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::Colon) && !self.at(TokenKind::Newline) {
            self.advance();
        }
        if self.at(TokenKind::Colon) {
            self.advance();
        }
        self.parse_block_expr()
    }

    fn parse_select_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'select'
        if self.at(TokenKind::LBrace) {
            self.advance();
            let mut arms = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.consume_semi_or_newlines();
                if self.at(TokenKind::RBrace) || self.at_end() {
                    break;
                }
                let arm_span = self.span();
                if self.at(TokenKind::KwCase) {
                    self.advance();
                    // Either `val = <-ch` or `ch <- 100`
                    let mut bound_sym = SymbolId(0);
                    if self.is_ident_like() && self.peek_ahead(1) == TokenKind::Equal {
                        let name = self.text();
                        bound_sym = self.arena.interner.intern(name);
                        self.advance(); // consume ident
                        self.advance(); // consume '='
                    }
                    let _chan_expr = self.parse_expr();
                    if self.at(TokenKind::FatArrow) {
                        self.advance();
                    }
                    let body = self.parse_block_expr();
                    let arm_body = if bound_sym.0 != 0 {
                        let p_node = self.arena.alloc_node(NodeKind::ParamDecl, 0, arm_span, bound_sym.0, 0);
                        let (fn_start, fn_len) = self.arena.alloc_child_slice(&[body, p_node]);
                        self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, arm_span, bound_sym.0, fn_start)
                    } else {
                        body
                    };
                    arms.push(arm_body);
                } else if self.is_ident_like() && self.text() == "timeout" {
                    self.advance();
                    let _duration = self.parse_expr();
                    if self.at(TokenKind::FatArrow) {
                        self.advance();
                    }
                    let body = self.parse_block_expr();
                    arms.push(body);
                } else {
                    arms.push(self.parse_stmt_or_decl());
                }
                self.consume_semi_or_newlines();
            }
            self.expect(TokenKind::RBrace);
            return self.arena.alloc_list_node(NodeKind::BlockExpr, 0, start_span, &arms);
        }
        self.parse_block_expr()
    }

    fn parse_raise_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'raise'
        let val = if !self.at(TokenKind::Semicolon) && !self.at(TokenKind::Newline) && !self.at_end() {
            self.parse_expr()
        } else {
            NodeId::NONE
        };
        self.arena.alloc_node(NodeKind::ReturnStmt, 0, start_span, val.0, 0)
    }

    fn parse_guard_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'guard'
        let cond = self.parse_expr();
        if self.at(TokenKind::KwElse) {
            self.advance();
            self.parse_block_expr();
        }
        self.arena.alloc_node(NodeKind::GuardStmt, 0, start_span, cond.0, 0)
    }

    fn parse_defer_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'defer'
        let body = self.parse_block_expr();
        self.arena.alloc_node(NodeKind::DeferStmt, 0, start_span, body.0, 0)
    }

    fn parse_while_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'while'
        let cond = self.parse_expr();
        let body = self.parse_block_expr();
        self.arena.alloc_node(NodeKind::WhileStmt, 0, start_span, cond.0, body.0)
    }

    fn parse_for_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'for'
        let var_name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(var_name);
        self.expect(TokenKind::KwIn);
        let _iter = self.parse_expr();
        let body = self.parse_block_expr();
        self.arena.alloc_node(NodeKind::ForExpr, 0, start_span, sym.0, body.0)
    }

    fn parse_expr_stmt(&mut self) -> NodeId {
        let expr = self.parse_expr();
        if expr.is_some() {
            let k = self.peek();
            match k {
                TokenKind::Equal => {
                    self.advance();
                    let rhs = self.parse_expr();
                    let end = if rhs.is_some() { self.arena.span(rhs).end } else { self.arena.span(expr).end };
                    let span = ByteSpan::new(self.arena.span(expr).start, end);
                    return self.arena.alloc_node(NodeKind::AssignStmt, 0, span, expr.0, rhs.0);
                }
                TokenKind::PlusEqual
                | TokenKind::MinusEqual
                | TokenKind::StarEqual
                | TokenKind::SlashEqual
                | TokenKind::PercentEqual => {
                    let op = match k {
                        TokenKind::PlusEqual => BinaryOp::Add,
                        TokenKind::MinusEqual => BinaryOp::Sub,
                        TokenKind::StarEqual => BinaryOp::Mul,
                        TokenKind::SlashEqual => BinaryOp::Div,
                        TokenKind::PercentEqual => BinaryOp::Rem,
                        _ => unreachable!(),
                    };
                    self.advance();
                    let rhs = self.parse_expr();
                    let end = if rhs.is_some() { self.arena.span(rhs).end } else { self.arena.span(expr).end };
                    let span = ByteSpan::new(self.arena.span(expr).start, end);
                    let bin_expr = self.arena.alloc_node(NodeKind::BinaryExpr, op as u16, span, expr.0, rhs.0);
                    return self.arena.alloc_node(NodeKind::AssignStmt, 0, span, expr.0, bin_expr.0);
                }
                _ => {}
            }
            let span = self.arena.span(expr);
            self.arena.alloc_node(NodeKind::ExprStmt, 0, span, expr.0, 0)
        } else {
            NodeId::NONE
        }
    }

    pub fn parse_block_expr(&mut self) -> NodeId {
        let start_span = self.span();
        let is_virtual = self.at(TokenKind::VirtualLBrace);
        if self.at(TokenKind::LBrace) || is_virtual {
            self.advance();
        } else if self.at(TokenKind::Colon) {
            self.advance();
            if self.at(TokenKind::VirtualLBrace) {
                self.advance();
            }
        }

        let mut children = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) && !self.at(TokenKind::VirtualRBrace) {
            self.consume_semi_or_newlines();
            if self.at(TokenKind::RBrace) || self.at(TokenKind::VirtualRBrace) || self.at_end() {
                break;
            }
            let start_pos = self.pos;
            let child = self.parse_stmt_or_decl();
            if child.is_some() {
                children.push(child);
            }
            if self.pos == start_pos {
                self.advance();
            }
            self.consume_semi_or_newlines();
        }

        if self.at(TokenKind::RBrace) || self.at(TokenKind::VirtualRBrace) {
            self.advance();
        }

        self.arena.alloc_list_node(NodeKind::BlockExpr, 0, ByteSpan::new(start_span.start, self.prev_span().end), &children)
    }

    // =========================================================================
    // 30.1: Pratt Expression Parsing Engine (parse_expr_bp)
    // =========================================================================

    pub fn parse_expr(&mut self) -> NodeId {
        self.parse_expr_bp(ExprPrecedence::Lowest as u8)
    }

    pub fn parse_expr_bp(&mut self, min_bp: u8) -> NodeId {
        let mut lhs = self.parse_expr_prefix();
        if lhs.is_none() {
            return NodeId::NONE;
        }

        loop {
            let k = self.peek();
            // Handle postfix operators
            if k == TokenKind::Dot || k == TokenKind::SafeNav {
                let is_safe = k == TokenKind::SafeNav;
                self.advance();
                let member_name = self.text();
                self.advance();
                let sym = self.arena.interner.intern(member_name);
                let span = self.arena.span(lhs);
                lhs = self.arena.alloc_node(NodeKind::MemberExpr, is_safe as u16, span, lhs.0, sym.0);
                continue;
            }

            if k == TokenKind::LParen || self.is_call_quote_start(k) {
                lhs = self.parse_call_expr(lhs);
                continue;
            }

            if k == TokenKind::LBracket {
                self.advance();
                let idx = self.parse_expr();
                self.expect(TokenKind::RBracket);
                let span = self.arena.span(lhs);
                lhs = self.arena.alloc_node(NodeKind::IndexExpr, 0, span, lhs.0, idx.0);
                continue;
            }

            if k == TokenKind::Question {
                // Postfix try propagate: expr?
                self.advance();
                let span = self.arena.span(lhs);
                lhs = self.arena.alloc_node(NodeKind::TryPropagateExpr, 0, span, lhs.0, 0);
                continue;
            }

            if k == TokenKind::SafeCascade || (k == TokenKind::DotDot && self.is_cascade_lookahead()) {
                if (ExprPrecedence::Cascade as u8) < min_bp {
                    break;
                }
                let is_safe = k == TokenKind::SafeCascade;
                self.advance();
                let rhs = if self.at(TokenKind::KwLet) {
                    self.parse_binding_stmt(NodeKind::LetStmt)
                } else {
                    self.parse_expr_bp(ExprPrecedence::Cascade as u8 + 1)
                };
                let span = self.arena.span(lhs);
                lhs = self.arena.alloc_node(NodeKind::CascadeExpr, is_safe as u16, span, lhs.0, rhs.0);
                continue;
            }

            // Binary operators
            let (op, bp, r_assoc) = match self.get_binary_op(k) {
                Some(triple) => triple,
                None => break,
            };

            if bp < min_bp {
                break;
            }

            self.advance();
            let next_min_bp = if r_assoc { bp } else { bp + 1 };
            let rhs = self.parse_expr_bp(next_min_bp);
            let span = self.arena.span(lhs);
            lhs = self.arena.alloc_node(NodeKind::BinaryExpr, op as u16, span, lhs.0, rhs.0);
        }

        lhs
    }

    fn is_cascade_lookahead(&self) -> bool {
        if self.pos + 1 >= self.tape.len() {
            return false;
        }
        let next_k = self.tape.kind(self.pos + 1);
        if next_k == TokenKind::KwLet {
            return true;
        }
        if next_k == TokenKind::Ident || self.tape.flag(self.pos + 1).contains(TokenFlags::IS_CONTEXTUAL_IDENT) {
            if self.pos + 2 < self.tape.len() {
                let after_k = self.tape.kind(self.pos + 2);
                return matches!(
                    after_k,
                    TokenKind::LParen
                        | TokenKind::Equal
                        | TokenKind::Arrow
                        | TokenKind::DotDot
                        | TokenKind::SafeCascade
                ) || self.is_call_quote_start(after_k);
            }
        }
        false
    }

    fn parse_expr_prefix(&mut self) -> NodeId {
        let k = self.peek();
        let span = self.span();

        if self.peek_is_polyglot_block() {
            return self.parse_polyglot_block();
        }

        if self.is_ident_like() {
            let name = self.text();
            self.advance();
            return self.arena.alloc_ident(name, span);
        }

        match k {
            TokenKind::IntLit => {
                let txt = self.text();
                self.advance();
                let val: i64 = txt.replace('_', "").parse().unwrap_or(0);
                self.arena.alloc_int(val, span)
            }
            TokenKind::HexIntLit => {
                let txt = self.text();
                self.advance();
                let clean = txt.trim_start_matches("0x").trim_start_matches("0X").replace('_', "");
                let val = i64::from_str_radix(&clean, 16).unwrap_or(0);
                self.arena.alloc_int(val, span)
            }
            TokenKind::BinIntLit => {
                let txt = self.text();
                self.advance();
                let clean = txt.trim_start_matches("0b").trim_start_matches("0B").replace('_', "");
                let val = i64::from_str_radix(&clean, 2).unwrap_or(0);
                self.arena.alloc_int(val, span)
            }
            TokenKind::OctIntLit => {
                let txt = self.text();
                self.advance();
                let clean = txt.trim_start_matches("0o").trim_start_matches("0O").replace('_', "");
                let val = i64::from_str_radix(&clean, 8).unwrap_or(0);
                self.arena.alloc_int(val, span)
            }
            TokenKind::FloatLit => {
                let txt = self.text();
                self.advance();
                let val: f64 = txt.replace('_', "").parse().unwrap_or(0.0);
                self.arena.alloc_float(val, span)
            }
            TokenKind::StrLit | TokenKind::TripleStrLit => {
                let txt = self.text();
                self.advance();
                let clean = txt.trim_matches('"').trim_matches('\'');
                self.arena.alloc_str(clean, span)
            }
            TokenKind::KwTrue => {
                self.advance();
                self.arena.alloc_node(NodeKind::BoolLit, 0, span, 1, 0)
            }
            TokenKind::KwFalse => {
                self.advance();
                self.arena.alloc_node(NodeKind::BoolLit, 0, span, 0, 0)
            }
            TokenKind::KwNone | TokenKind::KwNull => {
                self.advance();
                self.arena.alloc_node(NodeKind::NoneLit, 0, span, 0, 0)
            }
            TokenKind::KeywordLit => {
                let txt = self.text();
                self.advance();
                self.arena.alloc_keyword(txt, span)
            }
            TokenKind::LParen => {
                // Check for C-style cast: `(type)` or `(type*)` or `(long long)` e.g. `(char*)malloc(...)`, `(int)x`, `(long long)count`
                let mut lookahead = self.pos + 1;
                while lookahead < self.tape.len()
                    && (self.tape.kind(lookahead) == TokenKind::Ident
                        || self.tape.flag(lookahead).contains(TokenFlags::IS_CONTEXTUAL_IDENT)
                        || self.tape.kind(lookahead) == TokenKind::Star
                        || self.tape.kind(lookahead) == TokenKind::KwConst)
                {
                    lookahead += 1;
                }
                if lookahead > self.pos + 1 && lookahead < self.tape.len() && self.tape.kind(lookahead) == TokenKind::RParen {
                    let next_k = if lookahead + 1 < self.tape.len() { self.tape.kind(lookahead + 1) } else { TokenKind::Eof };
                    if next_k == TokenKind::Ident
                        || matches!(next_k, TokenKind::IntLit | TokenKind::FloatLit | TokenKind::StrLit | TokenKind::LParen)
                        || self.tape.flag(lookahead + 1).contains(TokenFlags::IS_CONTEXTUAL_IDENT)
                    {
                        self.pos = lookahead + 1;
                        self.skip_whitespace_and_comments();
                        return self.parse_expr_prefix();
                    }
                }

                self.advance();
                let mut items = Vec::new();
                while !self.at_end() && !self.at(TokenKind::RParen) {
                    let start_pos = self.pos;
                    let item = self.parse_expr();
                    if item.is_some() {
                        items.push(item);
                    }
                    if self.at(TokenKind::Comma) {
                        self.advance();
                    }
                    if self.pos == start_pos {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RParen);
                if items.len() == 1 {
                    items[0]
                } else {
                    self.arena.alloc_list_node(NodeKind::TupleExpr, 0, ByteSpan::new(span.start, self.prev_span().end), &items)
                }
            }
            TokenKind::LBracket => {
                let mut depth = 0;
                let mut is_comp = false;
                let mut p = self.pos + 1;
                while p < self.tape.len() {
                    let tk = self.tape.kind(p);
                    match tk {
                        TokenKind::LBracket | TokenKind::LBrace | TokenKind::LParen => depth += 1,
                        TokenKind::RBracket => {
                            if depth == 0 {
                                break;
                            }
                            depth -= 1;
                        }
                        TokenKind::RBrace | TokenKind::RParen => {
                            if depth > 0 {
                                depth -= 1;
                            }
                        }
                        TokenKind::KwFor => {
                            if depth == 0 {
                                is_comp = true;
                                break;
                            }
                        }
                        _ => {}
                    }
                    p += 1;
                }

                if is_comp {
                    self.advance(); // consume '['
                    let yield_expr = self.parse_expr();
                    self.expect(TokenKind::KwFor);
                    let var_name = self.text();
                    self.advance();
                    let sym = self.arena.interner.intern(var_name);
                    if self.at(TokenKind::KwIn) {
                        self.advance();
                    }
                    let _iter = self.parse_expr();
                    let body = if self.at(TokenKind::KwIf) {
                        self.advance();
                        let filter_cond = self.parse_expr();
                        self.arena.alloc_node(NodeKind::IfExpr, 0, span, filter_cond.0, yield_expr.0)
                    } else {
                        yield_expr
                    };
                    self.expect(TokenKind::RBracket);
                    return self.arena.alloc_node(NodeKind::ForExpr, 0, span, sym.0, body.0);
                }

                self.advance();
                let mut items = Vec::new();
                while !self.at_end() && !self.at(TokenKind::RBracket) {
                    let start_pos = self.pos;
                    let item = self.parse_expr();
                    if item.is_some() {
                        items.push(item);
                    }
                    if self.at(TokenKind::Comma) {
                        self.advance();
                    }
                    if self.pos == start_pos {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RBracket);
                self.arena.alloc_list_node(NodeKind::ListExpr, 0, ByteSpan::new(span.start, self.prev_span().end), &items)
            }
            TokenKind::Minus
            | TokenKind::Plus
            | TokenKind::Tilde
            | TokenKind::Exclamation
            | TokenKind::KwNot
            | TokenKind::Amper
            | TokenKind::KwRef => {
                let is_amper = k == TokenKind::Amper || k == TokenKind::KwRef;
                self.advance();
                let operand = self.parse_expr_bp(ExprPrecedence::UnaryPrefix as u8);
                let flags = if is_amper { 1 } else { 0 };
                self.arena.alloc_node(NodeKind::UnaryExpr, flags, span, operand.0, 0)
            }
            TokenKind::KwIf => self.parse_if_expr(),
            TokenKind::KwMatch => self.parse_match_expr(),
            TokenKind::KwLambda => self.parse_lambda_expr(),
            TokenKind::KwLet | TokenKind::KwVar => {
                let is_binding = if self.pos + 2 < self.tape.len() {
                    let k2 = self.tape.kind(self.pos + 2);
                    k2 == TokenKind::Walrus || k2 == TokenKind::Equal
                } else {
                    false
                };
                if is_binding {
                    self.advance(); // consume let/var
                    let name = self.text();
                    self.advance();
                    let sym = self.arena.interner.intern(name);
                    let pat_node = self.arena.alloc_node(NodeKind::PatBind, 0, span, sym.0, 0);
                    self.advance(); // consume := or =
                    let val_node = self.parse_expr();
                    self.arena.alloc_node(NodeKind::BinaryExpr, BinaryOp::Walrus as u16, span, pat_node.0, val_node.0)
                } else {
                    NodeId::NONE
                }
            }
            TokenKind::LBrace | TokenKind::VirtualLBrace => {
                if k == TokenKind::LBrace {
                    let mut depth = 0;
                    let mut is_comp = false;
                    let mut p = self.pos + 1;
                    while p < self.tape.len() {
                        let tk = self.tape.kind(p);
                        match tk {
                            TokenKind::LBracket | TokenKind::LBrace | TokenKind::LParen => depth += 1,
                            TokenKind::RBrace => {
                                if depth == 0 {
                                    break;
                                }
                                depth -= 1;
                            }
                            TokenKind::RBracket | TokenKind::RParen => {
                                if depth > 0 {
                                    depth -= 1;
                                }
                            }
                            TokenKind::KwFor => {
                                if depth == 0 {
                                    is_comp = true;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        p += 1;
                    }

                    if is_comp {
                        self.advance(); // consume '{'
                        let key_or_val = self.parse_expr();
                        let yield_expr = if self.at(TokenKind::Colon) {
                            self.advance();
                            let val_expr = self.parse_expr();
                            self.arena.alloc_list_node(NodeKind::MapExpr, 0, span, &[key_or_val, val_expr])
                        } else {
                            key_or_val
                        };
                        self.expect(TokenKind::KwFor);
                        let var_name = self.text();
                        self.advance();
                        let sym = self.arena.interner.intern(var_name);
                        if self.at(TokenKind::KwIn) {
                            self.advance();
                        }
                        let _iter = self.parse_expr();
                        let body = if self.at(TokenKind::KwIf) {
                            self.advance();
                            let filter_cond = self.parse_expr();
                            self.arena.alloc_node(NodeKind::IfExpr, 0, span, filter_cond.0, yield_expr.0)
                        } else {
                            yield_expr
                        };
                        self.expect(TokenKind::RBrace);
                        return self.arena.alloc_node(NodeKind::ForExpr, 0, span, sym.0, body.0);
                    }
                }
                self.parse_block_expr()
            }
            _ => NodeId::NONE,
        }
    }

    fn is_call_quote_start(&self, k: TokenKind) -> bool {
        matches!(
            k,
            TokenKind::CallQuoteShallow
                | TokenKind::CallQuoteDeep
                | TokenKind::CallQuoteAbi
                | TokenKind::CallQuoteReact
                | TokenKind::CallQuoteAdjoint
                | TokenKind::CallQuoteEffect
        )
    }

    fn parse_call_expr(&mut self, callee: NodeId) -> NodeId {
        let start_span = self.arena.span(callee);
        let quote_kind = match self.peek() {
            TokenKind::CallQuoteShallow => CallQuoteKind::ShallowQuote,
            TokenKind::CallQuoteDeep => CallQuoteKind::DeepSplice,
            TokenKind::CallQuoteAbi => CallQuoteKind::AbiFrame,
            TokenKind::CallQuoteReact => CallQuoteKind::ReactiveSignal,
            TokenKind::CallQuoteAdjoint => CallQuoteKind::SymbolicAdjoint,
            TokenKind::CallQuoteEffect => CallQuoteKind::EffectDryRun,
            _ => CallQuoteKind::None,
        };

        if quote_kind != CallQuoteKind::None {
            self.advance();
        }

        self.expect(TokenKind::LParen);
        let mut args = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RParen) {
            let start_pos = self.pos;
            let arg = self.parse_expr();
            if arg.is_some() {
                args.push(arg);
            }
            if self.at(TokenKind::Comma) {
                self.advance();
            }
            if self.pos == start_pos {
                self.advance();
            }
        }
        self.expect(TokenKind::RParen);

        let mut all_call = Vec::with_capacity(1 + args.len());
        all_call.push(callee);
        all_call.extend(args);
        let (arg_start, arg_len) = self.arena.alloc_child_slice(&all_call);
        self.arena.alloc_node(
            NodeKind::CallExpr,
            quote_kind as u16,
            start_span,
            arg_start,
            arg_len,
        )
    }

    fn parse_if_expr(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'if'
        let cond = self.parse_expr();
        let then_branch = self.parse_block_expr();
        let else_branch = if self.at(TokenKind::KwElse) {
            self.advance();
            if self.at(TokenKind::KwIf) {
                self.parse_if_expr()
            } else {
                self.parse_block_expr()
            }
        } else {
            NodeId::NONE
        };

        if else_branch.is_some() {
            let (slice_start, slice_len) = self.arena.alloc_child_slice(&[then_branch, else_branch]);
            self.arena.alloc_node(NodeKind::IfExpr, slice_len as u16, start_span, cond.0, slice_start)
        } else {
            self.arena.alloc_node(NodeKind::IfExpr, 0, start_span, cond.0, then_branch.0)
        }
    }

    fn parse_match_expr(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'match'
        let scrutinee = self.parse_expr();
        self.expect(TokenKind::LBrace);
        let mut arms = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) {
            self.consume_semi_or_newlines();
            if self.at(TokenKind::RBrace) || self.at_end() {
                break;
            }
            if self.at(TokenKind::KwCase) {
                let arm_span = self.span();
                self.advance();
                let pat = self.parse_pattern();
                let has_guard = if self.at(TokenKind::KwIf) || self.at(TokenKind::KwWhen) {
                    self.advance();
                    let _ = self.parse_expr();
                    true
                } else {
                    false
                };
                if self.at(TokenKind::FatArrow) {
                    self.advance();
                }
                let body = self.parse_stmt_or_decl();
                let arm = self.arena.alloc_node(NodeKind::MatchArm, has_guard as u16, arm_span, pat.0, body.0);
                arms.push(arm);
            } else {
                self.advance();
            }
            self.consume_semi_or_newlines();
        }
        self.expect(TokenKind::RBrace);
        let mut all_match = Vec::with_capacity(1 + arms.len());
        all_match.push(scrutinee);
        all_match.extend(arms);
        let (start, len) = self.arena.alloc_child_slice(&all_match);
        self.arena.alloc_node(NodeKind::MatchExpr, 0, start_span, start, len)
    }

    fn parse_lambda_expr(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'lambda'
        let mut params = Vec::new();
        while !self.at_end() && !self.at(TokenKind::Colon) && !self.at(TokenKind::Arrow) {
            if self.is_ident_like() {
                let p_name = self.text();
                self.advance();
                let p_sym = self.arena.interner.intern(p_name);
                let p_node = self.arena.alloc_node(NodeKind::ParamDecl, 0, self.span(), p_sym.0, 0);
                params.push(p_node);
            } else {
                self.advance();
            }
            if self.at(TokenKind::Comma) {
                self.advance();
            }
        }
        if self.at(TokenKind::Colon) || self.at(TokenKind::Arrow) {
            self.advance();
        }
        let body = self.parse_expr();
        let (param_start, param_len) = self.arena.alloc_child_slice(&params);
        self.arena.alloc_node(NodeKind::LambdaExpr, param_len as u16, start_span, param_start, body.0)
    }

    fn get_binary_op(&self, k: TokenKind) -> Option<(BinaryOp, u8, bool)> {
        match k {
            TokenKind::Plus => Some((BinaryOp::Add, ExprPrecedence::AddSub as u8, false)),
            TokenKind::Minus => Some((BinaryOp::Sub, ExprPrecedence::AddSub as u8, false)),
            TokenKind::Star => Some((BinaryOp::Mul, ExprPrecedence::MulDivRem as u8, false)),
            TokenKind::Slash => Some((BinaryOp::Div, ExprPrecedence::MulDivRem as u8, false)),
            TokenKind::DoubleSlash => Some((BinaryOp::FloorDiv, ExprPrecedence::MulDivRem as u8, false)),
            TokenKind::Percent => Some((BinaryOp::Rem, ExprPrecedence::MulDivRem as u8, false)),
            TokenKind::DoubleStar => Some((BinaryOp::Pow, ExprPrecedence::Power as u8, true)),
            TokenKind::EqEqual => Some((BinaryOp::Eq, ExprPrecedence::Comparison as u8, false)),
            TokenKind::NotEqual => Some((BinaryOp::Ne, ExprPrecedence::Comparison as u8, false)),
            TokenKind::Less => Some((BinaryOp::Lt, ExprPrecedence::Comparison as u8, false)),
            TokenKind::LessEqual => Some((BinaryOp::Le, ExprPrecedence::Comparison as u8, false)),
            TokenKind::Greater => Some((BinaryOp::Gt, ExprPrecedence::Comparison as u8, false)),
            TokenKind::GreaterEqual => Some((BinaryOp::Ge, ExprPrecedence::Comparison as u8, false)),
            TokenKind::KwAnd => Some((BinaryOp::And, ExprPrecedence::LogicalAnd as u8, false)),
            TokenKind::KwOr => Some((BinaryOp::Or, ExprPrecedence::LogicalOr as u8, false)),
            TokenKind::Amper => Some((BinaryOp::BitAnd, ExprPrecedence::BitAnd as u8, false)),
            TokenKind::Vbar => Some((BinaryOp::BitOr, ExprPrecedence::BitOr as u8, false)),
            TokenKind::Caret => Some((BinaryOp::BitXor, ExprPrecedence::BitXor as u8, false)),
            TokenKind::ShiftLeft => Some((BinaryOp::Shl, ExprPrecedence::Shift as u8, false)),
            TokenKind::ShiftRight => Some((BinaryOp::Shr, ExprPrecedence::Shift as u8, false)),
            TokenKind::PipeForward => Some((BinaryOp::Pipe, ExprPrecedence::Pipe as u8, false)),
            TokenKind::Coalesce => Some((BinaryOp::Coalesce, ExprPrecedence::Coalesce as u8, true)),
            TokenKind::DotDot | TokenKind::DotDotEqual => Some((BinaryOp::RangeInclusive, ExprPrecedence::Range as u8, false)),
            TokenKind::DotDotLess => Some((BinaryOp::RangeHalfOpen, ExprPrecedence::Range as u8, false)),
            TokenKind::Walrus => Some((BinaryOp::Walrus, ExprPrecedence::Assign as u8, true)),
            _ => None,
        }
    }

    // =========================================================================
    // 30.1: Pratt Type-Spec Parsing Engine (parse_type_bp)
    // Enforcing Invariant #6: T? (30) > & (20) > | (15) > -> (10) > ! (5)
    // =========================================================================

    pub fn parse_type(&mut self) -> NodeId {
        self.parse_type_bp(TypePrecedence::Lowest as u8)
    }

    pub fn parse_type_bp(&mut self, min_bp: u8) -> NodeId {
        let mut lhs = self.parse_type_prefix();

        loop {
            let k = self.peek();
            if k == TokenKind::Star {
                let span = self.span();
                self.advance();
                let inner_node = self.arena.node(lhs);
                let inner_name = self.arena.interner.resolve(SymbolId(inner_node.lhs));
                let ptr_str = format!("{}*", inner_name);
                let ptr_sym = self.arena.interner.intern(&ptr_str);
                lhs = self.arena.alloc_node(NodeKind::TypeNamed, 0, span, ptr_sym.0, 0);
                continue;
            }
            if k == TokenKind::Question {
                // PostfixOptional: T? (bp = 30)
                if (TypePrecedence::PostfixOptional as u8) < min_bp {
                    break;
                }
                let span = self.span();
                self.advance();
                lhs = self.arena.alloc_node(NodeKind::TypeOptional, 0, span, lhs.0, 0);
                continue;
            }

            if k == TokenKind::Amper {
                // Intersection: A & B (bp = 20)
                if (TypePrecedence::Intersection as u8) < min_bp {
                    break;
                }
                let span = self.span();
                self.advance();
                let rhs = self.parse_type_bp(TypePrecedence::Intersection as u8 + 1);
                lhs = self.arena.alloc_node(NodeKind::TypeIntersection, 0, span, lhs.0, rhs.0);
                continue;
            }

            if k == TokenKind::Vbar {
                // Union: A | B (bp = 15)
                if (TypePrecedence::Union as u8) < min_bp {
                    break;
                }
                let span = self.span();
                self.advance();
                let rhs = self.parse_type_bp(TypePrecedence::Union as u8 + 1);
                lhs = self.arena.alloc_node(NodeKind::TypeUnion, 0, span, lhs.0, rhs.0);
                continue;
            }

            if k == TokenKind::Arrow {
                // FnArrow: (A) -> B (bp = 10)
                if (TypePrecedence::FnArrow as u8) < min_bp {
                    break;
                }
                let span = self.span();
                self.advance();
                let ret = self.parse_type_bp(TypePrecedence::FnArrow as u8);
                lhs = self.arena.alloc_node(NodeKind::TypeFn, 0, span, lhs.0, ret.0);
                continue;
            }

            if k == TokenKind::Exclamation {
                // Fallible: T ! E (bp = 5)
                if (TypePrecedence::Fallible as u8) < min_bp {
                    break;
                }
                let span = self.span();
                self.advance();
                let err = self.parse_type_bp(TypePrecedence::Fallible as u8 + 1);
                lhs = self.arena.alloc_node(NodeKind::TypeFallible, 0, span, lhs.0, err.0);
                continue;
            }

            if k == TokenKind::LBracket {
                let start_span = self.span();
                self.advance();
                let mut type_args = Vec::new();
                while !self.at_end() && !self.at(TokenKind::RBracket) {
                    let t = self.parse_type();
                    type_args.push(t);
                    if self.at(TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RBracket);
                let span = ByteSpan::new(start_span.start, self.prev_span().end);
                lhs = self.arena.alloc_list_node(NodeKind::TypeList, 0, span, &type_args);
                continue;
            }

            break;
        }

        lhs
    }

    fn parse_type_prefix(&mut self) -> NodeId {
        let span = self.span();
        if self.at(TokenKind::Star) {
            self.advance();
            let inner = self.parse_type_prefix();
            let ptr_str = if inner.is_some() {
                let in_node = self.arena.node(inner);
                let in_name = self.arena.interner.resolve(SymbolId(in_node.lhs));
                format!("*{}", in_name)
            } else {
                "*void".to_string()
            };
            let inner_sym = self.arena.interner.intern(&ptr_str);
            self.arena.alloc_node(NodeKind::TypeNamed, 0, span, inner_sym.0, 0)
        } else if self.is_ident_like() {
            let name = self.text();
            self.advance();
            let sym = self.arena.interner.intern(name);
            self.arena.alloc_node(NodeKind::TypeNamed, 0, span, sym.0, 0)
        } else if self.at(TokenKind::LBracket) {
            self.advance();
            let elem = self.parse_type();
            self.expect(TokenKind::RBracket);
            self.arena.alloc_node(NodeKind::TypeList, 0, span, elem.0, 0)
        } else if self.at(TokenKind::LBrace) {
            self.advance();
            let key = self.parse_type();
            self.expect(TokenKind::Colon);
            let val = self.parse_type();
            self.expect(TokenKind::RBrace);
            self.arena.alloc_node(NodeKind::TypeMap, 0, span, key.0, val.0)
        } else if self.at(TokenKind::LParen) {
            self.advance();
            let mut types = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RParen) {
                let t = self.parse_type();
                types.push(t);
                if self.at(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RParen);
            self.arena.alloc_list_node(NodeKind::TypeTuple, 0, span, &types)
        } else {
            self.advance();
            self.arena.alloc_node(NodeKind::TypeNamed, 0, span, 0, 0)
        }
    }

    // =========================================================================
    // 30.1: Pratt Pattern Parsing Engine
    // =========================================================================

    pub fn parse_pattern(&mut self) -> NodeId {
        let span = self.span();
        if self.at(TokenKind::Underscore) {
            self.advance();
            self.arena.alloc_node(NodeKind::PatWildcard, 0, span, 0, 0)
        } else if self.is_ident_like() {
            let mut name = self.text().to_string();
            self.advance();
            // Handle qualified variant, e.g. HttpResponse.Ok
            while self.at(TokenKind::Dot) {
                self.advance();
                if self.is_ident_like() {
                    name.push('.');
                    name.push_str(self.text());
                    self.advance();
                }
            }
            // Check if followed by constructor args `( ... )`
            if self.at(TokenKind::LParen) {
                self.advance();
                let mut sub_pats = Vec::new();
                while !self.at_end() && !self.at(TokenKind::RParen) {
                    let sub = self.parse_pattern();
                    sub_pats.push(sub);
                    if self.at(TokenKind::Comma) {
                        self.advance();
                    }
                }
                self.expect(TokenKind::RParen);
                let sym = self.arena.interner.intern(&name);
                let (start, len) = self.arena.alloc_child_slice(&sub_pats);
                self.arena.alloc_node(NodeKind::PatVariant, len as u16, span, sym.0, start)
            } else if name.chars().next().map_or(false, |c| c.is_uppercase()) || name.contains('.') {
                let sym = self.arena.interner.intern(&name);
                self.arena.alloc_node(NodeKind::PatVariant, 0, span, sym.0, 0)
            } else {
                let sym = self.arena.interner.intern(&name);
                self.arena.alloc_node(NodeKind::PatBind, 0, span, sym.0, 0)
            }
        } else if self.at(TokenKind::IntLit)
            || self.at(TokenKind::HexIntLit)
            || self.at(TokenKind::BinIntLit)
            || self.at(TokenKind::OctIntLit)
            || self.at(TokenKind::FloatLit)
            || self.at(TokenKind::StrLit)
            || self.at(TokenKind::TripleStrLit)
            || self.at(TokenKind::KwTrue)
            || self.at(TokenKind::KwFalse)
            || self.at(TokenKind::KwNone)
            || self.at(TokenKind::KwNull)
        {
            let lit = self.parse_expr_prefix();
            self.arena.alloc_node(NodeKind::PatLiteral, 0, span, lit.0, 0)
        } else if self.at(TokenKind::LParen) {
            self.advance();
            let mut items = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RParen) {
                let p = self.parse_pattern();
                items.push(p);
                if self.at(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RParen);
            self.arena.alloc_list_node(NodeKind::PatTuple, 0, span, &items)
        } else if self.at(TokenKind::LBracket) {
            self.advance();
            let mut items = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RBracket) {
                if self.at(TokenKind::DotDot) || self.at(TokenKind::Ellipsis) {
                    let dot_span = self.span();
                    self.advance();
                    if self.is_ident_like() {
                        let name = self.text();
                        self.advance();
                        let sym = self.arena.interner.intern(name);
                        let p = self.arena.alloc_node(NodeKind::PatBind, 0, dot_span, sym.0, 0);
                        items.push(p);
                    } else {
                        let p = self.arena.alloc_node(NodeKind::PatWildcard, 0, dot_span, 0, 0);
                        items.push(p);
                    }
                } else {
                    let p = self.parse_pattern();
                    items.push(p);
                }
                if self.at(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBracket);
            self.arena.alloc_list_node(NodeKind::PatSlice, 0, span, &items)
        } else {
            self.advance();
            self.arena.alloc_node(NodeKind::PatWildcard, 0, span, 0, 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_program_and_sexpr() {
        let code = "import std.math;\nlet x: i64 = 40 + 2;\nreturn x;\n";
        let arena = parse(code);

        assert!(!arena.nodes.is_empty());
        let sexpr = arena.to_sexpr(arena.root);
        assert_eq!(
            sexpr,
            "(Program (ImportDecl std.math) (LetStmt x (BinaryExpr op=1 (Int 40) (Int 2))) (ReturnStmt (Ident x)))"
        );
    }

    #[test]
    fn test_type_precedence_invariant_6() {
        // T? (30) > & (20) > | (15) > -> (10) > ! (5)
        // User? ! IoError | ParseError should parse as:
        // (TypeFallible (TypeOptional User) (TypeUnion IoError ParseError))
        let code = "type MyFallible = User? ! IoError | ParseError";
        let mut parser = Parser::new(code, tokenize(code));
        parser.advance(); // type
        parser.advance(); // MyFallible
        parser.advance(); // =
        let t = parser.parse_type();

        let n = parser.arena.node(t);
        assert_eq!(n.node_kind(), NodeKind::TypeFallible);

        let lhs = parser.arena.node(NodeId(n.lhs));
        assert_eq!(lhs.node_kind(), NodeKind::TypeOptional);

        let rhs = parser.arena.node(NodeId(n.rhs));
        assert_eq!(rhs.node_kind(), NodeKind::TypeUnion);
    }

    #[test]
    fn test_call_quoting_in_ast() {
        let code = "let res = compute!(10, 20) + compute!!(x) + compute!&(f);";
        let arena = parse(code);
        let calls: Vec<&AstNode> = arena
            .nodes
            .iter()
            .filter(|n| n.node_kind() == NodeKind::CallExpr)
            .collect();

        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].flags, CallQuoteKind::ShallowQuote as u16);
        assert_eq!(calls[1].flags, CallQuoteKind::DeepSplice as u16);
        assert_eq!(calls[2].flags, CallQuoteKind::AbiFrame as u16);
    }

    #[test]
    fn test_parse_01_type_safety() {
        let src = std::fs::read_to_string("../../tests/01_type_safety.pilot").unwrap();
        let arena = parse(&src);
        assert!(arena.nodes.len() > 10);
    }
}
