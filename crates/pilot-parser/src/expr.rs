//! Pratt expression climbing, prefix atoms, calls, lambdas, literals, and cascades (`pilot-parser`).

use crate::*;

impl<'a> Parser<'a> {
    pub fn parse_expr(&mut self) -> NodeId {
        self.parse_expr_bp(0)
    }
    pub(crate) fn parse_expr_bp(&mut self, min_bp: u8) -> NodeId {
        let mut lhs = self.parse_prefix();
        if lhs.is_none() {
            return NodeId::NONE;
        }
        loop {
            let raw_p = self.cur();
            if raw_p < self.tape.len() && self.tape.kind(raw_p) == TokenKind::Newline {
                let nx = self.next_non_nl(raw_p);
                if nx < self.tape.len()
                    && matches!(
                        self.tape.kind(nx),
                        TokenKind::InfixDot
                            | TokenKind::PrefixDot
                            | TokenKind::SafeNav
                            | TokenKind::PipeForward
                            | TokenKind::SafeCascade
                            | TokenKind::DotDot
                    )
                {
                    self.pos = nx;
                }
            }
            if self.peek() == TokenKind::Less
                && matches!(
                    self.arena.node(lhs).node_kind(),
                    NodeKind::IdentExpr | NodeKind::MemberExpr
                )
            {
                if let Some(after) = self.try_skip_generic_expr_args() {
                    self.rewrite_generic_receiver(lhs);
                    self.pos = after;
                    let cp = self.cur();
                    if cp < self.tape.len() && self.tape.kind(cp) == TokenKind::PrefixDot {
                        self.tape.kinds[cp] = TokenKind::InfixDot;
                    }
                }
            }
            // Read after any generic arguments: `Name<T>(..)` continues with the call.
            let k = self.peek();
            if !self.no_struct_lit
                && self.at(TokenKind::LBrace)
                && matches!(
                    self.arena.node(lhs).node_kind(),
                    NodeKind::IdentExpr | NodeKind::MemberExpr | NodeKind::TypeNamed
                )
                && self.looks_like_struct_lit()
            {
                let m = self.parse_brace_map();
                lhs = self.arena.alloc_list_node(
                    NodeKind::CallExpr,
                    0,
                    ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end),
                    &[lhs, m],
                );
                continue;
            }
            if matches!(
                k,
                TokenKind::InfixDot | TokenKind::SafeNav | TokenKind::ColonColon
            ) {
                let safe = (k == TokenKind::SafeNav) as u16;
                self.advance();
                if let Some(path) = self.parse_tuple_path(lhs) {
                    lhs = path;
                    continue;
                }
                let sym = if self.eat(TokenKind::Tilde) {
                    let dsym = self.intern_cur();
                    let dnm = format!("~{}", self.arena.interner.resolve(dsym));
                    self.arena.interner.intern(&dnm)
                } else {
                    self.intern_cur()
                };
                lhs = self.arena.alloc_node(
                    NodeKind::MemberExpr,
                    safe,
                    ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end),
                    lhs.0,
                    sym.0,
                );
                continue;
            }
            if k == TokenKind::LParen
                || matches!(
                    k,
                    TokenKind::CallQuoteShallow
                        | TokenKind::CallQuoteDeep
                        | TokenKind::CallQuoteAbi
                        | TokenKind::CallQuoteReact
                        | TokenKind::CallQuoteAdjoint
                        | TokenKind::CallQuoteEffect
                )
            {
                let (pend, cstart) = (
                    self.arena.span(lhs).end as usize,
                    self.span().start as usize,
                );
                if self
                    .src
                    .as_bytes()
                    .get(pend..cstart)
                    .is_some_and(|gap| gap.contains(&b'\n'))
                {
                    break;
                }
                lhs = self.parse_call(lhs);
                continue;
            }
            if k == TokenKind::LBracket {
                let (pend, cstart) = (
                    self.arena.span(lhs).end as usize,
                    self.span().start as usize,
                );
                if self
                    .src
                    .as_bytes()
                    .get(pend..cstart)
                    .is_some_and(|gap| gap.contains(&b'\n'))
                {
                    break;
                }
                self.advance();
                let idx = if self.at(TokenKind::RBracket) {
                    NodeId::NONE
                } else if matches!(self.peek(), TokenKind::DotDot | TokenKind::DotDotEqual) {
                    self.parse_open_range()
                } else {
                    self.parse_expr()
                };
                self.extra_indices_loudly();
                self.expect(TokenKind::RBracket);
                lhs = self.arena.alloc_node(
                    NodeKind::IndexExpr,
                    0,
                    ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end),
                    lhs.0,
                    idx.0,
                );
                continue;
            }
            // Postfix `x?`: its power is the one its grammar entry declares.
            if k == TokenKind::Question
                && self
                    .tables
                    .postfix_ops
                    .get(self.text())
                    .is_some_and(|&bp| min_bp <= bp)
            {
                self.advance();
                lhs = self.arena.alloc_node(
                    NodeKind::TryPropagateExpr,
                    0,
                    ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end),
                    lhs.0,
                    0,
                );
                continue;
            }
            if k == TokenKind::Exclamation && self.tables.receptors.contains_key("!") {
                let bsp = self.span();
                self.advance();
                self.arena.diagnostics.push((
                    bsp,
                    "NULL001: Unchecked postfix `!` is prohibited; use `?`, `??`, or `guard`"
                        .into(),
                ));
                continue;
            }
            if (k == TokenKind::SafeCascade
                || (k == TokenKind::DotDot && self.is_cascade_dotdot(lhs)))
                && min_bp <= 1
            {
                // A step, including the value of `..field = value`, ends at the next step.
                let safe = (k == TokenKind::SafeCascade) as u16;
                self.advance();
                let ssp = self.span();
                let outer = std::mem::replace(&mut self.in_cascade_step, true);
                let mut step = self.parse_expr_bp(2);
                if step.is_some() && self.peek_assign_op().is_some() {
                    step = self.parse_assign_tail(step, ssp, 2);
                }
                self.in_cascade_step = outer;
                lhs = self.arena.alloc_node(
                    NodeKind::CascadeExpr,
                    safe,
                    ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end),
                    lhs.0,
                    step.0,
                );
                continue;
            }
            // `expr catch err => fallback`: the fallback is a `CatchClause` binding `err`.
            if self.is_rec(19) && min_bp <= CLAUSE_BP {
                let csp = self.span();
                self.advance();
                let nx = self.next_non_nl(self.cur() + 1);
                let psym = if self.is_word()
                    && nx < self.tape.len()
                    && self.tape.kind(nx) == TokenKind::FatArrow
                {
                    self.intern_cur()
                } else {
                    SymbolId(0)
                };
                let _ = self.eat(TokenKind::FatArrow);
                let fb = if self.at(TokenKind::LBrace) {
                    self.parse_block()
                } else {
                    self.parse_expr_bp(CLAUSE_BP)
                };
                let clause = self.arena.alloc_node(
                    NodeKind::CatchClause,
                    0,
                    ByteSpan::new(csp.start, self.prev_span().end),
                    psym.0,
                    fb.0,
                );
                lhs = self.arena.alloc_node(
                    NodeKind::CatchRecoveryExpr,
                    0,
                    ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end),
                    lhs.0,
                    clause.0,
                );
                continue;
            }
            // `x in xs`: the word that heads a `for` loop's iterable, between
            // two expressions, calls the bound membership function.
            // Its power is the one the grammar declares for the word.
            let membership_bp = self
                .tables
                .infix_ops
                .get(self.text())
                .filter(|e| e.shape as usize == SHAPE_MEMBERSHIP)
                .map(|e| e.bp);
            if self.receptor() == Some((10, 0)) && membership_bp.is_some_and(|bp| min_bp <= bp) {
                if let Some(function) = self.tables.membership().map(str::to_string) {
                    let wsp = self.span();
                    self.advance();
                    let rhs =
                        self.parse_expr_bp(membership_bp.unwrap_or(LEVEL_BASE).saturating_add(1));
                    let span = ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end);
                    let range_fn = if rhs.is_some() {
                        let rhs_node = *self.arena.node(rhs);
                        if rhs_node.node_kind() == NodeKind::BinaryExpr
                            && matches!(rhs_node.flags, 23 | 24)
                        {
                            self.tables.membership_range(rhs_node.flags == 23).map(|f| {
                                (f.to_string(), NodeId(rhs_node.lhs), NodeId(rhs_node.rhs))
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if let Some((rf, rl, rr)) = range_fn {
                        let callee = self.arena.alloc_ident(&rf, wsp);
                        lhs = self.arena.alloc_list_node(
                            NodeKind::CallExpr,
                            0,
                            span,
                            &[callee, lhs, rl, rr],
                        );
                    } else {
                        let callee = self.arena.alloc_ident(&function, wsp);
                        lhs = self.arena.alloc_list_node(
                            NodeKind::CallExpr,
                            0,
                            span,
                            &[callee, lhs, rhs],
                        );
                    }
                    continue;
                }
            }
            // `x as T`: its binding power is the one Pilot registers for the word.
            if self.receptor() == Some((29, 0)) {
                let Some(bp) = self.tables.infix_ops.get(self.text()).map(|e| e.bp) else {
                    break;
                };
                if bp < min_bp {
                    break;
                }
                self.advance();
                let is_view = self.receptor() == Some((29, 1));
                if is_view {
                    self.advance();
                }
                let (ty, op) = (
                    self.parse_type_bp(self.tables.type_tight_bp()),
                    if is_view {
                        BinaryOp::AsView
                    } else {
                        BinaryOp::As
                    } as u16,
                );
                lhs = self.arena.alloc_node(
                    NodeKind::BinaryExpr,
                    op,
                    ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end),
                    lhs.0,
                    ty.0,
                );
                continue;
            }
            if k == TokenKind::DotDot && self.in_cascade_step && self.cascade_step_ahead() {
                break;
            }
            // Symbols and readable words alike (`&&` and `and`) come from the
            // grammar; `as` and `in` are read by their own shapes above.
            let op_entry = self
                .tables
                .infix_ops
                .get(self.text())
                .copied()
                .filter(|e| !matches!(e.shape as usize, SHAPE_TYPE_RIGHT | SHAPE_MEMBERSHIP));
            let Some(pe) = op_entry else {
                break;
            };
            if pe.bp < min_bp {
                break;
            }
            self.advance();
            let rhs = self.parse_expr_bp(if pe.right_assoc { pe.bp } else { pe.bp + 1 });
            lhs = self.arena.alloc_node(
                NodeKind::BinaryExpr,
                pe.bin_op,
                ByteSpan::new(self.arena.span(lhs).start, self.prev_span().end),
                lhs.0,
                rhs.0,
            );
            if matches!(pe.bin_op, 23 | 24) {
                lhs = self.range_step(lhs, pe.bp + 1);
            }
        }
        lhs
    }
    pub(crate) fn parse_prefix(&mut self) -> NodeId {
        let sp = self.span();
        match self.peek() {
            // `yield e` in an expression: its value is what the next
            // `send(v)` passes in.
            TokenKind::KwYield => {
                self.advance();
                let v = self.parse_expr();
                let span = ByteSpan::new(sp.start, self.prev_span().end);
                self.arena.alloc_node(NodeKind::YieldStmt, 0, span, v.0, 0)
            }
            TokenKind::IntLit
            | TokenKind::FloatLit
            | TokenKind::HexIntLit
            | TokenKind::BinIntLit
            | TokenKind::OctIntLit
                if self
                    .tape
                    .flag(self.cur())
                    .contains(pilot_lexer::TokenFlags::IS_DIMENSIONAL) =>
            {
                self.parse_suffixed_number(sp)
            }
            TokenKind::IntLit => self.parse_int_lit(sp, 10),
            TokenKind::HexIntLit => self.parse_int_lit(sp, 16),
            TokenKind::BinIntLit => self.parse_int_lit(sp, 2),
            TokenKind::OctIntLit => self.parse_int_lit(sp, 8),
            TokenKind::FloatLit => {
                let text = self.text().to_string();
                self.advance();
                let Ok(v) = text.replace('_', "").parse() else {
                    let msg = format!("LIT002: the float literal `{text}` has no value");
                    self.arena.diagnostics.push((sp, msg));
                    return self.arena.alloc_float(0.0, sp);
                };
                self.arena.alloc_float(v, sp)
            }
            TokenKind::StrLit | TokenKind::TripleStrLit => self.parse_str_lit(sp),
            TokenKind::KeywordLit => {
                let t = self.text();
                self.advance();
                self.arena.alloc_keyword(t, sp)
            }
            TokenKind::KwIf => self.parse_if(),
            // `quote { .. }` / `quote expr` as a value: the code, never run.
            TokenKind::KwQuote => {
                self.advance();
                let body = if self.at(TokenKind::LBrace) {
                    self.parse_block()
                } else {
                    self.parse_expr()
                };
                let span = ByteSpan::new(sp.start, self.prev_span().end);
                self.arena
                    .alloc_node(NodeKind::QuoteExpr, 0, span, body.0, 0)
            }
            // `comptime expr` / `comptime { .. }` as a value: flags = 1.
            TokenKind::KwComptime if self.at_comptime_if() => self.parse_comptime_if(),
            TokenKind::KwComptime => {
                self.advance();
                let e = if self.at(TokenKind::LBrace) {
                    self.parse_block()
                } else {
                    self.parse_expr()
                };
                self.arena.alloc_node(
                    NodeKind::ComptimeExpr,
                    1,
                    ByteSpan::new(sp.start, self.prev_span().end),
                    e.0,
                    0,
                )
            }
            // `$name`: a hole of a quote. `$(expr)` is not a form of the
            // language: it is reported, never read as `expr`.
            TokenKind::Dollar => {
                self.advance();
                if self.eat(TokenKind::LParen) {
                    let e = self.parse_expr();
                    self.expect(TokenKind::RParen);
                    self.arena.diagnostics.push((
                        ByteSpan::new(sp.start, self.prev_span().end),
                        "a hole splices a name (`$x`); bind the value with `let` and splice that name".to_string(),
                    ));
                    e
                } else {
                    let sym = self.intern_cur();
                    self.arena.alloc_node(
                        NodeKind::IdentExpr,
                        1,
                        ByteSpan::new(sp.start, self.prev_span().end),
                        sym.0,
                        0,
                    )
                }
            }
            TokenKind::PrefixDot => {
                self.advance();
                let rcv_nm = if self.tables.receiver_ident.is_empty() {
                    "_self".to_string()
                } else {
                    self.tables.receiver_ident.clone()
                };
                let (rcv, fld) = (self.arena.alloc_ident(&rcv_nm, sp), self.intern_cur());
                self.arena.alloc_node(
                    NodeKind::MemberExpr,
                    MEMBER_IMPLIED_RECEIVER,
                    ByteSpan::new(sp.start, self.prev_span().end),
                    rcv.0,
                    fld.0,
                )
            }
            TokenKind::ColonColon => {
                self.advance();
                let sym = self.intern_cur();
                self.arena.alloc_node(
                    NodeKind::IdentExpr,
                    0,
                    ByteSpan::new(sp.start, self.prev_span().end),
                    sym.0,
                    0,
                )
            }
            TokenKind::Minus
            | TokenKind::Plus
            | TokenKind::Exclamation
            | TokenKind::CallQuoteShallow
            | TokenKind::Tilde
            | TokenKind::Amper
            | TokenKind::KwRef
            | TokenKind::ArrowLeft
            | TokenKind::Star
            | TokenKind::DoubleStar
            | TokenKind::StarDotDot
            | TokenKind::GreaterDotDot => {
                let fl = prefix_flag(self.peek());
                // The power its grammar entry declares; `ref` (a seed word)
                // binds like the borrow operator `&`.
                let declared = if self.at(TokenKind::KwRef) {
                    self.tables.prefix_bp_of_kind(TokenKind::Amper)
                } else {
                    self.tables.prefix_ops.get(self.text()).copied()
                };
                let bp = match declared {
                    Some(bp) => bp,
                    None => {
                        let msg = format!("SYN001: `{}` is not a declared prefix operator (stdlib/core/operators)", self.text());
                        self.arena.diagnostics.push((sp, msg));
                        LEVEL_BASE
                    }
                };
                self.advance();
                let v = self.parse_expr_bp(bp);
                self.arena.alloc_node(
                    NodeKind::UnaryExpr,
                    fl,
                    ByteSpan::new(sp.start, self.prev_span().end),
                    v.0,
                    0,
                )
            }
            TokenKind::LParen => self.parse_paren_or_lambda(),
            TokenKind::LBracket => self.parse_bracket_lit(),
            TokenKind::LBrace => {
                if self.looks_like_brace_map() {
                    self.parse_brace_map()
                } else {
                    self.parse_block()
                }
            }
            _ if self.is_word() => {
                let nx = self.next_non_nl(self.cur() + 1);
                if self.tables.modifiers.get(self.text()) == Some(&ModAct::Mut)
                    && nx < self.tape.len()
                    && (is_word_tok(self.tape.kind(nx))
                        || matches!(self.tape.kind(nx), TokenKind::KwRef | TokenKind::Amper))
                {
                    self.advance();
                    return self.parse_prefix();
                }
                // A readable prefix word (`not`): the node of the symbol it
                // pairs with, at that symbol's power.
                if let Some(sym) = self.tables.prefix_words.get(self.text()).cloned() {
                    let bp = self
                        .tables
                        .prefix_ops
                        .get(self.text())
                        .copied()
                        .unwrap_or(LEVEL_BASE);
                    let fl = first_token_kind(&sym).map_or(0, prefix_flag);
                    self.advance();
                    let v = self.parse_expr_bp(bp);
                    return self.arena.alloc_node(
                        NodeKind::UnaryExpr,
                        fl,
                        ByteSpan::new(sp.start, self.prev_span().end),
                        v.0,
                        0,
                    );
                }
                if let Some(n) = self.expand_seq_rule() {
                    return n;
                }
                if let Some((rk, pl)) = self.receptor() {
                    match rk {
                        1 => {
                            self.advance();
                            return self
                                .arena
                                .alloc_node(NodeKind::BoolLit, 0, sp, pl as u32, 0);
                        }
                        2 => {
                            self.advance();
                            return self.arena.alloc_node(NodeKind::NoneLit, 0, sp, 0, 0);
                        }
                        contracts::CONTRACT_RECEPTOR
                            if pl == contracts::OLD_SUB && self.in_ensures =>
                        {
                            return self.parse_old()
                        }
                        // `some x` binds its operand like the prefix `!`.
                        31 if pl == 2 => {
                            self.advance();
                            let bp = self
                                .tables
                                .prefix_bp_of_kind(TokenKind::Exclamation)
                                .unwrap_or(LEVEL_BASE);
                            return self.parse_expr_bp(bp);
                        }
                        37 if pl >= 1 && pl <= 6 => {
                            if nx < self.tape.len()
                                && (is_word_tok(self.tape.kind(nx))
                                    || is_seed_kw(self.tape.kind(nx)))
                            {
                                let mut p_check = nx;
                                while p_check < self.tape.len()
                                    && (is_word_tok(self.tape.kind(p_check))
                                        || is_seed_kw(self.tape.kind(p_check))
                                        || self.tape.kind(p_check) == TokenKind::InfixDot)
                                {
                                    p_check += 1;
                                }
                                if p_check < self.tape.len()
                                    && self.tape.kind(p_check) == TokenKind::LParen
                                    && p_check > nx
                                {
                                    self.advance();
                                    let mut callee = self.parse_prefix();
                                    if callee.is_some() {
                                        while matches!(
                                            self.peek(),
                                            TokenKind::InfixDot | TokenKind::SafeNav
                                        ) {
                                            let safe = (self.peek() == TokenKind::SafeNav) as u16;
                                            self.advance();
                                            let sym = self.intern_cur();
                                            callee = self.arena.alloc_node(
                                                NodeKind::MemberExpr,
                                                safe,
                                                ByteSpan::new(
                                                    self.arena.span(callee).start,
                                                    self.prev_span().end,
                                                ),
                                                callee.0,
                                                sym.0,
                                            );
                                        }
                                        return self.parse_call_args(callee, pl as u16);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                let w = self.text().to_string();
                self.advance();
                self.arena.alloc_ident(&w, sp)
            }
            _ if is_seed_kw(self.peek()) => {
                let w = self.text();
                self.advance();
                self.arena.alloc_ident(w, sp)
            }
            _ => NodeId::NONE,
        }
    }
    pub(crate) fn parse_call(&mut self, callee: NodeId) -> NodeId {
        let qk = match self.peek() {
            TokenKind::CallQuoteShallow => CallQuoteKind::ShallowQuote,
            TokenKind::CallQuoteDeep => CallQuoteKind::DeepSplice,
            TokenKind::CallQuoteAbi => CallQuoteKind::AbiFrame,
            TokenKind::CallQuoteReact => CallQuoteKind::ReactiveSignal,
            TokenKind::CallQuoteAdjoint => CallQuoteKind::SymbolicAdjoint,
            TokenKind::CallQuoteEffect => CallQuoteKind::EffectDryRun,
            _ => CallQuoteKind::None,
        };
        if qk != CallQuoteKind::None {
            self.advance();
        }
        self.parse_call_args(callee, qk as u16)
    }
    pub(crate) fn parse_call_args(&mut self, callee: NodeId, qk: u16) -> NodeId {
        let is_sym_quote = self.arena.node(callee).node_kind() == NodeKind::IdentExpr
            && self.tables.quotes_symbol(
                self.arena
                    .interner
                    .resolve(SymbolId(self.arena.node(callee).lhs)),
            );
        self.expect(TokenKind::LParen);
        let mut ch = vec![callee];
        while !self.at_end() && !self.at(TokenKind::RParen) {
            if is_sym_quote {
                let tsp = self.span();
                let one_word = self.is_word() || is_seed_kw(self.peek());
                let txt = self.text().to_string();
                if one_word {
                    self.advance();
                }
                if !one_word || !(self.at(TokenKind::RParen) || self.at(TokenKind::Comma)) {
                    let quoter = self
                        .arena
                        .interner
                        .resolve(SymbolId(self.arena.node(callee).lhs))
                        .to_string();
                    let msg = format!("`{quoter}` quotes one name, such as `{quoter}(x)`");
                    self.arena.diagnostics.push((tsp, msg));
                    while !self.at_end()
                        && !self.at(TokenKind::RParen)
                        && !self.at(TokenKind::Comma)
                    {
                        self.advance();
                    }
                }
                let sym = self.arena.interner.intern(&txt);
                ch.push(self.arena.alloc_node(NodeKind::StrLit, 0, tsp, sym.0, 0));
            } else {
                let nx = self.next_non_nl(self.cur() + 1);
                // `name: value` keeps its name in a `CallArg` (lhs = name, rhs = value).
                if self.is_word()
                    && nx < self.tape.len()
                    && matches!(self.tape.kind(nx), TokenKind::Equal | TokenKind::Colon)
                {
                    let nsp = self.span();
                    let name = self.intern_cur();
                    self.advance();
                    let a = self.parse_expr();
                    if a.is_some() {
                        ch.push(self.arena.alloc_node(
                            NodeKind::CallArg,
                            0,
                            ByteSpan::new(nsp.start, self.prev_span().end),
                            name.0,
                            a.0,
                        ));
                    }
                } else {
                    let a = self.parse_expr();
                    if a.is_some() {
                        ch.push(a);
                    }
                }
            }
            if self.eat(TokenKind::Comma) {
                if self.at(TokenKind::RParen) {
                    self.arena.diagnostics.push((
                        self.prev_span(),
                        "Trailing comma not permitted in argument list".into(),
                    ));
                    break;
                }
            } else {
                break;
            }
        }
        self.expect(TokenKind::RParen);
        self.arena.alloc_list_node(
            NodeKind::CallExpr,
            qk,
            ByteSpan::new(self.arena.span(callee).start, self.prev_span().end),
            &ch,
        )
    }

    pub(crate) fn parse_paren_or_lambda(&mut self) -> NodeId {
        let sp = self.span();
        if let Some(close) = self.tape.matching_bracket(self.cur()) {
            let nx = self.next_non_nl(close as usize + 1);
            if nx < self.tape.len() && self.tape.kind(nx) == TokenKind::FatArrow {
                let params = self.parse_param_list();
                self.expect(TokenKind::FatArrow);
                let body = if self.at(TokenKind::LBrace) {
                    self.parse_block()
                } else {
                    self.parse_expr()
                };
                let (st, ln) = self.arena.alloc_child_slice(&params);
                return self.arena.alloc_node(
                    NodeKind::LambdaExpr,
                    ln as u16,
                    ByteSpan::new(sp.start, self.prev_span().end),
                    st,
                    body.0,
                );
            }
        }
        self.advance();
        let (mut items, mut had_comma) = (Vec::new(), false);
        while !self.at_end() && !self.at(TokenKind::RParen) {
            let e = self.parse_expr();
            if e.is_some() {
                items.push(e);
            }
            if self.eat(TokenKind::Comma) {
                had_comma = true;
            } else {
                break;
            }
        }
        self.expect(TokenKind::RParen);
        if items.len() == 1 && !had_comma {
            items[0]
        } else {
            self.arena.alloc_list_node(
                NodeKind::TupleExpr,
                0,
                ByteSpan::new(sp.start, self.prev_span().end),
                &items,
            )
        }
    }
    pub(crate) fn parse_bracket_lit(&mut self) -> NodeId {
        let sp = self.span();
        self.advance();
        if self.eat(TokenKind::Colon) {
            self.expect(TokenKind::RBracket);
            return self.arena.alloc_list_node(
                NodeKind::MapExpr,
                0,
                ByteSpan::new(sp.start, self.prev_span().end),
                &[],
            );
        }
        let (mut items, mut is_map) = (Vec::new(), false);
        while !self.at_end() && !self.at(TokenKind::RBracket) {
            if self.at(TokenKind::RBracket) {
                break;
            }
            let mp = self.pos;
            let k = self.parse_expr();
            if self.eat(TokenKind::Colon) {
                is_map = true;
                let v = self.parse_expr();
                if k.is_some() {
                    items.push(k);
                }
                if v.is_some() {
                    items.push(v);
                }
            } else if k.is_some() {
                let kn = self.arena.node(k);
                if kn.node_kind() == NodeKind::UnaryExpr && kn.flags == UNARY_OP_DOUBLE_SPLAT {
                    is_map = true;
                }
                items.push(k);
            }
            if self.eat(TokenKind::Comma) {
            } else if self.pos == mp {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(TokenKind::RBracket);
        self.arena.alloc_list_node(
            if is_map {
                NodeKind::MapExpr
            } else {
                NodeKind::ListExpr
            },
            0,
            ByteSpan::new(sp.start, self.prev_span().end),
            &items,
        )
    }
    pub(crate) fn looks_like_brace_map(&self) -> bool {
        let p1 = self.next_non_nl(self.cur() + 1);
        if p1 >= self.tape.len() {
            return false;
        }
        if matches!(
            self.tape.kind(p1),
            TokenKind::RBrace | TokenKind::DoubleStar
        ) {
            return true;
        }
        let p2 = self.next_non_nl(p1 + 1);
        if p2 < self.tape.len()
            && (is_word_tok(self.tape.kind(p1))
                || matches!(self.tape.kind(p1), TokenKind::StrLit | TokenKind::IntLit))
            && matches!(self.tape.kind(p2), TokenKind::Colon | TokenKind::Equal)
        {
            let w = self.tape.text(p1, self.src);
            !matches!(self.tables.receptors.get(w), Some((15 | 16 | 21 | 22, _)))
                && !self.braces_hold_statement()
        } else {
            false
        }
    }
    pub(crate) fn looks_like_struct_lit(&self) -> bool {
        let p1 = self.next_non_nl(self.cur() + 1);
        if p1 < self.tape.len() && self.tape.kind(p1) == TokenKind::RBrace {
            return true;
        }
        if !self.looks_like_brace_map() {
            return false;
        }
        let Some(close) = self.tape.matching_bracket(self.cur()) else {
            return false;
        };
        let mut d = 0;
        for i in (self.cur() + 1)..(close as usize) {
            match self.tape.kind(i) {
                TokenKind::LBrace | TokenKind::LParen | TokenKind::LBracket => d += 1,
                TokenKind::RBrace | TokenKind::RParen | TokenKind::RBracket => d -= 1,
                TokenKind::Semicolon if d == 0 => return false,
                _ => {}
            }
        }
        true
    }
    pub(crate) fn parse_brace_map(&mut self) -> NodeId {
        let sp = self.span();
        self.advance();
        let mut pairs = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) {
            self.skip_seps();
            if self.at(TokenKind::RBrace) || self.at_end() {
                break;
            }
            let mp = self.pos;
            let nx = self.next_non_nl(self.cur() + 1);
            let k = if self.is_word()
                && nx < self.tape.len()
                && matches!(self.tape.kind(nx), TokenKind::Colon | TokenKind::Equal)
            {
                let (ksp, nm) = (self.span(), self.text().to_string());
                self.advance();
                self.arena.alloc_ident(&nm, ksp)
            } else {
                self.parse_expr()
            };
            if self.eat(TokenKind::Colon) || self.eat(TokenKind::Equal) {
                let v = self.parse_expr();
                if k.is_some() {
                    pairs.push(k);
                }
                if v.is_some() {
                    pairs.push(v);
                }
            } else if k.is_some() {
                pairs.push(k);
            }
            let _ = self.eat(TokenKind::Comma) || self.eat(TokenKind::Semicolon);
            if self.pos == mp {
                self.advance();
            }
        }
        self.expect(TokenKind::RBrace);
        self.arena.alloc_list_node(
            NodeKind::MapExpr,
            0,
            ByteSpan::new(sp.start, self.prev_span().end),
            &pairs,
        )
    }
    /// `from x in src [where cond]* select e`: the list of `e` for each `x`
    /// of `src` that every `where` holds for, built as the collecting
    /// `for x in src { if cond { yield e; } }`. Any other clause is an
    /// error, not skipped.
    pub(crate) fn try_skip_generic_expr_args(&self) -> Option<usize> {
        let (mut p, mut d) = (self.cur(), 0i32);
        while p < self.tape.len() {
            match self.tape.kind(p) {
                TokenKind::Less => d += 1,
                TokenKind::Greater => {
                    d -= 1;
                    if d == 0 {
                        break;
                    }
                }
                TokenKind::ShiftRight => {
                    d -= 2;
                    if d <= 0 {
                        break;
                    }
                }
                TokenKind::Comma
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
                | TokenKind::IntLit => {}
                tk if is_word_tok(tk) || is_seed_kw(tk) => {}
                _ => return None,
            }
            p += 1;
        }
        let after = self.next_non_nl(p + 1);
        (d == 0
            && after < self.tape.len()
            && matches!(
                self.tape.kind(after),
                TokenKind::LBrace | TokenKind::InfixDot | TokenKind::PrefixDot | TokenKind::LParen
            ))
        .then_some(after)
    }
    /// Whether the `..` at the cursor starts a cascade step rather than a range.
    /// A numeric lower bound is always a range. `..name(` and `..name =` are
    /// cascade steps; `..a.b` is one only when the member chain is assigned
    /// (`..a.b = v`), so `0..xs.len()` stays a range. A range bounded by a
    /// plain function call needs parentheses: `i..(f(x))`.
    pub(crate) fn is_cascade_dotdot(&self, lhs: NodeId) -> bool {
        !matches!(
            self.arena.node(lhs).node_kind(),
            NodeKind::IntLit | NodeKind::FloatLit
        ) && self.cascade_step_ahead()
    }
    /// Whether the tokens after the `..` at the cursor have the shape of a cascade step.
    pub(crate) fn cascade_step_ahead(&self) -> bool {
        let mut nx = self.next_non_nl(self.cur() + 1);
        if nx >= self.tape.len() || !is_word_tok(self.tape.kind(nx)) {
            return false;
        }
        let mut after = self.next_non_nl(nx + 1);
        let mut chained = false;
        while after < self.tape.len() && self.tape.kind(after) == TokenKind::InfixDot {
            nx = self.next_non_nl(after + 1);
            if nx >= self.tape.len() || !is_word_tok(self.tape.kind(nx)) {
                return false;
            }
            after = self.next_non_nl(nx + 1);
            chained = true;
        }
        if after >= self.tape.len() {
            return false;
        }
        let k = self.tape.kind(after);
        (k == TokenKind::LParen && !chained) || assign_op_of(k).is_some()
    }
}

/// The `UnaryExpr` flag of the prefix operator that starts with token `k`.
pub(crate) fn prefix_flag(k: TokenKind) -> u16 {
    match k {
        TokenKind::Amper | TokenKind::KwRef | TokenKind::Plus => 1,
        TokenKind::ArrowLeft => 3,
        TokenKind::Tilde => 4,
        TokenKind::Exclamation | TokenKind::CallQuoteShallow => 5,
        TokenKind::Star => UNARY_OP_SPLAT,
        TokenKind::DoubleStar => UNARY_OP_DOUBLE_SPLAT,
        TokenKind::StarDotDot => UNARY_OP_DEEP_FLATTEN,
        _ => 0,
    }
}
