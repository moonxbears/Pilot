//! `syntax` declaration parsing: rule groups, sequence rule heads and `syntax` classes (`pilot-parser`).

use crate::*;

impl<'a> Parser<'a> {
    pub(crate) fn parse_syntax_decl(&mut self, mods: &DeclMods) -> NodeId {
        let sp = self.span();
        self.advance();
        let (is_tc, is_ac) = (self.is_rec(5), self.is_rec(6));
        if is_tc || is_ac {
            self.advance();
        }
        // `syntax Name { head; .. }`: a syntax class, a hole kind made of words and holes.
        if !is_tc && !is_ac && self.is_word() {
            return self.parse_syntax_class(sp);
        }
        self.expect(TokenKind::LBrace);
        let mut rules = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) {
            self.skip_seps();
            if self.at(TokenKind::RBrace) || self.at_end() {
                break;
            }
            let rsp = self.span();
            if is_tc || is_ac {
                let mut pat_toks = Vec::new();
                if is_tc && self.eat(TokenKind::LParen) {
                    let mut d = 1;
                    while !self.at_end() && d > 0 {
                        let k = self.peek();
                        if k == TokenKind::LParen {
                            d += 1;
                        } else if k == TokenKind::RParen {
                            d -= 1;
                            if d == 0 {
                                self.advance();
                                break;
                            }
                        }
                        pat_toks.push((k, self.text().to_string()));
                        self.advance();
                    }
                } else {
                    let (mut pd, mut bd) = (0i32, 0i32);
                    while !self.at_end() && !(pd == 0 && bd == 0 && self.at(TokenKind::FatArrow)) {
                        match self.peek() {
                            TokenKind::LParen => pd += 1,
                            TokenKind::RParen => pd -= 1,
                            TokenKind::LBrace => bd += 1,
                            TokenKind::RBrace => bd -= 1,
                            _ => {}
                        }
                        self.advance();
                    }
                }
                self.expect(TokenKind::FatArrow);
                let body = if self.eat(TokenKind::KwQuote) && self.eat(TokenKind::LBrace) {
                    self.in_quote_type = is_tc;
                    let t = if is_tc {
                        self.parse_type()
                    } else {
                        self.parse_expr()
                    };
                    self.in_quote_type = false;
                    self.expect(TokenKind::RBrace);
                    t
                } else {
                    self.parse_expr()
                };
                let _ = self.eat(TokenKind::Semicolon);
                if is_tc && body.is_some() {
                    if let Some(rule) =
                        compile_type_rule(&pat_toks, TemplateNode::from_arena(&self.arena, body))
                    {
                        let t = self.tables.to_mut();
                        t.type_rules.retain(|e| !same_type_trigger(e, &rule));
                        t.type_rules.push(rule);
                    }
                }
                rules.push(body);
            } else {
                // `[Infix(..)] rule(params) => ..`: an operator's function, and
                // `rule(params) => hook(..)`: a comptime function. Any other rule
                // has a sequence head: `word $a:Kind word => quote { .. }` (PROP-009).
                let rule_attrs = if self.at(TokenKind::LBracket) {
                    self.parse_attrs()
                } else {
                    Vec::new()
                };
                let kw = self.text().to_string();
                self.advance();
                if rule_attrs.is_empty()
                    && (self.rule_body_is_quote() || !self.at(TokenKind::LParen))
                {
                    if self.at(TokenKind::LParen)
                        && self.tape.matching_bracket(self.cur()).is_some_and(|c| {
                            c as usize > self.cur() + 1
                                && !(self.cur()..c as usize)
                                    .any(|i| self.tape.kind(i) == TokenKind::Dollar)
                        })
                    {
                        self.arena.diagnostics.push((rsp, format!("SYN010: the parameters of the rule `{kw}` are holes: `{kw} ( $x:Expr , .. )`; a list `{kw}(x)` declares an operator's function, which takes its attribute")));
                    }
                    let items = self.parse_seq_head(None, false, None);
                    self.expect(TokenKind::FatArrow);
                    let body = self.parse_expr();
                    let _ = self.eat(TokenKind::Semicolon);
                    if body.is_some() && self.arena.node(body).node_kind() == NodeKind::QuoteExpr {
                        self.arena.nodes[body.0 as usize].flags = 1;
                        let tpl = TemplateNode::from_arena(&self.arena, body);
                        let bsp = self.arena.span(body);
                        self.add_seq_rule(&kw, items, tpl, bsp);
                    } else {
                        self.arena.diagnostics.push((rsp, format!("SYN010: the rule `{kw}` with a sequence head expands to a `quote {{ .. }}`")));
                    }
                    continue;
                }
                let params = self.parse_param_list();
                self.expect(TokenKind::FatArrow);
                let body = self.parse_expr();
                let _ = self.eat(TokenKind::Semicolon);
                // An operator's template is no value: flags = 1 tells it from a quote value.
                if body.is_some() && self.arena.node(body).node_kind() == NodeKind::QuoteExpr {
                    self.arena.nodes[body.0 as usize].flags = 1;
                }
                let sym = self.arena.interner.intern(&kw);
                let mut ch = vec![body];
                ch.extend(params);
                let (st, ln) = self.arena.alloc_child_slice(&ch);
                let f = self.arena.alloc_node(
                    NodeKind::FnDecl,
                    ln as u16,
                    ByteSpan::new(rsp.start, self.prev_span().end),
                    sym.0,
                    st,
                );
                let mut fm = mods.clone();
                fm.attrs.extend(rule_attrs);
                fm.flags |= DECL_FLAG_COMPTIME;
                self.apply_mods(f, &fm, 7);
                rules.push(f);
            }
        }
        self.expect(TokenKind::RBrace);
        self.arena.alloc_list_node(
            NodeKind::BlockExpr,
            0,
            ByteSpan::new(sp.start, self.prev_span().end),
            &rules,
        )
    }
    /// `syntax Name { head; .. }`: each arm a sequence head with no template.
    fn parse_syntax_class(&mut self, sp: ByteSpan) -> NodeId {
        let name = self.text().to_string();
        self.advance();
        self.expect(TokenKind::LBrace);
        let mut arms = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) {
            self.skip_seps();
            if self.at(TokenKind::RBrace) || self.at_end() {
                break;
            }
            arms.push(self.parse_seq_head(None, true, None));
            if self.at(TokenKind::FatArrow) {
                self.arena.diagnostics.push((
                    self.span(),
                    format!("SYN010: an arm of the syntax class `{name}` has no template"),
                ));
                while !self.at_end()
                    && !self.at(TokenKind::Semicolon)
                    && !self.at(TokenKind::RBrace)
                {
                    self.advance();
                }
            }
        }
        self.expect(TokenKind::RBrace);
        let span = ByteSpan::new(sp.start, self.prev_span().end);
        self.add_syntax_class(&name, arms, span);
        self.arena
            .alloc_list_node(NodeKind::BlockExpr, 0, span, &[])
    }
    /// Whether the rule at the cursor (after its word) expands to a `quote`.
    fn rule_body_is_quote(&self) -> bool {
        let mut i = self.cur();
        let mut depth = 0i32;
        while i < self.tape.len() {
            match self.tape.kind(i) {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => depth -= 1,
                TokenKind::FatArrow if depth == 0 => {
                    let n = self.next_non_nl(i + 1);
                    return n < self.tape.len() && self.tape.kind(n) == TokenKind::KwQuote;
                }
                TokenKind::Semicolon if depth == 0 => return false,
                _ => {}
            }
            i += 1;
        }
        false
    }
}
