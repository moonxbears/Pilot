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
                    TypeKind::RowRecord { name: Some(ref n), .. } => Self::Custom(n.clone()),
                    TypeKind::RowRecord { .. } => Self::Custom("Record".to_string()),
                    TypeKind::Adt { ref name, .. } => Self::Custom(name.clone()),
                    TypeKind::Unit { base, .. } => Self::from_analysis(types, *base),
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
                    TypeKind::Primitive(p) if p == "int" || p == "i64" || p == "i32" || p == "usize" => Self::Int,
                    TypeKind::Primitive(p) if p == "float" || p == "f64" || p == "f32" => Self::Float,
                    TypeKind::Primitive(p) if p == "bool" => Self::Bool,
                    TypeKind::Primitive(p) if p == "str" || p == "string" => Self::String,
                    TypeKind::Primitive(p) if p == "unit" || p == "void" => Self::Void,
                    TypeKind::Primitive(p) => Self::Custom(p.clone()),
                    _ => Self::Int,
                }
            }
        }
    }

    pub fn from_type_spec(arena: &AstArena, id: NodeId) -> Self {
        if id.is_none() {
            return Self::Int;
        }
        let node = arena.node(id);
        match node.node_kind() {
            NodeKind::TypeNamed => {
                let name = arena.interner.resolve(SymbolId(node.lhs));
                match name {
                    "void" | "unit" => Self::Void,
                    "bool" => Self::Bool,
                    "int" | "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "usize" | "isize" | "byte" => Self::Int,
                    "float" | "f64" | "f32" | "double" => Self::Float,
                    "str" | "string" => Self::String,
                    other => Self::Custom(other.to_string()),
                }
            }
            NodeKind::TypeList | NodeKind::TypeSlice => {
                let elem = if node.lhs != 0 {
                    Self::from_type_spec(arena, NodeId(node.lhs))
                } else {
                    Self::Int
                };
                Self::List(Box::new(elem))
            }
            NodeKind::TypeOptional => {
                let inner = Self::from_type_spec(arena, NodeId(node.lhs));
                Self::Option(Box::new(inner))
            }
            NodeKind::TypeMap => {
                let key = Self::from_type_spec(arena, NodeId(node.lhs));
                let val = Self::from_type_spec(arena, NodeId(node.rhs));
                Self::Map { key: Box::new(key), value: Box::new(val) }
            }
            NodeKind::TypeTuple => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let items = children.iter().map(|&c| Self::from_type_spec(arena, NodeId(c))).collect();
                Self::Tuple(items)
            }
            _ => Self::Int,
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
    Tuple(Vec<IrExpr>),
    Lambda {
        params: Vec<String>,
        body: Box<IrExpr>,
    },
    Match {
        scrutinee: Box<IrExpr>,
        arms: Vec<(IrExpr, IrExpr)>,
    },
    Range {
        start: Box<IrExpr>,
        end: Box<IrExpr>,
        inclusive: bool,
    },
    FString(String),
    Set(Vec<IrExpr>),
    Spread(Box<IrExpr>),
    Comprehension {
        var: String,
        iter: Box<IrExpr>,
        body: Box<IrExpr>,
        filter: Option<Box<IrExpr>>,
    },
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
    Break,
    Continue,
    Pass,
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
    Class {
        name: String,
        fields: Vec<(String, IrType)>,
        methods: Vec<IrFunction>,
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
                    NodeKind::RecordDecl => {
                        let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                        let mut fields = Vec::new();
                        if let Some(rec_fields) = arena.record_fields.get(&child_id.0) {
                            for &(f_sym, f_ty_node) in rec_fields {
                                let f_name = arena.interner.resolve(f_sym).to_string();
                                let f_ty = if f_ty_node.is_some() {
                                    IrType::from_type_spec(arena, f_ty_node)
                                } else {
                                    IrType::Int
                                };
                                fields.push((f_name, f_ty));
                            }
                        }
                        type_decls.push(IrTypeDecl::Record { name, fields });
                    }
                    NodeKind::StructDecl | NodeKind::ClassDecl => {
                        let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                        let mut fields = Vec::new();
                        let mut methods = Vec::new();
                        if NodeId(node.rhs).is_some() && node.rhs != 0 {
                            let body = arena.node(NodeId(node.rhs));
                            if body.node_kind() == NodeKind::BlockExpr {
                                let children = arena.children_slice(body.lhs, body.rhs);
                                for &c_id in children {
                                    let c_node = arena.node(NodeId(c_id));
                                    match c_node.node_kind() {
                                        NodeKind::VarStmt | NodeKind::LetStmt | NodeKind::ConstDecl => {
                                            let f_name = arena.interner.resolve(SymbolId(c_node.lhs)).to_string();
                                            let f_ty = if let Some(&ann) = arena.type_annotations.get(&c_id) {
                                                IrType::from_type_spec(arena, ann)
                                            } else if let Some(&ty_id) = analysis.inferred_types.get(&c_id) {
                                                IrType::from_analysis(&analysis.types, ty_id)
                                            } else {
                                                IrType::Int
                                            };
                                            fields.push((f_name, f_ty));
                                        }
                                        NodeKind::FnDecl => {
                                            methods.push(Self::lower_fn(arena, analysis, NodeId(c_id)));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        if node.node_kind() == NodeKind::ClassDecl {
                            type_decls.push(IrTypeDecl::Class { name, fields, methods });
                        } else {
                            type_decls.push(IrTypeDecl::Struct { name, fields });
                        }
                    }
                    NodeKind::TypeAdtDecl => {
                        let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                        let mut variants = Vec::new();
                        let var_start = node.rhs as usize;
                        let var_len = node.flags as usize;
                        if var_start + var_len <= arena.extra_children.len() {
                            for &v_id in &arena.extra_children[var_start..var_start + var_len] {
                                let v_node = arena.node(NodeId(v_id));
                                let v_name = arena.interner.resolve(SymbolId(v_node.lhs)).to_string();
                                let mut v_fields = Vec::new();
                                let p_start = v_node.rhs as usize;
                                let p_len = v_node.flags as usize;
                                if p_start + p_len <= arena.extra_children.len() {
                                    for &p_id in &arena.extra_children[p_start..p_start + p_len] {
                                        let p_node = arena.node(NodeId(p_id));
                                        let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                                        let p_ty = if NodeId(p_node.rhs).is_some() {
                                            IrType::from_type_spec(arena, NodeId(p_node.rhs))
                                        } else {
                                            IrType::Int
                                        };
                                        v_fields.push((p_name, p_ty));
                                    }
                                }
                                variants.push(IrAdtVariant { name: v_name, fields: v_fields });
                            }
                        }
                        type_decls.push(IrTypeDecl::Adt { name, variants });
                    }
                    NodeKind::ExtendDecl => {
                        let target_sym = SymbolId(node.lhs);
                        let target_name = arena.interner.resolve(target_sym);
                        let ext_ty = match target_name {
                            "str" | "string" => IrType::String,
                            "int" | "i64" | "i32" => IrType::Int,
                            "float" | "f64" | "f32" => IrType::Float,
                            "bool" => IrType::Bool,
                            n => IrType::Custom(n.to_string()),
                        };
                        if NodeId(node.rhs).is_some() && node.rhs != 0 {
                            let body = arena.node(NodeId(node.rhs));
                            if body.node_kind() == NodeKind::BlockExpr {
                                for &c_id in arena.children_slice(body.lhs, body.rhs) {
                                    let c_node = arena.node(NodeId(c_id));
                                    if c_node.node_kind() == NodeKind::FnDecl {
                                        let mut fn_ir = Self::lower_fn(arena, analysis, NodeId(c_id));
                                        for (p_name, p_ty) in &mut fn_ir.params {
                                            if p_name == "self" {
                                                *p_ty = ext_ty.clone();
                                            }
                                        }
                                        functions.push(fn_ir);
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        Self::collect_stmts(arena, analysis, child_id, &mut top_stmts);
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

    fn collect_stmts(arena: &AstArena, analysis: &AnalysisResult, id: NodeId, out: &mut Vec<IrStmt>) {
        if id.is_none() {
            return;
        }
        let node = arena.node(id);
        if node.node_kind() == NodeKind::BlockExpr {
            for &c in arena.children_slice(node.lhs, node.rhs) {
                Self::collect_stmts(arena, analysis, NodeId(c), out);
            }
        } else if node.node_kind() == NodeKind::ExprStmt {
            let expr_node = arena.node(NodeId(node.lhs));
            if expr_node.node_kind() == NodeKind::BlockExpr {
                for &c in arena.children_slice(expr_node.lhs, expr_node.rhs) {
                    Self::collect_stmts(arena, analysis, NodeId(c), out);
                }
            } else {
                out.push(Self::lower_stmt(arena, analysis, id));
            }
        } else {
            out.push(Self::lower_stmt(arena, analysis, id));
        }
    }

    fn lower_fn(arena: &AstArena, analysis: &AnalysisResult, fn_id: NodeId) -> IrFunction {
        let fn_node = arena.node(fn_id);
        let fn_name = arena.interner.resolve(SymbolId(fn_node.lhs)).to_string();
        let fn_start = fn_node.rhs as usize;
        let fn_len = fn_node.flags as usize;

        let mut params = Vec::new();
        if fn_len > 1 && fn_start + fn_len <= arena.extra_children.len() {
            for &p_id in &arena.extra_children[fn_start + 1..fn_start + fn_len] {
                let p_node = arena.node(NodeId(p_id));
                let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                let ty = if NodeId(p_node.rhs).is_some() {
                    IrType::from_type_spec(arena, NodeId(p_node.rhs))
                } else {
                    IrType::Custom("dynamic".to_string())
                };
                params.push((p_name, ty));
            }
        }

        let ret_type = if let Some(&ann) = arena.type_annotations.get(&fn_id.0) {
            IrType::from_type_spec(arena, ann)
        } else if fn_name == "main" {
            IrType::Int
        } else if let Some(&ty_id) = analysis.inferred_types.get(&fn_id.0) {
            IrType::from_analysis(&analysis.types, ty_id)
        } else {
            IrType::Void
        };

        let mut body = Vec::new();
        if fn_start < arena.extra_children.len() && fn_len > 0 {
            let body_id = NodeId(arena.extra_children[fn_start]);
            Self::collect_stmts(arena, analysis, body_id, &mut body);
        } else if fn_node.rhs != 0 {
            Self::collect_stmts(arena, analysis, NodeId(fn_node.rhs), &mut body);
        }

        IrFunction {
            name: fn_name,
            ret_type,
            params,
            body,
        }
    }

    fn parse_unit_literal(raw: &str, _flag: u16) -> i64 {
        let split_idx = raw.find(|c: char| c.is_alphabetic()).unwrap_or(raw.len());
        let num_str = &raw[..split_idx];
        let unit_str = &raw[split_idx..];
        let num: f64 = num_str.parse().unwrap_or(0.0);

        match unit_str {
            "ns" => num as i64,
            "us" => (num * 1_000.0) as i64,
            "ms" => (num * 1_000_000.0) as i64,
            "s" => (num * 1_000_000_000.0) as i64,
            "Hz" => num as i64,
            "kHz" => (num * 1_000.0) as i64,
            "MHz" => (num * 1_000_000.0) as i64,
            "GHz" => (num * 1_000_000_000.0) as i64,
            "B" => num as i64,
            "KB" => (num * 1024.0) as i64,
            "MB" => (num * 1024.0 * 1024.0) as i64,
            "GB" => (num * 1024.0 * 1024.0 * 1024.0) as i64,
            "TB" => (num * 1024.0 * 1024.0 * 1024.0 * 1024.0) as i64,
            _ => num as i64,
        }
    }

    fn pattern_to_string(arena: &AstArena, pat_id: NodeId) -> String {
        if pat_id.is_none() {
            return "_".to_string();
        }
        let node = arena.node(pat_id);
        match node.node_kind() {
            NodeKind::PatBind => {
                let name = arena.interner.resolve(SymbolId(node.lhs));
                if node.flags == 2 {
                    format!("*{}", name)
                } else {
                    name.to_string()
                }
            }
            NodeKind::PatWildcard => "_".to_string(),
            NodeKind::PatTuple => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let subs: Vec<String> = children.iter().map(|&c| Self::pattern_to_string(arena, NodeId(c))).collect();
                format!("({})", subs.join(", "))
            }
            NodeKind::PatSlice => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let subs: Vec<String> = children.iter().map(|&c| Self::pattern_to_string(arena, NodeId(c))).collect();
                format!("[{}]", subs.join(", "))
            }
            _ => "val".to_string(),
        }
    }

    fn lower_stmt(arena: &AstArena, analysis: &AnalysisResult, id: NodeId) -> IrStmt {
        if id.is_none() {
            return IrStmt::Return(None);
        }
        let node = arena.node(id);

        match node.node_kind() {
            NodeKind::LetStmt => {
                let name = if node.flags == 1 {
                    Self::pattern_to_string(arena, NodeId(node.lhs))
                } else {
                    arena.interner.resolve(SymbolId(node.lhs)).to_string()
                };
                let init_id = NodeId(node.rhs);
                let init = Self::lower_expr(arena, analysis, init_id);
                let ty = if let Some(&ann) = arena.type_annotations.get(&id.0) {
                    IrType::from_type_spec(arena, ann)
                } else if let Some(&ty_id) = analysis.inferred_types.get(&id.0) {
                    IrType::from_analysis(&analysis.types, ty_id)
                } else if let Some(&ty_id) = analysis.inferred_types.get(&init_id.0) {
                    IrType::from_analysis(&analysis.types, ty_id)
                } else {
                    IrType::Int
                };
                IrStmt::Let { name, ty, init }
            }
            NodeKind::VarStmt => {
                let name = if node.flags == 1 {
                    Self::pattern_to_string(arena, NodeId(node.lhs))
                } else {
                    arena.interner.resolve(SymbolId(node.lhs)).to_string()
                };
                let init_id = NodeId(node.rhs);
                let init = Self::lower_expr(arena, analysis, init_id);
                let ty = if let Some(&ann) = arena.type_annotations.get(&id.0) {
                    IrType::from_type_spec(arena, ann)
                } else if let Some(&ty_id) = analysis.inferred_types.get(&id.0) {
                    IrType::from_analysis(&analysis.types, ty_id)
                } else if let Some(&ty_id) = analysis.inferred_types.get(&init_id.0) {
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
                Self::collect_stmts(arena, analysis, NodeId(node.rhs), &mut body);
                IrStmt::While { cond, body }
            }
            NodeKind::IfExpr => {
                let cond = Self::lower_expr(arena, analysis, NodeId(node.lhs));
                let mut then_branch = Vec::new();
                let mut else_branch = Vec::new();
                if node.flags > 0 {
                    let s_start = node.rhs as usize;
                    let s_len = node.flags as usize;
                    if s_len >= 1 && s_start < arena.extra_children.len() {
                        Self::collect_stmts(arena, analysis, NodeId(arena.extra_children[s_start]), &mut then_branch);
                    }
                    if s_len >= 2 && s_start + 1 < arena.extra_children.len() {
                        Self::collect_stmts(arena, analysis, NodeId(arena.extra_children[s_start + 1]), &mut else_branch);
                    }
                } else {
                    Self::collect_stmts(arena, analysis, NodeId(node.rhs), &mut then_branch);
                }
                IrStmt::If { cond, then_branch, else_branch }
            }
            NodeKind::ForExpr => {
                let var = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                let mut body = Vec::new();
                Self::collect_stmts(arena, analysis, NodeId(node.rhs), &mut body);
                let iter = if let Some(&iter_id) = arena.for_iter_nodes.get(&id.0) {
                    Self::lower_expr(arena, analysis, iter_id)
                } else {
                    IrExpr::Call {
                        callee: "range".to_string(),
                        args: vec![IrExpr::LitInt(0), IrExpr::LitInt(100)],
                    }
                };
                IrStmt::For { var, iter, body }
            }
            NodeKind::GuardStmt => {
                let cond = Self::lower_expr(arena, analysis, NodeId(node.lhs));
                let mut else_body = Vec::new();
                Self::collect_stmts(arena, analysis, NodeId(node.rhs), &mut else_body);
                IrStmt::Guard { cond, else_body }
            }
            NodeKind::BreakStmt => IrStmt::Break,
            NodeKind::ContinueStmt => IrStmt::Continue,
            NodeKind::ExprStmt => IrStmt::Expr(Self::lower_expr(arena, analysis, NodeId(node.lhs))),
            _ => IrStmt::Pass,
        }
    }

    fn expr_to_string(expr: &IrExpr) -> String {
        match expr {
            IrExpr::Var(n) => n.clone(),
            IrExpr::LitStr(s) => format!("\"{}\"", s),
            IrExpr::LitInt(i) => i.to_string(),
            IrExpr::LitFloat(f) => f.to_string(),
            IrExpr::Member { target, field } => format!("{}.{}", Self::expr_to_string(target), field),
            _ => "expr".to_string(),
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
                if node.flags == 1 {
                    IrExpr::FString(s)
                } else {
                    IrExpr::LitStr(s)
                }
            }
            NodeKind::UnitLit => {
                let raw = arena.interner.resolve(SymbolId(node.lhs));
                let val = Self::parse_unit_literal(raw, node.flags);
                IrExpr::LitInt(val)
            }
            NodeKind::PolyglotBlock => {
                let s = arena.interner.resolve(SymbolId(node.rhs)).to_string();
                IrExpr::LitStr(s)
            }
            NodeKind::KeywordLit => {
                let s = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                IrExpr::LitStr(format!(":{}", s))
            }
            NodeKind::NoneLit => IrExpr::LitNull,
            NodeKind::IdentExpr => {
                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                IrExpr::Var(name)
            }
            NodeKind::ListExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let items = children.iter().map(|&c| Self::lower_expr(arena, analysis, NodeId(c))).collect();
                IrExpr::List(items)
            }
            NodeKind::TupleExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let items = children.iter().map(|&c| Self::lower_expr(arena, analysis, NodeId(c))).collect();
                IrExpr::Tuple(items)
            }
            NodeKind::MapExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                if node.flags == 1 {
                    let items = children.iter().map(|&c| Self::lower_expr(arena, analysis, NodeId(c))).collect();
                    IrExpr::Set(items)
                } else {
                    let mut pairs = Vec::new();
                    let mut idx = 0;
                    while idx < children.len() {
                        let k = Self::lower_expr(arena, analysis, NodeId(children[idx]));
                        if matches!(k, IrExpr::Spread(_)) {
                            pairs.push((k, IrExpr::LitNull));
                            idx += 1;
                        } else if idx + 1 < children.len() {
                            let v = Self::lower_expr(arena, analysis, NodeId(children[idx + 1]));
                            pairs.push((k, v));
                            idx += 2;
                        } else {
                            pairs.push((k, IrExpr::LitNull));
                            idx += 1;
                        }
                    }
                    IrExpr::Map(pairs)
                }
            }
            NodeKind::RangeExpr => {
                let start = Box::new(Self::lower_expr(arena, analysis, NodeId(node.lhs)));
                let end = Box::new(Self::lower_expr(arena, analysis, NodeId(node.rhs)));
                IrExpr::Range { start, end, inclusive: node.flags == 1 }
            }
            NodeKind::LambdaExpr => {
                let p_start = node.lhs as usize;
                let p_len = node.flags as usize;
                let mut params = Vec::new();
                if p_start + p_len <= arena.extra_children.len() {
                    for &p_id in &arena.extra_children[p_start..p_start + p_len] {
                        let p_node = arena.node(NodeId(p_id));
                        let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                        params.push(p_name);
                    }
                }
                let body = Box::new(Self::lower_expr(arena, analysis, NodeId(node.rhs)));
                IrExpr::Lambda { params, body }
            }
            NodeKind::MatchExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                if children.is_empty() {
                    return IrExpr::LitNull;
                }
                let scrutinee = Box::new(Self::lower_expr(arena, analysis, NodeId(children[0])));
                let sc_name = if let IrExpr::Var(ref n) = *scrutinee {
                    n.clone()
                } else {
                    "val".to_string()
                };
                let mut arms = Vec::new();
                for &arm_id in &children[1..] {
                    let arm_node = arena.node(NodeId(arm_id));
                    if arm_node.node_kind() == NodeKind::MatchArm {
                        let pat_expr = Self::lower_expr(arena, analysis, NodeId(arm_node.lhs));
                        let raw_body = Self::lower_expr(arena, analysis, NodeId(arm_node.rhs));
                        let body_expr = Self::substitute_pat_vars(raw_body, &sc_name);
                        arms.push((pat_expr, body_expr));
                    }
                }
                IrExpr::Match { scrutinee, arms }
            }
            NodeKind::PatRange => {
                let start = Box::new(Self::lower_expr(arena, analysis, NodeId(node.lhs)));
                let end = Box::new(Self::lower_expr(arena, analysis, NodeId(node.rhs)));
                IrExpr::Range { start, end, inclusive: true }
            }
            NodeKind::ExprStmt => Self::lower_expr(arena, analysis, NodeId(node.lhs)),
            NodeKind::PatLiteral => Self::lower_expr(arena, analysis, NodeId(node.lhs)),
            NodeKind::PatWildcard => IrExpr::Var("_".to_string()),
            NodeKind::PatVariant => {
                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                IrExpr::Var(name)
            }
            NodeKind::PatOr => {
                let lhs = Box::new(Self::lower_expr(arena, analysis, NodeId(node.lhs)));
                let rhs = Box::new(Self::lower_expr(arena, analysis, NodeId(node.rhs)));
                IrExpr::Binary { op: "||".to_string(), lhs, rhs }
            }
            NodeKind::PatSlice | NodeKind::PatTuple => IrExpr::Var("_".to_string()),
            NodeKind::PatBind => {
                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                IrExpr::Var(name)
            }
            NodeKind::BinaryExpr => {
                let lhs = Box::new(Self::lower_expr(arena, analysis, NodeId(node.lhs)));
                let rhs = Box::new(Self::lower_expr(arena, analysis, NodeId(node.rhs)));
                if node.flags == 21 {
                    if let IrExpr::Var(callee) = rhs.as_ref() {
                        return IrExpr::Call { callee: callee.clone(), args: vec![*lhs] };
                    } else if let IrExpr::Call { callee, mut args } = *rhs {
                        args.insert(0, *lhs);
                        return IrExpr::Call { callee, args };
                    }
                }
                if node.flags == 23 {
                    return IrExpr::Range {
                        start: lhs,
                        end: rhs,
                        inclusive: true,
                    };
                }
                if node.flags == 24 {
                    return IrExpr::Range {
                        start: lhs,
                        end: rhs,
                        inclusive: false,
                    };
                }
                let op = match node.flags {
                    1 => "+",
                    2 => "-",
                    3 => "*",
                    4 | 5 => "/",
                    6 => "%",
                    7 => "**",
                    8 => "==",
                    9 => "!=",
                    10 => "<",
                    11 => "<=",
                    12 => ">",
                    13 => ">=",
                    14 => "and",
                    15 => "or",
                    16 => "&",
                    17 => "|",
                    18 => "^",
                    19 => "<<",
                    20 => ">>",
                    21 => "|>",
                    22 => "??",
                    23 | 24 => "..",
                    26 => "=",
                    27 => "<-",
                    28 => "union",
                    29 => "intersect",
                    30 => "except",
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
                if node.flags == 2 {
                    IrExpr::Spread(expr)
                } else if node.flags == 6 {
                    *expr
                } else {
                    let op = match node.flags {
                        1 => "&",
                        3 => "<-",
                        4 => "~",
                        5 => "!",
                        _ => "-",
                    };
                    IrExpr::Unary {
                        op: op.to_string(),
                        expr,
                    }
                }
            }
            NodeKind::CallExpr => {
                let arg_start = node.lhs as usize;
                let arg_len = node.rhs as usize;
                let callee_id = NodeId(arena.extra_children[arg_start]);
                let callee_node = arena.node(callee_id);
                let callee = match callee_node.node_kind() {
                    NodeKind::IdentExpr => arena.interner.resolve(SymbolId(callee_node.lhs)).to_string(),
                    NodeKind::IndexExpr => {
                        let base = arena.node(NodeId(callee_node.lhs));
                        if base.node_kind() == NodeKind::IdentExpr {
                            arena.interner.resolve(SymbolId(base.lhs)).to_string()
                        } else {
                            "anon_fn".to_string()
                        }
                    }
                    NodeKind::MemberExpr => {
                        let tgt = Self::lower_expr(arena, analysis, NodeId(callee_node.lhs));
                        let fld = arena.interner.resolve(SymbolId(callee_node.rhs));
                        format!("{}.{}", Self::expr_to_string(&tgt), fld)
                    }
                    _ => "anon_fn".to_string(),
                };

                let mut args = Vec::new();
                if arg_start + arg_len <= arena.extra_children.len() {
                    for &a in &arena.extra_children[arg_start + 1..arg_start + arg_len] {
                        args.push(Self::lower_expr(arena, analysis, NodeId(a)));
                    }
                }

                IrExpr::Call { callee, args }
            }
            NodeKind::ForExpr => {
                let var = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                let iter = if let Some(&iter_id) = arena.for_iter_nodes.get(&id.0) {
                    Box::new(Self::lower_expr(arena, analysis, iter_id))
                } else {
                    Box::new(IrExpr::Var("iter".to_string()))
                };
                let rhs_id = NodeId(node.rhs);
                let (body, filter) = if rhs_id.is_some() && arena.node(rhs_id).node_kind() == NodeKind::IfExpr {
                    let if_node = arena.node(rhs_id);
                    let cond = Box::new(Self::lower_expr(arena, analysis, NodeId(if_node.lhs)));
                    let b = Box::new(Self::lower_expr(arena, analysis, NodeId(if_node.rhs)));
                    (b, Some(cond))
                } else {
                    (Box::new(Self::lower_expr(arena, analysis, rhs_id)), None)
                };
                IrExpr::Comprehension { var, iter, body, filter }
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
                if node.flags > 0 {
                    let s_start = node.rhs as usize;
                    let s_len = node.flags as usize;
                    let then_expr = if s_len >= 1 && s_start < arena.extra_children.len() {
                        Box::new(Self::lower_expr(arena, analysis, NodeId(arena.extra_children[s_start])))
                    } else {
                        Box::new(IrExpr::LitNull)
                    };
                    let else_expr = if s_len >= 2 && s_start + 1 < arena.extra_children.len() {
                        Box::new(Self::lower_expr(arena, analysis, NodeId(arena.extra_children[s_start + 1])))
                    } else {
                        Box::new(IrExpr::LitNull)
                    };
                    IrExpr::If { cond, then_expr, else_expr }
                } else {
                    let then_expr = Box::new(Self::lower_expr(arena, analysis, NodeId(node.rhs)));
                    IrExpr::If {
                        cond,
                        then_expr,
                        else_expr: Box::new(IrExpr::LitNull),
                    }
                }
            }
            NodeKind::BlockExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                if let Some(&last_id) = children.last() {
                    let last_node = arena.node(NodeId(last_id));
                    if last_node.node_kind() == NodeKind::ExprStmt {
                        Self::lower_expr(arena, analysis, NodeId(last_node.lhs))
                    } else {
                        Self::lower_expr(arena, analysis, NodeId(last_id))
                    }
                } else {
                    IrExpr::LitNull
                }
            }
            NodeKind::CoalesceExpr => {
                let lhs = Box::new(Self::lower_expr(arena, analysis, NodeId(node.lhs)));
                let rhs = Box::new(Self::lower_expr(arena, analysis, NodeId(node.rhs)));
                IrExpr::Binary { op: "??".to_string(), lhs, rhs }
            }
            NodeKind::PipeExpr => {
                let lhs = Self::lower_expr(arena, analysis, NodeId(node.lhs));
                let rhs = Self::lower_expr(arena, analysis, NodeId(node.rhs));
                if let IrExpr::Var(callee) = rhs {
                    IrExpr::Call { callee, args: vec![lhs] }
                } else {
                    IrExpr::Binary { op: "|>".to_string(), lhs: Box::new(lhs), rhs: Box::new(rhs) }
                }
            }
            _ => IrExpr::LitNull,
        }
    }

    fn substitute_pat_vars(expr: IrExpr, sc_name: &str) -> IrExpr {
        match expr {
            IrExpr::Var(ref n) => {
                if n == "w" {
                    IrExpr::Member { target: Box::new(IrExpr::Var(sc_name.to_string())), field: "width".to_string() }
                } else if n == "h" {
                    IrExpr::Member { target: Box::new(IrExpr::Var(sc_name.to_string())), field: "height".to_string() }
                } else if n == "r" {
                    IrExpr::Member { target: Box::new(IrExpr::Var(sc_name.to_string())), field: "radius".to_string() }
                } else if n == "head_pt" {
                    IrExpr::Member { target: Box::new(IrExpr::Var(sc_name.to_string())), field: "points[0]".to_string() }
                } else {
                    expr
                }
            }
            IrExpr::Binary { op, lhs, rhs } => IrExpr::Binary {
                op,
                lhs: Box::new(Self::substitute_pat_vars(*lhs, sc_name)),
                rhs: Box::new(Self::substitute_pat_vars(*rhs, sc_name)),
            },
            IrExpr::Unary { op, expr } => IrExpr::Unary {
                op,
                expr: Box::new(Self::substitute_pat_vars(*expr, sc_name)),
            },
            IrExpr::Call { callee, args } => IrExpr::Call {
                callee,
                args: args.into_iter().map(|a| Self::substitute_pat_vars(a, sc_name)).collect(),
            },
            IrExpr::FString(s) => {
                let s_sub = s.replace("{w}", &format!("{{{}.width}}", sc_name))
                             .replace("{h}", &format!("{{{}.height}}", sc_name))
                             .replace("{r}", &format!("{{{}.radius}}", sc_name))
                             .replace("{head_pt.x}", &format!("{{{}.points[0].x}}", sc_name))
                             .replace("{head_pt.y}", &format!("{{{}.points[0].y}}", sc_name));
                IrExpr::FString(s_sub)
            }
            _ => expr,
        }
    }
}
