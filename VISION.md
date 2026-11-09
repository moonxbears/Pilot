# VISION.md — Pilot Language & Unified Toolchain Master Reference (`1.0` $\rightarrow$ `2.0`)

> **Purpose:** Canonical reference for **human design continuity** and **AI agent recontextualization**. Capture every core ambition, design invariant, syntax pairing, Universal Foreign-Language Adapter architecture, `.pchor` sublanguage choreography model, boilerplate shortcut, MCP/code-assistance capability, and unified `pilot` toolchain goal driving the **100-Milestone Journey** ([`MILESTONES_100.md`](file:///home/moony/Documents/dev/projects/Pilot/MILESTONES_100.md)).

---

## 1. Core Philosophy & North Star

**Pilot** is both a **modern, self-hosted programming language** and an **all-in-one unified developer platform (`pilot`)** engineered around six non-negotiable pillars:

1. **Scalable & Extendible (Zero Rigid Corners):**
   - Never hardcode closed lists, magic numbers, or domain-specific assumptions in the compiler engine.
   - Everything is driven by open registries, protocol interfaces, structural physics (`[Bits]`, `[Layout]`, 2-bit lattice), and declarative tables so adding new types, targets, sublanguages, or CLI tools never requires rewriting core compiler logic.
2. **Hyper-Adaptable (Everything First-Class):**
   - Types (`type`), syntax trees (`ast`, `cst`), intermediate representation (`ir`), memory layouts, allocators/arenas, attributes, permissions (`lvl`), call frames (`frame`), reactive signals (`sig`), derivatives (`grad`), effect plans (`plan`), build graphs, and architectural blueprints (`.pchor`) are all **first-class citizens**.
   - Codebase-wide refactors (changing memory layouts from AoS to SoA, splitting monoliths into async tasks or services, evolving struct schemas) take a fraction of the time via `.pchor` choreography and semantic `pilot migrate` codemods.
3. **Top-of-Chart Performance (Zero GC, Zero Excuses):**
   - Target #1-tier benchmark performance alongside or surpassing hand-tuned C, C++, Rust, and Zig.
   - Achieved via the **2-Bit Hardware Lattice (`0b00`–`0b11`)**, deterministic **Perceus ARC with in-place constructor recycling (`drop_reuse`)**, lexical arenas, `0.0 ms` GC pauses, `comptime` specialization, SoA cache-line choreography, portable SIMD, LLVM TBAA/ThinLTO, and direct GPU kernel lowering.
4. **Seamless Foreign-Language Interoperability (Priority Pillar):**
   - Use C, C++, Rust, Zig, Python, JS/TypeScript, WASM, SQL, JVM, .NET, and Swift libraries *naturally* inside Pilot with minimal-to-zero downsides—no manual glue files, no copy-marshalling bottlenecks, no leaked foreign handles, and idiomatic Pilot API ergonomics via first-class `adapter` blueprints.
5. **Visually Beautiful & Natural to Read:**
   - Reject cryptic punctuation soup and lifetime-annotation noise (`'a`, `Pin<Box<...>>`).
   - Native built-in types and keywords use **short, readable, single-word `lowercase` names** (`bool`, `str`, `pkg`, `fn`, `let`, `var`, `opt`, `res`, `ext`, `iso`, `ast`).
   - **Dual Readability Law:** Every compact symbolic operator or projection MUST always have a short, readable `lowercase` keyword/projection equivalent so code reads naturally at every experience level.
6. **Progressive Learning Curve & Minimal Underlying Engine Dependency:**
   - Beginners write clean, intuitive code on Day 1 without touching advanced features—and are richly rewarded in speed, safety, and expressiveness as they progressively adopt facets, contracts, arenas, `comptime`, or `.pchor`.
   - **Pilot-First Independence:** Implement everything in Pilot first. Actively shrink and retire the underlying Rust bootstrap dependency (`crates/pilot-*`) down to an auditable, zero-Rust C11/machine seed—and even where hardware/OS boundaries exist, control them via adaptive Pilot descriptors.

---

## 2. Progressive 4-Tier Learning Curve

Pilot never forces esoteric type theory on everyday code. Developers opt into power progressively:

| Tier | Mode | What You Use | What Pilot Handles Automatically |
| :--- | :--- | :--- | :--- |
| **Tier 1** | **Everyday & Scripting** | `fn`, `let`, `var`, `str`, `List<T>`, `Map<K, V>` (`m[k]`), `if`, `for`, `switch`, `?` errors, `f"hello {name}"`, `pilot file.pilot` | Full type inference, deterministic Perceus memory management (`0` lifetime annotations, `0` GC pauses), clean error messages. |
| **Tier 2** | **Domain & Clean Architecture** | `struct`, `interface`, `*Bundle` & `*Facet` composition, implicit `field`/`value` properties, `..` cascades, `adapter`, `requires`/`ensures` contracts | 4-tier `MethodTable` dispatch, automatic boilerplate weavers (`[Eq]`, `[Json]`, `[Clone]`), zero-overhead contract checks. |
| **Tier 3** | **Systems & Peak Performance** | `slice<T>`/`stride<T>` (`0b01` views), `iso` zero-copy transfers, `with arena`, SIMD `T@N`, `[Kernel]` GPU, `[HardRealtime]`, `noalloc`, `.pchor` `SoA` layouts | Static borrow-escape proofs (`ESCAPE001`–`003`), Perceus in-place `drop_reuse`, SIMD vectorization, `[Wcet]` cycle verification. |
| **Tier 4** | **Meta & Language Design** | `comptime`, `quote` / `$splice` / `~unquote`, `[...]` AST weavers, custom `syntax` rules, `.pchor` sublanguages, `pilot.ast`/`pilot.ir` passes | Capability-sandboxed deterministic `comptime` execution, hygienic lexical alpha-renaming, automatic IDE highlighting & MCP schema sync. |

---

## 3. Canonical Syntax & Dual Readable/Symbolic Reference

### 3.1 Core Naming & Lattice Personality
- **Single-Word `lowercase` Primitives & Keywords:**
  `void`, `never`, `bool`, `char`, `i8`..`i128`, `isize`, `u8`..`u128`, `usize`, `f16`..`f128`, `str`, `ptr<T>`, `slice<T>`, `stride<T>`, `array<T, N>`, `simd<T, N>`, `ref<T>`, `iso<T>`, `opt<T>` (`option<T>`), `res<T, E>` (`result<T, E>`), `map<K, V>`, `fn<A, R>`, `frame<TSig>`, `sig<T>`, `grad<T, G>`, `plan<T>`, `type`, `ast`, `cst`, `ir`, `pkg`, `ext`, `let`, `var`, `switch`, `use`.
- **2-Bit Hardware Lattice (`IAnyValue` — `0` runtime header bytes):**
  - `0b00` **`IValue`** (`PascalCase`, e.g., `Vec3`, `Order`): Trivial `memcpy`, zero-copy `.pchor` cue & wire castable.
  - `0b01` **`IReference`** (`...View` / `...Lens` / `slice<T>` / `str`): Borrowed zero-copy window; cannot escape lexical owner.
  - `0b10` **`IObject`** (`...Buffer` / `List<T>` / `Map<K, V>`): Owns `~self()`; Perceus move-by-default + exact last-use `drop_reuse`.
  - `0b11` **`IGuard`** (`...Guard` / `...Scope` / `...Lease`): Borrows memory AND runs LIFO `~self()` at scope exit (locks, arenas, Python GIL, transactions).

### 3.2 Mandatory Readable Pairings for Every Compact Symbol
Every compact symbolic shorthand desugars through pure-Pilot syntax/receptor rules and has an explicit, readable `lowercase` form:

| Concept | Compact Symbolic Form | Readable `lowercase` Form | Result Type / Semantics |
| :--- | :--- | :--- | :--- |
| **Optional Type** | `User?` | `opt<User>` / `option<User>` | Niche-packed optional value (`some(x)` / `none`) |
| **Fallible Result** | `Data ! IoErr` | `res<Data, IoErr>` / `result<Data, IoErr>` | Zero-exception values-first error union |
| **Contiguous View** | `u8[]` | `slice<u8>` | `0b01` `{ ptr<u8>, usize }` borrowed view |
| **Inline Fixed Array** | `f32[16]` | `array<f32, 16>` | `0b00` inline value storage |
| **Strided Tensor View** | `f32[..]` | `stride<f32>` | `0b01` `{ ptr<f32>, usize, isize }` zero-copy lens |
| **SIMD Vector** | `f32@8` | `simd<f32, 8>` | Hardware SIMD register vector |
| **Shallow Call Quote** | `compute!(a, b)` | `call compute(a, b)` | `CallExpr<TRet>` inspectable typed call descriptor |
| **Deep AST Splice** | `compute!!(~a, b)` | `splice compute(~a, b)` | `ast<TRet>` homoiconic AST with `~` comptime unquoting |
| **Zero-Marshal ABI Frame** | `compute!&(a, b)` | `frame compute(a, b)` | `frame<FnSig>` `#[repr(C)]` call packet |
| **Reactive Signal Node** | `compute!^(a, b)` | `sig compute(a, b)` | `Signal<TRet>` glitch-free push-pull reactive node |
| **Symbolic Derivative** | `loss!@(w, x)` | `grad loss(w, x)` | `Dual<TRet, TGrad>` compile-time forward/reverse autodiff |
| **Transactional Dry-Run** | `transfer!?(a, b)` | `plan transfer(a, b)` | `EffectPlan<TRet>` CoW sandboxed execution plan |
| **1-Level Sequence Unpack** | `*items` | `splat(items)` | `splat<T>` positional unpack or `*Bundle` field splice |
| **Key-Value / Field Unpack** | `**config` | `pairs(config)` | `pairs<K, V>` structural field/map unpack |
| **Recursive Deep Flatten** | `*..nested` | `flat(nested)` | `expand<T>` leaf-element flattening |

### 3.3 Ergonomic Boilerplate Shortcuts
```pilot
pub pkg App.Billing;

// 1. Distributive headers eliminate repetitive modifiers & types:
pub static pure:
    fn cents_to_dollars(c: i64) -> f64 => (c as f64) / 100.0;
    fn tax_for(amount: f64, rate: f64) -> f64 => amount * rate;

// 2. Auto-woven attributes + *Bundle/*Facet composition + implicit properties:
[Eq, Clone, Json]
pub struct Account : IValue {
    *AuditHeaderBundle coalesce { id };
    *PrintableFacet;

    // Implicit backing `field` and setter `value` with reactive attribute pipeline:
    [Clamp(0.0, 1_000_000.0)]
    pub balance: f64 {
        get => field;
        set => { field = value; }
    }
}

// 3. Clean Map/List indexing, fluid cascades `..`, PrefixDot `.member`, and `?` errors:
pub fn process(accounts: var ref<Map<str, Account>>) -> f64 ! BillingErr {
    var acct = accounts["acct_01"]?;
    acct..deposit(250.0)..mark_verified();
    accounts["acct_01"] = acct;
    return acct.balance;
}
```

---

## 4. Priority Pillar: The Universal Foreign-Language Adapter

Pilot treats cross-language interoperability as a **first-class compiler & type-lattice capability**, allowing developers to absorb and use C, C++, Rust, Zig, Python, JS/TypeScript, WASM, SQL, JVM, .NET, and Swift libraries naturally with minimal downsides.

### 4.1 Five Laws of Zero-Downside Foreign Adaptation
1. **Zero Glue Files (`use <lang>` Semantic Absorption):**
   Import headers, crates, wheels, `.d.ts` definitions, WIT interfaces, or SQL/OpenAPI schemas directly at `comptime` without running external bindgen scripts or maintaining brittle wrapper files.
2. **Zero-Copy Memory Sharing Across Languages (`0b01` Views):**
   Map C `(ptr, len)`, Rust `&[T]`, C++ `std::span<T>`, Python PEP 3118 `memoryview` / NumPy `ndarray` / DLPack / Apache Arrow buffers, JS `TypedArray` / `SharedArrayBuffer`, and WASM linear memory directly onto Pilot `slice<T>` and `stride<T>`—zero byte copying!
3. **Automatic Cross-Runtime Lifecycle Safety (`0b10` & `0b11` Lattice Binding):**
   - Foreign heap handles (`FILE*`, `sqlite3*`, Rust `Box`, C++ RAII objects, `PyObject*`, JS `napi_ref`) bind to `0b10` `IObject` so Perceus `~self()` deterministically releases the foreign resource on exact last use.
   - Foreign runtime execution scopes (Python GIL, V8 `HandleScope`, JNI local frames) bind automatically to `0b11` `IGuard` LIFO stack scopes—preventing leaks, use-after-free, and GIL deadlocks invisibly.
4. **Declarative Idiom Lifting (`adapter` Blueprints):**
   Transform raw foreign APIs (C prefixes like `sqlite3_`, `(ptr, len)` pairs, integer error codes + out-pointers, foreign `snake_case`/`CamelCase`, JS `Promise<T>`) into natural Pilot methods, properties, `slice<T>`, `T ! Err`, and `task` awaitables.
5. **In-Process Native Hosting & Bidirectional SDK Export:**
   Never shell out to slow subprocesses for library calls. Link native ABIs directly (`0 ns` register calls + LLVM LTO inlining) and host managed runtimes (`libpython`, N-API/QuickJS, JVM, CLR) in-process. Conversely, `pilot ship --sdk py,ts,rust,csharp,c` exports any Pilot package to other ecosystems automatically.

### 4.2 Proposed Universal Adapter Syntax
```pilot
// Direct semantic imports — resolved, cached, and type-checked at comptime:
use c "sqlite3.h" as raw_sqlite;
use py "numpy" as np;
use rust "regex" as rx;
use ts "./payments.d.ts" as pay;

// Declarative adapter lifts a raw C API into clean, idiomatic, memory-safe Pilot:
pub adapter raw_sqlite -> SqliteDb {
    strip_prefix: "sqlite3_";
    handle:       raw_sqlite.sqlite3* as IObject (drop: raw_sqlite.sqlite3_close);
    slice_pair:   (ptr<u8>, i32) <-> str;
    error_code:   i32 (ok: raw_sqlite.SQLITE_OK) -> SqliteErr;
}

pub fn analyze_scores(db: var ref<SqliteDb>, scores: f64[]) -> f64 ! AppErr {
    // 1. Calls C sqlite3 at 0ns overhead with idiomatic Pilot error handling:
    let rows = db.exec("SELECT weight FROM factors")?;

    // 2. Passes `scores` (`0b01` slice<f64>) zero-copy to NumPy in-process (auto-GIL guard):
    let mean: f64 = np.mean(scores)?;
    return mean;
}
```

---

## 5. Sublanguage Choreographer & Architectural Blueprinter (`.pchor`)

Most large codebases decay because **domain logic** gets tangled with **infrastructure mechanics** (memory layout, threading, networking, serialization, locking). Pilot solves this with `.pchor` choreography files.

### 5.1 What `.pchor` Controls Without Rewriting Business Code
1. **Storage & Layout Choreography (`AoS` $\leftrightarrow$ `SoA` / Arena / GPU):**
   Keep `struct Particle { pub pos: Vec3; pub vel: Vec3; pub mass: f32; }` clean in domain code while `.pchor` transforms storage to `SoA<Particle>` aligned to 64B cache lines or stages it in GPU VRAM.
2. **Execution & Topology Choreography (In-Process $\leftrightarrow$ Actors $\leftrightarrow$ RPC):**
   Define `role` components and typed `cue` channels. Enforce the **2-Bit Cue Boundary Law** at `comptime` (only `0b00` `IValue` or `iso`-transferred `0b10` `IObject` may cross a `cue`, guaranteeing zero data races!). Re-route cues from direct calls to M:N work-stealing queues, shared-memory IPC, or network services purely in `.pchor`.
3. **Architectural Guardrails & Sublanguages:**
   Enforce dependency laws (*"UI roles cannot import DB drivers directly"*, *"Audio role must be `noalloc` + `[Wcet(500us)]`"*) and expose tailored domain sublanguages (`state`, `rule`, `route`, `query`) for team ergonomics.

```pilot
// physics_sim.pchor — Architectural & Layout Blueprint
pub blueprint PhysicsWorld {
    storage ParticleTable : SoA<Particle, align: 64, arena: FrameArena>;

    role Integrator [HardRealtime, Wcet(250us), noalloc] {
        reads:  ParticleTable.{vel, mass};
        writes: ParticleTable.pos;
    }

    role Broadphase [Kernel(workgroup: 256)] {
        reads: ParticleTable.pos;
        emits: cue CollisionBatch (iso List<ContactPair>);
    }

    route CollisionBatch -> Solver.resolve (transport: ring_buffer);
}
```

---

## 6. Unified `pilot` All-in-One Toolchain & AI/MCP Co-Design

`pilot` is a single self-hosted binary serving beginners (1-command scripting) and systems architects alike, backed by **one shared incremental compiler database (`PilotDb`)** and **one declarative `[Command]` registry**:

| Command | Responsibility |
| :--- | :--- |
| `pilot run [file]` | Instant zero-config script execution (`PilVM` / sub-ms native backend) or state-preserving `--watch` hot-reload. |
| `pilot build` / `pilot ship` | Compile to Native (LLVM / fast x86_64 & ARM64), C11, WASM, or 8 GPU targets; cross-compile static binaries or export polyglot `--sdk` packages. |
| `pilot check [--fix] [--prove]` | Millisecond incremental type, lattice, Perceus escape, `.pchor` architecture, and SMT contract verification with lossless CST auto-fixes. |
| `pilot pkg` | Manage `pkg.pilot` workspaces, hermetic lockfiles, content-addressed caches, AND foreign C/Rust/Python/NPM/WASM dependencies. |
| `pilot fmt` / `pilot doc` | Wadler-Lindig lossless CST formatter; living documentation generator with verified `///` examples, facet badges, and `.pchor` diagrams. |
| `pilot test [--fuzz]` / `pilot bench` | Unit, doc, differential, and type-derived coverage-guided fuzz testing; hardware-counter statistical benchmarking vs. C/Rust. |
| `pilot inspect <target>` | **Zero-Black-Box Transparency:** View expanded macro/attribute weaves, exact byte offsets, lattice bits, and Perceus `drop`/`drop_reuse` proofs. |
| `pilot absorb <target>` | Convert foreign headers/schemas or incrementally migrate C/Python/TS/C#/Rust modules into idiomatic Pilot + `adapter` bridges. |
| `pilot migrate` | Lossless CST codebase refactoring engine applying `[Migrate]` API upgrades or Edition transitions (`1.0` $\rightarrow$ `2.0`) across entire repos. |
| `pilot lsp` / `pilot debug` | Resident sub-10ms Language Server (with programmable snippets & weave lenses) and unified Native + VM + Polyglot + Time-Travel DAP debugger. |
| `pilot mcp` | **Native Model Context Protocol Server:** Compiler-in-the-loop AI collaboration engine. |

### 6.1 Deep AI & Developer Assistance (`pilot mcp`, Object Synthesis & Adaptable Snippets)
- **Token-Efficient Semantic Context:** Instead of dumping raw 1,000-line files, `pilot mcp` serves exact resolved types, 4-tier `MethodTable` origins, Perceus ownership constraints, and `.pchor` role contracts.
- **Millisecond In-Memory Speculative Sandbox:** Agents and IDE tools test candidate AST edits, fuzz properties, and run `plan` (`!?`) dry-runs inside `PilotDb` in memory before touching disk.
- **Context-Aware Object & Snippet Generation:** Given an interface, SQL/OpenAPI schema, typed hole (`_?`), or `.pchor` role, `pilot` synthesizes valid struct implementations, test fixtures, and live context-sensitive snippets tailored to variables in scope.

---

## 7. AI Agent Recontextualization Checklist

Whenever starting a task on Pilot, align every design and code change against these 7 checks:

1. **Pilot-First, Never Hardcode:** Can this live in Pilot (`stdlib/`, `selfhost/`) using tables, interfaces, attributes, or syntax rules instead of hardcoding names or growing low-level bootstrap code?
2. **Universal Rename Litmus:** Did you avoid hardcoding ANY user-space primitive or composite type name (`"i64"`, `"str"`, `"slice"`, `"Map"`, etc.) inside the seed engine?
3. **Scalable Table-Driven Design:** Did you use extensible registries/maps rather than rigid `if/else` chains or magic counts?
4. **Dual Readability:** Is the syntax clean and natural, pairing any symbolic shorthand with a short, single-word `lowercase` readable form?
5. **Boilerplate & Ergonomics:** Does this reduce friction for Tier-1 beginners while giving Tier-3/4 systems users full layout and `comptime` control?
6. **Zero Legacy Clutter:** When replacing an older mechanism or shim, did you delete the superseded legacy code in the same milestone?
7. **Three Mandatory Gates:** Does the change pass:
   - *Gate 1 (Adaptability):* Zero hardcoded names + Universal Rename Litmus.
   - *Gate 2 (Bootstrap):* Deterministic self-compilation parity + strict compile-speed budget.
   - *Gate 3 (Performance):* Zero unintended allocations + C/Rust-competitive runtime execution.
