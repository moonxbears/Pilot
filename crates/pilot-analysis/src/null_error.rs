//! Milestone 33.2: Concise Modern Null Safety (NULL001–NULL005) & Values-First Error Flow (ERR001–ERR005).

use pilot_span::ByteSpan;
use crate::diagnostics::AnalysisDiagnostic;

pub struct NullSafetyAnalyzer {
    pub diagnostics: Vec<AnalysisDiagnostic>,
}

impl Default for NullSafetyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl NullSafetyAnalyzer {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn report_unsafe_deref(&mut self, var_name: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "NULL001",
            format!("Cannot access member or index on nullable value '{}' without unwrapping (use 'guard some {} else', '??', or '?.')", var_name, var_name),
            span,
        ));
    }

    pub fn report_redundant_null_check(&mut self, var_name: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::warning(
            "NULL002",
            format!("Redundant null check: '{}' is statically known to be non-null", var_name),
            span,
        ));
    }

    pub fn report_null_assigned_to_non_null(&mut self, target_name: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "NULL003",
            format!("Cannot assign 'null' to non-nullable variable '{}'", target_name),
            span,
        ));
    }

    pub fn report_redundant_guard_some(&mut self, expr_name: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::warning(
            "NULL004",
            format!("'guard some' on definitely non-null expression '{}' is redundant", expr_name),
            span,
        ));
    }

    pub fn report_deprecated_bang_assertion(&mut self, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "NULL005",
            "Unchecked postfix non-null assertion '!' is removed in Pilot. Use 'guard some x else', '??', or explicit '.or_panic(\"reason\")'.",
            span,
        ));
    }

    pub fn check_node(
        &mut self,
        arena: &pilot_ast::AstArena,
        node_id: pilot_ast::NodeId,
        infer: &crate::infer::TypeInferenceEngine,
    ) {
        let node = *arena.node(node_id);
        match node.node_kind() {
            pilot_ast::NodeKind::MemberExpr => {
                let target = pilot_ast::NodeId(node.lhs);
                if let Some(&ty) = infer.inferred_node_types.get(&target.0) {
                    if let crate::types::TypeKind::Option(_) = infer.types.get(infer.types.prune(ty)) {
                        // Check if safe navigation (flags == 1) or intrinsic Option member
                        if node.flags == 0 {
                            let member_name = arena.interner.resolve(pilot_intern::SymbolId(node.rhs));
                            if !matches!(
                                member_name,
                                "is_some"
                                    | "is_none"
                                    | "is_unboxed"
                                    | "OrPanic"
                                    | "or_panic"
                                    | "UnwrapOr"
                                    | "unwrap_or"
                                    | "unwrap"
                                    | "expect"
                                    | "map"
                                    | "and_then"
                            ) {
                                self.report_unsafe_deref(member_name, arena.span(node_id));
                            }
                        }
                    }
                }
            }
            pilot_ast::NodeKind::IndexExpr => {
                let target = pilot_ast::NodeId(node.lhs);
                if let Some(&ty) = infer.inferred_node_types.get(&target.0) {
                    if let crate::types::TypeKind::Option(_) = infer.types.get(infer.types.prune(ty)) {
                        self.report_unsafe_deref("indexed value", arena.span(node_id));
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct ErrorFlowAnalyzer {
    pub diagnostics: Vec<AnalysisDiagnostic>,
}

impl Default for ErrorFlowAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorFlowAnalyzer {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn report_must_use_result(&mut self, expr_desc: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "ERR001",
            format!("Expression returning Result '{}' must be handled (use '?', 'match', or assign to a variable)", expr_desc),
            span,
        ));
    }

    pub fn report_undeclared_error_return(&mut self, err_type: &str, fn_name: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "ERR002",
            format!("Function '{}' returns error '{}' but does not declare it in its return signature (e.g. '-> T ! {}')", fn_name, err_type, err_type),
            span,
        ));
    }

    pub fn report_unreachable_error_arm(&mut self, err_type: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::warning(
            "ERR004",
            format!("Unreachable error handler arm: error type '{}' cannot be thrown in this block", err_type),
            span,
        ));
    }

    pub fn report_incompatible_error_sets(&mut self, expected: &str, actual: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "ERR005",
            format!("Error set mismatch: caller expects '! {}', but callee can throw '! {}'", expected, actual),
            span,
        ));
    }

    pub fn check_node(
        &mut self,
        arena: &pilot_ast::AstArena,
        node_id: pilot_ast::NodeId,
        infer: &crate::infer::TypeInferenceEngine,
    ) {
        let node = *arena.node(node_id);
        if node.node_kind() == pilot_ast::NodeKind::ExprStmt {
            let inner = pilot_ast::NodeId(node.lhs);
            if inner.is_some() {
                if let Some(&ty) = infer.inferred_node_types.get(&inner.0) {
                    if let crate::types::TypeKind::Result { .. } = infer.types.get(infer.types.prune(ty)) {
                        self.report_must_use_result("Result", arena.span(node_id));
                    }
                }
            }
        }
    }

    /// Section 4.2: Universal Postfix `expr?` (`ITryOperator`) Verification & Propagation Plan.
    /// - If `expr` has type `result<TOk, TErr>`: verifies enclosing function returns `result<U, F>`,
    ///   unwraps `TOk` on `.is_ok`, and plans early `return result<U, F>.err(expr.err_val)` on failure.
    /// - If `expr` has type `option<T>`: verifies enclosing function returns `option<U>`,
    ///   unwraps `T` on `.has_value`, and plans early `return option<U>.none()` on empty.
    pub fn analyze_try_operator(
        &mut self,
        expr_ty: crate::types::TypeId,
        enclosing_return_ty: Option<crate::types::TypeId>,
        types: &crate::types::TypeArena,
        fn_name: &str,
        span: ByteSpan,
    ) -> TryPropagationPlan {
        let pruned = types.prune(expr_ty);
        match types.get(pruned).clone() {
            crate::types::TypeKind::Option(inner_ty) => {
                if let Some(ret_ty) = enclosing_return_ty {
                    let ret_pruned = types.prune(ret_ty);
                    if !matches!(types.get(ret_pruned), crate::types::TypeKind::Option(_))
                        && ret_pruned != crate::types::TypeId::ANY
                    {
                        self.report_undeclared_error_return("option<none>", fn_name, span);
                    }
                }
                TryPropagationPlan::OptionUnwrap {
                    unwrapped_ty: inner_ty,
                    lang_item: crate::comptime::LangItem::Option,
                }
            }
            crate::types::TypeKind::Result { ok, err } => {
                if let Some(ret_ty) = enclosing_return_ty {
                    let ret_pruned = types.prune(ret_ty);
                    match types.get(ret_pruned) {
                        crate::types::TypeKind::Result { err: expected_err, .. } => {
                            let exp_e = types.prune(*expected_err);
                            let act_e = types.prune(err);
                            if exp_e != act_e
                                && exp_e != crate::types::TypeId::ANY
                                && act_e != crate::types::TypeId::ANY
                            {
                                self.report_incompatible_error_sets(
                                    &types.display(exp_e),
                                    &types.display(act_e),
                                    span,
                                );
                            }
                        }
                        _ if ret_pruned != crate::types::TypeId::ANY => {
                            self.report_undeclared_error_return(&types.display(err), fn_name, span);
                        }
                        _ => {}
                    }
                }
                TryPropagationPlan::ResultUnwrap {
                    unwrapped_ok_ty: ok,
                    propagated_err_ty: err,
                    lang_item: crate::comptime::LangItem::Result,
                }
            }
            _ => {
                if let Some(td) = types.type_defs.get(&pruned) {
                    if let crate::types::StructuralShape::TaggedOutcome { ok_ty, err_ty, .. } = td.shape {
                        if let Some(err) = err_ty {
                            return TryPropagationPlan::ResultUnwrap {
                                unwrapped_ok_ty: ok_ty,
                                propagated_err_ty: err,
                                lang_item: crate::comptime::LangItem::Result,
                            };
                        } else {
                            return TryPropagationPlan::OptionUnwrap {
                                unwrapped_ty: ok_ty,
                                lang_item: crate::comptime::LangItem::Option,
                            };
                        }
                    }
                }
                TryPropagationPlan::Passthrough(pruned)
            }
        }
    }

    /// Section 4.2: Implicit Value Lifting for `option<T>` (`T?`) and `result<TOk, TErr>` (`T ! E`).
    pub fn synthesize_implicit_lift(
        expected_ty: crate::types::TypeId,
        actual_ty: crate::types::TypeId,
        is_fail_call: bool,
        types: &crate::types::TypeArena,
    ) -> Option<ImplicitLiftKind> {
        let exp = types.prune(expected_ty);
        let act = types.prune(actual_ty);
        match types.get(exp).clone() {
            crate::types::TypeKind::Option(inner) => {
                let inner_p = types.prune(inner);
                if act == crate::types::TypeId::NULL {
                    Some(ImplicitLiftKind::OptionNone { elem_ty: inner_p })
                } else if act == inner_p || inner_p == crate::types::TypeId::ANY {
                    Some(ImplicitLiftKind::OptionSome { elem_ty: inner_p })
                } else {
                    None
                }
            }
            crate::types::TypeKind::Result { ok, err } => {
                let ok_p = types.prune(ok);
                let err_p = types.prune(err);
                if is_fail_call {
                    Some(ImplicitLiftKind::ResultErr {
                        ok_ty: ok_p,
                        err_ty: err_p,
                    })
                } else if act == ok_p || ok_p == crate::types::TypeId::ANY {
                    Some(ImplicitLiftKind::ResultOk {
                        ok_ty: ok_p,
                        err_ty: err_p,
                    })
                } else {
                    None
                }
            }
            _ => {
                if let Some(td) = types.type_defs.get(&exp) {
                    if let crate::types::StructuralShape::TaggedOutcome { ok_ty, err_ty, .. } = td.shape {
                        let ok_p = types.prune(ok_ty);
                        if let Some(err) = err_ty {
                            let err_p = types.prune(err);
                            if is_fail_call {
                                return Some(ImplicitLiftKind::ResultErr { ok_ty: ok_p, err_ty: err_p });
                            } else if act == ok_p || ok_p == crate::types::TypeId::ANY {
                                return Some(ImplicitLiftKind::ResultOk { ok_ty: ok_p, err_ty: err_p });
                            }
                        } else if act == crate::types::TypeId::NULL {
                            return Some(ImplicitLiftKind::OptionNone { elem_ty: ok_p });
                        } else if act == ok_p || ok_p == crate::types::TypeId::ANY {
                            return Some(ImplicitLiftKind::OptionSome { elem_ty: ok_p });
                        }
                    }
                }
                None
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TryPropagationPlan {
    OptionUnwrap {
        unwrapped_ty: crate::types::TypeId,
        lang_item: crate::comptime::LangItem,
    },
    ResultUnwrap {
        unwrapped_ok_ty: crate::types::TypeId,
        propagated_err_ty: crate::types::TypeId,
        lang_item: crate::comptime::LangItem,
    },
    Passthrough(crate::types::TypeId),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImplicitLiftKind {
    /// Synthesizes `option<T>.some(expr)`
    OptionSome { elem_ty: crate::types::TypeId },
    /// Synthesizes `option<T>.none()`
    OptionNone { elem_ty: crate::types::TypeId },
    /// Synthesizes `result<TOk, TErr>.ok(expr)`
    ResultOk {
        ok_ty: crate::types::TypeId,
        err_ty: crate::types::TypeId,
    },
    /// Synthesizes `result<TOk, TErr>.err(err_expr)` (from `fail(err_expr)`)
    ResultErr {
        ok_ty: crate::types::TypeId,
        err_ty: crate::types::TypeId,
    },
}
