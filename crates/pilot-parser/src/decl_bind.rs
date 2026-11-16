//! Binding declarations and their `= value` initializers (`pilot-parser`).

use crate::*;

impl<'a> Parser<'a> {
    pub(crate) fn parse_bindings(
        &mut self,
        out: &mut Vec<NodeId>,
        mods: &DeclMods,
        def_vis: u8,
        kw_consumed: bool,
    ) {
        let sp = self.span();
        if !kw_consumed {
            self.advance();
        }
        let kind = if mods.is_const {
            NodeKind::ConstDecl
        } else if (mods.flags & DECL_FLAG_MUT) != 0 {
            NodeKind::VarStmt
        } else {
            NodeKind::LetStmt
        };
        let nx = self.next_non_nl(self.cur() + 1);
        if self.is_word()
            && nx < self.tape.len()
            && !matches!(
                self.tape.kind(nx),
                TokenKind::Colon | TokenKind::Equal | TokenKind::Semicolon | TokenKind::Comma
            )
        {
            let saved = self.pos;
            let dist_ty = self.parse_type();
            if dist_ty.is_some() && self.eat(TokenKind::LBrace) {
                while !self.at_end() && !self.at(TokenKind::RBrace) {
                    self.skip_seps();
                    if self.at(TokenKind::RBrace) || self.at_end() {
                        break;
                    }
                    if !self.is_word() {
                        self.advance();
                        continue;
                    }
                    let (bsp, sym) = (self.span(), self.intern_cur());
                    let init = if self.eat(TokenKind::Equal) {
                        self.parse_expr()
                    } else {
                        NodeId::NONE
                    };
                    let _ = self.eat(TokenKind::Comma) || self.eat(TokenKind::Semicolon);
                    let id = self.arena.alloc_node(
                        kind,
                        0,
                        ByteSpan::new(bsp.start, self.prev_span().end.max(bsp.end)),
                        sym.0,
                        init.0,
                    );
                    let cty = clone_ast(&mut self.arena, dist_ty);
                    self.arena.set_type_annot(id, cty);
                    self.apply_mods(id, mods, def_vis);
                    out.push(id);
                }
                self.expect(TokenKind::RBrace);
                let _ = self.eat(TokenKind::Semicolon);
                return;
            }
            self.pos = saved;
            if self.is_word() && nx < self.tape.len() && is_word_tok(self.tape.kind(nx)) {
                let (esp, bad) = (self.span(), self.text().to_string());
                self.arena.diagnostics.push((
                    esp,
                    format!("Prohibited or unregistered binding modifier '{}'", bad),
                ));
                self.advance();
            }
        }
        if matches!(
            self.peek(),
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace
        ) {
            let pat = self.parse_pattern();
            let ty = if self.eat(TokenKind::Colon) {
                self.parse_type()
            } else {
                NodeId::NONE
            };
            let init = self.parse_binding_init();
            let _ = self.eat(TokenKind::Semicolon);
            let id = self.arena.alloc_node(
                kind,
                1,
                ByteSpan::new(sp.start, self.prev_span().end),
                pat.0,
                init.0,
            );
            if ty.is_some() {
                self.arena.set_type_annot(id, ty);
            }
            self.apply_mods(id, mods, def_vis);
            out.push(id);
            return;
        }
        // `let $name = ..` inside `quote { .. }` splices the name.
        if !self.is_word() && !self.at(TokenKind::Dollar) {
            let found = self.text().to_string();
            self.arena.diagnostics.push((
                self.span(),
                format!("expected a name to bind, found `{found}`"),
            ));
        }
        // `let SignedMathFacet<TSelf: ..> = { .. }` (stdlib/core) names a facet's
        // type parameters, which nothing reads yet: not reported, so core parses.
        let sym = self.intern_binder();
        let _ = self.parse_generic_params();
        let ty = if self.eat(TokenKind::Colon) {
            self.parse_type()
        } else {
            NodeId::NONE
        };
        let init = self.parse_binding_init();
        let _ = self.eat(TokenKind::Semicolon);
        let id = self.arena.alloc_node(
            kind,
            0,
            ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)),
            sym.0,
            init.0,
        );
        if ty.is_some() {
            self.arena.set_type_annot(id, ty);
        }
        if kind == NodeKind::ConstDecl {
            self.note_const(sym, init);
        } else {
            self.note_runtime(sym);
        }
        self.apply_mods(id, mods, def_vis);
        out.push(id);
    }
    /// The `= value` of a binding, or `NONE` without one. An `=` followed
    /// by no value is an error.
    pub(crate) fn parse_binding_init(&mut self) -> NodeId {
        if !self.eat(TokenKind::Equal) {
            return NodeId::NONE;
        }
        if matches!(
            self.peek(),
            TokenKind::Semicolon | TokenKind::RBrace | TokenKind::Comma
        ) {
            let sp = self.prev_span();
            self.arena
                .diagnostics
                .push((sp, "`=` in a binding needs a value".to_string()));
            return NodeId::NONE;
        }
        self.parse_expr()
    }
}
