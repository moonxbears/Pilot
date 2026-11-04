//! Milestone 33.4: Perceus Precise ARC Insertion & Drop-Guided In-Place Memory Reuse.

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArcOp {
    /// Increments reference count at point of cloning/sharing
    Dup { var_name: String },
    /// Decrements reference count at point of last use
    Drop { var_name: String },
    /// In-place recycling token: reuses memory block of dropped object if rc == 1
    DropReuse { dropped_var: String, target_var: String },
    /// Deterministic RAII `~self()` invocation at scope exit (required for `IGuard` `0b11` and `IObject` `0b10`)
    ScopeExitDrop { var_name: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BindingLifecycleState {
    pub lattice_bits: u8,
    pub move_by_default: bool,
    pub is_moved: bool,
    pub allow_early_last_use_drop: bool,
    pub needs_scope_exit_drop: bool,
}

pub struct PerceusArcEngine {
    ops: Vec<ArcOp>,
    drop_reuse_count: usize,
    elided_retains_count: usize,
    bindings: HashMap<String, BindingLifecycleState>,
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
            bindings: HashMap::new(),
        }
    }

    /// 1-cycle bitwise lattice check for local binding lifecycle registration:
    /// - If `(lattice_bits & 0b10) == 0` (`IValue` `0b00` or `IReference` `0b01`), skips drop-scope
    ///   insertion and move-invalidation entirely.
    /// - If `(lattice_bits & 0b10) != 0` (`IObject` `0b10` or `IGuard` `0b11`), marks the binding as
    ///   move-by-default on assignment/argument passing and schedules deterministic `~self()` invocation
    ///   at scope exit (or early drop on last use if `(lattice_bits & 0b01) == 0`, i.e. not an `IGuard`).
    #[inline]
    pub fn track_binding_by_lattice(&mut self, var_name: &str, lattice_bits: u8) -> Option<BindingLifecycleState> {
        if (lattice_bits & 0b10) == 0 {
            self.elided_retains_count += 1;
            return None;
        }
        let is_guard = (lattice_bits & 0b01) != 0;
        let state = BindingLifecycleState {
            lattice_bits: lattice_bits & 0b11,
            move_by_default: true,
            is_moved: false,
            allow_early_last_use_drop: !is_guard,
            needs_scope_exit_drop: true,
        };
        self.bindings.insert(var_name.to_string(), state);
        Some(state)
    }

    /// Marks a binding as moved when assigned or passed by value if `(lattice_bits & 0b10) != 0`.
    /// Returns `Err` if the binding was already moved (`use-after-move`).
    pub fn record_move_or_copy(&mut self, var_name: &str, lattice_bits: u8) -> Result<bool, String> {
        if (lattice_bits & 0b10) == 0 {
            // Trivial bit-copy (`IValue` `0b00` or `IReference` `0b01`)
            return Ok(false);
        }
        if let Some(state) = self.bindings.get_mut(var_name) {
            if state.is_moved {
                return Err(format!(
                    "Use of moved value `{}` (lattice `{:#04b}` is move-by-default)",
                    var_name, state.lattice_bits
                ));
            }
            state.is_moved = true;
            state.needs_scope_exit_drop = false;
        }
        Ok(true)
    }

    /// Emits early drop on last use for `IObject` (`0b10`, where `(lattice_bits & 0b01) == 0`),
    /// or defers to scope exit for `IGuard` (`0b11`).
    pub fn emit_drop_for_binding(&mut self, var_name: &str, at_scope_exit: bool) {
        if let Some(state) = self.bindings.get_mut(var_name) {
            if state.is_moved || !state.needs_scope_exit_drop {
                return;
            }
            if !at_scope_exit && state.allow_early_last_use_drop {
                state.needs_scope_exit_drop = false;
                self.ops.push(ArcOp::Drop {
                    var_name: var_name.to_string(),
                });
            } else if at_scope_exit {
                state.needs_scope_exit_drop = false;
                self.ops.push(ArcOp::ScopeExitDrop {
                    var_name: var_name.to_string(),
                });
            }
        }
    }

    pub fn binding_state(&self, var_name: &str) -> Option<&BindingLifecycleState> {
        self.bindings.get(var_name)
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
