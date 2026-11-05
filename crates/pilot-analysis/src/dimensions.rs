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
    pub unit_exponents: std::collections::HashMap<String, std::collections::BTreeMap<String, i32>>,
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
            unit_exponents: std::collections::HashMap::new(),
        }
    }

    pub fn register_base_unit(&mut self, unit_name: &str) {
        let mut map = std::collections::BTreeMap::new();
        map.insert(unit_name.to_string(), 1);
        self.unit_exponents.insert(unit_name.to_string(), map);
    }

    pub fn combine_unit_exponents(
        &self,
        lhs_unit: &str,
        op_flags: u16,
        rhs_unit: &str,
    ) -> std::collections::BTreeMap<String, i32> {
        let mut res = self
            .unit_exponents
            .get(lhs_unit)
            .cloned()
            .unwrap_or_else(|| {
                let mut m = std::collections::BTreeMap::new();
                m.insert(lhs_unit.to_string(), 1);
                m
            });
        let rhs_map = self
            .unit_exponents
            .get(rhs_unit)
            .cloned()
            .unwrap_or_else(|| {
                let mut m = std::collections::BTreeMap::new();
                m.insert(rhs_unit.to_string(), 1);
                m
            });
        let sign = if op_flags == 3 { 1 } else { -1 }; // 3 = Mul (*), 4/5 = Div (/)
        for (k, v) in rhs_map {
            let entry = res.entry(k.clone()).or_insert(0);
            *entry += sign * v;
            if *entry == 0 {
                res.remove(&k);
            }
        }
        res
    }

    pub fn register_derived_unit(&mut self, unit_name: &str, exps: std::collections::BTreeMap<String, i32>) {
        self.unit_exponents.insert(unit_name.to_string(), exps);
    }

    pub fn find_unit_by_exponents(&self, exps: &std::collections::BTreeMap<String, i32>) -> Option<String> {
        for (name, map) in &self.unit_exponents {
            if map == exps {
                return Some(name.clone());
            }
        }
        None
    }

    pub fn units_match(&self, u1: &str, u2: &str) -> bool {
        if u1 == u2 {
            return true;
        }
        if let (Some(m1), Some(m2)) = (self.unit_exponents.get(u1), self.unit_exponents.get(u2)) {
            return m1 == m2;
        }
        false
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
}
