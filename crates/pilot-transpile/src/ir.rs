//! Milestone 41.1: `AstArena` -> Canonical Typed `TranspileIr` Lowerer.
//!
//! Represents the normalized, polyglot intermediate representation for Pilot
//! that maps 1-to-1 onto idiomatic target language constructs.

use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_analysis::AnalysisResult;
use pilot_analysis::types::{TypeId, TypeKind};
use pilot_intern::SymbolId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TargetLanguage {
    C,
    Cpp,
    CSharp,
    Rust,
    Python,
    JavaScript,
    Lua,
}

impl TargetLanguage {
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "c" => Some(Self::C),
            "cpp" | "c++" => Some(Self::Cpp),
            "csharp" | "c#" | "cs" => Some(Self::CSharp),
            "rust" | "rs" => Some(Self::Rust),
            "python" | "py" => Some(Self::Python),
            "javascript" | "js" | "typescript" | "ts" => Some(Self::JavaScript),
            "lua" => Some(Self::Lua),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrType {
    Void,
    Bool,
    Int,
    Float,
    String,
    Option(Box<IrType>),
    Result {
        ok: Box<IrType>,
        err: Box<IrType>,
    },
    List(Box<IrType>),
    Map {
        key: Box<IrType>,
        value: Box<IrType>,
    },
    Tuple(Vec<IrType>),
    Custom(String),
}

impl IrType {
    pub fn from_analysis(types: &pilot_analysis::types::TypeArena, id: TypeId) -> Self {
        match id {
            TypeId::VOID => Self::Void,
            TypeId::BOOL => Self::Bool,
            TypeId::I64 | TypeId::I32 => Self::Int,
            TypeId::F64 | TypeId::F32 => Self::Float,
            TypeId::STR => Self::String,
            _ => {
                let kind = types.get(id);
                match kind {
                    TypeKind::Option(inner) => {
                        Self::Option(Box::new(Self::from_analysis(types, *inner)))
                    }
                    TypeKind::Result { ok, err } => Self::Result {
                        ok: Box::new(Self::from_analysis(types, *ok)),
                        err: Box::new(Self::from_analysis(types, *err)),
                    },
                    TypeKind::Collection { element, .. } => {
                        Self::List(Box::new(Self::from_analysis(types, *element)))
                    }
                    TypeKind::Map { key, value } => Self::Map {
                        key: Box::new(Self::from_analysis(types, *key)),
                        value: Box::new(Self::from_analysis(types, *value)),
                    },
                    TypeKind::Tuple(fields) => {
                        Self::Tuple(fields.iter().map(|&f| Self::from_analysis(types, f)).collect())
                    }
                    TypeKind::Primitive(p) if p == "int" => Self::Int,
                    TypeKind::Primitive(p) if p == "float" => Self::Float,
                    TypeKind::Primitive(p) if p == "bool" => Self::Bool,
                    TypeKind::Primitive(p) if p == "str" => Self::String,
                    _ => Self::Int,
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum IrExpr {
    LitInt(i64),
    LitFloat(f64),
    LitStr(String),
    LitBool(bool),
    LitNull,
    Var(String),
    Binary {
        op: String,
        lhs: Box<IrExpr>,
        rhs: Box<IrExpr>,
    },
    Unary {
        op: String,
        expr: Box<IrExpr>,
    },
    Call {
        callee: String,
        args: Vec<IrExpr>,
    },
    Member {
        target: Box<IrExpr>,
        field: String,
    },
    Index {
        target: Box<IrExpr>,
        index: Box<IrExpr>,
    },
    If {
        cond: Box<IrExpr>,
        then_expr: Box<IrExpr>,
        else_expr: Box<IrExpr>,
    },
    List(Vec<IrExpr>),
    Map(Vec<(IrExpr, IrExpr)>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum IrStmt {
    Let {
        name: String,
        ty: IrType,
        init: IrExpr,
    },
    Var {
        name: String,
        ty: IrType,
        init: IrExpr,
    },
    Assign {
        target: IrExpr,
        val: IrExpr,
    },
    If {
        cond: IrExpr,
        then_branch: Vec<IrStmt>,
        else_branch: Vec<IrStmt>,
    },
    While {
        cond: IrExpr,
        body: Vec<IrStmt>,
    },
    For {
        var: String,
        iter: IrExpr,
        body: Vec<IrStmt>,
    },
    Guard {
        cond: IrExpr,
        else_body: Vec<IrStmt>,
    },
    Return(Option<IrExpr>),
    Expr(IrExpr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrAdtVariant {
    pub name: String,
    pub fields: Vec<(String, IrType)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IrTypeDecl {
    Struct {
        name: String,
        fields: Vec<(String, IrType)>,
    },
    Record {
        name: String,
        fields: Vec<(String, IrType)>,
    },
    Adt {
        name: String,
        variants: Vec<IrAdtVariant>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrFunction {
    pub name: String,
    pub ret_type: IrType,
    pub params: Vec<(String, IrType)>,
    pub body: Vec<IrStmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrModule {
    pub name: String,
    pub type_decls: Vec<IrTypeDecl>,
    pub functions: Vec<IrFunction>,
}

impl IrModule {
    /// Lowers an `AstArena` and `AnalysisResult` directly into `IrModule`.
    pub fn from_ast(arena: &AstArena, analysis: &AnalysisResult, module_name: &str) -> Self {
        let mut functions = Vec::new();
        let mut type_decls = Vec::new();
        let mut top_stmts = Vec::new();

        if arena.root.is_some() {
            let root_node = arena.node(arena.root);
            let top_ids = arena.children_slice(root_node.lhs, root_node.rhs);

            for &child_raw in top_ids {
                let child_id = NodeId(child_raw);
                let node = arena.node(child_id);

                match node.node_kind() {
                    NodeKind::FnDecl => {
                        functions.push(Self::lower_fn(arena, analysis, child_id));
                    }
                    NodeKind::RecordDecl | NodeKind::StructDecl => {
                        let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                        type_decls.push(IrTypeDecl::Struct {
                            name,
                            fields: Vec::new(),
                        });
                    }
                    NodeKind::TypeAdtDecl => {
                        let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                        type_decls.push(IrTypeDecl::Adt {
                            name,
                            variants: Vec::new(),
                        });
                    }
                    _ => {
                        top_stmts.push(Self::lower_stmt(arena, analysis, child_id));
                    }
                }
            }
        }

        // Wrap orphan top-level statements into a synthetic main function
        if !top_stmts.is_empty() && !functions.iter().any(|f| f.name == "main") {
            functions.push(IrFunction {
                name: "main".to_string(),
                ret_type: IrType::Int,
                params: Vec::new(),
                body: top_stmts,
            });
        }

        Self {
            name: module_name.to_string(),
            type_decls,
            functions,
        }
    }

    fn lower_fn(arena: &AstArena, analysis: &AnalysisResult, fn_id: NodeId) -> IrFunction {
        let fn_node = arena.node(fn_id);
        let fn_name = arena.interner.resolve(SymbolId(fn_node.lhs)).to_string();
        let fn_start = fn_node.rhs as usize;
        let fn_len = fn_node.flags as usize;

        let mut params = Vec::new();
        if fn_len > 1 {
            for &p_id in &arena.extra_children[fn_start + 1..fn_start + fn_len] {
                let p_node = arena.node(NodeId(p_id));
                let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                let ty = if let Some(&ty_id) = analysis.inferred_types.get(&p_id) {
                    IrType::from_analysis(&analysis.types, ty_id)
                } else {
                    IrType::Int
                };
                params.push((p_name, ty));
            }
        }

        let ret_type = if let Some(&ty_id) = analysis.inferred_types.get(&fn_id.0) {
            IrType::from_analysis(&analysis.types, ty_id)
        } else if fn_name == "main" {
            IrType::Int
        } else {
            IrType::Void
        };

        let mut body = Vec::new();
        let body_id = NodeId(arena.extra_children[fn_start]);
        if body_id.is_some() {
            let body_node = arena.node(body_id);
            if body_node.node_kind() == NodeKind::BlockExpr {
                let child_ids = arena.children_slice(body_node.lhs, body_node.rhs);
                for &c_raw in child_ids {
                    body.push(Self::lower_stmt(arena, analysis, NodeId(c_raw)));
                }
            } else {
                body.push(Self::lower_stmt(arena, analysis, body_id));
            }
        }

        IrFunction {
            name: fn_name,
            ret_type,
            params,
            body,
        }
    }

    fn lower_stmt(arena: &AstArena, analysis: &AnalysisResult, id: NodeId) -> IrStmt {
        if id.is_none() {
            return IrStmt::Return(None);
        }
        let node = arena.node(id);

        match node.node_kind() {
            NodeKind::LetStmt => {
                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                let init_id = NodeId(node.rhs);
                let init = Self::lower_expr(arena, analysis, init_id);
                let ty = if let Some(&ty_id) = analysis.inferred_types.get(&init_id.0) {
                    IrType::from_analysis(&analysis.types, ty_id)
                } else {
                    IrType::Int
                };
                IrStmt::Let { name, ty, init }
            }
            NodeKind::VarStmt => {
                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                let init_id = NodeId(node.rhs);
                let init = Self::lower_expr(arena, analysis, init_id);
                let ty = if let Some(&ty_id) = analysis.inferred_types.get(&init_id.0) {
                    IrType::from_analysis(&analysis.types, ty_id)
                } else {
                    IrType::Int
                };
                IrStmt::Var { name, ty, init }
            }
            NodeKind::AssignStmt => {
                let target = Self::lower_expr(arena, analysis, NodeId(node.lhs));
                let val = Self::lower_expr(arena, analysis, NodeId(node.rhs));
                IrStmt::Assign { target, val }
            }
            NodeKind::ReturnStmt => {
                let val_id = NodeId(node.lhs);
                if val_id.is_some() {
                    IrStmt::Return(Some(Self::lower_expr(arena, analysis, val_id)))
                } else {
                    IrStmt::Return(None)
                }
            }
            NodeKind::WhileStmt => {
                let cond = Self::lower_expr(arena, analysis, NodeId(node.lhs));
                let mut body = Vec::new();
                let body_id = NodeId(node.rhs);
                if body_id.is_some() {
                    let bn = arena.node(body_id);
                    if bn.node_kind() == NodeKind::BlockExpr {
                        for &c in arena.children_slice(bn.lhs, bn.rhs) {
                            body.push(Self::lower_stmt(arena, analysis, NodeId(c)));
                        }
                    } else {
                        body.push(Self::lower_stmt(arena, analysis, body_id));
                    }
                }
                IrStmt::While { cond, body }
            }
            NodeKind::IfExpr => {
                let cond = Self::lower_expr(arena, analysis, NodeId(node.lhs));
                let mut then_branch = Vec::new();
                let then_id = NodeId(node.rhs);
                if then_id.is_some() {
                    let bn = arena.node(then_id);
                    if bn.node_kind() == NodeKind::BlockExpr {
                        for &c in arena.children_slice(bn.lhs, bn.rhs) {
                            then_branch.push(Self::lower_stmt(arena, analysis, NodeId(c)));
                        }
                    } else {
                        then_branch.push(Self::lower_stmt(arena, analysis, then_id));
                    }
                }
                IrStmt::If {
                    cond,
                    then_branch,
                    else_branch: Vec::new(),
                }
            }
            NodeKind::ForExpr => {
                let var = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                let mut body = Vec::new();
                let body_id = NodeId(node.rhs);
                if body_id.is_some() {
                    let bn = arena.node(body_id);
                    if bn.node_kind() == NodeKind::BlockExpr {
                        for &c in arena.children_slice(bn.lhs, bn.rhs) {
                            body.push(Self::lower_stmt(arena, analysis, NodeId(c)));
                        }
                    } else {
                        body.push(Self::lower_stmt(arena, analysis, body_id));
                    }
                }
                IrStmt::For {
                    var,
                    iter: IrExpr::Call {
                        callee: "range".to_string(),
                        args: vec![IrExpr::LitInt(0), IrExpr::LitInt(100)],
                    },
                    body,
                }
            }
            NodeKind::GuardStmt => {
                let cond = Self::lower_expr(arena, analysis, NodeId(node.lhs));
                let mut else_body = Vec::new();
                let else_id = NodeId(node.rhs);
                if else_id.is_some() {
                    let bn = arena.node(else_id);
                    if bn.node_kind() == NodeKind::BlockExpr {
                        for &c in arena.children_slice(bn.lhs, bn.rhs) {
                            else_body.push(Self::lower_stmt(arena, analysis, NodeId(c)));
                        }
                    } else {
                        else_body.push(Self::lower_stmt(arena, analysis, else_id));
                    }
                }
                IrStmt::Guard { cond, else_body }
            }
            NodeKind::ExprStmt => IrStmt::Expr(Self::lower_expr(arena, analysis, NodeId(node.lhs))),
            _ => IrStmt::Expr(IrExpr::LitInt(0)),
        }
    }

    fn lower_expr(arena: &AstArena, analysis: &AnalysisResult, id: NodeId) -> IrExpr {
        if id.is_none() {
            return IrExpr::LitNull;
        }
        let node = arena.node(id);

        match node.node_kind() {
            NodeKind::IntLit => {
                let val = arena.i64_pool.get(node.lhs as usize).copied().unwrap_or(0);
                IrExpr::LitInt(val)
            }
            NodeKind::FloatLit => {
                let val = arena.f64_pool.get(node.lhs as usize).copied().unwrap_or(0.0);
                IrExpr::LitFloat(val)
            }
            NodeKind::BoolLit => IrExpr::LitBool(node.lhs != 0),
            NodeKind::StrLit => {
                let s = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                IrExpr::LitStr(s)
            }
            NodeKind::PolyglotBlock => {
                let s = arena.interner.resolve(SymbolId(node.rhs)).to_string();
                IrExpr::LitStr(s)
            }
            NodeKind::IdentExpr => {
                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                IrExpr::Var(name)
            }
            NodeKind::BinaryExpr => {
                let lhs = Box::new(Self::lower_expr(arena, analysis, NodeId(node.lhs)));
                let rhs = Box::new(Self::lower_expr(arena, analysis, NodeId(node.rhs)));
                let op = match node.flags {
                    1 => "+",
                    2 => "-",
                    3 => "*",
                    4 | 5 => "/",
                    6 => "%",
                    8 => "==",
                    9 => "!=",
                    10 => "<",
                    11 => "<=",
                    12 => ">",
                    13 => ">=",
                    14 => "and",
                    15 => "or",
                    _ => "+",
                };
                IrExpr::Binary {
                    op: op.to_string(),
                    lhs,
                    rhs,
                }
            }
            NodeKind::UnaryExpr => {
                let expr = Box::new(Self::lower_expr(arena, analysis, NodeId(node.lhs)));
                let op = if node.flags == 1 { "&" } else { "-" };
                IrExpr::Unary {
                    op: op.to_string(),
                    expr,
                }
            }
            NodeKind::CallExpr => {
                let arg_start = node.lhs as usize;
                let arg_len = node.rhs as usize;
                let callee_id = NodeId(arena.extra_children[arg_start]);
                let callee_node = arena.node(callee_id);
                let callee = if callee_node.node_kind() == NodeKind::IdentExpr {
                    arena.interner.resolve(SymbolId(callee_node.lhs)).to_string()
                } else {
                    "anon_fn".to_string()
                };

                let mut args = Vec::new();
                for &a in &arena.extra_children[arg_start + 1..arg_start + arg_len] {
                    args.push(Self::lower_expr(arena, analysis, NodeId(a)));
                }

                IrExpr::Call { callee, args }
            }
            NodeKind::MemberExpr => {
                let target = Box::new(Self::lower_expr(arena, analysis, NodeId(node.lhs)));
                let field = arena.interner.resolve(SymbolId(node.rhs)).to_string();
                IrExpr::Member { target, field }
            }
            NodeKind::IndexExpr => {
                let target = Box::new(Self::lower_expr(arena, analysis, NodeId(node.lhs)));
                let index = Box::new(Self::lower_expr(arena, analysis, NodeId(node.rhs)));
                IrExpr::Index { target, index }
            }
            NodeKind::IfExpr => {
                let cond = Box::new(Self::lower_expr(arena, analysis, NodeId(node.lhs)));
                let then_expr = Box::new(Self::lower_expr(arena, analysis, NodeId(node.rhs)));
                IrExpr::If {
                    cond,
                    then_expr,
                    else_expr: Box::new(IrExpr::LitNull),
                }
            }
            _ => IrExpr::LitNull,
        }
    }
}
