//! Top-level, type-member, modifier, attribute, and `syntax` declaration parsing (`pilot-parser`).

use crate::*;

impl<'a> Parser<'a> {
    pub(crate) fn parse_program(&mut self) {
        // Whether the items feed the tables; the whole text is searched once, not once per item.
        let ingests = self.ingest || self.src.contains("__engine") || self.src.contains("comptime") || self.src.contains('[');
        let mut items = Vec::new();
        while !self.at_end() { self.skip_seps(); if self.at_end() { break; }
            let (mark, first_new) = (self.pos, items.len());
            self.parse_item(&mut items, false);
            if self.pos == mark { self.advance(); }
            // Every item the declaration made: a header block (`pub interface
            // { .. }`) makes one per inner declaration.
            if ingests {
                for &item in &items[first_new..] {
                    self.tables.to_mut().ingest_node(&self.arena, item);
                }
            } }
        if ingests { self.tables.to_mut().finish_ingest(self.src); }
        self.report_stray_characters();
        self.arena.root = self.arena.alloc_list_node(NodeKind::Program, 0, ByteSpan::new(0, self.src.len() as u32), &items);
        }
    pub(crate) fn is_attr_prefix(&self, _in_type_body: bool) -> bool { if self.peek() != TokenKind::LBracket { return false; }
        let mut p = self.cur();
        while p < self.tape.len() && self.tape.kind(p) == TokenKind::LBracket {
            let Some(close) = self.tape.matching_bracket(p) else { return false; };
            p = self.next_non_nl(close as usize + 1);
            }
        if p >= self.tape.len() { return false; }
        let tk = self.tape.kind(p);
        is_seed_kw(tk) || is_word_tok(tk) || matches!(tk, TokenKind::LBrace | TokenKind::Tilde) }
    pub(crate) fn parse_attrs(&mut self) -> Vec<PendingAttr> { let mut out = Vec::new(); while self.at(TokenKind::LBracket) { self.advance();
            while !self.at_end() && !self.at(TokenKind::RBracket) { let (mut is_mut, mut bind_sym) = (false, SymbolId(0));
                if self.at(TokenKind::KwLet) || (self.is_word() && self.tables.modifiers.get(self.text()) == Some(&ModAct::Mut)) {
                    is_mut = !self.at(TokenKind::KwLet);
                    self.advance();
                    if self.is_word() { bind_sym = self.intern_cur(); }
                    let _ = self.eat(TokenKind::Equal); }
                let (mark, mut nm) = (self.pos, String::new());
                while self.is_word() || is_seed_kw(self.peek()) {
                    nm.push_str(self.text()); self.advance();
                    if self.eat(TokenKind::InfixDot) || self.eat(TokenKind::ColonColon) { nm.push('.'); } else { break; } }
                let (name, mut args) = (self.arena.interner.intern(&nm), Vec::new());
                if self.eat(TokenKind::LParen) {
                    while !self.at_end() && !self.at(TokenKind::RParen) { let nx = self.next_non_nl(self.cur() + 1);
                        if self.is_word() && nx < self.tape.len() && matches!(self.tape.kind(nx), TokenKind::Equal | TokenKind::Colon) { self.advance(); self.advance(); }
                        let a = self.parse_expr(); if a.is_some() { args.push(a); }
                        if self.eat(TokenKind::Comma) {
                                                        if self.at(TokenKind::RParen) { self.arena.diagnostics.push((self.prev_span(), "Trailing comma not permitted in argument list".into())); break; }
                        } else { break; } }
                    self.expect(TokenKind::RParen); }
                if !nm.is_empty() { out.push(PendingAttr { name, args, is_mut, bind_sym }); }
                if self.eat(TokenKind::Comma) {
                                        if self.at(TokenKind::RBracket) { self.arena.diagnostics.push((self.prev_span(), "Trailing comma not permitted in attribute brackets".into())); break; }
                } else if self.pos == mark { self.advance(); } }
            self.expect(TokenKind::RBracket); }
        out }
    pub(crate) fn parse_mods(&mut self, in_type_body: bool) -> DeclMods { let mut m = DeclMods::default();
        if self.is_attr_prefix(in_type_body) { m.attrs = self.parse_attrs(); }
        loop { let nx = self.next_non_nl(self.cur() + 1); if nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LBrace
                && self.next_non_nl(nx + 1) < self.tape.len() && self.tape.kind(self.next_non_nl(nx + 1)) == TokenKind::RBrace
            { break; }
            // `comptime { .. }` is a comptime block, not a modifier on a block.
            if self.at(TokenKind::KwComptime) && !(nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LBrace) { self.advance(); m.flags |= DECL_FLAG_COMPTIME; continue; }
            if !self.is_word() { break; }
            let w = self.text();
            if self.tables.param_modifiers.contains(w) && nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LParen {
                let attr = self.parse_param_modifier(); m.attrs.push(attr); continue; }
            if self.tables.module_kws.get(w) == Some(&true) && nx < self.tape.len() && (is_word_tok(self.tape.kind(nx)) || self.tape.kind(nx) == TokenKind::LBrace) && !self.tables.modifiers.contains_key(self.tape.text(nx, self.src)) {
                break; }
            let Some(&act) = self.tables.modifiers.get(w) else { break; };
            // An unbuilt modifier followed by what follows a name (`:`, `=`, `(`,
            // ..) is that name (`pub override: bool;`), not the modifier.
            if matches!(act, ModAct::Unbuilt) && (nx >= self.tape.len() || matches!(self.tape.kind(nx), TokenKind::Colon | TokenKind::Question
                | TokenKind::Equal | TokenKind::Semicolon | TokenKind::Comma | TokenKind::InfixDot | TokenKind::LParen)) { break; }
            // After `var`/`const`, a modifier word followed by `=`, `:`, `;` or `,`
            // is the binding's name (`var pkg = 1;`), not another modifier.
            let binding_started = (m.flags & DECL_FLAG_MUT) != 0 || m.is_const;
            if binding_started && nx < self.tape.len()
                && matches!(self.tape.kind(nx), TokenKind::Equal | TokenKind::Colon | TokenKind::Semicolon | TokenKind::Comma)
            { break; }
            // After a visibility word, a second one followed by what follows a
            // name is that name (`pub file: str;`).
            let names_member = nx >= self.tape.len() || matches!(self.tape.kind(nx), TokenKind::Colon | TokenKind::Question
                | TokenKind::Equal | TokenKind::Semicolon | TokenKind::Comma | TokenKind::InfixDot | TokenKind::LParen);
            if matches!(act, ModAct::Vis(_)) && m.vis.is_some() && names_member { break; }
            // A visibility word followed by neither a declaration (a word, a
            // keyword, `[` attributes) nor a header (`:` or `{`) is a name used
            // in an expression (`open + text`).
            let starts_decl = nx < self.tape.len() && (is_word_tok(self.tape.kind(nx)) || is_seed_kw(self.tape.kind(nx))
                || matches!(self.tape.kind(nx), TokenKind::Colon | TokenKind::LBrace | TokenKind::LBracket));
            if matches!(act, ModAct::Vis(_)) && !starts_decl { break; }
            let (mod_span, mod_word) = (self.span(), w.to_string());
            self.advance(); match act {
                ModAct::Unbuilt => self.report_unbuilt(mod_span, &mod_word),
                ModAct::Vis(v) => {
                    if m.vis.is_some() {
                        let msg = format!("a declaration takes one visibility word; `{mod_word}` is a second one");
                        self.arena.diagnostics.push((mod_span, msg));
                    } else if self.fn_body_depth > 0 && !in_type_body {
                        let msg = format!("`{mod_word}` sets who may use a declaration of a package or type; a local in a block has no such reach");
                        self.arena.diagnostics.push((mod_span, msg));
                    }
                    m.vis = Some(v);
                }
                ModAct::Partial => m.flags |= DECL_FLAG_PARTIAL,
                ModAct::Static => m.flags |= DECL_FLAG_STATIC, ModAct::Pure => m.flags |= DECL_FLAG_PURE,
                ModAct::Mut => m.flags |= DECL_FLAG_MUT, ModAct::Const => m.is_const = true,
                ModAct::Ext => m.is_ext = true, ModAct::NoAlloc => m.flags |= DECL_FLAG_NOALLOC,
                ModAct::Override => m.flags |= DECL_FLAG_OVERRIDE,
                ModAct::Iso => {} } }
        m }
    pub(crate) fn parse_item(&mut self, out: &mut Vec<NodeId>, in_type_body: bool) { let start_pos = self.pos;
        if self.at_comptime_if() {
            out.push(self.parse_comptime_if());
            return;
        }
        let raw_mods = self.parse_mods(in_type_body);
        if raw_mods.is_non_empty() { let (mut target, saved) = (0u8, self.pos);
            if self.eat(TokenKind::KwFn) { target = 1; }
            else if self.eat(TokenKind::KwStruct) { target = 2; }
            else if self.eat(TokenKind::KwInterface) { target = 3; }
            else if self.is_rec(27) { self.advance(); target = 4; }
                        let mut hdr_mods = raw_mods.clone(); if self.eat(TokenKind::Arrow) {
                let rt = self.parse_type();
                hdr_mods.ret_ty = Some(TemplateNode::from_arena(&self.arena, rt));
                }
            if self.eat(TokenKind::Colon) { let merged = hdr_mods.merge_under(self.hdr_stack.last().and_then(|x| x.as_ref()));
                self.active_hdr = Some(ScopeHeader { mods: merged, target }); return; }
            if self.at(TokenKind::LBrace) {
                let (merged, blk_sp) = (hdr_mods.merge_under(self.active_hdr.as_ref()), self.span());
                self.advance();
                self.hdr_stack.push(self.active_hdr.take());
                self.active_hdr = Some(ScopeHeader { mods: merged, target });
                let mut inner = Vec::new();
                while !self.at_end() && !self.at(TokenKind::RBrace) {
                    self.skip_seps(); if self.at_end() || self.at(TokenKind::RBrace) { break; }
                    let mp = self.pos;
                    self.parse_headless_or_item(&mut inner, target, in_type_body);
                    if self.pos == mp { self.advance(); } }
                self.expect(TokenKind::RBrace);
                self.active_hdr = self.hdr_stack.pop().unwrap_or(None);
                out.push(self.arena.alloc_list_node(NodeKind::BlockExpr, 0, ByteSpan::new(blk_sp.start, self.prev_span().end), &inner));
                return;
                }
            self.pos = saved;
            if (raw_mods.flags & DECL_FLAG_COMPTIME) != 0 && self.at(TokenKind::LBrace) && raw_mods.vis.is_none() && raw_mods.attrs.is_empty() {
                let b = self.parse_block();
                out.push(self.arena.alloc_node(NodeKind::ComptimeExpr, 0, self.arena.span(b), b.0, 0));
                return;
                } }
        let hdr_snap = self.active_hdr.clone();
        let (mods, def_vis) = (raw_mods.merge_under(hdr_snap.as_ref()), if in_type_body { 0 } else { 15 });
        if !raw_mods.is_non_empty() && hdr_snap.as_ref().is_some_and(|h| h.target != 0) && self.is_word() && !self.is_stmt_receptor_start() {
            self.parse_headless_or_item(out, hdr_snap.unwrap().target, in_type_body); return; }
        match self.peek() { TokenKind::KwStruct => out.push(self.parse_type_decl(NodeKind::StructDecl, &mods, def_vis, false)),
            TokenKind::KwInterface => out.push(self.parse_type_decl(NodeKind::InterfaceDecl, &mods, def_vis, false)),
            TokenKind::KwFn if mods.is_ext && self.at_ext_fn() => out.push(self.parse_ext_fn(&mods)),
            TokenKind::KwFn => out.push(self.parse_fn(&mods, def_vis, false)),
            TokenKind::KwLet => self.parse_bindings(out, &mods, def_vis, false), TokenKind::KwComptime => { self.advance();
                let b = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_stmt() };
                out.push(self.arena.alloc_node(NodeKind::ComptimeExpr, 0, self.arena.span(b), b.0, 0));
                }
            TokenKind::Tilde if in_type_body => out.push(self.parse_dtor(&mods)),
            TokenKind::Star if in_type_body => out.push(self.parse_bundle_splice()),
            // `ref name: T;` is a field that borrows what it holds (flags 1, as a `ref` parameter).
            TokenKind::KwRef if in_type_body => { self.advance(); let f = self.parse_field_or_prop(&mods, def_vis); self.arena.nodes[f.0 as usize].flags = 1; out.push(f); }
            _ if self.is_word() => {
                let nx = self.next_non_nl(self.cur() + 1);
                let decl_follows = nx < self.tape.len() && (is_word_tok(self.tape.kind(nx)) || self.tape.kind(nx) == TokenKind::LBrace);
                if let Some(&is_pkg) = self.tables.module_kws.get(self.text()).filter(|_| decl_follows) {
                    self.parse_module_decl(is_pkg, &mods, out); return; }
                if mods.is_ext && !self.at(TokenKind::KwFn) { out.push(self.parse_extend_decl(&mods)); return; }
                if raw_mods.flags & DECL_FLAG_MUT != 0 || raw_mods.is_const { self.parse_bindings(out, &mods, def_vis, true); return; }
                // `syntax { .. }` / `syntax <kind> { .. }`; `syntax: T` is a field named after the keyword.
                if self.is_rec(4) && self.tape.kind(self.next_non_nl(self.cur() + 1)) != TokenKind::Colon { out.push(self.parse_syntax_decl(&mods)); return; }
                if self.is_rec(7) { out.push(self.parse_extern_decl(&mods)); return; }
                if self.is_rec(26) { out.push(self.parse_type_adt(&mods, def_vis)); return; }
                if self.is_rec(27) { out.push(self.parse_enum_decl(&mods, def_vis, false)); return; }
                if self.is_rec(25) && self.op_decl_follows(in_type_body) { out.push(self.parse_op_decl(&mods, def_vis)); return; }
                if self.is_rec(20) && in_type_body { out.push(self.parse_scope_decl()); return; }
                if self.is_receiver() && in_type_body { let nx = self.next_non_nl(self.cur() + 1);
                    if nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LParen { out.push(self.parse_ctor(&mods)); return; }
                    if nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LBracket { out.push(self.parse_indexer(&mods, def_vis)); return; } }
                if in_type_body || raw_mods.is_non_empty() { let nx = self.next_non_nl(self.cur() + 1);
                    if nx < self.tape.len() && matches!(self.tape.kind(nx), TokenKind::Colon | TokenKind::Question) { out.push(self.parse_field_or_prop(&mods, def_vis)); return; }
                    if let Some(h) = &hdr_snap { if h.target != 0 { self.parse_headless_or_item(out, h.target, in_type_body); return; } } }
                self.pos = start_pos;
                let st = self.parse_stmt();
                if st.is_some() { out.push(st); } }
            _ => { self.pos = start_pos; let st = self.parse_stmt(); if st.is_some() { out.push(st); } } } }
    pub(crate) fn parse_headless_or_item(&mut self, out: &mut Vec<NodeId>, target: u8, in_type_body: bool) {
        let local = self.parse_mods(true);
        let mods = local.merge_under(self.active_hdr.as_ref());
        let nx = self.next_non_nl(self.cur() + 1);
        let next_is_brace = nx < self.tape.len() && self.tape.kind(nx) == TokenKind::LBrace;
        if !next_is_brace && matches!(self.peek(), TokenKind::KwFn | TokenKind::KwStruct | TokenKind::KwInterface | TokenKind::KwLet) {
            match self.peek() { TokenKind::KwFn => out.push(self.parse_fn(&mods, 15, false)),
                TokenKind::KwStruct => out.push(self.parse_type_decl(NodeKind::StructDecl, &mods, 15, false)),
                TokenKind::KwInterface => out.push(self.parse_type_decl(NodeKind::InterfaceDecl, &mods, 15, false)),
                _ => self.parse_bindings(out, &mods, 15, false), }
            return; }
        match target { 1 => out.push(self.parse_fn(&mods, if in_type_body { 0 } else { 15 }, true)),
            2 => out.push(self.parse_type_decl(NodeKind::StructDecl, &mods, 15, true)),
            3 => out.push(self.parse_type_decl(NodeKind::InterfaceDecl, &mods, 15, true)),
            4 => out.push(self.parse_enum_decl(&mods, 15, true)), _ => { let saved = self.active_hdr.clone();
                self.active_hdr = Some(ScopeHeader { mods, target: 0 });
                self.parse_item(out, in_type_body);
                self.active_hdr = saved;
                }
        } }
    pub(crate) fn parse_type_decl(&mut self, kind: NodeKind, mods: &DeclMods, def_vis: u8, headless: bool) -> NodeId {
        let sp = self.span(); if !headless { self.advance(); }
        let (name_sym, tparams, mut bases) = (self.intern_cur(), self.parse_generic_params(), Vec::new());
        if self.eat(TokenKind::Colon) { loop { let b = self.parse_type(); if b.is_some() { bases.push(b); }
                if self.eat(TokenKind::Comma) { if self.at(TokenKind::LBrace) || self.at(TokenKind::Semicolon) {
                        self.arena.diagnostics.push((self.prev_span(), "Trailing comma not permitted in inheritance list".into()));
                        break;
                        }
                } else { break; } } }
        self.where_loudly("a `where` clause on a type declaration is not checked yet");
        let mut has_dtor = false;
        let body = if self.at(TokenKind::LBrace) { let bsp = self.span(); self.advance();
            self.hdr_stack.push(self.active_hdr.take());
            let mut members = Vec::new();
            while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.skip_seps(); if self.at_end() || self.at(TokenKind::RBrace) { break; }
                let m = self.pos; self.parse_item(&mut members, true);
                if members.last().is_some_and(|&id| self.arena.node(id).node_kind() == NodeKind::DestructorDecl) { has_dtor = true; }
                if self.pos == m { self.advance(); } }
            self.expect(TokenKind::RBrace);
            self.active_hdr = self.hdr_stack.pop().unwrap_or(None);
            self.arena.alloc_list_node(NodeKind::BlockExpr, 0, ByteSpan::new(bsp.start, self.prev_span().end), &members)
        } else { let _ = self.eat(TokenKind::Semicolon); NodeId::NONE };
        let id = self.arena.alloc_node(kind, 0, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), name_sym.0, body.0);
        let mut fm = mods.clone(); if has_dtor { fm.flags |= DECL_FLAG_DTOR; }
        self.apply_mods(id, &fm, def_vis); self.add_type_params(id, tparams);
        for b in bases { self.arena.base_types.push((id, b)); }
        id }
    /// At `fn Owner.name` after the extension modifier.
    pub(crate) fn at_ext_fn(&self) -> bool {
        let owner = self.next_non_nl(self.cur() + 1);
        let dot = owner + 1;
        let name = dot + 1;
        name < self.tape.len()
            && is_word_tok(self.tape.kind(owner))
            && self.tape.kind(dot) == TokenKind::InfixDot
            && is_word_tok(self.tape.kind(name))
    }
    /// `ext fn Owner.name(..) ..`: one extension method, the same
    /// declaration as the block `ext Owner { fn name(..) .. }`. The other
    /// modifiers written before it belong to the method.
    pub(crate) fn parse_ext_fn(&mut self, mods: &DeclMods) -> NodeId {
        let sp = self.span();
        self.advance();
        let owner_sym = self.intern_cur();
        self.expect(TokenKind::InfixDot);
        let mut fn_mods = mods.clone();
        fn_mods.is_ext = false;
        let member = self.parse_fn(&fn_mods, 0, true);
        let span = ByteSpan::new(sp.start, self.prev_span().end);
        let body = self.arena.alloc_list_node(NodeKind::BlockExpr, 0, span, &[member]);
        let id = self.arena.alloc_node(NodeKind::ExtendDecl, 0, span, owner_sym.0, body.0);
        let ext_mods = DeclMods { is_ext: true, ..Default::default() };
        self.apply_mods(id, &ext_mods, 7);
        id
    }
    pub(crate) fn parse_extend_decl(&mut self, mods: &DeclMods) -> NodeId { let (sp, name_sym) = (self.span(), self.intern_cur());
        let _ = self.parse_generic_params();
        if self.eat(TokenKind::Colon) { while !self.at_end() && !self.at(TokenKind::LBrace) { let _ = self.parse_type(); if !self.eat(TokenKind::Comma) { break; } } }
        let body = if self.at(TokenKind::LBrace) { let bsp = self.span(); self.advance(); self.hdr_stack.push(self.active_hdr.take());
            let mut members = Vec::new(); while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
                let m = self.pos;
                self.parse_item(&mut members, true);
                if self.pos == m { self.advance(); } }
            self.expect(TokenKind::RBrace);
            self.active_hdr = self.hdr_stack.pop().unwrap_or(None);
            self.arena.alloc_list_node(NodeKind::BlockExpr, 0, ByteSpan::new(bsp.start, self.prev_span().end), &members)
        } else { NodeId::NONE };
        let id = self.arena.alloc_node(NodeKind::ExtendDecl, 0, ByteSpan::new(sp.start, self.prev_span().end), name_sym.0, body.0);
        self.apply_mods(id, mods, 7); id }
    pub(crate) fn parse_param_list(&mut self) -> Vec<NodeId> { let mut params = Vec::new(); if !self.eat(TokenKind::LParen) { return params; }
        let mut saw_default = false;
        while !self.at_end() && !self.at(TokenKind::RParen) {
            if matches!(self.peek(), TokenKind::Ellipsis | TokenKind::DotDot | TokenKind::PrefixDot | TokenKind::InfixDot) {
                while matches!(self.peek(), TokenKind::Ellipsis | TokenKind::DotDot | TokenKind::PrefixDot | TokenKind::InfixDot) { self.advance(); }
                if self.at(TokenKind::RParen) || self.eat(TokenKind::Comma) { continue; }
            }
            let (psp, mut pflags) = (self.span(), 0u16);
            loop {
                let bit = if self.eat(TokenKind::KwRef) { PARAM_FLAG_REF }
                    else if self.eat(TokenKind::KwComptime) { PARAM_FLAG_COMPTIME }
                    else if self.is_word() && self.tables.modifiers.get(self.text()) == Some(&ModAct::Mut) { self.advance(); PARAM_FLAG_MUT }
                    else if self.is_word() && self.tables.modifiers.get(self.text()) == Some(&ModAct::Iso) { self.advance(); PARAM_FLAG_ISO }
                    else { break };
                pflags |= PARAM_FLAG_LENT | bit;
            }
            self.rest_param_loudly();
            let psym = self.intern_cur();
            self.note_runtime(psym);
            let ty = if self.eat(TokenKind::Colon) { self.parse_type() } else { NodeId::NONE };
            let default = self.param_default();
            let pid = self.arena.alloc_node(NodeKind::ParamDecl, pflags, ByteSpan::new(psp.start, self.prev_span().end.max(psp.end)), psym.0, ty.0);
            if ty.is_some() { self.arena.set_type_annot(pid, ty); }
            if default.is_some() {
                self.arena.field_defaults.push((pid, default));
                saw_default = true;
            } else if saw_default && !self.at_end() {
                let name = self.arena.interner.resolve(psym).to_string();
                self.arena.diagnostics.push((ByteSpan::new(psp.start, psp.end), format!("`{name}` has no default but follows a parameter that has one")));
            }
            params.push(pid); if self.eat(TokenKind::Comma) {
                                if self.at(TokenKind::RParen) { self.arena.diagnostics.push((self.prev_span(), "Trailing comma not permitted in parameter list".into())); break; }
            } else { break; } }
        self.expect(TokenKind::RParen); params }
    pub(crate) fn parse_fn(&mut self, mods: &DeclMods, def_vis: u8, headless: bool) -> NodeId { let sp = self.span(); if !headless { self.advance(); }
        let mut full_name = self.text().to_string();
        self.advance();
        let mut tparams = self.parse_generic_params();
        if self.eat(TokenKind::InfixDot) { full_name.push('.'); full_name.push_str(self.text()); self.advance(); tparams.extend(self.parse_generic_params()); }
        let outer_runtime = self.folds.take_runtime();
        let (name_sym, params) = (self.arena.interner.intern(&full_name), self.parse_param_list());
        let ret_ty = if !self.eat(TokenKind::Arrow) { NodeId::NONE } else if self.tuple_return_before_body() { self.parse_type_bp(ret_type::ABOVE_FN_ARROW) } else { self.parse_type() };
        let mut clauses = Vec::new();
        while self.is_rec(28) || self.at_contract_clause() {
            if self.at_contract_clause() {
                clauses.push(self.parse_contract_clause());
                continue;
            }
            self.where_loudly("a `where` clause on a function is not checked yet; write the bound in `<T : I>`");
        }
        let body = if self.eat(TokenKind::FatArrow) { let e = self.parse_expr(); let _ = self.eat(TokenKind::Semicolon); e }
        else if self.at(TokenKind::LBrace) { self.parse_fn_body() } else { let _ = self.eat(TokenKind::Semicolon); NodeId::NONE };
        let body = self.with_contracts(body, &clauses);
        self.folds.restore_runtime(outer_runtime);
        let mut ch = vec![body];
        ch.extend(params);
        let (st, ln) = self.arena.alloc_child_slice(&ch);
        let id = self.arena.alloc_node(NodeKind::FnDecl, ln as u16, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), name_sym.0, st);
        if ret_ty.is_some() { self.arena.set_type_annot(id, ret_ty); }
        self.apply_mods(id, mods, def_vis); self.add_type_params(id, tparams);
        id }
    pub(crate) fn parse_ctor(&mut self, mods: &DeclMods) -> NodeId { let sp = self.span(); self.advance(); let params = self.parse_param_list();
        let body = if self.eat(TokenKind::FatArrow) { let e = self.parse_expr(); let _ = self.eat(TokenKind::Semicolon); e }
        else if self.at(TokenKind::LBrace) { self.parse_fn_body() } else { let _ = self.eat(TokenKind::Semicolon); NodeId::NONE };
        let mut ch = vec![body]; ch.extend(params);
        let id = self.arena.alloc_list_node(NodeKind::ConstructorDecl, 0, ByteSpan::new(sp.start, self.prev_span().end), &ch);
        self.apply_mods(id, mods, 7); id }
    pub(crate) fn parse_dtor(&mut self, mods: &DeclMods) -> NodeId { let sp = self.span(); self.advance(); self.advance(); self.dtor_params_loudly();
        let body = if self.at(TokenKind::LBrace) { self.parse_fn_body() } else { let _ = self.eat(TokenKind::Semicolon); NodeId::NONE };
        let id = self.arena.alloc_node(NodeKind::DestructorDecl, 0, ByteSpan::new(sp.start, self.prev_span().end), body.0, 0);
        let mut fm = mods.clone();
        fm.flags |= DECL_FLAG_DTOR;
        self.apply_mods(id, &fm, 7);
        id }
    pub(crate) fn parse_scope_decl(&mut self) -> NodeId { let sp = self.span(); self.advance(); if self.is_receiver() { self.advance(); }
        let (mut en, mut ex) = (NodeId::NONE, NodeId::NONE); if self.eat(TokenKind::LBrace) {
            while !self.at_end() && !self.at(TokenKind::RBrace) { self.skip_seps();
                if self.is_rec(21) { self.advance(); let _ = self.eat(TokenKind::Colon); en = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_stmt() }; }
                else if self.is_rec(22) { self.advance(); let _ = self.eat(TokenKind::Colon); ex = if self.at(TokenKind::LBrace) { self.parse_block() } else { self.parse_stmt() }; }
                else { self.advance(); } }
            self.expect(TokenKind::RBrace); }
        self.arena.alloc_node(NodeKind::ScopeDecl, 0, ByteSpan::new(sp.start, self.prev_span().end), en.0, ex.0) }
    pub(crate) fn parse_indexer(&mut self, mods: &DeclMods, def_vis: u8) -> NodeId {
        let sp = self.span();
        self.advance();
        self.expect(TokenKind::LBracket);
        let mut params = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBracket) { let (psp, psym) = (self.span(), self.intern_cur());
            let ty = if self.eat(TokenKind::Colon) { self.parse_type() } else { NodeId::NONE };
            params.push(self.arena.alloc_node(NodeKind::ParamDecl, 0, psp, psym.0, ty.0));
            if !self.eat(TokenKind::Comma) { break; } }
        self.expect(TokenKind::RBracket);
        let ret = if self.eat(TokenKind::Arrow) { self.parse_type() } else { NodeId::NONE };
        let is_block = self.at(TokenKind::LBrace);
        let body = if self.eat(TokenKind::FatArrow) { let e = self.parse_expr(); let _ = self.eat(TokenKind::Semicolon); e }
        else if is_block { self.skip_accessor_block() } else { NodeId::NONE };
        let mut ch = vec![body];
        ch.extend(params);
        let id = self.arena.alloc_list_node(NodeKind::IndexerDecl, 0, ByteSpan::new(sp.start, self.prev_span().end), &ch);
        if is_block { self.arena.property_accessors.push((id, body)); }
        if ret.is_some() { self.arena.set_type_annot(id, ret); }
        self.apply_mods(id, mods, def_vis); id }
    pub(crate) fn parse_op_decl(&mut self, mods: &DeclMods, def_vis: u8) -> NodeId { let sp = self.span(); self.advance(); let mut op_txt = String::new();
        if self.eat(TokenKind::LBracket) { let _ = self.eat(TokenKind::RBracket); op_txt.push_str(if self.eat(TokenKind::Equal) { INDEX_SET_OP } else { INDEX_GET_OP }); }
        else { while !self.at_end() && !self.at(TokenKind::LParen) { op_txt.push_str(self.text()); self.advance(); } }
        let (params, ret) = (self.parse_param_list(), if self.eat(TokenKind::Arrow) { self.parse_type() } else { NodeId::NONE });
        let body = if self.eat(TokenKind::FatArrow) { let e = self.parse_expr(); let _ = self.eat(TokenKind::Semicolon); e }
        else if self.at(TokenKind::LBrace) { self.parse_fn_body() } else { let _ = self.eat(TokenKind::Semicolon); NodeId::NONE };
        // Laid out like a function, named by the operator's text: `op ?` is
        // the method `?` of its type.
        let mut ch = vec![body];
        ch.extend(params);
        let (st, ln) = self.arena.alloc_child_slice(&ch);
        let name = self.arena.interner.intern(&op_txt);
        let id = self.arena.alloc_node(NodeKind::OperatorDecl, ln as u16, ByteSpan::new(sp.start, self.prev_span().end), name.0, st);
        if ret.is_some() { self.arena.set_type_annot(id, ret); }
        self.apply_mods(id, mods, def_vis); id }
    pub(crate) fn parse_field_or_prop(&mut self, mods: &DeclMods, def_vis: u8) -> NodeId { let (sp, sym) = (self.span(), self.intern_cur());
        self.field_question_loudly();
        let ty = if self.eat(TokenKind::Colon) { self.parse_type() } else { NodeId::NONE };
        let (mut default, mut accessors) = (NodeId::NONE, NodeId::NONE);
        // A property may have both: `{ get => .. } = v` or `= v { get => .. }`.
        if self.at(TokenKind::LBrace) { accessors = self.skip_accessor_block(); }
        if self.eat(TokenKind::Equal) {
            self.no_struct_lit = true;
            default = self.parse_expr();
            self.no_struct_lit = false;
        }
        if accessors.is_none() && self.at(TokenKind::LBrace) { accessors = self.skip_accessor_block(); }
        let _ = self.eat(TokenKind::Semicolon) || self.eat(TokenKind::Comma);
        let id = self.arena.alloc_node(NodeKind::ParamDecl, 0, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), sym.0, ty.0);
        if default.is_some() { self.arena.field_defaults.push((id, default)); }
        if accessors.is_some() { self.arena.property_accessors.push((id, accessors)); }
        if ty.is_some() { self.arena.set_type_annot(id, ty); }
        self.apply_mods(id, mods, def_vis); id }
    /// `*Bundle<T> coalesce { id } with { flags as net_flags } without { x };`
    /// Each listed member is kept in `splice_mods` under the receptor
    /// sub-slot of the modifier that lists it.
    pub(crate) fn parse_bundle_splice(&mut self) -> NodeId { let sp = self.span(); self.advance(); let ty = self.parse_type();
        let mut mods: Vec<(u8, SymbolId, SymbolId)> = Vec::new();
        while self.is_rec(30) {
            let sub = self.receptor().map_or(0, |(_, s)| s);
            self.advance();
            if !self.eat(TokenKind::LBrace) { continue; }
            while !self.at_end() && !self.at(TokenKind::RBrace) {
                if self.is_word() {
                    let member = self.intern_cur();
                    let mut alias = SymbolId(0);
                    if self.is_rec(29) { self.advance(); if self.is_word() { alias = self.intern_cur(); } }
                    mods.push((sub, member, alias));
                } else { self.advance(); }
                let _ = self.eat(TokenKind::Comma) || self.eat(TokenKind::Semicolon);
            }
            self.expect(TokenKind::RBrace);
        }
        let _ = self.eat(TokenKind::Semicolon);
        let u = self.arena.alloc_node(NodeKind::UnaryExpr, UNARY_OP_SPLAT, ByteSpan::new(sp.start, self.prev_span().end), ty.0, 0);
        for (sub, member, alias) in mods { self.arena.splice_mods.push((u, sub, member, alias)); }
        self.arena.alloc_node(NodeKind::ExprStmt, 0, ByteSpan::new(sp.start, self.prev_span().end), u.0, 0) }
    pub(crate) fn parse_type_adt(&mut self, mods: &DeclMods, def_vis: u8) -> NodeId { let sp = self.span(); self.advance();
        let (sym, tps) = (self.intern_cur(), self.parse_generic_params());
        self.expect(TokenKind::Equal);
        let _ = self.eat(TokenKind::Vbar);
        let nx = self.next_non_nl(self.cur() + 1);
        if self.is_word() && nx < self.tape.len() && matches!(self.tape.kind(nx), TokenKind::LParen | TokenKind::Vbar) {
            let mut variants = Vec::new();
            loop { let (vsp, vsym) = (self.span(), self.intern_cur());
                let params = if self.at(TokenKind::LParen) { self.parse_param_list() } else { Vec::new() };
                let (st, ln) = self.arena.alloc_child_slice(&params);
                variants.push(self.arena.alloc_node(NodeKind::AdtVariant, ln as u16, ByteSpan::new(vsp.start, self.prev_span().end.max(vsp.end)), vsym.0, st));
                if !self.eat(TokenKind::Vbar) { break; } }
            let _ = self.eat(TokenKind::Semicolon);
            let (st, ln) = self.arena.alloc_child_slice(&variants);
            let id = self.arena.alloc_node(NodeKind::TypeAdtDecl, ln as u16, ByteSpan::new(sp.start, self.prev_span().end), sym.0, st);
            self.apply_mods(id, mods, def_vis); self.add_type_params(id, tps);
            id
        } else { let rhs = self.parse_type(); let _ = self.eat(TokenKind::Semicolon);
            let id = self.arena.alloc_node(NodeKind::TypeAdtDecl, 0, ByteSpan::new(sp.start, self.prev_span().end), sym.0, rhs.0);
            self.apply_mods(id, mods, def_vis); self.add_type_params(id, tps); id } }
    pub(crate) fn parse_enum_decl(&mut self, mods: &DeclMods, def_vis: u8, headless: bool) -> NodeId { let sp = self.span(); if !headless { self.advance(); }
        let sym = self.intern_cur();
        self.generics_loudly("an `enum`");
        let base = if self.eat(TokenKind::Colon) { self.parse_type() } else { NodeId::NONE };
        let mut variants = Vec::new();
        if self.eat(TokenKind::LBrace) { while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
                let _ = self.parse_mods(true);
                let (vsp, vsym) = (self.span(), self.intern_cur());
                let params = if self.at(TokenKind::LParen) { self.parse_param_list() } else { Vec::new() };
                // `Low = 5`: the variant's value is kept as its initializer.
                let value = if self.eat(TokenKind::Equal) { self.parse_expr() } else { NodeId::NONE };
                let (st, ln) = self.arena.alloc_child_slice(&params);
                let v = self.arena.alloc_node(NodeKind::AdtVariant, ln as u16, vsp, vsym.0, st);
                if value.is_some() { self.arena.field_defaults.push((v, value)); }
                variants.push(v); let _ = self.eat(TokenKind::Comma); }
            self.expect(TokenKind::RBrace); }
        let (st, ln) = self.arena.alloc_child_slice(&variants);
        let id = self.arena.alloc_node(NodeKind::TypeAdtDecl, ln as u16, ByteSpan::new(sp.start, self.prev_span().end), sym.0, st);
        // `enum E : u8`: the type that holds the variants' values.
        if base.is_some() { self.arena.base_types.push((id, base)); }
        self.apply_mods(id, mods, def_vis); id }
    pub(crate) fn parse_extern_decl(&mut self, mods: &DeclMods) -> NodeId { let sp = self.span(); self.advance();
        if self.at(TokenKind::StrLit) || self.is_word() { self.advance(); }
        if self.is_rec(8) { self.advance(); if self.at(TokenKind::StrLit) || self.is_word() { self.advance(); } }
        if self.at(TokenKind::KwFn) { return self.parse_fn(mods, 7, false); }
        let mut fns = Vec::new();
        if self.eat(TokenKind::LBrace) { while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
                let m = self.pos;
                self.parse_item(&mut fns, false);
                if self.pos == m { self.advance(); } }
            self.expect(TokenKind::RBrace); }
        self.arena.alloc_list_node(NodeKind::BlockExpr, 0, ByteSpan::new(sp.start, self.prev_span().end), &fns) }
    pub(crate) fn parse_syntax_decl(&mut self, mods: &DeclMods) -> NodeId { let sp = self.span(); self.advance();
        let (is_tc, is_ac) = (self.is_rec(5), self.is_rec(6));
        if is_tc || is_ac { self.advance(); }
        // `syntax Name { head; .. }`: a syntax class, a hole kind made of words and holes.
        if !is_tc && !is_ac && self.is_word() { return self.parse_syntax_class(sp); }
        self.expect(TokenKind::LBrace);
        let mut rules = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) {
            self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
            let rsp = self.span();
            if is_tc || is_ac { let mut pat_toks = Vec::new(); if is_tc && self.eat(TokenKind::LParen) {
                    let mut d = 1; while !self.at_end() && d > 0 { let k = self.peek();
                        if k == TokenKind::LParen { d += 1; } else if k == TokenKind::RParen { d -= 1; if d == 0 { self.advance(); break; } }
                        pat_toks.push((k, self.text().to_string())); self.advance(); }
                } else { let (mut pd, mut bd) = (0i32, 0i32); while !self.at_end() && !(pd == 0 && bd == 0 && self.at(TokenKind::FatArrow)) {
                        match self.peek() { TokenKind::LParen => pd += 1, TokenKind::RParen => pd -= 1, TokenKind::LBrace => bd += 1, TokenKind::RBrace => bd -= 1, _ => {} }
                        self.advance(); } }
                self.expect(TokenKind::FatArrow);
                let body = if self.eat(TokenKind::KwQuote) && self.eat(TokenKind::LBrace) {
                    self.in_quote_type = is_tc;
                    let t = if is_tc { self.parse_type() } else { self.parse_expr() };
                    self.in_quote_type = false; self.expect(TokenKind::RBrace); t
                } else { self.parse_expr() };
                let _ = self.eat(TokenKind::Semicolon);
                if is_tc && body.is_some() {
                    if let Some(rule) = compile_type_rule(&pat_toks, TemplateNode::from_arena(&self.arena, body)) {
                        let t = self.tables.to_mut();
                        t.type_rules.retain(|e| !same_type_trigger(e, &rule));
                        t.type_rules.push(rule);
                        } }
                rules.push(body);
                } else {
                // `[Infix(..)] rule(params) => ..`: an operator's function, and
                // `rule(params) => hook(..)`: a comptime function. Any other rule
                // has a sequence head: `word $a:Kind word => quote { .. }` (PROP-009).
                let rule_attrs = if self.at(TokenKind::LBracket) { self.parse_attrs() } else { Vec::new() };
                let kw = self.text().to_string(); self.advance();
                if rule_attrs.is_empty() && (self.rule_body_is_quote() || !self.at(TokenKind::LParen)) {
                    if self.at(TokenKind::LParen) && self.tape.matching_bracket(self.cur()).is_some_and(|c| c as usize > self.cur() + 1
                        && !(self.cur()..c as usize).any(|i| self.tape.kind(i) == TokenKind::Dollar)) {
                        self.arena.diagnostics.push((rsp, format!("SYN010: the parameters of the rule `{kw}` are holes: `{kw} ( $x:Expr , .. )`; a list `{kw}(x)` declares an operator's function, which takes its attribute")));
                    }
                    let items = self.parse_seq_head(None, false, None);
                    self.expect(TokenKind::FatArrow);
                    let body = self.parse_expr(); let _ = self.eat(TokenKind::Semicolon);
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
                let body = self.parse_expr(); let _ = self.eat(TokenKind::Semicolon);
                // An operator's template is no value: flags = 1 tells it from a quote value.
                if body.is_some() && self.arena.node(body).node_kind() == NodeKind::QuoteExpr {
                    self.arena.nodes[body.0 as usize].flags = 1;
                }
                let sym = self.arena.interner.intern(&kw);
                let mut ch = vec![body];
                ch.extend(params);
                let (st, ln) = self.arena.alloc_child_slice(&ch);
                let f = self.arena.alloc_node(NodeKind::FnDecl, ln as u16, ByteSpan::new(rsp.start, self.prev_span().end), sym.0, st);
                let mut fm = mods.clone();
                fm.attrs.extend(rule_attrs);
                fm.flags |= DECL_FLAG_COMPTIME;
                self.apply_mods(f, &fm, 7);
                rules.push(f);
                } }
        self.expect(TokenKind::RBrace);
        self.arena.alloc_list_node(NodeKind::BlockExpr, 0, ByteSpan::new(sp.start, self.prev_span().end), &rules) }
    /// `syntax Name { head; .. }`: each arm a sequence head with no template.
    fn parse_syntax_class(&mut self, sp: ByteSpan) -> NodeId {
        let name = self.text().to_string(); self.advance();
        self.expect(TokenKind::LBrace);
        let mut arms = Vec::new();
        while !self.at_end() && !self.at(TokenKind::RBrace) {
            self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
            arms.push(self.parse_seq_head(None, true, None));
            if self.at(TokenKind::FatArrow) {
                self.arena.diagnostics.push((self.span(), format!("SYN010: an arm of the syntax class `{name}` has no template")));
                while !self.at_end() && !self.at(TokenKind::Semicolon) && !self.at(TokenKind::RBrace) { self.advance(); }
            }
        }
        self.expect(TokenKind::RBrace);
        let span = ByteSpan::new(sp.start, self.prev_span().end);
        self.add_syntax_class(&name, arms, span);
        self.arena.alloc_list_node(NodeKind::BlockExpr, 0, span, &[])
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
    pub(crate) fn parse_bindings(&mut self, out: &mut Vec<NodeId>, mods: &DeclMods, def_vis: u8, kw_consumed: bool) {
        let sp = self.span(); if !kw_consumed { self.advance(); }
        let kind = if mods.is_const { NodeKind::ConstDecl } else if (mods.flags & DECL_FLAG_MUT) != 0 { NodeKind::VarStmt } else { NodeKind::LetStmt };
        let nx = self.next_non_nl(self.cur() + 1);
        if self.is_word() && nx < self.tape.len() && !matches!(self.tape.kind(nx), TokenKind::Colon | TokenKind::Equal | TokenKind::Semicolon | TokenKind::Comma) {
            let saved = self.pos;
            let dist_ty = self.parse_type();
            if dist_ty.is_some() && self.eat(TokenKind::LBrace) {
                while !self.at_end() && !self.at(TokenKind::RBrace) { self.skip_seps(); if self.at(TokenKind::RBrace) || self.at_end() { break; }
                    if !self.is_word() { self.advance(); continue; }
                    let (bsp, sym) = (self.span(), self.intern_cur());
                    let init = if self.eat(TokenKind::Equal) { self.parse_expr() } else { NodeId::NONE };
                    let _ = self.eat(TokenKind::Comma) || self.eat(TokenKind::Semicolon);
                    let id = self.arena.alloc_node(kind, 0, ByteSpan::new(bsp.start, self.prev_span().end.max(bsp.end)), sym.0, init.0);
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
                self.arena.diagnostics.push((esp, format!("Prohibited or unregistered binding modifier '{}'", bad)));
                self.advance();
                } }
        if matches!(self.peek(), TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace) { let pat = self.parse_pattern();
            let ty = if self.eat(TokenKind::Colon) { self.parse_type() } else { NodeId::NONE };
            let init = self.parse_binding_init();
            let _ = self.eat(TokenKind::Semicolon);
            let id = self.arena.alloc_node(kind, 1, ByteSpan::new(sp.start, self.prev_span().end), pat.0, init.0);
            if ty.is_some() { self.arena.set_type_annot(id, ty); }
            self.apply_mods(id, mods, def_vis); out.push(id); return; }
        // `let $name = ..` inside `quote { .. }` splices the name.
        if !self.is_word() && !self.at(TokenKind::Dollar) {
            let found = self.text().to_string();
            self.arena.diagnostics.push((self.span(), format!("expected a name to bind, found `{found}`")));
        }
        // `let SignedMathFacet<TSelf: ..> = { .. }` (stdlib/core) names a facet's
        // type parameters, which nothing reads yet: not reported, so core parses.
        let sym = self.intern_binder();
        let _ = self.parse_generic_params();
        let ty = if self.eat(TokenKind::Colon) { self.parse_type() } else { NodeId::NONE };
        let init = self.parse_binding_init();
        let _ = self.eat(TokenKind::Semicolon);
        let id = self.arena.alloc_node(kind, 0, ByteSpan::new(sp.start, self.prev_span().end.max(sp.end)), sym.0, init.0);
        if ty.is_some() { self.arena.set_type_annot(id, ty); }
        if kind == NodeKind::ConstDecl {
            self.note_const(sym, init);
        } else {
            self.note_runtime(sym);
        }
        self.apply_mods(id, mods, def_vis); out.push(id); }
    /// The `= value` of a binding, or `NONE` without one. An `=` followed
    /// by no value is an error.
    pub(crate) fn parse_binding_init(&mut self) -> NodeId {
        if !self.eat(TokenKind::Equal) {
            return NodeId::NONE;
        }
        if matches!(self.peek(), TokenKind::Semicolon | TokenKind::RBrace | TokenKind::Comma) {
            let sp = self.prev_span();
            self.arena.diagnostics.push((sp, "`=` in a binding needs a value".to_string()));
            return NodeId::NONE;
        }
        self.parse_expr()
    }
}
