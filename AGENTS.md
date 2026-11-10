# AGENTS.md — Pilot Language AI Coding & Architecture Guidelines

> **Master Companion References:** Always consult [`VISION.md`](file:///home/moony/Documents/dev/projects/Pilot/VISION.md) (North Star philosophy, syntax pairings, Universal Foreign Adapter, `.pchor`, and toolchain design), [`MILESTONES_100.md`](file:///home/moony/Documents/dev/projects/Pilot/MILESTONES_100.md) (`M1` $\rightarrow$ `M50` Pilot 1.0 $\rightarrow$ `M100` Pilot 2.0 roadmap), and [`imp_type.md`](file:///home/moony/Documents/dev/projects/Pilot/imp_type.md) (type selection decision matrix based on projected values, cardinality, domain constraints, and 2-bit hardware lattice rules) when designing or constructing any implementation.

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
  - **Contraction is PAUSED until `F8` (decided 2026-11-09, see `MILESTONES_100.md` Phase F and `Audit.md`).** The seed must execute the whole language correctly before it can be replaced, so Phase F milestones `F1`–`F3` (and later the process/file intrinsics of `F6`) may **grow** `crates/pilot-*`. While paused:
    - Rust growth is limited to execution-engine correctness: the tagged value model, lowering of every AST node kind, operators, module linking, error surfacing and exit codes, and the small set of OS intrinsics Pilot cannot provide itself (stdin, process spawn, file write). The 14-seed-token rule, the zero-hardcoded-type-name rule, and the ban on user-facing tools or domain semantics in Rust still apply.
    - Write new Rust in normal `rustfmt` style. Do not pack code onto long lines to satisfy a line count.
    - The gate enforces the `<= 1,500` LOC law per Rust file and a total budget instead of the per-crate ratchet. The single source of truth is `seed_contraction_paused()`, `paused_max_total_rust_loc()` and `paused_max_rust_file_loc()` in `selfhost/gate/manifest.pilot`; milestone tests read the same values through `crates/pilot-analysis/tests/common/seed_policy.rs`. `pilot gate --ratchet-update` is refused while paused.
    - Resume contraction after `F8` by setting `seed_contraction_paused()` to `false`.
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
- **No Fake Results — Leave a Seam Instead:** Never return static, pre-computed, caller-supplied or dataset-specific values in place of real behaviour, and never use fake pointers, mock strings, or values chosen to satisfy a test. Never leave `// TODO` or `unimplemented!()`.
  - **When something cannot be made real yet** because what it connects to does not exist, stop the code at a **seam**: `seam("<area>.<what>", "<what is missing>")` as the body of the empty entry point, or at the hand-off of the exit point. A seam always fails at run time with `SEAM001` and a call trace, so it can never pass for a working feature.
  - **When a value cannot exist yet** because its real type cannot be represented (for example a typed pointer before `ptr<T>` works), hold a **pending value**: a field or variable of type `Pending<T>` built with `pending("<area>.<what>", "<what is missing>")` (`stdlib/core/pending.pilot`). It can be stored and passed around, and reading it (`.get()`) fails with `SEAM001` like a seam. Never stand an integer, an empty string or a zero in for it.
  - **When working code uses a temporary representation** that differs from the spec (an integer address for `ptr<T>`, FNV for BLAKE3, Linux errno numbers on every target), mark the field, type or function with `[StandIn("<area>.<what>", "<what the spec calls for>", "<how it differs>", "<owner>")]`. A stand-in computes real results of the wrong kind; it is never a value chosen to satisfy a test (that is a fake). Name the stand-in in the `blocked_by` of every wiring ledger row it affects; those rows stay at `DRAFT` or below until it is replaced. Expected values never go on a seam, pending value or stand-in: they belong in tests, from a source outside the code.
  - **When the engine or library gives a wrong answer** (a bug, not a missing piece), do not quietly code around it. Write a repro program in `tests/conformance/known_bugs/<name>.pilot` whose `.stdout`/`.exit` hold the *correct* output (from a reference, not from the buggy run), and register it in `bug_registry()` in `selfhost/gate/ledger/seams.pilot` (id, component, summary, expected, actual, repro, owner). The conformance runner requires every known-bug program to keep failing and reports when one starts passing. If you must shape code around the bug, mark it `[Workaround("<bug id>", "<what was done instead>")]`; when the bug is fixed, move its program up into `tests/conformance/`, delete its entry, and undo every workaround that names it. Never change a test's expected value to match a bug.
  - **Register every seam, pending value, stand-in and known bug** in `selfhost/gate/ledger/seams.pilot` (`seam_registry()` with kind `::Entry`, `::Exit` or `::Value`; `stand_in_registry()`; `bug_registry()`). `pilot gate` (the `seams` gate, `GATE011`–`GATE020`) fails on an unregistered id, a stale entry, a duplicate id, a seam registered with the wrong kind, a ledger row naming an unknown id, a stand-in no ledger row names, a row above `DRAFT` resting on a stand-in, a workaround naming an unregistered bug, or a known bug without its repro program. Connecting a seam or replacing a stand-in means writing the real code and deleting its entry in the same change.
  - **Test each side of a seam on its own** with real inputs and answers the code did not choose (a reference tool's output, published test vectors, a conformance program's expected output). A test passes a fake if replacing the function body with its expected value would still pass it; such a test proves nothing. A test may assert that a path stops at its seam (`SEAM001`).
  - **Record how far each capability works** in `selfhost/gate/ledger/wiring.pilot` (`SPEC`, `DRAFT`, `CORE`, `WIRED`, `COMPLETE`, or `SIMULATED` for legacy fakes) with evidence, in the same change that moves it. Read the ledger before assuming any feature works. Only the project owner sets `COMPLETE`.
  - **Reporting a blocker is success; faking a result is failure.** When a task cannot be finished for real, leave the seam, pending value, stand-in or known-bug entry, update both ledgers, and say plainly what is not done.

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
  - **Ring 2 (`std.*`):** Hosted standard library (`std.col`, `std.text`, `std.io`, `std.fs`, `std.net`, `std.http`, `std.sync`, `std.chan`, `std.time`, `std.math`, `std.rand`, `std.crypto`, `std.db`, `std.serde.*`, `std.compress`, `std.proc`, `std.cli`, `std.test`, `std.bench`, `std.log`, `std.polyglot.*` partitioned cleanly into execution-tier `std.polyglot.prog.*` and data-tier `std.polyglot.transport.*`). Never name modules after Python/foreign clones (`pickle`, `urllib`, `pandas`, `unittest`, `tomllib`, `shutil`).
  - **Ring 3 (`pilot.*`):** First-class Compiler-as-a-Library (`pilot.lex`, `pilot.cst`, `pilot.ast`, `pilot.types`, `pilot.ir`, `pilot.fmt`, `pilot.db`, `pilot.vm`, `pilot.diag`, `pilot.abi`, `pilot.chor`, `pilot.pkg`, `pilot.mcp`).
- **Mandatory Type Selection & Implementation Rules ([`imp_type.md`](file:///home/moony/Documents/dev/projects/Pilot/imp_type.md)):**
  When constructing any implementation, data structure, or domain model, agents MUST consult and strictly obey [`imp_type.md`](file:///home/moony/Documents/dev/projects/Pilot/imp_type.md):
  1. **Predefined Finite Sets ($\le 19$ items):** Implement as `enum : u8/u16` with target-typed `::Variant`. Never use stringly-typed constants or magic integers.
  2. **Large Sets ($20+$ varieties / elements):** Implement as table-driven `array<T, N>` or `comptime Map<K, V>` open registries with integer ID handles to prevent closed, rigid switches.
  3. **Fixed Composite Records:** Implement as `[Layout(c_abi)] struct : IValue` (`0b00`) with trivial `memcpy` and zero allocation overhead.
  4. **Varying Implementations / Polymorphic Behavior:** Implement as an `interface` contract with concrete `struct`s, composing shared state via `*Bundle` and shared methods via `*Facet`.
  5. **Orthogonal Combinations:** Implement as `bitflags : u32/u64`, never multiple booleans.
  6. **Generics:** Always constrain type parameters (`<T : IConstraint>`), never write unconstrained `<T>` with runtime reflection guesswork.
  7. **Unambiguous Absences & Failures:** Represent absence as `opt<T>` (`T?`) and expected domain errors as `res<T, E>` (`T ! E`), never sentinels (`null`, `-1`).
  8. **Memory Boundaries:** Distinguish read-only borrowed views (`0b01` `...View` / `slice<T>`), owned dynamic allocations (`0b10` `...Buffer` / `List<T>`), and scoped cleanup resources (`0b11` `...Guard`).
  9. **Single Source of Truth & Targeted Importing:** Types must never be duplicated, copied, or re-declared across files. Always define a type once in its canonical owning module and import it into any file that needs it (`use ...;`). Only import what you need instead of the entire library unless you need the entire library or are importing several items from that library.
  10. **Intrinsic Encapsulation (Never Use Raw `__engine` Directly):** Never call `__engine(...)` directly to manipulate data structures, collections, or runtime operations in user, selfhost, or stdlib code. Always use strongly-typed encapsulating types (like `Map<K, V>`, `List<T>`, `SliceBuffer<T>`, `fs.*`, `time.*`) that incorporate these intrinsics safely within the 2-bit hardware type lattice.

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
Interoperability with foreign languages is partitioned into two distinct architectural subsystems: **Programming Languages** (execution, native ABIs, in-process runtimes, and zero-copy memory) and **Transport Languages** (structural data representations, tree/DOM manipulation, nested querying, and serialization).

### 7.1 Programming Languages Subsystem (`std.polyglot.prog.*` / `std.polyglot.<lang>`)
Covers C, C++, Rust, Zig, Python, JS/TypeScript, WebAssembly, JVM, .NET, and Swift at native register speed with minimal-to-zero overhead:
1. **Zero Manual Glue Files (`use <lang>`):** Absorb foreign headers, crates, `.pyi` stubs, `.d.ts` declarations, `.wit` components, and `.sql`/OpenAPI/`.proto` schemas directly at `comptime` through open `PilotDb` absorber registries.
2. **In-Process Execution Only (Zero Subprocess-Per-Call):** Call native ABIs at `0 ns` register speed via declarative `pilot.abi.*` `AbiFramePlan`s, and host managed runtimes (`libpython` PEP 384/Vectorcall, Node-API v9/QuickJS, WASM, JNI, CLR) strictly in-process. Never spawn `python3`/`node` subprocesses or JSON-serialize buffers to invoke library code.
3. **Zero-Copy `0b01` Memory Sharing:** Map foreign `(ptr, len)`, `&[T]`, `std::span<T>`, PEP 3118 `Py_buffer`/NumPy/DLPack/Arrow, JS `TypedArray`, and WASM linear memory directly onto `0b01` `slice<T>` and `stride<T>` with `0` byte copies and `ESCAPE001`/`ESCAPE004` static borrow enforcement.
4. **Perceus `0b10` & `0b11` Cross-Runtime Safety:** Bind foreign heap resources to `0b10 : IObject` (`ForeignHandle<T>` + `ThreadAffineDropQueue`), bind runtime execution scopes (GIL, N-API `HandleScope`, buffer leases) to `0b11 : IGuard`, and catch all foreign exceptions/panics/traps at the ABI boundary into `res<T, ForeignErr>`.
5. **Zero-Cost `adapter` Idiom Lifting:** Compile prefix stripping, `(ptr, len)` slice pairs, stack out-parameters, status-code lifting, closure trampolines, and getter/setter properties into force-inlined `0`-allocation `PilIR`.

### 7.2 Transport Languages Subsystem (`std.polyglot.transport.*`)
Covers serialization, configuration, and data-interchange formats: **JSON**, **YAML/YML**, **XML**, and **TOML**:
1. **First-Class Dynamic AST/DOM Records:** Strongly typed AST nodes (`JsonValue`, `YamlNode`, `XmlNode`, `TomlValue`) implementing the 2-bit lattice (`0b00: IValue` for scalar leaves/pairs and `0b10: IObject` for composite trees).
2. **Full Bidirectional Manipulation:** Native query and mutation APIs (`get`, `set`, `has`, `remove`, `push`, `get_at`, `len`, `query_path("a.b.c")`, `find_first`, `find_all`) enabling Pilot code to inspect, alter, and build transport payloads in memory without string hacking.
3. **Lossless Emitter / Serializer & Parser:** Format-compliant serializers (`stringify()`, `emit()`) and parsers (`json_parse`, `yaml_parse`, `xml_parse`, `toml_parse`) operating directly between Pilot strings and structured trees.

### 7.3 Phased Roadmap: Cross-Language Generative Metaprogramming
Enables natural embedding and execution of Pilot generative code inside foreign host languages across 4 defined phases:
- **Phase 1 — Bidirectional Transport & AST Representation [COMPLETE]:** Full bidirectional DOM/AST manipulation for JSON, YAML, XML, and TOML via `std.polyglot.transport.*`.
- **Phase 2 — Homoiconic Quote & Template Synthesis:** Pilot `quote { ... }` and `ast<T>` metaprogramming compiles foreign-language source templates and structured configuration payloads directly into typed transport trees.
- **Phase 3 — In-Process Foreign Macro Provider (`pilot-proc-macro` / Foreign Absorbers):** Foreign compiler plugins (e.g. Rust `proc_macro`, Python import hooks, Babel/SWC plugins) call into Pilot's in-process compiler library to execute generative compile-time expansions.
- **Phase 4 — Universal Embedded Generative Blocks:** Direct inline multi-language code embedding (`pilot in rust`, `pilot in python`) where Pilot generative blocks synthesize types, bindings, and algorithms natively into the host language compilation pipeline.

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

### 9.1 Definition of Done (what "start milestone X" asks for)
An agent asked to start or finish a milestone or Phase F step is done when every point below holds. Passing tests the agent wrote for itself is not, on its own, done.
1. **Acceptance tests first, then frozen.** Before implementing, write the milestone's acceptance tests from its prose: conformance programs (`tests/conformance/*.pilot` with `.stdout`/`.stderr`/`.exit`) or CLI runs. Their expected output comes from outside the code under test: a reference tool, published test vectors, a spec example or hand-worked arithmetic stated in the test. Include failure cases (bad input gives the documented error code). Once implementation starts, do not loosen or delete them. If one is wrong, say so in the report and let the owner decide.
2. **Reachable through the real toolchain.** Each deliverable is shown working through `pilot run`, `pilot <command>` or a conformance program, not only through a library function called from a Rust test.
3. **No fakes.** Nothing returns a fixed, caller-supplied or test-fitted value in place of real behaviour. Anything that cannot be built yet stops at a registered seam (Section 1), with a test that the path fails with `SEAM001`. Every new test must fail if the function body is replaced with its expected value.
4. **All gates pass:** `pilot gate --full`, including `seams`, and the full `cargo test --workspace`.
5. **Mandated deletions are done** in the same change: code, files and tests the milestone says it replaces.
6. **Ledgers updated.** Every capability the milestone touches has a row in `selfhost/gate/ledger/wiring.pilot` with its real level and evidence, and every seam left is in `selfhost/gate/ledger/seams.pilot`. An agent may raise a row to at most `WIRED`; only the project owner sets `COMPLETE` or changes a milestone's `[STATUS: ..]` tag to complete.
7. **Report what is not done first.** The final report opens with each seam left, each ledger row below `WIRED`, each skipped step and each acceptance test believed wrong, then what works with the command that shows it. A milestone that honestly ends at `CORE`, blocked at a named seam, is a successful outcome.

---

## 10. Multiplatform Portability & Cross-Target Invariance
- **Tier 1 Native Platforms (Linux, macOS, Windows):** The `pilot` toolchain, runtime, self-hosted compiler, and standard library MUST run natively and pass all gates on Linux (`x86_64`, `aarch64`, `riscv64`), macOS (Apple Silicon `aarch64`, Intel `x86_64`), and Windows (`x86_64`, `aarch64`).
- **Tier 2 WebAssembly & Tier 3 Embedded/Mobile:** First-class compilation to `wasm32-wasip2`, `wasm32-unknown-unknown`, Android (`aarch64-linux-android`), iOS (`aarch64-apple-ios`), and freestanding bare-metal/RTOS (`thumbv7em`, `riscv32`).
- **Canonical Path & Line-Ending Invariance:** Always normalize filesystem paths internally using `/` (forward slashes). Transparently handle Windows drive letters (`C:/...`) and UNC paths (`//...`) without leaking host OS quirks into compiler hashes or spans. Losslessly accept `\n`, `\r\n`, and `\r` line endings in CST/AST without column/line drift.
- **OS-Agnostic Core (`RING001`) vs Native Hosted `std.*`:** `core.*` must strictly contain 0 OS syscalls or libc dependencies. Hosted `std.*` delegates to native high-performance OS backends (Linux `io_uring`/`epoll`, macOS `kqueue`, Windows `IOCP`/Overlapped I/O, WASI `poll_oneoff`) while maintaining identical public signatures.
- **ABI & Calling Convention Fidelity:** Codegen and FFI bridges must respect target calling conventions declaratively: SysV AMD64, Microsoft x64 (with mandatory 32-byte shadow space), ARM64 AAPCS64 (handling Darwin vs Linux stack variadics and HFA packing), RISC-V LP64D, and WASM Canonical ABI.
- **Zero-Host Cross-Compilation:** Cross-compile from any host OS to any target triple (`--target <triple>`) out of the box with zero requirement for target-host development headers.
