//! Milestone 33.4: Perceus Precise ARC Insertion & Drop-Guided In-Place Memory Reuse.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArcOp {
    /// Increments reference count at point of cloning/sharing
    Dup { var_name: String },
    /// Decrements reference count at point of last use
    Drop { var_name: String },
    /// In-place recycling token: reuses memory block of dropped object if rc == 1
    DropReuse { dropped_var: String, target_var: String },
}

pub struct PerceusArcEngine {
    ops: Vec<ArcOp>,
    drop_reuse_count: usize,
    elided_retains_count: usize,
}

impl Default for PerceusArcEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PerceusArcEngine {
    pub fn new() -> Self {
        Self {
            ops: Vec::new(),
            drop_reuse_count: 0,
            elided_retains_count: 0,
        }
    }

    pub fn insert_dup(&mut self, var_name: &str) {
        self.ops.push(ArcOp::Dup {
            var_name: var_name.to_string(),
        });
    }

    pub fn insert_drop(&mut self, var_name: &str) {
        self.ops.push(ArcOp::Drop {
            var_name: var_name.to_string(),
        });
    }

    pub fn insert_drop_reuse(&mut self, dropped_var: &str, target_var: &str) {
        self.ops.push(ArcOp::DropReuse {
            dropped_var: dropped_var.to_string(),
            target_var: target_var.to_string(),
        });
        self.drop_reuse_count += 1;
    }

    pub fn elide_borrowed_retain(&mut self) {
        self.elided_retains_count += 1;
    }

    pub fn ops(&self) -> &[ArcOp] {
        &self.ops
    }

    pub fn drop_reuse_count(&self) -> usize {
        self.drop_reuse_count
    }

    pub fn elided_retains_count(&self) -> usize {
        self.elided_retains_count
    }
}
