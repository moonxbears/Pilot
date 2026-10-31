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
        if lookahead < self.tape.len() && self.tape.kind(lookahead) == TokenKind::LParen {
            let close_p = self.tape.bracket_links[lookahead] as usize;
            if close_p > lookahead && close_p < self.tape.len() {
                lookahead = close_p + 1;
            }
        }
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
        let mut conn_node = NodeId::NONE;
        if self.at(TokenKind::LParen) {
            self.advance();
            conn_node = self.parse_expr();
            if self.at(TokenKind::RParen) {
                self.advance();
            }
            self.skip_whitespace_and_comments();
        }
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
        let block_id = self.arena.alloc_polyglot_block(&lang, &code_str, flags, full_span);
        if conn_node.is_some() {
            self.arena.polyglot_conns.insert(block_id.0, conn_node);
        }
        block_id
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

        // Labeled loop statement (`outer_loop: for ...`) vs typed field declaration (`x: f64`)
        if self.is_ident_like() && self.peek_ahead(1) == TokenKind::Colon && self.peek_ahead(2) != TokenKind::Colon {
            let label_name = self.text();
            if label_name != "default" && label_name != "case" {
                let after_colon = self.peek_ahead(2);
                if matches!(after_colon, TokenKind::KwFor | TokenKind::KwWhile) {
                    self.advance(); // consume label
                    self.advance(); // consume ':'
                    return self.parse_stmt_or_decl();
                } else if after_colon == TokenKind::Ident
                    || after_colon.is_keyword()
                    || matches!(after_colon, TokenKind::LBracket | TokenKind::LParen | TokenKind::Star)
                {
                    let start_span = self.span();
                    let field_name = self.text();
                    self.advance(); // consume field name
                    let sym = self.arena.interner.intern(field_name);
                    self.advance(); // consume ':'
                    let ty_node = self.parse_type();
                    let init = if self.at(TokenKind::Equal) {
                        self.advance();
                        self.parse_expr()
                    } else {
                        NodeId::NONE
                    };
                    if self.at(TokenKind::Comma) {
                        self.advance();
                    }
                    let end_pos = self.prev_span().end.max(start_span.end);
                    let node_id = self.arena.alloc_node(
                        NodeKind::LetStmt,
                        0,
                        ByteSpan::new(start_span.start, end_pos),
                        sym.0,
                        init.0,
                    );
                    if ty_node.is_some() {
                        self.arena.type_annotations.insert(node_id.0, ty_node);
                    }
                    return node_id;
                }
            }
        }

        if self.at(TokenKind::KwComptime) || (self.is_ident_like() && self.text() == "comptime") {
            let next_pos = self.pos + 1;
            if next_pos < self.tape.len() {
                let nk = self.tape.kind(next_pos);
                if nk == TokenKind::KwDef || nk == TokenKind::KwFn {
                    self.advance();
                    return self.parse_fn_decl();
                } else if nk == TokenKind::KwAssert {
                    self.advance();
                    return self.parse_assert_stmt();
                }
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
            } else if txt == "and" {
                let next_pos = self.pos + 1;
                if next_pos < self.tape.len() && (self.tape.kind(next_pos) == TokenKind::KwDef || self.tape.kind(next_pos) == TokenKind::KwFn) {
                    self.advance();
                    return self.parse_fn_decl();
                }
            } else if txt == "noalloc" || txt == "region" {
                self.advance();
                if self.is_ident_like() {
                    self.advance();
                }
                return self.parse_block_expr();
            }
        }

        if self.peek_is_indexer_decl() {
            return self.parse_indexer_decl(self.span());
        }

        let k = self.peek();
        match k {
            TokenKind::KwImport => self.parse_import_stmt(),
            TokenKind::KwFrom => {
                if self.is_from_import_lookahead() {
                    self.parse_from_import_stmt()
                } else {
                    self.parse_query_expr()
                }
            }
            TokenKind::KwDatabase => self.parse_database_stmt(),
            TokenKind::KwTransaction => self.parse_transaction_stmt(),
            TokenKind::KwSavepoint | TokenKind::KwCommit | TokenKind::KwRollback => self.parse_transaction_op_stmt(),
            TokenKind::KwInsert | TokenKind::KwUpdate | TokenKind::KwDelete => self.parse_sql_mutation_stmt(),
            TokenKind::KwDef | TokenKind::KwFn | TokenKind::KwDefkeyword => self.parse_fn_decl(),
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
        let cond = self.parse_expr();
        if self.at(TokenKind::Comma) {
            self.advance();
            let _ = self.parse_expr();
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

    fn is_from_import_lookahead(&self) -> bool {
        let mut p = self.pos + 1;
        while p < self.tape.len() {
            let k = self.tape.kind(p);
            if k == TokenKind::KwImport {
                return true;
            }
            if k == TokenKind::Newline || k == TokenKind::Semicolon || k == TokenKind::RBrace || k == TokenKind::Eof {
                return false;
            }
            p += 1;
        }
        false
    }

    fn parse_database_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'database'
        if self.is_ident_like() && self.peek_ahead(1) == TokenKind::Equal {
            let name = self.text();
            self.advance();
            let sym = self.arena.interner.intern(name);
            self.advance(); // consume '='
            let init = self.parse_expr();
            let end_pos = self.prev_span().end.max(start_span.end);
            self.arena.alloc_node(NodeKind::VarStmt, 0, ByteSpan::new(start_span.start, end_pos), sym.0, init.0)
        } else {
            self.parse_expr_stmt()
        }
    }

    fn parse_transaction_stmt(&mut self) -> NodeId {
        self.advance(); // consume 'transaction'
        if self.at(TokenKind::LBrace) || self.at(TokenKind::VirtualLBrace) {
            self.parse_block_expr()
        } else {
            self.parse_stmt_or_decl()
        }
    }

    fn parse_transaction_op_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'savepoint', 'commit', or 'rollback'
        while !self.at_end() && !self.at(TokenKind::Newline) && !self.at(TokenKind::Semicolon) && !self.at(TokenKind::RBrace) {
            let prev_end = self.span().end as usize;
            self.advance();
            let cur_start = self.span().start as usize;
            if prev_end <= cur_start && cur_start <= self.src.len() && self.src[prev_end..cur_start].contains('\n') {
                break;
            }
        }
        let end_pos = self.prev_span().end.max(start_span.end);
        let raw_txt = &self.src[start_span.start as usize..end_pos as usize];
        let sym = self.arena.interner.intern(raw_txt);
        let ident = self.arena.alloc_node(NodeKind::IdentExpr, 0, ByteSpan::new(start_span.start, end_pos), sym.0, 0);
        self.arena.alloc_node(NodeKind::ExprStmt, 0, ByteSpan::new(start_span.start, end_pos), ident.0, 0)
    }

    fn parse_sql_mutation_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        while !self.at_end() && !self.at(TokenKind::Newline) && !self.at(TokenKind::Semicolon) && !self.at(TokenKind::RBrace) {
            let prev_end = self.span().end as usize;
            self.advance();
            let cur_start = self.span().start as usize;
            if prev_end <= cur_start && cur_start <= self.src.len() && self.src[prev_end..cur_start].contains('\n') {
                break;
            }
        }
        let end_pos = self.prev_span().end.max(start_span.end);
        let raw_sql = &self.src[start_span.start as usize..end_pos as usize];
        let block_id = self.arena.alloc_polyglot_block("sql", raw_sql, 0, ByteSpan::new(start_span.start, end_pos));
        self.arena.alloc_node(NodeKind::ExprStmt, 0, ByteSpan::new(start_span.start, end_pos), block_id.0, 0)
    }

    fn parse_query_expr(&mut self) -> NodeId {
        let start_span = self.span();
        let mut depth_paren = 0;
        let mut depth_bracket = 0;
        let mut depth_brace = 0;

        while !self.at_end() {
            let k = self.peek();
            match k {
                TokenKind::LParen => depth_paren += 1,
                TokenKind::RParen => {
                    if depth_paren == 0 {
                        break;
                    }
                    depth_paren -= 1;
                }
                TokenKind::LBracket => depth_bracket += 1,
                TokenKind::RBracket => {
                    if depth_bracket == 0 {
                        break;
                    }
                    depth_bracket -= 1;
                }
                TokenKind::LBrace | TokenKind::VirtualLBrace => depth_brace += 1,
                TokenKind::RBrace | TokenKind::VirtualRBrace => {
                    if depth_brace == 0 {
                        break;
                    }
                    depth_brace -= 1;
                }
                TokenKind::Semicolon => break,
                TokenKind::KwUnion | TokenKind::KwIntersect | TokenKind::KwExcept => {
                    if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 {
                        break;
                    }
                }
                TokenKind::Newline => {
                    if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 {
                        // Lookahead to see if next line continues the query
                        let mut p = self.pos + 1;
                        while p < self.tape.len() && self.tape.kind(p) == TokenKind::Newline {
                            p += 1;
                        }
                        if p < self.tape.len() {
                            let next_k = self.tape.kind(p);
                            let is_continuation = matches!(
                                next_k,
                                TokenKind::KwWhere
                                    | TokenKind::KwOrder
                                    | TokenKind::KwGroup
                                    | TokenKind::KwSelect
                                    | TokenKind::KwLimit
                                    | TokenKind::KwOffset
                                    | TokenKind::KwJoin
                                    | TokenKind::KwFrom
                                    | TokenKind::KwHaving
                                    | TokenKind::KwWith
                                    | TokenKind::KwInto
                                    | TokenKind::KwOn
                                    | TokenKind::KwAs
                                    | TokenKind::KwAsc
                                    | TokenKind::KwDesc
                                    | TokenKind::KwAscending
                                    | TokenKind::KwDescending
                            ) || (next_k == TokenKind::Ident && {
                                let txt = &self.src[self.tape.span(p).start as usize..self.tape.span(p).end as usize];
                                matches!(txt, "where" | "order" | "group" | "select" | "limit" | "offset" | "join" | "from" | "having" | "with" | "into" | "on" | "as" | "ascending" | "descending" | "asc" | "desc")
                            });
                            if !is_continuation {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
                _ => {}
            }
            self.advance();
        }

        let end_pos = self.prev_span().end.max(start_span.start);
        let raw_query = &self.src[start_span.start as usize..end_pos as usize];
        let full_span = ByteSpan::new(start_span.start, end_pos);
        self.arena.alloc_polyglot_block("sql", raw_query, 0, full_span)
    }

    fn parse_from_import_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'from'
        let mut path = String::new();
        while !self.at_end() && !self.at(TokenKind::KwImport) && !self.at(TokenKind::Newline) && !self.at(TokenKind::Semicolon) {
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
                    || self.at(TokenKind::KwComptime)
                    || (self.is_ident_like() && (self.text() == "context" || self.text() == "var" || self.text() == "ref" || self.text() == "val" || self.text() == "comptime"))
                {
                    if self.pos + 1 < self.tape.len() {
                        let next_k = self.tape.kind(self.pos + 1);
                        if next_k == TokenKind::Colon || next_k == TokenKind::Comma || next_k == TokenKind::RParen || next_k == TokenKind::Equal {
                            break;
                        }
                    }
                    self.advance();
                }
                if self.is_ident_like() {
                    let p_span = self.span();
                    let p_name = self.text();
                    self.advance();
                    let p_sym = self.arena.interner.intern(p_name);
                    let mut type_node = NodeId::NONE;
                    if self.at(TokenKind::Colon) {
                        self.advance();
                        type_node = self.parse_type();
                    }
                    if self.at(TokenKind::Equal) {
                        self.advance();
                        let _ = self.parse_expr();
                    }
                    let p_end = self.prev_span().end.max(p_span.end);
                    let p_node = self.arena.alloc_node(
                        NodeKind::ParamDecl,
                        0,
                        ByteSpan::new(p_span.start, p_end),
                        p_sym.0,
                        type_node.0,
                    );
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
        let mut ret_type_node = NodeId::NONE;
        if self.at(TokenKind::Arrow) {
            self.advance();
            ret_type_node = self.parse_type();
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

        let fn_node = self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, start_span, sym.0, fn_start);
        if ret_type_node.is_some() {
            self.arena.type_annotations.insert(fn_node.0, ret_type_node);
        }
        fn_node
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
        if self.at(TokenKind::LParen) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::RParen) {
                self.advance();
            }
            if self.at(TokenKind::RParen) {
                self.advance();
            }
        }
        if self.at(TokenKind::Colon) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::Newline) && !self.at(TokenKind::Equal) {
                self.advance();
            }
        }
        if self.at(TokenKind::Less) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::Equal) {
                self.advance();
            }
        }
        if self.at(TokenKind::Equal) {
            self.advance(); // consume '='
            let expr = self.parse_expr();
            self.consume_semi_or_newlines();
            let span = ByteSpan::new(start_span.start, self.arena.span(expr).end);
            return self.arena.alloc_node(NodeKind::ClassDecl, 0, span, sym.0, expr.0);
        }
        let body = self.parse_block_expr();
        let span = ByteSpan::new(start_span.start, self.arena.span(body).end);
        self.arena.alloc_node(NodeKind::ClassDecl, 0, span, sym.0, body.0)
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
        let span = ByteSpan::new(start_span.start, self.arena.span(body).end);
        self.arena.alloc_node(NodeKind::StructDecl, 0, span, sym.0, body.0)
    }

    fn parse_record_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'record'
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);
        // Parameters / fields: (x: i64, y: i64)
        let mut fields = Vec::new();
        if self.at(TokenKind::LParen) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::RParen) {
                if self.is_ident_like() {
                    let f_span = self.span();
                    let f_name = self.text();
                    self.advance();
                    let f_sym = self.arena.interner.intern(f_name);
                    let mut ty_node = NodeId::NONE;
                    if self.at(TokenKind::Colon) {
                        self.advance();
                        ty_node = self.parse_type();
                    }
                    let f_end = self.prev_span().end.max(f_span.end);
                    let _ = self.arena.alloc_node(
                        NodeKind::ParamDecl,
                        0,
                        ByteSpan::new(f_span.start, f_end),
                        f_sym.0,
                        ty_node.0,
                    );
                    fields.push((f_sym, ty_node));
                } else {
                    self.advance();
                }
                if self.at(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RParen);
        }
        let body = if self.at(TokenKind::LBrace) || self.at(TokenKind::Colon) {
            self.parse_block_expr()
        } else {
            NodeId::NONE
        };
        let rec_node = self.arena.alloc_node(NodeKind::RecordDecl, 0, start_span, sym.0, body.0);
        if !fields.is_empty() {
            self.arena.record_fields.insert(rec_node.0, fields);
        }
        rec_node
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
        if self.at(TokenKind::LParen) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::RParen) {
                self.advance();
            }
            if self.at(TokenKind::RParen) {
                self.advance();
            }
        }
        if self.at(TokenKind::Less) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::Equal) {
                self.advance();
            }
        }
        let body = self.parse_block_expr();
        let span = ByteSpan::new(start_span.start, self.arena.span(body).end);
        self.arena.alloc_node(NodeKind::InterfaceDecl, 0, span, sym.0, body.0)
    }

    fn parse_extend_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'extend' or 'extension'
        if self.at(TokenKind::LBracket) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::RBracket) {
                self.advance();
            }
            if self.at(TokenKind::RBracket) {
                self.advance();
            }
        }
        let mut target = self.text().to_string();
        self.advance();
        if self.at(TokenKind::KwOn) || (self.is_ident_like() && self.text() == "on") {
            self.advance();
            if !self.at_end() {
                target = self.text().to_string();
                self.advance();
            }
        }
        if self.at(TokenKind::LBracket) && self.peek_ahead(1) == TokenKind::RBracket {
            self.advance();
            self.advance();
            target = "List".to_string();
        }
        let sym = self.arena.interner.intern(&target);
        while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::Colon) && !self.at(TokenKind::Newline) {
            self.advance();
        }
        let body = self.parse_block_expr();
        let span = ByteSpan::new(start_span.start, self.arena.span(body).end);
        self.arena.alloc_node(NodeKind::ExtendDecl, 0, span, sym.0, body.0)
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
        if self.at(TokenKind::KwMut) || self.at(TokenKind::KwImmut) {
            self.advance();
        }
        if self.at(TokenKind::LParen) || self.at(TokenKind::LBracket) || self.at(TokenKind::LBrace) {
            let pat = self.parse_pattern();
            let mut ty_node = NodeId::NONE;
            if self.at(TokenKind::Colon) {
                self.advance();
                ty_node = self.parse_type();
            }
            let init = if self.at(TokenKind::Equal) {
                self.advance();
                self.parse_expr()
            } else {
                NodeId::NONE
            };
            let end_pos = self.prev_span().end.max(start_span.end);
            let node_id = self.arena.alloc_node(
                kind,
                1,
                ByteSpan::new(start_span.start, end_pos),
                pat.0,
                init.0,
            );
            if ty_node.is_some() {
                self.arena.type_annotations.insert(node_id.0, ty_node);
            }
            return node_id;
        }
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);

        // Optional type annotation: : type_spec
        let mut ty_node = NodeId::NONE;
        if self.at(TokenKind::Colon) {
            self.advance();
            ty_node = self.parse_type();
        }

        let init = if self.at(TokenKind::Equal) {
            self.advance();
            self.parse_expr()
        } else {
            NodeId::NONE
        };

        let end_pos = self.prev_span().end.max(start_span.end);
        let node_id = self.arena.alloc_node(
            kind,
            0,
            ByteSpan::new(start_span.start, end_pos),
            sym.0,
            init.0,
        );
        if ty_node.is_some() {
            self.arena.type_annotations.insert(node_id.0, ty_node);
        }
        node_id
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
        if !self.at(TokenKind::Semicolon)
            && !self.at(TokenKind::Newline)
            && !self.at(TokenKind::RBrace)
            && !self.at(TokenKind::VirtualRBrace)
            && !self.at_end()
        {
            if self.is_ident_like() && (self.peek_ahead(1) == TokenKind::Semicolon || self.peek_ahead(1) == TokenKind::Newline) {
                let name = self.text();
                let sym = self.arena.interner.intern(name);
                self.advance();
                return self.arena.alloc_node(NodeKind::BreakStmt, 0, span, sym.0, 0);
            }
            let val = self.parse_expr();
            return self.arena.alloc_node(NodeKind::BreakStmt, 1, span, val.0, 0);
        }
        self.arena.alloc_node(NodeKind::BreakStmt, 0, span, 0, 0)
    }

    fn parse_continue_stmt(&mut self) -> NodeId {
        let span = self.span();
        self.advance();
        while !self.at(TokenKind::Semicolon)
            && !self.at(TokenKind::Newline)
            && !self.at(TokenKind::RBrace)
            && !self.at(TokenKind::VirtualRBrace)
            && !self.at_end()
        {
            let start_pos = self.pos;
            if self.at(TokenKind::Comma) || self.at(TokenKind::Ellipsis) || self.at(TokenKind::DotDot) {
                self.advance();
            } else {
                let _ = self.parse_expr();
            }
            if self.pos == start_pos {
                self.advance();
            }
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

    fn peek_is_indexer_decl(&self) -> bool {
        if !self.is_ident_like() || self.text() != "this" {
            return false;
        }
        if self.peek_ahead(1) != TokenKind::LBracket {
            return false;
        }
        let mut depth = 0;
        let mut idx = 1;
        while self.pos + idx < self.tape.len() {
            let k = self.tape.kind(self.pos + idx);
            if k == TokenKind::LBracket {
                depth += 1;
            } else if k == TokenKind::RBracket {
                depth -= 1;
                if depth == 0 {
                    idx += 1;
                    while self.pos + idx < self.tape.len() && self.tape.kind(self.pos + idx) == TokenKind::Newline {
                        idx += 1;
                    }
                    if self.pos + idx < self.tape.len() {
                        let after_k = self.tape.kind(self.pos + idx);
                        if after_k == TokenKind::Colon || after_k == TokenKind::LBrace || after_k == TokenKind::FatArrow {
                            return true;
                        }
                    }
                    return false;
                }
            } else if k == TokenKind::Semicolon || k == TokenKind::Eof {
                return false;
            }
            idx += 1;
        }
        false
    }

    fn parse_property_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'property'
        if self.is_ident_like() && self.text() == "this" && self.peek_ahead(1) == TokenKind::LBracket {
            return self.parse_indexer_decl(start_span);
        }
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);
        let mut prop_ty = NodeId::NONE;
        if self.at(TokenKind::Colon) {
            self.advance();
            prop_ty = self.parse_type();
        }
        if self.at(TokenKind::FatArrow) {
            self.advance();
            let expr = self.parse_expr();
            let ret = self.arena.alloc_node(NodeKind::ReturnStmt, 0, start_span, expr.0, 0);
            let body = self.arena.alloc_list_node(NodeKind::BlockExpr, 0, start_span, &[ret]);
            let fn_node = self.arena.alloc_node(NodeKind::FnDecl, 0, start_span, sym.0, body.0);
            if prop_ty.is_some() {
                self.arena.type_annotations.insert(fn_node.0, prop_ty);
            }
            return fn_node;
        }
        if self.at(TokenKind::LBrace) {
            self.advance();
            let mut stmts = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RBrace) {
                let loop_start_pos = self.pos;
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
                        self.parse_expr_stmt()
                    } else {
                        self.parse_block_expr()
                    };
                    let set_sym = self.arena.interner.intern("set");
                    let p_node = self.arena.alloc_node(NodeKind::ParamDecl, 0, span, p_sym.0, prop_ty.0);
                    let (fn_start, fn_len) = self.arena.alloc_child_slice(&[set_body, p_node]);
                    let setter_fn = self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, span, set_sym.0, fn_start);
                    stmts.push(setter_fn);
                } else {
                    stmts.push(self.parse_stmt_or_decl());
                }
                self.consume_semi_or_newlines();
                if self.pos == loop_start_pos {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBrace);
            let body = self.arena.alloc_list_node(NodeKind::BlockExpr, 0, start_span, &stmts);
            let fn_node = self.arena.alloc_node(NodeKind::FnDecl, 0, start_span, sym.0, body.0);
            if prop_ty.is_some() {
                self.arena.type_annotations.insert(fn_node.0, prop_ty);
            }
            return fn_node;
        }
        let body = self.parse_block_expr();
        let fn_node = self.arena.alloc_node(NodeKind::FnDecl, 0, start_span, sym.0, body.0);
        if prop_ty.is_some() {
            self.arena.type_annotations.insert(fn_node.0, prop_ty);
        }
        fn_node
    }

    fn parse_indexer_decl(&mut self, start_span: ByteSpan) -> NodeId {
        self.advance(); // consume 'this'
        self.expect(TokenKind::LBracket);

        let mut params = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBracket) {
            self.consume_semi_or_newlines();
            if self.at(TokenKind::RBracket) || self.at_end() {
                break;
            }
            if self.is_ident_like() {
                let p_span = self.span();
                let p_name = self.text();
                self.advance();
                let p_sym = self.arena.interner.intern(p_name);
                let mut type_node = NodeId::NONE;
                if self.at(TokenKind::Colon) {
                    self.advance();
                    type_node = self.parse_type();
                }
                let p_end = self.prev_span().end.max(p_span.end);
                let p_node = self.arena.alloc_node(
                    NodeKind::ParamDecl,
                    0,
                    ByteSpan::new(p_span.start, p_end),
                    p_sym.0,
                    type_node.0,
                );
                params.push(p_node);
            } else {
                self.advance();
            }
            if self.at(TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(TokenKind::RBracket);

        let mut prop_ty = NodeId::NONE;
        if self.at(TokenKind::Colon) {
            self.advance();
            prop_ty = self.parse_type();
        }

        self.consume_semi_or_newlines();

        let indexer_sym = self.arena.interner.intern("this[]");

        if self.at(TokenKind::FatArrow) {
            self.advance();
            let expr = self.parse_expr();
            let ret = self.arena.alloc_node(NodeKind::ReturnStmt, 0, start_span, expr.0, 0);
            let body = self.arena.alloc_list_node(NodeKind::BlockExpr, 0, start_span, &[ret]);
            let mut all_fn = Vec::with_capacity(1 + params.len());
            all_fn.push(body);
            all_fn.extend(params);
            let (fn_start, fn_len) = self.arena.alloc_child_slice(&all_fn);
            let end_span = self.prev_span();
            let fn_span = ByteSpan::new(start_span.start, end_span.end);
            let fn_node = self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, fn_span, indexer_sym.0, fn_start);
            if prop_ty.is_some() {
                self.arena.type_annotations.insert(fn_node.0, prop_ty);
            }
            return fn_node;
        }

        if self.at(TokenKind::LBrace) {
            self.advance();
            let mut stmts = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RBrace) {
                let loop_start_pos = self.pos;
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
                    let get_sym = self.arena.interner.intern("get");
                    let mut get_all = Vec::with_capacity(1 + params.len());
                    get_all.push(get_body);
                    get_all.extend(params.clone());
                    let (fn_start, fn_len) = self.arena.alloc_child_slice(&get_all);
                    let getter_fn = self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, span, get_sym.0, fn_start);
                    if prop_ty.is_some() {
                        self.arena.type_annotations.insert(getter_fn.0, prop_ty);
                    }
                    stmts.push(getter_fn);
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
                        self.parse_expr_stmt()
                    } else {
                        self.parse_block_expr()
                    };
                    let set_sym = self.arena.interner.intern("set");
                    let val_node = self.arena.alloc_node(NodeKind::ParamDecl, 0, span, p_sym.0, prop_ty.0);
                    let mut set_all = Vec::with_capacity(1 + params.len() + 1);
                    set_all.push(set_body);
                    set_all.extend(params.clone());
                    set_all.push(val_node);
                    let (fn_start, fn_len) = self.arena.alloc_child_slice(&set_all);
                    let setter_fn = self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, span, set_sym.0, fn_start);
                    stmts.push(setter_fn);
                } else {
                    stmts.push(self.parse_stmt_or_decl());
                }
                self.consume_semi_or_newlines();
                if self.pos == loop_start_pos {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBrace);
            let body = self.arena.alloc_list_node(NodeKind::BlockExpr, 0, start_span, &stmts);
            let mut all_fn = Vec::with_capacity(1 + params.len());
            all_fn.push(body);
            all_fn.extend(params);
            let (fn_start, fn_len) = self.arena.alloc_child_slice(&all_fn);
            let end_span = self.prev_span();
            let fn_span = ByteSpan::new(start_span.start, end_span.end);
            let fn_node = self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, fn_span, indexer_sym.0, fn_start);
            if prop_ty.is_some() {
                self.arena.type_annotations.insert(fn_node.0, prop_ty);
            }
            return fn_node;
        }

        let body = self.parse_block_expr();
        let mut all_fn = Vec::with_capacity(1 + params.len());
        all_fn.push(body);
        all_fn.extend(params);
        let (fn_start, fn_len) = self.arena.alloc_child_slice(&all_fn);
        let end_span = self.prev_span();
        let fn_span = ByteSpan::new(start_span.start, end_span.end);
        let fn_node = self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, fn_span, indexer_sym.0, fn_start);
        if prop_ty.is_some() {
            self.arena.type_annotations.insert(fn_node.0, prop_ty);
        }
        fn_node
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
                    let p_span = self.span();
                    let p_name = self.text();
                    self.advance();
                    let p_sym = self.arena.interner.intern(p_name);
                    let mut ty_node = NodeId::NONE;
                    if self.at(TokenKind::Colon) {
                        self.advance();
                        ty_node = self.parse_type();
                    }
                    let p_end = self.prev_span().end.max(p_span.end);
                    let p_node = self.arena.alloc_node(
                        NodeKind::ParamDecl,
                        0,
                        ByteSpan::new(p_span.start, p_end),
                        p_sym.0,
                        ty_node.0,
                    );
                    params.push(p_node);
                } else {
                    self.advance();
                }
                if self.at(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RParen);
            let mut ret_ty = NodeId::NONE;
            if self.at(TokenKind::Arrow) {
                self.advance();
                ret_ty = self.parse_type();
            }
            let body = self.parse_block_expr();
            let mut all_fn = Vec::with_capacity(1 + params.len());
            all_fn.push(body);
            all_fn.extend(params);
            let (fn_start, fn_len) = self.arena.alloc_child_slice(&all_fn);
            let fn_node = self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, start_span, sym.0, fn_start);
            if ret_ty.is_some() {
                self.arena.type_annotations.insert(fn_node.0, ret_ty);
            }
            return fn_node;
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
                    let p_span = self.span();
                    let p_name = self.text();
                    self.advance();
                    let p_sym = self.arena.interner.intern(p_name);
                    let mut ty_node = NodeId::NONE;
                    if self.at(TokenKind::Colon) {
                        self.advance();
                        ty_node = self.parse_type();
                    }
                    let p_end = self.prev_span().end.max(p_span.end);
                    let p_node = self.arena.alloc_node(
                        NodeKind::ParamDecl,
                        0,
                        ByteSpan::new(p_span.start, p_end),
                        p_sym.0,
                        ty_node.0,
                    );
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
        let mut ret_ty = NodeId::NONE;
        if self.at(TokenKind::Arrow) {
            self.advance();
            ret_ty = self.parse_type();
        }
        let body = self.parse_block_expr();
        let mut all_fn = Vec::with_capacity(1 + params.len());
        all_fn.push(body);
        all_fn.extend(params);
        let (fn_start, fn_len) = self.arena.alloc_child_slice(&all_fn);
        let fn_node = self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, start_span, sym.0, fn_start);
        if ret_ty.is_some() {
            self.arena.type_annotations.insert(fn_node.0, ret_ty);
        }
        fn_node
    }

    fn parse_adapter_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'adapter'
        let mut name_parts = Vec::new();
        while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::Colon) && !self.at(TokenKind::Newline) {
            if self.is_ident_like() {
                name_parts.push(self.text().to_string());
            }
            self.advance();
        }
        if self.at(TokenKind::Colon) {
            self.advance();
        }
        let body = self.parse_block_expr();
        let adapter_name = if name_parts.is_empty() {
            "__pilot_adapter".to_string()
        } else {
            format!("__pilot_adapter_{}", name_parts.join("_"))
        };
        let sym = self.arena.interner.intern(&adapter_name);
        let (fn_start, fn_len) = self.arena.alloc_child_slice(&[body]);
        self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, start_span, sym.0, fn_start)
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
                    let _chan_expr = self.parse_expr_bp(ExprPrecedence::Assign as u8 + 1);
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
                    let _duration = self.parse_expr_bp(ExprPrecedence::Assign as u8 + 1);
                    if self.at(TokenKind::FatArrow) {
                        self.advance();
                    }
                    let body = self.parse_block_expr();
                    arms.push(body);
                } else if self.at(TokenKind::KwDefault) || (self.is_ident_like() && self.text() == "default") {
                    self.advance();
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
        let cond = if self.at(TokenKind::KwLet) || self.at(TokenKind::KwSome) || (self.is_ident_like() && self.text() == "some") {
            self.advance(); // consume 'let' or 'some'
            if self.is_ident_like() && (self.peek_ahead(1) == TokenKind::KwElse || self.peek_ahead(1) == TokenKind::LBrace) {
                // In-place `guard some acct else { ... }`
                self.parse_expr()
            } else {
                let pat = self.parse_pattern();
                if self.at(TokenKind::Equal) || self.at(TokenKind::Walrus) {
                    self.advance();
                }
                let val_node = self.parse_expr();
                self.arena.alloc_node(NodeKind::BinaryExpr, BinaryOp::Walrus as u16, start_span, pat.0, val_node.0)
            }
        } else {
            self.parse_expr()
        };
        let else_body = if self.at(TokenKind::KwElse) {
            self.advance();
            self.parse_block_expr()
        } else {
            NodeId::NONE
        };
        self.arena.alloc_node(NodeKind::GuardStmt, 0, start_span, cond.0, else_body.0)
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
        if self.at(TokenKind::KwBounded) || (self.is_ident_like() && self.text() == "bounded") {
            self.advance();
            let _bound = self.parse_expr();
        }
        let body = self.parse_block_expr();
        self.arena.alloc_node(NodeKind::WhileStmt, 0, start_span, cond.0, body.0)
    }

    fn parse_for_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'for'
        let has_paren = self.at(TokenKind::LParen);
        if has_paren {
            self.advance();
        }
        let var_name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(var_name);
        while self.at(TokenKind::Comma) {
            self.advance();
            if self.is_ident_like() {
                self.advance();
            }
        }
        self.expect(TokenKind::KwIn);
        let iter_expr = self.parse_expr();
        if has_paren && self.at(TokenKind::RParen) {
            self.advance();
        }
        if self.at(TokenKind::KwBounded) || (self.is_ident_like() && self.text() == "bounded") {
            self.advance();
            let _bound = self.parse_expr();
        }
        let body = self.parse_block_expr();
        if self.at(TokenKind::KwThen) {
            self.advance();
            let _ = self.parse_expr();
        }
        let for_node = self.arena.alloc_node(NodeKind::ForExpr, 0, start_span, sym.0, body.0);
        if iter_expr.is_some() {
            self.arena.for_iter_nodes.insert(for_node.0, iter_expr);
        }
        for_node
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
        let mut lhs = self.parse_expr_prefix(min_bp);
        if lhs.is_none() {
            return NodeId::NONE;
        }

        loop {
            if self.peek() == TokenKind::Newline {
                let mut p = self.pos;
                while p < self.tape.len() && self.tape.kind(p) == TokenKind::Newline {
                    p += 1;
                }
                if p < self.tape.len() {
                    let nk = self.tape.kind(p);
                    if nk == TokenKind::Dot
                        || nk == TokenKind::SafeNav
                        || nk == TokenKind::PipeForward
                        || nk == TokenKind::SafeCascade
                        || (nk == TokenKind::DotDot && self.is_cascade_lookahead_at(p))
                    {
                        self.pos = p;
                    }
                }
            }

            let k = self.peek();
            // Handle unit expression `<m/s**2>` immediately following a numeric literal
            if k == TokenKind::Less
                && matches!(self.arena.node(lhs).node_kind(), NodeKind::IntLit | NodeKind::FloatLit)
            {
                let mut p = self.pos + 1;
                let mut is_unit_expr = false;
                while p < self.tape.len() {
                    let tk = self.tape.kind(p);
                    if tk == TokenKind::Greater {
                        is_unit_expr = p > self.pos + 1;
                        p += 1;
                        break;
                    } else if tk == TokenKind::Ident
                        || tk.is_keyword()
                        || matches!(
                            tk,
                            TokenKind::Slash
                                | TokenKind::Star
                                | TokenKind::DoubleStar
                                | TokenKind::Caret
                                | TokenKind::IntLit
                                | TokenKind::Minus
                        )
                    {
                        p += 1;
                    } else {
                        break;
                    }
                }
                if is_unit_expr {
                    self.pos = p;
                    continue;
                }
            }

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

            if k == TokenKind::Star && (self.peek_ahead(1) == TokenKind::LParen || self.is_call_quote_start(self.peek_ahead(1))) {
                // Multi-cast broadcast `fn*(args)`
                self.advance(); // consume '*'
                lhs = self.parse_call_expr(lhs);
                continue;
            }

            if k == TokenKind::LParen || self.is_call_quote_start(k) {
                let prev_end = self.arena.span(lhs).end as usize;
                let cur_start = self.span().start as usize;
                if prev_end <= cur_start && cur_start <= self.src.len() && self.src[prev_end..cur_start].contains('\n') {
                    break;
                }
                lhs = self.parse_call_expr(lhs);
                continue;
            }

            if k == TokenKind::LBracket {
                let prev_end = self.arena.span(lhs).end as usize;
                let cur_start = self.span().start as usize;
                if prev_end <= cur_start && cur_start <= self.src.len() && self.src[prev_end..cur_start].contains('\n') {
                    break;
                }
                self.advance();
                let mut indices = Vec::new();
                if !self.at(TokenKind::RBracket) {
                    indices.push(self.parse_expr());
                    while self.at(TokenKind::Comma) {
                        self.advance();
                        if self.at(TokenKind::RBracket) {
                            break;
                        }
                        indices.push(self.parse_expr());
                    }
                }
                self.expect(TokenKind::RBracket);
                let idx = if indices.is_empty() {
                    NodeId::NONE
                } else if indices.len() == 1 {
                    indices[0]
                } else {
                    let tuple_span = ByteSpan::new(
                        self.arena.span(indices[0]).start,
                        self.arena.span(*indices.last().unwrap()).end,
                    );
                    self.arena.alloc_list_node(NodeKind::TupleExpr, 0, tuple_span, &indices)
                };
                let span = ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end);
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

            if k == TokenKind::KwCatch {
                // Values-First Error Catch:
                // 1) `expr catch 8080`
                // 2) `expr catch err => match err { ... }`
                // 3) `expr catch err { ... }`
                if (ExprPrecedence::Coalesce as u8) < min_bp {
                    break;
                }
                let catch_span = self.span();
                self.advance(); // consume 'catch'
                if self.is_ident_like()
                    && (self.peek_ahead(1) == TokenKind::FatArrow || self.peek_ahead(1) == TokenKind::LBrace)
                {
                    let err_name = self.text();
                    let err_span = self.span();
                    self.advance(); // consume err var
                    let err_sym = self.arena.interner.intern(err_name);
                    let p_node = self.arena.alloc_node(NodeKind::ParamDecl, 0, err_span, err_sym.0, 0);
                    let handler = if self.at(TokenKind::FatArrow) {
                        self.advance();
                        self.parse_expr()
                    } else {
                        self.parse_block_expr()
                    };
                    let (p_start, p_len) = self.arena.alloc_child_slice(&[p_node]);
                    let lambda = self.arena.alloc_node(NodeKind::LambdaExpr, p_len as u16, catch_span, p_start, handler.0);
                    let span = self.arena.span(lhs);
                    lhs = self.arena.alloc_node(NodeKind::BinaryExpr, BinaryOp::Coalesce as u16, span, lhs.0, lambda.0);
                } else {
                    let fallback = self.parse_expr_bp(ExprPrecedence::Coalesce as u8);
                    let span = self.arena.span(lhs);
                    lhs = self.arena.alloc_node(NodeKind::BinaryExpr, BinaryOp::Coalesce as u16, span, lhs.0, fallback.0);
                }
                continue;
            }

            if k == TokenKind::Arrow {
                // Structural projection / cascade output capture:
                // 1) `..checksum_tag() -> let tag`
                // 2) `heavy_worker -> { full_name, department }`
                // 3) `pkt -> ControlSample`
                if (ExprPrecedence::Pipe as u8) < min_bp {
                    break;
                }
                self.advance(); // consume '->'
                if self.at(TokenKind::KwLet) || self.at(TokenKind::KwVar) {
                    let bind_span = self.span();
                    self.advance(); // consume 'let' or 'var'
                    let name = self.text();
                    self.advance();
                    let sym = self.arena.interner.intern(name);
                    let pat_node = self.arena.alloc_node(NodeKind::PatBind, 0, bind_span, sym.0, 0);
                    let span = self.arena.span(lhs);
                    lhs = self.arena.alloc_node(NodeKind::BinaryExpr, BinaryOp::Walrus as u16, span, pat_node.0, lhs.0);
                } else if self.at(TokenKind::LBrace) {
                    let proj_span = self.span();
                    self.advance(); // consume '{'
                    let mut pairs = Vec::new();
                    while !self.at_end() && !self.at(TokenKind::RBrace) {
                        if self.is_ident_like() {
                            let f_span = self.span();
                            let f_name = self.text();
                            self.advance();
                            let key = self.arena.alloc_str(f_name, f_span);
                            let sym = self.arena.interner.intern(f_name);
                            let val = self.arena.alloc_node(NodeKind::MemberExpr, 0, f_span, lhs.0, sym.0);
                            pairs.push(key);
                            pairs.push(val);
                        } else {
                            self.advance();
                        }
                        if self.at(TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RBrace);
                    lhs = self.arena.alloc_list_node(NodeKind::MapExpr, 0, proj_span, &pairs);
                } else {
                    let target = self.parse_expr_bp(ExprPrecedence::Pipe as u8 + 1);
                    let (arg_start, arg_len) = self.arena.alloc_child_slice(&[target, lhs]);
                    let span = self.arena.span(lhs);
                    lhs = self.arena.alloc_node(NodeKind::CallExpr, 0, span, arg_start, arg_len);
                }
                continue;
            }

            if k == TokenKind::KwIf && min_bp <= ExprPrecedence::Lowest as u8 + 1 {
                let prev_end = self.arena.span(lhs).end as usize;
                let cur_start = self.span().start as usize;
                let same_line_as_lhs = prev_end <= cur_start
                    && cur_start <= self.src.len()
                    && !self.src[prev_end..cur_start].contains('\n');
                if same_line_as_lhs {
                    // Pythonic inline conditional `true if n == 0 else is_odd_step(n - 1)`
                    // Only match if followed by `else` on the same logical expression level (not a comprehension filter `[x for x in list if cond]`)
                    let mut p = self.pos + 1;
                    let mut depth = 0;
                    let mut has_else = false;
                    while p < self.tape.len() {
                        if depth == 0 && p > 0 {
                            let pe = self.tape.span(p - 1).end as usize;
                            let cs = self.tape.span(p).start as usize;
                            if pe <= cs && cs <= self.src.len() && self.src[pe..cs].contains('\n') {
                                break;
                            }
                        }
                        let tk = self.tape.kind(p);
                        match tk {
                            TokenKind::LParen | TokenKind::LBracket => depth += 1,
                            TokenKind::LBrace => {
                                if depth == 0 {
                                    break;
                                }
                                depth += 1;
                            }
                            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                                if depth == 0 {
                                    break;
                                }
                                depth -= 1;
                            }
                            TokenKind::Semicolon | TokenKind::Newline if depth == 0 => break,
                            TokenKind::KwElse if depth == 0 => {
                                has_else = true;
                                break;
                            }
                            _ => {}
                        }
                        p += 1;
                    }
                    if has_else {
                        let if_span = self.span();
                        self.advance(); // consume 'if'
                        let cond = self.parse_expr_bp(ExprPrecedence::LogicalOr as u8);
                        let else_val = if self.at(TokenKind::KwElse) {
                            self.advance();
                            self.parse_expr_bp(ExprPrecedence::Lowest as u8)
                        } else {
                            NodeId::NONE
                        };
                        let (slice_start, slice_len) = self.arena.alloc_child_slice(&[lhs, else_val]);
                        lhs = self.arena.alloc_node(NodeKind::IfExpr, slice_len as u16, if_span, cond.0, slice_start);
                        continue;
                    }
                }
            }

            if k == TokenKind::SafeCascade || (k == TokenKind::DotDot && self.is_cascade_lookahead()) {
                if (ExprPrecedence::Cascade as u8) < min_bp {
                    break;
                }
                let is_safe = k == TokenKind::SafeCascade;
                self.advance();
                let rhs = if self.at(TokenKind::KwLet) || self.at(TokenKind::KwVar) {
                    let start_span = self.span();
                    let kind = if self.at(TokenKind::KwVar) { NodeKind::VarStmt } else { NodeKind::LetStmt };
                    self.advance(); // consume 'let' or 'var'
                    let name = self.text();
                    self.advance();
                    let sym = self.arena.interner.intern(name);
                    let init = if self.at(TokenKind::Equal) {
                        self.advance();
                        self.parse_expr_bp(ExprPrecedence::Cascade as u8 + 1)
                    } else {
                        NodeId::NONE
                    };
                    let end_pos = self.prev_span().end.max(start_span.end);
                    self.arena.alloc_node(kind, 0, ByteSpan::new(start_span.start, end_pos), sym.0, init.0)
                } else {
                    let expr = self.parse_expr_bp(ExprPrecedence::Cascade as u8 + 1);
                    if self.at(TokenKind::Equal) {
                        self.advance();
                        let val = self.parse_expr_bp(ExprPrecedence::Cascade as u8 + 1);
                        let span = self.arena.span(expr);
                        self.arena.alloc_node(NodeKind::AssignStmt, 0, span, expr.0, val.0)
                    } else {
                        expr
                    }
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
            if op == BinaryOp::Union && (self.at(TokenKind::KwAll) || (self.is_ident_like() && self.text() == "all")) {
                self.advance();
            }
            let next_min_bp = if r_assoc { bp } else { bp + 1 };
            let rhs = self.parse_expr_bp(next_min_bp);
            let span = self.arena.span(lhs);
            lhs = self.arena.alloc_node(NodeKind::BinaryExpr, op as u16, span, lhs.0, rhs.0);
        }

        lhs
    }

    fn is_cascade_lookahead(&self) -> bool {
        self.is_cascade_lookahead_at(self.pos)
    }

    fn is_cascade_lookahead_at(&self, dot_pos: usize) -> bool {
        if dot_pos + 1 >= self.tape.len() {
            return false;
        }
        let next_k = self.tape.kind(dot_pos + 1);
        if next_k == TokenKind::KwLet || next_k == TokenKind::KwVar {
            return true;
        }
        if next_k == TokenKind::Ident || self.tape.flag(dot_pos + 1).contains(TokenFlags::IS_CONTEXTUAL_IDENT) {
            if dot_pos + 2 < self.tape.len() {
                let after_k = self.tape.kind(dot_pos + 2);
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

    fn parse_expr_prefix(&mut self, min_bp: u8) -> NodeId {
        let k = self.peek();
        let span = self.span();

        if self.peek_is_polyglot_block() {
            return self.parse_polyglot_block();
        }

        if self.at(TokenKind::KwEach) {
            self.advance();
            let operand = self.parse_expr_bp(ExprPrecedence::UnaryPrefix as u8);
            return self.arena.alloc_node(NodeKind::UnaryExpr, 0, span, operand.0, 0);
        }

        if self.is_ident_like() {
            // Check for C# single-parameter lambda: x => expr
            if min_bp <= (ExprPrecedence::Assign as u8)
                && self.pos + 1 < self.tape.len()
                && self.tape.kind(self.pos + 1) == TokenKind::FatArrow
            {
                let p_name = self.text();
                self.advance(); // consume ident
                let sym = self.arena.interner.intern(p_name);
                let p_node = self.arena.alloc_node(NodeKind::ParamDecl, 0, span, sym.0, 0);
                self.advance(); // consume '=>'
                let body = if self.at(TokenKind::LBrace) || self.at(TokenKind::VirtualLBrace) {
                    self.parse_block_expr()
                } else {
                    self.parse_expr()
                };
                let (param_start, param_len) = self.arena.alloc_child_slice(&[p_node]);
                return self.arena.alloc_node(NodeKind::LambdaExpr, param_len as u16, span, param_start, body.0);
            }

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
            TokenKind::DurationLit | TokenKind::FrequencyLit | TokenKind::MemoryLit => {
                let txt = self.text();
                self.advance();
                let unit_sym = self.arena.interner.intern(txt);
                let kind_flag = match k {
                    TokenKind::DurationLit => 1,
                    TokenKind::FrequencyLit => 2,
                    TokenKind::MemoryLit => 3,
                    _ => 0,
                };
                self.arena.alloc_node(NodeKind::UnitLit, kind_flag, span, unit_sym.0, 0)
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
                // Check for reified operator `(+)` / `(-)` / `(*)` / `(/)`
                if self.pos + 2 < self.tape.len()
                    && self.tape.kind(self.pos + 2) == TokenKind::RParen
                    && matches!(
                        self.tape.kind(self.pos + 1),
                        TokenKind::Plus
                            | TokenKind::Minus
                            | TokenKind::Star
                            | TokenKind::Slash
                            | TokenKind::Percent
                            | TokenKind::EqEqual
                    )
                {
                    self.advance(); // consume '('
                    let op_txt = self.text();
                    self.advance(); // consume op
                    self.advance(); // consume ')'
                    return self.arena.alloc_ident(op_txt, span);
                }

                // Check for C# parenthesized lambda: `(params) => expr` or `() => expr`
                let mut p = self.pos + 1;
                let mut paren_depth = 0;
                let mut is_lambda = false;
                while p < self.tape.len() {
                    let tk = self.tape.kind(p);
                    match tk {
                        TokenKind::LParen => paren_depth += 1,
                        TokenKind::RParen => {
                            if paren_depth == 0 {
                                if p + 1 < self.tape.len() && self.tape.kind(p + 1) == TokenKind::FatArrow {
                                    is_lambda = true;
                                }
                                break;
                            }
                            paren_depth -= 1;
                        }
                        _ => {}
                    }
                    p += 1;
                }

                if is_lambda {
                    self.advance(); // consume '('
                    let mut params = Vec::new();
                    while !self.at_end() && !self.at(TokenKind::RParen) {
                        while self.at(TokenKind::KwVar)
                            || self.at(TokenKind::KwRef)
                            || self.at(TokenKind::KwMut)
                            || (self.is_ident_like() && (self.text() == "ref" || self.text() == "val" || self.text() == "var" || self.text() == "mut"))
                        {
                            self.advance();
                        }
                        if self.is_ident_like() {
                            let p_span = self.span();
                            let p_name = self.text();
                            self.advance();
                            let p_sym = self.arena.interner.intern(p_name);
                            let type_node = if self.at(TokenKind::Colon) {
                                self.advance();
                                self.parse_type()
                            } else {
                                NodeId::NONE
                            };
                            let p_end = self.prev_span().end.max(p_span.end);
                            let p_node = self.arena.alloc_node(
                                NodeKind::ParamDecl,
                                0,
                                ByteSpan::new(p_span.start, p_end),
                                p_sym.0,
                                type_node.0,
                            );
                            params.push(p_node);
                        } else {
                            self.advance();
                        }
                        if self.at(TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RParen);
                    self.expect(TokenKind::FatArrow);
                    let body = if self.at(TokenKind::LBrace) || self.at(TokenKind::VirtualLBrace) {
                        self.parse_block_expr()
                    } else {
                        self.parse_expr()
                    };
                    let (param_start, param_len) = self.arena.alloc_child_slice(&params);
                    return self.arena.alloc_node(NodeKind::LambdaExpr, param_len as u16, span, param_start, body.0);
                }

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
                        return self.parse_expr_prefix(min_bp);
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
                    let iter_expr = self.parse_expr();
                    let body = if self.at(TokenKind::KwIf) {
                        self.advance();
                        let filter_cond = self.parse_expr();
                        self.arena.alloc_node(NodeKind::IfExpr, 0, span, filter_cond.0, yield_expr.0)
                    } else {
                        yield_expr
                    };
                    self.expect(TokenKind::RBracket);
                    let for_node = self.arena.alloc_node(NodeKind::ForExpr, 0, span, sym.0, body.0);
                    if iter_expr.is_some() {
                        self.arena.for_iter_nodes.insert(for_node.0, iter_expr);
                    }
                    return for_node;
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
            TokenKind::Dot => {
                // Fluent member piping shorthand: `|> .Trim()`
                self.advance(); // consume '.'
                if self.is_ident_like() {
                    let name = self.text();
                    self.advance();
                    let self_node = self.arena.alloc_ident("self", span);
                    let sym = self.arena.interner.intern(name);
                    self.arena.alloc_node(NodeKind::MemberExpr, 0, span, self_node.0, sym.0)
                } else {
                    NodeId::NONE
                }
            }
            TokenKind::DotDot | TokenKind::Ellipsis => {
                self.advance();
                let operand = self.parse_expr_bp(ExprPrecedence::UnaryPrefix as u8);
                self.arena.alloc_node(NodeKind::UnaryExpr, 2, span, operand.0, 0)
            }
            TokenKind::Minus
            | TokenKind::Plus
            | TokenKind::Tilde
            | TokenKind::Exclamation
            | TokenKind::KwNot
            | TokenKind::KwEach
            | TokenKind::Amper
            | TokenKind::KwRef
            | TokenKind::ArrowLeft => {
                let flags = match k {
                    TokenKind::Amper | TokenKind::KwRef => 1,
                    TokenKind::ArrowLeft => 3,
                    TokenKind::Tilde => 4,
                    TokenKind::Exclamation | TokenKind::KwNot => 5,
                    _ => 0,
                };
                self.advance();
                let operand = self.parse_expr_bp(ExprPrecedence::UnaryPrefix as u8);
                self.arena.alloc_node(NodeKind::UnaryExpr, flags, span, operand.0, 0)
            }
            TokenKind::KwComptime => {
                self.advance(); // consume 'comptime'
                if self.at(TokenKind::LBrace) {
                    self.parse_block_expr()
                } else {
                    self.parse_expr_prefix(min_bp)
                }
            }
            TokenKind::KwFor => self.parse_for_stmt(),
            TokenKind::KwIf => self.parse_if_expr(),
            TokenKind::KwMatch => self.parse_match_expr(),
            TokenKind::KwLambda => self.parse_lambda_expr(),
            TokenKind::KwFrom => self.parse_query_expr(),
            TokenKind::KwSelect => {
                if self.peek_ahead(1) == TokenKind::LBrace {
                    self.parse_select_stmt()
                } else {
                    self.parse_query_expr()
                }
            }
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
                        let iter_expr = self.parse_expr();
                        let body = if self.at(TokenKind::KwIf) {
                            self.advance();
                            let filter_cond = self.parse_expr();
                            self.arena.alloc_node(NodeKind::IfExpr, 0, span, filter_cond.0, yield_expr.0)
                        } else {
                            yield_expr
                        };
                        self.expect(TokenKind::RBrace);
                        let for_node = self.arena.alloc_node(NodeKind::ForExpr, 0, span, sym.0, body.0);
                        if iter_expr.is_some() {
                            self.arena.for_iter_nodes.insert(for_node.0, iter_expr);
                        }
                        return for_node;
                    }

                    // Check for Map literal `{ k1: v1, k2: v2, ... }` or `{}`
                    let is_map_lit = if self.pos + 1 < self.tape.len() && self.tape.kind(self.pos + 1) == TokenKind::RBrace {
                        true
                    } else if self.pos + 2 < self.tape.len() {
                        let k1 = self.tape.kind(self.pos + 1);
                        let k2 = self.tape.kind(self.pos + 2);
                        k2 == TokenKind::Colon
                            && (matches!(
                                k1,
                                TokenKind::StrLit
                                    | TokenKind::KeywordLit
                                    | TokenKind::IntLit
                                    | TokenKind::HexIntLit
                                    | TokenKind::Ident
                            ) || self.tape.flag(self.pos + 1).contains(TokenFlags::IS_CONTEXTUAL_IDENT))
                    } else {
                        false
                    };

                    if is_map_lit {
                        self.advance(); // consume '{'
                        let mut pairs = Vec::new();
                        while !self.at_end() && !self.at(TokenKind::RBrace) {
                            let start_pos = self.pos;
                            let key = self.parse_expr();
                            if self.at(TokenKind::Colon) {
                                self.advance();
                                let val = self.parse_expr();
                                if key.is_some() {
                                    pairs.push(key);
                                }
                                if val.is_some() {
                                    pairs.push(val);
                                }
                            } else if key.is_some() {
                                pairs.push(key);
                            }
                            if self.at(TokenKind::Comma) {
                                self.advance();
                            }
                            if self.pos == start_pos {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::RBrace);
                        return self.arena.alloc_list_node(
                            NodeKind::MapExpr,
                            0,
                            ByteSpan::new(span.start, self.prev_span().end),
                            &pairs,
                        );
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
            // Support named arguments `param_name = value` or `param_name: value`
            if self.is_ident_like()
                && self.pos + 1 < self.tape.len()
                && (self.tape.kind(self.pos + 1) == TokenKind::Equal || self.tape.kind(self.pos + 1) == TokenKind::Colon)
            {
                self.advance(); // consume param_name
                self.advance(); // consume '=' or ':'
            }
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
        self.advance(); // consume 'if' or 'elif'
        let cond = if self.at(TokenKind::KwLet) || self.at(TokenKind::KwSome) || (self.is_ident_like() && self.text() == "some") {
            self.advance(); // consume 'let' or 'some'
            let pat = self.parse_pattern();
            if self.at(TokenKind::Equal) || self.at(TokenKind::Walrus) {
                self.advance();
                let rhs = self.parse_expr();
                self.arena.alloc_node(NodeKind::BinaryExpr, BinaryOp::Walrus as u16, start_span, pat.0, rhs.0)
            } else {
                pat
            }
        } else {
            self.parse_expr()
        };
        let then_branch = self.parse_block_expr();
        let else_branch = if self.at(TokenKind::KwElif) {
            self.parse_if_expr()
        } else if self.at(TokenKind::KwElse) {
            self.advance();
            if self.at(TokenKind::KwIf) || self.at(TokenKind::KwElif) {
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
        let scrutinee = if self.at(TokenKind::LBrace) {
            NodeId::NONE
        } else {
            self.parse_expr()
        };
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
                let p_span = self.span();
                let p_name = self.text();
                self.advance();
                let p_sym = self.arena.interner.intern(p_name);
                let p_node = self.arena.alloc_node(NodeKind::ParamDecl, 0, p_span, p_sym.0, 0);
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
            TokenKind::KwIs => Some((BinaryOp::Is, ExprPrecedence::Comparison as u8, false)),
            TokenKind::KwIn => Some((BinaryOp::Eq, ExprPrecedence::Comparison as u8, false)),
            TokenKind::Walrus => Some((BinaryOp::Walrus, ExprPrecedence::Assign as u8, true)),
            TokenKind::ArrowLeft => Some((BinaryOp::Send, ExprPrecedence::Pipe as u8, false)),
            TokenKind::KwUnion => Some((BinaryOp::Union, ExprPrecedence::BitOr as u8, false)),
            TokenKind::KwIntersect => Some((BinaryOp::Intersect, ExprPrecedence::BitOr as u8, false)),
            TokenKind::KwExcept => Some((BinaryOp::Except, ExprPrecedence::BitOr as u8, false)),
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
                // Fallible: T ! E or inferred error set T! (bp = 5)
                if (TypePrecedence::Fallible as u8) < min_bp {
                    break;
                }
                let span = self.span();
                self.advance();
                let next_k = self.peek();
                let err = if matches!(
                    next_k,
                    TokenKind::LBrace
                        | TokenKind::VirtualLBrace
                        | TokenKind::Colon
                        | TokenKind::Equal
                        | TokenKind::FatArrow
                        | TokenKind::Comma
                        | TokenKind::RParen
                        | TokenKind::RBracket
                        | TokenKind::Newline
                        | TokenKind::Semicolon
                        | TokenKind::KwRequires
                        | TokenKind::KwEnsures
                        | TokenKind::Eof
                ) {
                    NodeId::NONE
                } else {
                    self.parse_type_bp(TypePrecedence::Fallible as u8 + 1)
                };
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
                let base_name = if lhs.is_some() && self.arena.node(lhs).node_kind() == NodeKind::TypeNamed {
                    self.arena
                        .interner
                        .resolve(SymbolId(self.arena.node(lhs).lhs))
                        .to_string()
                } else {
                    String::new()
                };
                lhs = match base_name.as_str() {
                    "Map" | "map" | "Dict" | "dict" | "HashMap" | "NativeMap" if type_args.len() >= 2 => {
                        self.arena.alloc_node(NodeKind::TypeMap, 0, span, type_args[0].0, type_args[1].0)
                    }
                    "Option" | "option" if !type_args.is_empty() => {
                        self.arena.alloc_node(NodeKind::TypeOptional, 0, span, type_args[0].0, 0)
                    }
                    "Result" | "result" if !type_args.is_empty() => {
                        let err_id = if type_args.len() >= 2 { type_args[1].0 } else { 0 };
                        self.arena.alloc_node(NodeKind::TypeFallible, 0, span, type_args[0].0, err_id)
                    }
                    "Tuple" | "tuple" => {
                        self.arena.alloc_list_node(NodeKind::TypeTuple, 0, span, &type_args)
                    }
                    _ => {
                        if type_args.is_empty() {
                            self.arena.alloc_node(NodeKind::TypeSlice, 0, span, lhs.0, 0)
                        } else if type_args.len() == 1 {
                            self.arena.alloc_node(NodeKind::TypeList, 0, span, type_args[0].0, 0)
                        } else {
                            self.arena.alloc_list_node(NodeKind::TypeList, 0, span, &type_args)
                        }
                    }
                };
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
        } else if self.is_ident_like() || self.at(TokenKind::KeywordLit) {
            let name = self.text();
            self.advance();
            let sym = self.arena.interner.intern(name);
            self.arena.alloc_node(NodeKind::TypeNamed, 0, span, sym.0, 0)
        } else if self.at(TokenKind::LBracket) {
            self.advance();
            let elem = self.parse_type();
            if self.at(TokenKind::Semicolon) {
                self.advance();
                let _len = self.parse_expr();
            }
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
        let mut lhs = self.parse_pattern_atom();
        while self.at(TokenKind::Vbar) {
            let span = self.span();
            self.advance(); // consume '|'
            let rhs = self.parse_pattern_atom();
            lhs = self.arena.alloc_node(NodeKind::PatOr, 0, span, lhs.0, rhs.0);
        }
        lhs
    }

    fn parse_pattern_atom(&mut self) -> NodeId {
        let span = self.span();
        if self.at(TokenKind::Underscore) {
            self.advance();
            self.arena.alloc_node(NodeKind::PatWildcard, 0, span, 0, 0)
        } else if self.at(TokenKind::KeywordLit) {
            let name = self.text().to_string();
            self.advance();
            if self.at(TokenKind::LParen) {
                self.advance();
                let mut sub_pats = Vec::new();
                while !self.at_end() && !self.at(TokenKind::RParen) {
                    if self.is_ident_like() && self.peek_ahead(1) == TokenKind::Colon {
                        self.advance();
                        self.advance();
                    }
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
            } else {
                let lit = self.arena.alloc_keyword(&name, span);
                self.arena.alloc_node(NodeKind::PatLiteral, 0, span, lit.0, 0)
            }
        } else if self.is_ident_like() || self.at(TokenKind::KwSome) {
            let mut name = self.text().to_string();
            self.advance();
            // Handle pattern alias `sq @ Rectangle(w, h)`
            if self.at(TokenKind::At) {
                self.advance(); // consume '@'
                let sub = self.parse_pattern_atom();
                let sym = self.arena.interner.intern(&name);
                return self.arena.alloc_node(NodeKind::PatBind, 1, span, sym.0, sub.0);
            }
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
                    if self.is_ident_like() && self.peek_ahead(1) == TokenKind::Colon {
                        self.advance();
                        self.advance();
                    }
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
        } else if self.at(TokenKind::Minus)
            || self.at(TokenKind::IntLit)
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
            let lit = self.parse_expr_prefix(ExprPrecedence::Lowest as u8);
            if self.at(TokenKind::DotDot) || self.at(TokenKind::DotDotLess) || self.at(TokenKind::DotDotEqual) {
                self.advance();
                let end_lit = self.parse_expr_prefix(ExprPrecedence::Lowest as u8);
                self.arena.alloc_node(NodeKind::PatRange, 0, span, lit.0, end_lit.0)
            } else {
                self.arena.alloc_node(NodeKind::PatLiteral, 0, span, lit.0, 0)
            }
        } else if self.at(TokenKind::LBrace) {
            self.advance();
            let mut items = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RBrace) {
                let start_pos = self.pos;
                if (self.at(TokenKind::StrLit) || self.at(TokenKind::KeywordLit) || self.is_ident_like())
                    && self.peek_ahead(1) == TokenKind::Colon
                {
                    self.advance(); // consume key
                    self.advance(); // consume ':'
                    let sub = self.parse_pattern();
                    items.push(sub);
                } else {
                    let sub = self.parse_pattern();
                    items.push(sub);
                }
                if self.at(TokenKind::Comma) {
                    self.advance();
                }
                if self.pos == start_pos {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBrace);
            self.arena.alloc_list_node(NodeKind::PatTuple, 0, span, &items)
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
                        let rest_span = ByteSpan::new(dot_span.start, self.prev_span().end);
                        let p = self.arena.alloc_node(NodeKind::PatBind, 2, rest_span, sym.0, 0);
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
        let tape = pilot_lexer::tokenize(&src);
        for i in 0..tape.len() {
            let sp = tape.span(i);
            if sp.start >= 14890 && sp.start <= 15250 {
                println!("  token {:?}: '{}' [{}:{}]", tape.kind(i), &src[sp.start as usize..sp.end as usize], sp.start, sp.end);
            }
        }
        let arena = parse(&src);
        assert!(arena.nodes.len() > 10);
    }

    #[test]
    fn test_parse_all_syntax() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/all_syntax.pilot");
        let src = std::fs::read_to_string(&path).unwrap();
        let arena = parse(&src);
        let root = arena.node(arena.root);
        let children = arena.children_slice(root.lhs, root.rhs);
        let top_fn_names: Vec<_> = children
            .iter()
            .map(|&c| arena.node(NodeId(c)))
            .filter(|n| n.node_kind() == NodeKind::FnDecl)
            .map(|n| arena.interner.resolve(SymbolId(n.lhs)))
            .collect();
        println!("Top FN names: {:?}", top_fn_names);
        assert!(top_fn_names.contains(&"run_omnibus_showcase"));
        assert!(top_fn_names.contains(&"main"));
    }

    #[test]
    fn test_parse_crucible_200() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/crucible_200.pilot");
        let src = std::fs::read_to_string(&path).unwrap();
        let arena = parse(&src);
        let root = arena.node(arena.root);
        let children = arena.children_slice(root.lhs, root.rhs);
        let top_kinds: Vec<_> = children
            .iter()
            .map(|&c| (arena.node(NodeId(c)).node_kind(), arena.interner.resolve(SymbolId(arena.node(NodeId(c)).lhs))))
            .collect();
        println!("Crucible top nodes ({}): {:?}", children.len(), top_kinds);
    }
}
