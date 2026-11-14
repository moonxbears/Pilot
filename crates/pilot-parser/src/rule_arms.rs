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
use pilot_ast::{NodeId, NodeKind};
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
        if let TemplateNode::Let(name, _, _) = n {
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
        T::Binary(_, l, r) | T::Index(l, r) | T::Assign(_, l, r) => {
            walk(l, f);
            walk(r, f);
        }
        T::Loop(x) | T::Unary(_, x) | T::Member(x, _) | T::Defer(_, x) => walk(x, f),
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
        T::Index(x, i) => {
            let x = Box::new(f(*x));
            T::Index(x, Box::new(f(*i)))
        }
        other => other,
    }
}
