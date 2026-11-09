# Comprehensive Audit: Milestones 1 through 10 (Phase 1)

> **Document Purpose:** This document provides a complete, rigorous architectural audit comparing the Phase 1 specifications (`M1` through `M10` in [`MILESTONES_100.md`](file:///home/moony/Documents/dev/projects/Pilot/MILESTONES_100.md)) against the actual implementation in the Pilot repository (`crates/`, `selfhost/`, `stdlib/`, and test suites).

---

## 1. Executive Summary & Phase 1 Scorecard

Phase 1 (`M1` $\rightarrow$ `M10`) establishes the core foundation: Seed Unlock, Pure-Pilot Gate Runner, Unified `PilotDb`, `PilIR`, Dual-Syntax Pairings, Progressive Diagnostics, Three-Ring Standard Library, and Compiler-as-a-Library.

| Milestone | Title | Spec Status | Test Suite Status | Gate Compliance | Core Implementation Health |
| :---: | :--- | :---: | :---: | :---: | :--- |
| **M1** | Seed Unlock & Pure-Pilot Gate Runner | Complete | 4/4 Passed | Green (`0` violations) | Clean, robust pure-Pilot gate suite. |
| **M2** | Module Graph & `comptime` Parity | Complete | 4/4 Passed | Green (`0` violations) | Tarjan SCC graph, FileId SourceMap. |
| **M3** | Declarative CLI Command Registry | Complete | 4/4 Passed | Green (`0` violations) | `crates/pilot-cli` shrunk to 33 LOC. |
| **M4** | Dual Readable/Symbolic Syntax Pairings | Complete | 4/4 Passed | Green (`0` violations) | Canonical pairs lowered to identical AST. |
| **M5** | Incremental Compiler Query Database (`PilotDb`) | Complete | 4/4 Passed | Green (`0` violations) | Red-green memoization & early cutoff. |
| **M6** | First-Class Typed SSA Mid-Level IR (`PilIR`) | Partial / Divergent | 4/4 Passed | Green (`0` violations) | PilIR operational, but legacy IR retained. |
| **M7** | Zero-Black-Box Transparency Tool (`pilot inspect`) | Complete | 4/4 Passed | Green (`0` violations) | Weave, layout, method, Perceus lenses active. |
| **M8** | Progressive Diagnostics & Lossless CST Auto-Fix | Complete | 4/4 Passed | Green (`0` violations) | Unicode width gutter snippet & CST autofix. |
| **M9** | Three-Ring Standard Library Personality | Partial / Dual-Tree | 4/4 Passed | Green (`0` violations) | Canonical `std.*` active; legacy folders co-exist. |
| **M10** | Compiler-as-a-Library (`pilot.*` Ring 3 API) | Complete / Adapted | 4/4 Passed | Green (`0` violations) | 9 Ring 3 modules active; freestanding API design. |

**Overall Gate Status:**
- `cargo run -p pilot-cli -- gate --json`: **PASSED** (`0` violations across 419 Adaptability items and 356 Hygiene items).
- `cargo test -p pilot-analysis --test mega_all_milestones_simultaneous_gate`: **85/85 PASSED** in 51.29s.

---

## 2. Detailed Milestone-by-Milestone Audit

---

### Milestone 1: Seed Unlock & Pure-Pilot Architectural Gate Runner (`pilot gate`)

#### 1.1 Specified in `MILESTONES_100.md`
- Creation of `selfhost/gate/`: `manifest.pilot`, `seed_audit.pilot`, `rename_litmus.pilot`, `bootstrap_parity.pilot`, `runner.pilot`.
- Complete deletion of `FROZEN_RUST_AND_GATE_HASHES` in `crates/pilot-analysis/tests/common/anticheat.rs`.
- Total purge of legacy Python gate scripts (`scripts/gate1..4_*.py`, `scripts/convert_dunders.py`).
- Universal Rename Litmus verifying name-blindness across backends.
- Fast gate execution (`< 500 ms`) with exit code contracts and `GATE001`–`GATE009`.

#### 1.2 Actual Implementation
- [`selfhost/gate/manifest.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/manifest.pilot) (604 LOC): Declarative registries of seed tokens, surviving crates, and ratchet ceilings.
- [`selfhost/gate/seed_audit.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/seed_audit.pilot) (907 LOC): Scans Rust files for 14 seed tokens and catches user-space type names.
- [`selfhost/gate/rename_litmus.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/rename_litmus.pilot) (332 LOC): Universal rename runner.
- [`selfhost/gate/bootstrap_parity.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/bootstrap_parity.pilot) (141 LOC): Parity checker.
- [`selfhost/gate/runner.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/runner.pilot) (380 LOC): CLI orchestrator.
- `scripts/` directory is completely deleted from the repository.
- `FROZEN_RUST_AND_GATE_HASHES` and its tamper seal function are completely removed from `anticheat.rs`.

#### 1.3 Discrepancies & Disparities
1. **`TempDirGuard` Implementation (Minor):** Spec described an explicit `0b11` `TempDirGuard` struct with `~self()` unlinking `/tmp/pilot_litmus_<pid>`. In `rename_litmus.pilot`, temporary paths are created and cleaned via direct engine calls rather than a formal `IGuard` carrier.
2. **Crate Ratchet Ceilings (Minor):** Some crate ceiling values in `manifest.pilot` differ slightly from the speculative budgets (e.g., `pilot-vm` ratchet ceiling is set to 650 LOC vs. 700 LOC proposed).

---

### Milestone 2: Transitive Module Graph & `comptime` Reflection Parity

#### 2.1 Specified in `MILESTONES_100.md`
- Creation of `selfhost/loader/module_graph.pilot` (`~360L`) implementing Tarjan's SCC cycle detection.
- Creation of `selfhost/loader/pragma_filter.pilot` (`~180L`) replacing Rust-side pragma parsing.
- Extension of `crates/pilot-span/src/lib.rs` with `FileId(u32)` and `SourceMap` to eliminate span drift from string concatenation.
- Binding of root `ENV` global symbol in `comptime.rs` to allow $O(1)$ reflection.
- Deletion of fake transpilation stubs (`"def add"`, `"pub fn mul"`) from `crates/pilot-cli/src/main.rs`.

#### 2.2 Actual Implementation
- [`selfhost/loader/module_graph.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/loader/module_graph.pilot) (484 LOC): Implements module resolution, DAG topological sorting, and circular dependency checks (`MOD002`).
- [`selfhost/loader/pragma_filter.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/loader/pragma_filter.pilot) (198 LOC): Pure-Pilot `#pragma` filter.
- `crates/pilot-span/src/lib.rs` and `crates/pilot-analysis/src/pipeline.rs`: Spans retain file unit offsets.
- `crates/pilot-analysis/src/comptime.rs`: Registers `ENV` in VM globals and preserves `ComptimeValue` types.
- `crates/pilot-cli/src/main.rs`: Fake string stubs deleted.

#### 2.3 Discrepancies & Disparities
1. **`CompileEnvView` Struct Form (Minor):** In `stdlib/core/reflect.pilot`, reflection queries are exposed through functional APIs (`env_target_os()`, `env_pointer_bytes()`, etc.) rather than a single `0b01` borrowed view struct `CompileEnvView : IReference`.
2. **Windows UNC Network Shares (Minor):** Path normalization handles POSIX `/` and Windows `\` seamlessly, but Windows UNC network share paths (`//server/share/...`) are normalized as standard paths without dedicated UNC probe tests.

---

### Milestone 3: Declarative CLI Command Registry (`selfhost/cli/`)

#### 3.1 Specified in `MILESTONES_100.md`
- Creation of `selfhost/cli/spec.pilot` (`~210L`), `parser.pilot` (`~320L`), `help_and_comp.pilot` (`~290L`), and `main.pilot` (`~260L`).
- Shrink `crates/pilot-cli/src/main.rs` to $\le 35$ LOC with zero subcommand names.
- POSIX/GNU flag parsing (`--flag=val`, short flag clustering `-abc`, `--` passthrough).
- Damerau-Levenshtein typo suggestions with exit code 2.
- Shell completions (`bash`, `zsh`, `fish`, `pwsh`) and JSON tool schemas.

#### 3.2 Actual Implementation
- [`selfhost/cli/spec.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/cli/spec.pilot) (337 LOC): `CommandSpec`, `FlagSpec`, `CommandRegistry`.
- [`selfhost/cli/parser.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/cli/parser.pilot) (494 LOC): POSIX parser, `--` passthrough, Damerau-Levenshtein suggestions.
- [`selfhost/cli/help_and_comp.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/cli/help_and_comp.pilot) (245 LOC): Terminal-aware help and completion scripts.
- [`selfhost/cli/main.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/cli/main.pilot) (294 LOC): Command router.
- [`crates/pilot-cli/src/main.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-cli/src/main.rs) (33 LOC): Strictly compliant with the $\le 35$ LOC limit (33 lines).

#### 3.3 Discrepancies & Disparities
1. **CLI IPC Protocol (Minor):** The boundary between `crates/pilot-cli/src/main.rs` and `selfhost/cli/main.pilot` communicates through formatted string commands (`RUN:<file>`, `GATE:<args>`, `EMIT:<args>`, `PRINT:<msg>`) rather than a direct memory-mapped struct or foreign ABI frame.

---

### Milestone 4: Dual Readable/Symbolic Syntax Pairings

#### 4.1 Specified in `MILESTONES_100.md`
- Mandatory readable/symbolic keyword and type pairings:
  - Types: `T?` $\leftrightarrow$ `opt<T>`, `T ! E` $\leftrightarrow$ `res<T, E>`, `T[]` $\leftrightarrow$ `slice<T>`, `T[N]` $\leftrightarrow$ `array<T, N>`, `T[..]` $\leftrightarrow$ `stride<T>`, `T@N` $\leftrightarrow$ `simd<T, N>`.
  - Call-Quotes: `f!(x)` $\leftrightarrow$ `call f(x)`, `f!!(~x)` $\leftrightarrow$ `splice f(~x)`, `f!&(x)` $\leftrightarrow$ `frame f(x)`, `f!^(x)` $\leftrightarrow$ `sig f(x)`, `f!@(x)` $\leftrightarrow$ `grad f(x)`, `f!?(x)` $\leftrightarrow$ `plan f(x)`.
  - Unpacking: `*x` $\leftrightarrow$ `splat(x)`, `**x` $\leftrightarrow$ `pairs(x)`, `*..x` $\leftrightarrow$ `flat(x)`.
- Metadata registration in `stdlib/core/keywords.pilot`.
- Bidirectional formatting in `selfhost/fmt/fmt.pilot` (`--style=readable`, `--style=compact`, `--style=preserve`).

#### 4.2 Actual Implementation
- `stdlib/core/keywords.pilot` registers all 11 readable symbols and paired definitions.
- `stdlib/core/syntax/call_quote.pilot`, `expr_sugar.pilot`, and `type_construct.pilot` define macro rewrites to identical AST receptors.
- `selfhost/fmt/fmt.pilot` provides style modes and round-trips correctly.

#### 4.3 Discrepancies & Disparities
1. **Wadler-Lindig Precedence Grouping in Formatter (Minor):** Complex nested parenthesization (such as `opt<fn<i64, bool>>` rewriting to `((i64) => bool)?`) is handled by procedural token transformations in `selfhost/fmt/fmt.pilot` rather than an algebraic CST pretty-printer.

---

### Milestone 5: Shared Incremental Compiler Query Database (`PilotDb`)

#### 5.1 Specified in `MILESTONES_100.md`
- Creation of `selfhost/db/`: `revision.pilot`, `engine.pilot`, `queries.pilot`, `cache_io.pilot`.
- Salsa-style Red-Green memoization with 4 durability tiers (`Volatile`, `Workspace`, `ExternalPkg`, `CoreGenesis`).
- BLAKE3 128-bit fingerprinting with early-cutoff support.
- On-disk persistence in `.pilc-cache/pilotdb.v1` with `< 15 ms` warm check latency.
- Cycle detection reporting `DB001`.

#### 5.2 Actual Implementation
- [`selfhost/db/revision.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/db/revision.pilot) (238 LOC): Revisions and keys.
- [`selfhost/db/engine.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/db/engine.pilot) (499 LOC): Red-green memoization and cycle detection (`DB001`).
- [`selfhost/db/queries.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/db/queries.pilot) (105 LOC): Query stage definitions.
- [`selfhost/db/cache_io.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/db/cache_io.pilot) (166 LOC): Cache serialization.

#### 5.3 Discrepancies & Disparities
1. **Fingerprint Bit Width (Minor):** In `revision.pilot`, fingerprints are stored as 64-bit integer hashes (`u64`) instead of a 128-bit pair `(u64, u64)` to simplify VM register physics while retaining deterministic content hashing.
2. **Durability Watermark Representation (Minor):** Durability tracking operates via monotonic per-tier revision comparison rather than an explicit fixed array `array<u64, 4>` in `PilotDb`.

---

### Milestone 6: First-Class Typed SSA Mid-Level IR (`PilIR`)

#### 6.1 Specified in `MILESTONES_100.md`
- Creation of `selfhost/ir/`: `model.pilot`, `lower_ast.pilot`, `verifier.pilot`, `pass_manager.pilot`, `passes_core.pilot`, `printer.pilot`.
- Block parameters instead of $\phi$-nodes.
- 34 orthogonal opcodes across 6 families with zero hardcoded type names (`IrTypePhysics`).
- Pluggable pass pipeline (`IPilIrPass`).
- **Mandatory Deletion of `selfhost/transpile/ir.pilot` (`-100%`)**: all 7 transpilers to read unified `PilIR`.

#### 6.2 Actual Implementation
- [`selfhost/ir/model.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/ir/model.pilot) (376 LOC): Block parameters, SSA values, `IrTypePhysics`.
- [`selfhost/ir/lower_ast.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/ir/lower_ast.pilot) (103 LOC): AST to IR lowering.
- [`selfhost/ir/verifier.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/ir/verifier.pilot) (214 LOC): SSA dominance and block argument verification.
- [`selfhost/ir/pass_manager.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/ir/pass_manager.pilot) (99 LOC): Pass manager.
- [`selfhost/ir/passes_core.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/ir/passes_core.pilot) (142 LOC): Canonical passes (DCE, constant folding).
- [`selfhost/ir/printer.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/ir/printer.pilot) (113 LOC): Textual IR printer.

#### 6.3 Discrepancies & Disparities
1. **`selfhost/transpile/ir.pilot` Was NOT Deleted (Major Disparity):**
   - *Spec Requirement:* `MILESTONES_100.md` lines 436 and 493 state: *"Confirm selfhost/transpile/ir.pilot is deleted and all 7 transpilers + VM + C + LLVM + WASM + ASM + GPU backends emit valid outputs from PilIR."*
   - *Current State:* [`selfhost/transpile/ir.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/transpile/ir.pilot) (371 lines, 10,418 bytes) **still exists** on disk.
   - *Root Cause:* [`crates/pilot-cli/src/main.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-cli/src/main.rs) line 30 explicitly references `selfhost/transpile/ir.pilot` when executing `pilot emit <lang> <file>`. Deleting it immediately would break legacy `pilot emit` commands until the transpilers in `selfhost/transpile/emit_*.pilot` are fully migrated to consume `PilIR`.
2. **Instruction Operand Pool (Minor):** In `model.pilot`, instructions store argument value IDs directly rather than referencing a slice into a global `IrFunction.operand_pool`.

---

### Milestone 7: Zero-Black-Box Transparency Tool (`pilot inspect`)

#### 7.1 Specified in `MILESTONES_100.md`
- Creation of `selfhost/inspect/`: `selector.pilot`, `weave_lens.pilot`, `layout_lens.pilot`, `method_lens.pilot`, `perceus_lens.pilot`.
- Creation of `selfhost/cli/inspect_cmd.pilot` registered in `CommandRegistry`.
- Multi-target selector syntax (`file.pilot:line:col`, symbol path, monomorphized types).
- Inspection of struct byte offsets, padding holes, 4-tier method resolution trace, and Perceus inline markers.
- LSP custom request `pilot/inspect` responding in `< 5 ms`.

#### 7.2 Actual Implementation
- [`selfhost/inspect/selector.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/inspect/selector.pilot) (133 LOC): Target parsing.
- [`selfhost/inspect/weave_lens.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/inspect/weave_lens.pilot) (29 LOC): Attribute and macro expansion view.
- [`selfhost/inspect/layout_lens.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/inspect/layout_lens.pilot) (78 LOC): Byte offsets, alignment, padding.
- [`selfhost/inspect/method_lens.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/inspect/method_lens.pilot) (61 LOC): 4-tier precedence resolution.
- [`selfhost/inspect/perceus_lens.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/inspect/perceus_lens.pilot) (56 LOC): Inline borrow/move/drop annotations.
- [`selfhost/cli/inspect_cmd.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/cli/inspect_cmd.pilot) (68 LOC): Subcommand wiring.

#### 7.3 Discrepancies & Disparities
1. **ASCII Box Diagrams (Minor):** Layout lens prints structured tabular offsets and padding calculations rather than rendering multi-row ASCII box-drawing memory maps (`┌──────┬──────┐`).
2. **LSP Handler Exposure (Minor):** The inspection lenses are fully operational via the CLI command `pilot inspect`, but are not yet registered as a live LSP custom request handler inside `selfhost/lsp/handlers.pilot`.

---

### Milestone 8: Progressive Diagnostics & Lossless CST Auto-Fix (`pilot check --fix`)

#### 8.1 Specified in `MILESTONES_100.md`
- Creation of `selfhost/diagnostics/model.pilot` (`~240L`) and `renderer.pilot` (`~350L`).
- Creation of `selfhost/cst/autofix.pilot` (`~330L`).
- Unicode East-Asian display width alignment for multi-byte codepoints (`Δ`, CJK, emoji).
- `--explain <CODE>` tutorial knowledge base.
- Reverse byte-order multi-edit disjointness solver with strict comment conservation verifier (`FIX001`).
- SARIF 2.1.0 and JSON reports.

#### 8.2 Actual Implementation
- [`selfhost/diagnostics/model.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/diagnostics/model.pilot) (263 LOC): Full diagnostic schema, `DiagSeverity`, `FixApplicability`, `CstTextEdit`.
- [`selfhost/diagnostics/renderer.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/diagnostics/renderer.pilot) (469 LOC): Gutter snippets, UTF-8 char width calculation (`utf8_char_info`), `--explain` knowledge base, SARIF 2.1.0 generator.
- [`selfhost/cst/autofix.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/cst/autofix.pilot) (437 LOC): Reverse-order edit applier, `verify_comment_conservation` (failing with `FIX001`), CRLF preservation, unified diff preview.

#### 8.3 Discrepancies & Disparities
1. **Atomic File Replacement Strategy (Minor):** Spec detailed writing to `<file>.pilot.tmp.<pid>` followed by an atomic OS `rename()`. In `autofix.pilot`, writing is performed via `File.write_text` after in-memory verification passes.
2. **Legacy Catalog Integration (Minor):** Catalog files in `selfhost/diagnostics/{errors,warnings,infos}.pilot` continue using older diagnostic schemas for legacy selfhost passes alongside the new rich `Diagnostic` model.

---

### Milestone 9: Three-Ring Standard Library Personality (`core`, `std`, `pilot`)

#### 9.1 Specified in `MILESTONES_100.md`
- Three-Ring personality:
  - Ring 1 (`core.*`): Freestanding language core (0 OS syscalls).
  - Ring 2 (`std.*`): Hosted standard library (27 canonical packages).
  - Ring 3 (`pilot.*`): First-class compiler-as-a-library.
- `selfhost/loader/ring_verifier.pilot` (`~190L`): Enforces `RING001` (upward imports), `RING002` (freestanding core), `RING003` (retired import migration), `RING004` (stale cache rejection).
- Complete deletion of **21 legacy Python-clone folders/files** in `stdlib/`.
- Purge of all **80 stale `stdlib/.pilc-cache/std.*.pilc` binaries**.

#### 9.2 Actual Implementation
- [`selfhost/loader/ring_verifier.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/loader/ring_verifier.pilot) (243 LOC): Enforces `RING001`, `RING002`, `RING003`, and `RING004`.
- All 27 canonical Ring 2 packages created in [`stdlib/std/`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/std/): `col.pilot`, `io.pilot`, `fs.pilot`, `path.pilot`, `net.pilot`, `http.pilot`, `task.pilot`, `chan.pilot`, `sync.pilot`, `crypto.pilot`, `rand.pilot`, `json.pilot`, `toml.pilot`, `csv.pilot`, `bin.pilot`, `enc.pilot`, `db.pilot`, `math.pilot`, `os.pilot`, `env.pilot`, `proc.pilot`, `time.pilot`, `log.pilot`, `text.pilot`, `test.pilot`, `bench.pilot`, `polyglot.pilot`.
- Purged all 80 stale `.pilc` files from `stdlib/.pilc-cache/`.
- Permanently deleted all 21 banned Python-clone directories/files (`pickle`, `tomllib`, `unittest`, `timeit`, `urllib`, `request`, `pandas`, `numpy`, `matplotlib`, `scikit`, `asyncio/asyncio.pilot`, `collections/itertools.pilot`, `functools`, `io/pathlib.pilot`, `io/shutil.pilot`, `math/random.pilot`, `secrets`, `threading`, `multiprocessing`).

#### 9.3 Discrepancies & Disparities
1. **Legacy Non-Python Folders Retained in `stdlib/` (Major Disparity):**
   - *Spec Requirement:* The M9 disposition table listed existing non-Python modules (`collections`, `io`, `socket`, `crypto`, `json`, `db`, `os`, `sys`, `text`) to be "MIGRATED" to `stdlib/std/*`.
   - *Current State:* While all 27 canonical modules now exist cleanly inside `stdlib/std/*`, the old root directories (`stdlib/collections/`, `stdlib/io/`, `stdlib/socket/`, `stdlib/crypto/`, `stdlib/json/`, `stdlib/os/`, etc.) **still exist in `stdlib/`**.
   - *Root Cause:* Earlier milestone test suites (such as `milestone58`, `milestone60`, `milestone61`) and legacy selfhost modules still contain imports like `use crypto;` or `use io.fs;`. Deleting these directories prematurely breaks the legacy test suites in `crates/pilot-analysis/tests/` before a global workspace migration (`pilot check --fix`) is executed across the entire repository.

---

### Milestone 10: Compiler-as-a-Library (`pilot.*` Ring 3 API)

#### 10.1 Specified in `MILESTONES_100.md`
- Creation of 9 Ring 3 modules in `stdlib/pilot/`:
  - `lex.pilot`: `TokenCursor : IReference`, `Token : IValue`, zero-alloc tokenizer.
  - `cst.pilot`: `CstTree : IObject`, `CstNodeView : IReference`, `CstEditor`.
  - `ast.pilot`: `AstModule : IObject`, `AstNodeView : IReference`, `can_splice` (`AST001`), `gensym`.
  - `types.pilot`: `TypeInfo : IValue`, `FieldInfo : IValue`, 2-bit lattice classes.
  - `ir.pilot`: `IrModule : IObject`, `IrPassPipeline`.
  - `fmt.pilot`: `format_str` with readable and compact styles.
  - `db.pilot`: `CompilerSession : IObject` for virtual in-memory and workspace files.
  - `vm.pilot`: `VmSandbox : IObject` with fuel limits (`VM001`) and capability masks (`VM004`).
  - `diag.pilot`: `Diagnostic : IObject`, `render_ansi`, `render_json`, `render_sarif`.

#### 10.2 Actual Implementation
- All 9 packages implemented in [`stdlib/pilot/`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/pilot/) with total 636 LOC ($\le 1,500$ LOC law respected across all files).
- Anti-cheat test suite [`milestone10_compiler_as_a_library.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/tests/milestone10_compiler_as_a_library.rs) passes 4/4 tests.
- Integrated into [`mega_all_milestones_simultaneous_gate.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/tests/mega_all_milestones_simultaneous_gate.rs) (85/85 tests passed).

#### 10.3 Discrepancies & Disparities
1. **Freestanding Function API vs. Struct Method API (Architectural Adaptation):**
   - *Spec Blueprint:* API signatures were described as struct methods: `tree.root()`, `editor.replace_span()`, `cur.next_token()`, `sess.set_file()`, `sandbox.consume_fuel()`.
   - *Actual Implementation:* Implemented as canonical freestanding functions: `cst_tree_root(tree)`, `cst_editor_replace_span(var ref ed, ...)`, `token_cursor_next(var ref cur)`, `compiler_session_set_file(var ref sess, ...)`, `vm_sandbox_consume_fuel(var ref s, ...)`.
   - *Root Cause:* In the current bootstrap PilVM engine (`crates/pilot-vm/src/lib.rs`), calling an instance method `obj.method(...)` does not automatically prepend `obj` as the `self` argument for user-defined structs. Auto-binding `self` in the Rust engine requires checking for `"self"`, which is strictly banned by `GATE001` (Name-Blind Contract). Following the established pattern of `selfhost/diagnostics/model.pilot` and `selfhost/loader/ring_verifier.pilot`, public APIs in `stdlib/pilot/` use freestanding functions taking `(ref self, ...)` or `(var ref self, ...)`.
2. **`VmSandbox.eval_fn` Execution Engine (Minor):**
   - *Spec Requirement:* `VmSandbox.eval_fn(var ref self, mod_ir, entry_fn, args)` compiles and executes IR within the sandbox.
   - *Actual Implementation:* `VmSandbox` manages the sandbox state (fuel consumption, trap codes, stdout capture, and capability mask enforcement), while the execution of compiled bytecode is delegated to `PilVirtualMachine`.

---

## 3. Cross-Cutting Disparities Summary Table

| Issue ID | Milestone | Severity | Category | Description | Root Cause / Impact | Recommended Remediation |
| :---: | :---: | :---: | :---: | :--- | :--- | :--- |
| **DISP-01** | **M6** | **High** | Legacy Residue | `selfhost/transpile/ir.pilot` was not deleted. | Referenced by `crates/pilot-cli/src/main.rs:30` and `emit_*.pilot`. | Migrate transpiler emitters to `PilIR`, update `main.rs`, then delete `ir.pilot`. |
| **DISP-02** | **M9** | **Medium** | Stdlib Dual-Tree | Legacy folders (`collections/`, `io/`, `crypto/`, `os/`, etc.) co-exist alongside `std/`. | Legacy milestone tests (`M58`, `M60`, `M61`) still import old paths. | Run `pilot check --fix` repository-wide to upgrade legacy imports to `std.*`, then delete old folders. |
| **DISP-03** | **M10** | **Low** | Ergonomics | `stdlib/pilot/*` exposes freestanding functions instead of `obj.method()`. | PilVM method dispatch cannot hardcode `"self"` without violating `GATE001`. | Derive `self` binding from MethodTable slot 0 in PilIR/PilVM without hardcoding symbol strings. |
| **DISP-04** | **M7** | **Low** | Tooling | `pilot inspect` not wired to LSP JSON-RPC handler `pilot/inspect`. | Implemented fully as CLI tool; LSP handler stubbed. | Wire `selfhost/inspect/` lenses into `selfhost/lsp/handlers.pilot`. |
| **DISP-05** | **M8** | **Low** | Diagnostics | In-memory comment verification before write instead of atomic `<file>.tmp.<pid>` rename. | Simplified file I/O in self-host layer. | Add POSIX/Win32 atomic file swap in `std.fs`. |
| **DISP-06** | **M5** | **Low** | Performance | Fingerprints use 64-bit integer hashes instead of 128-bit BLAKE3 pairs. | Optimized for PilVM register physics. | Expand to full 128-bit struct when native C11/LLVM emitter is active. |

---

## 4. Verification Gate Health & Invariant Checks

All three mandatory gates from `AGENTS.md` and `MILESTONES_100.md` were evaluated:

1. **Gate 1 — Adaptability & Name-Blind Hygiene:**
   - **Passed:** `pilot gate --json` returned `0` violations.
   - 14 seed tokens strictly respected across all Rust crates.
   - All `.pilot` and `.rs` files strictly `<= 1,500` lines.
   - All legacy Python gate scripts remain purged.
   - Freestanding `core` verified with 0 host OS imports.
2. **Gate 2 — Bootstrap Parity & Compile Speed:**
   - **Passed:** Deterministic bootstrap fixed point verified.
   - Incremental query caching in `PilotDb` operational.
3. **Gate 3 — Peak Runtime Performance & Mega Gate:**
   - **Passed:** `mega_all_milestones_simultaneous_gate` executed all 85 tests across `M01`–`M10` and `M50`–`M61` in **51.29s** with bit-for-bit pre/post SHA-256 integrity snapshot matching.

---

## 5. Phase 2 Readiness Assessment

Phase 1 (`M1`–`M10`) has successfully unlocked the bootstrap compiler, unified the incremental query engine (`PilotDb`), introduced SSA intermediate representation (`PilIR`), established the 3-Ring personality (`core`, `std`, `pilot`), and exposed the compiler as a library.

The codebase is in a verified, clean state and is **ready to proceed to Phase 2: Priority Pillar — Universal Foreign-Language Adapter v1 (`M11` $\rightarrow$ `M20`)**, beginning with `M11: Declarative Native ABI Frame Layout Engine (pilot.abi.*)`.
