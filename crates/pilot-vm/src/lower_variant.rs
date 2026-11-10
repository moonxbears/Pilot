//! Enum variants that carry values: `enum Shape { Circle(r: f64), Empty }`.
//!
//! A variant without a payload is its integer discriminant. A variant with
//! one is a record built as that variant (`Shape.Circle`), holding each value
//! under its declared field name, so `shape.r` reads it. `Shape.Circle(2.0)`,
//! `Shape::Circle(2.0)` and a bare `Circle(2.0)` (when only one enum declares
//! `Circle`) build one. The pattern `case Circle(r):` matches a record built
//! as that variant and matches each value against its sub-pattern by
//! position; `case Circle:` matches the variant whatever its values.

use crate::instruction::Opcode;
use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_rt::PilotVal;

impl BytecodeCompiler {
    /// Records the payload fields of variant `variant` of enum `owner`.
    pub(crate) fn register_variant_payload(&mut self, arena: &AstArena, owner: &str, variant: NodeId) {
        let vn = *arena.node(variant);
        let fields: Vec<String> = arena
            .children_slice(vn.rhs, vn.flags as u32)
            .iter()
            .map(|&f| arena.node(NodeId(f)))
            .filter(|f| f.node_kind() == NodeKind::ParamDecl)
            .map(|f| Self::name(arena, f.lhs).to_string())
            .collect();
        if !fields.is_empty() {
            let full = format!("{owner}.{}", Self::name(arena, vn.lhs));
            self.variant_payloads.insert(full, fields);
        }
    }

    /// The `Enum.Variant` a name refers to, when that variant has a payload.
    /// A qualified name names its enum; a bare one is looked up in `hint`'s
    /// enum, then in every enum, and must be unique.
    pub(crate) fn payload_variant(&self, name: &str, hint: Option<&str>) -> Option<String> {
        let mut parts = name.rsplit('.');
        let variant = parts.next()?;
        if let Some(owner) = parts.next() {
            let full = format!("{owner}.{variant}");
            return self.variant_payloads.contains_key(&full).then_some(full);
        }
        if let Some(h) = hint {
            let full = format!("{h}.{variant}");
            if self.variant_payloads.contains_key(&full) {
                return Some(full);
            }
        }
        let suffix = format!(".{variant}");
        let mut found = self.variant_payloads.keys().filter(|k| k.ends_with(&suffix));
        let first = found.next()?;
        found.next().is_none().then(|| first.clone())
    }

    /// Builds the variant `full` from `args`.
    pub(crate) fn compile_variant_ctor(&mut self, arena: &AstArena, full: &str, args: &[NodeId]) -> u8 {
        let fields = self.variant_payloads.get(full).cloned().unwrap_or_default();
        if fields.len() != args.len() {
            return self.error(
                crate::codes::ARG_COUNT,
                format!(
                    "`{full}` carries {} value(s), but {} were given",
                    fields.len(),
                    args.len()
                ),
            );
        }
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r_obj = self.emit_intrinsic("map_new", &[]);
        let r_tag = self.emit_const(PilotVal::from_str(full));
        self.emit_intrinsic("record_set_type", &[r_obj, r_tag]);
        for (field, &arg) in fields.iter().zip(args) {
            let r_v = self.compile_node(arena, arg);
            let r_k = self.emit_const(PilotVal::from_str(field));
            self.emit_intrinsic("map_set", &[r_obj, r_k, r_v]);
        }
        self.emit(Opcode::Move, dst, r_obj, 0);
        self.release_to(mark);
        dst
    }

    /// Tests `r_subject` against the payload variant pattern `pat` for `full`.
    pub(crate) fn compile_payload_pattern(
        &mut self,
        arena: &AstArena,
        pat: NodeId,
        full: &str,
        r_subject: u8,
        fails: &mut Vec<usize>,
    ) {
        let pn = *arena.node(pat);
        let subs = arena.children_slice(pn.rhs, pn.flags as u32).to_vec();
        let fields = self.variant_payloads.get(full).cloned().unwrap_or_default();
        if !subs.is_empty() && subs.len() != fields.len() {
            self.error(
                crate::codes::PATTERN,
                format!(
                    "`{full}` carries {} value(s); the pattern matches {}",
                    fields.len(),
                    subs.len()
                ),
            );
            return;
        }
        let r_type = self.emit_intrinsic("record_type", &[r_subject]);
        let r_tag = self.emit_const(PilotVal::from_str(full));
        let r_is = self.alloc_reg();
        self.emit(Opcode::CmpEq, r_is, r_type, r_tag);
        fails.push(self.emit_branch(Opcode::JumpIfFalse, r_is, 0));
        for (field, &sub) in fields.iter().zip(&subs) {
            let r_k = self.emit_const(PilotVal::from_str(field));
            let r_e = self.emit_intrinsic("map_get", &[r_subject, r_k]);
            self.compile_pattern(arena, NodeId(sub), r_e, &None, fails);
        }
    }
}
