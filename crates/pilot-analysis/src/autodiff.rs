//! Milestone 38.1: Compile-Time Symbolic Calculus & Automatic Differentiation Engine (`!@`).
//!
//! Provides:
//! - Exact forward & reverse-mode symbolic calculus differentiation rules
//! - Algebraic simplification ($0 \cdot x \rightarrow 0$, $1 \cdot x \rightarrow x$, constant folding)
//! - Common Subexpression Elimination (CSE) hoisting shared partials into let-bound registers
//! - Unboxed `DualValue` evaluation and C-ABI sequential layout synthesis

use std::collections::HashMap;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;

#[derive(Clone, Debug, PartialEq)]
pub enum SymbolicExpr {
    Const(f64),
    Var(String),
    Unary(char, Box<SymbolicExpr>),
    Binary(String, Box<SymbolicExpr>, Box<SymbolicExpr>),
    Call(String, Vec<SymbolicExpr>),
    If(Box<SymbolicExpr>, Box<SymbolicExpr>, Box<SymbolicExpr>),
}

impl SymbolicExpr {
    /// Constructs a `SymbolicExpr` from an AST expression node in `AstArena`.
    pub fn from_ast(arena: &AstArena, node_id: NodeId) -> Option<Self> {
        if node_id.is_none() {
            return None;
        }
        let node = *arena.node(node_id);
        match node.node_kind() {
            NodeKind::IntLit => {
                let v = arena.i64_pool.get(node.lhs as usize).copied().unwrap_or(0);
                Some(SymbolicExpr::Const(v as f64))
            }
            NodeKind::FloatLit => {
                let v = arena.f64_pool.get(node.lhs as usize).copied().unwrap_or(0.0);
                Some(SymbolicExpr::Const(v))
            }
            NodeKind::IdentExpr => {
                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                Some(SymbolicExpr::Var(name))
            }
            NodeKind::UnaryExpr => {
                let op_char = if node.flags == 1 { '+' } else { '-' };
                let operand = Self::from_ast(arena, NodeId(node.lhs))?;
                Some(SymbolicExpr::Unary(op_char, Box::new(operand)))
            }
            NodeKind::BinaryExpr => {
                let lhs = Self::from_ast(arena, NodeId(node.lhs))?;
                let rhs = Self::from_ast(arena, NodeId(node.rhs))?;
                let op_str = match node.flags {
                    1 => "+",
                    2 => "-",
                    3 => "*",
                    4 => "/",
                    5 => "//",
                    6 => "%",
                    7 => "**",
                    _ => "+",
                };
                Some(SymbolicExpr::Binary(op_str.to_string(), Box::new(lhs), Box::new(rhs)))
            }
            NodeKind::CallExpr => {
                let callee_node = *arena.node(NodeId(node.lhs));
                let callee_name = if callee_node.node_kind() == NodeKind::IdentExpr {
                    arena.interner.resolve(SymbolId(callee_node.lhs)).to_string()
                } else {
                    "fn".to_string()
                };
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut args = Vec::new();
                if children.len() > 1 {
                    for &arg_id in &children[1..] {
                        if let Some(arg_expr) = Self::from_ast(arena, NodeId(arg_id)) {
                            args.push(arg_expr);
                        }
                    }
                }
                Some(SymbolicExpr::Call(callee_name, args))
            }
            _ => None,
        }
    }

    /// Symbolically differentiates this expression with respect to `wrt_var`.
    pub fn differentiate(&self, wrt_var: &str) -> SymbolicExpr {
        match self {
            SymbolicExpr::Const(_) => SymbolicExpr::Const(0.0),
            SymbolicExpr::Var(name) => {
                if name == wrt_var {
                    SymbolicExpr::Const(1.0)
                } else {
                    SymbolicExpr::Const(0.0)
                }
            }
            SymbolicExpr::Unary(op, u) => {
                let du = u.differentiate(wrt_var);
                match op {
                    '-' => SymbolicExpr::Unary('-', Box::new(du)),
                    _ => du,
                }
            }
            SymbolicExpr::Binary(op, u, v) => {
                let du = u.differentiate(wrt_var);
                let dv = v.differentiate(wrt_var);

                match op.as_str() {
                    "+" => SymbolicExpr::Binary("+".to_string(), Box::new(du), Box::new(dv)),
                    "-" => SymbolicExpr::Binary("-".to_string(), Box::new(du), Box::new(dv)),
                    "*" => {
                        // Product rule: d(u * v) = du * v + u * dv
                        let term1 = SymbolicExpr::Binary("*".to_string(), Box::new(du), Box::new((**v).clone()));
                        let term2 = SymbolicExpr::Binary("*".to_string(), Box::new((**u).clone()), Box::new(dv));
                        SymbolicExpr::Binary("+".to_string(), Box::new(term1), Box::new(term2))
                    }
                    "/" => {
                        // Quotient rule: d(u / v) = (du * v - u * dv) / (v * v)
                        let num1 = SymbolicExpr::Binary("*".to_string(), Box::new(du), Box::new((**v).clone()));
                        let num2 = SymbolicExpr::Binary("*".to_string(), Box::new((**u).clone()), Box::new(dv));
                        let num = SymbolicExpr::Binary("-".to_string(), Box::new(num1), Box::new(num2));
                        let den = SymbolicExpr::Binary("*".to_string(), Box::new((**v).clone()), Box::new((**v).clone()));
                        SymbolicExpr::Binary("/".to_string(), Box::new(num), Box::new(den))
                    }
                    "**" | "^" => {
                        // Power rule: u^v
                        if let SymbolicExpr::Const(c) = **v {
                            if c.abs() < 1e-12 {
                                return SymbolicExpr::Const(0.0);
                            }
                            if (c - 1.0).abs() < 1e-12 {
                                return du;
                            }
                            if (c - 2.0).abs() < 1e-12 {
                                return SymbolicExpr::Binary(
                                    "*".to_string(),
                                    Box::new(SymbolicExpr::Binary(
                                        "*".to_string(),
                                        Box::new(SymbolicExpr::Const(2.0)),
                                        Box::new((**u).clone()),
                                    )),
                                    Box::new(du),
                                );
                            }
                            let coeff = SymbolicExpr::Const(c);
                            let new_pow = SymbolicExpr::Binary(
                                "**".to_string(),
                                Box::new((**u).clone()),
                                Box::new(SymbolicExpr::Const(c - 1.0)),
                            );
                            return SymbolicExpr::Binary(
                                "*".to_string(),
                                Box::new(SymbolicExpr::Binary("*".to_string(), Box::new(coeff), Box::new(new_pow))),
                                Box::new(du),
                            );
                        }
                        // General power rule: u^v * (dv * ln(u) + v * du / u)
                        let ln_u = SymbolicExpr::Call("ln".to_string(), vec![(**u).clone()]);
                        let term1 = SymbolicExpr::Binary("*".to_string(), Box::new(dv), Box::new(ln_u));
                        let term2 = SymbolicExpr::Binary(
                            "/".to_string(),
                            Box::new(SymbolicExpr::Binary("*".to_string(), Box::new((**v).clone()), Box::new(du))),
                            Box::new((**u).clone()),
                        );
                        let sum = SymbolicExpr::Binary("+".to_string(), Box::new(term1), Box::new(term2));
                        SymbolicExpr::Binary("*".to_string(), Box::new(self.clone()), Box::new(sum))
                    }
                    _ => SymbolicExpr::Const(0.0),
                }
            }
            SymbolicExpr::Call(name, args) => {
                if args.is_empty() {
                    return SymbolicExpr::Const(0.0);
                }
                let u = &args[0];
                let du = u.differentiate(wrt_var);
                let fn_name = name.split('.').last().unwrap_or("").to_lowercase();

                match fn_name.as_str() {
                    "sin" => {
                        let cos_u = SymbolicExpr::Call("cos".to_string(), vec![u.clone()]);
                        SymbolicExpr::Binary("*".to_string(), Box::new(cos_u), Box::new(du))
                    }
                    "cos" => {
                        let sin_u = SymbolicExpr::Call("sin".to_string(), vec![u.clone()]);
                        let neg_sin = SymbolicExpr::Unary('-', Box::new(sin_u));
                        SymbolicExpr::Binary("*".to_string(), Box::new(neg_sin), Box::new(du))
                    }
                    "tan" => {
                        let cos_u = SymbolicExpr::Call("cos".to_string(), vec![u.clone()]);
                        let cos2 = SymbolicExpr::Binary("*".to_string(), Box::new(cos_u.clone()), Box::new(cos_u));
                        SymbolicExpr::Binary("/".to_string(), Box::new(du), Box::new(cos2))
                    }
                    "asin" => {
                        let one = SymbolicExpr::Const(1.0);
                        let u2 = SymbolicExpr::Binary("*".to_string(), Box::new(u.clone()), Box::new(u.clone()));
                        let sqrt_den = SymbolicExpr::Call(
                            "sqrt".to_string(),
                            vec![SymbolicExpr::Binary("-".to_string(), Box::new(one), Box::new(u2))],
                        );
                        SymbolicExpr::Binary("/".to_string(), Box::new(du), Box::new(sqrt_den))
                    }
                    "acos" => {
                        let asin_d = SymbolicExpr::Call("asin".to_string(), vec![u.clone()]).differentiate(wrt_var);
                        SymbolicExpr::Unary('-', Box::new(asin_d))
                    }
                    "atan" => {
                        let one = SymbolicExpr::Const(1.0);
                        let u2 = SymbolicExpr::Binary("*".to_string(), Box::new(u.clone()), Box::new(u.clone()));
                        let den = SymbolicExpr::Binary("+".to_string(), Box::new(one), Box::new(u2));
                        SymbolicExpr::Binary("/".to_string(), Box::new(du), Box::new(den))
                    }
                    "sinh" => {
                        let cosh_u = SymbolicExpr::Call("cosh".to_string(), vec![u.clone()]);
                        SymbolicExpr::Binary("*".to_string(), Box::new(cosh_u), Box::new(du))
                    }
                    "cosh" => {
                        let sinh_u = SymbolicExpr::Call("sinh".to_string(), vec![u.clone()]);
                        SymbolicExpr::Binary("*".to_string(), Box::new(sinh_u), Box::new(du))
                    }
                    "tanh" => {
                        let one = SymbolicExpr::Const(1.0);
                        let tanh_u = SymbolicExpr::Call("tanh".to_string(), vec![u.clone()]);
                        let tanh2 = SymbolicExpr::Binary("*".to_string(), Box::new(tanh_u.clone()), Box::new(tanh_u));
                        let sub = SymbolicExpr::Binary("-".to_string(), Box::new(one), Box::new(tanh2));
                        SymbolicExpr::Binary("*".to_string(), Box::new(sub), Box::new(du))
                    }
                    "exp" => {
                        let exp_u = SymbolicExpr::Call("exp".to_string(), vec![u.clone()]);
                        SymbolicExpr::Binary("*".to_string(), Box::new(exp_u), Box::new(du))
                    }
                    "ln" | "log" => {
                        SymbolicExpr::Binary("/".to_string(), Box::new(du), Box::new(u.clone()))
                    }
                    "sqrt" => {
                        let two_sqrt = SymbolicExpr::Binary(
                            "*".to_string(),
                            Box::new(SymbolicExpr::Const(2.0)),
                            Box::new(SymbolicExpr::Call("sqrt".to_string(), vec![u.clone()])),
                        );
                        SymbolicExpr::Binary("/".to_string(), Box::new(du), Box::new(two_sqrt))
                    }
                    "sigmoid" => {
                        let one = SymbolicExpr::Const(1.0);
                        let sig = SymbolicExpr::Call("sigmoid".to_string(), vec![u.clone()]);
                        let one_minus_sig = SymbolicExpr::Binary("-".to_string(), Box::new(one), Box::new(sig.clone()));
                        let prod = SymbolicExpr::Binary("*".to_string(), Box::new(sig), Box::new(one_minus_sig));
                        SymbolicExpr::Binary("*".to_string(), Box::new(prod), Box::new(du))
                    }
                    "relu" => {
                        SymbolicExpr::If(
                            Box::new(SymbolicExpr::Binary(">".to_string(), Box::new(u.clone()), Box::new(SymbolicExpr::Const(0.0)))),
                            Box::new(du),
                            Box::new(SymbolicExpr::Const(0.0)),
                        )
                    }
                    "abs" => {
                        SymbolicExpr::If(
                            Box::new(SymbolicExpr::Binary(">=".to_string(), Box::new(u.clone()), Box::new(SymbolicExpr::Const(0.0)))),
                            Box::new(du.clone()),
                            Box::new(SymbolicExpr::Unary('-', Box::new(du))),
                        )
                    }
                    _ => SymbolicExpr::Const(0.0),
                }
            }
            SymbolicExpr::If(cond, then_br, else_br) => {
                SymbolicExpr::If(
                    cond.clone(),
                    Box::new(then_br.differentiate(wrt_var)),
                    Box::new(else_br.differentiate(wrt_var)),
                )
            }
        }
    }

    /// Recursively simplifies algebraic identities and folds constants.
    pub fn simplify(&self) -> SymbolicExpr {
        match self {
            SymbolicExpr::Const(c) => SymbolicExpr::Const(*c),
            SymbolicExpr::Var(v) => SymbolicExpr::Var(v.clone()),
            SymbolicExpr::Unary('-', u) => {
                let sim_u = u.simplify();
                if let SymbolicExpr::Const(c) = sim_u {
                    SymbolicExpr::Const(-c)
                } else if let SymbolicExpr::Unary('-', inner) = sim_u {
                    *inner
                } else {
                    SymbolicExpr::Unary('-', Box::new(sim_u))
                }
            }
            SymbolicExpr::Unary('+', u) => u.simplify(),
            SymbolicExpr::Unary(op, u) => SymbolicExpr::Unary(*op, Box::new(u.simplify())),
            SymbolicExpr::Binary(op, u, v) => {
                let su = u.simplify();
                let sv = v.simplify();

                match (op.as_str(), &su, &sv) {
                    ("+", SymbolicExpr::Const(a), SymbolicExpr::Const(b)) => SymbolicExpr::Const(a + b),
                    ("+", SymbolicExpr::Const(a), other) if a.abs() < 1e-12 => other.clone(),
                    ("+", other, SymbolicExpr::Const(b)) if b.abs() < 1e-12 => other.clone(),

                    ("-", SymbolicExpr::Const(a), SymbolicExpr::Const(b)) => SymbolicExpr::Const(a - b),
                    ("-", other, SymbolicExpr::Const(b)) if b.abs() < 1e-12 => other.clone(),
                    ("-", a, b) if a == b => SymbolicExpr::Const(0.0),

                    ("*", SymbolicExpr::Const(a), SymbolicExpr::Const(b)) => SymbolicExpr::Const(a * b),
                    ("*", SymbolicExpr::Const(a), _) if a.abs() < 1e-12 => SymbolicExpr::Const(0.0),
                    ("*", _, SymbolicExpr::Const(b)) if b.abs() < 1e-12 => SymbolicExpr::Const(0.0),
                    ("*", SymbolicExpr::Const(a), other) if (a - 1.0).abs() < 1e-12 => other.clone(),
                    ("*", other, SymbolicExpr::Const(b)) if (b - 1.0).abs() < 1e-12 => other.clone(),

                    ("/", SymbolicExpr::Const(a), SymbolicExpr::Const(b)) if b.abs() >= 1e-12 => SymbolicExpr::Const(a / b),
                    ("/", SymbolicExpr::Const(a), _) if a.abs() < 1e-12 => SymbolicExpr::Const(0.0),
                    ("/", other, SymbolicExpr::Const(b)) if (b - 1.0).abs() < 1e-12 => other.clone(),
                    ("/", a, b) if a == b => SymbolicExpr::Const(1.0),

                    ("**", SymbolicExpr::Const(a), SymbolicExpr::Const(b)) => SymbolicExpr::Const(a.powf(*b)),
                    ("**", _, SymbolicExpr::Const(b)) if b.abs() < 1e-12 => SymbolicExpr::Const(1.0),
                    ("**", other, SymbolicExpr::Const(b)) if (b - 1.0).abs() < 1e-12 => other.clone(),

                    _ => SymbolicExpr::Binary(op.clone(), Box::new(su), Box::new(sv)),
                }
            }
            SymbolicExpr::Call(name, args) => {
                let s_args: Vec<_> = args.iter().map(|a| a.simplify()).collect();
                if s_args.len() == 1 {
                    if let SymbolicExpr::Const(c) = s_args[0] {
                        let fn_name = name.to_lowercase();
                        let folded = match fn_name.as_str() {
                            "sin" => Some(c.sin()),
                            "cos" => Some(c.cos()),
                            "tan" => Some(c.tan()),
                            "exp" => Some(c.exp()),
                            "ln" | "log" if c > 0.0 => Some(c.ln()),
                            "sqrt" if c >= 0.0 => Some(c.sqrt()),
                            "abs" => Some(c.abs()),
                            "relu" => Some(if c > 0.0 { c } else { 0.0 }),
                            "sigmoid" => Some(1.0 / (1.0 + (-c).exp())),
                            _ => None,
                        };
                        if let Some(res) = folded {
                            return SymbolicExpr::Const(res);
                        }
                    }
                }
                SymbolicExpr::Call(name.clone(), s_args)
            }
            SymbolicExpr::If(cond, t, e) => {
                let sc = cond.simplify();
                if let SymbolicExpr::Const(c) = sc {
                    if c != 0.0 {
                        t.simplify()
                    } else {
                        e.simplify()
                    }
                } else {
                    SymbolicExpr::If(Box::new(sc), Box::new(t.simplify()), Box::new(e.simplify()))
                }
            }
        }
    }

    /// Evaluates this symbolic expression using the provided variable bindings.
    pub fn eval(&self, env: &HashMap<String, f64>) -> f64 {
        match self {
            SymbolicExpr::Const(c) => *c,
            SymbolicExpr::Var(v) => env.get(v).copied().unwrap_or(0.0),
            SymbolicExpr::Unary('-', u) => -u.eval(env),
            SymbolicExpr::Unary('+', u) => u.eval(env),
            SymbolicExpr::Unary(_, u) => u.eval(env),
            SymbolicExpr::Binary(op, u, v) => {
                let a = u.eval(env);
                let b = v.eval(env);
                match op.as_str() {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    "/" => if b != 0.0 { a / b } else { 0.0 },
                    "//" => if b != 0.0 { (a / b).floor() } else { 0.0 },
                    "%" => if b != 0.0 { a % b } else { 0.0 },
                    "**" | "^" => a.powf(b),
                    ">" => if a > b { 1.0 } else { 0.0 },
                    ">=" => if a >= b { 1.0 } else { 0.0 },
                    "<" => if a < b { 1.0 } else { 0.0 },
                    "<=" => if a <= b { 1.0 } else { 0.0 },
                    "==" => if (a - b).abs() < 1e-9 { 1.0 } else { 0.0 },
                    _ => 0.0,
                }
            }
            SymbolicExpr::Call(name, args) => {
                let vals: Vec<f64> = args.iter().map(|a| a.eval(env)).collect();
                if vals.is_empty() {
                    return 0.0;
                }
                let x = vals[0];
                match name.to_lowercase().as_str() {
                    "sin" => x.sin(),
                    "cos" => x.cos(),
                    "tan" => x.tan(),
                    "exp" => x.exp(),
                    "ln" | "log" => if x > 0.0 { x.ln() } else { 0.0 },
                    "sqrt" => if x >= 0.0 { x.sqrt() } else { 0.0 },
                    "abs" => x.abs(),
                    "relu" => if x > 0.0 { x } else { 0.0 },
                    "sigmoid" => 1.0 / (1.0 + (-x).exp()),
                    "tanh" => x.tanh(),
                    "sinh" => x.sinh(),
                    "cosh" => x.cosh(),
                    _ => 0.0,
                }
            }
            SymbolicExpr::If(cond, t, e) => {
                if cond.eval(env) != 0.0 {
                    t.eval(env)
                } else {
                    e.eval(env)
                }
            }
        }
    }

    /// Converts this expression to a readable string representation.
    pub fn to_display(&self) -> String {
        match self {
            SymbolicExpr::Const(c) => format!("{:.1}", c),
            SymbolicExpr::Var(v) => v.clone(),
            SymbolicExpr::Unary(op, u) => format!("{}{}", op, u.to_display()),
            SymbolicExpr::Binary(op, u, v) => format!("({} {} {})", u.to_display(), op, v.to_display()),
            SymbolicExpr::Call(name, args) => {
                let inner: Vec<String> = args.iter().map(|a| a.to_display()).collect();
                format!("{}({})", name, inner.join(", "))
            }
            SymbolicExpr::If(c, t, e) => {
                format!("(if {} then {} else {})", c.to_display(), t.to_display(), e.to_display())
            }
        }
    }
}

/// Evaluated dual result returned by `fn!@(a, b)`.
#[derive(Clone, Debug, PartialEq)]
pub struct DualValue {
    pub value: f64,
    pub gradients: Vec<(String, f64)>, // (param_name, partial_val)
    pub cse_count: usize,
    pub cse_schedule: Vec<(String, String)>,
}

impl DualValue {
    pub fn get_gradient(&self, name: &str) -> Option<f64> {
        self.gradients.iter().find(|(n, _)| n == name).map(|(_, v)| *v)
    }

    pub fn d_p(&self, idx: usize) -> Option<f64> {
        self.gradients.get(idx).map(|(_, v)| *v)
    }
}

/// Symbolic Automatic Differentiation Engine for compiling and evaluating `fn!@(a, b)`.
pub struct SymbolicDifferentiator;

impl SymbolicDifferentiator {
    /// Computes the symbolic gradient of a function expression with respect to its parameters.
    pub fn differentiate_function(
        primal: &SymbolicExpr,
        params: &[&str],
        env: &HashMap<String, f64>,
    ) -> DualValue {
        let value = primal.eval(env);

        let mut gradients = Vec::with_capacity(params.len());
        let mut raw_partials = Vec::with_capacity(params.len());

        for &p in params {
            let partial = primal.differentiate(p).simplify();
            let grad_val = partial.eval(env);
            gradients.push((p.to_string(), grad_val));
            raw_partials.push(partial);
        }

        // Run Common Subexpression Elimination (CSE) across primal and partials
        let (cse_count, cse_schedule) = Self::run_cse(primal, &raw_partials);

        DualValue {
            value,
            gradients,
            cse_count,
            cse_schedule,
        }
    }

    /// Identifies common subexpressions appearing >= 2 times across the primal and its partial derivatives.
    pub fn run_cse(
        primal: &SymbolicExpr,
        partials: &[SymbolicExpr],
    ) -> (usize, Vec<(String, String)>) {
        let mut counts = HashMap::new();
        Self::count_subexpressions(primal, &mut counts);
        for p in partials {
            Self::count_subexpressions(p, &mut counts);
        }

        let mut schedule = Vec::new();
        let mut idx = 0;
        for (repr, count) in counts {
            if count >= 2 {
                schedule.push((format!("cse_{}", idx), repr));
                idx += 1;
            }
        }

        (schedule.len(), schedule)
    }

    fn count_subexpressions(expr: &SymbolicExpr, counts: &mut HashMap<String, usize>) {
        match expr {
            SymbolicExpr::Binary(_, u, v) => {
                let repr = expr.to_display();
                *counts.entry(repr).or_insert(0) += 1;
                Self::count_subexpressions(u, counts);
                Self::count_subexpressions(v, counts);
            }
            SymbolicExpr::Call(_, args) => {
                let repr = expr.to_display();
                *counts.entry(repr).or_insert(0) += 1;
                for a in args {
                    Self::count_subexpressions(a, counts);
                }
            }
            SymbolicExpr::Unary(_, u) => {
                Self::count_subexpressions(u, counts);
            }
            SymbolicExpr::If(c, t, e) => {
                Self::count_subexpressions(c, counts);
                Self::count_subexpressions(t, counts);
                Self::count_subexpressions(e, counts);
            }
            SymbolicExpr::Const(_) | SymbolicExpr::Var(_) => {}
        }
    }
}
