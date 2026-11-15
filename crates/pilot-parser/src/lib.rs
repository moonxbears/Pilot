//! Homoiconic 14-Token Seed Parser & `$param` AST-Splicing Macro Engine (`pilot-parser`).
//! Hardcodes strictly the 14 seed `TokenKind`s; zero Pilot scalar/composite type names exist here.
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use pilot_ast::*;
use pilot_intern::SymbolId;
use pilot_lexer::{tokenize, TokenKind, TokenTape}; use pilot_span::ByteSpan;
mod accessors;
pub use accessors::{INDEX_GET_OP, INDEX_SET_OP};
mod attr_hooks;
mod generic_params;
mod generic_receiver;
mod op_decl;
mod modules;
mod pilot_impls;
mod fstring;
mod num_suffix;
mod comptime_if;
mod contracts;
mod declared_names;
mod int_lit;
mod open_range;
mod ret_type;
mod tuple_member;
mod forms;
mod inline_block;
mod unbuilt;
mod param_mods;
mod rule_arms;
mod loud;
mod template;
pub use modules::parse_unit;
use modules::EXCLUDE_RECEPTOR;
pub use pilot_impls::{EnumeratorProtocol, GeneratorProtocol, NativeKind, OperatorFn, TryProtocol, UnitLiteralHook};
use pilot_impls::*;
pub use attr_hooks::RegLayout;
pub use forms::{DEFER_ON_FAILURE, RETURN_TAIL_CALL};
pub use contracts::{CONTRACT_ENSURES, CONTRACT_REQUIRES};
pub use template::TemplateNode;
use template::clone_ast;
mod tables;
pub use tables::{MEMBER_IMPLIED_RECEIVER, PrattEntry, ParserTables, TypeConstructRule, TypePostfixKind};
pub(crate) use tables::*;
mod decl;
mod stmt;
pub(crate) use stmt::assign_op_of;
mod expr;
pub fn set_source_root(root: &std::path::Path) {
    let _ = SOURCE_ROOT.set(root.to_path_buf());
}

/// The directory holding the `stdlib/` the toolchain runs with.
pub fn source_root() -> std::path::PathBuf {
    SOURCE_ROOT
        .get()
        .cloned()
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

/// The sources of the bootstrap tables: `BOOTSTRAP_SOURCES`, then every
/// primitive file in file-name order, so adding a primitive file needs no
/// change here. A missing file is reported and skipped.
pub(crate) fn bootstrap_sources() -> Vec<(String, String)> {
    let root = source_root();
    let mut out = Vec::new();
    for rel in BOOTSTRAP_SOURCES {
        match std::fs::read_to_string(root.join(rel)) {
            Ok(src) => out.push((rel.to_string(), src)),
            Err(e) => eprintln!("internal error: bootstrap source `{rel}` unreadable: {e}"),
        }
    }
    let mut files: Vec<_> = std::fs::read_dir(root.join(PRIMITIVES_DIR))
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "pilot"))
        .collect();
    files.sort();
    for p in files {
        if let Ok(src) = std::fs::read_to_string(&p) {
            out.push((format!("{PRIMITIVES_DIR}/{}", p.file_name().unwrap_or_default().to_string_lossy()), src));
        }
    }
    out
}

/// What the parser reported while building the bootstrap tables, as
/// `file:line: message`, in bootstrap order. The tables are built without
/// showing these, so a source that does not parse would otherwise lose its
/// declarations unseen; `tests/bootstrap_sources.rs` requires none.
pub fn bootstrap_diagnostics() -> Vec<String> {
    let mut out = Vec::new();
    let _ = build_bootstrap_tables(Some(&mut out));
    out
}

/// The bootstrap tables, built by reading every source until the operator
/// grammar stops growing (PROP-008 Amendment 008-A). Operators are declared
/// by attributes in files that load after files that already use them, so
/// each pass after the first starts from the grammar the previous pass
/// collected; the last pass's reports go to `diags`.
fn build_bootstrap_tables(mut diags: Option<&mut Vec<String>>) -> ParserTables {
    let sources = bootstrap_sources();
    let mut grammar = ParserTables::new();
    loop {
        let mut t = ParserTables::with_grammar(&grammar);
        let mut out = Vec::new();
        for (rel, src) in &sources {
            let arena = parse_and_ingest(src, &mut t);
            for (span, msg) in &arena.diagnostics {
                let line = src[..(span.start as usize).min(src.len())].matches('\n').count() + 1;
                out.push(format!("{rel}:{line}: {msg}"));
            }
        }
        if t.grammar.len() <= grammar.grammar.len() && t.grammar_enums == grammar.grammar_enums {
            if let Some(d) = diags.as_deref_mut() {
                *d = out;
            }
            return t;
        }
        grammar = t;
    }
}

static BOOTSTRAP_TABLES: OnceLock<ParserTables> = OnceLock::new();

/// The tables built from the stdlib at the source root (`set_source_root`).
pub fn bootstrap_tables() -> &'static ParserTables {
    BOOTSTRAP_TABLES.get_or_init(|| build_bootstrap_tables(None))
}
pub fn parse(src: &str) -> AstArena { parse_with_tables(src, bootstrap_tables()) }
pub fn parse_with_tables(src: &str, tables: &ParserTables) -> AstArena { let mut p = Parser { src, tape: tokenize(src), pos: 0, arena: AstArena::with_capacity((src.len() / 12).max(16)), tables: Cow::Borrowed(tables), active_hdr: None, hdr_stack: Vec::new(), in_quote_type: false, no_struct_lit: false, in_cascade_step: false, ingest: false, folds: Default::default(), in_ensures: false, fn_body_depth: 0, }; p.parse_program(); p.arena }
pub fn parse_and_ingest(src: &str, tables: &mut ParserTables) -> AstArena { let mut p = Parser { src, tape: tokenize(src), pos: 0, arena: AstArena::with_capacity((src.len() / 12).max(16)), tables: Cow::Owned(std::mem::take(tables)), active_hdr: None, hdr_stack: Vec::new(), in_quote_type: false, no_struct_lit: false, in_cascade_step: false, ingest: true, folds: Default::default(), in_ensures: false, fn_body_depth: 0, };
    // The parser owns the tables while it fills them: moved in and back out, never copied.
    p.parse_program(); *tables = p.tables.into_owned(); tables.finish_ingest(src); p.arena }
#[inline]
pub(crate) fn is_seed_kw(k: TokenKind) -> bool { matches!(k, TokenKind::KwEngine | TokenKind::KwStruct | TokenKind::KwInterface | TokenKind::KwFn
        | TokenKind::KwLet | TokenKind::KwRef | TokenKind::KwIf | TokenKind::KwLoop
        | TokenKind::KwReturn | TokenKind::KwYield | TokenKind::KwDefer | TokenKind::KwComptime
        | TokenKind::KwQuote | TokenKind::KwAsm) }
#[inline]
pub(crate) fn is_word_tok(k: TokenKind) -> bool { k == TokenKind::Ident }
#[derive(Clone, Debug, Default)]
pub(crate) struct PendingAttr { pub(crate) name: SymbolId, pub(crate) args: Vec<NodeId>, pub(crate) is_mut: bool, pub(crate) bind_sym: SymbolId }
#[derive(Clone, Debug, Default)]
pub(crate) struct DeclMods { pub(crate) vis: Option<u8>, pub(crate) flags: u16, pub(crate) is_const: bool, pub(crate) is_ext: bool, pub(crate) attrs: Vec<PendingAttr>, pub(crate) ret_ty: Option<TemplateNode> }
impl DeclMods {
    pub(crate) fn merge_under(&self, hdr: Option<&ScopeHeader>) -> Self { let Some(h) = hdr else { return self.clone(); };
        let mut attrs: Vec<PendingAttr> = h.mods.attrs.iter().filter(|ha| !self.attrs.iter().any(|la| la.name == ha.name)).cloned().collect();
        attrs.extend(self.attrs.iter().cloned()); Self {
            vis: self.vis.or(h.mods.vis), flags: h.mods.flags | self.flags, is_const: self.is_const || h.mods.is_const,
            is_ext: self.is_ext || h.mods.is_ext, attrs, ret_ty: self.ret_ty.clone().or_else(|| h.mods.ret_ty.clone()), } }
    pub(crate) fn is_non_empty(&self) -> bool {
        self.vis.is_some() || self.flags != 0 || self.is_const || self.is_ext || !self.attrs.is_empty() || self.ret_ty.is_some() } }
#[derive(Clone, Debug)]
pub(crate) struct ScopeHeader { pub(crate) mods: DeclMods, pub(crate) target: u8 }
pub(crate) struct Parser<'a> { pub(crate) src: &'a str, pub(crate) tape: TokenTape, pub(crate) pos: usize, pub(crate) arena: AstArena, pub(crate) tables: Cow<'a, ParserTables>,
    pub(crate) active_hdr: Option<ScopeHeader>, pub(crate) hdr_stack: Vec<Option<ScopeHeader>>, pub(crate) in_quote_type: bool, pub(crate) no_struct_lit: bool,
    /// Parsing a cascade step (`..f(x)`, `..field = value`), which ends at the next step.
    pub(crate) in_cascade_step: bool,
    /// Feeding the bootstrap tables: every item is ingested, not only engine calls.
    pub(crate) ingest: bool,
    /// The constants and run-time names `comptime if` folds against.
    pub(crate) folds: comptime_if::FoldScope,
    /// Parsing an `ensures` clause, where `old(..)` is the entry value.
    pub(crate) in_ensures: bool,
    /// How many function bodies enclose the parser.
    pub(crate) fn_body_depth: u32, }
impl<'a> Parser<'a> {
    #[inline] pub(crate) fn cur(&self) -> usize { self.next_non_nl(self.pos) }
    #[inline] pub(crate) fn skip_seps(&mut self) { while self.eat(TokenKind::Semicolon) {} }
    #[inline] pub(crate) fn peek(&self) -> TokenKind { let p = self.cur(); if p < self.tape.len() { self.tape.kind(p) } else { TokenKind::Eof } }
    #[inline] pub(crate) fn at(&self, k: TokenKind) -> bool { self.peek() == k }
    #[inline] pub(crate) fn at_end(&self) -> bool { self.peek() == TokenKind::Eof }
    #[inline] pub(crate) fn is_word(&self) -> bool { is_word_tok(self.peek()) }
    #[inline] pub(crate) fn span(&self) -> ByteSpan { let p = self.cur(); if p < self.tape.len() { self.tape.span(p) } else { ByteSpan::new(self.src.len() as u32, self.src.len() as u32) } }
    #[inline] pub(crate) fn prev_span(&self) -> ByteSpan { if self.pos > 0 && self.pos - 1 < self.tape.len() { self.tape.span(self.pos - 1) } else { ByteSpan::new(0, 0) } }
    #[inline] pub(crate) fn text(&self) -> &'a str { let p = self.cur(); if p < self.tape.len() { self.tape.text(p, self.src) } else { "" } }
    #[inline] pub(crate) fn advance(&mut self) { let p = self.cur(); if p < self.tape.len() { self.pos = p + 1; } }
    #[inline] pub(crate) fn eat(&mut self, k: TokenKind) -> bool { let p = self.cur(); if p < self.tape.len() && self.tape.kind(p) == k { self.pos = p + 1; true } else { false } }
    #[inline] pub(crate) fn intern_cur(&mut self) -> SymbolId { let s = self.arena.interner.intern(self.text()); self.advance(); s }
    pub(crate) fn expect(&mut self, k: TokenKind) {
        if !self.eat(k) { let (sp, got) = (self.span(), self.peek()); self.arena.diagnostics.push((sp, format!("Expected {:?}, found {:?}", k, got))); }
    }
    /// Whether a line break separates the previous token from the cursor.
    pub(crate) fn newline_before_cur(&self) -> bool {
        let (from, to) = (self.prev_span().end as usize, self.span().start as usize);
        self.src.as_bytes().get(from..to).is_some_and(|gap| gap.contains(&b'\n'))
    }
    pub(crate) fn next_non_nl(&self, mut p: usize) -> usize {
        while p < self.tape.len() && matches!(self.tape.kind(p), TokenKind::Newline | TokenKind::Comment | TokenKind::BlockComment | TokenKind::Shebang) { p += 1; }
        p }
    #[inline] pub(crate) fn receptor(&self) -> Option<(u8, u8)> { if self.is_word() { self.tables.receptors.get(self.text()).copied() } else { None } }
    #[inline] pub(crate) fn is_rec(&self, kind: u8) -> bool { self.receptor().is_some_and(|(k, _)| k == kind) }
    #[inline] pub(crate) fn is_receiver(&self) -> bool { self.is_word() && !self.tables.receiver_ident.is_empty() && self.text() == self.tables.receiver_ident }
    pub(crate) fn apply_mods(&mut self, id: NodeId, mods: &DeclMods, def_vis: u8) { if id.is_none() || (id.0 as usize) >= self.arena.nodes.len() { return; }
        let vis = mods.vis.unwrap_or(def_vis) as u16;
        let written = if mods.vis.is_some() { DECL_FLAG_VIS_WRITTEN } else { 0 };
        self.arena.decl_flags[id.0 as usize] = (self.arena.decl_flags[id.0 as usize] & !DECL_VIS_MASK) | (vis & DECL_VIS_MASK) | mods.flags | written;
        for a in &mods.attrs { let args: Vec<NodeId> = a.args.iter().map(|&x| clone_ast(&mut self.arena, x)).collect();
            let (st, ln) = self.arena.alloc_child_slice(&args);
            self.arena.attrs.push(AstAttribute { target: id, name: a.name, args_start: st, args_len: ln as u16, is_mut_binding: a.is_mut, binding_sym: a.bind_sym });
        }
        if let Some(rt) = &mods.ret_ty { if self.arena.type_annot(id).is_none() {
                let sp = self.arena.span(id);
                let tid = rt.instantiate(&mut self.arena, &[], sp);
                self.arena.set_type_annot(id, tid);
                }
        } }
}
