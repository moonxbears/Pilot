//! Bootstrap receptor, modifier, operator, and `type_construct` tables (`pilot-parser`).

use crate::*;

#[derive(Clone, Debug)]
pub enum TypePostfixKind {
    BracketEmpty,
    BracketDotDot,
    BracketColon,
    BracketArgs(usize),
    Sym(String),
}
#[derive(Clone, Debug)]
pub enum TypeConstructRule {
    Prefix {
        op: String,
        param: String,
        template: TemplateNode,
    },
    Postfix {
        kind: TypePostfixKind,
        params: Vec<String>,
        template: TemplateNode,
    },
    /// `T1 | T2`; also `T @ N`, a declared postfix symbol with an argument.
    Infix {
        op: String,
        lhs_param: String,
        rhs_param: String,
        template: TemplateNode,
    },
    NamedGeneric {
        name: String,
        params: Vec<String>,
        template: TemplateNode,
    },
}
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum ModAct {
    Vis(u8),
    Partial,
    Static,
    Pure,
    Mut,
    Const,
    Iso,
    Ext,
    NoAlloc,
    Override,
    Unbuilt,
}
/// An infix operator: its binding power, associativity, native `BinaryOp`
/// code (until `PROP-022` §3 lowers operators to their owners) and parse
/// shape (an ordinal of the shape enum, `SHAPE_*`).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PrattEntry {
    pub bp: u8,
    pub right_assoc: bool,
    pub bin_op: u16,
    pub shape: u8,
}

/// One operator declared by a grammar attribute (`grammar_entry`, PROP-008):
/// its symbol, readable word (or empty), and the variant names of its shape,
/// level and associativity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GrammarEntry {
    pub(crate) symbol: String,
    pub(crate) shape: String,
    pub(crate) level: String,
    pub(crate) assoc: String,
    pub(crate) word: String,
    /// Declared for a type-position rule: its level is a type level (PROP-019).
    pub(crate) for_type: bool,
}

/// The variant names, in declaration order, of the three enums named by
/// `register_grammar`: levels (loosest first), associativities and shapes.
/// The seed reads only their positions, never their names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct GrammarEnums {
    pub(crate) levels: Vec<String>,
    pub(crate) assocs: Vec<String>,
    pub(crate) shapes: Vec<String>,
    /// The type-position levels, loosest first (PROP-019).
    pub(crate) type_levels: Vec<String>,
}

pub(crate) const HOOK_GRAMMAR_ENTRY: &str = "grammar_entry";
pub(crate) const HOOK_REGISTER_GRAMMAR: &str = "register_grammar";
pub(crate) const HOOK_REGISTER_MACHINE_OPS: &str = "register_machine_ops";
pub(crate) const HOOK_REGISTER_HOLE_KINDS: &str = "register_hole_kinds";
pub(crate) const HOOK_REGISTER_CORE_FORMS: &str = "register_core_forms";
/// The receptor kind of the `type_construct` word.
pub(crate) const RECEPTOR_TYPE_CONSTRUCT: u8 = 5;

/// Parse shapes by position in the shape enum (PROP-008 §4; closed in the
/// seed, append-only under B+).
pub(crate) const SHAPE_PREFIX: usize = 1;
pub(crate) const SHAPE_POSTFIX: usize = 2;
pub(crate) const SHAPE_TYPE_RIGHT: usize = 3;
pub(crate) const SHAPE_MEMBERSHIP: usize = 5;
pub(crate) const SHAPE_CASCADE: usize = 8;
pub(crate) const SHAPE_ADJACENT_CALL: usize = 9;
/// Associativities by position in the associativity enum.
pub(crate) const ASSOC_RIGHT: usize = 1;

/// The binding power of the loosest level; each tighter level adds
/// `LEVEL_STEP`. Powers below `LEVEL_BASE` belong to statement-like clauses.
pub(crate) const LEVEL_BASE: u8 = 10;
pub(crate) const LEVEL_STEP: u8 = 4;
/// `expr catch e => ..` and `try expr`: looser than every operator except
/// the loosest level, whose right side may hold them (`x := f() catch ..`).
pub(crate) const CLAUSE_BP: u8 = LEVEL_BASE + 1;
#[derive(Clone, Debug)]
pub(crate) enum CVal {
    Str(String),
    Int(i64),
    Map(HashMap<String, CVal>),
}
#[derive(Clone, Debug, Default)]
pub struct ParserTables {
    pub(crate) modifiers: HashMap<String, ModAct>,
    pub(crate) param_modifiers: HashSet<String>,
    pub(crate) module_kws: HashMap<String, bool>,
    pub receptors: HashMap<String, (u8, u8)>,
    pub receiver_ident: String,
    /// `field` and `value` inside accessor bodies (`bind_receiver_ident`).
    pub backing_ident: String,
    pub setter_value_ident: String,
    pub prefix_ops: HashMap<String, u8>,
    pub infix_ops: HashMap<String, PrattEntry>,
    pub postfix_ops: HashMap<String, u8>,
    /// Readable prefix words (`not`) and the symbol each pairs with (`!`).
    pub(crate) prefix_words: HashMap<String, String>,
    /// The operators declared by grammar attributes, in declaration order.
    pub(crate) grammar: Vec<GrammarEntry>,
    pub(crate) grammar_enums: GrammarEnums,
    /// The variant names of every enum ingested, by enum name.
    pub(crate) adt_variants: HashMap<String, Vec<String>>,
    /// The enum whose variants are the machine operations of `asm(..)`
    /// (PROP-009 step 1b).
    pub(crate) machine_ops: String,
    /// The enum whose variants are the hole kinds of a `syntax` rule's
    /// sequence head (PROP-009 step 1a).
    pub(crate) hole_kinds: String,
    /// The enum whose variants are the nodes a rule template builds with
    /// `form(..)`, and the word that builder is spelled with (PROP-009 step 2).
    pub(crate) core_forms: (String, String),
    /// `syntax` rules with a sequence head, by trigger word, in the order
    /// written (`rule_arms::SeqRule`).
    pub(crate) seq_rules: HashMap<String, Vec<crate::rule_arms::SeqRule>>,
    /// Syntax classes (`syntax Name { head; .. }`): each class's arms.
    pub(crate) syntax_classes: HashMap<String, Vec<Vec<crate::rule_arms::HeadItem>>>,
    pub type_rules: Vec<TypeConstructRule>,
    /// Type-position symbols by shape, with their power (and, for infix,
    /// right associativity), from the declared type levels (PROP-019).
    pub(crate) type_prefix: HashMap<String, u8>,
    pub(crate) type_infix: HashMap<String, (u8, bool)>,
    pub(crate) type_postfix: HashMap<String, u8>,
    pub(crate) const_maps: HashMap<String, CVal>,
    /// Attribute functions by name (see `attr_hooks`).
    pub(crate) attr_hooks: HashMap<String, attr_hooks::AttrHook>,
    /// Register layouts declared with `[Bits(..)]`, by type name.
    pub(crate) reg_layouts: HashMap<String, RegLayout>,
    /// The Pilot functions that value dimensional literals (see `pilot_impls`).
    pub(crate) unit_literal: Option<UnitLiteralHook>,
    /// The Pilot function each infix operator calls, by symbol (see `pilot_impls`).
    pub(crate) operator_fns: HashMap<String, OperatorFn>,
    /// Names bound to engine intrinsics, by kind (see `pilot_impls`).
    pub(crate) natives: HashMap<(NativeKind, String), String>,
    pub(crate) enumerator: Option<EnumeratorProtocol>,
    pub(crate) generator: Option<GeneratorProtocol>,
    pub(crate) try_protocol: Option<TryProtocol>,
    /// The Pilot function an f-string hole's `:pattern` calls (see `fstring`).
    pub(crate) format_spec: Option<String>,
    /// The function `x in xs` calls, as `f(x, xs)`, when one is bound.
    pub(crate) membership: Option<String>,
    pub(crate) membership_range: (Option<String>, Option<String>),
    pub(crate) native_members: HashMap<(String, String), String>,
    pub(crate) scalar_text: Option<String>,
}
impl ParserTables {
    pub fn new() -> Self {
        Self::default()
    }

    /// Empty tables holding only the operator grammar of `t`: the start of
    /// the bootstrap's next pass, so files read before the operator
    /// declarations already see every operator.
    pub(crate) fn with_grammar(t: &ParserTables) -> Self {
        let mut out = Self {
            grammar: t.grammar.clone(),
            grammar_enums: t.grammar_enums.clone(),
            ..Self::default()
        };
        out.rebuild_grammar();
        out
    }

    /// `register_grammar(levels, assocs, shapes)`: the enums whose variant
    /// order places every operator. Naming the `type_construct` word as
    /// `shapes` registers the type-position levels instead (PROP-019).
    fn register_grammar(&mut self, levels: &str, assocs: &str, shapes: &str) {
        let get = |n: &str| self.adt_variants.get(n).cloned().unwrap_or_default();
        if self.is_type_construct_word(shapes) {
            self.grammar_enums.type_levels = get(levels);
        } else {
            self.grammar_enums.levels = get(levels);
            self.grammar_enums.assocs = get(assocs);
            self.grammar_enums.shapes = get(shapes);
        }
        self.rebuild_grammar();
    }

    fn is_type_construct_word(&self, w: &str) -> bool {
        self.receptors.get(w).map(|r| r.0) == Some(RECEPTOR_TYPE_CONSTRUCT)
    }

    /// The power of the tightest type level: postfix forms bind at it.
    pub(crate) fn type_tight_bp(&self) -> u8 {
        Self::level_bp(self.grammar_enums.type_levels.len().saturating_sub(1))
    }

    /// The operator maps, rebuilt from the declared entries.
    fn rebuild_grammar(&mut self) {
        self.infix_ops.clear();
        self.prefix_ops.clear();
        self.postfix_ops.clear();
        self.prefix_words.clear();
        self.type_prefix.clear();
        self.type_infix.clear();
        self.type_postfix.clear();
        for e in self.grammar.clone() {
            self.apply_grammar_entry(&e);
        }
    }

    /// The binding power of level ordinal `level`.
    pub(crate) fn level_bp(level: usize) -> u8 {
        LEVEL_BASE.saturating_add(LEVEL_STEP.saturating_mul(level.min(60) as u8))
    }

    /// Adds `e` to the operator maps. An entry whose level or shape is not a
    /// variant of the registered enums, or whose symbol is structural syntax
    /// or a seed word (GRAM005, reported by `selfhost/check/grammar_attrs.pilot`),
    /// adds nothing.
    fn apply_grammar_entry(&mut self, e: &GrammarEntry) {
        let pos = |names: &[String], n: &str| names.iter().position(|x| x == n);
        let levels = match e.for_type {
            true => &self.grammar_enums.type_levels,
            false => &self.grammar_enums.levels,
        };
        let Some(level) = pos(levels, &e.level) else {
            return;
        };
        let Some(shape) = pos(&self.grammar_enums.shapes, &e.shape) else {
            return;
        };
        if e.for_type {
            let (s, bp) = (e.symbol.clone(), Self::level_bp(level));
            let right = pos(&self.grammar_enums.assocs, &e.assoc) == Some(ASSOC_RIGHT);
            match shape {
                SHAPE_PREFIX => self.type_prefix.insert(s, bp),
                SHAPE_POSTFIX => self.type_postfix.insert(s, bp),
                _ => self.type_infix.insert(s, (bp, right)).map(|x| x.0),
            };
            return;
        }
        if is_structural_symbol(&e.symbol) || (!e.word.is_empty() && is_structural_symbol(&e.word))
        {
            return;
        }
        let bp = Self::level_bp(level);
        match shape {
            SHAPE_PREFIX => {
                self.prefix_ops.insert(e.symbol.clone(), bp);
                if !e.word.is_empty() {
                    self.prefix_ops.insert(e.word.clone(), bp);
                    self.prefix_words.insert(e.word.clone(), e.symbol.clone());
                }
            }
            SHAPE_POSTFIX => {
                self.postfix_ops.insert(e.symbol.clone(), bp);
            }
            SHAPE_CASCADE | SHAPE_ADJACENT_CALL => {}
            _ => {
                let right_assoc = pos(&self.grammar_enums.assocs, &e.assoc) == Some(ASSOC_RIGHT);
                let entry = PrattEntry {
                    bp,
                    right_assoc,
                    bin_op: binop_code(&e.symbol),
                    shape: shape as u8,
                };
                self.infix_ops.insert(e.symbol.clone(), entry);
                if !e.word.is_empty() {
                    self.infix_ops.insert(e.word.clone(), entry);
                }
            }
        }
    }

    /// The binding power of the infix operator whose native code is `op`;
    /// `fallback` when none is declared.
    pub(crate) fn bp_of(&self, op: BinaryOp, fallback: u8) -> u8 {
        self.infix_ops
            .values()
            .filter(|e| e.bin_op == op as u16)
            .map(|e| e.bp)
            .max()
            .unwrap_or(fallback)
    }

    /// The binding power of the prefix operator whose symbol lexes as `k`.
    pub(crate) fn prefix_bp_of_kind(&self, k: TokenKind) -> Option<u8> {
        self.prefix_ops
            .iter()
            .filter(|(s, _)| first_token_kind(s) == Some(k))
            .map(|(_, &bp)| bp)
            .max()
    }
    pub(crate) fn eval_cval(&self, arena: &AstArena, id: NodeId) -> Option<CVal> {
        if id.is_none() {
            return None;
        }
        let n = arena.node(id);
        match n.node_kind() {
            NodeKind::StrLit => Some(CVal::Str(
                arena.interner.resolve(SymbolId(n.lhs)).to_string(),
            )),
            NodeKind::IntLit => Some(CVal::Int(
                arena.i64_pool.get(n.lhs as usize).copied().unwrap_or(0),
            )),
            NodeKind::IdentExpr => {
                let s = arena.interner.resolve(SymbolId(n.lhs));
                self.const_maps
                    .get(s)
                    .cloned()
                    .or_else(|| Some(CVal::Str(s.to_string())))
            }
            // A member of a named constant is looked up in place: copying the
            // maps on the way made ingesting a file quadratic in its size.
            NodeKind::MemberExpr => match self.cval_ref(arena, id) {
                Some(v) => Some(v.clone()),
                None => {
                    if let Some(CVal::Map(m)) = self.eval_cval(arena, NodeId(n.lhs)) {
                        m.get(arena.interner.resolve(SymbolId(n.rhs))).cloned()
                    } else {
                        None
                    }
                }
            },
            NodeKind::CallExpr => {
                let ch = arena.children_slice(n.lhs, n.rhs);
                ch.get(1).and_then(|&x| self.eval_cval(arena, NodeId(x)))
            }
            NodeKind::MapExpr => {
                let (ch, mut m, mut i) = (arena.children_slice(n.lhs, n.rhs), HashMap::new(), 0);
                while i + 1 < ch.len() {
                    let kn = arena.node(NodeId(ch[i]));
                    if matches!(kn.node_kind(), NodeKind::IdentExpr | NodeKind::StrLit) {
                        if let Some(v) = self.eval_cval(arena, NodeId(ch[i + 1])) {
                            m.insert(arena.interner.resolve(SymbolId(kn.lhs)).to_string(), v);
                        }
                    }
                    i += 2;
                }
                Some(CVal::Map(m))
            }
            _ => None,
        }
    }
    /// The constant a chain of names (`a.b.c`) refers to, found without
    /// copying the maps on the way. `None` sends `eval_cval` the long way.
    pub(crate) fn cval_ref(&self, arena: &AstArena, id: NodeId) -> Option<&CVal> {
        let n = arena.node(id);
        match n.node_kind() {
            NodeKind::IdentExpr => self.const_maps.get(arena.interner.resolve(SymbolId(n.lhs))),
            NodeKind::MemberExpr => match self.cval_ref(arena, NodeId(n.lhs))? {
                CVal::Map(m) => m.get(arena.interner.resolve(SymbolId(n.rhs))),
                _ => None,
            },
            _ => None,
        }
    }
    pub(crate) fn ingest_node(&mut self, arena: &AstArena, id: NodeId) {
        if id.is_none() {
            return;
        }
        let n = *arena.node(id);
        match n.node_kind() {
            NodeKind::Program | NodeKind::BlockExpr => {
                for &c in arena.children_slice(n.lhs, n.rhs).to_vec().iter() {
                    self.ingest_node(arena, NodeId(c));
                }
            }
            NodeKind::ComptimeExpr | NodeKind::ExprStmt => self.ingest_node(arena, NodeId(n.lhs)),
            // A function, or a `syntax` rule (laid out as one): its attributes
            // too, so `[InfixWord(..)]` on a rule declares its operator.
            NodeKind::FnDecl => {
                self.ingest_attr_fn(arena, id);
                self.ingest_decl_attrs(arena, id);
            }
            // A type, and the attributes on its members: `[Infix(..)]` on an
            // `op` declares that operator (PROP-008).
            NodeKind::StructDecl | NodeKind::InterfaceDecl => {
                self.ingest_decl_attrs(arena, id);
                let body = NodeId(n.rhs);
                if body.is_some() && arena.node(body).node_kind() == NodeKind::BlockExpr {
                    let b = *arena.node(body);
                    for &m in arena.children_slice(b.lhs, b.rhs).to_vec().iter() {
                        let mk = arena.node(NodeId(m)).node_kind();
                        if matches!(mk, NodeKind::OperatorDecl | NodeKind::FnDecl) {
                            self.ingest_decl_attrs(arena, NodeId(m));
                        }
                    }
                }
            }
            // An enum's variant names, in order: `register_grammar` reads the
            // operator levels, associativities and shapes from them.
            NodeKind::TypeAdtDecl => {
                // `lhs` is the name; the variants are `flags` children at `rhs`.
                let names = arena
                    .children_slice(n.rhs, n.flags as u32)
                    .iter()
                    .map(|&v| {
                        arena
                            .interner
                            .resolve(SymbolId(arena.node(NodeId(v)).lhs))
                            .to_string()
                    })
                    .collect();
                self.adt_variants
                    .insert(arena.interner.resolve(SymbolId(n.lhs)).to_string(), names);
                self.ingest_decl_attrs(arena, id);
            }
            NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
                if let Some(v) = self.eval_cval(arena, NodeId(n.rhs)) {
                    self.const_maps
                        .insert(arena.interner.resolve(SymbolId(n.lhs)).to_string(), v);
                }
            }
            NodeKind::CallExpr => {
                let ch = arena.children_slice(n.lhs, n.rhs);
                if ch.is_empty()
                    || NodeId(ch[0]).is_none()
                    || arena.node(NodeId(ch[0])).node_kind() != NodeKind::IdentExpr
                {
                    return;
                }
                let callee = arena
                    .interner
                    .resolve(SymbolId(arena.node(NodeId(ch[0])).lhs));
                let (hook, vals): (String, Vec<Option<CVal>>) = if callee == "__engine" {
                    if ch.len() < 2 {
                        return;
                    }
                    let Some(CVal::Str(h)) = self.eval_cval(arena, NodeId(ch[1])) else {
                        return;
                    };
                    (
                        h,
                        ch[2..]
                            .iter()
                            .map(|&x| self.eval_cval(arena, NodeId(x)))
                            .collect(),
                    )
                } else if let Some(f) = self.attr_hooks.get(callee).cloned() {
                    let v = f
                        .args
                        .iter()
                        .map(|a| match a {
                            attr_hooks::HookArg::Param(p) => ch
                                .get(p + 1)
                                .and_then(|&x| self.eval_cval(arena, NodeId(x)))
                                .or_else(|| f.defaults.get(*p).and_then(Clone::clone)),
                            attr_hooks::HookArg::Const(c) => Some(c.clone()),
                        })
                        .collect();
                    (f.hook, v)
                } else {
                    return;
                };
                self.apply_hook(&hook, &vals);
            }
            _ => {}
        }
    }

    /// Runs genesis hook `hook` with argument values `vals`, from a
    /// `comptime` call or from an attribute on a declaration.
    /// The position of `variant` in the enum `register_machine_ops` named:
    /// the machine operation an `asm(..)` names.
    pub fn machine_op(&self, variant: &str) -> Option<usize> {
        self.adt_variants
            .get(&self.machine_ops)?
            .iter()
            .position(|v| v == variant)
    }

    /// The position of `variant` in the enum `register_hole_kinds` named.
    pub(crate) fn hole_kind(&self, variant: &str) -> Option<u8> {
        self.adt_variants
            .get(&self.hole_kinds)?
            .iter()
            .position(|v| v == variant)
            .map(|i| i as u8)
    }

    /// The position of `variant` in the enum `register_core_forms` named.
    pub(crate) fn core_form(&self, variant: &str) -> Option<u8> {
        self.adt_variants
            .get(&self.core_forms.0)?
            .iter()
            .position(|v| v == variant)
            .map(|i| i as u8)
    }

    pub(crate) fn apply_hook(&mut self, hook: &str, vals: &[Option<CVal>]) {
        let s_arg = |i: usize| {
            vals.get(i)
                .and_then(|x| match x.as_ref()? {
                    CVal::Str(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default()
        };
        let i_arg = |i: usize| {
            vals.get(i).and_then(|x| match x.as_ref()? {
                CVal::Int(v) => Some(*v),
                _ => None,
            })
        };
        match hook {
            "register_modifier" => {
                let act = if let Some(v) = i_arg(1) {
                    ModAct::Vis(v as u8)
                } else {
                    match s_arg(1).as_str() {
                        "merge_type_members" => ModAct::Partial,
                        "storage_static" => ModAct::Static,
                        "effect_pure" => ModAct::Pure,
                        "binding_mutable" => ModAct::Mut,
                        "binding_const" => ModAct::Const,
                        "perceus_isolated" => ModAct::Iso,
                        "extension_member" => ModAct::Ext,
                        "effect_noalloc" => ModAct::NoAlloc,
                        "override_member" => ModAct::Override,
                        _ => ModAct::Unbuilt,
                    }
                };
                let w = s_arg(0);
                if !w.is_empty() {
                    self.modifiers.insert(w, act);
                }
            }
            "register_parametric_modifier" => {
                let w = s_arg(0);
                if !w.is_empty() {
                    self.param_modifiers.insert(w);
                }
            }
            "register_module_kw" => {
                let w = s_arg(0);
                if !w.is_empty() {
                    self.module_kws
                        .insert(w, s_arg(1) == "declare_package_scope");
                }
            }
            HOOK_UNIT_LITERAL => self.bind_unit_literal(s_arg(0), s_arg(1)),
            HOOK_OPERATOR_FN => self.bind_operator_fn(s_arg(0), s_arg(1)),
            HOOK_NATIVE_FN => self.bind_native(NativeKind::Function, s_arg(0), s_arg(1)),
            HOOK_NATIVE_METHOD => self.bind_native(NativeKind::Method, s_arg(0), s_arg(1)),
            HOOK_NATIVE_PROPERTY => self.bind_native(NativeKind::Property, s_arg(0), s_arg(1)),
            HOOK_NATIVE_VALUE => self.bind_native(NativeKind::Value, s_arg(0), s_arg(1)),
            "bind_native_member" => self.bind_native_member(s_arg(0), s_arg(1), s_arg(2)),
            HOOK_TYPE_QUERY => self.bind_native(NativeKind::TypeQuery, s_arg(0), s_arg(1)),
            HOOK_ENUMERATOR => self.bind_enumerator(s_arg(0), s_arg(1), s_arg(2), s_arg(3)),
            HOOK_GENERATOR => self.bind_generator(s_arg(0), s_arg(1), s_arg(2), s_arg(3)),
            HOOK_TRY_PROTOCOL => self.bind_try_protocol(s_arg(0), s_arg(1), s_arg(2)),
            HOOK_FORMAT_SPEC => self.bind_format_spec(s_arg(0)),
            HOOK_MEMBERSHIP => self.bind_membership(s_arg(0), s_arg(1), s_arg(2)),
            HOOK_SCALAR_TEXT => self.bind_scalar_text(s_arg(0)),
            attr_hooks::HOOK_REG_LAYOUT => {
                if let [Some(CVal::Str(ty)), Some(CVal::Int(width)), Some(CVal::Str(class))] = vals
                {
                    self.reg_layouts.insert(
                        ty.clone(),
                        RegLayout {
                            width: *width,
                            class: class.clone(),
                        },
                    );
                }
            }
            "bind_receiver_ident" => {
                let (w, b, v) = (s_arg(0), s_arg(1), s_arg(2));
                if !w.is_empty() {
                    self.receiver_ident = w;
                }
                self.backing_ident = b;
                self.setter_value_ident = v;
            }
            "bind_receptor" => {
                let (w, k, p) = (
                    s_arg(0),
                    i_arg(1).unwrap_or(0) as u8,
                    i_arg(2).unwrap_or(0) as u8,
                );
                if !w.is_empty() && k > 0 {
                    self.receptors.insert(w, (k, p));
                }
            }
            HOOK_REGISTER_GRAMMAR => self.register_grammar(&s_arg(0), &s_arg(1), &s_arg(2)),
            HOOK_REGISTER_MACHINE_OPS => self.machine_ops = s_arg(0),
            HOOK_REGISTER_HOLE_KINDS => self.hole_kinds = s_arg(0),
            HOOK_REGISTER_CORE_FORMS => self.core_forms = (s_arg(0), s_arg(1)),
            HOOK_GRAMMAR_ENTRY => {
                // An empty symbol is the declaration's own name without its
                // holes: `[Infix(..)]` on `op +` declares `+`, `[Prefix(..)]`
                // on `op *_` declares `*`.
                let (target, symbol) = (s_arg(0), s_arg(1));
                let e = GrammarEntry {
                    symbol: if symbol.is_empty() {
                        crate::decl::strip_holes(&target)
                    } else {
                        symbol
                    },
                    shape: s_arg(2),
                    level: s_arg(3),
                    assoc: s_arg(4),
                    word: s_arg(5),
                    for_type: self.is_type_construct_word(&target),
                };
                if !e.symbol.is_empty() && !self.grammar.contains(&e) {
                    self.apply_grammar_entry(&e);
                    self.grammar.push(e);
                }
            }
            _ => {}
        }
    }
}
/// `MemberExpr.flags` of a prefix-dot member (`.name`), whose receiver the
/// parser filled in. `0` is `a.b` and `1` is `a?.b`.
pub const MEMBER_IMPLIED_RECEIVER: u16 = 2;
/// The kind of the first token `sym` lexes to.
pub(crate) fn first_token_kind(sym: &str) -> Option<TokenKind> {
    let tape = tokenize(sym);
    (0..tape.len())
        .map(|i| tape.kind(i))
        .find(|k| !matches!(k, TokenKind::Newline | TokenKind::Eof))
}

/// Whether `sym` starts with one of the 13 structural tokens (PROP-008 §2)
/// or a seed word: no operator may be declared with it (GRAM005).
pub(crate) fn is_structural_symbol(sym: &str) -> bool {
    match first_token_kind(sym) {
        None => true,
        Some(k) => {
            is_seed_kw(k)
                || matches!(
                    k,
                    TokenKind::LParen
                        | TokenKind::RParen
                        | TokenKind::LBracket
                        | TokenKind::RBracket
                        | TokenKind::LBrace
                        | TokenKind::RBrace
                        | TokenKind::Comma
                        | TokenKind::Semicolon
                        | TokenKind::Colon
                        | TokenKind::ColonColon
                        | TokenKind::InfixDot
                        | TokenKind::PrefixDot
                        | TokenKind::FatArrow
                        | TokenKind::Equal
                )
        }
    }
}

/// The native `BinaryOp` code of infix symbol or word `sym` (0 when it has
/// none), until `PROP-022` §3 lowers operators to their owner methods.
/// A word's code is found from `BinaryOp::symbol`, so no word is written here.
pub(crate) fn binop_code(sym: &str) -> u16 {
    let c = map_binop_sym(sym);
    if c != 0 {
        return c;
    }
    (1..=u8::MAX as u16)
        .filter_map(BinaryOp::from_u16)
        .find(|o| o.symbol() == sym)
        .map_or(0, |o| o as u16)
}
pub(crate) fn map_binop_sym(sym: &str) -> u16 {
    use BinaryOp::*;
    (match sym {
        "+" => Add,
        "-" => Sub,
        "*" => Mul,
        "/" => Div,
        "//" => FloorDiv,
        "%" => Rem,
        "**" => Pow,
        "==" => Eq,
        "!=" => Ne,
        "<" => Lt,
        "<=" => Le,
        ">" => Gt,
        ">=" => Ge,
        "&&" => And,
        "||" => Or,
        "&" => BitAnd,
        "|" => BitOr,
        "^" => BitXor,
        "<<" => Shl,
        ">>" => Shr,
        "|>" => Pipe,
        "<|" => PipeBackward,
        "??" => Coalesce,
        "..=" => RangeInclusive,
        ".." | "..<" => RangeHalfOpen,
        ":=" => Walrus,
        "<-" => Send,
        "<=>" => Spaceship,
        "+|" => SatAdd,
        "-|" => SatSub,
        "*|" => SatMul,
        "+%" => WrapAdd,
        "-%" => WrapSub,
        "*%" => WrapMul,
        "+?" => CheckedAdd,
        "-?" => CheckedSub,
        "*?" => CheckedMul,
        "%%" => EucMod,
        "~>" => Lens,
        ">..<" => OpenRange,
        ">.." => AfterRange,
        ":=:" => Swap,
        _ => return 0,
    }) as u16
}
/// The stdlib files whose `syntax`, keyword, native-binding and protocol
/// declarations fill the bootstrap tables, relative to the source root, in
/// ingest order. Every `stdlib/core/primitives/*.pilot` file follows them.
pub(crate) const BOOTSTRAP_SOURCES: &[&str] = &[
    "stdlib/core/bridge_lang.pilot",
    "stdlib/core/core.pilot",
    "stdlib/core/syntax/type_construct.pilot",
    "stdlib/core/syntax/module_ext.pilot",
    // The hole kinds of a `syntax` rule's sequence head (PROP-009 step 1a).
    "stdlib/core/syntax/holes.pilot",
    // Clauses shared by rules: syntax classes (PROP-009 step 1c).
    "stdlib/core/syntax/clauses.pilot",
    "stdlib/core/syntax/forms.pilot",
    "stdlib/core/syntax/visibility.pilot",
    "stdlib/core/syntax/lifecycle.pilot",
    "stdlib/core/syntax/properties.pilot",
    "stdlib/core/syntax/attribute.pilot",
    // The machine operations `asm(..)` names (PROP-009 step 1b).
    "stdlib/core/asm/machine_ops.pilot",
    // The operator grammar (PROP-008): the levels, the attributes, then every
    // operator's declaration. Files before these use operators too; the
    // bootstrap reads all sources again with the grammar they declare
    // (`build_bootstrap_tables`).
    "stdlib/core/operators/levels.pilot",
    "stdlib/core/operators/shapes.pilot",
    "stdlib/core/operator.pilot",
    "stdlib/core/operators/logic.pilot",
    "stdlib/core/operators/flow.pilot",
    "stdlib/core/syntax/for.pilot",
    "stdlib/core/generator.pilot",
    "stdlib/core/syntax/while.pilot",
    "stdlib/core/syntax/until.pilot",
    "stdlib/core/syntax/switch.pilot",
    "stdlib/core/syntax/guard.pilot",
    "stdlib/core/syntax/exits.pilot",
    "stdlib/core/syntax/try_catch.pilot",
    "stdlib/core/syntax/expr_sugar.pilot",
    "stdlib/core/format.pilot",
    "stdlib/core/syntax/call_quote.pilot",
    "stdlib/core/assert.pilot",
    "stdlib/core/syntax/unit_decl.pilot",
    "stdlib/core/builtins/native_calls.pilot",
    "stdlib/db/sql_syntax.pilot",
    "stdlib/db/set_ops.pilot",
    "stdlib/polyglot/polyglot_syntax.pilot",
    "stdlib/realtime/rt_syntax.pilot",
    "stdlib/asyncio/select_syntax.pilot",
    "stdlib/contracts/contracts_syntax.pilot",
];

/// Where the primitive declarations live, relative to the source root.
pub(crate) const PRIMITIVES_DIR: &str = "stdlib/core/primitives";

pub(crate) static SOURCE_ROOT: OnceLock<std::path::PathBuf> = OnceLock::new();

/// Sets the directory holding the `stdlib/` the bootstrap tables are read
/// from. The toolchain calls it once, before anything is parsed, with the
/// root it runs with; later calls have no effect. Without it the tables come
/// from the workspace this crate was built in.
pub(crate) fn match_postfix(a: &TypePostfixKind, b: &TypePostfixKind) -> bool {
    matches!(
        (a, b),
        (TypePostfixKind::BracketEmpty, TypePostfixKind::BracketEmpty)
            | (
                TypePostfixKind::BracketDotDot,
                TypePostfixKind::BracketDotDot
            )
            | (TypePostfixKind::BracketColon, TypePostfixKind::BracketColon)
    ) || matches!((a, b), (TypePostfixKind::BracketArgs(x), TypePostfixKind::BracketArgs(y)) if x == y)
}
pub(crate) fn same_type_trigger(a: &TypeConstructRule, b: &TypeConstructRule) -> bool {
    match (a, b) {
        (
            TypeConstructRule::NamedGeneric { name: x, .. },
            TypeConstructRule::NamedGeneric { name: y, .. },
        )
        | (TypeConstructRule::Prefix { op: x, .. }, TypeConstructRule::Prefix { op: y, .. })
        | (TypeConstructRule::Infix { op: x, .. }, TypeConstructRule::Infix { op: y, .. }) => {
            x == y
        }
        (
            TypeConstructRule::Postfix {
                kind: TypePostfixKind::BracketArgs(x),
                ..
            },
            TypeConstructRule::Postfix {
                kind: TypePostfixKind::BracketArgs(y),
                ..
            },
        ) => x == y,
        (
            TypeConstructRule::Postfix {
                kind: TypePostfixKind::Sym(x),
                ..
            },
            TypeConstructRule::Postfix {
                kind: TypePostfixKind::Sym(y),
                ..
            },
        ) => x == y,
        (
            TypeConstructRule::Postfix { kind: x, .. },
            TypeConstructRule::Postfix { kind: y, .. },
        ) => core::mem::discriminant(x) == core::mem::discriminant(y),
        _ => false,
    }
}
pub(crate) fn compile_type_rule(
    pat: &[(TokenKind, String)],
    template: TemplateNode,
) -> Option<TypeConstructRule> {
    if pat.is_empty() {
        return None;
    }
    if pat.len() >= 4
        && is_word_tok(pat[0].0)
        && pat[1].0 == TokenKind::Less
        && pat.last().map(|x| x.0) == Some(TokenKind::Greater)
    {
        let (name, mut params) = (pat[0].1.clone(), Vec::new());
        for (tk, s) in &pat[2..pat.len() - 1] {
            if is_word_tok(*tk) {
                params.push(s.clone());
            }
        }
        return Some(TypeConstructRule::NamedGeneric {
            name,
            params,
            template,
        });
    }
    if pat.len() == 2 && !is_word_tok(pat[0].0) {
        return Some(TypeConstructRule::Prefix {
            op: pat[0].1.clone(),
            param: pat[1].1.clone(),
            template,
        });
    }
    if pat.len() >= 3 && pat[1].0 == TokenKind::LBracket {
        let t = pat[0].1.clone();
        if pat[2].0 == TokenKind::RBracket {
            return Some(TypeConstructRule::Postfix {
                kind: TypePostfixKind::BracketEmpty,
                params: vec![t],
                template,
            });
        }
        if pat[2].0 == TokenKind::DotDot {
            return Some(TypeConstructRule::Postfix {
                kind: TypePostfixKind::BracketDotDot,
                params: vec![t],
                template,
            });
        }
        if pat[2].0 == TokenKind::Colon {
            return Some(TypeConstructRule::Postfix {
                kind: TypePostfixKind::BracketColon,
                params: vec![t],
                template,
            });
        }
        let mut ps = vec![t];
        for (tk, s) in &pat[2..] {
            if is_word_tok(*tk) {
                ps.push(s.clone());
            }
        }
        return Some(TypeConstructRule::Postfix {
            kind: TypePostfixKind::BracketArgs(ps.len() - 1),
            params: ps,
            template,
        });
    }
    if pat.len() == 2 && !is_word_tok(pat[1].0) {
        return Some(TypeConstructRule::Postfix {
            kind: TypePostfixKind::Sym(pat[1].1.clone()),
            params: vec![pat[0].1.clone()],
            template,
        });
    }
    let clean: Vec<&(TokenKind, String)> = pat
        .iter()
        .filter(|(k, _)| !matches!(k, TokenKind::LParen | TokenKind::RParen))
        .collect();
    if clean.len() == 3 && !is_word_tok(clean[1].0) {
        return Some(TypeConstructRule::Infix {
            op: clean[1].1.clone(),
            lhs_param: clean[0].1.clone(),
            rhs_param: clean[2].1.clone(),
            template,
        });
    }
    None
}
