//! Milestone 33.1 & 37.1: Flow-Sensitive Bidirectional Type Inference, Complete AstArena Coverage & Narrowing.

use std::collections::HashMap;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;
use crate::diagnostics::AnalysisDiagnostic;
use crate::types::{TypeArena, TypeId, TypeKind};

pub struct Scope {
    pub bindings: HashMap<String, TypeId>,
    pub narrowed_types: HashMap<String, TypeId>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            narrowed_types: HashMap::new(),
        }
    }
}

pub struct TypeInferenceEngine {
    pub types: TypeArena,
    scopes: Vec<Scope>,
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub inferred_node_types: HashMap<u32, TypeId>,
    pub type_defs: HashMap<String, TypeId>,
    pub struct_fields: HashMap<String, Vec<(String, TypeId)>>,
    pub adt_variants: HashMap<String, (String, Vec<TypeId>)>, // variant -> (adt_name, field_types)
}

impl Default for TypeInferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeInferenceEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            types: TypeArena::new(),
            scopes: vec![Scope::new()],
            diagnostics: Vec::new(),
            inferred_node_types: HashMap::new(),
            type_defs: HashMap::new(),
            struct_fields: HashMap::new(),
            adt_variants: HashMap::new(),
        };

        engine.init_builtins();
        engine
    }

    fn init_builtins(&mut self) {
        // Standard primitives in type_defs
        self.type_defs.insert("int".to_string(), TypeId::I64);
        self.type_defs.insert("i64".to_string(), TypeId::I64);
        self.type_defs.insert("i32".to_string(), TypeId::I32);
        self.type_defs.insert("float".to_string(), TypeId::F64);
        self.type_defs.insert("f64".to_string(), TypeId::F64);
        self.type_defs.insert("f32".to_string(), TypeId::F32);
        self.type_defs.insert("bool".to_string(), TypeId::BOOL);
        self.type_defs.insert("str".to_string(), TypeId::STR);
        self.type_defs.insert("string".to_string(), TypeId::STR);
        self.type_defs.insert("bytes".to_string(), TypeId::STR);
        self.type_defs.insert("char".to_string(), TypeId::I64);
        self.type_defs.insert("int64_t".to_string(), TypeId::I64);
        self.type_defs.insert("uint64_t".to_string(), TypeId::I64);
        self.type_defs.insert("timespec".to_string(), TypeId::ANY);
        self.type_defs.insert("struct".to_string(), TypeId::ANY);
        self.type_defs.insert("void".to_string(), TypeId::VOID);
        self.type_defs.insert("any".to_string(), TypeId::ANY);

        // Standard functions in scope
        let fn_print = self.types.function_of(vec![TypeId::ANY], TypeId::VOID);
        self.define_var("print", fn_print);
        self.define_var("println", fn_print);

        let fn_assert = self.types.function_of(vec![TypeId::BOOL], TypeId::VOID);
        self.define_var("assert", fn_assert);

        let fn_len = self.types.function_of(vec![TypeId::ANY], TypeId::I64);
        self.define_var("len", fn_len);

        let int_list = self.types.list_of(TypeId::I64);
        let fn_range = self.types.function_of(vec![TypeId::I64], int_list);
        self.define_var("range", fn_range);

        let fn_malloc = self.types.function_of(vec![TypeId::I64], TypeId::ANY);
        self.define_var("malloc", fn_malloc);

        let fn_free = self.types.function_of(vec![TypeId::ANY], TypeId::VOID);
        self.define_var("free", fn_free);

        let fn_memset = self.types.function_of(vec![TypeId::ANY, TypeId::I64, TypeId::I64], TypeId::ANY);
        self.define_var("memset", fn_memset);

        let fn_clock = self.types.function_of(vec![TypeId::I64, TypeId::ANY], TypeId::I64);
        self.define_var("clock_gettime", fn_clock);

        let fn_puts = self.types.function_of(vec![TypeId::ANY], TypeId::VOID);
        self.define_var("puts", fn_puts);

        let fn_printf = self.types.function_of(vec![TypeId::ANY], TypeId::VOID);
        self.define_var("printf", fn_printf);

        let fn_sprintf = self.types.function_of(vec![TypeId::ANY], TypeId::I64);
        self.define_var("sprintf", fn_sprintf);

        let fn_fopen = self.types.function_of(vec![TypeId::ANY, TypeId::ANY], TypeId::ANY);
        self.define_var("fopen", fn_fopen);

        let fn_fclose = self.types.function_of(vec![TypeId::ANY], TypeId::I64);
        self.define_var("fclose", fn_fclose);

        let fn_fputs = self.types.function_of(vec![TypeId::ANY, TypeId::ANY], TypeId::I64);
        self.define_var("fputs", fn_fputs);

        let fn_fgets = self.types.function_of(vec![TypeId::ANY, TypeId::ANY, TypeId::ANY], TypeId::ANY);
        self.define_var("fgets", fn_fgets);

        let fn_strcmp = self.types.function_of(vec![TypeId::ANY, TypeId::ANY], TypeId::I64);
        self.define_var("strcmp", fn_strcmp);

        let fn_strlen = self.types.function_of(vec![TypeId::ANY], TypeId::I64);
        self.define_var("strlen", fn_strlen);

        let fn_system = self.types.function_of(vec![TypeId::ANY], TypeId::I64);
        self.define_var("system", fn_system);

        let fn_exit = self.types.function_of(vec![TypeId::I64], TypeId::VOID);
        self.define_var("exit", fn_exit);
        self.define_var("CLOCK_MONOTONIC", TypeId::I64);

        // Result and Option constructors
        let fn_ok = self.types.function_of(vec![TypeId::ANY], TypeId::ANY);
        self.define_var("Ok", fn_ok);
        let fn_err = self.types.function_of(vec![TypeId::ANY], TypeId::ANY);
        self.define_var("Err", fn_err);
        let fn_some = self.types.function_of(vec![TypeId::ANY], TypeId::ANY);
        self.define_var("Some", fn_some);
        self.define_var("None", TypeId::NULL);
        self.define_var("null", TypeId::NULL);
        self.define_var("True", TypeId::BOOL);
        self.define_var("False", TypeId::BOOL);

        self.define_var("some", fn_some);
        self.define_var("none", TypeId::NULL);
        self.define_var("nil", TypeId::NULL);
        self.define_var("ok", fn_ok);
        self.define_var("err", fn_err);
        self.define_var("true", TypeId::BOOL);
        self.define_var("false", TypeId::BOOL);

        // Concurrency & Async
        let fn_async = self.types.function_of(vec![TypeId::ANY], TypeId::ANY);
        self.define_var("async", fn_async);
        self.define_var("await", fn_async);
        self.define_var("spawn", fn_async);
        self.define_var("atomic", fn_async);
        self.define_var("sync", fn_async);
        self.define_var("isolate", fn_async);
        self.define_var("select", fn_async);
        self.define_var("parallel", fn_async);
        self.define_var("timeout", fn_async);
        self.define_var("yield_execution", fn_async);

        // Realtime
        self.define_var("deadline", fn_async);
        self.define_var("within", fn_async);
        self.define_var("periodic", fn_async);
        self.define_var("priority", fn_async);
        self.define_var("jitter", fn_async);
        self.define_var("stackalloc", fn_async);
        self.define_var("bounded", fn_async);
        self.define_var("on_missed", fn_async);
        self.define_var("fallback", fn_async);

        // Meta, Keywords & DSL
        self.define_var("default", fn_async);
        self.define_var("keyword", fn_async);
        self.define_var("defkeyword", fn_async);
        self.define_var("pure", fn_async);
        self.define_var("noalloc", fn_async);
        self.define_var("unsafe", fn_async);
        self.define_var("current_scope", TypeId::ANY);

        // Polyglot
        self.define_var("js", fn_async);
        self.define_var("lua", fn_async);
        self.define_var("python", fn_async);
        self.define_var("java", fn_async);
        self.define_var("c", fn_async);
        self.define_var("cpp", fn_async);
        self.define_var("asm", fn_async);
        self.define_var("rust", fn_async);
        self.define_var("csharp", fn_async);
        self.define_var("sql", fn_async);
        self.define_var("postgres", fn_async);
        self.define_var("postgresql", fn_async);
        self.define_var("mysql", fn_async);
        self.define_var("sqlite", fn_async);

        // Types and math constants
        self.define_var("PI", TypeId::F64);
        self.define_var("E", TypeId::F64);
        self.define_var("Math", TypeId::ANY);
        self.define_var("undefined", TypeId::ANY);
        self.define_var("Channel", fn_async);
        self.define_var("Chan", fn_async);
        self.define_var("old", fn_async);
        self.define_var("result", TypeId::ANY);
        self.define_var("Exception", TypeId::ANY);
        self.define_var("Error", TypeId::ANY);
        self.define_var("DivideByZeroError", TypeId::ANY);
        self.define_var("NullReferenceError", TypeId::ANY);
        self.define_var("TypeError", TypeId::ANY);
        self.define_var("AssertionError", TypeId::ANY);
        self.type_defs.insert("Channel".to_string(), TypeId::ANY);
        self.type_defs.insert("Chan".to_string(), TypeId::ANY);
        self.type_defs.insert("Exception".to_string(), TypeId::ANY);
        self.type_defs.insert("Error".to_string(), TypeId::ANY);
        self.type_defs.insert("DivideByZeroError".to_string(), TypeId::ANY);
        self.type_defs.insert("NullReferenceError".to_string(), TypeId::ANY);
        self.type_defs.insert("TypeError".to_string(), TypeId::ANY);
        self.type_defs.insert("AssertionError".to_string(), TypeId::ANY);
        self.type_defs.insert("list".to_string(), self.types.list_of(TypeId::ANY));
        self.type_defs.insert("map".to_string(), TypeId::ANY);
        self.type_defs.insert("tuple".to_string(), TypeId::ANY);
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn define_var(&mut self, name: &str, ty: TypeId) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.insert(name.to_string(), ty);
        }
    }

    pub fn narrow_var(&mut self, name: &str, narrowed_ty: TypeId) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.narrowed_types.insert(name.to_string(), narrowed_ty);
        }
    }

    pub fn lookup_var(&self, name: &str) -> Option<TypeId> {
        for scope in self.scopes.iter().rev() {
            if let Some(&narrowed) = scope.narrowed_types.get(name) {
                return Some(narrowed);
            }
            if let Some(&ty) = scope.bindings.get(name) {
                return Some(ty);
            }
        }
        if name == "self" || name == "this" {
            return Some(TypeId::ANY);
        }
        self.type_defs.get(name).copied()
    }

    pub fn resolve_type_spec(&mut self, arena: &AstArena, node_id: NodeId) -> TypeId {
        if node_id.is_none() {
            return TypeId::ANY;
        }

        let node = *arena.node(node_id);
        match node.node_kind() {
            NodeKind::TypeNamed => {
                let name = arena.interner.resolve(SymbolId(node.lhs));
                if let Some(&ty) = self.type_defs.get(name) {
                    ty
                } else {
                    let ty = self.types.alloc(TypeKind::Primitive(name.to_string()));
                    self.type_defs.insert(name.to_string(), ty);
                    ty
                }
            }
            NodeKind::TypeOptional => {
                let inner = self.resolve_type_spec(arena, NodeId(node.lhs));
                self.types.option_of(inner)
            }
            NodeKind::TypeFallible => {
                let ok = self.resolve_type_spec(arena, NodeId(node.lhs));
                let err = self.resolve_type_spec(arena, NodeId(node.rhs));
                self.types.result_of(ok, err)
            }
            NodeKind::TypeList | NodeKind::TypeSlice => {
                let elem = self.resolve_type_spec(arena, NodeId(node.lhs));
                self.types.list_of(elem)
            }
            NodeKind::TypeMap => {
                let key = self.resolve_type_spec(arena, NodeId(node.lhs));
                let val = self.resolve_type_spec(arena, NodeId(node.rhs));
                self.types.map_of(key, val)
            }
            NodeKind::TypeTuple => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let elems = children
                    .iter()
                    .map(|&c| self.resolve_type_spec(arena, NodeId(c)))
                    .collect();
                self.types.tuple_of(elems)
            }
            NodeKind::TypeUnion => {
                let lhs = self.resolve_type_spec(arena, NodeId(node.lhs));
                let rhs = self.resolve_type_spec(arena, NodeId(node.rhs));
                self.types.alloc(TypeKind::Union(vec![lhs, rhs]))
            }
            NodeKind::TypeIntersection => {
                let lhs = self.resolve_type_spec(arena, NodeId(node.lhs));
                let rhs = self.resolve_type_spec(arena, NodeId(node.rhs));
                self.types.alloc(TypeKind::Union(vec![lhs, rhs]))
            }
            NodeKind::TypeFn => {
                let lhs = self.resolve_type_spec(arena, NodeId(node.lhs));
                let ret = self.resolve_type_spec(arena, NodeId(node.rhs));
                self.types.function_of(vec![lhs], ret)
            }
            _ => TypeId::ANY,
        }
    }

    /// Bidirectional: synthesizes the type of an AST expression or declaration node.
    pub fn synth(&mut self, arena: &AstArena, node_id: NodeId) -> TypeId {
        if node_id.is_none() {
            return TypeId::VOID;
        }

        let node = *arena.node(node_id);
        let ty = match node.node_kind() {
            // --- Literals ---
            NodeKind::IntLit => TypeId::I64,
            NodeKind::FloatLit => TypeId::F64,
            NodeKind::StrLit => TypeId::STR,
            NodeKind::BoolLit => TypeId::BOOL,
            NodeKind::NoneLit => TypeId::NULL,
            NodeKind::KeywordLit => self.types.alloc(TypeKind::Primitive("keyword".to_string())),
            NodeKind::UnitLit => self.types.alloc(TypeKind::Unit {
                base: TypeId::I64,
                unit_name: "dim".to_string(),
            }),
            NodeKind::PolyglotBlock => {
                if (node.flags & 1) != 0 {
                    self.types.function_of(vec![TypeId::ANY], TypeId::ANY)
                } else {
                    TypeId::ANY
                }
            },

            // --- Identifier & Variable Lookup ---
            NodeKind::IdentExpr => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym);
                if name == "_" {
                    TypeId::ANY
                } else if let Some(t) = self.lookup_var(name) {
                    t
                } else if let Some(&t) = self.type_defs.get(name) {
                    t
                } else if self.adt_variants.contains_key(name) {
                    let (adt_name, _) = self.adt_variants[name].clone();
                    self.type_defs.get(&adt_name).copied().unwrap_or(TypeId::ANY)
                } else {
                    self.diagnostics.push(AnalysisDiagnostic::error(
                        "TYPE001",
                        format!("Unresolved identifier '{}'", name),
                        arena.span(node_id),
                    ));
                    TypeId::ANY
                }
            }

            // --- Arithmetic & Logical Binary Expressions ---
            NodeKind::BinaryExpr => {
                let op = node.flags;
                if op == 26 {
                    // BinaryOp::Walrus (pat := val or let pat := val)
                    let val_ty = self.synth(arena, NodeId(node.rhs));
                    let lhs = NodeId(node.lhs);
                    self.bind_pattern_vars(arena, lhs);
                    let lhs_node = *arena.node(lhs);
                    if lhs_node.node_kind() == NodeKind::IdentExpr || lhs_node.node_kind() == NodeKind::PatBind {
                        let name = arena.interner.resolve(SymbolId(lhs_node.lhs));
                        self.define_var(name, val_ty);
                    }
                    return val_ty;
                }
                if op == 25 {
                    // BinaryOp::Is (e.g. s is Circle(r))
                    let rhs = NodeId(node.rhs);
                    let rhs_node = *arena.node(rhs);
                    if rhs_node.node_kind() == NodeKind::CallExpr {
                        let arg_start = rhs_node.lhs as usize;
                        let arg_len = rhs_node.rhs as usize;
                        if arg_len > 1 && arg_start + arg_len <= arena.extra_children.len() {
                            for &arg_id in &arena.extra_children[arg_start + 1..arg_start + arg_len] {
                                let a_node = *arena.node(NodeId(arg_id));
                                if a_node.node_kind() == NodeKind::IdentExpr {
                                    let a_name = arena.interner.resolve(SymbolId(a_node.lhs));
                                    self.define_var(a_name, TypeId::ANY);
                                }
                            }
                        }
                    } else {
                        self.bind_pattern_vars(arena, rhs);
                    }
                    return TypeId::BOOL;
                }
                let lhs_ty = self.synth(arena, NodeId(node.lhs));
                let rhs_ty = self.synth(arena, NodeId(node.rhs));
                // Comparison operators (Eq=8..Ge=13) and logical operators (And=14, Or=15)
                if (8..=15).contains(&op) {
                    let _ = self.types.unify(lhs_ty, rhs_ty);
                    TypeId::BOOL
                } else if op == 1 && lhs_ty == TypeId::STR {
                    // String concatenation (str + any -> str)
                    TypeId::STR
                } else if (lhs_ty.is_integer() && rhs_ty.is_float()) || (lhs_ty.is_float() && rhs_ty.is_integer()) {
                    // Automatic numeric widening: int <op> float -> f64
                    TypeId::F64
                } else if op == 17 || op == 16 || op == 2 {
                    // Algebraic class union (|), intersection (&), and subtraction (-)
                    lhs_ty
                } else if op == 21 {
                    // BinaryOp::Pipe (|>)
                    if let TypeKind::Function { ret, .. } = self.types.get(rhs_ty).clone() {
                        ret
                    } else {
                        TypeId::ANY
                    }
                } else if op == 22 {
                    // BinaryOp::Coalesce (??)
                    if lhs_ty == TypeId::NULL {
                        rhs_ty
                    } else {
                        lhs_ty
                    }
                } else {
                    match self.types.unify(lhs_ty, rhs_ty) {
                        Ok(unified) => unified,
                        Err(e) => {
                            self.diagnostics.push(AnalysisDiagnostic::error(
                                "TYPE002",
                                format!("Type mismatch in binary expression: {}", e),
                                arena.span(node_id),
                            ));
                            TypeId::ANY
                        }
                    }
                }
            }

            NodeKind::UnaryExpr => self.synth(arena, NodeId(node.lhs)),

            // --- Compound Collections ---
            NodeKind::ListExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                if children.is_empty() {
                    let var = self.types.new_var(0);
                    self.types.list_of(var)
                } else {
                    let mut elem_ty = self.synth(arena, NodeId(children[0]));
                    for &c in &children[1..] {
                        let c_ty = self.synth(arena, NodeId(c));
                        match self.types.unify(elem_ty, c_ty) {
                            Ok(unified) => elem_ty = unified,
                            Err(_) => {
                                elem_ty = TypeId::ANY;
                            }
                        }
                    }
                    self.types.list_of(elem_ty)
                }
            }

            NodeKind::TupleExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut elem_types = Vec::with_capacity(children.len());
                for &c in children {
                    elem_types.push(self.synth(arena, NodeId(c)));
                }
                self.types.tuple_of(elem_types)
            }

            NodeKind::MapExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut k_ty = TypeId::ANY;
                let mut v_ty = TypeId::ANY;
                for chunk in children.chunks(2) {
                    if chunk.len() == 2 {
                        k_ty = self.synth(arena, NodeId(chunk[0]));
                        v_ty = self.synth(arena, NodeId(chunk[1]));
                    }
                }
                self.types.map_of(k_ty, v_ty)
            }

            NodeKind::RangeExpr => self.types.list_of(TypeId::I64),

            // --- Blocks & Program Root ---
            NodeKind::Program => {
                let children = arena.children_slice(node.lhs, node.rhs);
                // Pass 1: Register top-level function, class, struct, record, and ADT signatures
                for &c in children {
                    let child_node = *arena.node(NodeId(c));
                    match child_node.node_kind() {
                        NodeKind::FnDecl => {
                            let fn_name = arena.interner.resolve(SymbolId(child_node.lhs)).to_string();
                            let fn_start = child_node.rhs;
                            let fn_len = child_node.flags as usize;
                            let mut param_tys = Vec::new();
                            if fn_len > 0 {
                                let fn_children = arena.children_slice(fn_start, fn_len as u32);
                                for &p_id in &fn_children[1..] {
                                    let p_node = *arena.node(NodeId(p_id));
                                    let p_ty = if p_node.rhs != 0 {
                                        self.resolve_type_spec(arena, NodeId(p_node.rhs))
                                    } else {
                                        TypeId::ANY
                                    };
                                    param_tys.push(p_ty);
                                }
                            }
                            let fn_ty = self.types.function_of(param_tys, TypeId::ANY);
                            self.define_var(&fn_name, fn_ty);
                        }
                        NodeKind::RecordDecl | NodeKind::StructDecl => {
                            let rec_name = arena.interner.resolve(SymbolId(child_node.lhs)).to_string();
                            let ty = self.types.row_record_of(Vec::new(), true, None);
                            self.type_defs.insert(rec_name.clone(), ty);
                            // Define constructor in scope
                            let ctor_ty = self.types.function_of(vec![TypeId::ANY], ty);
                            self.define_var(&rec_name, ctor_ty);
                        }
                        NodeKind::TypeAdtDecl => {
                            let adt_name = arena.interner.resolve(SymbolId(child_node.lhs)).to_string();
                            let adt_ty = self.types.adt_of(adt_name.clone(), Vec::new());
                            self.type_defs.insert(adt_name.clone(), adt_ty);

                            let var_start = child_node.rhs as usize;
                            let var_len = child_node.flags as usize;
                            if var_len > 0 && var_start + var_len <= arena.extra_children.len() {
                                for &v_id in &arena.extra_children[var_start..var_start + var_len] {
                                    let v_node = arena.node(NodeId(v_id));
                                    if v_node.node_kind() == NodeKind::AdtVariant {
                                        let v_name = arena.interner.resolve(SymbolId(v_node.lhs)).to_string();
                                        let param_len = v_node.flags as usize;
                                        self.adt_variants.insert(v_name.clone(), (adt_name.clone(), Vec::new()));
                                        if param_len == 0 {
                                            self.define_var(&v_name, adt_ty);
                                        } else {
                                            let mut p_tys = Vec::new();
                                            for _ in 0..param_len {
                                                p_tys.push(TypeId::ANY);
                                            }
                                            let ctor_ty = self.types.function_of(p_tys, adt_ty);
                                            self.define_var(&v_name, ctor_ty);
                                        }
                                    }
                                }
                            }
                        }
                        NodeKind::ClassDecl | NodeKind::InterfaceDecl => {
                            let cls_name = arena.interner.resolve(SymbolId(child_node.lhs)).to_string();
                            let ty = self.types.alloc(TypeKind::Primitive(cls_name.clone()));
                            self.type_defs.insert(cls_name.clone(), ty);
                            self.define_var(&cls_name, ty);
                        }
                        NodeKind::ImportDecl => {
                            if child_node.rhs != u32::MAX {
                                let alias = arena.interner.resolve(SymbolId(child_node.rhs)).to_string();
                                self.define_var(&alias, TypeId::ANY);
                                self.type_defs.insert(alias, TypeId::ANY);
                            }
                        }
                        NodeKind::FromImportDecl => {
                            let sub_children = arena.children_slice(child_node.lhs, child_node.rhs);
                            for &sym_id in sub_children {
                                let sym_node = arena.node(NodeId(sym_id));
                                if sym_node.node_kind() == NodeKind::IdentExpr {
                                    let name = arena.interner.resolve(SymbolId(sym_node.lhs)).to_string();
                                    self.define_var(&name, TypeId::ANY);
                                    self.type_defs.insert(name, TypeId::ANY);
                                }
                            }
                        }
                        NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
                            if child_node.flags == 1 {
                                self.bind_pattern_vars(arena, NodeId(child_node.lhs));
                            } else {
                                let name = arena.interner.resolve(SymbolId(child_node.lhs)).to_string();
                                self.define_var(&name, TypeId::ANY);
                            }
                        }
                        _ => {}
                    }
                }

                // Pass 2: Synthesize all statements and expressions
                let mut last_ty = TypeId::VOID;
                for &c in children {
                    last_ty = self.synth(arena, NodeId(c));
                }
                last_ty
            }

            NodeKind::BlockExpr => {
                self.enter_scope();
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut last_ty = TypeId::VOID;
                for &c in children {
                    let child_node = *arena.node(NodeId(c));
                    let ty = self.synth(arena, NodeId(c));
                    if child_node.node_kind() == NodeKind::ReturnStmt {
                        last_ty = ty;
                    } else if last_ty == TypeId::VOID {
                        last_ty = ty;
                    }
                }
                self.exit_scope();
                last_ty
            }

            NodeKind::ExprStmt => {
                let expr_node = NodeId(node.lhs);
                if expr_node.is_some() {
                    self.synth(arena, expr_node)
                } else {
                    TypeId::VOID
                }
            }

            NodeKind::TryStmt => {
                let children = arena.children_slice(node.lhs, node.rhs);
                for &child_id in children {
                    self.synth(arena, NodeId(child_id));
                }
                TypeId::VOID
            }

            NodeKind::CatchClause => {
                let var_sym = SymbolId(node.lhs);
                let body_node = NodeId(node.rhs);
                self.enter_scope();
                if var_sym.0 != 0 {
                    let var_name = arena.interner.resolve(var_sym);
                    self.define_var(var_name, TypeId::ANY);
                }
                if body_node.is_some() {
                    self.synth(arena, body_node);
                }
                self.exit_scope();
                TypeId::VOID
            }

            // --- Variable Declarations & Assignments ---
            NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
                let init_node = NodeId(node.rhs);
                let val_ty = if init_node.is_some() {
                    self.synth(arena, init_node)
                } else {
                    TypeId::ANY
                };
                if node.flags == 1 {
                    self.bind_pattern_vars(arena, NodeId(node.lhs));
                } else {
                    let sym = SymbolId(node.lhs);
                    let name = arena.interner.resolve(sym).to_string();
                    self.define_var(&name, val_ty);
                }
                val_ty
            }

            NodeKind::AssignStmt => {
                let target = NodeId(node.lhs);
                let val = NodeId(node.rhs);
                let val_ty = self.synth(arena, val);
                if target.is_some() {
                    let target_ty = self.synth(arena, target);
                    let _ = self.types.unify(target_ty, val_ty);
                }
                TypeId::VOID
            }

            NodeKind::ReturnStmt => {
                let expr_node = NodeId(node.lhs);
                if expr_node.is_some() {
                    self.synth(arena, expr_node)
                } else {
                    TypeId::VOID
                }
            }

            NodeKind::YieldStmt => {
                let expr_node = NodeId(node.lhs);
                if expr_node.is_some() {
                    self.synth(arena, expr_node)
                } else {
                    TypeId::VOID
                }
            }

            // --- Function, Class, Struct, and ADT Declarations ---
            NodeKind::FnDecl => {
                let sym = SymbolId(node.lhs);
                let fn_name = arena.interner.resolve(sym).to_string();
                let fn_start = node.rhs;
                let fn_len = node.flags as usize;

                let fn_placeholder = self.types.function_of(Vec::new(), TypeId::ANY);
                self.define_var(&fn_name, fn_placeholder);

                self.enter_scope();
                let mut param_tys = Vec::new();
                let body_ty = if fn_len > 0 {
                    let fn_children = arena.children_slice(fn_start, fn_len as u32);
                    let body_node = NodeId(fn_children[0]);
                    for &p_id in &fn_children[1..] {
                        let p_node = *arena.node(NodeId(p_id));
                        let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                        let p_ty = if p_node.rhs != 0 {
                            self.resolve_type_spec(arena, NodeId(p_node.rhs))
                        } else {
                            TypeId::ANY
                        };
                        self.define_var(&p_name, p_ty);
                        param_tys.push(p_ty);
                    }
                    self.synth(arena, body_node)
                } else {
                    let body_node = NodeId(node.rhs);
                    self.synth(arena, body_node)
                };
                self.exit_scope();

                let fn_ty = self.types.function_of(param_tys, body_ty);
                self.define_var(&fn_name, fn_ty);
                fn_ty
            }

            NodeKind::RecordDecl | NodeKind::StructDecl => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym).to_string();
                let ty = self.types.row_record_of(Vec::new(), true, None);
                self.type_defs.insert(name.clone(), ty);
                let ctor = self.types.function_of(vec![TypeId::ANY], ty);
                self.define_var(&name, ctor);
                ty
            }

            NodeKind::TypeAdtDecl => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym).to_string();
                let ty = self.types.adt_of(name.clone(), Vec::new());
                self.type_defs.insert(name, ty);
                ty
            }

            NodeKind::ClassDecl | NodeKind::InterfaceDecl => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym).to_string();
                let ty = self.types.alloc(TypeKind::Primitive(name.clone()));
                self.type_defs.insert(name.clone(), ty);
                self.define_var(&name, ty);
                let body = NodeId(node.rhs);
                if body.is_some() {
                    self.synth(arena, body);
                }
                ty
            }

            NodeKind::ExtendDecl => {
                let body = NodeId(node.rhs);
                if body.is_some() {
                    self.synth(arena, body);
                }
                TypeId::VOID
            }

            // --- Control Flow ---
            NodeKind::IfExpr => {
                let cond_node = NodeId(node.lhs);
                self.check(arena, cond_node, TypeId::BOOL);
                let (then_node, else_node) = if node.flags == 2 {
                    let children = arena.children_slice(node.rhs, 2);
                    (NodeId(children[0]), NodeId(children[1]))
                } else {
                    (NodeId(node.rhs), NodeId::NONE)
                };
                let t_then = self.synth(arena, then_node);
                if else_node.is_some() {
                    self.synth(arena, else_node);
                }
                t_then
            }

            NodeKind::WhileStmt => {
                let cond_node = NodeId(node.lhs);
                self.check(arena, cond_node, TypeId::BOOL);
                let body_node = NodeId(node.rhs);
                self.enter_scope();
                self.synth(arena, body_node);
                self.exit_scope();
                TypeId::VOID
            }

            NodeKind::ForExpr => {
                let sym = SymbolId(node.lhs);
                let var_name = arena.interner.resolve(sym).to_string();
                let body_node = NodeId(node.rhs);

                self.enter_scope();
                self.define_var(&var_name, TypeId::I64);
                let body_ty = self.synth(arena, body_node);
                self.exit_scope();

                if body_ty != TypeId::VOID {
                    self.types.list_of(body_ty)
                } else {
                    TypeId::VOID
                }
            }

            NodeKind::GuardStmt => {
                let cond_node = NodeId(node.lhs);
                if cond_node.is_some() {
                    let cond_node_val = *arena.node(cond_node);
                    if cond_node_val.node_kind() == NodeKind::IdentExpr {
                        let var_name = arena.interner.resolve(SymbolId(cond_node_val.lhs));
                        self.narrow_on_guard_some(var_name);
                    }
                    self.synth(arena, cond_node);
                }
                TypeId::VOID
            }

            NodeKind::DeferStmt => {
                let body = NodeId(node.lhs);
                if body.is_some() {
                    self.synth(arena, body);
                }
                TypeId::VOID
            }

            NodeKind::BreakStmt | NodeKind::ContinueStmt => TypeId::NEVER,

            // --- Function & Method Invocations ---
            NodeKind::CallExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                if children.is_empty() {
                    return TypeId::ANY;
                }
                let callee_node = NodeId(children[0]);
                let callee_ty = self.synth(arena, callee_node);

                // Synthesize argument types
                for &arg_id in &children[1..] {
                    self.synth(arena, NodeId(arg_id));
                }

                match self.types.get(self.types.prune(callee_ty)) {
                    TypeKind::Function { ret, .. } => *ret,
                    TypeKind::RowRecord { .. } => callee_ty,
                    TypeKind::Adt { .. } => callee_ty,
                    _ => TypeId::ANY,
                }
            }

            // --- Member & Index Projections ---
            NodeKind::MemberExpr => {
                let target_node = NodeId(node.lhs);
                let target_ty = self.synth(arena, target_node);
                let member_name = arena.interner.resolve(SymbolId(node.rhs));

                // Member on Option without unwrap
                if let TypeKind::Option(_) = self.types.get(self.types.prune(target_ty)) {
                    self.diagnostics.push(AnalysisDiagnostic::error(
                        "NULL001",
                        format!("Cannot access member '{}' on nullable value without unwrapping", member_name),
                        arena.span(node_id),
                    ));
                }

                // Well-known metadata attributes
                match member_name {
                    "len" | "size_of" | "tag_bytes" | "length" | "count" => TypeId::I64,
                    "is_unboxed" | "is_recursive" => TypeId::BOOL,
                    _ => TypeId::ANY,
                }
            }

            NodeKind::IndexExpr => {
                let target_node = NodeId(node.lhs);
                let target_ty = self.synth(arena, target_node);
                let idx_node = NodeId(node.rhs);
                if idx_node.is_some() {
                    self.synth(arena, idx_node);
                }

                match self.types.get(self.types.prune(target_ty)) {
                    TypeKind::Collection { element, .. } => *element,
                    TypeKind::Map { value, .. } => self.types.option_of(*value),
                    TypeKind::Option(_) => {
                        self.diagnostics.push(AnalysisDiagnostic::error(
                            "NULL001",
                            "Cannot index into nullable value without unwrapping",
                            arena.span(node_id),
                        ));
                        TypeId::ANY
                    }
                    _ => TypeId::ANY,
                }
            }

            // --- Operators: Pipe, Cascade, Coalesce, TryPropagate, CatchRecovery ---
            NodeKind::PipeExpr => {
                let lhs_ty = self.synth(arena, NodeId(node.lhs));
                let rhs_ty = self.synth(arena, NodeId(node.rhs));
                match self.types.get(self.types.prune(rhs_ty)) {
                    TypeKind::Function { ret, .. } => *ret,
                    _ => lhs_ty,
                }
            }

            NodeKind::CascadeExpr => {
                let receiver_ty = self.synth(arena, NodeId(node.lhs));
                let rhs_id = NodeId(node.rhs);
                self.synth_cascade_op(arena, rhs_id);
                receiver_ty
            }

            NodeKind::TryPropagateExpr => {
                let inner_ty = self.synth(arena, NodeId(node.lhs));
                match self.types.get(self.types.prune(inner_ty)) {
                    TypeKind::Option(elem) => *elem,
                    TypeKind::Result { ok, .. } => *ok,
                    _ => inner_ty,
                }
            }

            NodeKind::CatchRecoveryExpr => {
                let lhs_ty = self.synth(arena, NodeId(node.lhs));
                let rhs_ty = self.synth(arena, NodeId(node.rhs));
                let _ = self.types.unify(lhs_ty, rhs_ty);
                lhs_ty
            }

            NodeKind::CoalesceExpr => {
                let lhs_ty = self.synth(arena, NodeId(node.lhs));
                let rhs_ty = self.synth(arena, NodeId(node.rhs));
                let kind = self.types.get(self.types.prune(lhs_ty)).clone();
                match kind {
                    TypeKind::Option(elem) => {
                        let _ = self.types.unify(elem, rhs_ty);
                        elem
                    }
                    _ => lhs_ty,
                }
            }

            NodeKind::LambdaExpr => {
                self.enter_scope();
                let param_start = node.lhs;
                let param_len = node.flags as usize;
                if param_len > 0 {
                    let params = arena.children_slice(param_start, param_len as u32);
                    for &p_id in params {
                        let p_node = *arena.node(NodeId(p_id));
                        let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                        let p_ty = if p_node.rhs != 0 {
                            self.resolve_type_spec(arena, NodeId(p_node.rhs))
                        } else {
                            TypeId::ANY
                        };
                        self.define_var(&p_name, p_ty);
                    }
                }
                let body_node = NodeId(node.rhs);
                let body_ty = self.synth(arena, body_node);
                self.exit_scope();
                let mut param_tys = Vec::new();
                for _ in 0..param_len {
                    param_tys.push(TypeId::ANY);
                }
                self.types.function_of(param_tys, body_ty)
            }

            // --- Pattern Matching ---
            NodeKind::MatchExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                if children.is_empty() {
                    return TypeId::ANY;
                }
                let scrutinee = NodeId(children[0]);
                let _scrutinee_ty = self.synth(arena, scrutinee);

                let mut unified_arm_ty = TypeId::VOID;
                for &arm_id in &children[1..] {
                    let arm_ty = self.synth(arena, NodeId(arm_id));
                    if unified_arm_ty == TypeId::VOID {
                        unified_arm_ty = arm_ty;
                    } else if arm_ty != TypeId::VOID && arm_ty != TypeId::NEVER {
                        let _ = self.types.unify(unified_arm_ty, arm_ty);
                    }
                }
                unified_arm_ty
            }

            NodeKind::MatchArm => {
                let pat_node = NodeId(node.lhs);
                let body_node = NodeId(node.rhs);
                self.enter_scope();
                if pat_node.is_some() {
                    self.bind_pattern_vars(arena, pat_node);
                }
                let arm_ty = self.synth(arena, body_node);
                self.exit_scope();
                arm_ty
            }

            NodeKind::ComptimeExpr => {
                let inner = NodeId(node.lhs);
                self.synth(arena, inner)
            }

            _ => TypeId::ANY,
        };

        self.inferred_node_types.insert(node_id.0, ty);
        ty
    }

    /// Bidirectional: checks that an expression node conforms to expected type.
    pub fn check(&mut self, arena: &AstArena, node_id: NodeId, expected: TypeId) {
        if node_id.is_none() {
            return;
        }

        let actual = self.synth(arena, node_id);
        if let Err(e) = self.types.unify(actual, expected) {
            self.diagnostics.push(AnalysisDiagnostic::error(
                "TYPE004",
                format!(
                    "Type check failed: expected '{}', found '{}' ({})",
                    self.types.display(expected),
                    self.types.display(actual),
                    e
                ),
                arena.span(node_id),
            ));
        }
    }

    /// Binds variables declared inside a pattern into the current scope.
    pub fn bind_pattern_vars(&mut self, arena: &AstArena, pat_id: NodeId) {
        if pat_id.is_none() {
            return;
        }
        let node = *arena.node(pat_id);
        match node.node_kind() {
            NodeKind::PatBind => {
                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                if name != "_" {
                    self.define_var(&name, TypeId::ANY);
                }
                if node.flags == 1 && node.rhs != 0 {
                    self.bind_pattern_vars(arena, NodeId(node.rhs));
                }
            }
            NodeKind::PatVariant => {
                let start = node.rhs as usize;
                let len = node.flags as usize;
                if len > 0 && start + len <= arena.extra_children.len() {
                    for &sub_id in &arena.extra_children[start..start + len] {
                        self.bind_pattern_vars(arena, NodeId(sub_id));
                    }
                }
            }
            NodeKind::PatTuple | NodeKind::PatSlice => {
                let start = node.lhs as usize;
                let len = node.rhs as usize;
                if len > 0 && start + len <= arena.extra_children.len() {
                    for &sub_id in &arena.extra_children[start..start + len] {
                        self.bind_pattern_vars(arena, NodeId(sub_id));
                    }
                } else if node.lhs != 0 {
                    let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                    if name != "_" && !name.is_empty() {
                        self.define_var(&name, TypeId::ANY);
                    }
                }
            }
            NodeKind::PatOr => {
                self.bind_pattern_vars(arena, NodeId(node.lhs));
                self.bind_pattern_vars(arena, NodeId(node.rhs));
            }
            NodeKind::IdentExpr => {
                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                if name != "_" && !name.chars().next().map_or(false, |c| c.is_uppercase()) {
                    self.define_var(&name, TypeId::ANY);
                }
            }
            _ => {}
        }
    }

    fn synth_cascade_op(&mut self, arena: &AstArena, op_id: NodeId) {
        if op_id.is_none() {
            return;
        }
        let op_node = *arena.node(op_id);
        match op_node.node_kind() {
            NodeKind::CallExpr => {
                let arg_start = op_node.lhs as usize;
                let arg_len = op_node.rhs as usize;
                // Callee is at arg_start (method name on receiver); synthesize arguments without global lookup of method name
                if arg_len > 1 && arg_start + arg_len <= arena.extra_children.len() {
                    for &arg_id in &arena.extra_children[arg_start + 1..arg_start + arg_len] {
                        self.synth(arena, NodeId(arg_id));
                    }
                }
            }
            NodeKind::IdentExpr => {
                // Member property access on receiver in cascade
            }
            NodeKind::CascadeExpr => {
                self.synth_cascade_op(arena, NodeId(op_node.lhs));
                self.synth_cascade_op(arena, NodeId(op_node.rhs));
            }
            NodeKind::BinaryExpr => {
                self.synth(arena, NodeId(op_node.rhs));
            }
            NodeKind::LetStmt | NodeKind::VarStmt => {
                self.synth(arena, op_id);
            }
            _ => {
                self.synth(arena, op_id);
            }
        }
    }

    /// Dominator flow narrowing: narrows an `Option[T]` to `T` in downstream block.
    pub fn narrow_on_guard_some(&mut self, var_name: &str) {
        if let Some(existing) = self.lookup_var(var_name) {
            let existing = self.types.prune(existing);
            if let TypeKind::Option(inner) = self.types.get(existing).clone() {
                self.narrow_var(var_name, inner);
            }
        }
    }

    /// Dominator flow narrowing: narrows a base type to active typestate.
    pub fn narrow_on_typestate_transition(&mut self, var_name: &str, state: &str) {
        if let Some(existing) = self.lookup_var(var_name) {
            let base = match self.types.get(existing) {
                TypeKind::Typestate { base, .. } => *base,
                _ => existing,
            };
            let narrowed = self.types.typestate_of(base, state.to_string());
            self.narrow_var(var_name, narrowed);
        }
    }
}
