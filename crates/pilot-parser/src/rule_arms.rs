//! Expression `syntax` rules with several arms, default arguments and the
//! source text of an argument.
//!
//! A plain rule (`spawn(body) => quote { .. };`) binds each argument to one
//! name, as before. A rule head may also:
//!
//! * give a parameter a default (`assert(cond, note = "")`), so a call may
//!   leave the trailing arguments out;
//! * write its first parameter as `left OP right` with any binary operator
//!   (`assert(left == right, note = "")`): that arm is taken only when the
//!   first argument is that operator, and binds its two sides.
//!
//! Several arms for one word are tried in the order written; the first whose
//! shape and argument count fit is expanded. Inside a template,
//! `stringify($p)` (the word the library binds to receptor
//! [`SOURCE_TEXT_RECEPTOR`]) is the source text of the argument bound to
//! `p`, as a string literal. The parser names no rule and no word here.

use crate::{Parser, TemplateNode};
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_lexer::TokenKind;
use pilot_span::ByteSpan;

/// The receptor of the template word that gives an argument's source text.
const SOURCE_TEXT_RECEPTOR: (u8, u8) = (36, 4);

/// The binding name under which a parameter's source text is kept. It
/// starts with a byte no identifier holds, so it cannot meet a user name.
fn text_binding(param: &str) -> String {
    format!("\u{1}{param}")
}

/// One arm of an expression rule.
#[derive(Clone, Debug)]
pub struct RuleArm {
    /// The names the arm binds, in order; with a shape, the first two are
    /// the operator's left and right sides.
    pub params: Vec<String>,
    /// The default of each parameter, if it has one.
    pub defaults: Vec<Option<TemplateNode>>,
    /// The binary operator the first argument must be, if any.
    pub shape: Option<u16>,
    pub template: TemplateNode,
}

impl RuleArm {
    /// How many arguments a call passes at most (a shape takes one).
    fn max_args(&self) -> usize {
        self.params.len() - usize::from(self.shape.is_some())
    }

    /// How many arguments a call must pass.
    fn min_args(&self) -> usize {
        let skip = if self.shape.is_some() { 2 } else { 0 };
        let trailing_defaults = self.defaults[skip..].iter().rev().take_while(|d| d.is_some()).count();
        self.max_args() - trailing_defaults
    }
}

/// An expression rule: one plain arm (`params`, `template`), or `arms`.
#[derive(Clone, Debug)]
pub struct SyntaxMacroRule {
    pub params: Vec<String>,
    pub template: TemplateNode,
    pub arms: Vec<RuleArm>,
}

/// A parsed rule head.
pub(crate) struct RuleHead {
    pub nodes: Vec<NodeId>,
    pub arm: RuleArm,
}

impl<'a> Parser<'a> {
    /// Whether the rule head at `(` uses a default or an operator pattern,
    /// so it needs `parse_rule_head` rather than a plain parameter list.
    pub(crate) fn rule_head_is_rich(&self) -> bool {
        if !self.at(TokenKind::LParen) {
            return false;
        }
        let Some(close) = self.tape.matching_bracket(self.cur()) else {
            return false;
        };
        let mut depth = 0i32;
        for i in self.cur() + 1..close as usize {
            match self.tape.kind(i) {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => depth -= 1,
                TokenKind::Comma | TokenKind::Ident | TokenKind::Newline => {}
                _ if depth == 0 => return true,
                _ => {}
            }
        }
        false
    }

    /// `(p, q = default, ..)` or `(left OP right, ..)`.
    pub(crate) fn parse_rule_head(&mut self) -> RuleHead {
        let mut arm = RuleArm { params: Vec::new(), defaults: Vec::new(), shape: None, template: TemplateNode::NoneLit };
        let mut nodes = Vec::new();
        self.expect(TokenKind::LParen);
        while !self.at_end() && !self.at(TokenKind::RParen) {
            let sp = self.span();
            let nx = self.next_non_nl(self.cur() + 1);
            let plain = self.is_word()
                && nx < self.tape.len()
                && matches!(self.tape.kind(nx), TokenKind::Comma | TokenKind::RParen | TokenKind::Equal);
            if plain {
                let sym = self.intern_cur();
                let default = if self.eat(TokenKind::Equal) {
                    let d = self.parse_expr();
                    Some(TemplateNode::from_arena(&self.arena, d))
                } else {
                    None
                };
                nodes.push(self.rule_param_node(sym, sp));
                arm.params.push(self.arena.interner.resolve(sym).to_string());
                arm.defaults.push(default);
            } else if arm.params.is_empty() {
                let e = self.parse_expr();
                let n = if e.is_some() { Some(*self.arena.node(e)) } else { None };
                let sides = n.filter(|n| n.node_kind() == NodeKind::BinaryExpr).and_then(|n| {
                    let side = |id: u32| {
                        let s = *self.arena.node(NodeId(id));
                        (s.node_kind() == NodeKind::IdentExpr).then_some(SymbolId(s.lhs))
                    };
                    Some((n.flags, side(n.lhs)?, side(n.rhs)?))
                });
                match sides {
                    Some((op, l, r)) => {
                        arm.shape = Some(op);
                        for sym in [l, r] {
                            nodes.push(self.rule_param_node(sym, sp));
                            arm.params.push(self.arena.interner.resolve(sym).to_string());
                            arm.defaults.push(None);
                        }
                    }
                    None => self.rule_head_error(sp),
                }
            } else {
                self.rule_head_error(sp);
                while !self.at_end() && !self.at(TokenKind::Comma) && !self.at(TokenKind::RParen) {
                    self.advance();
                }
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen);
        RuleHead { nodes, arm }
    }

    fn rule_param_node(&mut self, sym: SymbolId, sp: ByteSpan) -> NodeId {
        let span = ByteSpan::new(sp.start, self.prev_span().end.max(sp.end));
        self.arena.alloc_node(NodeKind::ParamDecl, 0, span, sym.0, NodeId::NONE.0)
    }

    fn rule_head_error(&mut self, sp: ByteSpan) {
        self.arena.diagnostics.push((
            sp,
            "a rule parameter is a name or `name = default`; only the first may be `left OP right`".to_string(),
        ));
    }

    /// Records `arm` (its template not yet set) for `word` with `template`.
    /// A rich arm joins the word's other rich arms; a plain rule replaces
    /// whatever the word had, as before. A template holding a construct
    /// templates cannot carry is reported at `body`, not dropped.
    pub(crate) fn add_rule(&mut self, word: &str, mut arm: RuleArm, template: TemplateNode, rich: bool, body: ByteSpan) {
        if let Some(kind) = first_unsupported(&template) {
            self.arena.diagnostics.push((
                body,
                format!("the template of `{word}` holds a node of kind `{kind}`, which a rule template cannot carry yet"),
            ));
        }
        let template = hygienic(self.with_source_text(template), &arm.params);
        let tables = self.tables.to_mut();
        if !rich {
            let rule = SyntaxMacroRule { params: arm.params, template, arms: Vec::new() };
            tables.syntax_macros.insert(word.to_string(), rule);
            return;
        }
        arm.template = template;
        let entry = tables.syntax_macros.entry(word.to_string()).or_insert_with(|| SyntaxMacroRule {
            params: Vec::new(),
            template: TemplateNode::NoneLit,
            arms: Vec::new(),
        });
        if entry.arms.is_empty() {
            entry.params.clear();
            entry.template = TemplateNode::NoneLit;
        }
        entry.arms.push(arm);
    }

    /// `template` with each `stringify($p)` replaced by the source text of
    /// the argument bound to `p`.
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

    /// Expands a call of a rule with arms: parses the arguments, takes the
    /// first arm that fits them, and instantiates its template.
    pub(crate) fn expand_rule_arms(&mut self, rule: &SyntaxMacroRule) -> NodeId {
        let sp = self.span();
        let word = self.text().to_string();
        self.advance();
        if !self.at(TokenKind::LParen) {
            self.arena.diagnostics.push((sp, format!("`{word}` takes its arguments in parentheses: `{word}(..)`")));
            return NodeId::NONE;
        }
        let mut args = Vec::new();
        if self.eat(TokenKind::LParen) {
            while !self.at_end() && !self.at(TokenKind::RParen) {
                args.push(self.parse_expr());
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RParen);
        }
        let _ = self.eat(TokenKind::Semicolon);
        let span = ByteSpan::new(sp.start, self.prev_span().end.max(sp.end));
        let Some(arm) = rule.arms.iter().find(|a| self.arm_fits(a, &args)) else {
            self.arena.diagnostics.push((span, format!("no form of `{word}` takes these {} arguments", args.len())));
            return NodeId::NONE;
        };
        let mut bound: Vec<NodeId> = Vec::new();
        if arm.shape.is_some() {
            let n = *self.arena.node(args[0]);
            bound.push(NodeId(n.lhs));
            bound.push(NodeId(n.rhs));
            bound.extend(args.iter().skip(1));
        } else {
            bound.extend(args.iter());
        }
        let mut bindings = Vec::new();
        for (i, name) in arm.params.iter().enumerate() {
            let (node, text) = match bound.get(i) {
                Some(&id) => (id, self.source_text(id)),
                None => {
                    let default = arm.defaults[i].clone().unwrap_or(TemplateNode::NoneLit);
                    (default.instantiate(&mut self.arena, &[], span), String::new())
                }
            };
            let sym = self.arena.interner.intern(&text);
            let text_node = self.arena.alloc_node(NodeKind::StrLit, 0, span, sym.0, 0);
            bindings.push((name.clone(), node));
            bindings.push((text_binding(name), text_node));
        }
        arm.template.instantiate(&mut self.arena, &bindings, span)
    }

    fn arm_fits(&self, arm: &RuleArm, args: &[NodeId]) -> bool {
        if args.len() < arm.min_args() || args.len() > arm.max_args() {
            return false;
        }
        match arm.shape {
            None => true,
            Some(op) => args.first().is_some_and(|&a| {
                a.is_some() && {
                    let n = self.arena.node(a);
                    n.node_kind() == NodeKind::BinaryExpr && n.flags == op
                }
            }),
        }
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
    /// `$( .. )?`: the items, or nothing; the group starts with a word.
    Optional(Vec<HeadItem>),
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

    /// Whether the rule head after the rule's word is a sequence (it does
    /// not open a parameter list).
    pub(crate) fn rule_head_is_seq(&self) -> bool {
        !self.at(TokenKind::LParen) && !self.at(TokenKind::FatArrow)
    }

    /// The items of a sequence head, up to `=>` (or the `)` closing a group).
    pub(crate) fn parse_seq_head(&mut self, end: Option<usize>) -> Vec<HeadItem> {
        let mut items = Vec::new();
        while !self.at_end() && end.map_or(!self.at(TokenKind::FatArrow), |e| self.cur() < e) {
            let sp = self.span();
            if !self.eat(TokenKind::Dollar) {
                items.push(HeadItem::Word(self.text().to_string()));
                self.advance();
                continue;
            }
            if self.at(TokenKind::LParen) {
                let close = self.tape.matching_bracket(self.cur()).map_or(self.tape.len(), |c| c as usize);
                self.advance();
                let inner = self.parse_seq_head(Some(close));
                self.expect(TokenKind::RParen);
                let starts_with_word = matches!(inner.first(), Some(HeadItem::Word(_)));
                if self.text() == "?" {
                    self.advance();
                    if !starts_with_word {
                        self.seq_head_error(sp, "an optional group `$( .. )?` starts with a word");
                    }
                    items.push(HeadItem::Optional(inner));
                } else {
                    let sep = (self.text() != "*").then(|| self.text().to_string());
                    if sep.is_some() {
                        self.advance();
                    }
                    if self.text() != "*" {
                        self.seq_head_error(sp, "a group is `$( .. )?` or `$( .. ) sep *`");
                    }
                    self.advance();
                    if sep.is_none() && !starts_with_word {
                        self.seq_head_error(sp, "a repeated group with no separator starts with a word");
                    }
                    items.push(HeadItem::Repeated(inner, sep));
                }
                continue;
            }
            let name = self.text().to_string();
            self.advance();
            self.expect(TokenKind::Colon);
            let kind_name = self.text().to_string();
            self.advance();
            match self.tables.hole_kind(&kind_name) {
                Some(k) if k <= HOLE_STMT => items.push(HeadItem::Hole(name, k)),
                _ => self.seq_head_error(sp, &format!("`{kind_name}` is not a hole kind (a variant of `HoleKind`)")),
            }
        }
        items
    }

    fn seq_head_error(&mut self, sp: ByteSpan, what: &str) {
        self.arena.diagnostics.push((sp, format!("SYN010: {what}")));
    }

    /// Records a sequence rule for `word`.
    pub(crate) fn add_seq_rule(&mut self, word: &str, items: Vec<HeadItem>, template: TemplateNode, body: ByteSpan) {
        if let Some(kind) = first_unsupported(&template) {
            self.arena.diagnostics.push((
                body,
                format!("the template of `{word}` holds a node of kind `{kind}`, which a rule template cannot carry yet"),
            ));
        }
        let mut names = Vec::new();
        hole_names(&items, &mut names);
        let template = hygienic(template, &names);
        self.tables.to_mut().seq_rules.entry(word.to_string()).or_default().push(SeqRule { items, template });
    }

    /// The sequence rule the code at the cursor (a rule's word) takes: the
    /// first, in the order written, whose leading words, names and blocks
    /// fit the tokens that follow.
    pub(crate) fn seq_rule_at(&self) -> Option<SeqRule> {
        if !self.is_word() || self.file_declares_fn(self.text()) {
            return None;
        }
        let rules = self.tables.seq_rules.get(self.text())?;
        rules.iter().find(|r| self.quick_fit(&r.items, self.cur() + 1)).cloned()
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
                HeadItem::Optional(inner) => {
                    if self.quick_fit(&inner[..1], at) {
                        return self.quick_fit(inner, at);
                    }
                }
                HeadItem::Word(_) | HeadItem::Hole(_, HOLE_IDENT) | HeadItem::Hole(_, HOLE_BLOCK) => return false,
                _ => return true,
            }
        }
        true
    }

    /// Expands the sequence rule `seq_rule_at` gives, at its word.
    pub(crate) fn expand_seq_rule(&mut self) -> NodeId {
        let Some(rule) = self.seq_rule_at() else { return NodeId::NONE };
        let sp = self.span();
        let word = self.text().to_string();
        self.advance();
        let mut bindings = Vec::new();
        if !self.match_items(&word, &rule.items, &mut bindings) {
            return NodeId::NONE;
        }
        let span = ByteSpan::new(sp.start, self.prev_span().end.max(sp.end));
        rule.template.instantiate(&mut self.arena, &bindings, span)
    }

    /// Matches `items` at the cursor, binding their holes into `out`; false
    /// (with a diagnostic) at the first token that does not fit.
    fn match_items(&mut self, word: &str, items: &[HeadItem], out: &mut Vec<(String, NodeId)>) -> bool {
        for (i, item) in items.iter().enumerate() {
            let next_is_block = matches!(items.get(i + 1), Some(HeadItem::Hole(_, HOLE_BLOCK)));
            match item {
                HeadItem::Word(w) if self.text() == w => self.advance(),
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
                    let node = self.parse_hole(word, name, *kind, next_is_block);
                    out.push((name.clone(), node));
                }
                // An optional group binds a `bool` named after its first word:
                // whether it was written.
                HeadItem::Optional(inner) => {
                    let Some(HeadItem::Word(w)) = inner.first() else { continue };
                    let written = self.text() == w;
                    let flag = self.arena.alloc_node(NodeKind::BoolLit, 0, self.span(), written as u32, 0);
                    out.push((w.clone(), flag));
                    if written {
                        if !self.match_items(word, inner, out) {
                            return false;
                        }
                    } else {
                        let mut names = Vec::new();
                        hole_names(inner, &mut names);
                        out.extend(names.into_iter().map(|n| (n, NodeId::NONE)));
                    }
                }
                // A group that starts with a word may be written no times.
                HeadItem::Repeated(inner, sep) => {
                    let lead = match inner.first() {
                        Some(HeadItem::Word(w)) => Some(w.clone()),
                        _ => None,
                    };
                    let mut runs: Vec<Vec<(String, NodeId)>> = Vec::new();
                    while !self.at_end() && lead.as_ref().is_none_or(|w| self.text() == w) {
                        let mut one = Vec::new();
                        if !self.match_items(word, inner, &mut one) {
                            return false;
                        }
                        runs.push(one);
                        if let Some(s) = sep {
                            if self.text() != s {
                                break;
                            }
                            self.advance();
                        } else if lead.is_none() {
                            break;
                        }
                    }
                    let mut names = Vec::new();
                    hole_names(inner, &mut names);
                    let span = self.prev_span();
                    for n in names {
                        let ids: Vec<NodeId> = runs.iter().filter_map(|r| r.iter().find(|(k, _)| *k == n).map(|(_, id)| *id)).collect();
                        let seq = self.arena.alloc_list_node(NodeKind::TupleExpr, 0, span, &ids);
                        out.push((seq_binding(&n), NodeId::NONE));
                        out.push((n, seq));
                    }
                }
            }
        }
        true
    }

    fn parse_hole(&mut self, word: &str, name: &str, kind: u8, before_block: bool) -> NodeId {
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
                let e = self.parse_expr();
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
}

/// The hole names in `items`, nested groups included.
fn hole_names(items: &[HeadItem], out: &mut Vec<String>) {
    for item in items {
        match item {
            HeadItem::Hole(n, _) => out.push(n.clone()),
            HeadItem::Optional(inner) => {
                if let Some(HeadItem::Word(w)) = inner.first() {
                    out.push(w.clone());
                }
                hole_names(inner, out);
            }
            HeadItem::Repeated(inner, _) => hole_names(inner, out),
            HeadItem::Word(_) => {}
        }
    }
}
