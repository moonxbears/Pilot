//! Milestone 33.1 & Self-Hosting Phase 3: Data-Oriented `TypeArena`, 2-Bit Hardware Type Lattice,
//! `TypeDef` Layout Sealing, `InterfaceDef`, `BundleDef`, and O(1) 4-Tier `MethodTable` Solver.

use std::collections::{HashMap, HashSet};
use pilot_intern::{Interner, SymbolId};
pub use pilot_parser::{FunctionId, FxHashMap, PackageId};
use pilot_span::ByteSpan;
use crate::comptime::LangItem;
use crate::diagnostics::AnalysisDiagnostic;

pub type FxHashSet<T> = HashSet<T>;
pub type Span = ByteSpan;

// =========================================================================
// PHASE 3: 2-BIT HARDWARE TYPE LATTICE (`0b00..0b11`)
// =========================================================================

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum LatticeBits {
    #[default]
    Value     = 0b00, // Bit 1 (Drop) = 0 | Bit 0 (Borrow) = 0 -> `IValue` (memcpy safe, `as view` safe)
    Reference = 0b01, // Bit 1 (Drop) = 0 | Bit 0 (Borrow) = 1 -> `IReference` (lexical borrow lifetime)
    Object    = 0b10, // Bit 1 (Drop) = 1 | Bit 0 (Borrow) = 0 -> `IObject` (Perceus move + `~self()` drop)
    Guard     = 0b11, // Bit 1 (Drop) = 1 | Bit 0 (Borrow) = 1 -> `IGuard` (Perceus move + `~self()` + borrow)
}

impl LatticeBits {
    #[inline(always)]
    pub const fn from_raw(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => Self::Value,
            0b01 => Self::Reference,
            0b10 => Self::Object,
            _    => Self::Guard,
        }
    }

    #[inline(always)]
    pub const fn raw(self) -> u8 {
        self as u8
    }

    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self::from_raw((self as u8) | (other as u8))
    }

    /// Bit 0 (`0b01`): True for `IReference` (`0b01`) and `IGuard` (`0b11`).
    /// Must be tracked by `pilot-perceus/src/escape.rs` so it never outlives its borrowed source.
    #[inline(always)]
    pub const fn has_borrow(self) -> bool {
        (self as u8 & 0b01) != 0
    }

    /// Bit 1 (`0b10`): True for `IObject` (`0b10`) and `IGuard` (`0b11`).
    /// Must be moved by default and have `~self()` emitted by `pilot-perceus/src/arc.rs`.
    #[inline(always)]
    pub const fn has_drop(self) -> bool {
        (self as u8 & 0b10) != 0
    }

    /// True ONLY for `IValue` (`0b00`).
    /// Required for zero-copy `as view` (`OffsetLens`) reinterpretation and raw `memcpy`.
    #[inline(always)]
    pub const fn is_pure_value(self) -> bool {
        (self as u8) == 0b00
    }

    /// Safe to send across `.pchor` cues or threads if it has no borrowed pointers (`Bit 0 == 0`),
    /// AND either is an `IValue` (`0b00`) or an `iso`-qualified `IObject` (`0b10`).
    #[inline(always)]
    pub const fn can_cross_cue_boundary(self, is_iso_qualified: bool) -> bool {
        !self.has_borrow() && (self.is_pure_value() || is_iso_qualified)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OwnershipAction {
    /// `0b00` (`IValue`): Trivial bitwise register copy; no Perceus RC/drop tracking needed.
    TrivialBitCopy,
    /// `0b01` (`IReference`): Bitwise copy with lexical borrow-escape check in `escape.rs`.
    LexicalBorrowCheckOnly,
    /// `0b10` (`IObject`): Move-by-default; Perceus `arc.rs` inserts deterministic `~self()` or RC drop at scope exit;
    /// may cross `.pchor` cues if `iso`-qualified.
    MoveAndDropAtScopeExit,
    /// `0b11` (`IGuard`): Move-by-default + lexical borrow-escape check + deterministic `~self()` at scope exit;
    /// strictly forbidden from crossing `.pchor` cues.
    ScopedGuardMoveAndDrop,
}

#[inline(always)]
pub const fn classify_perceus_action(lattice: u8) -> OwnershipAction {
    match lattice & 0b11 {
        0b00 => OwnershipAction::TrivialBitCopy,
        0b01 => OwnershipAction::LexicalBorrowCheckOnly,
        0b10 => OwnershipAction::MoveAndDropAtScopeExit,
        _ => OwnershipAction::ScopedGuardMoveAndDrop,
    }
}

// =========================================================================
// PHASE 3: 4-TIER METHOD TABLE, INTERFACE, BUNDLE & TYPEDEF STRUCTURES
// =========================================================================

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InterfaceId(pub u32);

impl InterfaceId {
    pub const IANY_VALUE: Self = Self(0);
    pub const IVALUE: Self = Self(1);
    pub const IREFERENCE: Self = Self(2);
    pub const IOBJECT: Self = Self(3);
    pub const IGUARD: Self = Self(4);
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BundleId(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum OriginId {
    Struct(TypeId),
    Bundle(BundleId),
    Interface(InterfaceId),
}

/// O(1) BitSet for interface implementation and `[T: IComparable<T>]` checks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BitSet {
    words: Vec<u64>,
}

impl BitSet {
    pub fn new() -> Self {
        Self { words: Vec::new() }
    }

    #[inline]
    pub fn insert(&mut self, bit: u32) {
        let word_idx = (bit / 64) as usize;
        let bit_idx = bit % 64;
        if word_idx >= self.words.len() {
            self.words.resize(word_idx + 1, 0);
        }
        self.words[word_idx] |= 1u64 << bit_idx;
    }

    #[inline]
    pub fn contains(&self, bit: u32) -> bool {
        let word_idx = (bit / 64) as usize;
        let bit_idx = bit % 64;
        self.words
            .get(word_idx)
            .map_or(false, |&w| (w & (1u64 << bit_idx)) != 0)
    }

    #[inline]
    pub fn insert_interface(&mut self, id: InterfaceId) {
        self.insert(id.0);
    }

    #[inline]
    pub fn contains_interface(&self, id: InterfaceId) -> bool {
        self.contains(id.0)
    }

    #[inline]
    pub fn insert_lang_item(&mut self, item: LangItem) {
        let bit = match item {
            LangItem::IAnyValue => InterfaceId::IANY_VALUE.0,
            LangItem::IValue => InterfaceId::IVALUE.0,
            LangItem::IReference => InterfaceId::IREFERENCE.0,
            LangItem::IObject => InterfaceId::IOBJECT.0,
            LangItem::IGuard => InterfaceId::IGUARD.0,
            other => 1000 + (other as u32),
        };
        self.insert(bit);
    }

    #[inline]
    pub fn contains_lang_item(&self, item: LangItem) -> bool {
        let bit = match item {
            LangItem::IAnyValue => InterfaceId::IANY_VALUE.0,
            LangItem::IValue => InterfaceId::IVALUE.0,
            LangItem::IReference => InterfaceId::IREFERENCE.0,
            LangItem::IObject => InterfaceId::IOBJECT.0,
            LangItem::IGuard => InterfaceId::IGUARD.0,
            other => 1000 + (other as u32),
        };
        self.contains(bit)
    }

    #[inline]
    pub fn is_subset_of(&self, other: &BitSet) -> bool {
        for (idx, &w) in self.words.iter().enumerate() {
            if w == 0 {
                continue;
            }
            let ow = other.words.get(idx).copied().unwrap_or(0);
            if (w & ow) != w {
                return false;
            }
        }
        true
    }

    pub fn union_with(&mut self, other: &BitSet) {
        if other.words.len() > self.words.len() {
            self.words.resize(other.words.len(), 0);
        }
        for (idx, &ow) in other.words.iter().enumerate() {
            self.words[idx] |= ow;
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MethodTier {
    Tier4BaseInterfaceDefault = 1, // Lowest priority: default method on root interface
    Tier3SubInterfaceDefault  = 2, // Overrides Tier 4: default method on derived interface
    Tier2BundleOrFacetSplice  = 3, // Overrides Tiers 3 & 4; fulfills interface contracts
    Tier1ExplicitStructMember = 4, // Highest priority: explicit `fn` / `op` in `struct` or `partial struct`
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodSignature {
    pub sym: SymbolId,
    pub params: Vec<TypeId>,
    pub ret: TypeId,
    pub is_static: bool,
    pub is_mutating_var: bool,
    pub is_pure: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodEntry {
    pub sym: SymbolId,
    pub fn_id: FunctionId,
    pub tier: MethodTier,
    pub source_origin: OriginId,   // StructId, BundleId, or InterfaceId for collision diagnostics
    pub is_static: bool,
    pub is_mutating_var: bool,     // True if declared with `var fn` or `var op`
    pub is_pure: bool,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDef {
    pub name: SymbolId,
    pub ty: TypeId,
    pub byte_offset: usize,
    pub visibility: u8,            // 0 (priv) .. 9 (export)
    pub source_bundle: Option<BundleId>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct BundleDef {
    pub id: BundleId,
    pub name: SymbolId,
    pub fields: Vec<FieldDef>,     // Non-empty => `...Bundle` (Primary struct only); Empty => `...Facet` (`partial` safe)
    pub methods: Vec<MethodEntry>,
    pub span: Span,
}

impl BundleDef {
    #[inline(always)]
    pub fn is_method_only_facet(&self) -> bool {
        self.fields.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct SpliceDirectives {
    pub bundle_id: BundleId,
    pub coalesce_fields: FxHashSet<SymbolId>,          // `coalesce { id }`
    pub renamed_symbols: FxHashMap<SymbolId, SymbolId>,// `with { flags as net_flags }`
    pub excluded_symbols: FxHashSet<SymbolId>,         // `without { clear_state }`
    pub span: Span,
}

impl SpliceDirectives {
    pub fn new(bundle_id: BundleId, span: Span) -> Self {
        Self {
            bundle_id,
            coalesce_fields: FxHashSet::new(),
            renamed_symbols: FxHashMap::new(),
            excluded_symbols: FxHashSet::new(),
            span,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InterfaceDef {
    pub id: InterfaceId,
    pub name: SymbolId,
    pub home_pkg: PackageId,
    pub super_interfaces: Vec<InterfaceId>,
    pub explicit_lattice: Option<LatticeBits>,         // Set by `[Lattice(0b00..0b11)]`
    pub required_contracts: Vec<MethodSignature>,      // Must be satisfied by Tier 1 or Tier 2
    pub default_methods: Vec<MethodEntry>,             // Tier 3 (if overriding super) or Tier 4
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TypeDef {
    pub id: TypeId,
    pub name: SymbolId,
    pub home_pkg: PackageId,
    pub is_lang_primitive: bool,                      // True if annotated with `[Lang("...")]`
    pub size: usize,
    pub align: usize,
    pub layout_sealed: bool,                          // True as soon as primary `struct` block closes
    pub lattice: LatticeBits,                         // 0b00..0b11
    pub destructor_fn: Option<FunctionId>,            // `~self()`
    pub fields: Vec<FieldDef>,
    pub field_index: FxHashMap<SymbolId, usize>,
    pub implemented_interfaces: BitSet,               // O(1) bitset for `[T: IComparable<T>]` checks
    pub method_table: FxHashMap<SymbolId, MethodEntry>,
    pub static_delegates: FxHashMap<(SymbolId, SymbolId), FunctionId>, // `(BundleOrIfaceName, MethodName)`
    pub declared_interfaces: Vec<InterfaceId>,
}

impl TypeDef {
    pub fn new(id: TypeId, name: SymbolId, home_pkg: PackageId) -> Self {
        Self {
            id,
            name,
            home_pkg,
            is_lang_primitive: false,
            size: 0,
            align: 1,
            layout_sealed: false,
            lattice: LatticeBits::Value,
            destructor_fn: None,
            fields: Vec::new(),
            field_index: FxHashMap::new(),
            implemented_interfaces: BitSet::new(),
            method_table: FxHashMap::new(),
            static_delegates: FxHashMap::new(),
            declared_interfaces: Vec::new(),
        }
    }
}


#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub u32);

impl TypeId {
    pub const VOID: Self = Self(0);
    pub const NEVER: Self = Self(1);
    pub const ANY: Self = Self(2);
    pub const NULL: Self = Self(3);
    pub const BOOL: Self = Self(4);
    pub const I64: Self = Self(5);
    pub const I32: Self = Self(6);
    pub const F64: Self = Self(7);
    pub const F32: Self = Self(8);
    pub const STR: Self = Self(9);

    #[inline]
    pub const fn is_primitive(self) -> bool {
        self.0 <= 9
    }

    #[inline]
    pub const fn is_integer(self) -> bool {
        self.0 == Self::I64.0 || self.0 == Self::I32.0
    }

    #[inline]
    pub const fn is_float(self) -> bool {
        self.0 == Self::F64.0 || self.0 == Self::F32.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CollectionKind {
    List,
    Set,
    Future,
    Slice,
    SliceBuffer,
    SliceView,
    StridedSlice,
    View,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeKind {
    Primitive(String),
    Collection {
        kind: CollectionKind,
        element: TypeId,
    },
    Map {
        key: TypeId,
        value: TypeId,
    },
    SliceMap {
        key: TypeId,
        value: TypeId,
    },
    Tuple(Vec<TypeId>),
    Option(TypeId),
    Result {
        ok: TypeId,
        err: TypeId,
    },
    Function {
        params: Vec<TypeId>,
        ret: TypeId,
    },
    Matrix {
        rows: u32,
        cols: u32,
    },
    Unit {
        base: TypeId,
        unit_name: String,
    },
    Union(Vec<TypeId>),
    TypeVar {
        id: u32,
        constraint: u32, // bitmask: 1=Numeric, 2=Comparable, 4=Equatable, 8=Addable, 16=Iterable
    },
    RowRecord {
        fields: Vec<(String, TypeId)>, // sorted by field name for canonical identity
        is_open: bool,
        tail_var: Option<u32>,
        name: Option<String>,
    },
    Typestate {
        base: TypeId,
        state_name: String,
    },
    Adt {
        name: String,
        variants: Vec<(String, Vec<TypeId>)>,
    },
    Void,
    Never,
    Any,
    Null,
}

#[derive(Debug)]
pub struct TypeArena {
    types: Vec<TypeKind>,
    type_map: HashMap<TypeKind, TypeId>,
    pub nominal_names: HashMap<TypeId, String>,
    next_var_id: u32,
    subst: HashMap<u32, TypeId>,
    // --- Phase 3: Lattice, TypeDef, InterfaceDef, BundleDef & 4-Tier MethodTable ---
    pub type_defs: FxHashMap<TypeId, TypeDef>,
    pub type_defs_by_sym: FxHashMap<SymbolId, TypeId>,
    pub type_defs_by_name: FxHashMap<String, TypeId>,
    pub interface_defs: FxHashMap<InterfaceId, InterfaceDef>,
    pub interfaces_by_sym: FxHashMap<SymbolId, InterfaceId>,
    pub interfaces_by_name: FxHashMap<String, InterfaceId>,
    pub bundle_defs: FxHashMap<BundleId, BundleDef>,
    pub bundles_by_sym: FxHashMap<SymbolId, BundleId>,
    pub bundles_by_name: FxHashMap<String, BundleId>,
    pub method_signatures: FxHashMap<FunctionId, MethodSignature>,
    pub method_ast_nodes: FxHashMap<FunctionId, pilot_ast::NodeId>,
    pub rewritten_method_fields: FxHashMap<FunctionId, FxHashMap<SymbolId, SymbolId>>,
    pub pending_tier2_collisions: FxHashMap<(TypeId, SymbolId), (OriginId, OriginId, Span)>,
    pub pending_iface_collisions: FxHashMap<(TypeId, SymbolId), (OriginId, OriginId, Span)>,
    pub registered_attributes: FxHashSet<String>,
    next_interface_id: u32,
    next_bundle_id: u32,
    next_fn_id: u32,
}

impl Default for TypeArena {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeArena {
    pub fn new() -> Self {
        let mut arena = Self {
            types: Vec::with_capacity(64),
            type_map: HashMap::with_capacity(64),
            nominal_names: HashMap::new(),
            next_var_id: 100,
            subst: HashMap::new(),
            type_defs: FxHashMap::new(),
            type_defs_by_sym: FxHashMap::new(),
            type_defs_by_name: FxHashMap::new(),
            interface_defs: FxHashMap::new(),
            interfaces_by_sym: FxHashMap::new(),
            interfaces_by_name: FxHashMap::new(),
            bundle_defs: FxHashMap::new(),
            bundles_by_sym: FxHashMap::new(),
            bundles_by_name: FxHashMap::new(),
            method_signatures: FxHashMap::new(),
            method_ast_nodes: FxHashMap::new(),
            rewritten_method_fields: FxHashMap::new(),
            pending_tier2_collisions: FxHashMap::new(),
            pending_iface_collisions: FxHashMap::new(),
            registered_attributes: FxHashSet::new(),
            next_interface_id: 5,
            next_bundle_id: 1,
            next_fn_id: 1,
        };

        // Initialize predefined primitive types at 0..9
        arena.register_builtin(TypeId::VOID, TypeKind::Void);
        arena.register_builtin(TypeId::NEVER, TypeKind::Never);
        arena.register_builtin(TypeId::ANY, TypeKind::Any);
        arena.register_builtin(TypeId::NULL, TypeKind::Null);
        arena.register_builtin(TypeId::BOOL, TypeKind::Primitive("bool".to_string()));
        arena.register_builtin(TypeId::I64, TypeKind::Primitive("i64".to_string()));
        arena.register_builtin(TypeId::I32, TypeKind::Primitive("i32".to_string()));
        arena.register_builtin(TypeId::F64, TypeKind::Primitive("f64".to_string()));
        arena.register_builtin(TypeId::F32, TypeKind::Primitive("f32".to_string()));
        arena.register_builtin(TypeId::STR, TypeKind::Primitive("str".to_string()));

        // Initialize the 5 canonical lattice origin interfaces at InterfaceId(0..=4)
        arena.init_origin_interfaces();

        arena
    }

    fn init_origin_interfaces(&mut self) {
        let origins = [
            (InterfaceId::IANY_VALUE, "IAnyValue", None, vec![]),
            (InterfaceId::IVALUE, "IValue", Some(LatticeBits::Value), vec![InterfaceId::IANY_VALUE]),
            (InterfaceId::IREFERENCE, "IReference", Some(LatticeBits::Reference), vec![InterfaceId::IANY_VALUE]),
            (InterfaceId::IOBJECT, "IObject", Some(LatticeBits::Object), vec![InterfaceId::IANY_VALUE]),
            (
                InterfaceId::IGUARD,
                "IGuard",
                Some(LatticeBits::Guard),
                vec![InterfaceId::IOBJECT, InterfaceId::IREFERENCE],
            ),
        ];
        for (id, name, explicit_lattice, super_interfaces) in origins {
            self.interfaces_by_name.insert(name.to_string(), id);
            self.interface_defs.insert(
                id,
                InterfaceDef {
                    id,
                    name: SymbolId(0),
                    home_pkg: PackageId(0),
                    super_interfaces,
                    explicit_lattice,
                    required_contracts: Vec::new(),
                    default_methods: Vec::new(),
                    span: ByteSpan::default(),
                },
            );
        }
    }

    fn register_builtin(&mut self, expected_id: TypeId, kind: TypeKind) {
        let id = TypeId(self.types.len() as u32);
        assert_eq!(id, expected_id);
        self.types.push(kind.clone());
        self.type_map.insert(kind, id);
    }

    pub fn alloc(&mut self, kind: TypeKind) -> TypeId {
        if let Some(&existing) = self.type_map.get(&kind) {
            return existing;
        }

        let id = TypeId(self.types.len() as u32);
        self.types.push(kind.clone());
        self.type_map.insert(kind, id);
        id
    }

    pub fn get(&self, id: TypeId) -> &TypeKind {
        &self.types[id.0 as usize]
    }

    pub fn new_var(&mut self, constraint: u32) -> TypeId {
        let id = self.next_var_id;
        self.next_var_id += 1;
        self.alloc(TypeKind::TypeVar { id, constraint })
    }

    pub fn list_of(&mut self, elem: TypeId) -> TypeId {
        self.alloc(TypeKind::Collection {
            kind: CollectionKind::List,
            element: elem,
        })
    }

    pub fn map_of(&mut self, key: TypeId, value: TypeId) -> TypeId {
        self.alloc(TypeKind::Map { key, value })
    }

    pub fn slice_of(&mut self, elem: TypeId) -> TypeId {
        self.alloc(TypeKind::Collection {
            kind: CollectionKind::Slice,
            element: elem,
        })
    }

    pub fn slice_buffer_of(&mut self, elem: TypeId) -> TypeId {
        self.alloc(TypeKind::Collection {
            kind: CollectionKind::SliceBuffer,
            element: elem,
        })
    }

    pub fn slice_view_of(&mut self, elem: TypeId) -> TypeId {
        self.alloc(TypeKind::Collection {
            kind: CollectionKind::SliceView,
            element: elem,
        })
    }

    pub fn strided_slice_of(&mut self, elem: TypeId) -> TypeId {
        self.alloc(TypeKind::Collection {
            kind: CollectionKind::StridedSlice,
            element: elem,
        })
    }

    pub fn view_of(&mut self, elem: TypeId) -> TypeId {
        self.alloc(TypeKind::Collection {
            kind: CollectionKind::View,
            element: elem,
        })
    }

    pub fn slice_map_of(&mut self, key: TypeId, value: TypeId) -> TypeId {
        self.alloc(TypeKind::SliceMap { key, value })
    }

    pub fn option_of(&mut self, elem: TypeId) -> TypeId {
        if let TypeKind::Option(_) = self.get(elem) {
            return elem;
        }
        self.alloc(TypeKind::Option(elem))
    }

    pub fn result_of(&mut self, ok: TypeId, err: TypeId) -> TypeId {
        self.alloc(TypeKind::Result { ok, err })
    }

    pub fn tuple_of(&mut self, elems: Vec<TypeId>) -> TypeId {
        self.alloc(TypeKind::Tuple(elems))
    }

    pub fn function_of(&mut self, params: Vec<TypeId>, ret: TypeId) -> TypeId {
        self.alloc(TypeKind::Function { params, ret })
    }

    pub fn unit_of(&mut self, base: TypeId, unit_name: String) -> TypeId {
        self.alloc(TypeKind::Unit { base, unit_name })
    }

    pub fn typestate_of(&mut self, base: TypeId, state_name: String) -> TypeId {
        self.alloc(TypeKind::Typestate { base, state_name })
    }

    pub fn adt_of(&mut self, name: String, variants: Vec<(String, Vec<TypeId>)>) -> TypeId {
        self.alloc(TypeKind::Adt { name, variants })
    }

    pub fn register_nominal_name(&mut self, id: TypeId, name: String) {
        self.nominal_names.insert(id, name);
    }

    pub fn row_record_of(
        &mut self,
        fields: Vec<(String, TypeId)>,
        is_open: bool,
        tail_var: Option<u32>,
    ) -> TypeId {
        self.row_record_with_name(fields, is_open, tail_var, None)
    }

    pub fn row_record_with_name(
        &mut self,
        mut fields: Vec<(String, TypeId)>,
        is_open: bool,
        tail_var: Option<u32>,
        name: Option<String>,
    ) -> TypeId {
        fields.sort_by(|a, b| a.0.cmp(&b.0));
        let id = self.alloc(TypeKind::RowRecord {
            fields,
            is_open,
            tail_var,
            name: name.clone(),
        });
        if let Some(n) = name {
            self.nominal_names.insert(id, n);
        }
        id
    }

    /// Forms a canonical, flattened, and deduplicated Union type from a set of types.
    pub fn union_of(&mut self, types: Vec<TypeId>) -> TypeId {
        let mut flattened = Vec::new();
        for t in types {
            let pruned = self.prune(t);
            if pruned == TypeId::ANY {
                return TypeId::ANY;
            }
            if pruned == TypeId::NEVER || pruned == TypeId::VOID {
                continue;
            }
            if let TypeKind::Union(inner) = self.get(pruned).clone() {
                for u in inner {
                    let u_pruned = self.prune(u);
                    if !flattened.contains(&u_pruned) {
                        flattened.push(u_pruned);
                    }
                }
            } else if !flattened.contains(&pruned) {
                flattened.push(pruned);
            }
        }
        if flattened.is_empty() {
            TypeId::NEVER
        } else if flattened.len() == 1 {
            flattened[0]
        } else {
            flattened.sort_by_key(|t| t.0);
            self.alloc(TypeKind::Union(flattened))
        }
    }

    /// Unifies all types if they are mutually compatible; otherwise forms a constrained Union.
    pub fn unify_or_union_all(&mut self, types: Vec<TypeId>) -> TypeId {
        if types.is_empty() {
            return self.new_var(0);
        }
        let mut unified = self.prune(types[0]);
        let mut all_unified = true;
        for &t in &types[1..] {
            let prev_subst = self.subst.clone();
            match self.unify(unified, t) {
                Ok(u) => unified = u,
                Err(_) => {
                    self.subst = prev_subst;
                    all_unified = false;
                    break;
                }
            }
        }
        if all_unified {
            return unified;
        }
        self.union_of(types)
    }

    /// Resolves any assigned type substitutions for a given TypeId.
    pub fn prune(&self, mut id: TypeId) -> TypeId {
        while let TypeKind::TypeVar { id: var_id, .. } = self.get(id) {
            if let Some(&next) = self.subst.get(var_id) {
                id = next;
            } else {
                break;
            }
        }
        id
    }

    /// Recursively substitutes all type variables across nested composite types.
    pub fn deep_prune(&mut self, ty: TypeId) -> TypeId {
        let pruned = self.prune(ty);
        match self.get(pruned).clone() {
            TypeKind::Collection { kind, element } => {
                let e = self.deep_prune(element);
                self.alloc(TypeKind::Collection { kind, element: e })
            }
            TypeKind::Map { key, value } => {
                let k = self.deep_prune(key);
                let v = self.deep_prune(value);
                self.alloc(TypeKind::Map { key: k, value: v })
            }
            TypeKind::SliceMap { key, value } => {
                let k = self.deep_prune(key);
                let v = self.deep_prune(value);
                self.alloc(TypeKind::SliceMap { key: k, value: v })
            }
            TypeKind::Tuple(elems) => {
                let pr_elems: Vec<_> = elems.into_iter().map(|e| self.deep_prune(e)).collect();
                self.alloc(TypeKind::Tuple(pr_elems))
            }
            TypeKind::Option(inner) => {
                let inn = self.deep_prune(inner);
                self.alloc(TypeKind::Option(inn))
            }
            TypeKind::Result { ok, err } => {
                let o = self.deep_prune(ok);
                let e = self.deep_prune(err);
                self.alloc(TypeKind::Result { ok: o, err: e })
            }
            TypeKind::Function { params, ret } => {
                let pr_params: Vec<_> = params.into_iter().map(|p| self.deep_prune(p)).collect();
                let pr_ret = self.deep_prune(ret);
                self.alloc(TypeKind::Function { params: pr_params, ret: pr_ret })
            }
            TypeKind::Union(types) => {
                let pr_types: Vec<_> = types.into_iter().map(|t| self.deep_prune(t)).collect();
                self.union_of(pr_types)
            }
            TypeKind::RowRecord { fields, is_open, tail_var, name } => {
                let pr_fields: Vec<_> = fields.into_iter().map(|(n, t)| (n, self.deep_prune(t))).collect();
                self.row_record_with_name(pr_fields, is_open, tail_var, name)
            }
            _ => pruned,
        }
    }

    /// Unifies two types in the arena, recording type variable substitutions.
    pub fn unify(&mut self, a: TypeId, b: TypeId) -> Result<TypeId, String> {
        let a = self.prune(a);
        let b = self.prune(b);

        if a == b {
            return Ok(a);
        }

        // ANY unifies with everything
        if a == TypeId::ANY {
            return Ok(b);
        }
        if b == TypeId::ANY {
            return Ok(a);
        }

        // Never unifies as bottom type
        if a == TypeId::NEVER {
            return Ok(b);
        }
        if b == TypeId::NEVER {
            return Ok(a);
        }

        match (self.get(a).clone(), self.get(b).clone()) {
            (TypeKind::TypeVar { id: v, .. }, _) => {
                self.subst.insert(v, b);
                Ok(b)
            }
            (_, TypeKind::TypeVar { id: v, .. }) => {
                self.subst.insert(v, a);
                Ok(a)
            }
            (TypeKind::Option(elem_a), TypeKind::Option(elem_b)) => {
                let elem = self.unify(elem_a, elem_b)?;
                Ok(self.option_of(elem))
            }
            (TypeKind::Collection { kind: k1, element: e1 }, TypeKind::Collection { kind: k2, element: e2 }) if k1 == k2 => {
                let elem = self.unify(e1, e2)?;
                Ok(self.alloc(TypeKind::Collection { kind: k1, element: elem }))
            }
            (TypeKind::Map { key: k1, value: v1 }, TypeKind::Map { key: k2, value: v2 }) => {
                let key = self.unify(k1, k2)?;
                let value = self.unify(v1, v2)?;
                Ok(self.map_of(key, value))
            }
            (TypeKind::SliceMap { key: k1, value: v1 }, TypeKind::SliceMap { key: k2, value: v2 }) => {
                let key = self.unify(k1, k2)?;
                let value = self.unify(v1, v2)?;
                Ok(self.slice_map_of(key, value))
            }
            (TypeKind::Tuple(t1), TypeKind::Tuple(t2)) if t1.len() == t2.len() => {
                let mut unified = Vec::with_capacity(t1.len());
                for (x, y) in t1.iter().zip(t2.iter()) {
                    unified.push(self.unify(*x, *y)?);
                }
                Ok(self.tuple_of(unified))
            }
            (TypeKind::Function { params: p1, ret: r1 }, TypeKind::Function { params: p2, ret: r2 }) if p1.len() == p2.len() => {
                let mut unified_params = Vec::with_capacity(p1.len());
                for (x, y) in p1.iter().zip(p2.iter()) {
                    unified_params.push(self.unify(*x, *y)?);
                }
                let ret = self.unify(r1, r2)?;
                Ok(self.function_of(unified_params, ret))
            }
            (
                TypeKind::RowRecord { fields: f1, is_open: open1, tail_var: t1, name: n1 },
                TypeKind::RowRecord { fields: f2, is_open: open2, tail_var: t2, name: n2 }
            ) => {
                // Structural row unification
                let mut merged_fields = HashMap::new();
                for (name, ty) in &f1 {
                    merged_fields.insert(name.clone(), *ty);
                }
                for (name, ty) in &f2 {
                    if let Some(existing) = merged_fields.get(name) {
                        let unified = self.unify(*existing, *ty)?;
                        merged_fields.insert(name.clone(), unified);
                    } else if open1 {
                        merged_fields.insert(name.clone(), *ty);
                    } else {
                        return Err(format!("Closed row record missing field '{}'", name));
                    }
                }
                if !open2 {
                    for (name, _) in &f1 {
                        if !f2.iter().any(|(n, _)| n == name) {
                            return Err(format!("Closed row record missing field '{}'", name));
                        }
                    }
                }
                let mut sorted: Vec<(String, TypeId)> = merged_fields.into_iter().collect();
                sorted.sort_by(|x, y| x.0.cmp(&y.0));
                let is_open = open1 && open2;
                let tail = if is_open { t1.or(t2) } else { None };
                let resolved_name = n1.clone().or_else(|| n2.clone());
                Ok(self.row_record_with_name(sorted, is_open, tail, resolved_name))
            }
            (TypeKind::RowRecord { fields, .. }, TypeKind::Map { key, value })
            | (TypeKind::Map { key, value }, TypeKind::RowRecord { fields, .. }) => {
                let unified_key = self.unify(key, TypeId::STR)?;
                let field_tys: Vec<TypeId> = fields.into_iter().map(|(_, t)| t).collect();
                let record_val_ty = self.unify_or_union_all(field_tys);
                let unified_val = self.unify(value, record_val_ty)?;
                Ok(self.map_of(unified_key, unified_val))
            }
            (TypeKind::Typestate { base: b1, state_name: s1 }, TypeKind::Typestate { base: b2, state_name: s2 }) => {
                if s1 != s2 {
                    return Err(format!("Typestate mismatch: '{}' vs '{}'", s1, s2));
                }
                let base = self.unify(b1, b2)?;
                Ok(self.typestate_of(base, s1))
            }
            (TypeKind::Unit { base: b1, unit_name: u1 }, TypeKind::Unit { base: b2, unit_name: u2 }) => {
                if u1 != u2 {
                    return Err(format!("Unit dimension mismatch: '{}' vs '{}'", u1, u2));
                }
                let base = self.unify(b1, b2)?;
                Ok(self.unit_of(base, u1))
            }
            (TypeKind::Adt { name: n1, .. }, TypeKind::Adt { name: n2, .. }) => {
                if n1 == n2 {
                    Ok(a)
                } else {
                    Err(format!("ADT type mismatch: '{}' vs '{}'", n1, n2))
                }
            }
            (TypeKind::Union(types_a), TypeKind::Union(types_b)) => {
                let all_a_in_b = types_a.iter().all(|ta| types_b.contains(ta));
                let all_b_in_a = types_b.iter().all(|tb| types_a.contains(tb));
                if all_a_in_b && all_b_in_a {
                    return Ok(a);
                }
                let mut can_sub = true;
                for &ta in &types_a {
                    let mut found = false;
                    for &tb in &types_b {
                        let prev_subst = self.subst.clone();
                        if self.unify(ta, tb).is_ok() {
                            found = true;
                            break;
                        }
                        self.subst = prev_subst;
                    }
                    if !found {
                        can_sub = false;
                        break;
                    }
                }
                if can_sub {
                    Ok(b)
                } else {
                    Err(format!("Cannot unify union '{}' with '{}'", self.display(a), self.display(b)))
                }
            }
            (TypeKind::Union(types), _) => {
                for &u in &types {
                    self.unify(u, b)?;
                }
                Ok(b)
            }
            (_, TypeKind::Union(types)) => {
                for &u in &types {
                    let prev_subst = self.subst.clone();
                    if self.unify(a, u).is_ok() {
                        return Ok(b);
                    }
                    self.subst = prev_subst;
                }
                Err(format!("Type '{}' is not a member of union '{}'", self.display(a), self.display(b)))
            }
            _ => Err(format!("Cannot unify type {:?} with {:?}", self.get(a), self.get(b))),
        }
    }

    /// Performs structural row monomorphization (Milestone 16.3 / 33.1):
    /// closes an open row record against required concrete fields.
    pub fn monomorphize_row(
        &mut self,
        row_type: TypeId,
        required_fields: &[(String, TypeId)],
    ) -> Result<TypeId, String> {
        let row_type = self.prune(row_type);
        match self.get(row_type).clone() {
            TypeKind::RowRecord { fields, name, .. } => {
                let mut map: HashMap<String, TypeId> = fields.into_iter().collect();
                for (k, v) in required_fields {
                    if let Some(existing) = map.get(k) {
                        let unified = self.unify(*existing, *v)?;
                        map.insert(k.clone(), unified);
                    } else {
                        map.insert(k.clone(), *v);
                    }
                }
                let mut res: Vec<(String, TypeId)> = map.into_iter().collect();
                res.sort_by(|a, b| a.0.cmp(&b.0));
                Ok(self.row_record_with_name(res, false, None, name))
            }
            _ => Err("Target is not a row record".to_string()),
        }
    }

    pub fn display(&self, id: TypeId) -> String {
        let id = self.prune(id);
        if !id.is_primitive() {
            if let Some(name) = self.nominal_names.get(&id) {
                return name.clone();
            }
        }
        match self.get(id) {
            TypeKind::Void => "void".to_string(),
            TypeKind::Never => "never".to_string(),
            TypeKind::Any => "any".to_string(),
            TypeKind::Null => "null".to_string(),
            TypeKind::Primitive(name) => name.clone(),
            TypeKind::Collection { kind, element } => {
                let k_str = match kind {
                    CollectionKind::List => "List",
                    CollectionKind::Set => "Set",
                    CollectionKind::Future => "Future",
                    CollectionKind::Slice => "Slice",
                    CollectionKind::SliceBuffer => "SliceBuffer",
                    CollectionKind::SliceView => "SliceView",
                    CollectionKind::StridedSlice => "StridedSlice",
                    CollectionKind::View => "View",
                };
                format!("{}[{}]", k_str, self.display(*element))
            }
            TypeKind::Map { key, value } => {
                format!("Map[{}, {}]", self.display(*key), self.display(*value))
            }
            TypeKind::SliceMap { key, value } => {
                format!("SliceMap[{}, {}]", self.display(*key), self.display(*value))
            }
            TypeKind::Tuple(elems) => {
                let inner: Vec<String> = elems.iter().map(|e| self.display(*e)).collect();
                format!("({})", inner.join(", "))
            }
            TypeKind::Option(elem) => format!("{}?", self.display(*elem)),
            TypeKind::Result { ok, err } => {
                format!("Result[{}, {}]", self.display(*ok), self.display(*err))
            }
            TypeKind::Function { params, ret } => {
                let p_str: Vec<String> = params.iter().map(|p| self.display(*p)).collect();
                format!("({}) -> {}", p_str.join(", "), self.display(*ret))
            }
            TypeKind::Matrix { rows, cols } => format!("matrix{}x{}", rows, cols),
            TypeKind::Unit { base, unit_name } => format!("{}<{}>", self.display(*base), unit_name),
            TypeKind::Union(types) => {
                let t_str: Vec<String> = types.iter().map(|t| self.display(*t)).collect();
                t_str.join(" | ")
            }
            TypeKind::TypeVar { id, .. } => format!("'T{}", id),
            TypeKind::RowRecord { fields, is_open, name, .. } => {
                if let Some(n) = name {
                    return n.clone();
                }
                let f_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.display(*v)))
                    .collect();
                if *is_open {
                    format!("{{ {}, .. }}", f_str.join(", "))
                } else {
                    format!("{{ {} }}", f_str.join(", "))
                }
            }
            TypeKind::Typestate { base, state_name } => {
                format!("{}@{}", self.display(*base), state_name)
            }
            TypeKind::Adt { name, .. } => name.clone(),
        }
    }

    pub fn display_structural(&self, id: TypeId) -> String {
        let id = self.prune(id);
        match self.get(id) {
            TypeKind::RowRecord { fields, is_open, .. } => {
                let f_str: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.display(*v)))
                    .collect();
                if *is_open {
                    format!("{{ {}, .. }}", f_str.join(", "))
                } else {
                    format!("{{ {} }}", f_str.join(", "))
                }
            }
            _ => self.display(id),
        }
    }

    // =========================================================================
    // PHASE 3: 2-BIT LATTICE, LAYOUT SEALING & 4-TIER METHOD/BUNDLE SOLVER
    // =========================================================================

    pub fn alloc_fn_id(&mut self) -> FunctionId {
        let id = FunctionId(self.next_fn_id);
        self.next_fn_id += 1;
        id
    }

    pub fn alloc_interface_id(&mut self) -> InterfaceId {
        let id = InterfaceId(self.next_interface_id);
        self.next_interface_id += 1;
        id
    }

    pub fn alloc_bundle_id(&mut self) -> BundleId {
        let id = BundleId(self.next_bundle_id);
        self.next_bundle_id += 1;
        id
    }

    pub fn register_type_def(&mut self, type_def: TypeDef, interner: &Interner) -> TypeId {
        let id = type_def.id;
        let sym = type_def.name;
        if sym.0 != 0 {
            let name_str = interner.resolve(sym).to_string();
            self.type_defs_by_sym.insert(sym, id);
            self.type_defs_by_name.insert(name_str.clone(), id);
            self.nominal_names.insert(id, name_str);
        }
        self.type_defs.insert(id, type_def);
        id
    }

    pub fn register_interface_def(&mut self, mut iface_def: InterfaceDef, interner: &Interner) -> InterfaceId {
        let sym = iface_def.name;
        let name_str = if sym.0 != 0 {
            interner.resolve(sym).to_string()
        } else {
            String::new()
        };
        // If this matches one of the 5 pre-allocated lattice origin interfaces, reuse its canonical ID
        if let Some(&existing_id) = self.interfaces_by_name.get(&name_str) {
            if existing_id.0 <= 4 {
                iface_def.id = existing_id;
            }
        }
        let id = iface_def.id;
        if sym.0 != 0 {
            self.interfaces_by_sym.insert(sym, id);
            self.interfaces_by_name.insert(name_str, id);
        }
        self.interface_defs.insert(id, iface_def);
        id
    }

    pub fn register_bundle_def(&mut self, bundle_def: BundleDef, interner: &Interner) -> BundleId {
        let id = bundle_def.id;
        let sym = bundle_def.name;
        if sym.0 != 0 {
            let name_str = interner.resolve(sym).to_string();
            self.bundles_by_sym.insert(sym, id);
            self.bundles_by_name.insert(name_str, id);
        }
        self.bundle_defs.insert(id, bundle_def);
        id
    }

    /// Computes or looks up the 2-bit `LatticeBits` of `ty` in O(1) time.
    pub fn type_lattice(&self, ty: TypeId) -> LatticeBits {
        let pruned = self.prune(ty);
        if let Some(td) = self.type_defs.get(&pruned) {
            return td.lattice;
        }
        match self.get(pruned) {
            TypeKind::Collection { kind, .. } => match kind {
                CollectionKind::Slice
                | CollectionKind::SliceView
                | CollectionKind::StridedSlice
                | CollectionKind::View => LatticeBits::Reference,
                CollectionKind::List
                | CollectionKind::Set
                | CollectionKind::Future
                | CollectionKind::SliceBuffer => LatticeBits::Object,
            },
            TypeKind::Map { .. } | TypeKind::SliceMap { .. } => LatticeBits::Object,
            TypeKind::Primitive(name) => {
                if let Some(&mapped_id) = self.type_defs_by_name.get(name) {
                    if let Some(td) = self.type_defs.get(&mapped_id) {
                        return td.lattice;
                    }
                }
                if name == "slice"
                    || name == "stride"
                    || name == "view"
                    || name.starts_with("ref ")
                    || name.starts_with('&')
                {
                    LatticeBits::Reference
                } else {
                    LatticeBits::Value
                }
            }
            TypeKind::Tuple(elems) => {
                let mut bits = LatticeBits::Value;
                for &e in elems {
                    bits = bits.union(self.type_lattice(e));
                }
                bits
            }
            TypeKind::Option(inner) => self.type_lattice(*inner),
            TypeKind::Result { ok, err } => self.type_lattice(*ok).union(self.type_lattice(*err)),
            TypeKind::RowRecord { fields, name, .. } => {
                if let Some(n) = name {
                    if let Some(&mapped_id) = self.type_defs_by_name.get(n) {
                        if let Some(td) = self.type_defs.get(&mapped_id) {
                            return td.lattice;
                        }
                    }
                }
                let mut bits = LatticeBits::Value;
                for (_, fty) in fields {
                    bits = bits.union(self.type_lattice(*fty));
                }
                bits
            }
            _ => LatticeBits::Value,
        }
    }

    /// Returns `(size_bytes, align_bytes)` for a type.
    pub fn type_size_align(&self, ty: TypeId) -> (usize, usize) {
        let pruned = self.prune(ty);
        if let Some(td) = self.type_defs.get(&pruned) {
            if td.size > 0 || td.is_lang_primitive {
                return (td.size, td.align.max(1));
            }
        }
        match self.get(pruned) {
            TypeKind::Void | TypeKind::Never | TypeKind::Null => (0, 1),
            TypeKind::Primitive(name) => {
                if let Some(&mapped_id) = self.type_defs_by_name.get(name) {
                    if let Some(td) = self.type_defs.get(&mapped_id) {
                        if td.size > 0 || td.is_lang_primitive {
                            return (td.size, td.align.max(1));
                        }
                    }
                }
                match name.as_str() {
                    "void" | "never" => (0, 1),
                    "bool" | "u8" | "i8" | "byte" => (1, 1),
                    "u16" | "i16" | "f16" => (2, 2),
                    "u32" | "i32" | "f32" | "char" => (4, 4),
                    "u128" | "i128" | "f128" => (16, 16),
                    "slice" | "str" | "string" => (16, 8),
                    "stride" => (24, 8),
                    _ => (8, 8), // usize, isize, i64, u64, f64, ptr, ref
                }
            }
            TypeKind::Collection { kind, .. } => match kind {
                CollectionKind::Slice | CollectionKind::SliceView | CollectionKind::View => (16, 8),
                CollectionKind::StridedSlice => (24, 8),
                _ => (24, 8),
            },
            TypeKind::Tuple(elems) => {
                let mut offset = 0usize;
                let mut max_align = 1usize;
                for &e in elems {
                    let (sz, al) = self.type_size_align(e);
                    let al = al.max(1);
                    offset = (offset + al - 1) & !(al - 1);
                    offset += sz;
                    max_align = max_align.max(al);
                }
                let total = (offset + max_align - 1) & !(max_align - 1);
                (total, max_align)
            }
            _ => (8, 8),
        }
    }

    /// Adds a stored field to a primary `struct` before layout sealing.
    pub fn add_struct_field(
        &mut self,
        type_id: TypeId,
        name: SymbolId,
        ty: TypeId,
        visibility: u8,
        span: Span,
    ) {
        let (f_size, f_align) = self.type_size_align(ty);
        let f_align = f_align.max(1);
        if let Some(td) = self.type_defs.get_mut(&type_id) {
            if td.layout_sealed {
                return;
            }
            if let Some(&idx) = td.field_index.get(&name) {
                td.fields[idx].ty = ty;
                return;
            }
            let current_offset = if td.fields.is_empty() {
                0
            } else {
                td.size
            };
            let aligned_offset = (current_offset + f_align - 1) & !(f_align - 1);
            let idx = td.fields.len();
            td.fields.push(FieldDef {
                name,
                ty,
                byte_offset: aligned_offset,
                visibility,
                source_bundle: None,
                span,
            });
            td.field_index.insert(name, idx);
            td.align = td.align.max(f_align);
            let raw_end = aligned_offset + f_size;
            td.size = (raw_end + td.align - 1) & !(td.align - 1);
        }
    }

    /// Adds a stored field to a `struct`, or emits `PartialStructCannotSpliceFields` if the primary
    /// struct's layout is already sealed (`layout_sealed == true`).
    pub fn add_struct_field_checked(
        &mut self,
        type_id: TypeId,
        name: SymbolId,
        ty: TypeId,
        visibility: u8,
        span: Span,
        interner: &Interner,
        diags: &mut Vec<AnalysisDiagnostic>,
    ) {
        if let Some(td) = self.type_defs.get(&type_id) {
            if td.layout_sealed {
                let f_name = interner.resolve(name);
                diags.push(AnalysisDiagnostic::partial_struct_cannot_splice_fields(
                    span,
                    f_name,
                ));
                return;
            }
        }
        self.add_struct_field(type_id, name, ty, visibility, span);
    }

    /// Section 3.1: Bitwise-OR Propagation & Verification Algorithm (`seal_primary_struct_layout`).
    pub fn seal_primary_struct_layout(
        &mut self,
        type_id: TypeId,
        interner: &Interner,
        diags: &mut Vec<AnalysisDiagnostic>,
    ) {
        let (has_dtor, field_list, declared_ifaces, type_name_sym, is_lang_prim, preset_size, preset_align) =
            match self.type_defs.get(&type_id) {
                Some(td) => (
                    td.destructor_fn.is_some(),
                    td.fields.clone(),
                    td.declared_interfaces.clone(),
                    td.name,
                    td.is_lang_primitive,
                    td.size,
                    td.align,
                ),
                None => return,
            };

        // 1. Start with 0b10 if ~self() destructor is present, else 0b00.
        let mut computed_bits: u8 = if has_dtor { 0b10 } else { 0b00 };
        let mut borrow_field: Option<(SymbolId, Span)> = None;
        let mut drop_field: Option<(SymbolId, Span)> = None;

        // 2. Bitwise-OR across all resolved fields + finalize field byte offsets
        let mut offset = 0usize;
        let mut max_align = if is_lang_prim { preset_align.max(1) } else { 1usize };
        let mut updated_fields = field_list;

        for field in &mut updated_fields {
            let f_bits = self.type_lattice(field.ty).raw();
            if (f_bits & 0b01) != 0 && borrow_field.is_none() {
                borrow_field = Some((field.name, field.span));
            }
            if (f_bits & 0b10) != 0 && drop_field.is_none() {
                drop_field = Some((field.name, field.span));
            }
            computed_bits |= f_bits;

            let (f_size, f_align) = self.type_size_align(field.ty);
            let f_align = f_align.max(1);
            offset = (offset + f_align - 1) & !(f_align - 1);
            field.byte_offset = offset;
            offset += f_size;
            max_align = max_align.max(f_align);
        }

        let final_size = if is_lang_prim && updated_fields.is_empty() && preset_size > 0 {
            preset_size
        } else if updated_fields.is_empty() {
            preset_size
        } else {
            (offset + max_align - 1) & !(max_align - 1)
        };

        let lattice = LatticeBits::from_raw(computed_bits);
        let type_name = if type_name_sym.0 != 0 {
            interner.resolve(type_name_sym).to_string()
        } else {
            self.nominal_names
                .get(&type_id)
                .cloned()
                .unwrap_or_else(|| format!("Type#{}", type_id.0))
        };

        // 4. Explicit Lattice Contract Verification against the most specific declared lattice origin
        let mut explicit_origins: Vec<(InterfaceId, LatticeBits, String, Span)> = Vec::new();
        for &iface_id in &declared_ifaces {
            if let Some(iface_def) = self.interface_defs.get(&iface_id) {
                if let Some(exp_bits) = iface_def.explicit_lattice {
                    let iname = if iface_def.name.0 != 0 {
                        interner.resolve(iface_def.name).to_string()
                    } else {
                        match iface_id {
                            InterfaceId::IVALUE => "IValue".to_string(),
                            InterfaceId::IREFERENCE => "IReference".to_string(),
                            InterfaceId::IOBJECT => "IObject".to_string(),
                            InterfaceId::IGUARD => "IGuard".to_string(),
                            _ => "IInterface".to_string(),
                        }
                    };
                    explicit_origins.push((iface_id, exp_bits, iname, iface_def.span));
                }
            }
        }

        let has_iguard = explicit_origins
            .iter()
            .any(|(_, b, _, _)| *b == LatticeBits::Guard);

        for (_iface_id, exp_bits, iname, span) in &explicit_origins {
            match exp_bits {
                LatticeBits::Value => {
                    if computed_bits != 0b00 {
                        let reason = if let Some((fname, fspan)) = borrow_field {
                            let fn_str = interner.resolve(fname);
                            let use_span = if fspan.end > 0 { fspan } else { *span };
                            diags.push(AnalysisDiagnostic::lattice_contract_mismatch(
                                use_span,
                                &type_name,
                                iname,
                                0b00,
                                computed_bits,
                                &format!("borrowed reference field '{}' introduces Bit 0 (0b01)", fn_str),
                            ));
                            continue;
                        } else if has_dtor {
                            "destructor '~self()' introduces Bit 1 (0b10)".to_string()
                        } else if let Some((fname, _)) = drop_field {
                            format!("object field '{}' introduces Bit 1 (0b10)", interner.resolve(fname))
                        } else {
                            "non-zero lattice bits".to_string()
                        };
                        diags.push(AnalysisDiagnostic::lattice_contract_mismatch(
                            *span,
                            &type_name,
                            iname,
                            0b00,
                            computed_bits,
                            &reason,
                        ));
                    }
                }
                LatticeBits::Reference if !has_iguard => {
                    if (computed_bits & 0b10) != 0 {
                        let reason = if has_dtor {
                            "a '~self()' destructor makes the type an 'IGuard' (0b11), not a pure 'IReference' (0b01); declare ': IGuard'"
                        } else {
                            "an owning 'IObject' field makes the type an 'IGuard' (0b11), not a pure 'IReference' (0b01); declare ': IGuard'"
                        };
                        diags.push(AnalysisDiagnostic::lattice_contract_mismatch(
                            *span,
                            &type_name,
                            iname,
                            0b01,
                            computed_bits,
                            reason,
                        ));
                    }
                }
                LatticeBits::Object if !has_iguard => {
                    if (computed_bits & 0b01) != 0 {
                        let field_detail = borrow_field
                            .map(|(f, _)| format!(" (from field '{}')", interner.resolve(f)))
                            .unwrap_or_default();
                        diags.push(AnalysisDiagnostic::lattice_contract_mismatch(
                            *span,
                            &type_name,
                            iname,
                            0b10,
                            computed_bits,
                            &format!(
                                "borrowed field{} makes the type an 'IGuard' (0b11), not a self-contained 'IObject' (0b10); declare ': IGuard'",
                                field_detail
                            ),
                        ));
                    }
                }
                _ => {}
            }
        }

        // 3, 5, 6. Store lattice, auto-attach lattice origin interfaces, and seal layout
        if let Some(td) = self.type_defs.get_mut(&type_id) {
            td.fields = updated_fields;
            td.size = final_size;
            td.align = max_align;
            td.lattice = lattice;

            // Unconditionally set LangItem::IAnyValue
            td.implemented_interfaces.insert_lang_item(LangItem::IAnyValue);
            match lattice {
                LatticeBits::Value => {
                    td.implemented_interfaces.insert_lang_item(LangItem::IValue);
                }
                LatticeBits::Reference => {
                    td.implemented_interfaces.insert_lang_item(LangItem::IReference);
                }
                LatticeBits::Object => {
                    td.implemented_interfaces.insert_lang_item(LangItem::IObject);
                }
                LatticeBits::Guard => {
                    td.implemented_interfaces.insert_lang_item(LangItem::IGuard);
                    td.implemented_interfaces.insert_lang_item(LangItem::IObject);
                    td.implemented_interfaces.insert_lang_item(LangItem::IReference);
                }
            }

            td.layout_sealed = true;
        }
    }

    /// Section 3.3 Step A: Splicing `*Bundle` / `*Facet` into a `struct` or `partial struct`.
    pub fn apply_bundle_splice(
        &mut self,
        type_id: TypeId,
        splice: &SpliceDirectives,
        interner: &Interner,
        mut ast_arena: Option<&mut pilot_ast::AstArena>,
        diags: &mut Vec<AnalysisDiagnostic>,
    ) {
        let bundle = match self.bundle_defs.get(&splice.bundle_id).cloned() {
            Some(b) => b,
            None => return,
        };
        let (is_sealed, type_name_sym) = match self.type_defs.get(&type_id) {
            Some(td) => (td.layout_sealed, td.name),
            None => return,
        };

        // 1. Layout-Seal Guard for `partial struct`
        if is_sealed && !bundle.is_method_only_facet() {
            let b_name = interner.resolve(bundle.name);
            diags.push(AnalysisDiagnostic::partial_struct_cannot_splice_fields(
                splice.span,
                b_name,
            ));
            return;
        }

        // 2. Field Splicing (Primary `struct` Only)
        for bundle_field in &bundle.fields {
            if splice.excluded_symbols.contains(&bundle_field.name) {
                continue;
            }
            let target_name = splice
                .renamed_symbols
                .get(&bundle_field.name)
                .copied()
                .unwrap_or(bundle_field.name);

            let existing_field_info = self
                .type_defs
                .get(&type_id)
                .and_then(|td| td.field_index.get(&target_name).map(|&idx| (idx, td.fields[idx].ty)));

            if let Some((_existing_idx, existing_ty)) = existing_field_info {
                let is_coalesced = splice.coalesce_fields.contains(&target_name)
                    || splice.coalesce_fields.contains(&bundle_field.name);
                if is_coalesced {
                    let (_, existing_align) = self.type_size_align(existing_ty);
                    let (_, bundle_align) = self.type_size_align(bundle_field.ty);
                    if self.prune(existing_ty) != self.prune(bundle_field.ty)
                        || existing_align != bundle_align
                    {
                        diags.push(AnalysisDiagnostic::coalesce_field_type_mismatch(
                            splice.span,
                            interner.resolve(target_name),
                        ));
                    }
                    // Share existing byte_offset without adding a duplicate field to type_def.fields!
                } else {
                    diags.push(AnalysisDiagnostic::bundle_field_collision(
                        splice.span,
                        interner.resolve(target_name),
                    ));
                }
            } else {
                let (f_size, f_align) = self.type_size_align(bundle_field.ty);
                let f_align = f_align.max(1);
                if let Some(td) = self.type_defs.get_mut(&type_id) {
                    let current_offset = if td.fields.is_empty() { 0 } else { td.size };
                    let aligned_offset = (current_offset + f_align - 1) & !(f_align - 1);
                    let idx = td.fields.len();
                    td.fields.push(FieldDef {
                        name: target_name,
                        ty: bundle_field.ty,
                        byte_offset: aligned_offset,
                        visibility: bundle_field.visibility,
                        source_bundle: Some(bundle.id),
                        span: bundle_field.span,
                    });
                    td.field_index.insert(target_name, idx);
                    td.align = td.align.max(f_align);
                    let raw_end = aligned_offset + f_size;
                    td.size = (raw_end + td.align - 1) & !(td.align - 1);
                }
            }
        }

        // 3 & 4. AST Field-Access Rewriting + Tier 2 Method Insertion & Static Delegation Registration
        for method in &bundle.methods {
            let cloned_fn_id = if !splice.renamed_symbols.is_empty() {
                let new_fn_id = self.alloc_fn_id();
                if let Some(sig) = self.method_signatures.get(&method.fn_id).cloned() {
                    self.method_signatures.insert(new_fn_id, sig);
                }
                self.rewritten_method_fields
                    .insert(new_fn_id, splice.renamed_symbols.clone());
                if let Some(&orig_ast) = self.method_ast_nodes.get(&method.fn_id) {
                    if let Some(ref mut arena) = ast_arena {
                        let rewritten_ast = crate::infer::clone_and_rewrite_self_field_accesses(
                            arena,
                            orig_ast,
                            &splice.renamed_symbols,
                        );
                        self.method_ast_nodes.insert(new_fn_id, rewritten_ast);
                    } else {
                        self.method_ast_nodes.insert(new_fn_id, orig_ast);
                    }
                }
                new_fn_id
            } else {
                method.fn_id
            };

            if let Some(td) = self.type_defs.get_mut(&type_id) {
                // Always register `(bundle.name, method.sym) -> cloned_fn_id` in static_delegates
                // even if `without { method }` was used!
                td.static_delegates
                    .insert((bundle.name, method.sym), cloned_fn_id);
            }

            if splice.excluded_symbols.contains(&method.sym) {
                continue;
            }

            let target_sym = splice
                .renamed_symbols
                .get(&method.sym)
                .copied()
                .unwrap_or(method.sym);

            let new_entry = MethodEntry {
                sym: target_sym,
                fn_id: cloned_fn_id,
                tier: MethodTier::Tier2BundleOrFacetSplice,
                source_origin: OriginId::Bundle(bundle.id),
                is_static: method.is_static,
                is_mutating_var: method.is_mutating_var,
                is_pure: method.is_pure,
                span: method.span,
            };

            let mut collision_detected = false;
            if let Some(td) = self.type_defs.get_mut(&type_id) {
                match td.method_table.get(&target_sym) {
                    Some(existing) if existing.tier == MethodTier::Tier1ExplicitStructMember => {
                        // Tier 1 silently shadows Tier 2
                    }
                    Some(existing)
                        if existing.tier == MethodTier::Tier2BundleOrFacetSplice
                            && existing.source_origin != OriginId::Bundle(bundle.id) =>
                    {
                        let orig_a = existing.source_origin;
                        let orig_b = OriginId::Bundle(bundle.id);
                        self.pending_tier2_collisions
                            .insert((type_id, target_sym), (orig_a, orig_b, splice.span));
                        collision_detected = true;
                    }
                    _ => {
                        td.method_table.insert(target_sym, new_entry);
                        self.pending_iface_collisions.remove(&(type_id, target_sym));
                    }
                }
            }

            let m_name = interner.resolve(target_sym);
            let t_name = if type_name_sym.0 != 0 {
                interner.resolve(type_name_sym)
            } else {
                ""
            };
            // Clear any earlier missing-contract or diamond-interface-default diagnostic now satisfied by Tier 2
            diags.retain(|d| {
                !(d.code == "MissingInterfaceContract"
                    && d.message.contains(&format!("'{}'", m_name))
                    && (t_name.is_empty() || d.message.contains(&format!("'{}'", t_name))))
                    && !(d.code == "AmbiguousInterfaceDefault"
                        && d.message.contains(&format!("'{}'", m_name)))
            });

            if collision_detected {
                diags.push(AnalysisDiagnostic::ambiguous_bundle_method_collision(
                    splice.span,
                    m_name,
                ));
            }
        }
    }

    /// Inserts an explicit Tier 1 `fn` or `op` member declared directly on `struct` or `partial struct`.
    pub fn insert_tier1_method(
        &mut self,
        type_id: TypeId,
        mut entry: MethodEntry,
        sig: Option<MethodSignature>,
        interner: &Interner,
        diags: &mut Vec<AnalysisDiagnostic>,
    ) {
        entry.tier = MethodTier::Tier1ExplicitStructMember;
        entry.source_origin = OriginId::Struct(type_id);
        let sym = entry.sym;
        if let Some(s) = sig {
            self.method_signatures.insert(entry.fn_id, s);
        }
        let type_name_sym = if let Some(td) = self.type_defs.get_mut(&type_id) {
            td.method_table.insert(sym, entry);
            td.name
        } else {
            return;
        };
        // Tier 1 resolves any pending Tier 2 bundle collision or Tier 3/4 interface default collision
        self.pending_tier2_collisions.remove(&(type_id, sym));
        self.pending_iface_collisions.remove(&(type_id, sym));
        let m_name = interner.resolve(sym);
        let t_name = if type_name_sym.0 != 0 {
            interner.resolve(type_name_sym)
        } else {
            ""
        };
        diags.retain(|d| {
            !(d.code == "AmbiguousBundleMethodCollision" && d.message.contains(&format!("'{}'", m_name)))
                && !(d.code == "AmbiguousInterfaceDefault" && d.message.contains(&format!("'{}'", m_name)))
                && !(d.code == "MissingInterfaceContract"
                    && d.message.contains(&format!("'{}'", m_name))
                    && (t_name.is_empty() || d.message.contains(&format!("'{}'", t_name))))
        });
    }

    /// Helper: checks whether `sym` is a required contract or default method on any ancestor of `iface_id`.
    fn method_exists_in_super_interfaces(&self, iface_id: InterfaceId, sym: SymbolId) -> bool {
        let iface = match self.interface_defs.get(&iface_id) {
            Some(i) => i,
            None => return false,
        };
        let mut stack = iface.super_interfaces.clone();
        let mut visited = FxHashSet::new();
        while let Some(sup_id) = stack.pop() {
            if !visited.insert(sup_id) {
                continue;
            }
            if let Some(sup_def) = self.interface_defs.get(&sup_id) {
                if sup_def.required_contracts.iter().any(|c| c.sym == sym)
                    || sup_def.default_methods.iter().any(|m| m.sym == sym)
                {
                    return true;
                }
                stack.extend(sup_def.super_interfaces.iter().copied());
            }
        }
        false
    }

    /// Collects `iface_id` and all its super-interfaces in topological base-first order.
    fn collect_interfaces_base_first(&self, root_id: InterfaceId) -> Vec<InterfaceId> {
        let mut order = Vec::new();
        let mut visited = FxHashSet::new();
        fn dfs(
            arena: &TypeArena,
            cur: InterfaceId,
            visited: &mut FxHashSet<InterfaceId>,
            order: &mut Vec<InterfaceId>,
        ) {
            if !visited.insert(cur) {
                return;
            }
            if let Some(def) = arena.interface_defs.get(&cur) {
                for &sup in &def.super_interfaces {
                    dfs(arena, sup, visited, order);
                }
            }
            order.push(cur);
        }
        dfs(self, root_id, &mut visited, &mut order);
        order
    }

    /// Section 3.3 Step B: Interface Inheritance & Tier 3 / Tier 4 Default Method Resolution.
    pub fn attach_interface_to_type(
        &mut self,
        type_id: TypeId,
        iface_id: InterfaceId,
        interner: &Interner,
        diags: &mut Vec<AnalysisDiagnostic>,
    ) {
        if let Some(td) = self.type_defs.get_mut(&type_id) {
            if !td.declared_interfaces.contains(&iface_id) {
                td.declared_interfaces.push(iface_id);
            }
        } else {
            return;
        }

        // 1. Recursively traverse iface.super_interfaces in topological base-first order.
        let topo_ifaces = self.collect_interfaces_base_first(iface_id);

        for cur_iface_id in topo_ifaces {
            let cur_iface = match self.interface_defs.get(&cur_iface_id).cloned() {
                Some(i) => i,
                None => continue,
            };

            // 2. Set iface.id in type_def.implemented_interfaces (BitSet)
            if let Some(td) = self.type_defs.get_mut(&type_id) {
                td.implemented_interfaces.insert_interface(cur_iface.id);
            }

            // 3. Process default_methods on cur_iface
            for default_method in &cur_iface.default_methods {
                let overrides_super =
                    self.method_exists_in_super_interfaces(cur_iface.id, default_method.sym);
                let computed_tier = if overrides_super {
                    MethodTier::Tier3SubInterfaceDefault
                } else {
                    MethodTier::Tier4BaseInterfaceDefault
                };

                let mut entry = default_method.clone();
                entry.tier = computed_tier;
                entry.source_origin = OriginId::Interface(cur_iface.id);

                let mut diamond_collision = false;
                if let Some(td) = self.type_defs.get_mut(&type_id) {
                    td.static_delegates
                        .insert((cur_iface.name, default_method.sym), default_method.fn_id);

                    match td.method_table.get(&default_method.sym) {
                        None => {
                            td.method_table.insert(default_method.sym, entry);
                        }
                        Some(existing) if existing.tier < computed_tier => {
                            td.method_table.insert(default_method.sym, entry);
                            self.pending_iface_collisions
                                .remove(&(type_id, default_method.sym));
                            let m_name = interner.resolve(default_method.sym);
                            diags.retain(|d| {
                                !(d.code == "AmbiguousInterfaceDefault"
                                    && d.message.contains(&format!("'{}'", m_name)))
                            });
                        }
                        Some(existing)
                            if existing.tier == computed_tier
                                && existing.source_origin != OriginId::Interface(cur_iface.id) =>
                        {
                            self.pending_iface_collisions.insert(
                                (type_id, default_method.sym),
                                (
                                    existing.source_origin,
                                    OriginId::Interface(cur_iface.id),
                                    default_method.span,
                                ),
                            );
                            diamond_collision = true;
                        }
                        _ => {}
                    }
                }

                if diamond_collision {
                    diags.push(AnalysisDiagnostic::ambiguous_interface_default(
                        default_method.span,
                        interner.resolve(default_method.sym),
                    ));
                }
            }

            // 4. Verify Required Contracts
            let type_name_sym = self.type_defs.get(&type_id).map(|td| td.name).unwrap_or(SymbolId(0));
            let type_name = if type_name_sym.0 != 0 {
                interner.resolve(type_name_sym).to_string()
            } else {
                format!("Type#{}", type_id.0)
            };
            let iface_name = if cur_iface.name.0 != 0 {
                interner.resolve(cur_iface.name).to_string()
            } else {
                format!("Interface#{}", cur_iface.id.0)
            };

            for req_sig in &cur_iface.required_contracts {
                let satisfied = if let Some(td) = self.type_defs.get(&type_id) {
                    if let Some(found) = td.method_table.get(&req_sig.sym) {
                        let effect_ok = (!req_sig.is_pure || found.is_pure)
                            && (req_sig.is_mutating_var == found.is_mutating_var)
                            && (req_sig.is_static == found.is_static);
                        let sig_ok = if let Some(found_sig) = self.method_signatures.get(&found.fn_id) {
                            found_sig.params.len() == req_sig.params.len()
                                && found_sig
                                    .params
                                    .iter()
                                    .zip(req_sig.params.iter())
                                    .all(|(&a, &b)| self.prune(a) == self.prune(b) || a == TypeId::ANY || b == TypeId::ANY)
                                && (self.prune(found_sig.ret) == self.prune(req_sig.ret)
                                    || found_sig.ret == TypeId::ANY
                                    || req_sig.ret == TypeId::ANY)
                        } else {
                            true
                        };
                        effect_ok && sig_ok
                    } else {
                        false
                    }
                } else {
                    false
                };

                if !satisfied {
                    diags.push(AnalysisDiagnostic::missing_interface_contract(
                        req_sig.span,
                        &type_name,
                        &iface_name,
                        interner.resolve(req_sig.sym),
                    ));
                }
            }
        }
    }

    /// O(1) verification that `type_id` implements `iface_id` via `TypeDef::implemented_interfaces` (`BitSet`).
    #[inline]
    pub fn implements_interface(&self, type_id: TypeId, iface_id: InterfaceId) -> bool {
        let pruned = self.prune(type_id);
        if let Some(td) = self.type_defs.get(&pruned) {
            return td.implemented_interfaces.contains_interface(iface_id);
        }
        false
    }
}

