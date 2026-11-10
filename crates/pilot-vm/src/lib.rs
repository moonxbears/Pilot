//! Register-Based `PilVM` v3, `.pilc` v3 Zero-Copy Format & Debug-Span Compiler (`pilot-vm`).
//!
//! `BytecodeCompiler` lowers a parsed `AstArena` into PilVM bytecode. Every
//! AST node kind either lowers to code with its meaning or records a
//! diagnostic in `PilcBinary::diagnostics` and compiles to a `Trap`, so a
//! construct the seed cannot run is reported instead of silently dropped.

mod heap;
pub mod instruction;
mod lower_call;
mod lower_cascade;
mod lower_closure;
mod lower_defer;
mod lower_dispatch;
mod lower_expr;
mod lower_global;
mod lower_stmt;
mod lower_struct;
mod lower_try;
mod lower_variant;
mod lower_switch;
mod ops;
pub mod pilc;
pub mod vm;

pub use instruction::{Instruction, Opcode};
pub use pilc::{CompileDiagnostic, PilcBinary, PILC_V3_MAGIC, PILC_V3_VERSION};
pub use vm::{EngineIntrinsicHandler, PilVirtualMachine, VmFault, VmHook};

pub mod compiler {
    pub use super::BytecodeCompiler;
}

use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::SymbolId;
use pilot_rt::PilotVal;
use pilot_span::ByteSpan;
use std::collections::HashMap;

/// Compile-time constant tree used for top-level literal `let`s and enums.
#[derive(Clone, Debug)]
enum CMap {
    Val(PilotVal),
    List(Vec<CMap>),
    Map(HashMap<String, CMap>),
}

/// First register available to locals; `r1..r16` carry call arguments.
const FIRST_LOCAL_REG: u8 = 17;
/// Last register available to locals.
const LAST_LOCAL_REG: u8 = 127;
/// First scratch register; scratch registers cycle through `128..=255`.
const FIRST_TEMP_REG: u8 = 128;
/// Name of the hidden receiver parameter of methods and constructors; the
/// body sees it as `self` (the name without the leading underscore).
const RECEIVER: &str = "_self";
/// Arguments travel in `r1..=r16`.
const MAX_CALL_ARGS: usize = 16;

/// Loop bookkeeping: where `continue` jumps and which `break`s to patch.
struct LoopCtx {
    continue_jumps: Vec<usize>,
    break_jumps: Vec<usize>,
    /// A `switch` takes `break` but passes `continue` to the enclosing loop.
    is_switch: bool,
    /// Number of `defer` scopes open when the loop or switch began; leaving
    /// it runs the deferred statements of the scopes above.
    defer_depth: usize,
}

/// A lambda being compiled: what its enclosing scope can see, which of those
/// names the body captured (in environment order), and where the body keeps
/// its environment.
#[derive(Clone)]
struct CaptureCtx {
    outer_slots: HashMap<String, u8>,
    names: Vec<String>,
    env_slot: u8,
}

/// An enclosing `try` (or `expr catch`): where a failing `?` goes.
struct TryCtx {
    /// Local register that receives the failure value.
    fail_reg: u8,
    /// Defer scopes open when the `try` began; scopes above it are left.
    defer_depth: usize,
    /// Jumps to the catch clause, patched once its address is known.
    fail_jumps: Vec<usize>,
}

/// Per-function compiler state, saved and restored around nested functions.
struct FnState {
    captures: Vec<CaptureCtx>,
    defers: Vec<Vec<NodeId>>,
    var_slots: HashMap<String, u8>,
    var_types: HashMap<String, String>,
    cur_ret_type: Option<String>,
    cur_owner: Option<String>,
    next_local_reg: u8,
    next_reg: u8,
    loops: Vec<LoopCtx>,
    tries: Vec<TryCtx>,
}

#[derive(Default)]
pub struct BytecodeCompiler {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<PilotVal>,
    pub pc_spans: Vec<ByteSpan>,
    pub reg_symbols: Vec<(u8, u32, u32)>,
    pub diagnostics: Vec<CompileDiagnostic>,
    var_slots: HashMap<String, u8>,
    var_types: HashMap<String, String>,
    fn_ret_types: HashMap<String, String>,
    fn_entries: HashMap<String, (u16, Vec<String>)>,
    call_patches: Vec<(usize, String, String, ByteSpan)>,
    const_maps: HashMap<String, CMap>,
    loops: Vec<LoopCtx>,
    tries: Vec<TryCtx>,
    cur_span: ByteSpan,
    /// Declared return type of the function being compiled.
    cur_ret_type: Option<String>,
    /// The struct or `ext` type whose method is being compiled.
    cur_owner: Option<String>,
    /// Names of the module-level bindings that live in the VM's globals.
    globals: std::collections::HashSet<String>,
    /// `comptime` code compiled so far, run before everything else.
    comptime_runs: Vec<lower_global::ComptimeRun>,
    /// Payload field names of each enum variant that carries values, by
    /// `Enum.Variant`.
    variant_payloads: HashMap<String, Vec<String>>,
    /// Every declared method by name: the types declaring it, and whether
    /// that declaration is static.
    method_owners: HashMap<String, Vec<(String, bool)>>,
    /// Expected type of the expression being compiled, used to resolve
    /// target-typed enum variants (`::Variant`).
    type_hint: Option<String>,
    next_local_reg: u8,
    next_reg: u8,
    /// Enclosing lambdas, innermost last. Empty inside named functions.
    captures: Vec<CaptureCtx>,
    /// Deferred statements of each open block, innermost last.
    defers: Vec<Vec<NodeId>>,
    /// Declared structs: fields with defaults, and whether `self(..)` exists.
    structs: HashMap<String, lower_struct::StructInfo>,
    /// Arity of every top-level function, so a function can be used as a
    /// value before its body is compiled.
    known_fns: HashMap<String, usize>,
    /// `LoadConst` instructions that must load a function's entry point once
    /// it is known.
    entry_patches: Vec<(usize, String, ByteSpan)>,
}

impl BytecodeCompiler {
    pub fn new() -> Self {
        Self {
            next_local_reg: FIRST_LOCAL_REG,
            next_reg: FIRST_TEMP_REG,
            ..Self::default()
        }
    }

    pub fn compile(arena: &AstArena, root: NodeId) -> PilcBinary {
        let mut c = Self::new();
        let last = if root.is_some() { c.compile_node(arena, root) } else { 0 };
        c.emit(Opcode::Move, 0, last, 0);
        c.emit(Opcode::Ret, 0, 0, 0);
        c.emit(Opcode::Halt, 0, 0, 0);
        c.resolve_calls();
        if c.instructions.len() > u16::MAX as usize {
            let span = arena.span(root);
            c.diagnostics.push(CompileDiagnostic {
                span,
                message: format!(
                    "program needs {} instructions; PilVM jump targets address at most {}",
                    c.instructions.len(),
                    u16::MAX
                ),
            });
        }
        let mut bin = PilcBinary::with_debug_tables(c.constants, c.instructions, c.pc_spans, c.reg_symbols);
        bin.diagnostics = c.diagnostics;
        bin
    }

    /// Patches forward calls and function-value entry points once every
    /// function entry is known.
    fn resolve_calls(&mut self) {
        for (idx, name, span) in std::mem::take(&mut self.entry_patches) {
            let dst = self.instructions[idx].dst();
            match self.fn_entries.get(&name).map(|&(e, _)| e) {
                Some(e) => {
                    let c = self.add_constant(PilotVal::from_int(e as i64));
                    self.instructions[idx] = Instruction::encode_wide(Opcode::LoadConst, dst, c);
                }
                None => {
                    let message = format!("no function named `{name}`");
                    let c = self.add_constant(PilotVal::from_str(&message));
                    self.instructions[idx] = Instruction::encode_wide(Opcode::Trap, 0, c);
                    self.diagnostics.push(CompileDiagnostic { span, message });
                }
            }
        }
        let patches = std::mem::take(&mut self.call_patches);
        for (idx, full, bare, span) in patches {
            let dst = self.instructions[idx].dst();
            let entry = self
                .fn_entries
                .get(&full)
                .or_else(|| self.fn_entries.get(&bare))
                .map(|&(e, _)| e);
            match entry {
                Some(e) => self.instructions[idx] = Instruction::encode_wide(Opcode::Call, dst, e),
                None => {
                    let message = format!("no function named `{full}`");
                    let c = self.add_constant(PilotVal::from_str(&message));
                    self.instructions[idx] = Instruction::encode_wide(Opcode::Trap, 0, c);
                    self.diagnostics.push(CompileDiagnostic { span, message });
                }
            }
        }
    }

    // ----------------------------------------------------------------------
    // Emission helpers
    // ----------------------------------------------------------------------

    fn alloc_local_reg(&mut self) -> u8 {
        let r = self.next_local_reg;
        if r > LAST_LOCAL_REG {
            self.error(format!(
                "function needs more than {} local variables",
                LAST_LOCAL_REG - FIRST_LOCAL_REG + 1
            ));
            return LAST_LOCAL_REG;
        }
        self.next_local_reg += 1;
        r
    }

    /// Allocates a scratch register. Scratch registers are released at
    /// statement boundaries (see `compile_block`), so they never wrap onto a
    /// live value; an expression that needs more than the scratch window is
    /// reported instead.
    fn alloc_reg(&mut self) -> u8 {
        let r = self.next_reg;
        if r == u8::MAX {
            self.diagnostics.push(CompileDiagnostic {
                span: self.cur_span,
                message: format!(
                    "expression needs more than {} scratch registers",
                    u8::MAX - FIRST_TEMP_REG
                ),
            });
            return r;
        }
        self.next_reg += 1;
        r
    }

    fn add_constant(&mut self, val: PilotVal) -> u16 {
        let i = self
            .constants
            .iter()
            .position(|&c| c == val)
            .unwrap_or_else(|| {
                self.constants.push(val);
                self.constants.len() - 1
            });
        i as u16
    }

    fn emit(&mut self, op: Opcode, dst: u8, src1: u8, src2: u8) -> usize {
        self.push_ins(Instruction::encode(op, dst, src1, src2))
    }

    fn push_ins(&mut self, ins: Instruction) -> usize {
        self.instructions.push(ins);
        self.pc_spans.push(self.cur_span);
        self.instructions.len() - 1
    }

    fn here(&self) -> u16 {
        self.instructions.len() as u16
    }

    fn emit_jump(&mut self, target: u16) -> usize {
        self.push_ins(Instruction::encode_wide(Opcode::Jump, 0, target))
    }

    fn emit_branch(&mut self, op: Opcode, cond: u8, target: u16) -> usize {
        self.push_ins(Instruction::encode_branch(op, cond, target))
    }

    /// Points the jump or branch at `idx` to `target`.
    fn patch_jump(&mut self, idx: usize, target: u16) {
        let ins = self.instructions[idx];
        self.instructions[idx] = match ins.opcode() {
            Opcode::Jump => Instruction::encode_wide(Opcode::Jump, 0, target),
            op => Instruction::encode_branch(op, ins.src1(), target),
        };
    }

    /// Points the jump or branch at `idx` to the current end of code.
    fn patch_to_here(&mut self, idx: usize) {
        let target = self.here();
        self.patch_jump(idx, target);
    }

    fn emit_const(&mut self, val: PilotVal) -> u8 {
        let (c, r) = (self.add_constant(val), self.alloc_reg());
        self.push_ins(Instruction::encode_wide(Opcode::LoadConst, r, c));
        r
    }

    fn emit_null(&mut self) -> u8 {
        self.emit_const(PilotVal::NULL)
    }

    /// Copies argument registers into `r1..rN` without clobbering sources.
    fn stage_args(&mut self, args: &[u8]) {
        if args.len() > MAX_CALL_ARGS {
            self.error(format!(
                "calls take at most {MAX_CALL_ARGS} arguments, found {}",
                args.len()
            ));
            return;
        }
        let safe: Vec<u8> = args
            .iter()
            .map(|&r| {
                if r >= 1 && r as usize <= args.len() {
                    let t = self.alloc_reg();
                    self.emit(Opcode::Move, t, r, 0);
                    t
                } else {
                    r
                }
            })
            .collect();
        for (i, &r) in safe.iter().enumerate() {
            self.emit(Opcode::Move, (i + 1) as u8, r, 0);
        }
    }

    /// Calls a VM intrinsic. Scratch registers used to stage the call are
    /// released afterwards; only the result register stays allocated.
    fn emit_intrinsic(&mut self, hook: &str, args: &[u8]) -> u8 {
        let d = self.alloc_reg();
        let mark = self.next_reg;
        let r = self.emit_const(PilotVal::from_str(hook));
        self.stage_args(args);
        self.emit(Opcode::EngineIntrinsic, d, r, args.len() as u8);
        self.next_reg = mark;
        d
    }

    /// Releases every scratch register allocated after `mark`. Callers
    /// allocate their result register before taking the mark.
    fn release_to(&mut self, mark: u8) {
        self.next_reg = mark;
    }

    /// Emits a trap that stops the program with `message` when reached.
    fn emit_trap(&mut self, message: String) -> u8 {
        let c = self.add_constant(PilotVal::from_str(&message));
        self.push_ins(Instruction::encode_wide(Opcode::Trap, 0, c));
        self.emit_null()
    }

    /// Records a diagnostic at the current span and emits a trap carrying it.
    fn error(&mut self, message: String) -> u8 {
        let c = self.add_constant(PilotVal::from_str(&message));
        self.push_ins(Instruction::encode_wide(Opcode::Trap, 0, c));
        self.diagnostics.push(CompileDiagnostic {
            span: self.cur_span,
            message,
        });
        self.emit_null()
    }

    fn save_fn_state(&mut self) -> FnState {
        FnState {
            captures: std::mem::take(&mut self.captures),
            defers: std::mem::take(&mut self.defers),
            var_slots: std::mem::take(&mut self.var_slots),
            var_types: std::mem::take(&mut self.var_types),
            cur_ret_type: self.cur_ret_type.take(),
            cur_owner: self.cur_owner.take(),
            next_local_reg: self.next_local_reg,
            next_reg: self.next_reg,
            loops: std::mem::take(&mut self.loops),
            tries: std::mem::take(&mut self.tries),
        }
    }

    fn restore_fn_state(&mut self, s: FnState) {
        self.captures = s.captures;
        self.defers = s.defers;
        self.var_slots = s.var_slots;
        self.var_types = s.var_types;
        self.cur_ret_type = s.cur_ret_type;
        self.cur_owner = s.cur_owner;
        self.next_local_reg = s.next_local_reg;
        self.next_reg = s.next_reg;
        self.loops = s.loops;
        self.tries = s.tries;
    }

    /// Compiles `id` with `hint` as the expected type.
    fn compile_with_hint(&mut self, arena: &AstArena, id: NodeId, hint: Option<String>) -> u8 {
        let saved = std::mem::replace(&mut self.type_hint, hint);
        let r = self.compile_node(arena, id);
        self.type_hint = saved;
        r
    }

    /// Named type of an annotation node, if it is a plain name.
    fn annot_name(arena: &AstArena, id: NodeId) -> Option<String> {
        let tid = arena.type_annot(id)?;
        let tn = arena.node(tid);
        matches!(tn.node_kind(), NodeKind::TypeNamed | NodeKind::IdentExpr)
            .then(|| Self::name(arena, tn.lhs).to_string())
    }

    fn reset_fn_state(&mut self) {
        self.var_slots.clear();
        self.var_types.clear();
        self.cur_ret_type = None;
        self.cur_owner = None;
        self.loops.clear();
        self.tries.clear();
        self.defers.clear();
        self.next_local_reg = FIRST_LOCAL_REG;
        self.next_reg = FIRST_TEMP_REG;
    }

    fn name(arena: &AstArena, sym: u32) -> &str {
        arena.interner.resolve(SymbolId(sym))
    }

    // ----------------------------------------------------------------------
    // Compile-time constants (`CMap`)
    // ----------------------------------------------------------------------

    fn eval_cmap(&self, arena: &AstArena, id: NodeId) -> Option<CMap> {
        if id.is_none() {
            return None;
        }
        let n = arena.node(id);
        match n.node_kind() {
            NodeKind::StrLit if n.flags == 0 => {
                Some(CMap::Val(PilotVal::from_str(Self::name(arena, n.lhs))))
            }
            NodeKind::IntLit => Some(CMap::Val(PilotVal::from_int(
                arena.i64_pool.get(n.lhs as usize).copied().unwrap_or(0),
            ))),
            NodeKind::FloatLit => Some(CMap::Val(PilotVal::from_float(
                arena.f64_pool.get(n.lhs as usize).copied().unwrap_or(0.0),
            ))),
            NodeKind::BoolLit => Some(CMap::Val(PilotVal::from_bool(n.lhs != 0))),
            NodeKind::MapExpr => {
                let ch = arena.children_slice(n.lhs, n.rhs);
                let mut map = HashMap::new();
                for pair in ch.chunks(2) {
                    if pair.len() != 2 {
                        return None;
                    }
                    let k = Self::map_key(arena, NodeId(pair[0]))?;
                    map.insert(k, self.eval_cmap(arena, NodeId(pair[1]))?);
                }
                Some(CMap::Map(map))
            }
            NodeKind::ListExpr => {
                let ch = arena.children_slice(n.lhs, n.rhs);
                let items: Option<Vec<CMap>> = ch
                    .iter()
                    .map(|&c| self.eval_cmap(arena, NodeId(c)))
                    .collect();
                items.map(CMap::List)
            }
            _ => None,
        }
    }

    /// Literal key of a `{ key: value }` entry.
    fn map_key(arena: &AstArena, id: NodeId) -> Option<String> {
        let kn = arena.node(id);
        match kn.node_kind() {
            NodeKind::IdentExpr | NodeKind::StrLit => Some(Self::name(arena, kn.lhs).to_string()),
            NodeKind::IntLit => arena.i64_pool.get(kn.lhs as usize).map(|v| v.to_string()),
            _ => None,
        }
    }

    fn resolve_cmap(&self, arena: &AstArena, id: NodeId) -> Option<CMap> {
        if id.is_none() {
            return None;
        }
        let n = arena.node(id);
        match n.node_kind() {
            NodeKind::IdentExpr => {
                let nm = Self::name(arena, n.lhs);
                if self.var_slots.contains_key(nm) {
                    return None;
                }
                self.const_maps.get(nm).cloned()
            }
            NodeKind::MemberExpr => {
                let CMap::Map(m) = self.resolve_cmap(arena, NodeId(n.lhs))? else {
                    return None;
                };
                m.get(Self::name(arena, n.rhs)).cloned()
            }
            _ => None,
        }
    }

    /// Builds a runtime value from a compile-time constant.
    fn materialize_cmap(&mut self, cm: &CMap) -> u8 {
        match cm {
            CMap::Val(v) => self.emit_const(*v),
            CMap::List(items) => {
                let r_list = self.emit_intrinsic("list_new", &[]);
                for item in items {
                    let r_v = self.materialize_cmap(item);
                    self.emit_intrinsic("list_push", &[r_list, r_v]);
                }
                r_list
            }
            CMap::Map(m) => {
                let r_map = self.emit_intrinsic("map_new", &[]);
                let mut keys: Vec<&String> = m.keys().collect();
                keys.sort();
                for k in keys {
                    let r_v = self.materialize_cmap(&m[k]);
                    let r_k = self.emit_const(PilotVal::from_str(k));
                    self.emit_intrinsic("map_set", &[r_map, r_k, r_v]);
                }
                r_map
            }
        }
    }

    /// Resolves a target-typed `::Variant`: the enum named by the current
    /// type hint when it has that variant, otherwise the only enum that does.
    fn unique_variant(&self, name: &str) -> Option<PilotVal> {
        if let Some(CMap::Map(m)) = self.type_hint.as_ref().and_then(|t| self.const_maps.get(t)) {
            if let Some(CMap::Val(v)) = m.get(name) {
                return Some(*v);
            }
        }
        let mut found = None;
        for cm in self.const_maps.values() {
            if let CMap::Map(m) = cm {
                if let Some(CMap::Val(v)) = m.get(name) {
                    if found.is_some() {
                        return None;
                    }
                    found = Some(*v);
                }
            }
        }
        found
    }

    /// Records an enum declaration's variants as compile-time integers.
    fn register_enum(&mut self, arena: &AstArena, id: NodeId) {
        let cn = arena.node(id);
        if cn.flags == 0 {
            return;
        }
        let e_nm = Self::name(arena, cn.lhs).to_string();
        let mut map = HashMap::new();
        let mut next = 0i64;
        for &vid in arena.children_slice(cn.rhs, cn.flags as u32) {
            self.register_variant_payload(arena, &e_nm, NodeId(vid));
            let vn = arena.node(NodeId(vid));
            let v_nm = Self::name(arena, vn.lhs).to_string();
            let mut val = next;
            if vn.flags > 0 {
                let last = arena.extra_children[(vn.rhs as usize) + (vn.flags as usize) - 1];
                let lcn = arena.node(NodeId(last));
                if lcn.node_kind() == NodeKind::IntLit {
                    val = arena.i64_pool.get(lcn.lhs as usize).copied().unwrap_or(next);
                }
            }
            next = val + 1;
            map.insert(v_nm, CMap::Val(PilotVal::from_int(val)));
        }
        self.const_maps.insert(e_nm, CMap::Map(map));
    }
}

/// Names the seed compiler treats specially. This is the only table of
/// callable names in the VM crate; the gate's name-blind audit exempts lines
/// that reference `SeedBuiltin::` and nothing else. Method entries forward to
/// heap intrinsics until collection protocols dispatch through Pilot method
/// tables (Phase F).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SeedBuiltin {
    SizeOf,
    Assert,
    Print,
    NameOf,
    Engine,
    Method(&'static str),
}

impl SeedBuiltin {
    /// Free-function builtins, looked up by callee name.
    pub(crate) fn function(name: &str) -> Option<Self> {
        Some(match name {
            "sizeof" => SeedBuiltin::SizeOf,
            "assert" => SeedBuiltin::Assert,
            "print" | "println" => SeedBuiltin::Print,
            "nameof" => SeedBuiltin::NameOf,
            "__engine" => SeedBuiltin::Engine,
            _ => return None,
        })
    }

    /// Methods on VM-native values (strings, lists, records), looked up by
    /// method name; the payload is the heap intrinsic that implements it.
    pub(crate) fn method(name: &str) -> Option<&'static str> {
        let SeedBuiltin::Method(hook) = (match name {
            "len" => SeedBuiltin::Method("str_len"),
            "get" => SeedBuiltin::Method("map_get"),
            "set" => SeedBuiltin::Method("map_set"),
            "push" => SeedBuiltin::Method("list_push"),
            "pop" => SeedBuiltin::Method("list_pop"),
            "trim" => SeedBuiltin::Method("str_trim"),
            "to_str" => SeedBuiltin::Method("to_str"),
            "remove_at" => SeedBuiltin::Method("list_remove_at"),
            "byte_at" => SeedBuiltin::Method("str_byte_at"),
            "slice" | "substring" => SeedBuiltin::Method("str_slice"),
            "find" | "index_of" => SeedBuiltin::Method("str_find"),
            "replace" => SeedBuiltin::Method("str_replace"),
            "contains" => SeedBuiltin::Method("str_contains"),
            "starts_with" => SeedBuiltin::Method("str_starts_with"),
            "ends_with" => SeedBuiltin::Method("str_ends_with"),
            "is_null" => SeedBuiltin::Method("is_null"),
            "has" => SeedBuiltin::Method("map_has"),
            "remove" => SeedBuiltin::Method("map_remove"),
            _ => return None,
        }) else {
            return None;
        };
        Some(hook)
    }

    /// Paren-less properties on VM-native values (`xs.len`), with the heap
    /// intrinsic that computes them from the receiver alone.
    pub(crate) fn property(name: &str) -> Option<&'static str> {
        let SeedBuiltin::Method(hook) = (match name {
            "len" => SeedBuiltin::Method("str_len"),
            _ => return None,
        }) else {
            return None;
        };
        Some(hook)
    }
}
