//! Milestone 33.3 & Self-Hosting Phase 2: Interleaved `comptime` Bridge, Line-1 `Cfg` `AstBundle` Arena,
//! 30 `__engine(...)` Callback Hooks, and Call-Quoting / Static Demotion Pass.

use std::sync::Arc;
use pilot_ast::{AstArena, NodeId, NodeKind};
use pilot_intern::{Interner, SymbolId};
use pilot_lexer::tokenize;
use pilot_parser::{
    Assoc, Fixity, FunctionId, FxHashMap, ModifierSpec, ModuleKwKind, Parser, ParserTables,
    PrattOpSpec,
};
use pilot_span::ByteSpan;
use crate::diagnostics::AnalysisDiagnostic;
use crate::macros::{desugar_bracket_attributes, splice_quote_template};

// =========================================================================
// PHASE 2: COMPTIME VALUE & AST BUNDLE ARENA
// =========================================================================

#[derive(Clone, Debug, PartialEq)]
pub enum ComptimeValue {
    Void,
    Bool(bool),
    Int(i128),
    Uint(u128),
    Float(f64),
    Str(SymbolId),
    Ident(SymbolId),
    AstNode(NodeId),
    FnHandle(FunctionId),
    /// Untyped structural key-value bundle (`let Cfg = { Hooks = { ... }, ... }`)
    AstBundle(Arc<FxHashMap<SymbolId, ComptimeValue>>),
    /// Lowered backend SSA instruction descriptor (`__ssa`, `__ssa_unary`, `__ssa_cast`, etc.)
    SsaIntrinsic {
        opcode: String,
        args: Vec<ComptimeValue>,
        target_type: Option<String>,
    },
    /// Compile-time tuple of values (e.g. `type_fields_of(T)`)
    Tuple(Vec<ComptimeValue>),
    /// Loop exit CFG jump marker (`break`)
    CfgJumpLoopExit,
    /// Loop head CFG jump marker (`continue`)
    CfgJumpLoopHead,
}

impl ComptimeValue {
    pub fn as_uint(&self) -> Option<u128> {
        match self {
            ComptimeValue::Uint(v) => Some(*v),
            ComptimeValue::Int(v) if *v >= 0 => Some(*v as u128),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i128> {
        match self {
            ComptimeValue::Int(v) => Some(*v),
            ComptimeValue::Uint(v) => Some(*v as i128),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ComptimeValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str<'a>(&self, interner: &'a Interner) -> Option<&'a str> {
        match self {
            ComptimeValue::Str(sym) | ComptimeValue::Ident(sym) => Some(interner.resolve(*sym)),
            _ => None,
        }
    }

    pub fn get_field(&self, sym: SymbolId) -> Option<&ComptimeValue> {
        match self {
            ComptimeValue::AstBundle(map) => map.get(&sym),
            _ => None,
        }
    }

    pub fn get_field_by_name<'a>(&'a self, name: &str, interner: &Interner) -> Option<&'a ComptimeValue> {
        match self {
            ComptimeValue::AstBundle(map) => {
                for (k, v) in map.iter() {
                    if interner.resolve(*k) == name {
                        return Some(v);
                    }
                }
                None
            }
            _ => None,
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ReceptorSlot {
    // Literal & Condition Receptors
    BoolCondition,         // Type produced by `true`, `false`, and consumed by `if` conditions
    IntLiteral,            // Default integer literal receptor (`42`)
    UintLiteral,           // Default unsigned/size literal receptor
    FloatLiteral,          // Default float literal receptor (`3.14`)
    CharLiteral,           // Default character literal receptor (`'a'`)
    Utf8StrLiteral,        // Default string literal receptor (`"hello"`)
    VoidUnit,              // 0-bit unit receptor (`()`)
    NeverBottom,           // 0-bit diverging bottom receptor

    // 6 Homoiconic Call-Quoting Receptors (`!`, `!!`, `!&`, `!^`, `!@`, `!?`)
    ShallowQuoteSlot,      // Receptor for `fn!(a, b)`
    DeepSpliceSlot,        // Receptor for `fn!!(~a, b)`
    AbiFrameSlot,          // Receptor for `fn!&(a, b)`
    ReactiveSignalSlot,    // Receptor for `fn!^(a, b)`
    SymbolicAdjointSlot,   // Receptor for `fn!@(a, b)`
    EffectDryRunSlot,      // Receptor for `fn!?(a, b)`

    // 7 Attribute Pillar Receptors
    AttributeSlot,         // Receptor for `IAttribute`
    InterceptorSlot,       // Receptor for `IInterceptor`
    FormulaSlot,           // Receptor for `IFormulaAttribute`
    ReactiveSlot,          // Receptor for `IReactiveAttribute`
    InheritedSlot,         // Receptor for `IInheritedAttribute`
    EcsRuleSlot,           // Receptor for `IEcsRule`
    PatchAttributeSlot,    // Receptor for `IPatchAttribute`
}

impl ReceptorSlot {
    pub fn from_slot_or_lang_name(name: &str) -> Option<Self> {
        match name {
            "bool_condition" | "bool" => Some(Self::BoolCondition),
            "int_literal" | "i64" => Some(Self::IntLiteral),
            "uint_literal" | "usize" => Some(Self::UintLiteral),
            "float_literal" | "f64" => Some(Self::FloatLiteral),
            "char_literal" | "char" => Some(Self::CharLiteral),
            "utf8_str_literal" | "str" => Some(Self::Utf8StrLiteral),
            "void_unit" | "void" => Some(Self::VoidUnit),
            "never_bottom" | "never" => Some(Self::NeverBottom),
            "shallow_quote" | "CallExpr" => Some(Self::ShallowQuoteSlot),
            "deep_splice" | "ast" => Some(Self::DeepSpliceSlot),
            "abi_frame" | "frame" => Some(Self::AbiFrameSlot),
            "reactive_signal" | "Signal" => Some(Self::ReactiveSignalSlot),
            "symbolic_adjoint" | "Dual" => Some(Self::SymbolicAdjointSlot),
            "effect_dry_run" | "EffectPlan" => Some(Self::EffectDryRunSlot),
            "attribute" | "IAttribute" => Some(Self::AttributeSlot),
            "interceptor" | "IInterceptor" => Some(Self::InterceptorSlot),
            "formula" | "IFormulaAttribute" => Some(Self::FormulaSlot),
            "reactive" | "IReactiveAttribute" => Some(Self::ReactiveSlot),
            "inherited" | "IInheritedAttribute" => Some(Self::InheritedSlot),
            "ecs_rule" | "IEcsRule" => Some(Self::EcsRuleSlot),
            "patch_attribute" | "IPatchAttribute" => Some(Self::PatchAttributeSlot),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LangItem {
    IAnyValue,
    IValue,
    IReference,
    IObject,
    IGuard,
    Void,
    Never,
    Bool,
    Char,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    F16,
    F32,
    F64,
    F128,
    Str,
    Ptr,
    Slice,
    Stride,
    Array,
    Tuple,
    Option,
    Result,
    Union,
    Fn,
    Range,
    Span,
    Step,
    From,
    Until,
    Full,
    Splat,
    Pairs,
    Expand,
    Map,
    Frame,
    Type,
    Ast,
    Simd,
    Grid,
    Ring,
    Pin,
    Open,
    After,
    Order,
    Signal,
    IAttribute,
    IInterceptor,
    IFormulaAttribute,
    IReactiveAttribute,
    IInheritedAttribute,
    IEcsRule,
    IPatchAttribute,
}

impl LangItem {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "IAnyValue" => Some(Self::IAnyValue),
            "IValue" => Some(Self::IValue),
            "IReference" => Some(Self::IReference),
            "IObject" => Some(Self::IObject),
            "IGuard" => Some(Self::IGuard),
            "IAttribute" => Some(Self::IAttribute),
            "IInterceptor" => Some(Self::IInterceptor),
            "IFormulaAttribute" => Some(Self::IFormulaAttribute),
            "IReactiveAttribute" => Some(Self::IReactiveAttribute),
            "IInheritedAttribute" => Some(Self::IInheritedAttribute),
            "IEcsRule" => Some(Self::IEcsRule),
            "IPatchAttribute" => Some(Self::IPatchAttribute),
            "void" => Some(Self::Void),
            "never" => Some(Self::Never),
            "bool" => Some(Self::Bool),
            "char" => Some(Self::Char),
            "i8" => Some(Self::I8),
            "i16" => Some(Self::I16),
            "i32" => Some(Self::I32),
            "i64" => Some(Self::I64),
            "i128" => Some(Self::I128),
            "isize" => Some(Self::Isize),
            "u8" => Some(Self::U8),
            "u16" => Some(Self::U16),
            "u32" => Some(Self::U32),
            "u64" => Some(Self::U64),
            "u128" => Some(Self::U128),
            "usize" => Some(Self::Usize),
            "f16" => Some(Self::F16),
            "f32" => Some(Self::F32),
            "f64" => Some(Self::F64),
            "f128" => Some(Self::F128),
            "str" => Some(Self::Str),
            "ptr" => Some(Self::Ptr),
            "slice" => Some(Self::Slice),
            "stride" => Some(Self::Stride),
            "array" => Some(Self::Array),
            "tuple" => Some(Self::Tuple),
            "option" => Some(Self::Option),
            "result" => Some(Self::Result),
            "union" => Some(Self::Union),
            "fn" => Some(Self::Fn),
            "range" => Some(Self::Range),
            "span" => Some(Self::Span),
            "step" => Some(Self::Step),
            "from" => Some(Self::From),
            "until" => Some(Self::Until),
            "full" => Some(Self::Full),
            "splat" => Some(Self::Splat),
            "pairs" => Some(Self::Pairs),
            "expand" => Some(Self::Expand),
            "map" => Some(Self::Map),
            "frame" => Some(Self::Frame),
            "type" => Some(Self::Type),
            "ast" => Some(Self::Ast),
            "simd" => Some(Self::Simd),
            "grid" => Some(Self::Grid),
            "ring" => Some(Self::Ring),
            "pin" => Some(Self::Pin),
            "open" => Some(Self::Open),
            "after" => Some(Self::After),
            "order" => Some(Self::Order),
            "signal" => Some(Self::Signal),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RegisterClass {
    Unit,
    NoReturn,
    GprUnsigned,
    GprSigned,
    SimdFloat,
    SimdVector { lanes: u16 },
    RawPtr,
    Composite,
}

impl RegisterClass {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "unit" | "void" => Some(Self::Unit),
            "no_return" | "never" => Some(Self::NoReturn),
            "gpr_unsigned" => Some(Self::GprUnsigned),
            "gpr_signed" => Some(Self::GprSigned),
            "simd_float" => Some(Self::SimdFloat),
            "simd_vector" => Some(Self::SimdVector { lanes: 4 }),
            "raw_ptr" => Some(Self::RawPtr),
            "composite" => Some(Self::Composite),
            _ => None,
        }
    }

    pub fn to_reg_class(self) -> crate::types::RegClass {
        match self {
            Self::Unit => crate::types::RegClass::Unit,
            Self::NoReturn => crate::types::RegClass::NoReturn,
            Self::GprSigned => crate::types::RegClass::GprSigned,
            Self::GprUnsigned => crate::types::RegClass::GprUnsigned,
            Self::SimdFloat => crate::types::RegClass::SimdFloat,
            Self::SimdVector { lanes } => crate::types::RegClass::SimdVector { lanes },
            Self::RawPtr => crate::types::RegClass::RawPtr,
            Self::Composite => crate::types::RegClass::Composite,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MemoryLayoutMode {
    CAbi,
    Soa,
    Transparent,
    Packed,
    Niche,
    RegisterPair,
    InlineArray,
}

impl MemoryLayoutMode {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "c_abi" | "c" => Some(Self::CAbi),
            "soa" => Some(Self::Soa),
            "transparent" => Some(Self::Transparent),
            "packed" => Some(Self::Packed),
            "niche" => Some(Self::Niche),
            "register_pair" => Some(Self::RegisterPair),
            "inline_array" => Some(Self::InlineArray),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InlineHint {
    Always,
    Never,
    Auto,
}

impl InlineHint {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "always" => Some(Self::Always),
            "never" => Some(Self::Never),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypeLayoutSpec {
    pub type_name: String,
    pub decl_node: NodeId,
    pub bit_width: u32,
    pub reg_class: RegisterClass,
    pub has_explicit_register_layout: bool,
    pub memory_layout: Option<MemoryLayoutMode>,
    pub lattice_bits: u8,
    pub type_params: Vec<String>,
    pub fields: Vec<(String, usize, usize)>, // (name, offset_bytes, size_bytes)
    pub field_types: Vec<String>,            // Declared field type expressions for structural/generic deduction
    /// Operator symbol -> lowered SSA opcode (e.g. "+" -> "add", "<" -> "ult", "!" -> "xor")
    pub operators: FxHashMap<String, String>,
}

impl TypeLayoutSpec {
    pub fn new(type_name: String, decl_node: NodeId) -> Self {
        Self {
            type_name,
            decl_node,
            bit_width: 0,
            reg_class: RegisterClass::Composite,
            has_explicit_register_layout: false,
            memory_layout: None,
            lattice_bits: 0b00,
            type_params: Vec::new(),
            fields: Vec::new(),
            field_types: Vec::new(),
            operators: FxHashMap::default(),
        }
    }

    pub fn hardware_physics(&self) -> crate::types::HardwarePhysics {
        if self.has_explicit_register_layout {
            let rc = if self.bit_width == 0 && self.fields.is_empty() {
                if self.reg_class == RegisterClass::NoReturn {
                    crate::types::RegClass::NoReturn
                } else {
                    crate::types::RegClass::Unit
                }
            } else {
                self.reg_class.to_reg_class()
            };
            crate::types::HardwarePhysics {
                reg_class: rc,
                bit_width: self.bit_width,
            }
        } else {
            crate::types::HardwarePhysics {
                reg_class: crate::types::RegClass::Composite,
                bit_width: (self.byte_size() * 8) as u32,
            }
        }
    }

    pub fn byte_size(&self) -> usize {
        if self.bit_width == 0 {
            if self.fields.is_empty() {
                0
            } else {
                let last_end = self
                    .fields
                    .iter()
                    .map(|(_, off, sz)| *off + *sz)
                    .max()
                    .unwrap_or(0);
                let align = self.byte_align().max(1);
                (last_end + align - 1) & !(align - 1)
            }
        } else {
            ((self.bit_width as usize) + 7) / 8
        }
    }

    pub fn byte_align(&self) -> usize {
        if self.bit_width == 0 {
            self.fields
                .iter()
                .filter(|(_, _, fsz)| *fsz > 0)
                .map(|(_, _, fsz)| (*fsz).next_power_of_two().min(8))
                .max()
                .unwrap_or(1)
        } else {
            let sz = self.byte_size();
            if sz == 0 {
                1
            } else {
                sz.next_power_of_two().min(16)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BootstrapDiagnostic {
    /// Emitted when any code after `seal_bootstrap_bridge` attempts to invoke `__engine(...)` directly.
    BootstrapBridgeSealedError {
        hook: String,
        span: ByteSpan,
    },
    UnknownEngineHook {
        hook: String,
        span: ByteSpan,
    },
    UnknownBracketAttribute {
        name: String,
        span: ByteSpan,
    },
    UnknownSyntaxKeyword {
        keyword: String,
        span: ByteSpan,
    },
    UserCompileError {
        message: String,
        span: ByteSpan,
    },
    InvalidArguments {
        hook: String,
        details: String,
        span: ByteSpan,
    },
    ParseError {
        errors: Vec<String>,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PatchPartKind {
    All = 0,
    Signature = 1,
    Params = 2,
    Body = 3,
}

#[derive(Clone, Debug)]
pub struct PatchHandleState {
    pub target_fn_id: NodeId,
    pub binding_sym: SymbolId,
    pub is_mutable: bool,
    pub window_lo: u32,
    pub window_hi: u32,
    pub line_overrides: FxHashMap<u32, String>,
    pub ast_replacements: Vec<(NodeId, NodeId)>,
    pub synthesized_clone_fn: Option<NodeId>,
}

#[derive(Clone, Debug)]
pub struct ComptimeFnDef {
    pub id: FunctionId,
    pub name: SymbolId,
    pub name_str: String,
    pub params: Vec<String>,
    pub param_is_unquote: Vec<bool>,
    pub body_node: NodeId,
    pub def_arena: Arc<AstArena>,
    pub clearance_ring: u32,
    pub is_genesis_wrapper: bool,
}

#[derive(Default)]
pub struct ComptimeEngine {
    pub globals: FxHashMap<SymbolId, ComptimeValue>,
    pub globals_by_name: FxHashMap<String, ComptimeValue>,
    pub functions: Vec<ComptimeFnDef>,
    pub lang_items: FxHashMap<LangItem, NodeId>,
    pub lang_items_by_name: FxHashMap<String, NodeId>,
    pub receptor_slots: FxHashMap<ReceptorSlot, NodeId>,
    pub receptor_type_names: FxHashMap<ReceptorSlot, String>,
    pub type_layouts: FxHashMap<String, TypeLayoutSpec>,
    pub bundle_layouts: FxHashMap<String, TypeLayoutSpec>,
    pub node_to_type_name: FxHashMap<u32, String>,
    pub inline_hints: FxHashMap<u32, InlineHint>,
    pub lattice_interfaces: FxHashMap<String, u8>,
    pub type_construct_targets: FxHashMap<String, String>,
    pub core_naming_enforced_pkgs: Vec<String>,
    pub active_pkg: Option<String>,
    pub active_clearance: u32,
    pub diagnostics: Vec<BootstrapDiagnostic>,
    pub patch_handles: FxHashMap<SymbolId, PatchHandleState>,
    pub patch_handles_by_name: FxHashMap<String, SymbolId>,
    in_genesis_wrapper_depth: u32,
}

impl ComptimeEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolves a dotted path inside `Cfg` (e.g. `&["Hooks", "bind_attr"]` or `&["Lattice", "guard"]`).
    pub fn resolve_cfg_path(&self, path: &[&str], interner: &Interner) -> Option<ComptimeValue> {
        let mut cur = self.globals_by_name.get("Cfg")?;
        for &seg in path {
            cur = cur.get_field_by_name(seg, interner)?;
        }
        Some(cur.clone())
    }

    pub fn resolve_cfg_str(&self, path: &[&str], interner: &Interner) -> Option<String> {
        let val = self.resolve_cfg_path(path, interner)?;
        val.as_str(interner).map(|s| s.to_string())
    }

    pub fn resolve_cfg_uint(&self, path: &[&str], interner: &Interner) -> Option<u128> {
        let val = self.resolve_cfg_path(path, interner)?;
        val.as_uint()
    }

    pub fn lang_item_node(&self, item: LangItem) -> Option<NodeId> {
        self.lang_items.get(&item).copied()
    }

    pub fn type_layout(&self, type_name: &str) -> Option<&TypeLayoutSpec> {
        self.type_layouts.get(type_name)
    }

    pub fn ssa_opcode_for_operator(&self, type_name: &str, op_sym: &str) -> Option<&str> {
        self.type_layouts
            .get(type_name)
            .and_then(|spec| spec.operators.get(op_sym).map(|s| s.as_str()))
    }

    pub fn ssa_opcode_for_physics(&self, reg_class: RegisterClass, bit_width: u32, op_sym: &str) -> Option<&str> {
        for spec in self.type_layouts.values() {
            if spec.has_explicit_register_layout
                && spec.reg_class == reg_class
                && spec.bit_width == bit_width
            {
                if let Some(op) = spec.operators.get(op_sym) {
                    return Some(op.as_str());
                }
            }
        }
        None
    }

    pub fn receptor_type_name(&self, slot: ReceptorSlot) -> Option<&str> {
        self.receptor_type_names.get(&slot).map(|s| s.as_str())
    }

    pub fn create_patch_handle(
        &mut self,
        target_fn_id: NodeId,
        binding_sym: SymbolId,
        binding_name: &str,
        is_mutable: bool,
        window_lo: u32,
        window_hi: u32,
    ) {
        let handle = PatchHandleState {
            target_fn_id,
            binding_sym,
            is_mutable,
            window_lo,
            window_hi,
            line_overrides: FxHashMap::default(),
            ast_replacements: Vec::new(),
            synthesized_clone_fn: None,
        };
        self.patch_handles.insert(binding_sym, handle);
        self.patch_handles_by_name.insert(binding_name.to_string(), binding_sym);
    }

    pub fn get_patch_line(&self, handle_sym: SymbolId, line_idx: u32, arena: &AstArena) -> Result<String, AnalysisDiagnostic> {
        let handle = match self.patch_handles.get(&handle_sym) {
            Some(h) => h,
            None => return Ok(String::new()),
        };
        if line_idx < handle.window_lo || line_idx > handle.window_hi {
            return Err(AnalysisDiagnostic::error(
                "PATCH001",
                format!("Line {} is outside permitted patch window {}..{}", line_idx, handle.window_lo, handle.window_hi),
                arena.span(handle.target_fn_id),
            ));
        }
        if let Some(text) = handle.line_overrides.get(&line_idx) {
            return Ok(text.clone());
        }
        Ok(String::new())
    }

    pub fn set_patch_line(&mut self, handle_sym: SymbolId, line_idx: u32, new_text: String, span: ByteSpan) -> Result<(), AnalysisDiagnostic> {
        let handle = match self.patch_handles.get_mut(&handle_sym) {
            Some(h) => h,
            None => return Ok(()),
        };
        if !handle.is_mutable {
            return Err(AnalysisDiagnostic::error(
                "PATCH002",
                "Cannot mutate line through immutable patch binding; declare with 'var'".to_string(),
                span,
            ));
        }
        if line_idx < handle.window_lo || line_idx > handle.window_hi {
            return Err(AnalysisDiagnostic::error(
                "PATCH001",
                format!("Line {} is outside permitted patch window {}..{}", line_idx, handle.window_lo, handle.window_hi),
                span,
            ));
        }
        handle.line_overrides.insert(line_idx, new_text);
        Ok(())
    }

    pub fn replace_ast_pattern(&mut self, handle_sym: SymbolId, target: NodeId, replacement: NodeId) {
        if let Some(handle) = self.patch_handles.get_mut(&handle_sym) {
            handle.ast_replacements.push((target, replacement));
        }
    }

    pub fn synthesize_patched_fn(&mut self, handle_sym: SymbolId, arena: &mut AstArena) -> Option<NodeId> {
        let handle = self.patch_handles.get_mut(&handle_sym)?;
        if let Some(clone_id) = handle.synthesized_clone_fn {
            return Some(clone_id);
        }
        let target_id = handle.target_fn_id;
        let target_node = *arena.node(target_id);
        if target_node.node_kind() != NodeKind::FnDecl {
            return None;
        }
        let orig_name = arena.interner.resolve(SymbolId(target_node.lhs)).to_string();
        let clone_name = format!("__patched_{}_{}", orig_name, arena.nodes.len());
        let clone_sym = arena.interner.intern(&clone_name);
        let span = arena.span(target_id);

        let fn_start = target_node.rhs;
        let fn_len = target_node.flags as usize;
        let children: Vec<u32> = if fn_len > 0 {
            arena.children_slice(fn_start, fn_len as u32).to_vec()
        } else {
            vec![target_node.rhs]
        };

        let replacements = handle.ast_replacements.clone();
        let mut cloned_children = Vec::with_capacity(children.len());
        for &cid in &children {
            let cloned = crate::infer::clone_ast_subtree_with_replacements(arena, NodeId(cid), &replacements);
            cloned_children.push(cloned);
        }

        let (new_start, _) = arena.alloc_child_slice(&cloned_children);
        let clone_node_id = arena.alloc_node(NodeKind::FnDecl, fn_len as u16, span, clone_sym.0, new_start);

        if let Some(&ann) = arena.type_annotations.get(&target_id.0) {
            arena.type_annotations.insert(clone_node_id.0, ann);
        }

        handle.synthesized_clone_fn = Some(clone_node_id);
        Some(clone_node_id)
    }

    pub fn register_comptime_fn(
        &mut self,
        arena: &AstArena,
        fn_node: NodeId,
        parser_tables: &mut ParserTables,
    ) -> Result<FunctionId, BootstrapDiagnostic> {
        let node = *arena.node(fn_node);
        let name_sym = SymbolId(node.lhs);
        let name_str = arena.interner.resolve(name_sym).to_string();
        let tab_sym = parser_tables.interner.intern(&name_str);

        let slice = arena.children_slice(node.rhs, node.flags as u32);
        let body_node = slice.first().copied().map(NodeId).unwrap_or(NodeId::NONE);
        let mut params = Vec::new();
        let mut param_is_unquote = Vec::new();
        for &p_raw in slice.iter().skip(1) {
            let p_node = *arena.node(NodeId(p_raw));
            if p_node.node_kind() == NodeKind::ParamDecl {
                let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                params.push(p_name);
                param_is_unquote.push(p_node.flags == 1);
            }
        }

        let mut clearance_ring = arena
            .node_clearance
            .get(&fn_node.0)
            .map(|c| c.level)
            .unwrap_or(self.active_clearance);

        if let Some(&clr_expr) = arena.clearance_exprs.get(&fn_node.0) {
            let mut arena_clone = arena.clone();
            if let Ok(val) = self.eval_expr(&mut arena_clone, clr_expr, parser_tables) {
                if let Some(u) = val.as_uint() {
                    clearance_ring = u as u32;
                }
            }
        }

        let fn_id = FunctionId(self.functions.len() as u32);
        let is_genesis_wrapper = !parser_tables.bootstrap_sealed;
        let def = ComptimeFnDef {
            id: fn_id,
            name: tab_sym,
            name_str: name_str.clone(),
            params,
            param_is_unquote,
            body_node,
            def_arena: Arc::new(arena.clone()),
            clearance_ring,
            is_genesis_wrapper,
        };
        self.functions.push(def);

        let handle = ComptimeValue::FnHandle(fn_id);
        self.globals.insert(tab_sym, handle.clone());
        self.globals_by_name.insert(name_str, handle);

        Ok(fn_id)
    }

    /// Evaluates a single top-level AST item in interleaved bootstrap order, mutating `parser_tables` in place.
    pub fn eval_top_level_item(
        &mut self,
        arena: &mut AstArena,
        item_id: NodeId,
        parser_tables: &mut ParserTables,
    ) -> Result<ComptimeValue, BootstrapDiagnostic> {
        if item_id.is_none() {
            return Ok(ComptimeValue::Void);
        }

        // Sync interner symbols and type construct targets from arena/parser_tables
        for i in 0..arena.interner.symbol_count() {
            let s = arena.interner.resolve(SymbolId(i as u32));
            parser_tables.interner.intern(s);
        }
        for (k, v) in &parser_tables.type_construct_targets {
            self.type_construct_targets.insert(k.clone(), v.clone());
        }

        // Check for package declaration `pkg Pilot.Core.Genesis;`
        if arena.pkg_decls.contains(&item_id.0) {
            let pkg_node = *arena.node(item_id);
            let pkg_path = arena.interner.resolve(SymbolId(pkg_node.lhs)).to_string();
            if let Some(&clr_expr) = arena.clearance_exprs.get(&item_id.0) {
                if let Ok(val) = self.eval_expr(arena, clr_expr, parser_tables) {
                    if let Some(u) = val.as_uint() {
                        self.active_clearance = u as u32;
                    }
                }
            }
            self.active_pkg = Some(pkg_path);
            desugar_bracket_attributes(arena, item_id, parser_tables, self)?;
            return Ok(ComptimeValue::Void);
        }

        let node = *arena.node(item_id);
        match node.node_kind() {
            NodeKind::ComptimeExpr => {
                let inner_id = NodeId(node.lhs);
                self.eval_comptime_stmt_or_block(arena, inner_id, parser_tables)
            }
            NodeKind::FnDecl => {
                if arena.comptime_fns.contains(&item_id.0) {
                    self.register_comptime_fn(arena, item_id, parser_tables)?;
                }
                desugar_bracket_attributes(arena, item_id, parser_tables, self)?;
                Ok(ComptimeValue::Void)
            }
            NodeKind::LetStmt | NodeKind::VarStmt => {
                let var_name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                desugar_bracket_attributes(arena, item_id, parser_tables, self)?;
                let rhs_id = NodeId(node.rhs);
                if rhs_id.is_some() && (rhs_id.0 as usize) < arena.nodes.len() {
                    let rhs_node = *arena.node(rhs_id);
                    if rhs_node.node_kind() == NodeKind::BlockExpr {
                        let mut bspec = TypeLayoutSpec::new(var_name.clone(), item_id);
                        let children: Vec<u32> = arena.children_slice(rhs_node.lhs, rhs_node.rhs).to_vec();
                        let mut field_offset = 0usize;
                        for c_raw in children {
                            let c_id = NodeId(c_raw);
                            let c_node = *arena.node(c_id);
                            if c_node.node_kind() == NodeKind::LetStmt {
                                let f_name = arena.interner.resolve(SymbolId(c_node.lhs)).to_string();
                                let f_ty_id = arena.type_annotations.get(&c_id.0).copied().unwrap_or(NodeId::NONE);
                                let f_ty_str = self.format_type_node(arena, f_ty_id);
                                let f_size = self.estimate_type_node_size(arena, f_ty_id);
                                let f_lat = self.estimate_type_node_lattice(arena, f_ty_id);
                                let aligned = if f_size > 0 {
                                    (field_offset + (f_size - 1)) & !(f_size - 1)
                                } else {
                                    field_offset
                                };
                                bspec.fields.push((f_name, aligned, f_size));
                                bspec.field_types.push(f_ty_str);
                                bspec.lattice_bits |= f_lat;
                                field_offset = aligned + f_size;
                            } else if c_node.node_kind() == NodeKind::OperatorDecl {
                                let op_sym_id = arena.operator_symbols.get(&c_id.0).copied().unwrap_or(SymbolId(0));
                                let op_str = arena.interner.resolve(op_sym_id).to_string();
                                let raw_body = arena.operator_bodies.get(&c_id.0).copied().unwrap_or(NodeId::NONE);
                                if raw_body.is_some() {
                                    let mut local_env = FxHashMap::default();
                                    let self_sym = parser_tables.interner.intern("self");
                                    local_env.insert("self".to_string(), ComptimeValue::Ident(self_sym));
                                    if let Some(params) = arena.operator_params.get(&c_id.0).cloned() {
                                        for p_id in params {
                                            let p_node = *arena.node(p_id);
                                            if p_node.node_kind() == NodeKind::ParamDecl {
                                                let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                                                let p_sym = parser_tables.interner.intern(&p_name);
                                                local_env.insert(p_name, ComptimeValue::Ident(p_sym));
                                            }
                                        }
                                    }
                                    let prev_depth = self.in_genesis_wrapper_depth;
                                    self.in_genesis_wrapper_depth += 1;
                                    let body_val = self.eval_expr_with_env(arena, raw_body, &mut local_env, parser_tables)?;
                                    self.in_genesis_wrapper_depth = prev_depth;
                                    if let ComptimeValue::SsaIntrinsic { opcode, .. } = body_val {
                                        let is_unary = arena
                                            .operator_params
                                            .get(&c_id.0)
                                            .map_or(true, |p| p.is_empty());
                                        if is_unary && bspec.operators.contains_key(&op_str) {
                                            bspec.operators.insert(format!("unary_{}", op_str), opcode);
                                        } else {
                                            bspec.operators.insert(op_str, opcode);
                                        }
                                    }
                                }
                            }
                        }
                        self.bundle_layouts.insert(var_name, bspec);
                    }
                }
                Ok(ComptimeValue::Void)
            }
            NodeKind::StructDecl => {
                let name_str = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                self.node_to_type_name.insert(item_id.0, name_str.clone());
                let mut init_lattice = 0b00;
                if let Some(bases) = arena.struct_base_types.get(&item_id.0) {
                    for &base_ty_node in bases {
                        let base_name = self.format_type_node(arena, base_ty_node);
                        let base_head = base_name.split('<').next().unwrap_or(&base_name).trim();
                        if let Some(&lat) = self.lattice_interfaces.get(base_head) {
                            init_lattice |= lat;
                        }
                    }
                }
                let type_params: Vec<String> = arena
                    .struct_type_params
                    .get(&item_id.0)
                    .map(|syms| syms.iter().map(|&s| arena.interner.resolve(s).to_string()).collect())
                    .unwrap_or_default();

                let entry = self
                    .type_layouts
                    .entry(name_str.clone())
                    .or_insert_with(|| TypeLayoutSpec::new(name_str.clone(), item_id));
                entry.lattice_bits |= init_lattice;
                if !type_params.is_empty() && entry.type_params.is_empty() {
                    entry.type_params = type_params;
                }

                // Execute `[Lang(...), Bits(...), Layout(...)]` attributes left-to-right
                desugar_bracket_attributes(arena, item_id, parser_tables, self)?;

                // Record declared struct fields from `arena.record_fields`
                if let Some(fields) = arena.record_fields.get(&item_id.0).cloned() {
                    let mut field_offset = self
                        .type_layouts
                        .get(&name_str)
                        .map(|l| l.byte_size())
                        .unwrap_or(0);
                    for (f_sym, f_ty_id) in fields {
                        let f_name = arena.interner.resolve(f_sym).to_string();
                        let f_ty_str = self.format_type_node(arena, f_ty_id);
                        let f_size = self.estimate_type_node_size(arena, f_ty_id);
                        let f_lat = self.estimate_type_node_lattice(arena, f_ty_id);
                        let aligned = if f_size > 0 {
                            (field_offset + (f_size - 1)) & !(f_size - 1)
                        } else {
                            field_offset
                        };
                        if let Some(layout) = self.type_layouts.get_mut(&name_str) {
                            layout.fields.push((f_name, aligned, f_size));
                            layout.field_types.push(f_ty_str);
                            layout.lattice_bits |= f_lat;
                        }
                        field_offset = aligned + f_size;
                    }
                }

                // Evaluate fields, `*Bundle`/`*Facet` splices, destructors, and `op` declarations inside the struct body
                let body_id = NodeId(node.rhs);
                let children: Vec<u32> = if body_id.is_some() && (body_id.0 as usize) < arena.nodes.len() {
                    let body_node = *arena.node(body_id);
                    if body_node.node_kind() == NodeKind::BlockExpr {
                        arena.children_slice(body_node.lhs, body_node.rhs).to_vec()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };
                for c_raw in children {
                    let c_id = NodeId(c_raw);
                    let c_node = *arena.node(c_id);
                    match c_node.node_kind() {
                        NodeKind::LetStmt => {
                            let f_name = arena.interner.resolve(SymbolId(c_node.lhs)).to_string();
                            let f_ty_id = arena.type_annotations.get(&c_id.0).copied().unwrap_or(NodeId::NONE);
                            let f_ty_str = self.format_type_node(arena, f_ty_id);
                            let f_size = self.estimate_type_node_size(arena, f_ty_id);
                            let f_lat = self.estimate_type_node_lattice(arena, f_ty_id);
                            if let Some(layout) = self.type_layouts.get_mut(&name_str) {
                                let cur_off = layout.byte_size();
                                let aligned = if f_size > 0 {
                                    (cur_off + (f_size - 1)) & !(f_size - 1)
                                } else {
                                    cur_off
                                };
                                layout.fields.push((f_name, aligned, f_size));
                                layout.field_types.push(f_ty_str);
                                layout.lattice_bits |= f_lat;
                            }
                        }
                        NodeKind::DestructorDecl => {
                            if let Some(layout) = self.type_layouts.get_mut(&name_str) {
                                layout.lattice_bits |= 0b10;
                            }
                        }
                        NodeKind::ExprStmt => {
                            let expr_id = NodeId(c_node.lhs);
                            if expr_id.is_some() && (expr_id.0 as usize) < arena.nodes.len() {
                                let expr_node = *arena.node(expr_id);
                                if expr_node.node_kind() == NodeKind::UnaryExpr
                                    && expr_node.flags == pilot_ast::UNARY_OP_SPLAT
                                {
                                    let op_id = NodeId(expr_node.lhs);
                                    if op_id.is_some() && (op_id.0 as usize) < arena.nodes.len() {
                                        let op_node = *arena.node(op_id);
                                        if op_node.node_kind() == NodeKind::IdentExpr {
                                            let bname = arena.interner.resolve(SymbolId(op_node.lhs)).to_string();
                                            if let Some(bspec) = self.bundle_layouts.get(&bname).cloned() {
                                                if let Some(layout) = self.type_layouts.get_mut(&name_str) {
                                                    for ((fnm, _, fsz), fty) in bspec.fields.into_iter().zip(bspec.field_types.into_iter().chain(std::iter::repeat(String::new()))) {
                                                        let cur_off = layout.byte_size();
                                                        let aligned = if fsz > 0 {
                                                            (cur_off + (fsz - 1)) & !(fsz - 1)
                                                        } else {
                                                            cur_off
                                                        };
                                                        layout.fields.push((fnm, aligned, fsz));
                                                        layout.field_types.push(fty);
                                                    }
                                                    layout.lattice_bits |= bspec.lattice_bits;
                                                    for (op_k, op_v) in bspec.operators {
                                                        layout.operators.insert(op_k, op_v);
                                                    }
                                                }
                                                let spliced_ftys: Vec<String> = self
                                                    .type_layouts
                                                    .get(&name_str)
                                                    .map(|l| l.field_types.clone())
                                                    .unwrap_or_default();
                                                let host_lat = self
                                                    .type_layouts
                                                    .get(&name_str)
                                                    .map(|l| l.lattice_bits)
                                                    .unwrap_or(0);
                                                let mut extra_lat = 0u8;
                                                for fty in &spliced_ftys {
                                                    let f_base = fty.split('<').next().unwrap_or(fty).trim();
                                                    let is_raw_ptr = self
                                                        .type_layouts
                                                        .get(f_base)
                                                        .map_or(false, |s| s.has_explicit_register_layout && s.reg_class == RegisterClass::RawPtr);
                                                    if !((host_lat & 0b10) != 0 && is_raw_ptr) {
                                                        extra_lat |= self.query_type_size_align_lattice(fty).2;
                                                    }
                                                }
                                                if let Some(layout) = self.type_layouts.get_mut(&name_str) {
                                                    layout.lattice_bits |= extra_lat;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        NodeKind::OperatorDecl => {
                            let op_sym_id = arena.operator_symbols.get(&c_id.0).copied().unwrap_or(SymbolId(0));
                            let op_str = arena.interner.resolve(op_sym_id).to_string();
                            let raw_body = arena.operator_bodies.get(&c_id.0).copied().unwrap_or(NodeId::NONE);

                            // Invoke the registered `op` syntax macro if present
                            if let Some(&op_macro_fn) = parser_tables.syntax_macros_by_name.get("op") {
                                let args = [
                                    ComptimeValue::AstNode(c_id),
                                    ComptimeValue::AstNode(raw_body),
                                ];
                                let _ = self.invoke_function(
                                    op_macro_fn,
                                    &args,
                                    arena,
                                    parser_tables,
                                    arena.span(c_id),
                                )?;
                            }

                            // Evaluate the operator body (`__ssa(Cfg.Ssa.add, self, rhs)`) to bind its SSA opcode
                            if raw_body.is_some() {
                                let mut local_env = FxHashMap::default();
                                let self_sym = parser_tables.interner.intern("self");
                                local_env.insert("self".to_string(), ComptimeValue::Ident(self_sym));
                                if let Some(params) = arena.operator_params.get(&c_id.0).cloned() {
                                    for p_id in params {
                                        let p_node = *arena.node(p_id);
                                        if p_node.node_kind() == NodeKind::ParamDecl {
                                            let p_name = arena.interner.resolve(SymbolId(p_node.lhs)).to_string();
                                            let p_sym = parser_tables.interner.intern(&p_name);
                                            local_env.insert(p_name, ComptimeValue::Ident(p_sym));
                                        }
                                    }
                                }
                                let prev_depth = self.in_genesis_wrapper_depth;
                                self.in_genesis_wrapper_depth += 1;
                                let body_val = self.eval_expr_with_env(arena, raw_body, &mut local_env, parser_tables)?;
                                self.in_genesis_wrapper_depth = prev_depth;
                                if let ComptimeValue::SsaIntrinsic { opcode, .. } = body_val {
                                    let is_unary = arena
                                        .operator_params
                                        .get(&c_id.0)
                                        .map_or(true, |p| p.is_empty());
                                    if let Some(layout) = self.type_layouts.get_mut(&name_str) {
                                        if is_unary && layout.operators.contains_key(&op_str) {
                                            layout.operators.insert(format!("unary_{}", op_str), opcode);
                                        } else {
                                            layout.operators.insert(op_str, opcode);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(ComptimeValue::Void)
            }
            NodeKind::InterfaceDecl => {
                let name_str = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                self.node_to_type_name.insert(item_id.0, name_str);
                desugar_bracket_attributes(arena, item_id, parser_tables, self)?;
                Ok(ComptimeValue::Void)
            }
            NodeKind::BlockExpr => {
                // Check if this is a `pub syntax { ... }` block (`flags == 1`)
                let children: Vec<u32> = arena.children_slice(node.lhs, node.rhs).to_vec();
                if node.flags == 1 {
                    for c_raw in children {
                        let rule_fn_id = NodeId(c_raw);
                        let rule_node = *arena.node(rule_fn_id);
                        if rule_node.node_kind() == NodeKind::FnDecl {
                            let kw_name_str = arena.interner.resolve(SymbolId(rule_node.lhs)).to_string();
                            let kw_sym = parser_tables.interner.intern(&kw_name_str);
                            let _ = arena.interner.intern(&kw_name_str);
                            let slice: Vec<u32> = arena.children_slice(rule_node.rhs, rule_node.flags as u32).to_vec();
                            let body_node = slice.first().copied().map(NodeId).unwrap_or(NodeId::NONE);
                            let first_param = slice.get(1).copied().map(NodeId).unwrap_or(NodeId::NONE);

                            if let Some(&syntax_macro_id) = parser_tables.syntax_macros_by_name.get("syntax") {
                                let args = [
                                    ComptimeValue::Str(kw_sym),
                                    ComptimeValue::AstNode(first_param),
                                    ComptimeValue::AstNode(body_node),
                                ];
                                let res = self.invoke_function(
                                    syntax_macro_id,
                                    &args,
                                    arena,
                                    parser_tables,
                                    arena.span(rule_fn_id),
                                )?;
                                // Also register the rule directly with all its parameters (`sig, body`)
                                arena.comptime_fns.insert(rule_fn_id.0);
                                let registered_fn_id = self.register_comptime_fn(arena, rule_fn_id, parser_tables)?;
                                parser_tables.syntax_macros.insert(kw_sym, registered_fn_id);
                                parser_tables
                                    .syntax_macros_by_name
                                    .insert(kw_name_str, registered_fn_id);
                                let _ = res;
                            } else {
                                arena.comptime_fns.insert(rule_fn_id.0);
                                let registered_fn_id = self.register_comptime_fn(arena, rule_fn_id, parser_tables)?;
                                parser_tables.syntax_macros.insert(kw_sym, registered_fn_id);
                                parser_tables
                                    .syntax_macros_by_name
                                    .insert(kw_name_str, registered_fn_id);
                            }
                        }
                    }
                    return Ok(ComptimeValue::Void);
                }

                // Distributive block (`pub partial struct { ... }`, `pub lvl(...) comptime fn { ... }`)
                let mut last = ComptimeValue::Void;
                for c_raw in children {
                    last = self.eval_top_level_item(arena, NodeId(c_raw), parser_tables)?;
                }
                Ok(last)
            }
            _ => {
                desugar_bracket_attributes(arena, item_id, parser_tables, self)?;
                Ok(ComptimeValue::Void)
            }
        }
    }

    pub fn format_type_node(&self, arena: &AstArena, ty_node: NodeId) -> String {
        if ty_node.is_none() || (ty_node.0 as usize) >= arena.nodes.len() {
            return String::new();
        }
        let n = *arena.node(ty_node);
        let override_base = arena
            .type_base_symbols
            .get(&ty_node.0)
            .map(|&s| arena.interner.resolve(s).to_string());
        match n.node_kind() {
            NodeKind::IntLit => {
                arena
                    .i64_pool
                    .get(n.lhs as usize)
                    .copied()
                    .unwrap_or(0)
                    .to_string()
            }
            NodeKind::IdentExpr | NodeKind::TypeNamed => {
                arena.interner.resolve(SymbolId(n.lhs)).to_string()
            }
            NodeKind::TypeOptional => {
                let inner = self.format_type_node(arena, NodeId(n.lhs));
                let base = override_base
                    .or_else(|| self.type_construct_targets.get("?").cloned())
                    .unwrap_or_else(|| "option".to_string());
                format!("{}<{}>", base, inner)
            }
            NodeKind::TypeFallible => {
                let ok = self.format_type_node(arena, NodeId(n.lhs));
                let err = self.format_type_node(arena, NodeId(n.rhs));
                let base = override_base
                    .or_else(|| self.type_construct_targets.get("!").cloned())
                    .unwrap_or_else(|| "result".to_string());
                if err.is_empty() {
                    format!("{}<{}>", base, ok)
                } else {
                    format!("{}<{}, {}>", base, ok, err)
                }
            }
            NodeKind::TypeSlice => {
                if n.flags == 9 {
                    let children = arena.children_slice(n.lhs, n.rhs);
                    let elem = children
                        .first()
                        .map(|&c| self.format_type_node(arena, NodeId(c)))
                        .unwrap_or_default();
                    let n_str = children
                        .get(1)
                        .map(|&c| self.format_type_node(arena, NodeId(c)))
                        .unwrap_or_else(|| "1".to_string());
                    let m_str = children
                        .get(2)
                        .map(|&c| self.format_type_node(arena, NodeId(c)))
                        .unwrap_or_else(|| "1".to_string());
                    let base = override_base
                        .or_else(|| self.type_construct_targets.get("[N,M]").cloned())
                        .unwrap_or_else(|| "grid".to_string());
                    return format!("{}<{}, {}, {}>", base, elem, n_str, m_str);
                }
                let elem = self.format_type_node(arena, NodeId(n.lhs));
                let base = override_base.unwrap_or_else(|| match n.flags {
                    0 => self
                        .type_construct_targets
                        .get("[]")
                        .cloned()
                        .unwrap_or_else(|| "slice".to_string()),
                    1 => self
                        .type_construct_targets
                        .get("[N]")
                        .cloned()
                        .unwrap_or_else(|| "SliceBuffer".to_string()),
                    2 => "SliceView".to_string(),
                    3 => self
                        .type_construct_targets
                        .get("[..]")
                        .cloned()
                        .unwrap_or_else(|| "stride".to_string()),
                    4 => "View".to_string(),
                    5 => self
                        .type_construct_targets
                        .get("@")
                        .cloned()
                        .unwrap_or_else(|| "simd".to_string()),
                    6 => self
                        .type_construct_targets
                        .get("[:]")
                        .cloned()
                        .unwrap_or_else(|| "ring".to_string()),
                    7 => self
                        .type_construct_targets
                        .get("^")
                        .cloned()
                        .unwrap_or_else(|| "signal".to_string()),
                    8 => self
                        .type_construct_targets
                        .get("@prefix")
                        .cloned()
                        .unwrap_or_else(|| "pin".to_string()),
                    _ => "slice".to_string(),
                });
                if (n.flags == 1 || n.flags == 5) && n.rhs != 0 {
                    let count_str = self.format_type_node(arena, NodeId(n.rhs));
                    if !count_str.is_empty() {
                        return format!("{}<{}, {}>", base, elem, count_str);
                    }
                }
                format!("{}<{}>", base, elem)
            }
            NodeKind::TypeList => {
                let base = override_base.unwrap_or_else(|| "List".to_string());
                if n.rhs == 0 {
                    let elem = self.format_type_node(arena, NodeId(n.lhs));
                    format!("{}<{}>", base, elem)
                } else {
                    let args: Vec<String> = arena
                        .children_slice(n.lhs, n.rhs)
                        .iter()
                        .map(|&c| self.format_type_node(arena, NodeId(c)))
                        .collect();
                    format!("{}<{}>", base, args.join(", "))
                }
            }
            NodeKind::TypeMap => {
                let k = self.format_type_node(arena, NodeId(n.lhs));
                let v = self.format_type_node(arena, NodeId(n.rhs));
                let base = override_base.unwrap_or_else(|| "map".to_string());
                format!("{}<{}, {}>", base, k, v)
            }
            NodeKind::TypeTuple => {
                let args: Vec<String> = arena
                    .children_slice(n.lhs, n.rhs)
                    .iter()
                    .map(|&c| self.format_type_node(arena, NodeId(c)))
                    .collect();
                format!("tuple<{}>", args.join(", "))
            }
            _ => String::new(),
        }
    }

    fn estimate_type_node_size(&self, arena: &AstArena, ty_node: NodeId) -> usize {
        if ty_node.is_none() || (ty_node.0 as usize) >= arena.nodes.len() {
            return 8;
        }
        let formatted = self.format_type_node(arena, ty_node);
        if !formatted.is_empty() {
            let (sz, _, _) = self.query_type_size_align_lattice(&formatted);
            if sz > 0 {
                return sz;
            }
        }
        8
    }

    fn estimate_type_node_lattice(&self, arena: &AstArena, ty_node: NodeId) -> u8 {
        if ty_node.is_none() || (ty_node.0 as usize) >= arena.nodes.len() {
            return 0b00;
        }
        let formatted = self.format_type_node(arena, ty_node);
        if !formatted.is_empty() {
            return self.query_type_size_align_lattice(&formatted).2;
        }
        0b00
    }

    pub fn query_type_size_align_lattice(&self, tname: &str) -> (usize, usize, u8) {
        let trimmed = tname.trim();
        if let Some(lt_idx) = trimmed.find('<') {
            let base = trimmed[..lt_idx].trim();
            let args_part = trimmed[lt_idx + 1..trimmed.len().saturating_sub(1)].trim();
            let args: Vec<&str> = args_part.split(',').map(|s| s.trim()).collect();
            if let Some(spec) = self.type_layouts.get(base) {
                if spec.memory_layout == Some(MemoryLayoutMode::InlineArray) && !args.is_empty() {
                    let (elem_sz, elem_align, elem_lat) = self.query_type_size_align_lattice(args[0]);
                    let count: usize = args[1..]
                        .iter()
                        .filter_map(|s| s.parse::<usize>().ok())
                        .product::<usize>()
                        .max(1);
                    let total_sz = elem_sz * count;
                    let align = if matches!(spec.reg_class, RegisterClass::SimdVector { .. }) {
                        total_sz.next_power_of_two().min(64).max(elem_align)
                    } else {
                        elem_align.max(1)
                    };
                    return (total_sz, align, spec.lattice_bits | elem_lat);
                }
                if spec.has_explicit_register_layout {
                    return (spec.byte_size(), spec.byte_align(), spec.lattice_bits);
                }
                if !spec.field_types.is_empty() {
                    let mut subst_map: FxHashMap<&str, &str> = FxHashMap::default();
                    for (idx, param_name) in spec.type_params.iter().enumerate() {
                        if let Some(&arg_val) = args.get(idx) {
                            subst_map.insert(param_name.as_str(), arg_val);
                        }
                    }
                    let mut offset = 0usize;
                    let mut max_align = 1usize;
                    let mut lat = spec.lattice_bits;
                    for f_ty_expr in &spec.field_types {
                        let f_trimmed = f_ty_expr.trim();
                        let resolved_fty = if let Some(&mapped) = subst_map.get(f_trimmed) {
                            mapped.to_string()
                        } else if let Some(flt) = f_trimmed.find('<') {
                            let fbase = f_trimmed[..flt].trim();
                            let fargs: Vec<String> = f_trimmed[flt + 1..f_trimmed.len().saturating_sub(1)]
                                .split(',')
                                .map(|fa| {
                                    let fat = fa.trim();
                                    subst_map.get(fat).copied().unwrap_or(fat).to_string()
                                })
                                .collect();
                            format!("{}<{}>", fbase, fargs.join(", "))
                        } else {
                            f_trimmed.to_string()
                        };
                        let (fsz, fal, flat) = self.query_type_size_align_lattice(&resolved_fty);
                        let fal = fal.max(1);
                        offset = (offset + fal - 1) & !(fal - 1);
                        offset += fsz;
                        max_align = max_align.max(fal);
                        let f_base = resolved_fty.split('<').next().unwrap_or(&resolved_fty).trim();
                        let is_raw_ptr = self
                            .type_layouts
                            .get(f_base)
                            .map_or(false, |s| s.has_explicit_register_layout && s.reg_class == RegisterClass::RawPtr);
                        if !((spec.lattice_bits & 0b10) != 0 && is_raw_ptr) {
                            lat |= flat;
                        }
                    }
                    let total_sz = (offset + max_align - 1) & !(max_align - 1);
                    return (total_sz, max_align, lat);
                }
                return (spec.byte_size(), spec.byte_align(), spec.lattice_bits);
            }
            return self.query_type_size_align_lattice(base);
        }
        if let Some(spec) = self.type_layouts.get(trimmed) {
            return (spec.byte_size(), spec.byte_align(), spec.lattice_bits);
        }
        if let Some(&lat) = self.lattice_interfaces.get(trimmed) {
            return (0, 1, lat);
        }
        if trimmed.starts_with('*') || trimmed.ends_with('*') {
            return (8, 8, 0b00);
        }
        (8, 8, 0b00)
    }

    fn eval_comptime_stmt_or_block(
        &mut self,
        arena: &mut AstArena,
        node_id: NodeId,
        parser_tables: &mut ParserTables,
    ) -> Result<ComptimeValue, BootstrapDiagnostic> {
        if node_id.is_none() {
            return Ok(ComptimeValue::Void);
        }
        let node = *arena.node(node_id);
        match node.node_kind() {
            NodeKind::BlockExpr => {
                let children: Vec<u32> = arena.children_slice(node.lhs, node.rhs).to_vec();
                let mut last_val = ComptimeValue::Void;
                for c_raw in children {
                    last_val = self.eval_comptime_stmt_or_block(arena, NodeId(c_raw), parser_tables)?;
                }
                Ok(last_val)
            }
            NodeKind::LetStmt | NodeKind::VarStmt => {
                let var_sym = SymbolId(node.lhs);
                let var_name = arena.interner.resolve(var_sym).to_string();
                let tab_sym = parser_tables.interner.intern(&var_name);
                let val = self.eval_expr(arena, NodeId(node.rhs), parser_tables)?;
                self.globals.insert(tab_sym, val.clone());
                self.globals.insert(var_sym, val.clone());
                self.globals_by_name.insert(var_name, val.clone());
                Ok(val)
            }
            NodeKind::ReturnStmt => self.eval_expr(arena, NodeId(node.lhs), parser_tables),
            _ => self.eval_expr(arena, node_id, parser_tables),
        }
    }

    pub fn eval_expr(
        &mut self,
        arena: &mut AstArena,
        expr_id: NodeId,
        parser_tables: &mut ParserTables,
    ) -> Result<ComptimeValue, BootstrapDiagnostic> {
        let mut empty_env = FxHashMap::default();
        self.eval_expr_with_env(arena, expr_id, &mut empty_env, parser_tables)
    }

    pub fn eval_expr_with_env(
        &mut self,
        arena: &mut AstArena,
        expr_id: NodeId,
        local_env: &mut FxHashMap<String, ComptimeValue>,
        parser_tables: &mut ParserTables,
    ) -> Result<ComptimeValue, BootstrapDiagnostic> {
        if expr_id.is_none() {
            return Ok(ComptimeValue::Void);
        }
        let node = *arena.node(expr_id);
        let span = arena.span(expr_id);

        match node.node_kind() {
            NodeKind::IntLit => {
                let v = arena.i64_pool.get(node.lhs as usize).copied().unwrap_or(0);
                if v >= 0 {
                    Ok(ComptimeValue::Uint(v as u128))
                } else {
                    Ok(ComptimeValue::Int(v as i128))
                }
            }
            NodeKind::FloatLit => {
                let v = arena.f64_pool.get(node.lhs as usize).copied().unwrap_or(0.0);
                Ok(ComptimeValue::Float(v))
            }
            NodeKind::BoolLit => Ok(ComptimeValue::Bool(node.lhs != 0)),
            NodeKind::StrLit => {
                let s = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                let sym = parser_tables.interner.intern(&s);
                let _ = arena.interner.intern(&s);
                Ok(ComptimeValue::Str(sym))
            }
            NodeKind::IdentExpr => {
                let name = arena.interner.resolve(SymbolId(node.lhs)).to_string();
                if let Some(val) = local_env.get(&name) {
                    return Ok(val.clone());
                }
                if let Some(val) = self.globals_by_name.get(&name) {
                    return Ok(val.clone());
                }
                let sym = parser_tables.interner.intern(&name);
                let _ = arena.interner.intern(&name);
                Ok(ComptimeValue::Ident(sym))
            }
            NodeKind::MapExpr => {
                let children: Vec<u32> = arena.children_slice(node.lhs, node.rhs).to_vec();
                let mut bundle = FxHashMap::default();
                let mut idx = 0;
                while idx + 1 < children.len() {
                    let key_id = NodeId(children[idx]);
                    let val_id = NodeId(children[idx + 1]);
                    let key_node = *arena.node(key_id);
                    let key_str = match key_node.node_kind() {
                        NodeKind::IdentExpr | NodeKind::StrLit => {
                            arena.interner.resolve(SymbolId(key_node.lhs)).to_string()
                        }
                        _ => format!("field_{}", idx / 2),
                    };
                    let key_sym = parser_tables.interner.intern(&key_str);
                    let arena_sym = arena.interner.intern(&key_str);
                    let evaluated_val = self.eval_expr_with_env(arena, val_id, local_env, parser_tables)?;
                    bundle.insert(key_sym, evaluated_val.clone());
                    bundle.insert(arena_sym, evaluated_val);
                    idx += 2;
                }
                Ok(ComptimeValue::AstBundle(Arc::new(bundle)))
            }
            NodeKind::MemberExpr => {
                let base_val = self.eval_expr_with_env(arena, NodeId(node.lhs), local_env, parser_tables)?;
                let field_sym = SymbolId(node.rhs);
                let field_name = arena.interner.resolve(field_sym).to_string();
                if let Some(field_val) = base_val.get_field(field_sym) {
                    return Ok(field_val.clone());
                }
                if let Some(field_val) = base_val.get_field_by_name(&field_name, &arena.interner) {
                    return Ok(field_val.clone());
                }
                if let Some(field_val) = base_val.get_field_by_name(&field_name, &parser_tables.interner) {
                    return Ok(field_val.clone());
                }
                Ok(ComptimeValue::Void)
            }
            NodeKind::BlockExpr => {
                let children: Vec<u32> = arena.children_slice(node.lhs, node.rhs).to_vec();
                let mut last = ComptimeValue::Void;
                for c_raw in children {
                    let c_id = NodeId(c_raw);
                    let c_node = *arena.node(c_id);
                    match c_node.node_kind() {
                        NodeKind::LetStmt | NodeKind::VarStmt => {
                            let vname = arena.interner.resolve(SymbolId(c_node.lhs)).to_string();
                            let vval = self.eval_expr_with_env(arena, NodeId(c_node.rhs), local_env, parser_tables)?;
                            local_env.insert(vname, vval.clone());
                            last = vval;
                        }
                        NodeKind::ReturnStmt => {
                            return self.eval_expr_with_env(arena, NodeId(c_node.lhs), local_env, parser_tables);
                        }
                        NodeKind::ExprStmt => {
                            last = self.eval_expr_with_env(arena, NodeId(c_node.lhs), local_env, parser_tables)?;
                        }
                        _ => {
                            last = self.eval_expr_with_env(arena, c_id, local_env, parser_tables)?;
                        }
                    }
                }
                Ok(last)
            }
            NodeKind::ReturnStmt | NodeKind::ExprStmt => {
                self.eval_expr_with_env(arena, NodeId(node.lhs), local_env, parser_tables)
            }
            NodeKind::CallExpr => {
                let slice: Vec<u32> = arena.children_slice(node.lhs, node.rhs).to_vec();
                if slice.is_empty() {
                    return Ok(ComptimeValue::Void);
                }
                let callee_id = NodeId(slice[0]);
                let callee_node = *arena.node(callee_id);
                let callee_name = if callee_node.node_kind() == NodeKind::IdentExpr {
                    Some(arena.interner.resolve(SymbolId(callee_node.lhs)).to_string())
                } else {
                    None
                };

                // 1. Homoiconic `quote { ... }` block
                if callee_name.as_deref() == Some("quote") && slice.len() >= 2 {
                    let template_block = NodeId(slice[1]);
                    let mut target_arena = arena.clone();
                    let spliced = splice_quote_template(arena, template_block, local_env, &mut target_arena);
                    *arena = target_arena;
                    return Ok(ComptimeValue::AstNode(spliced));
                }

                // Evaluate arguments
                let mut arg_vals = Vec::with_capacity(slice.len() - 1);
                for &arg_raw in &slice[1..] {
                    let val = self.eval_expr_with_env(arena, NodeId(arg_raw), local_env, parser_tables)?;
                    arg_vals.push(val);
                }

                // 2. Direct `__engine(hook_name, args...)` intrinsic call
                if callee_name.as_deref() == Some("__engine") {
                    if arg_vals.is_empty() {
                        return Err(BootstrapDiagnostic::InvalidArguments {
                            hook: "__engine".to_string(),
                            details: "Missing hook name argument".to_string(),
                            span,
                        });
                    }
                    let hook_name = arg_vals[0]
                        .as_str(&parser_tables.interner)
                        .or_else(|| arg_vals[0].as_str(&arena.interner))
                        .unwrap_or("")
                        .to_string();
                    return self.eval_engine_intrinsic(&hook_name, &arg_vals[1..], arena, parser_tables, span);
                }

                // 3. Registered `comptime fn` or `syntax` macro call
                if let Some(ref cname) = callee_name {
                    if let Some(ComptimeValue::FnHandle(fn_id)) = local_env
                        .get(cname)
                        .or_else(|| self.globals_by_name.get(cname))
                        .cloned()
                    {
                        return self.invoke_function(fn_id, &arg_vals, arena, parser_tables, span);
                    }
                    if let Some(&fn_id) = parser_tables.syntax_macros_by_name.get(cname) {
                        return self.invoke_function(fn_id, &arg_vals, arena, parser_tables, span);
                    }
                }

                Ok(ComptimeValue::Void)
            }
            NodeKind::UnaryExpr => {
                let operand = self.eval_expr_with_env(arena, NodeId(node.lhs), local_env, parser_tables)?;
                match (node.flags, operand) {
                    (5, ComptimeValue::Bool(b)) => {
                        let ssa = self
                            .ssa_opcode_for_physics(RegisterClass::GprUnsigned, 1, "!")
                            .unwrap_or("xor");
                        if ssa == "xor" || ssa == "not" {
                            Ok(ComptimeValue::Bool(!b))
                        } else {
                            Ok(ComptimeValue::Bool(!b))
                        }
                    }
                    (4, ComptimeValue::Uint(v)) => Ok(ComptimeValue::Uint(!v)),
                    (0, ComptimeValue::Uint(v)) => Ok(ComptimeValue::Int(-(v as i128))),
                    (0, ComptimeValue::Int(v)) => Ok(ComptimeValue::Int(-v)),
                    _ => Ok(ComptimeValue::Void),
                }
            }
            NodeKind::BinaryExpr => {
                let lhs = self.eval_expr_with_env(arena, NodeId(node.lhs), local_env, parser_tables)?;
                let rhs = self.eval_expr_with_env(arena, NodeId(node.rhs), local_env, parser_tables)?;
                let op_sym = match node.flags {
                    1 => "+",
                    2 => "-",
                    3 => "*",
                    4 => "/",
                    6 => "%",
                    8 => "==",
                    9 => "!=",
                    10 => "<",
                    11 => "<=",
                    12 => ">",
                    13 => ">=",
                    16 => "&",
                    17 => "|",
                    18 => "^",
                    19 => "<<",
                    20 => ">>",
                    _ => "",
                };
                match (&lhs, &rhs) {
                    (ComptimeValue::Uint(a), ComptimeValue::Uint(b)) => {
                        let ssa = self
                            .ssa_opcode_for_physics(RegisterClass::GprUnsigned, 64, op_sym)
                            .unwrap_or("");
                        match ssa {
                            "add" => Ok(ComptimeValue::Uint(a.wrapping_add(*b))),
                            "sub" => Ok(ComptimeValue::Uint(a.wrapping_sub(*b))),
                            "mul" => Ok(ComptimeValue::Uint(a.wrapping_mul(*b))),
                            "udiv" => Ok(ComptimeValue::Uint(if *b != 0 { a / b } else { 0 })),
                            "urem" => Ok(ComptimeValue::Uint(if *b != 0 { a % b } else { 0 })),
                            "and" => Ok(ComptimeValue::Uint(a & b)),
                            "or" => Ok(ComptimeValue::Uint(a | b)),
                            "xor" => Ok(ComptimeValue::Uint(a ^ b)),
                            "shl" => Ok(ComptimeValue::Uint(a << (*b as u32))),
                            "lshr" => Ok(ComptimeValue::Uint(a >> (*b as u32))),
                            "eq" => Ok(ComptimeValue::Bool(a == b)),
                            "ne" => Ok(ComptimeValue::Bool(a != b)),
                            "ult" => Ok(ComptimeValue::Bool(a < b)),
                            "ule" => Ok(ComptimeValue::Bool(a <= b)),
                            "ugt" => Ok(ComptimeValue::Bool(a > b)),
                            "uge" => Ok(ComptimeValue::Bool(a >= b)),
                            _ => Ok(ComptimeValue::Void),
                        }
                    }
                    (ComptimeValue::Bool(a), ComptimeValue::Bool(b)) => {
                        let ssa = self
                            .ssa_opcode_for_physics(RegisterClass::GprUnsigned, 1, op_sym)
                            .unwrap_or("");
                        match ssa {
                            "and" => Ok(ComptimeValue::Bool(*a & *b)),
                            "or" => Ok(ComptimeValue::Bool(*a | *b)),
                            "xor" => Ok(ComptimeValue::Bool(*a ^ *b)),
                            "eq" => Ok(ComptimeValue::Bool(a == b)),
                            "ne" => Ok(ComptimeValue::Bool(a != b)),
                            _ => Ok(ComptimeValue::Void),
                        }
                    }
                    _ => Ok(ComptimeValue::Void),
                }
            }
            _ => Ok(ComptimeValue::Void),
        }
    }

    pub fn invoke_function(
        &mut self,
        fn_id: FunctionId,
        args: &[ComptimeValue],
        caller_arena: &mut AstArena,
        parser_tables: &mut ParserTables,
        call_span: ByteSpan,
    ) -> Result<ComptimeValue, BootstrapDiagnostic> {
        let def = match self.functions.get(fn_id.0 as usize) {
            Some(d) => d.clone(),
            None => {
                return Err(BootstrapDiagnostic::InvalidArguments {
                    hook: format!("fn#{}", fn_id.0),
                    details: "Invalid FunctionId".to_string(),
                    span: call_span,
                })
            }
        };

        let mut fn_env = FxHashMap::default();
        for (i, param_name) in def.params.iter().enumerate() {
            let arg_val = args.get(i).cloned().unwrap_or(ComptimeValue::Void);
            fn_env.insert(param_name.clone(), arg_val);
        }

        let mut exec_arena = (*def.def_arena).clone();
        // Sync interner strings from caller_arena so symbols resolve across arenas
        for i in 0..caller_arena.interner.symbol_count() {
            let s = caller_arena.interner.resolve(SymbolId(i as u32));
            exec_arena.interner.intern(s);
            parser_tables.interner.intern(s);
        }

        if def.is_genesis_wrapper {
            self.in_genesis_wrapper_depth += 1;
        }
        let result = self.eval_expr_with_env(&mut exec_arena, def.body_node, &mut fn_env, parser_tables);
        if def.is_genesis_wrapper {
            self.in_genesis_wrapper_depth -= 1;
        }
        let val = result?;

        // If the function returned a quoted AST block (like `__bootstrap_syntax`), evaluate its top-level items!
        if let ComptimeValue::AstNode(quoted_id) = val {
            if quoted_id.is_some() && (quoted_id.0 as usize) < exec_arena.nodes.len() {
                let qnode = *exec_arena.node(quoted_id);
                if qnode.node_kind() == NodeKind::BlockExpr {
                    let children: Vec<u32> = exec_arena.children_slice(qnode.lhs, qnode.rhs).to_vec();
                    for c_raw in children {
                        let _ = self.eval_top_level_item(&mut exec_arena, NodeId(c_raw), parser_tables)?;
                    }
                }
            }
        }

        for i in 0..exec_arena.interner.symbol_count() {
            let s = exec_arena.interner.resolve(SymbolId(i as u32));
            caller_arena.interner.intern(s);
            parser_tables.interner.intern(s);
        }

        Ok(val)
    }

    // =========================================================================
    // SECTION 3.5: THE 30 `__engine(...)` HOOKS & SECURITY SEAL GATE
    // =========================================================================

    pub fn eval_engine_intrinsic(
        &mut self,
        hook_name: &str,
        args: &[ComptimeValue],
        arena: &mut AstArena,
        parser_tables: &mut ParserTables,
        span: ByteSpan,
    ) -> Result<ComptimeValue, BootstrapDiagnostic> {
        // SECURITY GATE (First Line): Once `seal_bootstrap_bridge` has been called at the end of
        // `stdlib/core/core.pilot`, any direct `__engine(...)` call in user or downstream files
        // is rejected with `BootstrapBridgeSealedError`!
        if parser_tables.bootstrap_sealed && self.in_genesis_wrapper_depth == 0 {
            let err = BootstrapDiagnostic::BootstrapBridgeSealedError {
                hook: hook_name.to_string(),
                span,
            };
            self.diagnostics.push(err.clone());
            return Err(err);
        }

        match hook_name {
            // 1. `register_bracket_attribute`: `(attr_name: Str, fn_handle: FnHandle)`
            "register_bracket_attribute" => {
                let attr_name = args
                    .first()
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("")
                    .to_string();
                let fn_id = match args.get(1) {
                    Some(ComptimeValue::FnHandle(id)) => *id,
                    _ => FunctionId(0),
                };
                let sym = parser_tables.interner.intern(&attr_name);
                let arena_sym = arena.interner.intern(&attr_name);
                parser_tables.bracket_attributes.insert(sym, fn_id);
                parser_tables.bracket_attributes.insert(arena_sym, fn_id);
                parser_tables.bracket_attributes_by_name.insert(attr_name, fn_id);
                Ok(ComptimeValue::Void)
            }

            // 2. `bind_lang_item` / `bind_receptor_slot`: `(target_decl: AstNode, item_or_slot_name: Str)`
            "bind_lang_item" | "bind_receptor_slot" => {
                let target_node = match args.first() {
                    Some(ComptimeValue::AstNode(nid)) => *nid,
                    _ => NodeId::NONE,
                };
                let item_name = args
                    .get(1)
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("")
                    .to_string();
                if let Some(lang_item) = LangItem::from_name(&item_name) {
                    self.lang_items.insert(lang_item, target_node);
                }
                if let Some(slot) = ReceptorSlot::from_slot_or_lang_name(&item_name) {
                    self.receptor_slots.insert(slot, target_node);
                    let actual_type_name = self.resolve_type_name_from_node(arena, target_node);
                    if !actual_type_name.is_empty() {
                        self.receptor_type_names.insert(slot, actual_type_name);
                    }
                }
                self.lang_items_by_name.insert(item_name.clone(), target_node);
                if target_node.is_some() {
                    self.node_to_type_name.entry(target_node.0).or_insert(item_name);
                }
                Ok(ComptimeValue::Void)
            }

            // 3. `set_register_layout`: `(target_type: AstNode, bit_width: Uint, reg_class: Ident)`
            "set_register_layout" => {
                let target_node = match args.first() {
                    Some(ComptimeValue::AstNode(nid)) => *nid,
                    _ => NodeId::NONE,
                };
                let bit_width = args.get(1).and_then(|v| v.as_uint()).unwrap_or(0) as u32;
                let reg_class = if let Some(reg_name) = args
                    .get(2)
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                {
                    RegisterClass::from_name(reg_name).unwrap_or(RegisterClass::GprUnsigned)
                } else if let Some(code) = args.get(2).and_then(|v| v.as_uint()) {
                    match code {
                        0 => RegisterClass::Unit,
                        1 => RegisterClass::NoReturn,
                        2 => RegisterClass::GprSigned,
                        3 => RegisterClass::GprUnsigned,
                        4 => RegisterClass::SimdFloat,
                        6 => RegisterClass::RawPtr,
                        _ => RegisterClass::GprUnsigned,
                    }
                } else {
                    RegisterClass::GprUnsigned
                };

                let type_name = self.resolve_type_name_from_node(arena, target_node);
                let spec = self
                    .type_layouts
                    .entry(type_name.clone())
                    .or_insert_with(|| TypeLayoutSpec::new(type_name, target_node));
                spec.bit_width = bit_width;
                spec.reg_class = reg_class;
                spec.has_explicit_register_layout = true;
                if reg_class == RegisterClass::RawPtr {
                    spec.lattice_bits |= 0b01;
                }
                Ok(ComptimeValue::Void)
            }

            // 4. `set_memory_layout`: `(target_type: AstNode, mode: Ident)`
            "set_memory_layout" => {
                let target_node = match args.first() {
                    Some(ComptimeValue::AstNode(nid)) => *nid,
                    _ => NodeId::NONE,
                };
                let mode_name = args
                    .get(1)
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("c_abi");
                let mode = MemoryLayoutMode::from_name(mode_name).unwrap_or(MemoryLayoutMode::CAbi);
                let type_name = self.resolve_type_name_from_node(arena, target_node);
                let spec = self
                    .type_layouts
                    .entry(type_name.clone())
                    .or_insert_with(|| TypeLayoutSpec::new(type_name, target_node));
                spec.memory_layout = Some(mode);
                Ok(ComptimeValue::Void)
            }

            // 5. `register_pratt_op`: `(op_sym: Str, fixity: Ident, prec: Uint, assoc: Ident)`
            "register_pratt_op" => {
                let op_str = args
                    .first()
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("")
                    .to_string();
                let fixity_str = args
                    .get(1)
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("infix");
                let prec = args.get(2).and_then(|v| v.as_uint()).unwrap_or(50) as u8;
                let assoc_str = args
                    .get(3)
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("left");

                let fixity = match fixity_str {
                    "prefix" => Fixity::Prefix,
                    "postfix" => Fixity::Postfix,
                    _ => Fixity::Infix,
                };
                let assoc = match assoc_str {
                    "right" => Assoc::Right,
                    "none" => Assoc::None,
                    _ => Assoc::Left,
                };
                let sym = parser_tables.interner.intern(&op_str);
                let arena_sym = arena.interner.intern(&op_str);
                let spec = PrattOpSpec {
                    op_sym: sym,
                    fixity,
                    precedence: prec,
                    assoc,
                    home_pkg: None,
                };
                match fixity {
                    Fixity::Infix => {
                        parser_tables.pratt_infix.insert(sym, spec);
                        parser_tables.pratt_infix.insert(arena_sym, spec);
                        parser_tables.pratt_infix_by_name.insert(op_str, spec);
                    }
                    Fixity::Prefix => {
                        parser_tables.pratt_prefix.insert(sym, spec);
                        parser_tables.pratt_prefix.insert(arena_sym, spec);
                        parser_tables.pratt_prefix_by_name.insert(op_str, spec);
                    }
                    Fixity::Postfix => {
                        parser_tables.pratt_postfix.insert(sym, spec);
                        parser_tables.pratt_postfix.insert(arena_sym, spec);
                        parser_tables.pratt_postfix_by_name.insert(op_str, spec);
                    }
                }
                Ok(ComptimeValue::Void)
            }

            // 6. `set_inline_hint`: `(target_fn: AstNode, mode: Ident)`
            "set_inline_hint" => {
                let target_node = match args.first() {
                    Some(ComptimeValue::AstNode(nid)) => *nid,
                    _ => NodeId::NONE,
                };
                let mode_str = args
                    .get(1)
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("auto");
                let hint = InlineHint::from_name(mode_str).unwrap_or(InlineHint::Auto);
                if target_node.is_some() {
                    self.inline_hints.insert(target_node.0, hint);
                }
                Ok(ComptimeValue::Void)
            }

            // 7. `register_syntax_keyword`: `(kw_token: Str, transformer_fn: FnHandle)`
            "register_syntax_keyword" => {
                let kw_name = args
                    .first()
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("")
                    .to_string();
                let fn_id = match args.get(1) {
                    Some(ComptimeValue::FnHandle(id)) => *id,
                    _ => FunctionId(0),
                };
                let sym = parser_tables.interner.intern(&kw_name);
                let arena_sym = arena.interner.intern(&kw_name);
                parser_tables.syntax_macros.insert(sym, fn_id);
                parser_tables.syntax_macros.insert(arena_sym, fn_id);
                parser_tables.syntax_macros_by_name.insert(kw_name, fn_id);
                Ok(ComptimeValue::Void)
            }

            // 8. `register_modifier`: `(mod_token: Str, semantic_action: Uint | Str)`
            "register_modifier" => {
                let mod_name = args
                    .first()
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("")
                    .to_string();
                let spec = if let Some(vis_ring) = args.get(1).and_then(|v| v.as_uint()) {
                    ModifierSpec::VisibilityRing(vis_ring as u8)
                } else {
                    let act_str = args
                        .get(1)
                        .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                        .unwrap_or("")
                        .to_string();
                    ModifierSpec::SemanticFlag(act_str)
                };
                let sym = parser_tables.interner.intern(&mod_name);
                let arena_sym = arena.interner.intern(&mod_name);
                parser_tables.modifiers.insert(sym, spec.clone());
                parser_tables.modifiers.insert(arena_sym, spec.clone());
                parser_tables.modifiers_by_name.insert(mod_name, spec);
                Ok(ComptimeValue::Void)
            }

            // 9. `register_parametric_modifier`: `(mod_token: Str, action: Str)`
            "register_parametric_modifier" => {
                let mod_name = args
                    .first()
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("")
                    .to_string();
                let act_name = args
                    .get(1)
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("")
                    .to_string();
                let mod_sym = parser_tables.interner.intern(&mod_name);
                let act_sym = parser_tables.interner.intern(&act_name);
                parser_tables.param_modifiers.insert(mod_sym, act_sym);
                parser_tables.param_modifiers_by_name.insert(mod_name, act_name);
                Ok(ComptimeValue::Void)
            }

            // 10. `register_module_kw`: `(kw_token: Str, action: Str)`
            "register_module_kw" => {
                let kw_name = args
                    .first()
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("")
                    .to_string();
                let act_name = args
                    .get(1)
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("");
                let kind = if act_name == "declare_package_scope" || kw_name == "pkg" {
                    ModuleKwKind::DeclarePkg
                } else {
                    ModuleKwKind::ImportPath
                };
                let sym = parser_tables.interner.intern(&kw_name);
                parser_tables.module_keywords.insert(sym, kind);
                parser_tables.module_keywords_by_name.insert(kw_name, kind);
                Ok(ComptimeValue::Void)
            }

            // 11. `bind_receiver_ident`: `(ident_str: Str)`
            "bind_receiver_ident" => {
                let recv_str = args
                    .first()
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("self")
                    .to_string();
                let sym = parser_tables.interner.intern(&recv_str);
                parser_tables.receiver_ident = Some(sym);
                parser_tables.receiver_ident_name = Some(recv_str);
                Ok(ComptimeValue::Void)
            }

            // 12. `cfg_jump_loop_exit`: `()`
            "cfg_jump_loop_exit" => Ok(ComptimeValue::CfgJumpLoopExit),

            // 13. `cfg_jump_loop_head`: `()`
            "cfg_jump_loop_head" => Ok(ComptimeValue::CfgJumpLoopHead),

            // 14. `decl_operator_fn`: `(sig: AstNode, body: AstNode)`
            "decl_operator_fn" => {
                let sig_node = match args.first() {
                    Some(ComptimeValue::AstNode(nid)) => *nid,
                    _ => NodeId::NONE,
                };
                Ok(ComptimeValue::AstNode(sig_node))
            }

            // 15. `emit_ssa_binop`: `(opcode: Str, lhs: AstNode, rhs: AstNode)`
            "emit_ssa_binop" => {
                let opcode = args
                    .first()
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("")
                    .to_string();
                let lhs = args.get(1).cloned().unwrap_or(ComptimeValue::Void);
                let rhs = args.get(2).cloned().unwrap_or(ComptimeValue::Void);
                Ok(ComptimeValue::SsaIntrinsic {
                    opcode,
                    args: vec![lhs, rhs],
                    target_type: None,
                })
            }

            // 16. `emit_ssa_unop`: `(opcode: Str, val: AstNode)`
            "emit_ssa_unop" => {
                let opcode = args
                    .first()
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("")
                    .to_string();
                let val = args.get(1).cloned().unwrap_or(ComptimeValue::Void);
                Ok(ComptimeValue::SsaIntrinsic {
                    opcode,
                    args: vec![val],
                    target_type: None,
                })
            }

            // 17. `emit_ssa_cast`: `(opcode: Str, val: AstNode, target_t: AstNode)`
            "emit_ssa_cast" => {
                let opcode = args
                    .first()
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("")
                    .to_string();
                let val = args.get(1).cloned().unwrap_or(ComptimeValue::Void);
                let target_t = args
                    .get(2)
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .map(|s| s.to_string());
                Ok(ComptimeValue::SsaIntrinsic {
                    opcode,
                    args: vec![val],
                    target_type: target_t,
                })
            }

            // 18. `emit_ssa_load`: `(ptr_val: AstNode)`
            "emit_ssa_load" => {
                let ptr_val = args.first().cloned().unwrap_or(ComptimeValue::Void);
                Ok(ComptimeValue::SsaIntrinsic {
                    opcode: "load".to_string(),
                    args: vec![ptr_val],
                    target_type: None,
                })
            }

            // 19. `emit_ssa_store`: `(ptr_val: AstNode, val: AstNode)`
            "emit_ssa_store" => {
                let ptr_val = args.first().cloned().unwrap_or(ComptimeValue::Void);
                let val = args.get(1).cloned().unwrap_or(ComptimeValue::Void);
                Ok(ComptimeValue::SsaIntrinsic {
                    opcode: "store".to_string(),
                    args: vec![ptr_val, val],
                    target_type: None,
                })
            }

            // 20. `emit_ssa_gep`: `(base_ptr: AstNode, elem_idx: AstNode)`
            "emit_ssa_gep" => {
                let base_ptr = args.first().cloned().unwrap_or(ComptimeValue::Void);
                let elem_idx = args.get(1).cloned().unwrap_or(ComptimeValue::Void);
                Ok(ComptimeValue::SsaIntrinsic {
                    opcode: "gep".to_string(),
                    args: vec![base_ptr, elem_idx],
                    target_type: None,
                })
            }

            // 21. `query_is_comptime_vm`: `()`
            "query_is_comptime_vm" => Ok(ComptimeValue::Bool(true)),

            // 22. `type_sizeof`: `(target_t: AstNode | Ident | Str)`
            "type_sizeof" => {
                let tname = self.resolve_type_arg(args.first(), arena, parser_tables);
                let (bytes, _, _) = self.query_type_size_align_lattice(&tname);
                Ok(ComptimeValue::Uint(bytes as u128))
            }

            // 23. `type_alignof`: `(target_t: AstNode | Ident | Str)`
            "type_alignof" => {
                let tname = self.resolve_type_arg(args.first(), arena, parser_tables);
                let (_, align, _) = self.query_type_size_align_lattice(&tname);
                Ok(ComptimeValue::Uint(align as u128))
            }

            // 24. `type_offsetof`: `(target_t: AstNode, field_sym: Ident | Str)`
            "type_offsetof" => {
                let tname = self.resolve_type_arg(args.first(), arena, parser_tables);
                let fname = args
                    .get(1)
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("");
                let offset = self
                    .type_layouts
                    .get(&tname)
                    .and_then(|spec| spec.fields.iter().find(|(n, _, _)| n == fname).map(|(_, off, _)| *off))
                    .unwrap_or(0);
                Ok(ComptimeValue::Uint(offset as u128))
            }

            // 25. `type_fields_of`: `(target_t: AstNode)`
            "type_fields_of" => {
                let tname = self.resolve_type_arg(args.first(), arena, parser_tables);
                let fields = self
                    .type_layouts
                    .get(&tname)
                    .map(|spec| {
                        spec.fields
                            .iter()
                            .map(|(fname, _, _)| ComptimeValue::Ident(parser_tables.interner.intern(fname)))
                            .collect()
                    })
                    .unwrap_or_default();
                Ok(ComptimeValue::Tuple(fields))
            }

            // 26. `emit_diagnostic_error`: `(msg: Str)`
            "emit_diagnostic_error" => {
                let msg = args
                    .first()
                    .and_then(|v| v.as_str(&parser_tables.interner).or_else(|| v.as_str(&arena.interner)))
                    .unwrap_or("compile_error invoked")
                    .to_string();
                let err = BootstrapDiagnostic::UserCompileError { message: msg, span };
                self.diagnostics.push(err.clone());
                Err(err)
            }

            // 27. `bind_lattice_kind`: `(target_iface: AstNode, mask: Uint)`
            "bind_lattice_kind" => {
                let target_node = match args.first() {
                    Some(ComptimeValue::AstNode(nid)) => *nid,
                    _ => NodeId::NONE,
                };
                let mask = (args.get(1).and_then(|v| v.as_uint()).unwrap_or(0) & 0b11) as u8;
                let tname = self.resolve_type_name_from_node(arena, target_node);
                self.lattice_interfaces.insert(tname.clone(), mask);
                if let Some(spec) = self.type_layouts.get_mut(&tname) {
                    spec.lattice_bits = mask;
                }
                Ok(ComptimeValue::Void)
            }

            // 28. `get_lattice_bits`: `(target_t: AstNode | Ident | Str)`
            "get_lattice_bits" => {
                let tname = self.resolve_type_arg(args.first(), arena, parser_tables);
                let bits = self
                    .lattice_interfaces
                    .get(&tname)
                    .copied()
                    .unwrap_or_else(|| self.query_type_size_align_lattice(&tname).2);
                Ok(ComptimeValue::Uint(bits as u128))
            }

            // 29. `enforce_core_naming_lint`: `(target_pkg: AstNode)`
            "enforce_core_naming_lint" => {
                let pkg_name = if let Some(ref active) = self.active_pkg {
                    active.clone()
                } else {
                    match args.first() {
                        Some(ComptimeValue::AstNode(nid)) if arena.pkg_decls.contains(&nid.0) => {
                            let pkg_node = *arena.node(*nid);
                            arena.interner.resolve(SymbolId(pkg_node.lhs)).to_string()
                        }
                        _ => "Pilot.Core".to_string(),
                    }
                };
                if !self.core_naming_enforced_pkgs.contains(&pkg_name) {
                    self.core_naming_enforced_pkgs.push(pkg_name);
                }
                Ok(ComptimeValue::Void)
            }

            // 30. `seal_bootstrap_bridge`: `()`
            "seal_bootstrap_bridge" => {
                parser_tables.bootstrap_sealed = true;
                Ok(ComptimeValue::Void)
            }

            other => {
                let err = BootstrapDiagnostic::UnknownEngineHook {
                    hook: other.to_string(),
                    span,
                };
                self.diagnostics.push(err.clone());
                Err(err)
            }
        }
    }

    fn resolve_type_name_from_node(&self, arena: &AstArena, node_id: NodeId) -> String {
        if let Some(name) = self.node_to_type_name.get(&node_id.0) {
            return name.clone();
        }
        if node_id.is_some() && (node_id.0 as usize) < arena.nodes.len() {
            let n = *arena.node(node_id);
            if matches!(
                n.node_kind(),
                NodeKind::StructDecl | NodeKind::InterfaceDecl | NodeKind::IdentExpr
            ) {
                return arena.interner.resolve(SymbolId(n.lhs)).to_string();
            }
        }
        String::new()
    }

    fn resolve_type_arg(
        &self,
        arg: Option<&ComptimeValue>,
        arena: &AstArena,
        parser_tables: &ParserTables,
    ) -> String {
        match arg {
            Some(ComptimeValue::AstNode(nid)) => self.resolve_type_name_from_node(arena, *nid),
            Some(ComptimeValue::Ident(sym) | ComptimeValue::Str(sym)) => {
                let s1 = parser_tables.interner.resolve(*sym);
                if !s1.is_empty() {
                    s1.to_string()
                } else {
                    arena.interner.resolve(*sym).to_string()
                }
            }
            _ => String::new(),
        }
    }
}

/// Top-Level Item Interleaved Bootstrap Loop (`Section 3.1`):
/// Parses `stdlib/core/core.pilot` one top-level item at a time using a live `ParserTables`,
/// immediately executing each item in `comptime_vm` so newly registered modifiers, `[...]` bracket
/// attributes, `syntax` macros, and Pratt operator precedences take effect on the very next token!
pub fn bootstrap_core_file(
    source: &str,
    parser_tables: &mut ParserTables,
    comptime_vm: &mut ComptimeEngine,
) -> Result<AstArena, BootstrapDiagnostic> {
    let tape = tokenize(source);
    let mut parser = Parser::with_tables(source, tape, parser_tables.clone());

    while let Some(item_id) = parser.parse_next_top_level_item(parser_tables) {
        comptime_vm.eval_top_level_item(&mut parser.arena, item_id, parser_tables)?;
    }

    if !parser.errors.is_empty() {
        return Err(BootstrapDiagnostic::ParseError {
            errors: parser.errors.clone(),
        });
    }

    Ok(parser.finish_program())
}

/// Evaluates a downstream Pilot file against an already-bootstrapped (and sealed) `ParserTables`
/// and `ComptimeEngine`.
pub fn eval_file_with_tables(
    source: &str,
    parser_tables: &mut ParserTables,
    comptime_vm: &mut ComptimeEngine,
) -> Result<AstArena, BootstrapDiagnostic> {
    bootstrap_core_file(source, parser_tables, comptime_vm)
}

// =========================================================================
// PRESERVED MILESTONE 33.3: CALL-QUOTING PROJECTIONS & STATIC DEMOTION PASS
// =========================================================================

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum QuoteKind {
    /// `!expr`: Shallow quote (unevaluated AST node)
    Shallow,
    /// `!!expr`: Deep splice with hygienic constant folding
    DeepSplice,
    /// `!&fn`: C-ABI call frame layout synthesis
    CallFrameLayout,
    /// `!^lval`: Observable LValue extraction
    ObservableLValue,
    /// `!@expr`: Symbolic differentiation + Common Subexpression Elimination (CSE)
    SymbolicDiff,
    /// `!?call`: Copy-on-Write dry run
    CoWDryRun,
}

#[derive(Clone, Debug)]
pub struct CallFrameLayoutInfo {
    pub fn_name: String,
    pub total_size_bytes: usize,
    pub param_offsets: Vec<(String, usize, usize)>, // (name, offset, size)
}

#[derive(Clone, Debug)]
pub struct StaticDemotedSlot {
    pub var_name: String,
    pub frame_depth: u32,
    pub slot_index: u32,
    pub is_read_only: bool,
}

pub struct ComptimeEvaluator {
    pub diagnostics: Vec<AnalysisDiagnostic>,
    pub demoted_slots: Vec<StaticDemotedSlot>,
}

impl Default for ComptimeEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl ComptimeEvaluator {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            demoted_slots: Vec::new(),
        }
    }

    pub fn compute_call_frame_layout(fn_name: &str, params: &[(&str, usize)]) -> CallFrameLayoutInfo {
        let mut offset = 0;
        let mut param_offsets = Vec::new();
        for (name, size) in params {
            // Align to 8 bytes
            let aligned = (offset + 7) & !7;
            param_offsets.push((name.to_string(), aligned, *size));
            offset = aligned + size;
        }
        let total_size_bytes = (offset + 7) & !7;
        CallFrameLayoutInfo {
            fn_name: fn_name.to_string(),
            total_size_bytes,
            param_offsets,
        }
    }

    pub fn report_comptime_side_effect(&mut self, operation: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "COMPTIME001",
            format!("Side-effect '{}' is illegal in comptime execution context", operation),
            span,
        ));
    }

    pub fn report_dynamic_comptime_dependency(&mut self, var_name: &str, span: ByteSpan) {
        self.diagnostics.push(AnalysisDiagnostic::error(
            "COMPTIME002",
            format!("Comptime evaluation depends on runtime variable '{}' which cannot be known at compile time", var_name),
            span,
        ));
    }

    pub fn record_demoted_slot(&mut self, var_name: &str, frame_depth: u32, slot_index: u32, is_read_only: bool) {
        self.demoted_slots.push(StaticDemotedSlot {
            var_name: var_name.to_string(),
            frame_depth,
            slot_index,
            is_read_only,
        });
    }

    /// Performs the Milestone 19 compiler-wide static demotion pass across `AstArena`.
    pub fn demote_arena(&mut self, arena: &pilot_ast::AstArena, root: pilot_ast::NodeId) {
        if root.is_none() {
            return;
        }

        let mut current_depth: u32 = 0;
        let mut slot_counter: u32 = 0;

        for i in 0..arena.nodes.len() {
            let node_id = pilot_ast::NodeId(i as u32);
            let node = *arena.node(node_id);
            let span = arena.span(node_id);

            match node.node_kind() {
                pilot_ast::NodeKind::FnDecl => {
                    current_depth += 1;
                    slot_counter = 0;
                }
                pilot_ast::NodeKind::LetStmt => {
                    let sym = pilot_intern::SymbolId(node.lhs);
                    let var_name = arena.interner.resolve(sym);
                    self.record_demoted_slot(var_name, current_depth, slot_counter, true);
                    slot_counter += 1;
                }
                pilot_ast::NodeKind::VarStmt => {
                    let sym = pilot_intern::SymbolId(node.lhs);
                    let var_name = arena.interner.resolve(sym);
                    self.record_demoted_slot(var_name, current_depth, slot_counter, false);
                    slot_counter += 1;
                }
                pilot_ast::NodeKind::ParamDecl => {
                    let sym = pilot_intern::SymbolId(node.lhs);
                    let var_name = arena.interner.resolve(sym);
                    self.record_demoted_slot(var_name, current_depth, slot_counter, true);
                    slot_counter += 1;
                }
                pilot_ast::NodeKind::ComptimeExpr => {
                    // Check if comptime block contains illegal dynamic constructs
                    let inner_id = pilot_ast::NodeId(node.lhs);
                    if inner_id.is_some() {
                        let inner_node = *arena.node(inner_id);
                        if inner_node.node_kind() == pilot_ast::NodeKind::CallExpr {
                            let callee = *arena.node(pilot_ast::NodeId(inner_node.lhs));
                            if callee.node_kind() == pilot_ast::NodeKind::IdentExpr {
                                let cname = arena.interner.resolve(pilot_intern::SymbolId(callee.lhs));
                                if cname == "print" || cname == "write" || cname == "malloc" {
                                    self.report_comptime_side_effect(cname, span);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Recursively flattens nested list/tuple literals in `arena` into a 1D vector of leaf node IDs.
    pub fn flatten_literal_to_1d(arena: &pilot_ast::AstArena, node_id: pilot_ast::NodeId) -> Vec<pilot_ast::NodeId> {
        let mut leaves = Vec::new();
        Self::collect_leaves_rec(arena, node_id, &mut leaves);
        leaves
    }

    fn collect_leaves_rec(arena: &pilot_ast::AstArena, node_id: pilot_ast::NodeId, out: &mut Vec<pilot_ast::NodeId>) {
        if node_id.is_none() {
            return;
        }
        let node = *arena.node(node_id);
        match node.node_kind() {
            pilot_ast::NodeKind::Program => {
                let children = arena.children_slice(node.lhs, node.rhs);
                for &c in children {
                    Self::collect_leaves_rec(arena, pilot_ast::NodeId(c), out);
                }
            }
            pilot_ast::NodeKind::LetStmt | pilot_ast::NodeKind::VarStmt => {
                Self::collect_leaves_rec(arena, pilot_ast::NodeId(node.rhs), out);
            }
            pilot_ast::NodeKind::ListExpr | pilot_ast::NodeKind::TupleExpr => {
                let children = arena.children_slice(node.lhs, node.rhs);
                for &c in children {
                    Self::collect_leaves_rec(arena, pilot_ast::NodeId(c), out);
                }
            }
            pilot_ast::NodeKind::UnaryExpr if node.flags == pilot_ast::UNARY_OP_DEEP_FLATTEN => {
                Self::collect_leaves_rec(arena, pilot_ast::NodeId(node.lhs), out);
            }
            _ => {
                out.push(node_id);
            }
        }
    }
}
