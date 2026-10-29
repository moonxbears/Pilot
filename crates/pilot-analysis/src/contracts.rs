//! Milestone 33.2: Compile-Time Design-by-Contract Interval Verification.

use pilot_span::ByteSpan;
use crate::diagnostics::AnalysisDiagnostic;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Interval {
    pub min: f64,
    pub max: f64,
}

impl Interval {
    pub fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    pub fn point(val: f64) -> Self {
        Self { min: val, max: val }
    }

    pub fn is_empty(&self) -> bool {
        self.min > self.max
    }

    pub fn add(self, other: Self) -> Self {
        Self {
            min: self.min + other.min,
            max: self.max + other.max,
        }
    }

    pub fn sub(self, other: Self) -> Self {
        Self {
            min: self.min - other.max,
            max: self.max - other.min,
        }
    }

    pub fn contains_zero(&self) -> bool {
        self.min <= 0.0 && self.max >= 0.0
    }
}

pub struct ContractVerifier {
    pub diagnostics: Vec<AnalysisDiagnostic>,
}

impl Default for ContractVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ContractVerifier {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn check_precondition(&mut self, condition: bool, message: &str, span: ByteSpan) {
        if !condition {
            self.diagnostics.push(AnalysisDiagnostic::error(
                "CONTRACT001",
                format!("Precondition violation: {}", message),
                span,
            ));
        }
    }

    pub fn check_postcondition(&mut self, condition: bool, message: &str, span: ByteSpan) {
        if !condition {
            self.diagnostics.push(AnalysisDiagnostic::error(
                "CONTRACT002",
                format!("Postcondition violation: {}", message),
                span,
            ));
        }
    }

    pub fn check_division_by_zero(&mut self, divisor_interval: Interval, span: ByteSpan) {
        if divisor_interval.min == 0.0 && divisor_interval.max == 0.0 {
            self.diagnostics.push(AnalysisDiagnostic::error(
                "DIV001",
                "Compile-time division by zero detected",
                span,
            ));
        } else if divisor_interval.contains_zero() {
            self.diagnostics.push(AnalysisDiagnostic::warning(
                "DIV002",
                "Divisor interval contains zero; potential division by zero at runtime",
                span,
            ));
        }
    }

    pub fn check_node(&mut self, arena: &pilot_ast::AstArena, node_id: pilot_ast::NodeId) {
        let node = *arena.node(node_id);
        if node.node_kind() == pilot_ast::NodeKind::BinaryExpr {
            let op = node.flags;
            if op == 4 || op == 5 {
                let rhs = pilot_ast::NodeId(node.rhs);
                if rhs.is_some() {
                    let rhs_node = *arena.node(rhs);
                    if rhs_node.node_kind() == pilot_ast::NodeKind::IntLit {
                        let v = arena.i64_pool.get(rhs_node.lhs as usize).copied().unwrap_or(0);
                        if v == 0 {
                            self.check_division_by_zero(Interval::point(0.0), arena.span(node_id));
                        }
                    }
                }
            }
        }
    }
}
