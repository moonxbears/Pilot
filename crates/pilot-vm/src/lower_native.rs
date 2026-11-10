//! Calls, methods, properties and values the seed answers with engine
//! intrinsics.
//!
//! Which names these are, and which intrinsic each runs, is declared in
//! Pilot (`stdlib/core/builtins/native_calls.pilot`) and read from the parser
//! tables; no such name is written here. `__engine(..)`, a seed token, calls
//! an intrinsic by name, and the receptor `nameof` quotes its argument.

use crate::instruction::Opcode;
use crate::BytecodeCompiler;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_parser::NativeKind;
use pilot_rt::PilotVal;

/// The question a `TypeQuery` binding asks: the type's size in bytes.
const QUERY_BYTE_SIZE: &str = "byte_size";

/// The engine intrinsic (or question) bound to `name` in table `kind`.
pub(crate) fn native(kind: NativeKind, name: &str) -> Option<&'static str> {
    pilot_parser::bootstrap_tables().native(kind, name)
}

impl BytecodeCompiler {
    /// `name(args)` when `name` is native, quoting or the seed token
    /// `__engine`; `None` otherwise.
    pub(crate) fn compile_native_call(&mut self, arena: &AstArena, name: &str, args: &[NodeId]) -> Option<u8> {
        if name == crate::ENGINE_TOKEN {
            return Some(self.compile_engine_call(arena, args));
        }
        if pilot_parser::bootstrap_tables().quotes_symbol(name) {
            return Some(self.compile_symbol_quote(arena, name, args));
        }
        if let Some(query) = native(NativeKind::TypeQuery, name) {
            return Some(self.compile_type_query(arena, name, query, args));
        }
        let hook = native(NativeKind::Function, name)?;
        // `f((a, b))` and `f(a, b)` pass the same arguments.
        let spread: Vec<NodeId> = match args {
            [only] if arena.node(*only).node_kind() == NodeKind::TupleExpr => {
                let tn = arena.node(*only);
                arena.children_slice(tn.lhs, tn.rhs).iter().map(|&c| NodeId(c)).collect()
            }
            _ => args.to_vec(),
        };
        let regs = self.compile_args(arena, &spread);
        Some(self.emit_intrinsic(hook, &regs))
    }

    /// `__engine("intrinsic", args..)`.
    fn compile_engine_call(&mut self, arena: &AstArena, args: &[NodeId]) -> u8 {
        let Some((&hook, rest)) = args.split_first() else {
            return self.error(
                crate::codes::SYNTAX,
                format!("{} expects an intrinsic name", crate::ENGINE_TOKEN),
            );
        };
        let r_hook = self.compile_node(arena, hook);
        let regs = self.compile_args(arena, rest);
        self.stage_args(&regs);
        let dst = self.alloc_reg();
        self.emit(Opcode::EngineIntrinsic, dst, r_hook, regs.len() as u8);
        dst
    }

    /// `nameof(x)`: the name `x` as a string.
    fn compile_symbol_quote(&mut self, arena: &AstArena, name: &str, args: &[NodeId]) -> u8 {
        let Some(&arg) = args.first() else {
            return self.error(crate::codes::ARG_COUNT, format!("`{name}` expects one argument"));
        };
        let an = arena.node(arg);
        match an.node_kind() {
            NodeKind::IdentExpr | NodeKind::TypeNamed | NodeKind::StrLit => {
                self.emit_const(PilotVal::from_str(Self::name(arena, an.lhs)))
            }
            _ => self.error(crate::codes::SYNTAX, format!("`{name}` expects a name")),
        }
    }

    /// `sizeof(T)` and other compile-time questions about a type.
    fn compile_type_query(&mut self, arena: &AstArena, name: &str, query: &str, args: &[NodeId]) -> u8 {
        let ty = args.first().map(|&a| arena.node(a)).filter(|n| {
            matches!(n.node_kind(), NodeKind::IdentExpr | NodeKind::TypeNamed)
        });
        let Some(ty) = ty.map(|n| Self::name(arena, n.lhs).to_string()) else {
            return self.error(crate::codes::SYNTAX, format!("`{name}` expects a type name"));
        };
        if query != QUERY_BYTE_SIZE {
            return self.error(
                crate::codes::SEED,
                format!("`{name}` asks `{query}`, which the seed cannot answer"),
            );
        }
        match pilot_parser::bootstrap_tables().reg_layout(&ty) {
            Some(layout) => self.emit_const(PilotVal::from_int((layout.width + 7) / 8)),
            None => self.error(
                crate::codes::SEED,
                format!(
                    "`{name}({ty})`: the seed knows only the register layouts of primitive types; struct layouts come with the type checker (F4)"
                ),
            ),
        }
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
            None => self.emit_intrinsic("map_get", &[r_obj, r_key]),
        }
    }
}
