//! Calls, methods, properties and values the seed answers with engine
//! intrinsics.
//!
//! Which names these are, and which intrinsic each runs, is declared in
//! Pilot (`stdlib/core/builtins/native_calls.pilot`) and read from the parser
//! tables; no such name is written here. `__engine(..)`, a seed token, calls
//! an intrinsic by name, and the receptor `nameof` quotes its argument.

use crate::instruction::Opcode;
use crate::value::PilotVal;
use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_parser::NativeKind;

/// The seed token that runs one machine operation: `asm(op, operands..)`.
const ASM_TOKEN: &str = "asm";

/// The VM instruction of each machine operation, by its position in the
/// enum `register_machine_ops` names (stdlib/core/asm/machine_ops.pilot,
/// append-only). `None`: the seed has no instruction that does exactly that
/// operation (unsigned division, comparison and shift, conversions,
/// saturating and Euclidean forms, three-way compares, memory access).
const MACHINE_OPS: &[Option<Opcode>] = &[
    Some(Opcode::Add),
    Some(Opcode::Sub),
    Some(Opcode::Mul),
    Some(Opcode::Div),
    None,
    Some(Opcode::Rem),
    None,
    Some(Opcode::Add),
    Some(Opcode::Sub),
    Some(Opcode::Mul),
    Some(Opcode::Div),
    Some(Opcode::BitAnd),
    Some(Opcode::BitOr),
    Some(Opcode::Xor),
    Some(Opcode::Shl),
    None,
    Some(Opcode::Shr),
    Some(Opcode::CmpEq),
    Some(Opcode::CmpNe),
    None,
    None,
    None,
    None,
    Some(Opcode::CmpLt),
    Some(Opcode::CmpLe),
    Some(Opcode::CmpGt),
    Some(Opcode::CmpGe),
    Some(Opcode::CmpLt),
    Some(Opcode::CmpLe),
    Some(Opcode::CmpEq),
    Some(Opcode::Neg),
    Some(Opcode::Neg),
    Some(Opcode::Not),
    Some(Opcode::BitNot),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some(Opcode::Add),
    Some(Opcode::Sub),
    Some(Opcode::Mul),
];

/// The question a `TypeQuery` binding asks: the type's size in bytes.
const QUERY_BYTE_SIZE: &str = "byte_size";

/// The engine intrinsic (or question) bound to `name` in table `kind`.
pub(crate) fn native(kind: NativeKind, name: &str) -> Option<&'static str> {
    pilot_parser::bootstrap_tables().native(kind, name)
}

impl BytecodeCompiler {
    /// `name(args)` when `name` is native, quoting or the seed token
    /// `__engine`; `None` otherwise.
    pub(crate) fn compile_native_call(
        &mut self,
        arena: &AstArena,
        name: &str,
        args: &[NodeId],
    ) -> Option<u8> {
        if name == crate::ENGINE_TOKEN {
            return Some(self.compile_engine_call(arena, args));
        }
        if name == ASM_TOKEN {
            return Some(self.compile_asm(arena, args));
        }
        if pilot_parser::bootstrap_tables().quotes_symbol(name) {
            return Some(self.compile_symbol_quote(arena, name, args));
        }
        if let Some(query) = native(NativeKind::TypeQuery, name) {
            return Some(self.compile_type_query(arena, name, query, args));
        }
        let spread: Vec<NodeId> = match args {
            [only] if arena.node(*only).node_kind() == NodeKind::TupleExpr => {
                let tn = arena.node(*only);
                arena
                    .children_slice(tn.lhs, tn.rhs)
                    .iter()
                    .map(|&c| NodeId(c))
                    .collect()
            }
            _ => args.to_vec(),
        };
        if let Some(hook) = native(NativeKind::Function, name) {
            let regs = self.compile_args(arena, &spread);
            return Some(self.emit_intrinsic(hook, &regs));
        }
        let (hook, defs, call_args) = pilot_parser::bootstrap_tables().call_hook(name)?;
        let hook_owned = hook.to_string();
        let passed = self.compile_args(arena, &spread);
        let mapped: Vec<u8> =
            call_args
                .into_iter()
                .map(|ca| match ca {
                    Ok(p) => passed.get(p).copied().unwrap_or_else(|| {
                        match defs.get(p).and_then(|d| *d) {
                            Some(Ok(n)) => self.emit_const(PilotVal::from_int(n)),
                            Some(Err(s)) => self.emit_const(PilotVal::from_str(s)),
                            None => self.emit_null(),
                        }
                    }),
                    Err(Ok(n)) => self.emit_const(PilotVal::from_int(n)),
                    Err(Err(s)) => self.emit_const(PilotVal::from_str(s)),
                })
                .collect();
        Some(self.emit_intrinsic(&hook_owned, &mapped))
    }

    /// `__engine("intrinsic", args..)`.
    /// `asm(MachineOp::Add, a, b)` (or `asm(::Add, a, b)`): the VM
    /// instruction of that machine operation on the operands. A wrong
    /// operation or operand count is the checker's to report (ASM001,
    /// ASM002, selfhost/check/asm.pilot); the seed only stops there.
    fn compile_asm(&mut self, arena: &AstArena, args: &[NodeId]) -> u8 {
        let variant = args.first().map_or("", |&op| {
            let on = arena.node(op);
            match on.node_kind() {
                NodeKind::MemberExpr => Self::name(arena, on.rhs),
                NodeKind::IdentExpr => Self::name(arena, on.lhs),
                _ => "",
            }
        });
        let index = pilot_parser::bootstrap_tables().machine_op(variant);
        let Some(opcode) = index.and_then(|i| MACHINE_OPS.get(i).copied().flatten()) else {
            return self.emit_trap(format!(
                "SEAM001: unconnected seam `asm.machine_op`: the seed has no instruction for `{variant}`"
            ));
        };
        let unary = matches!(opcode, Opcode::Neg | Opcode::Not | Opcode::BitNot);
        let dst = self.alloc_reg();
        let mark = self.next_reg;
        let r_a = args.get(1).map_or(0, |&a| self.compile_node(arena, a));
        let r_b = if unary {
            0
        } else {
            args.get(2).map_or(0, |&b| self.compile_node(arena, b))
        };
        self.emit(opcode, dst, r_a, r_b);
        self.release_to(mark);
        dst
    }

    fn compile_engine_call(&mut self, arena: &AstArena, args: &[NodeId]) -> u8 {
        let Some((&hook, rest)) = args.split_first() else {
            return self.error(
                crate::codes::SYNTAX,
                format!("{} expects an intrinsic name", crate::ENGINE_TOKEN),
            );
        };
        // A name known while compiling is a constant: the call reads it
        // from there.
        let hn = arena.node(hook);
        let known = match hn.node_kind() {
            NodeKind::StrLit => Some(PilotVal::from_str(Self::name(arena, hn.lhs))),
            _ => match self.resolve_cmap(arena, hook) {
                Some(crate::CMap::Val(v)) if v.is_str() => Some(v),
                _ => None,
            },
        };
        if let Some(name) = known {
            let c = self.add_constant(name);
            let regs = self.compile_args(arena, rest);
            let dst = self.alloc_reg();
            self.emit_named_intrinsic(dst, c, &regs);
            return dst;
        }
        let r_hook = self.compile_node(arena, hook);
        let regs = self.compile_args(arena, rest);
        self.stage_args(&regs);
        let dst = self.alloc_reg();
        self.emit(Opcode::EngineIntrinsic, dst, r_hook, regs.len() as u8);
        dst
    }

    /// `nameof(x)`: the name `x` as a string.
    fn compile_symbol_quote(&mut self, arena: &AstArena, name: &str, args: &[NodeId]) -> u8 {
        let [arg] = args else {
            return self.error(
                crate::codes::ARG_COUNT,
                format!("`{name}` expects one argument"),
            );
        };
        let arg = *arg;
        let an = arena.node(arg);
        match an.node_kind() {
            NodeKind::IdentExpr | NodeKind::TypeNamed | NodeKind::StrLit => {
                self.emit_const(PilotVal::from_str(Self::name(arena, an.lhs)))
            }
            _ => self.error(crate::codes::SYNTAX, format!("`{name}` expects a name")),
        }
    }

    /// `sizeof(T)` and other compile-time questions about a type.
    fn compile_type_query(
        &mut self,
        arena: &AstArena,
        name: &str,
        query: &str,
        args: &[NodeId],
    ) -> u8 {
        let ty = args
            .first()
            .map(|&a| arena.node(a))
            .filter(|n| matches!(n.node_kind(), NodeKind::IdentExpr | NodeKind::TypeNamed));
        let Some(ty) = ty.map(|n| Self::name(arena, n.lhs).to_string()) else {
            return self.error(
                crate::codes::SYNTAX,
                format!("`{name}` expects a type name"),
            );
        };
        if query != QUERY_BYTE_SIZE {
            return self.error(
                crate::codes::SEED,
                format!("`{name}` asks `{query}`, which the seed cannot answer"),
            );
        }
        match self.type_extent(&ty, 0) {
            Some((size, _align)) => self.emit_const(PilotVal::from_int(size)),
            None => self.error(
                crate::codes::SEED,
                format!("`{name}({ty})`: layout is not known for `{ty}`"),
            ),
        }
    }

    fn type_extent(&self, ty: &str, depth: usize) -> Option<(i64, i64)> {
        if let Some(layout) = pilot_parser::bootstrap_tables().reg_layout(ty) {
            let size = ((layout.width + 7) / 8) as i64;
            let mut align = 1_i64;
            while align < size {
                align *= 2;
            }
            return Some((size, align));
        }
        if depth > 8 {
            return None;
        }
        let info = self.structs.get(ty)?;
        let mut offset = 0_i64;
        let mut max_align = 1_i64;
        for (fname, _) in &info.fields {
            let ftype = info.field_types.get(fname)?;
            let (fsize, falign) = self.type_extent(ftype, depth + 1)?;
            let align = falign.max(1);
            offset = ((offset + align - 1) / align) * align;
            offset += fsize;
            max_align = max_align.max(align);
        }
        let total_size = ((offset + max_align - 1) / max_align) * max_align;
        Some((total_size, max_align))
    }

    /// `name` on its own, when it is a native value.
    pub(crate) fn compile_native_value(&mut self, name: &str) -> Option<u8> {
        let hook = native(NativeKind::Value, name)?;
        Some(self.emit_intrinsic(hook, &[]))
    }

    /// Reads `field` of the value in `r_obj`: the record's field of that name
    /// if it has one, else the native property, else a plain field read.
    pub(crate) fn emit_field_read(&mut self, r_obj: u8, field: &str) -> u8 {
        let r_key = match field.parse::<i64>() {
            Ok(i) => self.emit_const(PilotVal::from_int(i)),
            Err(_) => self.emit_const(PilotVal::from_str(field)),
        };
        match native(NativeKind::Property, field) {
            Some(hook) => {
                let r_hook = self.emit_const(PilotVal::from_str(hook));
                self.emit_intrinsic("field_or_native", &[r_obj, r_key, r_hook])
            }
            None => self.emit_keyed_read(r_obj, r_key),
        }
    }
}
