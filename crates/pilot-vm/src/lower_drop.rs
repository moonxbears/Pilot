//! F7: dropping owned values where the drop plan says.
//!
//! The plan comes from the self-hosted checker (`selfhost/check/drops.pilot`),
//! which knows the program's types; the seed only carries it out. Each entry
//! names a statement or expression by its span and node kind:
//!
//! - `before` / `after`: drop the named locals before or after it runs;
//! - `exit`: a `return`, `break` or `continue` drops them before it leaves
//!   (after a returned value is computed);
//! - `on_else`: an `if` without `else` drops them on its false path;
//! - `replace`: an assignment drops the value its target held, after the
//!   new value is made and before it is stored;
//! - `guard`: a `let` binds a scoped guard, released when its block ends,
//!   with the block's deferred statements (last in, first out);
//! - `seams`: the node stops at a seam when it runs.
//!
//! Dropping a value calls its type's destructor, then drops the fields the
//! type's glue lists, in order. Types are named only as the plan names them.

use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId};
use std::collections::HashMap;
use std::rc::Rc;

/// What a row of the drop plan says, numbered as `DropAction` in
/// `selfhost/check/drops.pilot` numbers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropAction {
    Glue = 0,
    Before = 1,
    After = 2,
    Exit = 3,
    Else = 4,
    Guard = 5,
    Seam = 6,
    Copy = 7,
    CopyGlue = 8,
    Replace = 9,
}

impl DropAction {
    pub fn from_code(code: i64) -> Option<Self> {
        [
            Self::Glue,
            Self::Before,
            Self::After,
            Self::Exit,
            Self::Else,
            Self::Guard,
            Self::Seam,
            Self::Copy,
            Self::CopyGlue,
            Self::Replace,
        ]
            .into_iter()
            .find(|a| *a as i64 == code)
    }
}

/// A node of the program, by its span and kind.
pub type NodeKey = (u32, u32, u16);

/// A local to drop, and the type its glue is listed under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropItem {
    pub binding: String,
    pub ty: String,
}

/// How to drop a value of one type: the type whose destructor runs (none
/// when it has none), then each field to drop with the field's type.
#[derive(Clone, Debug, Default)]
pub struct DropGlue {
    pub destructor_owner: Option<String>,
    pub fields: Vec<(String, String)>,
}

/// Where the program's owned values are dropped.
#[derive(Clone, Debug, Default)]
pub struct DropPlan {
    pub before: HashMap<NodeKey, Vec<DropItem>>,
    pub after: HashMap<NodeKey, Vec<DropItem>>,
    pub exit: HashMap<NodeKey, Vec<DropItem>>,
    pub on_else: HashMap<NodeKey, Vec<DropItem>>,
    pub replace: HashMap<NodeKey, Vec<DropItem>>,
    pub guard: HashMap<NodeKey, Vec<DropItem>>,
    /// Seam id and note.
    pub seams: HashMap<NodeKey, Vec<(String, String)>>,
    pub glue: HashMap<String, DropGlue>,
    /// Nodes whose value is copied, with the value type it is copied as.
    pub copies: HashMap<NodeKey, String>,
    /// The value-type fields a copy of each value type copies too.
    pub copy_glue: HashMap<String, Vec<(String, String)>>,
}

impl DropPlan {
    pub fn is_empty(&self) -> bool {
        self.before.is_empty()
            && self.after.is_empty()
            && self.exit.is_empty()
            && self.on_else.is_empty()
            && self.replace.is_empty()
            && self.guard.is_empty()
            && self.seams.is_empty()
            && self.copies.is_empty()
    }
}

/// What a block runs when it is left.
#[derive(Clone, Debug)]
pub(crate) enum Deferred {
    /// A `defer` statement.
    Stmt(NodeId),
    /// Releasing a scoped guard.
    Drop(DropItem),
}

/// The seams a drop plan may stop a node at, each with the start of the
/// message it stops with; the plan's note about the node follows it.
const PLAN_SEAMS: &[(&str, &str)] = &[
    ("own.capture", "SEAM001: unconnected seam `own.capture`"),
    ("own.collection_drop", "SEAM001: unconnected seam `own.collection_drop`"),
    ("own.conditional_move", "SEAM001: unconnected seam `own.conditional_move`"),
    ("own.reassign", "SEAM001: unconnected seam `own.reassign`"),
    ("own.try_exit", "SEAM001: unconnected seam `own.try_exit`"),
];

/// Glue nesting deeper than this is a planner error: a type that holds
/// itself does so through an `opt` or a collection, which the plan stops at
/// `own.collection_drop`.
const MAX_GLUE_DEPTH: usize = 32;

fn key_of(arena: &AstArena, id: NodeId) -> NodeKey {
    let span = arena.span(id);
    (span.start, span.end, arena.node(id).kind)
}

impl BytecodeCompiler {
    pub(crate) fn set_drop_plan(&mut self, plan: DropPlan) {
        if !plan.is_empty() {
            self.drop_plan = Some(Rc::new(plan));
        }
    }

    fn plan_items(&self, id: NodeKey, pick: fn(&DropPlan) -> &HashMap<NodeKey, Vec<DropItem>>) -> Vec<DropItem> {
        self.drop_plan
            .as_ref()
            .and_then(|p| pick(p).get(&id).cloned())
            .unwrap_or_default()
    }

    /// Runs before node `id` is compiled: its seams, then its `before`
    /// drops. A `return`, `break` or `continue` keeps its `exit` drops for
    /// its own lowering.
    pub(crate) fn drops_before(&mut self, arena: &AstArena, id: NodeId) {
        if self.drop_plan.is_none() || id.is_none() {
            return;
        }
        let key = key_of(arena, id);
        let seams = self
            .drop_plan
            .as_ref()
            .and_then(|p| p.seams.get(&key).cloned())
            .unwrap_or_default();
        if let Some((seam, note)) = seams.first() {
            let message = match PLAN_SEAMS.iter().find(|(id, _)| id == seam) {
                Some((_, head)) => format!("{head}: {note}"),
                None => format!("internal error: the drop plan names `{seam}`, which is not a drop seam"),
            };
            self.emit_trap(message);
        }
        let before = self.plan_items(key, |p| &p.before);
        self.emit_drops(arena, &before);
        let exit = self.plan_items(key, |p| &p.exit);
        if !exit.is_empty() {
            self.pending_exit_drops = exit;
        }
        let replace = self.plan_items(key, |p| &p.replace);
        if !replace.is_empty() {
            self.pending_replace = replace;
        }
    }

    /// Takes the `replace` drops of the assignment about to be lowered; the
    /// assignment emits them once its new value is made.
    pub(crate) fn take_replace_drops(&mut self) -> Vec<DropItem> {
        std::mem::take(&mut self.pending_replace)
    }

    /// Runs after node `id` is compiled: its `after` drops, and the guards
    /// it binds join its block's deferred releases.
    pub(crate) fn drops_after(&mut self, arena: &AstArena, id: NodeId) {
        if self.drop_plan.is_none() || id.is_none() {
            return;
        }
        let key = key_of(arena, id);
        let after = self.plan_items(key, |p| &p.after);
        self.emit_drops(arena, &after);
        for g in self.plan_items(key, |p| &p.guard) {
            if let Some(scope) = self.defers.last_mut() {
                scope.push(Deferred::Drop(g));
            }
        }
    }

    /// The value of node `id`, in register `r`, or its copy when the plan
    /// copies it.
    pub(crate) fn copy_if_planned(&mut self, arena: &AstArena, id: NodeId, r: u8) -> u8 {
        let Some(ty) = self.drop_plan.as_ref().and_then(|p| p.copies.get(&key_of(arena, id)).cloned()) else {
            return r;
        };
        self.emit_copy_value(r, &ty, 0)
    }

    /// A copy of the value in register `r`, of value type `ty`, in a new
    /// register.
    fn emit_copy_value(&mut self, r: u8, ty: &str, depth: usize) -> u8 {
        let copy = self.emit_intrinsic("record_copy", &[r]);
        if depth > MAX_GLUE_DEPTH {
            self.emit_trap(format!(
                "internal error: the copy glue of `{ty}` nests more than {MAX_GLUE_DEPTH} levels of fields"
            ));
            return copy;
        }
        let fields = self.drop_plan.as_ref().and_then(|p| p.copy_glue.get(ty).cloned()).unwrap_or_default();
        for (field, field_ty) in fields {
            let mark = self.next_reg;
            let rf = self.emit_member_read(copy, Some(ty), &field);
            let rc = self.emit_copy_value(rf, &field_ty, depth + 1);
            let rk = self.emit_const(crate::PilotVal::from_str(&field));
            self.emit_intrinsic("map_set", &[copy, rk, rc]);
            self.release_to(mark);
        }
        copy
    }

    /// The `on_else` drops of `if` node `id`.
    pub(crate) fn else_drops(&self, arena: &AstArena, id: NodeId) -> Vec<DropItem> {
        if self.drop_plan.is_none() {
            return Vec::new();
        }
        self.plan_items(key_of(arena, id), |p| &p.on_else)
    }

    /// Emits the drops a `return`, `break` or `continue` makes as it leaves.
    pub(crate) fn emit_exit_drops(&mut self, arena: &AstArena) {
        let items = std::mem::take(&mut self.pending_exit_drops);
        self.emit_drops(arena, &items);
    }

    pub(crate) fn emit_drops(&mut self, arena: &AstArena, items: &[DropItem]) {
        for item in items {
            let Some(&r) = self.var_slots.get(&item.binding) else {
                continue;
            };
            let _ = arena;
            self.emit_drop_value(r, &item.ty, 0);
        }
    }

    /// Drops the value in register `r`, of the type the glue lists as `ty`.
    fn emit_drop_value(&mut self, r: u8, ty: &str, depth: usize) {
        let Some(glue) = self.drop_plan.as_ref().and_then(|p| p.glue.get(ty).cloned()) else {
            return;
        };
        if depth > MAX_GLUE_DEPTH {
            self.emit_trap(format!(
                "internal error: the drop glue of `{ty}` nests more than {MAX_GLUE_DEPTH} levels of fields"
            ));
            return;
        }
        let mark = self.next_reg;
        if let Some(owner) = &glue.destructor_owner {
            let dtor = Self::destructor_name(owner);
            self.emit_user_call(dtor.clone(), dtor, &[r]);
        }
        for (field, field_ty) in &glue.fields {
            let rf = self.emit_member_read(r, Some(ty), field);
            self.emit_drop_value(rf, field_ty, depth + 1);
        }
        self.release_to(mark);
    }
}
