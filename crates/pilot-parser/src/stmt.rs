//! Type-expression, block, statement, control-flow, and pattern parsing (`pilot-parser`).

use crate::*;

impl<'a> Parser<'a> {
    pub fn parse_type(&mut self) -> NodeId { self.parse_type_bp(0) }
    pub(crate) fn parse_type_bp(&mut self, min_bp: u8) -> NodeId { let sp = self.span(); let mut lhs = if self.in_quote_type && self.eat(TokenKind::Dollar) {
            let sym = self.intern_cur();
            self.arena.alloc_node(NodeKind::TypeNamed, 0, sp, sym.0, u32::MAX)
        } else if matches!(self.peek(), TokenKind::Star | TokenKind::At) { let op = self.text().to_string(); self.advance();
            let inner = self.parse_type_bp(TypePrecedence::PostfixOptional as u8);
            if let Some(TypeConstructRule::Prefix { param, template, .. }) = self.tables.type_rules.iter().find(|r| matches!(r, TypeConstructRule::Prefix { op: o, .. } if o == &op)).cloned() {
                let end = self.prev_span().end;
                template.instantiate(&mut self.arena, &[(param, inner)], ByteSpan::new(sp.start, end))
            } else { inner }
        } else if self.eat(TokenKind::LParen) { let mut items = Vec::new(); while !self.at_end() && !self.at(TokenKind::RParen) {
                let nx = self.next_non_nl(self.cur() + 1);
                if self.is_word() && nx < self.tape.len() && self.tape.kind(nx) == TokenKind::Colon { self.advance(); self.advance(); }
                items.push(self.parse_type()); if !self.eat(TokenKind::Comma) { break; } }
            self.expect(TokenKind::RParen);
            if items.len() == 1 && !matches!(self.peek(), TokenKind::FatArrow | TokenKind::Arrow) { items[0] }
            else { self.arena.alloc_list_node(NodeKind::TypeTuple, 0, ByteSpan::new(sp.start, self.prev_span().end), &items) }
        } else if self.is_word() || is_seed_kw(self.peek()) { let nx = self.next_non_nl(self.cur() + 1);
            if self.is_word() && matches!(self.tables.modifiers.get(self.text()), Some(ModAct::Iso | ModAct::Mut)) && nx < self.tape.len() && (is_word_tok(self.tape.kind(nx)) || is_seed_kw(self.tape.kind(nx))) { self.advance(); }
            let mut nm = self.text().to_string(); self.advance();
            while self.eat(TokenKind::InfixDot) || self.eat(TokenKind::ColonColon) { nm.push('.'); nm.push_str(self.text()); self.advance(); }
            let (sym, mut args) = (self.arena.interner.intern(&nm), Vec::new());
            if self.eat(TokenKind::Less) {
                while !self.at_end() && !self.at(TokenKind::Greater) && !self.at(TokenKind::ShiftRight) {
                    args.push(if matches!(self.peek(), TokenKind::IntLit | TokenKind::Minus) {
                        // A number argument stops before `>`, at the power of `|`.
                        let bp = self.tables.bp_of(pilot_ast::BinaryOp::BitOr, LEVEL_BASE);
                        self.parse_expr_bp(bp)
                    } else {
                        self.parse_type()
                    });
                    if self.eat(TokenKind::Comma) { if self.at(TokenKind::Greater) { self.arena.diagnostics.push((self.prev_span(), "Trailing comma not permitted in generic parameters".into())); break; } } else { break; } }
                if !self.eat(TokenKind::Greater) && self.at(TokenKind::ShiftRight) { let p = self.cur(); self.tape.kinds[p] = TokenKind::Greater; } }
            if let Some(TypeConstructRule::NamedGeneric { params, template, .. }) = self.tables.type_rules.iter().find(|r| matches!(r, TypeConstructRule::NamedGeneric { name, params, .. } if name == &nm && params.len() == args.len())).cloned() {
                let (b, end) = (params.into_iter().zip(args).collect::<Vec<_>>(), self.prev_span().end.max(sp.end));
                template.instantiate(&mut self.arena, &b, ByteSpan::new(sp.start, end))
            } else { self.arena.alloc_type_named(sym, &args, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end))) }
        } else { return NodeId::NONE; };
        loop { if TypePrecedence::PostfixOptional as u8 >= min_bp { if self.eat(TokenKind::LBracket) {
                    let (kind, extra) = if self.eat(TokenKind::RBracket) { (TypePostfixKind::BracketEmpty, Vec::new()) }
                    else if self.eat(TokenKind::DotDot) { self.expect(TokenKind::RBracket); (TypePostfixKind::BracketDotDot, Vec::new()) }
                    else if self.eat(TokenKind::Colon) { self.expect(TokenKind::RBracket); (TypePostfixKind::BracketColon, Vec::new()) }
                    else { let mut dims = Vec::new(); while !self.at_end() && !self.at(TokenKind::RBracket) {
                            dims.push(if self.in_quote_type && self.at(TokenKind::Dollar) { self.parse_type() } else { self.parse_expr() });
                            if !self.eat(TokenKind::Comma) { break; } }
                        self.expect(TokenKind::RBracket);
                        (TypePostfixKind::BracketArgs(dims.len()), dims) };
                    if let Some(TypeConstructRule::Postfix { params, template, .. }) = self.tables.type_rules.iter().find(|r| matches!(r, TypeConstructRule::Postfix { kind: k, .. } if match_postfix(k, &kind))).cloned() {
                        let mut b = vec![(params[0].clone(), lhs)];
                        for (i, d) in extra.into_iter().enumerate() { if let Some(pn) = params.get(i + 1) { b.push((pn.clone(), d)); } }
                        let end = self.prev_span().end;
                        lhs = template.instantiate(&mut self.arena, &b, ByteSpan::new(sp.start, end));
                        }
                    continue; }
                if matches!(self.peek(), TokenKind::Question | TokenKind::Caret | TokenKind::At) { let sym = self.text().to_string();
                    if let Some(TypeConstructRule::Postfix { kind: TypePostfixKind::Sym(_, has_arg), params, template }) =
                        self.tables.type_rules.iter().find(|r| matches!(r, TypeConstructRule::Postfix { kind: TypePostfixKind::Sym(s, _), .. } if s == &sym)).cloned()
                    { self.advance(); let mut b = vec![(params[0].clone(), lhs)]; if has_arg {
                            // A number argument may be a range: at the power of `..`.
                            let arg = if self.at(TokenKind::IntLit) {
                                let bp = self.tables.bp_of(pilot_ast::BinaryOp::RangeHalfOpen, LEVEL_BASE);
                                self.parse_expr_bp(bp)
                            } else {
                                self.parse_type_bp(31)
                            };
                            if let Some(p1) = params.get(1) { b.push((p1.clone(), arg)); } }
                        let end = self.prev_span().end;
                        lhs = template.instantiate(&mut self.arena, &b, ByteSpan::new(sp.start, end));
                        continue; } } }
            let op_str = match self.peek() {
                TokenKind::Amper => "&", TokenKind::Vbar => "|", TokenKind::Arrow => "->", TokenKind::Exclamation => "!",
                TokenKind::FatArrow if self.arena.node(lhs).node_kind() == NodeKind::TypeTuple => "=>", _ => break, };
            let Some(TypeConstructRule::Infix { bp, right_assoc, lhs_param, rhs_param, template, .. }) =
                self.tables.type_rules.iter().find(|r| matches!(r, TypeConstructRule::Infix { op, .. } if op == op_str)).cloned()
            else { break; }; if bp < min_bp { break; }
            let op_span = self.span();
            self.advance();
            let rhs = self.parse_type_bp(if right_assoc { bp } else { bp + 1 });
            if rhs == NodeId::NONE {
                let msg = format!("a type is needed after `{op_str}`");
                self.arena.diagnostics.push((op_span, msg));
            }
            let lhs_clean = if self.arena.node(lhs).node_kind() == NodeKind::TypeTuple {
                let ch = self.arena.children_slice(self.arena.node(lhs).lhs, self.arena.node(lhs).rhs);
                if ch.len() == 1 { NodeId(ch[0]) } else { lhs }
            } else { lhs }; let end = self.prev_span().end;
            lhs = template.instantiate(&mut self.arena, &[(lhs_param, lhs_clean), (rhs_param, rhs)], ByteSpan::new(sp.start, end));
            }
        lhs }
    /// Whether the word at the cursor starts a statement: a `syntax` rule's
    /// word, or a polyglot or contract block.
    pub(crate) fn is_stmt_receptor_start(&self) -> bool {
        self.tables.seq_rules.contains_key(self.text()) || self.receptor().is_some_and(|(rk, _)| matches!(rk, 32 | 33))
    }
    pub(crate) fn parse_block(&mut self) -> NodeId { let sp = self.span(); self.expect(TokenKind::LBrace);
        self.hdr_stack.push(self.active_hdr.take()); let mut items = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) {
            self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
            let m = self.pos;
            self.parse_item(&mut items, false);
            if self.pos == m { self.advance(); } }
        self.expect(TokenKind::RBrace); self.active_hdr = self.hdr_stack.pop().unwrap_or(None);
        self.arena.alloc_list_node(NodeKind::BlockExpr, 0, ByteSpan::new(sp.start, self.prev_span().end), &items) }
    /// Whether the `asm` at the cursor opens `asm(..)`.
    fn asm_operation_follows(&self) -> bool {
        let nx = self.next_non_nl(self.cur() + 1);
        nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LParen
    }
    pub(crate) fn parse_stmt(&mut self) -> NodeId { if self.at_unbuilt_decl() { return self.parse_unbuilt(); }
        let sp = self.span(); match self.peek() {
            TokenKind::KwReturn => { self.advance(); let v = if self.at(TokenKind::Semicolon) || self.at(TokenKind::RBrace) || self.at_end() { NodeId::NONE } else { self.parse_expr() }; let _ = self.eat(TokenKind::Semicolon); self.arena.alloc_node(NodeKind::ReturnStmt, 0, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), v.0, 0) }
            TokenKind::KwYield => { self.advance(); let v = self.parse_expr(); let _ = self.eat(TokenKind::Semicolon); self.arena.alloc_node(NodeKind::YieldStmt, 0, ByteSpan::new(sp.start, self.prev_span().end), v.0, 0) }
            TokenKind::KwDefer => { self.advance(); let b = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_stmt() }; self.arena.alloc_node(NodeKind::DeferStmt, 0, ByteSpan::new(sp.start, self.prev_span().end), b.0, 0) }
            TokenKind::KwLoop => { self.advance(); let (tr, b) = (self.arena.alloc_node(NodeKind::BoolLit, 0, sp, 1, 0), self.parse_block()); self.arena.alloc_node(NodeKind::WhileStmt, 0, ByteSpan::new(sp.start, self.prev_span().end), tr.0, b.0) }
            TokenKind::KwIf => self.parse_if(),
            // `asm(op, ..)` is one machine operation, an expression (PROP-009).
            TokenKind::KwAsm if !self.asm_operation_follows() => { let word = self.text().to_string(); self.advance();
                // No per-target assembler exists yet: the block is read past and the build stops.
                if self.at(TokenKind::LBrace) { let _ = self.inline_block_code(); }
                self.report_unbuilt(ByteSpan::new(sp.start, self.prev_span().end), &word);
                NodeId::NONE }
            TokenKind::RawDslBlock => { let raw = self.text(); self.advance(); let inner = raw.trim_start_matches('`').trim_end_matches('`'); let (lang, code) = inner.split_once('\n').unwrap_or(("dsl", inner)); self.arena.alloc_polyglot_block(lang.trim(), code.trim(), 0, sp) }
            _ if self.is_word() => { if let Some(n) = self.expand_seq_rule() { return n; }
                 if let Some((rk, _)) = self.receptor() { match rk {
                        3 => {
                            // An `else` reached as a statement has no `if` before it.
                            self.advance();
                            let msg = "`else` must follow the block of an `if` (or a `guard` condition)";
                            self.arena.diagnostics.push((sp, msg.into()));
                            if self.at(TokenKind::LBrace) { let _ = self.parse_block(); } else { let _ = self.parse_stmt(); }
                            return NodeId::NONE;
                        }
                        32 if matches!(self.tape.kind(self.next_non_nl(self.cur() + 1)), TokenKind::LBrace | TokenKind::LParen) => return self.parse_polyglot_inline(),
                        _ => {} } }
                if self.receptor() == Some(EXCLUDE_RECEPTOR) && self.word_follows() {
                    return self.parse_exclude_decl();
                }
                let w = self.text().to_string(); let nx = self.next_non_nl(self.cur() + 1);
                if nx < self.tape.len() && (is_word_tok(self.tape.kind(nx)) || matches!(self.tape.kind(nx), TokenKind::IntLit | TokenKind::StrLit | TokenKind::KwFn))
                    && !self.tables.infix_ops.contains_key(self.tape.text(nx, self.src))
                    && self.tables.receptors.get(self.tape.text(nx, self.src)).map_or(true, |&(rk, _)| !matches!(rk, 19 | 29 | 31))
                { self.arena.diagnostics.push((sp, format!("Unregistered keyword or unexpected identifier '{}'", w)));
                    while !self.at_end() && !self.at(TokenKind::Semicolon) && !self.at(TokenKind::RBrace) {
                        if self.newline_before_cur() { break; }
                        self.advance(); }
                    let _ = self.eat(TokenKind::Semicolon);
                    return self.arena.alloc_node(NodeKind::ErrorNode, 0, sp, 0, 0);
                    }
                self.parse_expr_or_assign(sp) }
            _ => self.parse_expr_or_assign(sp), } }
    pub(crate) fn parse_expr_or_assign(&mut self, sp: ByteSpan) -> NodeId { let e = self.parse_expr(); if e.is_none() { self.advance(); return NodeId::NONE; }
        if self.peek_assign_op().is_some() { let a = self.parse_assign_tail(e, sp, 0); let _ = self.eat(TokenKind::Semicolon); return a; }
        let _ = self.eat(TokenKind::Semicolon);
        if matches!(self.arena.node(e).node_kind(), NodeKind::IfExpr | NodeKind::ForExpr | NodeKind::MatchExpr | NodeKind::BlockExpr) { e }
        else { self.arena.alloc_node(NodeKind::ExprStmt, 0, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), e.0, 0) } }
    pub(crate) fn peek_assign_op(&self) -> Option<u16> { assign_op_of(self.peek()) }
    /// `target <op>= value` with the cursor on the operator; the value is parsed at `value_bp`.
    pub(crate) fn parse_assign_tail(&mut self, target: NodeId, sp: ByteSpan, value_bp: u8) -> NodeId { let op = self.peek_assign_op().unwrap_or(0); self.advance();
        let rhs = self.parse_expr_bp(value_bp);
        let val = if op == 0 { rhs } else { let lc = clone_ast(&mut self.arena, target); self.arena.alloc_node(NodeKind::BinaryExpr, op, sp, lc.0, rhs.0) };
        self.arena.alloc_node(NodeKind::AssignStmt, 0, ByteSpan::new(sp.start, self.prev_span().end), target.0, val.0) }
    pub(crate) fn parse_if(&mut self) -> NodeId { let sp = self.span(); self.advance();
        self.no_struct_lit = true;
        let cond = self.parse_some_condition().unwrap_or_else(|| self.parse_expr());
        self.no_struct_lit = false;
        let then_b = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_expr() };
        if self.is_rec(3) { self.advance();
            let else_b = if self.at(TokenKind::KwIf) { self.parse_if() } else if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_expr() };
            let (st, _) = self.arena.alloc_child_slice(&[then_b, else_b]);
            self.arena.alloc_node(NodeKind::IfExpr, 2, ByteSpan::new(sp.start, self.prev_span().end), cond.0, st)
        } else { self.arena.alloc_node(NodeKind::IfExpr, 0, ByteSpan::new(sp.start, self.prev_span().end), cond.0, then_b.0) } }
    /// `some x = v` or `some v` as a condition (`if some mail = acct.email`).
    /// `some(..)` is a call, not a condition.
    pub(crate) fn parse_some_condition(&mut self) -> Option<NodeId> {
        let nx = self.next_non_nl(self.cur() + 1);
        if self.receptor() != Some((31, 2)) || nx >= self.tape.len() || self.tape.kind(nx) == TokenKind::LParen { return None; }
        let sp = self.span(); self.advance(); let mut c = self.parse_expr();
        if self.eat(TokenKind::Equal) { let v = self.parse_expr();
            c = self.arena.alloc_node(NodeKind::AssignStmt, 0, ByteSpan::new(sp.start, self.prev_span().end), c.0, v.0);
            }
        Some(self.arena.alloc_node(NodeKind::UnaryExpr, pilot_ast::UNARY_OP_SOME, ByteSpan::new(sp.start, self.prev_span().end), c.0, 0)) }
    /// Binding power for a literal inside a pattern: tighter than the range
    /// operators (from the Pilot operator table), so `case 4 ..= 9:` keeps the
    /// range as a pattern instead of folding it into the literal.
    pub(crate) fn pattern_literal_bp(&self) -> u8 {
        let is_range = |op: u16| matches!(pilot_ast::BinaryOp::from_u16(op), Some(pilot_ast::BinaryOp::RangeInclusive | pilot_ast::BinaryOp::RangeHalfOpen));
        self.tables.infix_ops.values().filter(|e| is_range(e.bin_op)).map(|e| e.bp.saturating_add(1)).max().unwrap_or(36).max(36)
    }
    pub(crate) fn parse_pattern(&mut self) -> NodeId { let mut lhs = self.parse_pat_atom();
        if matches!(self.peek(), TokenKind::DotDot | TokenKind::DotDotEqual | TokenKind::DotDotLess) {
            // PatRange flags: 1 = inclusive `..=`, 0 = half-open `..` / `..<`.
            let inclusive = u16::from(self.at(TokenKind::DotDotEqual));
            let sp = self.arena.span(lhs); self.advance(); let rhs = self.parse_pat_atom();
            lhs = self.arena.alloc_node(NodeKind::PatRange, inclusive, sp, lhs.0, rhs.0); }
        while self.eat(TokenKind::Vbar) { let (sp, rhs) = (self.arena.span(lhs), self.parse_pat_atom()); lhs = self.arena.alloc_node(NodeKind::PatOr, 0, sp, lhs.0, rhs.0); }
        lhs }
    pub(crate) fn parse_pat_atom(&mut self) -> NodeId { let sp = self.span();
        if self.is_word() && self.text() == "_" { self.advance(); return self.arena.alloc_node(NodeKind::PatWildcard, 0, sp, 0, 0); }
        if self.eat(TokenKind::LParen) { let mut ps = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RParen) { ps.push(self.parse_pattern()); if !self.eat(TokenKind::Comma) { break; } }
            self.expect(TokenKind::RParen);
            return self.arena.alloc_list_node(NodeKind::PatTuple, 0, ByteSpan::new(sp.start, self.prev_span().end), &ps);
            }
        // `{ "key": p, field: p, name }`: a record or map of any type.
        if self.eat(TokenKind::LBrace) { let fields = self.parse_record_pattern_fields();
            return self.arena.alloc_list_node(NodeKind::PatRecord, 0, ByteSpan::new(sp.start, self.prev_span().end), &fields);
            }
        if self.eat(TokenKind::LBracket) { let mut ps = Vec::new(); while !self.at_end() && !self.at(TokenKind::RBracket) {
                let is_splat = self.eat(TokenKind::Star) || self.eat(TokenKind::DotDot);
                let p = self.parse_pattern();
                if is_splat && self.arena.node(p).node_kind() == NodeKind::PatBind { self.arena.nodes[p.0 as usize].flags = 2; }
                ps.push(p); if !self.eat(TokenKind::Comma) { break; } }
            self.expect(TokenKind::RBracket);
            return self.arena.alloc_list_node(NodeKind::PatSlice, 0, ByteSpan::new(sp.start, self.prev_span().end), &ps);
            }
        if self.eat(TokenKind::ColonColon) || (self.is_word() && (self.text().starts_with(char::is_uppercase) || self.receptor() == Some((31, 2)))) {
            let mut nm = self.text().to_string(); self.advance();
            while self.eat(TokenKind::InfixDot) || self.eat(TokenKind::ColonColon) { nm.push('.'); nm.push_str(self.text()); self.advance(); }
            let (sym, mut sub) = (self.arena.interner.intern(&nm), Vec::new());
            if self.eat(TokenKind::LParen) {
                while !self.at_end() && !self.at(TokenKind::RParen) { sub.push(self.parse_pattern()); if !self.eat(TokenKind::Comma) { break; } }
                self.expect(TokenKind::RParen); } else if self.eat(TokenKind::LBrace) {
                // `Name { .. }`: flags = 1 and the first child names the type.
                let mut fields = vec![self.arena.alloc_node(NodeKind::IdentExpr, 0, sp, sym.0, 0)];
                fields.extend(self.parse_record_pattern_fields());
                return self.arena.alloc_list_node(NodeKind::PatRecord, 1, ByteSpan::new(sp.start, self.prev_span().end), &fields);
                }
            let (st, ln) = self.arena.alloc_child_slice(&sub);
            return self.arena.alloc_node(NodeKind::PatVariant, ln as u16, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), sym.0, st);
            }
        if matches!(self.peek(), TokenKind::IntLit | TokenKind::HexIntLit | TokenKind::FloatLit | TokenKind::StrLit | TokenKind::Minus) || self.is_rec(1) || self.is_rec(2) {
            let lit = self.parse_expr_bp(self.pattern_literal_bp());
            return self.arena.alloc_node(NodeKind::PatLiteral, 0, sp, lit.0, 0);
            }
        if self.is_word() { let sym = self.intern_cur(); return self.arena.alloc_node(NodeKind::PatBind, 0, sp, sym.0, 0); }
        NodeId::NONE }
    /// The fields of a record pattern after its `{`, through the `}`. A field
    /// with a sub-pattern (`key: p`, `"key": p`) is `CallArg(key, p)`; a bare
    /// `name` binds the field of that name.
    pub(crate) fn parse_record_pattern_fields(&mut self) -> Vec<NodeId> { let mut fields = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) { self.skip_seps(); if self.at(TokenKind::RBrace) { break; }
            let fsp = self.span(); let quoted = self.at(TokenKind::StrLit);
            let key = if quoted { let raw = self.text(); let k = raw[1..raw.len().saturating_sub(1)].to_string(); self.advance(); self.arena.interner.intern(&k) } else { self.intern_cur() };
            if self.eat(TokenKind::Colon) { let p = self.parse_pattern();
                fields.push(self.arena.alloc_node(NodeKind::CallArg, 0, ByteSpan::new(fsp.start, self.prev_span().end), key.0, p.0));
            } else { fields.push(self.arena.alloc_node(NodeKind::PatBind, 0, fsp, key.0, 0)); }
            if !self.eat(TokenKind::Comma) { self.skip_seps(); if !self.at(TokenKind::RBrace) { break; } } }
        self.expect(TokenKind::RBrace); fields }
    pub(crate) fn parse_polyglot_inline(&mut self) -> NodeId { let (sp, lang) = (self.span(), self.text().to_string()); self.advance();
        if self.at(TokenKind::LBrace) {
            let code = self.inline_block_code();
            self.arena.alloc_polyglot_block(&lang, code.trim(), 0, ByteSpan::new(sp.start, self.prev_span().end))
        } else if self.eat(TokenKind::LParen) { let e = self.parse_expr(); self.expect(TokenKind::RParen);
            let txt = if !e.is_none() && self.arena.node(e).node_kind() == NodeKind::StrLit { self.arena.interner.resolve(SymbolId(self.arena.node(e).lhs)).to_string() } else { String::new() };
            self.arena.alloc_polyglot_block(&lang, &txt, 1, ByteSpan::new(sp.start, self.prev_span().end))
        } else { NodeId::NONE } }
}

/// The `BinaryOp` an assignment token stores: 0 for `=`, the combined
/// operator for `+=` and the other compound forms.
pub(crate) fn assign_op_of(k: TokenKind) -> Option<u16> { match k { TokenKind::Equal => Some(0u16), TokenKind::PlusEqual => Some(1), TokenKind::MinusEqual => Some(2),
        TokenKind::StarEqual => Some(3), TokenKind::SlashEqual => Some(4), TokenKind::PercentEqual => Some(6),
        TokenKind::AmperEqual => Some(16), TokenKind::VbarEqual => Some(17), TokenKind::CaretEqual => Some(18),
        TokenKind::ShiftLeftEqual => Some(19), TokenKind::ShiftRightEqual => Some(20), TokenKind::CoalesceEqual => Some(22),
        _ => None, } }
