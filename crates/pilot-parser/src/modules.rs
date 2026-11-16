//! Module-level declarations: `pkg` and `use`, `without` export exclusions,
//! and parsing a file as one unit of a linked program.

use crate::*;

/// The receptor `stdlib/core/syntax/module_ext.pilot` binds the export
/// exclusion keyword to.
pub(crate) const EXCLUDE_RECEPTOR: (u8, u8) = (30, 2);

/// Parses `src` as one more unit of a linked program into `arena`, which may
/// already hold other units. The unit's spans start at `base` in the
/// program-wide offset space, so no two units overlap.
pub fn parse_unit(arena: AstArena, src: &str, file: pilot_span::FileId, base: u32) -> AstArena {
    let (first_node, first_diag) = (arena.nodes.len() as u32, arena.diagnostics.len() as u32);
    let mut p = Parser {
        src,
        tape: tokenize(src),
        pos: 0,
        arena,
        tables: Cow::Borrowed(bootstrap_tables()),
        active_hdr: None,
        hdr_stack: Vec::new(),
        in_quote_type: false,
        no_struct_lit: false,
        in_cascade_step: false,
        ingest: false,
        folds: Default::default(),
        in_ensures: false,
        fn_body_depth: 0,
    };
    p.parse_program();
    let mut arena = p.arena;
    arena.shift_spans_from(first_node, first_diag, base);
    let root = arena.root;
    arena.units.push(pilot_ast::UnitInfo {
        file,
        root,
        base,
        first_node,
        first_diag,
    });
    arena
}
impl<'a> Parser<'a> {
    pub(crate) fn parse_module_decl(
        &mut self,
        is_pkg: bool,
        mods: &DeclMods,
        out: &mut Vec<NodeId>,
    ) {
        let sp = self.span();
        self.advance();
        if !is_pkg && self.is_word() && self.tables.modifiers.get(self.text()) == Some(&ModAct::Ext)
        {
            self.advance();
            self.advance();
            if self.at(TokenKind::LBrace) {
                out.push(self.parse_block());
                return;
            }
        }
        let in_block = is_pkg && self.eat(TokenKind::LBrace);
        loop {
            if in_block && (self.at_end() || self.at(TokenKind::RBrace)) {
                break;
            }
            let mut path = String::new();
            // Commas inside `{ .. }` (`use A.{X, Y};`) separate selected names, not declarations.
            let mut depth = 0usize;
            while !self.at_end()
                && !self.at(TokenKind::Semicolon)
                && (depth > 0 || !self.at(TokenKind::Comma))
                && (depth > 0 || !in_block || !self.at(TokenKind::RBrace))
            {
                if depth == 0
                    && (self.is_rec(29)
                        || (!path.is_empty() && self.newline_before_cur() && !in_block))
                {
                    break;
                }
                if self.at(TokenKind::LBrace) {
                    depth += 1;
                } else if self.at(TokenKind::RBrace) {
                    depth = depth.saturating_sub(1);
                }
                path.push_str(self.text());
                self.advance();
            }
            let alias = if self.is_rec(29) {
                self.advance();
                if self.is_word() {
                    self.intern_cur()
                } else {
                    SymbolId(0)
                }
            } else {
                SymbolId(0)
            };
            let p = path.trim();
            if !p.is_empty() {
                let sym = self.arena.interner.intern(p);
                let id = self.arena.alloc_node(
                    NodeKind::ImportDecl,
                    0,
                    ByteSpan::new(sp.start, self.prev_span().end),
                    sym.0,
                    alias.0,
                );
                let mut m = mods.clone();
                if is_pkg {
                    m.flags |= DECL_FLAG_PKG;
                }
                self.apply_mods(id, &m, 7);
                out.push(id);
            }
            if in_block {
                let _ = self.eat(TokenKind::Comma);
                let _ = self.eat(TokenKind::Semicolon);
            } else if self.eat(TokenKind::Comma) {
                continue;
            } else {
                break;
            }
        }
        if in_block {
            self.expect(TokenKind::RBrace);
        }
        let _ = self.eat(TokenKind::Semicolon);
    }
    /// Whether the token after the current one is a word.
    pub(crate) fn word_follows(&self) -> bool {
        let nx = self.next_non_nl(self.cur() + 1);
        nx < self.tape.len() && is_word_tok(self.tape.kind(nx))
    }
    /// `without Item;` or `without Item { member, .. };`: the module does not
    /// export `Item`, or exports it without the listed members. The node's
    /// children are the item's name and then each member's name.
    pub(crate) fn parse_exclude_decl(&mut self) -> NodeId {
        let sp = self.span();
        self.advance();
        let mut names = vec![self.parse_exclude_name()];
        if self.eat(TokenKind::LBrace) {
            while !self.at_end() && !self.at(TokenKind::RBrace) {
                self.skip_seps();
                if self.at(TokenKind::RBrace) {
                    break;
                }
                if !self.is_word() {
                    let found = self.text().to_string();
                    self.arena.diagnostics.push((
                        self.span(),
                        format!("expected a member name in `without`, found `{found}`"),
                    ));
                    self.advance();
                    continue;
                }
                names.push(self.parse_exclude_name());
                let _ = self.eat(TokenKind::Comma);
            }
            self.expect(TokenKind::RBrace);
        }
        let _ = self.eat(TokenKind::Semicolon);
        let span = ByteSpan::new(sp.start, self.prev_span().end);
        self.arena
            .alloc_list_node(NodeKind::ExcludeDecl, 0, span, &names)
    }
    fn parse_exclude_name(&mut self) -> NodeId {
        let sp = self.span();
        let sym = self.intern_cur();
        self.arena.alloc_node(NodeKind::IdentExpr, 0, sp, sym.0, 0)
    }
}
