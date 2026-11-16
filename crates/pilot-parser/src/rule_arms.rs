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
        T::For(s, i, b) => T::For(
            hide(s),
            Box::new(rename(*i, own)),
            Box::new(rename(*b, own)),
        ),
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
        T::TypeNamed(_, v)
        | T::TypeTuple(v)
        | T::Block(v)
        | T::Call(_, v)
        | T::Tuple(v)
        | T::List(v)
        | T::Form(_, v) => v.iter().for_each(|x| walk(x, f)),
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
    let all = |v: Vec<TemplateNode>, f: &mut dyn FnMut(TemplateNode) -> TemplateNode| {
        v.into_iter().map(f).collect::<Vec<_>>()
    };
    match t {
        T::Call(k, v) => T::Call(k, all(v, f)),
        T::Form(k, v) => T::Form(k, all(v, f)),
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
const HOLE_STMTS: u8 = 6;
const HOLE_REST: u8 = 7;

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

/// The binding of a node spanning the code one repetition read: a `form(..)`
/// built for that repetition takes its span (a `switch` arm, a `catch`).
pub(crate) const RUN_SPAN: &str = "\u{3}";

/// The binding of the spans of the repetitions that bound `name`.
fn run_spans(name: &str) -> String {
    format!("{RUN_SPAN}{name}")
}

/// When `t` uses a hole bound to a sequence (a repeated hole), the bindings
/// for each repetition of `t`, in order: each such hole bound to its element.
pub(crate) fn repetitions(
    t: &TemplateNode,
    bindings: &[(String, NodeId)],
    arena: &AstArena,
) -> Option<Vec<Vec<(String, NodeId)>>> {
    let mut seqs: Vec<String> = Vec::new();
    walk(t, &mut |n| {
        if let TemplateNode::Splice(s) = n {
            if !seqs.contains(s) && bindings.iter().any(|(k, _)| *k == seq_binding(s)) {
                seqs.push(s.clone());
            }
        }
    });
    // Holes of one repeated group stay aligned: a repetition that left one
    // of them out holds `NONE` for it. A template that repeats over one hole
    // only skips those.
    let children = |name: &str| -> Vec<u32> {
        let id = bindings
            .iter()
            .find(|(k, _)| k == name)
            .map_or(NodeId::NONE, |b| b.1);
        id.is_some()
            .then(|| {
                let n = arena.node(id);
                arena.children_slice(n.lhs, n.rhs).to_vec()
            })
            .unwrap_or_default()
    };
    let elems: Vec<Vec<u32>> = seqs.iter().map(|s| children(s)).collect();
    let spans = children(&run_spans(seqs.first()?));
    let keep: Vec<usize> = if seqs.len() == 1 {
        (0..elems[0].len())
            .filter(|&i| NodeId(elems[0][i]).is_some())
            .collect()
    } else {
        (0..elems.iter().map(Vec::len).min()?).collect()
    };
    // A hole repeated inside a repetition is a sequence in each repetition.
    let nested: Vec<(String, NodeId)> = seqs
        .iter()
        .filter(|s| {
            bindings
                .iter()
                .any(|(k, _)| *k == seq_binding(&seq_binding(s)))
        })
        .map(|s| (seq_binding(s), NodeId::NONE))
        .collect();
    let rest = bindings
        .iter()
        .filter(|(k, _)| !seqs.iter().any(|s| *k == seq_binding(s)))
        .cloned();
    let rest: Vec<(String, NodeId)> = nested.into_iter().chain(rest).collect();
    Some(
        keep.into_iter()
            .map(|i| {
                let span = spans.get(i).map(|&c| (RUN_SPAN.to_string(), NodeId(c)));
                seqs.iter()
                    .zip(&elems)
                    .map(|(s, e)| (s.clone(), NodeId(e[i])))
                    .chain(span)
                    .chain(rest.iter().cloned())
                    .collect()
            })
            .collect(),
    )
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

    /// The items of a sequence head, up to `=>` (up to `;` when `arm_end`,
    /// or the `)` closing a group). `defaults` collects the defaults of the
    /// holes of an optional group.
    pub(crate) fn parse_seq_head(
        &mut self,
        end: Option<usize>,
        arm_end: bool,
        mut defaults: Option<&mut Vec<(String, TemplateNode)>>,
    ) -> Vec<HeadItem> {
        let mut items = Vec::new();
        // Repeated groups with no separator and no leading word: each must
        // be followed by a word, which ends it.
        let mut until_word: Vec<(usize, ByteSpan)> = Vec::new();
        // A class arm ends at `;` (and may spell `=>`); a rule's head at `=>`.
        let stop = |p: &Self| {
            if arm_end {
                p.at(TokenKind::Semicolon) || p.at(TokenKind::RBrace)
            } else {
                p.at(TokenKind::FatArrow)
            }
        };
        while !self.at_end() && end.map_or(!stop(self), |e| self.cur() < e) {
            let sp = self.span();
            if !self.eat(TokenKind::Dollar) {
                items.push(HeadItem::Word(self.text().to_string()));
                self.advance();
                continue;
            }
            if self.at(TokenKind::LParen) {
                let close = self
                    .tape
                    .matching_bracket(self.cur())
                    .map_or(self.tape.len(), |c| c as usize);
                self.advance();
                let mut defs = Vec::new();
                let inner = self.parse_seq_head(Some(close), false, Some(&mut defs));
                self.expect(TokenKind::RParen);
                if self.text() == "?" {
                    self.advance();
                    if !matches!(
                        inner.first(),
                        Some(
                            HeadItem::Word(_)
                                | HeadItem::Hole(_, HOLE_IDENT | HOLE_BLOCK)
                                | HeadItem::Class(_)
                        )
                    ) {
                        self.seq_head_error(sp, "an optional group `$( .. )?` starts with a word, a name or block hole, or a syntax class");
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
                    until_word.push((items.len(), sp));
                }
                if !defs.is_empty() {
                    self.seq_head_error(
                        sp,
                        "only a hole of an optional group `$( .. )?` has a default",
                    );
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
                Some(k) if k <= HOLE_REST => items.push(HeadItem::Hole(name.clone(), k)),
                _ if self.tables.syntax_classes.contains_key(&kind_name) => items.push(HeadItem::Class(kind_name)),
                _ => self.seq_head_error(sp, &format!("`{kind_name}` is not a hole kind (a variant of `HoleKind`) or a syntax class")),
            }
            if let Some(defs) = defaults.as_deref_mut().filter(|_| self.text() == "=") {
                self.advance();
                let d = self.parse_prefix();
                defs.push((name, TemplateNode::from_arena(&self.arena, d)));
            }
        }
        for (i, sp) in until_word {
            if !matches!(items.get(i + 1), Some(HeadItem::Word(_))) {
                self.seq_head_error(
                    sp,
                    "a repeated group with no separator starts with a word or is followed by one",
                );
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
            Some(HeadItem::Class(c)) => self
                .tables
                .syntax_classes
                .get(c)
                .is_some_and(|arms| arms.iter().all(|a| self.word_led(a))),
            _ => false,
        }
    }

    /// Records a sequence rule for `word`. Two holes of one name reachable
    /// from it are SYN011.
    pub(crate) fn add_seq_rule(
        &mut self,
        word: &str,
        items: Vec<HeadItem>,
        template: TemplateNode,
        body: ByteSpan,
    ) {
        if let Some(kind) = first_unsupported(&template) {
            self.arena.diagnostics.push((
                body,
                format!("the template of `{word}` holds a node of kind `{kind}`, which a rule template cannot carry yet"),
            ));
        }
        let mut names = Vec::new();
        self.hole_names(&items, &mut names);
        if let Some(d) = names
            .iter()
            .enumerate()
            .find(|(i, n)| names[..*i].contains(n))
        {
            self.arena.diagnostics.push((
                body,
                format!("SYN011: the rule `{word}` binds `{}` twice", d.1),
            ));
        }
        // A template that is one `form(..)` builds that node itself, not a
        // block around it (a `guard`'s binding outlives the statement).
        let template = match self.with_forms(template, body) {
            TemplateNode::Block(v) if v.len() == 1 && matches!(v[0], TemplateNode::Form(..)) => {
                v.into_iter().next().unwrap_or(TemplateNode::NoneLit)
            }
            t => t,
        };
        let template = hygienic(self.with_source_text(template), &names);
        self.tables
            .to_mut()
            .seq_rules
            .entry(word.to_string())
            .or_default()
            .push(SeqRule { items, template });
    }

    /// `t` with each `form(Forms::X, ..)` (the builder word and enum
    /// `register_core_forms` named) made a core form node.
    fn with_forms(&mut self, t: TemplateNode, body: ByteSpan) -> TemplateNode {
        if let TemplateNode::Call(_, v) = &t {
            if let [TemplateNode::Ident(b), TemplateNode::Member(e, variant), ..] = v.as_slice() {
                let (forms_enum, builder) = self.tables.core_forms.clone();
                if !builder.is_empty()
                    && *b == builder
                    && matches!(e.as_ref(), TemplateNode::Ident(n) if *n == forms_enum)
                {
                    let variant = variant.clone();
                    let ops: Vec<TemplateNode> = v[2..]
                        .iter()
                        .map(|o| self.with_forms(o.clone(), body))
                        .collect();
                    return match self
                        .tables
                        .core_form(&variant)
                        .filter(|&f| crate::forms::has_constructor(f))
                    {
                        Some(f) => TemplateNode::Form(f, ops),
                        None => {
                            let msg = format!("SYN010: `{variant}` is not a node the seed can build (a variant of `{forms_enum}`)");
                            self.arena.diagnostics.push((body, msg));
                            TemplateNode::NoneLit
                        }
                    };
                }
            }
        }
        rebuild(t, &mut |c| self.with_forms(c, body))
    }

    /// Records the syntax class `name` (`syntax Name { head; .. }`). An arm
    /// is chosen by its first token, so two arms one use could both start
    /// are SYN011; only the last arm may start with a hole that reads any
    /// code (it is taken when no other arm fits). Arms may bind the same
    /// names; one arm binding a name twice is SYN011.
    pub(crate) fn add_syntax_class(&mut self, name: &str, arms: Vec<Vec<HeadItem>>, sp: ByteSpan) {
        for (j, b) in arms.iter().enumerate() {
            let fallback = j + 1 == arms.len() && self.leads(b).is_none();
            if let Some(i) = arms[..j]
                .iter()
                .position(|a| !fallback && !self.leads_differ(a, b))
            {
                let msg = format!("SYN011: arms {} and {} of the syntax class `{name}` start alike, so a use cannot tell them apart", i + 1, j + 1);
                self.arena.diagnostics.push((sp, msg));
            }
            let mut names = Vec::new();
            self.hole_names(b, &mut names);
            if let Some(d) = names
                .iter()
                .enumerate()
                .find(|(i, n)| names[..*i].contains(n))
            {
                self.arena.diagnostics.push((
                    sp,
                    format!("SYN011: the syntax class `{name}` binds `{}` twice", d.1),
                ));
            }
        }
        self.tables
            .to_mut()
            .syntax_classes
            .insert(name.to_string(), arms);
    }

    /// The tokens a use of `items` can start with, when they are known.
    fn leads(&self, items: &[HeadItem]) -> Option<Vec<String>> {
        match items.first() {
            Some(HeadItem::Word(w)) => Some(vec![w.clone()]),
            Some(HeadItem::Hole(_, HOLE_BLOCK)) => Some(vec!["{".to_string()]),
            Some(HeadItem::Class(c)) => {
                let arms = self.tables.syntax_classes.get(c)?;
                let mut out = Vec::new();
                for a in arms {
                    out.extend(self.leads(a)?);
                }
                Some(out)
            }
            _ => None,
        }
    }

    /// Whether no token can start both `a` and `b`.
    fn leads_differ(&self, a: &[HeadItem], b: &[HeadItem]) -> bool {
        match (self.leads(a), self.leads(b)) {
            (Some(x), Some(y)) => !x.iter().any(|w| y.contains(w)),
            _ => false,
        }
    }

    /// The words the uses of `items` start with.
    fn lead_words(&self, items: &[HeadItem], out: &mut Vec<String>) {
        match items.first() {
            Some(HeadItem::Word(w)) => out.push(w.clone()),
            Some(HeadItem::Class(c)) => {
                for a in self
                    .tables
                    .syntax_classes
                    .get(c)
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                {
                    self.lead_words(a, out);
                }
            }
            _ => {}
        }
    }

    /// Whether the code after `items[..i]` may start with a block `{`
    /// (`follow`: whether what comes after all of `items` may).
    fn block_follows(&self, items: &[HeadItem], i: usize, follow: bool) -> bool {
        match items.get(i) {
            None => follow,
            Some(HeadItem::Hole(_, HOLE_BLOCK)) => true,
            Some(HeadItem::Word(w)) => w == "{",
            Some(HeadItem::Class(c)) => self
                .tables
                .syntax_classes
                .get(c)
                .is_some_and(|arms| arms.iter().any(|a| self.block_follows(a, 0, false))),
            Some(HeadItem::Optional(inner, _) | HeadItem::Repeated(inner, _)) => {
                self.block_follows(inner, 0, false) || self.block_follows(items, i + 1, follow)
            }
            Some(HeadItem::Hole(..)) => false,
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
        let fitting: Vec<&SeqRule> = rules
            .iter()
            .filter(|r| self.quick_fit(&r.items, after))
            .collect();
        for rule in fitting {
            self.advance();
            let (mut bindings, mut words) = (Vec::new(), 0usize);
            // What the matched code itself reports (an error inside a
            // block hole) stays: only rules that did not match are undone.
            if self.match_items(&word, &rule.items, &mut bindings, &mut words, false, &[]) {
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
            if !((crate::is_word_tok(k) && !self.goes_on(nx))
                || matches!(k, TokenKind::IntLit | TokenKind::StrLit))
            {
                return None;
            }
            self.advance();
            let _ = self.match_items(&word, &rules[0].items, &mut Vec::new(), &mut 0, false, &[]);
            return Some(NodeId::NONE);
        };
        self.pos = reached;
        self.arena.diagnostics.extend(found);
        Some(NodeId::NONE)
    }

    /// Whether the token at `at` starts a name (`$name` inside a template).
    fn name_at(&self, at: usize) -> bool {
        at < self.tape.len()
            && (crate::is_word_tok(self.tape.kind(at))
                || (self.tape.kind(at) == TokenKind::Dollar
                    && at + 1 < self.tape.len()
                    && crate::is_word_tok(self.tape.kind(at + 1))))
    }

    /// Whether `items` fit the tokens from `at`, as far as words, names and
    /// blocks show; an expression, pattern or type hole fits anything.
    fn quick_fit(&self, items: &[HeadItem], mut at: usize) -> bool {
        for item in items {
            at = self.next_non_nl(at);
            let here =
                (at < self.tape.len()).then(|| (self.tape.kind(at), self.tape.text(at, self.src)));
            match item {
                HeadItem::Word(w) if here.is_some_and(|(_, t)| t == w) => at += 1,
                HeadItem::Hole(_, HOLE_IDENT) if self.name_at(at) => {
                    at += if self.tape.kind(at) == TokenKind::Dollar {
                        2
                    } else {
                        1
                    };
                }
                HeadItem::Hole(_, HOLE_BLOCK)
                    if here.is_some_and(|(k, _)| k == TokenKind::LBrace) =>
                {
                    at = self
                        .tape
                        .matching_bracket(at)
                        .map_or(self.tape.len(), |c| c as usize + 1);
                }
                HeadItem::Optional(inner, _) => {
                    if self.quick_fit(&inner[..1], at) {
                        return self.quick_fit(inner, at);
                    }
                }
                HeadItem::Hole(_, HOLE_REST) if self.rest_starts(at) => return true,
                HeadItem::Class(c) => {
                    let arms = self.tables.syntax_classes.get(c);
                    return arms.is_some_and(|arms| {
                        arms.iter()
                            .any(|a| self.quick_fit(&a[..a.len().min(1)], at))
                    });
                }
                HeadItem::Word(_) | HeadItem::Hole(_, HOLE_IDENT | HOLE_BLOCK | HOLE_REST) => {
                    return false
                }
                _ => return true,
            }
        }
        true
    }

    /// Matches `items` at the cursor, binding their holes into `out` and
    /// counting the words matched in `words`; false (with a diagnostic) at
    /// the first token that does not fit. `follow` tells whether the code
    /// after `items` may start with a block; `stops` are the words that end
    /// a `Stmts` hole (those that start the next repetition around it).
    fn match_items(
        &mut self,
        word: &str,
        items: &[HeadItem],
        out: &mut Vec<(String, NodeId)>,
        words: &mut usize,
        follow: bool,
        stops: &[String],
    ) -> bool {
        for (i, item) in items.iter().enumerate() {
            let next_block = self.block_follows(items, i + 1, follow);
            match item {
                HeadItem::Word(w) if self.text() == w => {
                    self.advance();
                    *words += 1;
                }
                HeadItem::Word(w) => {
                    let (sp, found) = (self.span(), self.text().to_string());
                    self.arena.diagnostics.push((
                        sp,
                        format!("SYN010: `{word}` expects `{w}` here, found `{found}`"),
                    ));
                    return false;
                }
                HeadItem::Hole(name, kind) => {
                    let bp = self.hole_power(items, i);
                    let (sp, found) = (self.span(), self.text().to_string());
                    let node = self.parse_hole(word, name, *kind, next_block, bp, stops);
                    if node.is_none() {
                        if !matches!(*kind, HOLE_BLOCK | HOLE_IDENT) {
                            self.arena.diagnostics.push((
                                sp,
                                format!(
                                    "SYN010: `{word}` expects code for `${name}`, found `{found}`"
                                ),
                            ));
                        }
                        return false;
                    }
                    let text = self.source_text(node);
                    let sym = self.arena.interner.intern(&text);
                    let text_node =
                        self.arena
                            .alloc_node(NodeKind::StrLit, 0, self.prev_span(), sym.0, 0);
                    out.push((name.clone(), node));
                    out.push((text_binding(name), text_node));
                }
                // The arm the next token starts; the holes of the others are
                // bound absent.
                HeadItem::Class(c) => {
                    let arms = self
                        .tables
                        .syntax_classes
                        .get(c)
                        .cloned()
                        .unwrap_or_default();
                    let at = self.cur();
                    let Some(k) = arms
                        .iter()
                        .position(|a| self.quick_fit(&a[..a.len().min(1)], at))
                    else {
                        let msg =
                            format!("SYN010: `{word}` expects {c} here, found `{}`", self.text());
                        self.arena.diagnostics.push((self.span(), msg));
                        return false;
                    };
                    if !self.match_items(word, &arms[k], out, words, next_block, stops) {
                        return false;
                    }
                    for (j, arm) in arms.iter().enumerate() {
                        if j != k {
                            self.bind_absent(arm, out);
                        }
                    }
                }
                // An optional group led by a word binds a `bool` named after
                // it: whether it was written. Not written, its holes are
                // their defaults, or absent.
                HeadItem::Optional(inner, defaults) => {
                    let flag_word = match inner.first() {
                        Some(HeadItem::Word(w)) => Some(w.clone()),
                        _ => None,
                    };
                    let written = match &flag_word {
                        Some(w) => self.text() == w,
                        None => self.quick_fit(&inner[..1], self.cur()),
                    };
                    if let Some(w) = &flag_word {
                        let flag = self.arena.alloc_node(
                            NodeKind::BoolLit,
                            0,
                            self.span(),
                            written as u32,
                            0,
                        );
                        out.push((w.clone(), flag));
                    }
                    if written {
                        if !self.match_items(word, inner, out, words, next_block, stops) {
                            return false;
                        }
                        continue;
                    }
                    let span = self.span();
                    for (n, t) in defaults {
                        let node = t.instantiate(&mut self.arena, &[], span);
                        out.push((n.clone(), node));
                    }
                    self.bind_absent(&inner[usize::from(flag_word.is_some())..], out);
                }
                // A group that starts with a word may be written no times; a
                // group with no separator that does not is written until the
                // word after it.
                HeadItem::Repeated(inner, sep) => {
                    let led = self.word_led(inner);
                    let until = match items.get(i + 1) {
                        Some(HeadItem::Word(w)) if sep.is_none() && !led => Some(w.clone()),
                        _ => None,
                    };
                    let mut inner_stops = stops.to_vec();
                    self.lead_words(inner, &mut inner_stops);
                    let inner_block = self.block_follows(inner, 0, false) || next_block;
                    let mut runs: Vec<Vec<(String, NodeId)>> = Vec::new();
                    while !self.at_end() {
                        if until.as_ref().is_some_and(|u| self.text() == u)
                            || (led && !self.quick_fit(&inner[..1], self.cur()))
                        {
                            break;
                        }
                        let (m, run_start, mut one) = (self.pos, self.span().start, Vec::new());
                        if !self.match_items(
                            word,
                            inner,
                            &mut one,
                            words,
                            inner_block,
                            &inner_stops,
                        ) {
                            return false;
                        }
                        let run = ByteSpan::new(run_start, self.prev_span().end.max(run_start));
                        one.push((
                            RUN_SPAN.to_string(),
                            self.arena.alloc_node(NodeKind::NoneLit, 0, run, 0, 0),
                        ));
                        if self.pos == m {
                            let msg =
                                format!("SYN010: `{word}` cannot read `{}` here", self.text());
                            self.arena.diagnostics.push((self.span(), msg));
                            return false;
                        }
                        runs.push(one);
                        if let Some(s) = sep {
                            if self.text() != s {
                                break;
                            }
                            self.advance();
                        } else if !led && until.is_none() {
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
    /// bound it to, `NONE` where a run left it out. A name `out` already
    /// binds is skipped.
    fn bind_runs(
        &mut self,
        inner: &[HeadItem],
        runs: &[Vec<(String, NodeId)>],
        out: &mut Vec<(String, NodeId)>,
    ) {
        let mut names = Vec::new();
        self.hole_names(inner, &mut names);
        let span = self.prev_span();
        for n in names {
            if out.iter().any(|(k, _)| *k == n) {
                continue;
            }
            let ids: Vec<NodeId> = runs
                .iter()
                .map(|r| {
                    r.iter()
                        .find(|(k, _)| *k == n)
                        .map_or(NodeId::NONE, |(_, id)| *id)
                })
                .collect();
            let seq = self
                .arena
                .alloc_list_node(NodeKind::TupleExpr, 0, span, &ids);
            let at: Vec<NodeId> = runs
                .iter()
                .filter_map(|r| r.iter().find(|(k, _)| k == RUN_SPAN).map(|(_, id)| *id))
                .collect();
            let spans = self
                .arena
                .alloc_list_node(NodeKind::TupleExpr, 0, span, &at);
            out.push((run_spans(&n), spans));
            // A name repeated inside each run is a sequence of sequences.
            let mut marks = vec![seq_binding(&n)];
            for r in runs {
                for (k, _) in r {
                    let deeper = seq_binding(k);
                    if k.starts_with('\u{2}')
                        && k.trim_start_matches('\u{2}') == n
                        && !marks.contains(&deeper)
                    {
                        marks.push(deeper);
                    }
                }
            }
            out.extend(marks.into_iter().map(|m| (m, NodeId::NONE)));
            out.push((n, seq));
        }
    }

    /// Binds the holes of `items`, which were not written: a flag false, a
    /// repeated hole an empty sequence, any other hole absent. A name `out`
    /// already binds is skipped.
    fn bind_absent(&mut self, items: &[HeadItem], out: &mut Vec<(String, NodeId)>) {
        for item in items {
            match item {
                HeadItem::Hole(n, _) => {
                    if !out.iter().any(|(k, _)| k == n) {
                        out.push((n.clone(), NodeId::NONE));
                    }
                }
                HeadItem::Optional(inner, _) => {
                    if let Some(HeadItem::Word(w)) = inner.first() {
                        if !out.iter().any(|(k, _)| k == w) {
                            let flag =
                                self.arena
                                    .alloc_node(NodeKind::BoolLit, 0, self.span(), 0, 0);
                            out.push((w.clone(), flag));
                        }
                    }
                    self.bind_absent(inner, out);
                }
                HeadItem::Repeated(inner, _) => self.bind_runs(inner, &[], out),
                HeadItem::Class(c) => {
                    for arm in self
                        .tables
                        .syntax_classes
                        .get(c)
                        .cloned()
                        .unwrap_or_default()
                    {
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
        infix(i.checked_sub(1).and_then(|p| items.get(p)))
            .filter(|_| expr_before)
            .map(|e| e.bp + u8::from(!e.right_assoc))
    }

    fn parse_hole(
        &mut self,
        word: &str,
        name: &str,
        kind: u8,
        before_block: bool,
        bp: Option<u8>,
        stops: &[String],
    ) -> NodeId {
        match kind {
            HOLE_BLOCK if self.at(TokenKind::LBrace) => self.parse_block(),
            // `$name` inside a template: the name a hole of the rule holds.
            HOLE_IDENT if self.name_at(self.cur()) => {
                let sp = self.span();
                let in_template = self.eat(TokenKind::Dollar);
                let sym = self.intern_cur();
                let span = ByteSpan::new(sp.start, self.prev_span().end);
                self.arena
                    .alloc_node(NodeKind::IdentExpr, u16::from(in_template), span, sym.0, 0)
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
                let what = if kind == HOLE_BLOCK {
                    "a block `{ .. }`"
                } else {
                    "a name"
                };
                let msg = format!(
                    "SYN010: `{word}` expects {what} for `${name}`, found `{}`",
                    self.text()
                );
                self.arena.diagnostics.push((self.span(), msg));
                NodeId::NONE
            }
            HOLE_STMT => self.parse_stmt(),
            HOLE_STMTS => self.parse_stmts_hole(stops),
            HOLE_REST if self.rest_starts(self.cur()) => self.parse_rest(),
            _ => NodeId::NONE,
        }
    }

    /// Statements up to `}`, `,` or a word in `stops`: one statement as it
    /// is (an expression statement as its expression), several as a block.
    fn parse_stmts_hole(&mut self, stops: &[String]) -> NodeId {
        let sp = self.span();
        let mut stmts = Vec::new();
        while !self.at_end()
            && !self.at(TokenKind::RBrace)
            && !self.at(TokenKind::Comma)
            && !stops.iter().any(|s| self.text() == s)
        {
            let m = self.pos;
            let s = self.parse_stmt();
            if s.is_some() {
                stmts.push(s);
            }
            if self.pos == m {
                break;
            }
        }
        match stmts.as_slice() {
            [one] if self.arena.node(*one).node_kind() == NodeKind::ExprStmt => {
                NodeId(self.arena.node(*one).lhs)
            }
            [one] => *one,
            _ => self.arena.alloc_list_node(
                NodeKind::BlockExpr,
                0,
                ByteSpan::new(sp.start, self.prev_span().end.max(sp.start)),
                &stmts,
            ),
        }
    }

    /// Whether a `Rest` hole can start at `at`: at a name, a literal, or a
    /// `{` where a block may follow (not after the condition of `if`,
    /// `while` or `switch`, whose block that `{` opens). A word that goes on
    /// with the expression before it (an operator word such as `is`, or a
    /// bound word such as `as`) starts nothing: `c as u32` keeps `c` a name.
    fn rest_starts(&self, at: usize) -> bool {
        let at = self.next_non_nl(at);
        at < self.tape.len()
            && match self.tape.kind(at) {
                TokenKind::LBrace => !self.no_struct_lit,
                k if crate::is_word_tok(k) => !self.goes_on(at),
                k => matches!(
                    k,
                    TokenKind::IntLit
                        | TokenKind::HexIntLit
                        | TokenKind::BinIntLit
                        | TokenKind::OctIntLit
                        | TokenKind::FloatLit
                        | TokenKind::StrLit
                        | TokenKind::TripleStrLit
                ),
            }
    }

    /// Whether the word at `at` goes on with the expression before it: an
    /// operator word (`is`, `as`, `in`) or a word bound to a receptor that
    /// follows an expression (`catch`, the same kinds `parse_stmt` lets
    /// follow a word). A value word (`true`) does not go on.
    fn goes_on(&self, at: usize) -> bool {
        let w = self.tape.text(at, self.src);
        self.tables.infix_ops.contains_key(w)
            || self
                .tables
                .receptors
                .get(w)
                .is_some_and(|&(rk, _)| matches!(rk, 19 | 29 | 31))
    }

    /// A `Rest` hole: the code up to a `;` (not read), a closer it did not
    /// open, or through the `}` that closes a block it opened, read past
    /// and kept only as its span, for a rule that reports the construct.
    fn parse_rest(&mut self) -> NodeId {
        let sp = self.span();
        let mut depth = 0usize;
        while !self.at_end() {
            match self.peek() {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace if depth == 0 => break,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 && self.peek() == TokenKind::RBrace {
                        self.advance();
                        break;
                    }
                }
                TokenKind::Semicolon if depth == 0 => break,
                _ => {}
            }
            self.advance();
        }
        let span = ByteSpan::new(sp.start, self.prev_span().end.max(sp.start));
        self.arena.alloc_node(NodeKind::ErrorNode, 0, span, 0, 0)
    }

    /// The hole names in `items`, nested groups and classes included; the
    /// arms of a class may share names.
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
                    let mut seen: Vec<String> = Vec::new();
                    for arm in self
                        .tables
                        .syntax_classes
                        .get(c)
                        .map(Vec::as_slice)
                        .unwrap_or_default()
                    {
                        let mut a = Vec::new();
                        self.hole_names(arm, &mut a);
                        for n in a {
                            if !seen.contains(&n) {
                                seen.push(n);
                            }
                        }
                    }
                    out.extend(seen);
                }
                HeadItem::Word(_) => {}
            }
        }
    }
}
