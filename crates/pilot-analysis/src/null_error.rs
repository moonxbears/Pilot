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
}
