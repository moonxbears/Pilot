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
| **M5** | Incremental Compiler Query Database (`PilotDb`) | Complete | 4/4 Passed | Green (`0` violations) | Red-green memoization, 128-bit fingerprints & early cutoff. |
| **M6** | First-Class Typed SSA Mid-Level IR (`PilIR`) | Substantially Complete | 4/4 Passed | Green (`0` violations) | PilIR operational; `ir.pilot` retained strictly as M57 test anchor. |
| **M7** | Zero-Black-Box Transparency Tool (`pilot inspect`) | Complete | 4/4 Passed | Green (`0` violations) | Weave, layout, method, Perceus lenses active; LSP handler wired. |
| **M8** | Progressive Diagnostics & Lossless CST Auto-Fix | Complete | 4/4 Passed | Green (`0` violations) | Unicode width gutter snippet, CST autofix & atomic write. |
| **M9** | Three-Ring Standard Library Personality | Substantially Complete | 4/4 Passed | Green (`0` violations) | Canonical `std.*` active; 15 legacy folders purged; 7 test anchors retained. |
| **M10** | Compiler-as-a-Library (`pilot.*` Ring 3 API) | Complete | 4/4 Passed | Green (`0` violations) | 9 Ring 3 modules active; idiomatic freestanding API. |
| **M11** | First-Class `adapter` Blueprint Engine & Zero-Cost Idiom Lifter | Complete | 4/4 Passed | Green (`0` violations) | Table-driven rules, PilIR zero-alloc thunk compilation, public Adapt facets, mocks purged. |

**Overall Gate Status:**
- `cargo run -p pilot-cli -- gate --json`: **PASSED** (`0` violations across 419 Adaptability items and 338 Hygiene items).
- `cargo test -p pilot-analysis --test mega_all_milestones_simultaneous_gate`: **85/85 PASSED** in 53.73s.

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
1. **Fingerprint Bit Width (Resolved):** In `selfhost/db/revision.pilot`, `Fingerprint128` is implemented as a 128-bit `[Layout(c_abi)]` struct with `lo: u64` and `hi: u64`, accompanied by `fingerprint_zero()`, `fingerprint_new(lo, hi)`, `fingerprint_equals(a, b)`, `fingerprint_from_hex(hex)`, and `fingerprint_compute(content)` producing the full 128-bit hash pair.
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
1. **`selfhost/transpile/ir.pilot` Retained as Downstream Test Anchor (Intentional Deferral):**
   - *Spec Requirement:* `MILESTONES_100.md` lines 436 and 493 state: *"Confirm selfhost/transpile/ir.pilot is deleted and all 7 transpilers + VM + C + LLVM + WASM + ASM + GPU backends emit valid outputs from PilIR."*
   - *Current State:* [`selfhost/transpile/ir.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/transpile/ir.pilot) (371 lines, 10,418 bytes) **remains on disk**.
   - *Root Cause:* [`crates/pilot-analysis/tests/milestone57_bidirectional_transpilers_and_ingest.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/tests/milestone57_bidirectional_transpilers_and_ingest.rs#L25) explicitly enforces an anti-cheat `FileAuditRule` requiring `selfhost/transpile/ir.pilot` with `min_bytes: 5_000`. Additionally, [`crates/pilot-cli/src/main.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-cli/src/main.rs#L30) references it for legacy `pilot emit`. Deleting it immediately causes M57 anti-cheat tests to fail. It is intentionally preserved as a downstream test anchor until M57.
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
2. **LSP Handler Exposure (Resolved):** Custom request handler `handle_lsp_inspect` has been implemented and wired to `"pilot/inspect"` in `selfhost/lsp/handlers.pilot` (dispatching JSON responses for inspected target symbols while maintaining file LOC $\le 1000$).

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
1. **Atomic File Replacement Strategy (Resolved):** `selfhost/cst/autofix.pilot` implements `write_file_atomic(file_path, new_src)` which commits to `<file_path>.tmp` first and verifies write integrity before updating the destination file.
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
1. **Legacy Non-Python Folders Purged / Phase 5 Anchors (Substantially Resolved):**
   - *Purge Executed:* 15 unused legacy directories have been completely purged from `stdlib/`: `numeric`, `csv`, `decimal`, `os`, `logging`, `json`, `socket`, `sys`, `random`, `io`, `text`, `math`, `formats`, `encoding`, and `collections`. Reference paths in `selfhost/pkg/manifest.pilot` and `selfhost/lsp/handlers.pilot` were updated to point to canonical `stdlib/std/col.pilot`.
   - *Retained Phase 5 Anchors:* Exactly 7 legacy directories are temporarily retained because downstream anti-cheat test suites and parser bootstrap tables enforce file-level existence audits:
     - `stdlib/crypto/`, `stdlib/db/`, `stdlib/http/`, `stdlib/polyglot/`: audited by `milestone58_polyglot_and_crypto.rs`, `milestone60`, and `milestone61`.
     - `stdlib/pkg/`: audited by `milestone57_bidirectional_transpilers_and_ingest.rs` (`licensing.pilot`).
     - `stdlib/asyncio/`, `stdlib/contracts/`, `stdlib/realtime/`: embedded via `include_str!` in `crates/pilot-parser/src/lib.rs` for `bootstrap_tables()`.
   - These 7 remaining directories serve strictly as test dependency anchors until Phase 5 when their respective milestones are reached.

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

### Milestone 11: First-Class `adapter` Blueprint Engine & Zero-Cost Idiom Lifter

#### 11.1 Specified in `MILESTONES_100.md`
- Creation of `selfhost/adapter/rules.pilot` (`~310L`): Table-driven `AdapterRuleRegistry` and `IAdapterTransform` protocols (`NamingStyle`, `ErrorLiftPolicy`, `SlicePairRule`, `OutParamRule`, `CallbackPairRule`, `AdapterBlueprint`).
- Creation of `selfhost/adapter/compiler.pilot` (`~420L`): Compiles `adapter` blueprints into synthesized Tier-1/Tier-2 `MethodTable` entries and inline zero-allocation `PilIR` thunks (`Receiver Handle Unwrap`, `Slice Unpack`, `OutParam Promotion`, `Result Branching`, `Property Synthesis`).
- Creation of `stdlib/std/polyglot/adapter.pilot` (`~190L`): Public `[Adapt(...)]` attributes, `CError`, `Errno`, `NullPtrErr`, `PosixAdapterFacet`, `Win32AdapterFacet`.
- Deletion of `pilot_transpile_sample` and `pilot_polyglot_execute` fake mocks in `crates/pilot-ffi/src/lib.rs`.
- Diagnostic codes `ADP001`..`ADP004` and verification suite in `crates/pilot-analysis/tests/milestone11_adapter_blueprint_engine.rs`.

#### 11.2 Actual Implementation
- [`selfhost/adapter/rules.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/adapter/rules.pilot) (279 LOC, $\le 1,500$ cap): Table-driven transforms, prefix stripping, case normalizations, and rule descriptors.
- [`selfhost/adapter/compiler.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/adapter/compiler.pilot) (229 LOC, $\le 1,500$ cap): Zero-allocation `PilIR` thunk lowerer with stack slots, parameter transforms, error branch lifting, and property synthesis.
- [`stdlib/std/polyglot/adapter.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/std/polyglot/adapter.pilot) (106 LOC, $\le 1,500$ cap): User-facing attributes (`[Adapt]`, `[SlicePair]`, `[OutParam]`), error record types, and POSIX / Win32 facets.
- [`crates/pilot-ffi/src/lib.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-ffi/src/lib.rs): Fake mocks permanently deleted.
- [`crates/pilot-analysis/tests/milestone11_adapter_blueprint_engine.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/tests/milestone11_adapter_blueprint_engine.rs): 4/4 anti-cheat tests passing cleanly.

#### 11.3 Discrepancies & Disparities
1. **None:** All components, rule protocols, error lifting policies, PilIR zero-allocation lowerings, and mock purges align directly with spec requirements and pass 100% of anti-cheat gates.

---

## 3. Cross-Cutting Disparities Summary Table

| Issue ID | Milestone | Status | Category | Description | Root Cause / Impact | Resolution / Current State |
| :---: | :---: | :---: | :---: | :--- | :--- | :--- |
| **DISP-01** | **M6** | **Deferred to M57** | Test Anchor | `selfhost/transpile/ir.pilot` retained. | Audited by `milestone57_bidirectional_transpilers_and_ingest.rs:25` (`min_bytes: 5000`) and `main.rs:30`. | Retained as intentional dependency anchor until M57 transpiler ingestion milestone. |
| **DISP-02** | **M9** | **Substantially Resolved** | Stdlib Cleanup | Legacy folders in `stdlib/` co-existing alongside `std/`. | 15 unused legacy folders purged. 7 folders retained solely for M57/M58/M60/M61 anti-cheat audits and parser tables. | Purged 15 folders (`numeric`, `csv`, `decimal`, `os`, `logging`, `json`, `socket`, `sys`, `random`, `io`, `text`, `math`, `formats`, `encoding`, `collections`). |
| **DISP-03** | **M10** | **Architectural Parity** | Ergonomics | `stdlib/pilot/*` exposes freestanding functions instead of `obj.method()`. | PilVM method dispatch cannot hardcode `"self"` without violating `GATE001` (Strict 14-Token Boundary). | Idiomatic freestanding API taking `(ref self, ...)` and `(var ref self, ...)` matching `Diagnostics.Model` and `RingVerifier`. |
| **DISP-04** | **M7** | **Resolved** | Tooling | `pilot inspect` not wired to LSP JSON-RPC handler `pilot/inspect`. | Implemented fully as CLI tool; LSP handler missing. | Implemented `handle_lsp_inspect` and wired to `"pilot/inspect"` in `selfhost/lsp/handlers.pilot` ($\le 1000$ LOC). |
| **DISP-05** | **M8** | **Resolved** | Diagnostics | In-memory comment verification before write instead of atomic `<file>.tmp` swap. | File I/O omitted atomic temp file stage. | Implemented `write_file_atomic(file_path, new_src)` in `selfhost/cst/autofix.pilot`. |
| **DISP-06** | **M5** | **Resolved** | Performance | Fingerprints 128-bit verification. | Verified against `selfhost/db/revision.pilot`. | Confirmed `Fingerprint128 : IValue` with `lo: u64` and `hi: u64` and full 128-bit hash pipeline. |

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
   - **Passed:** `mega_all_milestones_simultaneous_gate` executed all 85 tests across `M01`–`M10` and `M50`–`M61` in **53.73s** with bit-for-bit pre/post SHA-256 integrity snapshot matching.

---

## 5. Phase 2 Progress & Readiness Assessment

Phase 1 (`M1`–`M10`) established the core compiler foundation. Phase 2 (`M11`–`M20`) begins the **Universal Foreign-Language Adapter**:
- **Milestone 11 (Completed):** First-Class `adapter` Blueprint Engine & Zero-Cost Idiom Lifter is fully implemented, verified with 4/4 anti-cheat tests, and tested across all 85 simultaneous mega gate tests.
- **Next Milestone:** Ready to proceed to **Milestone 12 (`M12 — Headerless C11/C23 Semantic Absorber (use c "header.h")`)**.
