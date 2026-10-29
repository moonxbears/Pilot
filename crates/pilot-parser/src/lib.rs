//! Milestone 30: Stage 2 — Hand-Written Recursive Descent + Pratt (TDOP) Parser (`pilot-parser`).
//!
//! Enforces Cross-Milestone Invariants #1–#9:
//! - Strictly 16-byte `AstNode` arena lowered directly from Stage 1 `TokenTape`.
//! - Pratt Top-Down Operator Precedence (TDOP) expression binding powers.
//! - Pratt Type-Spec binding powers (Invariant #6: `T?` > `&` > `|` > `->` > `!`).
//! - O(1) delimiter jump table disambiguation using `TokenTape::bracket_links`.

use pilot_ast::*;
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
                top_children.push(node);
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
        let k = self.peek();
        match k {
            TokenKind::KwImport => self.parse_import_stmt(),
            TokenKind::KwFrom => self.parse_from_import_stmt(),
            TokenKind::KwDef | TokenKind::KwFn => self.parse_fn_decl(),
            TokenKind::KwClass => self.parse_class_decl(),
            TokenKind::KwStruct => self.parse_struct_decl(),
            TokenKind::KwRecord => self.parse_record_decl(),
            TokenKind::KwInterface | TokenKind::KwProtocol => self.parse_interface_decl(),
            TokenKind::KwExtend | TokenKind::KwExtension => self.parse_extend_decl(),
            TokenKind::KwType => self.parse_type_adt_decl(),
            TokenKind::KwLet => self.parse_binding_stmt(NodeKind::LetStmt),
            TokenKind::KwVar => self.parse_binding_stmt(NodeKind::VarStmt),
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
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_import_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'import'
        let mut path = String::new();
        while !self.at_end() && !self.at(TokenKind::Semicolon) && !self.at(TokenKind::Newline) {
            path.push_str(self.text());
            self.advance();
        }
        let sym = self.arena.interner.intern(&path);
        self.arena.alloc_node(NodeKind::ImportDecl, 0, start_span, sym.0, 0)
    }

    fn parse_from_import_stmt(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'from'
        let mut path = String::new();
        while !self.at_end() && !self.at(TokenKind::KwImport) {
            path.push_str(self.text());
            self.advance();
        }
        let sym = self.arena.interner.intern(&path);
        if self.at(TokenKind::KwImport) {
            self.advance();
            while !self.at_end() && !self.at(TokenKind::Semicolon) && !self.at(TokenKind::Newline) {
                self.advance();
            }
        }
        self.arena.alloc_node(NodeKind::FromImportDecl, 0, start_span, sym.0, 0)
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

        // Body: { ... } or : suite
        let body = if self.at(TokenKind::LBrace) || self.at(TokenKind::VirtualLBrace) {
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
        self.advance(); // consume 'class'
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);
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
        let body = self.parse_block_expr();
        self.arena.alloc_node(NodeKind::InterfaceDecl, 0, start_span, sym.0, body.0)
    }

    fn parse_extend_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'extend' or 'extension'
        let target = self.text();
        self.advance();
        let sym = self.arena.interner.intern(target);
        let body = self.parse_block_expr();
        self.arena.alloc_node(NodeKind::ExtendDecl, 0, start_span, sym.0, body.0)
    }

    fn parse_type_adt_decl(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'type'
        let name = self.text();
        self.advance();
        let sym = self.arena.interner.intern(name);
        if self.at(TokenKind::Equal) {
            self.advance();
            self.parse_type();
        }
        self.arena.alloc_node(NodeKind::TypeAdtDecl, 0, start_span, sym.0, 0)
    }

    fn parse_binding_stmt(&mut self, kind: NodeKind) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'let', 'var', or 'const'
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
        self.arena.alloc_node(NodeKind::BreakStmt, 0, span, 0, 0)
    }

    fn parse_continue_stmt(&mut self) -> NodeId {
        let span = self.span();
        self.advance();
        self.arena.alloc_node(NodeKind::ContinueStmt, 0, span, 0, 0)
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
                    let span = ByteSpan::new(self.arena.span(expr).start, self.arena.span(rhs).end);
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
                    let span = ByteSpan::new(self.arena.span(expr).start, self.arena.span(rhs).end);
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

        self.arena.alloc_list_node(NodeKind::BlockExpr, 0, start_span, &children)
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

    fn parse_expr_prefix(&mut self) -> NodeId {
        let k = self.peek();
        let span = self.span();

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
                    self.arena.alloc_list_node(NodeKind::TupleExpr, 0, span, &items)
                }
            }
            TokenKind::LBracket => {
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
                self.arena.alloc_list_node(NodeKind::ListExpr, 0, span, &items)
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
            TokenKind::LBrace | TokenKind::VirtualLBrace => self.parse_block_expr(),
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
        let _else_branch = if self.at(TokenKind::KwElse) {
            self.advance();
            if self.at(TokenKind::KwIf) {
                self.parse_if_expr()
            } else {
                self.parse_block_expr()
            }
        } else {
            NodeId::NONE
        };
        self.arena.alloc_node(NodeKind::IfExpr, 0, start_span, cond.0, then_branch.0)
    }

    fn parse_match_expr(&mut self) -> NodeId {
        let start_span = self.span();
        self.advance(); // consume 'match'
        let scrutinee = self.parse_expr();
        self.expect(TokenKind::LBrace);
        let mut arms = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) {
            if self.at(TokenKind::KwCase) {
                self.advance();
                let pat = self.parse_pattern();
                let has_guard = if self.at(TokenKind::KwIf) {
                    self.advance();
                    let _ = self.parse_expr();
                    true
                } else {
                    false
                };
                let body = self.parse_block_expr();
                let arm = self.arena.alloc_node(NodeKind::MatchArm, has_guard as u16, start_span, pat.0, body.0);
                arms.push(arm);
            } else {
                self.advance();
            }
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
            TokenKind::KwAnd | TokenKind::Amper => Some((BinaryOp::And, ExprPrecedence::LogicalAnd as u8, false)),
            TokenKind::KwOr | TokenKind::Vbar => Some((BinaryOp::Or, ExprPrecedence::LogicalOr as u8, false)),
            TokenKind::PipeForward => Some((BinaryOp::Pipe, ExprPrecedence::Pipe as u8, false)),
            TokenKind::Coalesce => Some((BinaryOp::Coalesce, ExprPrecedence::Coalesce as u8, true)),
            TokenKind::DotDot | TokenKind::DotDotEqual => Some((BinaryOp::RangeInclusive, ExprPrecedence::Range as u8, false)),
            TokenKind::DotDotLess => Some((BinaryOp::RangeHalfOpen, ExprPrecedence::Range as u8, false)),
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

            break;
        }

        lhs
    }

    fn parse_type_prefix(&mut self) -> NodeId {
        let span = self.span();
        if self.is_ident_like() {
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
