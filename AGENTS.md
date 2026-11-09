# AGENTS.md — Pilot Language AI Coding & Architecture Guidelines

> **Master Companion References:** Always consult [`VISION.md`](file:///home/moony/Documents/dev/projects/Pilot/VISION.md) (North Star philosophy, syntax pairings, Universal Foreign Adapter, `.pchor`, and toolchain design) and [`MILESTONES_100.md`](file:///home/moony/Documents/dev/projects/Pilot/MILESTONES_100.md) (`M1` $\rightarrow$ `M50` Pilot 1.0 $\rightarrow$ `M100` Pilot 2.0 roadmap) when designing or implementing features.

---

## 1. Implementation Integrity, Scalability & The Universal Name-Blind Contract
- **The Strict 14-Token Boundary:** The underlying bootstrap engine (`crates/pilot-*`) is a **Name-Blind Hardware & Structural Physics Engine**. The ONLY words the bootstrap compiler may ever hardcode are the **14 seed tokens**:
  `__engine`, `struct`, `interface`, `fn`, `let`, `ref`, `if`, `loop`, `return`, `yield`, `defer`, `comptime`, `quote`, `asm`.
- **Zero Hardcoded Type or Symbol Names (Primitives AND Composites):**
  - Never hardcode ANY Pilot type name—whether scalar (`"i64"`, `"u8"`, `"f64"`, `"char"`, `"bool"`, `"usize"`) or composite/algebraic (`"str"`, `"slice"`, `"stride"`, `"array"`, `"simd"`, `"opt"`, `"option"`, `"res"`, `"result"`, `"range"`, `"span"`, `"map"`, `"Map"`, `"List"`, `"SliceBuffer"`, `"RangeWindow"`, `"Dual"`, `"Signal"`, `"frame"`)—inside Rust `match` arms, string comparisons, or codegen tables.
- **Scalable Open-Registry Design (Zero Rigid Corners or Magic Counts):**
  - Everything must be extendible, table-driven, and protocol-oriented to prevent duplication or locking into rigid paths. Never hardcode closed `if/else` chains, magic counts, or stdout scraping (`@@WRITE:`). For example, `BootstrapStage` must never hardcode `return 8` for stage count—store stages in a keyed registry/map and query `stages.len()`. Apply this open-registry rule to CLI `[Command]` dispatch, `PilIR` passes (`IPilIrPass`), foreign `use <lang>` absorbers, `[...]` weavers, target backends, and diagnostic catalogs so adding item $N+1$ requires zero core dispatcher rewrites.
- **File Modularity Cap (`<= 1,500` LOC Law):**
  - Keep every `.pilot` and `.rs` file focused on a single cohesive responsibility and strictly `<= 1,500` lines (`MOD001`). Split growing subsystems into clean sub-packages rather than monoliths.
- **Pilot First & Active Rust Engine Contraction:**
  - Implement every feature in Pilot first (`stdlib/core/`, `stdlib/std/`, `stdlib/pilot/`, `selfhost/`). Only touch an underlying bootstrap language when strictly irreducible at the OS/hardware boundary—and even then, drive behavior declaratively from Pilot descriptors (e.g., `pilot.abi.*` `AbiFramePlan`).
  - Treat `crates/pilot-*` as a **shrinking bootstrap seed** (unlocked in `M1` strictly for contraction toward a minimal C11 seed in `M38`–`M39` and a 100% pure-Pilot repository in `M89`). Never add user-facing CLI tools, linters, formatters, or domain semantics in Rust.
- **Same-Milestone Legacy & Cache Purge:**
  - Anytime a replacement is introduced, delete all superseded legacy code, mock stubs, obsolete scripts, foreign-clone shims, and stale `.pilc` blobs within the **same milestone** so dead paths never accumulate.
- **The Dual-Pass Universal Rename Litmus Test:**
  - *Pass 0 (Canonical):* Renaming `char` $\rightarrow$ `c32`, `slice<T>` $\rightarrow$ `window<T>`, `option<T>`/`opt<T>` $\rightarrow$ `maybe<T>`, `result<T, E>`/`res<T, E>` $\rightarrow$ `outcome<T, E>`, and `range<T>` $\rightarrow$ `interval<T>` inside `stdlib/core/*.pilot` MUST compile, type-check, and emit valid LLVM/C/WASM/GPU/ASM/PilVM code with zero seed changes.
  - *Pass 1 (Randomized):* Deterministic seeded identifier permutation (`pilot gate --litmus --seed <u64>`) across all `[Lang]` primitives and receptors MUST pass identically.
- **How the Engine Must Derive All Types (5 Name-Blind Mechanisms):**
  1. **Register & Layout Physics (`[Bits]` & `[Layout]`):** Deduce CPU register classes (`gpr_signed`, `gpr_unsigned`, `simd_float`, `simd_vector`, `raw_ptr`, `unit`, `noreturn`), bit widths, inline array repetition (`repeat`), and niche packing (`niche`) strictly from layout descriptors.
  2. **Structural Field Anatomy (`TypeDef.fields` & `LatticeBits`):** Deduce contiguous views (`{ ptr<T>, usize }` + `0b01`), strided lenses (`{ ptr<T>, usize, isize }` + `0b01`), owned buffers (`{ ptr<T>, usize, usize }` + `0b10`), and bounds pairs (`{ T, T }` + `0b00`) from field types, byte offsets, and the 2-bit lattice (`0b00..0b11`).
  3. **Pure Pilot Syntax Rewrites (`syntax type_construct`):** Symbolic types (`T[]`, `T[N]`, `T[..]`, `T@N`, `T?`, `T!E`, `(A) => R`) are rewritten into canonical AST types by `syntax type_construct` rules in `stdlib/core/`—never by hardcoded name mappings in the parser.
  4. **Operator & Protocol Contracts (`MethodTable`):** Deduce behavior for postfix `expr?` (any type implementing `op ?`), `for` loops (any type implementing `get_enumerator()` or structural bounds), indexing (`IIndexGet` / `IIndexSet`), and implicit value lifting (`implicit` constructors) from the type's resolved `MethodTable`.
  5. **`TypeId` Receptor Slots (`__engine("bind_receptor", ...)`):** Bind lexed literals (`42`, `3.14`, `'a'`, `"hi"`, `true`) and homoiconic call-quote projections (`!`/`call`, `!!`/`splice`, `!&`/`frame`, `!^`/`sig`, `!@`/`grad`, `!?`/`plan`) to the registered `TypeId` handle, populating composite fields by structural index (`0`, `1`, ...) rather than field or type names.
- **Zero Shortcuts or Elided Logic:** Never return static, pre-computed, or dataset-specific values to make a test pass. Never leave `// TODO`, `unimplemented!()`, mock strings, or stubbed branches.

---

## 2. Canonical Syntax Rules & Dual Readability ("One Keyword Per Responsibility")
1. **Functions & Closures:** Use `fn` (never `def`) and C#-style `(x) => expr` or `(x, y) => { return x + y; }` (never `lambda`).
2. **Blocks & Terminators:** Use `{ ... }` for blocks and `=> expr;` for inline bodies (never Pythonic `:` indentation blocks). End all statements with `;`.
3. **Colon (`:`) Responsibility:** Restricted strictly to type annotations (`x: i64`), inheritance (`: IComparable<i64>, IValue`), `switch` arms (`case 1:`), stepped ranges (`0 .. 10 : 2`), `:ns/kw` keywords, and C++-style distributive header labels (`pub:`, `priv:`, `pub static pure:`).
4. **String Interpolation vs. AST Splicing:** Use `f"hello {x}"` for string interpolation (never `$"..."`). Reserve `$` strictly for homoiconic `quote { $node }` AST splicing.
5. **Generics (`<T>`) vs. Square Brackets (`[]`):**
   - Use `<T>` for all generic type parameters (`slice<T>`, `List<i32>`, `Map<str, i64>`). Never use `Type[T]` for generics.
   - `[...]` has 4 unambiguous roles by position:
     1. **Declaration Prefix (`[Attr1, Attr2]`):** Compile-time/reactive attribute pipeline (replaces `@` decorators).
     2. **Type Suffix (`T[]`, `T[N]`, `T[..]`):** Lowers to `slice<T>`, `array<T, N>`, and `stride<T>`.
     3. **Expression Primary (`[1, 2, 3,]`):** Target-typed collection or `[k: v,]` map literal.
     4. **Expression Postfix (`arr[i]`):** Index or sub-slice operation (`m[k]`, `m[k] = v` via `IIndexGet` / `IIndexSet`, paired with short `.get()` / `.set()`) or dimensional unit literal (`100[m]`).
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
10. **Mandatory Dual Readable/Symbolic Pairing Law:**
    - Native built-in types and keywords use short, natural, single-word `lowercase` names (`bool`, `str`, `pkg`, `ext`, `opt`, `res`, `sig`, `grad`, `plan`, `call`, `splice`, `frame`, `splat`, `pairs`, `flat`).
    - Every compact symbolic construct MUST have an equivalent readable `lowercase` counterpart lowering to the **exact same canonical AST/receptor node** (round-trippable via `pilot fmt`):
      - **Types:** `T?` $\leftrightarrow$ `opt<T>` (`option<T>`), `T ! E` $\leftrightarrow$ `res<T, E>` (`result<T, E>`), `T[]` $\leftrightarrow$ `slice<T>`, `T[N]` $\leftrightarrow$ `array<T, N>`, `T[..]` $\leftrightarrow$ `stride<T>`, `T@N` $\leftrightarrow$ `simd<T, N>`.
      - **Call-Quoting:** `f!(x)` $\leftrightarrow$ `call f(x)`, `f!!(~x)` $\leftrightarrow$ `splice f(~x)`, `f!&(x)` $\leftrightarrow$ `frame f(x)`, `f!^(x)` $\leftrightarrow$ `sig f(x)`, `f!@(x)` $\leftrightarrow$ `grad f(x)`, `f!?(x)` $\leftrightarrow$ `plan f(x)`.
      - **Unpacking:** `*x` $\leftrightarrow$ `splat(x)`, `**x` $\leftrightarrow$ `pairs(x)`, `*..x` $\leftrightarrow$ `flat(x)`.

---

## 3. Naming Taxonomy, 3-Ring Namespaces & The 2-Bit Hardware Type Lattice
Every type originates from `IAnyValue` (0 runtime header bytes) and is classified in 1 CPU cycle via bitwise-OR of its fields and `~self()` destructor (`Bit 0` = borrows memory, `Bit 1` = owns `~self()` destructor). Enforce this taxonomy in `Pilot.Core.*` / `core.*` via `[EnforceCoreNaming]`:

| Category / Lattice | Naming Pattern | Canonical Examples | Lifecycle & Perceus Action |
| :--- | :--- | :--- | :--- |
| **`[Lang]` Primitives** | Single-word `lowercase` | `void`, `never`, `bool`, `char`, `i8`..`i128`, `isize`, `u8`..`u128`, `usize`, `f16`..`f128`, `str`, `ptr<T>`, `slice<T>`, `stride<T>`, `array<T, N>`, `simd<T, N>`, `ref<T>`, `iso<T>`, `tuple<...>`, `opt<T>` (`option<T>`), `res<T, E>` (`result<T, E>`), `union<T1, T2>`, `fn<A, R>`, `range<T>`, `span<T>`, `step<T>`, `from<T>`, `until<T>`, `full`, `splat<T>`, `pairs<K, V>`, `expand<T>`, `map<K, V>`, `frame<TSig>`, `sig<T>`, `grad<T, G>`, `plan<T>`, `type`, `ast`, `cst`, `ir` | Hardware register / flat C-ABI primitives bound in `stdlib/core/`. |
| **Interfaces** | `I` + `PascalCase` | `IAnyValue`, `IValue`, `IReference`, `IObject`, `IGuard`, `IEnumerable<T>`, `IAddOperator<T>` | 0-byte compile-time contract, default method provider, and lattice carrier. |
| **Field Bundles** | `PascalCase` + `Bundle` | `RawStorageBundle`, `SpatialHeaderBundle` | Contains stored fields + methods. Spliced via `*Bundle` in primary `struct` only. |
| **Method Facets** | `PascalCase` + `Facet` | `SignedMathFacet`, `ContiguousViewFacet` | 0 stored fields (methods/ops only). Safe to splice in `partial struct`. |
| **`0b00` Value Types** | Unadorned `PascalCase` | `Vec3`, `RangeWindow<T>`, `Dual<T, G>`, `CallExpr<T>` | `: IValue`. Trivial `memcpy`, eligible for `as view` casts and `.pchor` cues. |
| **`0b01` Reference Views** | `...View` / `...Lens` / `...Cursor` | `SliceView<T>`, `OffsetLens<T>`, `TokenCursor` | `: IReference`. Borrowed window; lexical escape checked (`ESCAPE001`–`004`). |
| **`0b10` Owned Objects** | `...Buffer` / `...List` / `...Handle` | `SliceBuffer<T>`, `List<T>`, `Map<K, V>`, `ForeignHandle<T>`, `Signal<T>`, `EffectPlan<T>` | `: IObject`. Has `~self()`; Perceus move-by-default + `drop_reuse`. |
| **`0b11` Scoped Guards** | `...Guard` / `...Scope` / `...Lease` | `MutexGuard<T>`, `ArenaScope`, `TxLease`, `PyGilGuard` | `: IGuard`. Borrows memory AND runs LIFO `~self()` at scope exit; cannot escape scope. |
| **`[...]` Attributes** | `...Attr` (Def) / `[Name]` (Use) | `struct ClampAttr : IAttribute` $\rightarrow$ `[Clamp(0, 1)]` | Sorted by `weight: i32` at compile time and woven into SSA/getters/setters. |

- **3-Ring Standard Library Personality (Zero Python-Clone Module Names):**
  - **Ring 1 (`core.*`):** Freestanding hardware/language core (`0` OS/libc/filesystem/network imports — `RING001`).
  - **Ring 2 (`std.*`):** Hosted standard library (`std.col`, `std.text`, `std.io`, `std.fs`, `std.net`, `std.http`, `std.sync`, `std.chan`, `std.time`, `std.math`, `std.rand`, `std.crypto`, `std.db`, `std.serde.*`, `std.compress`, `std.proc`, `std.cli`, `std.test`, `std.bench`, `std.log`, `std.polyglot.*`). Never name modules after Python/foreign clones (`pickle`, `urllib`, `pandas`, `unittest`, `tomllib`, `shutil`).
  - **Ring 3 (`pilot.*`):** First-class Compiler-as-a-Library (`pilot.lex`, `pilot.cst`, `pilot.ast`, `pilot.types`, `pilot.ir`, `pilot.fmt`, `pilot.db`, `pilot.vm`, `pilot.diag`, `pilot.abi`, `pilot.chor`, `pilot.pkg`, `pilot.mcp`).

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

### 5.2 Splat & Recursive Flatten Tokens (`*`, `**`, `*..` / `splat`, `pairs`, `flat`)
- **`*expr` / `splat(expr)` (`splat<T>` / `ISplatOperator`):** 1-level positional sequence unpack or `*Bundle` splice.
- **`**expr` / `pairs(expr)` (`pairs<K, V>` / `IPairsOperator`):** 1-level compile-time structural field/key-value unpack.
- **`*..expr` / `flat(expr)` (`expand<T>` / `IFlattenOperator`):** Recursive deep-flatten to 1D leaf elements via `op flat () -> TLeaf`.

### 5.3 Dunder-Free Lifecycle & Variation B Property/Indexer Setters
- **Lifecycle:** `self(params...)` constructor, `~self()` destructor, and `scope self { enter: ... exit: ... }`.
- **Implicit Properties & Indexers (`self[params...] -> T`):** Omit `property` keyword. Inside `get` and `set` clauses, `field` accesses synthesized backing storage and `value` accesses the incoming setter parameter (`pub balance: f64 { get => field; set => { field = value; } }`). Support type-overloaded setters (`set(scale: f64)`), guarded setters (`set(v: T) if cond`), and ownership setters (`set(iso v: OwnedT)`).

### 5.4 Values-First Errors (`T ! E` / `res`), Null-Safety (`T?` / `opt`), 3-Tier `for` & Call-Quoting
- **Universal Postfix `?` (`ITryOperator`):** Unwraps `res<T, E>` (`T ! E`) or `opt<T>` (`T?`) and propagates failure/`none` early. Pair with `guard some x else { ... }`, `expr ?? fallback`, and `expr catch err => ...`. Unchecked postfix `x!` is prohibited (`NULL001`–`NULL005`, `ERR001`–`ERR005`).
- **3-Tier Dynamic `for` Expressions (`loops.rs` / `PilIR`):** Preserve `FixedScalar`, `MutableBounds` (`continue a..b;`), and `WorklistStack` (`continue a..b, ...;` backed by `#[repr(C)] RangeWindow<TIter>`) alongside `CollectionYield` (`SliceBuffer<T>`) and `ScalarBreak` (`break val;` + `then fallback`).
- **6 Homoiconic Call-Quoting Modes (Zero-Whitespace `(` Symbolic OR Readable Prefix Keyword):**
  - `fn!(a, b)` / `call fn(a, b)` $\rightarrow$ `CallExpr<TReturn>` (`ShallowQuote`)
  - `fn!!(~a, b)` / `splice fn(~a, b)` $\rightarrow$ `ast<TReturn>` (`DeepSplice` with `~` compile-time unquoting)
  - `fn!&(a, b)` / `frame fn(a, b)` $\rightarrow$ `frame<FnSig>` (`AbiFrame` zero-marshalling `#[repr(C)]` struct)
  - `fn!^(a, b)` / `sig fn(a, b)` $\rightarrow$ `Signal<TReturn>` (`ReactiveSignal` glitch-free push-pull node)
  - `fn!@(a, b)` / `grad fn(a, b)` $\rightarrow$ `Dual<TReturn, TGrad>` (`SymbolicAdjoint` compile-time CSE derivative)
  - `fn!?(a, b)` / `plan fn(a, b)` $\rightarrow$ `EffectPlan<TReturn>` (`EffectDryRun` CoW transactional sandbox)

### 5.5 Preserved Domain Paradigms & Two-Axis Visibility (`lvl`)
- **Two-Axis Visibility:** Axis 1 short modifiers (`priv`, `file`, `prot`, `pkg`, `int`, `pub`, `open`, `export`) + Axis 2 compile-time clearance rings (`lvl(N)` / `lvlN` with flow-typed `if lvl >= N` elevation, `LVL001`, `VIS001`).
- **Contracts, DI, Adapters, Real-Time & Polyglot:** Preserve `requires`/`ensures`/`old()`, `context` / `with context` DI, `adapter A -> B` & `each` call vectorization, method group switchboards (`|`, `*`), algebraic type composition (`|`, `&`, `-`) & `state` typestates, `[HardRealtime]` / `[Wcet(50us)]` / `noalloc`, dimensional units (`250ms`, `64KB`), native `database` / `Table<T>` LINQ & SQL, and inline polyglot blocks.

---

## 6. Everything First-Class, Shared `PilotDb`, `PilIR` & Progressive Learning Curve
- **One Shared Compiler Engine (`PilotDb` + Lossless `CST` + `PilIR`):**
  - Never build disconnected per-tool parsers, AST walkers, or string-concatenated source loaders. Every tool (`check`, `build`, `run`, `fmt`, `doc`, `lsp`, `debug`, `inspect`, `migrate`, `mcp`) must query the single demand-driven incremental `PilotDb` (`pilot.db`), lossless trivia-preserving Red/Green `CST` (`pilot.cst`), and typed block-parameter SSA `PilIR` (`pilot.ir`).
- **Progressive 4-Tier Ergonomics:**
  - Never force Tier-3/4 systems or metaprogramming syntax (lifetime parameters, raw pointers, manual `comptime` plumbing) onto Tier-1/2 everyday code. Simple Pilot code must read naturally with full local inference and zero ceremony, while unlocking zero-cost control when attributes, facets, arenas, `iso`, or `.pchor` are applied.
- **Zero-Black-Box Transparency (`pilot inspect`):**
  - Every implicit compiler action—attribute weave order (`weight: i32`), struct byte offsets/niches, 4-tier `MethodTable` winner, Perceus `drop`/`drop_reuse` insertion, `adapter` lowering, and `.pchor` rewrite—must be inspectable via `pilot inspect` and IDE lenses.

---

## 7. Priority Pillar: Universal Foreign-Language Adapter (`use <lang>` & `adapter`)
Interoperability with C, C++, Rust, Zig, Python, JS/TypeScript, WASM, SQL/OpenAPI/Protobuf, JVM, .NET, and Swift must feel native with minimal-to-zero downsides:
1. **Zero Manual Glue Files (`use <lang>`):** Absorb foreign headers, crates, `.pyi` stubs, `.d.ts` declarations, `.wit` components, and `.sql`/OpenAPI/`.proto` schemas directly at `comptime` through open `PilotDb` absorber registries.
2. **In-Process Execution Only (Zero Subprocess-Per-Call):** Call native ABIs at `0 ns` register speed via declarative `pilot.abi.*` `AbiFramePlan`s, and host managed runtimes (`libpython` PEP 384/Vectorcall, Node-API v9/QuickJS, WASM, JNI, CLR) strictly in-process. Never spawn `python3`/`node` subprocesses or JSON-serialize buffers to invoke library code.
3. **Zero-Copy `0b01` Memory Sharing:** Map foreign `(ptr, len)`, `&[T]`, `std::span<T>`, PEP 3118 `Py_buffer`/NumPy/DLPack/Arrow, JS `TypedArray`, and WASM linear memory directly onto `0b01` `slice<T>` and `stride<T>` with `0` byte copies and `ESCAPE001`/`ESCAPE004` static borrow enforcement.
4. **Perceus `0b10` & `0b11` Cross-Runtime Safety:** Bind foreign heap resources to `0b10 : IObject` (`ForeignHandle<T>` + `ThreadAffineDropQueue`), bind runtime execution scopes (GIL, N-API `HandleScope`, buffer leases) to `0b11 : IGuard`, and catch all foreign exceptions/panics/traps at the ABI boundary into `res<T, ForeignErr>`.
5. **Zero-Cost `adapter` Idiom Lifting:** Compile prefix stripping, `(ptr, len)` slice pairs, stack out-parameters, status-code lifting, closure trampolines, and getter/setter properties into force-inlined `0`-allocation `PilIR`.

---

## 8. `.pchor` Choreography, Unified `pilot` Toolchain & AI Co-Design (`pilot mcp`)
- **Sublanguage & Layout Choreographer (`.pchor`):** Keep domain `struct` and `fn` code clean while `.pchor` blueprints control physical layout (`AoS` $\leftrightarrow$ `SoA<T, align: 64>` / Arena / GPU), execution topology (direct call $\leftrightarrow$ work-stealing M:N $\leftrightarrow$ IPC/RPC), and dependency/WCET guardrails. Enforce the **2-Bit Cue Boundary Law** (`CHOR001`: only `0b00` `IValue` or `iso`-moved `0b10` `IObject` may cross a `cue`).
- **Declarative Unified `pilot` Binary:** Register every `pilot` subcommand (`run`, `build`, `check`, `fmt`, `test`, `bench`, `doc`, `pkg`, `lsp`, `debug`, `mcp`, `repl`, `inspect`, `absorb`, `migrate`, `chor`, `doctor`, `ship`, `studio`, `gate`) via `[Command]` into `CommandRegistry`, auto-deriving CLI help, shell completions, and `pilot mcp` tool schemas from a single source of truth.
- **Lossless CST Codemods (`--fix` & `migrate`):** All automated diagnostic repairs (`pilot check --fix`) and `[Migrate]` API/Edition upgrades (`pilot migrate`) must edit `pilot.cst` losslessly, conserving 100% of surrounding comments and trivia.

---

## 9. Mandatory 3-Gate Milestone Verification Discipline
No milestone (`M1`–`M100` in [`MILESTONES_100.md`](file:///home/moony/Documents/dev/projects/Pilot/MILESTONES_100.md)) is complete until all three automated gates pass (`pilot gate`):
1. **Gate 1 — Adaptability & Hygiene:** `14` seed tokens respected, `0` hardcoded user-space names, all files `<= 1,500` LOC, `0` dead legacy shims/mocks, freestanding `core.*` (`RING001`), and Dual-Pass Universal Rename Litmus green.
2. **Gate 2 — Bootstrap Parity & Compile Speed:** Deterministic Stage-2 == Stage-3 self-compilation fixed point (`pilotc`) + warm incremental `PilotDb` latency within budget.
3. **Gate 3 — Top-of-Chart Performance & Zero Leaks:** `0` unintended hot-path heap allocations, `0` Perceus/foreign handle leaks, and C/Rust-competitive runtime execution verified.
