//! Dimensional literals: `250ms`, `64KB`, `2.4GHz`, and the bracketed
//! spelling `100[m]`, which means `100m`.
//!
//! A Pilot module names the functions that value these literals (see
//! `pilot_parser::UnitLiteralHook`; today `stdlib/core/syntax/unit_decl.pilot`).
//! The seed runs them on each literal while compiling and emits the result,
//! the value in the unit family's base unit, as a constant. A literal whose
//! unit the module does not know is a compile error.

use crate::BytecodeCompiler;
use crate::PilVirtualMachine;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_rt::PilotVal;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Values already computed, by literal text.
static UNIT_VALUES: OnceLock<Mutex<HashMap<String, Result<i64, String>>>> = OnceLock::new();

impl BytecodeCompiler {
    /// Lowers the dimensional literal whose text is the symbol `sym`.
    pub(crate) fn compile_unit_lit(&mut self, arena: &AstArena, sym: u32) -> u8 {
        let raw = Self::name(arena, sym).to_string();
        self.compile_unit_text(&raw)
    }

    /// Lowers the dimensional literal spelled `raw` (`250ms`).
    pub(crate) fn compile_unit_text(&mut self, raw: &str) -> u8 {
        match unit_value(raw) {
            Ok(v) => self.emit_const(PilotVal::from_int(v)),
            Err(message) => self.error(crate::codes::UNKNOWN_UNIT, message),
        }
    }

    /// The literal `number[unit]` stands for, written as `numberunit`. A number
    /// cannot be indexed, so a number literal indexed by a name that is not a
    /// variable is always a unit.
    pub(crate) fn bracket_unit_text(&self, arena: &AstArena, num: NodeId, unit: NodeId) -> Option<String> {
        let (n, u) = (arena.node(num), arena.node(unit));
        if u.node_kind() != NodeKind::IdentExpr {
            return None;
        }
        let name = Self::name(arena, u.lhs);
        if self.var_slots.contains_key(name) || self.globals.contains(name) {
            return None;
        }
        let digits = match n.node_kind() {
            NodeKind::IntLit => arena.i64_pool.get(n.lhs as usize)?.to_string(),
            NodeKind::FloatLit => arena.f64_pool.get(n.lhs as usize)?.to_string(),
            _ => return None,
        };
        Some(format!("{digits}{name}"))
    }
}

/// The value of the dimensional literal `raw`, from the registered Pilot
/// functions.
fn unit_value(raw: &str) -> Result<i64, String> {
    let cache = UNIT_VALUES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(v) = cache.lock().map_err(|e| e.to_string())?.get(raw) {
        return v.clone();
    }
    let v = compute_unit_value(raw);
    cache
        .lock()
        .map_err(|e| e.to_string())?
        .insert(raw.to_string(), v.clone());
    v
}

fn compute_unit_value(raw: &str) -> Result<i64, String> {
    let hook = pilot_parser::bootstrap_tables()
        .unit_literal()
        .ok_or_else(|| format!("`{raw}`: no Pilot module registers dimensional literals"))?;
    if run_pilot_fn(&hook.module, &hook.classify, raw)? == 0 {
        return Err(format!("`{raw}` has a unit that no unit family declares"));
    }
    run_pilot_fn(&hook.module, &hook.convert, raw)
}

/// Runs `module`'s function `func` on the string `arg` and returns its integer
/// result.
fn run_pilot_fn(module: &str, func: &str, arg: &str) -> Result<i64, String> {
    let script = format!("{module}\n{func}(\"{}\");\n", arg.escape_default());
    let arena = pilot_parser::parse(&script);
    if let Some(d) = arena.diagnostics.first() {
        return Err(format!("`{func}` does not parse: {d:?}"));
    }
    let bin = BytecodeCompiler::compile(&arena, arena.root);
    if let Some(d) = bin.diagnostics.first() {
        return Err(format!("`{func}` does not compile: {}", d.message));
    }
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(bin);
    let v = vm
        .run_checked()
        .map_err(|f| format!("`{func}(\"{arg}\")` failed: {f:?}"))?;
    v.as_int()
        .ok_or_else(|| format!("`{func}(\"{arg}\")` did not return an integer"))
}
