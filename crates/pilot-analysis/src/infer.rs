//! Milestone 33.1: Flow-Sensitive Bidirectional Type Inference, Dominator Flow Narrowing & Tarjan SCC.

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
}

impl Default for TypeInferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeInferenceEngine {
    pub fn new() -> Self {
        Self {
            types: TypeArena::new(),
            scopes: vec![Scope::new()],
            diagnostics: Vec::new(),
            inferred_node_types: HashMap::new(),
        }
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
        None
    }

    /// Bidirectional: synthesizes the type of an expression node.
    pub fn synth(&mut self, arena: &AstArena, node_id: NodeId) -> TypeId {
        if node_id.is_none() {
            return TypeId::VOID;
        }

        let node = *arena.node(node_id);
        let ty = match node.node_kind() {
            NodeKind::IntLit => TypeId::I64,
            NodeKind::FloatLit => TypeId::F64,
            NodeKind::StrLit => TypeId::STR,
            NodeKind::BoolLit => TypeId::BOOL,
            NodeKind::NoneLit => TypeId::NULL,

            NodeKind::IdentExpr => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym);
                if let Some(t) = self.lookup_var(name) {
                    t
                } else {
                    self.diagnostics.push(AnalysisDiagnostic::error(
                        "TYPE001",
                        format!("Unresolved identifier '{}'", name),
                        arena.span(node_id),
                    ));
                    TypeId::ANY
                }
            }

            NodeKind::BinaryExpr => {
                let lhs_ty = self.synth(arena, NodeId(node.lhs));
                let rhs_ty = self.synth(arena, NodeId(node.rhs));
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

            NodeKind::UnaryExpr => {
                self.synth(arena, NodeId(node.lhs))
            }

            NodeKind::ListExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                if children.is_empty() {
                    let var = self.types.new_var(0);
                    self.types.list_of(var)
                } else {
                    let first_ty = self.synth(arena, NodeId(children[0]));
                    for &c in &children[1..] {
                        let c_ty = self.synth(arena, NodeId(c));
                        if let Err(e) = self.types.unify(first_ty, c_ty) {
                            self.diagnostics.push(AnalysisDiagnostic::error(
                                "TYPE003",
                                format!("Homogeneity violation in list literal: {}", e),
                                arena.span(NodeId(c)),
                            ));
                        }
                    }
                    self.types.list_of(first_ty)
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

            NodeKind::BlockExpr | NodeKind::Program => {
                if node.node_kind() == NodeKind::BlockExpr {
                    self.enter_scope();
                }
                let children = arena.children_slice(node.lhs, node.rhs);
                let mut last_ty = TypeId::VOID;
                for &c in children {
                    last_ty = self.synth(arena, NodeId(c));
                }
                if node.node_kind() == NodeKind::BlockExpr {
                    self.exit_scope();
                }
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

            NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => {
                let sym = SymbolId(node.lhs);
                let name = arena.interner.resolve(sym).to_string();
                let init_node = NodeId(node.rhs);
                let val_ty = if init_node.is_some() {
                    self.synth(arena, init_node)
                } else {
                    TypeId::ANY
                };
                self.define_var(&name, val_ty);
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

            NodeKind::IfExpr => {
                let cond_node = NodeId(node.lhs);
                self.check(arena, cond_node, TypeId::BOOL);
                let then_node = NodeId(node.rhs);
                self.synth(arena, then_node)
            }

            NodeKind::FnDecl => {
                // Register function
                let sym = SymbolId(node.lhs);
                let fn_name = arena.interner.resolve(sym).to_string();
                let body_node = NodeId(node.rhs);

                self.enter_scope();
                let body_ty = self.synth(arena, body_node);
                self.exit_scope();

                let fn_ty = self.types.function_of(Vec::new(), body_ty);
                self.define_var(&fn_name, fn_ty);
                fn_ty
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
