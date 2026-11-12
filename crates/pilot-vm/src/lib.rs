//! Register-Based `PilVM` v3, `.pilc` v3 Zero-Copy Format & Debug-Span Compiler (`pilot-vm`).
//!
//! `BytecodeCompiler` lowers a parsed `AstArena` into PilVM bytecode. Every
//! AST node kind either lowers to code with its meaning or records a
//! diagnostic in `PilcBinary::diagnostics` and compiles to a `Trap`, so a
//! construct the seed cannot run is reported instead of silently dropped.

mod checked;
pub mod collect;
pub mod codes;
mod heap;
pub mod instruction;
pub mod host;
mod lower_args;
mod lower_call;
mod lower_cascade;
mod lower_closure;
mod lower_defer;
mod lower_destructure;
pub mod lower_drop;
mod lower_dispatch;
mod lower_expr;
mod lower_for;
mod lower_global;
mod lower_members;
mod lower_native;
mod lower_pipe;
pub mod lower_probe;
mod lower_program;
mod lower_stmt;
mod lower_struct;
mod lower_switch;
mod lower_try;
mod lower_unit;
mod lower_variant;
mod lower_yield;
mod ops;
pub mod pilc;
pub mod strings;
pub mod vm;
mod vm_gen;

pub use instruction::{Instruction, Opcode};
pub use pilc::{CompileDiagnostic, PilcBinary, PILC_V3_MAGIC, PILC_V3_VERSION};
pub use host::HostValue;
pub use vm::{EngineIntrinsicHandler, HostOpHandler, PilVirtualMachine, VmFault, VmHook};

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
/// The seed token that calls an engine intrinsic by name.
const ENGINE_TOKEN: &str = "__engine";
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
    /// The local holding the enumerator of a `for` that walks one; leaving
    /// the loop early closes it (`lower_yield`).
    release: Option<u8>,
}

/// Who a `yield` hands its value to (`lower_yield`).
#[derive(Clone, Copy)]
enum YieldOwner {
    /// A `for` in value position, collecting into the list in this local.
    /// Its `bool` records that some `yield` did.
    Collect(u8, bool),
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
    drop_flags: HashMap<String, u8>,
    defers: Vec<Vec<lower_drop::Deferred>>,
    var_slots: HashMap<String, u8>,
    var_types: HashMap<String, String>,
    cur_ret_type: Option<String>,
    cur_owner: Option<String>,
    backing: Option<String>,
    next_local_reg: u8,
    next_reg: u8,
    loops: Vec<LoopCtx>,
    tries: Vec<TryCtx>,
    block_scopes: Vec<BlockScope>,
    yield_owners: Vec<YieldOwner>,
    gen_body: Option<ByteSpan>,
}

/// The bindings visible where a block starts. A `let` in the block that
/// reuses one of these names gets its own slot, and the block's end brings
/// these bindings back.
struct BlockScope {
    slots: HashMap<String, u8>,
    types: HashMap<String, String>,
}

#[derive(Default)]
pub struct BytecodeCompiler {
    pub instructions: Vec<Instruction>,
    /// Where owned values are dropped (F7), when the CLI gave a plan.
    drop_plan: Option<std::rc::Rc<lower_drop::DropPlan>>,
    /// The `exit` drops of the `return` / `break` / `continue` being lowered.
    pending_exit_drops: Vec<lower_drop::DropItem>,
    /// The `replace` drops of the assignment being lowered: the value its
    /// target held, dropped once the new value is made.
    pending_replace: Vec<lower_drop::DropItem>,
    /// Binding -> the register holding its drop flag, for a binding moved
    /// on only some paths: its drops run only while the flag is set.
    drop_flags: HashMap<String, u8>,
    /// Calls made around statements (`lower_probe`), when the CLI gave a plan.
    probe_plan: Option<std::rc::Rc<lower_probe::ProbePlan>>,
    pub constants: Vec<PilotVal>,
    pub pc_spans: Vec<ByteSpan>,
    pub reg_symbols: Vec<(u8, u32, u32)>,
    pub diagnostics: Vec<CompileDiagnostic>,
    var_slots: HashMap<String, u8>,
    var_types: HashMap<String, String>,
    block_scopes: Vec<BlockScope>,
    fn_ret_types: HashMap<String, String>,
    fn_entries: HashMap<String, (u32, Vec<String>)>,
    /// The declaration whose parameters each callee takes (`f`, `Owner.m`,
    /// `Owner.self`), for binding named arguments.
    param_decls: HashMap<String, NodeId>,
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
    defers: Vec<Vec<lower_drop::Deferred>>,
    /// The collecting `for`s around the code being compiled, innermost last;
    /// a `yield` outside all of them makes the function a generator.
    yield_owners: Vec<YieldOwner>,
    /// Where the function being compiled first yields, when it does: it is
    /// a generator.
    gen_body: Option<ByteSpan>,
    /// The `for` about to be compiled as a block item (statement position),
    /// which collects nothing.
    stmt_for: Option<NodeId>,
    /// Declared structs: fields with defaults, and whether `self(..)` exists.
    structs: HashMap<String, lower_struct::StructInfo>,
    /// Arity of every top-level function, so a function can be used as a
    /// value before its body is compiled.
    known_fns: HashMap<String, usize>,
    /// `LoadConst` instructions that must load a function's entry point once
    /// it is known.
    entry_patches: Vec<(usize, String, ByteSpan)>,
    /// Functions and methods of imported units, compiled on first use.
    lazy_decls: HashMap<String, lower_program::LazyDecl>,
    /// Names a unit hides with `without`, and that unit.
    excluded: HashMap<String, Option<pilot_span::FileId>>,
    /// The unit of the node being compiled, in a linked program.
    cur_unit: Option<pilot_span::FileId>,
    /// The unit whose top-level function a bare name reaches, by name.
    fn_homes: HashMap<String, Option<pilot_span::FileId>>,
    /// Struct and interface declarations of every unit, by name.
    type_decls: HashMap<String, Vec<NodeId>>,
    /// Accessor block of each property, by its field node.
    accessor_blocks: HashMap<u32, NodeId>,
    /// Parameter and guard of each setter body, by the body node.
    setter_rows: HashMap<u32, (NodeId, NodeId)>,
    /// The structs whose records the compiled code builds.
    constructed: std::collections::HashSet<String>,
    /// Member dispatchers the code calls, by function name, with the owners
    /// each was last built for (`None` before it is built).
    dispatchers: std::collections::BTreeMap<String, (lower_members::Switch, Option<Vec<String>>)>,
    /// The structs with a getter for each property name.
    getters: HashMap<String, Vec<String>>,
    /// The structs with a setter for each property name.
    setters: HashMap<String, Vec<String>>,
    /// The property whose accessor is being compiled: the backing name
    /// reads and writes its storage on the record.
    backing: Option<String>,
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
        c.finish(arena, root, last)
    }

    /// Compiles a program linked from the units of `arena` (see
    /// `pilot_parser::parse_unit`), in link order with the entry unit last.
    pub fn compile_linked(arena: &AstArena) -> PilcBinary {
        Self::compile_linked_with_drops(arena, lower_drop::DropPlan::default())
    }

    /// `compile_linked`, dropping owned values where `plan` says (F7).
    pub fn compile_linked_with_drops(arena: &AstArena, plan: lower_drop::DropPlan) -> PilcBinary {
        Self::compile_linked_with_plans(arena, plan, lower_probe::ProbePlan::default())
    }

    /// `compile_linked_with_drops`, also making the calls `probes` plans
    /// around statements.
    pub fn compile_linked_with_plans(
        arena: &AstArena,
        plan: lower_drop::DropPlan,
        probes: lower_probe::ProbePlan,
    ) -> PilcBinary {
        let mut c = Self::new();
        c.set_drop_plan(plan);
        c.set_probe_plan(probes);
        let units = Self::unit_items(arena);
        let last = c.compile_units(arena, &units);
        let root = arena.units.last().map_or(NodeId::NONE, |u| u.root);
        c.finish(arena, root, last)
    }

    fn finish(mut self, arena: &AstArena, root: NodeId, last: u8) -> PilcBinary {
        let c = &mut self;
        c.emit(Opcode::Move, 0, last, 0);
        c.emit(Opcode::Ret, 0, 0, 0);
        c.emit(Opcode::Halt, 0, 0, 0);
        loop {
            c.link_lazy(arena);
            if !c.emit_dispatchers() {
                break;
            }
        }
        c.resolve_calls();
        if c.instructions.len() > u32::MAX as usize {
            let span = arena.span(root);
            c.diagnostics.push(CompileDiagnostic {
                span,
                code: codes::SEED,
                message: format!(
                    "program needs {} instructions; PilVM jump targets address at most {}",
                    c.instructions.len(),
                    u32::MAX
                ),
            });
        }
        let mut bin = PilcBinary::with_debug_tables(
            std::mem::take(&mut c.constants),
            std::mem::take(&mut c.instructions),
            std::mem::take(&mut c.pc_spans),
            std::mem::take(&mut c.reg_symbols),
        );
        bin.diagnostics = std::mem::take(&mut c.diagnostics);
        bin
    }

    /// Compiles every imported function or method the program calls, and
    /// the ones those call, until no call is left to an uncompiled one.
    fn link_lazy(&mut self, arena: &AstArena) {
        loop {
            let wanted: Vec<String> = self
                .call_patches
                .iter()
                .flat_map(|(_, full, bare, _)| [full.clone(), bare.clone()])
                .chain(self.entry_patches.iter().map(|(_, name, _)| name.clone()))
                .collect();
            let mut progress = false;
            for name in wanted {
                progress |= self.compile_lazy(arena, &name);
            }
            if !progress {
                break;
            }
        }
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
                    self.diagnostics.push(CompileDiagnostic {
                        span,
                        code: codes::UNDECLARED,
                        message,
                    });
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
                    self.diagnostics.push(CompileDiagnostic {
                        span,
                        code: codes::UNDECLARED,
                        message,
                    });
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
            self.error(codes::SEED, format!(
                "function needs more than {} local variables in scope at once",
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
                code: codes::SEED,
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

    fn add_constant(&mut self, val: PilotVal) -> u32 {
        let i = self
            .constants
            .iter()
            .position(|&c| c == val)
            .unwrap_or_else(|| {
                self.constants.push(val);
                self.constants.len() - 1
            });
        i as u32
    }

    fn emit(&mut self, op: Opcode, dst: u8, src1: u8, src2: u8) -> usize {
        self.push_ins(Instruction::encode(op, dst, src1, src2))
    }

    fn push_ins(&mut self, ins: Instruction) -> usize {
        self.instructions.push(ins);
        self.pc_spans.push(self.cur_span);
        self.instructions.len() - 1
    }

    fn here(&self) -> u32 {
        self.instructions.len() as u32
    }

    fn emit_jump(&mut self, target: u32) -> usize {
        self.push_ins(Instruction::encode_wide(Opcode::Jump, 0, target))
    }

    fn emit_branch(&mut self, op: Opcode, cond: u8, target: u32) -> usize {
        self.push_ins(Instruction::encode_branch(op, cond, target))
    }

    /// Points the jump or branch at `idx` to `target`.
    fn patch_jump(&mut self, idx: usize, target: u32) {
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
            self.error(codes::SEED, format!(
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
    fn error(&mut self, code: &'static str, message: String) -> u8 {
        let c = self.add_constant(PilotVal::from_str(&message));
        self.push_ins(Instruction::encode_wide(Opcode::Trap, 0, c));
        self.diagnostics.push(CompileDiagnostic {
            span: self.cur_span,
            code,
            message,
        });
        self.emit_null()
    }

    fn save_fn_state(&mut self) -> FnState {
        FnState {
            captures: std::mem::take(&mut self.captures),
            drop_flags: std::mem::take(&mut self.drop_flags),
            defers: std::mem::take(&mut self.defers),
            var_slots: std::mem::take(&mut self.var_slots),
            var_types: std::mem::take(&mut self.var_types),
            cur_ret_type: self.cur_ret_type.take(),
            cur_owner: self.cur_owner.take(),
            backing: self.backing.take(),
            next_local_reg: self.next_local_reg,
            next_reg: self.next_reg,
            loops: std::mem::take(&mut self.loops),
            tries: std::mem::take(&mut self.tries),
            block_scopes: std::mem::take(&mut self.block_scopes),
            yield_owners: std::mem::take(&mut self.yield_owners),
            gen_body: std::mem::take(&mut self.gen_body),
        }
    }

    fn restore_fn_state(&mut self, s: FnState) {
        self.captures = s.captures;
        self.drop_flags = s.drop_flags;
        self.defers = s.defers;
        self.var_slots = s.var_slots;
        self.var_types = s.var_types;
        self.cur_ret_type = s.cur_ret_type;
        self.cur_owner = s.cur_owner;
        self.backing = s.backing;
        self.next_local_reg = s.next_local_reg;
        self.next_reg = s.next_reg;
        self.loops = s.loops;
        self.tries = s.tries;
        self.block_scopes = s.block_scopes;
        self.yield_owners = s.yield_owners;
        self.gen_body = s.gen_body;
    }

    /// Compiles `id` with `hint` as the expected type.
    fn compile_with_hint(&mut self, arena: &AstArena, id: NodeId, hint: Option<String>) -> u8 {
        let saved = std::mem::replace(&mut self.type_hint, hint);
        let r = self.compile_node(arena, id);
        self.type_hint = saved;
        r
    }

    /// Named type of an annotation node, if it is a plain name.
    pub(crate) fn annot_name(arena: &AstArena, id: NodeId) -> Option<String> {
        let tid = arena.type_annot(id)?;
        let tn = arena.node(tid);
        matches!(tn.node_kind(), NodeKind::TypeNamed | NodeKind::IdentExpr)
            .then(|| Self::name(arena, tn.lhs).to_string())
    }

    fn reset_fn_state(&mut self) {
        self.var_slots.clear();
        self.drop_flags.clear();
        self.var_types.clear();
        self.cur_ret_type = None;
        self.cur_owner = None;
        self.backing = None;
        self.loops.clear();
        self.tries.clear();
        self.block_scopes.clear();
        self.defers.clear();
        self.yield_owners.clear();
        self.gen_body = None;
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
    /// The integer an integer literal, or a negated one, denotes.
    fn const_int(arena: &AstArena, id: NodeId) -> Option<i64> {
        let n = arena.node(id);
        match n.node_kind() {
            NodeKind::IntLit => arena.i64_pool.get(n.lhs as usize).copied(),
            NodeKind::UnaryExpr if n.flags == 0 => Self::const_int(arena, NodeId(n.lhs)).map(i64::wrapping_neg),
            _ => None,
        }
    }

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
            // `Low = 5` gives the variant its value; the next variant
            // without one counts on from it.
            let value = arena.field_default(NodeId(vid));
            let val = if value.is_none() {
                next
            } else {
                match Self::const_int(arena, value) {
                    Some(v) => v,
                    None => {
                        self.error(codes::SEED, format!("the value of `{e_nm}.{v_nm}` must be an integer literal"));
                        next
                    }
                }
            };
            next = val.wrapping_add(1);
            map.insert(v_nm, CMap::Val(PilotVal::from_int(val)));
        }
        self.const_maps.insert(e_nm, CMap::Map(map));
    }
}

