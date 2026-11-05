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

pub type FxHashMap<K, V> = std::collections::HashMap<K, V>;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionId(pub u32);

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageId(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModifierSpec {
    VisibilityRing(u8),
    SemanticFlag(String),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ModuleKwKind {
    DeclarePkg,
    ImportPath,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Fixity {
    Prefix,
    Infix,
    Postfix,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Assoc {
    Left,
    Right,
    None,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PrattOpSpec {
    pub op_sym: SymbolId,
    pub fixity: Fixity,
    pub precedence: u8,
    pub assoc: Assoc,
    pub home_pkg: Option<PackageId>,
}

#[derive(Clone, Debug, Default)]
pub struct ParserTables {
    pub bracket_attributes: FxHashMap<SymbolId, FunctionId>,
    pub modifiers: FxHashMap<SymbolId, ModifierSpec>,
    pub param_modifiers: FxHashMap<SymbolId, SymbolId>,
    pub module_keywords: FxHashMap<SymbolId, ModuleKwKind>,
    pub syntax_macros: FxHashMap<SymbolId, FunctionId>,
    pub pratt_infix: FxHashMap<SymbolId, PrattOpSpec>,
    pub pratt_prefix: FxHashMap<SymbolId, PrattOpSpec>,
    pub pratt_postfix: FxHashMap<SymbolId, PrattOpSpec>,
    pub receiver_ident: Option<SymbolId>,
    pub bootstrap_sealed: bool,
    pub interner: pilot_intern::Interner,
    pub bracket_attributes_by_name: FxHashMap<String, FunctionId>,
    pub modifiers_by_name: FxHashMap<String, ModifierSpec>,
    pub param_modifiers_by_name: FxHashMap<String, String>,
    pub module_keywords_by_name: FxHashMap<String, ModuleKwKind>,
    pub syntax_macros_by_name: FxHashMap<String, FunctionId>,
    pub pratt_infix_by_name: FxHashMap<String, PrattOpSpec>,
    pub pratt_prefix_by_name: FxHashMap<String, PrattOpSpec>,
    pub pratt_postfix_by_name: FxHashMap<String, PrattOpSpec>,
    pub receiver_ident_name: Option<String>,
    pub prop_field_sym: Option<SymbolId>,
    pub prop_value_sym: Option<SymbolId>,
    pub prop_field_name: Option<String>,
    pub prop_value_name: Option<String>,
    /// Symbolic type construct targets registered via `syntax type_construct { ... }`
    /// (e.g. `"[]"` -> `"window"`, `"?"` -> `"maybe"`, `"!"` -> `"outcome"`, `"[N]"` -> `"array"`, `"[..]"` -> `"stride"`).
    pub type_construct_targets: FxHashMap<String, String>,
}

impl ParserTables {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn infix_precedence(&self, op: &str) -> Option<u8> {
        self.pratt_infix_by_name.get(op).map(|s| s.precedence)
    }

    pub fn prefix_precedence(&self, op: &str) -> Option<u8> {
        self.pratt_prefix_by_name.get(op).map(|s| s.precedence)
    }

    pub fn postfix_precedence(&self, op: &str) -> Option<u8> {
        self.pratt_postfix_by_name.get(op).map(|s| s.precedence)
    }
}

/// Parses a Pilot source string into an `AstArena` using an existing `ParserTables` configuration.
pub fn parse_with_tables(src: &str, tables: &ParserTables) -> AstArena {
    let tape = tokenize(src);
    let mut parser = Parser::with_tables(src, tape, tables.clone());
    parser.parse_program()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeclModifiers {
    pub vis: Option<VisibilityTier>,
    pub clearance: Option<ClearanceLevel>,
    pub clearance_expr: Option<NodeId>,
    pub is_static: bool,
    pub is_pure: bool,
    pub is_mut: bool,
    pub is_partial: bool,
    pub is_comptime: bool,
    pub is_ext: bool,
}

pub struct Parser<'a> {
    src: &'a str,
    tape: TokenTape,
    pos: usize,
    pub arena: AstArena,
    pub errors: Vec<String>,
    pub active_header: Option<DeclModifiers>,
    pub header_stack: Vec<Option<DeclModifiers>>,
    pub tables: ParserTables,
    pub top_children: Vec<NodeId>,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str, tape: TokenTape) -> Self {
        Self {
            src,
            tape,
            pos: 0,
            arena: AstArena::with_capacity(32),
            errors: Vec::new(),
            active_header: None,
            header_stack: Vec::new(),
            tables: ParserTables::new(),
            top_children: Vec::new(),
        }
    }

    pub fn with_tables(src: &'a str, tape: TokenTape, tables: ParserTables) -> Self {
        let mut arena = AstArena::with_capacity(32);
        arena.interner = tables.interner.clone();
        Self {
            src,
            tape,
            pos: 0,
            arena,
            errors: Vec::new(),
            active_header: None,
            header_stack: Vec::new(),
            tables,
            top_children: Vec::new(),
        }
    }

    /// Parses one top-level item using the live `tables` state and returns `Some(NodeId)` or `None` at EOF.
    pub fn parse_next_top_level_item(&mut self, tables: &mut ParserTables) -> Option<NodeId> {
        self.tables = tables.clone();
        self.skip_whitespace_and_comments();
        self.consume_semi_or_newlines();
        if self.at_end() {
            return None;
        }
        let start_pos = self.pos;
        let node = self.parse_stmt_or_decl();
        if node.is_some() {
            if self.arena.node(node).node_kind() == NodeKind::Program {
                let s = self.arena.node(node).lhs;
                let l = self.arena.node(node).rhs;
                let sub_children = self.arena.children_slice(s, l).to_vec();
                for c in sub_children {
                    self.top_children.push(NodeId(c));
                }
            } else {
                self.top_children.push(node);
            }
        }
        if self.pos == start_pos {
            self.advance();
        }
        self.consume_semi_or_newlines();
        tables.interner = self.arena.interner.clone();
        tables.type_construct_targets = self.tables.type_construct_targets.clone();
        Some(node)
    }

    fn extract_quote_target_type_name(&self, expr_id: NodeId) -> Option<String> {
        if expr_id.is_none() || (expr_id.0 as usize) >= self.arena.nodes.len() {
            return None;
        }
        let n = self.arena.node(expr_id);
        match n.node_kind() {
            NodeKind::IdentExpr | NodeKind::TypeNamed => {
                let s = self.arena.interner.resolve(SymbolId(n.lhs));
                if s != "quote" && !s.starts_with('$') && !s.is_empty() {
                    return Some(s.to_string());
                }
            }
            NodeKind::CallExpr | NodeKind::BlockExpr | NodeKind::Program => {
                for &c in self.arena.children_slice(n.lhs, n.rhs) {
                    if let Some(found) = self.extract_quote_target_type_name(NodeId(c)) {
                        return Some(found);
                    }
                }
            }
            NodeKind::ExprStmt | NodeKind::ReturnStmt | NodeKind::BinaryExpr => {
                return self.extract_quote_target_type_name(NodeId(n.lhs));
            }
            NodeKind::TypeList | NodeKind::TypeSlice => {
                if let Some(&sym) = self.arena.type_base_symbols.get(&expr_id.0) {
                    return Some(self.arena.interner.resolve(sym).to_string());
                }
            }
            _ => {}
        }
        None
    }

    /// Finalizes `self.arena` after interleaved top-level parsing and returns the `AstArena`.
    pub fn finish_program(&mut self) -> AstArena {
        let root_span = ByteSpan::new(0, self.src.len() as u32);
        let root = self.arena.alloc_list_node(NodeKind::Program, 0, root_span, &self.top_children);
        self.arena.root = root;
        std::mem::take(&mut self.arena)
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
    fn is_backtick_escaped(&self) -> bool {
        let sp = self.span();
        self.src.as_bytes().get(sp.start as usize) == Some(&b'`')
    }

    #[inline]
    fn is_ident_like(&self) -> bool {
        if self.pos >= self.tape.len() {
            return false;
        }
        let k = self.peek();
        if k.is_keyword() {
            return false;
        }
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

    #[inline]
    fn next_non_newline_pos(&self, mut idx: usize) -> usize {
        while idx < self.tape.len() {
            let k = self.tape.kind(idx);
            if k == TokenKind::Newline || k == TokenKind::Comment || k == TokenKind::BlockComment {
                idx += 1;
            } else {
                break;
            }
        }
        idx
    }

    #[inline]
    fn at_greater(&self) -> bool {
        self.at(TokenKind::Greater) || self.at(TokenKind::ShiftRight)
    }

    #[inline]
    fn expect_greater(&mut self) -> bool {
        if self.at(TokenKind::Greater) {
            self.advance();
            true
        } else if self.at(TokenKind::ShiftRight) {
            self.tape.kinds[self.pos] = TokenKind::Greater;
            true
        } else {
            let ctx_start = (self.span().start as usize).saturating_sub(60);
            let ctx_end = (self.span().end as usize + 20).min(self.src.len());
            self.errors.push(format!(
                "Expected '>', found {:?} near {:?}",
                self.peek(),
                &self.src[ctx_start..ctx_end]
            ));
            false
        }
    }

    fn parse_generic_params(&mut self) -> Vec<SymbolId> {
        let mut params = Vec::new();
        if !self.at(TokenKind::Less) {
            return params;
        }
        self.advance(); // consume '<'
        let mut last_was_comma = false;
        while !self.at_end() && !self.at_greater() {
            let start_pos = self.pos;
            if self.is_ident_like() || self.peek().is_keyword() {
                let p_name = self.text();
                self.advance();
                if p_name == "const" && (self.is_ident_like() || self.peek().is_keyword()) {
                    let const_param_name = self.text();
                    self.advance();
                    params.push(self.arena.interner.intern(const_param_name));
                } else {
                    params.push(self.arena.interner.intern(p_name));
                }
                if self.at(TokenKind::Colon) {
                    self.advance();
                    if self.at(TokenKind::KwConst) || (self.is_ident_like() && self.text() == "const") {
                        self.advance();
                    }
                    let _ = self.parse_type();
                }
            }
            while !self.at_end() && !self.at(TokenKind::Comma) && !self.at_greater() {
                if self.at(TokenKind::Equal) {
                    self.advance();
                    if self.at(TokenKind::StrLit) {
                        self.advance();
                    } else {
                        let _ = self.parse_type();
                    }
                } else if self.at(TokenKind::Less) {
                    let _ = self.parse_generic_params();
                } else {
                    self.advance();
                }
            }
            last_was_comma = false;
            if self.at(TokenKind::Comma) {
                self.advance();
                last_was_comma = true;
            } else {
                break;
            }
            if self.pos == start_pos {
                self.advance();
            }
        }
        if last_was_comma {
            self.errors.push("Trailing comma rejected in generic parameters".to_string());
        }
        self.expect_greater();
        params
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

    fn parse_modifiers(&mut self) -> DeclModifiers {
        let mut vis = None;
        let mut clearance = None;
        let mut clearance_expr = None;
        let mut is_static = false;
        let mut is_pure = false;
        let mut is_mut = false;
        let mut is_partial = false;
        let mut is_comptime = false;
        let mut is_ext = false;

        loop {
            self.skip_whitespace_and_comments();
            if self.at_end() {
                break;
            }

            let is_mod_candidate = self.is_ident_like()
                || self.at(TokenKind::KwPublic)
                || self.at(TokenKind::KwPrivate)
                || self.at(TokenKind::KwProtected)
                || self.at(TokenKind::KwInternal)
                || self.at(TokenKind::KwStatic)
                || self.at(TokenKind::KwComptime);

            if !is_mod_candidate {
                break;
            }

            let txt = self.text();
            match txt {
                "priv" | "private" => {
                    self.advance();
                    if self.is_ident_like() || self.at(TokenKind::KwProtected) {
                        let next_t = self.text();
                        if next_t == "prot" || next_t == "protected" {
                            self.advance();
                            vis = Some(VisibilityTier::PrivateProtected);
                            continue;
                        }
                    }
                    vis = Some(VisibilityTier::Private);
                }
                "file" | "fileprivate" => {
                    self.advance();
                    vis = Some(VisibilityTier::FilePrivate);
                }
                "prot" | "protected" => {
                    self.advance();
                    if self.is_ident_like() || self.at(TokenKind::KwInternal) {
                        let next_t = self.text();
                        if next_t == "int" || next_t == "internal" {
                            self.advance();
                            vis = Some(VisibilityTier::ProtectedInternal);
                            continue;
                        }
                    }
                    vis = Some(VisibilityTier::Protected);
                }
                "pkg" | "package" => {
                    // Disambiguate `pkg Pilot.Core.Genesis;` package declaration from `pkg` visibility modifier
                    let is_pkg_decl = vis.is_some()
                        || (self.pos + 2 < self.tape.len()
                            && self.is_ident_like_pos(self.pos + 1)
                            && matches!(
                                self.tape.kind(self.pos + 2),
                                TokenKind::InfixDot | TokenKind::Semicolon
                            ));
                    if is_pkg_decl {
                        break;
                    }
                    self.advance();
                    vis = Some(VisibilityTier::Package);
                }
                "int" | "internal" => {
                    self.advance();
                    vis = Some(VisibilityTier::Internal);
                }
                "pub" | "public" => {
                    self.advance();
                    vis = Some(VisibilityTier::Public);
                }
                "open" if !matches!(self.peek_ahead(1), TokenKind::Less | TokenKind::LBrace) => {
                    self.advance();
                    vis = Some(VisibilityTier::Open);
                }
                "export" => {
                    self.advance();
                    vis = Some(VisibilityTier::Export);
                }
                "static" => {
                    self.advance();
                    is_static = true;
                }
                "pure" => {
                    self.advance();
                    is_pure = true;
                }
                "var" => {
                    let next_pos = self.pos + 1;
                    if next_pos < self.tape.len() {
                        let nk = self.tape.kind(next_pos);
                        let is_fn_or_mod = nk == TokenKind::KwFn
                            || nk == TokenKind::KwStatic
                            || nk == TokenKind::Colon
                            || nk == TokenKind::KwOp
                            || (self.is_ident_like_pos(next_pos) && {
                                let t = self.tape.text(next_pos, self.src);
                                t == "fn" || t == "static" || t == "pure" || t == "op"
                            });
                        if is_fn_or_mod {
                            self.advance();
                            is_mut = true;
                            continue;
                        }
                    }
                    break;
                }
                "comptime" => {
                    let next_pos = self.pos + 1;
                    if next_pos < self.tape.len() && self.tape.kind(next_pos) == TokenKind::Colon {
                        self.advance();
                        is_comptime = true;
                        continue;
                    }
                    break;
                }
                "mut" => {
                    self.errors.push("The 'mut' keyword is replaced with 'var' in Pilot".to_string());
                    self.advance();
                    is_mut = true;
                }
                "partial" => {
                    self.advance();
                    is_partial = true;
                }
                "ext" | "extension" => {
                    self.advance();
                    is_ext = true;
                }
                t if t == "lvl" => {
                    self.advance();
                    if self.at(TokenKind::LParen) {
                        self.advance();
                        let mut domain = None;
                        if self.is_ident_like() && self.peek_ahead(1) == TokenKind::Comma {
                            domain = Some(self.arena.interner.intern(self.text()));
                            self.advance();
                            self.advance(); // consume ','
                        }
                        if self.at(TokenKind::IntLit) || (self.is_ident_like() && self.text().chars().all(|c| c.is_ascii_digit())) {
                            let n: u32 = self.text().parse().unwrap_or(0);
                            self.advance();
                            clearance = Some(ClearanceLevel { level: n, domain });
                        } else if !self.at(TokenKind::RParen) {
                            let expr_id = self.parse_expr();
                            clearance_expr = Some(expr_id);
                            clearance = Some(ClearanceLevel { level: 0, domain });
                        } else {
                            clearance = Some(ClearanceLevel { level: 0, domain });
                        }
                        self.expect(TokenKind::RParen);
                    }
                }
                t if t.starts_with("lvl") && t.len() > 3 && t[3..].chars().all(|c| c.is_ascii_digit()) => {
                    let lvl_num: u32 = t[3..].parse().unwrap_or(0);
                    self.advance();
                    clearance = Some(ClearanceLevel { level: lvl_num, domain: None });
                }
                _ => break,
            }
        }

        if let Some(active) = self.active_header {
            if vis.is_none() {
                vis = active.vis;
            }
            if clearance.is_none() {
                clearance = active.clearance;
            }
            if clearance_expr.is_none() {
                clearance_expr = active.clearance_expr;
            }
            if !is_static {
                is_static = active.is_static;
            }
            if !is_pure {
                is_pure = active.is_pure;
            }
            if !is_mut {
                is_mut = active.is_mut;
            }
            if !is_partial {
                is_partial = active.is_partial;
            }
            if !is_comptime {
                is_comptime = active.is_comptime;
            }
            if !is_ext {
                is_ext = active.is_ext;
            }
        }

        DeclModifiers {
            vis,
            clearance,
            clearance_expr,
            is_static,
            is_pure,
            is_mut,
            is_partial,
            is_comptime,
            is_ext,
        }
    }

    #[inline]
    fn is_ident_like_pos(&self, pos: usize) -> bool {
        if pos >= self.tape.len() {
            return false;
        }
        let k = self.tape.kind(pos);
        if k.is_keyword() {
            return false;
        }
        if k == TokenKind::Ident {
            return true;
        }
        self.tape.flag(pos).contains(TokenFlags::IS_CONTEXTUAL_IDENT)
    }

    fn parse_accessor_clauses(&mut self, prop_ty: NodeId) -> AccessorClauses {
        let mut clauses = AccessorClauses::default();

        if self.at(TokenKind::FatArrow) {
            self.advance();
            let expr = self.parse_expr();
            clauses.get_body = expr;
            return clauses;
        }

        if self.at(TokenKind::LBrace) || self.at(TokenKind::VirtualLBrace) || self.at(TokenKind::Colon) {
            let is_virtual = self.at(TokenKind::VirtualLBrace);
            if self.at(TokenKind::LBrace) || is_virtual {
                self.advance();
            } else if self.at(TokenKind::Colon) {
                self.advance();
                if self.at(TokenKind::VirtualLBrace) {
                    self.advance();
                }
            }

            while !self.at_end() && !self.at(TokenKind::RBrace) && !self.at(TokenKind::VirtualRBrace) {
                let loop_start_pos = self.pos;
                self.consume_semi_or_newlines();
                if self.at(TokenKind::RBrace) || self.at(TokenKind::VirtualRBrace) || self.at_end() {
                    break;
                }

                let span = self.span();
                let clause_mods = self.parse_modifiers();
                let clause_vis = clause_mods.vis;
                let clause_clr = clause_mods.clearance;

                if self.at(TokenKind::KwGet) || (self.is_ident_like() && self.text() == "get") {
                    self.advance();
                    if self.at(TokenKind::Semicolon) {
                        self.advance();
                        clauses.get_body = NodeId::NONE;
                    } else if self.at(TokenKind::RBrace) || self.at(TokenKind::VirtualRBrace) {
                        clauses.get_body = NodeId::NONE;
                    } else {
                        if self.at(TokenKind::Colon) || self.at(TokenKind::FatArrow) {
                            self.advance();
                        }
                        clauses.get_body = self.parse_block_or_expr_body();
                    }
                    clauses.get_visibility = clause_vis;
                    clauses.get_clearance = clause_clr;
                } else if self.at(TokenKind::KwDefault) || (self.is_ident_like() && self.text() == "default") {
                    self.advance();
                    if self.at(TokenKind::Colon) || self.at(TokenKind::FatArrow) {
                        self.advance();
                    }
                    clauses.default_body = self.parse_block_or_expr_body();
                } else if self.at(TokenKind::KwSet) || (self.is_ident_like() && self.text() == "set") {
                    self.advance();
                    let mut generic_param = None;
                    let mut generic_bound = NodeId::NONE;
                    if self.at(TokenKind::LBracket) {
                        self.advance();
                        if self.is_ident_like() {
                            generic_param = Some(self.arena.interner.intern(self.text()));
                            self.advance();
                            if self.at(TokenKind::Colon) {
                                self.advance();
                                generic_bound = self.parse_type();
                            }
                        }
                        self.expect(TokenKind::RBracket);
                    }

                    let mut param_name = self.arena.interner.intern("value");
                    let mut param_type = prop_ty;
                    let mut is_iso = false;
                    let mut guard_expr = NodeId::NONE;

                    if self.at(TokenKind::LParen) {
                        self.advance();
                        if self.is_ident_like() && self.text() == "iso" {
                            self.advance();
                            is_iso = true;
                        }
                        if self.at(TokenKind::LParen) || self.at(TokenKind::LBracket) {
                            param_type = self.parse_expr();
                            param_name = self.arena.interner.intern("value");
                        } else if self.is_ident_like() && self.peek_ahead(1) == TokenKind::LParen {
                            param_type = self.parse_expr();
                            param_name = self.arena.interner.intern("value");
                        } else if self.at(TokenKind::IntLit) || self.at(TokenKind::StrLit) {
                            param_type = self.parse_expr();
                            param_name = self.arena.interner.intern("value");
                        } else if self.is_ident_like() {
                            param_name = self.arena.interner.intern(self.text());
                            self.advance();
                            if self.at(TokenKind::Colon) {
                                self.advance();
                                param_type = self.parse_type();
                            }
                        }
                        self.expect(TokenKind::RParen);
                    }

                    if self.at(TokenKind::KwIf) || (self.is_ident_like() && self.text() == "if") {
                        self.advance();
                        guard_expr = self.parse_expr();
                    }

                    if self.at(TokenKind::Colon) || self.at(TokenKind::FatArrow) {
                        self.advance();
                    }
                    let body = if self.at(TokenKind::Semicolon) {
                        self.advance();
                        NodeId::NONE
                    } else if self.at(TokenKind::RBrace) || self.at(TokenKind::VirtualRBrace) {
                        NodeId::NONE
                    } else {
                        self.parse_block_or_expr_body()
                    };

                    let setter = SetterClause {
                        param_name,
                        param_type,
                        guard_expr,
                        is_iso,
                        generic_param,
                        generic_bound,
                        body,
                        span: ByteSpan::new(span.start, self.prev_span().end),
                        visibility: clause_vis,
                        clearance: clause_clr,
                    };
                    clauses.setters.push(setter);
                    if clauses.set_body.is_none() {
                        clauses.set_body = body;
                        clauses.set_param = Some((param_name, param_type));
                    }
                } else {
                    self.advance();
                }

                self.consume_semi_or_newlines();
                if self.pos == loop_start_pos {
                    self.advance();
                }
            }

            if self.at(TokenKind::RBrace) || self.at(TokenKind::VirtualRBrace) {
                self.advance();
            }
        }

        clauses
    }

    fn try_parse_decl_header(&mut self) -> bool {
        let mut p = self.pos;
        let mut count = 0;
        while p < self.tape.len() {
            let k = self.tape.kind(p);
            if k == TokenKind::Colon {
                if count > 0 && p + 1 < self.tape.len() && self.tape.kind(p + 1) != TokenKind::Colon {
                    let mods = self.parse_modifiers();
                    self.expect(TokenKind::Colon);
                    self.consume_semi_or_newlines();
                    self.active_header = Some(mods);
                    return true;
                }
                break;
            }
            if k == TokenKind::Ident || self.tape.flag(p).contains(TokenFlags::IS_KEYWORD) {
                let t = self.tape.text(p, self.src);
                if matches!(
                    t,
                    "pub" | "public" | "priv" | "private" | "prot" | "protected" | "pkg" | "package" | "int" | "internal" | "open" | "export" | "static" | "pure" | "partial" | "var" | "comptime"
                ) || (t.starts_with("lvl") && t[3..].chars().all(|c| c.is_ascii_digit()))
                {
                    count += 1;
                    p += 1;
                    continue;
                }
            }
            break;
        }
        false
    }

    fn try_parse_distributive_binding(&mut self, is_var: bool, mods: DeclModifiers) -> Option<NodeId> {
        let saved_pos = self.pos;
        let saved_err_len = self.errors.len();
        self.advance(); // consume 'let' or 'var'
        let is_generic_param_decl = self.peek_ahead(1) == TokenKind::Less && self.peek_ahead(3) == TokenKind::Colon;
        if self.is_ident_like()
            && self.peek_ahead(1) != TokenKind::Equal
            && self.peek_ahead(1) != TokenKind::Colon
            && !is_generic_param_decl
        {
            let ty_node = self.parse_type();
            if ty_node.is_some() && self.at(TokenKind::LBrace) {
                let first_p = self.next_non_newline_pos(self.pos + 1);
                let second_p = self.next_non_newline_pos(first_p + 1);
                let next_k = if first_p < self.tape.len() { self.tape.kind(first_p) } else { TokenKind::Eof };
                let next_k2 = if second_p < self.tape.len() { self.tape.kind(second_p) } else { TokenKind::Eof };
                let is_binding = (next_k == TokenKind::Ident
                    || (first_p < self.tape.len() && self.tape.flag(first_p).contains(TokenFlags::IS_CONTEXTUAL_IDENT)))
                    && next_k2 == TokenKind::Equal;
                if is_binding {
                    self.advance(); // consume '{'
                    let mut bindings = Vec::new();
                    while !self.at_end() && !self.at(TokenKind::RBrace) {
                        self.consume_semi_or_newlines();
                        if self.at(TokenKind::RBrace) {
                            break;
                        }
                        let b_span = self.span();
                        let name = self.text();
                        self.advance();
                        let sym = self.arena.interner.intern(name);
                        self.expect(TokenKind::Equal);
                        let init_expr = self.parse_expr();
                        let node_kind = if is_var { NodeKind::VarStmt } else { NodeKind::LetStmt };
                        let b_node = self.arena.alloc_node(node_kind, 0, b_span, sym.0, init_expr.0);
                        self.arena.type_annotations.insert(b_node.0, ty_node);
                        if let Some(v) = mods.vis {
                            self.arena.node_visibility.insert(b_node.0, v);
                        }
                        if let Some(c) = mods.clearance {
                            self.arena.node_clearance.insert(b_node.0, c);
                        }
                        bindings.push(b_node);
                        if self.at(TokenKind::Comma) {
                            self.advance();
                        }
                        self.consume_semi_or_newlines();
                    }
                    self.expect(TokenKind::RBrace);
                    if self.at(TokenKind::Semicolon) {
                        self.advance();
                    }
                    let total_span = ByteSpan::new(self.tape.span(saved_pos).start, self.prev_span().end);
                    return Some(self.arena.alloc_list_node(NodeKind::Program, 0, total_span, &bindings));
                }
            }
        }
        self.pos = saved_pos;
        self.errors.truncate(saved_err_len);
        None
    }

    fn parse_attributes(&mut self) -> Vec<AstAttribute> {
        let mut attrs = Vec::new();
        while self.at(TokenKind::LBracket) {
            self.advance(); // consume '['
            let mut last_was_comma = false;
            while !self.at_end() && !self.at(TokenKind::RBracket) {
                let attr_span = self.span();
                let mut binding_sym = None;
                let mut is_mutable_binding = false;
                if (self.at(TokenKind::KwVar) || self.at(TokenKind::KwLet) || (self.is_ident_like() && (self.text() == "var" || self.text() == "let")))
                    && self.pos + 2 < self.tape.len()
                    && self.is_ident_like_pos(self.pos + 1)
                    && self.tape.kind(self.pos + 2) == TokenKind::Equal
                {
                    is_mutable_binding = self.at(TokenKind::KwVar) || (self.is_ident_like() && self.text() == "var");
                    self.advance(); // consume 'var' or 'let'
                    let b_name = self.text();
                    binding_sym = Some(self.arena.interner.intern(b_name));
                    self.advance(); // consume binding name
                    self.advance(); // consume '='
                }
                if self.is_ident_like() || self.peek().is_keyword() {
                    let attr_name = self.arena.interner.intern(self.text());
                    self.advance();
                    let mut args = Vec::new();
                    if self.at(TokenKind::LParen) {
                        self.advance(); // consume '('
                        let mut arg_trailing_comma = false;
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
                            let arg_expr = self.parse_expr();
                            if arg_expr.is_some() {
                                args.push(arg_expr);
                            }
                            arg_trailing_comma = false;
                            if self.at(TokenKind::Comma) {
                                self.advance();
                                arg_trailing_comma = true;
                            } else {
                                break;
                            }
                            if self.pos == start_pos {
                                self.advance();
                            }
                        }
                        if arg_trailing_comma {
                            self.errors.push("Trailing comma not allowed in attribute arguments".to_string());
                        }
                        self.expect(TokenKind::RParen);
                    }
                    let end_span = self.prev_span();
                    attrs.push(AstAttribute {
                        name: attr_name,
                        args,
                        binding_sym,
                        is_mutable_binding,
                        span: ByteSpan::new(attr_span.start, end_span.end),
                    });
                } else {
                    self.advance();
                }
                last_was_comma = false;
                if self.at(TokenKind::Comma) {
                    self.advance();
                    last_was_comma = true;
                } else {
                    break;
                }
            }
            if last_was_comma {
                self.errors.push("Trailing comma not allowed in attribute brackets".to_string());
            }
            self.expect(TokenKind::RBracket);
            self.consume_semi_or_newlines();
        }
        attrs
    }

    fn apply_decl_mods(&mut self, node: NodeId, mods: DeclModifiers, default_vis: Option<VisibilityTier>) {
        if node.is_none() {
            return;
        }
        if let Some(v) = mods.vis.or(default_vis) {
            self.arena.node_visibility.insert(node.0, v);
        }
        if let Some(c) = mods.clearance {
            self.arena.node_clearance.insert(node.0, c);
        }
        if let Some(ce) = mods.clearance_expr {
            self.arena.clearance_exprs.insert(node.0, ce);
        }
        if mods.is_pure {
            self.arena.pure_methods.insert(node.0);
        }
        if mods.is_static {
            self.arena.static_methods.insert(node.0);
        }
        if mods.is_mut {
            self.arena.mut_methods.insert(node.0);
        }
        if mods.is_partial {
            self.arena.partial_types.insert(node.0);
        }
    }

    fn parse_stmt_or_decl(&mut self) -> NodeId {
        let attrs = self.parse_attributes();
        let node = self.parse_stmt_or_decl_inner();
        if node.is_some() && !attrs.is_empty() {
            self.arena.node_attributes.entry(node.0).or_default().extend(attrs);
        }
        node
    }

    fn parse_stmt_or_decl_inner(&mut self) -> NodeId {
        if self.try_parse_decl_header() {
            return NodeId::NONE;
        }

        let mods = self.parse_modifiers();
        let vis_opt = mods.vis;
        let clr_opt = mods.clearance;
        let is_static = mods.is_static;
        let is_mut = mods.is_mut;
        let is_pure = mods.is_pure;

        if mods.is_ext && !self.at(TokenKind::KwFn) && !(self.is_ident_like() && self.text() == "fn") {
            let n = self.parse_extend_decl();
            self.apply_decl_mods(n, mods, Some(VisibilityTier::Public));
            return n;
        }

        if (self.at(TokenKind::KwStruct) || (self.is_ident_like() && self.text() == "struct")) && self.peek_ahead(1) == TokenKind::LBrace {
            let start_span = self.span();
            self.advance(); // consume 'struct'
            self.advance(); // consume '{'
            self.header_stack.push(self.active_header);
            self.active_header = Some(mods);
            let mut children = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.consume_semi_or_newlines();
                if self.at(TokenKind::RBrace) || self.at_end() {
                    break;
                }
                let start_pos = self.pos;
                let item_attrs = self.parse_attributes();
                let item_mods = self.parse_modifiers();
                let s_node = if self.at(TokenKind::KwStruct) || (self.is_ident_like() && self.text() == "struct") {
                    self.parse_struct_decl()
                } else if self.is_ident_like() || self.peek().is_keyword() {
                    self.parse_struct_decl_headless()
                } else {
                    self.parse_stmt_or_decl()
                };
                if s_node.is_some() {
                    self.apply_decl_mods(s_node, item_mods, Some(VisibilityTier::Internal));
                    if !item_attrs.is_empty() {
                        self.arena.node_attributes.entry(s_node.0).or_default().extend(item_attrs);
                    }
                    children.push(s_node);
                }
                if self.pos == start_pos {
                    self.advance();
                }
                self.consume_semi_or_newlines();
            }
            self.expect(TokenKind::RBrace);
            self.active_header = self.header_stack.pop().unwrap_or(None);
            return self.arena.alloc_list_node(
                NodeKind::BlockExpr,
                0,
                ByteSpan::new(start_span.start, self.prev_span().end),
                &children,
            );
        }
        if (self.at(TokenKind::KwInterface) || (self.is_ident_like() && self.text() == "interface")) && self.peek_ahead(1) == TokenKind::LBrace {
            let start_span = self.span();
            self.advance(); // consume 'interface'
            self.advance(); // consume '{'
            self.header_stack.push(self.active_header);
            self.active_header = Some(mods);
            let mut children = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.consume_semi_or_newlines();
                if self.at(TokenKind::RBrace) || self.at_end() {
                    break;
                }
                let start_pos = self.pos;
                let item_attrs = self.parse_attributes();
                let item_mods = self.parse_modifiers();
                let i_node = if self.at(TokenKind::KwInterface) || (self.is_ident_like() && self.text() == "interface") {
                    self.parse_interface_decl()
                } else if self.is_ident_like() || self.peek().is_keyword() {
                    self.parse_interface_decl_headless()
                } else {
                    self.parse_stmt_or_decl()
                };
                if i_node.is_some() {
                    self.apply_decl_mods(i_node, item_mods, Some(VisibilityTier::Internal));
                    if !item_attrs.is_empty() {
                        self.arena.node_attributes.entry(i_node.0).or_default().extend(item_attrs);
                    }
                    children.push(i_node);
                }
                if self.pos == start_pos {
                    self.advance();
                }
                self.consume_semi_or_newlines();
            }
            self.expect(TokenKind::RBrace);
            self.active_header = self.header_stack.pop().unwrap_or(None);
            return self.arena.alloc_list_node(
                NodeKind::BlockExpr,
                0,
                ByteSpan::new(start_span.start, self.prev_span().end),
                &children,
            );
        }

        // Package declaration: `pkg Pilot.Core.Genesis;`
        if self.is_ident_like() && (self.text() == "pkg" || self.tables.module_keywords_by_name.get(self.text()) == Some(&ModuleKwKind::DeclarePkg)) {
            let start_span = self.span();
            self.advance(); // consume 'pkg'
            let mut pkg_path = String::new();
            while !self.at_end() && !self.at(TokenKind::Semicolon) && !self.at(TokenKind::Newline) {
                pkg_path.push_str(self.text());
                self.advance();
            }
            if self.at(TokenKind::Semicolon) {
                self.advance();
            }
            let path_sym = self.arena.interner.intern(&pkg_path);
            let pkg_node = self.arena.alloc_node(
                NodeKind::ImportDecl,
                1,
                ByteSpan::new(start_span.start, self.prev_span().end),
                path_sym.0,
                u32::MAX,
            );
            self.arena.pkg_decls.insert(pkg_node.0);
            self.apply_decl_mods(pkg_node, mods, Some(VisibilityTier::Public));
            return pkg_node;
        }

        // Use ext declaration: `use ext StrExt { ... }` or `use ext StrExt;`
        if self.is_ident_like() && self.text() == "use" && self.pos + 1 < self.tape.len() && self.is_ident_like_pos(self.pos + 1) && (self.tape.text(self.pos + 1, self.src) == "ext" || self.tape.text(self.pos + 1, self.src) == "extension") {
            let start_span = self.span();
            self.advance(); // consume 'use'
            self.advance(); // consume 'ext'
            let group_name = self.text().to_string();
            self.advance();
            let group_sym = self.arena.interner.intern(&group_name);
            if self.at(TokenKind::LBrace) {
                let block = self.parse_block_expr();
                self.arena.lexical_use_ext_scopes.insert(block.0, group_sym);
                return block;
            } else {
                if self.at(TokenKind::Semicolon) {
                    self.advance();
                }
                let span = ByteSpan::new(start_span.start, self.prev_span().end);
                let use_node = self.arena.alloc_node(NodeKind::ImportDecl, 0, span, group_sym.0, u32::MAX);
                self.arena.lexical_use_ext_scopes.insert(use_node.0, group_sym);
                return use_node;
            }
        }

        // Use import declaration: `use Pilot.Core.Genesis.*;`, `use Pkg.Mod as alias;`, `use Pkg.Mod.{A, B};`
        if self.is_ident_like() && (self.text() == "use" || self.tables.module_keywords_by_name.get(self.text()) == Some(&ModuleKwKind::ImportPath)) {
            let start_span = self.span();
            self.advance(); // consume 'use'
            let mut use_path = String::new();
            let mut alias: Option<String> = None;
            let mut braced_list = false;
            let mut imported_symbols = Vec::new();
            let mut saw_glob_in_braces = false;

            while !self.at_end() && !self.at(TokenKind::Semicolon) && !self.at(TokenKind::Newline) {
                if self.at(TokenKind::KwAs) || (self.is_ident_like() && self.text() == "as") {
                    self.advance(); // consume 'as'
                    if self.is_ident_like() {
                        alias = Some(self.text().to_string());
                        self.advance();
                    }
                    break;
                }
                if self.at(TokenKind::LBrace) {
                    braced_list = true;
                    self.advance(); // consume '{'
                    while !self.at_end() && !self.at(TokenKind::RBrace) {
                        self.consume_semi_or_newlines();
                        if self.at(TokenKind::RBrace) || self.at_end() {
                            break;
                        }
                        if self.is_ident_like() {
                            let mut sym_name = self.text().to_string();
                            let sym_span = self.span();
                            self.advance();
                            if self.at(TokenKind::KwAs) || (self.is_ident_like() && self.text() == "as") {
                                self.advance();
                                if self.is_ident_like() {
                                    sym_name = self.text().to_string();
                                    self.advance();
                                }
                            }
                            let sym_node = self.arena.alloc_ident(&sym_name, ByteSpan::new(sym_span.start, self.prev_span().end));
                            imported_symbols.push(sym_node);
                        } else if self.at(TokenKind::Star) {
                            saw_glob_in_braces = true;
                            self.advance();
                        } else if self.at(TokenKind::Comma) {
                            self.advance();
                        } else {
                            self.advance();
                        }
                    }
                    if self.at(TokenKind::RBrace) {
                        self.advance();
                    }
                    break;
                }
                use_path.push_str(self.text());
                self.advance();
            }
            if self.at(TokenKind::Semicolon) {
                self.advance();
            }

            if braced_list && !(saw_glob_in_braces && imported_symbols.is_empty()) {
                let clean_mod = use_path.trim_end_matches(['.', ':']);
                let path_sym = self.arena.interner.intern(clean_mod);
                let from_node = self.arena.alloc_list_node(
                    NodeKind::FromImportDecl,
                    0,
                    ByteSpan::new(start_span.start, self.prev_span().end),
                    &imported_symbols,
                );
                self.arena.from_import_paths.insert(from_node.0, path_sym);
                self.apply_decl_mods(from_node, mods, Some(VisibilityTier::Private));
                return from_node;
            }

            if braced_list && saw_glob_in_braces {
                let clean_mod = use_path.trim_end_matches(['.', ':']);
                use_path = format!("{}.*", clean_mod);
            }

            let path_sym = self.arena.interner.intern(&use_path);
            let alias_sym = if let Some(ref a) = alias {
                self.arena.interner.intern(a).0
            } else {
                u32::MAX
            };
            let use_node = self.arena.alloc_node(
                NodeKind::ImportDecl,
                0,
                ByteSpan::new(start_span.start, self.prev_span().end),
                path_sym.0,
                alias_sym,
            );
            self.apply_decl_mods(use_node, mods, Some(VisibilityTier::Private));
            return use_node;
        }

        // Syntax macro block: `pub syntax { break() => ...; continue() => ...; op(sig, body) => ...; }`
        if self.is_ident_like()
            && (self.text() == "syntax" || self.tables.syntax_macros_by_name.contains_key(self.text()))
            && self.peek_ahead(1) == TokenKind::LBrace
        {
            let start_span = self.span();
            self.advance(); // consume 'syntax'
            self.advance(); // consume '{'
            let mut rules = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.consume_semi_or_newlines();
                if self.at(TokenKind::RBrace) || self.at_end() {
                    break;
                }
                let start_pos = self.pos;
                let rule_span = self.span();
                let rule_name = self.text().to_string();
                let rule_fn = self.parse_fn_decl_headless(false, false);
                if rule_fn.is_some() {
                    self.arena.comptime_fns.insert(rule_fn.0);
                    self.apply_decl_mods(rule_fn, mods, Some(VisibilityTier::Public));
                    let syn_macro_sym = self.arena.interner.intern("SyntaxMacro");
                    let kw_str_node = self.arena.alloc_str(&rule_name, rule_span);
                    self.arena.node_attributes.entry(rule_fn.0).or_default().push(AstAttribute {
                        name: syn_macro_sym,
                        args: vec![kw_str_node],
                        binding_sym: None,
                        is_mutable_binding: false,
                        span: rule_span,
                    });
                    rules.push(rule_fn);
                }
                if self.pos == start_pos {
                    self.advance();
                }
                self.consume_semi_or_newlines();
            }
            self.expect(TokenKind::RBrace);
            return self.arena.alloc_list_node(
                NodeKind::BlockExpr,
                0,
                ByteSpan::new(start_span.start, self.prev_span().end),
                &rules,
            );
        }

        // Named syntax construct block: `pub syntax type_construct { ($t:type) [] => quote { slice<$t> }; ... }`
        if self.is_ident_like()
            && (self.text() == "syntax" || self.tables.syntax_macros_by_name.contains_key(self.text()))
            && self.is_ident_like_pos(self.pos + 1)
            && self.peek_ahead(2) == TokenKind::LBrace
        {
            let start_span = self.span();
            self.advance(); // consume 'syntax'
            let group_name = self.text().to_string();
            self.advance(); // consume group name (e.g. 'type_construct')
            self.advance(); // consume '{'
            let mut rules = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.consume_semi_or_newlines();
                if self.at(TokenKind::RBrace) || self.at_end() {
                    break;
                }
                let rule_span = self.span();
                let mut pat_str = String::new();
                while !self.at_end() && !self.at(TokenKind::FatArrow) && !self.at(TokenKind::RBrace) {
                    pat_str.push_str(self.text());
                    self.advance();
                }
                let body_expr = if self.at(TokenKind::FatArrow) {
                    self.advance();
                    self.parse_expr()
                } else {
                    NodeId::NONE
                };
                if self.at(TokenKind::Semicolon) {
                    self.advance();
                }
                if group_name == "type_construct" && body_expr.is_some() {
                    if let Some(target_base) = self.extract_quote_target_type_name(body_expr) {
                        let construct_key = if pat_str.contains("[..]") {
                            Some("[..]")
                        } else if pat_str.contains("[:]") {
                            Some("[:]")
                        } else if pat_str.contains("[]") {
                            Some("[]")
                        } else if pat_str.contains('[') && pat_str.contains(',') && pat_str.contains(']') {
                            Some("[N,M]")
                        } else if pat_str.contains('[') && pat_str.contains(']') {
                            Some("[N]")
                        } else if pat_str.starts_with('@') || pat_str.starts_with("(@") {
                            Some("@prefix")
                        } else if pat_str.contains('@') {
                            Some("@")
                        } else if pat_str.contains('^') {
                            Some("^")
                        } else if pat_str.contains('?') {
                            Some("?")
                        } else if pat_str.contains('!') {
                            Some("!")
                        } else if pat_str.contains("=>") {
                            Some("=>")
                        } else {
                            None
                        };
                        if let Some(key) = construct_key {
                            self.tables.type_construct_targets.insert(key.to_string(), target_base);
                        }
                    }
                }
                let sym = self.arena.interner.intern(&group_name);
                let pat_node = self.arena.alloc_str(&pat_str, rule_span);
                let (fn_start, fn_len) = self.arena.alloc_child_slice(&[body_expr, pat_node]);
                let rule_fn = self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, rule_span, sym.0, fn_start);
                self.arena.comptime_fns.insert(rule_fn.0);
                self.apply_decl_mods(rule_fn, mods, Some(VisibilityTier::Public));
                rules.push(rule_fn);
                self.consume_semi_or_newlines();
            }
            self.expect(TokenKind::RBrace);
            return self.arena.alloc_list_node(
                NodeKind::BlockExpr,
                0,
                ByteSpan::new(start_span.start, self.prev_span().end),
                &rules,
            );
        }

        if self.at(TokenKind::KwLet) || (self.is_ident_like() && self.text() == "let") {
            if let Some(node) = self.try_parse_distributive_binding(false, mods) {
                return node;
            }
        } else if self.at(TokenKind::KwVar) || (self.is_ident_like() && self.text() == "var") {
            if let Some(node) = self.try_parse_distributive_binding(true, mods) {
                return node;
            }
        }

        if self.at(TokenKind::KwExtern) || (self.is_ident_like() && self.text() == "extern") {
            return self.parse_extern_decl();
        }

        if self.peek_is_polyglot_block() {
            return self.parse_polyglot_block();
        }

        // Labeled loop statement (`outer_loop: loop ...`) vs typed field declaration (`x: f64`) vs property
        if self.is_ident_like() && self.peek_ahead(1) == TokenKind::Colon && self.peek_ahead(2) != TokenKind::Colon {
            let label_name = self.text();
            if label_name != "default" && label_name != "case" {
                let after_colon = self.peek_ahead(2);
                if matches!(after_colon, TokenKind::KwFor | TokenKind::KwWhile | TokenKind::KwLoop)
                    || (after_colon == TokenKind::Ident && self.pos + 2 < self.tape.len() && matches!(self.tape.text(self.pos + 2, self.src), "for" | "while"))
                {
                    self.advance(); // consume label
                    self.advance(); // consume ':'
                    return self.parse_stmt_or_decl();
                } else if after_colon == TokenKind::Ident
                    || after_colon.is_keyword()
                    || matches!(after_colon, TokenKind::LBracket | TokenKind::LParen | TokenKind::Star | TokenKind::Less)
                {
                    let start_span = self.span();
                    let field_name = self.text();
                    self.advance(); // consume field name
                    let sym = self.arena.interner.intern(field_name);
                    self.advance(); // consume ':'
                    let ty_node = self.parse_type();

                    if self.at(TokenKind::LBrace) {
                        let lookahead_p = self.next_non_newline_pos(self.pos + 1);
                        let is_prop = if lookahead_p < self.tape.len() {
                            let k = self.tape.kind(lookahead_p);
                            let t = self.tape.text(lookahead_p, self.src);
                            k == TokenKind::KwGet || t == "get" || t == "set" || t == "pub" || t == "priv"
                        } else {
                            false
                        };
                        if is_prop {
                            let clauses = self.parse_accessor_clauses(ty_node);
                            let end_span = self.prev_span();
                            let span = ByteSpan::new(start_span.start, end_span.end);
                            let prop_node = self.arena.alloc_node(NodeKind::FnDecl, 0, span, sym.0, 0);
                            if ty_node.is_some() {
                                self.arena.type_annotations.insert(prop_node.0, ty_node);
                            }
                            self.arena.accessor_clauses.insert(prop_node.0, clauses);
                            if let Some(v) = vis_opt {
                                self.arena.node_visibility.insert(prop_node.0, v);
                            }
                            return prop_node;
                        }
                    }

                    let init = if self.at(TokenKind::Equal) {
                        self.advance();
                        self.parse_expr()
                    } else {
                        NodeId::NONE
                    };
                    if self.at(TokenKind::Comma) {
                        self.advance();
                    }
                    if self.at(TokenKind::Semicolon) {
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
                    if let Some(v) = vis_opt {
                        self.arena.node_visibility.insert(node_id.0, v);
                    }
                    return node_id;
                }
            }
        }

        if self.at(TokenKind::KwFn) || (self.is_ident_like() && self.text() == "fn") {
            let fn_node = self.parse_fn_decl_modifiers(is_mut, is_static);
            let vis = vis_opt.unwrap_or(VisibilityTier::Private);
            self.arena.node_visibility.insert(fn_node.0, vis);
            if let Some(c) = clr_opt {
                self.arena.node_clearance.insert(fn_node.0, c);
            }
            if is_pure {
                self.arena.pure_methods.insert(fn_node.0);
            }
            if is_mut {
                self.arena.mut_methods.insert(fn_node.0);
            }
            if is_static {
                self.arena.static_methods.insert(fn_node.0);
            }
            return fn_node;
        }

        if self.at(TokenKind::KwLoop) {
            let start_span = self.span();
            self.advance(); // consume 'loop'
            let body = self.parse_block_expr();
            return self.arena.alloc_node(NodeKind::WhileStmt, 0, start_span, NodeId::NONE.0, body.0);
        }

        if self.at(TokenKind::KwComptime) || (self.is_ident_like() && self.text() == "comptime") {
            let start_span = self.span();
            let next_pos = self.pos + 1;
            if next_pos < self.tape.len() {
                let nk = self.tape.kind(next_pos);
                if nk == TokenKind::LBrace {
                    self.advance(); // consume 'comptime'
                    let block = self.parse_block_expr();
                    let end_span = self.prev_span();
                    let node = self.arena.alloc_node(
                        NodeKind::ComptimeExpr,
                        0,
                        ByteSpan::new(start_span.start, end_span.end),
                        block.0,
                        0,
                    );
                    self.apply_decl_mods(node, mods, None);
                    return node;
                } else if nk == TokenKind::KwDef || nk == TokenKind::KwFn || (self.is_ident_like_pos(next_pos) && self.tape.text(next_pos, self.src) == "fn") {
                    self.advance(); // consume 'comptime'
                    if self.peek_ahead(1) == TokenKind::LBrace {
                        // Distributive `comptime fn { name(params) => expr; ... }`
                        self.advance(); // consume 'fn'
                        self.advance(); // consume '{'
                        let mut fns = Vec::new();
                        while !self.at_end() && !self.at(TokenKind::RBrace) {
                            self.consume_semi_or_newlines();
                            if self.at(TokenKind::RBrace) || self.at_end() {
                                break;
                            }
                            let start_pos = self.pos;
                            let fn_node = self.parse_fn_decl_headless(is_mut, is_static);
                            if fn_node.is_some() {
                                self.arena.comptime_fns.insert(fn_node.0);
                                self.apply_decl_mods(fn_node, mods, Some(VisibilityTier::Private));
                                fns.push(fn_node);
                            }
                            if self.pos == start_pos {
                                self.advance();
                            }
                            self.consume_semi_or_newlines();
                        }
                        self.expect(TokenKind::RBrace);
                        return self.arena.alloc_list_node(
                            NodeKind::BlockExpr,
                            0,
                            ByteSpan::new(start_span.start, self.prev_span().end),
                            &fns,
                        );
                    } else {
                        let fn_node = self.parse_fn_decl_modifiers(is_mut, is_static);
                        self.arena.comptime_fns.insert(fn_node.0);
                        self.apply_decl_mods(fn_node, mods, Some(VisibilityTier::Private));
                        return fn_node;
                    }
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
            } else if mods == DeclModifiers::default()
                && (((txt == "deadline" || txt == "within" || txt == "periodic" || txt == "timeout")
                    && !matches!(
                        self.peek_ahead(1),
                        TokenKind::Equal
                            | TokenKind::Colon
                            | TokenKind::LParen
                            | TokenKind::InfixDot
                            | TokenKind::Semicolon
                            | TokenKind::LBracket
                            | TokenKind::PlusEqual
                            | TokenKind::MinusEqual
                    ))
                    || ((txt == "on_missed" || txt == "fallback") && self.peek_ahead(1) == TokenKind::LBrace))
            {
                self.advance();
                while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::VirtualLBrace) && !self.at(TokenKind::Newline) {
                    self.advance();
                }
                return self.parse_block_expr();
            } else if txt == "unit" && self.is_ident_like_pos(self.pos + 1) {
                let start_span = self.span();
                self.advance(); // consume 'unit'
                let unit_name = self.text().to_string();
                self.advance(); // consume unit name
                let unit_sym = self.arena.interner.intern(&unit_name);
                let mut base_ty = NodeId::NONE;
                if self.at(TokenKind::LParen) {
                    self.advance();
                    base_ty = self.parse_type();
                    self.expect(TokenKind::RParen);
                }
                let formula_expr = if self.at(TokenKind::Equal) {
                    self.advance();
                    self.parse_expr()
                } else {
                    NodeId::NONE
                };
                if self.at(TokenKind::Semicolon) {
                    self.advance();
                }
                let end_span = self.prev_span();
                let node_id = self.arena.alloc_node(
                    NodeKind::StructDecl,
                    2,
                    ByteSpan::new(start_span.start, end_span.end),
                    unit_sym.0,
                    formula_expr.0,
                );
                if base_ty.is_some() {
                    self.arena.type_annotations.insert(node_id.0, base_ty);
                }
                self.apply_decl_mods(node_id, mods, Some(VisibilityTier::Public));
                return node_id;
            }
        }

        if self.peek_is_destructor_decl() {
            return self.parse_destructor_decl();
        }

        if self.peek_is_scope_decl() {
            return self.parse_scope_decl();
        }

        if self.peek_is_dynamic_member_decl() {
            return self.parse_dynamic_member_decl();
        }

        if self.peek_is_indexer_decl() {
            return self.parse_indexer_decl(self.span());
        }

        if self.peek_is_constructor_decl() {
            return self.parse_constructor_decl();
        }

        if self.peek() == TokenKind::KwOp || (self.is_ident_like() && self.text() == "op") {
            return self.parse_operator_decl();
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
            TokenKind::KwFn => {
                let n = self.parse_fn_decl_modifiers(is_mut, is_static);
                let vis = vis_opt.unwrap_or(VisibilityTier::Private);
                self.arena.node_visibility.insert(n.0, vis);
                if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                if is_pure { self.arena.pure_methods.insert(n.0); }
                if is_mut { self.arena.mut_methods.insert(n.0); }
                if is_static { self.arena.static_methods.insert(n.0); }
                n
            }
            TokenKind::KwLoop => {
                let start_span = self.span();
                self.advance(); // consume 'loop'
                let body = self.parse_block_expr();
                self.arena.alloc_node(NodeKind::WhileStmt, 0, start_span, NodeId::NONE.0, body.0)
            }
            TokenKind::KwClass | TokenKind::KwActor | TokenKind::KwState if k != TokenKind::KwState || self.is_ident_like_pos(self.pos + 1) => {
                let n = self.parse_class_decl();
                let vis = vis_opt.unwrap_or(VisibilityTier::Internal);
                self.arena.node_visibility.insert(n.0, vis);
                if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                n
            }
            TokenKind::KwStruct => {
                let n = self.parse_struct_decl();
                let vis = vis_opt.unwrap_or(VisibilityTier::Internal);
                self.arena.node_visibility.insert(n.0, vis);
                if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                n
            }
            TokenKind::KwRecord => {
                let n = self.parse_record_decl();
                let vis = vis_opt.unwrap_or(VisibilityTier::Internal);
                self.arena.node_visibility.insert(n.0, vis);
                if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                n
            }
            TokenKind::KwInterface | TokenKind::KwProtocol => {
                let n = self.parse_interface_decl();
                let vis = vis_opt.unwrap_or(VisibilityTier::Internal);
                self.arena.node_visibility.insert(n.0, vis);
                if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                n
            }
            TokenKind::KwExtend | TokenKind::KwExtension => self.parse_extend_decl(),
            TokenKind::KwType | TokenKind::KwEnum => {
                let n = self.parse_type_adt_decl();
                let vis = vis_opt.unwrap_or(VisibilityTier::Internal);
                self.arena.node_visibility.insert(n.0, vis);
                if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                n
            }
            TokenKind::KwLet | TokenKind::KwDynamic => {
                let n = self.parse_binding_stmt(NodeKind::LetStmt);
                if let Some(v) = vis_opt { self.arena.node_visibility.insert(n.0, v); }
                if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                n
            }
            TokenKind::KwVar | TokenKind::KwRef => {
                let n = self.parse_binding_stmt(NodeKind::VarStmt);
                if let Some(v) = vis_opt { self.arena.node_visibility.insert(n.0, v); }
                if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                n
            }
            TokenKind::KwConst => {
                let n = self.parse_binding_stmt(NodeKind::ConstDecl);
                if let Some(v) = vis_opt { self.arena.node_visibility.insert(n.0, v); }
                if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                n
            }
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
            TokenKind::KwSpawn | TokenKind::KwSync | TokenKind::KwAtomic | TokenKind::KwIsolate => {
                self.advance();
                self.parse_block_expr()
            }
            TokenKind::KwSelect => self.parse_select_stmt(),
            TokenKind::KwTry => self.parse_try_stmt(),
            TokenKind::KwWith => self.parse_with_stmt(),
            TokenKind::KwProperty => {
                let n = self.parse_property_decl();
                let vis = vis_opt.unwrap_or(VisibilityTier::Private);
                self.arena.node_visibility.insert(n.0, vis);
                if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                n
            }
            TokenKind::KwOperator | TokenKind::KwOp => {
                let n = self.parse_operator_decl();
                let vis = vis_opt.unwrap_or(VisibilityTier::Public);
                self.arena.node_visibility.insert(n.0, vis);
                if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                n
            }
            TokenKind::KwMacro | TokenKind::KwDefmacro => self.parse_macro_decl(),
            TokenKind::KwAdapter | TokenKind::KwConverter => self.parse_adapter_decl(),
            TokenKind::KwRaise => self.parse_raise_stmt(),
            TokenKind::KwAssert => self.parse_assert_stmt(),
            TokenKind::Star if self.pos + 1 < self.tape.len() && self.is_ident_like_pos(self.pos + 1) => {
                let start_span = self.span();
                let expr = self.parse_expr();
                let mut renames = Vec::new();
                if self.is_ident_like() && self.text() == "with" {
                    self.advance(); // consume 'with'
                    if self.at(TokenKind::LBrace) {
                        self.advance();
                        while !self.at_end() && !self.at(TokenKind::RBrace) {
                            self.consume_semi_or_newlines();
                            if self.at(TokenKind::RBrace) { break; }
                            if self.is_ident_like() {
                                let old_name = self.text().to_string();
                                self.advance();
                                let old_sym = self.arena.interner.intern(&old_name);
                                if self.at(TokenKind::KwAs) || (self.is_ident_like() && self.text() == "as") {
                                    self.advance();
                                }
                                if self.is_ident_like() {
                                    let new_name = self.text().to_string();
                                    self.advance();
                                    let new_sym = self.arena.interner.intern(&new_name);
                                    renames.push((old_sym, new_sym));
                                }
                            } else {
                                self.advance();
                            }
                            if self.at(TokenKind::Comma) { self.advance(); }
                        }
                        self.expect(TokenKind::RBrace);
                    }
                }
                if self.at(TokenKind::Semicolon) {
                    self.advance();
                }
                let span = ByteSpan::new(start_span.start, self.prev_span().end);
                let node = self.arena.alloc_node(NodeKind::ExprStmt, 0, span, expr.0, 0);
                if !renames.is_empty() {
                    self.arena.statement_splice_renames.insert(node.0, renames);
                }
                return node;
            }
            _ => {
                if self.is_ident_like() && !self.is_backtick_escaped() {
                    let txt = self.text();
                    match txt {
                        "var" => {
                            let n = self.parse_binding_stmt(NodeKind::VarStmt);
                            if let Some(v) = vis_opt { self.arena.node_visibility.insert(n.0, v); }
                            if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                            return n;
                        }
                        "const" => {
                            let n = self.parse_binding_stmt(NodeKind::ConstDecl);
                            if let Some(v) = vis_opt { self.arena.node_visibility.insert(n.0, v); }
                            if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                            return n;
                        }
                        "while" => return self.parse_while_stmt(),
                        "for" => return self.parse_for_stmt(),
                        "break" => return self.parse_break_stmt(),
                        "continue" => return self.parse_continue_stmt(),
                        "class" | "actor" => {
                            let n = self.parse_class_decl();
                            let vis = vis_opt.unwrap_or(VisibilityTier::Internal);
                            self.arena.node_visibility.insert(n.0, vis);
                            if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                            return n;
                        }
                        "state" if self.is_ident_like_pos(self.pos + 1) && (self.peek_ahead(2) == TokenKind::LBrace || self.peek_ahead(2) == TokenKind::Colon) => {
                            let start_span = self.span();
                            self.advance(); // consume 'state'
                            let s_name = self.text().to_string();
                            self.advance();
                            let s_sym = self.arena.interner.intern(&s_name);
                            let mut variants = Vec::new();
                            if self.at(TokenKind::LBrace) {
                                self.advance();
                                while !self.at_end() && !self.at(TokenKind::RBrace) {
                                    self.consume_semi_or_newlines();
                                    if self.at(TokenKind::RBrace) { break; }
                                    if self.is_ident_like() {
                                        let v_sym = self.arena.interner.intern(self.text());
                                        self.advance();
                                        variants.push(v_sym);
                                    } else {
                                        self.advance();
                                    }
                                    if self.at(TokenKind::Comma) { self.advance(); }
                                }
                                self.expect(TokenKind::RBrace);
                            }
                            let span = ByteSpan::new(start_span.start, self.prev_span().end);
                            let n = self.arena.alloc_node(NodeKind::TypeAdtDecl, 0, span, s_sym.0, 0);
                            self.arena.state_typestates.insert(n.0, variants);
                            let vis = vis_opt.unwrap_or(VisibilityTier::Public);
                            self.arena.node_visibility.insert(n.0, vis);
                            return n;
                        }
                        "blueprint" | "role" => {
                            let start_span = self.span();
                            self.advance(); // consume 'blueprint' or 'role'
                            let b_name = self.text().to_string();
                            self.advance();
                            let b_sym = self.arena.interner.intern(&b_name);
                            let body = if self.at(TokenKind::LBrace) { self.parse_block_expr() } else { NodeId::NONE };
                            let span = ByteSpan::new(start_span.start, self.arena.span(body).end);
                            return self.arena.alloc_node(NodeKind::StructDecl, 0, span, b_sym.0, body.0);
                        }
                        "cue" => {
                            self.advance(); // consume 'cue'
                            let fn_node = self.parse_fn_decl_headless(false, false);
                            self.arena.cue_declarations.insert(fn_node.0);
                            return fn_node;
                        }
                        "route" => {
                            let start_span = self.span();
                            self.advance(); // consume 'route'
                            let expr = self.parse_expr();
                            self.consume_semi_or_newlines();
                            let span = ByteSpan::new(start_span.start, self.arena.span(expr).end);
                            return self.arena.alloc_node(NodeKind::ExprStmt, 0, span, expr.0, 0);
                        }
                        "record" => {
                            let n = self.parse_record_decl();
                            let vis = vis_opt.unwrap_or(VisibilityTier::Internal);
                            self.arena.node_visibility.insert(n.0, vis);
                            if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                            return n;
                        }
                        "type" | "enum" => {
                            let n = self.parse_type_adt_decl();
                            let vis = vis_opt.unwrap_or(VisibilityTier::Internal);
                            self.arena.node_visibility.insert(n.0, vis);
                            if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                            return n;
                        }
                        "try" => return self.parse_try_stmt(),
                        "with" => return self.parse_with_stmt(),
                        "import" => return self.parse_import_stmt(),
                        "from" => {
                            if self.is_from_import_lookahead() {
                                return self.parse_from_import_stmt();
                            } else {
                                return self.parse_query_expr();
                            }
                        }
                        "select" => return self.parse_select_stmt(),
                        "assert" => return self.parse_assert_stmt(),
                        "raise" => return self.parse_raise_stmt(),
                        "op" | "operator" => {
                            let n = self.parse_operator_decl();
                            let vis = vis_opt.unwrap_or(VisibilityTier::Public);
                            self.arena.node_visibility.insert(n.0, vis);
                            if let Some(c) = clr_opt { self.arena.node_clearance.insert(n.0, c); }
                            return n;
                        }
                        "ext" | "extend" | "extension" => return self.parse_extend_decl(),
                        "database" => return self.parse_database_stmt(),
                        "transaction" => return self.parse_transaction_stmt(),
                        "savepoint" | "commit" | "rollback" => return self.parse_transaction_op_stmt(),
                        "insert" | "update" | "delete" => return self.parse_sql_mutation_stmt(),
                        "switch" => return self.parse_switch_stmt(),
                        "guard" => return self.parse_guard_stmt(),
                        "macro" | "defmacro" | "dsl" => return self.parse_macro_decl(),
                        "adapter" | "converter" => return self.parse_adapter_decl(),
                        "spawn" | "sync" | "atomic" | "isolate" => {
                            self.advance();
                            if self.at(TokenKind::LParen) {
                                let _ = self.parse_expr();
                            }
                            return self.parse_block_expr();
                        }
                        "parallel" if self.pos + 1 < self.tape.len() && self.tape.text(self.pos + 1, self.src) == "for" => {
                            self.advance();
                            return self.parse_for_stmt();
                        }
                        _ => {}
                    }
                }
                self.parse_expr_stmt()
            }
        }
    }

    fn parse_assert_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'assert'
        let mut msg_node = None;
        let cond = self.parse_expr();
        if self.at(TokenKind::Comma) {
            self.advance();
            msg_node = Some(self.parse_expr());
        }
        let callee = self.arena.alloc_ident("assert", start_span);
        let (arg_start, arg_len) = match msg_node {
            None => {
                if self.arena.node(cond).node_kind() == NodeKind::TupleExpr {
                    let slice = self.arena.children_slice(self.arena.node(cond).lhs, self.arena.node(cond).rhs);
                    if slice.len() == 2 {
                        self.arena.alloc_child_slice(&[callee, NodeId(slice[0]), NodeId(slice[1])])
                    } else {
                        self.arena.alloc_child_slice(&[callee, cond])
                    }
                } else {
                    self.arena.alloc_child_slice(&[callee, cond])
                }
            }
            Some(msg) => self.arena.alloc_child_slice(&[callee, cond, msg]),
        };
        let call_node = self.arena.alloc_node(NodeKind::CallExpr, 0, start_span, arg_start, arg_len);
        self.arena.alloc_node(NodeKind::ExprStmt, 0, start_span, call_node.0, 0)
    }

    fn parse_import_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'import'
        let mut path = String::new();
        let mut alias = None;

        while !self.at_end() && !self.at(TokenKind::Semicolon) && !self.at(TokenKind::Newline) {
            if self.at(TokenKind::KwAs) || (self.is_ident_like() && self.text() == "as") {
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
            if k == TokenKind::KwImport || (k == TokenKind::Ident && self.tape.text(p, self.src) == "import") {
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
            let prev_end = self.prev_span().end as usize;
            let cur_start = self.span().start as usize;
            if prev_end <= cur_start && cur_start <= self.src.len() && self.src[prev_end..cur_start].contains('\n') {
                break;
            }
            self.advance();
        }
        let end_pos = self.prev_span().end.max(start_span.end);
        let raw_txt = &self.src[start_span.start as usize..end_pos as usize];
        let block_id = self.arena.alloc_polyglot_block("sql", raw_txt, 0, ByteSpan::new(start_span.start, end_pos));
        self.arena.alloc_node(NodeKind::ExprStmt, 0, ByteSpan::new(start_span.start, end_pos), block_id.0, 0)
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
        let start_pos = self.pos;

        while !self.at_end() {
            let k = self.peek();
            if self.pos > start_pos && depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 {
                let prev_end = self.prev_span().end as usize;
                let cur_start = self.span().start as usize;
                if prev_end <= cur_start && cur_start <= self.src.len() && self.src[prev_end..cur_start].contains('\n') {
                    let is_continuation = matches!(
                        k,
                        TokenKind::KwWhere
                            | TokenKind::KwOrder
                            | TokenKind::KwBy
                            | TokenKind::KwGroup
                            | TokenKind::KwSelect
                            | TokenKind::KwLimit
                            | TokenKind::KwOffset
                            | TokenKind::KwJoin
                            | TokenKind::KwInner
                            | TokenKind::KwLeft
                            | TokenKind::KwRight
                            | TokenKind::KwCross
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
                            | TokenKind::KwAnd
                            | TokenKind::KwOr
                            | TokenKind::KwNot
                            | TokenKind::KwIn
                            | TokenKind::Comma
                    ) || (k == TokenKind::Ident && {
                        let txt = self.text();
                        matches!(txt, "where" | "order" | "by" | "group" | "select" | "limit" | "offset" | "join" | "inner" | "left" | "right" | "cross" | "from" | "having" | "with" | "into" | "on" | "as" | "ascending" | "descending" | "asc" | "desc" | "and" | "or" | "not" | "in" | "set" | "values" | "returning")
                    });
                    if !is_continuation {
                        break;
                    }
                }
            }
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
                TokenKind::Ident if depth_paren == 0 && depth_bracket == 0 && depth_brace == 0 && matches!(self.text(), "union" | "intersect" | "except") => {
                    break;
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
                                    | TokenKind::KwBy
                                    | TokenKind::KwGroup
                                    | TokenKind::KwSelect
                                    | TokenKind::KwLimit
                                    | TokenKind::KwOffset
                                    | TokenKind::KwJoin
                                    | TokenKind::KwInner
                                    | TokenKind::KwLeft
                                    | TokenKind::KwRight
                                    | TokenKind::KwCross
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
                                    | TokenKind::KwAnd
                                    | TokenKind::KwOr
                                    | TokenKind::KwNot
                                    | TokenKind::KwIn
                                    | TokenKind::Comma
                            ) || (next_k == TokenKind::Ident && {
                                let txt = &self.src[self.tape.span(p).start as usize..self.tape.span(p).end as usize];
                                matches!(txt, "where" | "order" | "by" | "group" | "select" | "limit" | "offset" | "join" | "inner" | "left" | "right" | "cross" | "from" | "having" | "with" | "into" | "on" | "as" | "ascending" | "descending" | "asc" | "desc" | "and" | "or" | "not" | "in" | "set" | "values" | "returning")
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
        while !self.at_end() && !self.at(TokenKind::KwImport) && !(self.is_ident_like() && self.text() == "import") && !self.at(TokenKind::Newline) && !self.at(TokenKind::Semicolon) {
            path.push_str(self.text());
            self.advance();
        }
        let path_sym = self.arena.interner.intern(&path);

        let mut imported_symbols = Vec::new();
        if self.at(TokenKind::KwImport) || (self.is_ident_like() && self.text() == "import") {
            self.advance(); // consume 'import'
            if self.at(TokenKind::LParen) {
                self.advance();
            }
            while !self.at_end() && !self.at(TokenKind::Semicolon) && !self.at(TokenKind::Newline) && !self.at(TokenKind::RParen) {
                if self.is_ident_like() {
                    let mut name = self.text().to_string();
                    self.advance();
                    if self.at(TokenKind::KwAs) || (self.is_ident_like() && self.text() == "as") {
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
        let from_node = self.arena.alloc_list_node(NodeKind::FromImportDecl, 0, ByteSpan::new(start_span.start, self.prev_span().end), &imported_symbols);
        self.arena.from_import_paths.insert(from_node.0, path_sym);
        from_node
    }

    fn parse_fn_decl(&mut self) -> NodeId {
        self.parse_fn_decl_modifiers(false, false)
    }

    fn parse_fn_decl_modifiers(&mut self, is_mut: bool, is_static: bool) -> NodeId {
        self.advance(); // consume 'def' or 'fn'
        self.parse_fn_decl_headless(is_mut, is_static)
    }

    fn parse_fn_decl_headless(&mut self, is_mut: bool, is_static: bool) -> NodeId {
        let start_span = self.span();
        let name = if self.is_ident_like() || self.peek().is_keyword() {
            let s = self.text();
            self.advance();
            s
        } else if self.at(TokenKind::StrLit) || self.at(TokenKind::TripleStrLit) {
            let s = self.text();
            self.advance();
            s
        } else {
            "anon"
        };
        let mut ext_target = None;
        let mut final_name = name;
        if (self.at(TokenKind::InfixDot) || (self.at(TokenKind::Dot) && self.is_ident_like_pos(self.pos + 1))) && !self.is_backtick_escaped() {
            self.advance();
            if self.is_ident_like() || self.peek().is_keyword() {
                ext_target = Some(name);
                final_name = self.text();
                self.advance();
            }
        }
        let sym = self.arena.interner.intern(final_name);

        if name.starts_with("__") && name.ends_with("__") && name.len() > 4 {
            let suggestion = match name {
                "__init__" | "__new__" => "Use 'self(params...)' constructor instead.",
                "__del__" => "Use destructor '~self' instead.",
                "__enter__" | "__exit__" => "Use 'scope self { enter: ... exit: ... }' instead.",
                "__getitem__" | "__setitem__" | "__missing__" => "Use indexer 'self[params...] -> T' instead.",
                "__getattr__" | "__setattr__" => "Use dynamic member dispatch 'self.(name: str) -> T' instead.",
                "__add__" => "Use 'op self + (rhs: R) -> Out' instead.",
                "__sub__" => "Use 'op self - (rhs: R) -> Out' instead.",
                "__mul__" => "Use 'op self * (rhs: R) -> Out' instead.",
                "__truediv__" | "__div__" => "Use 'op self / (rhs: R) -> Out' instead.",
                "__floordiv__" => "Use 'op self // (rhs: R) -> Out' instead.",
                "__mod__" => "Use 'op self % (rhs: R) -> Out' instead.",
                "__pow__" => "Use 'op self ** (rhs: R) -> Out' instead.",
                "__radd__" => "Use 'op (lhs: L) + self -> Out' instead.",
                "__rsub__" => "Use 'op (lhs: L) - self -> Out' instead.",
                "__rmul__" => "Use 'op (lhs: L) * self -> Out' instead.",
                "__rtruediv__" => "Use 'op (lhs: L) / self -> Out' instead.",
                "__iadd__" => "Use 'op mut self += (rhs: R)' instead.",
                "__isub__" => "Use 'op mut self -= (rhs: R)' instead.",
                "__imul__" => "Use 'op mut self *= (rhs: R)' instead.",
                "__itruediv__" => "Use 'op mut self /= (rhs: R)' instead.",
                "__neg__" => "Use 'op - self -> Out' instead.",
                "__invert__" => "Use 'op ~ self -> Out' instead.",
                "__eq__" | "__ne__" => "Use 'op self == (rhs: R) -> bool' instead.",
                "__lt__" | "__le__" | "__gt__" | "__ge__" => "Use comparison operators like 'op self < (rhs: R) -> bool' instead.",
                "__call__" => "Use 'op self(params...) -> Out' instead.",
                "__contains__" => "Use 'op (item: T) in self -> bool' instead.",
                "__instancecheck__" => "Use 'op (val: T) is Self -> bool' instead.",
                "__bool__" => "Use 'op self -> bool' instead.",
                "__int__" | "__float__" | "__str__" => "Use 'op self as Target' instead.",
                "__format__" => "Use 'op self as str(fmt: Slice[u8])' instead.",
                "__match_args__" => "Use 'op match self -> (T1, T2, ...)' instead.",
                "__copy__" | "__deepcopy__" => "Use 'op clone self -> Self' instead.",
                "__hash__" => "Use 'op hash self -> u64' instead.",
                "__getstate__" | "__setstate__" => "Use 'op pack self' / 'op unpack self' instead.",
                "__iter__" | "__next__" => "Use 'op for self -> T' instead.",
                "__await__" => "Use 'op await self -> T' instead.",
                "__aiter__" | "__anext__" => "Use 'op async for self -> T' instead.",
                _ => "Dunder magic methods are prohibited in Pilot. Use unified first-class 'self' or 'op' declarations.",
            };
            self.errors.push(format!("Python-style dunder method '{}' is prohibited: {}", name, suggestion));
        }

        // Generic type parameters: <T, U>
        if self.at(TokenKind::Less) {
            self.parse_generic_params();
        }

        // Parameters: (a: i64, b: i64)
        let mut params = Vec::new();
        if self.at(TokenKind::LParen) {
            self.advance();
            let mut last_was_comma = false;
            while !self.at_end() && !self.at(TokenKind::RParen) {
                if self.is_ident_like() && self.text() == "self" {
                    self.advance();
                    if self.at(TokenKind::Colon) {
                        self.advance();
                        let _ = self.parse_type();
                    }
                    self.errors.push("Explicit 'self' parameter is forbidden; methods have implicit receiver".to_string());
                    last_was_comma = false;
                    if self.at(TokenKind::Comma) {
                        self.advance();
                        last_was_comma = true;
                    }
                    continue;
                }
                while self.at(TokenKind::KwVar)
                    || self.at(TokenKind::KwRef)
                    || self.at(TokenKind::KwComptime)
                    || (self.is_ident_like() && (self.text() == "var" || self.text() == "ref" || self.text() == "val" || self.text() == "comptime" || self.text() == "context"))
                {
                    if self.pos + 1 < self.tape.len() {
                        let next_k = self.tape.kind(self.pos + 1);
                        if next_k == TokenKind::Colon || next_k == TokenKind::Comma || next_k == TokenKind::RParen || next_k == TokenKind::Equal {
                            break;
                        }
                    }
                    self.advance();
                }
                let is_unquote_param = if self.at(TokenKind::Dollar) {
                    self.advance(); // consume '$'
                    true
                } else {
                    false
                };
                if self.is_ident_like() || self.peek().is_keyword() {
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
                        if is_unquote_param { 1 } else { 0 },
                        ByteSpan::new(p_span.start, p_end),
                        p_sym.0,
                        type_node.0,
                    );
                    params.push(p_node);
                } else {
                    self.advance();
                }
                last_was_comma = false;
                if self.at(TokenKind::Comma) {
                    self.advance();
                    last_was_comma = true;
                } else {
                    break;
                }
            }
            if last_was_comma {
                self.errors.push("Trailing comma not allowed in parameter list".to_string());
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
        while self.at(TokenKind::KwRequires)
            || self.at(TokenKind::KwEnsures)
            || (self.is_ident_like() && matches!(self.text(), "requires" | "ensures"))
        {
            self.advance();
            while !self.at_end()
                && !self.at(TokenKind::LBrace)
                && !self.at(TokenKind::FatArrow)
                && !self.at(TokenKind::Colon)
                && !self.at(TokenKind::KwRequires)
                && !self.at(TokenKind::KwEnsures)
                && !(self.is_ident_like() && matches!(self.text(), "requires" | "ensures"))
                && !self.at(TokenKind::KwFn)
            {
                self.advance();
            }
            self.consume_semi_or_newlines();
        }

        // Body: => expr;, => { ... }, or { ... }
        let body = if self.at(TokenKind::FatArrow) {
            self.advance();
            if self.at(TokenKind::LBrace) {
                self.parse_block_expr()
            } else {
                let ret_expr = self.parse_expr();
                if self.at(TokenKind::Semicolon) {
                    self.advance();
                } else {
                    self.errors.push("Missing semicolon after expression body".to_string());
                }
                self.arena.alloc_node(NodeKind::ReturnStmt, 0, start_span, ret_expr.0, 0)
            }
        } else if self.at(TokenKind::LBrace) {
            self.parse_block_expr()
        } else {
            NodeId::NONE
        };

        let mut all_fn = Vec::with_capacity(1 + params.len());
        all_fn.push(body);
        all_fn.extend(params);
        let (fn_start, fn_len) = self.arena.alloc_child_slice(&all_fn);

        let fn_node = self.arena.alloc_node(NodeKind::FnDecl, fn_len as u16, start_span, sym.0, fn_start);
        if let Some(target_str) = ext_target {
            let target_sym = self.arena.interner.intern(target_str);
            self.arena.extension_methods.insert(fn_node.0, (target_sym, sym));
        }
        if ret_type_node.is_some() {
            self.arena.type_annotations.insert(fn_node.0, ret_type_node);
        }
        if is_static {
            self.arena.static_methods.insert(fn_node.0);
        }
        if is_mut {
            self.arena.mut_methods.insert(fn_node.0);
        }
        if !self.arena.node_visibility.contains_key(&fn_node.0) {
            self.arena.node_visibility.insert(fn_node.0, VisibilityTier::Private);
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
        self.advance(); // consume 'struct'
        self.parse_struct_decl_headless()
    }

    fn parse_struct_decl_headless(&mut self) -> NodeId {
        let start_span = self.span();
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);

        // Generic type params: <T, U>
        let type_params = if self.at(TokenKind::Less) {
            self.parse_generic_params()
        } else {
            Vec::new()
        };

        // Inheritance list: : IBase1, IBase2
        let mut base_types = Vec::new();
        if self.at(TokenKind::Colon) {
            self.advance(); // consume ':'
            let mut last_was_comma = false;
            while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::Semicolon) {
                let base_ty = self.parse_type();
                if base_ty.is_some() {
                    base_types.push(base_ty);
                }
                last_was_comma = false;
                if self.at(TokenKind::Comma) {
                    self.advance();
                    last_was_comma = true;
                } else {
                    break;
                }
            }
            if last_was_comma {
                self.errors.push("Trailing comma rejected in inheritance list".to_string());
            }
        }

        if !self.at(TokenKind::LBrace) {
            if self.at(TokenKind::Semicolon) {
                self.advance();
            }
            let span = ByteSpan::new(start_span.start, self.prev_span().end);
            let node = self.arena.alloc_node(NodeKind::StructDecl, 0, span, sym.0, 0);
            if !type_params.is_empty() {
                self.arena.struct_type_params.insert(node.0, type_params);
            }
            if !base_types.is_empty() {
                self.arena.struct_base_types.insert(node.0, base_types);
            }
            return node;
        }
        let body = self.parse_block_expr();
        let span = ByteSpan::new(start_span.start, self.arena.span(body).end);
        let node = self.arena.alloc_node(NodeKind::StructDecl, 0, span, sym.0, body.0);
        if !type_params.is_empty() {
            self.arena.struct_type_params.insert(node.0, type_params);
        }
        if !base_types.is_empty() {
            self.arena.struct_base_types.insert(node.0, base_types);
        }
        node
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
        self.advance(); // consume 'interface' or 'protocol'
        self.parse_interface_decl_headless()
    }

    fn parse_interface_decl_headless(&mut self) -> NodeId {
        let start_span = self.span();
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);

        // Generic type params: <T, U>
        let type_params = if self.at(TokenKind::Less) {
            self.parse_generic_params()
        } else {
            Vec::new()
        };

        // Inheritance list: : IBase1, IBase2
        let mut base_types = Vec::new();
        if self.at(TokenKind::Colon) {
            self.advance(); // consume ':'
            let mut last_was_comma = false;
            while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::Semicolon) {
                let base_ty = self.parse_type();
                if base_ty.is_some() {
                    base_types.push(base_ty);
                }
                last_was_comma = false;
                if self.at(TokenKind::Comma) {
                    self.advance();
                    last_was_comma = true;
                } else {
                    break;
                }
            }
            if last_was_comma {
                self.errors.push("Trailing comma rejected in inheritance list".to_string());
            }
        }

        if !self.at(TokenKind::LBrace) {
            if self.at(TokenKind::Semicolon) {
                self.advance();
            }
            let span = ByteSpan::new(start_span.start, self.prev_span().end);
            let node = self.arena.alloc_node(NodeKind::InterfaceDecl, 0, span, sym.0, 0);
            if !type_params.is_empty() {
                self.arena.struct_type_params.insert(node.0, type_params);
            }
            if !base_types.is_empty() {
                self.arena.struct_base_types.insert(node.0, base_types);
            }
            return node;
        }
        let body = self.parse_block_expr();
        let span = ByteSpan::new(start_span.start, self.arena.span(body).end);
        let node = self.arena.alloc_node(NodeKind::InterfaceDecl, 0, span, sym.0, body.0);
        if !type_params.is_empty() {
            self.arena.struct_type_params.insert(node.0, type_params);
        }
        if !base_types.is_empty() {
            self.arena.struct_base_types.insert(node.0, base_types);
        }
        node
    }

    fn parse_extend_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'extend' or 'extension' or 'ext'
        if self.at(TokenKind::LBracket) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::RBracket) {
                self.advance();
            }
            if self.at(TokenKind::RBracket) {
                self.advance();
            }
        }
        let first_name = self.text().to_string();
        self.advance();
        let (group_name, mut target) = if self.at(TokenKind::KwOn) || (self.is_ident_like() && self.text() == "on") {
            self.advance();
            let on_target = if !self.at_end() {
                let s = self.text().to_string();
                self.advance();
                s
            } else {
                first_name.clone()
            };
            (Some(first_name), on_target)
        } else {
            (None, first_name.clone())
        };
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
        let node = self.arena.alloc_node(NodeKind::ExtendDecl, 0, span, sym.0, body.0);
        let g_sym = if let Some(ref g) = group_name {
            self.arena.interner.intern(g)
        } else {
            sym
        };
        self.arena.named_extension_groups.insert(node.0, g_sym);
        node
    }

    fn parse_type_adt_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'type' or 'enum'
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);

        if self.at(TokenKind::Less) {
            self.parse_generic_params();
        }

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
                let mut where_expr = NodeId::NONE;
                if self.at(TokenKind::KwWhere) || (self.is_ident_like() && self.text() == "where") {
                    self.advance();
                    where_expr = self.parse_expr();
                }
                if self.at(TokenKind::Semicolon) {
                    self.advance();
                }
                let node = self.arena.alloc_node(NodeKind::TypeAdtDecl, 0, start_span, sym.0, ty_node.0);
                if where_expr.is_some() {
                    self.arena.refinement_predicates.insert(node.0, (ty_node, where_expr));
                }
                return node;
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
        let is_ref = self.at(TokenKind::KwRef) || (self.is_ident_like() && self.text() == "ref");
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

        if self.at(TokenKind::Less) {
            self.parse_generic_params();
        }

        // Optional type annotation: : type_spec
        let mut ty_node = NodeId::NONE;
        if self.at(TokenKind::Colon) {
            self.advance();
            ty_node = self.parse_type();
            if self.is_ident_like() && self.text() == "index" {
                self.advance();
                if self.is_ident_like() && self.text() == "by" {
                    self.advance();
                }
                if self.at(TokenKind::LParen) {
                    self.advance();
                    let mut depth = 1;
                    while !self.at_end() && depth > 0 {
                        if self.at(TokenKind::LParen) {
                            depth += 1;
                        } else if self.at(TokenKind::RParen) {
                            depth -= 1;
                        }
                        self.advance();
                    }
                }
            }
        }

        let init = if self.at(TokenKind::Equal) {
            self.advance();
            self.parse_expr()
        } else {
            NodeId::NONE
        };

        let end_pos = self.prev_span().end.max(start_span.end);
        let flags = if is_ref { 2 } else { 0 };
        let node_id = self.arena.alloc_node(
            kind,
            flags,
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

        while self.at(TokenKind::KwExcept)
            || self.at(TokenKind::KwCatch)
            || (self.is_ident_like() && matches!(self.text(), "except" | "catch"))
        {
            let clause_span = self.span();
            let is_catch = self.at(TokenKind::KwCatch) || (self.is_ident_like() && self.text() == "catch");
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
                if self.at(TokenKind::KwAs) || (self.is_ident_like() && self.text() == "as") {
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
        if self.at(TokenKind::KwElse) || (self.is_ident_like() && self.text() == "else") {
            self.advance();
            let else_body = self.parse_block_expr();
            clauses.push(else_body);
        }

        // Optional finally block: `finally { ... }`
        if self.at(TokenKind::KwFinally) || (self.is_ident_like() && self.text() == "finally") {
            self.advance();
            let finally_body = self.parse_block_expr();
            clauses.push(finally_body);
        }

        let (start, len) = self.arena.alloc_child_slice(&clauses);
        self.arena.alloc_node(NodeKind::TryStmt, 0, start_span, start, len)
    }

    fn parse_with_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'with'
        if self.is_ident_like() && self.text() == "context" {
            self.advance(); // consume 'context'
            let mut children = Vec::new();
            while !self.at_end() && !self.at(TokenKind::LBrace) {
                self.consume_semi_or_newlines();
                if self.at(TokenKind::LBrace) || self.at_end() {
                    break;
                }
                let prev_pos = self.pos;
                if self.at(TokenKind::KwLet)
                    || self.at(TokenKind::KwVar)
                    || (self.is_ident_like() && matches!(self.text(), "let" | "var"))
                {
                    let kind = if self.at(TokenKind::KwVar) || self.text() == "var" {
                        NodeKind::VarStmt
                    } else {
                        NodeKind::LetStmt
                    };
                    children.push(self.parse_binding_stmt(kind));
                } else if self.is_ident_like() {
                    let b_span = self.span();
                    let name = self.text().to_string();
                    self.advance();
                    let sym = self.arena.interner.intern(&name);
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
                    let end_pos = self.prev_span().end.max(b_span.end);
                    let b_node = self.arena.alloc_node(
                        NodeKind::LetStmt,
                        0,
                        ByteSpan::new(b_span.start, end_pos),
                        sym.0,
                        init.0,
                    );
                    if ty_node.is_some() {
                        self.arena.type_annotations.insert(b_node.0, ty_node);
                    }
                    children.push(b_node);
                } else {
                    let stmt = self.parse_stmt_or_decl();
                    if stmt.is_some() {
                        children.push(stmt);
                    }
                }
                if self.at(TokenKind::Comma) || self.at(TokenKind::Semicolon) {
                    self.advance();
                }
                if self.pos == prev_pos {
                    self.advance();
                }
            }
            if self.at(TokenKind::LBrace) {
                let block = self.parse_block_expr();
                if block.is_some() {
                    let blk = *self.arena.node(block);
                    children.extend(self.arena.children_slice(blk.lhs, blk.rhs).iter().map(|&id| NodeId(id)));
                }
            }
            let end_pos = self.prev_span().end.max(start_span.end);
            return self.arena.alloc_list_node(
                NodeKind::BlockExpr,
                0,
                ByteSpan::new(start_span.start, end_pos),
                &children,
            );
        }
        while !self.at_end() && !self.at(TokenKind::LBrace) && !self.at(TokenKind::Colon) && !self.at(TokenKind::Newline) {
            self.advance();
        }
        if self.at(TokenKind::Colon) {
            self.advance();
        }
        self.parse_block_expr()
    }

    #[inline]
    fn is_self_token(&self) -> bool {
        self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self")
    }

    fn peek_is_destructor_decl(&self) -> bool {
        if self.peek() == TokenKind::Tilde {
            let nk = self.peek_ahead(1);
            if nk == TokenKind::KwSelf {
                return true;
            }
            if (self.pos + 1) < self.tape.len() && self.tape.text(self.pos + 1, self.src) == "self" {
                return true;
            }
        }
        false
    }

    fn peek_is_scope_decl(&self) -> bool {
        let is_scope = self.peek() == TokenKind::KwScope || (self.is_ident_like() && self.text() == "scope");
        if is_scope {
            let nk = self.peek_ahead(1);
            if nk == TokenKind::KwSelf {
                return true;
            }
            if (self.pos + 1) < self.tape.len() && self.tape.text(self.pos + 1, self.src) == "self" {
                return true;
            }
        }
        false
    }

    fn peek_is_dynamic_member_decl(&self) -> bool {
        self.is_self_token() && self.peek_ahead(1) == TokenKind::Dot && self.peek_ahead(2) == TokenKind::LParen
    }

    fn peek_is_indexer_decl(&self) -> bool {
        let is_rec = self.is_self_token() || (self.is_ident_like() && self.text() == "this");
        if !is_rec || self.peek_ahead(1) != TokenKind::LBracket {
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
                        if after_k == TokenKind::Colon
                            || after_k == TokenKind::LBrace
                            || after_k == TokenKind::FatArrow
                            || after_k == TokenKind::Arrow
                            || after_k == TokenKind::VirtualLBrace
                        {
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

    fn peek_is_constructor_decl(&self) -> bool {
        if !self.is_self_token() || self.peek_ahead(1) != TokenKind::LParen {
            return false;
        }
        let lparen_pos = self.pos + 1;
        let rparen_pos = if let Some(target) = self.tape.matching_bracket(lparen_pos) {
            target as usize
        } else {
            let mut p = lparen_pos + 1;
            let mut depth = 1;
            while p < self.tape.len() && depth > 0 {
                if self.tape.kind(p) == TokenKind::LParen {
                    depth += 1;
                } else if self.tape.kind(p) == TokenKind::RParen {
                    depth -= 1;
                }
                p += 1;
            }
            if depth == 0 { p - 1 } else { return false; }
        };
        let mut after = rparen_pos + 1;
        while after < self.tape.len() && self.tape.kind(after) == TokenKind::Newline {
            after += 1;
        }
        if after < self.tape.len() {
            let nk = self.tape.kind(after);
            if nk == TokenKind::Arrow
                || nk == TokenKind::LBrace
                || nk == TokenKind::VirtualLBrace
                || nk == TokenKind::Colon
                || nk == TokenKind::Semicolon
            {
                return true;
            }
        }
        for i in (lparen_pos + 1)..rparen_pos {
            if self.tape.kind(i) == TokenKind::Colon {
                return true;
            }
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
        let clauses = self.parse_accessor_clauses(prop_ty);
        let end_span = self.prev_span();
        let span = ByteSpan::new(start_span.start, end_span.end);
        let prop_node = self.arena.alloc_node(NodeKind::FnDecl, 0, span, sym.0, 0);
        if prop_ty.is_some() {
            self.arena.type_annotations.insert(prop_node.0, prop_ty);
        }
        self.arena.accessor_clauses.insert(prop_node.0, clauses);
        prop_node
    }

    fn parse_constructor_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'self'
        self.expect(TokenKind::LParen);
        let mut params = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RParen) {
            self.consume_semi_or_newlines();
            if self.at(TokenKind::RParen) || self.at_end() {
                break;
            }
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

        // Constructor chaining: `-> self(other_args...)`
        let mut chain_call = NodeId::NONE;
        if self.at(TokenKind::Arrow) {
            self.advance(); // consume '->'
            if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
                let chain_span = self.span();
                self.advance(); // consume 'self'
                if self.at(TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    while !self.at_end() && !self.at(TokenKind::RParen) {
                        let arg = self.parse_expr();
                        if arg.is_some() {
                            args.push(arg);
                        }
                        if self.at(TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::RParen);
                    let chain_sym = self.arena.interner.intern("self");
                    let chain_fn = self.arena.alloc_node(NodeKind::IdentExpr, 0, chain_span, chain_sym.0, 0);
                    chain_call = self.arena.alloc_list_node(NodeKind::CallExpr, 0, ByteSpan::new(chain_span.start, self.prev_span().end), &args);
                    let mut c_node = *self.arena.node(chain_call);
                    c_node.lhs = chain_fn.0;
                    self.arena.nodes[chain_call.0 as usize] = c_node;
                }
            }
        }

        self.consume_semi_or_newlines();

        let body = if self.at(TokenKind::LBrace) || self.at(TokenKind::VirtualLBrace) || self.at(TokenKind::Colon) {
            self.parse_block_expr()
        } else {
            NodeId::NONE
        };

        let mut all_children = Vec::with_capacity(1 + params.len());
        all_children.push(body);
        all_children.extend(params);
        let (c_start, c_len) = self.arena.alloc_child_slice(&all_children);
        let span = ByteSpan::new(start_span.start, self.prev_span().end);
        let flags = if chain_call.is_some() { 1 } else { 0 };
        let ctor_node = self.arena.alloc_node(NodeKind::ConstructorDecl, flags, span, c_start, c_len);
        if chain_call.is_some() {
            self.arena.constructor_chains.insert(ctor_node.0, chain_call);
        }
        ctor_node
    }

    fn parse_destructor_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume '~'
        if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
            self.advance(); // consume 'self'
        }
        if self.at(TokenKind::LParen) {
            self.advance();
            if self.at(TokenKind::RParen) {
                self.advance();
            }
        }
        self.consume_semi_or_newlines();
        let body = if self.at(TokenKind::LBrace) || self.at(TokenKind::VirtualLBrace) || self.at(TokenKind::Colon) {
            self.parse_block_expr()
        } else {
            NodeId::NONE
        };
        let span = ByteSpan::new(start_span.start, self.prev_span().end);
        self.arena.alloc_node(NodeKind::DestructorDecl, 0, span, body.0, 0)
    }

    fn parse_scope_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'scope'
        if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
            self.advance(); // consume 'self'
        }
        self.consume_semi_or_newlines();
        let mut enter_body = NodeId::NONE;
        let mut exit_body = NodeId::NONE;
        if self.at(TokenKind::LBrace) || self.at(TokenKind::VirtualLBrace) || self.at(TokenKind::Colon) {
            let is_virtual = self.at(TokenKind::VirtualLBrace);
            if self.at(TokenKind::LBrace) || is_virtual {
                self.advance();
            } else if self.at(TokenKind::Colon) {
                self.advance();
                if self.at(TokenKind::VirtualLBrace) {
                    self.advance();
                }
            }
            while !self.at_end() && !self.at(TokenKind::RBrace) && !self.at(TokenKind::VirtualRBrace) {
                self.consume_semi_or_newlines();
                if self.at(TokenKind::RBrace) || self.at(TokenKind::VirtualRBrace) || self.at_end() {
                    break;
                }
                if self.at(TokenKind::KwEnter) || (self.is_ident_like() && self.text() == "enter") {
                    self.advance();
                    if self.at(TokenKind::Colon) {
                        self.advance();
                    }
                    enter_body = self.parse_block_expr();
                } else if self.at(TokenKind::KwExit) || (self.is_ident_like() && self.text() == "exit") {
                    self.advance();
                    if self.at(TokenKind::Colon) {
                        self.advance();
                    }
                    exit_body = self.parse_block_expr();
                } else {
                    self.advance();
                }
            }
            if self.at(TokenKind::RBrace) || self.at(TokenKind::VirtualRBrace) {
                self.advance();
            }
        }
        let span = ByteSpan::new(start_span.start, self.prev_span().end);
        let scope_node = self.arena.alloc_node(NodeKind::ScopeDecl, 0, span, enter_body.0, exit_body.0);
        self.arena.scope_clauses.insert(scope_node.0, (enter_body, exit_body));
        scope_node
    }

    fn parse_dynamic_member_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'self'
        self.expect(TokenKind::Dot);
        self.expect(TokenKind::LParen);
        let p_span = self.span();
        let name_sym = if self.is_ident_like() {
            let s = self.text();
            self.advance();
            self.arena.interner.intern(s)
        } else {
            self.arena.interner.intern("name")
        };
        let mut param_ty = NodeId::NONE;
        if self.at(TokenKind::Colon) {
            self.advance();
            param_ty = self.parse_type();
        }
        self.expect(TokenKind::RParen);
        let p_node = self.arena.alloc_node(NodeKind::ParamDecl, 0, p_span, name_sym.0, param_ty.0);

        let mut ret_ty = NodeId::NONE;
        if self.at(TokenKind::Arrow) || self.at(TokenKind::Colon) {
            self.advance();
            ret_ty = self.parse_type();
        }

        self.consume_semi_or_newlines();

        let clauses = self.parse_accessor_clauses(ret_ty);
        let span = ByteSpan::new(start_span.start, self.prev_span().end);
        let dyn_node = self.arena.alloc_node(NodeKind::DynamicMemberDecl, 0, span, p_node.0, 0);
        if ret_ty.is_some() {
            self.arena.type_annotations.insert(dyn_node.0, ret_ty);
        }
        self.arena.accessor_clauses.insert(dyn_node.0, clauses);
        dyn_node
    }

    fn parse_indexer_decl(&mut self, start_span: ByteSpan) -> NodeId {
        self.advance(); // consume 'self' or 'this'
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
        if self.at(TokenKind::Arrow) || self.at(TokenKind::Colon) {
            self.advance();
            if self.at(TokenKind::KwRef) || (self.is_ident_like() && self.text() == "ref") {
                self.advance();
            }
            prop_ty = self.parse_type();
        }

        self.consume_semi_or_newlines();

        let clauses = self.parse_accessor_clauses(prop_ty);
        let (p_start, p_len) = self.arena.alloc_child_slice(&params);
        let end_span = self.prev_span();
        let fn_span = ByteSpan::new(start_span.start, end_span.end);
        let idx_node = self.arena.alloc_node(NodeKind::IndexerDecl, 0, fn_span, p_start, p_len);
        if prop_ty.is_some() {
            self.arena.type_annotations.insert(idx_node.0, prop_ty);
        }
        self.arena.accessor_clauses.insert(idx_node.0, clauses);
        idx_node
    }

    fn parse_param_list(&mut self) -> Vec<NodeId> {
        let mut params = Vec::new();
        if self.at(TokenKind::LParen) {
            self.advance();
            let mut last_was_comma = false;
            while !self.at_end() && !self.at(TokenKind::RParen) {
                self.consume_semi_or_newlines();
                if self.at(TokenKind::RParen) || self.at_end() {
                    break;
                }
                while self.at(TokenKind::KwVar)
                    || self.at(TokenKind::KwRef)
                    || self.at(TokenKind::KwComptime)
                    || (self.is_ident_like() && (self.text() == "var" || self.text() == "ref" || self.text() == "val" || self.text() == "comptime" || self.text() == "context"))
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
                    if p_name == "self" {
                        self.errors.push("Explicit 1st-parameter 'self' is forbidden".to_string());
                    }
                    self.advance();
                    let p_sym = self.arena.interner.intern(p_name);
                    let mut p_ty = NodeId::NONE;
                    if self.at(TokenKind::Colon) {
                        self.advance();
                        p_ty = self.parse_type();
                    }
                    let p_end = self.prev_span().end.max(p_span.end);
                    let p_node = self.arena.alloc_node(
                        NodeKind::ParamDecl,
                        0,
                        ByteSpan::new(p_span.start, p_end),
                        p_sym.0,
                        p_ty.0,
                    );
                    params.push(p_node);
                } else {
                    self.advance();
                }
                last_was_comma = false;
                if self.at(TokenKind::Comma) {
                    self.advance();
                    last_was_comma = true;
                }
            }
            if last_was_comma {
                self.errors.push("Trailing comma rejected in function parameters".to_string());
            }
            self.expect(TokenKind::RParen);
        }
        params
    }

    fn parse_block_or_expr_body(&mut self) -> NodeId {
        self.consume_semi_or_newlines();
        if self.at(TokenKind::LBrace) || self.at(TokenKind::VirtualLBrace) || self.at(TokenKind::Colon) {
            self.parse_block_expr()
        } else {
            self.parse_expr()
        }
    }

    fn parse_operator_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'op' or 'operator'

        let mut op_kind = OperatorKind::BinaryLeft;
        let op_sym;
        let mut params = Vec::new();
        let mut ret_ty = NodeId::NONE;

        if self.at(TokenKind::KwClone) || (self.is_ident_like() && self.text() == "clone") {
            self.advance(); // consume 'clone'
            op_kind = OperatorKind::Clone;
            op_sym = self.arena.interner.intern("clone");
            if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
                self.advance();
            }
            if self.at(TokenKind::Arrow) {
                self.advance();
                ret_ty = self.parse_type();
            }
        } else if self.at(TokenKind::KwPack) || (self.is_ident_like() && self.text() == "pack") {
            self.advance();
            op_kind = OperatorKind::Pack;
            op_sym = self.arena.interner.intern("pack");
            if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
                self.advance();
            }
        } else if self.at(TokenKind::KwUnpack) || (self.is_ident_like() && self.text() == "unpack") {
            self.advance();
            op_kind = OperatorKind::Unpack;
            op_sym = self.arena.interner.intern("unpack");
            if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
                self.advance();
            }
        } else if self.at(TokenKind::KwMatch) {
            self.advance(); // consume 'match'
            op_kind = OperatorKind::Deconstruct;
            op_sym = self.arena.interner.intern("match");
            if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
                self.advance();
            }
            if self.at(TokenKind::Arrow) {
                self.advance();
                ret_ty = self.parse_type();
            }
        } else if self.at(TokenKind::KwAwait) {
            self.advance(); // consume 'await'
            op_kind = OperatorKind::AsyncFuture;
            op_sym = self.arena.interner.intern("await");
            if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
                self.advance();
            }
            if self.at(TokenKind::Arrow) {
                self.advance();
                ret_ty = self.parse_type();
            }
        } else if self.at(TokenKind::KwAsync) {
            self.advance(); // consume 'async'
            if self.at(TokenKind::KwFor) {
                self.advance(); // consume 'for'
            }
            op_kind = OperatorKind::AsyncStream;
            op_sym = self.arena.interner.intern("async_for");
            if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
                self.advance();
            }
            if self.at(TokenKind::Arrow) {
                self.advance();
                ret_ty = self.parse_type();
            }
        } else if self.at(TokenKind::KwFor) {
            self.advance(); // consume 'for'
            op_kind = OperatorKind::SyncIterator;
            op_sym = self.arena.interner.intern("for");
            if self.at(TokenKind::KwMut) {
                self.advance();
            }
            if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
                self.advance();
            }
            if self.at(TokenKind::Arrow) {
                self.advance();
                if self.at(TokenKind::KwRef) || (self.is_ident_like() && self.text() == "ref") {
                    self.advance();
                }
                ret_ty = self.parse_type();
            }
        } else if self.at(TokenKind::KwMut) {
            // Compound Mut: `op mut self <binop>= (rhs: R)`
            self.advance(); // consume 'mut'
            if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
                self.advance(); // consume 'self'
            }
            op_kind = OperatorKind::CompoundMut;
            let op_txt = self.text();
            op_sym = self.arena.interner.intern(op_txt);
            self.advance();
            if self.at(TokenKind::LParen) {
                params = self.parse_param_list();
            }
        } else if self.at(TokenKind::LParen) {
            // Binary Right: `op (lhs: L) <binop> self -> Out`
            // OR Membership: `op (item: T) in self -> bool`
            // OR Type Guard: `op (val: T) is Self -> bool`
            let parsed_params = self.parse_param_list();
            params = parsed_params;
            if self.at(TokenKind::KwIn) {
                self.advance(); // consume 'in'
                op_kind = OperatorKind::Membership;
                op_sym = self.arena.interner.intern("in");
                if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
                    self.advance();
                }
            } else if self.at(TokenKind::KwIs) {
                self.advance(); // consume 'is'
                op_kind = OperatorKind::TypeGuard;
                op_sym = self.arena.interner.intern("is");
                if self.is_ident_like() {
                    self.advance(); // Self
                }
            } else {
                op_kind = OperatorKind::BinaryRight;
                let op_txt = self.text();
                op_sym = self.arena.interner.intern(op_txt);
                self.advance();
                if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
                    self.advance();
                }
            }
            if self.at(TokenKind::Arrow) {
                self.advance();
                ret_ty = self.parse_type();
            }
        } else if self.at(TokenKind::Minus) || self.at(TokenKind::Tilde) || self.at(TokenKind::Exclamation) {
            // Unary Prefix: `op <unop> self -> Out` or `op ! () -> bool` or Binary Left: `op - (rhs: R) -> Out`
            let op_txt = self.text();
            op_sym = self.arena.interner.intern(op_txt);
            self.advance();
            if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
                self.advance();
            }
            if self.at(TokenKind::LParen) {
                params = self.parse_param_list();
            }
            op_kind = if params.is_empty() {
                OperatorKind::UnaryPrefix
            } else {
                OperatorKind::BinaryLeft
            };
            if self.at(TokenKind::Arrow) {
                self.advance();
                ret_ty = self.parse_type();
            }
        } else if self.at(TokenKind::LBracket) {
            self.advance(); // consume '['
            if self.at(TokenKind::RBracket) {
                self.advance(); // consume ']'
            }
            let op_str = if self.at(TokenKind::Equal) {
                self.advance(); // consume '='
                "[]="
            } else {
                "[]"
            };
            op_sym = self.arena.interner.intern(op_str);
            if self.at(TokenKind::LParen) {
                params = self.parse_param_list();
            }
            if self.at(TokenKind::Arrow) {
                self.advance();
                ret_ty = self.parse_type();
            }
        } else if self.is_self_token() {
            self.advance(); // consume 'self'
            if self.at(TokenKind::LParen) {
                // Functor Call: `op self(params...) -> Out`
                op_kind = OperatorKind::FunctorCall;
                op_sym = self.arena.interner.intern("call");
                params = self.parse_param_list();
                if self.at(TokenKind::Arrow) {
                    self.advance();
                    ret_ty = self.parse_type();
                }
            } else if self.at(TokenKind::Arrow) {
                // Implicit Cast: `op self -> Target`
                self.advance(); // consume '->'
                op_kind = OperatorKind::ImplicitCast;
                op_sym = self.arena.interner.intern("cast");
                ret_ty = self.parse_type();
            } else if self.at(TokenKind::KwAs) {
                // Explicit Cast: `op self as Target` or `op self as str(fmt: Slice[u8])`
                self.advance(); // consume 'as'
                if self.is_ident_like() && self.text() == "str" && self.peek_ahead(1) == TokenKind::LParen {
                    op_kind = OperatorKind::FormattedStr;
                    op_sym = self.arena.interner.intern("format");
                    self.advance(); // consume 'str'
                    params = self.parse_param_list();
                    ret_ty = self.arena.alloc_ident("str", self.prev_span());
                } else {
                    op_kind = OperatorKind::ExplicitCast;
                    op_sym = self.arena.interner.intern("as");
                    ret_ty = self.parse_type();
                }
            } else if self.at(TokenKind::EqEqual) {
                // Equality: `op self == (rhs: R) -> bool`
                self.advance(); // consume '=='
                op_kind = OperatorKind::Equality;
                op_sym = self.arena.interner.intern("==");
                if self.at(TokenKind::LParen) {
                    params = self.parse_param_list();
                }
                if self.at(TokenKind::Arrow) {
                    self.advance();
                    ret_ty = self.parse_type();
                }
            } else {
                // Binary Left: `op self <binop> (rhs: R) -> Out`
                op_kind = OperatorKind::BinaryLeft;
                let op_txt = self.text();
                op_sym = self.arena.interner.intern(op_txt);
                self.advance();
                if self.at(TokenKind::LParen) {
                    params = self.parse_param_list();
                }
                if self.at(TokenKind::Arrow) {
                    self.advance();
                    ret_ty = self.parse_type();
                }
            }
        } else {
            // Fallback: `op hash self -> u64` or `op + (rhs: R) -> Out`
            let op_txt = self.text();
            if op_txt == "hash" {
                self.advance();
                op_kind = OperatorKind::Hash;
                op_sym = self.arena.interner.intern("hash");
                if self.at(TokenKind::KwSelf) || (self.is_ident_like() && self.text() == "self") {
                    self.advance();
                }
            } else {
                if op_txt == "==" || op_txt == "!=" {
                    op_kind = OperatorKind::Equality;
                }
                op_sym = self.arena.interner.intern(op_txt);
                self.advance();
                if self.at(TokenKind::LParen) {
                    params = self.parse_param_list();
                }
            }
            if self.at(TokenKind::Arrow) {
                self.advance();
                ret_ty = self.parse_type();
            }
        }

        self.consume_semi_or_newlines();

        let mut raw_body_expr = NodeId::NONE;
        let body = if self.at(TokenKind::LBrace) || self.at(TokenKind::VirtualLBrace) || self.at(TokenKind::Colon) {
            let b = self.parse_block_expr();
            raw_body_expr = b;
            b
        } else if self.at(TokenKind::FatArrow) {
            self.advance();
            if self.at(TokenKind::LBrace) {
                let b = self.parse_block_expr();
                raw_body_expr = b;
                b
            } else {
                let expr = self.parse_expr();
                raw_body_expr = expr;
                if self.at(TokenKind::Semicolon) {
                    self.advance();
                }
                let ret = self.arena.alloc_node(NodeKind::ReturnStmt, 0, self.arena.span(expr), expr.0, 0);
                self.arena.alloc_list_node(NodeKind::BlockExpr, 0, self.arena.span(expr), &[ret])
            }
        } else {
            NodeId::NONE
        };

        let mut all_fn = Vec::with_capacity(1 + params.len());
        all_fn.push(body);
        all_fn.extend(params.iter().copied());
        let (fn_start, fn_len) = self.arena.alloc_child_slice(&all_fn);
        let span = ByteSpan::new(start_span.start, self.prev_span().end);
        let op_node = self.arena.alloc_node(NodeKind::OperatorDecl, op_kind as u16, span, fn_start, fn_len);
        if ret_ty.is_some() {
            self.arena.type_annotations.insert(op_node.0, ret_ty);
        }
        self.arena.operator_symbols.insert(op_node.0, op_sym);
        self.arena.operator_bodies.insert(op_node.0, raw_body_expr);
        self.arena.operator_params.insert(op_node.0, params);
        op_node
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
                let prev_pos = self.pos;
                if self.at(TokenKind::KwCase) || (self.is_ident_like() && self.text() == "case") {
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
                    let _cond = self.parse_expr();
                    if self.is_ident_like() && self.text() == "after" {
                        self.advance();
                        let _dur = self.parse_expr();
                    }
                    if self.at(TokenKind::FatArrow) {
                        self.advance();
                        let body = self.parse_block_expr();
                        arms.push(body);
                    } else {
                        arms.push(_cond);
                        if self.pos == prev_pos {
                            self.advance();
                        }
                    }
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
            if self.is_ident_like() && (self.peek_ahead(1) == TokenKind::KwElse || self.peek_ahead(1) == TokenKind::LBrace || (self.pos + 1 < self.tape.len() && self.tape.text(self.pos + 1, self.src) == "else")) {
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
        let else_body = if self.at(TokenKind::KwElse) || (self.is_ident_like() && self.text() == "else") {
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
        let body = if self.at(TokenKind::LBrace) || self.at(TokenKind::VirtualLBrace) {
            self.parse_block_expr()
        } else {
            self.parse_stmt_or_decl()
        };
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
        if has_paren && self.at(TokenKind::RParen) {
            self.advance();
        }
        if self.at(TokenKind::KwIn) || (self.is_ident_like() && self.text() == "in") {
            self.advance();
        } else {
            self.expect(TokenKind::KwIn);
        }
        let iter_expr = self.parse_expr();
        if has_paren && self.at(TokenKind::RParen) {
            self.advance();
        }
        if self.at(TokenKind::KwBounded) || (self.is_ident_like() && self.text() == "bounded") {
            self.advance();
            let _bound = self.parse_expr();
        }
        let body = self.parse_block_expr();
        if self.at(TokenKind::KwThen) || (self.is_ident_like() && self.text() == "then") {
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
        if self.at(TokenKind::LBrace) || self.at(TokenKind::VirtualLBrace) {
            self.advance();
        } else {
            let ctx_start = (self.span().start as usize).saturating_sub(60);
            let ctx_end = (self.span().end as usize + 20).min(self.src.len());
            self.errors.push(format!(
                "Expected '{{' starting block, found {:?} near {:?}",
                self.peek(),
                &self.src[ctx_start..ctx_end]
            ));
        }

        self.header_stack.push(self.active_header);
        let mut children = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) && !self.at(TokenKind::VirtualRBrace) {
            self.consume_semi_or_newlines();
            if self.at(TokenKind::RBrace) || self.at(TokenKind::VirtualRBrace) || self.at_end() {
                break;
            }
            let start_pos = self.pos;
            let child = self.parse_stmt_or_decl();
            if child.is_some() {
                if self.arena.node(child).node_kind() == NodeKind::Program {
                    let s = self.arena.node(child).lhs;
                    let l = self.arena.node(child).rhs;
                    let sub_children = self.arena.children_slice(s, l).to_vec();
                    for c in sub_children {
                        children.push(NodeId(c));
                    }
                } else {
                    children.push(child);
                }
            }
            if self.pos == start_pos {
                self.advance();
            }
            self.consume_semi_or_newlines();
        }

        if self.at(TokenKind::RBrace) || self.at(TokenKind::VirtualRBrace) {
            self.advance();
        }
        self.active_header = self.header_stack.pop().unwrap_or(None);

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
                        || nk == TokenKind::PrefixDot
                        || nk == TokenKind::InfixDot
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

            // Handle generic type arguments `<T, U>` on an identifier/member expression and struct initializers `Type { field: val }`
            let mut had_generic_args = false;
            if k == TokenKind::Less
                && matches!(self.arena.node(lhs).node_kind(), NodeKind::IdentExpr | NodeKind::MemberExpr)
            {
                let mut p = self.pos;
                let mut angle_depth = 0i32;
                let mut valid_type_args = true;
                while p < self.tape.len() {
                    let tk = self.tape.kind(p);
                    match tk {
                        TokenKind::Less => angle_depth += 1,
                        TokenKind::Greater => {
                            angle_depth -= 1;
                            if angle_depth <= 0 {
                                break;
                            }
                        }
                        TokenKind::ShiftRight => {
                            angle_depth -= 2;
                            if angle_depth <= 0 {
                                break;
                            }
                        }
                        TokenKind::Ident
                        | TokenKind::KwFn
                        | TokenKind::KwConst
                        | TokenKind::Comma
                        | TokenKind::LBracket
                        | TokenKind::RBracket
                        | TokenKind::LParen
                        | TokenKind::RParen
                        | TokenKind::Star
                        | TokenKind::Question
                        | TokenKind::Exclamation
                        | TokenKind::Vbar
                        | TokenKind::Amper
                        | TokenKind::Arrow
                        | TokenKind::FatArrow
                        | TokenKind::DotDot
                        | TokenKind::IntLit
                        | TokenKind::StrLit => {}
                        _ if self.is_ident_like_pos(p) || tk.is_keyword() => {}
                        _ => {
                            valid_type_args = false;
                            break;
                        }
                    }
                    p += 1;
                }
                if valid_type_args && angle_depth == 0 && p < self.tape.len() {
                    let after_p = p + 1;
                    let after_k = if after_p < self.tape.len() {
                        self.tape.kind(after_p)
                    } else {
                        TokenKind::Eof
                    };
                    let is_bundle_mod = self.is_ident_like_pos(after_p) && {
                        let t = self.tape.text(after_p, self.src);
                        matches!(t, "with" | "without" | "coalesce")
                    };
                    if matches!(
                        after_k,
                        TokenKind::LBrace
                            | TokenKind::Dot
                            | TokenKind::PrefixDot
                            | TokenKind::LParen
                            | TokenKind::Semicolon
                            | TokenKind::RParen
                            | TokenKind::Comma
                    ) || is_bundle_mod
                    {
                        if after_k == TokenKind::PrefixDot {
                            self.tape.kinds[after_p] = TokenKind::InfixDot;
                        }
                        self.pos = after_p;
                        self.skip_whitespace_and_comments();
                        had_generic_args = true;
                    }
                }
            }

            if self.peek() == TokenKind::LBrace
                && matches!(self.arena.node(lhs).node_kind(), NodeKind::IdentExpr | NodeKind::MemberExpr)
            {
                let first_p = self.next_non_newline_pos(self.pos + 1);
                let second_p = self.next_non_newline_pos(first_p + 1);
                let third_p = self.next_non_newline_pos(second_p + 1);
                let first_k = if first_p < self.tape.len() { self.tape.kind(first_p) } else { TokenKind::Eof };
                let second_k = if second_p < self.tape.len() { self.tape.kind(second_p) } else { TokenKind::Eof };
                let after_colon = if third_p < self.tape.len() { self.tape.kind(third_p) } else { TokenKind::Eof };

                let is_struct_init = if had_generic_args && first_k == TokenKind::RBrace {
                    true
                } else if self.is_ident_like_pos(first_p) && second_k == TokenKind::Colon {
                    let first_t = self.tape.text(first_p, self.src);
                    if matches!(first_t, "case" | "default" | "pub" | "priv" | "prot" | "pkg" | "int" | "enter" | "exit")
                        || matches!(after_colon, TokenKind::KwLoop | TokenKind::KwFor | TokenKind::KwWhile)
                    {
                        false
                    } else {
                        let mut p = first_p;
                        let mut depth = 0;
                        let mut has_semi = false;
                        while p < self.tape.len() {
                            let tk = self.tape.kind(p);
                            match tk {
                                TokenKind::LBrace | TokenKind::LBracket | TokenKind::LParen => depth += 1,
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
                                TokenKind::Semicolon if depth == 0 => {
                                    has_semi = true;
                                    break;
                                }
                                _ => {}
                            }
                            p += 1;
                        }
                        !has_semi
                    }
                } else {
                    false
                };
                if is_struct_init {
                    let map_expr = self.parse_expr_prefix(ExprPrecedence::Lowest as u8);
                    let (arg_start, arg_len) = self.arena.alloc_child_slice(&[lhs, map_expr]);
                    let span = ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end);
                    lhs = self.arena.alloc_node(NodeKind::CallExpr, 0, span, arg_start, arg_len);
                    continue;
                }
            }
            if had_generic_args {
                continue;
            }

            let k = self.peek();
            // Handle postfix operators
            if k == TokenKind::Dot || k == TokenKind::InfixDot || k == TokenKind::SafeNav {
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

            if k == TokenKind::Exclamation {
                let prev_end = self.arena.span(lhs).end as usize;
                let cur_start = self.span().start as usize;
                if prev_end <= cur_start && cur_start <= self.src.len() && !self.src[prev_end..cur_start].contains('\n') {
                    self.advance();
                    let span = ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end);
                    lhs = self.arena.alloc_node(NodeKind::UnaryExpr, 6, span, lhs.0, 0);
                    continue;
                }
            }

            if k == TokenKind::KwCatch || (k == TokenKind::Ident && self.text() == "catch") {
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

            if k == TokenKind::KwAs || (self.is_ident_like() && self.text() == "as") {
                if (ExprPrecedence::UnaryPrefix as u8) < min_bp {
                    break;
                }
                self.advance(); // consume 'as'
                let is_view = if self.is_ident_like() && self.text() == "view" {
                    self.advance(); // consume 'view'
                    true
                } else {
                    false
                };
                let target_ty = if is_view && self.at(TokenKind::Less) {
                    self.advance(); // consume '<'
                    let ty = self.parse_type();
                    self.expect_greater();
                    ty
                } else {
                    self.parse_type()
                };
                let op = if is_view { BinaryOp::AsView } else { BinaryOp::As };
                let span = self.arena.span(lhs);
                lhs = self.arena.alloc_node(NodeKind::BinaryExpr, op as u16, span, lhs.0, target_ty.0);
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
            let rhs = if op == BinaryOp::AfterRange
                && matches!(
                    self.peek(),
                    TokenKind::Semicolon
                        | TokenKind::RParen
                        | TokenKind::RBracket
                        | TokenKind::RBrace
                        | TokenKind::Comma
                        | TokenKind::Newline
                        | TokenKind::Eof
                )
            {
                NodeId::NONE
            } else {
                self.parse_expr_bp(next_min_bp)
            };
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
        while self.at(TokenKind::Newline) {
            self.advance();
        }
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

        if self.at(TokenKind::KwSelect) {
            return if self.peek_ahead(1) == TokenKind::LBrace {
                self.parse_select_stmt()
            } else {
                self.parse_query_expr()
            };
        }

        if self.is_ident_like() {
            if self.text() == "for" && (self.is_ident_like_pos(self.pos + 1) || self.peek_ahead(1) == TokenKind::LParen) {
                return self.parse_for_stmt();
            }
            if self.text() == "each" {
                self.advance();
                let operand = self.parse_expr_bp(ExprPrecedence::UnaryPrefix as u8);
                return self.arena.alloc_node(NodeKind::UnaryExpr, 0, span, operand.0, 0);
            }
            if self.text() == "lookup" && self.is_ident_like_pos(self.pos + 1) {
                return self.parse_query_expr();
            }
            if self.text() == "stackalloc" && self.is_ident_like_pos(self.pos + 1) {
                self.advance();
                let _ty = self.parse_type();
                return self.arena.alloc_node(NodeKind::ListExpr, 0, span, 0, 0);
            }
            if self.text() == "switch" {
                return self.parse_switch_stmt();
            }
            if self.text() == "select" {
                return if self.peek_ahead(1) == TokenKind::LBrace {
                    self.parse_select_stmt()
                } else {
                    self.parse_query_expr()
                };
            }
            if self.text() == "from"
                && self.is_ident_like_pos(self.pos + 1)
                && self.pos + 2 < self.tape.len()
                && self.tape.text(self.pos + 2, self.src) == "in"
            {
                return self.parse_query_expr();
            }
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
            if (name == "quote" || name == "quasiquote") && (self.at(TokenKind::LBrace) || self.at(TokenKind::VirtualLBrace)) {
                let callee = self.arena.alloc_ident(name, span);
                let body = self.parse_block_expr();
                let (slice_start, slice_len) = self.arena.alloc_child_slice(&[callee, body]);
                return self.arena.alloc_node(NodeKind::CallExpr, 0, span, slice_start, slice_len);
            }
            if name == "true" {
                return self.arena.alloc_node(NodeKind::BoolLit, 0, span, 1, 0);
            }
            if name == "false" {
                return self.arena.alloc_node(NodeKind::BoolLit, 0, span, 0, 0);
            }
            return self.arena.alloc_ident(name, span);
        }

        match k {
            TokenKind::KwEngine => {
                self.advance();
                self.arena.alloc_ident("__engine", span)
            }
            TokenKind::KwQuote => {
                self.advance();
                let callee = self.arena.alloc_ident("quote", span);
                let body = self.parse_block_expr();
                let (slice_start, slice_len) = self.arena.alloc_child_slice(&[callee, body]);
                self.arena.alloc_node(NodeKind::CallExpr, 0, ByteSpan::new(span.start, self.prev_span().end), slice_start, slice_len)
            }
            TokenKind::Star | TokenKind::DoubleStar | TokenKind::StarDotDot => {
                let (flags, op_str) = match k {
                    TokenKind::Star => (UNARY_OP_SPLAT, "*"),
                    TokenKind::DoubleStar => (UNARY_OP_DOUBLE_SPLAT, "**"),
                    _ => (UNARY_OP_DEEP_FLATTEN, "*.."),
                };
                self.advance();
                let bp = self.tables.prefix_precedence(op_str).unwrap_or(ExprPrecedence::UnaryPrefix as u8);
                let operand = self.parse_expr_bp(bp);
                self.arena.alloc_node(NodeKind::UnaryExpr, flags, span, operand.0, 0)
            }
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
                let is_fstring = txt.starts_with("f\"") || txt.starts_with("f'") || txt.starts_with("F\"") || txt.starts_with("F'") || txt.starts_with("$\"") || txt.starts_with("$'");
                let raw_content = if is_fstring {
                    &txt[1..]
                } else {
                    txt
                };
                let clean = if raw_content.starts_with("\"\"\"") && raw_content.ends_with("\"\"\"") && raw_content.len() >= 6 {
                    &raw_content[3..raw_content.len() - 3]
                } else if raw_content.starts_with("'''") && raw_content.ends_with("'''") && raw_content.len() >= 6 {
                    &raw_content[3..raw_content.len() - 3]
                } else {
                    raw_content.trim_matches('"').trim_matches('\'')
                };
                let sym = self.arena.interner.intern(clean);
                let flags = if is_fstring { 1 } else { 0 };
                self.arena.alloc_node(NodeKind::StrLit, flags, span, sym.0, 0)
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
            TokenKind::PrefixDot | TokenKind::InfixDot => {
                // PrefixDot shorthand `.field` -> `self.field`, lane index `.0`, or fluent piping `|> .Trim()`
                self.advance(); // consume '.'
                if self.is_ident_like() || self.at(TokenKind::IntLit) {
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
            | TokenKind::CallQuoteShallow
            | TokenKind::KwNot
            | TokenKind::KwEach
            | TokenKind::Amper
            | TokenKind::KwRef
            | TokenKind::ArrowLeft => {
                let (flags, op_str) = match k {
                    TokenKind::Amper | TokenKind::KwRef => (1, "&"),
                    TokenKind::ArrowLeft => (3, "<-"),
                    TokenKind::Tilde => (4, "~"),
                    TokenKind::Exclamation | TokenKind::CallQuoteShallow | TokenKind::KwNot => (5, "!"),
                    TokenKind::Minus => (0, "-"),
                    _ => (0, "+"),
                };
                self.advance();
                let bp = self.tables.prefix_precedence(op_str).unwrap_or(ExprPrecedence::UnaryPrefix as u8);
                let operand = self.parse_expr_bp(bp);
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
            TokenKind::KwSpawn | TokenKind::KwSync | TokenKind::KwAtomic | TokenKind::KwIsolate => {
                self.advance();
                self.parse_block_expr()
            }
            TokenKind::KwLambda => self.parse_lambda_expr(),
            TokenKind::KwFrom => self.parse_query_expr(),
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
                        let (yield_expr, comp_flag) = if self.at(TokenKind::Colon) {
                            self.advance();
                            let val_expr = self.parse_expr();
                            (self.arena.alloc_list_node(NodeKind::MapExpr, 0, span, &[key_or_val, val_expr]), 1)
                        } else {
                            (key_or_val, 2)
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
                        let for_node = self.arena.alloc_node(NodeKind::ForExpr, comp_flag, span, sym.0, body.0);
                        if iter_expr.is_some() {
                            self.arena.for_iter_nodes.insert(for_node.0, iter_expr);
                        }
                        return for_node;
                    }

                    // Check for Map literal `{ k1: v1, ... }` or `{ Hooks = { ... }, ... }` or Set literal `{ 10, 20, ... }` or `{}`
                    let mut is_map_lit = false;
                    let mut is_set_lit = false;
                    let first_p = self.next_non_newline_pos(self.pos + 1);
                    let first_inside = if first_p < self.tape.len() {
                        self.tape.kind(first_p)
                    } else {
                        TokenKind::Eof
                    };
                    if first_inside == TokenKind::RBrace {
                        is_map_lit = true;
                    } else if first_inside != TokenKind::KwLet
                        && first_inside != TokenKind::KwVar
                        && first_inside != TokenKind::KwFn
                        && first_inside != TokenKind::KwReturn
                        && first_inside != TokenKind::KwIf
                        && first_inside != TokenKind::KwLoop
                        && first_inside != TokenKind::KwDefer
                        && !(self.is_ident_like_pos(first_p)
                            && matches!(
                                self.tape.text(first_p, self.src),
                                "pub" | "priv" | "prot" | "pkg" | "int" | "op" | "fn" | "struct" | "interface" | "for" | "while" | "switch"
                            ))
                    {
                        let mut p = self.pos + 1;
                        let mut brace_depth = 0;
                        while p < self.tape.len() {
                            let tk = self.tape.kind(p);
                            match tk {
                                TokenKind::LBrace | TokenKind::LBracket | TokenKind::LParen => brace_depth += 1,
                                TokenKind::RBrace => {
                                    if brace_depth == 0 {
                                        break;
                                    }
                                    brace_depth -= 1;
                                }
                                TokenKind::RBracket | TokenKind::RParen => {
                                    if brace_depth > 0 {
                                        brace_depth -= 1;
                                    }
                                }
                                TokenKind::Colon | TokenKind::Equal => {
                                    if brace_depth == 0 {
                                        is_map_lit = true;
                                    }
                                }
                                TokenKind::Comma => {
                                    if brace_depth == 0 {
                                        is_set_lit = true;
                                    }
                                }
                                TokenKind::Semicolon => {
                                    if brace_depth == 0 {
                                        is_map_lit = false;
                                        is_set_lit = false;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            p += 1;
                        }
                    }

                    if is_map_lit || is_set_lit {
                        self.advance(); // consume '{'
                        let mut pairs = Vec::new();
                        while !self.at_end() && !self.at(TokenKind::RBrace) {
                            self.consume_semi_or_newlines();
                            if self.at_end() || self.at(TokenKind::RBrace) {
                                break;
                            }
                            let start_pos = self.pos;
                            let key = if self.is_ident_like()
                                && self.pos + 1 < self.tape.len()
                                && (self.tape.kind(self.pos + 1) == TokenKind::Colon || self.tape.kind(self.pos + 1) == TokenKind::Equal)
                            {
                                let k_span = self.span();
                                let k_name = self.text();
                                self.advance();
                                self.arena.alloc_ident(k_name, k_span)
                            } else {
                                self.parse_expr()
                            };
                            if is_map_lit && (self.at(TokenKind::Colon) || self.at(TokenKind::Equal)) {
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
                        let flag = if is_set_lit && !is_map_lit { 1 } else { 0 };
                        return self.arena.alloc_list_node(
                            NodeKind::MapExpr,
                            flag,
                            ByteSpan::new(span.start, self.prev_span().end),
                            &pairs,
                        );
                    }
                }
                self.parse_block_expr()
            }
            TokenKind::Dollar => {
                self.advance(); // consume '$'
                if self.at(TokenKind::LParen) {
                    self.advance();
                    let inner = self.parse_expr();
                    self.expect(TokenKind::RParen);
                    inner
                } else if self.is_ident_like() || self.peek().is_keyword() {
                    let name = self.text();
                    self.advance();
                    let sym = self.arena.interner.intern(name);
                    self.arena.alloc_node(NodeKind::IdentExpr, 1, span, sym.0, 0)
                } else {
                    NodeId::NONE
                }
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
            let mut last_was_comma = false;
            if self.at(TokenKind::Comma) {
                self.advance();
                last_was_comma = true;
            }
            if last_was_comma && self.at(TokenKind::RParen) {
                self.errors.push("Trailing comma rejected in function call arguments".to_string());
                break;
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
        let else_branch = if self.at(TokenKind::KwElif) || (self.is_ident_like() && self.text() == "elif") {
            self.parse_if_expr()
        } else if self.at(TokenKind::KwElse) || (self.is_ident_like() && self.text() == "else") {
            self.advance();
            if self.at(TokenKind::KwIf)
                || self.at(TokenKind::KwElif)
                || (self.is_ident_like() && (self.text() == "if" || self.text() == "elif"))
            {
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

    fn parse_switch_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'switch'
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
            if self.at(TokenKind::KwCase) || (self.is_ident_like() && (self.text() == "case" || self.text() == "default")) {
                let arm_span = self.span();
                let is_default = self.is_ident_like() && self.text() == "default";
                self.advance(); // consume 'case' or 'default'
                let pat = if is_default {
                    let wild_sym = self.arena.interner.intern("_");
                    self.arena.alloc_node(NodeKind::PatWildcard, 0, arm_span, wild_sym.0, 0)
                } else {
                    self.parse_pattern()
                };
                let has_guard = if self.at(TokenKind::KwIf) || (self.is_ident_like() && self.text() == "when") {
                    self.advance();
                    let _ = self.parse_expr();
                    true
                } else {
                    false
                };
                if self.at(TokenKind::FatArrow) || self.at(TokenKind::Colon) {
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
        let (bin_op, op_str, default_bp, default_rassoc) = match k {
            TokenKind::Plus => (BinaryOp::Add, "+", ExprPrecedence::AddSub as u8, false),
            TokenKind::Minus => (BinaryOp::Sub, "-", ExprPrecedence::AddSub as u8, false),
            TokenKind::Star => (BinaryOp::Mul, "*", ExprPrecedence::MulDivRem as u8, false),
            TokenKind::Slash => (BinaryOp::Div, "/", ExprPrecedence::MulDivRem as u8, false),
            TokenKind::DoubleSlash => (BinaryOp::FloorDiv, "//", ExprPrecedence::MulDivRem as u8, false),
            TokenKind::Percent => (BinaryOp::Rem, "%", ExprPrecedence::MulDivRem as u8, false),
            TokenKind::DoubleStar => (BinaryOp::Pow, "**", ExprPrecedence::Power as u8, true),
            TokenKind::EqEqual => (BinaryOp::Eq, "==", ExprPrecedence::Comparison as u8, false),
            TokenKind::NotEqual => (BinaryOp::Ne, "!=", ExprPrecedence::Comparison as u8, false),
            TokenKind::Less => (BinaryOp::Lt, "<", ExprPrecedence::Comparison as u8, false),
            TokenKind::LessEqual => (BinaryOp::Le, "<=", ExprPrecedence::Comparison as u8, false),
            TokenKind::Greater => (BinaryOp::Gt, ">", ExprPrecedence::Comparison as u8, false),
            TokenKind::GreaterEqual => (BinaryOp::Ge, ">=", ExprPrecedence::Comparison as u8, false),
            TokenKind::KwAnd | TokenKind::DoubleAmper => (BinaryOp::And, "&&", ExprPrecedence::LogicalAnd as u8, false),
            TokenKind::KwOr | TokenKind::DoublePipe => (BinaryOp::Or, "||", ExprPrecedence::LogicalOr as u8, false),
            TokenKind::Amper => (BinaryOp::BitAnd, "&", ExprPrecedence::BitAnd as u8, false),
            TokenKind::Vbar => (BinaryOp::BitOr, "|", ExprPrecedence::BitOr as u8, false),
            TokenKind::Caret => (BinaryOp::BitXor, "^", ExprPrecedence::BitXor as u8, false),
            TokenKind::ShiftLeft => (BinaryOp::Shl, "<<", ExprPrecedence::Shift as u8, false),
            TokenKind::ShiftRight => (BinaryOp::Shr, ">>", ExprPrecedence::Shift as u8, false),
            TokenKind::PipeForward => (BinaryOp::Pipe, "|>", ExprPrecedence::Pipe as u8, false),
            TokenKind::PipeBackward => (BinaryOp::PipeBackward, "<|", ExprPrecedence::Pipe as u8, true),
            TokenKind::Coalesce => (BinaryOp::Coalesce, "??", ExprPrecedence::Coalesce as u8, true),
            TokenKind::DotDot => (BinaryOp::RangeInclusive, "..", ExprPrecedence::Range as u8, false),
            TokenKind::DotDotEqual => (BinaryOp::RangeInclusive, "..=", ExprPrecedence::Range as u8, false),
            TokenKind::DotDotLess => (BinaryOp::RangeHalfOpen, "..<", ExprPrecedence::Range as u8, false),
            TokenKind::GreaterDotDotLess => (BinaryOp::OpenRange, ">..<", ExprPrecedence::Range as u8, false),
            TokenKind::GreaterDotDot => (BinaryOp::AfterRange, ">..", ExprPrecedence::Range as u8, false),
            TokenKind::Spaceship => (BinaryOp::Spaceship, "<=>", ExprPrecedence::Comparison as u8, false),
            TokenKind::PlusPipe => (BinaryOp::SatAdd, "+|", ExprPrecedence::AddSub as u8, false),
            TokenKind::MinusPipe => (BinaryOp::SatSub, "-|", ExprPrecedence::AddSub as u8, false),
            TokenKind::StarPipe => (BinaryOp::SatMul, "*|", ExprPrecedence::MulDivRem as u8, false),
            TokenKind::PlusPercent => (BinaryOp::WrapAdd, "+%", ExprPrecedence::AddSub as u8, false),
            TokenKind::MinusPercent => (BinaryOp::WrapSub, "-%", ExprPrecedence::AddSub as u8, false),
            TokenKind::StarPercent => (BinaryOp::WrapMul, "*%", ExprPrecedence::MulDivRem as u8, false),
            TokenKind::PlusQuestion => (BinaryOp::CheckedAdd, "+?", ExprPrecedence::AddSub as u8, false),
            TokenKind::MinusQuestion => (BinaryOp::CheckedSub, "-?", ExprPrecedence::AddSub as u8, false),
            TokenKind::StarQuestion => (BinaryOp::CheckedMul, "*?", ExprPrecedence::MulDivRem as u8, false),
            TokenKind::DoublePercent => (BinaryOp::EucMod, "%%", ExprPrecedence::MulDivRem as u8, false),
            TokenKind::TildeGreater => (BinaryOp::Lens, "~>", ExprPrecedence::Pipe as u8, false),
            TokenKind::SwapEqual => (BinaryOp::Swap, ":=:", ExprPrecedence::Assign as u8, false),
            TokenKind::KwIs => (BinaryOp::Is, "is", ExprPrecedence::Comparison as u8, false),
            TokenKind::KwIn => (BinaryOp::Eq, "in", ExprPrecedence::Comparison as u8, false),
            TokenKind::Walrus => (BinaryOp::Walrus, ":=", ExprPrecedence::Assign as u8, true),
            TokenKind::ArrowLeft => (BinaryOp::Send, "<-", ExprPrecedence::Pipe as u8, false),
            TokenKind::KwUnion => (BinaryOp::Union, "union", ExprPrecedence::BitOr as u8, false),
            TokenKind::KwIntersect => (BinaryOp::Intersect, "intersect", ExprPrecedence::BitOr as u8, false),
            TokenKind::KwExcept => (BinaryOp::Except, "except", ExprPrecedence::BitOr as u8, false),
            TokenKind::Ident => match self.text() {
                "union" => (BinaryOp::Union, "union", ExprPrecedence::BitOr as u8, false),
                "intersect" => (BinaryOp::Intersect, "intersect", ExprPrecedence::BitOr as u8, false),
                "except" => (BinaryOp::Except, "except", ExprPrecedence::BitOr as u8, false),
                "and" => (BinaryOp::And, "&&", ExprPrecedence::LogicalAnd as u8, false),
                "or" => (BinaryOp::Or, "||", ExprPrecedence::LogicalOr as u8, false),
                "is" => (BinaryOp::Is, "is", ExprPrecedence::Comparison as u8, false),
                _ => return None,
            },
            _ => return None,
        };
        if let Some(spec) = self.tables.pratt_infix_by_name.get(op_str) {
            Some((bin_op, spec.precedence, spec.assoc == Assoc::Right))
        } else {
            Some((bin_op, default_bp, default_rassoc))
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
                let new_lhs = self.arena.alloc_node(NodeKind::TypeOptional, 0, span, lhs.0, 0);
                if let Some(target) = self.tables.type_construct_targets.get("?").cloned() {
                    let sym = self.arena.interner.intern(&target);
                    self.arena.type_base_symbols.insert(new_lhs.0, sym);
                }
                lhs = new_lhs;
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

            if k == TokenKind::Arrow
                || (k == TokenKind::FatArrow && self.arena.node(lhs).node_kind() == NodeKind::TypeTuple)
            {
                // FnArrow: (A) -> B or (A) => B (bp = 10)
                if (TypePrecedence::FnArrow as u8) < min_bp {
                    break;
                }
                let span = self.span();
                self.advance();
                let ret = self.parse_type_bp(TypePrecedence::FnArrow as u8);
                let new_lhs = self.arena.alloc_node(NodeKind::TypeFn, 0, span, lhs.0, ret.0);
                if let Some(target) = self.tables.type_construct_targets.get("=>").cloned() {
                    let sym = self.arena.interner.intern(&target);
                    self.arena.type_base_symbols.insert(new_lhs.0, sym);
                }
                lhs = new_lhs;
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
                let new_lhs = self.arena.alloc_node(NodeKind::TypeFallible, 0, span, lhs.0, err.0);
                if let Some(target) = self.tables.type_construct_targets.get("!").cloned() {
                    let sym = self.arena.interner.intern(&target);
                    self.arena.type_base_symbols.insert(new_lhs.0, sym);
                }
                lhs = new_lhs;
                continue;
            }

            if k == TokenKind::Less {
                let start_span = self.span();
                self.advance(); // consume '<'
                let mut type_args = Vec::new();
                let mut last_was_comma = false;
                while !self.at_end() && !self.at_greater() {
                    let t = self.parse_type();
                    type_args.push(t);
                    last_was_comma = false;
                    if self.at(TokenKind::Comma) {
                        self.advance();
                        last_was_comma = true;
                    } else {
                        break;
                    }
                }
                if last_was_comma {
                    self.errors.push("Trailing comma rejected in generic arguments".to_string());
                }
                self.expect_greater();
                let span = ByteSpan::new(start_span.start, self.prev_span().end);
                let base_sym = if lhs.is_some() && self.arena.node(lhs).node_kind() == NodeKind::TypeNamed {
                    Some(SymbolId(self.arena.node(lhs).lhs))
                } else {
                    None
                };
                let base_name = base_sym
                    .map(|s| self.arena.interner.resolve(s).to_string())
                    .unwrap_or_default();
                let new_lhs = if self.tables.type_construct_targets.get("[]").map(|s| s.as_str()) == Some(base_name.as_str()) && type_args.len() == 1 {
                    self.arena.alloc_node(NodeKind::TypeSlice, 0, span, type_args[0].0, 0)
                } else if self.tables.type_construct_targets.get("[..]").map(|s| s.as_str()) == Some(base_name.as_str()) && type_args.len() == 1 {
                    self.arena.alloc_node(NodeKind::TypeSlice, 3, span, type_args[0].0, 0)
                } else if self.tables.type_construct_targets.get("[:]").map(|s| s.as_str()) == Some(base_name.as_str()) && type_args.len() == 1 {
                    self.arena.alloc_node(NodeKind::TypeSlice, 6, span, type_args[0].0, 0)
                } else if self.tables.type_construct_targets.get("^").map(|s| s.as_str()) == Some(base_name.as_str()) && type_args.len() == 1 {
                    self.arena.alloc_node(NodeKind::TypeSlice, 7, span, type_args[0].0, 0)
                } else if self.tables.type_construct_targets.get("@prefix").map(|s| s.as_str()) == Some(base_name.as_str()) && type_args.len() == 1 {
                    self.arena.alloc_node(NodeKind::TypeSlice, 8, span, type_args[0].0, 0)
                } else if self.tables.type_construct_targets.get("?").map(|s| s.as_str()) == Some(base_name.as_str()) && !type_args.is_empty() {
                    self.arena.alloc_node(NodeKind::TypeOptional, 0, span, type_args[0].0, 0)
                } else if self.tables.type_construct_targets.get("!").map(|s| s.as_str()) == Some(base_name.as_str()) && !type_args.is_empty() {
                    let err_id = if type_args.len() >= 2 { type_args[1].0 } else { 0 };
                    self.arena.alloc_node(NodeKind::TypeFallible, 0, span, type_args[0].0, err_id)
                } else if type_args.is_empty() {
                    self.arena.alloc_node(NodeKind::TypeList, 0, span, lhs.0, 0)
                } else if type_args.len() == 1 {
                    self.arena.alloc_node(NodeKind::TypeList, 0, span, type_args[0].0, 0)
                } else {
                    self.arena.alloc_list_node(NodeKind::TypeList, 0, span, &type_args)
                };
                if let Some(bs) = base_sym {
                    self.arena.type_base_symbols.insert(new_lhs.0, bs);
                }
                lhs = new_lhs;
                continue;
            }

            if k == TokenKind::Caret {
                // T^ -> signal<T> (or registered '^' construct)
                let start_span = self.span();
                self.advance(); // consume '^'
                let span = ByteSpan::new(start_span.start, self.prev_span().end);
                let new_lhs = self.arena.alloc_node(NodeKind::TypeSlice, 7, span, lhs.0, 0);
                if let Some(target) = self.tables.type_construct_targets.get("^").cloned() {
                    let sym = self.arena.interner.intern(&target);
                    self.arena.type_base_symbols.insert(new_lhs.0, sym);
                }
                lhs = new_lhs;
                continue;
            }

            if k == TokenKind::At {
                // T@N -> simd<T, N> (or registered '@' construct)
                let start_span = self.span();
                self.advance(); // consume '@'
                let n_expr = self.parse_expr_prefix(ExprPrecedence::Lowest as u8);
                let span = ByteSpan::new(start_span.start, self.prev_span().end);
                let new_lhs = self.arena.alloc_node(NodeKind::TypeSlice, 5, span, lhs.0, n_expr.0);
                if let Some(target) = self.tables.type_construct_targets.get("@").cloned() {
                    let sym = self.arena.interner.intern(&target);
                    self.arena.type_base_symbols.insert(new_lhs.0, sym);
                }
                lhs = new_lhs;
                continue;
            }

            if k == TokenKind::LBracket {
                let start_span = self.span();
                self.advance(); // consume '['
                if self.at(TokenKind::RBracket) {
                    // T[] -> slice<T> (or registered '[]' construct)
                    self.advance();
                    let span = ByteSpan::new(start_span.start, self.prev_span().end);
                    let new_lhs = self.arena.alloc_node(NodeKind::TypeSlice, 0, span, lhs.0, 0);
                    if let Some(target) = self.tables.type_construct_targets.get("[]").cloned() {
                        let sym = self.arena.interner.intern(&target);
                        self.arena.type_base_symbols.insert(new_lhs.0, sym);
                    }
                    lhs = new_lhs;
                    continue;
                } else if self.at(TokenKind::DotDot) {
                    // T[..] -> stride<T> (or registered '[..]' construct)
                    self.advance();
                    self.expect(TokenKind::RBracket);
                    let span = ByteSpan::new(start_span.start, self.prev_span().end);
                    let new_lhs = self.arena.alloc_node(NodeKind::TypeSlice, 3, span, lhs.0, 0);
                    if let Some(target) = self.tables.type_construct_targets.get("[..]").cloned() {
                        let sym = self.arena.interner.intern(&target);
                        self.arena.type_base_symbols.insert(new_lhs.0, sym);
                    }
                    lhs = new_lhs;
                    continue;
                } else if self.at(TokenKind::Colon) {
                    // T[:] -> ring<T> (or registered '[:]' construct)
                    self.advance();
                    self.expect(TokenKind::RBracket);
                    let span = ByteSpan::new(start_span.start, self.prev_span().end);
                    let new_lhs = self.arena.alloc_node(NodeKind::TypeSlice, 6, span, lhs.0, 0);
                    if let Some(target) = self.tables.type_construct_targets.get("[:]").cloned() {
                        let sym = self.arena.interner.intern(&target);
                        self.arena.type_base_symbols.insert(new_lhs.0, sym);
                    }
                    lhs = new_lhs;
                    continue;
                } else {
                    // T[N] -> array<T, N> or T[N, M] -> grid<T, N, M>
                    let n_expr = self.parse_expr();
                    if self.at(TokenKind::Comma) {
                        self.advance();
                        let m_expr = self.parse_expr();
                        self.expect(TokenKind::RBracket);
                        let span = ByteSpan::new(start_span.start, self.prev_span().end);
                        let (dims_start, _) = self.arena.alloc_child_slice(&[lhs, n_expr, m_expr]);
                        let new_lhs = self.arena.alloc_node(NodeKind::TypeSlice, 9, span, dims_start, 3);
                        if let Some(target) = self.tables.type_construct_targets.get("[N,M]").cloned() {
                            let sym = self.arena.interner.intern(&target);
                            self.arena.type_base_symbols.insert(new_lhs.0, sym);
                        }
                        lhs = new_lhs;
                        continue;
                    }
                    self.expect(TokenKind::RBracket);
                    let span = ByteSpan::new(start_span.start, self.prev_span().end);
                    let new_lhs = self.arena.alloc_node(NodeKind::TypeSlice, 1, span, lhs.0, n_expr.0);
                    if let Some(target) = self.tables.type_construct_targets.get("[N]").cloned() {
                        let sym = self.arena.interner.intern(&target);
                        self.arena.type_base_symbols.insert(new_lhs.0, sym);
                    }
                    lhs = new_lhs;
                    continue;
                }
            }

            break;
        }

        lhs
    }

    fn parse_type_prefix(&mut self) -> NodeId {
        let span = self.span();
        if self.at(TokenKind::At) {
            // @T -> pin<T> (or registered '@prefix' construct)
            let start_span = self.span();
            self.advance(); // consume '@'
            let inner = self.parse_type_prefix();
            let full_span = ByteSpan::new(start_span.start, self.prev_span().end);
            let node = self.arena.alloc_node(NodeKind::TypeSlice, 8, full_span, inner.0, 0);
            if let Some(target) = self.tables.type_construct_targets.get("@prefix").cloned() {
                let sym = self.arena.interner.intern(&target);
                self.arena.type_base_symbols.insert(node.0, sym);
            }
            node
        } else if self.at(TokenKind::Star) {
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
        } else if self.at(TokenKind::KwConst) || (self.is_ident_like() && self.text() == "const") {
            self.advance();
            self.parse_type_prefix()
        } else if self.at(TokenKind::KwVar) || (self.is_ident_like() && self.text() == "var") {
            self.advance();
            self.parse_type_prefix()
        } else if self.is_ident_like() || self.at(TokenKind::KeywordLit) || self.at(TokenKind::KwFn) || self.at(TokenKind::KwRef) || self.at(TokenKind::IntLit) {
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
        } else if self.at(TokenKind::ColonColon) {
            self.advance(); // consume '::'
            let name = if self.is_ident_like() || self.peek().is_keyword() {
                let s = self.text().to_string();
                self.advance();
                s
            } else {
                String::new()
            };
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
                let sym = self.arena.interner.intern(&name);
                self.arena.alloc_node(NodeKind::PatVariant, 0, span, sym.0, 0)
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
    fn test_parse_type_safety_declarations() {
        let src = r#"
            type Result<T, E> = Ok(value: T) | Err(error: E);
            record Config(host: str, port: int, active: bool);
            class Handler {
                fn process(item: any) -> Result<str, str> {
                    return Result.Ok("done");
                }
            }
        "#;
        let arena = parse(src);
        assert!(arena.nodes.len() > 10);
    }

    #[test]
    fn test_parse_comprehensive_syntax() {
        let src = r#"
            fn helper(x: int) -> int {
                return x * 2;
            }

            fn main() {
                let list = [1, 2, 3];
                let doubled = [helper(i) for i in list];
                println(doubled);
            }
        "#;
        let arena = parse(src);
        let root = arena.node(arena.root);
        let children = arena.children_slice(root.lhs, root.rhs);
        let top_fn_names: Vec<_> = children
            .iter()
            .map(|&c| arena.node(NodeId(c)))
            .filter(|n| n.node_kind() == NodeKind::FnDecl)
            .map(|n| arena.interner.resolve(SymbolId(n.lhs)))
            .collect();
        assert!(top_fn_names.contains(&"helper"));
        assert!(top_fn_names.contains(&"main"));
    }

    #[test]
    fn test_parse_complex_declarations() {
        let src = r#"
            struct Point3D { x: float; y: float; z: float; }
            record Metadata(id: int, tags: List<str>);
            enum Status { Pending, Active, Complete }
            fn process_point(pt: Point3D) => pt.x + pt.y + pt.z;
        "#;
        let arena = parse(src);
        let root = arena.node(arena.root);
        let children = arena.children_slice(root.lhs, root.rhs);
        assert!(children.len() >= 4);
    }

    #[test]
    fn test_verification_gate_distributive_blocks_ast_equivalence() {
        // Gate 2.1: let List<i32> { myarr1 = [1, 2, 3], myarr2 = [4, 5, 6,], };
        let dist_let = "let List<i32> { myarr1 = [1, 2, 3], myarr2 = [4, 5, 6,], };";
        let indiv_let = "let myarr1: List<i32> = [1, 2, 3];\nlet myarr2: List<i32> = [4, 5, 6,];";
        let mut p1 = Parser::new(dist_let, tokenize(dist_let));
        let arena1 = p1.parse_program();
        assert!(p1.errors.is_empty(), "Distributive let block must have no errors: {:?}", p1.errors);

        let mut p2 = Parser::new(indiv_let, tokenize(indiv_let));
        let arena2 = p2.parse_program();
        assert!(p2.errors.is_empty(), "Individual let decls must have no errors: {:?}", p2.errors);

        let root1 = arena1.node(arena1.root);
        let root2 = arena2.node(arena2.root);
        let children1 = arena1.children_slice(root1.lhs, root1.rhs);
        let children2 = arena2.children_slice(root2.lhs, root2.rhs);
        assert_eq!(children1.len(), children2.len(), "Both ASTs must have identical child counts");

        for i in 0..children1.len() {
            let n1 = arena1.node(NodeId(children1[i]));
            let n2 = arena2.node(NodeId(children2[i]));
            assert_eq!(n1.node_kind(), n2.node_kind());
            let name1 = arena1.interner.resolve(SymbolId(n1.lhs));
            let name2 = arena2.interner.resolve(SymbolId(n2.lhs));
            assert_eq!(name1, name2, "Field names must match");
            assert!(arena1.type_annotations.contains_key(&children1[i]));
            assert!(arena2.type_annotations.contains_key(&children2[i]));
        }

        // Gate 2.2: var Vec3<f64> { a = (1.0, 2.0, 3.0), b = (9.0, 8.0, 7.0,), };
        let dist_var = "var Vec3<f64> { a = (1.0, 2.0, 3.0), b = (9.0, 8.0, 7.0,), };";
        let indiv_var = "var a: Vec3<f64> = (1.0, 2.0, 3.0);\nvar b: Vec3<f64> = (9.0, 8.0, 7.0,);";
        let mut pv1 = Parser::new(dist_var, tokenize(dist_var));
        let arena_v1 = pv1.parse_program();
        assert!(pv1.errors.is_empty(), "Distributive var block must have no errors: {:?}", pv1.errors);

        let mut pv2 = Parser::new(indiv_var, tokenize(indiv_var));
        let arena_v2 = pv2.parse_program();
        assert!(pv2.errors.is_empty(), "Individual var decls must have no errors: {:?}", pv2.errors);

        let root_v1 = arena_v1.node(arena_v1.root);
        let root_v2 = arena_v2.node(arena_v2.root);
        let ch_v1 = arena_v1.children_slice(root_v1.lhs, root_v1.rhs);
        let ch_v2 = arena_v2.children_slice(root_v2.lhs, root_v2.rhs);
        assert_eq!(ch_v1.len(), ch_v2.len());
        for i in 0..ch_v1.len() {
            let n1 = arena_v1.node(NodeId(ch_v1[i]));
            let n2 = arena_v2.node(NodeId(ch_v2[i]));
            assert_eq!(n1.node_kind(), n2.node_kind());
            assert_eq!(n1.node_kind(), NodeKind::VarStmt);
            let name1 = arena_v1.interner.resolve(SymbolId(n1.lhs));
            let name2 = arena_v2.interner.resolve(SymbolId(n2.lhs));
            assert_eq!(name1, name2);
        }

        // Gate 2.3: pub static pure: header label
        let header_src = "pub static pure:\nfn compute() -> i32 { return 42; }";
        let indiv_src = "pub static pure fn compute() -> i32 { return 42; }";
        let mut ph1 = Parser::new(header_src, tokenize(header_src));
        let arena_h1 = ph1.parse_program();
        assert!(ph1.errors.is_empty(), "Header label must have no errors: {:?}", ph1.errors);

        let mut ph2 = Parser::new(indiv_src, tokenize(indiv_src));
        let arena_h2 = ph2.parse_program();
        assert!(ph2.errors.is_empty(), "Individual modifiers must have no errors: {:?}", ph2.errors);

        let fn1_id = arena_h1.children_slice(arena_h1.node(arena_h1.root).lhs, arena_h1.node(arena_h1.root).rhs)[0];
        let fn2_id = arena_h2.children_slice(arena_h2.node(arena_h2.root).lhs, arena_h2.node(arena_h2.root).rhs)[0];
        assert_eq!(arena_h1.node_visibility.get(&fn1_id), arena_h2.node_visibility.get(&fn2_id));
        assert_eq!(arena_h1.node_visibility.get(&fn1_id), Some(&VisibilityTier::Public));
        assert!(arena_h1.static_methods.contains(&fn1_id));
        assert!(arena_h2.static_methods.contains(&fn2_id));
        assert!(arena_h1.pure_methods.contains(&fn1_id));
        assert!(arena_h2.pure_methods.contains(&fn2_id));
    }

    #[test]
    fn test_verification_gate_trailing_commas() {
        // Gate 3.1: Collections allow trailing comma
        let valid_collections = r#"
            let arr = [1, 2, 3,];
            let tup = (1, 2,);
            let map = { "a": 1, "b": 2, };
        "#;
        let mut p_ok = Parser::new(valid_collections, tokenize(valid_collections));
        p_ok.parse_program();
        assert!(p_ok.errors.is_empty(), "Collection trailing commas must be allowed: {:?}", p_ok.errors);

        // Gate 3.2: Attribute list rejects trailing comma
        let bad_attr = "[Attr1, Attr2,] fn foo() {}";
        let mut p_attr = Parser::new(bad_attr, tokenize(bad_attr));
        p_attr.parse_program();
        assert!(!p_attr.errors.is_empty(), "Trailing comma in attributes must be rejected");
        assert!(p_attr.errors.iter().any(|e| e.contains("attribute brackets")));

        // Gate 3.3: Function params reject trailing comma
        let bad_param = "fn foo(x: i32,) {}";
        let mut p_param = Parser::new(bad_param, tokenize(bad_param));
        p_param.parse_program();
        assert!(!p_param.errors.is_empty(), "Trailing comma in function params must be rejected");
        assert!(p_param.errors.iter().any(|e| e.contains("parameter list")));

        // Gate 3.4: Function call args reject trailing comma
        let bad_call = "let x = foo(1, 2,);";
        let mut p_call = Parser::new(bad_call, tokenize(bad_call));
        p_call.parse_program();
        assert!(!p_call.errors.is_empty(), "Trailing comma in call args must be rejected");

        // Gate 3.5: Generic parameters reject trailing comma
        let bad_generic = "struct Container<T, U,> {}";
        let mut p_generic = Parser::new(bad_generic, tokenize(bad_generic));
        p_generic.parse_program();
        assert!(!p_generic.errors.is_empty(), "Trailing comma in generic parameters must be rejected");

        // Gate 3.6: Inheritance list rejects trailing comma
        let bad_inherit = "struct Sub : Base1, Base2, {}";
        let mut p_inherit = Parser::new(bad_inherit, tokenize(bad_inherit));
        p_inherit.parse_program();
        assert!(!p_inherit.errors.is_empty(), "Trailing comma in inheritance list must be rejected");
    }

    #[test]
    fn test_verification_gate_switch_and_lambdas() {
        let code = r#"
            fn test_switch(x: i32) -> i32 {
                switch x {
                    case 1 => return 10;
                    case 2: return 20;
                    default => return 0;
                }
            }

            fn test_closures() {
                let f1 = (x) => x + 1;
                let f2 = (x, y) => { return x + y; };
                let f3 = () => 42;
            }
        "#;
        let mut p = Parser::new(code, tokenize(code));
        let arena = p.parse_program();
        assert!(p.errors.is_empty(), "Switch and C# closures must parse without errors: {:?}", p.errors);
        assert!(arena.nodes.len() > 10);
    }
}
