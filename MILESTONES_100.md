# MILESTONES_100.md — Pilot 100-Milestone Master Journey (`1.0` at M50 $\rightarrow$ `2.0` at M100)

> **Companion References:** Read alongside [`VISION.md`](file:///home/moony/Documents/dev/projects/Pilot/VISION.md) (philosophical goals, syntax pairings, Universal Foreign-Language Adapter spec, `.pchor` choreography, and AI recontextualization checklist), [`AGENTS.md`](file:///home/moony/Documents/dev/projects/Pilot/AGENTS.md) (core syntax & 2-bit lattice physics), and [`imp_type.md`](file:///home/moony/Documents/dev/projects/Pilot/imp_type.md) (foundational typing philosophy, 6 Core Laws, 12 Invariant Rules, 2-bit hardware lattice taxonomy, single source of truth / import discipline, and intrinsic encapsulation).
>
> **Baseline (`M0` / Current State — corrected by the 2026-11-09 audit, see [`Audit.md`](Audit.md)):** Pilot is pre-alpha. The version string `v1.1.51` in `Cargo.toml` overstates this and should be reset to `0.x`.
> - **What executes:** a Rust lexer/parser plus a Rust bytecode compiler and VM (`crates/pilot-*`, `5,363` lines / ~`353` KB across 12 crates). It correctly runs integer arithmetic, strings, maps, `while`, `if`, functions, structs, methods and interface dispatch.
> - **What does not execute yet:** `for`, `switch`, f-strings, closures, floats, `defer`, constructors, destructors, `?`/`??`, cascades and linked imports. Type, name and syntax errors are not reported. There is no type checker, no reference counting, and no path from Pilot source to native code.
> - **Self-hosting:** not achieved. `selfhost/pilotc.pilot` is a fixed-shape template, so the Stage-2 == Stage-3 parity gate is not yet meaningful. The `66,892` lines of Pilot in `selfhost/` and `stdlib/` run on the Rust VM and read the AST the Rust parser builds.
> - **Design:** the 2-Bit Hardware Lattice (`0b00`–`0b11`), 4-tier `MethodTable` and the typing physics in [`imp_type.md`](file:///home/moony/Documents/dev/projects/Pilot/imp_type.md) are specified; they are not yet enforced by a working analyzer.
>
> **Governing Rule:** Every milestone (`M1`–`M100`) delivers a **meaningful, non-trivial architectural, ergonomic, interoperability, performance, or tooling leap**, deletes any legacy code it supersedes, strictly implements [`imp_type.md`](file:///home/moony/Documents/dev/projects/Pilot/imp_type.md), and passes the **3 Continuous Gates** (Adaptability/Rename Gate, Fixed-Point & Compile-Speed Gate, Peak Runtime Performance Gate).

---

## High-Level Phase Architecture (10 Phases + Foundation Phase F)

| Phase | Milestones | Era | Core Theme |
| :--- | :--- | :--- | :--- |
| **Phase 1** | `M1` – `M10` | Road to `1.0` | **Seed Unlock, Unified `PilotDb`, `PilIR` & Dual-Syntax Foundation** |
| **Phase 2** | `M11` – `M20` | Road to `1.0` | **Priority Pillar: Universal Foreign-Language Adapter v1 (C, C++, Rust, Python, JS/TS, WASM, Schemas)** |
| **Phase 3** | `M21` – `M30` | Road to `1.0` | **Boilerplate Eradication, `.pchor` Sublanguage Choreographer v1 & Sandboxed `comptime`** |
| **Phase F** | `F1` – `F8` | Road to `1.0` | **Foundation (inserted 2026-11-09): Correct Seed, Module Linking, Surfaced Errors, Type Checker, Real `PilIR`, C11 Backend, Runtime Ownership & Real Self-Hosting — blocks `M31`+** |
| **Phase 4** | `M31` – `M40` | Road to `1.0` | **C/Rust-Beating Performance, LLVM/SIMD/GPU & Zero-Rust Toolchain Decoupling** |
| **Phase 5** | `M41` – `M50` | **PILOT 1.0** | **All-in-One `pilot` Toolchain (`pkg.pilot`, LSP, DAP, MCP v1, `ship`) & 1.0 Full Release** |
| **Phase 6** | `M51` – `M60` | Road to `2.0` | **Live Codebase Adaptation (`pilot migrate`, Editions, `.pchor` Distributed Topology & Hot-Reload)** |
| **Phase 7** | `M61` – `M70` | Road to `2.0` | **Deep Polyglot Mastery v2 (JVM, .NET, Swift, Zig/Go, Arrow/DLPack, `pilot absorb` & Universal SDK Export)** |
| **Phase 8** | `M71` – `M80` | Road to `2.0` | **AI Co-Design (`pilot mcp` v2, Hole Synthesis, Live Snippets), Reactive UI & Studio** |
| **Phase 9** | `M81` – `M90` | Road to `2.0` | **Extreme Hardware Physics (PGO, Loop Fusion, Custom ISAs, `[Wcet]`/SMT Proofs & 100% Pure-Pilot Repo)** |
| **Phase 10** | `M91` – `M100` | **PILOT 2.0** | **Global Registry, Multi-Editor Suite, Flagship Proof Engines & Pilot 2.0 Release** |

---

## Milestone Status Legend (2026-11-09 Audit Reset)

The earlier `[COMPLETE]` markers on `M1` and `M11`–`M30` recorded that a milestone's test file passed. The 2026-11-09 audit ([`Audit.md`](Audit.md)) found that those tests call library functions directly with hand-built inputs, so they pass whether or not the feature works in the language. Each heading now carries the audited status instead. The evidence for every tag is in `Audit.md` section 6.

| Tag | Meaning |
| :--- | :--- |
| `[STATUS: INTEGRATED]` | Works on arbitrary input through the `pilot` CLI or the normal compile pipeline. **No milestone has this status yet.** |
| `[STATUS: PARTLY INTEGRATED]` | Some of the milestone is reachable and working through the CLI; the rest is one of the lower states. |
| `[STATUS: LIBRARY ONLY]` | A genuine, general algorithm exists in Pilot, but only tests call it. Nothing in the parser, analyzer, VM or CLI is connected to it. |
| `[STATUS: SIMULATED]` | The code operates on hand-built descriptors or returns fixed values in place of the real behaviour. It must be reimplemented, not merely wired up. |
| `[STATUS: INCOMPLETE]` | Specified work (including mandatory migrations or deletions) was largely not carried out. |
| `[STATUS: NOT AUDITED]` | Not examined by the audit; the earlier completion claim is neither confirmed nor refuted. |
| *(no tag)* | Not started. |

**Rule for future status changes:** a milestone may be tagged `[STATUS: INTEGRATED]` only when its acceptance tests feed source files through the `pilot` CLI (`run`, `check`, `build`, …) and check stdout and exit codes. A passing function-level test alone moves a milestone no further than `[STATUS: LIBRARY ONLY]`.

---

## Universal `imp_type.md` Architecture & Typing Physics Contract Across All 100 Milestones

Every milestone from `M1` through `M100` is bound by the typing physics, structural taxonomy, and modular laws codified in [`imp_type.md`](file:///home/moony/Documents/dev/projects/Pilot/imp_type.md). No milestone may introduce unconstrained types, duplicate type definitions, blanket wildcard import pollution, or unmanaged intrinsic bypasses. The six foundational typing laws apply universally across standard library, selfhost compiler, foreign adapters, and tooling implementations:

### The 6 Foundational Typing Laws of `imp_type.md`
1. **Unambiguous Determinism (Law 1):** If values and lifecycle are known at design time, the definition and type must be immediately obvious. Never leave semantics up to interpretation or stringly-typed guesswork.
2. **Appropriate Constraining (Law 2):** Constrain as tightly as the domain requires, but no tighter. Broad, open-ended types (`any`, bare `str`, bare `i64`) should only represent genuinely open-ended varieties of values.
3. **Generic Precision (Law 3):** Never write unconstrained generics (`<T>`) when capabilities are required. Always constrain with interfaces (`<T : IComparable<T>>`, `<T : IAnyValue>`).
4. **Hardware Lattice Alignment (Law 4):** Every type must classify into the 2-bit hardware lattice in 1 CPU cycle (`0b00` Value Type `: IValue`, `0b01` Borrowed Reference View `: IReference`, `0b10` Owned Dynamic Object `: IObject`, `0b11` Scoped Guard `: IGuard`).
5. **Single Source of Truth & Targeted Importing (Law 5 / Rule 11 — Import-First Law):** Every type must have exactly one canonical definition in its owning module or package. Types must **never be duplicated, copied, re-declared, or shadowed** across different files or subsystems. When importing, code must **import only what is needed** (`use Pkg.Module.Type;`) rather than blanket wildcard importing an entire library (`use Pkg.Module.*;`), unless genuinely consuming the entire module or 3+ cohesive items.
6. **Intrinsic Encapsulation (Law 6 / Rule 12):** Raw `__engine(...)` calls are strictly forbidden in application, domain, standard library, and self-host code. Code must always use strongly-typed standard and core types that encapsulate engine intrinsics (such as `Map<K, V>`, `List<T>`, `SliceBuffer<T>`, `fs.*`, `time.*`).

### The 12 Invariant Rules Applied Across All Milestones
- **Rule 1 (Discrete States $\le 19$):** Strongly typed `enum : u8/u16` with target-typed `::Variant`. Zero magic numbers or stringly constants.
- **Rule 2 (Large Predefined Sets $20+$):** Open-registry pattern with table-driven `array<T, N>` or `comptime Map<K, V>` and integer ID handles.
- **Rule 3 (Fixed Composite Records):** `[Layout(c_abi)] struct : IValue` (`0b00`) with trivial `memcpy` and zero allocation overhead.
- **Rule 4 (Varying Implementations):** `interface` contracts with concrete `struct`s, composing shared state via `*Bundle` and shared methods via `*Facet` (no OOP base classes).
- **Rule 5 (Combinable Flags):** `bitflags : u32/u64` (`0b00 : IValue`). Zero multiple boolean fields or boolean blindness.
- **Rule 6 (Absence & Errors):** `opt<T>` (`T?`) for optionality and `res<T, E>` (`T ! E`) for fallible operations. Zero custom sentinels (`-1`, `null`, `""`).
- **Rule 7 (Memory Boundaries):** Distinguish read-only borrowed views (`0b01` `slice<T>` / `...View`), owned buffers (`0b10` `List<T>` / `...Buffer`), and scoped resources (`0b11` `...Guard`).
- **Rule 8 (Constrained Generics):** Always constrain generic type parameters (`<T : IConstraint>`).
- **Rule 9 (Bounded Typestates):** Model lifecycle state progressions as distinct types consumed via `iso self`.
- **Rule 10 (Quantitative Invariants):** Single-field `[Layout(c_abi)] struct : IValue` with construction validation returning `Self ! ValidationError`.
- **Rule 11 (Import-First & Targeted Import):** Single canonical definition, targeted imports, zero duplicates.
- **Rule 12 (Intrinsic Encapsulation):** All collections and OS operations encapsulated in safe, lattice-aware types (`Map<K, V>`, `List<T>`, `fs.*`).

---

## Multiplatform Compatibility Contract & Tiered Target Matrix

Multiplatform compatibility is a non-negotiable, first-class architectural invariant across the entire 100-milestone roadmap. Pilot does not treat non-Linux operating systems as secondary ports or afterthoughts: every subsystem—from the lossless CST and `PilotDb` to the `PilIR` intermediate representation, `comptime` sandbox, Perceus ARC, foreign language adapters, and `.pchor` choreographies—is designed for host and target cross-platform invariance from day 1.

### Target Tier Taxonomy

| Tier | Target Platforms & Architectures | Operational Mandate & Continuous CI Verification |
| :--- | :--- | :--- |
| **Tier 1 (First-Class Host & Target Parity)** | **Linux** (`x86_64`, `aarch64` / ARM64, `riscv64gc` glibc & musl)<br>**macOS** (`aarch64` Apple Silicon, `x86_64` Intel)<br>**Windows** (`x86_64`, `aarch64` Windows on ARM, MSVC & MinGW) | Full CLI toolchain (`pilot`), self-hosting compiler (`pilotc`), register-based PilVM runtime, incremental `PilotDb`, LSP server, DAP debugger, MCP server, all 10 foreign language adapters, and 100% test gate parity. Bit-for-bit reproducible self-hosting across all Tier 1 hosts. |
| **Tier 2 (Web & Cloud-Native Sandboxed Wasm)** | **WASI Preview 2** (`wasm32-wasip2`, `wasm32-wasi`)<br>**Web / Browser** (`wasm32-unknown-unknown` + WebGPU / Canvas) | First-class WebAssembly Component Model (`.wit` auto-generation, canonical ABI lowering), in-process fuel and memory sandbox isolation, and standard library polyfills with zero OS leaks. |
| **Tier 3 (Mobile & Embedded / RTOS Systems)** | **Mobile:** Android (`aarch64-linux-android`, `x86_64`), iOS (`aarch64-apple-ios`, `aarch64-apple-ios-sim`)<br>**Embedded:** Bare-metal ARM Cortex-M (`thumbv7em-none-eabi`), RISC-V (`riscv32imc-unknown-none-elf`), RTOS (FreeRTOS, Zephyr) | Strictly freestanding `core.*` (`RING001`, exactly `0` external OS/libc symbols), deterministic memory footprint, and low-latency realtime execution under `[HardRealtime]` and `[Wcet]`. |

### The 7 Universal Multiplatform Invariants

1. **Canonical Virtual Path Normalization:**
   All compiler internals (`PilotDb`, CST, AST, `ModuleGraph`, `pkg`, `use`, diagnostic spans) normalize filesystem paths internally using forward slashes (`/`). Windows drive letters (`C:/...`), UNC network shares (`//server/share/...`), and case-insensitive filesystem lookups are handled losslessly without leaking host-specific path quirks into compiler hashes, `FileId` maps, or diagnostic spans.
2. **Line-Ending Invariance (`\n`, `\r\n`, `\r`):**
   Lexers, parsers, lossless CST tools, and `SourceMap` coordinate resolvers process POSIX `\n`, Windows `\r\n`, and legacy `\r` bit-for-bit identically. Spans, line numbers, and column offsets never skew due to line ending variations or source file checkout normalization.
3. **Target ABI & Calling Convention Fidelity:**
   Codegen and FFI lowerings derive calling conventions, register partitions, and memory layouts declaratively from target descriptors (`TargetDescriptor`, `pilot.abi.*`) rather than the host system:
   - **System V AMD64:** POSIX x86_64 standard (`RDI`, `RSI`, `RDX`, `RCX`, `R8`, `R9`).
   - **Microsoft x64:** Windows x86_64 ABI (`RCX`, `RDX`, `R8`, `R9` with mandatory 32-byte caller shadow space and struct-return pointer rules).
   - **AAPCS64 & Apple Darwin:** ARM64 ABI (handling Darwin-specific Mach-O stack variadic packing and Homogeneous Floating-point Aggregates `HFA` per Apple ABI).
   - **RISC-V LP64D:** Standard RISC-V 64-bit calling convention (`a0..a7`, `fa0..fa7`).
   - **WASM Canonical ABI:** Direct register passing up to 16 flat values with indirect memory spill and `cabi_post_*` cleanup.
4. **Dynamic In-Process FFI & Linker Independence:**
   Dynamic library loading adapts dynamically per OS without external glue:
   - POSIX: `dlopen`, `dlsym`, `dlerror` (`.so` on Linux/Android, `.dylib` on macOS/iOS).
   - Windows: `LoadLibraryW`, `GetProcAddress`, `FormatMessageW` (`.dll`).
   - WASM: Component model table imports and linear memory buffer mapping.
5. **Freestanding `core.*` vs Hosted `std.*` Isolation (`RING001`):**
   `core.*` contains strictly 0 OS syscalls and 0 libc imports, enabling compilation on bare-metal, microcontrollers, and microkernels. Hosted `std.*` delegates to pluggable platform backends (`std.fs`, `std.io`, `std.net`, `std.sync`, `std.proc`) mapping to native OS facilities:
   - Linux: `io_uring` / `epoll`, `futex`, `clone`/`fork`+`execve`.
   - macOS: `kqueue`, `os_unfair_lock`, `posix_spawn`.
   - Windows: `IOCP` / Overlapped I/O, `WaitOnAddress` / `SRWLock`, `CreateProcessW`.
   - WASI: `poll_oneoff`, cooperative fibers.
6. **Zero-Host Cross-Compilation from Day 1:**
   The `pilot` toolchain can cross-compile from any host OS to any target triple (`--target <triple>`) out of the box using bundled target descriptors, pure-Pilot sysroots, and target-neutral `PilIR`.
7. **Deterministic Cross-Platform Reproducibility:**
   All hashes (`pilot.lock`, `.pilc` binaries, gate audits) use endian-neutral, path-normalized serialization. A project built on Windows, macOS, or Linux produces bit-for-bit identical binary artifacts.

---

# PART I: THE ROAD TO PILOT 1.0 (`M1` – `M50`)

## Phase 1 (`M1` – `M10`): Seed Unlock, Unified `PilotDb`, `PilIR` & Dual-Syntax Foundation (UNABRIDGED SPECIFICATION)

> **Unabridged Specification Contract:** Every milestone in Phase 1 (`M1`–`M10`) specifies the complete engineering blueprint: (1) Current Codebase Audit & Root-Cause Bottlenecks, (2) Complete File & Package Manifest with strict `<= 1,500` LOC ceilings, (3) Full Data Structures, 2-Bit Hardware Lattice Classifications & Open Registries, (4) Public Signatures & Engine Boundary Contracts, (5) Step-by-Step Algorithmic Execution & Complexity Bounds, (6) CLI, JSON & Diagnostic Surface Contracts, (7) Mandatory Legacy & Rust Code Deletions, (8) Edge-Case Matrix, and (9) Concrete Acceptance Tests + 3-Gate Pass Criteria. Only literal code bodies are omitted.

---

### M1 — Seed Unlock & Pure-Pilot Architectural Gate Runner (`pilot gate`) [STATUS: PARTLY INTEGRATED]

#### 1.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Frozen Rust Tamper Seal):** [`crates/pilot-analysis/tests/common/anticheat.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/tests/common/anticheat.rs) (lines `234`–`271`) enforces `FROZEN_RUST_AND_GATE_HASHES` across 36 Rust and Python files via `verify_frozen_rust_and_gate_tamper_seal()`. Any effort to shrink Rust crates, fix CLI reflection wiring, or retire Rust bridges immediately panics with `ANTI-CHEAT [PASS 1: ILLEGAL_RUST_OR_GATE_MODIFICATION]`.
- **Root Bottleneck 2 (Over-Broad Bootstrap Substring Ban):** Pass 8 in `anticheat.rs` bans valid reflection/bridge substrings inside `selfhost/ast.pilot` and `selfhost/parser.pilot`, forcing awkward indirection instead of clean Pilot abstractions.
- **Root Bottleneck 3 (External Python Gate Dependency):** Architectural verification currently depends on 4 external Python scripts (`scripts/gate1_check.py`, `scripts/gate2_check.py`, `scripts/gate3_check.py`, `scripts/gate4_selfhost_contraction.py`) plus legacy migration script `scripts/convert_dunders.py`, violating the Pilot-First self-reliance mandate and hardcoding static crate dictionaries in Python.

#### 1.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/gate/manifest.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Gate.Manifest;` | `~220L` | Declarative open registries of seed tokens, retired crates/files, LOC ceilings, and benchmark budgets (derived via `.len()`, zero magic numbers). |
| **CREATE** | `selfhost/gate/seed_audit.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Gate.SeedAudit;` | `~340L` | Pure-Pilot scanner verifying surviving Rust files contain strictly the 14 seed tokens, zero user-space type/symbol names, zero `#[allow(dead_code)]`, zero `todo!()`/`unimplemented!()`, and monotonically decreasing Rust LOC. |
| **CREATE** | `selfhost/gate/type_audit.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Gate.TypeAudit;` | `~280L` | Exhaustive `imp_type.md` compliance scanner: verifies zero duplicate type definitions across files (Single Source of Truth), targeted import discipline (no blanket wildcards), zero raw `__engine` invocations outside seed tokens, `bitflags` for orthogonal flags, explicit `res`/`opt` instead of sentinels, and proper `[Layout(c_abi)]` attributes. |
| **CREATE** | `selfhost/gate/rename_litmus.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Gate.RenameLitmus;` | `~310L` | Ephemeral in-memory/sandboxed Universal Rename Litmus runner (`char` $\rightarrow$ `c32`, `slice` $\rightarrow$ `window`, `option` $\rightarrow$ `maybe`, `result` $\rightarrow$ `outcome`, `range` $\rightarrow$ `interval`, `map` $\rightarrow$ `dict_view`) across VM, C11, LLVM, WASM, ASM, and GPU emitters. |
| **CREATE** | `selfhost/gate/bootstrap_parity.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Gate.BootstrapParity;` | `~260L` | Stage-2 == Stage-3 deterministic SHA-256 parity verifier (both `.pilc` bytecode and native C11 binary) + compile-time wall-clock & allocation ceiling monitor. |
| **CREATE** | `selfhost/gate/runner.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Gate.Runner;` | `~240L` | Unified orchestrator executing registered `IGateCheck` implementations, aggregating timings, and emitting human ANSI or `--json` machine reports. |
| **MODIFY** | `crates/pilot-analysis/tests/common/anticheat.rs` | Rust integration test helper | `-210L` | Delete `FROZEN_RUST_AND_GATE_HASHES` and `verify_frozen_rust_and_gate_tamper_seal`; replace static hash lock with live invocation of `selfhost/gate/runner.pilot` invariant checks while keeping zero-mock & real-oracle test assertions intact. |
| **MODIFY** | `crates/pilot-cli/src/main.rs` | Transitional CLI bootstrapper | `+4L` | Route `pilot gate [flags]` to `selfhost/gate/runner.pilot` until M3 unifies all CLI dispatch. |
| **DELETE** | `scripts/gate1_check.py` .. `scripts/gate4_selfhost_contraction.py` & `scripts/convert_dunders.py` | Legacy Python scripts | `-100%` | Permanently delete `scripts/` Python files once `pilot gate` achieves 100% check parity and stricter coverage. |

#### 1.3 Complete Data Structures, Registries & 2-Bit Lattice Physics
- **`GateCategory` (`enum : u8`, `0b00 : IValue`)**: `::Adaptability` (`0`), `::BootstrapParity` (`1`), `::Performance` (`2`), `::Hygiene` (`3`), `::TypeHygiene` (`4`).
- **`GateSeverity` (`enum : u8`, `0b00 : IValue`)**: `::Blocker` (`0`), `::Warning` (`1`), `::Advisory` (`2`).
- **`[Layout(c_abi)] pub struct SeedTokenRegistry : IValue`**:
  - Holds `tokens: slice<str>` populated from the single canonical 14-token declaration (`__engine`, `struct`, `interface`, `fn`, `let`, `ref`, `if`, `loop`, `return`, `yield`, `defer`, `comptime`, `quote`, `asm`).
  - `pub pure fn count() -> usize => .tokens.len();` (never returns hardcoded `14`).
  - `pub pure fn contains(word: str) -> bool` via pre-sorted binary search or open-addressed hash probe.
- **`[Layout(c_abi)] pub struct CrateCeilingRule : IValue`**:
  - Fields: `crate_path: str`, `max_loc: usize`, `must_be_absent: bool`.
  - Stored in `Map<str, CrateCeilingRule>` where `max_loc` acts as a **ratchet ceiling** (Rust LOC may decrease freely down to `0` and crates may transition to `must_be_absent = true`, never increase without failing `GATE003`).
- **`[Layout(c_abi)] pub struct GateViolation : IValue`**:
  - Fields: `code: str` (`GATE001`–`GATE009`), `file_path: str`, `line: usize`, `column: usize`, `offending_snippet: str`, `message: str`, `remediation: str`.
- **`[Layout(c_abi)] pub struct GateReport : IObject`**:
  - Fields: `gate_id: str`, `category: GateCategory`, `passed: bool`, `elapsed_us: u64`, `items_scanned: usize`, `violations: List<GateViolation>`, `~self()` releasing `violations`.
- **`pub interface IGateCheck : IAnyValue`**:
  - `pub pure fn id() -> str;`
  - `pub pure fn category() -> GateCategory;`
  - `pub fn execute(workspace_root: str, opts: ref<GateRunOptions>) -> GateReport;`

#### 1.4 Public Signatures & Engine Boundary Contract
- `pub fn run_all_gates(workspace_root: str, opts: ref<GateRunOptions>) -> List<GateReport>;`
- `pub fn run_gate_by_id(workspace_root: str, gate_id: str, opts: ref<GateRunOptions>) -> opt<GateReport>;`
- `pub pure fn verify_rust_source_name_blind(rs_path: str, rs_source: str, seed_reg: ref<SeedTokenRegistry>, forbidden_pilot_syms: ref<Map<str, bool>>) -> List<GateViolation>;`
- `pub fn execute_universal_rename_litmus(workspace_root: str, rename_pairs: slice<(str, str)>) -> GateReport;`
- **Host Filesystem Intrinsics Used (via `stdlib/io/fs.pilot` & `stdlib/os/os.pilot`):** Directory listing (`fs.read_dir` / recursive walker added cleanly to `stdlib/io/fs.pilot` wrapping POSIX `opendir`/`readdir` in `pilot-rt` with 0 hardcoded Pilot names), file reading (`fs.read_text`), monotonic microsecond clock (`time.now_us`), and SHA-256 (`stdlib/crypto/hash.pilot` — 100% pure Pilot!).

#### 1.5 End-to-End Algorithmic Pipeline & Complexity Bounds
1. **Dynamic Vocabulary Harvesting ($O(S)$ where $S$ = `stdlib/core/` bytes):**
   - Instead of maintaining a hardcoded list of banned Pilot type names in the gate scanner, `seed_audit.pilot` dynamically parses `stdlib/core/primitives/*.pilot` and `stdlib/core/*.pilot` to harvest *every* declared `struct` and `interface` identifier (`i64`, `u8`, `str`, `slice`, `option`, `result`, `IAnyValue`, `IValue`, `SliceBuffer`, etc.), subtracting `SeedTokenRegistry` and Rust standard keywords (`fn`, `let`, `struct`, `if`, `loop`, `return`, `ref`).
2. **Rust Lexer-Aware String & Ident Audit ($O(R)$ where $R$ = surviving `.rs` bytes):**
   - Scans every `crates/*/src/**/*.rs` file stripping `//` and `/* ... */` comments, then inspects every string literal `"..."` and match arm pattern. Any match against a harvested Pilot type/interface/keyword name outside `SeedTokenRegistry` emits `GATE001 (NameBlindBreach)` with exact `file:line:col`.
   - Simultaneously checks per-crate and total Rust LOC against `CrateCeilingRule` (`GATE003`), bans `#[allow(dead_code)]` (`GATE004`), bans `todo!`/`unimplemented!` (`GATE005`), and verifies all retired crate paths remain absent (`GATE006`).
3. **Pilot Modular Size & Stub Audit ($O(P)$ where $P$ = `.pilot` bytes):**
   - Scans all `.pilot` files in `selfhost/` and `stdlib/`: enforces `<= 1,500` lines per file (`GATE002`), zero `TODO`/`FIXME` stub markers in code (`GATE005`), and `[EnforceCoreNaming]` compliance in `stdlib/core/` (`GATE007`).
4. **Dual-Pass Universal Rename Litmus ($O(B)$ where $B$ = bootstrap + emitter suite):**
   - Clones `stdlib/core/` into an isolated temp workspace arena (cleaned up via `0b11` `TempDirGuard` `~self()` even on panic/early return!), applies simultaneous whole-token alpha-renaming across 6 core primitives/algebraics (`char` $\rightarrow$ `c32`, `slice` $\rightarrow$ `window`, `option` $\rightarrow$ `maybe`, `result` $\rightarrow$ `outcome`, `range` $\rightarrow$ `interval`, `str` $\rightarrow$ `utf8view`), runs full bootstrap + typecheck + codegen across PilVM, C11, LLVM IR, WASM, x86_64 ASM, and WGSL/PTX, and verifies valid output (`GATE008`).
5. **Bootstrap Parity & Speed Check:**
   - Verifies `SHA256(stage2.pilc) == SHA256(stage3.pilc)` and `SHA256(stage2.c) == SHA256(stage3.c)` using pure-Pilot `stdlib/crypto/hash.pilot`, asserting compile wall time stays within budget (`GATE009`).
6. **Type Hygiene & `imp_type.md` Audit ($O(P)$):**
   - Scans all `.pilot` files via `selfhost/gate/type_audit.pilot` ensuring: (a) zero duplicate struct/enum/interface definitions across files (Single Source of Truth), (b) targeted import discipline (zero blanket wildcard `use Pkg.*` imports), (c) zero raw `__engine(...)` invocations outside seed tokens (Intrinsic Encapsulation), (d) zero boolean blindness ($\ge 3$ bools $\rightarrow$ `bitflags`), (e) zero sentinel error returns (`-1`, `null` $\rightarrow$ `opt<T>`/`res<T, E>`), and (f) proper `[Layout(c_abi)]` on `0b00` value structs (`GATE010`).

#### 1.6 CLI, JSON & Diagnostic Surface Contracts
- **Invocation:**
  - `pilot gate` — runs fast mandatory gates (`Adaptability`, `Hygiene`, `TypeHygiene`, `LOC Ratchet`) in `< 500 ms`.
  - `pilot gate --full` — includes full Dual-Pass Universal Rename Litmus + Stage-3 Fixed-Point Parity + Benchmark Sanity.
  - `pilot gate --only <id>` / `pilot gate --json` / `pilot gate --ratchet-update` (lowers Rust LOC caps in `manifest.pilot` after shrinking Rust code).
- **Exit Codes:** `0` = all blocker gates passed; `1` = one or more `GATE001`–`GATE010` blocker violations; `2` = invalid CLI flags.
- **Diagnostic Codes:** `GATE001` (Name-Blind Breach), `GATE002` (Pilot File `>1,500` LOC), `GATE003` (Rust LOC Ceiling Exceeded), `GATE004` (`allow(dead_code)` Forbidden), `GATE005` (Stub/TODO Marker Forbidden), `GATE006` (Retired File/Crate Resurrected), `GATE007` (Lattice Naming Taxonomy Violation), `GATE008` (Universal Rename Litmus Failure), `GATE009` (Stage-3 Fixed-Point Parity or Speed Regression), `GATE010` (Type Hygiene / `imp_type.md` Violation).

#### 1.7 Edge Cases & Safeguards
- **Rust Identifier Overlap:** Rust shares keywords `fn`, `let`, `struct`, `if`, `loop`, `return`, `ref` and primitive identifiers `u8`, `i32`, `i64`, `f64`, `usize`, `bool`, `char`, `str` in *Rust type annotations*! `seed_audit.pilot` distinguishes *Rust host types* (`x: usize`, `as u8`) from *quoted string literals & parser/codegen lookup tables* (`"usize"`, `"i64"`, `"slice"`), ensuring zero false positives on valid Rust slice indexing while catching 100% of hardcoded Pilot symbol strings!
- **Crash-Safe Temp Cleanup:** `TempDirGuard : IGuard` (`0b11`) guarantees litmus temp directories in `/tmp/pilot_litmus_<pid>` are unlinked deterministically on scope exit.

#### 1.8 Verification & 3-Gate Acceptance Criteria
1. Deliberately inject `"slice"` into a test copy of `infer.rs` $\rightarrow$ `pilot gate` catches `GATE001` at exact line/col in `< 200 ms`.
2. Deliberately modify a comment in `crates/pilot-cli/src/main.rs` $\rightarrow$ Cargo tests + `pilot gate` **succeed** (proving frozen hash lock is retired while semantic name-blind enforcement holds!).
3. Deliberately inject a duplicate struct declaration, wildcard import, or raw `__engine("map_new")` call into a test module $\rightarrow$ `pilot gate` catches `GATE010` in `< 150 ms`.
4. Confirm `scripts/gate1..4_*.py` are deleted from git tree and `pilot gate --full` passes 100% green (`0` violations, Stage-2 == Stage-3 SHA-256 identical).

---

### M2 — Unified CLI Runtime, Transitive Module Graph & `comptime` Reflection Parity [STATUS: LIBRARY ONLY]

#### 2.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Broken Span & Line Offsets via Naive String Concatenation):** [`crates/pilot-analysis/src/pipeline.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/src/pipeline.rs) (`load_program`, lines `57`–`75`) and [`crates/pilot-cli/src/main.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-cli/src/main.rs) (`eval_selfhost`, lines `10`–`14`) concatenate all imported `.pilot` files with `\n` into a single synthetic string before calling `pilot_parser::parse(&full)`. This shifts every `ByteSpan` in the entry file by thousands of bytes, corrupts error line numbers, re-parses shared dependencies repeatedly, and breaks `#pragma` / package-level visibility boundaries!
- **Root Bottleneck 2 (Hardcoded Stubs & Prefix Hacks in `pilot-cli` & `pipeline.rs`):**
  - `crates/pilot-cli/src/main.rs` lines `16`–`17` contain hardcoded fake outputs in `cmd_emit` (`"def add(a, b):\n return a + b\n"`) and `cmd_from` (`"pub fn mul(x: i64, y: i64)..."`) instead of invoking `selfhost/transpile/*.pilot`!
  - `pipeline.rs` line `61` and `main.rs` lines `24`–`25` hardcode `"Pilot/"`, `"memory"`, and `"core/"` path heuristics in Rust, and `pipeline.rs` lines `27`–`49` parse `#pragma warning disable` in Rust instead of Pilot!
- **Root Bottleneck 3 (`comptime` vs. Runtime Reflection Asymmetry & Dropped Return Types):**
  - [`crates/pilot-analysis/src/comptime.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/src/comptime.rs) (lines `46`–`60`) pushes maps into `vm.heap_maps[0..3]` without binding a root `ENV` global symbol, forcing Pilot code into $O(N)$ `__engine` node-by-node loops (`proposals.md` Proposal 2).
  - `comptime.rs` lines `69`–`72` drop `Str`, `Slice`, `Map`, `Struct`, and `AstNode` return values from `eval_ast_node`, returning `ComptimeValue::Void`!
  - `BootstrapSnapshot::global()` (lines `104`–`110`) re-parses and recompiles `stdlib/core/bootstrap.pilot` `stage_count + 1` times in a loop instead of compiling once and invoking exported functions.

#### 2.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/loader/module_graph.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Loader.ModuleGraph;` | `~360L` | Topological DAG resolver for `pkg` declarations and `use` imports (`FileId` assignment, cycle detection `MOD001`–`MOD004`, search-path table). |
| **CREATE** | `selfhost/loader/pragma_filter.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Loader.PragmaFilter;` | `~180L` | Pure-Pilot `#pragma warning/no_hardcode disable|restore` interval solver replacing `pipeline.rs:27-49`. |
| **MODIFY** | `stdlib/core/reflect.pilot` | `pub lvl(0) pkg Pilot.Core.Reflect;` | `+140L` | Add `CompileEnvView : IReference` (`0b01`), $O(1)$ `env()` accessor, target layout queries, and runtime/comptime unified symbol lookup. |
| **MODIFY** | `crates/pilot-span/src/lib.rs` | Seed span physics | `+28L` | Add `FileId(u32)` + `SourceMap` table mapping `(FileId, ByteSpan)` to exact `(file_path, line, col)` without string-concatenation span drift. |
| **MODIFY** | `crates/pilot-analysis/src/comptime.rs` | Seed comptime host | `-15L / +35L` | Bind root `ENV` handle into VM globals once; preserve `Str`/`Map`/`Slice`/`AstNode` in `ComptimeValue`; compile `bootstrap.pilot` **once** in `BootstrapSnapshot::global()`. |
| **MODIFY** | `crates/pilot-analysis/src/pipeline.rs` | Seed analysis bridge | `-45L / +20L` | Delete Rust `filter_pragma_suppressions` and string-concat `load_program`; mount per-file `AstArena` units preserving `FileId` spans. |
| **MODIFY** | `crates/pilot-cli/src/main.rs` | Seed CLI entry | `-30L / +15L` | Delete fake `cmd_emit` & `cmd_from` hardcoded strings (route directly to `selfhost/transpile/`); delete duplicate `parse(&src)` on line 92; attach unified `ENV` + `PhysicsRegistry` handler to `run_file`. |

#### 2.3 Complete Data Structures & 2-Bit Lattice Physics
- **`[Layout(c_abi)] pub struct FileUnitId : IValue`** (`0b00`): `pub raw: u32;`
- **`[Layout(c_abi)] pub struct SourceSpan : IValue`** (`0b00`, 12 bytes): `pub file_id: FileUnitId; pub start: u32; pub end: u32;`
- **`[Layout(c_abi)] pub struct ModuleNode : IValue`** (`0b00`):
  - `pub id: FileUnitId;`
  - `pub pkg_name: str;`
  - `pub file_path: str;`
  - `pub content_hash: u64;`
  - `pub clearance_lvl: i32;`
- **`pub struct ModuleGraph : IObject`** (`0b10`):
  - `pub units: List<ModuleNode>;`
  - `pub pkg_to_unit: Map<str, FileUnitId>;`
  - `pub canonical_type_owners: Map<str, FileUnitId>;` // imp_type.md Rule 11: Single source of truth tracking owning file per type name (rejects duplicate definitions across modules)
  - `pub edges: Map<u32, List<FileUnitId>>;`
  - `pub topo_order: List<FileUnitId>;`
  - `~self()` deterministic Perceus drop of graph buffers.
- **`[Layout(c_abi)] pub struct TargetDescriptor : IValue`** (`0b00`):
  - `pub arch: str; pub os: str; pub pointer_bytes: usize; pub endian: str; pub backend: str; pub simd_width_bits: usize; pub cache_line_bytes: usize; pub opt_level: u8;`
- **`pub struct CompileEnvView : IReference`** (`0b01` borrowed read-only window over active `PhysicsRegistry` + `ModuleGraph`):
  - `pub target: TargetDescriptor;`
  - `pub is_comptime: bool;`
  - `pub unit_count: usize;`
  - `pub symbol_count: usize;`
  - Methods: `pub pure fn lookup_type(name: str) -> opt<TypeLayout>;`, `pub pure fn symbols_in_pkg(pkg_name: str) -> slice<str>;`, `pub pure fn ast_root(unit: FileUnitId) -> usize;`

#### 2.4 End-to-End Algorithmic Pipeline & Complexity Bounds
1. **Declarative Package Search Table & Canonical Path Normalization:**
   - `module_graph.pilot` resolves `use Pkg.Sub.Item;` via an ordered list of `SearchRoot` entries loaded from `stdlib/core/bootstrap.pilot` and project roots (`./`, `./src/`, `./stdlib/`, `./selfhost/`), converting namespace segments case-insensitively (`Pilot.Core.Reflect` $\rightarrow$ `stdlib/core/reflect.pilot`) in $O(D)$ filesystem stat probes cached in `pkg_to_unit` ($O(1)$ amortized).
   - Normalizes both POSIX `/` and Windows `\` path separators internally to canonical `/`, losslessly handling Windows drive letters (`C:/...`) and UNC network shares (`//server/share/...`) without leaking host-specific path syntax into `FileUnitId` keys, module caches, or compiler hashes.
2. **Tarjan Strongly-Connected Components & Topological Mount ($O(V + E)$):**
   - Parses each discovered `.pilot` file **once** into its own `FileUnitId`-tagged `AstArena`.
   - Detects import cycles via Tarjan's SCC algorithm: intra-package `partial struct` mutual references within the same `pkg` merge cleanly into one `CompilationGroup`, while illegal cross-package initialization cycles emit `MOD002 (CircularPackageImport)` showing the exact cycle chain `A -> B -> C -> A`.
3. **$O(1)$ `ENV` Hydration Across Both `comptime` and Runtime:**
   - Both `ComptimeEngine::eval_ast_node` and `pilot run` construct the identical `PhysicsRegistry` snapshot and bind global slot `0` (`ENV`) with target physics (`TargetDescriptor` populated with `os`: `"linux"`, `"darwin"`, `"windows"`, `"wasi"`, `"android"`, `"ios"`, or `"none"`; `arch`: `"x86_64"`, `"aarch64"`, `"riscv64"`, `"wasm32"`; pointer size and endianness), so `Pilot.Core.Reflect.env()` resolves target physics, layouts, and registered symbols in $O(1)$ hash lookup—eliminating all $O(N)$ AST node polling loops.
4. **Real Transpile Routing for `emit` & `from`:**
   - `pilot emit <lang> <file.pilot>` parses `<file.pilot>` via `ModuleGraph` and dispatches to `selfhost/transpile/emit_{c,cpp,csharp,rust,python,js,lua}.pilot`.
   - `pilot from <lang> <foreign_file>` dispatches to `selfhost/transpile/ingest.pilot` and formats output via `selfhost/fmt/fmt.pilot`.

#### 2.5 Diagnostic & Exit Contracts
- New module diagnostics registered in `selfhost/diagnostics/errors.pilot`:
  - `MOD001`: Unresolved package or module path in `use` declaration (lists searched roots + closest spelling match).
  - `MOD002`: Illegal cross-package circular dependency cycle.
  - `MOD003`: Visibility violation (`priv`/`file`/`pkg`/`lvl(N)` clearance ring insufficient for importer).
  - `MOD004`: Duplicate `pkg` file claiming conflicting `lvl` clearance ring.

#### 2.6 Edge Cases & Verification Criteria
- **Exact Line/Col Preservation Test:** Import 5 stdlib modules at the top of `app.pilot` and trigger a type error on line `12`, column `9` of `app.pilot` $\rightarrow$ diagnostic reports `app.pilot:12:9` (not line `2,847` of a concatenated string!).
- **`comptime` == Runtime Parity Test:** Execute identical reflection Function (`env().lookup_type(...)`, `Map<str, i64>` indexing, string slicing) inside `comptime { ... }` AND via `pilot run` $\rightarrow$ bit-identical output.
- **Multiplatform Path & Line-Ending Invariance Test:** Parse and index modules with Windows CRLF line endings and backslash path strings (`.\src\sub\mod.pilot`) $\rightarrow$ assert `SourceMap` line/column coordinates and `ModuleGraph` node lookups match POSIX LF inputs bit-for-bit without span drift.
- **Zero Fake Transpile Stubs Test:** Run `pilot emit python` and `pilot emit rust` on a randomized 50-line struct+loop `.pilot` program $\rightarrow$ verifies genuine transpiled Python/Rust of the input file (confirming `def add` / `pub fn mul` static mocks in `main.rs` are gone forever).

---

### M3 — First-Class Declarative CLI Command Registry (`selfhost/cli/`) [STATUS: PARTLY INTEGRATED]

#### 3.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Hardcoded Numeric Dispatch in `selfhost/cli/main.pilot`):** [`selfhost/cli/main.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/cli/main.pilot) (lines `34`–`51`) maps command names via an `if/else` chain returning magic integers `1..16` and manually concatenates a static help banner string (lines `10`–`30`), violating `AGENTS.md` §1 (*Scalable Designing*).
- **Root Bottleneck 2 (Split-Brain CLI Logic in Rust `crates/pilot-cli/src/main.rs`):** `crates/pilot-cli/src/main.rs` still implements `cmd_deps`, `cmd_map`, `cmd_pkg`, and `main()` `match` arms in Rust, passing escaped source strings into `eval_selfhost(...)` and scraping `@@WRITE:path@@\n` stdout markers (lines `61`–`63`) instead of letting Pilot own argument parsing, dispatch, and file I/O directly!

#### 3.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/cli/spec.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Cli.Spec;` | `~210L` | Declarative `CommandSpec`, `FlagSpec`, `ArgSpec`, `ValueKind`, `[Command]` attribute weaver, and `ICliHandler` protocol. |
| **CREATE** | `selfhost/cli/parser.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Cli.Parser;` | `~320L` | Zero-copy POSIX/GNU argument parser (`--flag=val`, `--flag val`, `-abc` short-flag clustering, `--` passthrough separator, repeatable flags, env-var fallbacks, Damerau-Levenshtein typo suggestions). |
| **CREATE** | `selfhost/cli/help_and_comp.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Cli.Help;` | `~290L` | Auto-generated aligned ANSI/plain `--help` renderer, man-page/markdown generator, bash/zsh/fish/powershell completion script emitters, and JSON/MCP tool schema exporter. |
| **MODIFY** | `selfhost/cli/main.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Cli.Main;` | `~260L` | Replace `1..16` `if/else` router and static banner with `CommandRegistry` registering all built-in subcommands (`run`, `build`, `check`, `fmt`, `test`, `gate`, `repl`, `pkg`, `venv`, `debug`, `lsp`, `perf`, `compile`, `emit`, `from`, `tree`, `deps`, `map`, `ide-sync`, `completions`). |
| **MODIFY** | `selfhost/cli/pkg_cmd.pilot`, `deps_cmd.pilot`, `map_cmd.pilot` | `Pilot.SelfHost.Cli.*` | `~60L` delta | Implement `ICliHandler` directly and perform real file reads/writes via `stdlib/io/fs.pilot` (deleting all `@@WRITE:` stdout string-scraping!). |
| **MODIFY** | `crates/pilot-cli/src/main.rs` | Minimal seed launcher | Shrink `100L` $\rightarrow$ `~30L` | Delete `cmd_emit`, `cmd_from`, `cmd_deps`, `cmd_map`, `cmd_pkg`, `print_tree`, `resolve_dep`, and `get_deps` from Rust! `main.rs` solely boots `PilVirtualMachine` (or cached `main.pilc`), passes `env::args()`, and exits with Pilot's returned status code. |

#### 3.3 Complete Data Structures & 2-Bit Lattice Physics
- **`FlagValueKind` (`enum : u8`, `0b00 : IValue`)**: `::Bool`, `::Int`, `::Float`, `::Str`, `::Path`, `::EnumChoice`, `::StringList`.
- **`FlagOptions` (`bitflags : u16`, `0b00 : IValue`)**: `::None = 0`, `::Repeatable = 1 << 0`, `::Required = 1 << 1`, `::Hidden = 1 << 2`. // imp_type.md Rule 5: Zero boolean blindness
- **`[Layout(c_abi)] pub struct FlagSpec : IValue`** (`0b00`):
  - `pub long_name: str;` (e.g. `"output"`)
  - `pub short_char: char;` (e.g. `'o'`, or `'\0'` if none)
  - `pub kind: FlagValueKind;`
  - `pub default_val: str;`
  - `pub env_fallback: str;` (e.g. `"PILOT_COLOR"`)
  - `pub choices: slice<str>;`
  - `pub options: FlagOptions;`
  - `pub summary: str;`
- **`[Layout(c_abi)] pub struct PositionalSpec : IValue`** (`0b00`):
  - `pub name: str; pub kind: FlagValueKind; pub required: bool; pub variadic: bool; pub summary: str;`
- **`[Layout(c_abi)] pub struct CommandSpec : IValue`** (`0b00`):
  - `pub name: str;`
  - `pub aliases: slice<str>;`
  - `pub category: str;` (`"Run & Build"`, `"Analysis & Quality"`, `"Packages & Deps"`, `"IDE, Debug & AI"`)
  - `pub summary: str;`
  - `pub examples: slice<str>;`
  - `pub flags: slice<FlagSpec>;`
  - `pub positionals: slice<PositionalSpec>;`
  - `pub subcommands: slice<CommandSpec>;` (recursive tree supporting nested `pilot pkg add`, `pilot deps tree`, etc.!)
- **`pub struct ParsedArgs : IObject`** (`0b10`):
  - `pub command_path: List<str>;`
  - `pub flag_values: Map<str, List<str>>;`
  - `pub positionals: List<str>;`
  - `pub passthrough_args: List<str>;` (everything after `--`, forwarded untouched to user scripts in `pilot run app.pilot -- --port 8080`!)
  - Typed helpers: `pub pure fn has(flag: str) -> bool`, `pub pure fn str_or(flag: str, fallback: str) -> str`, `pub pure fn int_or(flag: str, fallback: i64) -> i64 ! CliErr`.
- **`pub struct CommandRegistry : IObject`** (`0b10`):
  - `pub commands_by_name: Map<str, usize>;`
  - `pub alias_to_primary: Map<str, str>;`
  - `pub specs: List<CommandSpec>;`
  - `pub handlers: List<fn<ref<ParsedArgs>, i32>>;`
  - `pub pure fn count() -> usize => .specs.len();` (100% scalable—adding a command in M7 `inspect` or M45 `mcp` is a single `registry.register(...)` entry!).

#### 3.4 Algorithmic Pipeline & Single-Source Generation
1. **Direct Script Detection & Shebang Support:**
   - If `argv[1]` ends in `.pilot` or `.pilc` (or starts with `./` / `/` pointing to a file whose line 1 begins with `#!/usr/bin/env pilot`), the lexer skips line 1 `#!...` cleanly and `CommandRegistry` routes implicitly to `run` with `argv[2..]` bound as user script `passthrough_args`.
2. **Damerau-Levenshtein Typo Suggestion ($O(|A| \cdot |B|)$ bounded on `<32` char tokens):**
   - Unknown subcommand `pilot biuld` or flag `--josn` computes edit distance $\le 2$ across `CommandRegistry` keys and emits `CLI001: Unknown command 'biuld'. Did you mean 'build'?` with exit code `2`.
3. **Quad-Output Schema Synthesis from One `CommandRegistry`:**
   - `render_help(spec)` $\rightarrow$ grouped, terminal-width-aware `--help` output with automatic cross-platform ANSI/VT100 detection (supporting Windows Terminal and enabling `ENABLE_VIRTUAL_TERMINAL_PROCESSING` on legacy Windows Console).
   - `emit_shell_completions(shell)` $\rightarrow$ complete subcommand + flag + file/enum tab-completion scripts for `bash`, `zsh`, `fish`, and `pwsh` (PowerShell on Windows/macOS/Linux).
   - `export_cli_json_schema()` $\rightarrow$ JSON Schema of every `pilot` tool & flag, ready for VS Code tasks (`ide-sync`) and MCP tool registration (`M45`).
4. **Multiplatform Shell Argument Escaping:**
   - Command parser and pass-through argument serializer handle both POSIX `/bin/sh` shell tokenization and Windows `cmd.exe` / `PowerShell` escaping rules (`CommandLineToArgvW` quote-escaping and `^` caret rules).

#### 3.5 Diagnostic & Exit Contracts
- `CLI001` (Unknown Subcommand + suggestion), `CLI002` (Unknown Flag + suggestion), `CLI003` (Missing Required Argument/Flag), `CLI004` (Invalid Flag Type/Enum Choice), `CLI005` (Duplicate Non-Repeatable Flag), `CLI006` (Conflicting Mutually Exclusive Flags).

#### 3.6 Verification & 3-Gate Acceptance Criteria
1. Assert `crates/pilot-cli/src/main.rs` is `<= 35` LOC and contains **zero** subcommand string names.
2. Verify `pilot app.pilot -- --custom-flag 42` passes `--custom-flag 42` to `app.pilot` without `pilot` rejecting `--custom-flag`.
3. Verify `pilot completions zsh`, `pilot completions bash`, `pilot completions fish`, and `pilot --schema-json` dynamically include a newly registered test command with zero edits to help/completion generators.

---

### M4 — Dual Readable/Symbolic Syntax Pairings (`stdlib/core/keywords.pilot` & `syntax/`) [STATUS: PARTLY INTEGRATED]

#### 4.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Missing Readable Prefix Projections for the 6 Call-Quoting Modes):** Currently [`stdlib/core/syntax/call_quote.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/syntax/call_quote.pilot) defines carrier structs (`CallExpr`, `Signal`, `Dual`, `EffectPlan`), while the parser only recognizes compact symbolic zero-whitespace call suffixes `fn!(...)`, `fn!!(...)`, `fn!&(...)`, `fn!^(...)`, `fn!@(...)`, `fn!?(...)`. There are **no** readable keyword equivalents (`call`, `splice`, `frame`, `sig`, `grad`, `plan`), forcing newcomers to memorize 6 punctuation combinations!
- **Root Bottleneck 2 (Inconsistent Short Type Aliases & Unpack Call Forms):**
  - `option<T>` and `result<T, E>` lack concise 3-letter `lowercase` core aliases `opt<T>` and `res<T, E>` matching `str`, `ptr`, `ref`, `iso`, `ast`, `pkg`, `ext`.
  - `Signal<T>`, `Dual<T, G>`, and `EffectPlan<T>` have PascalCase carrier structs (`0b10`/`0b00`), while `frame<TSig>` and `ast<TRet>` are lowercase; readable lowercase type projections `sig<T>`, `grad<T, G>`, and `plan<T>` unify type-position readability.
  - Splat/flatten operators `*expr`, `**expr`, `*..expr` have primitive carrier types (`splat<T>`, `pairs<K, V>`, `expand<T>`), yet lack readable prefix expression forms `splat(expr)`, `pairs(expr)`, `flat(expr)`.
- **Root Bottleneck 3 (Formatter Style Lock-In):** [`selfhost/fmt/fmt.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/fmt/fmt.pilot) preserves whatever surface token was written without offering team/beginner formatting modes (`--style=readable` vs `--style=compact` vs `--style=preserve`).

#### 4.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **MODIFY** | `stdlib/core/keywords.pilot` | `pub pkg Pilot.Core.Keywords;` | `+95L` | Register `[KeywordDef]` metadata (`doc`, `tm_scope`, `badge`, `lsp_kind`, paired symbol cross-link) for `opt`, `res`, `call`, `splice`, `frame`, `sig`, `grad`, `plan`, `splat`, `pairs`, `flat`. |
| **MODIFY** | `stdlib/core/syntax/call_quote.pilot` | `pub lvl(0) pkg Pilot.Core.Syntax.CallQuote;` | `+110L` | Define `pub syntax` rewrite rules lowering `call f(args)`, `splice f(args)`, `frame f(args)`, `sig f(args)`, `grad f(args)`, and `plan f(args)` to the exact same receptor AST nodes as `f!(args)`..`f!?(args)`. |
| **MODIFY** | `stdlib/core/syntax/expr_sugar.pilot` | `pub lvl(0) pkg Pilot.Core.Syntax.ExprSugar;` | `+65L` | Define `pub syntax` rules lowering `splat(x)` $\leftrightarrow$ `*x`, `pairs(x)` $\leftrightarrow$ `**x`, and `flat(x)` $\leftrightarrow$ `*..x`. |
| **MODIFY** | `stdlib/core/syntax/type_construct.pilot` | `pub lvl(0) pkg Pilot.Core.Syntax.TypeConstruct;` | `+45L` | Register canonical lowercase type projections `opt<T>`, `res<T, E>`, `sig<T>`, `grad<T, G>`, `plan<T>` alongside `T?`, `T ! E`, `T^`. |
| **MODIFY** | `selfhost/parser.pilot` & `crates/pilot-parser/src/lib.rs` | Dynamic `ParserTables` | `+35L` | Ensure `ParserTables` dispatches prefix projection macros (`<kw> <callee>(<args>)`) cleanly without reserving those words as field/variable identifiers (`let plan = 1; obj.call` remain 100% valid!). |
| **MODIFY** | `selfhost/fmt/fmt.pilot` | `pub pkg Pilot.SelfHost.Fmt;` | `+130L` | Add `FmtStyle` (`::Preserve`, `::Readable`, `::Compact`) losslessly rewriting paired surface CST nodes while keeping all comments intact. |
| **MODIFY** | `selfhost/ide/vscode_gen.pilot` | `pub pkg Pilot.SelfHost.Ide.VscodeGen;` | `+30L` | Auto-emit hover paired-syntax equivalences and TextMate scopes from `keywords.pilot` (zero hardcoded keyword strings!). |

#### 4.3 Complete Dual-Syntax Equivalence Table & Contextual Disambiguation Rules
Every row below is guaranteed **100% AST-, Type-, Lattice-, and Codegen-Identical**:

| Construct Category | Compact Symbolic Form | Readable `lowercase` Form | Contextual Disambiguation Rule (Zero Identifier Theft!) |
| :--- | :--- | :--- | :--- |
| **Optional Type** | `T?` | `opt<T>` / `option<T>` | Type position only (`x: opt<i64>`). |
| **Fallible Result Type** | `T ! E` | `res<T, E>` / `result<T, E>` | Type position only (`-> res<Data, IoErr>`). |
| **Contiguous / Fixed / Stride / SIMD** | `T[]`, `T[N]`, `T[..]`, `T@N` | `slice<T>`, `array<T, N>`, `stride<T>`, `simd<T, N>` | Type position via `syntax type_construct`. |
| **Shallow Call Quote** | `f!(a, b)` | `call f(a, b)` | Triggered only when `call` is immediately followed by `<callee_expr>(` (never steals `let call = ...` or `x.call()`). |
| **Deep AST Splice** | `f!!(~a, b)` | `splice f(~a, b)` | Triggered only when `splice` precedes `<callee_expr>(`. |
| **Zero-Marshal ABI Frame** | `f!&(a, b)` | `frame f(a, b)` | In expression position before `<callee>(` $\rightarrow$ ABI frame; in type position `frame<Sig>` $\rightarrow$ frame type. |
| **Reactive Signal** | `f!^(a, b)` / `T^` | `sig f(a, b)` / `sig<T>` | Expression prefix `sig <callee>(` $\rightarrow$ `Signal<TRet>`; type `sig<T>` $\rightarrow$ `Signal<T>`. |
| **Symbolic Derivative** | `f!@(a, b)` | `grad f(a, b)` / `grad<T, G>` | Expression prefix `grad <callee>(` $\rightarrow$ `Dual<TRet, TGrad>`; type `grad<T, G>` $\rightarrow$ `Dual<T, G>`. |
| **Transactional Dry-Run** | `f!?(a, b)` | `plan f(a, b)` / `plan<T>` | Expression prefix `plan <callee>(` $\rightarrow$ `EffectPlan<TRet>`; type `plan<T>` $\rightarrow$ `EffectPlan<T>`. |
| **Sequence / Pair / Deep Unpack** | `*x`, `**x`, `*..x` | `splat(x)`, `pairs(x)`, `flat(x)` | Prefix call inside argument/collection/initializer lists; `ISplatOperator` / `IPairsOperator` / `IFlattenOperator`. |

#### 4.4 Bidirectional Formatter Algorithm (`selfhost/fmt/fmt.pilot`)
- **`--style=preserve` (Default):** Formats whitespace, indentation, and trailing commas via Wadler-Lindig layout algebra while leaving the author's choice of `T?` vs `opt<T>` or `sig compute(x)` vs `compute!^(x)` untouched.
- **`--style=readable`:** Rewrites compact punctuation constructs (`T?` $\rightarrow$ `opt<T>`, `T ! E` $\rightarrow$ `res<T, E>`, `f!^(x)` $\rightarrow$ `sig f(x)`, `f!@(x)` $\rightarrow$ `grad f(x)`, `f!?(x)` $\rightarrow$ `plan f(x)`, `*..xs` $\rightarrow$ `flat(xs)`) while respecting operator precedence parenthesization (`(A | B)?` $\rightarrow$ `opt<A | B>`).
- **`--style=compact`:** Rewrites canonical readable forms into concise symbolic forms (`opt<T>` $\rightarrow$ `T?`, `sig f(x)` $\rightarrow$ `f!^(x)`, etc.), automatically inserting grouping parentheses when binding power requires (`opt<fn<i64, bool>>` $\rightarrow$ `((i64) => bool)?`).

#### 4.5 Edge Cases & Verification Criteria
1. **Identifier Non-Collision Proof:** Compile a program with local variables named `let opt = 1; var call = 2; let sig = 3; let grad = 4; let plan = 5; let flat = 6;` alongside `let s = sig compute(opt);` $\rightarrow$ zero parse ambiguity!
2. **Round-Trip Bytecode Identity:** Take a 500-line module using all 15 constructs: compile original $\rightarrow$ `.pilc` #1; run `pilot fmt --style=readable` and compile $\rightarrow$ `.pilc` #2; run `pilot fmt --style=compact` and compile $\rightarrow$ `.pilc` #3. Assert `SHA256(#1) == SHA256(#2) == SHA256(#3)`!
3. **Universal Rename Litmus:** Rename `opt`/`option` and `sig`/`Signal` in `stdlib/core/` $\rightarrow$ zero Rust parser/analysis changes needed.

---

### M5 — Shared Incremental Compiler Query Database (`PilotDb`) [STATUS: SIMULATED]

#### 5.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`ide_db.pilot` Is Currently Only a 139-Line String Scanner):** [`selfhost/lsp/ide_db.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/lsp/ide_db.pilot) only implements `extract_item_tree_signature`, `apply_doc_span_edit`, and `should_early_cutoff` on raw strings. It has **no** persistent document table, **no** dependency graph tracking, **no** memoized query slots, and **no** sharing with `pilot check`, `pilot build`, or `pilot fmt`.
- **Root Bottleneck 2 (Cold Re-Parsing Across Every Command & Keystroke):** Every `pilot check`, `pilot build`, `pilot map`, and LSP hover/completion re-lexes, re-parses, and re-analyzes files from scratch instead of reusing unchanged upstream syntax/type tables.

#### 5.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/db/revision.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Db.Revision;` | `~210L` | Monotonic `RevisionId`, `Durability` tiers (`Low`/`Medium`/`High`/`Core`), 128-bit BLAKE3 fingerprint value types, and `QueryKey` encoding. |
| **CREATE** | `selfhost/db/engine.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Db.Engine;` | `~440L` | Demand-driven Salsa-style red-green memoization runtime: dependency stack recording, `verified_at` fast-path skip, `changed_at` propagation, and signature-hash early cutoff. |
| **CREATE** | `selfhost/db/queries.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Db.Queries;` | `~410L` | The 8 canonical compiler queries (`SourceText` $\rightarrow$ `LexTokens` $\rightarrow$ `LosslessCst` $\rightarrow$ `ItemSignature` $\rightarrow$ `ExpandedAst` $\rightarrow$ `ImportScope` $\rightarrow$ `DefTypesAndLattice` $\rightarrow$ `FileDiagnostics`). |
| **CREATE** | `selfhost/db/cache_io.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Db.CacheIo;` | `~240L` | Atomic `.pilc-cache/pilotdb.v1` on-disk persistence (header magic + compiler build hash + content-addressed query blobs) warming CLI invocations in `< 5 ms`. |
| **MODIFY** | `selfhost/lsp/ide_db.pilot` & `selfhost/lsp/handlers.pilot` | `Pilot.SelfHost.Lsp.*` | `-90L / +75L` | Delegate LSP document store, `didChange` incremental byte edits (`selfhost/cst/incremental.pilot`), and semantic queries directly to `PilotDb`. |
| **MODIFY** | `selfhost/cli/main.pilot` | `Pilot.SelfHost.Cli.Main` | `+40L` | Route `check`, `build`, `compile`, `fmt`, `map`, `deps`, and `lsp` through a shared `PilotDb` instance. |

#### 5.3 Complete Data Structures & 2-Bit Lattice Physics
- **`Durability` (`enum : u8`, `0b00 : IValue`)**:
  - `::Volatile` (`0` — unsaved open IDE buffer edited every keystroke)
  - `::Workspace` (`1` — local project `.pilot` file)
  - `::ExternalPkg` (`2` — locked dependency in `pilot.lock` / `pkg.lock`)
  - `::CoreGenesis` (`3` — `stdlib/core/` bootstrap files; tracked via single global `durability_last_changed[3]` revision so editing a user buffer **never** even walks dependencies inside `stdlib/core/`!).
- **`[Layout(c_abi)] pub struct Fingerprint128 : IValue`** (`0b00`, 16 bytes): `pub lo: u64; pub hi: u64;` (truncated BLAKE3 from `stdlib/crypto/hash.pilot`).
- **`[Layout(c_abi)] pub struct QueryKey : IValue`** (`0b00`, 12 bytes):
  - `pub stage_id: u16;` (registered dynamically in `QueryStageRegistry`, never hardcoded enum limit)
  - `pub flags: u16;`
  - `pub entity_id: u64;` (`FileUnitId` or interned `DefId`).
- **`[Layout(c_abi)] pub struct MemoHeader : IValue`** (`0b00`):
  - `pub verified_at: u64;`
  - `pub changed_at: u64;`
  - `pub output_fingerprint: Fingerprint128;`
  - `pub min_durability: Durability;`
- **`pub struct QueryNode : IObject`** (`0b10`):
  - `pub header: MemoHeader;`
  - `pub deps: List<QueryKey>;`
  - `pub payload_slot: usize;` (typed arena handle into stage storage table).
- **`pub struct PilotDb : IObject`** (`0b10`):
  - `pub current_rev: u64;`
  - `pub durability_watermarks: array<u64, 4>;`
  - `pub memos: Map<QueryKey, QueryNode>;`
  - `pub active_query_stack: List<QueryKey>;` (detects accidental compiler query cycles via `DB001` instead of stack-overflowing!).

#### 5.4 Red-Green Incremental Validation & Early-Cutoff Algorithm
1. **Durability O(1) Short-Circuit:** When querying `k`, if `memo.header.verified_at == current_rev`, return cached payload in $O(1)$. Furthermore, if `durability_watermarks[memo.header.min_durability] <= memo.header.verified_at`, mark `verified_at = current_rev` and return in **$O(1)$ with zero dependency traversal**! (All `stdlib/core/` queries hit this 1-instruction fast path!).
2. **Deep Dependency Check (`maybe_changed_after`):** Otherwise, iterate `dep` in `memo.deps` in execution order:
   - Recursively validate `dep`. If `dep.changed_at <= memo.verified_at` for all `dep`, update `memo.verified_at = current_rev` (**Green!** Zero recomputation of `k`!).
   - As soon as one `dep` has `dep.changed_at > memo.verified_at` (**Red!**), re-execute `k`'s compute function while recording fresh `deps` onto `active_query_stack`.
3. **BLAKE3 Early Cutoff Barrier:** Compute `new_fp = blake3_128(new_output)`.
   - If `new_fp == memo.header.output_fingerprint` (e.g., user edited a comment, reformatted whitespace, or changed a local variable inside a function body so `ItemSignature(file)` is unchanged!), keep `memo.header.changed_at` at its **old** revision and only bump `verified_at = current_rev`!
   - **Result:** Editing inside a function body in `module_a.pilot` re-typechecks *only* that single function body—**zero** downstream importers (`module_b`..`module_z`) re-typecheck!
4. **Cross-Platform Virtual URI Canonicalization:**
   - `FileUnitId` query resolution maps POSIX `file:///path`, Windows `file:///C:/path`, and UNC `file://server/share/path` into a canonical forward-slash virtual index.
   - Preserves CRLF and LF line endings losslessly in Green trivia nodes without altering AST spans or token boundaries.

#### 5.5 Complexity & Latency Guarantees
- Typing inside a function body in a 100-module workspace: $O(|\text{single file CST diff}| + |\text{single fn body}|)$, completing `FileDiagnostics` in **`< 8 ms`**.
- Warm CLI `pilot check` with zero changed files using `.pilc-cache/pilotdb.v1`: **`< 15 ms`** (cache format is endian-neutral and cross-platform portable).
- Corrupt or compiler-version-mismatched `.pilc-cache/pilotdb.v1`: silently discarded and rebuilt cleanly (`DB002` info log under `--verbose`, never panics).

#### 5.6 Verification & 3-Gate Acceptance Criteria
1. **Early-Cutoff Counter Proof:** Build a 25-module chain `M0 <- M1 <- ... <- M24`. Mutate a local statement inside a function body in `M0`. Assert `PilotDb` query execution telemetry shows `DefTypesAndLattice` ran `1` time (`M0`) and **`0` times** for `M1..M24`.
2. **Signature-Change Propagation Proof:** Change a `pub fn` return type in `M0`. Assert `PilotDb` invalidates direct callers and surfaces `TYPE001` in `M1` immediately.
3. **Identical Cold vs. Incremental Oracle:** Run 200 randomized sequential file edits (insertions, deletions, macro changes, `use` additions): after every edit, assert `PilotDb` incremental diagnostics & layouts match a fresh cold `PilotDb` build 100% bit-for-bit.

---

### M6 — First-Class Typed SSA Mid-Level IR (`PilIR` in `selfhost/ir/`) [STATUS: SIMULATED]

#### 6.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Direct AST-to-Backend Fan-Out Duplication):** Currently, [`selfhost/analyzer.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/analyzer.pilot) feeds ASTs directly into 10+ separate emitters (`selfhost/vm/compiler.pilot`, `selfhost/codegen/{c_emitter,llvm_emitter,wasm_emitter,asm_emitter,gpu_c_dialects,gpu_llvm,gpu_shaders}.pilot`, and `selfhost/transpile/emit_*.pilot`). Meanwhile [`selfhost/perceus/{arc,escape,realtime}.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/perceus) analyzes AST trees separately, and [`selfhost/transpile/ir.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/transpile/ir.pilot) maintains a disconnected, untyped mini-IR (`10,418` bytes) used only by source transpilers.
- **Root Bottleneck 2 (No Pluggable Optimization or Verification Boundary):** Without a single typed SSA IR carrying `LayoutId` and 2-bit `LatticeBits` on every value, optimizations (constant folding, dead-block removal, Perceus `drop_reuse`, loop fusion, SoA field remapping) would have to be reimplemented N times per backend.

#### 6.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/ir/model.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Ir.Model;` | `~380L` | Core `IrModule`, `IrFunction`, `IrBlock` (with Cranelift/MLIR-style block parameters instead of fragile $\phi$-nodes), `IrValue`, `IrInst`, and name-blind `IrTypePhysics`. |
| **CREATE** | `selfhost/ir/lower_ast.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Ir.LowerAst;` | `~460L` | Lowers expanded, typechecked AST + woven `[Attribute]` pipelines + 4-tier `MethodTable` calls + 3-tier `for` loops into canonical `PilIR` (cached as a `PilotDb` query stage!). |
| **CREATE** | `selfhost/ir/verifier.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Ir.Verifier;` | `~290L` | Static SSA dominance checker, block-argument arity/layout matcher, terminator completeness checker, and 2-bit lattice invariants verifier (`IR001`–`IR008`). |
| **CREATE** | `selfhost/ir/pass_manager.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Ir.PassManager;` | `~310L` | Open table-driven `IrPassRegistry`, `IPilIrPass` interface, fixed-point pass iteration, pass timing telemetry, and user `comptime` `[IrPass]` hook registration. |
| **CREATE** | `selfhost/ir/passes_core.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Ir.PassesCore;` | `~390L` | Built-in canonical passes: `CfgSimplify`, `ConstPropAndFold`, `DeadCodeElim`, `EscapeCheckPass` (wrapping `perceus/escape.pilot`), and `PerceusInsertPass` (wrapping `perceus/arc.pilot`). |
| **CREATE** | `selfhost/ir/printer.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Ir.Printer;` | `~210L` | Deterministic human-readable textual `PilIR` serializer + round-trip parser for unit testing IR passes in isolation. |
| **MODIFY** | `selfhost/codegen/*.pilot`, `selfhost/vm/compiler.pilot`, `selfhost/transpile/emit_*.pilot` | All target emitters | `~180L` net | Consume verified `IrModule` from `PilotDb` instead of raw AST lowering; preserve Stage-3 fixed-point parity. |
| **DELETE** | `selfhost/transpile/ir.pilot` | Superseded mini-IR | `-100%` | Delete disconnected transpiler-only IR (`10,418` bytes); all 7 transpilers now read unified `PilIR`. |

#### 6.3 Complete Data Structures, Instruction Set & 2-Bit Lattice Physics
- **Zero Type-Name Hardcoding Law:** `PilIR` never stores strings like `"i64"` or `"slice"`. Every `IrValueId(u32)` binds to `IrTypePhysics : IValue` (`0b00`, 12 bytes):
  - `pub layout_id: u32;`
  - `pub reg_class: u8;` (`0 = gpr_signed`, `1 = gpr_unsigned`, `2 = simd_float`, `3 = simd_vector`, `4 = raw_ptr`, `5 = aggregate`, `6 = unit`, `7 = noreturn` — derived strictly from `[Bits]` & `[Layout]`)
  - `pub bit_width: u16;`
  - `pub align_pow2: u8;`
  - `pub lattice_bits: u8;` (`0b00` `IValue`, `0b01` `IReference`, `0b10` `IObject`, `0b11` `IGuard`).
- **Core Arena & Function Schemas (`selfhost/ir/model.pilot`):**
  - **`CmpPred` (`enum : u8`, `0b00 : IValue`)**: `::Eq`, `::Ne`, `::Lt`, `::Le`, `::Gt`, `::Ge` (signed/unsigned/float ordered/unordered selected strictly via `IrTypePhysics.reg_class`, never type names!).
  - **`CastKind` (`enum : u8`, `0b00 : IValue`)**: `::ZeroExtend`, `::SignExtend`, `::Truncate`, `::FloatPromote`, `::FloatDemote`, `::IntToFloat`, `::FloatToInt`, `::Bitcast`, `::ViewReinterpret`.
  - **`[Layout(c_abi)] pub struct IrInst : IValue`** (`0b00`, 32 bytes):
    - `pub result: u32;` (`IrValueId`, or `u32.max` for void ops)
    - `pub opcode: u16;`
    - `pub subop: u16;` (`CmpPred`, `CastKind`, field index `u16`, or `EngineOpId`)
    - `pub physics: IrTypePhysics;`
    - `pub args_offset: u32; pub args_len: u16;` (slice window into `IrFunction.operand_pool` avoiding per-instruction heap `List` allocations!)
    - `pub span: SourceSpan;` (exact origin span for DWARF/DAP debugging & Perceus error reporting).
  - **`pub struct IrBlock : IObject`** (`0b10`):
    - `pub id: u32; pub label_sym: u32; pub params: List<(u32, IrTypePhysics)>; pub insts: List<IrInst>; pub preds: List<u32>; pub idom: u32;`
  - **`pub struct IrFunction : IObject`** (`0b10`):
    - `pub fn_id: u32; pub sym_id: u32; pub is_pure: bool; pub is_noalloc: bool; pub wcet_budget_ns: u64; pub clearance_lvl: i32;`
    - `pub sig_params: List<IrTypePhysics>; pub ret_physics: IrTypePhysics;`
    - `pub blocks: List<IrBlock>; pub operand_pool: List<u32>; pub value_physics: List<IrTypePhysics>;`
  - **`pub struct IrModule : IObject`** (`0b10`):
    - `pub unit_id: FileUnitId; pub target: TargetDescriptor; pub layouts: List<TypeLayout>; pub globals: List<IrGlobal>; pub functions: List<IrFunction>;`
  - **Target-Aware ABI Lowering & Multiplatform Representation:**
    - Every `IrModule` is parameterized by its `TargetDescriptor`. Calling conventions, register partitioning, aggregate passing rules, and stack alignments are computed relative to the target platform (SysV AMD64, Win64 with 32-byte caller shadow space, AAPCS64 Darwin/Linux with HFA packing, RISC-V LP64D, WASM) rather than the host system. This ensures cross-compilation from any host OS produces bit-for-bit identical target object representations.
- **Complete `IrOp` Instruction Taxonomy (34 Orthogonal Opcodes across 6 Families):**
  1. **Constants & Aggregates:** `ConstBits`, `ConstFloat`, `ConstBytes`, `Undef`, `ConstructAggregate`, `ExtractField`, `InsertField`.
  2. **Register Arithmetic & Bitwise:** `Add`, `Sub`, `Mul`, `Div`, `Rem`, `BitAnd`, `BitOr`, `BitXor`, `Shl`, `Shr`, `Cmp`, `Cast`, `SimdLaneOp`.
  3. **Memory, Views & Arenas:** `StackSlot`, `ArenaAlloc`, `Load`, `Store`, `PtrOffset`, `MakeView` (`{ ptr, len }` / `{ ptr, len, stride }`).
  4. **Explicit 2-Bit Lattice & Perceus Lifecycle Ops:** `PerceusDup`, `PerceusDrop`, `PerceusDropReuse` *(yields recyclable `ReuseToken`)*, `ConstructReuse`, `IsoFreeze`, `GuardEnter`, `GuardExitLifo`.
  5. **Calls & 6 Homoiconic Projections:** `CallDirect`, `CallIndirect`, `CallIntrinsic`, `PackAbiFrame` (`frame`/`!&`), `InitSignal` (`sig`/`!^`), `DiffAdjoint` (`grad`/`!@`), `BeginEffectPlan` (`plan`/`!?`).
  6. **Block Terminators:** `Jump`, `CondBr`, `SwitchTable`, `Return`, `Unreachable`.

#### 6.4 Public Signatures, Pass Protocol & Control-Flow Lowering Rules
- **`pub interface IPilIrPass : IAnyValue`**:
  - `pub pure fn pass_name() -> str;`
  - `pub pure fn mutates_cfg() -> bool;`
  - `pub fn run_on_fn(func: var ref<IrFunction>, ctx: ref<IrPassContext>) -> res<bool, List<Diagnostic>>;`
- **Public Module Functions:**
  - `pub fn lower_unit_to_ir(db: var ref<PilotDb>, unit: FileUnitId) -> res<IrModule, List<Diagnostic>>;`
  - `pub pure fn verify_ir_function(func: ref<IrFunction>, layouts: slice<TypeLayout>) -> List<Diagnostic>;`
  - `pub fn run_ir_pipeline(mod_ir: var ref<IrModule>, registry: ref<IrPassRegistry>, opt_level: u8) -> res<void, List<Diagnostic>>;`
  - `pub pure fn print_ir_module(mod_ir: ref<IrModule>) -> str;`
  - `pub fn parse_ir_module(text: str) -> res<IrModule, List<Diagnostic>>;`
- **Tricky AST Lowering Edge Cases Handled Explicitly in `lower_ast.pilot`:**
  1. **Short-Circuit `&&` / `||` & Null-Coalescing `??`:** Lowered into explicit `CondBr` diamond blocks passing result via a 1-parameter join `IrBlock`—never evaluating RHS eagerly.
  2. **`defer` + `0b11` `IGuard` Unwinding Across Early `return` / `break` / Postfix `?`:** Maintain a lexical scope-cleanup stack during lowering so every early exit (`return`, `break`, `continue`, or `ITryOperator` `?` failure branch) emits exact reverse-declaration `GuardExitLifo`, `defer` bodies, and `PerceusDrop` ops before branching!
  3. **3-Tier Dynamic `for` Loops:** Lower `FixedScalar`, `MutableBounds` (`continue a..b;`), and `WorklistStack` (`continue a..b, ...;` backed by `RangeWindow<TIter>`) into canonical loop-header block parameters so downstream SIMD/loop passes analyze a uniform CFG.

#### 6.5 Diagnostic & Verifier Codes (`IR001`–`IR008`)
- `IR001` (SSA Use Not Dominated by Def — Cooper-Harvey-Kennedy iterative dominator check in $O(V + E)$), `IR002` (Branch Argument Count or `IrTypePhysics` Mismatch), `IR003` (Missing or Mid-Block Terminator), `IR004` (`0b01`/`0b11` Borrowed Value Escaping Return or `.pchor` Cue), `IR005` (Double Drop or Unreleased `0b11` `IGuard` Along Any Exit Path), `IR006` (`noalloc` Function Contains Heap Allocation Op), `IR007` (Pass Fixed-Point Iteration Cap `>64` Exceeded), `IR008` (`ConstructReuse` Target Layout Size/Align Mismatch with `PerceusDropReuse` Token).

#### 6.6 Verification & 3-Gate Acceptance Criteria
1. Confirm `selfhost/transpile/ir.pilot` is deleted and all 7 transpilers + VM + C + LLVM + WASM + ASM + GPU backends emit valid outputs from `PilIR`.
2. Verify `parse_ir_module(print_ir_module(m))` round-trips identically on `selfhost/analyzer.pilot`.
3. Register a custom user `comptime` `[IrPass]` inside a test `.pilot` file that folds `x + 0 -> x` and counts instructions $\rightarrow$ verify `PassManager` executes it and `verifier.pilot` validates SSA dominance.
4. Pass Dual-Pass Universal Rename Litmus (`M1`) proving `PilIR` contains **zero** hardcoded type names.

---

### M7 — Zero-Black-Box Transparency Tool (`pilot inspect`) [STATUS: SIMULATED]

#### 7.1 Objective & Current Codebase Audit
- **Root Bottleneck:** Pilot achieves extreme conciseness via `syntax` macros, distributive headers (`pub static pure:`, `let List<i32> { ... }`), `*Bundle`/`*Facet` splicing (`coalesce`, `with`, `without`), `[Attribute]` AST weavers (`[Clamp]`, `[Formula]`, `[TraceSpan]`), implicit `field`/`value` properties, 4-tier `MethodTable` precedence, and automatic Perceus ARC. Currently there is **no built-in tool** for a developer (or AI agent) to inspect what those layers expand into, why a method resolved to Tier 2 instead of Tier 3, or where Perceus inserts `dup`/`drop`/`drop_reuse`.

#### 7.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/inspect/selector.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Inspect.Selector;` | `~180L` | Resolves target queries: whole file (`app.pilot`), cursor span (`app.pilot:42:10`), line range (`app.pilot:40-55`), or symbol path (`App.Billing.Account.balance`, including generic instantiations `List<i64>`). |
| **CREATE** | `selfhost/inspect/weave_lens.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Inspect.WeaveLens;` | `~270L` | Pretty-prints expanded Pilot source via `selfhost/fmt/fmt.pilot` with origin comments annotating which `syntax` macro, `*Bundle`, or `[Attribute]` weight stage synthesized each statement. |
| **CREATE** | `selfhost/inspect/layout_lens.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Inspect.LayoutLens;` | `~260L` | Renders visual ASCII/box-drawing byte-offset diagrams, cache-line (`64B`) boundaries, padding holes, niche bit patterns, register class, and the 2-bit lattice derivation tree. |
| **CREATE** | `selfhost/inspect/method_lens.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Inspect.MethodLens;` | `~220L` | Renders full 4-tier `MethodTable` resolution trace (winning candidate + shadowed lower-tier defaults + active `ext` extensions + collision modifier provenance). |
| **CREATE** | `selfhost/inspect/perceus_lens.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Inspect.PerceusLens;` | `~240L` | Renders source/IR interleaved with inline Perceus markers (`[borrow]`, `[move]`, `[dup +1]`, `[drop -1]`, `[reuse-in-place]`, `[guard-exit LIFO #N]`). |
| **CREATE** | `selfhost/cli/inspect_cmd.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Cli.InspectCmd;` | `~190L` | Registers `pilot inspect` in `CommandRegistry` (`M3`) and exposes LSP `pilot/inspect` custom request handler in `selfhost/lsp/handlers.pilot`. |

#### 7.3 Complete Data Structures, CLI Flags & JSON Output Contract
- **Data Schemas:**
  - **`InspectTargetKind` (`enum : u8`, `0b00 : IValue`)**: `::WholeFile`, `::LineColCursor`, `::LineRange`, `::SymbolPath`, `::MonomorphizedType`.
  - **`[Layout(c_abi)] pub struct FieldProbeEntry : IValue`** (`0b00`):
    - `pub field_name: str; pub type_repr: str; pub byte_offset: usize; pub byte_size: usize; pub bit_offset: u16; pub bit_width: u16; pub padding_before: usize; pub lattice_bits: u8; pub origin_bundle: str;`
  - **`pub struct LayoutInspection : IObject`** (`0b10`):
    - `pub type_name: str; pub target_triple: str; pub total_size: usize; pub alignment: usize; pub stride: usize; pub total_padding: usize; pub reg_class: str; pub niche_desc: str; pub lattice_bits: u8; pub lattice_proof: List<str>; pub fields: List<FieldProbeEntry>; pub optimal_field_order_saving: usize;`
  - **`[Layout(c_abi)] pub struct MethodCandidateTrace : IValue`** (`0b00`):
    - `pub member_name: str; pub signature: str; pub tier: u8` (`1..4`, or `5` for scoped `ext`); `pub provider_name: str; pub won_dispatch: bool; pub conflict_modifier: str` (`""`, `"coalesce"`, `"with"`, `"without"`).
  - **`[Layout(c_abi)] pub struct PerceusEventNote : IValue`** (`0b00`):
    - `pub span: SourceSpan; pub binding_name: str; pub action: str` (`"borrow"`, `"move"`, `"dup"`, `"drop"`, `"drop_reuse"`, `"guard_exit"`); `pub paired_reuse_site: SourceSpan;`
- **CLI Flags Registered in `CommandRegistry`:**
  - `pilot inspect <selector> [--weave] [--layout] [--methods] [--perceus] [--ir] [--pass <pass_name>] [--diff-passes] [--target <arch-os>] [--style readable|compact] [--json] [--no-color]`.
  - If no lens flag is passed, `--all` summary mode auto-selects `--weave` + `--layout` + `--methods` for `struct`/`interface` targets and `--weave` + `--perceus` + `--ir` for `fn` targets.
- **Diagnostic Codes:** `INSP001` (Selector File/Symbol Not Found), `INSP002` (Uninstantiated Generic Requires Concrete Type Args for Byte Layout, e.g. `pilot inspect "Map<str, i64>" --layout`), `INSP003` (Unknown IR Pass Name), `INSP004` (Unknown Target Triple).

#### 7.4 Edge Cases & 3-Gate Verification Criteria
1. **Partial Struct Multi-File Aggregation:** Define `pub partial struct Widget` across 2 files with a `*Facet` spliced in file 2; run `pilot inspect Widget --methods --layout` $\rightarrow$ merges both partial definitions accurately.
2. **Weave Compilability Proof:** Run `pilot inspect <target> --weave` on a struct using `[Clamp(0.0, 100.0, weight: 100), TraceSpan(name: "set_hp", weight: 50)]` + `*SpatialHeaderBundle` + implicit `field`/`value` setter $\rightarrow$ pipe the emitted Pilot source directly into `pilot check -` and assert `0` errors and identical `PilIR`!
3. **LSP Latency Proof:** Query `pilot/inspect` over JSON-RPC in `selfhost/lsp/server.pilot` $\rightarrow$ responds in `< 5 ms` from warm `PilotDb`.

---

### M8 — Progressive Diagnostics & Lossless CST Auto-Fix (`pilot check --fix`) [STATUS: LIBRARY ONLY]

#### 8.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Flat Single-Line Diagnostics Without Progressive Context):** [`selfhost/diagnostics/{errors,warnings,infos}.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/diagnostics/errors.pilot) currently stores static metadata arrays (`ErrorDef`), while `crates/pilot-cli/src/main.rs` (line `90`) prints raw debug spans (`at ByteSpan { start: 42, end: 49 }`) instead of visual multi-line source snippets, secondary labels, or beginner-friendly guidance!
- **Root Bottleneck 2 (No Automated Lossless Quick-Fix Rewriter):** Even when the compiler knows the exact fix (e.g., `DECL004`: changing `let x` to `var x`; missing `;` statement terminator; leading-zero float `.5` $\rightarrow$ `0.5`; legacy keyword `mut`/`match`/`def` replacement; adding `var ref` on mutable borrow; adding `iso` on `.pchor` cue; removing unused `use`), users must edit files manually.

#### 8.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/diagnostics/model.pilot` | `pub lvl(0) pkg Pilot.Diagnostics.Model;` | `~240L` | Single canonical home for `Diagnostic`, `LabeledSpan` (`Primary`/`Secondary`), `Note`, `FixSuggestion`, `CstTextEdit`, `FixApplicability`, and open `DiagnosticCatalog` registry (imported across compiler and public `pilot.diag` with zero type duplication per `imp_type.md` Rule 11). |
| **CREATE** | `selfhost/diagnostics/renderer.pilot` | `pub lvl(0) pkg Pilot.Diagnostics.Renderer;` | `~350L` | Unicode/ASCII gutter snippet renderer (multi-span underlines, UTF-8 display-column width calculation, terminal color auto-detection/`NO_COLOR`, `--explain <CODE>` tutorial printer, and `--json` / SARIF 2.1.0 emitter). |
| **CREATE** | `selfhost/cst/autofix.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Cst.Autofix;` | `~330L` | Transactional lossless CST edit applier: overlap conflict detector, trivia/comment preservation verifier, `--fix` atomic file writer, `--fix-dry-run` unified diff previewer, and automated `imp_type.md` transforms (expanding wildcard imports to targeted imports, rewriting raw `__engine("map_*")` to `Map<K, V>`). |
| **MODIFY** | `selfhost/diagnostics/{errors,warnings,infos}.pilot` | `Pilot.Diagnostics.*` | `+190L` | Enrich catalog entries with `why_summary`, `tier1_hint`, `example_bad`, `example_good`, and structured `FixBuilder` templates; add `TYP*` (`TYP001`–`TYP006` for `imp_type.md` violations), `MOD*`, `CLI*`, `DB*`, `IR*`, `INSP*`, `FIX*`, `RING*`, `VM*` codes. |
| **DELETE** | `crates/pilot-analysis/src/diagnostics.rs` formatting helpers & `main.rs` raw `ByteSpan` `Debug` printing | Legacy Rust diagnostic formatting | `-35L` | All human & JSON diagnostic rendering now runs 100% through `Pilot.Diagnostics.Renderer`. |

#### 8.3 Complete Data Structures, CLI Flags & Transactional CST Auto-Fix Algorithm
- **Data Schemas (`selfhost/diagnostics/model.pilot`):**
  - **`DiagSeverity` (`enum : u8`, `0b00 : IValue`)**: `::Error = 1`, `::Warning = 2`, `::Info = 3`, `::Hint = 4`.
  - **`FixApplicability` (`enum : u8`, `0b00 : IValue`)**:
    - `::MachineApplicable` (`0` — deterministic syntax/type/mutability correction; applied automatically by `pilot check --fix`).
    - `::HasPlaceholders` (`1` — contains `_?` template hole; offered in LSP Code Actions, skipped by batch `--fix` unless `--fix-all`).
    - `::MaybeIncorrect` (`2` — heuristic suggestion; requires IDE confirmation or `--fix-allow-unsure`).
  - **`[Layout(c_abi)] pub struct LabeledSpan : IValue`** (`0b00`): `pub span: SourceSpan; pub label: str; pub is_primary: bool;`
  - **`[Layout(c_abi)] pub struct CstTextEdit : IValue`** (`0b00`): `pub span: SourceSpan; pub replacement: str;`
  - **`pub struct FixSuggestion : IObject`** (`0b10`): `pub title: str; pub applicability: FixApplicability; pub edits: List<CstTextEdit>;`
  - **`pub struct Diagnostic : IObject`** (`0b10`): `pub code: str; pub severity: DiagSeverity; pub message: str; pub labels: List<LabeledSpan>; pub notes: List<str>; pub fixes: List<FixSuggestion>;`
- **Public Signatures & CLI Flags:**
  - `pilot check [paths...] [--fix] [--fix-dry-run] [--fix-allow-unsure] [--explain <CODE>] [--format human|json|sarif] [--max-errors <N>] [--deny-warnings]`.
  - `pub fn apply_file_fixes_lossless(old_src: str, fixes: slice<FixSuggestion>, max_passes: usize) -> res<(str, usize), Diagnostic>;`
- **Conflict-Free Atomic Application & Comment-Preservation Law (`selfhost/cst/autofix.pilot`):**
  1. Collect eligible `FixSuggestion` items per `FileUnitId` and sort `CstTextEdit` spans ascending by `(span.start, span.end)`.
  2. **Multi-Edit Disjointness Solver:** Apply edits in reverse byte order (`end` $\rightarrow$ `start`) so earlier byte offsets never shift! If `edit[i].span.end > edit[i+1].span.start`, skip `edit[i+1]` in pass 1 and re-analyze via `PilotDb` (`M5`) in pass 2 (up to `3` convergence passes).
  3. **Strict Comment Conservation Proof:** Re-lex `new_src` via `selfhost/cst/cst.pilot` and assert every `//` and `/*...*/` comment token from `old_src` (outside explicitly removed unused-import lines) is preserved 100% verbatim; otherwise abort file write with `FIX001 (CommentLossPrevented)`.
  4. **Atomic Write:** Write to `<file>.pilot.tmp.<pid>` and `rename()` atomically so an interrupted `--fix` never leaves half-written source files.

#### 8.4 Edge Cases & 3-Gate Verification Criteria
1. Create a file containing 6 auto-fixable defects (`let` mutated $\rightarrow$ `var`, missing `;`, `.5` $\rightarrow$ `0.5`, `mut` $\rightarrow$ `var`, `match` $\rightarrow$ `switch`, missing `var ref` at call site) interleaved with 12 `/* inline */` and `// line` comments + CRLF `\r\n` line endings $\rightarrow$ `pilot check --fix` fixes all 6 in `< 30 ms`, preserves CRLF and all 12 comments verbatim, and subsequent `pilot check` exits `0`!
2. Verify `pilot check --explain ESCAPE001` prints progressive Tier-1 plain-English explanation + runnable bad/good code examples.
3. Verify UTF-8 multi-byte codepoints (Greek math `Δ`, emoji, CJK) align `^^^` caret spans to exact Unicode East-Asian display widths.

---

### M9 — Three-Ring Standard Library Personality (`core`, `std`, `pilot`) [STATUS: INCOMPLETE]

#### 9.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Python-Clone Identity Sprawl in `stdlib/`):** Workspace audit of `stdlib/` reveals **21 legacy Python-clone or duplicate directories/files** (`pickle`, `urllib`, `request`, `unittest`, `timeit`, `tomllib`, `functools`, `itertools.pilot`, `pathlib.pilot`, `shutil.pilot`, `math/random.pilot` duplicating `random/random.pilot` + `secrets/`, `pandas`, `numpy`, `matplotlib`, `scikit`, `asyncio/asyncio.pilot`, `threading`, `multiprocessing`) plus **80 stale `stdlib/.pilc-cache/std.*.pilc` binaries** mirroring those names!
- **Root Bottleneck 2 (Blurred Freestanding vs. OS-Dependent Boundary):** Embedded, kernel, GPU, and tiny-WASM targets require a strictly freestanding, `noalloc`, OS-independent `core` ring guaranteed never to transitively pull in sockets, threads, or heap syscalls.

#### 9.2 Complete Disposition Matrix for All Existing `stdlib/` Directories
| Current `stdlib/` Path | Action | Target Canonical Ring & Package (`lowercase` Module Path) | Exported Native Pilot Surface |
| :--- | :--- | :--- | :--- |
| `stdlib/core/**` | **KEEP & HARDEN** | **Ring 1 (`core`)** — `core`, `core.mem`, `core.ops`, `core.opt`, `core.res`, `core.simd`, `core.meta`, `core.reflect`, `core.contracts`, `core.rt`, `core.syntax.*` | All `[Lang]` primitives, 2-bit lattice interfaces, `opt<T>`, `res<T, E>`, `slice<T>`, `stride<T>`, `simd<T, N>`, weavers, `.pchor` cue verifier. |
| `stdlib/collections/collections.pilot` | **MIGRATE** | **Ring 2 (`std`)** — `stdlib/std/col/` (`pub pkg std.col;`) | `List<T>`, `Map<K, V>`, `Set<T>`, `Deque<T>`, `Ring<T>`, `BinaryHeap<T>`. |
| `stdlib/collections/itertools.pilot` & `stdlib/functools/` | **DELETE** | Folded into `core` iterator facets & `std.col` lazy adapter views | Retires Python `itertools`/`functools` names. |
| `stdlib/io/{io,fs,path}.pilot` | **MIGRATE** | **Ring 2 (`std`)** — `stdlib/std/io/`, `std/fs/`, `std/path/` (`std.io`, `std.fs`, `std.path`) | `IReader`, `IWriter`, `BufReader`, `File`, `DirEntry`, `PathView` (`0b01`), `PathBuf` (`0b10`). Cross-platform POSIX + Win32 (automatic UTF-8 $\leftrightarrow$ UTF-16 `wchar_t` transcoding via stack-allocated buffer; maps `errno` / `GetLastError()` to unified `IoError`). |
| `stdlib/io/{pathlib,shutil}.pilot` | **DELETE** | Tree copy/remove & path join folded into `std.fs` & `std.path` | Retires Python `pathlib`/`shutil` duplicates. |
| `stdlib/socket/` & `stdlib/http/` | **MIGRATE** | **Ring 2 (`std`)** — `stdlib/std/net/` (`std.net`) & `std/http/` (`std.http`) | `IpAddr`, `TcpStream`, `TcpListener`, `UdpSocket`, `Url`, `HttpReq`, `HttpResp`, `HttpServer`. Cross-platform async I/O multiplexing (`epoll`/`kqueue`/`IOCP`/WASI `poll_oneoff`). |
| `stdlib/urllib/` & `stdlib/request/` | **DELETE** | URL parser in `std.net`; client unified in `std.http` | Eliminates 3-way `urllib`/`request`/`http` split. |
| `stdlib/asyncio/select_syntax.pilot` & `stdlib/core/task.pilot` | **MIGRATE** | **Ring 2 (`std`)** — `stdlib/std/task/` (`std.task`), `std/chan/` (`std.chan`), `std/sync/` (`std.sync`) | `Task<T>`, `Chan<T>`, `select` syntax, `Mutex<T>`, `MutexGuard<T>` (`0b11`), `RwLock<T>`, `Atomic<T>`. |
| `stdlib/asyncio/asyncio.pilot`, `threading/`, `multiprocessing/` | **DELETE** | Superseded by `std.task`, `std.chan`, `std.sync`, and `std.proc` | Purges Python concurrency module names. |
| `stdlib/crypto/` | **MIGRATE** | **Ring 2 (`std`)** — `stdlib/std/crypto/` (`std.crypto`) | `Sha256`, `Sha512`, `Blake3`, `Hmac`, `Ed25519Keypair`, `Ed25519Sig`, constant-time `ct_eq`. |
| `stdlib/random/`, `stdlib/math/random.pilot`, `stdlib/secrets/` | **CONSOLIDATE & DELETE** | **Ring 2 (`std`)** — `stdlib/std/rand/` (`std.rand`) | `Rng` (`Xoshiro256StarStar`), `SeededRng`, and OS-backed `Csprng` (`fill_secure_bytes`). |
| `stdlib/json/`, `stdlib/csv/`, `stdlib/encoding/`, `stdlib/formats/` | **MIGRATE** | **Ring 2 (`std`)** — `std.json`, `std.toml`, `std.csv`, `std.bin`, `std.enc` | `JsonVal`, zero-copy `JsonParser`, `TomlDoc`, `CsvReader`, binary codec `std.bin`, `hex`/`base64`. |
| `stdlib/pickle/` & `stdlib/tomllib/` | **DELETE** | Replaced by memory-safe `std.bin` and `std.toml` | Purges `pickle` (arbitrary-code-exec anti-pattern) & `tomllib`. |
| `stdlib/db/` | **MIGRATE** | **Ring 2 (`std`)** — `stdlib/std/db/` (`std.db`, `std.db.pg`, `std.db.mysql`, `std.db.sql`) | `Table<T>`, LINQ/SQL query AST, SQLite engine, Postgres v3 & MySQL binary wire drivers. |
| `stdlib/math/`, `stdlib/decimal/`, `stdlib/numeric/` | **MIGRATE** | `core.math` (freestanding scalars) + **Ring 2** `std.math`, `std.num` | `Decimal128` (`0b00`), `Ratio<T>` (`0b00`), `Complex<T>` (`0b00`), `BigInt` (`0b10`). |
| `stdlib/numpy/`, `stdlib/pandas/`, `stdlib/matplotlib/`, `stdlib/scikit/` | **DELETE** | Real Python ecosystems bind zero-copy via `use py` (`M17`); native strided math lives in `core`/`std.math` | Purges Python library clone names from `stdlib/`. |
| `stdlib/os/` & `stdlib/sys/` | **MIGRATE** | **Ring 2 (`std`)** — `std.os`, `std.env`, `std.proc`, `std.time` | `Instant`, `Duration`, `Command` (`Process` builder), env map, OS arch/signal helpers. |
| `stdlib/logging/` & `stdlib/text/` | **MIGRATE** | **Ring 2 (`std`)** — `std.log` & `std.text` | Structured ring-buffered `Logger`, UTF-8 grapheme/case/builder utilities. |
| `stdlib/unittest/` & `stdlib/timeit/` | **DELETE** | **Ring 2 (`std`)** — `stdlib/std/test/` (`std.test`) & `std/bench/` (`std.bench`) | `[Test]`, `[Bench]`, `assert_eq`, differential test runner, statistical timer. |
| `stdlib/polyglot/` & `stdlib/pkg/` | **MIGRATE** | `std.polyglot` & **Ring 3** `pilot.pkg` | SHM arenas, marshalers, SPDX license verifier. |
| `stdlib/.pilc-cache/std.*.pilc` | **PURGE** | Dynamically cached in `.pilc-cache/pilotdb.v1` (`M5`) | Deletes all 80 stale committed `.pilc` files. |

#### 9.3 Static Three-Ring Dependency & Freestanding Law (`selfhost/loader/ring_verifier.pilot`, `~190L`)
Enforced automatically in `PilotDb` (`M5`) and `pilot gate` (`M1`):
1. **`RING001` (Illegal Upward Ring Dependency):** `core.*` (Ring 1) cannot import `std.*` (Ring 2), `pilot.*` (Ring 3), or `Pilot.SelfHost.*`; `std.*` (Ring 2) cannot import `Pilot.SelfHost.*`.
2. **`RING002` (Freestanding `core` Violation):** Any `PilIR` (`M6`) function inside `stdlib/core/` invoking OS host intrinsics (`fs_*`, `tcp_*`, `proc_*`, `dlopen`, `env_*`) or implicit global heap allocation fails build immediately.
3. **`RING003` (Retired Python-Clone Import Auto-Migration):** `use std.unittest;` / `use std.tomllib;` / `use std.pathlib;` / `use std.pickle;` emits `RING003` with a `MachineApplicable` `pilot check --fix` CST rewrite (`M8`) mapping directly to `use std.test;` / `use std.toml;` / `use std.path;` / `use std.bin;`!
4. **`RING004` (Duplicate Stale Cache Artifact):** `pilot gate` rejects committed `.pilc` binaries inside `stdlib/`.

#### 9.4 Edge Cases & 3-Gate Verification Criteria
1. **Bootstrap Alias Continuity:** `Pilot.Core.*` continues resolving seamlessly to `core.*` in `module_graph.pilot` (`M2`) so all `selfhost/` modules build with zero disruption while user code writes concise `use core.mem; use std.col;`.
2. **Bare-Metal & Multiplatform Symbol Table Proof:** Compile a freestanding module importing *only* `core.*` (`opt`, `res`, `slice`, `simd`, `core.math`) to `wasm32`, `x86_64-unknown-none` (ELF), `aarch64-unknown-none` (Mach-O), and `x86_64-pc-windows-none` (PE/COFF) $\rightarrow$ inspect symbol import table and assert **`0` external OS/libc imports**!
3. **Full Test Suite Migration:** Migrate all existing showcase examples (`examples/showcase/*.pilot`) and milestone suites (`crypto`, `postgres_wire`, `mysql_wire`, `http`, `polyglot`) to `std.*` using `pilot check --fix` and verify 100% green execution.

---

### M10 — Compiler-as-a-Library (`pilot.lex`, `pilot.cst`, `pilot.ast`, `pilot.types`, `pilot.ir`, `pilot.fmt`, `pilot.db`, `pilot.vm`, `pilot.diag`) [STATUS: SIMULATED]

#### 10.1 Objective & Current Codebase Audit
- **Root Bottleneck:** All compiler intelligence currently lives inside internal `Pilot.SelfHost.*` modules (`selfhost/lexer.pilot`, `selfhost/cst/*.pilot`, `selfhost/ast.pilot`, `selfhost/analyzer.pilot`, `selfhost/fmt/fmt.pilot`, `selfhost/ir/*.pilot`). Ordinary Pilot programs cannot cleanly `use pilot.ast;` or `use pilot.ir;` with stable, lattice-safe (`0b00`/`0b01`/`0b10`/`0b11`), ergonomic public types to build custom code generators, linters, shader compilers, architectural verifiers, or sandboxed scripting plugins.

#### 10.2 Module & File Manifest (`<= 1,500` LOC Law — Ring 3 `stdlib/pilot/`)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/pilot/lex.pilot` | `pub pkg pilot.lex;` | `~180L` | Public `Token : IValue` (`0b00`), `TokenCursor : IReference` (`0b01`), trivia classifier, and zero-allocation streaming tokenizer over `str`/`slice<u8>`. |
| **CREATE** | `stdlib/pilot/cst.pilot` | `pub pkg pilot.cst;` | `~260L` | Lossless Red/Green `CstTree : IObject` (`0b10`), `CstNodeView : IReference` (`0b01`), cursor navigation, pattern query `find_all`, and trivia-preserving `CstEditor`. |
| **CREATE** | `stdlib/pilot/ast.pilot` | `pub pkg pilot.ast;` | `~310L` | Typed `AstModule : IObject` (`0b10`), `AstNodeView : IReference` (`0b01`), programmatic `QuoteBuilder` (`splice`/`unquote`), visitor/folder interfaces (`IAstVisitor`, `IAstFolder`), and hygienic `gensym`. |
| **CREATE** | `stdlib/pilot/types.pilot` | `pub pkg pilot.types;` | `~240L` | First-class `TypeInfo : IValue`, `FieldInfo : IValue`, `MethodEntry : IValue`, `LatticeClass` (`::Value`, `::Reference`, `::Object`, `::Guard`), target layout solver, and interface conformance queries. |
| **CREATE** | `stdlib/pilot/ir.pilot` | `pub pkg pilot.ir;` | `~250L` | Public facade over `PilIR` (`M6`): `IrModule`, `IrBuilder`, `IPilIrPass`, `verify()`, and pass pipeline runner for user-authored domain compilers & shader/DSL backends. |
| **CREATE** | `stdlib/pilot/fmt.pilot` | `pub pkg pilot.fmt;` | `~140L` | Public `format_str(src: str, style: FmtStyle) -> res<str, List<Diagnostic>>` and `format_cst(tree: ref<CstTree>, style: FmtStyle) -> str`. |
| **CREATE** | `stdlib/pilot/db.pilot` | `pub pkg pilot.db;` | `~210L` | Public `CompilerSession : IObject` (`0b10`) wrapping `PilotDb` (`M5`) supporting both virtual in-memory file overlays and real workspace directories. |
| **CREATE** | `stdlib/pilot/vm.pilot` | `pub pkg pilot.vm;` | `~240L` | Capability-gated `VmSandbox : IObject` (`0b10`) + `VmLeaseGuard : IGuard` (`0b11`) executing compiled `PilIR`/`.pilc` with instruction `fuel`, `max_heap_bytes`, and capability bitmasks. |
| **CREATE** | `stdlib/pilot/diag.pilot` | `pub pkg pilot.diag;` | `~140L` | Public facade re-exporting canonical `Diagnostic`, `FixSuggestion`, `render_ansi`, `render_json`, and `render_sarif` APIs from `Pilot.Diagnostics.Model` (`M8`) with zero duplicate type declarations (`imp_type.md` Rule 11). |

#### 10.3 Complete Public API Signatures & 2-Bit Lattice Safety Contracts
- **`pilot.lex`:**
  - `pub pure fn tokenize(source: str) -> TokenCursor;` (`0b01` zero-alloc lazy cursor yielding `Token { kind_id: u16, span: ByteSpan, is_trivia: bool }`).
- **`pilot.cst`:**
  - `pub fn parse_lossless(source: str) -> CstTree;` (never fails—syntax errors become `ErrorNode` green subtrees preserving 100% of source bytes: `tree.root().full_text() == source` invariant!).
  - `CstTree.root(ref self) -> CstNodeView` (`0b01` borrow; Perceus `ESCAPE001` statically prevents `CstNodeView` from escaping `CstTree`'s lexical lifetime!).
  - `CstEditor.replace_node(var ref self, target: CstNodeView, replacement_code: str)` $\rightarrow$ preserves leading/trailing comment trivia automatically.
- **`pilot.ast` & `pilot.db`:**
  - `CompilerSession.new_in_memory() -> CompilerSession` / `CompilerSession.open_workspace(root_path: str) -> res<CompilerSession, Diagnostic>`.
  - `CompilerSession.set_file(var ref self, virtual_path: str, content: str)` (triggers `PilotDb` incremental revision!).
  - `CompilerSession.check_all(var ref self) -> slice<Diagnostic>`.
  - `CompilerSession.expand_ast(var ref self, virtual_path: str) -> res<AstNodeView, List<Diagnostic>>`.
  - `CompilerSession.type_of(var ref self, virtual_path: str, offset: u32) -> opt<TypeInfo>`.
  - `CompilerSession.lower_ir(var ref self, virtual_path: str) -> res<ref<IrModule>, List<Diagnostic>>`.
- **`pilot.vm` Deterministic Capability Sandbox (`VmSandbox`):**
  - **`VmCapability` (`bitflags : u32`, `0b00 : IValue`)**: `::None = 0` (100% pure computation), `::Clock = 1`, `::RandomSeeded = 2`, `::StdoutCapture = 4`, `::VirtualFs = 8`, `::HostFsRead = 16`, `::HostFsWrite = 32`, `::HostNet = 64`, `::HostFfi = 128`.
  - **`[Layout(c_abi)] pub struct VmConfig : IValue`** (`0b00`): `pub fuel_limit: u64; pub max_heap_bytes: usize; pub max_call_depth: u32; pub capabilities: u32;`
  - **`VmTrap` (`enum : u8`, `0b00 : IValue`)**: `::FuelExhausted` (`VM001`), `::HeapLimitExceeded` (`VM002`), `::StackDepthExceeded` (`VM003`), `::CapabilityDenied` (`VM004`), `::DivisionByZero` (`VM005`), `::AssertionFailed` (`VM006`).
  - `VmSandbox.eval_fn(var ref self, mod_ir: ref<IrModule>, entry_fn: str, args: slice<VmVal>) -> res<VmOutcome, VmTrap>` (captures virtual stdout buffer + exact instructions executed + peak arena bytes!).

#### 10.4 Edge Cases & Security Invariants
1. **Cross-Arena Node Splice Protection (`AST001`):** Every `AstNodeView` carries `arena_generation_id: u32`; attempting to splice a node from `Arena A` directly into `Arena B` without `clone_subtree_into` is caught cleanly rather than indexing wrong arena slots.
2. **Infinite Loop & Bomb Isolation in `VmSandbox`:** Running `loop {}` inside `VmSandbox` with `fuel_limit: 100_000` deterministically traps with `VmTrap::FuelExhausted` in `< 2 ms`, and `~self()` on `VmSandbox` frees the entire sandbox heap slab in $O(1)$ with zero host memory leaked.
3. **Host Capability Denial (`VM004`):** Executing `std.fs.read_text("/etc/passwd")` inside a `::None` capability `VmSandbox` immediately traps with `VmTrap::CapabilityDenied("HostFsRead")` before touching any OS syscall!

#### 10.5 End-to-End Integration Proof ("Standalone User Mini-Compiler Test")
To prove **Everything First-Class** at the close of Phase 1 (`M10`), an external integration test outside `selfhost/` (importing *only* `core.*`, `std.*`, and `pilot.*`) performs all 5 steps in ~30 lines of clean Pilot:
1. Parses raw Pilot source containing comments + compact symbols (`T?`, `f!^(x)`) via `pilot.cst.parse_lossless`, rewrites a constant via `CstEditor`, and verifies 100% comment preservation.
2. Formats the result in both `FmtStyle::Readable` (`opt<T>`, `sig f(x)`) and `FmtStyle::Compact` via `pilot.fmt`.
3. Mounts the code in `pilot.db.CompilerSession.new_in_memory()`, queries `pilot.types` to verify `0b00` vs `0b10` lattice bits and struct field byte offsets, and runs a custom user `IPilIrPass` via `pilot.ir`.
4. Executes the compiled `IrModule` inside a `VmConfig { fuel_limit: 50_000, max_heap_bytes: 65_536, max_call_depth: 64, capabilities: VmCapability::StdoutCapture }` `VmSandbox` (`pilot.vm`) and asserts exact return value, captured stdout, and `0` leaked bytes.

#### 10.6 Phase 1 (`M1`–`M10`) Cumulative Exit Checkpoint
Before advancing to **Phase 2 (`M11` — Universal Foreign-Language Adapter v1)**:
- **Gate 1 (Adaptability & Multiplatform):** `pilot gate --full` runs 100% in pure Pilot; Python `scripts/gate*.py` and frozen Rust SHA-256 locks are retired; Dual-Pass Universal Rename Litmus passes across all backends and `PilIR`; cross-target compilation and path/CRLF invariance validated across Linux, macOS, Windows, WASM, and bare-metal targets.
- **Gate 2 (Bootstrap & Speed):** Stage-2 == Stage-3 SHA-256 fixed-point parity holds on `PilIR`-backed `pilotc`; warm incremental `PilotDb` single-function edit re-checks in `< 8 ms`; `crates/pilot-cli/src/main.rs` is `<= 35` LOC.
- **Gate 3 (Ergonomics, Purity & Cleanliness):** Every compact symbol has a verified readable `lowercase` counterpart (`M4`); `pilot inspect` exposes every weave/layout/method/Perceus/IR decision (`M7`); `pilot check --fix` repairs code with 100% comment conservation (`M8`); all 21 legacy Python-clone `stdlib/` folders and 80 stale `.pilc-cache` blobs are purged with freestanding `core` verified (`M9`); and the entire compiler is importable as `pilot.*` (`M10`).

---

## Phase 2 (`M11` – `M20`): Priority Pillar — Universal Foreign-Language Adapter v1 (UNABRIDGED SPECIFICATION)

> **Priority Mandate:** Using other languages from Pilot must feel completely natural, idiomatic, memory-safe, and near-zero-overhead—with **zero manual glue files**, **zero subprocess-per-call bottlenecks**, **zero-copy `0b01` buffer/tensor sharing**, and **automatic `0b10`/`0b11` Perceus lifecycle & foreign GC/GIL synchronization**. Every milestone below (`M11`–`M20`) specifies the complete engineering blueprint leaving only literal code bodies omitted.

---

### M11 — First-Class `adapter` Blueprint Engine & Zero-Cost Idiom Lifter [STATUS: SIMULATED]

#### 11.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`adapter` Is Currently an Unimplemented Receptor Stub):** [`stdlib/contracts/contracts_syntax.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/contracts/contracts_syntax.pilot) (line `14`) registers `__engine("bind_receptor", nameof(adapter), 33, 21)`, and [`stdlib/polyglot/polyglot_syntax.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/polyglot/polyglot_syntax.pilot) (lines `13`–`35`) registers language tag receptors (`c`, `cpp`, `rust`, `py`, `js`, `ts`, `sql`, etc.), yet neither analyzer nor IR compiler transforms `adapter` declarations into callable methods or zero-cost wrappers.
- **Root Bottleneck 2 (Hardcoded Mock Exports in `crates/pilot-ffi/src/lib.rs`):** [`crates/pilot-ffi/src/lib.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-ffi/src/lib.rs) still contains hardcoded mock functions `pilot_transpile_sample` (lines `143`–`155`, returning static `x * 2` strings) and `pilot_polyglot_execute` (lines `164`–`167`, returning `"Executed N chars..."` without running anything).
- **Root Bottleneck 3 (Raw Foreign Idiom Impedance):** Without declarative idiom lifting, calling foreign APIs forces ugly Cisms (`lib_foo_do_work(handle, slice.ptr, slice.len, &mut out_val)` + manual `if rc != 0` checks) directly into clean Pilot domain code.

#### 11.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/adapter/rules.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.Rules;` | `~310L` | Open table-driven `AdapterRuleRegistry` and `IAdapterTransform` protocol (`strip_prefix`, `case_norm`, `receiver_promote`, `slice_pair`, `cstr_view`, `out_param`, `error_lift`, `null_opt`, `callback_closure`). |
| **CREATE** | `selfhost/adapter/compiler.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.Compiler;` | `~420L` | Compiles `adapter RawMod -> CleanType { ... }` declarations into synthesized Tier-1/Tier-2 `MethodTable` entries and inline zero-allocation `PilIR` (`M6`) thunks. |
| **CREATE** | `stdlib/std/polyglot/adapter.pilot` | `pub pkg std.polyglot.adapter;` | `~190L` | Public user-facing `[Adapt(...)]` attributes, `CError`, `Errno`, `NullPtrErr`, and composable preset profiles (`PosixAdapterFacet`, `Win32AdapterFacet`). |
| **MODIFY** | `crates/pilot-ffi/src/lib.rs` | Seed C-ABI bridge | `-32L` | **Delete** `pilot_transpile_sample` and `pilot_polyglot_execute` fake mocks permanently. |

#### 11.3 Complete Data Structures, Transformation Rules & 2-Bit Lattice Physics
- **`NamingStyle` (`enum : u8`, `0b00 : IValue`)**: `::Preserve`, `::SnakeCase`, `::PascalCase`, `::CamelCase`.
- **`ErrorLiftPolicy` (`enum : u8`, `0b00 : IValue`)**:
  - `::ZeroIsOk` (`rc == 0` $\rightarrow$ `ok`, else `err(rc)`)
  - `::NonNegativeIsOk` (`rc >= 0` $\rightarrow$ `ok(rc as usize)`, `< 0` $\rightarrow$ `err(errno_or_rc)`)
  - `::NonNullIsOk` (`ptr != null` $\rightarrow$ `ok`, `null` $\rightarrow$ `err`)
  - `::CustomPredicate` (user `pure fn(TRaw) -> opt<TErr>`).
- **`[Layout(c_abi)] pub struct SlicePairRule : IValue`** (`0b00`):
  - `pub ptr_param_idx: u16; pub len_param_idx: u16; pub len_is_byte_size: bool; pub as_utf8_str: bool;`
- **`[Layout(c_abi)] pub struct OutParamRule : IValue`** (`0b00`):
  - `pub param_idx: u16; pub pointee_layout: u32; pub zero_init_stack: bool;`
- **`[Layout(c_abi)] pub struct CallbackPairRule : IValue`** (`0b00`):
  - `pub fn_ptr_param_idx: u16; pub userdata_void_ptr_idx: u16;` (automatically unpacks a Pilot closure `fn<A, R>` into `(trampoline_fn_ptr, env_ptr)`!).
- **`pub struct AdapterBlueprint : IObject`** (`0b10`):
  - `pub source_ns: str; pub target_type: str; pub strip_prefixes: List<str>; pub method_case: NamingStyle; pub handle_raw_type: u32; pub drop_symbol: str; pub slice_rules: List<SlicePairRule>; pub out_rules: List<OutParamRule>; pub err_policy: ErrorLiftPolicy; pub err_type_layout: u32; pub callbacks: List<CallbackPairRule>;`

#### 11.4 Zero-Overhead `PilIR` Lowering Pipeline (`selfhost/adapter/compiler.pilot`)
Every adapted method compiles to a **force-inlined `PilIR` sequence** with **0 heap allocations**:
1. **Receiver Handle Unwrap:** First foreign parameter matching `handle_raw_type` becomes `self` (immutable `ref<Target>` if `const T*`, mutable `var ref<Target>` if `T*`).
2. **Contiguous View Unpack (`0b01`):** A single Pilot parameter `data: slice<T>` (or `text: str`) unpacks via `ExtractField(0)` (`ptr`) and `ExtractField(1)` (`len`, scaled by `sizeof(T)` if `len_is_byte_size`), feeding the two raw C parameters directly in registers.
3. **Out-Parameter Promotion:** Allocates an unescaped `StackSlot(pointee_layout)` in the caller's stack frame, passes its address to the foreign function, and `Load`s the result register on the `ok` path.
4. **Status-Code $\rightarrow$ `res<TOut, TErr>` (`TOut ! TErr`) Branch:** Emits `Cmp` against `ErrorLiftPolicy` and constructs Pilot's niche/tagged `result` value directly in registers.
5. **Property Synthesis:** Foreign `get_foo(h)` / `set_foo(h, v)` pairs automatically synthesize an implicit Pilot property `.foo` supporting fluid cascades (`obj..foo = 10..bar = 20;`)!

#### 11.5 Diagnostic Codes & Verification Criteria
- `ADP001` (Adapter Handle Type Not Found in Source Namespace), `ADP002` (Ambiguous Method Collision After Prefix Stripping), `ADP003` (Callback Function Missing `void*` Userdata Slot for Stateful Closure — stateless `pure fn` still allowed!), `ADP004` (Invalid `slice_pair` Pointer/Length Types).
- **Acceptance Proof:** Adapt a raw 12-function C mock API (usingprefixes, `(const uint8_t*, size_t)` pairs, `int* out_result`, status codes, and `(cb_fn, void* ctx)` callbacks) $\rightarrow$ inspect generated `PilIR` via `pilot inspect --ir` (`M7`) and assert **0 heap allocations**, direct `StackSlot` + `CallDirect` lowering, and 100% deletion of `pilot-ffi` mock functions.

---

### M12 — Headerless C11/C23 Semantic Absorber (`use c "header.h"`) [STATUS: LIBRARY ONLY]

#### 12.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`ffi_bindgen.pilot` Is Outbound-Only & Single-Param):** [`selfhost/ffi_bindgen.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/ffi_bindgen.pilot) only generates outbound C prototypes *from* Pilot ASTs (and `generate_ffi_binding` on line `54` only accepts a single parameter!). Pilot has **zero** ability to absorb standard C `.h` headers (`<sqlite3.h>`, `<zlib.h>`, `<sys/socket.h>`, `<pthread.h>`) directly via `use c`.
- **Root Bottleneck 2 (External `clang`/Bindgen Script Fragility):** Relying on external CLI bindgen tools creates dead generated files that drift across Linux/macOS/Windows target architectures (`size_t`, `off_t`, `struct stat` differ per OS/arch!).

#### 12.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/adapter/c_pp.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.CPp;` | `~480L` | Full C11/C23 preprocessor in pure Pilot: `#include "..."` / `<...>`, `#include_next`, `#pragma once`, object- & function-like `#define` (`#`, `##`, `__VA_ARGS__`), `#if`/`#elif` 64-bit integer expression evaluator, and predefined target macros (`__x86_64__`, `__aarch64__`, `__LP64__`, `_WIN64`, `__STDC_VERSION__`). |
| **CREATE** | `selfhost/adapter/c_parser.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.CParser;` | `~540L` | Recursive-descent + declarator-spiral C11/C23 parser handling `typedef`, `struct`, `union`, anonymous nested records, bitfields, `enum`, fixed arrays `T[N]`, complex function-pointer declarators `int (*(*fp)(int))(double)`, `_Atomic`, `restrict`, `__attribute__((packed, aligned(N)))`, `_Pragma("pack")`, and `static inline` header helpers! |
| **CREATE** | `selfhost/adapter/c_sysroot.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.CSysroot;` | `~220L` | Cross-platform include-path & library discovery (`/usr/include`, `/usr/local/include`, multiarch `/usr/include/<arch>-linux-gnu`, macOS `xcrun --show-sdk-path`, Windows Kits/MSVC, `PKG_CONFIG_PATH` `.pc` parser in pure Pilot!). |
| **MODIFY** | `selfhost/ffi_bindgen.pilot` | `pub lvl(0) pkg Pilot.SelfHost.FfiBindgen;` | `-95L / +80L` | Upgrade outbound C header emitter to consume `PilIR`/`TypeLayout` (`M6`) supporting arbitrary N-parameter functions, nested structs, and zero hardcoded `"Bits"` AST scans. |

#### 12.3 Complete Semantic Mapping: C11/C23 $\rightarrow$ Pilot Name-Blind Physics
Every C construct lowers directly into `TypeLayout` + `IrTypePhysics` (`M6`) for the active `TargetDescriptor` (`M2`):
- **Target-Width Integers:** `char`, `short`, `int`, `long` (`4B` on Win64 LLP64 vs. `8B` on Unix LP64!), `long long`, `size_t`, `ssize_t`, `intptr_t`, `__int128` mapped by exact `(bit_width, signedness)` to Pilot's matching `[Bits]` scalar receptor!
- **Records, Unions & Bitfields:**
  - `struct S` $\rightarrow$ `0b00 : IValue` `[Layout(c_abi)]` with exact SysV / MSVC field alignment & `#pragma pack(N)` byte offsets.
  - C bitfields (`unsigned flags : 3; unsigned mode : 5;`) packed into shared storage units with synthesized Pilot getter/setter bit-mask/shift properties (`s.flags`, `s.mode = 4`)!
  - `union U` $\rightarrow$ `0b00 : IValue` sized to `max(sizeof(fields))`, aligned to `max(alignof(fields))`, with typed field lens accessors.
  - Incomplete opaque structs (`typedef struct sqlite3 sqlite3;`) $\rightarrow$ distinct zero-sized marker tag so `ptr<sqlite3>` cannot be accidentally mixed with `ptr<FILE>` (stronger type safety than `void*`!).
- **Macro Constant Promotion:** Every object-like `#define FOO 0x10` or `#define VERSION "1.4"` that evaluates cleanly without side effects becomes a typed `pub comptime let FOO` constant in the imported namespace!
- **Inline Header Functions (`static inline`):** Small C header inline functions (ubiquitous in POSIX/SIMD/Vulkan headers) are lowered via `c_parser.pilot` directly into `PilIR` (`M6`) so Pilot calls them with **zero external wrapper compilation needed**!

#### 12.4 Public Signatures, Data Schemas, Diagnostics & Edge Cases
- **Core Schemas (`selfhost/adapter/c_parser.pilot`):**
  - **`[Layout(c_abi)] pub struct CBitfieldSpec : IValue`** (`0b00`): `pub storage_unit_offset: usize; pub bit_offset: u8; pub bit_width: u8; pub is_signed: bool;`
  - **`pub struct CRecordDecl : IObject`** (`0b10`): `pub name: str; pub is_union: bool; pub is_opaque: bool; pub pack_align: u8; pub has_flexible_array_tail: bool; pub fields: List<CFieldDecl>;`
  - **`pub struct CTranslationUnit : IObject`** (`0b10`): `pub header_path: str; pub constants: Map<str, ComptimeConst>; pub records: List<CRecordDecl>; pub enums: List<CEnumDecl>; pub functions: List<CFnDecl>; pub inline_fn_bodies: List<IrFunction>;`
- **Public Signatures:**
  - `pub fn preprocess_c_header(path: str, include_dirs: slice<str>, defines: ref<Map<str, str>>, target: ref<TargetDescriptor>) -> res<str, List<Diagnostic>>;`
  - `pub fn absorb_c_header(db: var ref<PilotDb>, spec: str, target: ref<TargetDescriptor>) -> res<ref<CTranslationUnit>, List<Diagnostic>>;`
- **Tricky C11/C23 Edge Cases Handled Explicitly:**
  1. **Traditional `#ifndef H_ / #define H_ ... #endif` Include Guards:** Detected automatically alongside `#pragma once` so cyclic/diamond system headers never re-open files from disk!
  2. **C99 Flexible Array Members (`struct Packet { uint32_t len; uint8_t payload[]; };`):** `has_flexible_array_tail = true` synthesizes a safe Pilot trailing-slice accessor `.payload(total_bytes)` (`0b01` `slice<u8>`) bounds-checked against `len`!
  3. **Calling Convention Attributes (`__stdcall`, `__fastcall`, `__cdecl` on Win32):** Propagated directly into `CFnDecl.calling_conv` (`M13`).
- **`PilotDb` Integration (`M5`) & `imp_type.md` Compliance:** `AbsorbCHeader(header_spec, defines_hash, target_triple)` is cached at `Durability::ExternalPkg` in `.pilc-cache/pilotdb.v1` (`< 30 ms` cold, `< 1 ms` warm). All C adapter modules (`c_pp.pilot`, `c_parser.pilot`, `c_sysroot.pilot`) strictly implement `imp_type.md`: zero raw `__engine` calls (Core Law 6 & Rule 12, using strongly-typed `Map<K, V>`, `List<T>`, and `fs.*`), zero duplicate types (Rule 11), `[Layout(c_abi)] struct : IValue` (`0b00`) records (Rule 3), and `bitflags : u32` for C flag enums (Rule 5).
- **Diagnostics:** `CAB001` (Header Not Found in Sysroot/PkgConfig Paths), `CAB002` (Unsupported GNU Statement-Expression in Macro — suggests explicit signature override), `CAB003` (Preprocessor `#error` Directive Triggered), `CAB004` (Invalid Bitfield Width Exceeding Storage Type).
- **Acceptance Proof:** Write a 15-line Pilot file starting with `use c "sqlite3.h" as raw;` + an `M11` `adapter` block $\rightarrow$ open an in-memory SQLite database, `CREATE TABLE`, `INSERT`, `SELECT` rows, and close cleanly with **zero `.c` glue files and zero `bindgen` CLI step**!

---

### M13 — Adaptive Hardware Calling-Convention Descriptors & Dynamic Frame Trampoline (`frame` / `!&`) [STATUS: LIBRARY ONLY]

#### 13.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Hardcoded Unix-Only `dlopen` Without Call Trampoline):** [`crates/pilot-rt/src/c_abi.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt/src/c_abi.rs) (lines `19`–`50`) only wraps `dlopen`/`dlsym` under `#[cfg(unix)]` (returning `"Unsupported platform"` on Windows!) and only returns a raw `*const ()` pointer—`PilVM` (`comptime` and `pilot run`) has **no generic dynamic trampoline** to invoke arbitrary multi-argument/struct-by-value C functions resolved via `dlsym`!
- **Root Bottleneck 2 (Opaque ABI Lowering):** Calling conventions should never be hardcoded inside Rust codegen crates; Pilot's `frame<FnSig>` (`!&` / `frame fn(args)`) was explicitly designed to represent hardware call frames as first-class Pilot values.

#### 13.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/pilot/abi/descriptor.pilot` | `pub pkg pilot.abi;` | `~260L` | Declarative `CallingConvSpec`, `ArgLocation` (`::GprReg`, `::FloatSimdReg`, `::RegPair`, `::StackSlot`, `::IndirectPtr`), `AbiFramePlan`, and `ICallingConvClassifier`. |
| **CREATE** | `stdlib/pilot/abi/sysv_x64.pilot` | `pub pkg pilot.abi.sysv_x64;` | `~240L` | Exact AMD64 System V eightbyte classification (`INTEGER`/`SSE`/`MEMORY`), `RDI,RSI,RDX,RCX,R8,R9` + `XMM0..7`, `AL` vector count for variadics, and 16-byte 2-register struct passing (`slice<T>` passes in 2 registers!). |
| **CREATE** | `stdlib/pilot/abi/win64.pilot` | `pub pkg pilot.abi.win64;` | `~180L` | Microsoft x64 ABI (`RCX,RDX,R8,R9` / `XMM0..3`, 32-byte home shadow space, power-of-2 `1/2/4/8B` register structs vs. indirect pointer, `sret` rules). |
| **CREATE** | `stdlib/pilot/abi/aapcs64.pilot` | `pub pkg pilot.abi.aapcs64;` | `~220L` | ARM64 AAPCS64 + Apple Darwin ARM64 (`X0..X7`, `X8` indirect result, `V0..V7` Homogeneous Floating-Point Aggregates `HFA` up to 4 floats/doubles, tightly packed Darwin stack variadics). |
| **CREATE** | `stdlib/pilot/abi/riscv_wasm.pilot` | `pub pkg pilot.abi.riscv_wasm;` | `~190L` | RISC-V `LP64D` (`a0..a7`, `fa0..fa7`) and `wasm32` C-ABI linear-memory shadow stack lowering. |
| **MODIFY** | `crates/pilot-rt/src/c_abi.rs` | Irreducible OS loader & register invoker | `+55L` | Add Win32 `LoadLibraryW`/`GetProcAddress`/`FreeLibrary` parity + a single **45-line name-blind register-pack dispatcher** `pilot_rt_invoke_abi_frame(fn_ptr, gpr_buf, xmm_buf, stack_buf, stack_len, ret_buf)` executing any `AbiFramePlan` constructed by Pilot! |

#### 13.3 Complete `frame` (`!&`) Schemas, Public Signatures & Zero-Marshalling Model
- **`ArgPassKind` (`enum : u8`, `0b00 : IValue`)**: `::Void`, `::DirectGpr`, `::DirectSimd`, `::PairGprGpr`, `::PairGprSimd`, `::PairSimdGpr`, `::PairSimdSimd`, `::HfaSimd3`, `::HfaSimd4`, `::IndirectByRef`, `::StackBytes`.
- **`[Layout(c_abi)] pub struct ArgLocation : IValue`** (`0b00`): `pub kind: ArgPassKind; pub reg0: u8; pub reg1: u8; pub stack_offset: u32; pub byte_size: u32;`
- **`pub struct AbiFramePlan : IObject`** (`0b10`):
  - `pub conv_id: str; pub sret_uses_gpr0: bool` (`true` on SysV `RDI` & Win64 `RCX`; `false` on ARM64 where dedicated `X8` holds `sret` without shifting `X0..X7`!!); `pub ret_loc: ArgLocation; pub param_locs: List<ArgLocation>; pub aligned_stack_bytes: u32;`
- **`[Layout(c_abi)] pub struct RawDispatchPacket : IValue`** (`0b00`):
  - `pub gprs: array<u64, 8>; pub x8_sret: u64; pub vecs: array<f64, 16>; pub stack_bytes: slice<u8>; pub vec_reg_count: u8; pub ret_kind: u8;`
- **Public Signatures:**
  - `pub pure fn classify_call_frame(conv: str, params: slice<TypeLayout>, ret: ref<TypeLayout>, variadic_fixed_count: opt<usize>) -> AbiFramePlan;`
  - `pub fn invoke_dynamic_frame(fn_addr: ptr<void>, plan: ref<AbiFramePlan>, packet: var ref<RawDispatchPacket>);`
- **Dual-Mode Execution Parity:**
  1. **Native AOT (`llvm_emitter`, `c_emitter`, `asm_emitter`):** `AbiFramePlan` lowers directly to machine register moves + `call` / `bl` with `0` intermediate copies.
  2. **`PilVM` (`pilot run` & capability-permitted `comptime`):** `frame f(a, b)` (`f!&(a, b)`) populates `RawDispatchPacket` and invokes `pilot_rt_invoke_abi_frame` in `< 15 ns`!

#### 13.4 Edge Cases, Diagnostics & Verification Criteria
- **Critical `sret` Register Shift Edge Case:** Returning a `32B` struct by value on SysV x86_64 consumes `RDI` as the hidden `sret` pointer (shifting explicit arg 0 to `RSI`), whereas on ARM64 AAPCS64 it uses `X8` (keeping arg 0 in `X0`!)—tested explicitly across both architectures!
- **C Variadic Default Argument Promotions:** Arguments past `variadic_fixed_count` (`...`) automatically promote `f32` $\rightarrow$ `f64` and `i8`/`i16`/`bool` $\rightarrow$ `i32`/`u32`, setting `%al = vec_reg_count` on SysV x86_64.
- **Diagnostics:** `ABI001` (Symbol Not Found in Shared Library via `dlsym`/`GetProcAddress`), `ABI002` (Shared Library Failed to Load — prints full `dlerror()` / Win32 error code), `ABI003` (Unsupported Calling Convention for Target Architecture).
- **Acceptance Proof:** Call compiled C functions taking/returning `16B` `{ uint64_t, uint64_t }`, `32B` `sret` struct, `12B` ARM64 HFA `{ float x, y, z }`, 10 mixed int/double stack-spilled args, and variadic `snprintf` from *both* `PilVM` and native AOT $\rightarrow$ 100% bit-identical results and `RSP % 16 == 0` alignment verified!

---

### M14 — Perceus Foreign Resource & Scope Guard Synchronization (`0b10` & `0b11`) [STATUS: SIMULATED]

#### 14.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`shm.pilot` Simulates Lattice Bits as Untracked Integers):** [`stdlib/polyglot/shm.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/polyglot/shm.pilot) defines `ShmOwnedBuffer` and `ShmBorrowedView` as plain `0b00` structs with a dummy `pub lattice_bits: i64` field (`2` and `1`), meaning the compiler neither calls `munmap`/`free` on `ShmOwnedBuffer` drop nor enforces `ESCAPE001` lexical borrow lifetimes on `ShmBorrowedView`!
- **Root Bottleneck 2 (`pilot_rt_drop` Custom Destructor Hook Unused by Foreign Handles):** [`crates/pilot-rt/src/c_abi.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt/src/c_abi.rs) (line `92`) already implements `pilot_rt_drop(payload_ptr, dtor: Option<unsafe extern "C" fn(*mut c_void)>)` with atomic `AcqRel` reference counting, yet foreign pointers have no standard `0b10` `IObject` / `0b11` `IGuard` wrapper binding them into Perceus!

#### 14.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/std/polyglot/lifetime.pilot` | `pub pkg std.polyglot.lifetime;` | `~280L` | `ForeignHandle<T> : IObject` (`0b10`), `ForeignGuard<T> : IGuard` (`0b11`), `PinnedSlice<T> : IObject` (`0b10`), `ThreadAffineDropQueue`, and `BorrowCallbackScope : IGuard` (`0b11`). |
| **MODIFY** | `stdlib/std/polyglot/shm.pilot` | `pub pkg std.polyglot.shm;` | `~140L` | Upgrade `ShmBuffer<T> : IObject` (`0b10` with real `~self()` OS unmap/release) and `ShmView<T> : IReference` (`0b01` statically escape-checked by `selfhost/perceus/escape.pilot`). |
| **MODIFY** | `selfhost/perceus/escape.pilot` & `arc.pilot` | `Pilot.SelfHost.Perceus.*` | `+95L` | Emit foreign destructor function pointers into `PerceusDrop` ops in `PilIR` (`M6`) and enforce `ESCAPE004` on async/retained foreign pointer arguments. |

#### 14.3 Complete Schemas, Signatures & Four Cross-Language Memory-Safety Laws
- **Core Public Types (`std.polyglot.lifetime`):**
  - **`[Layout(c_abi)] pub struct ForeignHandle<T: IAnyValue> : IObject`** (`0b10`):
    - `priv raw_ptr: ptr<T>; priv dtor_fn: ptr<void>; priv owner_tid: u64;`
    - `pub static fn wrap(p: ptr<T>, dtor: ptr<void>) -> opt<ForeignHandle<T>>;` (returns `none` if `p == null`!)
    - `pub pure fn as_ptr(ref self) -> ptr<T>;`
    - `pub var fn take_raw(var ref self) -> ptr<T>;` (disarms destructor when transferring ownership back to C!)
    - `~self()` invokes `dtor_fn(raw_ptr)` on exact last reference—or enqueues onto `ThreadAffineDropQueue` if `owner_tid != 0 && current_tid() != owner_tid` (essential for single-threaded foreign runtimes like V8 isolates, OpenGL contexts, or COM apartments!).
  - **`[Layout(c_abi)] pub struct ForeignGuard<T: IAnyValue> : IGuard`** (`0b11`):
    - `priv state_token: T; priv release_fn: fn<T, void>;`
    - `~self()` guaranteed by `PilIR` `GuardExitLifo` to execute in strict reverse-declaration order at scope exit; statically forbidden from heap storage or return values (`ESCAPE001`).
  - **`[Layout(c_abi)] pub struct PinnedSlice<T: IValue> : IObject`** (`0b10`):
    - Heap-pinned, Perceus refcounted buffer whose address never moves on reallocation; safe for `[Retained]` foreign async I/O (`io_uring`, `libuv`, DMA).
- **Diagnostics:** `ESCAPE004` (Ephemeral `0b01` Borrow Passed to `[Retained]` Async Foreign Parameter — Use `PinnedSlice<T>` or `iso`), `FFI001` (Null Destructor Registered for Non-Trivial `ForeignHandle`).

#### 14.4 Verification & 3-Gate Acceptance Criteria
1. **Zero-Leak Stress Test:** Allocate and clone 100,000 `ForeignHandle` wrappers around a C atomic-counter resource across branching loops, `take_raw()` transfers, and early `?` error exits $\rightarrow$ assert `live_foreign_allocations == 0` at scope exit!
2. **Thread-Affine Drop Test:** Create a thread-affine `ForeignHandle` on Thread A, send it across `std.chan` to Thread B where it drops $\rightarrow$ verify `dtor_fn` executes safely on Thread A's drain hook with zero thread-affinity violations.
3. **Compile-Time Use-After-Free Rejection:** Attempt to return a `ShmView<f64>` (`0b01`) from a function owning a local `ShmBuffer<f64>` (`0b10`), or pass a stack slice to a `[Retained]` foreign async call $\rightarrow$ compiler rejects with `ESCAPE001` / `ESCAPE004`.

---

### M15 — Native C++ Class, RAII, Vtable & Exception Boundary Adapter (`use cpp`) [STATUS: SIMULATED]

#### 15.1 Objective & Current Codebase Audit
- **Root Bottleneck:** High-performance ecosystems (LLVM, Vulkan-Hpp, RocksDB, Physics engines, audio DSP, ONNX Runtime) expose C++ classes, namespaces, overloaded methods, references `T&`, virtual tables, RAII destructors, and C++ exceptions—none of which C11 `use c` (`M12`) alone can call without manual `extern "C"` C++ wrapper files!

#### 15.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/adapter/cpp_mangle.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.CppMangle;` | `~360L` | Bidirectional **Itanium C++ ABI** (`_ZN...E`, substitution compression `S_`, `St`, const/ref qualifiers, operator encodings) & **Microsoft Visual C++ x64** (`?name@ns@@...`) mangler/demangler in 100% pure Pilot! |
| **CREATE** | `selfhost/adapter/cpp_abi.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.CppAbi;` | `~440L` | C++ class layout engine: `vptr` primary/secondary vtable slot layout, Itanium tail-padding base subobject packing, non-trivial C++ Pass-By-Invisible-Reference (`sret`/copy-ctor ABI rule!), and overloaded method resolution onto Pilot `MethodTable`. |
| **CREATE** | `selfhost/adapter/cpp_parse.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.CppParse;` | `~490L` | Extends `c_parser.pilot` (`M12`) with `namespace`, `class`/`struct` access specifiers (`public:`/`protected:`/`private:`), constructors/destructors `~T()`, `virtual` / `= 0` pure virtuals, `const` methods, `noexcept`, `enum class`, and concrete `std::string_view` / `std::span<T>` layout adapters. |
| **CREATE** | `stdlib/std/polyglot/cpp.pilot` | `pub pkg std.polyglot.cpp;` | `~180L` | `CppException : IValue` (`what: str`, `type_name: str`), `CppStringView`, `CppSpan<T>`, and safe exception-catching boundary wrapper. |

#### 15.3 Schemas, Public Signatures & Key C++ Hardware Invariants
- **Schemas & Public Signatures:**
  - `pub pure fn mangle_itanium(ns_path: slice<str>, fn_name: str, is_const_method: bool, is_ctor_dtor: u8, param_types: slice<CppType>) -> str;`
  - `pub pure fn mangle_msvc_x64(ns_path: slice<str>, fn_name: str, access: u8, is_virtual: bool, param_types: slice<CppType>, ret_type: ref<CppType>) -> str;`
  - `pub pure fn compute_cpp_class_layout(cls: ref<CppClassDecl>, target: ref<TargetDescriptor>) -> CppClassLayout;` (computes `nvsize`, `dsize` tail-padding reuse for derived classes, `vptr_offset = 0`, and `AbiPassMode::HiddenRef` vs `AbiPassMode::TrivialRegisters`).
- **Four Critical C++ Correctness Laws:**
  1. **The Invisible Non-Trivial Struct ABI Rule:** Even an 8-byte C++ struct with a user-defined `~T()` or copy/move constructor is **forbidden** by Itanium & MSVC ABIs from passing in `RDI`/`RAX`—`cpp_abi.pilot` automatically forces `ArgPassKind::IndirectByRef` (`M13`), preventing silent register corruption!
  2. **Virtual Method Dispatch & Pilot Implementation of C++ Pure-Virtual Interfaces:** Loads `vptr = *(self_ptr as ptr<ptr<ptr<void>>>)` and emits `CallIndirect(vptr[slot_idx])` (handling Itanium's **2-slot complete-object vs. deleting-destructor vtable entry** for `virtual ~Base()`!). Pilot structs implementing a C++ interface synthesize a static `#[repr(C)]` vtable global at `comptime`.
  3. **Zero-Crash C++ Exception Boundary:** `noexcept` C++ methods map to `-> T`; throwing C++ methods route through a catch landing-pad converting `std::exception` (`__cxa_begin_catch` / `__cxa_current_exception_type`) into `res<T, CppException>` (`T ! CppException`)—never unwinding raw C++ exceptions across Perceus frames!
  4. **Zero-Copy Views:** Pilot `slice<T>` (`0b01`) $\leftrightarrow$ `std::span<const T>` and Pilot `str` (`0b01`) $\leftrightarrow$ `std::string_view` (`{ ptr, size }` on libc++/libstdc++/MSVC STL) with `0` allocations.

#### 15.4 Diagnostic Codes & Verification Criteria
- `CPP001` (Deleted `= delete` Copy/Method Invoked), `CPP002` (Private/Protected C++ Member Inaccessible), `CPP003` (Uninstantiated C++ Template Requires Concrete Type Arguments — full arbitrary template instantiation arrives in `M61`), `CPP004` (Ambiguous C++ Overload Call), `CPP005` (Abstract C++ Class With Pure Virtual `= 0` Cannot Be Value-Instantiated), `CPP006` (Multiple Virtual Diamond Inheritance Offset Thunk Adjusted).
- **Acceptance Proof:** Compile a real C++17 shared library containing namespaces, overloaded methods, a non-trivial RAII class with `~Tracker()`, a `virtual` interface callback with `virtual ~ICallback()`, `std::string_view`, and `throw std::runtime_error("boom")`; import via `use cpp "tracker.hpp"` in Pilot $\rightarrow$ verify exact mangling, zero-leak `~Tracker()`, virtual callback dispatch, and clean Pilot `catch err => ...` recovery!

---

### M16 — Native Rust Crate & Zero-Copy Slice/Result/Trait Adapter (`use rust`) [STATUS: SIMULATED]

#### 16.1 Objective & Current Codebase Audit
- **Root Bottleneck:** Consuming external Rust crates today requires hand-writing `#[no_mangle] pub extern "C"` wrapper crates because `repr(Rust)` field order is unspecified, Rust `&str`/`&[T]` are 2-word fat pointers, `Option<&T>` uses non-null niche optimization, and an uncaught Rust `panic!` unwinding across an FFI boundary triggers immediate undefined behavior (process abort).

#### 16.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/adapter/rust_sig.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.RustSig;` | `~380L` | Pure-Pilot parser for Rust public items (`pub struct`, `pub enum`, `pub fn`, `pub trait`, `impl`, `&[T]`, `&mut [T]`, `&str`, `Vec<T>`, `String`, `Option<T>`, `Result<T, E>`) and Cargo metadata JSON reader. |
| **CREATE** | `selfhost/adapter/rust_bridge.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.RustBridge;` | `~350L` | Direct `0 ns` binding for `#[repr(C)]` / `extern "C"` exports + automatic cached C-ABI bridge crate synthesizer (`.pilc-cache/rust_bridge/<hash>/`) for `repr(Rust)` items with `catch_unwind` safety! |
| **CREATE** | `stdlib/std/polyglot/rust.pilot` | `pub pkg std.polyglot.rust;` | `~170L` | `RustPanicErr : IValue` (`message: str`), zero-copy `RustVec<T> : IObject` (`0b10` calling Rust's allocator dealloc on `~self()`), `RustString : IObject`, and `DynTraitFatPtr`. |

#### 16.3 Exact Zero-Copy Lattice & Niche Mapping (`Rust` $\leftrightarrow$ `Pilot`)
| Rust Type | Memory Representation | Pilot Mapped Type & Lattice | Copy Cost |
| :--- | :--- | :--- | :--- |
| `&'a [T]` / `&'a str` | `{ ptr: NonNull<T>, len: usize }` (`16B` on 64b) | `slice<T>` / `str` (`0b01 : IReference`) | **`0` bytes (2-register pair!)** |
| `&'a mut [T]` / `&'a mut T` | Non-null exclusive pointer / fat pointer | `var slice<T>` / `var ref<T>` (`0b01` exclusive borrow) | **`0` bytes** |
| `Option<&'a T>` / `Option<NonNull<T>>` | `8B` nullable niche (`0 == None`) | `opt<ref<T>>` (`0b01` niche-packed `0 == none`) | **`0` bytes (bit-identical!)** |
| `Result<T, E>` | Discriminated tag + payload union | `res<T, E>` (`T ! E`) | **`0` heap allocs** |
| `Vec<T>` / `String` | `{ ptr, cap, len }` owned in Rust `GlobalAlloc` | `RustVec<T>` / `RustString` (`0b10 : IObject`, projects `.as_slice()` $\rightarrow$ `0b01` `slice<T>`, drops via Rust `Vec::from_raw_parts` in `~self()` so allocator heaps never cross-free!) | **`0` copy view; safe cross-allocator drop!** |
| `pub trait Foo` / `&dyn Foo` | `{ data_ptr, vtable_ptr }` (`16B` fat pointer) | `interface IFoo` + bidirectional vtable thunk | **`0` heap allocs** |

#### 16.4 Public Signatures, Diagnostics & Verification Criteria
- **Public Signatures:** `pub fn absorb_rust_crate(db: var ref<PilotDb>, crate_spec: str, features: slice<str>) -> res<RustCrateBinding, List<Diagnostic>>;`
- **Diagnostics:** `RST001` (Cross-Allocator Raw Free Prohibited — Use `RustVec`/`RustString` Destructor), `RST002` (Cargo Crate Resolution/Build Error), `RST003` (Lifetime-Bound Rust Return Borrow Must Tie to Input `ref` Parameter via `0b01`), `RST004` (Uncaught Rust Panic Converted to `RustPanicErr`).
- **Acceptance Proof:** Import a real Rust crate exposing `&str -> Vec<Token>`, `&mut [f64]` in-place sort, `Result<Config, ParseErr>`, and a panicking function (`panic!("index out of bounds: 99")`) $\rightarrow$ verify `0` byte copies on slices, safe `RustVec` drop on Rust's allocator, and clean `err(RustPanicErr)` recovery with zero process crash!

---

### M17 — In-Process Python C-API Adapter & Zero-Copy Tensor Views (`use py`) [STATUS: SIMULATED]

#### 17.1 Objective & Current Codebase Audit
- **Root Bottleneck (`bridge.pilot` Spawns a New `python3` Subprocess + JSON Copy Per Call):** [`stdlib/polyglot/bridge.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/polyglot/bridge.pilot) (lines `60`–`66`) currently runs `Process.spawn_eval("python3", "import json\nenv = json.loads(...)...")` on every invocation—incurring `~35 ms` OS process-spawn latency per call, losing Python object state between calls, and copying all arrays through JSON text!

#### 17.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/adapter/py_stub.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.PyStub;` | `~360L` | `comptime` parser for Python `.pyi` stub files & `inspect` signatures (`def`, `class`, `@property`, `*args`, `**kwargs`, type unions `X | None` $\rightarrow$ `opt<X>`, `numpy.typing.NDArray[np.float64]` $\rightarrow$ `stride<f64>`). |
| **CREATE** | `stdlib/std/polyglot/py_capi.pilot` | `pub pkg std.polyglot.py;` | `~460L` | Dynamic in-process loader for `libpython3` across Linux (`.so`), macOS (`.dylib` / `.framework`), and Windows (`python3*.dll` via Windows Registry / PEP 514) using **PEP 384 Stable ABI** (`Py_InitializeEx`, `PyImport_ImportModule`, `PyObject_Vectorcall`, `PyDict_SetItemString`, `PyErr_Fetch` + PEP 3118 `PyObject_GetBuffer` / `PyMemoryView_FromMemory`). |
| **CREATE** | `selfhost/ir/pass_gil_coalesce.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Ir.GilCoalesce;` | `~190L` | `PilIR` (`M6`) optimization pass merging adjacent `PyGilGuard` scopes within a basic block/loop and inserting `PyAllowThreadsGuard` around pure Pilot compute loops! |
| **MODIFY** | `stdlib/std/polyglot/bridge.pilot` | `pub pkg std.polyglot;` | `-35L` | **Delete** `polyglot_eval_json` Python subprocess string-concat hack; retain explicit `std.proc` (`M9`) only for genuine OS CLI commands. |

#### 17.3 Schemas, GIL Coalescing & Zero-Copy PEP 3118 Buffer Architecture
- **Core Schemas (`std.polyglot.py`):**
  - **`[Layout(c_abi)] pub struct PyBufferRaw : IValue`** (`0b00` exact CPython `Py_buffer` ABI struct): `pub buf: ptr<void>; pub obj: ptr<void>; pub len: isize; pub itemsize: isize; pub readonly: i32; pub ndim: i32; pub format: ptr<u8>; pub shape: ptr<isize>; pub strides: ptr<isize>; pub suboffsets: ptr<isize>; pub internal: ptr<void>;`
  - **`pub struct PyGilGuard : IGuard`** (`0b11`): wraps `PyGILState_STATE`; reentrant-safe when nested Pilot $\rightarrow$ Python $\rightarrow$ Pilot callbacks occur!
  - **`pub struct PyAllowThreadsGuard : IGuard`** (`0b11`): calls `PyEval_SaveThread()` on entry and `PyEval_RestoreThread()` on scope exit so multi-threaded Pilot SIMD/GPU kernels execute completely free of the Python GIL!
  - **`pub struct PyObj : IObject`** (`0b10`): wraps `PyObject*`; `PerceusDup` calls `Py_IncRef`, `~self()` acquires `PyGilGuard` and calls `Py_DecRef`.
  - **`pub struct PyBufferLease<T: IValue> : IGuard`** (`0b11`): holds pinned `PyBufferRaw`, projects `.view()` $\rightarrow$ `0b01` `stride<T>` (supporting non-contiguous sliced/transposed NumPy views via byte stride divided by `sizeof(T)`!), and calls `PyBuffer_Release` on scope exit.
  - **`[Layout(c_abi)] pub struct PyError : IValue`** (`0b00`): `pub exc_type: str; pub message: str; pub traceback: str;`
- **Keyword Arguments & Vectorcall Fast-Path:** Named Pilot call args (`model.predict(x, batch_size: 64, verbose: false)?`) compile to a pre-interned static `kwnames` Python `tuple` + stack `PyObject*[N]` vector passed to `PyObject_Vectorcall` (`< 60 ns` call overhead, `0` dict allocations!).
- **Diagnostics:** `PY001` (`libpython3` Shared Library Not Found in Venv/System), `PY002` (PEP 3118 Buffer `format` String Mismatch with Requested Pilot Element Type — e.g. `'f'` `f32` vs `'d'` `f64`), `PY003` (Read-Only Python Buffer Borrowed as Mutable `var slice<T>`), `PY004` (Sub-Byte Strided Python Buffer Not Directly Element-Addressable).

#### 17.4 Verification & 3-Gate Acceptance Criteria
1. **Zero-Copy Pointer Identity & Strided Slice Proof:** Allocate a 1,000,000-element `List<f64>` (`8 MB`) in Pilot; pass `var slice<f64>` to Python in-process (`memoryview` / NumPy) to mutate in place (`arr *= 2.5`), AND borrow a non-contiguous Python slice `arr[::2]` back into Pilot `stride<f64>` $\rightarrow$ assert **exact same virtual memory address** (`ptr_before == ptr_after`) and `< 0.5 ms` overhead!
2. **Zero `PyObject` Refcount Leak Test:** Run 50,000 calls creating dicts, kwargs calls, exceptions, and `PyBufferLease` guards $\rightarrow$ net `PyObject` refcount delta == `0`!

---

### M18 — In-Process JS/TypeScript Adapter (`use js` / `use ts` via Node-API & Embedded QuickJS) [STATUS: SIMULATED]

#### 18.1 Objective & Current Codebase Audit
- **Root Bottleneck (`bridge.pilot` Spawns Cold `node` Subprocess Per Call):** [`stdlib/polyglot/bridge.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/polyglot/bridge.pilot) (lines `67`–`70`) spawns a fresh `node` OS process per call (`~45 ms` V8 cold boot per expression!), cannot hold JS closures/objects, cannot share `Uint8Array`/`ArrayBuffer` zero-copy, and cannot await JS `Promise<T>`.

#### 18.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/adapter/ts_dts.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.TsDts;` | `~430L` | Pure-Pilot `.d.ts` TypeScript declaration absorber (`export function`, `interface`, `class`, `type` unions, optional `prop?: T` $\rightarrow$ `opt<T>`, `TypedArray`s $\rightarrow$ `slice<T>`, `bigint` $\rightarrow$ `i64`/`i128`, `Promise<T>` $\rightarrow$ `Task<res<T, JsError>>`). |
| **CREATE** | `stdlib/std/polyglot/js_napi.pilot` | `pub pkg std.polyglot.js;` | `~440L` | ABI-stable **Node-API v9 (`napi_*`)** host & addon adapter across Linux, macOS, and Windows supporting both `libnode`/Deno/Bun N-API hosting AND lightweight embedded QuickJS N-API fallback when Node is not installed! |
| **MODIFY** | `stdlib/std/polyglot/bridge.pilot` | `pub pkg std.polyglot;` | `-20L` | **Delete** `node` subprocess `with (env)` string-eval hack permanently. |

#### 18.3 Schemas, 64-Bit `BigInt` Fidelity, Zero-Copy `TypedArray` & `Promise` Model
- **Core Schemas & Edge-Case Protections (`std.polyglot.js`):**
  - **`NapiScopeGuard : IGuard`** (`0b11`): wraps `napi_handle_scope` preventing V8 handle table exhaustion inside tight loops.
  - **`JsRef<T> : IObject`** (`0b10`): wraps `napi_ref` bound via `M14` `ThreadAffineDropQueue` + `napi_threadsafe_function` so dropping a `JsRef` on a Pilot worker thread never touches a V8 `Isolate` off-thread (`JS002`)!
  - **IEEE-754 $2^{53}-1$ Integer Precision Protection (`JS003`):** JS `number` is an `f64` and silently corrupts `i64`/`u64` integers $> 9,007,199,254,740,991$! `js_napi.pilot` maps Pilot `i32`/`u32`/`f64` to JS `number`, while mapping Pilot `i64`/`u64`/`i128`/`u128` losslessly via `napi_create_bigint_int64` / `napi_get_value_bigint_words`!
  - **Zero-Copy `ArrayBuffer` / `TypedArray` (`0b01` `slice<T>`):** `napi_get_typedarray_info` projects `Uint8Array`/`Int32Array`/`Float64Array` directly onto `JsBufferLease<T> : IGuard` $\rightarrow$ `slice<T>` (handling `byte_offset` subarrays accurately!); `napi_create_external_arraybuffer` exposes Pilot `PinnedSlice<u8>` (`M14`) directly to JS with Perceus `pilot_rt_drop` as `finalize_cb`.
  - **Non-Blocking `Promise<T>` $\leftrightarrow$ Pilot `Task<T>`:** Attaches N-API `.then(onResolve, onReject)` callbacks waking a one-shot `std.chan` (`M9`) receiver without event-loop blocking.
- **Diagnostics:** `JS001` (`.d.ts` Parse or Unresolved Module Error), `JS002` (Cross-Isolate Synchronous Call Must Route via Threadsafe Channel), `JS003` (`i64` Mapped to JS `number` Without `bigint` Annotation Would Lose Precision), `JS004` (Detached `ArrayBuffer` Access Prevented).

#### 18.4 Verification & 3-Gate Acceptance Criteria
1. Import a `.d.ts`-typed JS module testing `i64.max` lossless `BigInt` round-tripping, `Float64Array` subarray (`offset > 0`) zero-copy mutation, async `Promise<User>` resolution, and thrown `new TypeError("bad input")` $\rightarrow$ 100% pass with `0` byte copies and `0` precision loss!
2. Drop 10,000 `JsRef` handles from a background OS worker thread $\rightarrow$ clean thread-marshalled release with zero V8/QuickJS assertions.

---

### Polyglot Architecture — Programming vs Transport Languages & Cross-Language Generative Metaprogramming [STATUS: NOT AUDITED]

#### P18.1 Objective & Architecture Taxonomy
- **Language Subsystem Separation:** Polyglot is partitioned into two clear architectural categories:
  - **Programming Languages (`std.polyglot.prog.*` / `std.polyglot.<lang>`):** Executable languages (C, C++, Rust, Python, JavaScript/TypeScript, WebAssembly, Zig, Java, .NET, Swift) driven by in-process execution, ABI register frames, zero-copy `0b01` memory sharing, and Perceus `0b10`/`0b11` resource handles and scope guards.
  - **Transport Languages (`std.polyglot.transport.*`):** Data interchange and structural markup languages (**JSON**, **YAML/YML**, **XML**, **TOML**) driven by dynamic AST/DOM trees, in-memory mutation, nested property and path querying, and serialization.
- **Bidirectional Data Manipulation:** Pilot code can seamlessly parse, inspect, mutate, traverse, and emit JSON, YAML, XML, and TOML data structures, converting between Pilot values and transport trees with full type safety.
- **Cross-Language Generative Metaprogramming Phased Roadmap:**
  - **Phase 1 [STATUS: NOT AUDITED]:** Bidirectional Transport & AST Representation in Ring 2 standard library (`std.polyglot.transport.json`, `std.polyglot.transport.yaml`, `std.polyglot.transport.xml`, `std.polyglot.transport.toml`, `std.polyglot.transport`).
  - **Phase 2:** Homoiconic quote and template synthesis (`quote { ... }` / `ast<T>`) emitting foreign language source text directly into transport documents and configuration files.
  - **Phase 3:** In-process foreign macro provider (`pilot-proc-macro` / Foreign Absorbers) allowing foreign compiler pipelines to invoke Pilot compile-time macro expansions natively via in-process FFI.
  - **Phase 4:** Universal embedded generative blocks (`pilot in rust`, `pilot in python`) where inline Pilot generative code compiles and expands directly into the host language's compilation pipeline.

#### P18.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/std/polyglot/transport/json.pilot` | `pub pkg std.polyglot.transport.json;` | `~395L` | Dynamic JSON DOM (`JsonValue`, `JsonPair`), object/array mutations, nested path query (`query_path`), `stringify`, and `json_parse`. |
| **CREATE** | `stdlib/std/polyglot/transport/yaml.pilot` | `pub pkg std.polyglot.transport.yaml;` | `~246L` | Dynamic YAML AST (`YamlNode`, `YamlPair`), sequence/mapping operations, `emit`, and `yaml_parse`. |
| **CREATE** | `stdlib/std/polyglot/transport/xml.pilot` | `pub pkg std.polyglot.transport.xml;` | `~290L` | Dynamic XML DOM (`XmlNode`, `XmlAttr`), attribute getters/setters, child search (`find_first`, `find_all`), `emit`, and `xml_parse`. |
| **CREATE** | `stdlib/std/polyglot/transport/toml.pilot` | `pub pkg std.polyglot.transport.toml;` | `~327L` | Dynamic TOML AST (`TomlValue`, `TomlKeyVal`), table/array mutations, `emit`, and `toml_parse`. |
| **CREATE** | `stdlib/std/polyglot/transport/mod.pilot` | `pub pkg std.polyglot.transport;` | `~58L` | Polyglot taxonomy facade and classification functions (`transport_format_from_str`, `transport_is_data_language`, `transport_is_programming_language`, `polyglot_subsystem_category`). |
| **CREATE** | `crates/pilot-analysis/tests/polyglot_transport_and_generative_embedding.rs` | Test Suite | `~256L` | Full integration test suite validating bidirectional manipulation across all 4 formats and taxonomy duality. |

#### P18.3 Verification & 3-Gate Acceptance Criteria
1. JSON bidirectional DOM creation, array indexing, nested path query (`server.host`), dynamic mutation/deletion, and serialization pass 100%.
2. YAML bidirectional mapping, sequence manipulation, formatted emission, and parsing pass 100%.
3. XML bidirectional DOM element hierarchy, attributes (`get_attr`/`set_attr`), child navigation (`find_first`/`find_all`), emission, and parsing pass 100%.
4. TOML AST table lookups, typed value inspection, emission, and language taxonomy classification (`transport` vs `programming`) pass 100%.
5. All 85 simultaneous milestone gate tests remain green with zero regressions.

---

### M19 — WebAssembly Component Model & WIT Universal Adapter (`use wasm`) [STATUS: SIMULATED]

#### 19.1 Objective & Current Codebase Audit
- **Root Bottleneck:** [`selfhost/codegen/wasm_emitter.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/codegen/wasm_emitter.pilot) only emits low-level numeric WASM 1.0 modules (`i32`/`i64`/`f32`/`f64` exports). It cannot consume `.wasm` libraries/plugins inside native Pilot apps (`use wasm`) nor emit/consume high-level **WASM Component Model (`.wit`)** records, strings, slices, variants, results, and owned/borrowed resources!

#### 19.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/adapter/wit_parser.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.WitParser;` | `~340L` | Complete WIT (`*.wit`) & binary component custom-section decoder (`package`, `world`, `interface`, `record`, `variant`, `enum`, `flags`, `option`, `result`, `list`, `tuple`, `resource`, `own<R>`, `borrow<R>`). |
| **CREATE** | `selfhost/adapter/wit_abi.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.WitAbi;` | `~390L` | Canonical ABI flattener (`MAX_FLAT_PARAMS = 16`, `MAX_FLAT_RESULTS = 1`, `cabi_realloc` string/list linear-memory lifter & lowerer, `cabi_post_*` cleanup, UTF-8 $\leftrightarrow$ UTF-16 transcoding, and resource handle table). |
| **CREATE** | `selfhost/adapter/wasm_runtime.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.WasmRuntime;` | `~490L` | Deterministic, capability-sandboxed in-process WASM 2.0 interpreter + `PilIR` AOT lifter (`use wasm "plugin.wasm"` compiles WASM bytecode straight through `PilIR` `M6` to native machine code with bounds-checked linear memory!). |
| **MODIFY** | `selfhost/codegen/wasm_emitter.pilot` | `Pilot.SelfHost.Codegen.Wasm` | `+160L` | Add `--target wasm32-wasip2` Component Model output + automatic `.wit` generation from `pub pkg` exports! |

#### 19.3 Exact Isomorphism Between WIT Resources & Pilot's 2-Bit Hardware Lattice
- WIT `record` / `enum` / `flags` / scalar `tuple` $\longleftrightarrow$ `0b00 : IValue`
- WIT `borrow<R>` & incoming `list<T>` / `string` view $\longleftrightarrow$ `0b01 : IReference` (statically prevented from escaping the component call by Perceus `ESCAPE001`, and invalidated if `memory.grow` relocates linear memory without re-slicing!)
- WIT `own<R>` $\longleftrightarrow$ `0b10 : IObject` (`~self()` automatically invokes `[resource-drop]R` in the WASM component handle table!)
- `WasmInstanceLease` $\longleftrightarrow$ `0b11 : IGuard` (enforces `fuel_limit`, `max_memory_pages`, and WASI capability virtual-dir mounts).
- **Diagnostics:** `WASM001` (Invalid Binary WASM Magic/Version or Malformed Section), `WASM002` (WIT Signature Mismatch with Component Import/Export), `WASM003` (Linear Memory Out-of-Bounds Trap), `WASM004` (Invalid UTF-8 String Across Canonical ABI), `WASM005` (`borrow<R>` Handle Still Active at Call Exit Violation).

#### 19.4 Verification & 3-Gate Acceptance Criteria
1. **Round-Trip Polyglot Component Test:** Compile Pilot package `PkgA` (exceeding `MAX_FLAT_PARAMS = 16` to exercise Canonical ABI indirect memory spill + `cabi_post_*` cleanup!) to `pkg_a.wasm` (`wasm32-wasip2`), then import `use wasm "./pkg_a.wasm" as a;` in `AppB` passing strings, `List<Record>`, `res<T, E>`, `own<Session>`, and `borrow<Session>` $\rightarrow$ bit-identical execution in both interpreter and `PilIR`-AOT modes!
2. **Sandbox Memory & Fuel Isolation Test:** Load an untrusted `.wasm` module attempting out-of-bounds memory read, unpermitted filesystem access, and infinite loop $\rightarrow$ cleanly returns `err(WasmTrap)` with zero host corruption.

---

### M20 — Live Schema & Wire Protocol Absorber (`use sql`, `use openapi`, `use proto`) [STATUS: LIBRARY ONLY]

#### 20.1 Objective & Current Codebase Audit
- **Root Bottleneck:** [`stdlib/db/sql_engine.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/db/sql_engine.pilot), [`postgres_wire.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/db/postgres_wire.pilot), [`mysql_wire.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/db/mysql_wire.pilot), [`stdlib/http/http.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/http/http.pilot), and [`stdlib/json/json.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/json/json.pilot) already implement real wire protocols in pure Pilot—yet developers still have to hand-write duplicate Pilot structs, JSON marshalers, HTTP route glue, and SQL column decoders whenever interacting with an external database, REST API, or gRPC/Protobuf schema!

#### 20.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/adapter/absorb_sql.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.AbsorbSql;` | `~390L` | `comptime` DDL & annotated-query absorber (`CREATE TABLE`, `ALTER TABLE`, `PRIMARY KEY`, `FOREIGN KEY`, `NOT NULL` vs nullable `opt<T>`, Postgres `$1` / MySQL `?` / SQLite `?1` parameter dialects, and `-- name: FindUsers :many` typed query synthesis). |
| **CREATE** | `selfhost/adapter/absorb_openapi.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.AbsorbOpenApi;` | `~440L` | `comptime` OpenAPI 3.0/3.1 JSON & YAML absorber (`components/schemas`, `$ref` cycle-aware DAG resolver, `allOf` $\rightarrow$ `*Bundle` splice!, `oneOf`/`discriminator` $\rightarrow$ tagged union, typed `std.http` client & server `interface` generator). |
| **CREATE** | `selfhost/adapter/absorb_proto.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Adapter.AbsorbProto;` | `~450L` | `comptime` Protobuf `.proto` v2/v3 parser (`syntax = "proto3"`, `import`, `message`, `oneof`, `repeated` packed/unpacked, `map<K, V>`, `enum`, `service rpc`, `sint32`/`sint64` ZigZag varints) + zero-copy `0b01` wire encoder & decoder weaver! |

#### 20.3 Schemas, Zero-Boilerplate Synthesis & Zero-Copy Wire Invariants
1. **`use sql "./migrations/" as db_schema;` (Multi-Migration Replay at `comptime`!):**
   - Replays sequential `001_init.sql`, `002_add_idx.sql` `CREATE`/`ALTER TABLE` statements in deterministic order at `comptime` to compute the exact current table catalog!
   - Synthesizes `0b00`/`0b10` row structs, `Table<TRow>` LINQ metadata, and binary wire decoders (`Postgres` network-endian binary format & `MySQL` `COM_STMT_EXECUTE` bitmap nulls). Changing a column from `NOT NULL` to `NULL` immediately flags unhandled call sites with `TYPE001` (`opt<str>` vs `str`) in `< 10 ms`!
2. **`use openapi "./api_spec.json" as api;`:**
   - Maps `allOf` schemas directly to Pilot `*Bundle` composition (`coalesce` on shared IDs), optional properties to `opt<T>`, and path parameters (`/users/{id}/orders`) to compile-time typed function arguments (`client.get_user_orders(id: user_id)?`).
3. **`use proto "./telemetry.proto" as tel;` (Dual Owned `0b10` + Zero-Copy `0b01` Wire Views):**
   - Synthesizes **both** `tel.MetricBatch` (`0b10 : IObject` mutable builder) and `tel.MetricBatchView` (`0b01 : IReference` zero-copy reader whose `str`, `slice<u8>`, and packed fixed-width scalar slices borrow **directly into the incoming network packet** with **`0` heap allocations**!), while preserving unrecognized wire tags in an optional `unknown_fields: slice<u8>` span for lossless proxy forwarding!

#### 20.4 Diagnostic Codes & Edge Cases
- `SCH001` (Unresolved OpenAPI `$ref` or `.proto` Import), `SCH002` (SQL Query Column/Parameter Type or Count Mismatch Against Replayed DDL Catalog), `SCH003` (Protobuf Reserved Field Number `19000..19999` or Duplicate Tag), `SCH004` (Recursive Self-Referential Schema Struct Automatically Indirected via `opt<Box<T>>` `0b10` so `0b00` Value Layout Never Loops Infinitely!), `SCH005` (Truncated Wire Varint or Malformed Length-Delimited Protobuf Packet).

#### 20.5 Phase 2 (`M11`–`M20`) Cumulative Exit Checkpoint
Before advancing to **Phase 3 (`M21` — Boilerplate Eradication & `.pchor` Choreographer v1)**:
- **Gate 1 (Adaptability & Multiplatform Parity):** All 10 foreign/schema adapters (`adapter`, `c`, `cpp`, `rust`, `py`, `js`/`ts`, `wasm`, `sql`, `openapi`, `proto`) register cleanly in open `PilotDb` absorber tables with zero new seed tokens and pass the Dual-Pass Universal Rename Litmus. Verified to compile, load, and execute natively across Linux, macOS, and Windows.
- **Gate 2 (Zero-Downside Safety):** Every foreign handle (`FILE*`, `sqlite3*`, C++ RAII, `RustVec`, `PyObj`, `JsRef`, WASM `own<R>`) is automatically freed by Perceus `0b10` `~self()` (including thread-affine queues across POSIX and Windows threads), every runtime scope (GIL, N-API `HandleScope`, `PyBufferLease`) is protected by `0b11` LIFO `IGuard`, and every C++ exception / Rust `panic!` / Python `Exception` / JS `Error` / WASM `trap` converts cleanly to `res<T, Err>` with **zero UB crashes and zero memory leaks**.
- **Gate 3 (Peak Interop Speed):** Native C/C++/Rust calls execute at `0 ns` register ABI speed; Python NumPy and JS `TypedArray` exchange megabyte buffers at **`0` byte-copy overhead** via `0b01` `slice<T>`/`stride<T>`; and Protobuf `View` decoding performs **`0` heap allocations**.

---

## Phase 3 (`M21` – `M30`): Boilerplate Eradication, `.pchor` Choreographer v1 & `comptime` (UNABRIDGED SPECIFICATION)

> **Unabridged Specification Contract:** Phase 3 (`M21`–`M30`) eliminates repetitive boilerplate across Pilot, replaces every placeholder metaprogramming stub with production compilers, launches the **`.pchor` Sublanguage & `SoA` Layout Choreographer**, enforces zero-byte compile-time typestates & sound `old()` contracts, empowers scoped user DSLs with live IDE sync, coaches developers across the 4-Tier learning curve, sandboxes `comptime` execution hermetically, and delivers self-generating `[Cli]`, `[Service]`, and `[Entity]` blueprints. Every milestone below specifies the complete engineering blueprint leaving only literal code bodies omitted.

---

### M21 — Standard `[...]` Boilerplate Weaver Suite & Structural Delegation [STATUS: SIMULATED]

#### 21.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Hardcoded Attribute Strings & $O(N^2)$ Scans in `attr_weaver.pilot`):** [`selfhost/attr_weaver.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/attr_weaver.pilot) (lines `63`–`76`) hardcodes `"IEcsRule"`, `"IReactiveAttribute"`, `"IFormulaAttribute"`, `"IInterceptor"`, `"IInheritedAttribute"`, `"Rule"`, `"Clamp"`, `"OnChange"`, `"Formula"`, `"TraceSpan"`, `"HardRealtime"`, `"Wcet"`, and `"ZeroRuntime"` inside closed `if/else` string comparisons—violating the Open-Registry Design law (`AGENTS.md §1`). Furthermore, `find_attr_struct_node` (lines `39`–`53`) linearly scans all `node_count` AST nodes for every attribute attachment ($O(N^2)$), and `weave_unit_attributes` (lines `201`–`210`) only validates pillar tags without executing structural type-synthesis weavers!
- **Root Bottleneck 2 (Stubbed Attribute Bodies in `meta.pilot` & Inert `[JsonObject]` in `vscode_gen.pilot`):** [`stdlib/core/meta.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/meta.pilot) leaves `FormulaAttr.evaluate` (`owner as TOut` on line `125`), `TraceSpanAttr.on_enter` (`=> 0` on line `134`), and `RuleAttr.apply` (`{}` on line `146`) as dummy stubs. Meanwhile, [`selfhost/ide/vscode_gen.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/ide/vscode_gen.pilot) (lines `34`–`74`) defines `[JsonObject]` and `[JsonProperty]` as inert markers while stubbing `dumps(obj)` as `return str(obj);` and `loads(s)` as `return s;`!
- **Root Bottleneck 3 (Manual Forwarding Boilerplate):** Wrapping an inner handle or stream currently forces developers to manually write 10–20 forwarding methods instead of 1-line zero-overhead structural delegation (`delegate IReader to .inner;`).

#### 21.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **MODIFY** | `selfhost/attr_weaver.pilot` | `pub lvl(0) pkg Pilot.SelfHost.AttrWeaver;` | `-75L / +210L` | Delete all hardcoded attribute-name `if/else` chains and $O(N^2)$ scans; install $O(1)$ `WeaverProtocolRegistry` indexed by base `IAttribute` sub-interface (`ITypeWeaverAttribute`, `IFieldWeaverAttribute`, `IInterceptor`, `IFormulaAttribute`, `IReactiveAttribute`, `IInheritedAttribute`, `IEcsRule`, `IPatchAttribute`) + deterministic `(weight DESC, attachment_index ASC)` topological scheduler. |
| **CREATE** | `stdlib/core/derive.pilot` | `pub lvl(0) pkg core.derive;` | `~380L` | Ring-1 freestanding (`0` OS imports) structural `comptime` weavers for `[Eq]`, `[Ord]`, `[Hash]`, `[Clone]`, `[Default]`, and `[Display]` (with `[SkipEq]`, `[SkipHash]`, `[Redact]` field modifiers). |
| **CREATE** | `stdlib/std/serde/derive_json.pilot` | `pub pkg std.serde.json;` | `~420L` | Zero-reflection streaming UTF-8 `[Json]` weaver supporting `NamingStyle` (`camelCase`, `snake_case`, `PascalCase`), `[JsonRename("...")]`, `[JsonSkip]`, `[JsonFlatten]`, `opt<T>` null/omission, and discriminated `enum`/union tags. |
| **CREATE** | `stdlib/std/serde/derive_bin.pilot` | `pub pkg std.serde.bin;` | `~310L` | Deterministic wire-safe `[Binary]` weaver supporting little/big-endian fixed scalars, LEB128 varints, length-prefixed slices, bounds-checked `0b01` slice readers, and zero-copy `0b00` pod fast-paths. |
| **CREATE** | `selfhost/types/delegation.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Types.Delegation;` | `~240L` | Compiles `delegate IContract to .member [except { m1 }];` and `delegate * to .member;` into inline Tier-2 `MethodTable` forwarding thunks with `0` vtable overhead. |
| **MODIFY** | `stdlib/core/meta.pilot` & `selfhost/ide/vscode_gen.pilot` | Core Meta & IDE Gen | `-30L / +45L` | Replace dummy `FormulaAttr`/`TraceSpanAttr`/`RuleAttr` bodies and replace `vscode_gen.pilot`'s fake `dumps`/`loads` stubs with real `[Json]` woven serializers. |

#### 21.3 Complete Data Structures, Registries & 2-Bit Lattice Physics
- **`WeaverTargetKind` (`bitflags : u16`, `0b00 : IValue`)**: `::Struct = 1`, `::Enum = 2`, `::Interface = 4`, `::Field = 8`, `::Property = 16`, `::Function = 32`, `::Parameter = 64`.
- **`pub interface ITypeWeaverAttribute : IAttribute`** (`0` runtime bytes):
  - `pub targets: WeaverTargetKind { get => WeaverTargetKind::Struct | WeaverTargetKind::Enum; }`
  - `pub weight: i32 { get => 100; }`
  - `pub comptime fn weave_type(ctx: var ref<WeaveContext>, decl: AstNodeView) -> res<List<AstModuleFragment>, Diagnostic>;`
- **`[Layout(c_abi)] pub struct DelegationSpec : IValue`** (`0b00`):
  - `pub owner_type_id: u32; pub contract_iface_id: u32` (`0` = wildcard `*`); `pub target_field_idx: u16; pub target_is_ref_or_ptr: bool; pub excluded_syms: slice<u32>;`
- **`[Layout(c_abi)] pub struct JsonDeriveConfig : IValue`** (`0b00`):
  - `pub case_style: u8; pub deny_unknown_fields: bool; pub omit_none_fields: bool; pub tag_field: str;`
- **2-Bit Lattice Enforcement Inside `[Clone]` & `[Binary]`:**
  1. **`0b00 : IValue` Structs:** `[Clone]` weaves a trivial register/stack value return (`*self` / `memcpy`), and `[Binary]` weaves a single bounds-checked endian-verified block read/write when padding bytes == `0`!
  2. **`0b10 : IObject` Structs:** `[Clone]` recursively invokes `.clone()` on every owned field so Perceus reference counts or deep buffers duplicate cleanly without aliasing bugs.
  3. **`0b01 : IReference` Views:** `[Clone]` copies the lightweight borrowed descriptor (`{ ptr, len }`) while preserving the lexical borrow lifetime constraint (`ESCAPE001`)!
  4. **`0b11 : IGuard` Scoped Guards:** Attaching `[Clone]` to any `0b11` `IGuard` (e.g., `MutexGuard`, `TxLease`) is **statically rejected at compile time** (`WEAVE003: Scoped 0b11 IGuard owns a unique LIFO exit effect and cannot implement ICloneable`)!

#### 21.4 End-to-End Algorithmic Weaving Pipeline & Complexity Bounds
1. **$O(1)$ Symbol-to-Protocol Indexing (`PilotDb` `AttrCatalogQuery`, $O(D)$ per module):**
   - Indexes every `struct *Attr : I...` declaration once per `PilotDb` file revision into an open hash table mapping `SymbolId -> AttrProtocolDescriptor` (supporting both `[Json]` short name and `[JsonAttr]` full name without linear AST scans).
2. **Deterministic Weight + Dependency Ordering ($O(K \log K)$ for $K$ attributes on a declaration):**
   - Sorts attached attributes by `(weight DESC, source_order ASC)`. Structural type weavers (`weight >= 100`, e.g., `[Default]`, `[Eq]`, `[Clone]`, `[Json]`) execute before method AOP interceptors (`weight: 50`, e.g., `[TraceSpan]`) so woven methods automatically participate in downstream inspection (`pilot inspect --weave`, `M7`)!
3. **Generic Bound Inference for Generic Structs:**
   - On `pub struct Pair<A, B> { pub first: A; pub second: B; }`, `[Eq, Hash, Json]` automatically synthesizes conditional conformance `impl<A: IEquatable<A>, B: IEquatable<B>> IEquatable<Pair<A, B>>` rather than failing when `Pair<FnPtr, i32>` is instantiated without calling `==`!
4. **Zero-Overhead Delegation Synthesis (`selfhost/types/delegation.pilot`):**
   - For `delegate IReader to .stream;`, enumerates all methods of `IReader` not overridden at Tier 1 on the enclosing `struct` and not listed in `except { ... }`, synthesizing force-inlined Tier-2 forwarding methods `pub fn read(var ref self, buf: var u8[]) -> usize ! IoErr => self.stream.read(buf);` (automatically propagating `ref self` vs `var ref self` vs `iso self` receiver mutability!).

#### 21.5 Edge Cases & Diagnostic Surface Contracts
- **Secret Masking (`[Redact]`):** Fields annotated `[Redact]` emit `"***REDACTED***"` in `[Display]` debug/log output (`std.log`) while still serializing in `[Json]` unless `[JsonSkip]` is also present—preventing accidental password/API-key leaks in logs!
- **IEEE-754 `NaN` & `-0.0` in `[Eq]` / `[Ord]` / `[Hash]`:** Normalizes `-0.0` to `+0.0` before hashing (`x == y => hash(x) == hash(y)` hash-equality coherence law!) and supports `[Ord(float: ::TotalOrder)]` via IEEE-754 sign-magnitude bit bijection.
- **Self-Referential / Cyclic Graphs:** Value cycles are already prevented by layout physics; `0b10` `List<Node>` / `opt<Box<Node>>` trees weave recursive traversal bounded by stack depth guard in `[Json]`/`[Binary]`.
- **Diagnostics:** `WEAVE001` (Attribute Attached to Incompatible Declaration Target), `WEAVE002` (Field Type Missing Required Protocol e.g. Field `ch: Channel<i32>` Does Not Implement `IEquatable` — Suggests `[SkipEq]`), `WEAVE003` (`0b11` `IGuard` Cannot Derive `[Clone]`/`[Default]`/`[Json]`), `WEAVE004` (Duplicate JSON Wire Field Name After Case Conversion), `WEAVE005` (`delegate` Target Field Does Not Implement Specified Interface), `WEAVE006` (Ambiguous Wildcard `delegate *` Across Two Fields Sharing Method Signature).

#### 21.6 Verification & 3-Gate Acceptance Criteria
1. Attach `[Eq, Ord, Hash, Clone, Default, Display, Json, Binary]` to a complex nested generic struct containing `opt<str>`, `List<Vec3>`, `Map<str, i64>`, a `[Redact] api_key: str` field, and an `enum` payload $\rightarrow$ verify round-trip JSON & Binary byte identity, redacted `Display` string, hash-equality law across 10,000 fuzzed instances, and `0` reflection lookups in `PilIR`!
2. Test `delegate IReadWrite to .socket except { flush };` with a custom Tier-1 `flush` override $\rightarrow$ verify Tier-1 wins over delegated Tier-2 and `pilot inspect --methods` displays exact delegation provenance.
3. Confirm `selfhost/attr_weaver.pilot` contains `0` hardcoded user-space attribute names (`pilot gate` passes 100%).

---

### M22 — Complete `*Bundle` & `*Facet` Algebraic Composition Engine [STATUS: LIBRARY ONLY]

#### 22.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Hardcoded 6-Field Ceiling & Zero Bundle Splicing in `analyzer.pilot`):** [`selfhost/analyzer.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/analyzer.pilot) (`infer_struct`, lines `163`–`182`) hardcodes local variables `f0, f1, f2, f3, f4, f5`—silently dropping field layouts beyond field #6!—contains unreachable duplicate lines `180`–`181`, and performs **zero** `*Bundle` field flattening or `*Facet` method injection!
- **Root Bottleneck 2 (Disconnected Collision Strings in `diagnostics.pilot`):** [`selfhost/diagnostics.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/diagnostics.pilot) (lines `50`–`140`) defines standalone string helper functions (`format_coalesce_field_type_mismatch`, `check_bundle_field_collision`) for unit tests, but they are not wired into real multi-bundle struct layout calculation, internal bundle method offset remapping, or incremental `PilotDb` (`M5`) method-table resolution!

#### 22.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/types/bundle_splice.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Types.BundleSplice;` | `~440L` | Arbitrary $N$-field layout flattener, transitive `*Bundle` DAG expander, `coalesce` slot unifier, `with` field/method aliaser, `without` projection filter, and `partial struct` zero-storage `*Facet` sealer. |
| **CREATE** | `selfhost/types/method_table.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Types.MethodTable;` | `~390L` | Deterministic 4-Tier `MethodTable` builder (`Tier 1` Struct/Partial $\succ$ `Tier 2` `*Bundle`/`*Facet`/`delegate` $\succ$ `Tier 3` Sub-Interface Default $\succ$ `Tier 4` Base Interface Default) + scoped `ext` / `use ext` overlay resolver. |
| **MODIFY** | `selfhost/analyzer.pilot` & `selfhost/diagnostics.pilot` | Self-Host Analyzer | `-95L / +40L` | Delete the 6-field `f0..f5` hack, dead lines `180`–`181`, and isolated string shims; route struct layout & member resolution through `bundle_splice.pilot` and `method_table.pilot`. |

#### 22.3 Complete Data Structures, 4-Tier Precedence & Horizontal Collision Physics
- **`MemberTier` (`enum : u8`, `0b00 : IValue`)**: `::Tier1ExplicitStruct = 1`, `::Tier2SplicedBundleOrFacet = 2`, `::Tier3SubInterfaceDefault = 3`, `::Tier4BaseInterfaceDefault = 4`, `::ScopedExtension = 5`.
- **`SpliceModifierKind` (`enum : u8`, `0b00 : IValue`)**: `::Coalesce`, `::AliasWith`, `::ExcludeWithout`.
- **`[Layout(c_abi)] pub struct PhysicalFieldSlot : IValue`** (`0b00`):
  - `pub canonical_sym: u32; pub layout_id: u32; pub byte_offset: u32; pub lattice_bits: u8; pub is_mutable: bool; pub is_hidden_backing: bool; pub origin_bundle_sym: u32;`
- **`pub struct BundleFieldRemapTable : IObject`** (`0b10`):
  - Maps `(bundle_type_id, bundle_local_field_idx) -> host_physical_field_idx` so methods defined inside a generic `*Bundle` (e.g., `SpatialHeaderBundle.translate(var ref self, dx: f32)`) compile cleanly against the host struct's **coalesced or aliased field offsets** with **zero runtime indirection**!
- **`[Layout(c_abi)] pub struct MethodTableEntry : IValue`** (`0b00`):
  - `pub method_sym: u32; pub tier: MemberTier; pub origin_type_id: u32; pub ir_fn_id: u32; pub field_remap_id: u32; pub clearance_lvl: i32; pub visibility: u8;`
- **The 2-Bit Lattice Bitwise-OR Law Across `*Bundle` Splices:**
  $$\text{Lattice}(\text{Struct}) = \text{ExplicitDtorBit} \;\lor\; \bigvee_{f \in \text{PhysicalFields}} \text{Lattice}(f)$$
  - Splicing a `*Bundle` containing a `0b01` `slice<u8>` into a `struct` declared `: IValue` (`0b00`) is **immediately caught** (`COMP005: Struct declared ': IValue' (0b00) splices bundle 'WindowBundle' containing borrowed 0b01 field 'data'`), preserving 1-cycle lattice truth!

#### 22.4 Step-by-Step Algorithmic Resolution & Subtlety Handling
1. **Cycle-Safe Transitive `*Bundle` Expansion ($O(B + F)$):**
   - Expands nested `*SubBundle` splices in declaration order using 3-color DFS (`COMP002` on recursive bundle cycle `A -> *B -> *A`).
2. **Horizontal Collision Algebra (`coalesce`, `with`, `without`):**
   - **`coalesce { id, version }`:** Unifies matching fields from two or more `*Bundle`s into **one shared `PhysicalFieldSlot`** if and only if their resolved `TypeLayout`, `lattice_bits`, and mutability match identically (`COMP003` on type/mutability mismatch). Both bundles' `BundleFieldRemapTable` entries point to that single physical byte offset!
   - **`with { flags as net_flags, reset as reset_net }`:** Exposes the field/method on the host `struct` under `net_flags` / `reset_net`, while rewriting internal field accesses inside that bundle's own methods via `BundleFieldRemapTable` to the `net_flags` physical slot!
   - **`without { clear_state, temp_ scratch }`:**
     - *Subtlety:* If a method is excluded via `without { clear_state }`, it is omitted from `MethodTable`. If a **field** is excluded via `without { scratch }`, `bundle_splice.pilot` checks whether any *retained* method of that bundle still reads/writes `.scratch`: if unused, `0` bytes are allocated; if referenced by a retained bundle method, `COMP006` guides the user to either exclude the dependent method too or alias `.scratch` private (`with { scratch as priv _scratch }`)—preventing dangling offset bugs!
3. **Layout-Sealed `partial struct` Enforcement (`COMP001`):**
   - Primary `struct Foo { ... }` seals physical size, alignment, field offsets, and 2-bit lattice bits. Any `partial struct Foo { *MyFacet; ... }` in the same or downstream extensions may splice unlimited **`0`-field `*Facet`s** (injecting Tier-2 methods & satisfying new `: IInterface` contracts!), but attempting to splice a `>0`-field `*Bundle` emits `COMP001`.
4. **Monomorphized Method Specialization vs. Generic Offset Thunks:**
   - When a spliced `*Bundle` method is called on concrete `HostStruct`, `PilIR` (`M6`) substitutes `BundleFieldRemapTable` constants directly into `FieldAddr` offsets at `comptime`—yielding identical machine instructions to hand-written Tier-1 struct methods (`0` overhead)!

#### 22.5 Diagnostic Surface & 3-Gate Acceptance Criteria
- **Diagnostics:** `COMP001` (Stateful `*Bundle` Spliced in Layout-Sealed `partial struct`), `COMP002` (Cyclic `*Bundle` Inclusion), `COMP003` (`coalesce` Field Type/Lattice/Mutability Mismatch), `COMP004` (Unresolved Tier-2 Field or Method Collision — prints exact `coalesce`/`with`/`without` `--fix` edits!), `COMP005` (Declared Lattice Contract Violated by Spliced Field), `COMP006` (`without` Excludes Field Referenced by Retained Bundle Method).
- **Acceptance Proof:** Construct a 24-field `Entity` struct (proving removal of the 6-field ceiling!) splicing 3 overlapping generic `*Bundle`s (`SpatialBundle<f32>`, `NetworkSyncBundle`, `AuditBundle`) using `coalesce { id }`, `with { flags as net_flags }`, `without { debug_reset }`, plus a separate `partial struct Entity { *InterpolateFacet; }` and a Tier-1 method override $\rightarrow$ verify exact byte offsets via `pilot inspect --layout`, verify `BundleA.method(var ref entity)` static delegation, and verify `0 ns` overhead vs. hand-flattened struct in `PilIR`!

---

### M23 — Zero-Allocation Pipeline Fusion, `each` Vectorization & Cascade Polish [STATUS: SIMULATED]

#### 23.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Toy String-Splicing Optimizer in `selfhost/perf/optimizer.pilot`):** [`selfhost/perf/optimizer.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/perf/optimizer.pilot) (lines `82`–`156`) operates by scanning raw source text for a single `if false` and a single `let k = a + b;` and string-concatenating `source_slice`! It cannot fuse loops, inline closures, or optimize real control flow.
- **Root Bottleneck 2 (`each` Receptor Unlowered & Chained Pipelines Allocate Temporary Buffers):** [`stdlib/contracts/contracts_syntax.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/contracts/contracts_syntax.pilot) (line `16`) binds `each` as receptor `23`, yet neither analyzer nor IR lowers `items.each.update(dt)` or `matrix.each.each * 2.0`. Similarly, chaining `items.filter(p).map(f).flat().take(10).fold(0, add)` without stream fusion allocates intermediate `List` buffers at every stage.
- **Root Bottleneck 3 (Cascade Single-Evaluation & Map Literal Pre-Sizing):** [`crates/pilot-parser/src/lib.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-parser/src/lib.rs) (line `896`) parses `..` and `?..`, and [`stdlib/core/syntax/expr_sugar.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/syntax/expr_sugar.pilot) registers `|>`, `<|`, `->`, and `->>`, but cascades need strict single-evaluation root temporaries in `PilIR` (`M6`), short-circuiting `opt<T>` propagation on `?..`, and single-allocation capacity reservation on `[k1: v1, k2: v2,]` target-typed literals.

#### 23.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/core/stream.pilot` | `pub lvl(0) pkg core.stream;` | `~340L` | Zero-allocation pull-cursor combinators (`0b01 : IReference` `FilterStream`, `MapStream`, `FlatStream`, `ZipStream`, `TakeStream`, `StepStream`, `ChunkStream`) + terminal `fold`/`sum`/`count`/`any`/`all`/`collect`. |
| **CREATE** | `selfhost/ir/pass_stream_fusion.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Ir.StreamFusion;` | `~410L` | `PilIR` (`M6`) deforestation & closure-inlining pass collapsing arbitrary combinator chains and pipe `|>` stages into a single tight SSA register loop with `0` heap allocations! |
| **CREATE** | `selfhost/ir/lower_each_cascade.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Ir.EachCascade;` | `~330L` | Lowers N-dimensional `each` member/operator broadcasts, `..` / `?..` fluid cascades, `->` / `->>` structural DTO projections, and exact-capacity `[k: v]` / `[elem]` collection literals into `PilIR`. |
| **DELETE** | `selfhost/perf/optimizer.pilot` | Legacy string-splice optimizer | `-156L` | Permanently delete toy source-string splicer (superseded by `PilIR` `pass_const_dce` in `M6` and `pass_stream_fusion` in `M23`). |

#### 23.3 Complete Lowering Semantics & Data Structures
1. **Pull-State-Machine Deforestation (`pass_stream_fusion.pilot`):**
   - Every combinator stage implements `pub interface IPullCursor<TItem: IAnyValue> : IReference { pub var fn next() -> opt<TItem>; pub pure fn size_hint() -> (usize, opt<usize>); }`.
   - `pass_stream_fusion.pilot` inlines nested `next()` calls + non-escaping C#-style `(x) => expr` closures across basic blocks, eliminating `opt<TItem>` tag packing/unpacking via scalar replacement of aggregates (`SROA`) so `slice.filter((x) => x > 0).map((x) => x * x).sum()` produces the **exact same 5-instruction machine loop** as a hand-written `while` index loop!
   - When terminated by `.collect()` with an exact upper `size_hint` (e.g. `.map()` or `each`), allocates `List<U>.with_capacity(exact_len)` **once** prior to loop entry (`0` reallocations!).
2. **`each` Broadcast Semantics (`lower_each_cascade.pilot`):**
   - **In-Place `var` Mutation vs. Value Projection:**
     - Statement position on mutable receiver (`particles.each.integrate(dt);` or `scores.each += 5;`): lowers to an in-place mutating loop (`0` allocations!).
     - Expression position (`let scaled = velocities.each * 0.5;` or `let names = users.each.name;`): allocates exact-length output (`array<U, N>` for fixed `array<T, N>`, `simd<U, N>` hardware lane op for `simd<T, N>`, `opt<U>` for `opt<T>`, `List<U>.with_capacity(len)` for `slice<T>`/`List<T>`).
     - Nested `each` (`grid.each.each.clamp(0.0, 1.0)`): fuses nested rank-K loops and marks contiguous inner stride-1 loops `Vectorizable` for `M34` SIMD!
3. **Fluid Cascades (`..` & `?..`) & DTO Projections (`->` / `->>`):**
   - `expr..m1(a)..prop = b..m2(c)` binds `var __root = expr;` **once** in `PilIR`, executes `m1`, setter `prop`, and `m2` against `__root`, and yields `__root` (moving ownership cleanly if `__root` is `0b10`!).
   - `opt_expr?..m1(a)..prop = b` evaluates `opt_expr` once; if `some(var __val)`, runs the full cascade block and wraps `some(__val)`, else branches directly to `none`—evaluating side-effecting argument `a` **zero times** when `opt_expr` is `none`!
   - `domain_val -> DtoType` (shallow structural field projection by matching field names & implicit conversions) and `list_val ->> List<DtoType>` (fused `each` projection) eliminate manual field-by-field DTO mapping functions (`FUSE003` if a required target field has no source match or default!).

#### 23.4 Diagnostics & 3-Gate Acceptance Criteria
- **Diagnostics:** `FUSE001` (`each` Applied to Non-Broadcastable Scalar Without `IEnumerable` or Lane Descriptor), `FUSE002` (In-Place Mutating `each` Method Invoked on Immutable `let` Collection), `FUSE003` (DTO Projection `->` Missing Source Field for Non-Defaulted Target Field), `FUSE004` (`?..` Null-Safe Cascade Applied to Non-Optional Receiver — `--fix` rewrites to `..`).
- **Acceptance Proof:**
  1. Benchmark `10_000_000`-element `slice<i64>` running `.filter((x) => (x & 1) == 0).map((x) => x * 3 + 1).fold(0, (a, b) => a + b)` vs. hand-written `while` loop $\rightarrow$ **0 heap bytes allocated** (`pilot gate` allocation counter == `0`) and runtime within `<= 1.02x` of hand-written C!
  2. Verify side-effect counter on `make_box()..add(1)..add(2)` invokes `make_box()` exactly **once**, and `(none as opt<Box>)?..add(side_effect())` invokes `side_effect()` **zero** times.
  3. Confirm `selfhost/perf/optimizer.pilot` is deleted from the repository.

---

### M24 — `.pchor` Sublanguage Choreographer Compiler (`blueprint`, `role`, `cue`, `route`) [STATUS: LIBRARY ONLY]

#### 24.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`blueprint`, `role`, `cue`, `route` Are Dummy Pass-Through Macros!):** [`stdlib/core/meta.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/meta.pilot) (lines `173`–`176`) defines `blueprint(d) => quote { $d }`, `role(d) => quote { $d }`, `cue(d) => quote { $d }`, and `route(e) => quote { $e }`—doing zero parsing, zero architectural graph construction, and zero transport lowering!
- **Root Bottleneck 2 (`verify_cue_payload` Never Invoked Across Transitive Fields):** While `meta.pilot` lines `161`–`169` sketch `verify_cue_payload<TPayload>`, nothing invokes it on `.pchor` files, nor does it inspect nested struct/tuple/enum payloads or verify role read/write storage isolation!

#### 24.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/chor/ast.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Chor.Ast;` | `~260L` | `.pchor` typed AST & topology graph model: `BlueprintDef`, `StorageBinding`, `RoleDef`, `CueDef`, `RouteEdge`, `TransportSpec`, and `ArchPolicyRule`. |
| **CREATE** | `selfhost/chor/parser.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Chor.Parser;` | `~380L` | Lossless CST/AST parser for standalone `.pchor` blueprint files and inline `blueprint` blocks (`storage`, `role`, `reads`, `writes`, `emits`, `receives`, `cue`, `route`, `law`). |
| **CREATE** | `selfhost/chor/verifier.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Chor.Verifier;` | `~420L` | Enforces the **2-Bit Cue Boundary Law (`CHOR001`)** recursively across type graphs, role storage `reads`/`writes` ACLs (`CHOR002`), acyclic `::DirectCall` routing (`CHOR004`), Real-Time transport lock-freedom (`CHOR005`), and dependency layer `law`s (`CHOR006`). |
| **CREATE** | `selfhost/chor/lower.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Chor.Lower;` | `~440L` | Compiles `.pchor` topologies into `PilIR` (`M6`) dispatchers (`::DirectCall`, cache-line padded `::RingBuffer`, `::WorkStealingPool`, or `::ShmIpc` zero-copy shared-memory channels) without modifying domain `.pilot` functions! |
| **CREATE** | `stdlib/pilot/chor.pilot` | `pub pkg pilot.chor;` | `~190L` | Ring-3 public introspection & diagram export API (`pilot chor check`, `pilot chor graph --mermaid`, `pilot inspect --chor`). |
| **MODIFY** | `stdlib/core/meta.pilot` | Core Meta | `-18L` | Delete placeholder identity macros `blueprint`, `role`, `cue`, `route` on lines `173`–`176`. |

#### 24.3 Complete `.pchor` Grammar, Data Structures & The 2-Bit Cue Boundary Law
- **`TransportKind` (`enum : u8`, `0b00 : IValue`)**:
  - `::DirectCall` (`0 ns` synchronous inline call when caller & callee share thread/affinity)
  - `::RingBuffer` (`cap: u32`, 64B cache-line padded false-sharing-free lock-free SPSC/MPSC queue; `0` heap allocs after init!)
  - `::WorkStealingPool` (`workers: u16`, M:N task submission via `std.chan` / `std.sync`)
  - `::ShmIpc` (`region_bytes: usize`, cross-process POSIX/Win32 shared memory via `M14` `ShmBuffer`).
- **`BackpressurePolicy` (`enum : u8`, `0b00 : IValue`)**: `::Block`, `::TryFail`, `::DropOldest`, `::CoalesceLatest`.
- **`[Layout(c_abi)] pub struct CueContract : IValue`** (`0b00`):
  - `pub cue_sym: u32; pub payload_layout_id: u32; pub payload_lattice: u8; pub requires_iso_move: bool; pub backpressure: BackpressurePolicy; pub priority: u8;`
- **`pub struct RoleContract : IObject`** (`0b10`):
  - `pub role_sym: u32; pub inherited_attrs: List<RawAttr>` (e.g. `[HardRealtime, Wcet(250us), noalloc]` automatically propagated via Pillar 6 to every domain function bound to this role!!); `pub read_storages: List<u32>; pub write_storages: List<u32>; pub emitted_cues: List<u32>; pub handled_cues: List<u32>;`
- **The 2-Bit Cue Boundary Law (`verifier.pilot` — Zero Data Races By Construction):**
  - Given `cue CollisionBatch(PayloadT)`:
    1. If `PayloadT` has `lattice_bits == 0b00` (`: IValue`): **Allowed** (copied by value or placed directly into `RingBuffer` slot).
    2. If `PayloadT` has `lattice_bits == 0b10` (`: IObject`): **Allowed IF AND ONLY IF** qualified `iso PayloadT`! At the `emit` site in `PilIR`, Perceus enforces uniqueness (`refcount == 1` or fresh allocation) and **consumes/moves** the binding so the sender role loses all access the instant the cue is emitted (`0` atomic refcount traffic, `0` shared mutable state!).
    3. If `PayloadT` (or any transitive field/variant inside `PayloadT`) has `Bit 0 == 1` (`0b01 : IReference` or `0b11 : IGuard`): **Statically Rejected (`CHOR001`)** with the exact field path (e.g. `CollisionBatch -> Pair.debug_label: str borrows stack/local memory; use owned StringBuffer or 0b00 SymbolId`)!

#### 24.4 Architectural Isolation & Zero-Touch Topology Refactors
1. **Role Storage Read/Write Enforcement (`CHOR002`):**
   - If `role Broadphase` declares `reads: ParticleTable.pos;` (and *not* `writes: ParticleTable.pos`), any domain function executing inside `Broadphase` that attempts `var ref` mutation on `ParticleTable.pos`—or even reads undeclared column `ParticleTable.secret_token`—fails at compile time with `CHOR002`!
2. **Architectural Dependency `law` Clauses (`CHOR006`):**
   - `.pchor` blueprints support declarative package/role firewall rules:
     `law NoDbInUi: role UiPresenter cannot use std.db.*, raw_sqlite.*;`
     `law PureDomain: pkg App.Domain.* must_be [noalloc] or cannot use std.net.*, std.fs.*;`
   - Verified in $O(E)$ over `ModuleGraph` (`M2`) during `pilot check` in `< 2 ms`!
3. **1-Line Topology Swap Proof:**
   - Switching `route Broadphase.CollisionBatch -> Solver.resolve` from `(transport: ::DirectCall)` to `(transport: ::RingBuffer(cap: 1024, backpressure: ::CoalesceLatest))` recompiles only the synthesized `.pchor` glue module in `PilotDb` (`M5`)—domain `Broadphase` and `Solver` `.pilot` files remain **100% untouched**!

#### 24.5 Diagnostics & 3-Gate Acceptance Criteria
- **Diagnostics:** `CHOR001` (2-Bit Cue Boundary Law Violation — `0b01`/`0b11` or Non-`iso` `0b10` Payload), `CHOR002` (Role Unauthorized Storage Read/Write Access), `CHOR003` (Emitted Cue Has Zero Active Routes / Unhandled Cue), `CHOR004` (Synchronous `::DirectCall` Route Cycle Would Deadlock/Overflow Stack), `CHOR005` (`[HardRealtime]` Role Routed Over Blocking Transport `::Block` — Require `::RingBuffer` + `::TryFail`/`::CoalesceLatest`), `CHOR006` (Architectural `law` Dependency Firewall Breach), `CHOR007` (Concurrent Write Race: Two Asynchronous Roles Both Declare `writes` to Same Unpartitioned Storage Field Without Epoch Barrier).
- **Acceptance Proof:** Build a 3-role pipeline (`Sensor -> Filter -> Logger`) in `.pchor`; verify `CHOR001` catches a nested `slice<u8>` inside a cue struct, verify `CHOR006` catches an illegal `std.fs` import inside `Filter`, and benchmark swapping `::DirectCall` $\leftrightarrow$ `::RingBuffer(4096)` passing `iso List<Sample>` with **zero domain file edits and zero Perceus leaks**!

---

### M25 — `.pchor` Storage & Memory Layout Choreography (`AoS` $\leftrightarrow$ `SoA`, `AoSoA` & Hot/Cold Split) [STATUS: SIMULATED]

#### 25.1 Objective & Current Codebase Audit
- **Root Bottleneck (Layout Refactoring Paralysis):** In conventional systems languages, developers start with clean object/struct domain code (`struct Particle { pub pos: Vec3; pub vel: Vec3; pub mass: f32; pub debug_tag: str; }`), only to discover later that iterating `pos += vel * dt` over `1_000_000` particles wastes `>60%` of CPU memory bandwidth fetching `mass` and `debug_tag` into L1 cache! Fixing this manually requires ripping apart `struct Particle` into parallel arrays (`pos_x: List<f32>`, `pos_y: List<f32>`, ...) and rewriting hundreds of call sites across the entire codebase. Pilot's `.pchor` choreographer solves this **declaratively in 1 line**.

#### 25.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/core/soa.pilot` | `pub lvl(0) pkg core.soa;` | `~360L` | Freestanding Ring-1 multi-column container primitives (`SoaTable<T>`, `AoSoaTable<T, Tile>`, `HotColdTable<T>`, `RowProxyRef<T> : IReference` `0b01`, `RowProxyMut<T> : IReference` `0b01`). |
| **CREATE** | `selfhost/chor/storage_plan.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Chor.StoragePlan;` | `~380L` | Computes physical column splits, 64-byte SIMD cache-line alignment padding, nested struct flattening (`Particle.pos.{x,y,z}` $\rightarrow$ 3 scalar `f32` columns!), bit-packed bool/enum planes, and Perceus per-column drop masks. |
| **CREATE** | `selfhost/ir/pass_soa_rewrite.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Ir.SoaRewrite;` | `~450L` | `PilIR` (`M6`) transformation pass rewriting `FieldAddr` / `Load` / `Store` on choreographed storage views from `AoS` record strides to `SoA` / `AoSoA` / `HotCold` column pointer arithmetic! |

#### 25.3 Physical Storage Geometries & `PilIR` Address Lowering Equations
Given domain `struct T` with leaf scalar/object fields $f_0, f_1, \dots, f_{m-1}$ of byte sizes $s_0, \dots, s_{m-1}$ and capacity $N$:

| `.pchor` Storage Strategy | Memory Allocation Geometry | `PilIR` Address of Element `i`, Field $f_k$ | Best Hardware Use-Case |
| :--- | :--- | :--- | :--- |
| `AoS<T, align: A>` | Single slab `N * sizeof(T)` aligned to `A` | $\text{base} + i \cdot \text{sizeof}(T) + \text{offset}(f_k)$ | Random single-entity point lookups (hash tables, trees) |
| `SoA<T, align: 64>` | Single allocation partitioned into $m$ 64B-aligned column planes `col_k` | $\text{col\_base}_k + i \cdot s_k$ | Streaming loops & GPU coalesced global memory loads (`100%` cache-line utilization!) |
| `AoSoA<T, tile: W>` ($W \in \{4, 8, 16\}$) | Blocks of $W$ elements where each block stores `f0[W], f1[W], ...` | $\text{base} + \lfloor i / W \rfloor \cdot \text{BlockBytes} + \text{tile\_off}(f_k) + (i \bmod W) \cdot s_k$ | AVX2 (`W=8`) / AVX-512 (`W=16`) SIMD + multi-field locality without TLB thrashing across 30 columns! |
| `HotCold<T, cold: { f_c... }>` | Primary dense `AoS` of hot fields + secondary `AoS` side-array of cold fields | Hot: $\text{hot\_base} + i \cdot S_{\text{hot}} + \text{off}_{\text{hot}}(f_k)$<br>Cold: $\text{cold\_base} + i \cdot S_{\text{cold}} + \text{off}_{\text{cold}}(f_k)$ | Keeps rare error/audit/debug fields out of L1 cache while preserving contiguous hot struct layout |
| `BitPacked<T>` | Bit-planes for `bool`, small `enum`, and `[Bits(k)]` integers ($k < 8$) | Word `(i * k) >> 6`, shift `(i * k) & 63`, mask `(1 << k) - 1` | Massive flag/voxel/categorical tables (`8x`–`64x` RAM reduction!) |

#### 25.4 Transparent Domain Ergonomics, Safety Invariants & Edge Cases
1. **Zero Domain Syntax Changes:**
   - Domain code writes `for var p in world.particles { p.pos.x += p.vel.x * dt; }` or `let snap: Particle = world.particles[idx];` (which gathers columns $f_0..f_{m-1}$ into a `0b00` `Particle` register value) and `world.particles[idx] = new_particle;` (which scatters fields into their respective columns!).
2. **Contiguous Field-Column Projection (`0b01` `slice<FieldT>`):**
   - Under `SoA<Particle, align: 64>`, `world.particles.col(.mass)` (or `.pchor` role projection `reads: ParticleTable.mass`) yields a **zero-copy 64-byte-aligned `0b01` `slice<f32>`**, directly passable to C/Fortran BLAS (`M12`), Python NumPy (`M17`), or GPU `[Kernel]`s (`M36`) at **`0` byte-copy cost**!
3. **Raw Pointer Escape Guard (`CHOR008`):**
   - Taking a raw contiguous struct pointer `ptr<Particle>` (`&p as ptr<Particle>`) to an element inside `SoA`, `AoSoA`, `HotCold`, or `BitPacked` storage is **statically rejected at compile time** (`CHOR008: Cannot form contiguous AoS 'ptr<Particle>' into SoA-choreographed storage 'ParticleTable'; borrow individual field or materialize 'Particle' by value`)—eliminating silent memory corruption!
4. **Perceus `0b10` Column-Selective Drop & Swap-Remove:**
   - When dropping a `SoA<Entity>` table of `1_000_000` rows where 7 columns are `0b00` (`f32`, `u64`) and 1 column (`tags: List<u32>`) is `0b10 : IObject`, Perceus skips all 7 `0b00` columns in $O(1)$ and **only** iterates the single `tags` column to run `~self()`, before freeing the single backing slab!

#### 25.5 Verification & 3-Gate Acceptance Criteria
1. **Identical Semantics Across All 5 Layouts:** Run the exact same `physics_step.pilot` domain module under 5 `.pchor` configurations (`AoS`, `SoA<align: 64>`, `AoSoA<tile: 8>`, `HotCold`, `BitPacked` flags) $\rightarrow$ assert **bit-for-bit identical numerical outputs** and `0` Perceus leaks!
2. **Cache-Line Speedup Benchmark (Gate 3):** On `1_000_000` 128-byte entities where the hot kernel updates 2 `f32` fields (`8B` of `128B`), verify `SoA<Entity, align: 64>` achieves **$\ge 3.5\times$ wall-clock speedup** over `AoS<Entity>` with **zero edits** to `physics_step.pilot`!
3. **Static Safety Check:** Verify attempting to pass `&world.particles[i]` as `ptr<Particle>` to a C function under `SoA` triggers `CHOR008` with a helpful fix hint.

---

### M26 — Compile-Time Typestates (`state`) & Design-by-Contract (`requires` / `ensures` / `old`) [STATUS: SIMULATED]

#### 26.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`state` Is an Unchecked Identity Macro Stub):** [`stdlib/core/meta.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/meta.pilot) (line `172`) defines `state(state_decl) => quote { $state_decl };`, performing zero state-transition tracking, zero branch-merge verification, and zero terminal-state obligation checks!
- **Root Bottleneck 2 (`old(expr)` in `contracts_syntax.pilot` Is Semantically Broken!):** [`stdlib/contracts/contracts_syntax.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/contracts/contracts_syntax.pilot) (line `27`) defines `old(expr) => quote { $expr };` while `ensures(cond)` (line `22`) expands to `defer { if !($cond) { panic(...) } }`! Because `defer` executes *after* the function body mutates `self` or parameters, `ensures(.balance == old(.balance) + amount)` evaluates the **post-mutation** `.balance` on both sides instead of snapshotting `old(.balance)` on entry! Furthermore, `ensures` has no binding for the function's return `result`, executes indiscriminately even when a function returns early via `?` `err(...)`, and `invariant` is not woven across `pub var fn` exits!

#### 26.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/types/typestate.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Types.Typestate;` | `~420L` | Zero-byte compile-time finite-state lattice solver over `PilIR` (`M6`) SSA blocks (`Type@State`, algebraic union states `Type@(S1 \| S2)`, linear `iso`/`var` transition tracking, and must-finish obligations). |
| **CREATE** | `selfhost/ir/pass_contracts.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Ir.Contracts;` | `~390L` | Sound `PilIR` weaver for `requires`, `ensures(res => ...)`, entry-hoisted `old(expr)` snapshots, struct `invariant`s, and `where` scalar/composite refinement types (`type Port = u16 where self >= 1`). |
| **MODIFY** | `stdlib/contracts/contracts_syntax.pilot` & `stdlib/core/meta.pilot` | Core & Contracts | `-12L / +35L` | Delete broken `old(expr) => quote { $expr }` macro and `state` identity macro; bind structured AST/PilIR contract descriptors (`ContractCheckMode`). |

#### 26.3 Zero-Byte Compile-Time Typestate Algebra (`selfhost/types/typestate.pilot`)
- **Core Schemas:**
  - **`[Layout(c_abi)] pub struct StateSetMask : IValue`** (`0b00`): `pub bits: u64;` (bitmask over declared `state S0, S1, ...` of a type; `0` runtime bytes in the target struct!).
  - **`[Layout(c_abi)] pub struct TransitionSig : IValue`** (`0b00`): `pub required_mask: StateSetMask; pub ok_next_mask: StateSetMask; pub err_next_mask: StateSetMask; pub is_terminal: bool;`
- **Typestate Flow-Typing & Branch-Merge Rules on `PilIR`:**
  1. **Zero-Byte Erasure:** `Socket@Disconnected` and `Socket@Connected` have the **exact same `LayoutId` and 0 extra bytes** at runtime—typestates exist strictly in the compile-time type lattice!
  2. **Fallible State Transitions (`T@Next ! Err`):** Calling `conn.connect(addr)` with signature `(var ref self: Socket@Disconnected, ...) -> void ! NetErr transitions(ok: Connected, err: Disconnected)`:
     - On the `?` / `ok` continuation block, `conn`'s state narrows strictly to `Connected`!
     - Inside `catch err => { ... }`, `conn`'s state is known to be `Disconnected`!
  3. **Join-Point Bitwise-OR & Switch Narrowing:** If `if cond` transitions `tx` to `Committed` in `then` and leaves `tx` in `Open` in `else`, the SSA merge block assigns `tx : Tx@(Committed | Open)`. Calling `tx.exec_sql(...)` (requiring strictly `Open`) emits `STATE001` (`Receiver may be in state 'Committed'; valid states: 'Open'`).
  4. **Linear Obligation Proof (`[MustReach(Closed | Committed)]` — `STATE004`):** If a non-terminal typestate binding reaches Perceus scope exit without transitioning to a declared terminal state, compilation fails with `STATE004` (`Typestate obligation unfulfilled: 'Tx@Open' dropped without reaching 'Committed' or 'RolledBack'`).

#### 26.4 Sound `PilIR` Design-by-Contract Lowering (`pass_contracts.pilot`)
1. **Precondition (`requires(P)`):** Woven at the very top of `EntryBlock` (and at caller call-sites when constant arguments allow compile-time proof or immediate `DBC001` compile error!).
2. **Entry-Hoisted `old(E)` Snapshot Law:**
   - Before any body instructions execute in `EntryBlock`, `pass_contracts.pilot` collects all distinct `old(E_k)` expressions appearing in `ensures` clauses:
     - Verifies each `E_k` is `pure` (`DBC003: Impure expression inside old(...) forbidden`).
     - Verifies `TypeOf(E_k)` is `0b00 : IValue` or implements `ICloneable` (`0b01` borrows of mutated storage would alias post-state! `DBC004`).
     - Emits `let __old_k = E_k.clone_or_copy();` right after `requires` checks in `EntryBlock`!
3. **`ok`-Path Postcondition (`ensures(result => Q)`):**
   - Woven immediately prior to every `Return(ret_val)` instruction:
     - If the function returns `res<T, E>` (`T ! E`), normal `ensures` runs **only when `ret_val` is `ok(v)`** (binding `result` to `v`!), so early `?` error returns do not falsely trip success postconditions! Optional `ensures_err(err => ...)` verifies failure guarantees.
4. **Struct `invariant { ... }` & `where` Refinement Types:**
   - Struct `invariant`s weave automatically at the `ok` exit of `self(...)` constructors and every `pub var fn` / property `set` method (suppressed during internal `priv` helper calls so temporary intermediate states inside a method remain ergonomic!).
   - `pub type NonEmptyStr = str where .len() > 0;` / `pub type Percentage = f64 where self >= 0.0 && self <= 100.0;` elides runtime checks whenever `PilIR` constant/interval range analysis proves satisfaction at `comptime` (`0 ns`), emitting `DBC001` at compile time if provably violated (`let p: Percentage = 150.0;` $\rightarrow$ compile error!).
5. **Configurable Contract Modes (`--contracts=all|boundary|proven`):**
   - `all` (default in `test`/`debug`), `boundary` (enforces on `pub`/`export` package boundaries in `release`), `proven` (elides checks proven by interval/SMT analysis in `M83`).

#### 26.5 Diagnostics & 3-Gate Acceptance Criteria
- **Diagnostics:** `STATE001` (Invalid Method Call for Current Typestate), `STATE002` (Ambiguous Merged Typestate Across Branches), `STATE003` (Typestate Reassigned Inside Loop Without Loop-Invariant State), `STATE004` (Unfulfilled Terminal Typestate Obligation on Drop), `STATE005` (Aliased Mutable Reference Cannot Undergo Typestate Transition), `DBC001` (Compile-Time Provable Precondition/Refinement Violation), `DBC002` (Impure Expression in Contract Clause), `DBC003` (Impure Expression Inside `old(...)`), `DBC004` (`old(...)` Captures Mutated `0b01` Borrow Without Value Snapshot), `DBC005` (Struct `invariant` Violated on Constructor/Mutator Exit).
- **Acceptance Proof:**
  1. Write `Account.deposit(var ref self, amt: i64) -> i64 ! BankErr requires(amt > 0) ensures(res => .balance == old(.balance) + amt && res == .balance)` $\rightarrow$ verify `old(.balance)` captures pre-mutation value accurately, `?` early error exits bypass `ensures`, and passing `-5` literal fails at **compile time** with `DBC001`!
  2. Verify `File@Closed` calling `.read()` fails at compile time with `STATE001`, forgetting `.close()` fails with `STATE004`, and `sizeof(File@Open) == sizeof(File)` (`0` extra bytes!).

---

### M27 — Scoped User Sublanguage Builder (`syntax` + Hygiene + Declarative IDE Sync) [STATUS: SIMULATED]

#### 27.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Hardcoded 110-Word Keyword Strings in `analyzer.pilot` & `cst_recovery.pilot`!):** [`selfhost/analyzer.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/analyzer.pilot) (lines `560`–`561`) and [`selfhost/cst/cst_recovery.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/cst/cst_recovery.pilot) (line `141`) both hardcode a giant static string of 110+ user-space keywords (`"else|while|until|for|...|blueprint|cue|role|route|unit|syntax|use|op|"`)—reserving words globally even when a domain module isn't imported, and violating the Open-Registry / 14-Token Name-Blind philosophy!
- **Root Bottleneck 2 (Global-Only Syntax Rules & Static IDE Grammar Generation):** Currently `syntax` macros in `stdlib/core/syntax/*.pilot` act globally rather than scoping cleanly to `use Pkg.Dsl;` or `use syntax Dsl { ... }` blocks, and `selfhost/ide/vscode_gen.pilot` only emits static offline JSON files instead of dynamically streaming custom user DSL semantic tokens, hover docs, and completions over `pilot lsp`!

#### 27.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/syntax/pattern_dsl.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Syntax.PatternDsl;` | `~440L` | Compiles declarative `pub syntax Name { ... }` pattern arms (`$x:ident`, `$e:expr`, `$t:type`, `$p:pat`, `$b:block`, `$lit:literal`, variadic `$($elem:expr),*` / `,+` / `?` with custom delimiters) into fast LL/Pratt parselets! |
| **CREATE** | `selfhost/syntax/hygiene_marks.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Syntax.Hygiene;` | `~320L` | Full **Scopes-As-Sets (Flatt)** hygienic alpha-renamer upgrading `stdlib/core/syntax/quote.pilot`: guarantees macro-introduced bindings never shadow caller locals and caller expressions resolve in caller lexical scope (with explicit `unhygienic($ident)` opt-in when anaphoric binding is desired!). |
| **CREATE** | `selfhost/ide/live_dsl_sync.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Ide.LiveDslSync;` | `~260L` | Automatically derives LSP `semanticTokens`, contextual keyword completions, hover signatures, and Wadler-Lindig `pilot fmt` formatting rules directly from active scoped `syntax` + `[KeywordDefAttr]` metadata in `PilotDb`! |
| **MODIFY** | `selfhost/analyzer.pilot` & `selfhost/cst/cst_recovery.pilot` | Analyzer & CST | `-35L / +45L` | **Delete** the hardcoded 110-word keyword strings on `analyzer.pilot:561` and `cst_recovery.pilot:141`; query `PilotDb`'s live `ActiveKeywordTable` (14 seed tokens + imported `[KeywordDefAttr]` & scoped `syntax` heads). |

#### 27.3 Complete Scoped `syntax` Architecture, Hygiene & Origin Spans
- **Core Data Structures:**
  - **`FragmentSpecKind` (`enum : u8`, `0b00 : IValue`)**: `::Ident`, `::Expr`, `::Stmt`, `::Type`, `::Pattern`, `::Block`, `::Literal`, `::TokenTree`, `::RepeatZeroOrMore`, `::RepeatOneOrMore`, `::Optional`.
  - **`[Layout(c_abi)] pub struct SyntaxOriginFrame : IValue`** (`0b00`):
    - `pub call_site_span: SourceSpan; pub def_site_span: SourceSpan; pub syntax_sym: u32; pub parent_expansion_id: u32;`
    - Stored in a side-table on `PilotDb` (`ExpansionTraceTable`) so diagnostics inside expanded `quote` templates point **both** to the exact user DSL argument token (`call_site_span`) AND show the macro rule note (`def_site_span`)!
  - **`[Layout(c_abi)] pub struct SyntaxScopeSet : IValue`** (`0b00`):
    - Interned bitset/slice of `HygieneMarkId(u32)` attached to every synthesized identifier; resolution matches binding $B$ for reference $R$ iff $\text{Scopes}(B) \subseteq \text{Scopes}(R)$ (maximal subset wins!).
- **Lexical Isolation Guarantee (Zero Keyword Pollution):**
  - Defining `pub syntax StateMachine { [KeywordDef(16, "Declares a state transition")] transition($from:ident -> $to:ident on $ev:Type => $body:block) => quote { ... }; }` in `Pkg.Fsm` **never** steals `transition` or `on` as keywords in other files!
  - A caller writes either file-scoped `use syntax Pkg.Fsm.StateMachine;` or block-scoped `use syntax StateMachine { transition(Idle -> Active on StartEvt => { ... }); }`—outside that block, `let transition = 42;` compiles with zero `@transition` escaping needed!

#### 27.4 Diagnostics & 3-Gate Acceptance Criteria
- **Diagnostics:** `SYN001` (Ambiguous Overlapping `syntax` Pattern Arms), `SYN002` (Direct/Indirect Left Recursion in Custom `syntax` Rule), `SYN003` (Macro Expansion Recursion Depth/Step Limit Exceeded — default `256` depth), `SYN004` (Mismatched Variadic Repetition Counts Between `$($a),*` and `$($b),*` in `quote` Splice), `SYN005` (Unbound Pattern Metavariable `$foo` in `quote` Body), `SYN006` (Hygiene Violation Audit Hint When Inspecting Captures).
- **Acceptance Proof:**
  1. Author 3 distinct user-space sublanguages in pure Pilot test packages—(a) a declarative UI tree DSL (`view { vstack(gap: 8) { button("Save") => on_save() } }`), (b) a finite-state-machine DSL, and (c) an SQL-like rule DSL—without touching a single compiler file!
  2. Verify local variable `let vstack = 10;` in the *same* file outside `use syntax UiDsl { ... }` compiles cleanly, hygienic local `let tmp = ...` inside the DSL macro never collides with caller `let tmp = "hello";`, and `pilot lsp` immediately highlights `vstack`/`button` with `[KeywordDef]` hover docs!
  3. Confirm `pilot gate` verifies `analyzer.pilot` and `cst_recovery.pilot` contain **zero** hardcoded non-seed keyword lists.

---

### M28 — 4-Tier Progressive Learning Profiles & Friendly Compiler Guidance [STATUS: LIBRARY ONLY]

#### 28.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (One-Size-Fits-All Systems Jargon in Diagnostics):** Currently [`selfhost/diagnostics/errors.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/diagnostics/errors.pilot) emits low-level compiler terminology (`0b01 IReference`, `LayoutTable`, `Tier 2 bundle`, `lvl(N)`) identically to a Day-1 scripting newcomer writing a 15-line `hello.pilot` and a Tier-4 kernel/compiler author editing `stdlib/core/`!
- **Root Bottleneck 2 (Ad-Hoc String-Parsed `lvl` Clearance in `analyzer.pilot`):** [`selfhost/analyzer.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/analyzer.pilot) (`get_node_clearance` & `get_caller_clearance`, lines `624`–`654`) re-walks 30 parent AST nodes per call parsing `"lvl"` substrings with `parse_num`, and lacks progressive **opt-in performance/architecture coaching** (`PerfCoach`) that rewards developers as they step up from Tier 1 $\rightarrow$ Tier 2 $\rightarrow$ Tier 3 $\rightarrow$ Tier 4!

#### 28.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/pilot/profile.pilot` | `pub pkg pilot.profile;` | `~210L` | Defines `LearningTier` (`::Tier1Everyday`, `::Tier2Domain`, `::Tier3Systems`, `::Tier4Meta`), `[Tier(...)]` / `#pragma tier` package descriptors, and typed `ClearanceRing` (`lvl(0)`..`lvl(N)`). |
| **CREATE** | `selfhost/diag/progressive_coach.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Diag.Coach;` | `~390L` | Adapts diagnostic phrasing, concrete `--fix` suggestions, and opt-in `PerfCoach` / `ArchCoach` hints to the active `LearningTier` without changing language semantics! |
| **CREATE** | `selfhost/types/clearance_flow.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Types.ClearanceFlow;` | `~250L` | Replaces `analyzer.pilot:624-654` string parsing with an $O(1)$-lookup flow-sensitive `lvl` clearance solver over `PilIR` (`if lvl >= 2 { ... }` branch elevation + audit trail). |

#### 28.3 The Non-Bifurcation Law & Complete 4-Tier Coaching Matrix
- **Non-Bifurcation Invariant:** *Every valid Pilot file compiles with identical runtime semantics regardless of `LearningTier`!* Tiers **never** split Pilot into incompatible dialects or disable safety (`ESCAPE001` and `NULL001` always hold!). Instead, `LearningTier` controls **ergonomic defaults, explanation depth, and proactive reward hints**:

| Active Tier | How Selected | Diagnostic Tone & Explanations | Proactive Coaching & Ergonomic Rewards |
| :--- | :--- | :--- | :--- |
| **Tier 1 (`everyday`)** | Default for standalone `.pilot` scripts & `pilot init` starter apps | Plain everyday English (`"Cannot return a slice of 'names' because 'names' only lives inside this function"` + 1-click `--fix`: `"return 'names' directly"`). Hides `0b00..0b11` bit codes unless `--explain` is passed. | Top-level script expressions / flexible `fn main()`, auto-formatted `println(val)` hints, gentle typo did-you-mean suggestions. |
| **Tier 2 (`domain`)** | `tier: "domain"` in `pkg.pilot` | Clean architectural terminology (`interface`, `*Facet`, `res<T, E>`, `opt<T>`, contracts). | `COACH001`: Suggests `[Eq, Json]` weaver when manual field-by-field `==` boilerplate is detected; `COACH002`: Suggests `adapter` or `->` projection on repetitive mapping funcs. |
| **Tier 3 (`systems`)** | `tier: "systems"` or `[PerfCoach]` / `[HardRealtime]` | Full hardware & lattice precision (`0b01` borrow lifetime, cache-line bytes, Perceus `drop_reuse`, SIMD lane width). | `PERF001`: Flags avoidable `0b10` `.clone()` in hot loops where `ref<T>` or `iso` achieves `0` allocs; `PERF002`: Reports struct field padding waste $\ge 8\text{B}$ (`sizeof` reorder hint saving RAM); `PERF003`: Highlights hot `AoS` loop eligible for `.pchor` `SoA<T>` speedup! |
| **Tier 4 (`meta`)** | `tier: "meta"` or `lvl(0..2)` packages | Full AST/CST/PilIR/ABI/Hygiene trace + expansion provenance frames (`M27`). | Enforces `[EnforceCoreNaming]`, `[NoHardcode]`, `RING001` freestanding purity, and `lvl` clearance ring boundaries (`LVL001`). |

#### 28.4 Verification & 3-Gate Acceptance Criteria
1. Compile the exact same borrow-escape error (`returning slice of local List<i64>`) under `--tier=1` vs. `--tier=3` $\rightarrow$ verify both reject with `ESCAPE001`, `--tier=1` emits jargon-free plain guidance + lossless `--fix`, and `--tier=3` includes exact `0b01` lexical owner span & Perceus drop point!
2. Enable `[PerfCoach]` on a struct `struct BadPad { a: u8; b: i64; c: u8; d: i64; }` (`32B` instead of `24B` / `16B`) and a loop cloning a `List<f64>` on every iteration $\rightarrow$ verify `PERF002` reports exact byte savings (`32B -> 24B`) and `PERF001` offers a lossless `ref<List<f64>>` CST fix!

---

### M29 — Capability-Sandboxed & Deterministic `comptime` Execution [STATUS: SIMULATED]

#### 29.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Unbounded, Unsandboxed `comptime` Execution in `vm.rs` & `comptime.rs`):** [`crates/pilot-vm/src/vm.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm/src/vm.rs) (`run()`, lines `48`–`145`) and [`crates/pilot-analysis/src/comptime.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/src/comptime.rs) run `comptime` code with **zero instruction fuel counter** (`while self.pc < code_len` hangs `pilot lsp` forever on `comptime { loop {} }`!), **zero heap memory ceiling**, and **unrestricted `EngineIntrinsic` dispatch**!
- **Root Bottleneck 2 (Non-Deterministic Build Cache Poisoning & Supply-Chain Risk):** `vm.rs` lines `104`–`105` iterate Rust `HashMap<String, PilotVal>` with randomized SipHash seeds, and nothing prevents a third-party dependency's `[...]` attribute weaver or `comptime` block from reading `~/.aws/credentials`, opening sockets, or reading `time.now()`—breaking reproducible builds and exposing developers to compile-time supply-chain attacks just by opening a repo in an IDE!

#### 29.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/comptime/guardian.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Comptime.Guardian;` | `~360L` | Enforces per-package `VmCapability` (`M10`) bitmasks, `fuel_limit`, `max_heap_bytes`, path-glob allowlists, and env-var dependency recording in `PilotDb` (`M5`). |
| **CREATE** | `selfhost/comptime/deterministic_env.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Comptime.DetEnv;` | `~240L` | Insertion-ordered deterministic map/set backing, virtualized monotonic `0`-epoch clock, content-seeded deterministic PRNG, and normalized `/` virtual paths across Linux/macOS/Windows. |
| **MODIFY** | `selfhost/pkg/manifest.pilot` | Package Manifest | `+95L` | Parse `[comptime.allow]` policy in root `pkg.pilot` (`read_globs`, `env_vars`, `ffi_targets`, `max_fuel`) and reject unapproved capability escalations requested by transitive dependencies! |
| **MODIFY** | `crates/pilot-vm/src/vm.rs` & `crates/pilot-analysis/src/comptime.rs` | Seed VM Host | `+32L` | Add deterministic `fuel_remaining: u64` decrement on back-edges/calls, replace `HashMap` in `heap_maps` with deterministic insertion-ordered slots, and gate `engine_handler` through capability mask! |

#### 29.3 Security & Reproducibility Laws (`CTIME001`–`CTIME006`)
1. **Pure-By-Default Zero-Trust Sandbox:**
   - Unless explicitly granted by the **root workspace `pkg.pilot`**, every `comptime` block, `[...]` weaver (`M21`), `syntax` macro (`M27`), and `.pchor` pass (`M24`) executes with `VmCapability::None` (`0` host FS, `0` net, `0` env, `0` subprocess, `0` FFI)!
2. **Transitive Dependency Privilege Escalation Block (`CTIME006`):**
   - If third-party package `evil_json_macro` declares `comptime_needs = ["fs.read", "net"]` in its own `pkg.pilot`, `pilot check` / `pilot build` **refuses to grant it** unless the user's **root** `pkg.pilot` explicitly trusts `[comptime.grants."evil_json_macro"] allow = [...]`!
3. **Hermetic File & Env Tracking in `PilotDb` (`M5`):**
   - Reading an allowed compile-time asset (`comptime embed_str("./queries/users.sql")`) checks `read_globs` (`./queries/*.sql` — blocking `../` path traversal `CTIME001`!) and registers `FileInput("./queries/users.sql", sha256)` on the active `PilotDb` query frame so editing `users.sql` triggers instant `< 10 ms` incremental recompilation!
4. **IDE Liveness Guarantee (`CTIME004`):**
   - Interactive `pilot lsp` caps per-invocation `comptime` fuel to `2_000_000` instructions (`~4 ms`) and batch `pilot build` defaults to `50_000_000` instructions (overridable via `[ComptimeFuel(N)]` up to workspace max). Infinite loops trap cleanly at the exact loop header `SourceSpan` without crashing or hanging the compiler!

#### 29.4 Diagnostics & 3-Gate Acceptance Criteria
- **Diagnostics:** `CTIME001` (Unauthorized `comptime` Filesystem Access or `..` Path Traversal Blocked), `CTIME002` (Unauthorized `comptime` Network/Subprocess/FFI Call Blocked), `CTIME003` (Non-Deterministic Host Clock/Entropy Blocked — Use Deterministic `comptime` Seed), `CTIME004` (`comptime` Instruction Fuel Exhausted — Possible Infinite Loop), `CTIME005` (`comptime` Sandbox Heap Quota Exceeded), `CTIME006` (Untrusted Dependency Requested Unapproved `comptime` Capability).
- **Acceptance Proof:**
  1. Test a malicious fixture macro attempting (a) `fs.read_text("/etc/passwd")`, (b) `net.connect("1.1.1.1:80")`, (c) `../../secret` path escape, and (d) `loop {}` inside `pilot check` & `pilot lsp` $\rightarrow$ all 4 trap deterministically in `< 5 ms` with `CTIME001`–`CTIME004` and `0` host side effects!
  2. Compile the entire Pilot workspace twice under shuffled environment variables, different system clocks, and different working directory paths $\rightarrow$ assert **100% bit-for-bit identical `.pilc` & native binary SHA-256 hashes**!

---

### M30 — Self-Generating Service, CLI & Persistence Blueprints (`[Cli]`, `[Service]`, `[Entity]`) [STATUS: SIMULATED]

#### 30.1 Objective & Current Codebase Audit
- **Root Bottleneck (Cross-Layer Application Boilerplate):** In typical ecosystems, adding a single field (`discount_bps: u16`) to an `Order` domain concept forces manual edits across 5 disconnected layers: CLI flags, HTTP route handlers, JSON request/response validators, OpenAPI YAML docs, and SQL `CREATE TABLE` / `INSERT` / `SELECT` column indices. With Phase 3's weavers (`M21`), `*Bundle`/`*Facet` composition (`M22`), `.pchor` choreography (`M24`), typestates/contracts (`M26`), and sandboxed `comptime` (`M29`) in place, Pilot can **self-generate** production-grade CLI, HTTP/RPC, and SQL persistence layers from **one clean Pilot source of truth** at `0` runtime reflection cost!

#### 30.2 Module & File Manifest (`<= 1,500` LOC Law — 100% Pure Pilot Ring 2 `stdlib/std/`)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/std/cli/weaver.pilot` | `pub pkg std.cli;` | `~380L` | `[Cli]` & `[Arg]` `ITypeWeaverAttribute` generating zero-alloc argv parsers, subcommands (from `enum` variants!), env-var fallbacks, `///` doc help screens, and shell completions. |
| **CREATE** | `stdlib/std/http/service_weaver.pilot` | `pub pkg std.http.service;` | `~460L` | `[Service]` + `[Get]`/`[Post]`/`[Put]`/`[Delete]`/`[Ws]` weaver generating static radix-tree HTTP/RPC routers, typed client SDK structs, `requires` $\rightarrow$ RFC 9457 `400/422` Problem JSON guards, and live OpenAPI 3.1 JSON specs! |
| **CREATE** | `stdlib/std/db/entity_weaver.pilot` | `pub pkg std.db.entity;` | `~470L` | `[Entity]` + `[Id]`/`[Unique]`/`[Index]`/`[ForeignKey]`/`[Version]` weaver generating compile-time typed SQL column lenses, parameterized Postgres/MySQL/SQLite CRUD & batch queries, and schema-diff DDL migrations! |

#### 30.3 Complete Self-Generation Blueprints & Zero-Overhead Lowering
1. **`[Cli]` Self-Generating Command-Line Apps (`std.cli`):**
   - Annotate any `0b00`/`0b10` `struct DeployArgs` or subcommand `enum Cmd` with `[Cli(name: "deploy", version: "1.0.0")]`:
     - Synthesizes `DeployArgs.parse_args(argv: slice<str>) -> res<DeployArgs, CliErr>` and `DeployArgs.help_text() -> str`.
     - Automatically enforces `M26` `where` refinement types (`port: Port` where `1..=65535` produces `"Invalid --port '70000': must satisfy self >= 1 && self <= 65535"` without writing a single `if` check!).
     - Plugs directly into `M3`'s `CommandRegistry` so user CLIs automatically gain `--completions bash|zsh|fish|pwsh` and MCP tool schema export!
2. **`[Service]` Self-Generating HTTP/RPC & OpenAPI 3.1 (`std.http.service`):**
   - Annotate a clean domain `interface` or `struct` with `[Service(base: "/v1/orders")]`:
     - **Static Radix-Tree Router:** Path templates (`"/users/{user_id:u64}/items/{slug:str}"`) compile at `comptime` into a zero-regex byte-trie extracting typed `u64` and `0b01` `str` slices directly out of the incoming HTTP request buffer (`0` allocations!).
     - **Contract-Driven Validation:** Method parameter refinements (`M26` `where` & `requires`) weave directly into HTTP `422 Unprocessable Content` responses with field-level JSON error pointers before the business method is even entered!
     - **Symmetric Client + Server + OpenAPI 3.1:** Generates `OrdersRouter` (server), `OrdersHttpClient` (typed caller), `.pchor` `role` metadata (`M24`), and `OrdersService.openapi_json() -> str`—forming a **100% lossless bidirectional pair** with `M20` `use openapi`!
3. **`[Entity]` Self-Generating Typed Persistence & Schema Diffs (`std.db.entity`):**
   - Annotate `pub struct Order { [Id] pub id: u64; [Index] pub customer_id: u64; pub total_cents: i64; [Version] pub ver: u32; }` with `[Entity(table: "orders")]`:
     - Synthesizes strongly-typed column tokens (`Order.Cols.customer_id : SqlCol<Order, u64>`) enabling compile-time type-checked predicates (`Order.where(Order.Cols.customer_id == cid && Order.Cols.total_cents >= 1000).order_by_desc(Order.Cols.id)`) that compile at `comptime` into prepared SQL strings (`SELECT id, customer_id, total_cents, ver FROM orders WHERE customer_id = $1 AND total_cents >= $2 ORDER BY id DESC`)!
     - Synthesizes binary wire row decoders (`M20` Postgres/MySQL/SQLite), batch `insert_many` (`UNNEST` / multi-tuple), optimistic locking (`UPDATE ... WHERE id = $1 AND ver = $2`, returning `DbErr::StaleVersion` if `rows_affected == 0`!), and `Order.diff_ddl(prev_snapshot)` emitting `CREATE TABLE` / `ALTER TABLE` SQL migrations!

#### 30.4 Transparency Invariant (`pilot inspect --weave`) & Diagnostics
- **Zero Magic Black-Box Rule:** Running `pilot inspect --weave Order` or clicking the `pilot lsp` weave code-lens displays 100% of the clean, formatted Pilot code generated by `[Cli]`, `[Service]`, and `[Entity]`, and allows ejecting or overriding any individual method at **Tier 1** (`M22` 4-tier `MethodTable` guarantees an explicit Tier-1 method on `struct Order` always overrides a Tier-2 woven method seamlessly!).
- **Diagnostics:** `GEN001` (Invalid `[Route]` Path Parameter Syntax or Unmatched Method Arg), `GEN002` (`[Service]` Payload Type Missing `[Json]` Serialization Contract), `GEN003` (`[Entity]` Missing `[Id]` Primary Key Field), `GEN004` (SQL Query Column Type Mismatch in Typed Predicate), `GEN005` (Duplicate CLI Short/Long Flag Collision in `[Cli]`), `GEN006` (Destructive Schema Column Drop Detected in `diff_ddl` Without `[AllowDropColumn]`).

#### 30.5 End-to-End Integration Proof ("80-Line Full-Stack Microservice & CLI Test")
Write a single **80-line pure-Pilot file** defining `Order` (`[Eq, Clone, Json, Entity]`), `OrderApi` (`[Service]`), `AdminCli` (`[Cli]`), and a 12-line `.pchor` blueprint (`M24`) wiring `OrderApi` $\rightarrow$ in-memory SQLite (`M12`)/SQL engine:
1. Invoke `AdminCli.parse_args(["create", "--customer-id", "42", "--total-cents", "2500"])` $\rightarrow$ dispatches HTTP `POST /v1/orders` via woven `OrderApiClient` $\rightarrow$ validates `total_cents > 0` contract $\rightarrow$ inserts into `orders` via woven `[Entity]` SQL $\rightarrow$ fetches via `GET /v1/orders/1` $\rightarrow$ round-trips JSON!
2. Feed `OrderApi.openapi_json()` directly into `M20`'s `absorb_openapi` $\rightarrow$ assert **100% schema & endpoint round-trip fidelity**!
3. Override `find_by_id` explicitly at Tier 1 on `Order` $\rightarrow$ assert Tier-1 custom query cleanly supersedes the woven default with zero conflict errors!

#### 30.6 Phase 3 (`M21`–`M30`) Cumulative Exit Checkpoint
Before advancing to **Phase 4 (`M31` — Peak Performance, Hardware Backends & Zero-Rust Decoupling)**:
- **Gate 1 (Adaptability & Zero Hardcoding):** `selfhost/attr_weaver.pilot`, `selfhost/analyzer.pilot`, and `selfhost/cst/cst_recovery.pilot` are 100% free of hardcoded attribute names, 6-field struct caps, and 110-word static keyword lists; custom user `syntax` DSLs (`M27`) and weavers (`M21`, `M30`) register dynamically; Dual-Pass Universal Rename Litmus passes 100% green.
- **Gate 2 (Hermetic Determinism & Bootstrap Parity):** Every `comptime` block, weaver, and `.pchor` compilation runs inside the capability-gated deterministic `VmSandbox` (`M29`); Stage-2 == Stage-3 SHA-256 parity holds across randomized host environments; `selfhost/perf/optimizer.pilot` and all `meta.pilot` / `contracts_syntax.pilot` placeholder macros are permanently deleted.
- **Gate 3 (Zero-Overhead Abstractions & Layout Speed):** Stream pipelines and `each` broadcasts fuse to `0` intermediate heap allocations (`M23`); `.pchor` `cue` boundaries statically block `0b01`/`0b11` data races (`M24`); `.pchor` `SoA<T, align: 64>` achieves $\ge 3.5\times$ cache-bandwidth speedup with zero domain edits (`M25`); `state` typestates add `0` runtime bytes (`M26`); and `[Cli]`/`[Service]`/`[Entity]` self-generate zero-reflection code (`M30`).

---

## Phase F (`F1` – `F8`): Foundation — Correct Seed, Type Checker, Real `PilIR`, C11 Backend & Real Self-Hosting

> **Why this phase exists (inserted 2026-11-09):** The audit in [`Audit.md`](Audit.md) found that `M1`–`M30` exist as libraries that only their own tests call, and that the four things `M31`–`M50` are specified on top of — a type checker, a typed SSA IR, a runtime ownership model and a native backend — do not exist yet. Phase F builds them. **No milestone from `M31` onward may start until its Phase F prerequisites (table at the end of this phase) are `[STATUS: INTEGRATED]`.**
>
> **Numbering:** Phase F uses `F1`–`F8` so that `M1`–`M100` keep their numbers.

### Phase F Verification Contract

These rules apply to every `F` milestone and replace the "four function-level tests" pattern used so far.

1. **Through the CLI, on source files.** Acceptance tests are `.pilot` programs under `tests/conformance/`, each with an expected-stdout file and an expected exit code. They are executed by the built `pilot` binary. A test that concatenates library files and calls functions by hand does not count toward completion.
2. **Positive and negative.** Every diagnostic code has at least one program that must trigger it and one similar program that must not. Identifiers in these programs must be neutral: no diagnostic may depend on the spelling of a user's names.
3. **No supplied verdicts.** A checker must not take as a parameter the fact it is meant to establish (for example `is_pure`, `can_prove`, `takes_raw_ptr`, `in_loop`).
4. **Exact assertions.** Compare full output or full diagnostic codes, never string length or substring presence alone.
5. **Unsupported means error.** Any construct the pipeline cannot yet handle must produce a diagnostic and a non-zero exit code. Silently dropping a node, returning `null`, or exiting `0` is a defect.
6. **Differential by default.** Once two execution paths exist (AST VM, `PilIR` interpreter, C11 native), the conformance corpus runs on all of them and the outputs must match.
7. **Deletions are part of done.** The stand-in code each milestone replaces (listed per milestone below) is deleted in the same milestone, as [`AGENTS.md`](file:///home/moony/Documents/dev/projects/Pilot/AGENTS.md) §1 already requires.

> **Decided 2026-11-09 — Rust seed contraction is paused until `F8`.** `F1`–`F3` need changes to the Rust seed (`crates/pilot-vm`, `crates/pilot-analysis`, `crates/pilot-cli`), and every crate sat 2–6 lines under its line ceiling. While paused:
> - The gate no longer enforces the per-crate and per-file ratchet ceilings. It enforces the `<= 1,500` LOC law per Rust file and a total budget of `30,000` Rust `src` lines (raised from `15,000` on 2026-11-09), which leaves room to reformat the dense seed to normal line lengths and land `F1`–`F3`.
> - The pre-pause ceilings stay recorded in `selfhost/gate/manifest.pilot` for when contraction resumes. `pilot gate --ratchet-update` is refused.
> - Milestone tests that assert Rust LOC ceilings read the same policy through `crates/pilot-analysis/tests/common/seed_policy.rs`.
> - `AGENTS.md` §1 ("Pilot First & Active Rust Engine Contraction") records the pause and what Rust growth is allowed: execution-engine correctness only.
> - `F8` exit includes setting `seed_contraction_paused()` to `false`.

---

### F1 — Correct Seed Execution (Every Construct Runs or Errors)

- **Objective:** `pilot run` executes the whole surface language correctly, or refuses loudly.
- **Current state:** `BytecodeCompiler::compile_node` (`crates/pilot-vm/src/lib.rs`) handles about 20 node kinds and drops the rest in a `_ => 0` arm. Binary operators always compile to integer opcodes and unknown operators fall back to `AddI64`. Integers and heap handles share one untagged representation.
- **Deliverables:**
  1. A tagged runtime value: integer, float, bool, string, list, map, struct instance, closure and null are distinguishable. `var z = 0; z.a` must be an error, not a lookup in map `0`.
  2. Lowering for every node kind the parser produces: `switch`/match, `defer`, `guard`, `try`/`catch`, lambdas and closures with capture, cascades (`..`), ranges, `??`, postfix `?`, `comptime` blocks, f-string interpolation, constructors `self(...)`, field defaults, and `~self()` at scope exit.
  3. Float and comparison opcodes selected from operand kinds; `&&` and `||` short-circuit; an unknown operator is a compile error.
  4. Process exit code equals `main`'s return value; an uncaught failure exits non-zero.
  5. `tests/conformance/` with a runner. Until `pilot test` works (`F3`), the runner may be one Rust integration test that invokes the built `pilot` binary and diffs stdout and exit code.
- **Done when:** every row of the table in `Audit.md` §3.2 behaves as the language reference says; `examples/overview.pilot` and `examples/*.pilot` produce checked output; the legacy-syntax files in `tests/*.pilot` (`def`, `match`, `record`) are either migrated to canonical syntax and passing, or deleted.
- **Deletes:** the `_ => 0` fall-through and the `_ => Opcode::AddI64` default.
- **Progress (2026-11-09, part 1):**
  - Done: deliverables 3, 4 and 5; deliverable 1 for lists and records (`TAG_LIST`, `TAG_MAP`); f-string interpolation, ranges and `??` from deliverable 2. Both deletions are done: unlowered node kinds and unknown operators are now compile errors (`is not lowered by the seed yet`), and erroneous nodes compile to a `Trap` opcode.
  - The VM records a `VmFault` with a source span. `pilot run` prints `file:line:col: error:` and exits 1. `run_pilot_analyzer`, the bootstrap manifest and the self-hosted tool scripts surface faults instead of returning `null`.
  - `.pilc` format is now version 4 (opcodes renumbered to tag-polymorphic `Add`/`Sub`/…; `selfhost/vm/compiler.pilot` and `selfhost/pilotc.pilot` updated to match).
  - `tests/conformance/` holds 24 programs after part 1 (10 error cases) run by `crates/pilot-cli/tests/conformance.rs`.
  - Turning faults on exposed latent bugs in Pilot sources that the old VM hid by reading null as `0`; these were fixed at the source (`?? 0` defaults, undeclared `ccount` in `analyzer.pilot`, `static` method reading `self` in `abi/aapcs64.pilot`, integer `0` used as the environment record, `< 0` checks on record handles).
- **Progress (2026-11-09, part 2):**
  - `switch` lowers literal, enum-variant (`Enum.V`, `::V`), range (`..`, `..=`), or (`|`), tuple, binding and wildcard patterns, plus arm guards (`case x if cond:`), which the parser used to discard. `break` leaves the `switch`; `continue` reaches the enclosing loop. With no matching arm the `switch` yields `none`.
  - `guard cond else`, `guard some x else` and `guard some x = v else` lower; the parser now keeps the binding name and the `some` form.
  - Postfix `?` propagates `none`. On a record (a `res`/`opt` struct) it faults, because unwrapping must go through the type's `op ?` method and the seed cannot dispatch methods yet.
  - Lambdas, captured variables, named functions used as values, and calls through function values are in. There is a function value kind (`TAG_FN`) and a `CallClosure` opcode. Captures are by value, so assigning to a captured variable is a compile error.
  - Parser fixes: `var <modifier-word> = …` and a statement starting with `pkg`/`mod` now parse as bindings and expressions. A variable named after a foreign language (`c`, `rust`, …) is no longer read as an inline foreign block in `switch`/`if`/`while` subjects. Range patterns use the range precedence from the Pilot operator table. A binary operator missing an operand is a compile error instead of a compiler crash.
  - Using a declared keyword as a name (`var pkg`, `fn default()`) parses but stays the analyzer's KW001 error, as `stdlib/core/keywords.pilot` declares; `pilot run` does not run the analyzer yet (F3).
  - `tests/conformance/` now holds 28 programs (11 error cases).
  - Remaining: `try`/`catch`, `defer`, cascades, `comptime` blocks, constructors, field defaults and `~self()`, dimensional units and `f64(x)` casts (`examples/overview.pilot` stops on these), runtime method dispatch (which also unlocks `op ?` on `res`/`opt`) and the enumerator protocol, variant patterns with payloads, structural equality for lists, the legacy `tests/*.pilot` corpus, and moving the `SeedBuiltin` method table into Pilot.
- **Progress (2026-11-09, part 3):**
  - `defer stmt` runs when its block exits: falling off the end, `return` (after the return value is computed), `break`/`continue`, and a failing `?`. Each block runs its deferred statements last-in first-out, innermost block first. The deferred code is emitted inline at each exit.
  - Field defaults (`retries: i64 = 3`) are kept by the parser and fill any field a struct literal leaves out. `Name(args)` builds the defaulted record and runs the constructor `self(params) { .. }` on it; `Name()` on a struct without a constructor gives the defaulted record, and `Name(args)` without one is a compile error. A constructor is compiled at its first `Name(..)` call, like a generic body checked at instantiation, so a struct whose constructor needs types from unlinked modules (`MapBuffer` in `map.pilot`) still works for its other members until F2.
  - Generic calls `Name<A, B>(..)` and `Name<A>.member` parse as calls and member accesses; the parser used to skip the type arguments and then read the call as a comparison. A field named `syntax` (`pub syntax: str,`) is no longer read as a `syntax { .. }` macro declaration.
  - `try { .. } catch err { .. }` and `expr catch err => fallback` lower, values-first: a failing `?` inside a `try` (or inside the `catch` expression) jumps to the catch clause with the failure bound, instead of returning from the function. VM faults are not failures and are never caught. A `try` takes one `catch` clause until failures carry a type the seed can tell apart. The `try`/`catch` rules in `stdlib/core/syntax/try_catch.pilot` only splice the body and handler together, so the seed lowers these nodes directly; moving the semantics into that Pilot rule needs `op ?` dispatch first.
  - `~self()` destructors are left to F7, where ownership and escape analysis decide when a value dies.
  - Known gaps: a bare `return;` inside a constructor returns `none` rather than the record. `ITryOperator` is declared twice (`stdlib/core/operator.pilot` and `stdlib/core/operators/try_op.pilot`), and neither declares how a type reports failure, which `op ?` dispatch on `res`/`opt` records needs.
  - `tests/conformance/` now holds 31 programs (11 error cases).
  - Remaining: cascades, `comptime` blocks, dimensional units and `f64(x)` casts, runtime method dispatch (including `op ?` on `res`/`opt`) and the enumerator protocol, variant patterns with payloads, structural equality for lists, the legacy `tests/*.pilot` corpus, and moving the `SeedBuiltin` method table into Pilot.
- **Progress (2026-11-09, part 4):**
  - Cascades lower: `recv..method(args)..field = value` evaluates the receiver once, applies each step to it and yields it; `recv?..step` skips the steps on `none`. `..field = value` (and compound forms) now parses inside a cascade, with the value ending at the next step.
  - Cascade or range: a `..` after a numeric literal is a range; `..name(` and `..name =` start a cascade step; `..a.b` is a step only when the chain is assigned. `0..xs.len()` and `0..empty.len` (in `examples/crucible_2.pilot`) used to parse as cascades. A range bounded by a plain call is written `i..(f(x))`.
  - `==`/`!=` compare lists and tuples element by element.
  - Methods are reachable only through their type. They used to be registered under their bare names too, so a free function and a method with the same name were confused, and `x.m()` on an untyped receiver reached whichever was declared first. Records built from a struct literal or constructor remember their struct (VM metadata, not a field). A method call whose receiver type is unknown tests it against every struct declaring the method, then falls back to the single `ext` type declaring it, the seed builtin, or a free function, and otherwise stops with "no method `m` for this value". Inside a method, a bare `m(..)` is `self.m(..)` (or the static `Type.m(..)`) when the type declares `m`, including inside lambdas.
  - `op ?` on `res`/`opt` still waits for F2: `stdlib/core/result.pilot` and `option.pilot` cannot be linked yet, so no such record exists at run time.
  - `tests/conformance/` now holds 34 programs (11 error cases).
  - Remaining: `comptime` blocks, dimensional units and `f64(x)` casts, `op ?` on `res`/`opt` and the enumerator protocol (after F2), variant patterns with payloads, the legacy `tests/*.pilot` corpus, and moving the `SeedBuiltin` method table into Pilot.
- **Progress (2026-11-09, part 5):**
  - Casts: `T(x)` and `x as T` convert to a primitive's register layout. The layout comes from the type's `[Bits(width, class)]` attribute in `stdlib/core/core.pilot` and `stdlib/core/primitives/*.pilot`: the attribute is run through the Pilot comptime function `Bits`, whose body reaches the `set_register_layout` engine hook, and the parser tables record the result (`crates/pilot-parser/src/attr_hooks.rs`). No type name appears in Rust. Floats truncate toward zero, integers wrap to the width, a 1-bit type is a boolean, and a string or other non-number is a runtime error. A heap handle (record, list or function) cast to a 64-bit integer type stays that handle, since selfhost code types handles as `i64` (`cli_map_get(..) as i64` in `selfhost/cli/spec.pilot`). `x as T` used to return `x` unchanged. Its precedence now comes from Pilot (`prec_cast` in `Cfg.Ops`, between `*` and prefix operators); the parser had it hard-coded below comparisons, so `a + b > n as u8` cast the comparison, which `selfhost/adapter/c_parser.pilot` relied on reading the other way.
  - Top-level `let`/`var`/`const` bindings that are not compile-time constants are globals every function reads and writes; reading one before its declaration has run is a runtime error. They used to be invisible inside functions (`global_counter` in `examples/overview.pilot`).
  - `comptime { .. }` and `comptime expr` lower: each is compiled out of line and all of them run once, in source order, before the top-level statements and `main`; an expression's value is read where it appears. Inside a function, `comptime { .. }` used to parse as a plain block (the parser read `comptime` as a modifier on it), and `comptime expr` ran as an ordinary expression.
  - Enum variants that carry values: `Shape.Circle(2.0)`, `Shape::Circle(2.0)` or a bare `Circle(2.0)` build a record tagged as the variant, with the values under their declared field names. `case Circle(r):` matches the variant and its values by position (with literal, wildcard and nested sub-patterns); `case Circle:` matches the variant whatever its values.
  - Known gaps: `quote { .. }` in an expression still evaluates its body instead of producing an AST value. A variant record prints as `{ r: 2 }`, without its variant name.
  - `tests/conformance/` now holds 39 programs (13 error cases).
  - Remaining: dimensional units (`UnitLit`), `|>` pipes, `op ?` on `res`/`opt` and the enumerator protocol (after F2), the legacy `tests/*.pilot` corpus, and moving the `SeedBuiltin` method table into Pilot.
- **Progress (2026-11-09, part 6):**
  - Pipes: `x |> f` calls `f(x)`, `x |> f(a)` calls `f(x, a)`, `x |> .m(a)` calls `x.m(a)`, and `f <| x` calls `f(x)`. Each step lowers as the call it stands for, so it dispatches like a written call (`crates/pilot-vm/src/lower_pipe.rs`). The parser now marks a prefix-dot member (`MEMBER_IMPLIED_RECEIVER`) so a pipe can tell `.m()` from `self.m()`.
  - Dimensional literals (`250ms`, `64KB`, `2.4GHz`, and the bracketed `3[ms]`) evaluate to their value in the family's base unit (nanoseconds, hertz, bytes). The values come from Pilot: `stdlib/core/syntax/unit_decl.pilot` registers `convert_unit_literal_to_base` and `unit_family_of_literal` through the `bind_unit_literal` engine hook, and the seed runs them on each literal while compiling (`crates/pilot-vm/src/lower_unit.rs`). No unit name appears in Rust. A literal with an unknown unit is a compile error. `convert_unit_literal_to_base` used to ignore the decimal point, so `2.4GHz` was 24 GHz.
  - Known gaps: units are not type-checked, so `2s + 64KB` adds two integers; compound units (`100[m/s]`) are not lowered.
  - `tests/conformance/` now holds 42 programs (14 error cases).
  - Remaining: `op ?` on `res`/`opt` and the enumerator protocol (after F2), the legacy `tests/*.pilot` corpus, and moving the `SeedBuiltin` method table into Pilot.
- **Progress (2026-11-09, part 7):**
  - Named arguments: `f(b: 1, a: 10)` binds each value to the parameter it names; positional arguments come first. The parser used to drop the names and pass the values in written order. It now keeps them in a `CallArg` node, and the seed binds them for functions, methods and constructors (`crates/pilot-vm/src/lower_args.rs`). An unknown or repeated name, a positional argument after a named one, a missing argument, or a named argument to a callee whose parameters are unknown (a closure) is a compile error.
  - Memberwise construction: a struct without `self(..)` is built by `T(a, b)` or `T(b: 2, a: 1)`; a field left out takes its default, and one with no default is a compile error. `var x: T;` members of a struct body are fields (they used to be skipped, so their defaults never applied).
  - Block scope: a `let` in an inner block that reuses an outer name now shadows it until the block ends. It used to overwrite the outer variable (`examples/crucible_200.pilot` pillar 4).
  - `[0, *xs, 99]` and `[*..nested]` (and `splat(xs)`, `flat(xs)`) unpack inside list literals; anywhere else `*x` is a compile error.
  - `union`, `intersect` and `except` on lists are Pilot functions in `stdlib/db/set_ops.pilot`, bound to their operators with the new `bind_operator_fn` engine hook (`crates/pilot-parser/src/pilot_impls.rs`). The first use of such an operator compiles that module's functions into the program (`crates/pilot-vm/src/lower_linked.rs`); F2's `use` linking replaces this.
  - `examples/crucible_2.pilot` now prints its expected output. `examples/crucible_200.pilot` matches except pillar 1, which needs value-type structs (`: IValue`) to be copied when passed and stored; the seed passes every record by handle. That needs the lattice classification of F4 and the copy insertion of F7.
  - `examples/overview.pilot` still stops on constructs outside F1: an inline `sql { .. }` block (reported as foreign code the seed cannot run), channel send/receive (`<-`), and `unit meter;`, which the keyword catalog defines as a compilation-unit wrapper rather than a unit of measure.
  - `tests/conformance/` now holds 50 programs (16 error cases).
  - The legacy `tests/*.pilot` corpus is deleted: its 40 files used pre-canonical syntax (`def`, `class`, `record`, `import`), nothing ran or referenced them, and most exercised features of later milestones (actors, Postgres, polyglot). `tests/conformance/` replaces them as the seed's checked corpus.
  - Remaining: `op ?` on `res`/`opt` and the enumerator protocol (after F2), and moving the `SeedBuiltin` method table into Pilot (the `bind_operator_fn` path is the mechanism for it).
- **Progress (2026-11-09, part 8) — `[STATUS: COMPLETE]`:**
  - The `SeedBuiltin` method table is gone. `stdlib/core/builtins/native_calls.pilot` binds every name the seed used to hardcode (`print`, `assert`, `len`, `push`, `ENV`, `sizeof`, ...) to a VM intrinsic through the `bind_native_fn`, `bind_native_method`, `bind_native_property`, `bind_native_value` and `bind_type_query` engine hooks (`crates/pilot-parser/src/pilot_impls.rs`, `crates/pilot-vm/src/lower_native.rs`). `assert(cond, msg)` reports its message. `sizeof(T)` answers from the type's `[Bits]` layout, and is a compile error for a struct until F4 computes struct layouts.
  - Destructuring: `let (a, b) = t;`, `let [head, ..rest, last] = xs;` and `let { "k": v, "nested": { "x": x }, name } = m;` bind through a pattern, locally or as globals at the top level; a value the pattern does not fit is a runtime error. `switch` matches record patterns (`case Point { x: 0, y }:`), typed or not, and slice patterns with one rest element (`case [first, ..rest]:`, `*rest`) (`crates/pilot-vm/src/lower_destructure.rs`). `examples/showcase/08_exhaustive_pattern_matching.pilot` now runs end to end.
  - `if some x = v { .. } else { .. }` binds `x` when `v` is present; it used to parse as a comparison. `f"{m[\"k\"]}"` (escaped quotes in an f-string hole) parses. A global holding a closure is callable, and globals take method calls.
  - Syntax errors: `let = 3;` and `let x = ;` are errors; they used to run with the binding set to `none`. Every row of `Audit.md` §3.2 that F1 owns now behaves as the reference says.
  - `tests/conformance/` now holds 57 programs (20 error cases).
  - **Moved out of F1**, each to the milestone that owns the mechanism:
    - `op ?` on `res`/`opt` records and the `for` enumerator protocol over user types → **F2**: `result.pilot`/`option.pilot` must be linked first. `ITryOperator` also needs a success predicate (it declares only `op ? () -> TOk`), which F2 adds when it links the prelude.
    - `use std.crypto;` and other imported functions, and `without` exports (`examples/test_exclusions.pilot`, `examples/mytest.pilot`) → **F2**.
    - Type errors such as `let x: i64 = "hello";` → **F4** (the seed has no type checker; F4 adds it).
    - Value-type copies for `: IValue` structs (`examples/crucible_200.pilot` pillar 1) → **F4** classifies, **F7** inserts the copies. `~self()` destructors → **F7**.
    - Constructs of later milestones that `examples/overview.pilot` and `examples/all_syntax.pilot` use: inline `sql { .. }` blocks, channels (`<-`), `yield` generators, and `unit meter;` (which collides with the keyword catalog's compilation-unit `unit`). Each stops the program with a compile error naming the construct; none is silently dropped.

### F2 — Real Module Linking

- **Objective:** `use` compiles and links the imported unit's code.
- **Current state:** `AnalysisPipeline::load_program` (`crates/pilot-analysis/src/pipeline.rs`) finds imports by scanning for lines that start with `use ` and only mounts them into the analysis registry. Imported functions evaluate to `null`. `for` loops fail for this reason: `get_enumerator` lives in an unlinked stdlib file.
- **Deliverables:**
  1. Import resolution from the parsed AST, not from text lines.
  2. Each unit parsed into its own arena with a `FileId`; spans in diagnostics name the right file.
  3. Topological load order with cycle detection (`MOD002`), driven by `selfhost/loader/module_graph.pilot` once that file has a real SCC pass, or by the seed until then.
  4. An implicit prelude so `for`, collections, `opt`/`res` and `print` work with no imports.
  5. Script arguments after `--` reach the program.
  6. Postfix `?` on `res`/`opt` records dispatches to the type's `op ?`. `ITryOperator` gains the success predicate it lacks (it declares only `op ? () -> TOk`), and its duplicate declaration (`stdlib/core/operator.pilot` and `stdlib/core/operators/try_op.pilot`) collapses to one (carried from F1).
  7. `use M without { .. }` exclusions (`examples/test_exclusions.pilot`). The F1 stopgap `link_pilot_module` (`crates/pilot-vm/src/lower_linked.rs`), which compiles an operator's module on first use, is replaced by real linking.
- **Done when:** `for x in xs`, `for i in 0..n`, `List<T>`/`Map<K, V>` methods and a function imported from `std.*` all work in conformance programs; an import cycle and a missing module each fail with their code.
- **Deletes:** the textual `use ` scanner in `pipeline.rs` and the string-concatenating `eval_sh` loader in `crates/pilot-cli/src/main.rs`.
- **Progress (2026-11-09) — `[STATUS: COMPLETE]`:**
  - **Loader in Pilot.** `selfhost/loader/module_graph.pilot` builds the graph and the link plan. `mg_link_plan(entry, toolchain_root)` returns the unit files in link order, plus `MOD001` (unresolved import) and `MOD002` (cycle, with the cycle path) diagnostics. An import resolves by its conventional path first, then through a package index built from the `pkg` declarations under the search roots (the entry's directory and `stdlib/`). The seed adds two intrinsics: `unit_header(path)`, which parses only a file's `pkg`/`use` header (`pilot_parser::parse_header`), and `proc_args()`.
  - **Units.** Each file is parsed with its own `FileId` into one shared arena (`pilot_parser::parse_unit`), and its spans are offset by a per-file base, rustc-`SourceMap` style. One program-wide offset space keeps instruction spans, compile diagnostics and VM faults unchanged; `AstArena.units` maps any node back to its file. This replaces the planned arena per unit: the seed compiles one program, and per-file arenas would only add a translation layer.
  - **Linking.** The entry unit compiles eagerly. Imported units register their types, functions and globals, and their functions and methods compile on first call (`lazy_decls`, a fixpoint over unresolved calls when compilation finishes). Comptime blocks of every unit run first, then each unit's top-level statements in link order, then `main`. A name the program declares (function, struct, local) now shadows a native function of the same name; receptors such as `nameof` stay syntax. Toolchain scripts (`pilot gate`, the CLI, emitters) link their files the same way; the string-concatenating tool loader is gone.
  - **Prelude.** `stdlib/core/prelude.pilot` (`Pilot.Core.Prelude`) is linked into every program: `opt`/`res` (`Pilot.Core.Option`, `Pilot.Core.Result`) and the set operators (`Pilot.Db.SetOps`). `for` needs no import: the enumerator protocol is bound in Pilot (`bind_enumerator` in `stdlib/core/syntax/for.pilot`), and a loop over a value of unknown type picks enumerating or indexing at run time. `List<T>()` and `Map<K, V>()` build the engine's list and map (`native_calls.pilot`) until a backend runs `std.col`'s raw-memory containers (F6). The genesis package (`stdlib/core/core.pilot`) is read by the toolchain itself, so importing it links nothing.
  - **Postfix `?`.** `ITryOperator` declares `is_success()` and `op ?`, once (`stdlib/core/operators/try_op.pilot` deleted). `bind_try_protocol(nameof(is_success), "?")` in `try_catch.pilot` names both. Struct `op` members compile as methods named by their operator. `x?` and `x catch e => ..` on a struct record call `is_success()`: on success the value is its `op ?`, otherwise the record itself is the failure. `result.ok`/`err` and `option.none` leave the absent payload out (zero-filled) instead of `__ssa_cast`, which the seed cannot evaluate.
  - **Exclusions.** `without Item;` and `without Type { member, .. };` in the declaring module hide those names from other units; a use is a compile error naming the item.
  - **Script arguments.** `pilot run file.pilot -- a b` passes `a`, `b` to the program; `std.env.args()` returns them. The CLI tokenizer now keeps argv entries verbatim (empty and space-padded arguments used to be dropped or trimmed).
  - **Deleted:** `AnalysisPipeline::load_program` (the text `use` scanner), the string-concatenating tool loader, `crates/pilot-vm/src/lower_linked.rs` (`link_pilot_module`), the `try_unwrap` intrinsic, `stdlib/core/operators/try_op.pilot`, and the unused `use Pilot.Core.Keywords` in `reflect.pilot` that formed an import cycle.
  - **Conformance:** `modules` (conventional and indexed resolution, struct methods, globals), `for_enumerator`, `containers`, `try_records`, `script_args` (with the new `<name>.args` runner file), and errors `err_import_cycle` (`MOD002`), `err_import_missing` (`MOD001`), `err_without`, `err_without_member`.
  - **Deviations and deferrals:** `pub`/`priv` visibility across units is not enforced (F4). Generic free constructors (`ok(x)`, `some(x)`) need return-type inference (F4); programs write `result<T, E>.ok(x)`. Computed properties (`pub is_err: bool { get => .. }`) still read as fields (F4). `examples/mytest.pilot` now links, but `Pilot.Core.Memory` (bundles, raw pointers, `__ssa` casts) cannot run on the seed (F6).

### F3 — Surfaced Errors, `pilot check` & `pilot test`

- **Objective:** errors reach the user, and the project can test itself without `cargo`.
- **Current state:** syntax errors, undefined names and analyzer diagnostics are discarded by `pilot run`. `check`, `test`, `fmt`, `build`, `compile` and a dozen other commands return `"0"` from the if-chain in `selfhost/cli/main.pilot` and do nothing.
- **Deliverables:**
  1. `pilot run` and `pilot check` print diagnostics (using the existing renderer in `selfhost/diagnostics/renderer.pilot`) and exit non-zero on any error.
  2. Parse errors and unresolved names are errors.
  3. `pilot test` discovers and runs the conformance corpus and `[Test]` functions. This brings the core of `M46` forward, because milestone tests currently live in Rust files that `M38` and `M89` are meant to delete.
  4. Every command listed in `pilot --help` either works or prints "not implemented" and exits non-zero. Commands are dispatched through the registry, not an if-chain.
  5. An engine intrinsic for reading standard input, so `repl` and `lsp` become possible later.
- **Done when:** a negative corpus (one bad program per diagnostic code) passes under `pilot test`; no listed command exits `0` without doing its job.
- **Deletes:** the fall-through `return "0";` in `run_cli_entry`.

### F4 — Type Checker & Method Tables

- **Objective:** a real semantic analyzer.
- **Current state:** `selfhost/analyzer.pilot` infers by scanning backwards for a node with the same name, and about 35 diagnostics fire on identifier prefixes (`bad_`, `mismatch`, `undefined`, `badcast`, `moved`, `esc1`…, lines 1172–1331). `selfhost/types/method_table.pilot` and `bundle_splice.pilot` are real but called only by tests.
- **Deliverables:**
  1. Scoped name resolution over the linked module graph.
  2. Type representation with inference and unification for locals, calls, generics with constraints, `opt<T>` / `res<T, E>`, and closures.
  3. Method tables built from `struct`, `interface`, `ext`, `*Bundle` and `*Facet` declarations with the 4-tier precedence, using the existing `method_table.pilot` and `bundle_splice.pilot`.
  4. 2-bit lattice classification computed from field types and `~self()`, stored on each type.
  5. Diagnostics `TYPE001`–`003`, `FLOW001`, `OWN001`, `NULL00x`, `ERR00x` derived from the analysis.
  6. `syntax type_construct` rewrites (`T?`, `T ! E`, `T[]`…) resolved to canonical types with no type names hardcoded in the seed.
- **Done when:** the positive/negative corpus for each code passes with neutral identifiers; a program that renames core types per the M1 rename pairs still type-checks **and runs** (this replaces the text-only rename litmus).
- **Deletes:** every name-prefix trigger in `analyzer.pilot`.

### F5 — Real `PilIR`

- **Objective:** the typed SSA IR that `M6` specified.
- **Current state:** `selfhost/ir/` has no function, block or module containers. `lower_ast.pilot` concatenates text and hardcodes `"i64"` and `"RangeWindow"`. The "parser" in `printer.pilot` counts occurrences of the substring `block`.
- **Deliverables:**
  1. `IrModule`, `IrFunction`, `IrBlock` (with block parameters) and `IrInst` as typed `[Layout(c_abi)]` structures. Fix the `M6` size contradiction first (`IrInst` is declared 32 bytes but its fields sum to 38).
  2. Lowering from the typed AST of `F4` for the whole language, including closures, `defer`, `switch`, loops with `break value`, and `?` propagation.
  3. A verifier that computes dominators itself.
  4. A textual printer and a parser that round-trip.
  5. A pass driver with an `IPilIrPass` registry, plus constant folding and dead-code elimination as the first two real passes.
  6. An IR interpreter, so the IR is executable before any native backend exists.
- **Done when:** the conformance corpus passes on the IR interpreter with output identical to the `F1` path; printer → parser → printer is byte-identical on the corpus.
- **Deletes:** the text-template lowering in `selfhost/ir/lower_ast.pilot` and `selfhost/transpile/ir.pilot` (the deletion `M6` mandated).

### F6 — One Real Native Backend (C11) & `pilot build`

- **Objective:** Pilot source compiles to a native executable.
- **Current state:** `selfhost/codegen/c_emitter.pilot` walks the AST for an integer-only subset (`f64` and `str` become `int64_t`; `-> i64` often becomes `void`). `crates/pilot-codegen` returns `int main(void){return 0;}`. Nothing is reachable from the CLI, and Pilot code cannot spawn a process.
- **Deliverables:**
  1. C11 emission from `PilIR` covering every type and instruction.
  2. A runtime library (allocation, strings, lists, maps, closures, printing, panics) with a stable ABI.
  3. An engine intrinsic for spawning a process and one for writing files, so `pilot build` can write C and invoke `cc`/`clang`.
  4. `pilot build file.pilot -o out` producing a runnable binary, with compiler failures reported as diagnostics.
- **Done when:** the conformance corpus passes when built natively and the outputs match the VM and the IR interpreter; a first benchmark set (`fib`, a tight numeric loop, a string-building loop, a map workload) is recorded against C as the baseline for Gate 3.
- **Deletes:** the `BuildDriver` template in `crates/pilot-codegen/src/lib.rs`, including its fixed GPU kernels.

### F7 — Runtime Ownership (Perceus for Real)

- **Objective:** values are freed, destructors run, and leaks are measurable.
- **Current state:** the VM never frees anything and `~self()` never runs. `ArcHeader`, `pilot_rt_drop` and `pilot_rt_drop_reuse` exist in `crates/pilot-rt` and are referenced by nothing. `selfhost/perceus/arc.pilot` returns strings such as `"Drop:x"` that no backend consumes.
- **Deliverables:**
  1. Reference-counted heap objects in the C11 runtime, and in the VM or IR interpreter.
  2. Dup/drop insertion as a `PilIR` pass with correct handling of branches and loops.
  3. `~self()` invoked at last use for `0b10` objects and at scope exit, in reverse declaration order, for `0b11` guards.
  4. Escape checks `ESCAPE001`–`003` for `0b01` views, computed from the lattice classes of `F4`.
  5. A live-allocation counter reported at exit under a flag.
  6. Copy semantics for `0b00` value types: a `: IValue` struct passed, returned or stored is copied, not shared by handle (`examples/crucible_200.pilot` pillar 1, carried from F1).
- **Done when:** the whole conformance corpus exits with zero live allocations on the native backend; guard-ordering and use-after-move programs behave or fail as specified.
- **Deletes:** the string-emitting drop schedulers in `selfhost/perceus/arc.pilot`.

### F8 — Real Self-Hosting Fixed Point

- **Objective:** the Pilot compiler compiles itself, and the parity gate measures that.
- **Current state:** `selfhost/pilotc.pilot` extracts four tokens from one fixed-shape function and fills a hardcoded C string and bytecode blob. `selfhost/gate/bootstrap_parity.pilot` calls it twice. `selfhost/parser.pilot` skips struct and interface bodies and keeps function signatures as raw text.
- **Deliverables:**
  1. The self-hosted front end parses the full language (fields, parameters, types, generics, methods, attributes) into the same node set the seed parser produces.
  2. A driver that runs lex → parse → expand → analyze (`F4`) → lower (`F5`) → Perceus (`F7`) → C11 (`F6`) entirely in Pilot.
  3. Stage 1 (the seed) builds `stage2` from `selfhost/` sources; `stage2` builds `stage3` from the same sources.
  4. `selfhost/` migrated off raw `__engine("map_*")` string-keyed tables to typed structures, with the `GATE010` type-hygiene gate that `M1` specified actually implemented.
- **Done when:** `sha256(stage2) == sha256(stage3)` where both are real compiler binaries built from the full `selfhost/` tree; the conformance corpus passes on `stage3`; the rename check from `F4` passes on `stage3`; Rust seed contraction is resumed (`seed_contraction_paused()` set to `false` in `selfhost/gate/manifest.pilot`, with ceilings re-set from the live seed).
- **Deletes:** the template body of `selfhost/pilotc.pilot` and the synthetic-unit parity check in `bootstrap_parity.pilot`.

### Phase F Exit Checkpoint & What It Unblocks

Before advancing to **Phase 4 (`M31`)**:
- **Gate 1 (Adaptability & Hygiene):** rename check executes real programs after renaming core types; raw `__engine` use, duplicate types and Ring 1 isolation are enforced by the gate, not just specified.
- **Gate 2 (Bootstrap & Speed):** real Stage-2 == Stage-3 parity (`F8`); compile time for the `selfhost/` tree recorded as the baseline.
- **Gate 3 (Performance & Leaks):** benchmark baseline against C recorded (`F6`); zero live allocations at exit across the corpus (`F7`).

| Prerequisite | Unblocks |
| :--- | :--- |
| `F1`–`F3` | Re-verifying `M3`, `M4`, `M8` through the CLI; re-doing `M21`–`M23`, `M26`, `M27`, `M30` as real language features |
| `F4` | `M22` integration, `M26` typestates and contracts, `M28` coach, `M43` LSP v2, `M45` MCP |
| `F5` | `M6` (becomes `[STATUS: INTEGRATED]`), `M23` fusion, `M25` SoA rewrite, `M31`, `M32`, `M33`, `M34`, `M35`, `M37`, `M48` |
| `F6` | `M13` native frame calls, `M12` `use c` end to end, `M33`, `M39`, `M40`, `M46` differential testing, `M48` |
| `F7` | `M14` foreign handles and guards, `M31`, `M32`, `M36` VRAM pooling, `M46` leak certification |
| `F8` | `M38` cutover, `M39` C11 seed, `M49` edition freeze, `M50` release, `M89`, `M90` |

`M1`–`M30` are not discarded: each should be revisited after its prerequisite lands and moved to `[STATUS: INTEGRATED]` by adding CLI-level acceptance tests and replacing any simulated parts. `Audit.md` §9 lists the existing code that is worth keeping.

---

## Phase 4 (`M31` – `M40`): Peak Performance, Hardware Backends & Zero-Rust Decoupling (UNABRIDGED SPECIFICATION)

> **Blocked on Phase F.** Every milestone in this phase is specified over `PilIR`, a type checker, a runtime ownership model or a native backend. Those are delivered by `F4`–`F8`. See the prerequisite table at the end of Phase F before starting any of `M31`–`M40`.

> **Unabridged Specification Contract:** Phase 4 (`M31`–`M40`) transforms Pilot into a C/Rust-beating native systems compiler and completes the **Zero-Rust Toolchain Decoupling**. It upgrades Perceus to interprocedural borrow inference and general CFG `drop_reuse` recycling (`M31`), ships composable `0b11` scoped arenas and whole-call-graph `noalloc` verification (`M32`), delivers an optimizing LLVM 18+ backend (`M33`), lowers `simd<T, N>` across AVX2/AVX-512/NEON/WASM128 via name-blind register physics (`M34`), emits sub-millisecond direct x86_64 & ARM64 machine code binaries without external assemblers (`M35`), compiles `[Kernel]` functions to 6 GPU targets with VRAM pooling (`M36`), lowers `grad` (`!@`) and `sig` (`!^`) into zero-allocation `PilIR` (`M37`), retires the legacy Rust lexer/parser/analysis crates (`M38`), generates the standalone C11 bootstrap seed (`M39`), and certifies C/Rust hardware-counter parity via `pilot bench` (`M40`). Every milestone below specifies the complete engineering blueprint leaving only literal code bodies omitted.

---

### M31 — Interprocedural Perceus Borrow Inference & `drop_reuse` Recycling (`selfhost/perceus/`)

#### 31.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Intraprocedural AST-Only Perceus & String-Map State in `arc.pilot`):** [`selfhost/perceus/arc.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/perceus/arc.pilot) (lines `490`–`671`) schedules `Dup`, `Drop`, and `DropReuse` by walking flat AST statement children (`__engine("ast_child_at", body_id, s)`) and storing per-variable liveness in untyped string-keyed maps (`__engine("map_set", state, "vtot_" + vn, ...)`), violating `imp_type.md` Rule 10 (Never Use Raw `__engine` Directly). Furthermore, it has **zero interprocedural visibility**: passing an owned `0b10` `List<T>` or `SliceBuffer<T>` to a read-only helper `fn sum(items: List<i64>) -> i64` forces an unnecessary `op_perceus_dup` before the call or consumes the caller's buffer even when `sum` never stores, moves, or returns `items`!
- **Root Bottleneck 2 (Adjacent-Statement-Only `drop_reuse` Window):** In [`selfhost/perceus/arc.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/perceus/arc.pilot) (lines `576`–`615`), `emit_drop_reuse_op` only matches a dead variable in statement `s` against an allocation in the *immediately following* AST statement `next_s = last_s + 1` with identical scalar AST size (`vsz == next_sz`). It cannot recycle constructor slots across `PilIR` (`M6`) SSA `switch`/`if` branch arms (e.g., functional tree balancing `Node { left, right, val }` or `List` map-in-place) or propagate reuse tokens through phi/block parameters!

#### 31.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/perceus/borrow_infer.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Perceus.BorrowInfer;` | `~360L` | Interprocedural SCC fixed-point analyzer over `PilIR` (`M6`) inferring `ParamOwnershipSummary` (`::ReadOnlyBorrow0b01`, `::MutInPlaceBorrow0b01`, `::ConsumedOwned0b10`) per parameter and eliding caller `op_perceus_dup`/`op_perceus_drop` pairs (`imp_type.md` Rule 7). |
| **CREATE** | `selfhost/perceus/drop_reuse.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Perceus.DropReuse;` | `~390L` | Dominator-tree & branch-aware constructor slot recycler matching dead `0b10` `op_perceus_drop` sites with downstream `op_construct_reuse` allocations of identical `(size_bytes, align_bytes)` across basic blocks and recursive updates. |
| **CREATE** | `selfhost/perceus/interproc.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Perceus.Interproc;` | `~290L` | Tarjan SCC call-graph builder, `IPilIrPass` integration (`pass_perceus_interproc`), uniqueness (`refcount == 1`) static specialization, and `pilot inspect --perceus` telemetry emitter. |
| **MODIFY** | `selfhost/perceus/arc.pilot` & `selfhost/perceus/optimizer.pilot` | Self-Host Perceus | `-110L / +65L` | Replace raw `__engine("map_*")` string-concatenated liveness tables with strongly-typed `[Layout(c_abi)]` `PilIR` SSA liveness vectors and wire `interproc.pilot` into the unified Perceus pipeline. |

#### 31.3 Complete Data Structures, Lattice Transfer Equations & `imp_type.md` Compliance
- **`ParamOwnershipMode` (`enum : u8`, `0b00 : IValue`, `imp_type.md` Rule 1)**:
  - `::TrivialValue0b00 = 0` (`0b00` `: IValue` register copy)
  - `::ReadOnlyBorrow0b01 = 1` (Parameter is only read/indexed; downgraded from `0b10` owned move to zero-cost `0b01` borrowed view!)
  - `::MutInPlaceBorrow0b01 = 2` (Parameter is mutated in-place without escaping or reallocating; passed as `var ref` `0b01`)
  - `::ConsumedOwned0b10 = 3` (Parameter escapes into return value, struct field, channel `cue`, or consuming callee)
  - `::ScopedGuard0b11 = 4` (`0b11` `: IGuard` LIFO scope-bound lease).
- **`PerceusEffectFlags` (`bitflags : u16`, `0b00 : IValue`, `imp_type.md` Rule 5)**:
  - `::None = 0`, `::ReadsMemory = 1`, `::MutatesPayload = 2`, `::EscapesToReturn = 4`, `::EscapesToFieldOrGlobal = 8`, `::PassedToConsumingCall = 16`, `::ReallocatesBuffer = 32`, `::EligibleForDropReuse = 64`.
- **`[Layout(c_abi)] pub struct FnPerceusSummary : IValue`** (`0b00`):
  - `pub fn_ir_id: u32; pub scc_id: u32; pub param_modes: array<ParamOwnershipMode, 16>; pub param_effects: array<PerceusEffectFlags, 16>; pub param_count: u8; pub returns_fresh_0b10: bool; pub is_converged: bool;`
- **`[Layout(c_abi)] pub struct ReusePairSite : IValue`** (`0b00`):
  - `pub drop_block_id: u32; pub drop_inst_idx: u32; pub dead_val_id: u32; pub alloc_block_id: u32; pub alloc_inst_idx: u32; pub target_val_id: u32; pub slot_size_bytes: u32; pub slot_align_bytes: u16; pub requires_field_drops_mask: u64;`
- **The Interprocedural 2-Bit Boundary Promotion Law (`imp_type.md` Rule 7):**
  For any parameter $p_k$ of declared `0b10` owned type $T$ in function $f$:
  $$\text{Mode}(f, p_k) = \begin{cases} \text{::ConsumedOwned0b10} & \text{if } \text{Effects}(f, p_k) \cap (\text{::EscapesToReturn} \mid \text{::EscapesToFieldOrGlobal} \mid \text{::PassedToConsumingCall} \mid \text{::ReallocatesBuffer}) \neq \emptyset \\ \text{::MutInPlaceBorrow0b01} & \text{else if } (\text{Effects}(f, p_k) \cap \text{::MutatesPayload}) \neq \emptyset \\ \text{::ReadOnlyBorrow0b01} & \text{otherwise} \end{cases}$$

#### 31.4 End-to-End Algorithmic Pipeline & Complexity Bounds
1. **Tarjan SCC Call-Graph Decomposition & Bottom-Up Fixed-Point ($O(V + E)$):**
   - Decomposes the module call graph into strongly connected components (`SCC`s). Initializes all recursive `0b10` parameters inside an SCC optimistically to `::ReadOnlyBorrow0b01` and iterates transfer equations until fixed-point convergence (at most $3 \times |\text{Params}_{\text{SCC}}|$ steps since `::ReadOnlyBorrow0b01` $\prec$ `::MutInPlaceBorrow0b01` $\prec$ `::ConsumedOwned0b10` forms a height-2 monotone lattice!).
2. **Caller `Dup`/`Drop` Pair Elision ($O(I)$ over `PilIR` instructions):**
   - When caller $C$ invokes $f(\text{arg}_k)$ where $\text{Mode}(f, k) == \text{::ReadOnlyBorrow0b01}$ and $C$ still uses $\text{arg}_k$ after the call:
     - Elides the pre-call `op_perceus_dup(arg_k)` in $C$ AND elides the callee-exit `op_perceus_drop(param_k)` in $f$, passing `arg_k` as a `0b01` borrowed register view (`0` atomic refcount increments/decrements!).
3. **Branch-Aware Dominator `drop_reuse` Pairing (`drop_reuse.pilot`):**
   - Scans `PilIR` basic blocks for `op_perceus_drop(x)` where `x` has `0b10` layout `(size_bytes: S, align_bytes: A)`.
   - Searches forward within the single-entry single-exit (`SESE`) region dominated by `drop_block_id` for a constructor allocation `y = alloc(S, A)` where `x` does not alias any live operand of `y`'s sub-expressions.
   - Rewrites `op_perceus_drop(x)` $\rightarrow$ `tok = op_perceus_drop_reuse(x, field_drop_mask)` (dropping only non-reused `0b10` child fields of `x` while recycling the outer $S$-byte heap node if `refcount(x) == 1`!) and rewrites `y = alloc(S, A)` $\rightarrow$ `y = op_construct_reuse(tok)`.
   - **Store Elimination on Unchanged Fields:** If `y` writes the exact same SSA value `v` to byte offset `off` that `x` already held before `op_perceus_drop_reuse(x)`, elides the redundant memory store on the fast-path (`tok != null`)!

#### 31.5 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `PERC001` (Inferred `0b01` Borrow View Cannot Escape Into `0b10` Owned Field or Global), `PERC002` (Aliased Mutable Borrow Hazard: Same `0b10` Binding Passed Simultaneously as `::MutInPlaceBorrow0b01` and `::ReadOnlyBorrow0b01` to Call — Automatically Retains `op_perceus_dup` CoW Split!), `PERC003` (`drop_reuse` Slot Size or Alignment Mismatch Blocked).
- **Verification Suite (`crates/pilot-analysis/tests/milestone31_interprocedural_perceus_borrow_inference.rs`):**
  1. `test_m31_1_interprocedural_readonly_borrow_inference_elides_dup_drop`: 3-level call chain (`main -> analyze -> summarize -> inspect_len`) passing an owned `List<i64>` that is used again in `main` $\rightarrow$ proves `0` `op_perceus_dup` and `0` intermediate `op_perceus_drop` instructions emitted!
  2. `test_m31_2_mut_recursion_scc_fixed_point_and_escape_promotion`: Mutually recursive `even_filter` / `odd_filter` where one branch stores the parameter into a returned struct $\rightarrow$ proves SCC fixed-point promotes both to `::ConsumedOwned0b10` and preserves memory safety (`PERC001`).
  3. `test_m31_3_branch_aware_drop_reuse_in_functional_tree_insert`: Functional red-black / binary tree insert `Node { key, left, right }` $\rightarrow$ proves `op_perceus_drop_reuse` + `op_construct_reuse` recycles `100%` of popped parent nodes in place with `0` new heap allocations on path updates!
  4. `test_m31_4_aliased_cow_hazard_detection_perc002`: Calling `merge_in_place(var ref buf, buf)` where `buf` is aliased across two arguments $\rightarrow$ triggers `PERC002` CoW safeguard so `buf` is never mutated while borrowed.

---

### M32 — First-Class Scoped Arenas (`with arena`) & Static `noalloc` Verifier

#### 32.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`ArenaScope` in `stdlib/core/mem.pilot` Is a 5-Line Empty Stub!):** [`stdlib/core/mem.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/mem.pilot) (lines `379`–`384`) defines `ArenaScope : IGuard { pub region_view: slice<u8>; ~self() {} }` with **zero bump pointer**, **zero watermark save/restore**, **zero slab/pool/ring allocator modes**, and **zero `context` injection wiring** into `List<T>`, `Map<K, V>`, or `SliceBuffer<T>`!
- **Root Bottleneck 2 (Shallow Single-Function `noalloc` Check in `realtime.pilot`):** [`selfhost/perceus/realtime.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/perceus/realtime.pilot) (lines `140`–`231`) only checks direct AST children of a single `[HardRealtime]` / `noalloc` function for obvious heap literals. It does **not** perform whole-call-graph transitive verification across indirect calls, generic instantiations, or `.pchor` role bindings, nor does it distinguish forbidden global heap allocations (`malloc`) from deterministic $O(1)$ pre-allocated `0b11` `ArenaScope` bump allocations!

#### 32.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/core/arena.pilot` | `pub lvl(0) pkg core.arena;` | `~390L` | Freestanding Ring-1 `IAllocator` contract + `0b11` `: IGuard` scoped allocators: `ArenaScope` (64-byte aligned bump + `watermark` rewind), `FixedPoolScope<T>` ($O(1)$ freelist slab), and `ScratchRingScope` (power-of-two wrap-around frame buffer) (`imp_type.md` Rule 7). |
| **CREATE** | `selfhost/perceus/noalloc_verifier.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Perceus.NoAllocVerifier;` | `~360L` | Whole-call-graph transitive `[ZeroAlloc]` / `noalloc` & `0b11` arena-escape verifier (`ALLOC001`–`ALLOC005`) over `PilIR` (`M6`). |
| **MODIFY** | `stdlib/core/mem.pilot` & `selfhost/perceus/realtime.pilot` | Core Memory & Realtime | `-25L / +70L` | Replace the 5-line stub `ArenaScope` in `mem.pilot` with `core.arena` exports and connect `realtime.pilot` to `noalloc_verifier.pilot`. |

#### 32.3 Complete Allocator Physics, `with arena` Context Injection & 2-Bit Lattice Rules
- **`AllocatorStrategy` (`enum : u8`, `0b00 : IValue`)**: `::GlobalHeap = 0`, `::BumpArena = 1`, `::FixedSizePool = 2`, `::ScratchRing = 3`, `::StackInlineBuffer = 4`.
- **`AllocErrorKind` (`enum : u8`, `0b00 : IValue`)**: `::OutOfCapacity = 1`, `::AlignmentOverflow = 2`, `::PoolSlotMismatch = 3`, `::PinnedLeaseActive = 4`.
- **`[Layout(c_abi)] pub struct ArenaScope : IGuard`** (`0b11`, 48 Bytes):
  - `pub base_ptr: ptr<u8>; pub capacity: usize; pub offset: usize; pub peak_offset: usize; pub parent_watermark: usize; pub active_child_leases: u32; pub strategy: AllocatorStrategy; pub flags: u8;`
  - **1-Instruction Bump Allocation (`alloc_aligned(size: usize, align: usize) -> res<ptr<u8>, AllocErrorKind>`):**
    $$\text{aligned\_off} = (\text{self.offset} + (\text{align} - 1)) \ \& \ \sim(\text{align} - 1), \quad \text{next\_off} = \text{aligned\_off} + \text{size}$$
    If $\text{next\_off} > \text{self.capacity}$, returns `err(::OutOfCapacity)` in `0` cycles of OS syscall overhead (`noalloc`-safe!). Otherwise updates `self.offset = next_off` and returns `ok(self.base_ptr + aligned_off)`.
  - **LIFO Scope-Exit Destructor (`~self()`):** Resets `self.offset = self.parent_watermark` in **1 CPU store instruction**, freeing all intermediate objects allocated within the lexical `with arena` block simultaneously (`0` per-object `free()` calls!).
- **Contextual Allocator Injection (`with context (Allocator = arena) { ... }`):**
  - Standard library `List<T>`, `Map<K, V>`, and `SliceBuffer<T>` read the ambient `context Allocator` handle at construction:
    - When bound to a `0b11` `ArenaScope`, their `lattice_bits` automatically acquire the **Regional Borrow Bit (`0b01`)** (`0b10 | 0b01 = 0b11` scoped lifetime!), statically forbidding any collection allocated inside `with arena` from escaping the `with arena` lexical block (`ALLOC003` / `ESCAPE004`)!

#### 32.4 Transitive Call-Graph `noalloc` Verification (`noalloc_verifier.pilot`)
1. **Whole-Graph Effect Propagation ($O(V + E)$):**
   - Computes `AllocSummary` for every `PilIR` function:
     - `::PureZeroAlloc`: Performs zero heap allocations and calls only `::PureZeroAlloc` functions.
     - `::ScopedArenaOnly`: Allocates exclusively from a caller-provided or stack-backed `0b11` `ArenaScope` (`with arena: u8[64KB]`) with bounded capacity (`ok ! AllocErrorKind`), never touching the global OS allocator.
     - `::UnboundedGlobalHeap`: Invokes global `malloc`/unbounded `List.push` without an arena or calls an unverified `extern` function (`ALLOC001`).
2. **Function Pointer & Interface Dispatch Soundness (`ALLOC002`):**
   - Inside a `noalloc` / `[ZeroAlloc]` / `[HardRealtime]` function, invoking a `fn(A) => R` closure or `: IInterface` method is permitted **if and only if** the function type or interface method signature itself carries the `noalloc` contract!

#### 32.5 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `ALLOC001` (`noalloc` / `[ZeroAlloc]` Function Calls Transitive Callee Performing Global Heap Allocation — Prints Full Call Chain `f -> g -> h`), `ALLOC002` (Indirect Call or Interface Dispatch Inside `noalloc` Target Lacks `noalloc` Signature Contract), `ALLOC003` (Object Allocated in `0b11` `ArenaScope` Escapes `with arena` Lexical Block via Return or Outer Assignment), `ALLOC004` (`ArenaScope.reset()` or Rewind Attempted While `0b01` Borrowed Slice into Arena Remains Live), `ALLOC005` (`FixedPoolScope<T>` Allocation Request Size Exceeds Fixed Slot Stride).
- **Verification Suite (`crates/pilot-analysis/tests/milestone32_scoped_arenas_and_noalloc_verifier.rs`):**
  1. `test_m32_1_arena_scope_bump_align_watermark_and_lifo_reset`: Allocates 1,000 mixed-alignment (`8B`, `16B`, `64B` SIMD) buffers inside nested `ArenaScope` watermark sub-scopes $\rightarrow$ verifies exact 64B alignment, $O(1)$ LIFO watermark rewind, and `::OutOfCapacity` graceful error when capacity is exceeded.
  2. `test_m32_2_fixed_pool_and_scratch_ring_allocators`: Exercises `FixedPoolScope` ($O(1)$ slot reuse after free) and `ScratchRingScope` (power-of-two wrap-around packet buffer).
  3. `test_m32_3_transitive_call_graph_noalloc_verifier_alloc001_alloc002`: Verifies a 4-hop call chain `audio_callback [noalloc] -> dsp_step -> filter_tap -> log_debug (global alloc)` is caught with `ALLOC001` showing the exact 4-hop trace, while switching `filter_tap` to stack-backed `ArenaScope` passes `noalloc` cleanly!
  4. `test_m32_4_arena_allocated_collection_escape_blocked_alloc003`: Attempts to return a `List<i64>` constructed inside `with arena` to the outer scope $\rightarrow$ statically blocked by `ALLOC003`.

---

### M33 — Optimizing Native LLVM 18+ Backend (`selfhost/codegen/llvm.pilot`)

#### 33.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`llvm_emitter.pilot` Lowers Untyped AST Instead of `PilIR` and Hardcodes `i64` Stack Slots):** [`selfhost/codegen/llvm_emitter.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/codegen/llvm_emitter.pilot) (lines `94`–`240`) walks raw AST nodes via `__engine("ast_node_kind", nid)` and emits naive `%slot_x = alloca i64` / `load i64, ptr %slot_x` for variables, ignoring `PilIR` (`M6`) typed SSA block parameters, `i128`/`f16`/`f128`/`simd<T, N>` register physics (lines `16`–`35`), and struct `[Layout(c_abi)]` byte offsets!
- **Root Bottleneck 2 (Zero 2-Bit Lattice Alias Metadata, Zero Tail Calls, Zero DWARF v5 & Single-Host Target Assumption):** `llvm_emitter.pilot` emits **zero** `noalias` / `readonly` / `nocapture` / `nonnull` parameter attributes—leaving LLVM blind to Pilot's strict `0b01` borrowed view and `0b10` `iso<T>` uniqueness guarantees! It also lacks `musttail` tail-call emission, `!llvm.loop` vectorization metadata, DWARF v5 `!DICompileUnit` debug records (needed for `M44` `pilot debug`), and multi-platform target data layouts (`x86_64`, `aarch64`, `riscv64`, `wasm32-wasip2`, `thumbv7em`).

#### 33.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/codegen/llvm_target.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Codegen.LlvmTarget;` | `~310L` | Open target registry for 10 cross-compilation triples (`linux-x64/arm64/riscv64`, `macos-arm64/x64`, `windows-x64/arm64`, `wasm32-wasip2`, `thumbv7em-none-eabihf`, `riscv32imac-unknown-none-elf`), data-layout strings, and ABI sret/byval lowering rules (`imp_type.md` Rule 2). |
| **CREATE** | `selfhost/codegen/llvm_alias_debug.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Codegen.LlvmAliasDebug;` | `~360L` | Translates 2-bit lattice (`0b00`–`0b11`) and `iso` contracts into LLVM `noalias`, `readonly`, `nocapture`, `dereferenceable(N)`, `!alias.scope` / `!noalias` metadata, `!llvm.loop` hints, and DWARF v5 (`!DICompileUnit`, `!DISubprogram`, `!DILocation`) debug trees. |
| **MODIFY** | `selfhost/codegen/llvm_emitter.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Codegen.LlvmEmitter;` | `-180L / +420L` | Upgrade from untyped AST walker to full `PilIR` (`M6`) SSA module emitter supporting `i8`..`i128`, `f16`..`f128`, `<N x T>` SIMD vectors, `phi` nodes from block parameters, `musttail` calls, and ThinLTO summary attributes. |

#### 33.3 Complete 2-Bit Lattice $\rightarrow$ LLVM 18+ Alias & Optimization Physics
Every parameter and memory operation in `PilIR` derives its LLVM 18+ optimization metadata strictly from its **2-bit hardware lattice (`0b00`–`0b11`)** and layout descriptor (`0` hardcoded type names!):

| Pilot Parameter / Binding Physics | Derived LLVM 18+ Parameter & Memory Attributes | Resulting LLVM Optimizer Unlock |
| :--- | :--- | :--- |
| **`0b00 : IValue`** (`<= 16B`, 1–2 registers) | Passed directly in SSA registers (`i64`, `double`, `{ i64, i64 }`, `<4 x float>`) with `noundef` | Zero stack `alloca`, 100% register promotion |
| **`0b00 : IValue`** (`> 16B` struct, read-only) | `ptr noalias nocapture readonly align(A) dereferenceable(S)` | Loop-invariant code motion (LICM) hoists all struct loads out of inner loops! |
| **`0b01 : IReference` (`slice<T>` / `ref<T>`)** | `{ ptr nocapture readonly align(A), i64 noundef }` + `!alias.scope` disjoint from mutable `var ref` / `iso` buffers | Enables automatic SLP & loop vectorization without runtime alias checks! |
| **`var ref<T>` (`0b01` mutable borrow)** | `ptr noalias nocapture nonnull align(A) dereferenceable(S)` | Guarantees exclusive write access (`restrict` semantics in C, safe `noalias` in LLVM) |
| **`iso<T>` (`0b10` unique owned object)** | `ptr noalias nonnull align(A)` + `!noalias` metadata on all inner heap loads/stores | Proves zero external aliasing across the entire function body! |
| **`0b11 : IGuard` (`ArenaScope`, `MutexGuard`)** | `ptr noalias nocapture align(A)` + `cleanup` landingpad / LIFO scope exit block | Deterministic unwind & scope-exit lowering |

#### 33.4 Cross-Compilation Target Matrix & DWARF v5 Pipeline
1. **Block-Parameter SSA $\rightarrow$ LLVM `phi` Lowering:**
   - Each `PilIR` `BasicBlock(b_id, params: [v_1: T_1, ...])` with predecessors $P_1, \dots, P_m$ lowers directly to LLVM `phi` instructions at the top of label `bb_<b_id>`:
     `%v_1 = phi <llvm_ty> [ %arg_from_P1, %bb_P1 ], [ %arg_from_P2, %bb_P2 ]`.
2. **Guaranteed Tail-Call Optimization (`musttail`):**
   - Recursive calls in tail position whose caller and callee share ABI register signatures emit `ret <ty> musttail call fastcc <ty> @callee(...)`, guaranteeing $O(1)$ stack frames for state machines and bytecode dispatchers.
3. **DWARF v5 Source-Line & Lattice Type Metadata (`llvm_alias_debug.pilot`):**
   - Emits `!DICompileUnit(language: DW_LANG_C_plus_plus_14, producer: "pilot 1.0")`, `!DICompositeType` with exact byte offsets and `DW_TAG_member` entries from `PhysicalFieldSlot` (`M22`), and `!dbg !DILocation(line: L, column: C)` on every instruction.

#### 33.5 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `LLVM001` (Unsupported Target Triple or Missing ABI Layout Descriptor), `LLVM002` (`musttail` Tail Call Signature or Stack-Escape Mismatch), `LLVM003` (Invalid Inline `asm` Clobber or Constraint Register for Target Architecture), `LLVM004` (Malformed DWARF Debug Scope Nesting).
- **Verification Suite (`crates/pilot-analysis/tests/milestone33_optimizing_native_llvm_backend.rs`):**
  1. `test_m33_1_lattice_driven_noalias_readonly_and_sroa_emission`: Lowers a `PilIR` kernel taking `0b01` `slice<f64>`, `var ref<Vec3>`, and `iso List<i64>` $\rightarrow$ verifies emitted LLVM IR contains exact `noalias`, `readonly`, `nocapture`, `dereferenceable`, and `!llvm.loop` vectorization metadata with `0` hardcoded type names!
  2. `test_m33_2_block_params_to_phi_and_musttail_recursion`: Compiles a tail-recursive state machine and multi-predecessor loop in `PilIR` to LLVM IR and executes it via `clang -O3 -x ir -` $\rightarrow$ verifies exact numerical output and `musttail` instruction presence.
  3. `test_m33_3_cross_compilation_ten_target_triples_and_data_layouts`: Emits LLVM IR across Linux (`x86_64`, `aarch64`, `riscv64`), macOS (`arm64-apple-darwin`), Windows (`x86_64-pc-windows-msvc`), WASM (`wasm32-wasip2`), and bare-metal (`thumbv7em`) $\rightarrow$ verifies target triples, pointer widths (`32`-bit vs `64`-bit), and struct ABI lowering.
  4. `test_m33_4_dwarf_v5_debug_info_and_struct_member_offsets`: Verifies emitted `!DICompileUnit`, `!DISubprogram`, `!DICompositeType`, and `!DILocation` metadata match source spans and `M22` byte offsets 1-for-1.

---

### M34 — Portable Hardware SIMD Vectorization (`simd<T, N>` / `T@N`)

#### 34.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`simd<T, N>` in `stdlib/core/mem.pilot` Only Defines `+`, `-`, `*` and Indexing!):** [`stdlib/core/mem.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/mem.pilot) (lines `257`–`265`) and [`stdlib/core/primitives/simd.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/primitives/simd.pilot) (lines `1`–`22`) only define `[]`, `+`, `-`, `*` on `simd<T, N>`. They lack `splat`, `fma` (fused multiply-add), horizontal reductions (`reduce_add`, `reduce_min`, `reduce_max`), lane-wise comparisons (`simd_eq`, `simd_lt` returning lane masks!), `select`/`blend`, `shuffle`/`swizzle`, and aligned/unaligned/masked slice loads and stores!
- **Root Bottleneck 2 (Hardcoded `f64x4` AVX2 String Pattern in `asm_emitter.pilot`):** [`selfhost/codegen/asm_emitter.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/codegen/asm_emitter.pilot) (lines `350`–`460`) only emits a single fixed AVX2 `vaddpd`/`vmulpd` template and has **zero** support for AVX-512 (`zmm`, `k1` masks), ARM64 NEON (`v0.4s`, `fmla`), or WebAssembly SIMD128 (`v128.load`, `f32x4.add`), and no `PilIR` loop auto-vectorizer!

#### 34.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **MODIFY** | `stdlib/core/primitives/simd.pilot` | `pub lvl(0) pkg Pilot.Core.Simd;` | `+340L` | Complete portable `simd<T, N>` (`T@N`) facet: `splat`, `/`, `&`, `|`, `^`, `<<`, `>>`, `fma`, `sqrt`, `abs`, `min`, `max`, `reduce_add/mul/min/max/and/or`, `lanes_lt/le/eq`, `Mask<N>.select(a, b)`, `shuffle<I...>`, `load_aligned`, `load_unaligned`, `store_aligned`, `load_masked`. |
| **CREATE** | `selfhost/ir/pass_loop_vectorize.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Ir.LoopVectorize;` | `~390L` | Name-blind `PilIR` (`M6`) stride-1 loop & SLP vectorizer converting scalar `FixedScalar` loops (`M23`/`M25` `SoA` columns!) into vector body + scalar remainder epilogue (`imp_type.md` Rule 3). |
| **CREATE** | `selfhost/codegen/simd_isa.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Codegen.SimdIsa;` | `~420L` | Table-driven hardware SIMD instruction selector mapping `(lane_reg_class, lane_bits, lane_count, simd_op)` to **x86_64 SSE4.2/AVX2/AVX-512**, **ARM64 NEON**, and **WASM SIMD128** (`0` hardcoded type names!). |

#### 34.3 Name-Blind Register Physics (`[Bits]` & `[Layout]`) $\rightarrow$ Hardware ISA Mapping
Per `AGENTS.md §1` Mechanism 1 and `imp_type.md` Rule 3, `simd_isa.pilot` never matches type names like `"f32x8"` or `"i64x4"`. It inspects **only** `(lane_reg_class, lane_bit_width, lane_count)`:
- $\text{TotalVectorBits} = \text{lane\_bit\_width} \times \text{lane\_count} \in \{64, 128, 256, 512\}$
- **Legalization by Target Vector Width ($W_{\text{target}}$):**
  - If $\text{TotalVectorBits} == W_{\text{target}}$ (e.g., `4 x 32b = 128b` on NEON/WASM128, `8 x 32b = 256b` on AVX2, `16 x 32b = 512b` on AVX-512): maps 1-to-1 to a single physical vector register (`xmm`/`ymm`/`zmm`/`v0`/`v128`).
  - If $\text{TotalVectorBits} == 2 \times W_{\text{target}}$ (e.g., `f32@8` on ARM64 NEON 128-bit): automatically splits into a register pair `(v_lo, v_hi)` in `PilIR` (`0` scalar fallback!).

| `(lane_reg_class, lane_bits, lanes)` | `op_simd_lane_op` | x86_64 AVX2 / AVX-512 | ARM64 NEON (AArch64) | WebAssembly SIMD128 |
| :--- | :--- | :--- | :--- | :--- |
| `(simd_float, 32, 4)` (`128b`) | `Add` / `Fma` | `vaddps xmm` / `vfmadd213ps xmm` | `fadd v.4s` / `fmla v.4s` | `f32x4.add` / `f32x4.mul+add` |
| `(simd_float, 32, 8)` (`256b`) | `Add` / `Fma` | `vaddps ymm` / `vfmadd213ps ymm` | Pair `(fadd v0.4s, fadd v1.4s)` | Pair `2x f32x4.add` |
| `(simd_float, 64, 4)` (`256b`) | `Add` / `ReduceAdd` | `vaddpd ymm` / `vhaddpd+vextractf128` | Pair `(fadd v0.2d, fadd v1.2d)` | Pair `2x f64x2.add` |
| `(gpr_signed/u, 32, 4)` (`128b`) | `Add` / `Mul` | `vpaddd xmm` / `vpmulld xmm` | `add v.4s` / `mul v.4s` | `i32x4.add` / `i32x4.mul` |
| `(gpr_unsigned, 8, 16)` (`128b`) | `Add` / `CmpEq` | `vpaddb xmm` / `vpcmpeqb xmm` | `add v.16b` / `cmeq v.16b` | `i8x16.add` / `i8x16.eq` |

#### 34.4 Automatic Loop Vectorization (`pass_loop_vectorize.pilot`)
1. **Dependence & Stride Analysis:**
   - Identifies `PilIR` natural loops with trip count $N$, unit-stride (`byte_step == sizeof(Elem)`) `0b01` slice or `M25` `SoA` column loads, and side-effect-free loop bodies (or associative reductions `+`, `*`, `min`, `max`, `&`, `|`, `^`).
2. **Vector Body + Remainder Epilogue Generation:**
   - For vector factor $VF = W_{\text{target}} / \text{lane\_bits}$, emits:
     - **Main Vector Loop (`0 .. (N & ~(VF - 1)) : VF`):** Executes `op_simd_lane_op` on $VF$ lanes per iteration (`unroll_factor = 2` or `4` to saturate dual FMA ports).
     - **Scalar/Masked Epilogue (`(N & ~(VF - 1)) .. N`):** On AVX-512, uses hardware tail mask `k1 = (1 << rem) - 1`; on AVX2/NEON/WASM128, runs a clean scalar cleanup loop for the remaining `< VF` elements.

#### 34.5 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `SIMD001` (`simd<T, N>` Lane Count $N$ Must Be a Power of Two in `2..=64`), `SIMD002` (`simd<T, N>` Element Type Must Be a `0b00` Hardware Scalar Register Class — Cannot Vectorize `0b10` Owned Objects), `SIMD003` (`shuffle` Index Out of Range `0..N`), `SIMD004` (`[SimdRequire]` Annotated Loop Cannot Be Vectorized Due to Loop-Carried Backward Memory Dependency at Offset $< VF$).
- **Verification Suite (`crates/pilot-analysis/tests/milestone34_portable_hardware_simd_vectorization.rs`):**
  1. `test_m34_1_portable_simd_arithmetic_fma_reductions_and_swizzles`: Exercises `f32@4`, `f64@4`, `i32@8`, `u8@16` (`splat`, `fma`, `reduce_add`, `reduce_max`, `select`, `shuffle`) and verifies bit-identical results across PilVM and native codegen.
  2. `test_m34_2_name_blind_isa_selection_avx2_avx512_neon_wasm128`: Verifies `simd_isa.pilot` emits exact AVX2 (`vfmadd213ps %ymm`), AVX-512 (`%zmm`), ARM64 NEON (`fmla v0.4s`), and WASM SIMD128 (`f32x4.add`) instructions strictly from `(reg_class, bit_width, lanes)` with `0` hardcoded type strings!
  3. `test_m34_3_auto_vectorization_of_soa_and_slice_loops_with_tail_epilogue`: Vectorizes a non-multiple-of-8 (`N = 1003`) `SoA` particle velocity update + dot-product reduction loop $\rightarrow$ verifies main vector loop + 3-element remainder epilogue produce exact answers.
  4. `test_m34_4_simd_diagnostics_non_power_of_two_and_backward_dependency_simd001_004`: Verifies `SIMD001`–`SIMD004` trigger accurately on invalid lane counts and loop-carried dependencies.

---

### M35 — Sub-Millisecond Direct x86_64 & ARM64 Native Backend (`selfhost/codegen/asm_*`)

#### 35.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`asm_emitter.pilot` Emits Textual `.s` Strings Requiring External `clang` & Spills Every Temp to Stack):** [`selfhost/codegen/asm_emitter.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/codegen/asm_emitter.pilot) (lines `70`–`180`) emits textual AT&T `.s` strings (`movq -24(%rbp), %rax`) and relies on spawning an external `clang`/`as`/`ld` subprocess to produce an executable binary! Moreover, it only supports x86_64 (zero ARM64 AArch64 support) and spills every intermediate expression to `-(base + depth + 1) * 8(%rbp)` on the stack instead of keeping `0b00` values in hardware registers!

#### 35.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/codegen/regalloc_linear.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Codegen.RegAlloc;` | `~340L` | Single-pass Poletto–Sarkar Linear-Scan Register Allocator over `PilIR` (`M6`) SSA live intervals, mapping `0b00` scalars & 2-register structs (`slice<T>` `{ ptr, len }`) directly into caller/callee-saved GPR and XMM/V registers. |
| **CREATE** | `selfhost/codegen/encode_x86_64.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Codegen.EncodeX64;` | `~410L` | Direct binary machine-code encoder for x86_64 (`REX.W`, `ModR/M`, `SIB`, `disp8/disp32`, `imm64`, `VEX` 2/3-byte SIMD prefixes, relative jump relaxation) emitting raw `u8` instruction bytes at $>500\text{K LOC/s}$. |
| **CREATE** | `selfhost/codegen/encode_arm64.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Codegen.EncodeArm64;` | `~380L` | Direct 32-bit fixed-width AArch64 A64 binary instruction encoder (`ADD/SUB/MUL`, `MOVZ/MOVK`, `LDR/STR` pair `LDP/STP`, `B/BL/CBZ/B.cond` 19/26-bit PC-relative branches, `FADD/FMUL` scalar & NEON). |
| **CREATE** | `selfhost/codegen/binfmt_exec.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Codegen.BinFmt;` | `~430L` | Zero-linker standalone executable container writer emitting executable **ELF64** (Linux `x86_64`/`aarch64`), **Mach-O 64** (macOS `arm64`/`x86_64` with `LC_MAIN` & ad-hoc code-signature blob), and **PE32+/COFF** (Windows `x86_64`/`aarch64`) byte buffers directly! |

#### 35.3 Direct Binary Encoding Physics & Linear-Scan Register Allocation
1. **Single-Pass Linear-Scan Register Allocation (`regalloc_linear.pilot`, $O(R \cdot V)$):**
   - Computes live intervals `[start_inst, end_inst]` for every `PilIR` SSA value in 1 backward pass over basic blocks.
   - **x86_64 Scratch/Allocatable Pool:** GPR `[%rax, %rcx, %rdx, %rsi, %rdi, %r8..%r11, %rbx, %r12..%r15]`, Vector `[%xmm0..%xmm15]`.
   - **ARM64 Scratch/Allocatable Pool:** GPR `[x0..x15, x19..x28]`, Vector `[v0..v31]`.
   - Keeps `0b00` value structs $\le 16$ bytes and `0b01` `slice<T>` (`{ ptr, len }`) in paired registers `(r_lo, r_hi)` with `0` stack traffic unless live pressure exceeds physical registers!
2. **Exact Binary Machine-Code Encoding (`encode_x86_64.pilot` & `encode_arm64.pilot`):**
   - **x86_64 Example (`add r64, r/m64`):**
     $$\text{REX} = \text{0x48} \mid (\text{reg}_3 \ll 2) \mid \text{rm}_3, \quad \text{Opcode} = \text{0x01}, \quad \text{ModR/M} = (\text{0b11} \ll 6) \mid ((\text{reg} \& 7) \ll 3) \mid (\text{rm} \& 7)$$
   - **ARM64 A64 Example (`ADD Xd, Xn, Xm` 64-bit):**
     $$\text{Insn}_{32} = (\text{0b10001011000} \ll 21) \mid (m \ll 16) \mid (\text{0b000000} \ll 10) \mid (n \ll 5) \mid d$$
3. **Standalone Executable Emission (`binfmt_exec.pilot`):**
   - **ELF64 (`\x7fELF`):** Emits 64-byte `Elf64_Ehdr` (`e_type = ET_EXEC/ET_DYN`, `e_machine = EM_X86_64 (62)` or `EM_AARCH64 (183)`), `Elf64_Phdr` (`PT_LOAD` `PF_R | PF_X` for `.text` + `.rodata`, `PF_R | PF_W` for `.data`), and patches `_start` entry point (`sys_exit` / `main` prologue).
   - **Mach-O 64 (`0xFEEDFACF`):** Emits `mach_header_64`, `LC_SEGMENT_64 (__TEXT, __DATA)`, `LC_MAIN`, and `LC_CODE_SIGNATURE` SHA-256 page directory so macOS Apple Silicon executes the binary immediately without external `codesign`!
   - **PE32+ (`MZ` + `PE\0\0`):** Emits DOS stub, `IMAGE_FILE_HEADER`, `IMAGE_OPTIONAL_HEADER64` (`Subsystem = IMAGE_SUBSYSTEM_WINDOWS_CUI`), `.text` / `.rdata` / `.idata` import tables (`kernel32.dll` `ExitProcess`, `WriteFile`).

#### 35.4 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `ASM001` (Branch Target Displacement Exceeds 32-Bit Signed Relative Range), `ASM002` (Unsatisfied Symbol Relocation in Direct Executable), `ASM003` (Register Allocation Spill Slot Alignment Overflow).
- **Verification Suite (`crates/pilot-analysis/tests/milestone35_direct_x86_64_and_arm64_native_backend.rs`):**
  1. `test_m35_1_linear_scan_register_allocator_and_zero_spill_0b00_records`: Allocates registers for a 12-variable loop with `0b00` `Vec2` and `0b01` `slice<i64>` $\rightarrow$ verifies 100% register residency (`0` stack spills) and correct spill/reload when live variables exceed 16 GPRs.
  2. `test_m35_2_x86_64_direct_machine_code_encoding_and_branch_relaxation`: Verifies exact byte sequences (`0x48 0x01 ...`, VEX AVX2 encodings, backward loop `jne rel8/rel32` relaxation) emitted by `encode_x86_64.pilot`.
  3. `test_m35_3_arm64_a64_fixed_width_instruction_encoding`: Verifies exact 32-bit little-endian instruction words (`ADD`, `SUB`, `MUL`, `LDP/STP`, `B.LT`, `RET`) emitted by `encode_arm64.pilot`.
  4. `test_m35_4_standalone_elf64_macho64_pe32plus_generation_and_native_execution`: Emits a complete standalone `ELF64` binary on Linux (plus `Mach-O 64` and `PE32+` byte-verified images) directly from Pilot without calling `clang` or `ld`, sets `chmod +x`, executes the generated binary directly on the host CPU, and asserts the process exit code / stdout matches!

---

### M36 — First-Class GPU Compute Pipeline (`[Kernel]` $\rightarrow$ PTX, AMDGPU, SPIR-V, WGSL, MSL, HLSL)

#### 36.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (String-Concatenated 1D Kernel Stubs & Raw `__engine("map_get")` in `gpu_*.pilot`):** [`selfhost/codegen/gpu_shaders.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/codegen/gpu_shaders.pilot), [`gpu_llvm.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/codegen/gpu_llvm.pilot), and [`gpu_c_dialects.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/codegen/gpu_c_dialects.pilot) hardcode `@workgroup_size(64, 1, 1)` 1D indexing (`let idx = i64(gid.x)`), use raw `__engine("map_get", st, "is_flt_" + vname)` string hacks, and emit **textual SPIR-V assembly strings** (`"; SPIR-V\nOpCapability Shader..."`) instead of valid binary SPIR-V (`0x07230203` magic word stream) or 2D/3D workgroup shared-memory tiles!
- **Root Bottleneck 2 (Zero 2-Bit Lattice Host$\leftrightarrow$Device Buffer Mapping or Perceus VRAM Pooling):** There is currently no verifier enforcing that `[Kernel]` parameters obey the 2-bit hardware lattice (`0b00` `: IValue` $\rightarrow$ constant/uniform buffer, `0b01` `slice<T>` $\rightarrow$ read-only storage buffer, `var slice<T>` $\rightarrow$ read-write storage buffer, while `0b10` heap pointers and `0b11` CPU locks are statically forbidden on GPU!), nor is there a Perceus VRAM buffer pool to recycle device memory across kernel dispatches!

#### 36.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/std/gpu/kernel.pilot` | `pub pkg std.gpu;` | `~360L` | `[Kernel(workgroup: (X, Y, Z))]` attribute, `WorkgroupShared<T, N>` tile memory, `gpu_barrier()`, atomic intrinsics, and `PerceusVramPool` (`0b10 : IObject` staging/device buffer recycler + `0b11` `GpuDispatchLease : IGuard`). |
| **CREATE** | `selfhost/codegen/gpu_lattice_abi.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Codegen.GpuLatticeAbi;` | `~340L` | Enforces 2-bit lattice GPU boundary rules (`GPU001`–`GPU005`), std140/std430 alignment padding for `0b00` uniform structs, and descriptor-set binding table synthesis across all 6 GPU backends. |
| **MODIFY** | `selfhost/codegen/gpu_shaders.pilot`, `gpu_llvm.pilot`, `gpu_c_dialects.pilot` | Self-Host GPU Backends | `-140L / +390L` | Purge raw `__engine("map_*")` string hacks; add binary SPIR-V (`0x07230203` 32-bit word stream) emitter alongside WGSL, Metal MSL, DirectX 12 HLSL (`cbuffer`/`StructuredBuffer`/`RWStructuredBuffer`), NVIDIA CUDA PTX 8.0, and AMD RDNA3 AMDGPU. |

#### 36.3 2-Bit Lattice Host $\leftrightarrow$ GPU Binding Physics & Perceus VRAM Pooling
1. **The 2-Bit GPU Parameter Mapping Law (`gpu_lattice_abi.pilot`):**
   - **`0b00 : IValue` Structs / Scalars (`params: SimConfig`):** Automatically mapped to **Uniform / Constant Buffer** (`@group(0) @binding(k) var<uniform>` in WGSL, `constant SimConfig& [[buffer(k)]]` in MSL, `cbuffer : register(bK)` in HLSL, `.param .align A .b8` in PTX) with compile-time `std140`/`std430` padding verification (`GPU003`).
   - **`0b01 : IReference` Immutable `slice<T>` (`input: slice<f32>`):** Mapped to **Read-Only Storage Buffer / SRV** (`var<storage, read>`, `const device T* [[buffer(k)]]`, `StructuredBuffer<T> : register(tK)`, `nonwritable` SPIR-V decoration) enabling GPU L1/texture read-cache paths!
   - **`0b01` Mutable `var slice<T>` (`output: var slice<f32>`):** Mapped to **Read-Write Storage Buffer / UAV** (`var<storage, read_write>`, `device T* [[buffer(k)]]`, `RWStructuredBuffer<T> : register(uK)`).
   - **`0b10 : IObject` & `0b11 : IGuard`:** **Statically Rejected (`GPU001`)** at the `[Kernel]` boundary (CPU heap pointers and OS mutex guards cannot be dereferenced by GPU shader cores; suggests passing `.as_slice()` `0b01` view!).
2. **Perceus VRAM Buffer Recycling (`PerceusVramPool`):**
   - When a `DeviceBuffer<T>` (`0b10 : IObject`) reaches its last use in a GPU compute graph (`a -> kernel1 -> b -> kernel2 -> c`), Perceus `drop_reuse` recycles `a`'s allocated VRAM buffer handle directly into `c` without invoking `vkAllocateMemory` / `cudaMalloc` / `wgpu::Device::create_buffer` (`0` VRAM allocation jitter during frame loops!).

#### 36.4 Multi-Dimensional Workgroups, Shared Memory & 6-Target Emission
- Supports 1D, 2D, and 3D `[Kernel(workgroup: (16, 16, 1))]` launch grids (`global_id.{x,y,z}`, `local_id.{x,y,z}`, `group_id.{x,y,z}`).
- Lowers `WorkgroupShared<f32, 256>` + `gpu_barrier()` to:
  - **SPIR-V (Binary `0x07230203` + Assembly):** `StorageClass Workgroup` + `OpControlBarrier Workgroup Workgroup AcquireRelease`.
  - **WGSL:** `var<workgroup> tile: array<f32, 256>;` + `workgroupBarrier();`.
  - **Metal MSL:** `threadgroup float tile[256];` + `threadgroup_barrier(mem_flags::mem_threadgroup);`.
  - **DX12 HLSL:** `groupshared float tile[256];` + `GroupMemoryBarrierWithGroupSync();`.
  - **CUDA PTX:** `.shared .align 4 .b8 tile[1024];` + `bar.sync 0;`.
  - **AMDGPU (GCN/RDNA3):** `.lds` local data share allocation + `s_waitcnt lgkmcnt(0); s_barrier`.

#### 36.5 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `GPU001` (`0b10` Host Heap Object or `0b11` CPU Guard Passed to `[Kernel]` — Pass `0b01` `slice<T>` or `0b00` Value Struct), `GPU002` (Forbidden CPU Syscall, Recursion, or Dynamic Allocation Inside `[Kernel]` Body), `GPU003` (`0b00` Uniform Struct Layout Violates `std140`/`std430` 16-Byte Vector Stride Alignment), `GPU004` (`WorkgroupShared` Total Byte Size Exceeds Target 32KB/48KB Shared Memory Limit), `GPU005` (`gpu_barrier()` Placed Inside Divergent Thread Branch `if local_id.x < ...` — Deadlock Hazard!).
- **Verification Suite (`crates/pilot-analysis/tests/milestone36_first_class_gpu_compute_pipeline.rs`):**
  1. `test_m36_1_gpu_lattice_abi_uniform_readonly_readwrite_and_gpu001_rejection`: Verifies `0b00` $\rightarrow$ Uniform, `slice<T>` $\rightarrow$ Read-Only Storage, `var slice<T>` $\rightarrow$ Read-Write Storage, and `GPU001`/`GPU005` rejection of `0b10` `List<T>` and divergent `gpu_barrier()`.
  2. `test_m36_2_six_gpu_targets_2d_tiled_matmul_with_shared_memory_barrier`: Compiles a 2D tiled matrix-multiply `[Kernel(workgroup: (16, 16, 1))]` using `WorkgroupShared` into **binary SPIR-V** (`0x07230203`), **WGSL**, **MSL**, **HLSL**, **PTX**, and **AMDGPU**!
  3. `test_m36_3_perceus_vram_buffer_pool_drop_reuse_zero_reallocation`: Runs a 5-stage GPU dispatch pipeline where intermediate `DeviceBuffer<f32>` tensors die after each stage $\rightarrow$ proves `PerceusVramPool` recycles VRAM handles (`total_vram_allocs == 2` instead of `5`).
  4. `test_m36_4_deterministic_cpu_reference_execution_parity`: Executes the `[Kernel]` on the CPU workgroup simulator and verifies exact numerical parity against reference matrix multiplication.

---

### M37 — `PilIR` Symbolic Autodiff (`grad` / `!@`) & Glitch-Free Signals (`sig` / `!^`)

#### 37.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Zero `PilIR` Symbolic Autodiff Pass for `grad` / `!@`):** While `stdlib/core/syntax/call_quote.pilot` registers `!@` / `grad` (`SymbolicAdjoint`) and `selfhost/ir/model.pilot` (line `238`) reserves `op_diff_adjoint() -> u16 => 39`, there is **no `PilIR` differentiation pass** that lowers forward-mode (`Dual<T, G>` `0b00 : IValue`) or reverse-mode (Wengert tape + backward adjoint block synthesis with CSE & dead-code elimination) derivatives!
- **Root Bottleneck 2 (`stdlib/core/reactive.pilot` Uses Raw `__engine("map_get")` String Concatenation & Hardcodes 5 Ops):** [`stdlib/core/reactive.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/reactive.pilot) (lines `45`–`139`) stores reactive signal graphs in untyped `__engine("map_new")` dictionaries keyed by heap-allocated strings (`"val_" + node_id`, `"dver_" + node_id`)—violating `imp_type.md` Rule 10—and `rx_eval_op` (lines `36`–`43`) only supports 5 hardcoded integer toy ops!

#### 37.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/core/autodiff.pilot` | `pub lvl(0) pkg core.autodiff;` | `~310L` | Freestanding Ring-1 `[Layout(c_abi)] pub struct Dual<T: IValue, G: IValue> : IValue` (`0b00`) forward-mode algebra (`+`, `-`, `*`, `/`, `sin`, `cos`, `exp`, `ln`, `sqrt`, `pow`, `max`, `relu`) + `[CustomGrad(vjp_fn)]` attribute. |
| **CREATE** | `selfhost/ir/pass_autodiff.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Ir.Autodiff;` | `~430L` | `PilIR` (`M6`) forward- & reverse-mode symbolic differentiation pass (`op_diff_adjoint`) generating adjoint basic blocks, checkpointing loop tapes, and running CSE + DCE on synthesized derivative instructions. |
| **MODIFY** | `stdlib/core/reactive.pilot` | `pub lvl(0) pkg core.reactive;` | `-95L / +320L` | Purge all raw `__engine("map_*")` string-concatenation hacks; implement flat, zero-allocation topological `SignalArena<T>` (`0b10 : IObject` backed by contiguous `[Layout(c_abi)]` `SignalSlot` arrays) with $O(1)$ bitset dirty propagation and glitch-free diamond evaluation. |

#### 37.3 Symbolic Autodiff Equations (`pass_autodiff.pilot`) & Topological Signal Arena
1. **Forward-Mode `Dual<T, G>` (`0b00 : IValue`) Lowering:**
   - Packs primal value $x$ and tangent $\dot{x}$ into a flat register pair `Dual { val: x, dval: dx }` (`sizeof(Dual<f64, f64>) == 16` bytes, passed in `xmm0`/`xmm1` with `0` heap allocations!).
2. **Reverse-Mode Vector-Jacobian Product (`VJP`) Synthesis over `PilIR`:**
   - Given primal function $y = f(x_1, \dots, x_n)$ invoked via `f!@(x_1, ..., x_n)` or `grad f(x_1, ..., x_n)`:
     - **Forward Primal Block:** Computes primal SSA values $v_i$ and saves only nonlinear intermediate multiplicands (e.g. operands of `op_mul`, `op_div`, transcendental calls) onto a flat stack/arena tape.
     - **Backward Adjoint Block (Reverse Topological Order):** Seeds $\bar{y} = 1$ and propagates adjoints $\bar{u} \mathrel{+}= \bar{v} \cdot \frac{\partial v}{\partial u}$ for every instruction:
       - `v = op_add(a, b)` $\implies \bar{a} \mathrel{+}= \bar{v}, \quad \bar{b} \mathrel{+}= \bar{v}$
       - `v = op_mul(a, b)` $\implies \bar{a} \mathrel{+}= \bar{v} \cdot b, \quad \bar{b} \mathrel{+}= \bar{v} \cdot a$
       - `v = op_div(a, b)` $\implies \bar{a} \mathrel{+}= \bar{v} / b, \quad \bar{b} \mathrel{-}= (\bar{v} \cdot v) / b$
     - Immediately runs `M6` **Common Subexpression Elimination (CSE)** and **Dead Code Elimination (DCE)** over the combined primal+adjoint CFG, folding `0 * x -> 0` and `1 * x -> x` so unused partial derivatives cost `0` instructions!
3. **Flat Topological `SignalArena` (`stdlib/core/reactive.pilot`):**
   - **`[Layout(c_abi)] pub struct SignalSlot : IValue`** (`0b00`, 32 Bytes):
     `pub val_bits: i64; pub version_epoch: u32; pub topo_rank: u16; pub dep_a: u16; pub dep_b: u16; pub op_code: u16; pub const_param: i64;`
   - Evaluates dirty signals in strictly increasing `topo_rank` order using a flat 64-bit `dirty_mask: u64` bitset—guaranteeing every diamond sink (`A -> B, A -> C, B + C -> D`) evaluates **at most once per epoch** with **zero heap allocations and zero `__engine("map_*")` calls**!

#### 37.4 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `GRAD001` (`grad` / `!@` Applied to Non-Differentiable Discrete/IO Function Without `[CustomGrad]` Rule), `GRAD002` (Unbounded Uncheckpointed Loop in Reverse-Mode `grad` — Suggests `[GradCheckpoint]` or Forward Mode), `SIG001` (Cyclic Reactive Signal Dependency Detected at Topological Sort), `SIG002` (Mutating `SignalSource` Inside Derived Signal Evaluation Callback — Glitch Hazard Blocked).
- **Verification Suite (`crates/pilot-analysis/tests/milestone37_pilir_symbolic_autodiff_and_glitch_free_signals.rs`):**
  1. `test_m37_1_forward_and_reverse_mode_autodiff_exact_gradients`: Computes analytical gradients of $f(x, y) = x^3 y + \sin(x \cdot y) - (x / y)$ via `f!@(x, y)` $\rightarrow$ verifies exact numerical match (to $10^{-12}$) against finite differences and verifies CSE/DCE eliminates `0 * dx` dead terms!
  2. `test_m37_2_custom_grad_vjp_rule_and_grad001_diagnostic`: Registers a `[CustomGrad]` rule on a fast approximation function and verifies `GRAD001` rejects differentiating an IO function.
  3. `test_m37_3_flat_topological_signal_arena_glitch_free_diamond_and_early_cutoff`: Constructs a 4-layer diamond reactive graph (`src -> (left, right) -> mid -> sink`) with an equality-preserving clamp node $\rightarrow$ proves `recompute_count == 1` on diamond updates, `recompute_count == 0` when clamped value is unchanged, and `0` `__engine("map_*")` calls in `reactive.pilot`!
  4. `test_m37_4_cyclic_signal_graph_rejection_sig001`: Attempts to wire a feedback cycle without a delay latch $\rightarrow$ statically caught by `SIG001`.

---

### M38 — Native `pilot` Self-Hosting Cutover (Retiring Rust Lexer, Parser & Analysis Crates)

#### 38.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (Raw `__engine("map_*")` / `__engine("ast_*")` Calls Remaining in Legacy Selfhost Modules):** While Phase 1–3 modules use strongly-typed `List<T>`, `Map<K, V>`, and `[Layout(c_abi)]` structs per [`imp_type.md`](file:///home/moony/Documents/dev/projects/Pilot/imp_type.md), several older selfhost modules (`selfhost/analyzer.pilot`, `selfhost/vm_compiler.pilot`, `selfhost/perf/profiler.pilot`, `selfhost/perf/wcet.pilot`) still invoke raw `__engine("map_new")` / `__engine("map_get")` with string-concatenated keys (`"dur:" + fn_name`).
- **Root Bottleneck 2 (Duplicate Bootstrap Rust Frontend Logic):** `selfhost/lexer.pilot` (405L), `selfhost/parser.pilot` (877L), `selfhost/preprocessor.pilot` (280L), `selfhost/db/` (`M5`), `selfhost/ir/` (`M6`), and `selfhost/types/` (`M22`–`M28`) now implement the complete Pilot frontend, CST/AST, type-checker, Perceus verifier, and bytecode/C/LLVM/ASM emitters in pure Pilot! Retaining complex domain logic inside `crates/pilot-lexer`, `crates/pilot-parser`, and `crates/pilot-analysis` duplicates the self-hosted compiler and blocks contraction toward the minimal C11 seed (`M39`).

#### 38.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/driver/cutover_bridge.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Driver.Cutover;` | `~380L` | Unified pure-Pilot compiler driver pipeline (`lex -> preprocess -> parse_cst -> expand_syntax -> weave_attrs -> infer_types -> lower_pilir -> perceus_interproc -> emit_target`) replacing the Rust frontend pipeline. |
| **CREATE** | `selfhost/lint/imp_type_auditor.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Lint.ImpTypeAuditor;` | `~340L` | Automated `imp_type.md` compliance verifier scanning `selfhost/` and `stdlib/` for raw `__engine` calls (`IMP001`), duplicate type definitions (`IMP002`), unconstrained generics (`IMP003`), boolean-blindness (`IMP004`), and magic string constants (`IMP005`). |
| **MODIFY** | `selfhost/perf/profiler.pilot`, `wcet.pilot`, `selfhost/analyzer.pilot` | Self-Host Cleanup | `-160L / +140L` | Replace remaining `"key:" + str` raw `__engine("map_*")` tables with strongly-typed `[Layout(c_abi)]` structs and `List<T>`/`DetMap` containers per `imp_type.md` Rule 10. |
| **CONTRACT** | `crates/pilot-lexer`, `crates/pilot-parser`, `crates/pilot-analysis` | Rust Seed Contraction | Ratchet Down | Contract Rust frontend crates to minimal seed loaders forwarding compilation queries directly to `selfhost/driver/cutover_bridge.pilot` and enforce stricter `CrateCeilingRule` LOC ratchets in `anticheat.rs`. |

#### 38.3 Workspace-Wide `imp_type.md` Compliance Rules (`imp_type_auditor.pilot`)
`imp_type_auditor.pilot` statically enforces all 10 rules of [`imp_type.md`](file:///home/moony/Documents/dev/projects/Pilot/imp_type.md) across `selfhost/` and `stdlib/`:
1. **`IMP001` (Zero Unencapsulated `__engine` Calls):** Raw `__engine(...)` is permitted **only** inside low-level Ring-1 `stdlib/core/` intrinsic wrappers (`mem.pilot`, `core.pilot`, `bootstrap.pilot`) and the `PhysicsRegistry` bridge adapter; any raw `__engine("map_new")` or `__engine("map_get")` in domain/selfhost passes is flagged with an autofix to `DetMap` / `Map<K, V>` / `List<T>`.
2. **`IMP002` (Single Source of Truth — Zero Type Duplication):** Hashes `(struct_name, field_names)` across the `ModuleGraph` (`M2`); if two modules declare duplicate struct/enum definitions instead of importing the canonical owning module via `use ...;`, emits `IMP002`.
3. **`IMP003` (Constrained Generics Only):** Flags any generic parameter `<T>` lacking an interface bound (`<T: IAnyValue>`, `<T: IValue>`, `<T: IEquatable<T>>`).
4. **`IMP004` (Boolean Blindness & Finite Set Discipline):** Flags functions taking $\ge 3$ adjacent `bool` parameters or stringly-typed mode parameters (`mode: str`), guiding conversion to `bitflags : u32` or `enum : u8`.

#### 38.4 Stage-1 $\rightarrow$ Stage-2 $\rightarrow$ Stage-3 Self-Hosting Cutover Verification
1. **Stage-1 (Seed Loader)** compiles the `selfhost/` pure-Pilot compiler sources into `stage2_pilot.pilc` (and native `stage2_pilot` binary).
2. **Stage-2 (`stage2_pilot`)** compiles the exact same `selfhost/` sources using `cutover_bridge.pilot` into `stage3_pilot.pilc` (and native `stage3_pilot` binary).
3. **Bit-Identical Fixed Point:** Asserts $\text{SHA-256}(\text{stage2\_pilot}) == \text{SHA-256}(\text{stage3\_pilot})$ across all modules!

#### 38.5 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `IMP001` (Raw `__engine` Call Outside Ring-1 Intrinsic Boundary), `IMP002` (Duplicate Type Definition Across Modules — Violates Single Source of Truth), `IMP003` (Unconstrained Generic Type Parameter), `IMP004` (Boolean Blindness / Stringly-Typed Finite Set), `CUT001` (Stage-2 vs Stage-3 Self-Hosted Compiler Output Divergence).
- **Verification Suite (`crates/pilot-analysis/tests/milestone38_native_pilot_self_hosting_cutover.rs`):**
  1. `test_m38_1_end_to_end_pure_pilot_cutover_driver_pipeline`: Compiles a multi-module Pilot program (`syntax` + `[Json]` weaver + `*Bundle` + `PilIR` + Perceus + codegen) 100% through `cutover_bridge.pilot` and verifies execution parity.
  2. `test_m38_2_imp_type_auditor_enforces_rules_imp001_to_imp004`: Tests `imp_type_auditor.pilot` on synthetic violations (raw `__engine`, duplicate struct across files, unconstrained `<T>`, 4 adjacent booleans) and verifies 100% clean audit across repaired `selfhost/perf/profiler.pilot`.
  3. `test_m38_3_stage2_stage3_deterministic_sha256_fixed_point`: Runs Stage-2 and Stage-3 self-compilation through `cutover_bridge.pilot` and asserts bit-identical SHA-256 output digests.
  4. `test_m38_4_rust_seed_loc_ratchet_contraction_gate`: Verifies all Rust crate LOC ceilings remain strictly below their contracted ratchet caps.

---

### M39 — Portable C11 Bootstrap Seed Snapshot (Zero Rust Toolchain Required)

#### 39.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (C11 Backend Emits Incomplete Runtime Glue & Requires Rust Cargo to Bootstrap):** [`selfhost/codegen/c_emitter.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/codegen/c_emitter.pilot) (348L) and [`selfhost/codegen/c_header.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/codegen/c_header.pilot) (35L) emit C11 code for individual functions, and [`crates/pilot-codegen/pilot_runtime.h`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-codegen/pilot_runtime.h) provides basic runtime macros, but there is **no self-contained `bootstrap/pilot_seed.c` amalgamated snapshot** or POSIX/Windows bootstrap script (`./bootstrap.sh` / `bootstrap.bat`) capable of building the `pilot` compiler on a fresh machine using **only a standard C11 compiler (`cc`/`clang`/`gcc`/`cl.exe`/`zig cc`) with zero Rust/Cargo installed**!

#### 39.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/codegen/c_bootstrap_bundler.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Codegen.CBootstrap;` | `~410L` | Deterministic topological C11 amalgamation generator (`pilot bootstrap --emit-c11`) lowering `stdlib/core/` + `selfhost/` into a freestanding, dependency-free ISO C11 translation unit with exact `[Layout(c_abi)]` `_Static_assert` checks and 2-bit lattice Perceus hooks. |
| **CREATE** | `bootstrap/pilot_seed_rt.h` | ISO C11 Freestanding Runtime Header | `~260L` | Portable single-header C11 runtime (`<stdint.h>`, `<stddef.h>`, `<stdbool.h>`, `<stdalign.h>`) implementing `pilot_rt_dup`, `pilot_rt_drop`, `pilot_rt_drop_reuse`, `ArenaScope` bump allocation, and POSIX/Win32 file & clock shims. |
| **CREATE** | `bootstrap/bootstrap.sh` & `bootstrap/bootstrap.bat` | Multiplatform Zero-Rust Bootstrap Scripts | `~140L` | Auto-detects `clang`, `gcc`, `cc`, `zig cc`, or `cl.exe`, compiles `pilot_seed.c` in $< 2$ seconds into `stage1_pilot`, and verifies `stage1_pilot` $\rightarrow$ `stage2_pilot` $\rightarrow$ `stage3_pilot` fixed-point! |
| **MODIFY** | `selfhost/codegen/c_emitter.pilot` & `c_header.pilot` | Self-Host C11 Emitter | `+95L` | Add `_Alignas(A)` struct emission, `_Static_assert(sizeof(T) == S, ...)` layout guards, and deterministic symbol mangling for multi-package C11 amalgamation. |

#### 39.3 ISO C11 Amalgamation Physics & Zero-Leak Perceus Runtime (`pilot_seed_rt.h`)
1. **Exact `[Layout(c_abi)]` Struct & `_Static_assert` Generation (`c_bootstrap_bundler.pilot`):**
   - Every `[Layout(c_abi)] struct T` with computed `size_bytes = S` and `align_bytes = A` emits:
     ```c
     typedef struct _Alignas(A) pilot_T { ... } pilot_T;
     _Static_assert(sizeof(pilot_T) == S, "ABI size mismatch for T");
     _Static_assert(_Alignof(pilot_T) == A, "ABI align mismatch for T");
     ```
2. **2-Bit Lattice Perceus C11 Runtime (`bootstrap/pilot_seed_rt.h`):**
   - Every `0b10` `: IObject` allocation carries an 8-byte aligned header `typedef struct { uint32_t rc; uint32_t size_bytes; } pilot_obj_hdr_t;` immediately preceding the payload pointer.
   - `pilot_rt_drop_reuse(void* ptr, uint32_t expected_size)`:
     - If `hdr->rc == 1`, zeroes or selectively drops child fields and **returns `ptr` immediately in 2 CPU instructions** (`0` `free()`/`malloc()` calls!).
     - If `hdr->rc > 1`, decrements `hdr->rc` and allocates a fresh buffer of `expected_size`.
   - Tracks `g_pilot_live_allocs` and `g_pilot_live_bytes` counters so `--verify-leaks` asserts `g_pilot_live_allocs == 0` at process exit!
3. **Zero-Rust Bootstrap Chain (`./bootstrap.sh` / `bootstrap.bat`):**
   - Step 1: `${CC:-cc} -std=c11 -O2 bootstrap/pilot_seed.c -o build/pilot_stage1`
   - Step 2: `./build/pilot_stage1 build selfhost/ -o build/pilot_stage2`
   - Step 3: `./build/pilot_stage2 build selfhost/ -o build/pilot_stage3`
   - Step 4: Verify `sha256sum build/pilot_stage2 == sha256sum build/pilot_stage3`!

#### 39.4 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `BOOT001` (C11 Amalgamation Cycle in Struct Value Dependency Graph), `BOOT002` (`_Static_assert` ABI Size or Alignment Divergence Between `PhysicsRegistry` and Host C11 Compiler), `BOOT003` (Non-Zero Perceus Live Allocation Count at Bootstrap Process Exit — Memory Leak Detected), `BOOT004` (Stage-1 C11 Seed vs Stage-2 Self-Hosted Output Mismatch).
- **Verification Suite (`crates/pilot-analysis/tests/milestone39_portable_c11_bootstrap_seed_snapshot.rs`):**
  1. `test_m39_1_c11_amalgamation_static_assert_layouts_and_clang_compilation`: Generates a multi-package C11 amalgamation with `0b00`, `0b01`, `0b10`, and `0b11` types, compiles it with `clang -std=c11 -Wall -Wextra -Werror`, and verifies all `_Static_assert` size/alignment checks pass!
  2. `test_m39_2_perceus_c11_runtime_drop_reuse_and_zero_leak_certification`: Executes a functional list/tree workload through `pilot_seed_rt.h` $\rightarrow$ proves `pilot_rt_drop_reuse` recycles unique nodes in place and asserts `g_pilot_live_allocs == 0` at exit (`BOOT003`).
  3. `test_m39_3_zero_rust_bootstrap_script_stage1_to_stage3_parity`: Invokes the C11 bootstrap compiler chain using the system C11 compiler (`cc`/`clang`) without invoking `cargo` or `rustc`, and verifies deterministic Stage-2 == Stage-3 output parity!
  4. `test_m39_4_cross_platform_posix_and_windows_bootstrap_manifest_validation`: Verifies `bootstrap.sh` and `bootstrap.bat` flags, compiler auto-detection (`clang`/`gcc`/`zig cc`/`cl.exe`), and reproducible deterministic timestamps.

---

### M40 — Integrated Hardware-Counter Benchmark Suite (`pilot bench` vs. C & Rust)

#### 40.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (No Hardware PMU Counter Reader or Statistical Outlier/Regression Engine in `stdlib/std/bench/`):** While [`selfhost/perf/profiler.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/perf/profiler.pilot) emits SVG flamegraphs from folded stack strings and [`selfhost/perf/wcet.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/perf/wcet.pilot) estimates static AST instruction weights, there is **no `stdlib/std/bench/` package** reading real hardware Performance Monitoring Unit (PMU) counters (CPU cycles, retired instructions, IPC, branch misses, L1D/LLC cache misses, heap allocations), **no Welch's $t$-test / median-absolute-deviation (MAD) statistical harness**, and **no automated head-to-head parity gate comparing compiled Pilot against `clang -O3` C11 and `rustc -O` across canonical workloads**!

#### 40.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/std/bench/pmu_counters.pilot` | `pub pkg std.bench.pmu;` | `~350L` | Cross-platform hardware counter reader (`0b11` `PmuSessionGuard : IGuard`) wrapping Linux `perf_event_open` (`PERF_COUNT_HW_CPU_CYCLES`, `INSTRUCTIONS`, `CACHE_MISSES`, `BRANCH_MISSES`) + `rdtsc`/`cntvct_el0`, macOS `proc_pid_rusage` / `kperf`, and Windows `QueryPerformanceCounter` + Perceus allocation counters. |
| **CREATE** | `stdlib/std/bench/stats_runner.pilot` | `pub pkg std.bench;` | `~390L` | `[Bench]` weaver, black-box optimizer barrier (`bench.keep(val)`), auto-calibrated warmup + sample loop, bootstrap confidence intervals, MAD outlier detector, and baseline regression comparator. |
| **CREATE** | `selfhost/cli/bench_cmd.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Cli.BenchCmd;` | `~340L` | `[Command(name: "bench")]` CLI tool executing `[Bench]` suites, running head-to-head differential benchmarks against C (`clang -O3`) and Rust (`rustc -C opt-level=3`), and exporting Markdown/JSON/SVG reports. |

#### 40.3 Hardware PMU Telemetry, Statistical Rigor & C/Rust Parity Equations
- **`[Layout(c_abi)] pub struct HardwarePmuSample : IValue`** (`0b00`, 64 Bytes):
  - `pub wall_ns: u64; pub cpu_cycles: u64; pub instructions: u64; pub l1d_misses: u64; pub llc_misses: u64; pub branch_misses: u64; pub heap_allocs: u64; pub heap_bytes_reused: u64;`
  - Derived metrics:
    $$\text{IPC} = \frac{\text{instructions}}{\max(1, \text{cpu\_cycles})}, \quad \text{BranchMissRate} = \frac{\text{branch\_misses}}{\max(1, \text{instructions})}, \quad \text{ReuseRatio} = \frac{\text{heap\_bytes\_reused}}{\max(1, \text{heap\_allocs} + \text{heap\_bytes\_reused})}$$
- **`[Layout(c_abi)] pub struct BenchSummaryStats : IValue`** (`0b00`):
  - `pub min_ns: f64; pub median_ns: f64; pub mean_ns: f64; pub std_dev_ns: f64; pub mad_ns: f64; pub p99_ns: f64; pub ipc_mean: f64; pub allocs_per_iter: u64; pub speedup_vs_c: f64; pub speedup_vs_rust: f64;`
- **Optimizer Black-Box Barrier (`pub noalloc fn keep<T: IAnyValue>(val: ref<T>)`):**
  - Emits an inline empty volatile assembly constraint (`asm volatile("" : : "r,m"(val) : "memory")`) that forces the compiler to materialize `val` in a register/memory slot without dead-code-eliminating the benchmarked computation (`0` extra instructions executed!).
- **5 Canonical C/Rust Parity Workloads Certified by `pilot bench --verify-parity`:**
  1. **Workload 1 (`SoA` Particle Kinematics — `M25` + `M34`):** 1M particles `pos += vel * dt` (`SoA<Particle, align: 64>` + SIMD $\ge 3.5\times$ faster than C `AoS`, $\le 1.00\times$ parity with hand-vectorized C `SoA`, `0` heap allocs).
  2. **Workload 2 (Functional Tree Update — `M31` `drop_reuse`):** 100K binary search tree insertions (`op_perceus_drop_reuse` recycles $100\%$ of path nodes in place $\rightarrow$ `0` `malloc` calls inside update loop!).
  3. **Workload 3 (Stream Pipeline Deforestation — `M23`):** `filter |> map |> take |> fold` over 10M `i64` slice (`0` intermediate allocations, identical instruction count to C `for` loop).
  4. **Workload 4 (`ArenaScope` Request Parser — `M32`):** JSON/HTTP telemetry parse & reset inside 64KB `ArenaScope` (`1`-instruction LIFO bump reset, $>4\times$ faster than per-node `malloc`/`free`).
  5. **Workload 5 (Zero-Copy C/Rust FFI Call — `M13` + `M16`):** 10M calls across `frame<TSig>` / `slice<u8>` boundary (`0` byte copies, $\le 1.02\times$ of direct C call).

#### 40.4 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `BENCH001` (`[Bench]` Function Elided by Dead-Code Elimination — Call `bench.keep(result)` on Output), `BENCH002` (`[Bench(noalloc: true)]` Benchmark Performed Unexpected Global Heap Allocation: `allocs_per_iter > 0`), `BENCH003` (Performance Regression Exceeded Threshold: Current Median Exceeds Baseline by $> \text{max\_regression\_pct}\%$), `BENCH004` (C/Rust Parity Gate Failure on Canonical Workload).
- **Verification Suite (`crates/pilot-analysis/tests/milestone40_integrated_hardware_counter_benchmark_suite.rs`):**
  1. `test_m40_1_pmu_counter_capture_ipc_mad_stats_and_black_box_keep`: Runs `[Bench]` functions through `pmu_counters.pilot` and `stats_runner.pilot` $\rightarrow$ verifies non-zero cycle/instruction/IPC capture, MAD outlier filtering, and `BENCH001` detection when `bench.keep()` is omitted on dead pure math.
  2. `test_m40_2_zero_alloc_certification_and_perceus_reuse_ratio_bench002`: Verifies `allocs_per_iter == 0` on fused streams (`M23`) and `ArenaScope` (`M32`), `ReuseRatio == 1.0` on `drop_reuse` (`M31`), and `BENCH002` failure when a `[Bench(noalloc: true)]` function leaks a heap allocation.
  3. `test_m40_3_head_to_head_c11_and_pilot_native_parity_gate`: Compiles canonical workloads via both Pilot native codegen and `clang -O3`, executes both, and verifies identical output and C parity compliance.
  4. `test_m40_4_pilot_bench_cli_markdown_json_and_regression_diff_bench003`: Exercises `pilot bench --json` and `--baseline` regression comparison (`BENCH003`).

#### 40.5 Phase 4 (`M31`–`M40`) Cumulative Exit Checkpoint
Before advancing to **Phase 5 (`M41` — Unified All-in-One `pilot` Toolchain & PILOT 1.0 RELEASE)**:
- **Gate 1 (100% `imp_type.md` & Name-Blind Purity):** `imp_type_auditor.pilot` (`M38`) reports **0 violations** across all `stdlib/` and `selfhost/` files; zero raw `__engine("map_*")` hacks remain outside Ring-1 intrinsics; Dual-Pass Universal Rename Litmus passes 100% green across LLVM (`M33`), SIMD (`M34`), Direct Native (`M35`), GPU (`M36`), and C11 (`M39`) backends.
- **Gate 2 (Zero-Rust Self-Hosting & Bootstrap Independence):** The pure-Pilot compiler (`M38`) achieves bit-identical Stage-2 == Stage-3 SHA-256 parity, and `./bootstrap/bootstrap.sh` (`M39`) builds a working native `pilot` compiler from `bootstrap/pilot_seed.c` using **only a standard C11 compiler (`clang`/`gcc`/`cc`) with zero Rust/Cargo dependency** and `0` Perceus memory leaks (`g_pilot_live_allocs == 0`).
- **Gate 3 (Verified C/Rust Hardware Parity):** `pilot bench --verify-parity` (`M40`) certifies `0` intermediate heap allocations on fused streams (`M23`), interprocedural `0b01` borrows (`M31`), and `ArenaScope` (`M32`), $100\%$ in-place node recycling on `drop_reuse` (`M31`), and C/Rust execution parity across all 5 canonical workloads.

---

## Phase 5 (`M41` – `M50`): Unified All-in-One `pilot` Toolchain & **PILOT 1.0 RELEASE** (UNABRIDGED SPECIFICATION)

> **Unabridged Specification Contract:** Phase 5 (`M41`–`M50`) completes the unified all-in-one `pilot` 1.0 developer platform and culminates in the **PILOT 1.0 FULL RELEASE**. It replaces legacy JSON manifests with sandboxed `pkg.pilot` manifests and content-addressed lockfiles (`M41`), orchestrates polyglot foreign dependencies without glue facades (`M42`), upgrades `pilot lsp` to sub-10ms `PilotDb`-backed intelligence with live `imp_type.md` coaching (`M43`), unifies native DWARF + `PilVM` + foreign-frame debugging in `pilot debug` (`M44`), launches the compiler-in-the-loop `pilot mcp` v1 AI engine (`M45`), unifies unit/doc/differential/fuzz testing in `pilot test` (`M46`), generates interactive 2-bit lattice + `.pchor` Mermaid documentation in `pilot doc` (`M47`), ships single-command cross-compiled static binaries with embedded assets in `pilot ship` (`M48`), freezes `Edition 1.0` with `pilot doctor` (`M49`), and certifies the complete Pilot 1.0 Release (`M50`). Every milestone below specifies the complete engineering blueprint leaving only literal code bodies omitted.

---

### M41 — `pkg.pilot` Declarative Manifest & Content-Addressed Package Manager

#### 41.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`manifest.pilot` Parses Legacy `pilot.json` Instead of First-Class `pkg.pilot`):** [`selfhost/pkg/manifest.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/pkg/manifest.pilot) (lines `1`–`464`) parses static `pilot.json` files (`parse_manifest_json`), forcing developers into a second configuration language instead of writing typed, `comptime`-evaluated `pkg.pilot` manifests inside `M29`'s zero-ambient-authority `VmSandbox`!
- **Root Bottleneck 2 (No Cross-Package Single-Source-of-Truth Type Deduplication or Merkle Tree Lockfile):** [`selfhost/pkg/resolver.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/pkg/resolver.pilot) (261L) and [`selfhost/pkg/version.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/pkg/version.pilot) (295L) resolve semver ranges, but they do not enforce `imp_type.md` Rule 9 (Single Source of Truth) across multi-package workspace graphs (allowing two packages to define conflicting duplicate structs instead of sharing the canonical owning package!), nor do they maintain a content-addressed SHA-256 Merkle tree store (`~/.pilot/store/sha256/...`) with cryptographic `pkg.lock` verification!

#### 41.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/pilot/pkg.pilot` | `pub pkg pilot.pkg;` | `~320L` | Ring-3 declarative `PackageSpec`, `TargetSpec`, `DepSpec`, `WorkspaceSpec`, and `ComptimeGrantSpec` types for `pkg.pilot` manifests (`0b00` & `0b10`). |
| **CREATE** | `selfhost/pkg/pkg_eval.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Pkg.Eval;` | `~360L` | Evaluates `pkg.pilot` inside a `0`-capability `M29` `ComptimeGuardian` (`fuel_limit = 50_000`, `caps = cap_none()`), enforces cross-package `imp_type.md` Rule 9 single-source-of-truth type ownership (`PKG001`–`PKG006`), and migrates legacy `pilot.json` $\rightarrow$ `pkg.pilot`. |
| **CREATE** | `selfhost/pkg/cas_lockfile.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Pkg.CasLock;` | `~380L` | Content-addressed SHA-256 Merkle tree package store, deterministic `pkg.lock` serializer/verifier, diamond dependency version unifier (PubGrub CDCL solver), and offline vendor bundle generator. |
| **MODIFY** | `selfhost/pkg/manifest.pilot` & `resolver.pilot` | Self-Host Package Manager | `-120L / +95L` | Route manifest loading through `pkg_eval.pilot` and lockfile verification through `cas_lockfile.pilot`. |

#### 41.3 `pkg.pilot` Schema, Content-Addressed Store & Cross-Package Type Uniqueness
- **`DepSourceKind` (`enum : u8`, `0b00 : IValue`)**: `::RegistryCas = 0`, `::WorkspaceLocal = 1`, `::GitCommitPinned = 2`, `::VendorArchive = 3`, `::ForeignPolyglot = 4`.
- **`[Layout(c_abi)] pub struct LockedPackageNode : IValue`** (`0b00`, 64 Bytes):
  - `pub pkg_sym: u32; pub semver_major: u16; pub semver_minor: u16; pub semver_patch: u16; pub source_kind: DepSourceKind; pub granted_caps_mask: u32; pub merkle_root_hi: u64; pub merkle_root_lo: u64; pub exported_types_hash: u64;`
- **Canonical `pkg.pilot` Syntax (Evaluated in Pure `comptime` Sandbox):**
  ```pilot
  use pilot.pkg.PackageSpec;

  pub comptime fn package() -> PackageSpec => PackageSpec {
      name:    "orders-service",
      version: "1.0.0",
      edition: ::Edition1_0,
      ring:    ::Ring2Hosted,
      deps: [
          DepSpec::registry("http-core", "^1.2.0"),
          DepSpec::workspace("../shared-domain"),
      ],
      comptime_grants: [
          ComptimeGrantSpec::allow_fs_read("shared-domain", ["schemas/*.sql"]),
      ],
  };
  ```
- **Cross-Package Single Source of Truth Enforcement (`imp_type.md` Rule 9 — `PKG004`):**
  - During workspace graph resolution, `pkg_eval.pilot` computes structural type fingerprints `(type_name, field_names_ layout_hash)` across all packages in the dependency DAG. If package `B` re-declares a struct `OrderId` or `Money` already exported by upstream dependency `A` instead of `use A.OrderId;`, `PKG004` flags the duplicate definition and offers a 1-click autofix to import from `A`!

#### 41.4 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `PKG001` (`pkg.pilot` Attempted Ambient I/O or Exceeded `50,000`-Instruction `comptime` Evaluation Budget), `PKG002` (Unsatisfiable Version Constraint Conflict in PubGrub Dependency Solver — Prints Minimal Conflict Derivation Tree), `PKG003` (`pkg.lock` SHA-256 Merkle Digest Mismatch Against Content-Addressed Archive — Tampered Package Blocked!), `PKG004` (Cross-Package Duplicate Type Definition Violates `imp_type.md` Single Source of Truth), `PKG005` (Circular Package Dependency Detected in Workspace Graph), `PKG006` (Unpinned Git Dependency Branch/Tag Without Commit SHA).
- **Verification Suite (`crates/pilot-analysis/tests/milestone41_pkg_pilot_manifest_and_cas_lockfile.rs`):**
  1. `test_m41_1_sandboxed_pkg_pilot_evaluation_and_legacy_json_migration`: Evaluates a declarative `pkg.pilot` manifest inside the `0`-capability `M29` sandbox, verifies `PKG001` blocks any I/O inside `pkg.pilot`, and tests lossless `pilot.json` $\rightarrow$ `pkg.pilot` migration.
  2. `test_m41_2_pubgrub_version_solver_and_conflict_derivation_pkg002`: Resolves a 6-package diamond dependency graph and verifies `PKG002` outputs the exact conflict explanation on incompatible semver constraints.
  3. `test_m41_3_content_addressed_sha256_merkle_lockfile_and_tamper_detection_pkg003`: Generates `pkg.lock` with SHA-256 Merkle digests, flips 1 byte in a cached package file, and verifies `PKG003` blocks loading immediately.
  4. `test_m41_4_cross_package_duplicate_type_rejection_imp_type_rule9_pkg004`: Defines an identical `CustomerRecord` struct in two sibling workspace packages $\rightarrow$ verifies `PKG004` catches the duplication and recommends `use domain.CustomerRecord;`.

---

### M42 — Polyglot Package & Build Orchestrator (`pilot pkg` Foreign Dependencies)

#### 42.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`interop.pilot` & `venv.pilot` Are 54-Line & 35-Line String Splitters!):** [`selfhost/pkg/interop.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/pkg/interop.pilot) (lines `1`–`54`) only splits `"cargo:serde@1.0"` on `:` and `@` into `"cargo|serde|1.0"`, and [`selfhost/pkg/venv.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/pkg/venv.pilot) (lines `1`–`35`) only formats a `.pilot-venv` directory path string! Neither module orchestrates compiling C/C++ sources, linking static/dynamic Rust crates (`M16`), provisioning isolated Python wheel environments (`M17`), resolving NPM Node-API/QuickJS modules (`M18`), or composing WASM `.wit` components (`M19`) declared in `pkg.pilot`!
- **Root Bottleneck 2 (Manual Foreign Build Scripts & Duplicate Glue Facades):** In existing workflows, developers have to run `cmake`, `cargo build`, `pip`, and `npm` manually outside Pilot and risk creating hand-written wrapper structs that duplicate the foreign library's types (violating `imp_type.md` Rule 9).

#### 42.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/pkg/polyglot_orchestrator.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Pkg.PolyglotOrch;` | `~420L` | Unified build & link graph orchestrating C/C++ (`M12`/`M15`), Rust Cargo crates (`M16`), Python wheels/PEP 517 venvs (`M17`), NPM/TypeScript packages (`M18`), and WASM components (`M19`) declared in `pkg.pilot` with content-addressed artifact caching. |
| **CREATE** | `selfhost/pkg/foreign_type_bridge.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Pkg.ForeignBridge;` | `~340L` | Connects resolved foreign packages directly to `PilotDb` (`M5`) `use <lang>` absorbers so foreign types project directly into canonical `0b00`/`0b01`/`0b10`/`0b11` Pilot types with **0 duplicate wrapper structs** (`POLY001`–`POLY005`). |
| **MODIFY** | `selfhost/pkg/interop.pilot` & `selfhost/pkg/venv.pilot` | Self-Host Pkg Interop | `+180L` | Upgrade `Ecosystem` (`::C`, `::Cpp`, `::Cargo`, `::Pip`, `::Npm`, `::WasmWit`) spec resolution, hermetic `.pilot-venv` layout, and native linker flag synthesis (`-L`, `-l`, `rpath`, `.lib`/`.dll`). |

#### 42.3 Hermetic Polyglot Build Graph & Zero-Facade Type Absorption
- **`ForeignEcosystem` (`enum : u8`, `0b00 : IValue`)**: `::NativeC = 1`, `::NativeCpp = 2`, `::RustCargo = 3`, `::PythonWheel = 4`, `::JsNpm = 5`, `::WasmComponent = 6`.
- **`[Layout(c_abi)] pub struct ForeignBuildArtifact : IValue`** (`0b00`):
  - `pub dep_sym: u32; pub ecosystem: ForeignEcosystem; pub abi_hash: u64; pub archive_path_sym: u32; pub header_or_stub_sym: u32; pub link_static: bool; pub requires_thread_affine_drop: bool;`
- **Direct Absorption Without Duplicate Glue Facades (`foreign_type_bridge.pilot`):**
  1. **C / C++ (`DepSpec::c_source("sqlite3", "vendor/sqlite3.c", flags: ["-DSQLITE_THREADSAFE=1"])`):** Compiles C/C++ TU with incremental SHA-256 flag caching, links `.a`/`.lib`, and mounts `sqlite3.h` directly into `M12`/`M15` so `use c "sqlite3";` yields `[Layout(c_abi)]` `0b00` structs and `0b10` `ForeignHandle<sqlite3>` automatically!
  2. **Rust (`DepSpec::cargo("regex", "1.10.0")`)**: Invokes `cargo build --release --lib` into `.pilot-cache/cargo/`, extracts `M16` `rust_sig` metadata, and maps `&[u8]` $\leftrightarrow$ `0b01` `slice<u8>` and `Vec<T>` $\leftrightarrow$ `0b10` `RustVec<T>`.
  3. **Python (`DepSpec::pip("numpy", "2.0.0")`) & JS (`DepSpec::npm("zod", "3.23.0")`)**: Locks wheel/tarball SHA-256 in `pkg.lock`, extracts `.pyi` (`M17`) and `.d.ts` (`M18`) stubs into `PilotDb`, and configures in-process `libpython` / Node-API paths at link time.

#### 42.4 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `POLY001` (Foreign Dependency Target Architecture / ABI Mismatch Against Active Pilot Target Triple), `POLY002` (Duplicate Hand-Written Wrapper Struct Shadows Absorbed Foreign Type — Violates `imp_type.md` Rule 9), `POLY003` (Conflicting Native Symbol Definitions Across Two Foreign C/Rust Static Archives), `POLY004` (Unpinned or Unhashed Foreign Wheel/Crate/Tarball in `pkg.lock`), `POLY005` (Missing Foreign Header, `.pyi`, `.d.ts`, or `.wit` Interface File in Resolved Package).
- **Verification Suite (`crates/pilot-analysis/tests/milestone42_polyglot_package_and_build_orchestrator.rs`):**
  1. `test_m42_1_multi_ecosystem_pkg_pilot_resolution_and_cas_artifact_caching`: Resolves a `pkg.pilot` combining C, Rust Cargo, Python Wheel, NPM, and WASM WIT dependencies $\rightarrow$ verifies deterministic build-graph topological order and 100% cache hit on unchanged SHA-256 inputs.
  2. `test_m42_2_direct_foreign_type_absorption_and_duplicate_facade_ban_poly002`: Mounts a foreign C/Rust dependency into `PilotDb` and verifies `POLY002` flags a redundant user-written facade struct that duplicates the absorbed `[Layout(c_abi)]` struct.
  3. `test_m42_3_target_abi_mismatch_and_symbol_collision_detection_poly001_003`: Verifies `POLY001` rejects linking an `x86_64` foreign static library into an `aarch64` build and `POLY003` detects duplicate C symbol exports.
  4. `test_m42_4_hermetic_venv_and_linker_rpath_flag_synthesis`: Verifies POSIX/Windows linker argument generation and isolated `.pilot-venv` environment setup.

---

### M43 — Resident LSP v2 (`pilot lsp` — Sub-10ms Intelligence & Adaptable Snippets)

#### 43.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`handlers.pilot` Uses Linear AST Scans & Raw `__engine(Cfg.ast.*)` Calls Instead of Incremental `PilotDb`):** [`selfhost/lsp/handlers.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/lsp/handlers.pilot) (lines `23`–`98`) scans all `node_count` AST nodes linearly on every hover/completion request (`while i < node_count { ... }`) and calls raw `__engine("source_slice", ...)` byte-by-byte backward to find `///` doc comments!
- **Root Bottleneck 2 (Missing 2-Bit Lattice Inlay Badges, Missing `imp_type.md` Real-Time Code Actions, and Missing Dual-Syntax/Weave Lenses):** Current LSP handlers do not render inline 2-bit hardware lattice inlay hints (`/* 0b00 Value */`, `/* 0b01 View */`, `/* 0b10 Object */`, `/* 0b11 Guard */`, `/* drop_reuse */`), do not surface real-time `imp_type.md` refactoring code actions (expanding wildcard `use pkg.*;` into targeted imports per Rule 9, converting $\ge 3$ booleans to `bitflags` per Rule 5, or replacing raw `__engine` calls per Rule 10), and do not offer 1-click Compact $\leftrightarrow$ Readable syntax toggling (`M4`)!

#### 43.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/lsp/inlay_and_lenses.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Lsp.InlayLenses;` | `~380L` | Sub-10ms inlay hints & code lenses displaying 2-bit hardware lattice badges (`0b00`–`0b11`), inferred `M31` Perceus borrows & `drop_reuse` sites, `M22` 4-tier `MethodTable` winners, `M21`/`M30` weave expansions, and `.pchor` role/cue routes. |
| **CREATE** | `selfhost/lsp/code_actions_imp.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Lsp.CodeActions;` | `~360L` | Lossless CST (`M53`/`M8`) code actions enforcing `imp_type.md`: wildcard import $\rightarrow$ targeted import expander, duplicate type consolidator, boolean-blindness $\rightarrow$ `bitflags` converter, and Compact (`T?`, `T!E`, `f!(x)`) $\leftrightarrow$ Readable (`opt<T>`, `res<T,E>`, `call f(x)`) syntax toggle! |
| **MODIFY** | `selfhost/lsp/handlers.pilot` & `selfhost/lsp/ide_db.pilot` | Self-Host LSP Server | `-140L / +260L` | Replace $O(N)$ linear AST scans with $O(1)$ `PilotDb` (`M5`) green-tree span index lookups, add semantic token delta streaming, and wire `inlay_and_lenses.pilot` + `code_actions_imp.pilot`. |

#### 43.3 Sub-10ms Incremental LSP Architecture & `imp_type.md` Live Refactorings
1. **$O(\log N)$ CST Span-to-Symbol Lookup (`ide_db.pilot`):**
   - Queries `PilotDb` (`M5`) red-green CST spine by byte offset in $O(\text{depth})$ ($\le 15$ tree hops) and reuses memoized `MethodTable` (`M22`) and `FnPerceusSummary` (`M31`) across keystrokes whenever function signatures are unchanged (`early cutoff`, `< 2 ms` latency!).
2. **5 First-Class LSP Inlay Hint & Code-Lens Categories (`inlay_and_lenses.pilot`):**
   - **Lattice Badge Hints:** `: Vec3 /* 0b00 Value, 12B */`, `: slice<u8> /* 0b01 View */`, `: List<Order> /* 0b10 Object */`, `: ArenaScope /* 0b11 Guard (LIFO exit line 48) */`.
   - **Perceus Lifecycle Hints (`M31`):** Marks parameters downgraded to `::ReadOnlyBorrow0b01` (`/* borrowed (0 refcount ops) */`) and constructor sites paired via `drop_reuse` (`/* recycles 'old_node' in-place */`).
   - **4-Tier Method Provenance Lens (`M22`):** Shows `[Tier 2: Spliced from SpatialBundle (coalesced 'id' @ +0B)]`.
   - **Weave Expansion Lens (`M21`/`M30`):** `[Weave: 4 methods synthesized by [Json, Entity] — Click to Inspect or Eject to Tier 1]`.
3. **Lossless CST Code Actions (`code_actions_imp.pilot`):**
   - **`lsp.action.expandTargetedImports` (`imp_type.md` Rule 9):** Scans symbol references in the current file against a wildcard `use std.col.*;` and rewrites it losslessly into `use std.col.{List, Map};`!

#### 43.4 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `LSP001` (Wildcard Import `use ...*;` Used Where $\le 4$ Specific Symbols Are Referenced — Suggests Targeted Import), `LSP002` (Cross-Package Rename Blocked on Sealed Foreign C-ABI Symbol Without `[RenameAlias]`), `LSP003` (Incremental Document Sync Range Out of Bounds).
- **Verification Suite (`crates/pilot-analysis/tests/milestone43_resident_lsp_v2_and_imp_type_actions.rs`):**
  1. `test_m43_1_sub10ms_inlay_hints_lattice_badges_and_perceus_drop_reuse_lenses`: Queries `inlay_and_lenses.pilot` on a module containing `0b00`, `0b01`, `0b10`, `0b11` bindings, `M31` inferred borrows, and `M22` `*Bundle` methods $\rightarrow$ verifies exact badge annotations and Tier provenance.
  2. `test_m43_2_targeted_import_expander_and_imp_type_code_actions`: Applies `expandTargetedImports` on a file with `use std.col.*;` referencing only `List` and `Map` $\rightarrow$ verifies lossless CST rewrite to `use std.col.{List, Map};` preserving comments!
  3. `test_m43_3_bidirectional_compact_readable_syntax_toggle_code_action`: Toggles `res<opt<slice<i64>>, IoErr>` $\leftrightarrow$ `i64[]? ! IoErr` and `call f(x)` $\leftrightarrow$ `f!(x)` via LSP code action and verifies AST round-trip identity.
  4. `test_m43_4_workspace_rename_and_semantic_tokens_delta_early_cutoff`: Exercises cross-file symbol rename and verifies `PilotDb` early-cutoff avoids re-typechecking downstream modules on body-only edits.

---

### M44 — Unified DAP Debugger v2 (`pilot debug` — Native DWARF + VM + Foreign Frames)

#### 44.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`selfhost/dap/server.pilot` Is a 39-Line Stub Returning a Hardcoded `main` Frame!):** [`selfhost/dap/server.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/dap/server.pilot) (lines `1`–`39`) hardcodes `build_dap_stack_trace_frame(seq, "fn main", 1, 1)` on line `33`! Meanwhile, [`selfhost/dap/vm_controller.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/dap/vm_controller.pilot) and [`selfhost/dap/inspector.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/dap/inspector.pilot) only inspect scalar `PilVM` bytecode registers.
- **Root Bottleneck 2 (No Native DWARF v5 Unwinder, No Foreign Frame Stitching, and No 2-Bit Lattice Memory Inspector):** When stepping from a compiled Pilot function (`M33`/`M35`) into a `comptime` weaver (`M21`/`M30`) or across a `use c`/`use cpp`/`use rust`/`use py` (`M12`–`M17`) boundary, there is no unified stack unwinder stitching DWARF `.eh_frame`/`.debug_line` frames with `PilVM` and foreign frames, nor does variable inspection display 2-bit hardware lattice classes (`0b00`–`0b11`), `SliceBuffer` `(len, cap)`, or active `0b11` `IGuard` scope countdowns!

#### 44.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/dap/dwarf_unwinder.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Dap.DwarfUnwind;` | `~390L` | Reads DWARF v5 `.debug_line`, `.debug_info`, and `.eh_frame` tables (`M33`) to map native instruction pointers (`RIP`/`PC`) to `(source_file, line, col)` and reconstruct struct fields by `[Layout(c_abi)]` byte offsets! |
| **CREATE** | `selfhost/dap/polyglot_stitcher.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Dap.PolyglotStitch;` | `~330L` | Unifies stack traces across Native Pilot frames, `PilVM` `comptime` frames, and C/C++/Rust/Python/WASM adapter frames (`M12`–`M19`), plus `.pchor` (`M24`) async `cue` causal sender $\rightarrow$ receiver trace links! |
| **MODIFY** | `selfhost/dap/server.pilot`, `inspector.pilot`, `repl_eval.pilot` | Self-Host DAP v2 | `-20L / +290L` | Replace hardcoded `stackTrace` in `server.pilot` with real multi-frame unwinding, add conditional/hit-count/data breakpoints, and enrich `inspector.pilot` with 2-bit lattice badges, `SoA` (`M25`) logical row views, and `0b11` guard lifecycles. |

#### 44.3 Lattice-Aware Variable Inspection & Cross-Boundary Frame Stitching
- **`DebugFrameKind` (`enum : u8`, `0b00 : IValue`)**: `::NativePilotDwarf = 1`, `::PilVmComptime = 2`, `::ForeignCOrCpp = 3`, `::ForeignRust = 4`, `::ForeignPython = 5`, `::ForeignWasm = 6`, `::PchorCueCausalLink = 7`.
- **`[Layout(c_abi)] pub struct LatticeWatchValue : IValue`** (`0b00`):
  - `pub var_name: str; pub type_name: str; pub lattice_bits: u8; pub byte_size: u32; pub ref_count: u32; pub len_elems: usize; pub cap_elems: usize; pub formatted_summary: str;`
- **Transparent `SoA` (`M25`) & `0b11` Guard Visualization in `inspector.pilot`:**
  1. When inspecting an `M25` `SoA<Particle>` storage table at index `i`, `inspector.pilot` gathers column pointers `col_pos_x[i]`, `col_pos_y[i]`, `col_vel_x[i]` using `StoragePlan` (`M25`) and displays a clean logical `Particle { pos: Vec3 { ... }, vel: Vec3 { ... } }` alongside `[Physical Layout: SoA (64B aligned, stride 4B)]`!
  2. For every `0b10` `: IObject`, displays `[0b10 Object | rc: 1 (unique -> drop_reuse eligible)]`.
  3. For every `0b11` `: IGuard` (`ArenaScope`, `MutexGuard`, `PyGilGuard`), displays `[0b11 Scoped Guard | LIFO Exit Order #2 | Arena used: 4,096 / 65,536 B]`.

#### 44.4 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `DAP001` (Missing or Stripped DWARF `.debug_info` Section — Suggests Building with `-g`), `DAP002` (Watch Expression Attempted Impure Side Effect or Mutation While Target Is Paused in Read-Only Inspection Mode), `DAP003` (Invalid Memory Address or Unmapped Foreign Frame Pointer During Stack Unwind).
- **Verification Suite (`crates/pilot-analysis/tests/milestone44_unified_dap_debugger_v2_dwarf_and_polyglot.rs`):**
  1. `test_m44_1_dwarf_v5_pc_to_source_line_and_struct_byte_offset_reconstruction`: Maps native `RIP`/`PC` addresses through `dwarf_unwinder.pilot` to exact `.pilot` `(file, line, col)` coordinates and reconstructs nested `[Layout(c_abi)]` struct fields from raw frame memory bytes!
  2. `test_m44_2_polyglot_stack_stitching_native_pilvm_foreign_and_pchor_cues`: Unwinds a 5-frame mixed call stack (`Python caller -> Pilot [Service] -> C sqlite3_step -> .pchor cue origin`) and verifies complete DAP `stackTrace` JSON output.
  3. `test_m44_3_lattice_inspector_0b00_to_0b11_soa_row_reconstruction_and_rc`: Inspects `0b00` values, `0b01` slices, `0b10` objects (`rc == 1`), `0b11` `ArenaScope` guards, and `M25` `SoA` logical rows.
  4. `test_m44_4_conditional_breakpoints_and_pure_watch_repl_eval_dap002`: Verifies hit-count/conditional breakpoints and verifies `DAP002` blocks mutating watch expressions in read-only pause mode.

---

### M45 — Native Pilot MCP Server v1 (`pilot mcp` — Compiler-in-the-Loop AI Engine)

#### 45.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`stdlib/pilot/` Has No `mcp.pilot` & AI Agents Must Blindly Grep Raw Text Files):** While [`AGENTS.md §3`](file:///home/moony/Documents/dev/projects/Pilot/AGENTS.md) lists `pilot.mcp` in Ring 3 (`stdlib/pilot/`) and [`MILESTONES_100.md` § M3](file:///home/moony/Documents/dev/projects/Pilot/MILESTONES_100.md) exports basic CLI JSON schemas, there is **no `stdlib/pilot/mcp.pilot` or `selfhost/mcp/` Model Context Protocol (MCP) JSON-RPC 2.0 server**! Without `pilot mcp`, AI coding assistants have to read thousands of lines of raw source files, guess struct layouts, risk duplicating types across files (`imp_type.md` Rule 9), or wait for full disk rebuilds to check if a generated snippet compiles!

#### 45.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/pilot/mcp.pilot` | `pub pkg pilot.mcp;` | `~290L` | Ring-3 public Model Context Protocol (`MCP` JSON-RPC 2.0 over stdio/SSE) schema types: `McpToolDef`, `McpResourceDef`, `McpSemanticQuery`, `McpDryRunPatch`, and `McpVerificationReceipt`. |
| **CREATE** | `selfhost/mcp/server.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Mcp.Server;` | `~420L` | Resident `PilotDb`-backed MCP server exposing 8 compiler-in-the-loop tools (`pilot_query_symbol`, `pilot_find_canonical_type`, `pilot_method_table`, `pilot_inspect_layout_perceus`, `pilot_pchor_topology`, `pilot_generate_targeted_imports`, `pilot_dry_run_patch`, `pilot_audit_imp_type`). |
| **CREATE** | `selfhost/cli/mcp_cmd.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Cli.McpCmd;` | `~210L` | `[Command(name: "mcp")]` CLI entrypoint (`pilot mcp serve --stdio` / `--port <u16>` / `--export-tools-json`). |

#### 45.3 The 8 Compiler-in-the-Loop MCP Tools & Millisecond Speculative Dry-Run
`selfhost/mcp/server.pilot` gives AI pair-programmers direct, zero-hallucination access to `PilotDb` (`M5`), `MethodTable` (`M22`), `Perceus` (`M31`), and `ImpTypeAuditor` (`M38`):

| MCP Tool Name | Input Parameters | Exact Compiler Intelligence Returned (`< 5 ms`) |
| :--- | :--- | :--- |
| **`pilot_find_canonical_type`** | `{ query_fields_or_name: str }` | Searches the workspace `ModuleGraph` (`M2`) for any existing `struct`/`enum`/`interface` matching the domain concept (`imp_type.md` Rule 9) and returns its canonical `use pkg.Type;` import path so the AI **never duplicates a type**! |
| **`pilot_query_symbol`** | `{ symbol: str }` | Returns compact + readable signature, 2-bit lattice class (`0b00`–`0b11`), byte size/alignment, active `state` typestates (`M26`), and `requires`/`ensures` contracts. |
| **`pilot_method_table`** | `{ type_name: str, clearance_lvl: i32 }` | Returns all callable methods across Tiers 1–4 (`M22`), spliced `*Bundle`/`*Facet` origins, and visible `ext` extensions filtered by `lvl(N)` (`M28`). |
| **`pilot_inspect_layout_perceus`** | `{ target: str }` | Returns exact field byte offsets, niche packing, `M31` inferred `ParamOwnershipMode` (`0b01` vs `0b10`), and `drop_reuse` pairing sites. |
| **`pilot_pchor_topology`** | `{ blueprint: str }` | Returns `.pchor` roles, `reads`/`writes` storage permissions, `cue` payload lattice constraints, and `law` firewall rules (`M24`/`M25`). |
| **`pilot_generate_targeted_imports`** | `{ file_uri: str, code_snippet: str }` | Resolves all unbound identifiers in `code_snippet` against workspace exports and synthesizes exact targeted `use a.b.{X, Y};` lines (`0` wildcard imports!). |
| **`pilot_audit_imp_type`** | `{ file_uri: str }` | Runs `imp_type_auditor.pilot` (`M38`) on the file/patch, checking all 10 `imp_type.md` rules (`IMP001`–`IMP004`). |
| **`pilot_dry_run_patch`** | `{ edits: List<FileEdit> }` | Forks an in-memory copy-on-write `PilotDb` revision (`0` disk writes!), runs full typechecking + Perceus + `.pchor` + `imp_type.md` verification in $< 10\text{ ms}$, and returns structured diagnostics + `--fix` suggestions! |

#### 45.4 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `MCP001` (Invalid JSON-RPC 2.0 Frame or Unknown MCP Tool Name), `MCP002` (`pilot_dry_run_patch` Rejected Candidate Patch Due to `imp_type.md` Violation or Type Error — Returns Structured Fix Payload), `MCP003` (Speculative Dry-Run Fork Exceeded Sandbox Memory/Fuel Quota).
- **Verification Suite (`crates/pilot-analysis/tests/milestone45_native_pilot_mcp_server_v1.rs`):**
  1. `test_m45_1_mcp_find_canonical_type_and_targeted_import_synthesis`: Queries `pilot_find_canonical_type` for `"Order"` and `pilot_generate_targeted_imports` for a snippet $\rightarrow$ verifies exact canonical `use` path returned with `0` duplicate types and `0` wildcard imports!
  2. `test_m45_2_mcp_query_symbol_method_table_and_perceus_layout_inspection`: Exercises `pilot_query_symbol`, `pilot_method_table`, and `pilot_inspect_layout_perceus` over JSON-RPC 2.0.
  3. `test_m45_3_in_memory_speculative_dry_run_patch_and_rollback_isolation`: Submits both a flawed patch (containing a raw `__engine` call and a `0b01` escape error) and a valid patch to `pilot_dry_run_patch` $\rightarrow$ proves the flawed patch returns exact `IMP001`/`ESCAPE001` diagnostics without mutating the base `PilotDb` state, and the valid patch returns `verified: true` in $< 10\text{ ms}$!
  4. `test_m45_4_mcp_pchor_topology_and_architectural_law_query`: Queries `pilot_pchor_topology` and verifies AI agent receives exact role storage ACLs and `law` constraints before writing code.

---

### M46 — Unified Test, Doc-Test, Differential & Coverage-Guided Fuzz Runner (`pilot test`)

#### 46.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`test_runner.pilot` & `diff_test.pilot` Lack Doc-Test Extraction, Coverage-Guided Fuzzing & Perceus Leak Gates):** [`selfhost/cli/test_runner.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/cli/test_runner.pilot) (111L) only discovers `test_*` functions, and [`selfhost/cli/diff_test.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/cli/diff_test.pilot) (59L) only compares two pre-supplied output strings! There is **no extractor** compiling `///` markdown code blocks as verified doc-tests, **no layout-aware coverage-guided fuzzer (`[Fuzz]` / `pilot test --fuzz`)** mutating `[Layout(c_abi)]` `0b00` structs and `0b01` slices via edge-coverage bitmaps, and **no automatic per-test Perceus zero-leak assertion**!

#### 46.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/std/test/fuzz_and_prop.pilot` | `pub pkg std.test.fuzz;` | `~390L` | Structure-aware coverage-guided fuzzer (`FuzzCorpus`, 64KB edge-hit counter bitmap, deterministic PRNG mutators driven by `[Layout(c_abi)]` field descriptors, and automatic counterexample shrinker!). |
| **CREATE** | `selfhost/cli/doctest_extractor.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Cli.DocTest;` | `~310L` | Extracts ```pilot fenced blocks from `///` and `//!` doc comments via lossless CST (`M10`), wraps them in hermetic test harnesses preserving exact source line numbers, and verifies `should_panic` / `compile_fail(CODE)` directives. |
| **MODIFY** | `selfhost/cli/test_runner.pilot` & `diff_test.pilot` | Self-Host Test Runner | `+240L` | Integrate unit tests (`[Test]`), doc-tests (`--doc`), differential foreign-oracle tests (`--diff` across PilVM/C11/LLVM/Native/Python/Node), coverage-guided fuzzing (`--fuzz`), and mandatory post-test Perceus `live_allocs == 0` checks. |

#### 46.3 Layout-Aware Coverage-Guided Fuzzing & Counterexample Shrinking (`fuzz_and_prop.pilot`)
1. **Name-Blind Layout-Guided Input Generation (`FuzzInputReader`):**
   - Given `[Fuzz] pub fn fuzz_wire_parser(pkt: PacketHeader, payload: slice<u8>) -> res<Frame, WireErr>`:
     - Inspects `PacketHeader`'s `PhysicalFieldSlot` (`M22`) layout descriptors (`gpr_unsigned`, `gpr_signed`, `simd_float`, `enum` discriminant bounds, `requires` refinement ranges `M26`) to synthesize type-valid and boundary-value (`0`, `1`, `MAX`, `NaN`, `-0.0`) struct instances automatically without user-written `Arbitrary` boilerplate!
2. **Edge-Coverage Feedback Loop & Delta-Debugging Shrinker:**
   - Instruments basic-block transitions $(b_{\text{prev}} \oplus b_{\text{curr}}) \bmod 65536$ in an 8-bit hit-count bitmap. Any mutation (`BitFlip`, `ByteArith`, `BoundaryInsert`, `SliceSplice`) that discovers a new basic-block edge or new shift bucket is retained in `FuzzCorpus`.
   - When a contract (`M26` `ensures`), invariant, or unhandled panic fails, runs binary-search delta-debugging (`shrink_input`) to minimize the failing input to its smallest reproducible counterexample and saves it to `tests/fuzz_regressions/<fn_name>.bin`!

#### 46.4 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `TEST001` (`///` Doc-Test Code Block Failed to Compile or Violated Expected `compile_fail(DIAG_CODE)`), `TEST002` (Perceus Memory or `0b11` Guard Leak Detected After `[Test]` Execution: `live_allocs > 0`), `TEST003` (Differential Oracle Divergence Between Execution Backends), `TEST004` (`[Fuzz]` Counterexample Triggered `ensures`/`invariant` Contract Violation — Minimal Shrunk Seed Emitted).
- **Verification Suite (`crates/pilot-analysis/tests/milestone46_unified_test_doctest_differential_and_fuzz_runner.rs`):**
  1. `test_m46_1_doctest_extraction_line_preservation_and_compile_fail_directives`: Extracts and executes `///` doc-tests (including a `/// ```pilot,compile_fail(ESCAPE001)` negative test) and verifies pass/fail reporting with exact file:line spans.
  2. `test_m46_2_layout_guided_fuzzer_discovers_edge_coverage_and_shrinks_counterexample`: Runs `fuzz_and_prop.pilot` on a parser with a planted boundary bug (`x == 0xDEAD && len > 3`) $\rightarrow$ proves edge-coverage mutation finds the bug and shrinks the failing slice down to the minimal 4-byte seed!
  3. `test_m46_3_multi_backend_differential_oracle_pilvm_c11_llvm_native`: Runs a suite of algebraic, bitwise, and `SoA` functions through `diff_test.pilot` across PilVM, C11, LLVM, and Direct Native backends $\rightarrow$ asserts 100% bit-identical outputs (`TEST003` triggers on planted divergence).
  4. `test_m46_4_per_test_perceus_zero_leak_and_guard_lifo_certification_test002`: Verifies `TEST002` immediately catches any test leaving an un-dropped `0b10` allocation or unclosed `0b11` guard.

---

### M47 — Living Documentation & Blueprint Visualizer (`pilot doc`)

#### 47.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (No `selfhost/doc/` Engine Generating Searchable Docs, Dual-Syntax Signatures, 2-Bit Lattice Badges, or `.pchor` Topology Diagrams):** While `selfhost/lsp/handlers.pilot` extracts single-node hover strings and `selfhost/chor/lower.pilot` (`M24`) emits basic Mermaid strings, there is **no `pilot doc` static site & JSON doc-index generator**! Developers cannot browse package APIs with live Readable $\leftrightarrow$ Compact (`M4`) signature toggles, 2-bit hardware lattice badges (`0b00` Value, `0b01` View, `0b10` Object, `0b11` Guard), exact `[Layout(c_abi)]` byte-offset tables (`M7`), 4-tier `MethodTable` (`M22`) inherited/spliced member sections, or canonical targeted import headers (`use pkg.Symbol;` per `imp_type.md` Rule 9)!

#### 47.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/doc/extractor.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Doc.Extractor;` | `~380L` | Queries `PilotDb` (`M5`) across workspace packages to build `DocPackageModel` containing dual-syntax signatures (`M4`), 2-bit lattice badges, `[Layout]` byte tables, `M22` `*Bundle`/`*Facet` provenance, `M26` contracts/typestates, and `M28` `lvl(N)` clearance rings. |
| **CREATE** | `selfhost/doc/html_and_search.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Doc.HtmlSearch;` | `~410L` | Emits self-contained, zero-external-CDN static HTML/CSS/JS documentation + client-side prefix/type-signature search index (`search-index.json`, supporting queries like `slice<T> -> res<usize, IoErr>` or `0b00`!) + embedded `.pchor` Mermaid architecture diagrams. |
| **CREATE** | `selfhost/cli/doc_cmd.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Cli.DocCmd;` | `~210L` | `[Command(name: "doc")]` CLI tool (`pilot doc --open`, `--json`, `--check-links`, `--max-lvl <N>`). |

#### 47.3 Living Documentation Model & 6 First-Class Visual Cards
For every documented `struct`, `enum`, `interface`, `*Bundle`, `*Facet`, `fn`, `syntax`, and `.pchor` `blueprint`, `extractor.pilot` synthesizes:
1. **Canonical Targeted Import Banner (`imp_type.md` Rule 9):** Displays copyable `use std.col.List;` (never wildcard `use std.col.*;`) at the top of every type/function card.
2. **Dual Readable $\leftrightarrow$ Compact Signature Switcher (`M4`):** Renders both forms side-by-side (`pub fn parse(src: u8[]) -> Packet? ! WireErr` $\leftrightarrow$ `pub fn parse(src: slice<u8>) -> res<opt<Packet>, WireErr>`) with instant toggle!
3. **2-Bit Hardware Lattice & Memory Physics Badge:**
   - `0b00 Value (16B, align 8B, register-passed, 0 heap allocs)`
   - `0b01 Reference View (16B borrow window — cannot outlive source)`
   - `0b10 Owned Object (Perceus ARC + drop_reuse eligible)`
   - `0b11 Scoped Guard (LIFO ~self() at scope exit — cannot escape scope)`
4. **4-Tier `MethodTable` & `*Bundle`/`*Facet` Provenance Accordion (`M22`):** Groups methods by `Tier 1: Explicit Struct`, `Tier 2: Spliced from *Bundle / *Facet / delegate`, and `Tier 3–4: Interface Defaults`.
5. **Typestate Transition Graph & Contracts (`M26`):** Renders `state` transitions (`::Closed -> connect() -> ::Open -> close() -> ::Closed`) and `requires`/`ensures` clauses as an interactive state diagram!
6. **`.pchor` Blueprint Architecture Map (`M24`/`M25`):** Embeds the full Mermaid `flowchart LR` diagram showing `role`s, `SoA` `storage` bindings, `cue` channels, and `law` firewalls.

#### 47.4 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `DOC001` (Broken Intra-Doc Symbol Link `[[UnresolvedSymbol]]` in `///` Comment), `DOC002` (`pub` Exported Symbol in Ring-1/Ring-2 Library Missing `///` Documentation Comment When `--require-docs` Is Enabled), `DOC003` (Documented `lvl(0)` Internal Symbol Leaked Into Public `--max-lvl 2` External Documentation Build).
- **Verification Suite (`crates/pilot-analysis/tests/milestone47_living_documentation_and_blueprint_visualizer.rs`):**
  1. `test_m47_1_doc_extractor_dual_signatures_lattice_badges_and_targeted_imports`: Extracts `DocPackageModel` for a package with `0b00`–`0b11` types, `*Bundle` splices, and `state` typestates $\rightarrow$ verifies exact lattice badges, byte sizes, and canonical targeted `use` paths.
  2. `test_m47_2_type_signature_search_index_and_pchor_mermaid_diagram_embedding`: Queries `search-index.json` by both symbol name and type signature (`"slice<u8> -> res"`) and verifies embedded `.pchor` Mermaid diagram output.
  3. `test_m47_3_intra_doc_link_checker_and_clearance_ring_filtering_doc001_003`: Verifies `DOC001` catches broken `[[MissingType]]` links and `DOC003` filters `lvl(0)` internal compiler symbols out of public docs.
  4. `test_m47_4_deterministic_html_and_json_doc_bundle_generation`: Verifies `pilot doc --json` and HTML generation are 100% deterministic across runs.

---

### M48 — Single-Command Cross-Compilation & Asset Bundling (`pilot ship`)

#### 48.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (`build_cmd.pilot` Only Compiles Single-File Local Outputs Without Whole-Program DCE, Cross-Target Packaging, or `[Embed]` Assets):** [`selfhost/cli/build_cmd.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/cli/build_cmd.pilot) (lines `1`–`129`) invokes single-unit compilation without whole-program reachability dead-code elimination (DCE), cross-package duplicate-type stripping (`imp_type.md` Rule 9), zero-allocation `[Embed("assets/...")]` binary asset baking, or reproducible single-command multi-OS release archiving (`linux-x64/arm64`, `macos-x64/arm64`, `windows-x64/arm64`, `wasm32-wasip2`)!

#### 48.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `selfhost/codegen/whole_program_dce.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Codegen.WholeDce;` | `~360L` | Whole-program call-graph & type-graph reachability pass from `main` / `export` roots, stripping unused functions, unused woven methods (`M21`/`M30`), and unreferenced layout descriptors (`40%`–`70%` binary size reduction!). |
| **CREATE** | `selfhost/codegen/asset_embedder.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Codegen.AssetEmbed;` | `~330L` | Compiles `[Embed("path/to/file")]` / `[EmbedDir("static/")]` into `.rodata` 64-byte aligned read-only sections exposed as `0b01` `slice<u8>` / `str` views with **`0` startup heap allocations and `0` runtime file I/O** (`SHIP001`–`SHIP004`)! |
| **CREATE** | `selfhost/cli/ship_cmd.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Cli.ShipCmd;` | `~380L` | `[Command(name: "ship")]` single-command cross-compiler & release packager emitting stripped native binaries (`M33`/`M35`), `.tar.gz`/`.zip` archives, SPDX SBOM manifests, and SHA-256 provenance checksums (`SOURCE_DATE_EPOCH = 0` reproducible builds). |

#### 48.3 Zero-Allocation `.rodata` Asset Embedding & Whole-Program LTO/DCE
1. **Zero-Allocation `0b01` Embedded Assets (`asset_embedder.pilot`):**
   - Writing `[Embed("migrations/001_init.sql")] pub const INIT_SQL: str;` or `[Embed("weights/model.bin", align: 64)] pub const MODEL_WEIGHTS: slice<u8>;`:
     - Validates the path inside `M29`'s `ComptimeGuardian` (blocking `..` traversal via `CTIME001` / `SHIP001`!).
     - Places the raw bytes directly into the binary's 64-byte aligned `.rodata` section (so `MODEL_WEIGHTS` can be cast directly to `simd<f32, 16>` or `SoA` views in place without copying a single byte to the heap!).
     - Binds `MODEL_WEIGHTS` as a compile-time constant `0b01` `slice<u8> { data: &__pilot_rodata_asset_0, len: N }`.
2. **Whole-Program Reachability Tree-Shaking (`whole_program_dce.pilot`):**
   - Marks `main`, `[Export]` C-ABI symbols, and `.pchor` active `role` handlers as roots.
   - Walks `PilIR` (`M6`) call edges and vtable/interface dispatch targets; strips all unreachable functions, unused `[Json]`/`[Binary]`/`[Entity]` woven methods, and dead string literals before invoking `M33` LLVM or `M35` Direct Native emission.
3. **Reproducible Multi-Target Matrix (`pilot ship --targets all`):**
   - Emits deterministic binaries for `linux-x86_64`, `linux-aarch64`, `macos-aarch64`, `macos-x86_64`, `windows-x86_64`, `windows-aarch64`, and `wasm32-wasip2`, zeroing archive timestamps (`SOURCE_DATE_EPOCH = 0`) and sorting symbol/section tables canonically so two builds on different machines produce **bit-identical SHA-256 release artifacts**!

#### 48.4 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `SHIP001` (`[Embed]` Asset Path Not Found or Escapes Package Root via `..`), `SHIP002` (`[Embed]` File Declared as `str` Contains Invalid UTF-8 Byte Sequence — Suggests `slice<u8>`), `SHIP003` (Cross-Compilation Target Requires Foreign Dynamic Library Not Available for Target OS/Arch), `SHIP004` (Non-Deterministic Build Input Detected During `--reproducible` Verification).
- **Verification Suite (`crates/pilot-analysis/tests/milestone48_single_command_cross_compilation_and_asset_bundling.rs`):**
  1. `test_m48_1_whole_program_dce_strips_unreachable_functions_and_woven_methods`: Compiles a module with 20 library functions and `[Json, Binary, Eq, Clone]` weavers where `main` only calls 2 functions and `to_json` $\rightarrow$ proves `whole_program_dce.pilot` strips all 18 unused functions and unused `[Binary]`/`[Clone]` weavers!
  2. `test_m48_2_rodata_asset_embedding_64b_alignment_and_zero_heap_alloc_slice`: Embeds binary and UTF-8 assets via `asset_embedder.pilot`, verifies 64-byte `.rodata` alignment, `0` startup heap allocations, and `SHIP001`/`SHIP002` diagnostics on missing/non-UTF-8 files.
  3. `test_m48_3_multi_target_cross_compilation_matrix_and_spdx_sbom`: Runs `ship_cmd.pilot` across all 7 release targets (`linux-x64/arm64`, `macos-arm64/x64`, `windows-x64/arm64`, `wasm32-wasip2`) and verifies target headers + SPDX SBOM JSON + SHA-256 manifest output.
  4. `test_m48_4_bit_identical_reproducible_build_across_timestamps`: Runs `pilot ship --reproducible` under two different simulated host clocks/paths and asserts bit-for-bit identical output SHA-256 hashes.

---

### M49 — `Edition 1.0` Specification Freeze, `pilot doctor` & Onboarding Polish

#### 49.1 Objective & Current Codebase Audit
- **Root Bottleneck 1 (No `Edition 1.0` Compatibility Charter Lock, No `pilot doctor` Workspace Health Auditor, and No Interactive Tier 1 $\rightarrow$ Tier 4 Onboarding Runner):** As Pilot approaches the 1.0 release (`M50`), the 14 seed tokens, dual syntax pairings (`M4`), 2-bit hardware lattice (`0b00`–`0b11`), `[Layout(c_abi)]` rules, and 3-Ring stdlib namespaces (`M9`) must be formally stamped under `Edition 1.0` so future `2.0` evolutions (`M52`) never break 1.0 code! Furthermore, developers need a single `pilot doctor` command that audits host toolchains, C11 seed parity (`M39`), `pkg.lock` integrity (`M41`), and workspace-wide `imp_type.md` compliance (`M38`/`M43`).

#### 49.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **CREATE** | `stdlib/pilot/edition.pilot` | `pub pkg pilot.edition;` | `~260L` | Defines `Edition` (`::Edition1_0`), ABI stability invariants, 14-seed-token grammar lock, and diagnostic severity profiles for `Edition 1.0` packages. |
| **CREATE** | `selfhost/cli/doctor_cmd.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Cli.DoctorCmd;` | `~390L` | `[Command(name: "doctor")]` comprehensive workspace & environment auditor verifying C11/LLVM/GPU host toolchains, `pkg.lock` Merkle integrity, 3-Ring `RING001` boundaries, and all 10 `imp_type.md` rules with 1-click `--fix` repair! |
| **CREATE** | `selfhost/cli/tour_cmd.pilot` | `pub lvl(0) pkg Pilot.SelfHost.Cli.TourCmd;` | `~310L` | `[Command(name: "tour")]` interactive progressive onboarding curriculum guiding developers from **Tier 1** (scripting & local inference) $\rightarrow$ **Tier 2** (`res`/`opt`, `*Bundle`, `[Service]`) $\rightarrow$ **Tier 3** (`0b00`–`0b11` lattice, `ArenaScope`, `.pchor` `SoA`, SIMD) $\rightarrow$ **Tier 4** (`syntax`, `quote`, `comptime` weavers). |

#### 49.3 `pilot doctor` 8-Point Workspace & Architecture Health Check
Running `pilot doctor [--fix] [--json]` executes 8 deterministic checks in $< 100\text{ ms}$:
1. **Check 1 (14-Seed-Token & Dual-Syntax Edition 1.0 Lock):** Verifies compiler and `stdlib/core/` grammar adherence to `Edition 1.0`.
2. **Check 2 (Host & Cross-Target Toolchain Readiness):** Probes C11 compiler (`cc`/`clang`/`gcc`), linker, WASM runtime, and GPU shader validators.
3. **Check 3 (`imp_type.md` Rule 9 — Single Source of Truth):** Scans the workspace for duplicate `struct`/`enum`/`interface` declarations across files/packages and flags wildcard `use ...*;` imports (`DOC_IMP001`).
4. **Check 4 (`imp_type.md` Rule 10 — Zero Raw `__engine` Calls):** Verifies zero unencapsulated `__engine(...)` calls exist in user or selfhost domain packages (`DOC_IMP002`).
5. **Check 5 (`imp_type.md` Rules 1–8 — Finite Sets, Bitflags, Constrained Generics & Lattice Boundaries):** Flags stringly-typed finite sets, boolean blindness ($\ge 3$ `bool` params), unconstrained `<T>`, and `0b10` parameters that could be `0b01` borrows (`M31`).
6. **Check 6 (3-Ring Dependency Firewall — `M9`):** Verifies `core.*` has `0` OS/host imports (`RING001`) and `std.*` has `0` Python-clone module names (`RING003`).
7. **Check 7 (`comptime` Sandbox & `pkg.lock` Security — `M29` & `M41`):** Verifies all `[comptime.grants]` in `pkg.pilot` are minimal and all `pkg.lock` SHA-256 Merkle digests match.
8. **Check 8 (Perceus & `.pchor` Topology Soundness — `M24` & `M31`):** Verifies zero `ESCAPE001`–`004` or `CHOR001`–`008` warnings exist across the workspace.

#### 49.4 Diagnostics & 4-Test Verification Suite
- **Diagnostics:** `ED001` (Package Declares Unsupported or Unknown `Edition` Version), `ED002` (Breaking C-ABI Layout Modification on `[StableAbi("1.0")]` Exported Struct), `DOCT001` (`pilot doctor` Detected Workspace `imp_type.md` Violation — Run `pilot doctor --fix`), `DOCT002` (Interactive `pilot tour` Exercise Verification Incomplete).
- **Verification Suite (`crates/pilot-analysis/tests/milestone49_edition_1_0_freeze_pilot_doctor_and_onboarding.rs`):**
  1. `test_m49_1_edition_1_0_abi_stability_and_14_seed_token_lock_ed001_002`: Verifies `Edition 1.0` manifest locking and `ED002` detection when a `[StableAbi("1.0")]` struct changes field offset or size.
  2. `test_m49_2_pilot_doctor_eight_point_audit_and_imp_type_autofix`: Runs `doctor_cmd.pilot` on a workspace seeded with a duplicate struct, a wildcard import, a raw `__engine` call, and a 4-bool function $\rightarrow$ verifies all 4 defects are caught and repaired via `--fix`!
  3. `test_m49_3_progressive_four_tier_interactive_tour_curriculum`: Exercises all 4 tiers of `tour_cmd.pilot` lessons and verifies live compilation + coach feedback (`M28`) on each tier's exercise.
  4. `test_m49_4_pilot_doctor_json_report_and_zero_warning_selfhost_certification`: Runs `pilot doctor --json` across the `stdlib/` and `selfhost/` directories and asserts `passed: true`.

---

### M50 — PILOT 1.0 FULL RELEASE & 4 Flagship Production Showcases

#### 50.1 Objective & Current Codebase Audit
- **Root Bottleneck (Unified 1.0 Release Certification Across All 50 Milestones):** [`crates/pilot-analysis/tests/milestone50_lossless_bridge_and_reflection.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/tests/milestone50_lossless_bridge_and_reflection.rs) already verifies the lossless AST bridge, open keyword reflection, and weight-sorted attribute weaver (`test_50_1` through `test_50_5`). To crown **Pilot 1.0**, `M50` unites those reflection invariants with the **4 Flagship Production Showcase Applications**, the **End-to-End Zero-Rust C11 Bootstrap Certification (`M39`)**, the **Dual-Pass Universal Rename Litmus (`M1`)**, and the **100% `imp_type.md` Workspace Certification (`M38`/`M49`)**!

#### 50.2 Module & File Manifest (`<= 1,500` LOC Law)
| Action | Path | Package & Ring | Budget | Purpose |
| :--- | :--- | :--- | ---: | :--- |
| **VERIFY** | `selfhost/ast_bridge.pilot` & `stdlib/core/reflect.pilot` | Lossless AST Bridge & Reflection | Existing | Preserves 100% lossless AST attributes, base interfaces, layout descriptors, and open keyword reflection (`test_50_1`–`test_50_5`). |
| **CREATE** | `examples/v1_showcases/microservice_orders.pilot` | Showcase 1: Web/DB/CLI Service | `~180L` | Full-stack `[Cli]` + `[Service]` + `[Entity]` + `.pchor` microservice (`M24`, `M30`, `M32` `ArenaScope`) handling HTTP + SQL + OpenAPI 3.1 with `0` reflection and `0` leaks. |
| **CREATE** | `examples/v1_showcases/polyglot_analytics.pilot` | Showcase 2: Polyglot Data Pipeline | `~180L` | Zero-copy C + Rust + Python NumPy `stride<f64>` + Protobuf/JSON pipeline (`M12`–`M20`, `M23` stream fusion) processing telemetry at native register speed. |
| **CREATE** | `examples/v1_showcases/gpu_autodiff_physics.pilot` | Showcase 3: SIMD + GPU + Autodiff | `~180L` | `.pchor` `SoA<Particle, align: 64>` (`M25`) + `f32@8` SIMD (`M34`) + `[Kernel]` GPU compute (`M36`) + `grad` (`!@`) symbolic optimization (`M37`) + `sig` (`!^`) telemetry. |
| **CREATE** | `examples/v1_showcases/custom_dsl_linter.pilot` | Showcase 4: Compiler-as-a-Library | `~160L` | Scoped `syntax` DSL (`M27`) + `pilot.db`/`pilot.cst`/`pilot.ir`/`pilot.mcp` (`M10`, `M45`) tool demonstrating Pilot's homoiconic metaprogramming power. |

#### 50.3 The Pilot 1.0 Release Certification Matrix (All 5 Phases `M1`–`M50`)
| Phase | Milestones | Core Architectural Pillar Certified for Pilot 1.0 | Verification Gate |
| :--- | :--- | :--- | :--- |
| **Phase 1** | `M1` – `M10` | **14-Seed-Token Name-Blind Engine, Dual Syntax (`M4`), `PilotDb` (`M5`), `PilIR` (`M6`), `pilot inspect` (`M7`), CST Autofix (`M8`), 3-Ring Stdlib (`M9`), Compiler-as-a-Library (`M10`)** | 40/40 Phase-1 Tests + Dual-Pass Rename Litmus PASS |
| **Phase 2** | `M11` – `M20` | **Universal Foreign-Language Adapter v1 (`adapter`, `use c`, `frame`/`!&`, Perceus Foreign Guards, `use cpp`, `use rust`, `use py`, `use js`/`ts`, `use wasm`, `use sql`/`openapi`/`proto`)** | 44/44 Phase-2 + Transport Tests PASS (`0` subprocess-per-call, `0` copy `0b01` slices) |
| **Phase 3** | `M21` – `M30` | **Declarative `[...]` Weavers (`M21`), 4-Tier `*Bundle`/`*Facet` (`M22`), Stream Fusion (`M23`), `.pchor` Choreographer & `SoA` (`M24`–`M25`), `state` & Contracts (`M26`), Scoped `syntax` (`M27`), 4-Tier Coach (`M28`), Sandboxed `comptime` (`M29`), `[Cli]`/`[Service]`/`[Entity]` (`M30`)** | 40/40 Phase-3 Tests PASS (`0` hardcoded keywords/attributes) |
| **Phase 4** | `M31` – `M40` | **Interprocedural Perceus & `drop_reuse` (`M31`), `ArenaScope` & `noalloc` (`M32`), LLVM 18+ (`M33`), Portable SIMD (`M34`), Direct x86_64/ARM64 Binary (`M35`), 6-Target GPU `[Kernel]` (`M36`), Autodiff & Topological Signals (`M37`), Self-Hosting Cutover (`M38`), C11 Seed (`M39`), `pilot bench` (`M40`)** | 40/40 Phase-4 Tests + Stage-2 == Stage-3 SHA-256 Fixed Point PASS |
| **Phase 5** | `M41` – `M50` | **`pkg.pilot` & CAS Lockfile (`M41`), Polyglot Orchestrator (`M42`), Resident LSP v2 (`M43`), Unified DAP v2 (`M44`), `pilot mcp` v1 (`M45`), Unified `pilot test --fuzz` (`M46`), `pilot doc` (`M47`), `pilot ship` (`M48`), `Edition 1.0` & `pilot doctor` (`M49`), **PILOT 1.0 RELEASE** (`M50`)** | 40/40 Phase-5 Tests + 4 Flagship Showcases + 85/85 Simultaneous Mega-Gate PASS |

#### 50.4 Pilot 1.0 Final Exit Sign-Off Criteria
1. **Zero Hardcoded Type or Attribute Names:** 100% of compiler, Perceus, IR, and codegen passes derive behavior strictly from the 5 Name-Blind Mechanisms (`[Bits]`/`[Layout]`, `TypeDef.fields` + 2-bit lattice `0b00..0b11`, `syntax type_construct`, `MethodTable`, and `TypeId` receptors).
2. **100% `imp_type.md` & Modularity Compliance:** Every `.pilot` and `.rs` file in the repository is $\le 1,500$ LOC (`MOD001`), obeys the 2-bit hardware lattice, and passes `pilot doctor` with `0` duplicate types, `0` unencapsulated `__engine` calls, and `0` memory leaks.
3. **Hermetic Zero-Rust Bootstrap:** `bootstrap/pilot_seed.c` (`M39`) builds the complete `pilot` 1.0 toolchain using only a standard C11 compiler on Linux, macOS, and Windows and reproduces bit-identical Stage-2 == Stage-3 binaries.

---

# PART II: THE ROAD TO PILOT 2.0 (`M51` – `M100`)

## Phase 6 (`M51` – `M60`): Live Codebase Adaptation, `pilot migrate` & Distributed `.pchor`

- [ ] **M51 — Semantic Codebase Refactoring Engine (`pilot migrate` & `[Migrate]` Rules)**
  - Attach declarative `[Migrate]` transformations to evolving APIs/structs; rewrite call sites, field splits, and argument reorderings losslessly across entire workspaces via Red/Green CST in milliseconds; automate migration of legacy codebases to `imp_type.md` (collapsing boolean blindness to `bitflags`, sentinel errors to `opt<T>`/`res<T, E>`, and duplicate types to canonical targeted imports).

- [ ] **M52 — Multi-Edition Seamless Compilation (`Edition 1.0` + `Edition 2.0` Interop)**
  - Compile packages targeting different Pilot Editions into shared `PilIR` within one binary, guaranteeing 2-bit hardware lattice classifications and `imp_type.md` typing contracts hold across editions.

- [ ] **M53 — `.pchor` Execution Topology Choreography (Monolith $\leftrightarrow$ Actors $\leftrightarrow$ Services)**
  - Re-bind `.pchor` `cue` routes between direct calls, M:N thread-pool queues, shared-memory IPC rings, and networked RPC/HTTP endpoints purely by editing `.pchor`—transferring `0b00` value records and `0b10` `iso` buffers with zero code duplication.

- [ ] **M54 — Zero-Copy `.pchor` Shared-Memory & Wire Transport (`0b00` Direct Framing)**
  - Transfer `0b00` `IValue` packets (`[Layout(c_abi)]`) and `iso` `0b10` slab arenas across process/network cues with zero serialization copies and compile-time layout hash/endianness verification per `imp_type.md` Rule 3 & 7.

- [ ] **M55 — Structured Deterministic Concurrency (`scope`, `task`, `chan`)**
  - Pure-Pilot M:N work-stealing scheduler where `0b11` `IGuard` task scopes statically guarantee child tasks never outlive parent scopes or leak background fibers, paired with typed `select` channel multiplexing.

- [ ] **M56 — Deterministic Simulation Testing & Time-Travel Debugging (`pilot debug --replay`)**
  - Record and deterministically replay multi-task `.pchor` schedules, timers, `res<T, E>` error branches, and `0b10` Perceus lifecycle events; step **backwards** in `pilot debug` to pinpoint concurrency bugs instantly.

- [ ] **M57 — Sub-20ms State-Preserving Binary Hot-Reload (`pilot run --watch`)**
  - Patch modified functions in running native/VM processes via `PilotDb` incremental codegen and automatically migrate live in-memory structs when fields are added or reordered based on `[Layout]` diffs.

- [ ] **M58 — Parallel `.pchor` ECS & Data-Oriented Scheduler**
  - Full archetype table storage combining `0b00` component value structs and `0b01` iteration views + compile-time disjoint read/write `role` scheduling, saturating all CPU cores and SIMD lanes automatically with zero heap allocations.

- [ ] **M59 — Production Transactional Sandboxes (`plan` / `!?` Copy-on-Write Runtime)**
  - Scale `EffectPlan<T>` (`plan` / `!?`) to snapshot `0b10` memory arenas, virtualize filesystem/DB mutations, inspect diff logs, and commit or roll back atomically in nanoseconds without resource leaks.

- [ ] **M60 — Architectural Law & Drift Verifier (`pilot chor verify`)**
  - Enforce custom codebase laws in `.pchor` (including `imp_type.md` rules: zero duplicate types, targeted import discipline, zero raw `__engine` calls, allocation ceilings) continuously on every keystroke in LSP and CI.

---

## Phase 7 (`M61` – `M70`): Deep Polyglot Mastery v2 — Bidirectional Ecosystem Fabric

- [ ] **M61 — Bidirectional C++20/23 Template Instantiation & Coroutine Adapter**
  - Instantiate C++ class/function templates on demand from Pilot generic arguments constrained via `interface` bounds (`<T : IConstraint>`) per `imp_type.md` Rule 8 and bridge C++20 `co_await` coroutines directly with Pilot `task` frames.

- [ ] **M62 — Bidirectional Rust Crate Export (`pilot ship --as rust-crate`)**
  - Export any Pilot package as an idiomatic, safe Rust crate (`Cargo.toml` + lifetime-checked wrapper types mapping Pilot `0b00` values to `Copy` structs, `0b01` `slice<T>` to `&[T]`, `0b10` to `Drop` objects, `opt<T>` to `Option<T>`, and `res<T, E>` to `Result<T, E>`) so Rust projects adopt Pilot modules effortlessly.

- [ ] **M63 — Universal Zero-Copy Data & Tensor Fabric (Apache Arrow, DLPack & Safetensors)**
  - Implement Arrow C Data Interface and DLPack natively on Pilot `0b01` `stride<T>` and `Table<T>`, enabling zero-copy CPU & GPU memory exchange with PyTorch, JAX, Polars, DuckDB, and cuDF.

- [ ] **M64 — Native JVM / Java / Kotlin Adapter (`use jvm` via Panama / JNI)**
  - Absorb `.jar`/class signatures at `comptime` with `0b11` JNI local-frame guards, Java Virtual Thread bridging, and `MemorySegment` zero-copy `slice<T>` sharing (`0b01`).

- [ ] **M65 — Native .NET / C# CLR Adapter (`use dotnet` via NativeAOT / HostFXR)**
  - Bidirectional C# `Span<T>` $\leftrightarrow$ `0b01` `slice<T>`, `Task<T>` $\leftrightarrow$ `task`, and value struct $\leftrightarrow$ `0b00 : IValue` bridging—giving C#/.NET codebases a natural zero-GC acceleration path.

- [ ] **M66 — Native Swift / Objective-C & Apple Metal Adapter (`use swift` / `use objc`)**
  - Unify Perceus ARC retain/release lifecycle with Swift/ObjC ARC, absorb Apple framework modules without duplicate facades, and pair seamlessly with Pilot's Metal (`MSL`) GPU backend.

- [ ] **M67 — Systems Interop Adapter for Zig & Go (`use zig`, `use go`)**
  - Bridge Zig C-ABI/build artifacts directly and handle Go cgo stack-switching / pinned-memory boundaries safely via `.pchor` async offload queues with zero memory leaks.

- [ ] **M68 — Incremental Legacy Codebase Absorber (`pilot absorb <dir>`)**
  - Upgrade Pilot's multi-language ingesters (`selfhost/transpile/`) to convert C, Python, TypeScript, or C# modules into clean Pilot + `.pchor` blueprints adhering strictly to `imp_type.md` (enums $\le 19$, open registries $20+$, `[Layout(c_abi)]` structs, `bitflags`, and targeted imports).

- [ ] **M69 — Universal Polyglot SDK Exporter (`pilot ship --sdk py,ts,rust,csharp,go,c,wasm`)**
  - Write core domain/engine logic **once** in Pilot and emit idiomatic, fully typed native packages (`.whl`, NPM N-API/WASM, Cargo crate, NuGet, Go module, `.h`/`.so`) adhering to target language equivalents of `imp_type.md`.

- [ ] **M70 — Unified Cross-Language Profiler (`pilot perf --polyglot`)**
  - Correlate stack samples, allocations, 2-bit lattice lifecycles, lock waits, and GPU kernel timelines across Pilot, C/C++, Rust, Python, and JS frames in a single interactive flamegraph.

---

## Phase 8 (`M71` – `M80`): Next-Gen AI Co-Design (`pilot mcp` v2), Self-Generation & UI

- [ ] **M71 — `pilot mcp` v2: Token-Minimal Semantic Graph Streaming**
  - Stream surgical, dependency-pruned semantic context slices (canonical types, active facets, contracts, `.pchor` role topology, targeted import paths) to AI agents in $<5\%$ of raw file token cost, ensuring generated code adheres to `imp_type.md` single-source definitions.

- [ ] **M72 — `pilot mcp` Speculative Multi-Patch Verifier & Counterexample Engine**
  - Evaluate candidate AI or refactoring patches inside isolated `PilotDb` memory forks: typecheck, verify `imp_type.md` compliance (rejecting duplicate types and raw `__engine` calls before disk commit), run Perceus checks, fuzz contracts, and return minimal failing inputs in $<50\text{ ms}$.

- [ ] **M73 — Typed Hole Sketching & Deductive Object Synthesis (`_?`)**
  - Support `_?` typed expression/implementation holes guided by surrounding types and `requires`/`ensures` contracts—synthesizing boilerplate adapters, mappers, and mock objects strictly adhering to `imp_type.md` target types and lattice classifications.

- [ ] **M74 — Programmable `comptime` Snippets & Live Refactoring Code Lenses**
  - Replace static text snippets with `comptime` Pilot snippet functions that inspect local scope types, `.pchor` roles, and interfaces to generate context-exact code blocks with targeted imports and zero boolean blindness.

- [ ] **M75 — Zero-VDOM Reactive Cross-Platform UI Engine (`std.ui`)**
  - Combine glitch-free `sig` (`!^`) signals, fluid `..` cascades, `0b00` layout nodes, and WGSL/native rendering into a declarative, `0.0 ms`-GC UI toolkit running identically on desktop, web (WASM+WebGPU), and mobile.

- [ ] **M76 — Compile-Time Verified Relational & Columnar Query Engine (`std.db` v2)**
  - Compile native Pilot query expressions against live schemas into vectorized SIMD in-memory scans or parameterized zero-allocation Postgres/MySQL/SQLite wire statements using strongly-typed column tokens and `[Layout(c_abi)]` row mappers.

- [ ] **M77 — SIMD-Accelerated Wire Codecs & Schema Evolution (`std.json` / `std.bin`)**
  - Synthesize branchless SIMD JSON and compact binary serializers at `comptime` (`>3 GB/s` throughput) generating `[Layout(c_abi)]` value packets with forward/backward field-version evolution rules.

- [ ] **M78 — Live Reactive Exploratory Workspace (`pilot studio`)**
  - Local reactive canvas/notebook driven by `PilotDb` and `sig` (`!^`): edit code, `.pchor` blueprints, or data inputs and watch plots, tables, and GPU views update deterministically with live type and lattice inspection.

- [ ] **M79 — Automated Layout & Performance Advisor (`pilot perf --advise`)**
  - Combine static struct padding analysis with hardware PMU profiles to recommend 1-click `imp_type.md` optimizations (`[Layout(optimal)]`, `.pchor` `SoA`, arena pooling, SIMD vectorization, or `bitflags` conversion) with projected speedup estimates.

- [ ] **M80 — Cryptographic Capability & Supply-Chain Proofs (`pilot audit`)**
  - Statically prove and lock the exact OS capabilities (`fs`, `net`, `env`, `ffi`, `asm`, `lvl` ring) reachable by every dependency in `pkg.pilot`, proving zero unauthorized unmanaged engine calls or memory leaks across the dependency graph.

---

## Phase 9 (`M81` – `M90`): Extreme Hardware Physics, Formal Proofs & 100% Pure-Pilot Repo

- [ ] **M81 — Profile-Guided `comptime` Specialization (`pilot build --pgo`)**
  - Feed runtime trace profiles back into `PilIR` `comptime` passes to devirtualize hot interface dispatches, specialize constrained generics (`<T : IConstraint>` per `imp_type.md` Rule 8), and inline across polyglot LTO boundaries.

- [ ] **M82 — Polyhedral Loop & GPU/SIMD Kernel Auto-Fusion**
  - Automatically tile, vectorize, and fuse multi-stage math loops over `0b01` slices and `grad` (`!@`) backward passes into single cache-resident CPU or GPU kernels, eliminating intermediate buffer allocations.

- [ ] **M83 — Declarative Hardware ISA & Accelerator Descriptor DSL**
  - Define CPU/DSP/NPU register classes, instruction encodings, and scheduling latencies in pure Pilot (`stdlib/arch/`) using name-blind `[Bits]` and `[Layout]` descriptors without compiler core edits.

- [ ] **M84 — Bare-Metal Kernel, Embedded Firmware & SVD MMIO Target (`core`)**
  - Generate type-safe volatile MMIO register structs directly from hardware `.svd` files at `comptime` as `[Layout(c_abi)] struct : IValue` with volatile bitfields, compiling strictly in freestanding `core.*` (`RING001`).

- [ ] **M85 — Static Cycle-Bounded `[Wcet]` & Stack-Depth Prover**
  - Prove static worst-case execution cycles and maximum stack bytes across `[HardRealtime]` call graphs (rejecting unbounded loops, recursion, or dynamic allocations on `[Layout(c_abi)]` structs).

- [ ] **M86 — SMT-Backed Static Contract & Bounds-Elision Prover (`pilot check --prove`)**
  - Discharge `requires`/`ensures` contracts, integer overflow checks, and slice bounds checks statically at compile time, formally verifying custom invariant value structs (`imp_type.md` Rule 10).

- [ ] **M87 — Multi-GPU & Cluster `.pchor` Sharding Choreography**
  - Extend `.pchor` storage/route blueprints to shard large `0b00` tables and `0b10` tensor buffers across multiple GPUs or cluster nodes with overlapped compute and collective communication.

- [ ] **M88 — Ultra-Tiny Binary & WASM Size Mode (`pilot build --tiny`)**
  - Layout-erased polymorphic codegen deduplication + aggressive dead-field and redundant type metadata stripping, producing `<50 KB` native CLI tools and `<15 KB` reactive WASM bundles.

- [ ] **M89 — Complete Retirement of Legacy `crates/` Workspace (100% Pure-Pilot Repo)**
  - Delete the remaining transitional Rust bootstrap directory (`crates/pilot-*`, `Cargo.toml`) completely. Every line of the compiler, runtime, Perceus verifier, backends, adapters, LSP, DAP, MCP, and gates is now **100% pure Pilot** adhering to `imp_type.md` (zero type duplication, strict targeted imports, zero raw `__engine` calls, 100% encapsulated standard types) bootstrapped from its portable C11/native seed.

- [ ] **M90 — Stage-4 Quad-Backend Fixed-Point Certification**
  - Prove deterministic Stage-2 == Stage-3 bit-for-bit self-compilation of `pilot` across all 4 primary native emitters simultaneously (LLVM, C11, direct machine code, and WASM), certifying type layout and lattice invariance across platforms.

---

## Phase 10 (`M91` – `M100`): Ecosystem Polish, Flagship Proof Systems & **PILOT 2.0 RELEASE**

- [ ] **M91 — Content-Addressed Package Registry & Semantic API Diffing (`pilot pkg publish/diff`)**
  - Launch the native `pilot pkg` registry protocol with reproducible build attestations, capability badges (`noalloc`, `pure`, `net`), and automatic semver breaking-change detection enforcing single source of truth across ecosystem types.

- [ ] **M92 — Universal Editor Suite (`pilot ide-sync` for VS Code, Zed, Neovim & JetBrains)**
  - Generate first-class editor extensions (Tree-sitter/TextMate grammars, LSP inlay lenses showing 2-bit lattice badges, DAP launch configs, MCP agent hooks) automatically from Pilot's single declarative registry.

- [ ] **M93 — Flagship Proof 1: Zero-GC High-Throughput HTTP/3 & Embedded DB Server**
  - Pure-Pilot service handling >1M req/s with flat sub-millisecond p99.99 latency, compile-time SQL verification, `0b00` framing, `0b11` leases, `res<T, E>` error flows, and live `.pchor` cluster switching.

- [ ] **M94 — Flagship Proof 2: Native + Polyglot AI Training & Inference Engine**
  - End-to-end neural training & inference engine in Pilot (`grad`/`!@` autodiff on `0b01` `stride<T>` and `Dual<T, G>` value records + fused `[Kernel]` GPU + zero-copy PyTorch/Arrow `use py` adapter) matching CUDA/C++ throughput in a fraction of the code.

- [ ] **M95 — Flagship Proof 3: Live-Reloading Multi-Target `.pchor` Simulation & UI**
  - 120fps `SoA` ECS physics + `std.ui` application running from a single Pilot codebase with `0b00` value components and `sig` reactive UI across desktop native, mobile, and WebGPU with sub-20ms state-preserving hot-reload.

- [ ] **M96 — Flagship Proof 4: Enterprise Polyglot Absorption & Universal SDK Showcase**
  - Demonstrate absorbing C (`sqlite3`), Rust, Python, and TypeScript modules into one cohesive Pilot application with zero duplicate type facades, then exporting back native Python, TS, Rust, C#, and C SDKs via `pilot ship --sdk`.

- [ ] **M97 — Global Benchmark & Readability Certification Suite**
  - Publish reproducible head-to-head benchmarks against C, C++, Rust, Zig, Go, C#, and Python across runtime speed, memory footprint, tail latency, compile speed (`>500K LOC/s`), and source LOC clarity, certifying `imp_type.md` zero-overhead abstractions.

- [ ] **M98 — Zero-Friction `Edition 1.0 -> 2.0` Auto-Migration & Stability Charter**
  - Verify `pilot migrate --edition 2.0` upgrades every 1.0 package and test cleanly with zero manual edits; publish the Pilot 2.0 Language Reference & Sublanguage Architect's Guide upholding `imp_type.md` invariants.

- [ ] **M99 — 72-Hour Continuous Fuzz, Polyglot Stress & Multi-OS Bootstrap Burn-In**
  - Run continuous differential fuzzing across compiler passes, Perceus ARC, 2-bit lattice classifications, `.pchor` cues, and all foreign-language adapters on Linux, macOS, Windows, x86_64, ARM64, and RISC-V with zero P0/P1 defects and zero engine resource leaks.

- [ ] **M100 — PILOT 2.0 RELEASE — The Universal Adaptable Programming Platform**
  - Tag Pilot `v2.0.0`, ship single-file hermetic installers for all major platforms, certify MCP v2 AI co-design workflows, and open the 2.0 ecosystem—fully powered by the 6 Core Laws and 12 Invariant Rules of `imp_type.md`!
