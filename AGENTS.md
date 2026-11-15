# AGENTS.md — Pilot Language AI Coding & Architecture Guidelines

> **Master Companion References:** Always consult [`VISION.md`](file:///home/moony/Documents/dev/projects/Pilot/VISION.md) (North Star philosophy, syntax pairings, Universal Foreign Adapter, `.pchor`, and toolchain design), [`MILESTONES_100.md`](file:///home/moony/Documents/dev/projects/Pilot/MILESTONES_100.md) (Phase F, `M30`–`M50` Pilot 1.0, and `M51`–`M100` Pilot 2.0 Reframed Roadmap), [`imp_type.md`](file:///home/moony/Documents/dev/projects/Pilot/imp_type.md) (type selection decision matrix and 2-bit hardware lattice rules), [`pilot.md`](file:///home/moony/Documents/dev/projects/Pilot/pilot.md) (living map of all Pilot implementations & responsibilities), [`rust.md`](file:///home/moony/Documents/dev/projects/Pilot/rust.md) (living map of all Rust implementations & irreducible justifications), [`proposals.md`](file:///home/moony/Documents/dev/projects/Pilot/proposals.md) (mandatory owner-approval gate for any Rust file change), [`docs/keyword_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/keyword_matrix.md) (verified per-engine status of every registered keyword), and [`docs/concept_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/concept_matrix.md) (verified per-engine status & Pilot/Rust ownership of every language, runtime, and toolchain concept) before designing or writing any code.

> **Supreme Architectural Directive — Pilot First, Frozen Airlock & Proposal-Gated Rust:**
> 1. **Pilot First:** `crates/pilot-*` is a temporary, actively contracting bootstrap execution seed (`-> ~1,800` LOC $\rightarrow$ C11 seed in `M46` $\rightarrow$ `0` non-Pilot LOC in `M95`). **Any concept, keyword, syntax rule, preprocessor, lexer, parser, scope/borrow/escape/Perceus pass, arena allocator, collection/string algorithm, IR lowering, bytecode/native emitter, test suite, or CLI/IDE tool representable in Pilot in `<= 1,500` LOC per file MUST be written in Pilot—NEVER in Rust.**
> 2. **Immutable Airlock Freeze:** Once `stdlib/core/bridge_lang.pilot`, `stdlib/core/bridge_os.pilot`, and `crates/pilot-vm/src/bridge.rs` are defined and locked, **absolutely NO modifications, deletions, or workarounds** to those three files are allowed.
> 3. **Mandatory `proposals.md` Approval Before Touching Any Rust:** **Never** create, modify, or delete any Rust file (`crates/**`, `Cargo.toml`) without first writing an explicit proposal in [`proposals.md`](file:///home/moony/Documents/dev/projects/Pilot/proposals.md) and receiving owner approval. Execution is **strictly restricted** to the exact file paths listed in the approved proposal.
> 4. **Delete Dead & Legacy Code Immediately Before Moving On:** Whenever you encounter dead, unreferenced, or superseded legacy code during any task, **do not walk past it**—delete dead/legacy Pilot code (`.pilot`, stale `.pilc`, dead shims) **at that exact moment before moving on**, and for dead/legacy Rust code (`crates/**`), **halt immediately and record the exact target files/items in [`proposals.md`](file:///home/moony/Documents/dev/projects/Pilot/proposals.md)** so it is purged upon owner approval rather than left behind.
> 5. **Anti-Duplication Architecture Maps (`pilot.md` & `rust.md`) & Matrix Tracking (`keyword_matrix.md` & `concept_matrix.md`):** Check [`pilot.md`](file:///home/moony/Documents/dev/projects/Pilot/pilot.md), [`rust.md`](file:///home/moony/Documents/dev/projects/Pilot/rust.md), [`docs/keyword_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/keyword_matrix.md), and [`docs/concept_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/concept_matrix.md) before writing code to prevent duplication or status drift, and update all affected maps and matrices in the same change.
> 6. **Break Loud & Seam Over Rust Retention:** Minimizing the Rust footprint takes **absolute precedence** over keeping legacy Rust-backed paths temporarily working. Breaking implementations or `.pilot` callers during Rust contraction is explicitly authorized and expected—**never** keep misplaced Rust alive or add Rust workarounds to mask unbuilt Pilot pieces. Leave broken paths loud in Pilot via `seam("<area>.<what>", "<what must be rebuilt in Pilot>")` (`SEAM001`), register them in `seams.pilot` / `known_bugs`, downgrade affected matrix/ledger rows honestly, and rebuild them later in pure Pilot.

---

## 1. Pilot-First Law, The 2-File Airlock & Universal Name-Blind Contract

### 1.1 The `<= 1,500` LOC Pilot-First Law & Active Seed Contraction
- **Pilot First Without Exception:** Implement every language feature, compiler pass, runtime data structure, and tool in Pilot (`stdlib/core/`, `stdlib/std/`, `stdlib/pilot/`, `selfhost/`). Never add AST lowering, semantic checks, operator desugaring, string/collection algorithms, module header scanners, formatters, linters, CLI commands, or integration test harnesses in Rust.
- **Active Rust Contraction (Zero Net Growth):**
  - `crates/pilot-*` may **never** grow to implement language semantics, stdlib helpers, or compiler passes. Only irreducible OS syscalls or raw VM byte-slab/dispatch fixes may touch Rust, and every wave touching `crates/` must hold flat or lower `paused_max_total_rust_loc()` in [`selfhost/gate/manifest.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/manifest.pilot) until `seed_contraction_paused()` flips to `false` at Stage-3 cutover.
  - Format all surviving Rust in standard `rustfmt` style (`cargo fmt --check`). **Never pack multiple statements onto long lines** to disguise a file's true line count under `MOD001`.
- **File Modularity Cap (`<= 1,500` LOC Law):** Keep every `.pilot` and `.rs` file focused on a single cohesive responsibility and strictly `<= 1,500` lines (`MOD001`). Split growing subsystems into clean sub-modules before approaching the cap.
- **Delete Dead & Legacy Code On Sight (Zero Lingering Cruft):** Whenever you come across dead, unreachable, or superseded legacy code—or whenever a Pilot implementation replaces an old crate, file, intrinsic, shim, or stale `.pilc` blob—**delete it at that exact moment before moving on** (deleting `.pilot`/`.pilc` artifacts immediately, and stopping to file an exact deletion entry in [`proposals.md`](file:///home/moony/Documents/dev/projects/Pilot/proposals.md) for any Rust files in `crates/**`) and mark retired crates `must_be_absent: true` in [`selfhost/gate/manifest.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/manifest.pilot).

### 1.2 The Hard Airlock Boundary, Immutable Freeze & `proposals.md` Gate
To make accidental reliance on the Rust engine structurally impossible, the **entire Pilot-to-Rust connection** is confined to a strict airlock enforced by the parser, checker (`ENGINE001`), `pilot gate` (`GATE021`), and owner change control:
1. **Pilot Side — Strictly 2 Files May Utter `__engine`:**
   - `stdlib/core/bridge_lang.pilot`: Compile-time genesis bindings (`[Lang]`, `[Bits]`, `[Layout]`, `[Lattice]`, `bind_receptor`, `register_modifier`, `register_pratt_op`) required only while the Stage-A seed parser reads `.pilot` source. Zero runtime functions.
   - `stdlib/core/bridge_os.pilot`: Thin `pkg`-private (`pkg Pilot.Core.Bridge`, `lvl(100)`) wrappers around irreducible raw OS syscalls and VM slab/slot primitives, consumed exclusively by typed Ring-1/Ring-2 wrappers (`List<T>`, `Map<K, V>`, `SliceBuffer<T>`, `ArenaScope`, `File`, `str`, `std.fs`, `std.proc`, `std.time`).
   - **Hard Ban Everywhere Else (`ENGINE001`):** Writing `__engine(...)` in **any** other `.pilot` file—across user code, `stdlib/*`, `selfhost/*`, and `tools/*`—is forbidden. Legacy files on the migration ratchet may only decrease to `0`; **zero new files or `__engine` call sites may ever be introduced**.
2. **Rust Side — Strictly 1 Runtime Host Dispatch File:**
   - `crates/pilot-vm/src/bridge.rs`: Single closed `#[repr(u8)] enum HostOp` (guarded by a compile-time `const` count assertion) mapping compact op IDs to 1–5 line raw OS/slab primitives.
   - No other `.rs` file in `crates/` may match intrinsic names or import `std::fs`, `std::process`, `std::net`, or `std::time` (outside thin CLI bootstrap startup).
3. **Permanent Immutability After Definition:**
   - Once `stdlib/core/bridge_lang.pilot`, `stdlib/core/bridge_os.pilot`, and `crates/pilot-vm/src/bridge.rs` are defined and approved, **zero modifications, deletions, or workarounds** to any of those 3 files are allowed. Design every wrapper above them in pure Pilot.
4. **Proposal-Gated Rust Changes (`proposals.md`) & Architecture Maps (`rust.md`, `pilot.md`):**
   - **Mandatory Pre-Approval (`proposals.md`):** Any creation, modification, or deletion of Rust code (`crates/**`, `Cargo.toml`) MUST first be written into [`proposals.md`](file:///home/moony/Documents/dev/projects/Pilot/proposals.md) naming the **exact** files to be created, modified, or deleted, and **paused until approved by the owner**. Touching any Rust file not explicitly listed in the approved proposal is prohibited.
   - **Living Implementation Maps (`rust.md` & `pilot.md`):** Consult [`pilot.md`](file:///home/moony/Documents/dev/projects/Pilot/pilot.md) and [`rust.md`](file:///home/moony/Documents/dev/projects/Pilot/rust.md) before writing code to prevent duplicate implementations, and update them in the same change whenever Pilot or (approved) Rust files, types, functions, or responsibilities change.

### 1.3 What Belongs in Pilot vs. The Irreducible Seed Floor

| Layer / Capability | Allowed in Rust? | Mandatory Home & Mechanism |
| :--- | :--- | :--- |
| **Keywords, Syntax, Preprocessor & Macros** | **No** *(Stage-A 14 seed tokens only until Wave 6; `0` at Stage B)* | `stdlib/core/syntax/`, `selfhost/preprocessor.pilot`, `selfhost/lexer.pilot`, `selfhost/parser.pilot`, `selfhost/syntax/`. |
| **Lexical Scopes, `defer`, `with`, `scope`/`enter`/`exit` & `0b11` Guards** | **No (`0` Rust LOC)** | Symbol resolution in `selfhost/check/resolve.pilot`; LIFO exit weaving on normal/`break`/`return`/`?` edges in `selfhost/check/drops.pilot` + `selfhost/ir/pass_perceus.pilot`. |
| **Arenas (`ArenaScope`, `arena()`, `region`) & Perceus ARC / `drop_reuse`** | **No (`0` Rust LOC)** | Bump pointer math (`{ base, offset, cap }`) in `stdlib/core/arena.pilot` and `rc` header / in-place `drop_reuse` in `stdlib/core/mem.pilot` + `selfhost/ir/pass_perceus.pilot` over raw byte slabs. |
| **Borrows, Aliasing, Escape & `noalloc` Proofs** | **No (`0` Rust LOC)** | Static graph passes in `selfhost/check/{borrows,aliasing,own,stack,noalloc}.pilot` and `selfhost/perceus/escape.pilot`. |
| **Closures, `yield` Generators, `async` Tasks & `MethodTable`s** | **No (`0` Rust LOC)** | Lowered in `selfhost/check/methods.pilot` and `selfhost/ir/{lower_generator,lower_expr}.pilot` into state-machine structs, environment records, and direct/table calls. |
| **Collections, Strings, Formatting, Crypto, DB, HTTP & Polyglot** | **No (`0` Rust LOC)** | `stdlib/core/primitives/{list,map,str,slice,stride}.pilot`, `stdlib/core/{format,convert}.pilot`, and `stdlib/std/*`. |
| **`comptime` Evaluation, `quote`/`ast<T>`, Codegen & `.pilc` Emission** | **No (`0` Rust LOC)** | `selfhost/ir/interp.pilot`, `stdlib/core/primitives/ast.pilot`, `selfhost/codegen/*`, and `selfhost/vm/{ir_to_pilc,pilc_writer}.pilot`. |
| **Bytecode Dispatch, Raw Byte Slabs & Thin OS Syscalls** | **Yes (`~1,800` LOC seed $\rightarrow$ C11 in `M46`)** | `.pilc` loader, VM ALU/jump/call dispatch loop, raw slab `alloc`/`free`/`load`/`store`/`memcpy`, and ~20 thin OS/ABI syscalls in `crates/pilot-vm/src/bridge.rs`. |

### 1.4 The Strict 14-Token & Universal Name-Blind Contract
- **The 14 Seed Tokens (Stage-A Parser Ceiling):** While the Stage-A seed still reads `.pilot` text (prior to precompiled `toolchain.pilc` cutover), the **only** words it may ever recognize are the **14 seed tokens**:
  `__engine`, `struct`, `interface`, `fn`, `let`, `ref`, `if`, `loop`, `return`, `yield`, `defer`, `comptime`, `quote`, `asm`.
- **Zero Hardcoded Type, Operator, Module, or Domain Names in Rust:**
  - Never hardcode ANY Pilot type name—scalar (`"i64"`, `"u8"`, `"f64"`, `"char"`, `"bool"`, `"usize"`) or composite (`"str"`, `"slice"`, `"stride"`, `"array"`, `"simd"`, `"opt"`, `"option"`, `"res"`, `"result"`, `"range"`, `"span"`, `"map"`, `"Map"`, `"List"`, `"SliceBuffer"`, `"RangeWindow"`, `"ArenaScope"`, `"Dual"`, `"Signal"`, `"frame"`, `"ast"`)—or English operator words (`"and"`, `"or"`, `"is"`, `"not"`, `"union"`, `"intersect"`, `"except"`) inside Rust `match` arms, string comparisons, or tables.
- **Scalable Open-Registry Design (Zero Rigid Corners or Magic Counts):**
  - Drive CLI `[Command]` dispatch, `PilIR` passes (`IPilIrPass`), foreign `use <lang>` absorbers, `[...]` weavers, target backends, and diagnostics from open table/map registries so adding item `N + 1` requires zero core dispatcher edits.
- **The Dual-Pass Universal Rename Litmus Test:**
  - *Pass 0 (Canonical):* Renaming `char` $\rightarrow$ `c32`, `slice<T>` $\rightarrow$ `window<T>`, `option<T>`/`opt<T>` $\rightarrow$ `maybe<T>`, `result<T, E>`/`res<T, E>` $\rightarrow$ `outcome<T, E>`, and `range<T>` $\rightarrow$ `interval<T>` inside `stdlib/core/*.pilot` MUST compile, check, and run across all engines with zero seed changes.
  - *Pass 1 (Randomized):* Deterministic seeded identifier permutation (`pilot gate --litmus --seed <u64>`) across all `[Lang]` primitives and receptors MUST pass identically.
- **5 Name-Blind Type Derivation Mechanisms (Owned by Pilot `selfhost/check/` + `selfhost/ir/` + `stdlib/core/`):**
  1. **Register & Layout Physics (`[Bits]` & `[Layout]`):** Deduce register classes (`gpr_signed`, `gpr_unsigned`, `simd_float`, `simd_vector`, `raw_ptr`, `unit`, `noreturn`), bit widths, inline repetition, and niches strictly from layout descriptors.
  2. **Structural Field Anatomy (`TypeDef.fields` & `LatticeBits`):** Deduce contiguous views (`{ ptr<T>, usize }` + `0b01`), strided lenses (`{ ptr<T>, usize, isize }` + `0b01`), owned buffers (`{ ptr<T>, usize, usize }` + `0b10`), and bounds pairs (`{ T, T }` + `0b00`) from field layouts and the 2-bit lattice (`0b00..0b11`).
  3. **Pure Pilot Syntax Rewrites (`syntax type_construct`):** Rewrite symbolic types (`T[]`, `T[N]`, `T[..]`, `T@N`, `T?`, `T!E`, `(A) => R`) via `syntax type_construct` rules in `stdlib/core/`—never hardcoded parser mappings.
  4. **Operator & Protocol Contracts (`MethodTable`):** Deduce `expr?` (`op ?`), `for` loops (`get_enumerator()` or structural bounds), indexing (`op []` / `op []=`), unpacking (`ISplatOperator`, `IPairsOperator`, `IFlattenOperator`), and `implicit` lifting from the type's resolved `MethodTable`.
  5. **`TypeId` Receptor Slots (`bind_receptor`):** Bind literals (`42`, `3.14`, `'a'`, `"hi"`, `true`) and call-quote projections (`!`/`call`, `!!`/`splice`, `!&`/`frame`, `!^`/`sig`, `!@`/`grad`, `!?`/`plan`) to registered `TypeId` handles by structural field index (`0`, `1`, ...).

### 1.5 Break Loud & Leave a Seam — Never Retain Misplaced Rust or Fake Results
Minimizing Rust takes precedence over keeping legacy shims working. Never keep misplaced Rust code alive to prevent `.pilot` callers from breaking, never return static, pre-computed, caller-supplied, or test-fitted values in place of real behavior, and never leave silent `// TODO` comments or `unimplemented!()`.
- **Break Loud During Rust Contraction $\rightarrow$ `seam(...)` in Pilot:** It is explicitly expected and acceptable for `.pilot` files and downstream features to break when ripping misplaced logic out of `crates/pilot-*`. **Leave broken Pilot implementations loud** by wiring them to `seam("<area>.<what>", "<what needs to be implemented in Pilot>")` (registered in [`seams.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/ledger/seams.pilot)) rather than keeping Rust shims or rushing half-baked workarounds. If an existing conformance program breaks because its legacy Rust backing was deleted before its pure-Pilot replacement lands, move/record it in `tests/conformance/known_bugs/` (`bug_registry()`) with its **true** expected output untouched and downgrade the corresponding rows in `wiring.pilot`, `keyword_matrix.md`, and `concept_matrix.md` to `SEAM` / `DRAFT`.
- **Unconnected Entry/Exit Points $\rightarrow$ `seam(...)`:** Stop at `seam("<area>.<what>", "<what is missing>")`, which always traps loudly at runtime with `SEAM001` and a stack trace.
- **Unrepresentable Values $\rightarrow$ `Pending<T>`:** Store `pending("<area>.<what>", "<what is missing>")` ([`stdlib/core/pending.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/pending.pilot)); reading `.get()` traps with `SEAM001`. Never substitute `0`, `""`, or dummy integers.
- **Temporary Representations $\rightarrow$ `[StandIn(...)]`:** Mark real working code that uses an interim representation (`[StandIn("<area>.<what>", "<spec>", "<difference>", "<owner>")]`), list it in `blocked_by` on every affected ledger row, and cap those rows at `DRAFT` until replaced.
- **Wrong Engine/Library Answers $\rightarrow$ `known_bugs` + `[Workaround]`:** Add a repro in `tests/conformance/known_bugs/<name>.pilot` with the **true** expected output, register it in `bug_registry()` in [`selfhost/gate/ledger/seams.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/ledger/seams.pilot), and tag any temporary caller accommodation with `[Workaround("<bug id>", "<what was done>")]`. Never alter a test's expected output to match a bug.
- **Ledger Synchronization (`GATE011`–`GATE020`):** Register every seam, pending value, stand-in, and known bug in [`selfhost/gate/ledger/seams.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/ledger/seams.pilot) and record capability maturity (`SPEC`, `DRAFT`, `CORE`, `WIRED`, `COMPLETE`, or `SIMULATED`) with concrete run evidence in [`selfhost/gate/ledger/wiring.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/ledger/wiring.pilot) in the same change. Only the project owner sets `COMPLETE`.

---

## 2. Canonical Syntax Rules & Dual Readability ("One Keyword Per Responsibility")
1. **Functions & Closures:** Use `fn` (never `def`) and C#-style `(x) => expr` or `(x, y) => { return x + y; }` (never `lambda`).
2. **Blocks & Terminators:** Use `{ ... }` for blocks and `=> expr;` for inline bodies (never Pythonic `:` indentation blocks). End all statements with `;`.
3. **Colon (`:`) Responsibility:** Restricted strictly to type annotations (`x: i64`), inheritance (`: IComparable<i64>, IValue`), `switch` arms (`case 1:`), stepped ranges (`0 .. 10 : 2`), `:ns/kw` keywords, and distributive header labels (`pub:`, `priv:`, `pub static pure:`).
4. **String Interpolation vs. AST Splicing:** Use `f"hello {x}"` for string interpolation (never `$"..."`); inner `{ ... }` holes lex full Pilot expressions cleanly (including unescaped quotes and nested `f"..."`). Reserve `$` strictly for homoiconic `quote { $node }` AST splicing.
5. **Generics (`<T>`) vs. Square Brackets (`[]`):**
   - Use `<T>` for all generic type parameters (`slice<T>`, `List<i32>`, `Map<str, i64>`). Never use `Type[T]` for generics.
   - `[...]` has 4 unambiguous roles by position:
     1. **Declaration Prefix (`[Attr1, Attr2]`):** Compile-time/reactive attribute pipeline (replaces `@` decorators).
     2. **Type Suffix (`T[]`, `T[N]`, `T[..]`):** Rewrites to `slice<T>`, `array<T, N>`, and `stride<T>`.
     3. **Expression Primary (`[1, 2, 3,]`):** Target-typed collection or `[k: v,]` map literal.
     4. **Expression Postfix (`arr[i]`):** Index/sub-slice (`m[k]`, `m[k] = v` via `op []` / `op []=`) or dimensional unit literal (`100[m]`).
6. **Collection-Only Optional Trailing Commas:**
   - **Allowed:** Collection literals (`[1, 2, 3,]`), tuple/struct initializers (`(1, 2,)`, `Vec3 { x: 1, y: 2, }`), `enum` variants, and distributive binding blocks (`let List<i32> { a = [1, 2,], b = [3, 4,], };`).
   - **Disallowed:** Attribute lists `[Attr1, Attr2]`, function parameters/arguments `fn(a: i32, b: i32)`, generic parameters `<T, U>`, and `: IInterface` lists.
7. **Mutability (`var` Only) & Pattern Branching (`switch` Only):** Use `let` for immutable bindings, `var` for mutable bindings (`var x = 0;`, `var ref<T>`, `var fn` — never `mut`), and `switch` (never `match`).
8. **Comments:** Use `//` and `/* ... */` (never `#` comments; line-leading `#` is reserved strictly for `#if`, `#elif`, `#else`, `#endif`, `#define`, `#region`, `#pragma` handled by `selfhost/preprocessor.pilot`).
9. **Distributive Header Grouping (`Header:` & `Header { ... }`):**
   - Section labels (`pub:`, `priv:`, `pub static pure:`) apply to subsequent declarations in the current `{ ... }` block.
   - Block prefixes (`let List<i32> { a = [1, 2], b = [3, 4] };`, `pub partial struct { ... }`, `pub interface { ... }`) distribute modifiers and types across every inner item.
10. **Mandatory Dual Readable/Symbolic Pairing Law:**
    - Native built-in types and keywords use short, single-word `lowercase` names (`bool`, `str`, `pkg`, `ext`, `opt`, `res`, `sig`, `grad`, `plan`, `call`, `splice`, `frame`, `splat`, `pairs`, `flat`).
    - Every compact symbolic construct MUST have an equivalent readable `lowercase` counterpart lowering to the **exact same canonical AST/receptor node**:
      - **Types:** `T?` $\leftrightarrow$ `opt<T>` (`option<T>`), `T ! E` $\leftrightarrow$ `res<T, E>` (`result<T, E>`), `T[]` $\leftrightarrow$ `slice<T>`, `T[N]` $\leftrightarrow$ `array<T, N>`, `T[..]` $\leftrightarrow$ `stride<T>`, `T@N` $\leftrightarrow$ `simd<T, N>`.
      - **Call-Quoting:** `f!(x)` $\leftrightarrow$ `call f(x)`, `f!!(~x)` $\leftrightarrow$ `splice f(~x)`, `f!&(x)` $\leftrightarrow$ `frame f(x)`, `f!^(x)` $\leftrightarrow$ `sig f(x)`, `f!@(x)` $\leftrightarrow$ `grad f(x)`, `f!?(x)` $\leftrightarrow$ `plan f(x)`.
      - **Unpacking:** `*x` $\leftrightarrow$ `splat(x)`, `**x` $\leftrightarrow$ `pairs(x)`, `*..x` $\leftrightarrow$ `flat(x)`.

---

## 3. Naming Taxonomy, 3-Ring Namespaces & The 2-Bit Hardware Type Lattice
Every type originates from `IAnyValue` (`0` runtime header bytes) and is classified via bitwise-OR of its fields and `~self()` destructor (`Bit 0` = borrows memory, `Bit 1` = owns `~self()` destructor). Enforce this taxonomy in `Pilot.Core.*` / `core.*` via `[EnforceCoreNaming]`:

| Category / Lattice | Naming Pattern | Canonical Examples | Lifecycle & Perceus Action |
| :--- | :--- | :--- | :--- |
| **`[Lang]` Primitives** | Single-word `lowercase` | `void`, `never`, `bool`, `char`, `i8`..`i128`, `isize`, `u8`..`u128`, `usize`, `f16`..`f128`, `str`, `ptr<T>`, `slice<T>`, `stride<T>`, `array<T, N>`, `simd<T, N>`, `ref<T>`, `iso<T>`, `tuple<...>`, `opt<T>`, `res<T, E>`, `union<T1, T2>`, `fn<A, R>`, `range<T>`, `span<T>`, `step<T>`, `from<T>`, `until<T>`, `full`, `splat<T>`, `pairs<K, V>`, `expand<T>`, `map<K, V>`, `frame<TSig>`, `sig<T>`, `grad<T, G>`, `plan<T>`, `type`, `ast`, `cst`, `ir` | Hardware register / flat C-ABI primitives declared in `stdlib/core/`. |
| **Interfaces** | `I` + `PascalCase` | `IAnyValue`, `IValue`, `IReference`, `IObject`, `IGuard`, `IEnumerable<T>`, `IAddOperator<T>` | `0`-byte compile-time contract, default method provider, and lattice carrier. |
| **Field Bundles** | `PascalCase` + `Bundle` | `RawStorageBundle`, `SpatialHeaderBundle` | Stored fields + methods. Spliced via `*Bundle` in primary `struct` only. |
| **Method Facets** | `PascalCase` + `Facet` | `SignedMathFacet`, `ContiguousViewFacet` | `0` stored fields (methods/ops only). Safe to splice in `partial struct`. |
| **`0b00` Value Types** | Unadorned `PascalCase` | `Vec3`, `RangeWindow<T>`, `Dual<T, G>`, `CallExpr<T>` | `: IValue`. Trivial `memcpy`, eligible for `as view` casts and `.pchor` cues. |
| **`0b01` Reference Views** | `...View` / `...Lens` / `...Cursor` | `SliceView<T>`, `BitLens<T>`, `OffsetLens<T>`, `TokenCursor` | `: IReference`. Borrowed window; lexical escape checked (`ESCAPE001`–`004`). |
| **`0b10` Owned Objects** | `...Buffer` / `...List` / `...Handle` | `SliceBuffer<T>`, `PackedBuffer<T>`, `List<T>`, `Map<K, V>`, `ForeignHandle<T>`, `Signal<T>`, `EffectPlan<T>` | `: IObject`. Has `~self()`; Perceus move-by-default + in-place `drop_reuse`. |
| **`0b11` Scoped Guards** | `...Guard` / `...Scope` / `...Lease` | `MutexGuard<T>`, `ArenaScope`, `TxLease`, `PyGilGuard` | `: IGuard`. Borrows memory AND runs LIFO `~self()` at scope exit; cannot escape scope. |
| **`[...]` Attributes** | `...Attr` (Def) / `[Name]` (Use) | `struct ClampAttr : IAttribute` $\rightarrow$ `[Clamp(0, 1)]` | Sorted by `weight: i32` at compile time and woven into Pilot AST/IR. |

- **3-Ring Standard Library Personality (Zero Python-Clone Module Names):**
  - **Ring 1 (`core.*`):** Freestanding hardware/language core (`0` OS/libc/filesystem/network imports — `RING001`).
  - **Ring 2 (`std.*`):** Hosted standard library (`std.col`, `std.text`, `std.io`, `std.fs`, `std.net`, `std.http`, `std.sync`, `std.chan`, `std.time`, `std.math`, `std.rand`, `std.crypto`, `std.db`, `std.serde.*`, `std.compress`, `std.proc`, `std.cli`, `std.test`, `std.bench`, `std.log`, `std.polyglot.prog.*`, `std.polyglot.transport.*`). Never name modules after Python/foreign clones (`pickle`, `urllib`, `pandas`, `unittest`, `tomllib`, `shutil`).
  - **Ring 3 (`pilot.*`):** First-class Compiler-as-a-Library (`pilot.lex`, `pilot.cst`, `pilot.ast`, `pilot.types`, `pilot.ir`, `pilot.fmt`, `pilot.db`, `pilot.vm`, `pilot.diag`, `pilot.abi`, `pilot.chor`, `pilot.pkg`, `pilot.mcp`).
- **Mandatory Type Selection & Implementation Rules ([`imp_type.md`](file:///home/moony/Documents/dev/projects/Pilot/imp_type.md)):**
  1. **Predefined Finite Sets (`<= 19` items):** `enum : u8/u16` with target-typed `::Variant`—never stringly-typed constants or magic integers.
  2. **Large Sets (`20+` items):** Table-driven `array<T, N>` or `comptime Map<K, V>` open registries with integer ID handles.
  3. **Fixed Composite Records:** `[Layout(c_abi)] struct : IValue` (`0b00`) with trivial `memcpy` and zero heap allocation.
  4. **Polymorphic Behavior:** `interface` contracts with concrete `struct`s composing shared state via `*Bundle` and shared methods via `*Facet`.
  5. **Orthogonal Combinations:** `bitflags : u32/u64` (`[Flags]`), never multiple booleans.
  6. **Generics:** Always constrain type parameters (`<T : IConstraint>`), never unconstrained `<T>` with runtime reflection guesswork.
  7. **Unambiguous Absences & Failures:** Use `opt<T>` (`T?`) and `res<T, E>` (`T ! E`), never sentinels (`null`, `-1`).
  8. **Memory Boundaries:** Distinguish `0b01` borrowed views (`slice<T>`), `0b10` owned buffers (`List<T>`, `SliceBuffer<T>`), and `0b11` scoped guards (`ArenaScope`).
  9. **Single Source of Truth & Targeted Importing:** Define every type once in its owning module and import explicitly (`use ...;`). Never copy or re-declare types across files.
  10. **Strict Intrinsic Encapsulation (`ENGINE001` Airlock):** Never call `__engine(...)` outside `stdlib/core/bridge_lang.pilot` and `stdlib/core/bridge_os.pilot`. Always use strongly typed 2-bit-lattice Pilot types (`Map<K, V>`, `List<T>`, `SliceBuffer<T>`, `ArenaScope`, `std.fs`, `std.proc`, `std.time`).

---

## 4. 4-Tier `MethodTable`, `*Bundle` Splicing & Extension Scoping
- **4-Tier Vertical Precedence (Replaces OOP Base Classes):**
  1. **Tier 1:** Explicit `struct` / `partial struct` member (wins unconditionally).
  2. **Tier 2:** Spliced `*Bundle` / `*Facet` member (fulfills `interface` contracts and overrides Tiers 3–4).
  3. **Tier 3:** Sub-`interface` default method (overrides Tier 4).
  4. **Tier 4:** Base `interface` default method.
- **Explicit Override & Cross-Package Openness:** `override` verifies that a base `interface` or spliced `*Bundle`/`*Facet` provides the member (`METH001`) and enforces `open` visibility across package boundaries (`VIS001`).
- **Horizontal Collision Modifiers:** Resolve overlapping `*Bundle` fields or methods explicitly using `coalesce { id }`, `with { flags as net_flags }`, `without { clear_state }`, and static delegation `BundleName.method(var ref self, args...)`.
- **Extensions (`ext` & `use ext`):** Support individual (`pub pure ext fn i64.is_even() -> bool => (self & 1) == 0;`) and block extensions (`pub ext str { ... }`), imported file-wide (`use Pkg.StrExt;`) or lexically scoped (`use ext StrExt { ... }`).

---

## 5. Lexing, Receivers, Properties, Operators & Call-Quoting Invariants

### 5.1 Two-Way Dot (`PrefixDot` vs. `InfixDot`), Floats, Enums & Cascades
- **`InfixDot` (`.`):** Preceded immediately (no whitespace) by an identifier, literal, `)`, `]`, or `}` (`obj.field`, `tuple.0`).
- **`PrefixDot` (`.`):** Preceded by whitespace or `;`, `(`, `[`, `{`, `,`, `:` (`.reset()`, `0 .. .count`). Desugars to `self.<ident>`. Never place dots adjacent without whitespace (`0.. .count`, not `0...count`).
- **Leading-Zero Floats & `::Variant` Enums:** Floats must start with a digit (`0.5`, never `.5`). Target-typed enum variants use `::Active` (bare `.Active` means `self.Active`).
- **Fluid Cascades (`..` / `?..`):** Bound at `Precedence::Cascade = 1` (`node..set_title("Master")..set_status("OK");`), mutating/calling on the root receiver and returning it.

### 5.2 Splat & Recursive Flatten Tokens (`*`, `**`, `*..` / `splat`, `pairs`, `flat`)
- **`*expr` / `splat(expr)` (`splat<T>` / `ISplatOperator`):** 1-level positional sequence unpack or `*Bundle` splice.
- **`**expr` / `pairs(expr)` (`pairs<K, V>` / `IPairsOperator`):** 1-level compile-time structural field or map key-value unpack.
- **`*..expr` / `flat(expr)` (`expand<T>` / `IFlattenOperator`):** Recursive deep-flatten to 1D leaf elements via `op flat () -> TLeaf`.

### 5.3 Dunder-Free Lifecycle & Variation B Property/Indexer Setters
- **Lifecycle:** `self(params...)` constructor, `~self()` destructor, and `scope self { enter: ... exit: ... }`.
- **Implicit Properties & Indexers (`self[params...] -> T`):** Omit `property` keyword. Inside `get` and `set` clauses, `field` accesses synthesized backing storage and `value` accesses the incoming setter parameter (`pub balance: f64 { get => field; set => { field = value; } }`). Support overloaded setters (`set(scale: f64)`), guarded setters (`set(v: T) if cond`), and `iso` setters (`set(iso v: OwnedT)`).

### 5.4 Values-First Errors (`T ! E` / `res`), Null-Safety (`T?` / `opt`), 3-Tier `for` & Call-Quoting
- **Universal Postfix `?` (`ITryOperator`):** Unwraps `res<T, E>` (`T ! E`) or `opt<T>` (`T?`) and propagates failure/`none` early. Pair with `guard some x else { ... }`, `expr ?? fallback`, and `expr catch err => ...`. Unchecked postfix `x!` is prohibited (`NULL001`–`NULL005`, `ERR001`–`ERR005`).
- **3-Tier Dynamic `for` Expressions (`PilIR`):** Lower `FixedScalar`, `MutableBounds` (`continue a..b;`), and `WorklistStack` (`continue a..b, ...;` backed by `RangeWindow<TIter>`) alongside `CollectionYield` (`SliceBuffer<T>`) and `ScalarBreak` (`break val;` + `then fallback`) in `selfhost/ir/`.
- **6 Homoiconic Call-Quoting Modes (Zero-Whitespace `(` Symbolic OR Readable Prefix Keyword):**
  - `fn!(a, b)` / `call fn(a, b)` $\rightarrow$ `CallExpr<TReturn>` (`ShallowQuote`)
  - `fn!!(~a, b)` / `splice fn(~a, b)` $\rightarrow$ `ast<TReturn>` (`DeepSplice` with `~` compile-time unquoting)
  - `fn!&(a, b)` / `frame fn(a, b)` $\rightarrow$ `frame<FnSig>` (`AbiFrame` zero-marshalling `#[repr(C)]` struct)
  - `fn!^(a, b)` / `sig fn(a, b)` $\rightarrow$ `Signal<TReturn>` (`ReactiveSignal` glitch-free push-pull node)
  - `fn!@(a, b)` / `grad fn(a, b)` $\rightarrow$ `Dual<TReturn, TGrad>` (`SymbolicAdjoint` compile-time CSE derivative)
  - `fn!?(a, b)` / `plan fn(a, b)` $\rightarrow$ `EffectPlan<TReturn>` (`EffectDryRun` CoW transactional sandbox)

### 5.5 Preserved Domain Paradigms & Two-Axis Visibility (`lvl`)
- **Two-Axis Visibility:** Axis 1 spatial modifiers (`priv`, `file`, `prot`, `pkg`, `int`, `pub`, `open`, `export`) + Axis 2 compile-time clearance rings (`lvl(N)` with flow-typed elevation, `LVL001`, `VIS001`, `MOD003`) enforced in `selfhost/check/{access,clearance}.pilot`.
- **Contracts, DI, Adapters, Real-Time & Polyglot:** Implement `requires`/`ensures`/`old()`/`invariant`, `context` DI, `adapter A -> B` & `each` vectorization, `state`/`transition` typestates, `[HardRealtime]` / `[Wcet]` / `noalloc` / `with arena`, dimensional units (`100[m]`), LINQ/SQL queries, and inline polyglot blocks strictly in Pilot passes (`stdlib/` + `selfhost/check/` + `selfhost/ir/`).

---

## 6. Everything First-Class, Shared `PilotDb`, `PilIR` & Progressive Learning Curve
- **One Shared Pure-Pilot Pipeline (`PilotDb` + Lossless `CST` + `selfhost/check` + `PilIR`):**
  - Never maintain parallel AST serializers, duplicate Rust type/layout tables, or disconnected per-tool walkers. Every command (`check`, `build`, `run`, `fmt`, `doc`, `lsp`, `debug`, `inspect`, `migrate`, `mcp`, `gate`) shares the single pure-Pilot pipeline (`pilot.db` $\rightarrow$ `pilot.cst` $\rightarrow$ `selfhost/check/` $\rightarrow$ `selfhost/ir/` $\rightarrow$ target emitters including `ir_to_pilc.pilot` and `c_emitter.pilot`).
- **Progressive 4-Tier Ergonomics:**
  - Never force Tier-3/4 systems or metaprogramming syntax (lifetime annotations, raw pointers, manual `comptime` plumbing) onto Tier-1/2 code. Everyday Pilot code reads naturally with full inference, while unlocking zero-cost control when attributes, facets, arenas, `iso`, Packed Representations, or `.pchor` blueprints are applied.
- **Zero-Black-Box Transparency (`pilot inspect`):**
  - Every compiler decision—attribute weave order (`weight: i32`), struct byte offsets/niches/packing, 4-tier `MethodTable` winners, Perceus `drop`/`drop_reuse` insertion, `adapter` lowering, and `.pchor` rewrites—must be inspectable via `pilot inspect`.

---

## 7. Priority Pillar: Universal Foreign-Language Adapter (`use <lang>` & `adapter`)
Interoperability with foreign languages is partitioned into **Programming Languages** (execution, native ABIs, in-process runtimes, zero-copy memory) and **Transport Languages** (structured trees, DOM querying, lossless serialization), all orchestrated in Pilot (`stdlib/polyglot/`, `selfhost/adapter/`).

### 7.1 Programming Languages Subsystem (`std.polyglot.prog.*`)
Covers C (Pilot `1.0`), and C++, Rust, Zig, Python, JS/TypeScript, WebAssembly, JVM, .NET, and Swift (Pilot `2.0`):
1. **Zero Manual Glue Files (`use <lang>`):** Absorb foreign headers, crates, `.pyi` stubs, `.d.ts` declarations, `.wit` components, and schemas at `comptime` through open Pilot absorber registries.
2. **In-Process Execution Only:** Call native ABIs via declarative `pilot.abi.*` `AbiFramePlan`s (`0 ns` direct C/ELF calls on native builds; thin `dlopen`/`ffi_call` in `bridge.rs` on `PilVM`), and host managed runtimes (`libpython`, Node-API/QuickJS, WASM, JNI, CLR) strictly in-process—never subprocess-per-call.
3. **Zero-Copy `0b01` Memory Sharing:** Map foreign `(ptr, len)` buffers, `&[T]`, `Py_buffer`/NumPy/Arrow/DLPack, `TypedArray`, and WASM linear memory directly onto `0b01` `slice<T>` and `stride<T>` with static `ESCAPE001`/`ESCAPE004` borrow proofs.
4. **Perceus `0b10` & `0b11` Cross-Runtime Safety:** Bind foreign heap handles to `0b10 : IObject` (`~self()` deterministic release) and execution scopes (GIL, `HandleScope`) to `0b11 : IGuard`, catching foreign traps/exceptions into `res<T, ForeignErr>`.
5. **Zero-Cost `adapter` Idiom Lifting:** Compile prefix stripping, `(ptr, len)` pairs, out-parameters, and status-code lifting into force-inlined `0`-allocation `PilIR`.

### 7.2 Transport Languages Subsystem (`std.polyglot.transport.*`)
Covers **JSON**, **YAML/YML**, **XML**, and **TOML** in pure Pilot:
1. **Typed 2-Bit Lattice DOM Records:** `JsonValue`, `YamlNode`, `XmlNode`, and `TomlValue` (`0b00: IValue` scalar leaves, `0b10: IObject` trees).
2. **Bidirectional Query & Mutation:** Native `get`, `set`, `has`, `remove`, `push`, `get_at`, `len`, `query_path`, `find_first`, and `find_all` without string hacking.
3. **Format-Compliant Parsers & Emitters:** Pure-Pilot `*_parse` and `stringify()` / `emit()`.

### 7.3 Cross-Language Generative Metaprogramming Roadmap
- **Phase 1 [COMPLETE]:** Bidirectional JSON, YAML, XML, and TOML DOM manipulation (`std.polyglot.transport.*`).
- **Phase 2:** Homoiconic `quote { ... }` / `ast<T>` synthesis into typed foreign source and transport trees.
- **Phase 3:** In-process foreign compiler plugins calling Pilot's compiler library for `comptime` expansion.
- **Phase 4:** Inline generative blocks (`pilot in rust`, `pilot in python`) synthesizing types and bindings into host builds.

---

## 8. `.pchor` Choreography, Unified `pilot` Toolchain & AI Co-Design (`pilot mcp`)
- **Sublanguage & Layout Choreographer (`.pchor`):** Keep domain `struct` and `fn` code clean while `.pchor` blueprints rewrite storage layouts (`AoS` $\leftrightarrow$ `SoA<T, align: 64>` / Bit-Packed / Arena / GPU), execution topology, and `[Wcet]`/`noalloc` guardrails as real `PilIR` passes (`selfhost/chor/`). Enforce the **2-Bit Cue Boundary Law** (`CHOR001`: only `0b00` `IValue` or `iso`-moved `0b10` `IObject` may cross a `cue`).
- **100% Pilot-Hosted Unified `pilot` CLI:** Register every subcommand (`run`, `build`, `check`, `fmt`, `test`, `bench`, `doc`, `pkg`, `lsp`, `debug`, `mcp`, `repl`, `inspect`, `absorb`, `migrate`, `chor`, `doctor`, `ship`, `studio`, `gate`) in Pilot (`selfhost/cli/`) via `[Command]` into `CommandRegistry`. `crates/pilot-cli` is strictly a thin `.pilc` bootstrapper.
- **Lossless CST Codemods (`--fix` & `migrate`):** All automated diagnostic repairs (`pilot check --fix`) and `[Migrate]` upgrades (`pilot migrate`) edit `pilot.cst` losslessly in Pilot, preserving 100% of comments and trivia.

---

## 9. Mandatory 3-Gate Milestone Verification Discipline
No wave or milestone (`M1`–`M100` in [`MILESTONES_100.md`](file:///home/moony/Documents/dev/projects/Pilot/MILESTONES_100.md)) is complete until all three automated gates pass (`pilot gate`):
1. **Gate 1 — Adaptability, Airlock & Seed Hygiene:** `14` seed tokens respected, `0` hardcoded user-space names, 2-file `__engine` airlock (`ENGINE001` / `GATE021`) respected, `cargo fmt --check` clean, all files `<= 1,500` LOC (`MOD001`), `paused_max_total_rust_loc()` ratcheted down or held flat, `0` dead legacy shims, freestanding `core.*` (`RING001`), and Dual-Pass Universal Rename Litmus green.
2. **Gate 2 — Bootstrap Parity:** Deterministic Stage-2 == Stage-3 self-compilation fixed point (`pilotc`). **Run-time performance matters; compile speed is intentionally ungated** (decided 2026-11-10).
3. **Gate 3 — Top-of-Chart Runtime Performance & Zero Leaks:** `0` unintended hot-path heap allocations, `0` Perceus/arena/foreign handle leaks, and C/Rust-competitive runtime execution verified (`pilot bench`).

### 9.1 Definition of Done
An agent asked to implement a feature, wave, or milestone is done only when every point below holds:
1. **Pilot Acceptance Tests First, Then Frozen:** Before writing implementation code, write acceptance tests in **Pilot** (`tests/conformance/<area>/<name>.pilot` with `.stdout`/`.stderr`/`.exit` + failure `err_*` cases, or `std.test` suites)—**never add new Rust `milestone*.rs` test harnesses for language or library behavior**. Derive expected outputs from the spec, hand-worked math stated in comments, or an external reference tool—never copied from the code's own output.
2. **Reachable Through the Real `pilot` CLI:** Every deliverable must execute end-to-end through `pilot run`, `pilot test`, or `pilot <subcommand>`, never solely via internal Rust functions.
3. **No Fakes, No Airlock Leaks & Break Loud With `seam(...)`:** Zero hardcoded/test-fitted return values, zero new `__engine(...)` calls outside `bridge_lang.pilot`/`bridge_os.pilot`, and zero domain/compiler work added to or retained in Rust. Anything unbuilt—or intentionally broken in Pilot as a result of deleting misplaced Rust—MUST fail loudly via a registered `seam(...)` (`SEAM001`) in Pilot (`seams.pilot`) or be tracked in `known_bugs` with its true expected output preserved. Every active passing test must fail if the implementation body is replaced with a constant.
4. **All 3 Engines & Affected Gates Pass (Or Stop Cleanly at Registered Pilot Seams):**
   - **3-Engine Conformance:** `pilot test --changed`, `pilot test --ir --changed`, and `pilot test --emit c --changed` (plus `--emit js` / `--emit python` if transpilers changed; `--since <rev>` when spanning unverified commits). Programs hitting an intentional contraction seam report `stopped at a seam` (`SEAM001`) or `known bugs reproduced` rather than silent wrong answers.
   - **Gates:** `pilot gate --full` (including `seams` and seed/airlock checks) and `cargo fmt --check` if any `.rs` file was touched.
   - **Targeted Rust Crate Tests (Only When Touching Seed Crates):** `cargo test -p <crate>`; after any lexer/parser edit, run `cargo test -p pilot-parser --test damaged_source -- --include-ignored`.
   - **Keyword & Concept Matrices:** Update [`docs/keyword_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/keyword_matrix.md) (`pilot run tools/keyword_matrix/main.pilot -- --check`) whenever any keyword changes, and update [`docs/concept_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/concept_matrix.md) whenever any symbolic, structural, memory, runtime, polyglot, or toolchain concept changes.
5. **Immediate Dead/Legacy Deletion & Map Sync:** Delete every discovered dead or superseded shim before moving on (via owner-approved [`proposals.md`](file:///home/moony/Documents/dev/projects/Pilot/proposals.md) for any Rust files), ratchet down the Rust LOC / airlock ceiling in [`selfhost/gate/manifest.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/manifest.pilot), and synchronize [`pilot.md`](file:///home/moony/Documents/dev/projects/Pilot/pilot.md) and [`rust.md`](file:///home/moony/Documents/dev/projects/Pilot/rust.md) in the same change.
6. **Ledgers Updated:** Update [`selfhost/gate/ledger/wiring.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/ledger/wiring.pilot) (at most `WIRED`; only the project owner sets `COMPLETE`) and [`selfhost/gate/ledger/seams.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/ledger/seams.pilot) in the same change.
7. **Report What Is Not Done First:** Open every completion report with any remaining seams, ledger rows below `WIRED`, skipped checks, or suspected test issues—followed by what works and the exact commands run.

---

## 10. Multiplatform Portability & Cross-Target Invariance
- **Tier 1 Native Platforms (Linux, macOS, Windows):** The `pilot` toolchain, runtime, self-hosted compiler, and standard library MUST pass all gates on Linux (`x86_64`, `aarch64`, `riscv64`), macOS (`aarch64`, `x86_64`), and Windows (`x86_64`, `aarch64`).
- **Tier 2 WebAssembly & Tier 3 Embedded/Mobile:** First-class compilation to `wasm32-wasip2`, `wasm32-unknown-unknown`, Android (`aarch64-linux-android`), iOS (`aarch64-apple-ios`), and freestanding bare-metal/RTOS (`thumbv7em`, `riscv32`).
- **Canonical Path & Line-Ending Invariance:** Normalize paths internally with `/` (handling Windows drive letters `C:/...` and UNC paths `//...` cleanly) and accept `\n`, `\r\n`, and `\r` without span/line drift.
- **OS-Agnostic Core (`RING001`) vs. Hosted `std.*`:** `core.*` contains `0` OS/libc imports (`bridge_os.pilot` is internal to the Ring-1/Ring-2 boundary). Hosted `std.*` exposes uniform Pilot signatures across Linux, macOS, Windows, and WASI.
- **Declarative Target ABIs & Zero-Host Cross-Compilation:** Drive SysV AMD64, Microsoft x64, ARM64 AAPCS64, RISC-V LP64D, and WASM Canonical ABI layouts from Pilot descriptors (`pilot.abi.*`), supporting `--target <triple>` cross-compilation out of the box.

---

## 11. Anti-Drift Discipline: Verify, Never Remember

Pilot is in no model's pre-training corpus. Earlier drift happened when agents guessed C#/Rust/Python syntax, wrote shortcuts in Rust instead of Pilot, scattered raw `__engine(...)` calls across 115 files, or trusted aspirational prose over real CLI runs (`Audit_keywords.md`, `rust_to_pilot_audit.md`). Every agent MUST obey these 12 rules:

1. **Run It Before Believing It:** Only behavior proven on `pilot run`, `pilot test --ir`, and `pilot test --emit c` (and evidenced in `wiring.pilot`) is real. `VISION.md`, `MILESTONES_100.md`, and `keywords.pilot` docstrings state target design; when a run disagrees with prose, trust the run and fix the prose or file the bug.
2. **Fail Loudly; Never Drop or Reinterpret:** The lexer, parser, and checker must reject unrecognized syntax with an explicit diagnostic code (`SYN001`, etc.) or a registered seam (`SEAM001`). Never parse and discard (`let _ = self.parse_expr()`), reinterpret unknown declarations as `struct`, or silently skip tokens.
3. **No Claim Without a Binding, a Passing Program & a Failing Program:** Every registered keyword, modifier, primitive, or `syntax` rule must ship in the same change with its grammar binding, a passing conformance program, and an `err_*` misuse program.
4. **Expected Values Come From Outside the Code:** Hand-work expected outputs from the spec (show the derivation in a test comment) or take them from published vectors / reference tools—never from running your own implementation.
5. **All Three Engines Must Agree:** Run runtime/language changes across `pilot test`, `pilot test --ir`, and `pilot test --emit c`. Disagreement between engines means one is wrong: fix it in Pilot or file a `known_bugs` repro in the same change.
6. **Fakes Are Replaced or Recorded Immediately:** Any rule, stdlib function, or engine path that imitates a capability (running synchronously instead of spawning, returning a constant, skipping a check) must be replaced by a real Pilot implementation, turned into a `seam(...)`, or marked `SIMULATED` in `wiring.pilot` immediately.
7. **Parser Edits Stay Crash-Free:** After any lexer or parser change, run `cargo test -p pilot-parser --test damaged_source -- --include-ignored` and fix any panic at the root cause.
8. **Keep `docs/keyword_matrix.md` & `docs/concept_matrix.md` Synchronized:** Check both matrices before using or claiming any keyword or language/runtime/toolchain concept. Whenever a keyword's binding, behavior, tests, or Pilot/Rust share changes, update its hand columns if needed and run `pilot run tools/keyword_matrix/main.pilot` (`-- --check` must exit `0`); whenever any non-keyword or multi-token concept changes status, engine parity, evidence, or Pilot/Rust ownership, update its row in [`docs/concept_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/concept_matrix.md) in the same change. Never mark any row higher than real CLI runs prove.
9. **Encode Knowledge in Executable Checks, Not Prose:** Put every invariant you uncover into a conformance test, a checker diagnostic, a `pilot gate` rule, or a `seams.pilot` entry so tools enforce it permanently.
10. **State Unchecked Claims Explicitly:** Opening reports must state any engine, gate, or platform not executed. Never present an unrun path as verified.
11. **Never Reach for Rust or Raw `__engine`:** Before touching any `.rs` file or typing `__engine`, ask: *"Can this be expressed in `<= 1,500` LOC of Pilot over existing typed core wrappers?"* If yes, putting it in Rust or adding a raw `__engine` call is an architectural violation—even if every test passes.
12. **Prefer Loud Pilot Seams Over Keeping Legacy Rust:** When deleting misplaced Rust breaks `.pilot` callers or unbuilt compiler stages, **do not keep the Rust code and do not paper over the break**. Replace the broken Pilot call site or pass with `seam("<area>.<what>", "<why broken & what Pilot pass replaces it>")`, register it in `seams.pilot` / `known_bugs`, downgrade its row in `keyword_matrix.md` / `concept_matrix.md` / `wiring.pilot`, and leave it loudly broken in Pilot until implemented in pure Pilot.
