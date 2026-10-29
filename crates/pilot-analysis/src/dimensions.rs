//! Milestone 33.2: Physical Units of Measure Dimensional Algebra.

use pilot_span::ByteSpan;
use crate::diagnostics::AnalysisDiagnostic;

/// SI base dimensions: [Length (m), Mass (kg), Time (s), Current (A), Temp (K), Amount (mol), Lum (cd)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DimensionVector {
    pub length: i8,
    pub mass: i8,
    pub time: i8,
    pub current: i8,
    pub temperature: i8,
    pub amount: i8,
    pub luminous_intensity: i8,
}

impl DimensionVector {
    pub const SCALAR: Self = Self {
        length: 0,
        mass: 0,
        time: 0,
        current: 0,
        temperature: 0,
        amount: 0,
        luminous_intensity: 0,
    };

    pub const METER: Self = Self {
        length: 1,
        mass: 0,
        time: 0,
        current: 0,
        temperature: 0,
        amount: 0,
        luminous_intensity: 0,
    };

    pub const SECOND: Self = Self {
        length: 0,
        mass: 0,
        time: 1,
        current: 0,
        temperature: 0,
        amount: 0,
        luminous_intensity: 0,
    };

    pub const KILOGRAM: Self = Self {
        length: 0,
        mass: 1,
        time: 0,
        current: 0,
        temperature: 0,
        amount: 0,
        luminous_intensity: 0,
    };

    pub fn mul(self, other: Self) -> Self {
        Self {
            length: self.length + other.length,
            mass: self.mass + other.mass,
            time: self.time + other.time,
            current: self.current + other.current,
            temperature: self.temperature + other.temperature,
            amount: self.amount + other.amount,
            luminous_intensity: self.luminous_intensity + other.luminous_intensity,
        }
    }

    pub fn div(self, other: Self) -> Self {
        Self {
            length: self.length - other.length,
            mass: self.mass - other.mass,
            time: self.time - other.time,
            current: self.current - other.current,
            temperature: self.temperature - other.temperature,
            amount: self.amount - other.amount,
            luminous_intensity: self.luminous_intensity - other.luminous_intensity,
        }
    }

    pub fn is_scalar(&self) -> bool {
        *self == Self::SCALAR
    }

    pub fn format_unit(&self) -> String {
        let mut parts = Vec::new();
        if self.mass != 0 { parts.push(format!("kg^{}", self.mass)); }
        if self.length != 0 { parts.push(format!("m^{}", self.length)); }
        if self.time != 0 { parts.push(format!("s^{}", self.time)); }
        if parts.is_empty() {
            "scalar".to_string()
        } else {
            parts.join("·")
        }
    }
}

pub struct DimensionChecker {
    pub diagnostics: Vec<AnalysisDiagnostic>,
}

impl Default for DimensionChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl DimensionChecker {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn check_addition_compatibility(
        &mut self,
        dim_a: DimensionVector,
        dim_b: DimensionVector,
        span: ByteSpan,
    ) {
        if dim_a != dim_b {
            self.diagnostics.push(AnalysisDiagnostic::error(
                "DIM001",
                format!(
                    "Incompatible dimensional units in addition/subtraction: '{}' vs '{}'",
                    dim_a.format_unit(),
                    dim_b.format_unit()
                ),
                span,
            ));
        }
    }

    pub fn check_node(
        &mut self,
        arena: &pilot_ast::AstArena,
        node_id: pilot_ast::NodeId,
        infer: &crate::infer::TypeInferenceEngine,
    ) {
        let node = *arena.node(node_id);
        if node.node_kind() == pilot_ast::NodeKind::BinaryExpr {
            let lhs = pilot_ast::NodeId(node.lhs);
            let rhs = pilot_ast::NodeId(node.rhs);
            if let (Some(&t1), Some(&t2)) = (
                infer.inferred_node_types.get(&lhs.0),
                infer.inferred_node_types.get(&rhs.0),
            ) {
                if let (
                    crate::types::TypeKind::Unit { unit_name: u1, .. },
                    crate::types::TypeKind::Unit { unit_name: u2, .. },
                ) = (
                    infer.types.get(infer.types.prune(t1)),
                    infer.types.get(infer.types.prune(t2)),
                ) {
                    if u1 != u2 {
                        self.diagnostics.push(AnalysisDiagnostic::error(
                            "DIM001",
                            format!("Unit dimension mismatch: '{}' vs '{}'", u1, u2),
                            arena.span(node_id),
                        ));
                    }
                }
            }
        }
    }
}
