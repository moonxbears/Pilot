//! `syntax` rules written as sequence heads (PROP-009): the words and holes
//! a use spells, matched at the rule's word, and the template the holes are
//! put into. Inside a template, `stringify($p)` (the word the library binds
//! to receptor [`SOURCE_TEXT_RECEPTOR`]) is the source text of the code bound
//! to `p`, as a string literal. The parser names no rule and no word here.

use crate::{Parser, TemplateNode};
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_lexer::TokenKind;
use pilot_span::ByteSpan;

/// The receptor of the template word that gives a hole's source text.
const SOURCE_TEXT_RECEPTOR: (u8, u8) = (36, 4);

/// The binding name under which a hole's source text is kept. It starts
/// with a byte no identifier holds, so it cannot meet a user name.
fn text_binding(param: &str) -> String {
    format!("\u{1}{param}")
}

impl<'a> Parser<'a> {
    /// `template` with each `stringify($p)` replaced by the source text of
    /// the code bound to `p`.
    fn with_source_text(&self, t: TemplateNode) -> TemplateNode {
        if let TemplateNode::Call(_, v) = &t {
            if let [TemplateNode::Ident(w), TemplateNode::Splice(p)] = v.as_slice() {
                if self.tables.receptors.get(w.as_str()) == Some(&SOURCE_TEXT_RECEPTOR) {
                    return TemplateNode::Splice(text_binding(p));
                }
            }
        }
        rebuild(t, &mut |c| self.with_source_text(c))
    }

    /// The source text of node `id`, its white space runs made one space.
    fn source_text(&self, id: NodeId) -> String {
        if id.is_none() {
            return String::new();
        }
        let s = self.arena.span(id);
        let raw = self.src.get(s.start as usize..s.end as usize).unwrap_or("");
        raw.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

/// The name a template's own binding `name` is renamed to. It ends in a byte
/// no identifier holds, so no name in the code a rule is used in can reach
/// it, and it cannot capture a name in an argument (hygiene).
fn hidden_name(name: &str) -> String {
    format!("{name}\u{1}")
}

/// `t` with each name the template binds itself (`let`/`var`, other than a
/// parameter) renamed by [`hidden_name`], in its binding and every use.
fn hygienic(t: TemplateNode, params: &[String]) -> TemplateNode {
    let mut own: Vec<String> = Vec::new();
    walk(&t, &mut |n| {
        if let TemplateNode::Let(name, _, _) | TemplateNode::For(name, _, _) = n {
            if !name.starts_with('$') && !params.contains(name) && !own.contains(name) {
                own.push(name.clone());
            }
        }
    });
    if own.is_empty() {
        return t;
    }
    rename(t, &own)
}

fn rename(t: TemplateNode, own: &[String]) -> TemplateNode {
    use TemplateNode as T;
    let hide = |s: String| if own.contains(&s) { hidden_name(&s) } else { s };
    match t {
        T::Ident(s) => T::Ident(hide(s)),
        T::Splice(s) => T::Splice(hide(s)),
        T::Let(s, m, v) => T::Let(hide(s), m, v.map(|x| Box::new(rename(*x, own)))),
        T::For(s, i, b) => T::For(hide(s), Box::new(rename(*i, own)), Box::new(rename(*b, own))),
        other => rebuild(other, &mut |c| rename(c, own)),
    }
}

/// The kind of the first node in `t` a template cannot carry, if any.
fn first_unsupported(t: &TemplateNode) -> Option<&'static str> {
    let mut found = None;
    walk(t, &mut |n| {
        if let TemplateNode::Unsupported(kind) = n {
            found.get_or_insert(*kind);
        }
    });
    found
}

/// Calls `f` on `t` and every node under it.
fn walk(t: &TemplateNode, f: &mut dyn FnMut(&TemplateNode)) {
    use TemplateNode as T;
    f(t);
    match t {
        T::TypeNamed(_, v) | T::TypeTuple(v) | T::Block(v) | T::Call(_, v) | T::Tuple(v) | T::List(v) => {
            v.iter().for_each(|x| walk(x, f))
        }
        T::If(c, a, b) => {
            walk(c, f);
            walk(a, f);
            if let Some(b) = b {
                walk(b, f);
            }
        }
        T::Binary(_, l, r) | T::Index(l, r) | T::Assign(_, l, r) | T::For(_, l, r) => {
            walk(l, f);
            walk(r, f);
        }
        T::Loop(x) | T::Unary(_, x) | T::Member(x, _) | T::Defer(_, x) | T::Yield(x) => walk(x, f),
        T::Let(_, _, v) | T::Return(v) | T::Break(v) | T::Continue(v) => {
            if let Some(x) = v {
                walk(x, f);
            }
        }
        _ => {}
    }
}

/// `t` with `f` applied to each node directly under it.
fn rebuild(t: TemplateNode, f: &mut dyn FnMut(TemplateNode) -> TemplateNode) -> TemplateNode {
    use TemplateNode as T;
    let all = |v: Vec<TemplateNode>, f: &mut dyn FnMut(TemplateNode) -> TemplateNode| v.into_iter().map(f).collect::<Vec<_>>();
    match t {
        T::Call(k, v) => T::Call(k, all(v, f)),
        T::TypeNamed(s, v) => T::TypeNamed(s, all(v, f)),
        T::TypeTuple(v) => T::TypeTuple(all(v, f)),
        T::Block(v) => T::Block(all(v, f)),
        T::Tuple(v) => T::Tuple(all(v, f)),
        T::List(v) => T::List(all(v, f)),
        T::If(c, a, b) => {
            let c = Box::new(f(*c));
            let a = Box::new(f(*a));
            T::If(c, a, b.map(|x| Box::new(f(*x))))
        }
        T::Loop(x) => T::Loop(Box::new(f(*x))),
        T::Let(n, m, v) => T::Let(n, m, v.map(|x| Box::new(f(*x)))),
        T::Return(v) => T::Return(v.map(|x| Box::new(f(*x)))),
        T::Break(v) => T::Break(v.map(|x| Box::new(f(*x)))),
        T::Continue(v) => T::Continue(v.map(|x| Box::new(f(*x)))),
        T::Unary(k, x) => T::Unary(k, Box::new(f(*x))),
        T::Binary(k, l, r) => {
            let l = Box::new(f(*l));
            T::Binary(k, l, Box::new(f(*r)))
        }
        T::Member(x, s) => T::Member(Box::new(f(*x)), s),
        T::Assign(k, t, v) => {
            let t = Box::new(f(*t));
            T::Assign(k, t, Box::new(f(*v)))
        }
        T::Defer(k, b) => T::Defer(k, Box::new(f(*b))),
        T::Yield(x) => T::Yield(Box::new(f(*x))),
        T::For(n, i, b) => {
            let i = Box::new(f(*i));
            T::For(n, i, Box::new(f(*b)))
        }
        T::Index(x, i) => {
            let x = Box::new(f(*x));
            T::Index(x, Box::new(f(*i)))
        }
        other => other,
    }
}

/// Hole kinds by position in the enum `register_hole_kinds` names
/// (stdlib/core/syntax/holes.pilot; closed in the seed, append-only under B+).
const HOLE_EXPR: u8 = 0;
const HOLE_BLOCK: u8 = 1;
const HOLE_PATTERN: u8 = 2;
const HOLE_IDENT: u8 = 3;
const HOLE_TYPE: u8 = 4;
const HOLE_STMT: u8 = 5;

/// One item of a sequence rule head (PROP-009 step 1a).
#[derive(Clone, Debug)]
pub enum HeadItem {
    /// A token the use must spell exactly (`in`, `else`, `{`).
    Word(String),
    /// `$name:Kind`: a piece of code of that kind, bound to `name`.
    Hole(String, u8),
    /// `$name:Class`: one arm of the syntax class `Class`, whose holes are
    /// bound into the rule as if written there.
    Class(String),
    /// `$( .. )?`: the items, or nothing; the group starts with a word.
    /// A hole in it may give the code it stands for when the group is not
    /// written (`$note:Expr = ""`).
    Optional(Vec<HeadItem>, Vec<(String, TemplateNode)>),
    /// `$( .. ) sep *`: the items once or more, separated by `sep` (or
    /// by nothing, when the group starts with a word).
    Repeated(Vec<HeadItem>, Option<String>),
}

/// A `syntax` rule written as a sequence: `with $open:Ident $body:Block => quote { .. }`.
#[derive(Clone, Debug)]
pub struct SeqRule {
    pub items: Vec<HeadItem>,
    pub template: TemplateNode,
}

/// The binding name that marks `name` as bound to a sequence (a repeated hole).
fn seq_binding(name: &str) -> String {
    format!("\u{2}{name}")
}

/// When `t` uses a hole bound to a sequence (a repeated hole), the bindings
/// for each repetition of `t`, in order: each such hole bound to its element.
pub(crate) fn repetitions(t: &TemplateNode, bindings: &[(String, NodeId)], arena: &AstArena) -> Option<Vec<Vec<(String, NodeId)>>> {
    let mut seqs: Vec<String> = Vec::new();
    walk(t, &mut |n| {
        if let TemplateNode::Splice(s) = n {
            if !seqs.contains(s) && bindings.iter().any(|(k, _)| *k == seq_binding(s)) {
                seqs.push(s.clone());
            }
        }
    });
    let elems: Vec<Vec<u32>> = seqs.iter().map(|s| {
        let id = bindings.iter().find(|(k, _)| k == s).map_or(NodeId::NONE, |b| b.1);
        id.is_some().then(|| { let n = arena.node(id); arena.children_slice(n.lhs, n.rhs).to_vec() }).unwrap_or_default()
    }).collect();
    let count = elems.iter().map(Vec::len).min()?;
    let rest = bindings.iter().filter(|(k, _)| !seqs.iter().any(|s| *k == seq_binding(s))).cloned();
    let rest: Vec<(String, NodeId)> = rest.collect();
    Some((0..count).map(|i| seqs.iter().zip(&elems).map(|(s, e)| (s.clone(), NodeId(e[i]))).chain(rest.iter().cloned()).collect()).collect())
}

impl<'a> Parser<'a> {
    /// The name a binding declares; in a rule template `$name` declares the
    /// name hole `name` holds.
    pub(crate) fn intern_binder(&mut self) -> SymbolId {
        if !self.eat(TokenKind::Dollar) {
            return self.intern_cur();
        }
        let hole = format!("${}", self.text());
        self.advance();
        self.arena.interner.intern(&hole)
    }

    /// The items of a sequence head, up to `=>` (or `;` when `arm_end`, or
    /// the `)` closing a group). `defaults` collects the defaults of the
    /// holes of an optional group.
    pub(crate) fn parse_seq_head(&mut self, end: Option<usize>, arm_end: bool, mut defaults: Option<&mut Vec<(String, TemplateNode)>>) -> Vec<HeadItem> {
        let mut items = Vec::new();
        let stop = |p: &Self| p.at(TokenKind::FatArrow) || (arm_end && (p.at(TokenKind::Semicolon) || p.at(TokenKind::RBrace)));
        while !self.at_end() && end.map_or(!stop(self), |e| self.cur() < e) {
            let sp = self.span();
            if !self.eat(TokenKind::Dollar) {
                items.push(HeadItem::Word(self.text().to_string()));
                self.advance();
                continue;
            }
            if self.at(TokenKind::LParen) {
                let close = self.tape.matching_bracket(self.cur()).map_or(self.tape.len(), |c| c as usize);
                self.advance();
                let mut defs = Vec::new();
                let inner = self.parse_seq_head(Some(close), false, Some(&mut defs));
                self.expect(TokenKind::RParen);
                if self.text() == "?" {
                    self.advance();
                    if !matches!(inner.first(), Some(HeadItem::Word(_))) {
                        self.seq_head_error(sp, "an optional group `$( .. )?` starts with a word");
                    }
                    items.push(HeadItem::Optional(inner, defs));
                    continue;
                }
                let sep = (self.text() != "*").then(|| self.text().to_string());
                if sep.is_some() {
                    self.advance();
                }
                if self.text() != "*" {
                    self.seq_head_error(sp, "a group is `$( .. )?` or `$( .. ) sep *`");
                }
                self.advance();
                if sep.is_none() && !self.word_led(&inner) {
                    self.seq_head_error(sp, "a repeated group with no separator starts with a word");
                }
                if !defs.is_empty() {
                    self.seq_head_error(sp, "only a hole of an optional group `$( .. )?` has a default");
                }
                items.push(HeadItem::Repeated(inner, sep));
                continue;
            }
            let name = self.text().to_string();
            self.advance();
            self.expect(TokenKind::Colon);
            let kind_name = self.text().to_string();
            self.advance();
            match self.tables.hole_kind(&kind_name) {
                Some(k) if k <= HOLE_STMT => items.push(HeadItem::Hole(name.clone(), k)),
                _ if self.tables.syntax_classes.contains_key(&kind_name) => items.push(HeadItem::Class(kind_name)),
                _ => self.seq_head_error(sp, &format!("`{kind_name}` is not a hole kind (a variant of `HoleKind`) or a syntax class")),
            }
            if let Some(defs) = defaults.as_deref_mut().filter(|_| self.text() == "=") {
                self.advance();
                let d = self.parse_prefix();
                defs.push((name, TemplateNode::from_arena(&self.arena, d)));
            }
        }
        items
    }

    fn seq_head_error(&mut self, sp: ByteSpan, what: &str) {
        self.arena.diagnostics.push((sp, format!("SYN010: {what}")));
    }

    /// Whether every use of `items` starts with a word: a word, or a class
    /// whose arms all do.
    fn word_led(&self, items: &[HeadItem]) -> bool {
        match items.first() {
            Some(HeadItem::Word(_)) => true,
            Some(HeadItem::Class(c)) => self.tables.syntax_classes.get(c).is_some_and(|arms| arms.iter().all(|a| self.word_led(a))),
            _ => false,
        }
    }

    /// Records a sequence rule for `word`. Two holes of one name reachable
    /// from it are SYN011.
    pub(crate) fn add_seq_rule(&mut self, word: &str, items: Vec<HeadItem>, template: TemplateNode, body: ByteSpan) {
        if let Some(kind) = first_unsupported(&template) {
            self.arena.diagnostics.push((
                body,
                format!("the template of `{word}` holds a node of kind `{kind}`, which a rule template cannot carry yet"),
            ));
        }
        let mut names = Vec::new();
        self.hole_names(&items, &mut names);
        if let Some(d) = names.iter().enumerate().find(|(i, n)| names[..*i].contains(n)) {
            self.arena.diagnostics.push((body, format!("SYN011: the rule `{word}` binds `{}` twice", d.1)));
        }
        let template = hygienic(self.with_source_text(template), &names);
        self.tables.to_mut().seq_rules.entry(word.to_string()).or_default().push(SeqRule { items, template });
    }

    /// Records the syntax class `name` (`syntax Name { head; .. }`). Two
    /// arms one use could both start (SYN011) are rejected: an arm is
    /// chosen by its first token.
    pub(crate) fn add_syntax_class(&mut self, name: &str, arms: Vec<Vec<HeadItem>>, sp: ByteSpan) {
        for (j, b) in arms.iter().enumerate() {
            if let Some(i) = arms[..j].iter().position(|a| !self.leads_differ(a, b)) {
                let msg = format!("SYN011: arms {} and {} of the syntax class `{name}` start alike, so a use cannot tell them apart", i + 1, j + 1);
                self.arena.diagnostics.push((sp, msg));
            }
        }
        let mut names = Vec::new();
        for a in &arms {
            self.hole_names(a, &mut names);
        }
        if let Some(d) = names.iter().enumerate().find(|(i, n)| names[..*i].contains(n)) {
            self.arena.diagnostics.push((sp, format!("SYN011: the syntax class `{name}` binds `{}` twice", d.1)));
        }
        self.tables.to_mut().syntax_classes.insert(name.to_string(), arms);
    }

    /// Whether no token can start both `a` and `b`.
    fn leads_differ(&self, a: &[HeadItem], b: &[HeadItem]) -> bool {
        let words = |x: &[HeadItem]| -> Option<Vec<String>> {
            match x.first() {
                Some(HeadItem::Word(w)) => Some(vec![w.clone()]),
                Some(HeadItem::Class(c)) => {
                    let arms = self.tables.syntax_classes.get(c)?;
                    arms.iter().map(|a| match a.first() {
                        Some(HeadItem::Word(w)) => Some(w.clone()),
                        _ => None,
                    }).collect()
                }
                _ => None,
            }
        };
        match (words(a), words(b)) {
            (Some(x), Some(y)) => !x.iter().any(|w| y.contains(w)),
            _ => false,
        }
    }

    /// Expands the rule for the word at the cursor: the first, in the order
    /// written, whose head the code that follows matches. `None` (nothing
    /// read) when the word is no rule's or no rule matches before its first
    /// spelled word after the rule's own; otherwise the expansion, or
    /// `NONE` with the diagnostic of the rule that read furthest.
    pub(crate) fn expand_seq_rule(&mut self) -> Option<NodeId> {
        if !self.is_word() || self.file_declares_fn(self.text()) {
            return None;
        }
        let rules = self.tables.seq_rules.get(self.text())?.clone();
        let (start, diags, sp) = (self.pos, self.arena.diagnostics.len(), self.span());
        let word = self.text().to_string();
        let mut best: Option<(usize, Vec<(ByteSpan, String)>)> = None;
        let after = self.cur() + 1;
        let fitting: Vec<&SeqRule> = rules.iter().filter(|r| self.quick_fit(&r.items, after)).collect();
        for rule in fitting {
            self.advance();
            let (mut bindings, mut words) = (Vec::new(), 0usize);
            if self.match_items(&word, &rule.items, &mut bindings, &mut words) {
                self.arena.diagnostics.truncate(diags);
                let span = ByteSpan::new(sp.start, self.prev_span().end.max(sp.end));
                return Some(rule.template.instantiate(&mut self.arena, &bindings, span));
            }
            let reached = self.pos;
            let found: Vec<_> = self.arena.diagnostics.drain(diags..).collect();
            if words > 0 && best.as_ref().is_none_or(|(p, _)| reached > *p) {
                best = Some((reached, found));
            }
            self.pos = start;
        }
        // No rule read past its first spelled word. Followed by a name or a
        // literal, the word is still meant as the rule: the first rule says
        // what it expected.
        let Some((reached, found)) = best else {
            let nx = self.next_non_nl(self.cur() + 1);
            let k = (nx < self.tape.len()).then(|| self.tape.kind(nx))?;
            if !(crate::is_word_tok(k) || matches!(k, TokenKind::IntLit | TokenKind::StrLit)) {
                return None;
            }
            self.advance();
            let _ = self.match_items(&word, &rules[0].items, &mut Vec::new(), &mut 0);
            return Some(NodeId::NONE);
        };
        self.pos = reached;
        self.arena.diagnostics.extend(found);
        Some(NodeId::NONE)
    }

    /// Whether `items` fit the tokens from `at`, as far as words, names and
    /// blocks show; an expression, pattern or type hole fits anything.
    fn quick_fit(&self, items: &[HeadItem], mut at: usize) -> bool {
        for item in items {
            at = self.next_non_nl(at);
            let here = (at < self.tape.len()).then(|| (self.tape.kind(at), self.tape.text(at, self.src)));
            match item {
                HeadItem::Word(w) if here.is_some_and(|(_, t)| t == w) => at += 1,
                HeadItem::Hole(_, HOLE_IDENT) if here.is_some_and(|(k, _)| crate::is_word_tok(k)) => at += 1,
                HeadItem::Hole(_, HOLE_BLOCK) if here.is_some_and(|(k, _)| k == TokenKind::LBrace) => {
                    at = self.tape.matching_bracket(at).map_or(self.tape.len(), |c| c as usize + 1);
                }
                HeadItem::Optional(inner, _) => {
                    if self.quick_fit(&inner[..1], at) {
                        return self.quick_fit(inner, at);
                    }
                }
                HeadItem::Class(c) => {
                    let arms = self.tables.syntax_classes.get(c);
                    return arms.is_some_and(|arms| arms.iter().any(|a| self.quick_fit(&a[..a.len().min(1)], at)));
                }
                HeadItem::Word(_) | HeadItem::Hole(_, HOLE_IDENT) | HeadItem::Hole(_, HOLE_BLOCK) => return false,
                _ => return true,
            }
        }
        true
    }

    /// Matches `items` at the cursor, binding their holes into `out` and
    /// counting the words matched in `words`; false (with a diagnostic) at
    /// the first token that does not fit.
    fn match_items(&mut self, word: &str, items: &[HeadItem], out: &mut Vec<(String, NodeId)>, words: &mut usize) -> bool {
        for (i, item) in items.iter().enumerate() {
            match item {
                HeadItem::Word(w) if self.text() == w => {
                    self.advance();
                    *words += 1;
                }
                HeadItem::Word(w) => {
                    let (sp, found) = (self.span(), self.text().to_string());
                    if self.at_unbuilt_clause() {
                        self.report_unbuilt(sp, &found);
                    } else {
                        self.arena.diagnostics.push((sp, format!("SYN010: `{word}` expects `{w}` here, found `{found}`")));
                    }
                    return false;
                }
                HeadItem::Hole(name, kind) => {
                    let bp = self.hole_power(items, i);
                    let before_block = matches!(items.get(i + 1), Some(HeadItem::Hole(_, HOLE_BLOCK)));
                    let (sp, found) = (self.span(), self.text().to_string());
                    let node = self.parse_hole(word, name, *kind, before_block, bp);
                    if node.is_none() {
                        if !matches!(*kind, HOLE_BLOCK | HOLE_IDENT) {
                            self.arena.diagnostics.push((sp, format!("SYN010: `{word}` expects code for `${name}`, found `{found}`")));
                        }
                        return false;
                    }
                    let text = self.source_text(node);
                    let sym = self.arena.interner.intern(&text);
                    let text_node = self.arena.alloc_node(NodeKind::StrLit, 0, self.prev_span(), sym.0, 0);
                    out.push((name.clone(), node));
                    out.push((text_binding(name), text_node));
                }
                HeadItem::Class(c) => {
                    let arms = self.tables.syntax_classes.get(c).cloned().unwrap_or_default();
                    let at = self.cur();
                    let Some(k) = arms.iter().position(|a| self.quick_fit(&a[..a.len().min(1)], at)) else {
                        let msg = format!("SYN010: `{word}` expects {c} here, found `{}`", self.text());
                        self.arena.diagnostics.push((self.span(), msg));
                        return false;
                    };
                    for (j, arm) in arms.iter().enumerate() {
                        if j != k {
                            self.bind_absent(arm, out);
                        }
                    }
                    if !self.match_items(word, &arms[k], out, words) {
                        return false;
                    }
                }
                // An optional group binds a `bool` named after its first word:
                // whether it was written.
                HeadItem::Optional(inner, defaults) => {
                    let Some(HeadItem::Word(w)) = inner.first() else { continue };
                    let written = self.text() == w;
                    let flag = self.arena.alloc_node(NodeKind::BoolLit, 0, self.span(), written as u32, 0);
                    out.push((w.clone(), flag));
                    if written {
                        if !self.match_items(word, inner, out, words) {
                            return false;
                        }
                        continue;
                    }
                    let mut names = Vec::new();
                    self.hole_names(inner, &mut names);
                    let span = self.span();
                    for n in names.into_iter().filter(|n| n != w) {
                        let node = match defaults.iter().find(|(d, _)| *d == n) {
                            Some((_, t)) => t.instantiate(&mut self.arena, &[], span),
                            None => NodeId::NONE,
                        };
                        out.push((n, node));
                    }
                }
                // A group that starts with a word may be written no times.
                HeadItem::Repeated(inner, sep) => {
                    let led = self.word_led(inner);
                    let mut runs: Vec<Vec<(String, NodeId)>> = Vec::new();
                    while !self.at_end() && (!led || self.quick_fit(&inner[..1], self.cur())) {
                        let mut one = Vec::new();
                        if !self.match_items(word, inner, &mut one, words) {
                            return false;
                        }
                        runs.push(one);
                        if let Some(s) = sep {
                            if self.text() != s {
                                break;
                            }
                            self.advance();
                        } else if !led {
                            break;
                        }
                    }
                    self.bind_runs(inner, &runs, out);
                }
            }
        }
        true
    }

    /// Binds each hole name of `inner` to the sequence of what the runs
    /// bound it to (absent ones left out).
    fn bind_runs(&mut self, inner: &[HeadItem], runs: &[Vec<(String, NodeId)>], out: &mut Vec<(String, NodeId)>) {
        let mut names = Vec::new();
        self.hole_names(inner, &mut names);
        let span = self.prev_span();
        for n in names {
            let ids: Vec<NodeId> = runs.iter().filter_map(|r| r.iter().find(|(k, _)| *k == n).map(|(_, id)| *id)).filter(|id| id.is_some()).collect();
            let seq = self.arena.alloc_list_node(NodeKind::TupleExpr, 0, span, &ids);
            out.push((seq_binding(&n), NodeId::NONE));
            out.push((n, seq));
        }
    }

    /// Binds the holes of `items`, which were not written: a flag false, a
    /// repeated hole an empty sequence, any other hole absent.
    fn bind_absent(&mut self, items: &[HeadItem], out: &mut Vec<(String, NodeId)>) {
        for item in items {
            match item {
                HeadItem::Hole(n, _) => out.push((n.clone(), NodeId::NONE)),
                HeadItem::Optional(inner, _) => {
                    if let Some(HeadItem::Word(w)) = inner.first() {
                        let flag = self.arena.alloc_node(NodeKind::BoolLit, 0, self.span(), 0, 0);
                        out.push((w.clone(), flag));
                    }
                    self.bind_absent(&inner[1..], out);
                }
                HeadItem::Repeated(inner, _) => self.bind_runs(inner, &[], out),
                HeadItem::Class(c) => {
                    for arm in self.tables.syntax_classes.get(c).cloned().unwrap_or_default() {
                        self.bind_absent(&arm, out);
                    }
                }
                HeadItem::Word(_) => {}
            }
        }
    }

    /// The binding power an expression hole at `items[i]` is read at: just
    /// above an operator word that follows it, or that stands between it
    /// and an expression hole before it (`$left:Expr == $right:Expr`), so
    /// the operator is left for the head.
    fn hole_power(&self, items: &[HeadItem], i: usize) -> Option<u8> {
        let infix = |it: Option<&HeadItem>| match it {
            Some(HeadItem::Word(w)) => self.tables.infix_ops.get(w),
            _ => None,
        };
        if let Some(e) = infix(items.get(i + 1)) {
            return Some(e.bp + 1);
        }
        let expr_before = i >= 2 && matches!(items.get(i - 2), Some(HeadItem::Hole(_, HOLE_EXPR)));
        infix(i.checked_sub(1).and_then(|p| items.get(p))).filter(|_| expr_before).map(|e| e.bp + u8::from(!e.right_assoc))
    }

    fn parse_hole(&mut self, word: &str, name: &str, kind: u8, before_block: bool, bp: Option<u8>) -> NodeId {
        match kind {
            HOLE_BLOCK if self.at(TokenKind::LBrace) => self.parse_block(),
            HOLE_IDENT if self.is_word() => {
                let (w, sp) = (self.text().to_string(), self.span());
                self.advance();
                self.arena.alloc_ident(&w, sp)
            }
            HOLE_PATTERN => self.parse_pattern(),
            HOLE_TYPE => self.parse_type(),
            HOLE_EXPR => {
                let saved = self.no_struct_lit;
                self.no_struct_lit = before_block;
                let e = match bp {
                    Some(bp) => self.parse_expr_bp(bp),
                    None => self.parse_expr(),
                };
                self.no_struct_lit = saved;
                e
            }
            HOLE_BLOCK | HOLE_IDENT => {
                let what = if kind == HOLE_BLOCK { "a block `{ .. }`" } else { "a name" };
                let msg = format!("SYN010: `{word}` expects {what} for `${name}`, found `{}`", self.text());
                self.arena.diagnostics.push((self.span(), msg));
                NodeId::NONE
            }
            HOLE_STMT => self.parse_stmt(),
            _ => NodeId::NONE,
        }
    }

    /// The hole names in `items`, nested groups and classes included.
    fn hole_names(&self, items: &[HeadItem], out: &mut Vec<String>) {
        for item in items {
            match item {
                HeadItem::Hole(n, _) => out.push(n.clone()),
                HeadItem::Optional(inner, _) => {
                    if let Some(HeadItem::Word(w)) = inner.first() {
                        out.push(w.clone());
                    }
                    self.hole_names(inner, out);
                }
                HeadItem::Repeated(inner, _) => self.hole_names(inner, out),
                HeadItem::Class(c) => {
                    for arm in self.tables.syntax_classes.get(c).map(Vec::as_slice).unwrap_or_default() {
                        self.hole_names(arm, out);
                    }
                }
                HeadItem::Word(_) => {}
            }
        }
    }
}
