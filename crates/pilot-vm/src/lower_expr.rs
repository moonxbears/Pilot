//! Node dispatch and expression lowering.

use crate::instruction::Opcode;
use crate::lower_native::native;
use crate::{BytecodeCompiler, CMap};
use pilot_parser::NativeKind;
use pilot_ast::{AstArena, BinaryOp, NodeId, NodeKind};
use pilot_rt::PilotVal;

pub(crate) const OP_RANGE_INCLUSIVE: u16 = BinaryOp::RangeInclusive as u16;
pub(crate) const OP_RANGE_HALF_OPEN: u16 = BinaryOp::RangeHalfOpen as u16;

/// Unary flags written by the parser (`UnaryExpr.flags`).
const UNARY_NEG: u16 = 0;
const UNARY_IDENTITY: u16 = 1;
const UNARY_BIT_NOT: u16 = 4;
const UNARY_NOT: u16 = 5;

/// Binary operators that map one-to-one onto a VM opcode.
fn direct_opcode(op: BinaryOp) -> Option<Opcode> {
    Some(match op {
        BinaryOp::Add | BinaryOp::WrapAdd => Opcode::Add,
        BinaryOp::Sub | BinaryOp::WrapSub => Opcode::Sub,
        BinaryOp::Mul | BinaryOp::WrapMul => Opcode::Mul,
        BinaryOp::Div => Opcode::Div,
        BinaryOp::FloorDiv => Opcode::FloorDiv,
        BinaryOp::Rem => Opcode::Rem,
        BinaryOp::Pow => Opcode::Pow,
        BinaryOp::Eq => Opcode::CmpEq,
        BinaryOp::Ne => Opcode::CmpNe,
        BinaryOp::Lt => Opcode::CmpLt,
        BinaryOp::Le => Opcode::CmpLe,
        BinaryOp::Gt => Opcode::CmpGt,
        BinaryOp::Ge => Opcode::CmpGe,
        BinaryOp::BitAnd => Opcode::BitAnd,
        BinaryOp::BitOr => Opcode::BitOr,
        BinaryOp::BitXor => Opcode::Xor,
        BinaryOp::Shl => Opcode::Shl,
        BinaryOp::Shr => Opcode::Shr,
        _ => return None,
    })
}

impl BytecodeCompiler {
    pub fn compile_node(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        if node_id.is_none() {
            return self.emit_null();
        }
        let node = *arena.node(node_id);
        self.cur_span = arena.span(node_id);
        match node.node_kind() {
            NodeKind::Program => self.compile_program(arena, node.lhs, node.rhs),
            NodeKind::BlockExpr => self.compile_block(arena, node.lhs, node.rhs),
            NodeKind::TupleExpr => self.compile_tuple(arena, node.lhs, node.rhs),
            NodeKind::IntLit => self.emit_const(PilotVal::from_int(
                arena.i64_pool.get(node.lhs as usize).copied().unwrap_or(0),
            )),
            NodeKind::FloatLit => self.emit_const(PilotVal::from_float(
                arena.f64_pool.get(node.lhs as usize).copied().unwrap_or(0.0),
            )),
            NodeKind::BoolLit => self.emit_const(PilotVal::from_bool(node.lhs != 0)),
            NodeKind::StrLit => self.emit_const(PilotVal::from_str(Self::name(arena, node.lhs))),
            NodeKind::NoneLit => self.emit_null(),
            NodeKind::IdentExpr => self.compile_ident(arena, node.lhs),
            NodeKind::MemberExpr => self.compile_member(arena, node_id),
            NodeKind::IndexExpr => self.compile_index(arena, node.lhs, node.rhs),
            NodeKind::LetStmt | NodeKind::VarStmt | NodeKind::ConstDecl => self.compile_let(arena, node_id),
            NodeKind::AssignStmt => self.compile_assign(arena, node.lhs, node.rhs),
            NodeKind::BinaryExpr => self.compile_binary(arena, node.flags, node.lhs, node.rhs),
            NodeKind::UnaryExpr => self.compile_unary(arena, node.flags, node.lhs),
            NodeKind::IfExpr => self.compile_if(arena, node_id),
            NodeKind::MatchExpr => self.compile_match(arena, node_id),
            NodeKind::LambdaExpr => self.compile_lambda(arena, node_id),
            NodeKind::GuardStmt => self.compile_guard(arena, node_id),
            NodeKind::TryPropagateExpr => self.compile_try_propagate(arena, node.lhs),
            NodeKind::WhileStmt => self.compile_while(arena, node.lhs, node.rhs),
            NodeKind::ForExpr => self.compile_for(arena, node_id),
            NodeKind::BreakStmt => self.compile_break(arena, NodeId(node.lhs).is_some()),
            NodeKind::ContinueStmt => self.compile_continue(arena, NodeId(node.lhs).is_some()),
            NodeKind::DeferStmt => self.compile_defer(node.lhs),
            NodeKind::TryStmt => self.compile_try_stmt(arena, node_id),
            NodeKind::CascadeExpr => self.compile_cascade(arena, node_id),
            NodeKind::ComptimeExpr => self.compile_comptime(arena, node_id),
            NodeKind::CatchRecoveryExpr => self.compile_catch_recovery(arena, node.lhs, node.rhs),
            NodeKind::ReturnStmt => self.compile_return(arena, node.lhs),
            NodeKind::ExprStmt => self.compile_node(arena, NodeId(node.lhs)),
            NodeKind::MapExpr => self.compile_map_literal(arena, node.lhs, node.rhs),
            NodeKind::ListExpr => self.compile_list_literal(arena, node.lhs, node.rhs),
            NodeKind::CallExpr => self.compile_call(arena, node.lhs, node.rhs),
            NodeKind::UnitLit => self.compile_unit_lit(arena, node.lhs),
            NodeKind::CallArg => self.compile_stray_named_arg(arena, node.lhs),
            NodeKind::FnDecl | NodeKind::StructDecl | NodeKind::ExtendDecl => {
                self.compile_local_decl(arena, node_id)
            }
            NodeKind::TypeAdtDecl => {
                self.register_enum(arena, node_id);
                self.emit_null()
            }
            // Declarations with no runtime effect of their own: imports are
            // resolved by the module loader, interfaces only constrain types.
            NodeKind::ImportDecl | NodeKind::InterfaceDecl => self.emit_null(),
            NodeKind::PolyglotBlock => {
                self.error("inline foreign-language blocks cannot run on the seed VM".to_string())
            }
            NodeKind::ErrorNode => self.error("syntax error".to_string()),
            other => self.error(format!("`{}` is not lowered by the seed yet", other.name())),
        }
    }

    fn compile_tuple(&mut self, arena: &AstArena, lhs: u32, rhs: u32) -> u8 {
        let children: Vec<u32> = arena.children_slice(lhs, rhs).to_vec();
        if children.len() == 1 {
            return self.compile_node(arena, NodeId(children[0]));
        }
        self.compile_list_literal(arena, lhs, rhs)
    }

    fn compile_ident(&mut self, arena: &AstArena, sym: u32) -> u8 {
        let name = Self::name(arena, sym);
        if let Some(&r) = self.var_slots.get(name) {
            return r;
        }
        if let Some(r) = self.capture_read(name) {
            return r;
        }
        if let Some(r) = self.compile_native_value(name) {
            return r;
        }
        if let Some(cm) = self.const_maps.get(name).cloned() {
            return self.materialize_cmap(&cm);
        }
        if self.globals.contains(name) {
            let name = name.to_string();
            return self.emit_global_get(&name);
        }
        if let Some(v) = self.unique_variant(name) {
            return self.emit_const(v);
        }
        if let Some(r) = self.function_value(name) {
            return r;
        }
        self.error(format!("undefined name `{name}`"))
    }

    fn compile_member(&mut self, arena: &AstArena, node_id: NodeId) -> u8 {
        let node = *arena.node(node_id);
        match self.resolve_cmap(arena, node_id) {
            Some(CMap::Val(v)) => return self.emit_const(v),
            Some(cm) => return self.materialize_cmap(&cm),
            None => {}
        }
        let field = Self::name(arena, node.rhs).to_string();
        // A property of a compile-time collection that is not one of its keys.
        if let (Some(container), Some(hook)) = (
            self.resolve_cmap(arena, NodeId(node.lhs)),
            native(NativeKind::Property, &field),
        ) {
            let dst = self.alloc_reg();
            let mark = self.next_reg;
            let r_obj = self.materialize_cmap(&container);
            let r = self.emit_intrinsic(hook, &[r_obj]);
            self.emit(Opcode::Move, dst, r, 0);
            self.release_to(mark);
            return dst;
        }
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r_obj = self.compile_node(arena, NodeId(node.lhs));
        // Positional fields (`tuple.0`) index the underlying sequence.
        let r = self.emit_field_read(r_obj, &field);
        self.emit(Opcode::Move, dst, r, 0);
        self.release_to(mark);
        dst
    }

    fn compile_index(&mut self, arena: &AstArena, lhs: u32, rhs: u32) -> u8 {
        if NodeId(rhs).is_none() {
            return self.error("empty index `[]` is not lowered by the seed yet".to_string());
        }
        if let Some(raw) = self.bracket_unit_text(arena, NodeId(lhs), NodeId(rhs)) {
            return self.compile_unit_text(&raw);
        }
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r = self.compile_index_value(arena, lhs, rhs);
        self.emit(Opcode::Move, dst, r, 0);
        self.release_to(mark);
        dst
    }

    fn compile_index_value(&mut self, arena: &AstArena, lhs: u32, rhs: u32) -> u8 {
        let idx = arena.node(NodeId(rhs));
        if idx.node_kind() == NodeKind::BinaryExpr && matches!(idx.flags, OP_RANGE_INCLUSIVE | OP_RANGE_HALF_OPEN) {
            let r_obj = self.compile_node(arena, NodeId(lhs));
            let r_a = self.compile_node(arena, NodeId(idx.lhs));
            let mut r_b = self.compile_node(arena, NodeId(idx.rhs));
            if idx.flags == OP_RANGE_INCLUSIVE {
                let r_one = self.emit_const(PilotVal::from_int(1));
                let r_end = self.alloc_reg();
                self.emit(Opcode::Add, r_end, r_b, r_one);
                r_b = r_end;
            }
            return self.emit_intrinsic("str_slice", &[r_obj, r_a, r_b]);
        }
        let r_obj = self.compile_node(arena, NodeId(lhs));
        let r_key = self.compile_node(arena, NodeId(rhs));
        self.emit_intrinsic("map_get", &[r_obj, r_key])
    }

    fn compile_binary(&mut self, arena: &AstArena, flags: u16, lhs: u32, rhs: u32) -> u8 {
        let Some(op) = BinaryOp::from_u16(flags) else {
            return self.error("unknown binary operator".to_string());
        };
        // `As` keeps its target type outside the operand slots.
        if NodeId(lhs).is_none() || (NodeId(rhs).is_none() && !matches!(op, BinaryOp::As | BinaryOp::AsView)) {
            return self.error(format!("operator `{}` is missing an operand", op.symbol()));
        }
        match op {
            BinaryOp::And | BinaryOp::Or => return self.compile_short_circuit(arena, op, lhs, rhs),
            BinaryOp::Coalesce => return self.compile_coalesce(arena, lhs, rhs),
            BinaryOp::Pipe => return self.compile_pipe(arena, NodeId(lhs), NodeId(rhs)),
            BinaryOp::PipeBackward => return self.compile_pipe(arena, NodeId(rhs), NodeId(lhs)),
            BinaryOp::As | BinaryOp::AsView => {
                let r = self.compile_node(arena, NodeId(lhs));
                let ty = NodeId(rhs);
                let layout = (op == BinaryOp::As && ty.is_some())
                    .then(|| pilot_parser::bootstrap_tables().reg_layout(Self::name(arena, arena.node(ty).lhs)))
                    .flatten()
                    .cloned();
                return match layout {
                    Some(layout) => self.emit_conversion(r, &layout),
                    None => r,
                };
            }
            _ => {}
        }
        let Some(opcode) = direct_opcode(op) else {
            if matches!(op, BinaryOp::RangeInclusive | BinaryOp::RangeHalfOpen) {
                return self.error(
                    "range values are only lowered as `for` iterables and slice indices by the seed".to_string(),
                );
            }
            if let Some(r) = self.compile_operator_fn(op.symbol(), arena, NodeId(lhs), NodeId(rhs)) {
                return r;
            }
            return self.error(format!("operator `{}` is not lowered by the seed yet", op.symbol()));
        };
        // `x == ::Variant`: each side's declared type types the other side.
        let (hint_l, hint_r) = if matches!(op, BinaryOp::Eq | BinaryOp::Ne) {
            (self.static_type(arena, NodeId(rhs)), self.static_type(arena, NodeId(lhs)))
        } else {
            (None, None)
        };
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r_l = self.compile_with_hint(arena, NodeId(lhs), hint_l);
        let r_r = self.compile_with_hint(arena, NodeId(rhs), hint_r);
        self.emit(opcode, dst, r_l, r_r);
        self.release_to(mark);
        dst
    }

    /// Declared type of a variable or call result, when statically known.
    pub(crate) fn static_type(&self, arena: &AstArena, id: NodeId) -> Option<String> {
        if id.is_none() {
            return None;
        }
        let n = arena.node(id);
        match n.node_kind() {
            NodeKind::IdentExpr => self.var_types.get(Self::name(arena, n.lhs)).cloned(),
            NodeKind::CallExpr if n.rhs > 0 => {
                let callee = arena.node(NodeId(*arena.extra_children.get(n.lhs as usize)?));
                if !matches!(callee.node_kind(), NodeKind::IdentExpr | NodeKind::TypeNamed) {
                    return None;
                }
                let name = Self::name(arena, callee.lhs);
                match self.fn_ret_types.get(name) {
                    Some(t) => Some(t.clone()),
                    None => self.structs.contains_key(name).then(|| name.to_string()),
                }
            }
            NodeKind::CascadeExpr => self.static_type(arena, NodeId(n.lhs)),
            _ => None,
        }
    }

    /// `a && b` / `a || b`: the right operand runs only when needed.
    fn compile_short_circuit(&mut self, arena: &AstArena, op: BinaryOp, lhs: u32, rhs: u32) -> u8 {
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r_l = self.compile_node(arena, NodeId(lhs));
        self.emit(Opcode::Move, dst, r_l, 0);
        self.release_to(mark);
        let skip_op = if op == BinaryOp::And { Opcode::JumpIfFalse } else { Opcode::JumpIfTrue };
        let skip = self.emit_branch(skip_op, dst, 0);
        let r_r = self.compile_node(arena, NodeId(rhs));
        // The right operand must itself be a boolean condition.
        let check = self.emit_branch(Opcode::JumpIfTrue, r_r, 0);
        self.patch_to_here(check);
        self.emit(Opcode::Move, dst, r_r, 0);
        self.release_to(mark);
        self.patch_to_here(skip);
        dst
    }

    /// `a ?? b`: `b` runs only when `a` is null.
    fn compile_coalesce(&mut self, arena: &AstArena, lhs: u32, rhs: u32) -> u8 {
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r_l = self.compile_node(arena, NodeId(lhs));
        self.emit(Opcode::Move, dst, r_l, 0);
        self.release_to(mark);
        let skip = self.emit_branch(Opcode::JumpIfNotNull, dst, 0);
        let r_r = self.compile_node(arena, NodeId(rhs));
        self.emit(Opcode::Move, dst, r_r, 0);
        self.release_to(mark);
        self.patch_to_here(skip);
        dst
    }

    fn compile_unary(&mut self, arena: &AstArena, flags: u16, operand: u32) -> u8 {
        let opcode = match flags {
            UNARY_IDENTITY => return self.compile_node(arena, NodeId(operand)),
            UNARY_NEG => Opcode::Neg,
            UNARY_BIT_NOT => Opcode::BitNot,
            UNARY_NOT => Opcode::Not,
            pilot_ast::UNARY_OP_SOME => return self.compile_some_test(arena, NodeId(operand)),
            pilot_ast::UNARY_OP_SPLAT | pilot_ast::UNARY_OP_DOUBLE_SPLAT | pilot_ast::UNARY_OP_DEEP_FLATTEN => {
                return self.error(
                    "the seed lowers `*x` and `*..x` only as elements of a list literal".to_string(),
                );
            }
            _ => return self.error("unary operator is not lowered by the seed yet".to_string()),
        };
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r_op = self.compile_node(arena, NodeId(operand));
        self.emit(opcode, dst, r_op, 0);
        self.release_to(mark);
        dst
    }

    pub(crate) fn compile_map_literal(&mut self, arena: &AstArena, lhs: u32, rhs: u32) -> u8 {
        let children: Vec<u32> = arena.children_slice(lhs, rhs).to_vec();
        let r_map = self.emit_intrinsic("map_new", &[]);
        let mark = self.next_reg;
        for pair in children.chunks(2) {
            if pair.len() != 2 {
                return self.error("map literal entry without a value".to_string());
            }
            let r_k = match Self::map_key(arena, NodeId(pair[0])) {
                Some(k) => self.emit_const(PilotVal::from_str(&k)),
                None => self.compile_node(arena, NodeId(pair[0])),
            };
            let r_v = self.compile_node(arena, NodeId(pair[1]));
            self.emit_intrinsic("map_set", &[r_map, r_k, r_v]);
            self.release_to(mark);
        }
        r_map
    }

    fn compile_list_literal(&mut self, arena: &AstArena, lhs: u32, rhs: u32) -> u8 {
        let children: Vec<u32> = arena.children_slice(lhs, rhs).to_vec();
        let r_list = self.emit_intrinsic("list_new", &[]);
        let mark = self.next_reg;
        for c in children {
            let cn = *arena.node(NodeId(c));
            let unpack = (cn.node_kind() == NodeKind::UnaryExpr).then_some(cn.flags);
            match unpack {
                // `*xs` adds xs's elements; `*..xs` adds its leaves at any depth.
                Some(f @ (pilot_ast::UNARY_OP_SPLAT | pilot_ast::UNARY_OP_DEEP_FLATTEN)) => {
                    let r_v = self.compile_node(arena, NodeId(cn.lhs));
                    let r_deep = self.emit_const(PilotVal::from_bool(f == pilot_ast::UNARY_OP_DEEP_FLATTEN));
                    self.emit_intrinsic("list_extend", &[r_list, r_v, r_deep]);
                }
                Some(pilot_ast::UNARY_OP_DOUBLE_SPLAT) => {
                    self.error("`**x` unpacks key-value pairs, which a list literal cannot hold".to_string());
                }
                _ => {
                    let r_v = self.compile_node(arena, NodeId(c));
                    self.emit_intrinsic("list_push", &[r_list, r_v]);
                }
            }
            self.release_to(mark);
        }
        r_list
    }
}
