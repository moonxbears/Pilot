# AGENTS.md — Pilot Language AI Coding & Architecture Guidelines

## 1. Implementation Integrity & Self-Hosting Ideology
- **Zero Shortcuts:** Never return static, pre-computed, or dataset-specific values to make a test pass. Implement the complete underlying algorithm. No assumptions should be made of elements between pilot and Rust that is not strictly rust engine seeded keywords.
- ** No Assuming:** Don't assume that Pilot and Rust know the same things. A good exercise to run is, if I change something in pilot, will the underlying rust engine account for that or will it break because of specific hard-coded values? (As a hypothetical, if I changed all instances of char to c32 in pilot, would rust break?). Only 14 seed keywords should be hardcoded because they come from the underlying rust engine.
- **No Elided Logic or Silent Fallbacks:** Never leave `// TODO`, `unimplemented!()`, or stubbed branches. Handle all states explicitly via `result<T, E>` (`T ! E`) or `option<T>` (`T?`).
- **Pilot First (Self-Hosted Core):** The Rust workspace (`crates/pilot-*`) is a minimal zero-GC hardware/CFG/SSA/Perceus engine with **strictly 14 seed keywords** (`__engine`, `struct`, `interface`, `fn`, `let`, `ref`, `if`, `loop`, `return`, `yield`, `defer`, `comptime`, `quote`, `asm`). Whenever logic can be written in Pilot without performance loss, implement it in `stdlib/core/*.pilot`, `stdlib/*`, or `selfhost/*.pilot`.
- **Hardware Arena & Register Invariants:**
  - `AstNode` is strictly `16 bytes` (`Copy`, 4 nodes per 64-byte L1 cache line) indexed by 32-bit `NodeId` handles inside `AstArena` (zero heap pointer graphs).
  - `PilotVal` is strictly `16 bytes` (`Copy`) with 14-byte inline Small-String Optimization (SSO), niche-unboxed `option<T>` (`T?`), and register-returned `result<T, E>` (`T ! E`).

---

## 2. Canonical Syntax Rules ("One Keyword Per Responsibility")
1. **Functions & Closures:** Use `fn` (never `def`) and C#-style `(x) => expr` or `(x, y) => { return x + y; }` (never `lambda`).
2. **Blocks & Terminators:** Use `{ ... }` for blocks and `=> expr;` for inline bodies (never Pythonic `:` indentation blocks). End all statements with `;`.
3. **Colon (`:`) Responsibility:** Restricted strictly to type annotations (`x: i64`), inheritance (`: IComparable<i64>, IValue`), `switch` arms (`case 1:`), stepped ranges (`0 .. 10 : 2`), `:ns/kw` keywords, and C++-style distributive header labels (`pub:`, `priv:`, `pub static pure:`).
4. **String Interpolation vs. AST Splicing:** Use `f"hello {x}"` for string interpolation (never `$"..."`). Reserve `$` strictly for homoiconic `quote { $node }` AST splicing.
5. **Generics (`<T>`) vs. Square Brackets (`[]`):**
   - Use `<T>` for all generic type parameters (`slice<T>`, `List<i32>`, `map<str, i64>`). Never use `Type[T]` for generics.
   - `[...]` has 4 unambiguous roles by position:
     1. **Declaration Prefix (`[Attr1, Attr2]`):** Compile-time/reactive attribute pipeline (replaces `@` decorators).
     2. **Type Suffix (`T[]`, `T[N]`, `T[..]`):** Lowers to `slice<T>`, `array<T, N>`, and `stride<T>`.
     3. **Expression Primary (`[1, 2, 3,]`):** Target-typed collection or `[k: v,]` map literal.
     4. **Expression Postfix (`arr[i]`):** Index or sub-slice operation (`IIndexGet` / `IIndexSet`) or dimensional unit literal (`100[m]`).
6. **Collection-Only Optional Trailing Commas:**
   - **Allowed (Optional):** Collection literals (`[1, 2, 3,]`), tuple/struct initializers (`(1, 2,)`, `Vec3 { x: 1, y: 2, }`), `enum` variants, and distributive binding blocks (`let List<i32> { a = [1, 2,], b = [3, 4,], };`).
   - **Disallowed:** Attribute lists `[Attr1, Attr2]`, function parameters/arguments `fn(a: i32, b: i32)`, generic parameters `<T, U>`, and `: IInterface` inheritance lists.
7. **Mutability (`var` Only) & Pattern Branching (`switch` Only):**
   - Use `let` for immutable bindings and `var` for mutable bindings (`var x = 0;`, `var ref<T>`, `var fn` — never `mut`).
   - Use `switch` (never `match`).
8. **Comments:** Use `//` and `/* ... */` (never `#` comments; `#` at line start is reserved strictly for preprocessor directives `#if`, `#define`, `#region`, `#pragma`).
9. **Distributive Header Grouping (`Header:` & `Header { ... }`):**
   - Section labels (`pub:`, `priv:`, `pub static pure:`) apply to subsequent declarations in the current `{ ... }` block.
   - Block prefixes (`let List<i32> { myarr1 = [1, 2, 3], myarr2 = [4, 5, 6,], };`, `var Vec3<f64> { ... };`, `pub partial struct { ... }`, `pub interface { ... }`) distribute modifiers and types across every inner item.

---

## 3. Naming Taxonomy & The 2-Bit Hardware Type Lattice
Every type originates from `IAnyValue` (0 runtime header bytes) and is classified in 1 CPU cycle via bitwise-OR of its fields and `~self()` destructor (`Bit 0` = borrows memory, `Bit 1` = owns `~self()` destructor). Enforce this taxonomy in `Pilot.Core.*` via `[EnforceCoreNaming]`:

| Category / Lattice | Naming Pattern | Canonical Examples | Lifecycle & Perceus Action |
| :--- | :--- | :--- | :--- |
| **`[Lang]` Primitives** | Single-word `lowercase` | `void`, `never`, `bool`, `char`, `i8`..`i128`, `isize`, `u8`..`u128`, `usize`, `f16`..`f128`, `str`, `ptr<T>`, `slice<T>`, `stride<T>`, `array<T, N>`, `ref<T>`, `iso<T>`, `tuple<...>`, `option<T>`, `result<T, E>`, `union<T1, T2>`, `fn<A, R>`, `range<T>`, `span<T>`, `step<T>`, `from<T>`, `until<T>`, `full`, `splat<T>`, `pairs<K, V>`, `expand<T>`, `map<K, V>`, `frame<TSig>`, `type`, `ast` | Hardware register / flat C-ABI primitives bound in `stdlib/core/`. |
| **Interfaces** | `I` + `PascalCase` | `IAnyValue`, `IValue`, `IReference`, `IObject`, `IGuard`, `IEnumerable<T>`, `IAddOperator<T>` | 0-byte compile-time contract, default method provider, and lattice carrier. |
| **Field Bundles** | `PascalCase` + `Bundle` | `RawStorageBundle`, `SpatialHeaderBundle` | Contains stored fields + methods. Spliced via `*Bundle` in primary `struct` only. |
| **Method Facets** | `PascalCase` + `Facet` | `SignedMathFacet`, `ContiguousViewFacet` | 0 stored fields (methods/ops only). Safe to splice in `partial struct`. |
| **`0b00` Value Types** | Unadorned `PascalCase` | `Vec3`, `RangeWindow<T>`, `Dual<T, G>`, `CallExpr<T>` | `: IValue`. Trivial `memcpy`, eligible for `as view` casts and `.pchor` cues. |
| **`0b01` Reference Views** | `...View` / `...Lens` / `...Cursor` | `SliceView<T>`, `OffsetLens<T>` | `: IReference`. Borrowed window; lexical escape checked in `pilot-perceus`. |
| **`0b10` Owned Objects** | `...Buffer` / `...List` / `...Handle` | `SliceBuffer<T>`, `List<T>`, `Signal<T>`, `EffectPlan<T>` | `: IObject`. Has `~self()`; Perceus move-by-default + `drop_reuse`. |
| **`0b11` Scoped Guards** | `...Guard` / `...Scope` / `...Lease` | `MutexGuard<T>`, `ArenaScope`, `TxLease` | `: IGuard`. Borrows memory AND runs `~self()` at scope exit; cannot escape scope. |
| **`[...]` Attributes** | `...Attr` (Def) / `[Name]` (Use) | `struct ClampAttr : IAttribute` $\rightarrow$ `[Clamp(0, 1)]` | Sorted by `weight: i32` at compile time and woven into SSA/getters/setters. |

---

## 4. 4-Tier `MethodTable`, `*Bundle` Splicing & Extension Scoping
- **4-Tier Vertical Precedence (Replaces OOP Base Classes):**
  1. **Tier 1:** Explicit `struct` / `partial struct` member (wins unconditionally).
  2. **Tier 2:** Spliced `*Bundle` / `*Facet` member (fulfills `interface` contracts and overrides Tiers 3–4).
  3. **Tier 3:** Sub-`interface` default method (overrides Tier 4).
  4. **Tier 4:** Base `interface` default method.
- **Horizontal Collision Modifiers:** Resolve overlapping `*Bundle` fields or methods explicitly using `coalesce { id }`, `with { flags as net_flags }`, `without { clear_state }`, and static delegation `BundleName.method(var ref self, args...)`.
- **Extensions (`ext` & `use ext`):** Support individual (`pub pure ext fn i64.is_even() -> bool => (self & 1) == 0;`) and block extensions (`pub ext str { ... }`), imported either file-wide (`use Pkg.StrExt;`) or lexically scoped (`use ext StrExt { ... }`).

---

## 5. Lexing, Receivers, Properties, Operators & Call-Quoting Invariants

### 5.1 Two-Way Dot (`PrefixDot` vs. `InfixDot`), Floats, Enums & Cascades
- **`InfixDot` (`.`):** Preceded immediately (no whitespace) by an identifier, literal, `)`, `]`, or `}` (`obj.field`, `tuple.0`).
- **`PrefixDot` (`.`):** Preceded by whitespace or `;`, `(`, `[`, `{`, `,`, `:` (`.reset()`, `0 .. .count`). Desugars to `self.<ident>`. Never place dots adjacent without whitespace (`0.. .count`, not `0...count`).
- **Leading-Zero Floats & `::Variant` Enums:** Floats must start with a digit (`0.5`, never `.5`). Target-typed enum variants use `::Active` (bare `.Active` is reserved for `self.Active`).
- **Fluid Cascades (`..` / `?..`):** Bound at `Precedence::Cascade = 1` (`node..set_title("Master")..set_status("OK");`), mutating/calling on the root receiver and returning it.

### 5.2 Splat & Recursive Flatten Tokens (`*`, `**`, `*..`)
- **`*expr` (`splat<T>` / `ISplatOperator`):** 1-level positional sequence unpack or `*Bundle` splice.
- **`**expr` (`pairs<K, V>` / `IPairsOperator`):** 1-level compile-time structural field/key-value unpack.
- **`*..expr` (`expand<T>` / `IFlattenOperator`):** Recursive deep-flatten to 1D leaf elements via `op flat () -> TLeaf`.

### 5.3 Dunder-Free Lifecycle & Variation B Property/Indexer Setters
- **Lifecycle:** `self(params...)` constructor, `~self()` destructor, and `scope self { enter: ... exit: ... }`.
- **Implicit Properties & Indexers (`self[params...] -> T`):** Omit `property` keyword. Inside `get` and `set` clauses, `field` accesses synthesized backing storage and `value` accesses the incoming setter parameter (`pub balance: f64 { get => field; set => { field = value; } }`). Support type-overloaded setters (`set(scale: f64)`), guarded setters (`set(v: T) if cond`), and ownership setters (`set(iso v: OwnedT)`).

### 5.4 Values-First Errors (`T ! E`), Null-Safety (`T?`), 3-Tier `for` & Call-Quoting (`!`, `!!`, `!&`, `!^`, `!@`, `!?`)
- **Universal Postfix `?` (`ITryOperator`):** Unwraps `result<T, E>` (`T ! E`) or `option<T>` (`T?`) and propagates failure/`none` early. Pair with `guard some x else { ... }`, `expr ?? fallback`, and `expr catch err => ...`. Unchecked postfix `x!` is prohibited (`NULL001`–`NULL005`, `ERR001`–`ERR005`).
- **3-Tier Dynamic `for` Expressions (`loops.rs`):** Preserve `FixedScalar`, `MutableBounds` (`continue a..b;`), and `WorklistStack` (`continue a..b, ...;` backed by `#[repr(C)] RangeWindow<TIter>`) alongside `CollectionYield` (`SliceBuffer<T>`) and `ScalarBreak` (`break val;` + `then fallback`).
- **6 Homoiconic Call-Quoting Modes (Zero-Whitespace `(` Lookahead):**
  - `fn!(a, b)` $\rightarrow$ `CallExpr<TReturn>` (`ShallowQuote`)
  - `fn!!(~a, b)` $\rightarrow$ `ast<TReturn>` (`DeepSplice` with `~` compile-time unquoting)
  - `fn!&(a, b)` $\rightarrow$ `frame<FnSig>` (`AbiFrame` zero-marshalling `#[repr(C)]` struct)
  - `fn!^(a, b)` $\rightarrow$ `Signal<TReturn>` (`ReactiveSignal` glitch-free push-pull node)
  - `fn!@(a, b)` $\rightarrow$ `Dual<TReturn, TGrad>` (`SymbolicAdjoint` compile-time CSE derivative)
  - `fn!?(a, b)` $\rightarrow$ `EffectPlan<TReturn>` (`EffectDryRun` CoW transactional sandbox)

### 5.5 Preserved Domain Paradigms & Two-Axis Visibility (`lvl`)
- **Two-Axis Visibility:** Axis 1 short modifiers (`priv`, `file`, `prot`, `pkg`, `int`, `pub`, `open`, `export`) + Axis 2 compile-time clearance rings (`lvl(N)` / `lvlN` with flow-typed `if lvl >= N` elevation, `LVL001`, `VIS001`).
- **Contracts, DI, Adapters, Real-Time & Polyglot:** Preserve `requires`/`ensures`/`old()`, `context` / `with context` DI, `adapter A -> B` & `each` call vectorization, method group switchboards (`|`, `*`), algebraic type composition (`|`, `&`, `-`) & `state` typestates, `[HardRealtime]` / `[Wcet(50us)]` / `noalloc`, dimensional units (`250ms`, `64KB`), native `database` / `Table<T>` LINQ & SQL, and inline polyglot blocks.