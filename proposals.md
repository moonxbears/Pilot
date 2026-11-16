# `proposals.md` — Mandatory Owner-Approval Ledger for All Rust Changes

> **Binding Governance Rules ([`AGENTS.md`](file:///home/moony/Documents/dev/projects/Pilot/AGENTS.md) §1):**
> 1. **Zero Unapproved Rust Changes:** **No** Rust file (`crates/**`, `Cargo.toml`) may ever be created, modified, or deleted until a proposal below is explicitly marked **`[STATUS: APPROVED BY OWNER]`**.
> 2. **Strict File-Scope Confinement:** When executing an approved proposal, edits, creations, and deletions are **strictly restricted to ONLY the exact Rust file paths listed in that proposal**. If an additional Rust file needs to be touched, stop immediately and amend the proposal for owner approval first.
> 3. **Immediate Dead / Legacy Rust Registration:** Whenever any agent or developer discovers dead, unreferenced, or superseded Rust code during any task, they MUST halt before moving on and log its exact file paths here for immediate owner-approved deletion.
> 4. **Airlock Freeze (Model B+, owner decision 2026-11-14; airlock contents per `PROP-002a` Amendment A1):** Once `PROP-002a` lands, [`stdlib/core/bridge_lang.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/bridge_lang.pilot), [`crates/pilot-vm/src/bridge.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm/src/bridge.rs), and the ABI snapshot with its owner registry (`selfhost/gate/airlock_abi.pilot`) are frozen under B+. Every existing `HostOp` ID, sub-code, `PilotErr` code, `arch_kind` value, struct field offset, and hook signature is **frozen forever**: it is never deleted, renumbered or given a new meaning. The owner registry is checked, not frozen: each op always has exactly one owner, but an owner may move (to a `*Bundle`, out of `core.*` for `RING001`, or away entirely when `F6` implements the op in Pilot) through an owner-approved entry here that moves it in one change. Additions are **append-only**, each through its own owner-approved entry here. **One op, one owner:** each engine op appears exactly once, as the body of one `[EngineOp]` method or `comptime fn` of its owning type. Workarounds that route around the owner stay forbidden. `GATE021` enforces this.
> 5. **Mandatory Same-Change Map & Matrix Sync:** Every executed proposal MUST update [`rust.md`](file:///home/moony/Documents/dev/projects/Pilot/rust.md), [`pilot.md`](file:///home/moony/Documents/dev/projects/Pilot/pilot.md), [`docs/keyword_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/keyword_matrix.md), [`docs/concept_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/concept_matrix.md), and [`selfhost/gate/manifest.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/manifest.pilot) in the exact same commit.
> 6. **Break Loud & Seam Over Rust Retention:** Shrinking Rust takes absolute priority over keeping legacy Rust-backed `.pilot` paths temporarily green. When deleting misplaced Rust in an approved proposal breaks `.pilot` callers or unbuilt passes, **leave them loudly broken in Pilot via `seam(...)` (`SEAM001`, registered in `seams.pilot` / `known_bugs`)** and downgrade affected rows in `wiring.pilot`, `keyword_matrix.md`, and `concept_matrix.md`—never keep legacy Rust shims alive to avoid breaking Pilot callers. **One exception** (owner goal: run-time speed first): native bindings listed in `GATE021`'s shrink-only `TRANSITIONAL_NATIVE_BINDINGS` table (`PROP-002a` Amendment A1) stay until their owner method is at least as fast on the seed (the C2 threshold, after `PROP-007`). The table may only shrink.

---

## Proposal Index

| ID | Title | Net Rust `src` Delta | Status |
| :--- | :--- | ---: | :--- |
| **`PROP-001`** | Purge Discovered Dead Rust Crates/Modules, Remove Hardcoded String Leaks & Split Packed `pilot-parser/src/lib.rs` (`MOD001`) | `-399` net (`20,812` $\rightarrow$ `20,413` `src` LOC; `-1,439` unpacked) | **`COMPLETED (APPROVED BY OWNER)`** |
| **`PROP-002a`** | Define the 3-File Airlock Under the B+ Freeze (51 scalar/pointer `HostOp`s), Retire `pilot-rt`, Consolidate Stage-A Natives & Arm `GATE021` | `<= 0` net (`9` $\rightarrow$ `8` crates; `-683` `pilot-rt`) | **`APPROVED; A1 MERGED (d055851); C2 RESOLVED 2026-11-15 (BASELINE MEASURED; OWNER THRESHOLD 5% SLOWER MAX; CHECKED BY record.pilot --check)`** |
| **`PROP-002b`** | Bucket B/D `.pilot` Migration to Owner Methods (`~1,875` `map_*` + `~800` `ast_*` sites), Retire Transitional Native Bindings & `MapBuffer` Duplicate, Retire `heap.rs`/`host.rs` | To be itemized | `DRAFT — MAP SLICE ITEMIZED 2026-11-15 (PILOT ONLY, 0 RUST; RUN BEFORE PROP-007 STEP 4 UNDER OPTION M) — APPROVED 2026-11-15, BUILT; C2 PASSES AFTER `PROP-013` (2026-11-15); PilIR STILL USES NATIVE BINDINGS FOR `Map`; OTHER TYPES AND BUCKETS B/D STILL TO ITEMIZE` |
| **`PROP-007`** | Implement `[Inline]`: PilIR Pass (Pilot), Forced Mode for `[EngineOp]` Owners, Seed Lowering (Rust) | Seed part to be itemized; offset by native-binding dispatch removal | `DRAFT — PILIR PART (STEPS 1–3) BUILT; SEED PART (STEP 4): OWNER CHOSE OPTION M 2026-11-15 (MEASURE ON PROP-002b's MAP SLICE FIRST; BUILD OPTION F ONLY IF C2 FAILS)` |
| **`PROP-008`** | Every Operator Declared by an Attribute on Its Interface or `syntax` Rule (Named Levels, Closed Parse Shapes), and Comptime User-Defined Operators | Lexer/parser operator code removed; to be itemized | `STEP 1 + AMENDMENT 008-A APPROVED 2026-11-15 — IN PROGRESS; STEPS 2–3 NEED ITEMIZED LISTS` |
| **`PROP-009`** | Every Receptor Word and `__ssa` Defined by a `syntax` Rule with a `quote` Template (Sequence Rule Heads, `asm` Machine Ops, Core Forms) | Step 1a must be a net deletion (`query.rs`, `with_scope.rs`); step 1b about +60 | `STEPS 1A + 1B BUILT 2026-11-15; OPTION (C) BUILT — STOPPED: STEP 1A IS ABOUT +56 RUST, NOT YET A NET DELETION (§9); STEP 1C (SYNTAX CLASSES + RULEARM DELETION, §10) BUILT 2026-11-15 (−60 RUST; PROP-009 TOTAL STILL +66; `milestone51` RUST TEST EDIT AWAITS APPROVAL); STEP 2 `CoreForm` PHASES 2A + 2B APPROVED TOGETHER 2026-11-15 AND BUILT (§11; +186 RUST, OVER THE −15..−40 ESTIMATE, ACCEPTED BY THE OWNER 2026-11-15; `true`/`false`/`none` NOT MOVED); STEP 2C GROUP 1 (`errdefer`/`become`, §12, about −30 RUST) APPROVED 2026-11-15 AND BUILT; STEP 2C GROUP 2 (UNBUILT KEYWORDS, §13, about −90 RUST) APPROVED 2026-11-15 AND BUILT (−63 RUST); STEP 2C GROUP 3 (STRAY `else`, DEAD `select` ARM, §14, about −9 RUST) APPROVED 2026-11-15 AND BUILT WITH AMENDMENT `RejectWord` (−5 RUST); §15 (A FUNCTION'S `where` CLAUSE IS DROPPED SILENTLY) OPTION B APPROVED 2026-11-15 AND BUILT (±0 RUST); STEP 3: OPTION (b) CHOSEN BY OWNER 2026-11-15 (FILE LIST TO ITEMIZE); `milestone51` TEST EDIT APPROVED 2026-11-15 AND MADE; `emit_ssa_*` HOOKS REMOVED (§16); §17 (INLINE FOREIGN BLOCKS) OPTION R APPROVED 2026-11-15 AND BUILT (−8 RUST, NOT −18; `Rest` FIX, TEST DELETION AND `milestone51` LINE APPROVED AS EXTRAS)` |
| **`PROP-010`** | Purge Dead Rust Found 2026-11-15: the Unused Rust `comptime` Evaluator and Unreferenced Items | `-191` | `APPROVED BY OWNER 2026-11-15 AND BUILT` |
| **`PROP-011`** | Delete the Unrun Bootstrap Stage List `stdlib/core/bootstrap.pilot` (Rust Test Edit Only) | `0` (`src` unchanged) | `APPROVED BY OWNER 2026-11-15 AND BUILT` |
| **`PROP-012`** | Delete Test-Only Rust: the AST S-Expression Printer, the Rust Module Header Scanner and `TokenKind::is_keyword` | `-186` | `APPROVED BY OWNER 2026-11-15 AND BUILT` |
| **`PROP-013`** | Link a Run's Extra Toolchain Files in the Loader's One Pass (`run.rs` `compile_entry`) | `-11` | `APPROVED BY OWNER 2026-11-15 ("Approve PROP-013") AND BUILT; C2 PASSES` |
| **`PROP-014`** | Update the Benchmark Baseline Test for the Seed Column `PROP-002a` C2 Added (Rust Test Edit Only) | `0` (`src` unchanged) | `APPROVED BY OWNER 2026-11-15 ("Approve PROP-014") AND BUILT` |
| **`PROP-015`** | Compile an `=> expr` Function Body Against Its Return Type, Delete the Dead `ProbeArg::Engine`, and (C) Let a Block's Last `for` Collect on the Seed | about `+1` (`+3`, `-4`); C about `+15` | `DRAFT — AWAITING OWNER APPROVAL` |
| **`PROP-003`** | Move Domain Helpers (`heap.rs`/`ops.rs`/`checked.rs`), `parse_header` & Parser Semantic Desugarings to Pilot | `~ -2,850` LOC | `PLANNED (WAVE 3)` |
| **`PROP-004`** | Retire Legacy `crates/pilot-analysis` Crate (`821` `src` LOC) & Migrate Milestone Harnesses to `.pilot` | `-821` LOC | `PLANNED (WAVE 4)` |
| **`PROP-005`** | Replace Rust `BytecodeCompiler` (`24` `lower_*.rs` + `vm_gen.rs` + `codes.rs`) with `selfhost/vm/ir_to_pilc.pilot` | `-7,833` LOC | `PLANNED (WAVE 5)` |
| **`PROP-006`** | Precompiled `toolchain.pilc` Stage-3 Cutover — Delete `pilot-{lexer,parser,ast,span,intern}` | `~ -5,950` LOC | `PLANNED (WAVE 6 -> ~1,800 LOC FLOOR)` |

---

## `PROP-001` — Purge Discovered Dead Rust Crates/Modules, Remove Hardcoded Leaks & Split Packed `pilot-parser/src/lib.rs`

- **Status:** **`COMPLETED (APPROVED BY OWNER)`** — Executed end-to-end: deleted `7` dead Rust files (`crates/pilot-arena`, `crates/pilot-ffi`, `crates/pilot-rt/src/{memory,concurrency,net}.rs`), pruned dead helpers in `pilot-rt/src/{lib,c_abi,time}.rs` & `pilot-span/src/lib.rs`, unpacked and split `pilot-parser/src/lib.rs` (`1,474` packed lines $\rightarrow$ `lib.rs` `200`L + `tables.rs` `222`L + `decl.rs` `626`L + `stmt.rs` `307`L + `expr.rs` `366`L, fixing `damaged_source` `rhs.is_some()` guard), marked retired crates/files `must_be_absent` in `selfhost/gate/manifest.pilot`, and ratcheted `paused_max_total_rust_loc()` from `30,000` $\rightarrow$ `20,450` (`20,413` live `src` LOC across `9` crates / `92` files).
- **Motivation (Dead/Legacy Code Discovered During Audit):**
  1. `crates/pilot-arena` (`251`L) and `crates/pilot-ffi` (`134`L) have **zero** compiler/VM callers and duplicate [`stdlib/core/arena.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/arena.pilot) (`ArenaScope`) and [`stdlib/pkg/licensing.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/pkg/licensing.pilot).
  2. `crates/pilot-rt/src/{memory,concurrency,net}.rs` (`312`L), dead helpers in `crates/pilot-rt/src/{c_abi,time}.rs` (`~140`L), and `SourceMap`/`LineIndex` in `crates/pilot-span/src/lib.rs` (`~170`L, superseded by [`stdlib/core/source_lines.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/source_lines.pilot)) are dead legacy code.
  3. Hardcoded domain strings leak into the seed (`"sql"` in `crates/pilot-lexer/src/{token,lexer}.rs`, `"Pilot.Core.Format"` in `crates/pilot-parser/src/fstring.rs`, unused `PilotType` re-export in `crates/pilot-ast/src/lib.rs`).
  4. `crates/pilot-parser/src/lib.rs` (`1,474` packed lines = `~2,520` `rustfmt` lines) packs multiple statements onto single lines to evade `MOD001` (`<= 1,500` LOC law). Splitting it into focused `<= 750` LOC sub-modules restores clean `cargo fmt --check` compliance.

### Exact Rust Files to **DELETE** (`7` Files — Strictly Bounded)
1. [`crates/pilot-arena/Cargo.toml`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-arena/Cargo.toml)
2. [`crates/pilot-arena/src/lib.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-arena/src/lib.rs)
3. [`crates/pilot-ffi/Cargo.toml`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-ffi/Cargo.toml)
4. [`crates/pilot-ffi/src/lib.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-ffi/src/lib.rs)
5. [`crates/pilot-rt/src/memory.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt/src/memory.rs)
6. [`crates/pilot-rt/src/concurrency.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt/src/concurrency.rs)
7. [`crates/pilot-rt/src/net.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt/src/net.rs)

### Exact Rust Files to **CREATE** (`4` Submodules Split From `pilot-parser/src/lib.rs` — Zero New Logic)
1. `crates/pilot-parser/src/tables.rs` — `ParserTables`, `TypePostfixKind`, `TypeConstructRule`, `ModAct`, `PrattEntry`, `CVal`, and `ingest_node`.
2. `crates/pilot-parser/src/decl.rs` — `PendingAttr`, `DeclMods`, `ScopeHeader`, and top-level/member declaration parsing split verbatim from `lib.rs`.
3. `crates/pilot-parser/src/stmt.rs` — Block, `let`/`var`, `if`, `loop`, `return`, `defer`, and statement dispatch split verbatim from `lib.rs`.
4. `crates/pilot-parser/src/expr.rs` — Pratt expression climbing, postfix projections, call/index/type parsing split verbatim from `lib.rs`.

### Exact Rust Files to **MODIFY** (`23` Files — Strictly Bounded)
1. [`Cargo.toml`](file:///home/moony/Documents/dev/projects/Pilot/Cargo.toml) — Remove `crates/pilot-arena` and `crates/pilot-ffi` from `[workspace].members` and `[workspace.dependencies]`.
2. [`crates/pilot-analysis/Cargo.toml`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/Cargo.toml) — Remove unused `pilot-arena` dependency.
3. [`crates/pilot-ast/Cargo.toml`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-ast/Cargo.toml) — Remove unused `pilot-arena` dependency.
4. [`crates/pilot-parser/Cargo.toml`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-parser/Cargo.toml) — Remove unused `pilot-arena` dependency.
5. [`crates/pilot-rt/Cargo.toml`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt/Cargo.toml) — Remove unused `pilot-arena` dependency.
6. [`crates/pilot-vm/Cargo.toml`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm/Cargo.toml) — Remove unused `pilot-arena` dependency.
7. [`crates/pilot-rt/src/lib.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt/src/lib.rs) — Remove `pub mod memory; pub mod concurrency; pub mod net;` declarations.
8. [`crates/pilot-rt/src/c_abi.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt/src/c_abi.rs) — Remove dead `memory::` import and unused C header generator functions, keeping only active `DynamicLibrary` (`dlopen`/`dlsym`), `invoke_abi_frame`, and `fill_os_random`.
9. [`crates/pilot-rt/src/time.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-rt/src/time.rs) — Remove unreferenced wrapper functions, keeping only `now_nanos` until `PROP-002` folds it into `bridge.rs`.
10. [`crates/pilot-span/src/lib.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-span/src/lib.rs) — Delete dead `SourceMap`, `SourceFile`, `LineIndex`, and `LineCol` (`FileId` + `ByteSpan` remain `~65` LOC).
11. [`crates/pilot-ast/src/lib.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-ast/src/lib.rs) — Remove unused `PilotType` re-export / dead helpers if present.
12. [`crates/pilot-lexer/src/token.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-lexer/src/token.rs) — Remove hardcoded `SqlStringLit` token kind (`sql` binds from `stdlib/db/sql_syntax.pilot` / `stdlib/polyglot/polyglot_syntax.pilot`).
13. [`crates/pilot-lexer/src/lexer.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-lexer/src/lexer.rs) — Remove hardcoded `"sql"` identifier prefix check.
14. [`crates/pilot-lexer/src/lib.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-lexer/src/lib.rs) — Keep exports aligned with `token.rs` and `rustfmt`.
15. [`crates/pilot-parser/src/lib.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-parser/src/lib.rs) — Split into `tables.rs`, `decl.rs`, `stmt.rs`, `expr.rs` and format with `cargo fmt` (`<= 500` LOC).
16. [`crates/pilot-parser/src/fstring.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-parser/src/fstring.rs) — Read format helper target from `[Lang]`/receptor registration instead of hardcoding `"Pilot.Core.Format"`.
17. [`crates/pilot-analysis/src/infer.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/src/infer.rs) — Drop `SqlStringLit` match arm if referenced.
18. [`crates/pilot-vm/src/lower_expr.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm/src/lower_expr.rs) — Drop `SqlStringLit` match arm if referenced.
19. [`crates/pilot-analysis/tests/common/anticheat.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/tests/common/anticheat.rs) — Remove deleted `pilot-arena` and `pilot-ffi` crate path assertions.
20. [`crates/pilot-analysis/tests/milestone02_module_graph_and_comptime_parity.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/tests/milestone02_module_graph_and_comptime_parity.rs) — Replace `SourceMap` test helper usage with `ByteSpan`.
21. [`crates/pilot-analysis/tests/milestone11_adapter_blueprint_engine.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/tests/milestone11_adapter_blueprint_engine.rs) — Remove deleted `pilot_ffi` test import in favor of Pilot `stdlib/pkg/licensing.pilot`.
22. [`crates/pilot-analysis/tests/milestone58_runtime_crypto_wire_db_and_polyglot.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/tests/milestone58_runtime_crypto_wire_db_and_polyglot.rs) — Remove deleted `pilot_ffi` and `pilot_rt::net` Rust-only test calls in favor of Pilot conformance coverage.
23. [`crates/pilot-analysis/tests/milestone59_persistent_pilvm_lsp_and_dap.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-analysis/tests/milestone59_persistent_pilvm_lsp_and_dap.rs) & [`crates/pilot-lexer/tests/milestone52_seed_lexer_and_preprocessor.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-lexer/tests/milestone52_seed_lexer_and_preprocessor.rs) — Update for removed `LineIndex` and `SqlStringLit`.

*(Accompanying `.pilot` & doc updates in the same commit: `selfhost/gate/manifest.pilot` marking `pilot-arena` & `pilot-ffi` `must_be_absent: true` and ratcheting `paused_max_total_rust_loc()`, `stdlib/core/format.pilot`, `stdlib/db/sql_syntax.pilot`, `rust.md`, `pilot.md`, `docs/keyword_matrix.md`, and `docs/concept_matrix.md`.)*

---

## `PROP-002a` — Define the 3-File Airlock Under the B+ Freeze, Retire `pilot-rt`, Consolidate Stage-A Natives & Arm `GATE021`

- **Status:** **`APPROVED 2026-11-14; AMENDMENT A1 APPROVED 2026-11-14 AND APPLIED ON BRANCH claude/prop002a-a1`**. Most of the original `PROP-002a` landed on `f4` in `236afaa` and `7920302` (including `bridge_os.pilot`) before A1 existed. A1 is applied on top of that, together with the 18 uncommitted files taken over from the main checkout. See "A1 as built" below.
- **Origin:** replaces the original `PROP-002` draft, following the Claude ↔ Gemini review in [`conversations.md`](file:///home/moony/Documents/dev/projects/Pilot/conversations.md) (points P1–P9 closed; the owner chose freeze model **B+** on 2026-11-14). The "Final Consensus" section of that file is the reasoning behind every item below. Amendment A1 (P10 in the same file) changes the Pilot side of the airlock.
- **Behaviour change:** none. Conformance counts measured on `f4` at `f8dca46` (488 seed, 480 `--ir`, 480 `--emit c`) must not drop.

### Amendment A1 — "One Op, One Owner" (Owner Direction 2026-11-14, Approved)

**Why.** `stdlib/core/bridge_os.pilot` as written is a file of pass-through wrappers (`os_fd_read(…) => __engine("host_op", 18, …)`). Typed wrappers such as `File` would then call those wrappers. That adds a second layer that emulates the engine call. The owner's rule replaces it.

**The rule.** Every engine operation appears **exactly once** in the whole tree: as the entire body of one method on the type that owns it. A "runtime op" is one `HostOp` ID, or one Stage-A native name. A "compile-time hook" is one hook name. Everything else reaches the operation only through that method.

```pilot
pub partial struct File : IObject {
    [EngineOp]   // `[Inline]` is added in PROP-007
    priv fn read_internal(dst: i64, cap: i64) -> i64 => __engine("host_op", 18, .fd, dst, cap);

    pub fn read(dst: i64, cap: i64) -> i64 => .read_internal(dst, cap);  // the only caller
}
```

**`GATE021` checks** (replacing "only 2 files may contain `__engine`"):
1. `__engine` appears only as the entire body of a `priv` method or `comptime fn` marked `[EngineOp]`, with no other logic.
2. Each runtime op ID, Stage-A native name and compile-time hook name appears in exactly one `[EngineOp]` body across the tree.
3. Each `[EngineOp]` method has exactly one caller: its matching public method. This is a lexical count of `.<name>(` inside the owning type's body, which needs no name resolution.
4. No `bind_native_*` binding exists for an op that has an owner. Bindings that the compiler consumes (`bind_owned_elements`, `bind_record_pattern`, `bind_map_literal`, the `"[]"` member path) name **owner methods**, never raw ops.
5. Interfaces with an engine-backed implementation have **one** registered implementing `*Bundle`, and no second implementation of the same interface may exist. For example, `IDictionaryBuffer` is implemented only by `MapStorageBundle`.

**Who owns what:**
- **Compile-time hooks:** free `comptime fn`s under `stdlib/core/`. The seed parser already expands a one-call `comptime fn` wrapper at each bare-name call site (`crates/pilot-parser/src/attr_hooks.rs:55`, `tables.rs:101-113`). That is compile-time inlining, and it needs no Rust change. `Type.method` callees are **not** expanded by the parser (`tables.rs:91`), so genesis owners stay free functions.
  - `bridge_lang.pilot` as written already has this shape, so it **survives**. The only change is that each hook is called only through its function, never by raw `__engine`.
  - Callers of hooks used many times (for example the 106 `bind_receptor` sites) call the owner function. Raw `__engine` for hooks never appears outside `stdlib/core/`.
- **Runtime OS ops:** the existing typed wrapper in its current home owns them:
  - `File` (`stdlib/core/file.pilot`)
  - `Env`, `Process`, `TcpSocket` (`stdlib/core/env.pilot`)
  - `Instant` (`stdlib/std/time.pilot`)
  - `Mutex` (`stdlib/std/sync.pilot`)
  - new owner types only where none exists, for example a raw-memory owner in `Pilot.Core.Memory`.

  Moving OS owners out of `core.*` to satisfy `RING001` is an existing issue and not part of this amendment.
- **Ops with no caller yet** (threads, futex, `MemMap`, `FdRawCtl`, callbacks, …) get **no owner yet**. They exist only in `bridge.rs` and the ABI snapshot, and an owner is registered with the first caller. This avoids designing 51 owners up front.

**What changes in the work in progress:**

| Item | Disposition |
| :--- | :--- |
| `crates/pilot-vm/src/bridge.rs`, the `HostOp` table, `PilotErr`, `arch_kind`, layouts | **Unchanged.** Same ABI under both designs. |
| `crates/pilot-vm/src/value.rs`, `stage_a_natives.rs`, `pilot-rt` removal | **Unchanged.** |
| `stdlib/core/bridge_lang.pilot` | **Kept**, as the home of the compile-time hook owners. |
| `stdlib/core/os_abi.pilot` | **Kept** (layouts). |
| `stdlib/core/bridge_os.pilot` | **Removed before commit.** Each wrapper with a current caller becomes a `[EngineOp] priv` method on its owner type. The `CLI`/`LOADER` prepends in `pilot-cli/src/main.rs` and `run.rs` drop `bridge_os.pilot`. |
| The frozen set under B+ | `bridge.rs`, `bridge_lang.pilot`, the ABI snapshot, and a new **owner registry** (op → `file`, `Type.method`) in `selfhost/gate/airlock_abi.pilot`. The registry is **checked, not frozen**: `GATE021` requires exactly one owner per op and an entry naming the proposal for any move (Rule 4). |

**`[Inline]` dependency.** `[Inline]` is accepted today but nothing reads it: no consumer of `set_inline_hint` exists in `crates/` or `selfhost/`. Until `PROP-007` lands:
- Owner methods for **OS ops** are introduced now. The extra call frame is negligible next to a syscall.
- The **native bindings for VM data ops** (`map_*`, `list_*`, `str_*`) stay, listed in a new shrink-only `GATE021` table `TRANSITIONAL_NATIVE_BINDINGS`. Moving them to owners, deleting `MapBuffer`'s duplicate implementation and removing the bindings happen in `PROP-002b` after `PROP-007`. Doing it earlier would slow the seed.
- Owner methods carry `[EngineOp]` only. `[Inline]` is added to them in `PROP-007`, when it does something.
- Until then `[Inline]` is reported, not ignored: a compile-time warning `INLINE001` ("`[Inline]` is not implemented yet; this call is not inlined") and a `wiring.pilot` row at `SPEC`. It is **not** a seam: a seam fails at run time with `SEAM001` and would break every caller of the method it marks.

**A1 as built (2026-11-14), where it differs from the plan above:**
- **Owners must live in `Pilot.Core.*`.** `ENGINE001` (`selfhost/analyzer.pilot`, `verify_engine_usage`) rejects `__engine` in any other package, which matches the owner's rule that engine calls never leave the core directory. So `Instant` (`std/time`) and `Mutex` (`std/sync`) are not owners. The owners are `RawMem` and `Fd` (new file `stdlib/core/host_raw.pilot`), `File` (`stdlib/core/file.pilot`), and `Env`, `Process`, `TcpSocket`, `Thread`, `Clock`, `Entropy`, `Ffi` (`stdlib/core/env.pilot`). `std/time`, `std/proc`, `std/io`, `std/env` and the gate call the public methods.
- **Conformance programs count as callers.** For the same reason a test cannot call `__engine` itself, so every op that a conformance program exercises has an owner. 44 of 51 ops have owners. Ops 21 (`FdPwrite`), 31 (`FsPathCtl`), 43–45 (`Dl*`), 49 (`TrapFault`) and 50 (`TrapSeam`) have no Pilot caller and no owner.
- **Owner method names avoid registered keywords** (`map`, `open`, `signal`, `call`): `RawMem.map_pages`, `Fd.open_path`, `Process.send_signal`, `Ffi.call_raw`.
- **The owner types are static-method types, not handle types.** `File`, `Fd` and the rest take raw descriptors and addresses as arguments, as `bridge_os.pilot` did. Giving them real handles (`File` holding its `fd`) is later work.
- **Owner methods carry `[EngineOp]` only.** `EngineOpAttr` is declared in `host_raw.pilot`. `[Inline]` reports `INLINE001`.
- **`GATE021` checks 1–4 are built** in `selfhost/gate/airlock_abi.pilot`, scoped to each owner's struct body. Check 5 is `SPEC` until `PROP-002b`. The legacy `__engine` ratchet in `seed_audit.pilot` no longer counts `host_op` sites, because the owner check governs those.
- **`register_pratt_op`** is removed from the frozen hook list (`airlock_lang_hooks()`) and listed in `airlock_transitional_hooks()`.
- **Fixed in the same change (found while verifying):**
  - `tests/conformance/runtime/bridge_os.pilot` failed with `TYPE001` (`str.byte_at` takes `usize`, returns `u8`) on `f4` before A1. It is renamed `runtime/host_ops.pilot` and casts explicitly. Rerun 2026-11-15: passes on the seed and `--ir`. On `--emit c` it printed `0` instead of `32` for `OsRandom`, because `runtime/c/pilot_rt_bridge.h` returned `0` rather than the byte count the frozen signature (`-> n`) requires; fixed there (C runtime, no Rust change), and it now passes on all three engines.
  - `std/proc` `proc_make_pipe` declared its outputs `ref` instead of `var ref` (committed in `7920302`), so pipes were never wired and `pilot test` captured no output from any case. Fixed; the checker gap that allowed it is known bug `check.assign_readonly_ref_param`.
  - `bridge.rs` created pipes without close-on-exec, so a spawned child inherited the write end of its own stdin and never saw end-of-input (`runtime/stdin_lines` hung the suite). Both ends are now close-on-exec. `bridge.rs` is in this proposal's file list; this is an implementation fix, not a change to any frozen item.
  - The two pipe fixes above also explain the `pilot-cli` `cli_attrs` failures on `f4` (`the C compiler rejected …` with empty compiler output: `cc` ran but its output and exit code were lost) and the `pilot gate --full` litmus in `milestone01` `m1_3` (it runs `pilot test` in a sandbox). After the fixes, `debug_traces_natively` passed on its own, and six simultaneous `pilot build`s of `attr_debug.pilot` all succeeded. The full `cli_attrs` and `m1_3` runs have not been repeated.
  - `crates/pilot-vm/tests/runtime_memory.rs` (in this proposal's file list) probed `UnitHeader.read(path, 0)`, `h.package` and `h.imports`, which `selfhost/loader/module_graph.pilot` no longer has, so the raw VM read a null and faulted in `str_len`. The probe now uses `UnitHeader.read(path)`, `package_name()` and `decls`; the expected counts (2, then 3) are unchanged. Rerun 2026-11-15: `cargo test -p pilot-vm --test runtime_memory` passes 8/8.
  - `File.list_dirs` / `File.walk_files` (`stdlib/core/file.pilot`) treated a symbolic link to a directory as a file (op 27 reports links as `stat_kind_symlink`), so `pilot gate` on a workspace whose crates are linked (`milestone01` `m1_2`) missed every seam in them and failed `GATE012` six times. A link now counts as what it points to. `pilot gate --json` on that overlay passes 4/4 after the fix.

**Two checks before resuming** (results recorded here):
1. **C1 — Constructor representation.** A conformance program that tests whether a constructor body can produce an engine handle as `self` on the seed (today `Map`'s constructor is a native binding to `map_new`).
   - If it can, owning types keep no constructor binding.
   - If not, each owning type keeps exactly one constructor binding, to its `priv new_internal`, listed in `TRANSITIONAL_NATIVE_BINDINGS`.
   - Either way, Stage-A `Map`/`List` declare no fields and carry `[StandIn(…, "F6")]` (precedent: `GeneratorFrameHandle`, `stdlib/core/generator.pilot:29`).
   - **Result (2026-11-14): no.** On the seed, `self = __engine("map_new")` in a constructor works: the program printed `7`, as expected. On PilIR (`pilot ir run`) the same program stops at the registered seam `ir.lower_construct` (node kind 102). So for now each data owner in `PROP-002b` keeps one constructor binding, listed in `transitional_native_bindings()`, until that seam is connected.
2. **C2 — Speed baseline.** `pilot bench` on the seed before and after, gated by a regression threshold the owner sets.
   - **Owner, 2026-11-15:** "all tests and speeds are now approved". Recorded as the owner accepting the pending test edits and the current seed speed. No `pilot bench` baseline was measured or written down, and no regression threshold was set.
   - **Baseline measured (2026-11-15).** `pilot bench` only shows the milestone tests' times (toolchain work, not run-time speed), so the baseline is the run-time benchmark suite instead: `benchmarks/native/record.pilot` now also times each program on the seed (`pilot run`, the release build) and writes `benchmarks/native/baseline.tsv`. Command: `target/release/pilot run benchmarks/native/record.pilot` (fastest of 5 wall-clock runs, process start included; working tree after `PROP-009` §17, on top of 2a82f59; Intel Core Ultra 9 275HX, 24 threads, Linux).

     | Benchmark | Seed µs | Native Pilot µs | C µs | Seed ÷ C | Native ÷ C |
     | :--- | ---: | ---: | ---: | ---: | ---: |
     | fib | 711,021 | 2,963 | 2,900 | 245.18 | 1.02 |
     | numeric_loop | 1,825,113 | 38,302 | 37,991 | 48.04 | 1.01 |
     | string_build | 468,387 | 1,898 | 923 | 507.46 | 2.06 |
     | map_workload | 499,508 | 75,152 | 44,062 | 11.34 | 1.71 |
     | records | 1,756,300 | 19,097 | 1,267 | 1386.19 | 15.07 |
     | string_compare | 1,374,717 | 96,680 | 21,166 | 64.95 | 4.57 |
     | string_find | 1,144,568 | 3,000 | 1,393 | 821.66 | 2.15 |
     | string_map | 538,926 | 37,749 | 24,430 | 22.06 | 1.55 |

     About 420 ms of every seed time is start-up (loading and checking the stdlib: an empty `fn main` takes 419,872 µs), so the seed's own work is the seed time minus about 420 ms. Separate runs differed by up to about 3%.
   - **Regression threshold (owner, 2026-11-15): "5% slower max".** No benchmark's seed time may be more than 5% above its baseline seed time. This is the pass/fail line for the gates that cite C2: Amendment A1's transitional bindings, `PROP-007` step 4 and `PROP-008` step 3. Check it with `target/release/pilot run benchmarks/native/record.pilot -- --check`, which times each program on the seed (fastest of 5), writes nothing, and exits 1 when any is over the limit. On this tree it passed (0.97x–1.00x of the baseline). With fib's baseline lowered to 600,000 µs as a test, it reported fib over the limit and exited 1. Re-record the baseline (`record.pilot` without `--check`) only with the owner's approval, and on the same machine and release build.

**Grammar hook (`PROP-008`).** The operator hooks frozen in `bridge_lang.pilot` are `grammar_entry(...)` (called by the operator attributes in `stdlib/core/operators/`) and `register_grammar(entries)`, with the `ParseShape` and `OpLevel` lists, not `register_pratt_op(sym, fixity, bp, assoc, bin_op)`. Freezing the latter would make integer binding powers and native `BinaryOp` codes permanent. Until `PROP-008` step 1 lands, `register_pratt_op` stays as a **transitional** hook listed in `TRANSITIONAL_NATIVE_BINDINGS`, not in the frozen set.

**Rust file list.** Unchanged by A1 except `crates/pilot-cli/src/main.rs` and `run.rs`, which drop the `bridge_os.pilot` prepend; both are already listed.

### 1. Freeze Model B+ (Owner Decision)

This applies to [`stdlib/core/bridge_lang.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/bridge_lang.pilot), [`crates/pilot-vm/src/bridge.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm/src/bridge.rs), the ABI snapshot and its owner registry. *(Amended by A1: `bridge_os.pilot` is no longer an airlock file.)*

**Frozen forever.** Each of the following, once written, is never deleted, renumbered or given a new meaning:
- every `HostOp` ID and signature
- every sub-code of a multiplexed op
- every `PilotErr` code
- every `arch_kind` value
- every `[Layout(c_abi)]` field offset
- every `bridge_lang` hook signature

**Append-only.** New `HostOp` IDs (≥ 51), new sub-codes, new `PilotErr` codes (≥ 52), new `arch_kind` values (≥ 7), and new `bridge_lang` hooks or `bind_lang_protocol` keys may be added. Each addition needs its own owner-approved entry in this file.

**Enforcement.** A checked-in ABI snapshot, `selfhost/gate/airlock_abi.pilot`, lists every frozen item above. `GATE021` compares the airlock files with it and fails on any change that is not a pure append naming its proposal.

**Same commit.** [`AGENTS.md`](file:///home/moony/Documents/dev/projects/Pilot/AGENTS.md) §1.2.3 ("zero modifications, deletions, or workarounds") is reworded to B+. §1.2.2 gains the Stage-A exception in §4 below.

### 2. `bridge_lang.pilot` (Compile-Time Hooks Only, Zero Runtime Functions)

`[EnforceCoreNaming] pub lvl(100) pkg Pilot.Core.Bridge.Lang;` It contains:
- One explicit 1:1 `pub comptime fn` per compile-time `__engine` op in the generated audit (§5). That includes the hooks the first draft folded under umbrella names:
  - `bind_list_literal`, `bind_map_literal`, `bind_list_splat`
  - `bind_slice_pattern`, `bind_record_pattern`
  - `bind_absent`, `bind_absent_test`
  - `register_keyword`, `register_keyword_category`
  - `bind_owned_elements`, `bind_owned_payload`
  - `bind_range_index`, `bind_operator_intrinsic`, `decl_operator_fn`
  - `query_registered_symbols`
- The type-query hooks `type_sizeof`, `type_alignof`, `type_offsetof`, `type_fields_of` and `type_is_copy`.
  - **Honest status:** only `sizeof` answers today. `alignof`, `offsetof` and `fields_of` stop at `SEED001` (`native_calls.pilot:67-68`, `wiring.pilot` row `K4` at `::Spec`). The hooks define the frozen signature; the answers remain at their registered seam.
- `bind_lang_protocol(protocol_key, slot_key, target_sym)`: the single generic extension hook. Its keys are gated by an allowlist in `GATE021`, changed only through this file.
- *(Superseded by A1 and `PROP-008`: the frozen operator hook is `register_grammar(entries)`; `register_pratt_op` is transitional.)* `register_pratt_op` takes a numeric `BinaryOp` code. That removes the six hardcoded English operator words (`"and"`, `"or"`, `"is"`, `"union"`, `"intersect"`, `"except"`) from `crates/pilot-parser/src/tables.rs` and their exemption in `seed_audit.pilot`.

`host_spawn_eval` gets **no** hook. It becomes pure Pilot over `ProcSpawn` + `ProcWait` + `FdRead`.

### 3. The `HostOp` ABI in `bridge.rs` (51 Scalar/Pointer `HostOp`s, `HOST_OP_COUNT = 51`)

*(Amended by A1. The table below is the frozen ABI. The `os_*` names in the "signature" column describe each op's arguments. They are **not** wrapper functions in a `bridge_os.pilot` file: each op with a caller is reached through one `[EngineOp]` owner method.)* Every argument and return is an `i64` or `f64` scalar or pointer. There is no `VmHostCall` and no VM value-slot op: list, map and string slots stay in the non-frozen Stage-A table (§4). Struct layouts live in a new non-frozen-by-name file, `stdlib/core/os_abi.pilot`, whose offsets are frozen through the snapshot.

**Conventions:**
- **Fallible ops** return `>= 0` on success and `-(PilotErr)` on failure.
- **`[Infallible]` ops** never return `-PilotErr`: 0, 1, 2, 4–11, 49 and 50.
- **Address outputs:** ops producing a 64-bit address (`MemMap`, `DlSym`, `FfiCallbackAlloc`) write it to `*out_ptr` and return `0` or `-PilotErr`.
- **Variable-length outputs** return `-44` (`E_RANGE`) and write the required length to `*out_len_ptr` when the buffer is short.
- **Descriptors:** `0`, `1` and `2` are stdin, stdout and stderr on every OS. Numbers `>= 3` index `bridge.rs`'s unified table of files, pipes and sockets.
- **Unsupported targets:** an op the target cannot provide returns `-43` (`E_NOSYS`). It never traps.
- **No unwind catching:** `os_ffi_call` performs no unwind catching. Foreign exceptions and hardware faults in foreign code end the process. Foreign errors are caught inside each `std.polyglot.prog.*` absorber's C-ABI shim and lifted to `res<T, ForeignErr>`.

| ID | Op | Op signature (arguments → result) | Sub-codes / notes |
| ---: | :--- | :--- | :--- |
| 0 | SlabAlloc | `os_slab_alloc(size, align) -> ptr` | Infallible; `0` = OOM or empty |
| 1 | SlabRealloc | `os_slab_realloc(ptr, old_sz, new_sz, align) -> ptr` | Infallible; `0` = OOM or freed |
| 2 | SlabFree | `os_slab_free(ptr, size, align) -> 0` | Infallible |
| 3 | MemMap | `os_mem_map(op, addr_or_fd, len, prot, offset, out_ptr) -> rc` | `0=anon,1=file,2=unmap,3=protect,4=advise` |
| 4 | MemCopy | `os_mem_copy(dst, src, len) -> len` | Infallible; overlap-safe |
| 5 | MemSet | `os_mem_set(dst, byte, len) -> len` | Infallible |
| 6 | MemCmp | `os_mem_cmp(a, b, len) -> -1/0/1` | Infallible |
| 7 | LoadInt | `os_load_int(ptr, bits, signed) -> i64` | Infallible; unaligned 8/16/32/64 |
| 8 | StoreInt | `os_store_int(ptr, val, bits) -> 0` | Infallible |
| 9 | LoadFloat | `os_load_float(ptr, bits) -> f64` | Infallible; 32/64 |
| 10 | StoreFloat | `os_store_float(ptr, val, bits) -> 0` | Infallible |
| 11 | AtomicInt | `os_atomic_int(op, ptr, expected, desired, order) -> i64` | Infallible; `0=load,1=store,2=swap,3=add,4=sub,5=and,6=or,7=xor,8=cas_prev,9=cas_ok,10=fence` |
| 12 | ThreadSpawn | `os_thread_spawn(c_entry_fn_ptr, env_ptr, stack_bytes) -> handle` | entry is `extern "C" fn(usize) -> i64` |
| 13 | ThreadJoin | `os_thread_join(handle, out_ret_ptr) -> rc` | |
| 14 | ThreadCtl | `os_thread_ctl(op, arg) -> val` | `0=tid,1=yield,2=parallelism,3=detach,4=last_raw_os_error,5=set_name` |
| 15 | Futex | `os_futex(op, u32_ptr, expected, timeout_ns) -> rc` | `0=wait,1=wake_one,2=wake_all`; `timeout_ns < 0` waits forever |
| 16 | FdOpen | `os_fd_open(path_ptr, path_len, flags, mode) -> fd` | flags `READ=1,WRITE=2,CREAT=4,TRUNC=8,APPEND=16,EXCL=32,NONBLOCK=64` |
| 17 | FdClose | `os_fd_close(fd) -> rc` | 0–2 allowed; later use of that index gives `-11` |
| 18 | FdRead | `os_fd_read(fd, dst, cap) -> n` | `0` = EOF |
| 19 | FdWrite | `os_fd_write(fd, src, len) -> n` | |
| 20 | FdPread | `os_fd_pread(fd, dst, cap, offset) -> n` | |
| 21 | FdPwrite | `os_fd_pwrite(fd, src, len, offset) -> n` | |
| 22 | FdSeekSync | `os_fd_seek_sync(op, fd, arg) -> pos_or_rc` | `0=seek_set,1=seek_cur,2=seek_end,3=fsync,4=fdatasync,5=ftruncate,6=fstat,7=isatty,8=set_nonblock,9=flock` |
| 23 | FdPipeDup | `os_fd_pipe_dup(op, a0, a1) -> fd_or_rc` | `0=pipe,1=dup,2=dup2` |
| 24 | IoPoll | `os_io_poll(pollfds_ptr, nfds, timeout_ms) -> ready` | Portable readiness baseline; a Windows non-socket handle gives `-42` |
| 25 | FdRawCtl | `os_fd_raw_ctl(op, fd_or_raw, kind_flags, out_raw_ptr) -> rc` | `0=as_raw,1=adopt_raw,2=release_raw`; kinds `1=file_or_pipe,2=socket` |
| 26 | FsStat | `os_fs_stat(follow, path_ptr, path_len, out_stat_ptr) -> rc` | `OsStat` |
| 27 | FsReadDir | `os_fs_read_dir(path_ptr, path_len, dst, cap, out_len_ptr) -> count` | Unsorted; packed `[kind u8, name_len u16le, utf8…]` |
| 28 | FsMkdir | `os_fs_mkdir(path_ptr, path_len, parents, mode) -> rc` | |
| 29 | FsRemove | `os_fs_remove(path_ptr, path_len, recursive) -> rc` | |
| 30 | FsRenameLink | `os_fs_rename_link(op, src_ptr, src_len, dst_ptr, dst_len, out_len_ptr) -> rc` | `0=rename,1=hard_link,2=symlink,3=copy_file,4=readlink` |
| 31 | FsPathCtl | `os_fs_path_ctl(op, path_ptr, path_len, out, cap, out_len_ptr) -> rc` | `0=getcwd,1=chdir,2=realpath,3=chmod,4=utimens` |
| 32 | ProcSpawn | `os_proc_spawn(spec_ptr, out_pid_ptr) -> handle` | `OsProcSpec` |
| 33 | ProcWait | `os_proc_wait(handle, nonblocking, out_code_ptr) -> state` | `0=running,1=exited` |
| 34 | ProcSignal | `os_proc_signal(op, handle_or_pid, sig) -> val_or_rc` | `0=signal_handle,1=signal_pid,2=self_pid,3=exit` |
| 35 | EnvCtl | `os_env_ctl(op, key_ptr, key_len, buf, cap, out_len_ptr) -> rc` | `0=get,1=environ,2=argv,3=exe,4=set,5=unset` |
| 36 | ClockNowNs | `os_clock_now_ns(clock_id) -> ns` | `0=realtime,1=monotonic,2=process_cpu,3=thread_cpu` |
| 37 | SleepNs | `os_sleep_ns(ns) -> rc` | |
| 38 | OsRandom | `os_random_bytes(dst, len) -> n` | OS CSPRNG |
| 39 | NetSocket | `os_net_socket(domain, type, proto, flags) -> fd` | domain `1=unix,4=ipv4,6=ipv6`; type `1=stream,2=dgram` |
| 40 | NetConnectBind | `os_net_connect_bind(op, fd, addr_ptr, arg) -> fd_or_rc` | `0=connect,1=bind,2=listen,3=accept,4=shutdown` |
| 41 | NetIo | `os_net_io(op, fd, buf, len, flags, addr_ptr) -> n` | `0=recv,1=send,2=recvfrom,3=sendto,4=setsockopt,5=getsockopt,6=local_addr,7=peer_addr` |
| 42 | NetResolve | `os_net_resolve(host_ptr, host_len, port, dst, cap, out_len_ptr) -> count` | writes `OsSockAddr[]` |
| 43 | DlOpen | `os_dl_open(path_ptr, path_len, flags) -> lib` | `path_len == 0` gives the default namespace |
| 44 | DlSym | `os_dl_sym(lib, sym_ptr, sym_len, out_fn_ptr) -> rc` | |
| 45 | DlClose | `os_dl_close(lib) -> rc` | |
| 46 | FfiCall | `os_ffi_call(fn_ptr, abi_desc_ptr, gpr_ptr, vec_ptr, stack_ptr, ret_ptr) -> rc` | `OsFfiAbiDesc` in, `OsFfiRetRegs` out; classification stays in `pilot.abi` |
| 47 | FfiCallbackAlloc | `os_ffi_callback_alloc(c_dispatch_fn_ptr, env_ptr, abi_desc_ptr, out_fn_ptr) -> slot` | C-callable trampoline |
| 48 | FfiCallbackFree | `os_ffi_callback_free(slot) -> rc` | |
| 49 | TrapFault | `os_trap_fault(msg_ptr, msg_len) -> never` | Infallible |
| 50 | TrapSeam | `os_trap_seam(id_ptr, id_len, note_ptr, note_len) -> never` | Infallible; `SEAM001` |

**`PilotErr` (returned as `-1` … `-51`):**

| Group | Codes |
| :--- | :--- |
| Permissions and lookup | `1=E_PERM`, `2=E_ACCES`, `3=E_NOENT`, `4=E_EXIST` |
| I/O and non-blocking state | `5=E_INTR`, `6=E_IO`, `7=E_AGAIN`, `8=E_INPROGRESS`, `9=E_ALREADY`, `10=E_NOMEM`, `11=E_BADF`, `12=E_INVAL` |
| Filesystem and storage | `13=E_ISDIR`, `14=E_NOTDIR`, `15=E_NOTEMPTY`, `16=E_BUSY`, `17=E_EXDEV`, `18=E_NOSPC`, `19=E_ROFS`, `20=E_DQUOT`, `21=E_MFILE`, `22=E_NFILE`, `23=E_NAMETOOLONG`, `24=E_LOOP`, `25=E_FBIG`, `26=E_SPIPE`, `27=E_PIPE` |
| Processes | `28=E_SRCH`, `29=E_CHILD` |
| Network and sockets | `30=E_TIMEDOUT`, `31=E_CONNREFUSED`, `32=E_CONNRESET`, `33=E_CONNABORTED`, `34=E_NOTCONN`, `35=E_ISCONN`, `36=E_ADDRINUSE`, `37=E_ADDRNOTAVAIL`, `38=E_NETUNREACH`, `39=E_HOSTUNREACH`, `40=E_MSGSIZE`, `41=E_AFNOSUPPORT` |
| Capability, buffers, DNS, other | `42=E_NOTSUP`, `43=E_NOSYS`, `44=E_RANGE`, `45=E_AI_NONAME`, `46=E_AI_AGAIN`, `47=E_AI_FAIL`, `48=E_DEADLK`, `49=E_ILSEQ`, `50=E_CANCELED`, `51=E_OTHER` (raw code via `ThreadCtl` sub-code 4) |

`bridge.rs` maps POSIX `errno`, Win32 `GetLastError`, `WSAGetLastError` and `getaddrinfo` codes to these on every host. That retires `stand_in_entry("adapter.errno_values", …)` in `seams.pilot`.

**`arch_kind`:** `0=host_default`, `1=x86_64_sysv`, `2=x86_64_win64`, `3=aarch64_aapcs`, `4=aarch64_darwin`, `5=riscv64_lp64d`, `6=wasm32_c`. A non-zero value that doesn't match the host gives `-43`.

**Layouts (`[Layout(c_abi)] pub struct … : IValue` in `stdlib/core/os_abi.pilot`, `#[repr(C)]` in `bridge.rs`):**

| Struct | Size / align | Fields (byte offset) |
| :--- | :--- | :--- |
| `OsSlice` | 16 / 8 | `ptr u64 (0)`, `len u64 (8)` |
| `OsPollFd` | 16 / 8 | `fd i64 (0)`, `events u32 (8)`, `revents u32 (12)`; `1=IN,2=OUT,4=ERR,8=HUP,16=NVAL` |
| `OsStat` | 72 / 8 | `kind u64 (0)`, `size (8)`, `mode (16)`, `mtime_ns i64 (24)`, `atime_ns (32)`, `ctime_ns (40)`, `dev u64 (48)`, `ino (56)`, `nlink (64)`; kind `0=other,1=file,2=dir,3=symlink,4=fifo,5=sock,6=dev` |
| `OsProcSpec` | 96 / 8 | `prog_ptr (0)`, `prog_len (8)`, `argv_ptr → OsSlice[] (16)`, `argc (24)`, `envp_ptr → OsSlice[] "K=V" (32)`, `envc (40)`, `cwd_ptr (48)`, `cwd_len (56)`, `stdin_fd i64 (64)`, `stdout_fd (72)`, `stderr_fd (80)` (`-1=inherit,-2=null,>=0=descriptor`), `env_mode u64 (88)` (`0=inherit_and_override,1=replace_all,2=empty`) |
| `OsSockAddr` | 144 / 8 | `family u32 (0)`, `port u16 (4)`, `addr_len u16 (6)`, `addr_bytes [u8;128] (8)`, `flowinfo u32 (136)`, `scope_id u32 (140)` |
| `OsFfiAbiDesc` | 32 / 8 | `arch_kind u32 (0)`, `gpr_count u16 (4)` (≤ 8), `vec_count u16 (6)` (≤ 8 × 128-bit), `stack_bytes u32 (8)`, `variadic_fixed_args u16 (12)` (`0xFFFF` = not variadic), `flags u16 (14)` (bit 0 = `has_sret`; other bits must be 0), `sret_ptr u64 (16)`, `_reserved u64 (24)` = 0 |
| `OsFfiRetRegs` | 80 / 16 | `gpr [u64;2] (0)`, `vec [[u8;16];4] (16)` |

### 4. Stage-A Native Table (Non-Frozen, Shrink-Only After `PROP-005`)

- **One file for string-matched native names.** Move the 72 string-matched `CallNative` arms, with no new logic, into one new file: `crates/pilot-vm/src/stage_a_natives.rs`. That is `heap.rs` (65) and `vm_gen.rs` (7).
  - It is the only `pilot-vm` file allowed to match native op names, and it is deleted in `PROP-005`/`PROP-006`.
  - The three `ops.rs` arms (`"simd_float"`, `"gpr_signed"`, `"gpr_unsigned"`) are `[Bits]` register-class descriptors and stay.
  - `crates/pilot-analysis/src/infer.rs` keeps its exception until `PROP-004`.
- **What the table holds:**
  - VM list, map and string slots
  - generator ops (`__gen_*`), bound in `stdlib/core/builtins/native_calls.pilot`
  - the marshalling natives `str_copy_to_ptr` and `str_from_ptr`
  - the entry-pointer natives `vm_thread_entry_ptr` and `vm_callback_dispatch_ptr`, which supply `ThreadSpawn`/`FfiCallbackAlloc` with C entry points on Stage A
- **`GATE021` tables in `selfhost/gate/manifest.pilot`:**
  - **`STAGE_A_VM_NATIVES`:** additions only through an entry in this file that names the wave which deletes them. Shrink-only after `PROP-005`.
  - **`STAGE_A_HOST_NATIVES`:** the closed list of 15 `cli_*`, `chk_snapshot`, `chk_report` and `tool_input_text`, bound in `selfhost/cli/host.pilot` and `selfhost/check/snapshot.pilot`. It reaches 0 at `PROP-006`.
  - **`LEGACY_ENGINE_RATCHET`:** checked-in `(file, op)` pairs for Bucket B and D call sites still on `__engine`, including `stdlib/core/keywords.pilot` lines 954–977 (`reflect_category_keywords`). The list may only shrink.

### 5. `.pilot` Migration in `PROP-002a`

- **Audit, generated by script and checked in.** The audit counts 122 files (49 `stdlib`, 66 `selfhost`, 2 `tools`, 3 `tests`, 2 `examples`) and 121 op names. `GATE021` runs it with lexer awareness and fails on any op name that has no mapping.
- **Bucket A** (genesis, syntax and native bindings) moves to `bridge_lang.pilot` hooks. Exception: `keywords.pilot` lines 954–977 stay on the ratchet.
- **Ring-1/Ring-2 OS callers** move to `[EngineOp]` owner methods on their types (Amendment A1): `file.pilot`, `env.pilot`, `std/{io,proc,time,env}.pilot`, `http.pilot` and `polyglot/bridge.pilot`. The domain algorithms (`walk_files`, sorted `list_dirs`, `str_find`, `proc_first_done`, `unit_header`) become pure Pilot.
- **Buckets B and D** (`map_*`, `ast_*`, `sym_*`, `layout_*`, `mount_unit`, `source_slice`) stay on `LEGACY_ENGINE_RATCHET` for `PROP-002b`/`PROP-004`.

### 6. Implementation Scope and Rust LOC Budget

- **The ABI is frozen, not the implementation.** All 51 ops are *defined* and frozen in this proposal. Fixing a bug in an op's implementation without changing its ABI needs only an ordinary proposal entry.
- **Implemented now:** an op (per target) is implemented in `PROP-002a` only when it replaces existing Rust behaviour (`pilot-rt`, or the OS code in `heap.rs`/`host.rs`) or a current `.pilot` caller needs it.
- **Not built yet:** every other op/target pair calls `os_trap_seam` with a registered seam `bridge.<op>[.<target>]` in `seams.pilot` and a `wiring.pilot` row at `SPEC`.
  - This covers, for example: threads, futex, `MemMap`, `FdRawCtl`, callbacks, and FFI on AArch64/RISC-V. Today's `invoke_abi_frame` is x86_64 only (`crates/pilot-rt/src/c_abi.rs:112`, `:187`, fallback at `:258`).
  - `E_NOSYS` is reserved for targets that genuinely lack the capability. It never means "not written yet".
- **Budget:** net Rust `src` LOC ≤ 0. Deleting `pilot-rt` removes 683 lines; `bridge.rs` takes over that behaviour. `paused_max_total_rust_loc()` is ratcheted to the measured post-change total, which must be ≤ 20,450.

### 7. Acceptance Tests (Written First, Then Frozen — `AGENTS.md` §9.1)

For each implemented op, a conformance program gets its expected result from outside the code. Examples:
- `FdOpen` on a missing path returns `-3` (POSIX `open(2)` `ENOENT`).
- `FsRenameLink` rename across filesystems returns `-17` (`rename(2)` `EXDEV`).
- `MemCmp` on known byte strings returns `-1`, `0` and `+1`.
- A mismatched `arch_kind` returns `-43`.
- `OsRandom` fills the requested length.

For each seam, a test checks that the path stops with `SEAM001`. The 488/480/480 conformance counts must hold.

### 8. Exact Rust File List (Strictly Bounded — `proposals.md` Rule 2)

#### CREATE (3)
1. `crates/pilot-vm/src/bridge.rs`: the `#[repr(u8)] HostOp` enum (`0..=50`), the `HOST_OP_COUNT = 51` assertion, `#[repr(C)]` layouts, the `PilotErr` mapping and the dispatch.
2. `crates/pilot-vm/src/value.rs`: `PilotVal`, moved verbatim from `crates/pilot-rt/src/value.rs`.
3. `crates/pilot-vm/src/stage_a_natives.rs`: the 72 string-matched `CallNative` arms, moved from `heap.rs` and `vm_gen.rs`.

#### DELETE (6, retiring `crates/pilot-rt`)
1. `crates/pilot-rt/Cargo.toml`
2. `crates/pilot-rt/src/lib.rs`
3. `crates/pilot-rt/src/value.rs` (moved)
4. `crates/pilot-rt/src/stdlib.rs` (OS calls go to `bridge.rs`; `walk_files` becomes pure Pilot)
5. `crates/pilot-rt/src/c_abi.rs` (`DynamicLibrary`, `invoke_abi_frame` and `fill_os_random` go into `bridge.rs`)
6. `crates/pilot-rt/src/time.rs` (`now_nanos` becomes `ClockNowNs`)

#### MODIFY (57)
- **Manifests & lockfile (5):** `Cargo.toml`, `Cargo.lock`, `crates/pilot-vm/Cargo.toml`, `crates/pilot-cli/Cargo.toml`, `crates/pilot-analysis/Cargo.toml`.
- **Parser, for `bridge_lang` hook resolution and non-bootstrap hook ingestion (4)** *(A1/`PROP-008`: not the numeric `register_pratt_op` change; the parser keeps reading the transitional `register_pratt_op` unchanged, and `register_grammar` lands with `PROP-008` step 1)*: `crates/pilot-parser/src/tables.rs`, `crates/pilot-parser/src/decl.rs`, `crates/pilot-parser/src/attr_hooks.rs`, `crates/pilot-parser/src/pilot_impls.rs`.
- **Parser tests (1):** `crates/pilot-parser/tests/milestone51_homoiconic_syntax_and_contraction.rs` (`expr_sugar.pilot` code-marker update after Bucket A migration).
- **`pilot-vm` sources (29):**
  - `lib.rs` (module declarations)
  - `vm.rs` (`HostOp` dispatch wiring; generator dispatch moves to `stage_a_natives.rs`)
  - `heap.rs` (arms move out; its OS calls go to `bridge.rs`)
  - `host.rs` (its OS calls go to `bridge.rs`)
  - `vm_gen.rs` (arms move out)
  - `checked.rs`, `collect.rs`, `ops.rs`, `pilc.rs`, `strings.rs`
  - `lower_args.rs`, `lower_call.rs`, `lower_cascade.rs`, `lower_closure.rs`, `lower_destructure.rs`, `lower_dispatch.rs`, `lower_drop.rs`, `lower_expr.rs`, `lower_for.rs`, `lower_global.rs`, `lower_members.rs`, `lower_native.rs`, `lower_probe.rs`, `lower_stmt.rs`, `lower_struct.rs`, `lower_switch.rs`, `lower_unit.rs`, `lower_variant.rs`, `lower_yield.rs`

  Except where noted, the change is the `pilot_rt::PilotVal` → `crate::value::PilotVal` import update.
- **`pilot-vm` tests (1):** `crates/pilot-vm/tests/runtime_memory.rs`.
- **`pilot-cli` (3):** `crates/pilot-cli/src/main.rs` (prepend `os_abi.pilot` to `CLI`), `crates/pilot-cli/src/run.rs` (prepend `os_abi.pilot` to `LOADER`; `PilotVal` import), `crates/pilot-cli/src/tool.rs`. *(A1: no `bridge_os.pilot` prepend.)*
- **`pilot-analysis` sources (2):** `crates/pilot-analysis/src/comptime.rs`, `crates/pilot-analysis/src/infer.rs`.
- **`pilot-analysis` tests (12):**
  - `tests/common/anticheat.rs`
  - `tests/milestone01_seed_unlock_and_pilot_gate.rs`
  - `tests/milestone02_module_graph_and_comptime_parity.rs`
  - `tests/milestone13_adaptive_hardware_calling_conventions.rs`
  - `tests/milestone50_lossless_bridge_and_reflection.rs`
  - `tests/milestone52_pilot_preprocessor_and_receptors.rs`
  - `tests/milestone53_cst_recovery_and_formatter.rs`
  - `tests/milestone54_perceus_ownership_and_escape.rs`
  - `tests/milestone56_layout_driven_codegen_and_gpu_backends.rs`
  - `tests/milestone58_runtime_crypto_wire_db_and_polyglot.rs`
  - `tests/milestone61_stage3_fixed_point_and_universal_rename_gate.rs` (prepend `stdlib/core/bridge_lang.pilot` to `bootstrap_rel_paths`)
  - `tests/phase1_rename_torture_gate.rs`

*This list is every Rust file that references `pilot_rt` today (`grep -rlE 'pilot_rt|pilot-rt' crates Cargo.toml`), plus the parser, `pilot-vm`, `pilot-cli` and test files required by §2, §4 and §5.*

#### Non-Rust files in the same commit
- New files: `stdlib/core/bridge_lang.pilot`, `stdlib/core/os_abi.pilot` and `selfhost/gate/airlock_abi.pilot` (ABI snapshot plus owner registry). *(A1: no `bridge_os.pilot`.)*
- `[EngineOp]` owner methods added to the existing OS types (A1), and the `[EngineOp]` attribute declared in `stdlib/core/`.
- The Bucket A and OS-caller `.pilot` migrations in §5.
- `selfhost/gate/manifest.pilot`:
  - the `GATE021` tables
  - mark `pilot-rt` and the stale `pilot-codegen` entry `must_be_absent: true`
  - rewrite the stale contraction-pause comment to match `AGENTS.md:20`
  - ratchet the total LOC
- `selfhost/gate/seed_audit.pilot`.
- `seams.pilot` and `wiring.pilot` rows.
- `AGENTS.md` §1.2.2/§1.2.3 (B+ and the Stage-A exception).
- `rust.md`, `pilot.md`, `docs/keyword_matrix.md` and `docs/concept_matrix.md` (rule 5).

### 9. Decisions Left to the Owner Before Approval

1. **`bridge.rs` versus the 1,500-line cap (`MOD001`).** 51 ops across Linux/macOS/Windows/WASI, plus FFI register shims for five architectures, will likely exceed 1,500 lines once fully built. `AGENTS.md` §1.2.2 says "strictly 1 runtime host dispatch file". Choose one:
   - **(a)** `bridge.rs` holds the enum, layouts, error mapping and dispatch, and per-subsystem implementation modules sit under `crates/pilot-vm/src/bridge/`. The ABI stays in one frozen file.
   - **(b)** Keep one file and accept that later ops stay at their seams until something else shrinks.
2. **Execution order:** `PROP-002a` must land before `PROP-002b`, and must be rebased on whatever `f4` contains when it is approved.
3. **Contraction pause.** `AGENTS.md` §1 pauses seed contraction until `F8` and limits Rust growth to execution-engine correctness. Retiring `pilot-rt` here, the seed inlining in `PROP-007` step 4 and the seed lexer change in `PROP-008` step 2 are not obviously inside that rule. Either approve them as explicit exceptions to the pause (and note it in `AGENTS.md` §1 in the same commit), or defer them to after `F8`.
4. **Grammar hook:** approve freezing `register_grammar(entries)` (`PROP-008` §8.1) in place of `register_pratt_op`, and pick the intended `||` / `&&` order (`PROP-008` §8.2).

---

## `PROP-002b` — Bucket B/D `.pilot` Migration & Retire `heap.rs`/`host.rs`

- **Status:** **`DRAFT — MUST BE ITEMIZED BEFORE OWNER APPROVAL`** *(depends on `PROP-002a` and `PROP-007`)*
- **Scope:**
  1. **VM data-op owners (Amendment A1):**
     - `Map`, `List` and `str` get `[EngineOp, Inline]` owner methods for their Stage-A ops (`map_set` → `Map.set_internal` → `Map.set`, and so on).
     - Indexers and other methods call the public method (`op []=` → `.set(k, v)`).
     - The storage and the single owner pair live in one `*Bundle` (for example `MapStorageBundle`) spliced into the type, not in a wrapper that forwards calls (today `Map.set` → `.inner.set`).
     - `MapBuffer`'s duplicate linear-search implementation is deleted. The pure-Pilot implementation is written once, in the bundle, when `F6` replaces the engine op.
  2. **Retire `TRANSITIONAL_NATIVE_BINDINGS`** to 0. Delete the `bind_native_method`/`bind_native_member` bindings for owned ops (`native_calls.pilot:39`, `:79`, `:85` and the rest), and delete the static and free duplicates (`Map.set_handle`, `map_set_pure` and similar in `map.pilot:154-164`).
  3. **Bucket B:** a mechanical migration of about 1,875 raw `map_*` call sites (`map_set` 939, `map_get` 842, `map_new` 94) to the owner methods `Map()`/`.get()`/`.set()`.
  4. **Bucket D:** about 800 `ast_*` call sites across 38 files, migrated to pure-Pilot `selfhost/ast.pilot` + `selfhost/check/` (with owners for any engine op that remains), or stopped at registered seams (rule 6).
  5. Delete `crates/pilot-vm/src/heap.rs` and `crates/pilot-vm/src/host.rs`. `stage_a_natives.rs` survives.
- **Gate:** the seed speed regression stays within the owner's threshold from the C2 baseline.
- **Required before approval:**
  - the exact Rust file list (CREATE / DELETE / MODIFY)
  - an itemized table of every conformance or milestone case allowed to become a `SEAM001`, with its seam ID and `wiring.pilot` downgrade
  - before and after conformance counts
- **`Map` slice, itemized 2026-11-15 under `PROP-007` option M (owner: "Option M"). APPROVED BY OWNER 2026-11-15 ("Approve Map slice").**
  - **What exists today (checked on the seed and PilIR, 2026-11-15).**
    - `Map`'s Pilot declaration (`stdlib/core/primitives/map.pilot`) is a stand-in that no engine runs for a user program. `pilot run` links only the program and what it `use`s; `map.pilot` reaches the type checker (`cli_checker_decls()` in `selfhost/cli/host.pilot`) but not the seed's code. `Map<K, V>()`, `.set`, `.get`, `.has`, `.keys`, `.remove`, `[]`, `[]=`, `in` and `for` all lower to Stage-A natives through `bind_native_*` in `stdlib/core/builtins/native_calls.pilot`.
    - So a method only `map.pilot` declares does not run. In a probe, `m.clear()` failed on the seed with "no method `clear` for this value", and on PilIR it stopped at the seam `ir.lower_construct` (node kind 207). The `inner: MapBuffer` field is never built on any engine.
    - `bind_native_method(get|set|has|keys|remove)` binds by method name for every receiver, not just `Map`. A receiver of unknown type relies on it.
    - The map ops (`map_new`, `map_set`, ...) are Stage-A natives in `transitional_native_bindings()`, not frozen `HostOp`s. `GATE021`'s owner registry (`airlock_host_ops()`) lists only `HostOp`s.
    - Separately, about 1,900 raw `__engine("map_*")` sites sit in 52 legacy files (`selfhost/vm/compiler.pilot` 196, `selfhost/parser.pilot` 175, `stdlib/crypto/ed25519.pilot` 146, ...). That is bucket B. It is compile-time code that C2 does not measure, and it is **not** in this slice.
  - **Changes (Pilot only; no Rust file is touched).**
    1. `stdlib/core/primitives/map.pilot`:
       - `Map` holds the engine's map handle, marked `[StandIn("map.storage", "a pure-Pilot hash map", "the engine's map is the storage", "F6")]`.
       - Its owners are `[EngineOp] priv static fn new_internal`, `get_internal`, `set_internal`, `has_internal`, `keys_internal` and `remove_internal`, each a single `__engine("map_*", ..)`.
       - The public `self()`, `get`, `set`, `has`, `keys`, `remove` and `count` call them. `op []`, `op []=` and `in` call the public methods.
       - Deleted: `MapBuffer` (the duplicate linear search), the `inner` field, `Map.new_handle`/`set_handle`/`get_handle`/`has_handle`, and `map_new_pure`/`map_set_pure`/`map_get_pure`/`map_has_pure`. `clear` is kept only if it has an engine op to own; otherwise it becomes `seam("map.clear", ..)`.
    2. `selfhost/cli/spec.pilot`: its 9 `Map.*_handle` calls move to the public methods.
    3. `selfhost/cli/host.pilot`: `map.pilot` joins the files every run links, so the seed compiles `Map`'s owner methods.
    4. `stdlib/core/builtins/native_calls.pilot`: delete the `Map` bindings (`bind_native_fn(Map)`, `bind_native_member(Map, "[]" | "[]=" | "in")`). Delete `bind_native_method(get|set|has|keys|remove)` once the suite shows no other receiver needs them; any that still do are listed here first. The lowercase `map` view's bindings stay, since `map` is another type.
    5. `selfhost/gate/airlock_abi.pilot`:
       - `GATE021` accepts an `[EngineOp]` owner whose op is a listed Stage-A native, with `Map` in the owner registry.
       - Each native `Map` owns leaves `transitional_native_bindings()`, except `map_new`, which keeps one constructor binding (C1).
    6. Records: `wiring.pilot`, `seams.pilot`, `pilot.md`, `concept_matrix.md` and the keyword matrix, if any row moves.
  - **Acceptance.**
    - New `tests/conformance/collections/map_owner_methods` (hand-worked output): `set`, `get`, `[]`, `[]=`, `in`, `has`, `remove`, `keys`, `for`, and a method only `map.pilot` declares. It runs on the seed, `--ir` and `--emit c`, and must fail if an owner body is replaced with a constant.
    - The seed suite goes from 539 passed, 0 failed (14 known bugs reproduced) to 540 passed, 0 failed.
    - `--ir` and `--emit c` pass on `collections/`, `types/` and `receptors/`.
    - `pilot gate` passes.
    - **C2:** `target/release/pilot run benchmarks/native/record.pilot -- --check` passes. Linking `map.pilot` into every run adds start-up time to all 8 benchmarks, not only the map ones, and that is measured too.
  - **Seams.** No conformance or milestone case is planned to become `SEAM001`. If one does, the work stops, and the case, its seam ID and its `wiring.pilot` downgrade are added here before going on.
  - **If C2 fails:** stop, record the numbers here, and size `PROP-007` option F to the failing benchmarks. The Pilot changes are kept unmerged until then.
  - **C2 result (2026-11-15): FAILED; stopped.** `record.pilot -- --check` exited 1:

    | Benchmark | Seed µs | Baseline µs | Over |
    | :--- | ---: | ---: | :--- |
    | fib | 760,821 | 711,021 | +7.0%, over |
    | numeric_loop | 1,839,146 | 1,825,113 | +0.8%, ok |
    | string_build | 505,454 | 468,387 | +7.9%, over |
    | map_workload | 553,006 | 499,508 | +10.7%, over |
    | records | 1,863,389 | 1,756,300 | +6.1%, over |
    | string_compare | 1,420,724 | 1,374,717 | +3.3%, ok |
    | string_find | 1,173,034 | 1,144,568 | +2.5%, ok |
    | string_map | 580,088 | 538,926 | +7.6%, over |

    - Cause: start-up, not `Map`'s calls. Linking any file through `cli_run_links()` costs about 27–31 ms per run, because `run.rs` runs the module loader again for it (`PROP-013`). `fib` uses no map and is over.
    - `PROP-007` option F (faster calls) would not remove that cost, so it was not sized.
    - Tried and reverted: linking `Map` through the implicit prelude (`stdlib/core/prelude.pilot`, like `List`), which has no start-up cost. PilIR then runs `Map`'s Pilot methods, so `get` returns a real `T?` record, and code that uses `m.get(k)` as a plain value fails on `--ir` and `--emit c` (`collections/containers`, `text/str_kernels`, `types/attr_coverage`, `runtime/__engine_wrappers`, `runtime/const_values`). The checker accepts `T?` where a `T` is expected (`f() + 1` with `f() -> i64?` passes `pilot check`); the seed hides it because its options are plain values. That gap is its own work.
    - Owner decision (2026-11-15): "Rust proposal, one pass" (`PROP-013`). The `Map` slice resumes when `PROP-013` is built and C2 passes.
  - **C2 after `PROP-013` (2026-11-15): PASSES** (all 8 benchmarks at 0.89x–0.99x of the baseline; numbers under `PROP-013`, "As built").
  - **As built (2026-11-15). 0 Rust in this slice.** Seed suite 542 passed, 0 failed, 14 known bugs reproduced (539 before, +3 new programs). `pilot gate` passes. Where it differs from the approved list:
    - **Kept: the name-wide `bind_native_method(get|set|has|keys|remove)` bindings.** Deleting them failed every test: the module loader (`LOADER` in `run.rs`) and other Pilot tools call these methods on receivers the seed cannot type (a struct field, a call's result), and their file lists do not link `map.pilot`. A comment in `native_calls.pilot` says so.
    - **Kept: `bind_native_member(Map, "[]" | "[]=")` and `Map`'s `in`, `for_seq`, `for_len`, `for_get` and constructor bindings.** Deleting the indexers turned four `--ir` cases into seams: PilIR does not lower a type's indexer to its method yet. `in` has no membership protocol; the constructor is C1.
    - **Split: the lowercase `map<K, V>` view moved to `stdlib/core/primitives/map_view.pilot`.** Its Pilot methods ran on engine maps once `map.pilot` was linked (`control/if_some` failed).
    - **`GATE021`: a new check 6** (`airlock_native_owners()`, `ab_check_native_owners` in `selfhost/gate/airlock_abi.pilot`) rather than a change to `transitional_native_bindings()`, since no binding could leave it (above).
    - **Fixed on the way:**
      - `scan_engine_bindings` (`selfhost/check/program.pilot`) took bind-hook rows for engine ops, so PilIR's `??` and `if some` on a declared `T?` called an op named `intrinsic_name`. This also cleared three older IR seams.
      - The C, JS and Python runtimes' `map_remove` returned the removed value instead of `true`/`false` (C printed `2 null` where the seed printed `true false`).
    - **Not done:**
      - On PilIR, `m.get(k)` still goes through the native binding, whose plain value PilIR reads as a tagged `T?`. So `get` is left out of `collections/map_owner_methods` (a comment says why). It cannot be a known bug, because the seed is right.
      - Making PilIR run `Map`'s own methods (linking `map.pilot` there) breaks code that uses `m.get(k)` as a plain value, which the checker accepts (see "C2 result" above).
      - `collections/map_declared_property` (`count`, `clear`) stops at the seam `ir.lower_construct` on `--ir` and `--emit c`.
      - Bucket B (the raw `__engine("map_*")` sites) and the other types (`List`, `str`) are not started.
  - **Rust:** none in this slice, so net 0. Rust shrinks only when a whole `bind_native_*` kind has no bindings left, or when `heap.rs` goes after F6.
- **Not in scope:** `Reflect`/`PhysicsRegistry` lives in `crates/pilot-analysis` and retires in `PROP-004`. `PROP-004` must name the pure-Pilot successor for layout and symbol reflection (`AGENTS.md` §6, `pilot inspect`), and register a `reflect.type_meta` seam plus a `::Spec` wiring row for anything not built.

---

## `PROP-007` — Implement `[Inline]` (Required Before Owners Replace Native Bindings)

- **Status:** **`DRAFT`**. The PilIR part is pure Pilot and may start. The seed part changes Rust and needs an itemized file list and owner approval.
- **Steps 1–3 as built (2026-11-15, uncommitted):**
  - `selfhost/ir/pass_inline.pilot` (`InlinePass`) runs first in `default_passes()`, so `pilot ir run`, `pilot test --ir` and `--emit c` all inline. Lowering sets `IrFunction.inline` from the declaration: `[Inline]` is a hint, `[EngineOp]` is forced. `pilot ir print` writes it as `inline` / `inline_forced` after the function's name, and the text parser reads it back (round-trip holds).
  - Each inlined call cuts its block; the rest moves to a continuation block whose parameter is the call's own result value, the callee's blocks are copied with values and block ids renumbered, and each `return` jumps to the continuation (`undef` when the callee returns nothing but the result is named). The entry takes the arguments directly unless something jumps back to it.
  - A function that reaches itself through inline calls is not inlined; for a forced one that is `INLINE002` (registered in `selfhost/diagnostics/errors.pilot`). Real owners cannot recurse (`GATE021` check 1), so there is no conformance program for it: the seed does not inline and would not report it, which would make the engines disagree.
  - `[Inline]` on its own public method is still the author's choice; the public caller of an owner is **not** forced automatically.
  - Step 3: `--emit c` inherits it from PilIR; no `always_inline` backstop was added.
  - Acceptance: `tests/conformance/ir/inline_calls` (hand-worked output; `.ir.inlined` names 8 functions). `pilot test --ir` checks no `call_direct` to them remains in `pilot ir opt`; with the pass unregistered all 8 fail. `INLINE001` now says the seed is the engine that does not inline.
  - **Not built (`SPEC` rows in `wiring.pilot`):** trace and debugger provenance (an error inside inlined code is reported in the caller's frame), `pilot inspect` and the PilotDb dependency, and step 4 on the seed. The `map[k] = v` acceptance case waits for `PROP-002b`, which makes `Map` an owner.
  - **Not run:** the full `pilot test` / `--ir` / `--emit c` corpus and `pilot gate --full` (paused by the owner on 2026-11-15).
- **Problem:** `[Inline]` is declared (`stdlib/core/core.pilot:349`, registered at `:366`), but nothing consumes its hook `set_inline_hint` in `crates/` or `selfhost/`. Every engine makes a real call. Under "one op, one owner", `map[k] = v` becomes three calls (`[]=` → `set` → `set_internal`) unless they are inlined.
- **Meaning:**
  - At each call site, the callee's body replaces the call, with arguments bound to parameters and the receiver bound to `self`.
  - It runs after type checking and the `GATE021` source checks, so the source still shows one caller.
  - It runs before Perceus drop insertion.
  - Evaluation order, `defer`, contracts and drops are preserved.
- **Steps:**
  1. **Now:** `[Inline]` gives the compile-time warning `INLINE001` instead of being ignored (not a seam; see A1). Add a `wiring.pilot` row at `SPEC`.
  2. **PilIR pass (Pilot, `selfhost/ir/`):**
     - substitute callee blocks: the entry block takes the arguments, and each `return` becomes a jump to a continuation block
     - forced mode for `[EngineOp]` owners and their single public caller: a compile error when inlining is impossible (recursion, address taken)
     - provenance recorded for stack traces, `SEAM001` traces and the debugger
     - inlining shown in `pilot inspect` (`AGENTS.md` §6)
     - a PilotDb dependency from each caller to each inlined callee
  3. **`--emit c`:** follows from PilIR, with `always_inline` on owner methods as a backstop.
  4. **Seed lowering (Rust, needs approval):** in `crates/pilot-vm/src/lower_call.rs`, lower a single-expression inline callee in place. The aim is to pay for it by deleting the native-binding dispatch for owned ops (`lower_native.rs`, `lower_members.rs`). The net LOC is measured, not assumed.
     - **Temporary by design:** `PROP-005` deletes every `lower_*.rs` file. This step exists only so the seed passes the C2 speed gate in the meantime. Keep it to the smallest change that does that.
     - `PROP-005` must lower `.pilc` from PilIR **after** the step-2 inlining pass, so inlining survives the cutover without being rewritten.
     - **Step 4 itemized (2026-11-15). Owner chose option M ("Option M").**
       - **What it costs not to inline on the seed (measured).** A probe ran `map_workload` (200,000 `set` and 200,000 reads) both directly and through two extra call levels, which is the shape `map[k] = v` takes after `PROP-002b` (`[]=` → `set` → `set_internal`). Fastest of 5 runs each, release seed: direct 501,650–501,934 µs, two levels 524,004–533,402 µs. That is +22–31 ms, or **+4.5–6.3% of the total time**, where C2's line is 5%. Most of the total is the about 420 ms start-up; the loop itself got about 30% slower. A tighter loop (integer keys, no `f"..."`) would lose more than that.
       - **How the seed would know what to inline.** Today nothing tells it. `[Inline]` calls the hook `set_inline_hint(target_fn, mode)`, which no Rust consumes. `[EngineOp]` (`EngineOpAttr` in `stdlib/core/host_raw.pilot`) has no hook at all; only `GATE021` and `selfhost/ir/pass_inline.pilot` read it. `ingest_decl_attrs` passes a bare declaration name, so a method would need its owner-qualified name (`Map.set`).
       - **Option F — build step 4 now.**
         - `crates/pilot-parser/src/tables.rs`: an `inline_fns` table (name → mode) filled by the `set_inline_hint` hook. About +8.
         - `crates/pilot-parser/src/attr_hooks.rs`: pass a method's owner-qualified name to its attribute hooks. About +5.
         - `crates/pilot-vm/src/lower_call.rs`: at a call to a function in `inline_fns` whose body is a single expression (`=> expr`) with no contract, `defer` or `where`, bind the parameters (and `self`) to the argument registers and compile the body in place; a depth guard stops recursion. About +45.
         - Pilot: `EngineOpAttr` gains a comptime hook that calls `set_inline_hint(target, forced)`; each owner's public method gets `[Inline]` where the author wants it.
         - Net about **+55 to +65 Rust**, with nothing deleted in this step. The native-binding dispatch it was meant to pay for (`lower_native.rs`, `lower_members.rs`) can only be deleted once `PROP-002b` has moved each type's callers to its owner, so that saving comes later and is not measured yet. `PROP-005` deletes `lower_call.rs` in the end.
         - Not covered: operators that the seed lowers through `lower_members.rs` dispatchers (`m[k]`, `m[k] = v`) unless those dispatchers also go through the new path (more Rust; to be itemized if needed).
       - **Option M — measure first (recommended; no Rust now).**
         - Itemize `PROP-002b` for one type first (`Map`, which `map_workload` and `string_map` exercise), move its callers to its owner methods without seed inlining, and run `target/release/pilot run benchmarks/native/record.pilot -- --check`.
         - If C2 passes, that type needs no step 4, and its native bindings and dispatch are deleted in the same change (net Rust down).
         - If C2 fails, stop, record the numbers here, and build option F, sized to what the failing benchmarks need.
         - This reverses `PROP-002b`'s "after `PROP-007`" ordering for that one type, and it is decided by the C2 check, not by the estimate above.
- **Does not solve** (recorded so nobody expects it):
  - calls through interfaces, stored method values or closures (needs devirtualization; deferred, with a `SPEC` row)
  - the engine op's own dispatch cost
  - deliberate work in the public method (contracts, validation)
  - string marshalling for OS ops
  - enforcing the owner rule, which is `GATE021`'s job
- **Acceptance:**
  - Conformance programs show `map[k] = v` lowers to one intrinsic on `--ir` and `--emit c` (checked through `pilot inspect` output).
  - Forced-inline failure cases give their error codes.
  - The seed bench meets the C2 threshold.

---

## `PROP-008` — Every Operator Declared by an Attribute on Its Interface or `syntax` Rule, and Comptime User-Defined Operators

- **Status:** **`DRAFT — OWNER DECISIONS §8.2–8.4 MADE 2026-11-15 (§9); STEP 1 RUST FILE LIST APPROVED 2026-11-15, IN PROGRESS`**. §8.1 was settled in Amendment A1. Step 1 is pure Pilot plus one hook. Steps 2 and 3 change Rust and need an itemized file list and owner approval.
- **Origin:** owner direction 2026-11-14: "a comptime way for the user to define their own operators to add to Grammar when it builds", then "maybe all operators should be defined this way so hardcoded values don't happen." Recorded for review as P11 in [`conversations.md`](file:///home/moony/Documents/dev/projects/Pilot/conversations.md).
- **Behaviour change:** none for existing programs. Every current operator keeps its spelling, precedence order, associativity and result on all three engines. Conformance counts must not drop.

### 1. Problem (measured on `f4` at `7920302`)

Operator facts are written down in at least five places, and the places disagree:

| Where | What it hardcodes |
| :--- | :--- |
| `stdlib/core/core.pilot:272-289` (`Cfg.Ops`) | 18 numeric levels (`prec_or: 15`, `prec_and: 20`, …), used by 48 `register_pratt_op` calls in the same file |
| `stdlib/core/syntax/expr_sugar.pilot:13-21` | its own raw numbers: `||` at 12, `&&` at 14 (versus 15 and 20 in `Cfg.Ops`); whichever runs last wins |
| `stdlib/db/sql_syntax.pilot:31-33`, `stdlib/asyncio/select_syntax.pilot:27-28` | more raw numbers (`union` 12, `<-` 5 and 80) |
| `selfhost/parser.pilot:11-19` (`sh_get_precedence`) | a third table: `||` 5, `&&` 10, `==` 20, `+` 30, `*` 40, everything else 0 |
| `crates/pilot-parser/src/tables.rs:177-186` | `MEMBERSHIP_BP = 25`, and `map_binop_sym`: 42 symbol → `BinaryOp` arms (`"+" => Add`, `"<-" => Send`, `"+|" => SatAdd`, `"~>" => Lens`, …) |
| `crates/pilot-lexer/src/lexer.rs` and `token.rs` | about 52 byte-pattern arms for multi-character symbols, and 119 `TokenKind` variants, many of them operators (`JsonContained` for `<@`) |

So the same expression can parse differently on the seed and the self-hosted parser, and adding an operator means touching Rust.

### 2. The Boundary: What Stays Built In

The operator files must be readable before any operator exists. The lexer and parser therefore keep only **structure**:
- delimiters and separators: `( ) [ ] { } , ; : :: . => =`
- string, character, number and comment lexing
- identifiers, and the 14 seed tokens
- the **parse shapes** in §4 (engine mechanics, not symbols)

Every **operator** is declared in Pilot by an attribute (§3): arithmetic, comparison, logical, bitwise, `??`, `?.`, `..`, `..=`, `..<`, `|>`, `<|`, `<-`, `<=>`, `->`, `->>`, `~>`, the saturating/wrapping/checked families, the call-quote suffixes, the word operators (`as`, `in`, `is`, `and`, `or`, `not`, `union`, `intersect`, `except`), and user operators.

### 3. Operators Are Declared Where They Are Defined (`stdlib/core/operators/`)

There is no hand-written operator table. Each operator is declared **once**, by an attribute on the declaration that already defines what it means. At comptime each attribute adds one entry to the grammar; the collected entries are the table.

**Two kinds of operator, two homes:**

| Kind | What `a OP b` becomes | Where the attribute goes | Examples |
| :--- | :--- | :--- | :--- |
| **Method-backed** | a call of the left operand's `op` method | the protocol **interface** that declares the `op` (today in `stdlib/core/operator.pilot`, 35 interfaces) | `+ - * / % ** == < <=> & \| ^ << >>`, the `+\|`/`+%`/`+?` families, `[]`, `in` |
| **Rewrite-backed** | other code, built by a `syntax` rule with a `quote` template | the **`syntax` rule** | `&& \|\|` (they must skip the right side), `??`, `?.`, postfix `?`, `\|>`, `<\|`, `->`, `:=`, cascades, ranges, the call-quote suffixes |

Method-backed operators need no `quote`: a template that turns `a + b` into a call of `op +` would add a step and change nothing. Every type that supports `+` implements the interface; the grammar entry belongs to the interface, because implementations are many and could disagree.

**Layout** (one file per family, not per operator: about 60 operators, a few lines each):

```
stdlib/core/operators/
  levels.pilot      // enum OpLevel and enum Assoc, declared once
  shapes.pilot      // enum ParseShape; the [Infix], [InfixWord], [Prefix], [Shaped] attributes
  arithmetic.pilot  // IAdd/ISub/IMul/IDiv/IRem/IPow and the sat/wrap/checked families
  compare.pilot     // IEquatable, IComparable, ICompareOperator
  bitwise.pilot
  logic.pilot       // && || ! with their readable pairs and/or/not
  range.pilot       // .. ..= ..< and the open forms
  absent.pilot      // ?? ?. and postfix ? (try)
  flow.pilot        // |> <| -> :=, cascades
  quoting.pilot     // the six call-quote suffixes (replaces syntax/call_quote.pilot)
```

The interfaces in `stdlib/core/operator.pilot` **move** into these files; they are not copied (`imp_type.md` rule 9). `operator.pilot` is deleted in the same change.

**Levels are named and ordered, never numbered** (`levels.pilot`):

```pilot
/// Binding levels, loosest first. Binding powers are computed from this
/// order after user operators merge; no operator states a number.
pub enum OpLevel : u8 {
    Assign, Coalesce, Pipe, Or, And, BitOr, BitXor, BitAnd,
    Range, Equality, Compare, Shift, Add, Multiply, Power,
    Cast, Unary, Postfix,
}

pub enum Assoc : u8 { Left, Right, None, }
```

**The attributes** (`shapes.pilot`) follow the pattern `[Operator]` and `[Inline]` use today (`stdlib/core/core.pilot:340-366`): a `comptime fn` registered with `register_bracket_attribute`. Each one calls the single new hook `grammar_entry`. `[Infix]` replaces today's `[Operator(target, fixity, prec, assoc)]`, which takes a number.

```pilot
comptime fn Infix(target, symbol, level, assoc) =>
    grammar_entry(target, symbol, ::Binary, level, assoc, "");
comptime fn InfixWord(target, symbol, word, level, assoc) =>        // `&&` ↔ `and`
    grammar_entry(target, symbol, ::Binary, level, assoc, word);
comptime fn Prefix(target, symbol, word) =>
    grammar_entry(target, symbol, ::Prefix, ::Unary, ::Right, word);
comptime fn Shaped(target, symbol, shape, level, assoc) =>
    grammar_entry(target, symbol, shape, level, assoc, "");
```

**A method-backed operator** (`arithmetic.pilot`):

```pilot
pub interface {
    [Infix("+", ::Add, ::Left)]
    IAddOperator<TRhs: IAnyValue, TOut: IAnyValue> : IAnyValue {
        pub pure op + (rhs: TRhs) -> TOut;
    }

    [Infix("**", ::Power, ::Right)]
    IPowOperator<TRhs: IAnyValue, TOut: IAnyValue> : IAnyValue {
        pub pure op ** (rhs: TRhs) -> TOut;
    }
}
```

**The primitive implements it with its one owner** (Amendment A1). `bool` already does this (`stdlib/core/primitives/bool.pilot:16`, `op == … => __ssa(Cfg.Ssa.eq, self, rhs)`):

```pilot
pub partial struct i64 : IAddOperator<i64, i64> {
    [EngineOp]
    priv pure fn add_internal(rhs: i64) -> i64 => __ssa(Cfg.Ssa.add, self, rhs);
    pub pure op + (rhs: i64) -> i64 => .add_internal(rhs);
}
```

**Rewrite-backed operators** (`logic.pilot`, `absent.pilot`, `flow.pilot`). Templates use only seed words, calls and structure, never the operators being declared, so load order cannot break them:

```pilot
pub syntax {
    // `&&` cannot be a method: a method evaluates both sides first.
    [InfixWord("&&", "and", ::And, ::Left)]
    and_also(lhs, rhs) => quote { if $lhs { $rhs } else { false } };

    [InfixWord("||", "or", ::Or, ::Left)]
    or_else(lhs, rhs) => quote { if $lhs { true } else { $rhs } };

    [Shaped("??", ::Safe, ::Coalesce, ::Right)]
    coalesce(lhs, rhs) => quote { $lhs.value_or_else(() => $rhs) };

    [Shaped("?", ::Postfix, ::Postfix, ::None)]
    try_unwrap(x) => quote { if $x.is_failure() { return $x.failure(); } else { $x.value() } };

    [Infix("|>", ::Pipe, ::Left)]
    pipe(lhs, rhs) => quote { $rhs($lhs) };
}
```

**A shape with missing operands** (`range.pilot`): the `::OpenRange` shape passes an absent marker, and the rule has one arm per form.

```pilot
pub syntax range_op {
    [Shaped("..", ::OpenRange, ::Range, ::None)]
    (lhs, rhs) => quote { range($lhs, $rhs) };
    (lhs, _)   => quote { from($lhs) };
    (_, rhs)   => quote { until($rhs) };
    (_, _)     => quote { full() };
}
```

**Symbol and readable word together** (`quoting.pilot`, replacing `stdlib/core/syntax/call_quote.pilot`):

```pilot
pub syntax {
    [Shaped("!",  ::AdjacentCall, ::Postfix, ::None)] call(callee, args)   => quote { CallExpr($callee, $args) };
    [Shaped("!!", ::AdjacentCall, ::Postfix, ::None)] splice(callee, args) => quote { splice_ast($callee, $args) };
    // !& frame, !^ sig, !@ grad, !? plan
}
```

The names inside templates (`value_or_else`, `is_failure`, `CallExpr`, `splice_ast`) stand for whatever `ITryOperator` and the call-quote types expose when the files are written; they are not new API.

**Handing the grammar to the parsers.** After the operator files load, one hook passes the collected entries to the seed:

```pilot
comptime { register_grammar(grammar_entries()); }
```

`selfhost/parser.pilot` reads the same `grammar_entries()` as data, which deletes `sh_get_precedence`.

### 4. Parse Shapes (Closed in Rust, Append-Only Under B+)

Not every operator is a plain binary operator. Each entry picks exactly one shape. The shapes are the only operator logic left in the parser:

| Shape | Meaning | Examples |
| :--- | :--- | :--- |
| `::Binary` | Pratt infix | `+` `==` `&&` `|>` `union` |
| `::Prefix` | prefix unary | `-x` `!x` `not x` `<-ch` |
| `::Postfix` | postfix unary | `x?` (try) |
| `::TypeRight` | the right operand is a type, not an expression | `x as T`, `x is T` |
| `::OpenRange` | either operand may be missing | `a..b` `..b` `a..` `..` |
| `::Membership` | infix with a diagnostic when the right side is missing | `x in xs` |
| `::Assign` | assignment and compound assignment; target must be a place | `=` `+=` `:=` |
| `::Safe` | short-circuits on absent | `?.` `??` `?..` |
| `::Cascade` | applies to the root receiver and returns it | `?..` always; `..` only when the lookahead rule below says so |
| `::AdjacentCall` | symbol must touch the callee and the `(` with no whitespace | `f!(…)` `f!!(…)` `f!&(…)` `f!^(…)` `f!@(…)` `f!?(…)` |

Rules the shapes carry, so the table never needs them:
- **Same symbol, two positions** (`*x` unpack versus `a * b`, `-x` versus `a - b`) is two entries with different shapes. Position decides; that is how the parser works today.
- **Whitespace-sensitive forms** (PrefixDot versus InfixDot, `AdjacentCall`) are flags of the shape, not of the symbol.
- **`..` is both a range and a cascade.** Position does not decide it (`a..b` versus `node..set(x)`). The `::Cascade` shape keeps today's lookahead rule (`is_cascade_dotdot`, `crates/pilot-parser/src/expr.rs:351`): the left side is not a number literal, and after `..` comes a name (or `name.name…`) followed by a call or an assignment. Otherwise `..` is `::OpenRange`. The rule moves with the shape; step 1 adds conformance programs for both readings and for the ambiguous `a..b(c)`, whose intended reading the owner decides (§8).
- **Generic `<`** is not an operator entry; the type-argument rule is structure and runs first.

Adding a shape is a Rust change and an append under B+. Adding an operator that uses an existing shape needs no Rust change.

### 5. User-Defined Operators (Comptime)

A user declares an operator the same way the core does: an attribute on an interface (method-backed) or on a `syntax` rule (rewrite-backed). Only the placement differs, because a user names neighbours instead of a built-in level:

```pilot
pkg My.Geometry;

[Infix("<.>", tighter_than: ::Add, looser_than: ::Multiply, assoc: ::Left)]
pub interface IDotOperator<TOut: IAnyValue> : IAnyValue {
    pub pure op <.> (rhs: Self) -> TOut;
}

pub struct Vec3 : IValue, IDotOperator<f64> {
    pub x: f64; pub y: f64; pub z: f64;
    pub pure op <.> (rhs: Vec3) -> f64 => .x * rhs.x + .y * rhs.y + .z * rhs.z;
}
```

- **Placement is relative.** `tighter_than` / `looser_than` name a level or another operator. The merge inserts a new level between them. A user never writes a number.
- **Scope is by import.** An operator is active only in files that `use` the declaring module, so two libraries can never fight over global precedence.

**Safety rules** (each a diagnostic, never silence, `AGENTS.md` §11 rule 2):
- `GRAM001`: a user entry redefines a built-in symbol or readable word.
- `GRAM002`: a new symbol changes how existing text lexes. The check re-lexes the importing file with and without the new symbol, and fails if any other token splits differently.
- `GRAM003`: two imported entries claim the same symbol and position.
- `GRAM004`: the `tighter_than` / `looser_than` constraints conflict (a cycle).
- `GRAM005`: the symbol uses structural characters (§2) or is a seed token.
- `GRAM006`: the declaration does not match the shape's arity (`op` parameters or `syntax` arms).
- `GRAM007`: an operator attribute on a declaration that cannot define one (not an interface with a matching `op`, and not a `syntax` rule).

### 6. Steps

1. **Operator files and one hook:**
   - **Pilot:** write `stdlib/core/operators/` with every current operator; move the interfaces out of `operator.pilot` and delete it; replace `syntax/call_quote.pilot`; replace `[Operator]` with `[Infix]`/`[InfixWord]`/`[Prefix]`/`[Shaped]`; delete the `Cfg.Ops` numbers, the raw numbers in `expr_sugar`/`sql_syntax`/`select_syntax`, and `sh_get_precedence`.
   - **Rust (needs an itemized file list and approval):** the seed's ingest must read an attribute on an item inside a `pub interface { … }` block and on an arm of a `pub syntax { … }` block (today it reads attributes on top-level `syntax` rules and functions); a `syntax` arm keyed on absent operands for `::OpenRange`; and the hooks `grammar_entry` / `register_grammar` replacing `register_pratt_op`.
   - The lexer still knows the built-in symbols.
   - Acceptance: one conformance program per level boundary, with the expected grouping worked by hand from the level order and stated in its comment; a test that the seed and the Pilot parser group the same expressions identically; one `err_*` program per `GRAM` code that step 1 can reach (`GRAM005`, `GRAM006`, `GRAM007`). The current `||`/`&&` disagreement must be resolved by the owner (§8) before these are written.
2. **Table-driven lexing (mostly Pilot):**
   - The full design (longest match over the declared symbols, user operators, `GRAM001`–`GRAM004`) is built in the **Pilot** lexer under `selfhost/`, because `PROP-006` deletes `crates/pilot-lexer` and `crates/pilot-parser`.
   - The Rust seed gets only the smallest change that stops it hardcoding operators: one generic operator token, plus a longest-match loop over the symbols `register_grammar` gave it. This replaces the operator byte-pattern arms and most operator `TokenKind` variants. It needs an itemized file list and approval, and it must be a net deletion.
   - Until `PROP-006`, user operators work on the seed only if this small change lands; otherwise they stop at a registered seam `grammar.user_ops_seed` on the seed.
3. **Method-backed operators lower to owner methods (needs `PROP-007`):** delete `map_binop_sym` and native `BinaryOp` dispatch for operators that have an owner. Gated on the C2 seed speed baseline. Integer `+` must lower to exactly what it lowers to now on the seed, PilIR and C.

### 7. What This Does Not Solve

- Error messages for operators become generic per shape. Good messages for common mistakes (`=` versus `==`) need entries in the diagnostic catalog.
- A user operator in an imported library can still make code hard to read. Scope by import limits the reach; it does not judge taste.
- Operators inside `quote { … }` and `syntax` rules use the grammar of the file that **defines** the rule, not the file that expands it. That is a hygiene rule to write as a conformance program in step 2.
- **Load order inside `operators/`:** a template may use only operators declared earlier in load order. The core templates avoid operators entirely (§3); a later template that needs one must be loaded after it, and step 1 adds a check that fails when a template names an operator not yet declared.
- `PROP-003` moves `attr_hooks.rs` to Pilot. The moved code must keep comptime attribute expansion for the operator attributes.

### 8. Decisions Left to the Owner

1. **Fold into Amendment A1** (recommended): freeze `grammar_entry(...)`, `register_grammar(entries)` and the `ParseShape` list in `bridge_lang.pilot`, instead of freezing `register_pratt_op(sym, fixity, bp, assoc, bin_op)`, which bakes integer binding powers and native op codes into a forever-frozen signature. The in-progress work already lists `register_pratt_op` in `selfhost/gate/airlock_abi.pilot:107`.
2. **The `||` / `&&` precedence today:** `Cfg.Ops` says `||` 15 and `&&` 20; `expr_sugar.pilot` says 12 and 14; `selfhost/parser.pilot` says 5 and 10. The order is the same in all three (`||` looser than `&&`), but their position relative to `??`, `|`, `..` and `->` differs. The owner picks the intended order once, in `OpLevel`.
3. **Readable word operators** (`and`, `or`, `not`, `is`) are today receptors (`bind_receptor`), not operators. Declare them as readable pairs with `[InfixWord]`/`[Prefix]` (recommended), or keep them as receptors.
4. **`a..b(c)`:** a range ending in a call, or a cascade step calling `b`? Today's lookahead reads it as a cascade unless `a` is a number literal. Keep that (recommended, since cascades are statements), or require spaces around a range `..`.

### 9. Owner Decisions (2026-11-15) and the Step 1 Rust File List

**Decisions.** §8.1 was settled in Amendment A1: `grammar_entry` and `register_grammar` are frozen, `register_pratt_op` is transitional. The owner chose the following on 2026-11-15:
- **§8.2, precedence:** the `Cfg.Ops` order in `stdlib/core/core.pilot` becomes the single `OpLevel` list, with `->` / `->>` placed just above `&&`. The `expr_sugar.pilot` numbers, the `sql_syntax`/`select_syntax` numbers, `sh_get_precedence` and the hardcoded word powers in `crates/pilot-parser/src/expr.rs:107` are deleted.
  - *Errata to the question as asked:* its preview listed `**` looser than the prefix operators. `Cfg.Ops` has it the other way round (prefix 80 < `**` 88, so `-x ** 2` is `-(x ** 2)`). The decision was "the `Cfg.Ops` order", so its numbers govern; the owner should confirm that if the preview was what they meant.
  - **No behaviour change.** `expr_sugar.pilot` loads after `core.pilot` (`BOOTSTRAP_SOURCES`), so the seed today uses `??` 10 < `||` 12 < `&&` 14 < `->` 20 < `|` 30. The chosen order keeps every one of those relative positions.
  - `OpLevel` (replacing the §3 sketch, loosest first, each line listing today's `Cfg.Ops` value or the file that sets it):

    | Level | Operators | Today |
    | :--- | :--- | :--- |
    | `Assign` | `:=`, `:=:`, infix `<-` | 5 |
    | `Pipe` | `\|>`, `<\|` | 8 (`expr_sugar`) |
    | `Coalesce` | `??` | 10 |
    | `Or` | `\|\|`, `or` | 12 effective (`Cfg.Ops` 15) |
    | `And` | `&&`, `and` | 14 effective (`Cfg.Ops` 20) |
    | `Arrow` | `->`, `->>` | 20 (`expr_sugar`) |
    | `Membership` | `in` | 25 (`MEMBERSHIP_BP`) |
    | `BitOr` / `BitXor` / `BitAnd` | `\|`, `^`, `&` | 30 / 32 / 35 |
    | `Range` | `..`, `..<`, `..=`, `>..<`, prefix `>..` | 40 |
    | `Equality` | `==`, `!=` | 45 |
    | `Compare` | `<`, `<=`, `>`, `>=`, `<=>` | 50 |
    | `Shift` | `<<`, `>>` | 55 |
    | `Add` | `+`, `-`, `+\|`, `-\|`, `+%`, `-%`, `+?`, `-?` | 60 |
    | `Multiply` | `*`, `/`, `%`, `//`, `%%`, `*\|`, `*%`, `*?` | 70 |
    | `Cast` | `as`, `is` | 75 |
    | `Unary` | prefix `-`, `!`, `~`, `not`, prefix `<-` | 80 |
    | `Lens` | `~>` | 85 |
    | `Power` | `**` | 88 |
    | `Splat` | prefix `*`, `**`, `*..` | 90 |
    | `Postfix` | postfix `?`, `!` | 95 |

    `union`, `intersect` and `except` (`sql_syntax.pilot`, 12 today) are declared by that file at `Or`. `<-` in `select_syntax.pilot` (5 and 80) uses `Assign` and `Unary`.
- **§8.3, word operators:** `and`, `or`, `not` and `is` are declared as readable pairs (`[InfixWord]` / `[Prefix]`) at their symbol's level, lowering to the same node. Their `bind_receptor` lines in `expr_sugar.pilot` go.
- **§8.4, `a..b(c)`:** today's lookahead is kept (a cascade step unless `a` is a number literal). Step 1 adds conformance programs for both readings.

**Step 1 Rust file list — APPROVED BY OWNER 2026-11-15.** With the approval the owner set the rule for step 1: every non-structural operator (for example `++`, `||`, `&&`) is defined, with its precedence, by an attribute on a `syntax` rule with a `quote` template or on an `op` declaration. Only the 13 structural tokens `( ) [ ] { } , ; : :: . => =` may be defined however the parser needs. For method-backed operators the attribute sits on the `op` declaration itself (the symbol is the `op`'s own), not on the surrounding interface as §3 sketched. Strictly these files, in `crates/pilot-parser` only; the lexer is untouched until step 2 and `map_binop_sym` stays until step 3.

| File | Change |
| :--- | :--- |
| `crates/pilot-parser/src/tables.rs` | Replace the `register_pratt_op` arm of `ingest_node` with `grammar_entry(target, symbol, shape, level, assoc, word)`: the level is a variant of `OpLevel`, whose ordinal the seed reads from the ingested `enum OpLevel` declaration; the binding power is computed from that ordinal, so no number appears in Pilot. Add `register_grammar` (closes the table). Ingest `InterfaceDecl` and `syntax` declarations, not only `FnDecl`/`StructDecl`. Delete `MEMBERSHIP_BP`. Add the `stdlib/core/operators/*.pilot` files to `BOOTSTRAP_SOURCES` and remove `stdlib/core/operator.pilot`. Keep `register_pratt_op` readable until every caller moves, then delete its arm. |
| `crates/pilot-parser/src/attr_hooks.rs` | `ingest_decl_attrs` runs any registered attribute hook (`grammar_entry`), not only `set_register_layout`, with the target being the interface, the `syntax` rule, or the arm it sits on. |
| `crates/pilot-parser/src/expr.rs` | Delete the hardcoded word powers at `:107` (`or` 12, `and` 14, others 45). Word operators, `in` (`::Membership`) and `as`/`is` (`::TypeRight`) take their level and shape from the table. `is_cascade_dotdot` stays as the `::Cascade` rule. |
| `crates/pilot-parser/src/decl.rs` | Keep the attributes written on an item inside `pub interface { … }` and on an arm inside `pub syntax { … }`, attached to that item or arm, so the ingest above can see them. |
| `crates/pilot-parser/src/rule_arms.rs` | A `syntax` arm may write `_` for an absent operand (`::OpenRange`: `(lhs, _)`, `(_, rhs)`, `(_, _)`); an arm fits only when its absent positions match. |
| `crates/pilot-parser/tests/milestone51_homoiconic_syntax_and_contraction.rs` | Update the `expr_sugar.pilot` code markers that move to `stdlib/core/operators/`. |

Every touched file is reformatted to standard `rustfmt` layout in the same change, which unpacks the multi-statement lines still present in `tables.rs`, `expr.rs` and `decl.rs` (see the note below). Net Rust LOC is measured before and after; the step is expected to be near zero, with the real deletions in steps 2 and 3. After the change: `cargo test -p pilot-parser --test damaged_source -- --include-ignored`.

**Pilot files in the same change (no approval needed, listed for completeness):** new `stdlib/core/operators/{levels,shapes,arithmetic,compare,bitwise,logic,range,absent,flow,quoting}.pilot`; delete `stdlib/core/operator.pilot` and `stdlib/core/syntax/call_quote.pilot`; remove `Cfg.Ops` and the 48 `register_pratt_op` calls from `core.pilot`, the numbers from `expr_sugar.pilot`, `stdlib/db/sql_syntax.pilot` and `stdlib/asyncio/select_syntax.pilot`; replace `sh_get_precedence` in `selfhost/parser.pilot` with `grammar_entries()`; add `grammar_entry` / `register_grammar` to `stdlib/core/bridge_lang.pilot` and append them to `airlock_lang_hooks()` in `selfhost/gate/airlock_abi.pilot` (an append under B+, approved through this entry); `GRAM005`–`GRAM007` in `selfhost/diagnostics/errors.pilot`; ledgers, matrices and maps.

**Acceptance (written before the code, per §6 step 1):** one conformance program per adjacent level boundary in the table above, each with its grouping worked by hand in a comment; one program showing `a and b` and `a && b` give the same result and the same grouping; the `a..b(c)` / `0..n(c)` pair; and `err_gram005`/`006`/`007`.

**Amendment 008-A (2026-11-15) — files found missing while building step 1. APPROVED BY OWNER 2026-11-15.**

Found while mapping the seed before writing code:

1. **The bootstrap needs two passes.** The operator attributes sit on `op` declarations, so their files need `op`, `pure`, `enum` and the scope headers, which `core.pilot`, `syntax/module_ext.pilot` and others register first. Those earlier files already use operators: `visibility.pilot` and `module_ext.pilot` bodies use `==` and `||`, and the `while` rule in `core.pilot` has `!` in its template. Read once, in order, they would parse before any operator exists. No ordering of the files fixes this, and re-reading a single file does not either.
   - **Change:** `bootstrap_tables()` and `bootstrap_diagnostics()` in `crates/pilot-parser/src/lib.rs` read the sources twice. Pass 1 only collects the grammar entries. Pass 2 starts from empty tables holding just that grammar and builds everything else.
   - **Size:** about 10 lines, plus `ParserTables::with_grammar` in `tables.rs` (already approved).
   - **Cost:** a second bootstrap read at every process start. Compile speed is ungated (2026-11-10).
2. **Two hardcoded binding powers outside the approved files.** Both use today's numbers, which stop meaning anything once powers come from level order.
   - `crates/pilot-parser/src/stmt.rs:30` uses `30` and `:58` uses `40` (integer arguments inside `<…>`).
   - `crates/pilot-parser/src/query.rs:31,37,44,92` uses `20` and `15` (LINQ clauses).
   - **Change:** each takes its power from the table, anchored to an operator's native code, the way `pattern_literal_bp` already does (`stmt.rs:229`):
     - `30` → the power of `|` (`BinaryOp::BitOr`);
     - `40` → the power of `..`;
     - `20` / `15` → just above `&&` (`BinaryOp::And`).
   - Today's grouping is kept exactly. No operator text and no level name appears in Rust.

| File | Change |
| :--- | :--- |
| `crates/pilot-parser/src/lib.rs` | Two-pass bootstrap in `bootstrap_tables()` / `bootstrap_diagnostics()`. |
| `crates/pilot-parser/src/stmt.rs` | `:30`, `:58`: anchored binding powers instead of `30` / `40`. |
| `crates/pilot-parser/src/query.rs` | `:31`, `:37`, `:44`, `:92`: anchored binding powers instead of `20` / `15`. |

**Design details fixed while mapping (no extra files):**
- **Seed binding powers.** The seed computes each power as `10 + 4 × ordinal`. Shapes and associativity are read by **ordinal** from the enums passed to `register_grammar(level_enum, assoc_enum, shape_enum)`, so no variant name appears in Rust. Shape ordinals follow the §4 table order and are append-only under B+.
- **`grammar_entry(target, symbol, shape, level, assoc, word)`.** An empty `symbol` means the target's own name, so `[Infix(::Add, ::Left)]` on `op +` declares `+`.
- **Native codes.** A word's native `BinaryOp` code until step 3 is found by matching `BinaryOp::symbol()`, so no English word is added to the parser.
- **`is` stays at `Equality`.** The §9 table above lists it under `Cast`, but today `is` binds at 45 (`Equality`). This is corrected to keep its grouping.
- **`not x` changes grouping.** `not` takes the `Unary` level of `!`, per §8.3. Today its operand binds at 16, below `==`, so `not a == b` changes from `not (a == b)` to `(not a) == b`, the same grouping as `!a == b`. No code or test in the repository writes `not` followed by a comparison.

**Discovered while itemizing (rule 3):** `crates/pilot-parser/src/tables.rs` (`ingest_node`, `map_binop_sym`), `expr.rs` (`:100-107`) and `decl.rs` (`parse_attrs`) still pack several statements per line, against `AGENTS.md` §1.1 and the `PROP-001` record that the parser was unpacked. `cargo fmt --check` could not be run to measure it (rustfmt is not installed for the stable toolchain). The reformat above fixes these three files; any other packed file needs its own entry.

**Step 1 as built (2026-11-15) — differences from the plan above, for owner review.**
- **Net Rust LOC grew by +382, not "near zero".** `crates/**/src` went from 20,416 to 20,798 lines, which is above `paused_max_total_rust_loc()` (20,408; the starting point was already 8 over). The growth is in `tables.rs` (+280: the grammar types, ordinal-based binding powers, `apply_hook` moved from `attr_hooks.rs`, and the structural-symbol and native-code lookups), `lib.rs` (+15, two-pass bootstrap), `expr.rs` (+49), `decl.rs` (+15), `stmt.rs` (+12), `query.rs` (+8) and `attr_hooks.rs` (+3). The deletions are expected in steps 2 and 3. Until then the manifest ceiling needs an owner decision: raise it, or trim step 1 first.
- **Not reformatted.** `rustfmt` is not installed for the stable toolchain, so the promised whole-file reformat of `tables.rs`, `expr.rs` and `decl.rs` was not done, and `cargo fmt --check` was not run.
- **Operator files.** `stdlib/core/operator.pilot` was kept, with the attributes on its `op` decls, instead of being split into `arithmetic`/`compare`/`bitwise`/`range`/`absent` files. The new files are `levels`, `shapes`, `logic` and `flow`. `quoting.pilot` and the `call_quote.pilot` deletion were not done, because call-quote is still token-based. The 19 old `stdlib/core/operators/*.pilot` files duplicated `operator.pilot` and were deleted.
- **Level changes from the §9 table:**
  - `intersect` stays at `And`, where it bound before (the table said `Or`).
  - `//` (floor division) is not declared, because it is comment syntax.
  - Unary `+` is not declared.
  - Prefix `>..` now binds at `Range`.
- **Still token-based (not yet declared):** `?.`, `?..`, cascade `..`, the six call-quote forms, `catch`, and the compound assignments. The `_` arms in `rule_arms.rs` are deferred.
- **`sh_get_precedence` is replaced by a seam, not by `grammar_entries()`.** No runtime hook gives a Pilot program the seed's grammar, so `selfhost/parser.pilot` now stops at `seam("parser.operator_grammar", ..)` on any binary operator, and its numeric table is deleted. Its only caller is the Rust test `milestone61_stage3_fixed_point_and_universal_rename_gate.rs`, whose `test_61_1` (Pratt tree) now fails at that seam. `test_61_3_and_61_4` was already failing before this change (`toolchain source stdlib/core/bridge_os.pilot not found`).
- **Quote printer limits (found, not fixed):**
  - `quote { u || not t }.source()` prints `u || t`, dropping `not`.
  - A quote whose statement starts with `not` gets `SYN001` (`stmt.rs` statement-start check).
- **`stmt.rs:229`** (`pattern_literal_bp`) still uses its own floor of 36.


---

## `PROP-009` — Every Receptor Word and `__ssa` Defined by a `syntax` Rule with a `quote` Template

**Status:** STEPS 1A AND 1B APPROVED BY OWNER 2026-11-15 ("Approve 1a and 1b"). Owner direction (2026-11-15): "do the same thing [as `PROP-008`] but with `bind_receptor`, and `__ssa`. It should be defined via syntax and quote". Owner answers the same day:
- **Coverage:** all receptor words, staged. Each step gets its own itemized file list and approval.
- **`__ssa`:** its template expands to an `asm` instruction.

### 1. What exists today (mapped 2026-11-15)

**Receptors.**
- `bind_receptor(word, category, slot)` appears 122 times in 19 files. It binds about 120 words to about 40 numbered categories. Each category is a hand-written parse routine in the Rust seed, chosen by number:
  - `stmt.rs:119` and `expr.rs:211` dispatch on the category number.
  - The other routines are in `decl.rs`, `switch.rs`, `exits.rs`, `contracts.rs`, `query.rs`, `with_scope.rs`, `modules.rs`, `unbuilt.rs`, `loud.rs`, `rule_arms.rs` and `pilot_impls.rs`.
- The numbers are magic counts (AGENTS.md §1.4, "zero rigid corners or magic counts"). The grammar each word gets is fixed in Rust and invisible from Pilot.
- Nine bindings are made twice: `as`, `break`, `continue`, `get`, `set`, `op`, `extern`, `lib`, `nameof`. They appear once in `core.pilot` and again in `stdlib/core/syntax/*.pilot`.

**`__ssa`.**
- `core.pilot:384-391` declares six comptime functions: `__ssa`, `__ssa_unary`, `__ssa_cast`, `__ssa_load`, `__ssa_store`, `__ssa_gep`. Each calls a frozen `emit_ssa_*` hook.
- About 465 primitive `op` bodies in `stdlib/core/primitives/*` and `builtins/builtins.pilot` use them (`op + (rhs: i64) -> i64 => __ssa(Cfg.Ssa.add, self, rhs);`).
- No Rust code implements any `emit_ssa_*` hook. Every engine lowers primitive operators natively and never runs these bodies, so they are decoration, not definition (AGENTS.md §11 rule 6: a fake to be replaced or recorded).
- `asm` is a seed token, but today it only reports `unbuilt` (`stmt.rs:112`, `errors/err_asm_unbuilt`).

### 2. Classification of the receptor categories

Every word ends as a `syntax` rule in Pilot. The categories differ in what the rule's template can expand to.

| Class | Categories (words) | Template target |
| :--- | :--- | :--- |
| **A. Sugar.** The meaning is already other Pilot. | `with` scope (30/1); LINQ `from … in … where … order by … select` (33/6–11); `splat`/`pairs`/`flat` call forms (35); the six call-quote words (37); `nameof` (33/25); `guard c else { }` without `some`; `unless`-style rules already written | `quote` of existing Pilot (`{ let $n = $f(); $body }`, `for` loops, `*$x`, `$f!($args)`, `stringify($x)`) |
| **B. Core forms.** The word builds an AST node that no other surface syntax produces. | literals `true`/`false`/`none` (1, 2); `else` (3); `for`/`in`/`then` (9–11); `break`/`continue` (13); `switch`/`case`/`default` (14–16); `guard some` (17); `try`/`catch` (18, 19); `scope`/`enter`/`exit` (20–22); `get`/`set`/`op` (23–25); `type`/`enum`/`where` (26–28); `as`/`view` (29); bundle `coalesce`/`with`/`without`/`on` (30); `some` (31); polyglot blocks (32); contracts (33/18–24); quote words `raw`/`splice`/`splat`/`stringify` (36); `errdefer`/`become` (38, 39); declaration heads `syntax`, `type_construct`, `attribute_construct`, `extern`, `lib` (4–8) | needs a way for a template to name a core node (step 2 design) |
| **C. Unbuilt.** | category 40 (`macro`, `actor`, `adapter`, `module`, `state`, …) | the template is a registered seam (it is today, via `unbuilt.rs`) |

### 3. Step 1a — rule-head patterns, and class A moved to rules

**Pattern language (Pilot side).** A `syntax` rule head may be a sequence instead of a call shape:

```pilot
pub syntax {
    with $open:Ident $name:Ident $body:Block => quote { { let $name = $open(); $body } };
    guard $cond:Expr else $fallback:Block   => quote { if !($cond) $fallback };
}
```

- The first item is the trigger word, followed by literal words and typed holes `$name:Kind`.
- `$( … )?` marks an optional group and `$( … )*` a repeated group. `$` is already reserved for splicing, so neither collides with structural `(`, `)`, `[`, `]`.
- Hole kinds are the variants of a Pilot enum, `HoleKind { Expr, Block, Pattern, Ident, Type, Stmt }`, in the new file `stdlib/core/syntax/holes.pilot`. They are handed to the seed by ordinal through a new hook, `register_hole_kinds(nameof(HoleKind))`, the way `register_grammar` hands over `OpLevel`.
- Rust never sees a kind name.
- Two rules with the same trigger word are tried in the order written; the first that matches wins (the same rule as `rule_arms.rs` today).

**Seed side.** The parser stores each head as a list of `(literal | hole(kind ordinal) | group(optional / repeated))` and matches it against the token tape. Matching is longest-first within a group, with no backtracking past the trigger word. The template is instantiated with the existing `template.rs`, and a repeated group binds `$*name` sequences, which `template.rs` already splices.

**Class A moves.** Each word below gets a Pilot rule, and its Rust routine and `bind_receptor` line are deleted:
- `with` scope: all of `with_scope.rs`;
- the LINQ query: all of `query.rs`, written as `for` and `if` in a quote;
- the `splat`, `pairs` and `flat` call forms: their `expr.rs` arm;
- the six call-quote words: their `expr.rs` arm;
- `nameof`: its `pilot_impls.rs` receptor;
- plain `guard … else`: its part of the `stmt.rs` arm.

While building, each word is checked to be pure sugar on all three engines. A word that is not moves to step 2 and is listed in this entry.

**Duplicates.** The nine double bindings in `core.pilot` are deleted. Each `stdlib/core/syntax/*.pilot` file keeps its own binding until its word moves.

**Rust files (step 1a; approval needed):**

| File | Change |
| :--- | :--- |
| `crates/pilot-parser/src/rule_arms.rs` | Sequence heads: literals, typed holes, `$( )?` and `$( )*` groups; matching against the tape. |
| `crates/pilot-parser/src/template.rs` | Bind repeated-group captures as sequences. |
| `crates/pilot-parser/src/decl.rs` | `parse_syntax_decl`: read a sequence head. |
| `crates/pilot-parser/src/tables.rs` | `register_hole_kinds` hook; rules keyed by trigger word, so a sequence rule may start a statement or an expression. |
| `crates/pilot-parser/src/stmt.rs` | Try sequence rules at statement start; drop the `guard`-without-`some` path. |
| `crates/pilot-parser/src/expr.rs` | Try sequence rules in prefix position; delete the category 35 and 37 arms. |
| `crates/pilot-parser/src/pilot_impls.rs` | Delete `RECEPTOR_SYMBOL_QUOTE` (`nameof`). |
| `crates/pilot-parser/src/query.rs` | **Delete** (250 lines). |
| `crates/pilot-parser/src/with_scope.rs` | **Delete** (65 lines). |
| `crates/pilot-parser/src/lib.rs` | Drop the two `mod` lines. |

- **Pilot side (no approval needed):** `holes.pilot`; the rules in `syntax/module_ext.pilot`, `db/sql_syntax.pilot`, `syntax/expr_sugar.pilot`, `syntax/call_quote.pilot`, `syntax/guard.pilot` and `core.pilot`; `register_hole_kinds` in `bridge_lang.pilot` and appended to `airlock_lang_hooks()` (an append under B+, approved through this entry).
- **Tests:** one passing and one `err_*` program per moved word, in `tests/conformance/receptors/`.
- **Diagnostics:**
  - `SYN010`: a sequence rule matched its trigger, but a hole did not match. The message names the hole and its kind.
  - `SYN011`: two sequence rules for one trigger, where the earlier one always wins.
- **Ledgers, maps and matrices** are updated in the same change.

**Rust LOC.** The step must be a **net deletion**, measured before and after. The pattern matcher is expected at about +200, against −315 for `query.rs` and `with_scope.rs` plus the deleted `expr.rs` and `stmt.rs` arms. If the measured result is not negative, the work stops and is reported before anything else moves. The files touched are reformatted to `rustfmt` layout once `rustfmt` is available (it is not installed today; `PROP-008` has the same gap).

### 4. Step 1b — `__ssa` as `syntax` rules over `asm`

**Pilot.**
- New file `stdlib/core/asm/machine_ops.pilot` declares `enum MachineOp { Add, Sub, Mul, SDiv, UDiv, … }`, one variant for each of the 55 `Cfg.Ssa` entries, in that order, then `Load`, `Store` and `ElemAddr`. It also calls `register_machine_ops(nameof(MachineOp))`, a new hook appended under B+.
- The six comptime functions in `core.pilot` become rules:

```pilot
pub lvl(Cfg.Tokens.ring_simd) syntax {
    __ssa(op, lhs, rhs)        => quote { asm($op, $lhs, $rhs) };
    __ssa_unary(op, val)       => quote { asm($op, $val) };
    __ssa_cast(op, val, T)     => quote { asm($op, $val, $T) };
    __ssa_load(p)              => quote { asm(MachineOp::Load, $p) };
    __ssa_store(p, val)        => quote { asm(MachineOp::Store, $p, $val) };
    __ssa_gep(base, idx)       => quote { asm(MachineOp::ElemAddr, $base, $idx) };
}
```

- `Cfg.Ssa` is deleted. The 465 call sites are rewritten mechanically from `Cfg.Ssa.add` to `MachineOp::Add`.

**Seed.**
- `asm(op, operands…)` in expression position parses to a new `NodeKind::AsmExpr`, appended to `pilot-ast`. The opcode is the variant's **ordinal**.
- The seed VM lowers it to the VM instruction for that ordinal. An ordinal with no VM instruction stops at `seam("asm.machine_op", …)`.
- `asm { … }` blocks keep reporting `unbuilt`.
- No opcode name appears in Rust: the seed reads ordinals, as `PROP-008` reads `ParseShape`.

**Pilot engines.**
- `selfhost/check/infer.pilot` types `AsmExpr`: the operand type; `bool` for compares; `T` for casts; `void` for a store.
- `selfhost/ir/lower_expr.pilot` lowers it to the matching PilIR instruction.
- The C emitter prints the C operator.

**What changes for primitive operators.** Nothing at first: the engines still lower `a + b` on `i64` natively. Step 1b makes the op bodies *real* (type-checked and runnable when called directly, e.g. through an interface). A later step makes the engines use them, by forced inlining on PilIR (`PROP-007`) and dropping the native operator table. That step is listed here, not approved.

**Rust files (step 1b; approval needed):**

| File | Change |
| :--- | :--- |
| `crates/pilot-ast/src/lib.rs` | Append `NodeKind::AsmExpr`. |
| `crates/pilot-parser/src/expr.rs` | `asm(` in prefix position → `AsmExpr`. |
| `crates/pilot-parser/src/stmt.rs` | The `KwAsm` arm: a `(` form goes to the expression path, and `{` stays `unbuilt`. |
| `crates/pilot-parser/src/tables.rs` | `register_machine_ops` hook: variant name → ordinal, used to resolve `MachineOp::X` in an `asm` head. |
| `crates/pilot-vm/src/lower_native.rs` | Lower `AsmExpr` by ordinal onto the existing VM instructions; any other ordinal → seam. |

- **Pilot side:** `machine_ops.pilot`; the `core.pilot` rules; the call-site rewrite; `register_machine_ops` in `bridge_lang.pilot` and `airlock_lang_hooks()`; the checker, PilIR and C emitter support.
- **Tests:** `tests/conformance/asm/` calls primitive ops through their bodies on all three engines, plus `err_asm_*` for a wrong operand count and an unknown op.
- **Rust LOC:** about +60. Step 1a must offset it.

### 5. Step 2 — core forms (DIRECTION APPROVED 2026-11-15: "Approve CoreForm"; the itemized Rust file list below still needs approval)

Class B words need a template to name an AST node that has no other spelling.

**Recommended design:** a closed `CoreForm` enum in Pilot (`BoolLit`, `NoneLit`, `For`, `Switch`, `Catch`, …), read by ordinal like `ParseShape`. A rule's template calls `asm`'s compile-time sibling, `quote { __form(CoreForm::For, $item, $seq, $body, $fallback) }`. The seed builds that node kind.

With this design, the grammar of every word lives in Pilot, and the seed keeps only a closed, append-only list of node constructors. Each category's Rust routine is deleted as its rule lands. The itemized list follows once step 1 shows the pattern matcher is enough for `for`, `switch` and `try`.

### 6. Step 3 — retire `bind_receptor` (OWNER CHOSE OPTION (b) 2026-11-15: "prop-009 step 3: choose B"; the Rust file list is itemized separately)

`bind_receptor` and the six `emit_ssa_*` hooks are in the **frozen** B+ list (`airlock_lang_hooks()`, `PROP-002a`). After steps 1–2, nothing calls them.

Under Rule 4 frozen items are never deleted, so they would stay in `bridge_lang.pilot` with no callers, and the seed would keep their dispatch arms. The owner chooses one:
- **(a)** keep them frozen and uncalled, and add a `GATE021` check that nothing new calls them; or
- **(b)** explicitly unfreeze these seven hooks, and remove them with their Rust arms. This is the only path that deletes the Rust.

### 7. Open questions for approval
1. Approve step 1a and step 1b with the file lists above. They are independent of each other.
2. Is the `CoreForm` direction acceptable for step 2, or should class B words get a different mechanism?
3. Which option, (a) or (b), for the frozen hooks in step 3?
4. The `PROP-008` step 1 Rust growth (+382) is still waiting for a decision. Should step 1a's net deletion count against it?

### 8. Steps 1a and 1b as built (2026-11-15) — STOPPED: step 1a is not a net deletion

**Measured Rust LOC.** `crates/**/src/*.rs` went from 20,798 (after `PROP-008` step 1) to **21,101: +303**. Per file: `rule_arms.rs` +255, `lower_native.rs` +54, `decl.rs` +28, `tables.rs` +28, `stmt.rs` +9 net, `template.rs` +5 net, `expr.rs` −7 net, `lib.rs` −1, `with_scope.rs` −65 (deleted). About +70 of this is step 1b (the `asm` lowering, its two `tables.rs` fields and `asm_operation_follows`), as proposed. Step 1a is about **+230**, not a net deletion. As §3 requires, the work stopped here and nothing else moved. The owner decides one of these:
- (a) keep the matcher and approve moving more words onto it, so later deletions pay it back;
- (b) revert step 1a and keep step 1b;
- (c) extend `template.rs` first (below), then delete `query.rs`. This is the only planned deletion big enough to make step 1a negative.

`paused_max_total_rust_loc()` (20,408) has been exceeded since `PROP-008` step 1, and it is not raised here. `rustfmt` is still not installed, so the touched files have not been reformatted.

**What moved to rules in step 1a:**
- `with arena name { .. }` / `with arena { .. }`. Now a sequence rule in `stdlib/core/syntax/lifecycle.pilot`; `with_scope.rs` is deleted.
- `splat(x)`, `pairs(x)`, `flat(x)`. The receptor category 35 arm in `expr.rs` and its three `bind_receptor` lines are deleted, and the existing rules in `expr_sugar.pilot` now take these calls. To make that work, `template.rs` now unwraps a quote whose only item is a unary expression (`quote { *$x }`), so a list literal or call sees the `*x` and not a block.

**What did not move, and why:**
- `nameof`. It is genesis and is used before any rule exists.
- `guard`. The parser node carries the divergence check that `GuardStmt` needs; a rule would lose it.
- LINQ (`query.rs`). Templates cannot hold `for`, lambdas or `switch`, and cannot splice a sequence. `template.rs` must be extended first; see option (c).
- Call-quote words (`call`, `splice`, `frame`, `sig`, `grad`, `plan`). These need a sequence spliced into call arguments, which templates cannot do yet.

**Deviations from the file list:**
- **`asm` needed no `NodeKind::AsmExpr`.** In expression position `asm(op, ..)` parses as a call whose callee is the identifier `asm`, the same way `__engine(..)` does. So `pilot-ast` and the `expr.rs` asm prefix were not touched.
- **No seed diagnostics for `asm`.** The seed lowering (`lower_native.rs`) reports no error for an unknown op or a wrong operand count. Those are ASM001 and ASM002 in the Pilot checker (`selfhost/check/asm.pilot`). An op with no seed instruction traps at run time at `seam("asm.machine_op", ..)`.
- **Seed limits.** The seed has no unsigned or width-specific instructions. Unsigned, saturating, conversion, load, store and element-address ops stop at the seam, and the others run at `i64` width.

**Verified:**
- seed / `--ir` / `--emit c` on `tests/conformance/asm/` (4) and `tests/conformance/receptors/` (3): all pass on all three engines.
- The seven splat/pairs/flat programs and `memory/arena_scope`: pass on all three engines.
- Full seed `pilot test`: 516 passed, 0 failed, 13 known bugs reproduced.
- `cargo test -p pilot-parser` and `damaged_source --include-ignored`: pass.

**Not run:** the full `--ir` and `--emit c` suites, `pilot gate --full`, `cargo fmt --check` (no `rustfmt`).

### 9. Option (c) as built (2026-11-15, owner: "Extend templates first") — step 1a is now about +56, still not a net deletion

**Measured Rust LOC:** 20,924 (was 21,101 after §8; baseline 20,798 after `PROP-008` step 1). That is **−177 since §8**, and +126 for steps 1a and 1b together. With step 1b at about +70, **step 1a is about +56**. It is still not negative, so the work stopped again for the owner. The rest of the deficit is the sequence matcher itself. The deletion large enough to cover it is the old call-shaped rule machinery (`RuleArm`, `SyntaxMacroRule` and the arms in `rule_arms.rs`): call-shaped heads could become sequence heads (`splat ( $x:Expr )`). That is a new step and needs its own approval.

**Rust changes (all in the approved step 1a file list):**
- `crates/pilot-parser/src/query.rs`: **deleted** (250 lines), with its `mod` line in `lib.rs` and the receptor 33/6 arm in `expr.rs`. `ParserTables::bp_above` in `tables.rs` was only used there, so it is deleted too.
- `crates/pilot-parser/src/template.rs`:
  - Templates can hold `for name in iter { .. }` (no `then`) and `yield`.
  - An `if` whose condition expands to `true` or `false` is decided when the rule expands.
  - A binary operator whose left operand uses a repeated hole is repeated once per element, right-nested (`$c && true` is `c1 && (c2 && true)`; with no elements, just `true`).
  - `let $name` and `for $name` share one `bound_name` helper.
- `crates/pilot-parser/src/rule_arms.rs`:
  - An optional group binds a `bool` named after its first word: whether it was written.
  - A repeated group that starts with a word may be written no times.
  - Matching stops at the first token that does not fit. An unbuilt query-clause word there still reports SEAM001 `keyword.unbuilt`.
  - `repetitions` computes each repetition's bindings, and `intern_binder` reads `$name` binders.
  - Hygiene also renames a template's own `for` names.
- `stmt.rs` (`parse_for` reads `$name`) and `decl.rs` both use `intern_binder`.

**Pilot side:**
- **The query rule.** `stdlib/db/sql_syntax.pilot` has the `from $x:Ident in $src:Expr $( where $c:Expr )* $( order $( by )? $( $k:Expr $( ascending )? $( descending )? ),* )? select $e:Expr` rule. Its six `bind_receptor` lines for `from`, `select`, `order`, `by`, `ascending` and `descending` are deleted. `select` stays bound in `stdlib/asyncio/select_syntax.pilot`.
  - Without `order`, a query is one collecting `for`, the same as `query.rs` built.
  - With `order`, a stable insertion sort runs on the kept elements. The keys are chained by `&&`, so a later key is computed only while the earlier ones tie.
- **Checker.** `selfhost/check/rules.pilot` now checks each side of `&&` and `||` as a condition, outside quotes (TYPE001), so `where x + 1` is still rejected at build time. `enclosing_quote` in `quote_value.pilot` is made `pub` for this.

**Design changes a reviewer should check:**
- `err_query_clause` and `err_query_order` now report `SYN010: `from` expects `select` here, found `…`` instead of their old SYN001 wording. They are still rejected at build time, and their expected `.stderr` was updated.
- A `where` operand is now a whole expression. `where a && b` reads as one clause; before, `&&` ended the clause.
- `1 && true` was accepted at build time and failed at run time. It is now TYPE001.

**Known bugs found and registered (true expected output kept):**
- `check.collecting_for_in_block` (`known_bugs/block_value_collecting_for`). A collecting `for` as the value of a block or `if .. else` branch makes the enclosing function a generator, so `main` never runs, and the program prints nothing and exits 0 on all three engines. The query rule avoids this by deciding `if $order` at expansion and ending its block in a variable.
- `parser.rule_template_while` (`known_bugs/rule_template_while`). A `while` inside a rule's `quote` loses its condition (QUOTE003, LOOP005). This also happens at HEAD. The query rule uses `loop { if .. { break; } .. }`.

**Not built:** a repeated hole spliced as call arguments or list elements (the call-quote words need it), and the call-quote, `nameof` and `guard` moves.

**Verified:**
- The query programs (`query_from_select`, `query_where`, `query_order_by`): pass on seed, `--ir` and `--emit c`. Before this change they also passed on all three, with `query.rs`.
- `receptors/` (5 programs) and `errors/err_and_operand_not_bool`: pass on all three engines.
- Full seed `pilot test`: 521 passed, 0 failed, 15 known bugs reproduced.
- `damaged_source --include-ignored`: passes.

### 10. Step 1c — syntax classes: a clause defined once, nested into every rule that uses it (APPROVED 2026-11-15 together with the call-shaped rule deletion: "With the RuleArm deletion")

**Owner direction (2026-11-15):** "For any keyword that doesn't vary in its interpretation between the two contexts, then I think we need to nest quoted syntax."

**Which words are shared.** Each query word was checked against every other place it is parsed:

| Word | In a query | Elsewhere | Same meaning? | Becomes |
| :--- | :--- | :--- | :--- | :--- |
| `x in xs` (binding) | `from x in xs`: each element of `xs` as `x` | `for x in xs` (`stmt.rs` `parse_for`): the same | **yes** | shared class `Iteration` |
| `in` (operator) | — | `x in xs`: membership, a `bool` (`operators/flow.pilot`) | no | stays an operator |
| `where cond` | keep the element when `cond` holds | `case p where cond` (`switch.rs`): take the arm when `cond` holds | **yes** | shared class `Filter` |
| `where T : I` | — | a constraint on a type parameter (`decl.rs`, `loud.rs`) | no | stays as it is |
| `select e` | projection | `select { .. }`: channel select (`asyncio/select_syntax.pilot`) | no | literal word in the `from` rule |
| `order`, `by`, `ascending`, `descending`, `from` | query only | ordinary names | query-local | class `QueryClause` in `sql_syntax.pilot` |

**Pilot syntax.** A named `syntax` block whose arms have heads but no `=> quote` is a **syntax class**: a hole kind made of words and holes. No new keyword is needed.

```pilot
// stdlib/core/syntax/clauses.pilot — shared by every rule that iterates or filters
pub syntax Iteration { $x:Ident in $src:Expr; }
pub syntax Filter    { where $c:Expr; }

// stdlib/db/sql_syntax.pilot — query-local clauses
pub syntax QueryClause {
    $w:Filter;
    order $( by )? $( $k:Expr $( ascending )? $( descending )? ),*;
}
pub syntax {
    from $it:Iteration $( $q:QueryClause )* select $e:Expr => quote { /* uses $x, $src, $c, $k, $descending */ };
}
```

- `$name:ClassName` matches one arm of the class, chosen by its leading word or hole kind with the same no-backtracking lookahead as today. Two arms of a class that the lookahead cannot tell apart are rejected where the class is declared (`SYN011`, the code §3 planned).
- The holes inside the chosen arm are bound into the using rule as if written there (`$x`, `$src`, `$c`, …). Inside a repeated group they become sequences, as other holes do. Holes of arms not taken are bound as absent, or as empty sequences inside a repeated group. So `from` gets every `where` condition and every `order` key in one fused loop: one pass, no list per clause.
- Two holes with one name reachable from a rule are rejected (`SYN011`).
- A class name is a hole kind beside the built-in `HoleKind` variants. Rust never sees a class name; it keeps the names a `syntax Name { .. }` block declares.

**Only the grammar of a shared clause is defined once.** Its meaning stays in each rule that uses it, because a `where` filters a loop in a query but guards an arm in `switch`. A class arm that carries its own template (`where $c:Expr => quote { .. }`) is left for later.

**What moves now, and what waits for step 2:**
- `from` uses `Iteration` and `QueryClause` (containing `Filter`) at once. Same programs and outputs. `skip` and unbuilt clause words keep their current diagnostics.
- `for` and `switch` are still parsed in Rust (`parse_for`, `switch.rs`). They take `Iteration` and `Filter` when they move to rules, which is step 2 (core forms). Until then each class has one user, so for those two words the "define once" goal is only half met.

**Rust files (approval needed):**

| File | Change |
| :--- | :--- |
| `crates/pilot-parser/src/decl.rs` | `parse_syntax_decl`: read `syntax Name { head; .. }` (arms without a template) as a class. |
| `crates/pilot-parser/src/tables.rs` | `syntax_classes: HashMap<String, Vec<Vec<HeadItem>>>`. |
| `crates/pilot-parser/src/rule_arms.rs` | `HeadItem::Class(name, hole)`; `parse_seq_head` takes a class name as a kind; `quick_fit` / `match_items` choose an arm and bind its holes into the rule; `hole_names` includes a class's holes; SYN011 for an arm the lookahead cannot tell apart and for a duplicate hole name. |

**Rust LOC:** about +50. Step 1a is about +56 today (§9), so the total would be about +106. This step deletes no Rust by itself. The deletion that can cover it is still turning the call-shaped rule machinery (`RuleArm`, `SyntaxMacroRule`, `expand_rule_arms`, `arm_fits`, `parse_rule_head`, and their callers in `stmt.rs` and `expr.rs`, about 300 lines) into sequence heads. Classes make that easier: a call-shaped head `name(a, b = d)` becomes a sequence head plus a class for its argument list. The owner chooses whether this step lands alone, or only together with that deletion, as one step that removes Rust overall.

**Pilot side (no approval needed):** the new file `stdlib/core/syntax/clauses.pilot` (added to `BOOTSTRAP_SOURCES`, which is a `tables.rs` change and already in the list above); the `from` rule rewritten over the classes; `SYN011` in `selfhost/diagnostics/errors.pilot`.

**Tests (Pilot):**
- `receptors/syntax_class.pilot`: a user class with two arms, used in a repeated group.
- `receptors/err_syntax_class_ambiguous.pilot` (SYN011).
- `receptors/err_syntax_class_duplicate_hole.pilot` (SYN011).
- The `collections/query_*` programs, unchanged.


**As built (2026-11-15).**
- **Call-shaped rule machinery deleted.** `RuleArm`, `SyntaxMacroRule`, `RuleHead`, `rule_head_is_rich`, `parse_rule_head`, `add_rule`, `expand_rule_arms`, `arm_fits` (`rule_arms.rs`), `expand_syntax_macro` and the `syntax_macros` lookups (`stmt.rs`, `expr.rs`), the `syntax_macros` table (`tables.rs`) and the re-export (`lib.rs`). A rule with a `quote` template and no attribute now always has a sequence head. A parameter list `name(params) =>` remains only for an operator's function (`[Infix]`, `[Prefix]`, `[Shaped]`) and for comptime rules (`break()`, `continue()`, `op(sig, body)`). A quote rule still written `name(x) => quote` is SYN010 ("the parameters of the rule are holes").
- **What the deletion needed in the matcher (`rule_arms.rs`).**
  - A word's rules are tried in order, with backtracking. A rule that read past a spelled word after its own reports its SYN010. If none did, the word is an ordinary name, except when a name or literal follows (`assert true;`), where the first rule says what it expected.
  - An expression hole that reads nothing fails the match.
  - An optional group's holes may have defaults (`$( , $note:Expr = "" )?`).
  - `stringify($p)` is a hole's source text.
  - An expression hole beside an operator word stops at that operator (`$left:Expr == $right:Expr`).
- **Syntax classes as designed above, with one simplification.** `from` reads `$it:Iteration $( $f:Filter )*` and keeps its `order` clause in its own head. There is no `QueryClause` class: a class arm holding a repeated group, inside a repeated group, would bind sequences of sequences, and templates cannot fold those yet. SYN011 covers arms that start alike and a hole name bound twice.
- **Pilot changes.**
  - Rewritten as sequence heads:
    - `__ssa*`, moved from `core.pilot` to `stdlib/core/asm/machine_ops.pilot`, because `core.pilot` is parsed before `HoleKind`;
    - `while` and `until` (both forms);
    - the 7 `assert` arms;
    - `splat`, `pairs`, `flat` calls;
    - the seam stubs in `rt_syntax.pilot`, `select_syntax.pilot` and `sql_syntax.pilot`, with the bare block forms the docs use (`deadline 50ms { .. }`) as their own arms.
  - Deleted dead rules, never expanded because receptors parse these words first:
    - `call`..`plan`, `guard`, `switch`/`case`/`default`, `try`/`catch`, `for`, `get`/`set`;
    - `coalesce`/`without`, `scope(..)`;
    - `raw`/`splice`/`splat`/`unquote_eval`/`syntax_overload`/`syntax_case` in `quote.pilot`;
    - the duplicate `while` in `core.pilot`.
  - `unit`'s pass-through rule became receptor 40 (seam `keyword.unbuilt`).
- **Rust LOC:**
  - 20,864, which is −60 against `eec5220` (20,924), so step 1c alone is a net deletion.
  - PROP-009 overall is still +66 against the 20,798 baseline: steps 1a/1b added about +126.
  - `paused_max_total_rust_loc()` is 20,408, already exceeded before this step.
- **Owner approval needed: `crates/pilot-parser/tests/milestone51_homoiconic_syntax_and_contraction.rs`.** APPROVED BY OWNER 2026-11-15 ("approve milestone51 edit"; the owner also approved all pending test edits and speed baselines: "all tests and speeds are now approved").
  - `test_51_1` writes user rules in the deleted form `name(cond, body) => quote` and uses them bare.
  - `test_51_3` requires every listed syntax module to have at least 15 lines; `guard.pilot` (12) and `switch.pilot` (14) are now shorter.
  - Both fail. Proposed change: write the rules in `test_51_1` as sequence heads (`name $cond:Expr $body:Block`), and drop the 15-line floor in `test_51_3`, or delete the test in favour of the Pilot conformance programs (PROP-004 direction).
  - Not edited, because the file is not in the approved list.

### 11. Step 2 — core forms, itemized (direction approved 2026-11-15; PHASES 2A + 2B APPROVED TOGETHER 2026-11-15: "2a and 2b together")

**Mechanism.**
- A Pilot enum lists the nodes the seed can build. Its variant order is the ABI, append-only like `MachineOp`:

  ```pilot
  // stdlib/core/syntax/forms.pilot
  pub enum CoreForm : u8 { BoolLit, NoneLit, For, Break, Continue, Guard, Try, Catch, .. }
  comptime { register_core_forms(nameof(CoreForm), nameof(form)); }
  ```

- A rule's template builds a node with `form(CoreForm::X, operands..)`. The builder word (`form`) is named by the library, not by Rust; Rust stores whichever word the hook names. An operand that holds a repeated hole is expanded once per repetition and spliced in as several operands. A `form(..)` operand is itself a node, so `try` gives one `Catch` per `catch` clause:

  ```pilot
  pub syntax {
      true  => quote { form(CoreForm::BoolLit, 1) };
      for $it:Iteration $body:Block $( then $fallback:Expr )?
          => quote { form(CoreForm::For, $x, $src, $body, $fallback) };
      try $body:Block $( $c:CatchClause )*
          => quote { form(CoreForm::Try, $body, form(CoreForm::Catch, $e, $handler)) };
  }
  ```

- Each word's rule replaces its receptor. The Rust routine that parsed the word is deleted in the same change. The node each rule builds is the one Rust built, so the checker, PilIR and the C emitter do not change.

**Phases.** Each phase lands only when every program that uses its words passes on all three engines.

| Phase | Words (receptors) | Needs in the matcher |
| :--- | :--- | :--- |
| **2a** | `true`/`false`/`none` (1, 2); `break`/`continue` with an optional value (13); `for x in xs { } then ..` (9–11); `guard [some] cond else { }` (17); `try { } catch [e] { }` / `catch e => ..` (18, 19 as a statement) | `form(..)`; a head that is only its word (`true`); an optional group that starts with a name or block hole (`$( $e:Ident )?`), its flag named after the hole; per-repetition `form(..)` operands |
| **2b** | `switch`/`case`/`default`/`_` with `case a, b:`, `case p if c:` / `where c` (the shared `Filter` class), `:` or `=>` (14–16) | a hole kind `Stmts` appended to `HoleKind`: statements up to the next token an enclosing head spells, or `}` |
| **2c** | each later as its own itemized list: `else` misuse, postfix `expr catch e => ..` (19 as an infix clause: needs postfix rules), declaration heads (4–8), accessors (23–25), `type`/`enum`/`where T: I` (26–28), `as`/`view` (29), bundle words (30), `some` (31), polyglot (32), contracts (33), quote words (36), `errdefer`/`become` (38, 39) | to be designed per group |

**Phase 2a file list.**

| File | Change |
| :--- | :--- |
| `crates/pilot-parser/src/forms.rs` (new, about 70 lines) | `build_form(arena, ordinal, operands, span)`: the closed list of node constructors by `CoreForm` ordinal (`BoolLit`, `NoneLit`, `For` via `alloc_for`, `Break`/`Continue`, `Guard`, `Try`, `Catch`); any other ordinal is SYN010 "not a core form yet". |
| `crates/pilot-parser/src/template.rs` | a `Form` node, built when a template call's callee is the registered builder word; repeated operands spliced. |
| `crates/pilot-parser/src/tables.rs` | the `register_core_forms` hook (enum and builder word); `core_form(variant)`; `forms.pilot` in `BOOTSTRAP_SOURCES` before any file that writes `true`, `for` and the rest. |
| `crates/pilot-parser/src/rule_arms.rs` | a head with no items after its word; an optional group led by a name or block hole. |
| `crates/pilot-parser/src/stmt.rs` | delete receptor arms 9, 13, 17, 18 and `parse_for`; `is_stmt_receptor_start` keeps only the words not yet moved. |
| `crates/pilot-parser/src/expr.rs` | delete receptor arms 1, 2, 9, 18. |
| `crates/pilot-parser/src/lib.rs` | `mod forms;`. |
| `stdlib/core/bridge_lang.pilot` + `selfhost/gate/airlock_abi.pilot` | append `register_core_forms(form_enum, builder_word)` under B+ (§11 of the hooks list). |

**Pilot side.**
- The new `stdlib/core/syntax/forms.pilot`.
- Rules in `for.pilot`, `guard.pilot` and `try_catch.pilot`, using `Iteration` from `clauses.pilot`. `true`/`false`/`none` go next to their types in `core.pilot`, or in `forms.pilot` if bootstrap order requires it.
- The `bind_receptor` lines for the moved words are deleted.
- Keyword and concept matrix rows are updated, and `wiring.pilot` evidence is added.

**Fixed loudly on the way.** `parse_for` reads `for x, i in xs` and silently drops `i` (`stmt.rs`: `if self.eat(Comma) { self.advance(); }`), which breaks AGENTS.md §11 rule 2. The `for` rule has no such arm, so the program gets SYN010. A destructuring binder (`for (a, b) in ..`) stays loud as it is now (`for_pattern_loudly`) until a pattern-binder arm is built.

**Tests.**
- Every existing `for`, `break`/`continue`, `guard`, `try`/`catch` and literal program passes unchanged on all three engines.
- New: `receptors/core_form_rules.pilot`, a user rule built with `form(CoreForm::For, ..)` that behaves like `for`.
- New: `err_core_form_unknown.pilot`, a `CoreForm` variant with no constructor yet (SYN010).
- New: `err_for_two_binders.pilot` (SYN010).

**Rust LOC (estimate).**

| Item | Lines |
| :--- | :--- |
| added: `forms.rs`, template `Form`, the hook, two matcher features | about +130 |
| deleted: receptor arms in `stmt.rs`/`expr.rs` and `parse_for` | about −60 to −70 |
| **phase 2a net** | **about +65** |

The large deletion comes in 2b (`switch.rs`, −101) and 2c (`accessors.rs`, `contracts.rs`, `exits.rs`, `modules.rs`, much of `decl.rs`). Phase 2a is the cost of the mechanism, so the owner decides: approve 2a alone at about +65, or require 2a and 2b to land together: about −15 to −40 net, depending on what the `Stmts` hole costs.

**As built (2026-11-15; uncommitted).**

*Over the estimate: phases 2a + 2b are +186 Rust LOC (20,864 → 21,050), not the estimated −15 to −40.* Lines added: `rule_arms.rs` +195 (the matcher features below), `forms.rs` +104, `template.rs` +25, `tables.rs` +11. Lines deleted: `switch.rs` −101, `stmt.rs` −32, `loud.rs` −15, `expr.rs` −1. The estimate counted the matcher at about +60; it needed nine features, not two. PROP-009 overall is now +252 against the 20,798 baseline. **The owner accepted the overage on 2026-11-15.** The phase 2c deletions (`accessors.rs`, `contracts.rs`, `exits.rs`, `modules.rs`, much of `decl.rs`) reuse this matcher and should add little to it.

- **Mechanism.** `stdlib/core/syntax/forms.pilot` holds `enum CoreForm { For, Break, Continue, Guard, Try, Catch, Switch, Arm, Reject }` and `register_core_forms(nameof(CoreForm), nameof(form))`. The hook is appended to `bridge_lang.pilot` §11 and `airlock_lang_hooks()`. `forms.rs` has one constructor per variant. A template `form(CoreForm::X, ..)` becomes a `TemplateNode::Form` when the rule is added; an unknown variant is SYN010. A template that is one `form(..)` builds that node, not a block around it, so a `guard`'s binding outlives the statement.
- **`Reject`** (not in the plan) is `form(CoreForm::Reject, "message", $code)`. When `$code` is present, the message is an error at it and nothing is built. Loud refusals now live in Pilot: `for (a, b) in ..` and a typed `catch e: T` (it replaces `for_pattern_loudly` and `catch_type_loudly`, deleted from `loud.rs`), and NULL002 for a `guard` with no `else`.
- **Matcher features** (`rule_arms.rs`):
  - optional groups led by a name or block hole or a class;
  - class arms may share hole names; the last arm may start with a hole that reads any code (a fallback);
  - a repeated group with no leading word runs until the word after it;
  - the `Stmts` hole kind (appended to `HoleKind`), ending at `}`, `,` or a lead word of the enclosing repetition;
  - `$name` inside a template fills a name hole;
  - the next item's chance of starting with `{` crosses class and group edges (no struct literal there);
  - each repetition keeps its span (`RUN_SPAN`), so a `switch` arm's diagnostics point at the arm;
  - holes of one repetition stay aligned (`NONE` for a hole a repetition left out); a repetition nested inside a repetition is a sequence of sequences;
  - `=>` may be a word in a class arm.
- **Rules.**
  - `for` (block or statement body, `then`, and the pattern-binder refusal) and `break`/`continue`: `for.pilot`.
  - `guard` (`some`, `let`, plain, and NULL002 without `else`): `guard.pilot`.
  - `try` with `CatchClause`/`CatchBody` classes, and `try expr`: `try_catch.pilot`. `catch` keeps receptor 19 for the infix `expr catch e => ..`.
  - `switch` with `ArmGuard`, `ArmBody` and `SwitchArm` classes: `switch.pilot`.
  - Their `bind_receptor` lines are deleted (`in` keeps receptor 10 for membership). So are the dead `break()`/`continue()` comptime rules in `core.pilot`.
  - `is_stmt_receptor_start` asks whether a word has rules.
- **Not done: `true`/`false`/`none` stay receptors 1/2.** Pattern parsing (`is_rec(1)`/`is_rec(2)`) and every bootstrap file before `forms.pilot` use them.
- **Bugs fixed on the way.**
  - A rule that matched dropped every diagnostic its holes reported. A syntax error inside a `while` body vanished (`errors/err_rule_block_error`).
  - `TemplateNode::from_arena` dropped a `while` loop's condition. The checker reported QUOTE003 for the `$hole` a rule's expansion inside a quote left unattached. `selfhost/check/quote_value.pilot` now checks only code in the program. Known bug `parser.rule_template_while` is closed (`receptors/rule_template_while`), and the query rule's `loop`/`break` workaround is a `while` again.
  - `for x, i in xs` silently dropped `i`; it is now SYN010 (`errors/err_for_two_binders`).
- **Diagnostics that changed.**
  - `err_catch_typed` points at the type (9:39), not at the `:` (9:37).
  - `err_switch_case_list_empty` is now SYN010 ``switch` expects code for `$more_patterns`, found `:``` at the `:`. It was PAT001 from the seed VM.
  - "Expected a switch arm" is now SYN010 "`switch` cannot read .. here".
  - A `try { .. } catch ..` written as an expression is now a `TryStmt`. It used to be the block with an infix `catch`. No program writes it.
- **Tests added:** `receptors/core_form_rules`, `receptors/rule_template_while` (moved from `known_bugs`), `receptors/err_syntax_class_fallback_not_last`, `errors/err_core_form_unknown`, `errors/err_for_two_binders`, `errors/err_rule_block_error`, `errors/err_guard_no_else`.
- **Owner approval still needed: `crates/pilot-parser/tests/milestone51_homoiconic_syntax_and_contraction.rs`** (see §10).
  - Now 4 of its tests fail. `test_51_1` uses the deleted call-shaped rules.
  - `test_51_2`, `test_51_3` and `test_51_4` require the code marker `nameof(` in `switch.pilot` and `guard.pilot`. Their `bind_receptor(nameof(..))` lines are gone, because the words are spelled in rule heads.
  - Adding a `nameof(` only to satisfy the marker would be a fake, so it is not done. Proposed change: drop those two markers. The 15-line floor of `test_51_3` is met again (`guard.pilot` 23, `switch.pilot` 36).

### 12. Step 2c, group 1 — `errdefer` and `become` (APPROVED BY OWNER 2026-11-15: "Approve §12")

The first 2c group: the two exit words of `stdlib/core/syntax/exits.pilot` (receptors 38, 39). It reuses the step 2 matcher as it is, so its Rust is a net deletion.

**Pilot side.**
- `CoreForm` (`stdlib/core/syntax/forms.pilot`) gains two variants, appended: `ErrDefer` (body: a block or one statement) and `Become` (the call).
- `exits.pilot` replaces its two `bind_receptor` lines with rules:

  ```pilot
  pub syntax {
      errdefer $body:Block => quote { form(CoreForm::ErrDefer, $body) };
      errdefer $body:Stmt  => quote { form(CoreForm::ErrDefer, $body) };
      become $call:Expr    => quote { form(CoreForm::Become, $call) };
  }
  ```

- The nodes stay as Rust built them: `DeferStmt` with flags 1 and `ReturnStmt` with flags 1. The checker (`err_become_*`, `err_errdefer_infallible`), PilIR and the C emitter do not change.

**Rust file list.** Strictly these files, in `crates/pilot-parser` only:

| File | Change | LOC |
| :--- | :--- | :--- |
| `crates/pilot-parser/src/exits.rs` | **delete** | −40 |
| `crates/pilot-parser/src/forms.rs` | constants `ERR_DEFER = 9`, `BECOME = 10` and their two arms; `has_constructor` covers them; `DEFER_ON_FAILURE` and `RETURN_TAIL_CALL` move here from `exits.rs` (`pilot-vm` reads them, unchanged) | about +12 |
| `crates/pilot-parser/src/stmt.rs` | delete the two receptor arms (38, 39) | −2 |
| `crates/pilot-parser/src/lib.rs` | drop `mod exits;`; re-export the two flags from `forms` | −1 |

Net: about −30 Rust LOC (21,050 → about 21,020).

**Acceptance.** The existing programs: `contracts/errdefer`, `become_shapes`, `become_tail`, `err_become_defer`, `err_become_ensures`, `err_become_not_call`, `err_become_type`, `err_errdefer_infallible`, and `control/errdefer`. That one already fails on `--ir` and `--emit c` at HEAD, and must fail in the same way after this group. New: `errors/err_errdefer_no_body` (`errdefer` with nothing after it is SYN010, not an empty defer). Each runs on `pilot test`, `--ir` and `--emit c`.

**Left for later 2c groups.** `old(..)` stays a receptor. Today it is a word only inside `ensures(..)`; as a rule it would be read everywhere, so it moves with the contract clauses (`requires`/`ensures`/`invariant`). Those sit after a function signature, where rules are not read yet, so that group needs a declaration-position rule head first.

**As built (2026-11-15).** The file list is as approved: `exits.rs` deleted, `forms.rs` +10 (`ERR_DEFER`, `BECOME`, the two flags), `stmt.rs` −2, `lib.rs` −1; net −33 Rust LOC (21,050 → 21,017).
- One acceptance test differs from the plan. `errdefer }` is not SYN010. A rule word that no code follows is read as a plain name (step 1c: words stay usable as names), so the program fails with DECL001 `undefined name `errdefer`` at the word. `errors/err_errdefer_no_body` pins that. It fails on the old parser, which silently accepted `errdefer }`.

### 13. Step 2c, group 2 — unbuilt keywords (APPROVED BY OWNER 2026-11-15: "Approve §13")

**What.**
- 24 words are registered but not built yet, and every use stops at the seam `keyword.unbuilt` (SEAM001). They are bound to receptor 40, whose second number says where the word may start a construct:
  - **place 0 (anywhere):** `adapter`, `context`, `each`, `database`;
  - **place 1 (declaration/statement):** `actor`, `timeout`, `bounded`, `blueprint`, `cue`, `role`, `route`, `state`, `transition`, `macro`, `unit`, `module`, `connect`, `insert`, `update`, `delete`, `priority`, `jitter`;
  - **place 2 (query clause):** `into`, `values`.
- `crates/pilot-parser/src/unbuilt.rs` (136 lines) recognises them, reads past the construct and reports it. Each word becomes a Pilot `syntax` rule that rejects it with the same message. The Rust that knows "receptor 40" goes.
- Unbuilt *modifiers* (`register_modifier(.., "keyword_unbuilt")`: `async`, `export`, `lvl0`..`lvl100`) are not rule heads and stay with the modifier table (`ModAct::Unbuilt` in `decl.rs`), as does the unbuilt `asm { .. }` block in `stmt.rs`. They keep `report_unbuilt`.

**Matcher addition: hole kind `Rest`.**
- Appended to `HoleKind` (`stdlib/core/syntax/holes.pilot`, ordinal 7).
- It must start with a name, a literal or `{`, which is today's test for "a construct follows the word". Then it reads tokens exactly as `parse_unbuilt` does: up to a `;` (read), past a `{ .. }` block, or before a `,` or a closer it did not open.
- It builds nothing; the rule's `Reject` operand is the hole.
- So `state.push(x);`, `f(each)`, `values[i] = 1;` and `update(x);` still don't match. As before, the word is then an ordinary name (`unbuilt_words_as_names`, `_2`).

**Pilot rules.**
- Each file that binds a word today binds it by rule instead:
  - `stdlib/asyncio/select_syntax.pilot`, `stdlib/contracts/contracts_syntax.pilot`, `stdlib/core/core.pilot` (`macro`);
  - `stdlib/core/syntax/unit_decl.pilot`, `stdlib/core/syntax/visibility.pilot` (`module`);
  - `stdlib/db/sql_syntax.pilot`, `stdlib/realtime/rt_syntax.pilot`.
- For example:

  ```pilot
  pub syntax {
      actor $r:Rest => quote { form(CoreForm::Reject,
          "SEAM001: unconnected seam `keyword.unbuilt`: `actor` is registered but not built yet (docs/keyword_roadmap.md lists the phase that builds it)", $r) };
  }
  ```

- Place-0 words also take a list (`each [1, 2, 3]`), spelled in the head: `each [ $( $e:Expr ),* ]`.
- The clause words `into` and `values` become arms of the `from .. select` rule in `sql_syntax.pilot` that reject with the same message, instead of the matcher's `at_unbuilt_clause` branch.
- A rule word only works where rules are read (statement start, which top-level items fall back to, and expression start) — the same places as today's places 0 and 1.

**Rust file list** (`crates/pilot-parser` only):

| File | Change | LOC |
| :--- | :--- | ---: |
| `crates/pilot-parser/src/unbuilt.rs` | keep only `report_unbuilt` (used for unbuilt modifiers and `asm { }`); delete `UNBUILT_RECEPTOR`, the place constants, `unbuilt_place`, `at_unbuilt`, `at_unbuilt_decl`, `at_unbuilt_clause`, `starts_unbuilt_body`, `parse_unbuilt` | about −110 |
| `crates/pilot-parser/src/rule_arms.rs` | `HOLE_REST = 7` and its reader; drop the `at_unbuilt_clause` branch | about +22 |
| `crates/pilot-parser/src/expr.rs` | delete the receptor-40 arm | −1 |
| `crates/pilot-parser/src/stmt.rs` | delete the `at_unbuilt_decl` check at the top of `parse_stmt` | −1 |
| `crates/pilot-parser/src/decl.rs` | delete the `at_unbuilt_decl` line in the item dispatch | −1 |

Net: about −90 Rust LOC.

**Acceptance.**
- The 26 `errors/err_*_unbuilt` programs and `unbuilt_words_as_names`, `_2` pass unchanged, with the same message and exit code, on `pilot test`. They are error and seed-only programs, but `_2` also runs on `--ir` and `--emit c`.
- New: `receptors/err_rest_hole_needs_start`. A rule `$r:Rest` before `.` is not taken, so the word is a name.
- The keyword matrix "Bound by" for the 24 words becomes "`syntax` rule (unbuilt: `Reject`)".
- Then `pilot test`, `pilot gate` and `cargo test -p pilot-parser` (`damaged_source --include-ignored`).

**Changes in behaviour.**
- A place-1 word inside an expression, followed by a name or `{` (`let y = update x;`), now stops at the seam too. Before, it was an unknown name, which was also an error.
- A place-0 word followed by `(` stays a call, as before.

**As built (2026-11-15).** Rust src is 20,640 → 20,577 (−63, not the estimated −90). `unbuilt.rs` is 136 → 28; `rule_arms.rs` is 857 → 904 (+47, not +22); `expr.rs`, `stmt.rs` and `decl.rs` lose one line each. Only the five listed files changed. Where the build differs from the text above:
- **`Rest` does not start at the `{` after a condition.** Rules are tried at expression start as well as statement start, so as specified `switch state {` or `while x == unit {` (selfhost/analyzer.pilot has one) would have stopped at the seam. A `Rest` hole now fits a `{` only where a struct literal could stand (the parser's `no_struct_lit` flag), so the word before a condition's block stays a name. `receptors/rest_hole` covers this.
- **`Rest` stops before a `;` and does not stop at `,`.** The statement reads the `;` itself. Not stopping at `,` lets the list form be `each [ $r:Rest ]` instead of `each [ $( $e:Expr ),* ]`; the repeated form gave `Reject` no operand for `each []`, so it reported nothing.
- **`macro` is bound in `stdlib/core/syntax/visibility.pilot`, beside `module`, not in `core.pilot`.** `core.pilot` (and `module_ext.pilot`) are read before `holes.pilot` and `forms.pilot` in `BOOTSTRAP_SOURCES`, so a rule there cannot name `Rest` or `form`.
- **The `into`/`values` rules follow the main `from` rule** in `sql_syntax.pilot`. When no rule matches, the first one to read furthest reports, so `errors/err_query_clause` keeps "`from` expects `select`".
- **Tests.** The 26 `err_*_unbuilt` programs, `err_unit_decl_seam`, `err_query_clause`, `err_lvl_shorthand` and `unbuilt_words_as_names`, `_2` pass unchanged on `pilot test`. New: `receptors/rest_hole` (passing: the word before an operator, `)`, `.`, `,` or a condition's `{` is a name) and `receptors/err_rest_hole_needs_start`. `cargo test -p pilot-parser`: `damaged_source --include-ignored` passes; `milestone51` has the same 4 failures as before this change (§10).
- **Changes in behaviour, as built.** A construct word followed by a name, a literal or a `{` (outside a condition) stops at the seam in expression position too, e.g. `let y = update x;` or `let s = state { .. };`. Before, a place-1 word did this only at statement start.

### 14. Step 2c, group 3 — the stray `else` and the dead `select` arm (APPROVED BY OWNER 2026-11-15: "Approve §14")

Two small pieces of seed code that the step 2 matcher and the §13 `Rest` hole already cover.

**A. A stray `else` becomes a Pilot rule.**
- Today `stmt.rs` has a receptor-3 arm. An `else` reached as a statement (no `if` before it) gets the error "`else` must follow the block of an `if` (or a `guard` condition)", and the parser reads past its block or statement.
- `else` stays bound to receptor 3: the seed `if` parser uses it to find its `else`. Only the misuse arm moves.
- New rules in `stdlib/core/syntax/guard.pilot`, which is read after `holes.pilot` and `forms.pilot`:

  ```pilot
  pub syntax {
      else if $r:Rest => quote { form(CoreForm::Reject, "`else` must follow the block of an `if` (or a `guard` condition)", $r) };
      else $r:Rest => quote { form(CoreForm::Reject, "`else` must follow the block of an `if` (or a `guard` condition)", $r) };
  }
  ```

- The rules only see an `else` nobody else took. `if .. else`, and the `guard .. else` rule, read their own `else` before any statement starts there.
- **Change in behaviour:** an `else` followed by nothing that starts code (`else;`) is read as a name (DECL001), as a rule word with nothing after it is (§12). Today it is the same SYN001.

**B. The dead `select { .. }` receptor arm.**
- `stdlib/asyncio/select_syntax.pilot` binds `select` to receptor 33, sub 7, and `expr.rs` has an arm that reads `select { .. }` as a plain block.
- The arm can no longer run. The rule `select $arms:Block` (which stops at the seam `concurrency.select`) is tried first and always matches `select {`, and `is_stmt_receptor_start` already knows `select` as a rule word.
- Its one other effect is in a file that declares a function named `select`: rules are skipped there, so the arm would run `select { .. }` as a block. That is the silent fake audit 2.3 removed.
- Delete the arm and the binding (Rule 3, dead code).

**Rust file list** (`crates/pilot-parser` only):

| File | Change | LOC |
| :--- | :--- | ---: |
| `crates/pilot-parser/src/stmt.rs` | delete the receptor-3 arm in `parse_stmt` | about −8 |
| `crates/pilot-parser/src/expr.rs` | delete the `33 if pl == 7` arm | −1 |

Net: about −9 Rust LOC.

**Pilot files:** `stdlib/core/syntax/guard.pilot` (the two rules); `stdlib/asyncio/select_syntax.pilot` (drop `bind_receptor(nameof(select), 33, 7)`).

**Acceptance.**
- `errors/err_else_without_if` passes unchanged: the same SYN001 at 7:5.
- `errors/err_select_seam` and every `if`/`else`, `guard` and `from .. select` program pass unchanged.
- New: `errors/err_else_if_without_if`, a stray `else if c { .. }`, which gets the same SYN001.
- Then `pilot test` (all three engines on the affected programs), `pilot gate`, and `cargo test -p pilot-parser` (`damaged_source --include-ignored`).
- Matrices: the keyword matrix row for `else` becomes "receptor 3 (`if`); `syntax` rule (stray `else`)". The `select` row already reads "`syntax` rules".

**Amendment (APPROVED BY OWNER 2026-11-15: "Add RejectWord form").** `Reject` puts its error at the code operand, so a stray `else { .. }` was reported at the `{` (7:10), not at the word (7:5) that `err_else_without_if` pins. Added to the file list: `crates/pilot-parser/src/forms.rs`, a variant `RejectWord` (ordinal 11, appended to `CoreForm` in `stdlib/core/syntax/forms.pilot`). It reports from the rule's first word up to the code. The §13 unbuilt rules use it too, so their errors point at the word again, as `parse_unbuilt` did before §13.

**As built (2026-11-15).** Rust src 20,577 → 20,572 (−5, not −9): `stmt.rs` −8, `expr.rs` −1, `forms.rs` +4. Pilot: the two `else` rules in `guard.pilot`, `RejectWord` in `forms.pilot`, the `select` binding dropped, and the 28 unbuilt rules switched to `RejectWord`.
- Tests: `err_else_without_if` (SYN001 at 7:5) and the new `err_else_if_without_if` pass on all three engines, as do `err_select_seam`, the `guard` programs, `else_chains`, the unbuilt tests and the receptors directory. Seed suite 536 passed, 0 failed, 14 known bugs. `control/` on `--ir` and `--emit c` fails `errdefer` and `try_records` as it did before (§12). `pilot gate` passes; `damaged_source --include-ignored` passes; `milestone51` has its same 4 failures (§10).
- Left stale: the doc comment of `crates/pilot-parser/src/unbuilt.rs` names `CoreForm::Reject`; that file was not in this list. It is fixed with the next change that lists it.

### 15. A function's `where` clause is read and thrown away (found 2026-11-15; OPTION B APPROVED BY OWNER 2026-11-15: "Option B: refuse loudly")

**The bug.** `parse_fn` (`crates/pilot-parser/src/decl.rs`, the `while self.is_rec(28) ..` loop after the return type) reads past a `where ..` clause after a function's signature and keeps nothing. So a bound written there is never checked, while the same bound in the parameter list is:

```pilot
interface IShout { fn shout(self) -> str; }
fn twice<T>(x: T) -> T where T: IShout => x;   // runs: `twice(5)` prints 5
fn twice<T : IShout>(x: T) -> T => x;          // TYPE005: `i64` does not satisfy `T : IShout`
```

This is the "parse and discard" AGENTS.md section 11 rule 2 forbids. A `where` after a *type's* header is already refused loudly (`loud.rs` `type_where_loudly`). No program in `stdlib/`, `selfhost/`, `tools/` or `tests/` writes a function `where` clause, so either fix breaks nothing.

**Option A — honour it (about +16 Rust).** `where T : IA + IB, U : IC` adds those bounds to the function's type parameters, exactly as `<T : IA + IB>` does. The checker already enforces them (TYPE005). A name that is not one of the function's type parameters is SYN010.
- `crates/pilot-parser/src/generic_params.rs`: `parse_where_bounds(&mut Vec<GenericParam>, fn_name)` (about +18).
- `crates/pilot-parser/src/decl.rs`: the skip loop becomes a call (about −2).
- Tests:
  - `types/where_bound` (passing; a bound written in `where` admits a type that implements the interface);
  - `errors/err_where_bound` (TYPE005, as for `<T : I>`);
  - `errors/err_where_not_type_param` (SYN010).

**Option B — refuse it loudly (about −2 Rust).** `parse_fn` calls the existing `type_where_loudly` reader, with the message "a `where` clause on a function is not checked yet; write the bound in `<T : I>`".
- `crates/pilot-parser/src/decl.rs` (about −4) and `crates/pilot-parser/src/loud.rs` (one message parameter, about +2).
- Test: `errors/err_fn_where_unchecked`.
- `where` on functions waits until a signature-position `syntax` rule can build it in Pilot. That is the same mechanism the contract clauses (`requires`/`ensures`) need.

Either way the keyword matrix row for `where` gets a note, and `wiring.pilot` gets a row.

**As built, option B (2026-11-15).** Rust src stays 20,572: `loud.rs` +1 (`type_where_loudly` became `where_loudly(message)`, which also stops at `=>` and a contract clause), `decl.rs` −1 (the skip loop calls it). The type message is unchanged; the function message is "a `where` clause on a function is not checked yet; write the bound in `<T : I>`".
- Test: `errors/err_fn_where_unchecked` (SYN001 at 8:24, by hand: 23 characters precede `where`) passes on `pilot test`, `--ir` and `--emit c`. It fails on the old parser, which ran the program.
- Unchanged: `err_type_where`, the `where` query programs, and the contract programs. On `--ir` and `--emit c`, `contracts/contracts` and `control/errdefer` fail as before.
- Seed suite 537 passed, 0 failed, 14 known bugs. `pilot gate` passes; `damaged_source --include-ignored` passes; `milestone51` has its same 4 failures.
- The wiring row is the existing loud-forms row (`err_type_where`'s), extended rather than a new one.

### 16. Step 3 under option (b), and the `milestone51` edit (owner, 2026-11-15: "prop-009 step 3: choose B. all tests and speeds are now approved. approve milestone51 edit")

**Done now (no Rust source changed).**
- **The six `emit_ssa_*` hooks are removed.** They are deleted from `stdlib/core/bridge_lang.pilot` and from the frozen list `airlock_lang_hooks()` in `selfhost/gate/airlock_abi.pilot` (70 → 64 hooks).
  - Nothing called them: `__ssa*` are `syntax` rules over `asm` since step 1b.
  - The seed had no arm for them: no `crates/**` file names them. So option (b) deletes no Rust here.
  - `pilot gate` passes with the shorter frozen list. Seed suite 537 passed, 0 failed, 14 known bugs; `asm/` passes on `--ir` and `--emit c`.
- **`milestone51` edited** (`crates/pilot-parser/tests/milestone51_homoiconic_syntax_and_contraction.rs`, a test file).
  - `test_51_1`'s two user rules are now sequence heads: `name $cond:Expr $body:Block` and `name ( $val:Expr , $min_v:Expr , $max_v:Expr )`.
  - The `nameof(` markers of `switch.pilot` and `guard.pilot` are replaced by the mechanism those files use now, `form(CoreForm::Switch` and `form(CoreForm::Guard`, rather than dropped, so the check still means something.
  - All 4 tests pass; `cargo test -p pilot-parser` passes in full for the first time since step 1c.

**Left: `bind_receptor` itself.** It cannot go yet: about 70 `bind_receptor(..)` lines still bind words the seed reads by receptor kind. They are in `core.pilot`, `module_ext.pilot`, `call_quote.pilot`, `polyglot_syntax.pilot`, `contracts_syntax.pilot`, `lifecycle.pilot`, `properties.pilot`, `try_catch.pilot`, `expr_sugar.pilot` and `for.pilot`.
- The kinds still read by the seed, with their groups:
  - 1/2: `true`/`false`/`none`;
  - 3: `else` for `if`;
  - 4–8: declaration heads;
  - 10: `in`;
  - 19: `catch`;
  - 20–25: `scope`/`enter`/`exit`, accessors, `op`;
  - 26–28: `type`/`enum`/`where`;
  - 29: `as`/`view`;
  - 30: bundle words;
  - 31: `some`;
  - 32: polyglot;
  - 33: contracts, `nameof`;
  - 34: `!`;
  - 36: `stringify`;
  - 37: the call-quote words.
- When the last group moves, a final list deletes:
  - `bind_receptor` from `bridge_lang.pilot` and the frozen list;
  - the receptor table and `receptor()`/`is_rec()` in `crates/pilot-parser` (`tables.rs`, `lib.rs`);
  - every remaining arm.

  That list is itemized for approval then.
- `decl_operator_fn`, `cfg_jump_loop_exit` and `cfg_jump_loop_head` are frozen hooks that still have callers, so they stay.

### 17. Step 2c, group 4 — inline foreign blocks (receptor 32) (OPTION R APPROVED BY OWNER 2026-11-15: "Option R: refuse at seam")

**Today.** `stdlib/polyglot/polyglot_syntax.pilot` binds 25 language words to receptor 32 (`c`, `cpp`, `rust`/`rs`, `python`/`py`, `js`/`javascript`, `ts`/`typescript`, `csharp`/`cs`, `lua`, `sql`, `postgres`/`postgresql`, `mysql`, `sqlite`, `wgsl`, `ptx`, `msl`, `java`, `assembly`). The seed reads `word { .. }`, and `word("..")` with a string literal, into a `PolyglotBlock` node:
- the code: `stmt.rs` `parse_polyglot_inline`, `declared_names.rs` `opens_inline_text`, and an arm each in `expr.rs` and `stmt.rs`;
- the node then **runs nowhere**:
  - the seed VM refuses it ("inline foreign-language blocks cannot run on the seed VM", `pilot-vm/src/lower_expr.rs`);
  - the checker has no case for it;
  - no conformance program writes one.

`wiring.pilot` M58 has "inline block parsing" at `DRAFT` and execution at `SPEC`.

**Option R — refuse them by rule, at a seam (recommended; about −18 Rust).**
- Each word becomes Pilot rules in `polyglot_syntax.pilot`: `word $r:Rest` (a block) and `word ( $r:Rest )` (the string form), with a `RejectWord` template carrying `SEAM001: unconnected seam `polyglot.inline`: inline foreign blocks are not built yet (M58)`.
- The 25 `bind_receptor(.., 32, ..)` lines go.
- Rust deleted:
  - `parse_polyglot_inline` (`stmt.rs`, about −8);
  - `opens_inline_text` (`declared_names.rs`, about −8);
  - the receptor-32 arm in `expr.rs` (−1) and `stmt.rs` (−1);
  - `32 |` in `is_stmt_receptor_start` (`stmt.rs`).
- Fenced blocks (the lexer's `RawDslBlock`) are not touched.
- The build stops at the word instead of at the seed VM, and on every engine, not just the seed.
- M58 "inline block parsing" goes to `SEAM` (`polyglot.inline` registered in `seams.pilot`). M58 builds it again later as Pilot rules: a raw-text hole and a `Foreign` core form.
- Changes in behaviour:
  - `word(x)` now stops at the seam even when `x` is not a string, when the file does not itself declare a function `word`. Before, that was a call.
  - Like every rule word, `word name` / `word 5` stop too; both were errors before.
- Tests:
  - new `errors/err_polyglot_block_seam` (`py { .. }`);
  - new `errors/err_polyglot_text_seam` (`sql("..")`);
  - `types/polyglot_words_as_names` (passing: `c`, `cs`, `ts`, `py` as variables, before `+`, `.`, `)`, and a condition's `{`).

**Option K — keep parsing them, in Pilot (about −6 Rust).**
- A new hole kind `Text` (the raw text between `{` and `}`; `rule_arms.rs` about +6), and a new core form `Foreign` (language, code, string form; `forms.rs` about +8). The rules are `word $code:Text` and `word ( $s:Expr )`.
- The same deletions as option R.
- Nodes and M58's `DRAFT` row stay as they are.
- It keeps a parse that nothing runs yet.

**Extras approved with option R (owner, 2026-11-15: "Approve both").** (1) Fix the `Rest` hole in `rule_arms.rs` so it never starts at a word that goes on with the expression before it; without it, `c as u32` and `c is i64` (a local named `c`) hit the seam. (2) Delete `errors/err_inline_block_text` (it tested the deleted Rust parse). (3) Drop the `let sq = sql("SELECT 1");` line from `milestone51` `test_51_3`.

**As built, option R (2026-11-15).** Rust src 20,572 → 20,564 (−8, not −18): `stmt.rs` −9 (`parse_polyglot_inline` and the receptor-32 arm; `is_stmt_receptor_start` now checks only kind 33), `expr.rs` −1, `declared_names.rs` −10 (`opens_inline_text`), `rule_arms.rs` +12 (extra 1: `goes_on`, an operator word in `infix_ops` or a receptor of the kinds `parse_stmt` already lets follow a word, 19/29/31; it is applied both where `Rest` starts and in the rule-word fallback; a value word such as `true` does not count, so `errors/err_assert_no_parens` keeps its SYN010). The test file `milestone51` lost one line (extra 3). Pilot: `polyglot_syntax.pilot` drops the 23 receptor-32 bindings and adds `w $r:Rest` and `w ( $r:Rest )` for each word, each a `CoreForm::RejectWord` reporting `SEAM001: unconnected seam \`polyglot.inline\``; `seams.pilot` registers `polyglot.inline` (M58). Tests: new `errors/err_polyglot_block_seam`, `errors/err_polyglot_text_seam`, `types/polyglot_words_as_names`; `receptors/rest_hole` gains a `probe as i64` line (expected `4`); `errors/err_inline_block_text` deleted (extra 2). `wiring.pilot` M58 "inline block parsing" is now `SPEC`, blocked by `polyglot.inline`. Verified: seed suite 539 passed, 0 failed, 14 known bugs; `cargo test -p pilot-parser` and `damaged_source --include-ignored` pass; the keyword matrix `--check` passes.

---

## `PROP-010` — Purge Dead Rust Found 2026-11-15: the Unused Rust `comptime` Evaluator and Unreferenced Items

**Status: APPROVED BY OWNER 2026-11-15 ("Approve PROP-010").** Logged under Rule 3: dead Rust found while sweeping for junk. Nothing below is called by any binary. `pilot` (`crates/pilot-cli`) and the other crates' `src/` never reach it. The one exception is section B: legacy test harnesses use it.

**How it was found.** Every `fn`, `const` and type in `crates/*/src` was searched for by name across all `src/` files, excluding its own definition and `pub use` re-exports. The hits below were then read by hand. Trait methods (`Hasher::write_u8`, `Deref::deref`) and in-crate `#[test]` functions were left out. `rust.md` already marks `pilot-analysis` "LEGACY → DELETE CRATE" (`PROP-004`), and this removes the part of it nothing needs.

### A. Unreferenced items (no test uses them)

| File | Delete | LOC |
| :--- | :--- | ---: |
| `crates/pilot-analysis/src/pipeline.rs` | `AnalysisPipeline::analyze_source` (only `analyze` is called, by `pilot-cli/src/run.rs`) | −4 |
| `crates/pilot-analysis/src/types.rs` | the alias `CapabilityPhysics`; the constants `CapabilityBits::IMMUTABLE` and `LINEAR` | −1 (and two items off packed lines) |
| `crates/pilot-lexer/src/token.rs` | `TokenFlags::IS_INACTIVE_PREPROC` (never set or read) | −1 |
| `crates/pilot-vm/src/host.rs` | `record_field` | −5 |
| `crates/pilot-vm/src/vm.rs` | `current_span` | −8 |

### B. The Rust `comptime` evaluator: `crates/pilot-analysis/src/comptime.rs` (166 LOC)

`ComptimeEngine`, `ComptimeValue`, `ComptimeEvaluator`, `BootstrapDiagnostic`, `bootstrap_core_file`, `eval_file_with_tables` and `BootstrapSnapshot` are used by no `src/` file outside `comptime.rs`. `BootstrapSnapshot` is also a second bootstrap: it runs `stdlib/core/bootstrap.pilot` in a VM to list stages, beside the one the parser uses (`pilot_parser::bootstrap_tables`). AGENTS.md §6 forbids that kind of parallel path. Real `comptime` runs in Pilot (`selfhost/ir/interp.pilot`), and the parser ingests bootstrap tables itself (`parse_and_ingest`).

| File | Change |
| :--- | :--- |
| `crates/pilot-analysis/src/comptime.rs` | **delete** (−166) |
| `crates/pilot-analysis/src/lib.rs` | drop `pub mod comptime;` and its `pub use` line (−2) |
| `crates/pilot-analysis/tests/milestone50_lossless_bridge_and_reflection.rs`, `milestone52_pilot_preprocessor_and_receptors.rs`, `milestone53_cst_recovery_and_formatter.rs`, `milestone54_perceus_ownership_and_escape.rs`, `phase1_rename_torture_gate.rs` | They parse a file with the bootstrap tables. Replace `BootstrapSnapshot::global().tables.clone()` + `eval_file_with_tables(src, &mut tables, &mut vm)` / `bootstrap_core_file(..)` with `pilot_parser::bootstrap_tables().clone()` + `pilot_parser::parse_and_ingest(src, &mut tables)`. No assertion changes. |
| `crates/pilot-analysis/tests/milestone02_module_graph_and_comptime_parity.rs` | delete the assertions on `ComptimeEngine::eval_ast_node` (lines ~194–300 and `engine_target_has`): they test the evaluator being deleted. The same facts (`arch`, `pointer_bytes`, `endian`, `env()`) are already checked through the VM in the same test, lines ~274–330, and those stay. This harness already fails 1 of 4 at HEAD. |
| `crates/pilot-analysis/tests/phase1_ironclad_killswitch.rs` | delete block "2. ComptimeEngine state persistence" (~lines 194–207): it tests that the deleted evaluator keeps its layout table. |

Section B's test harnesses don't count toward the `src` LOC that GATE003 measures.

**Net.** About −187 `src` LOC (21,017 → about 20,830). The Rust LOC budget (20,408) is still exceeded.

**Acceptance.**
- `cargo build`.
- `cargo test -p pilot-analysis` for the seven harnesses above: each passes the same tests it passes now, minus the deleted assertions. `milestone02` and `milestone50` already fail 1 and 2 tests at HEAD and must fail the same ones.
- The seed conformance suite and `pilot gate` are unchanged apart from GATE003's number.

**Not included.**
- `pilot_parser::bootstrap_diagnostics` and `PilVirtualMachine::set_hook` are used only by tests, but those tests guard live behaviour (no bootstrap parse errors; the debugger hook).
- The rest of `pilot-analysis` (`infer.rs`, the `__engine` AST bridge `pilot check` runs on) stays until `PROP-004`.


**As built (2026-11-15).** Exactly the files listed. Net −191 `src` LOC (21,017 → 20,826, measured by GATE003; the budget of 20,408 is still exceeded).
- **Deviations from the plan.**
  - `milestone50` lost two assertions the proposal did not list: `snap.arenas.len() >= 8` and `snap.arenas[0].0 == "Pilot.Core.Genesis"`. They described the deleted snapshot's own stage list and have no equivalent.
  - `milestone02`: the proposal said the facts behind its evaluator checks were already covered through the VM. That was only partly true: the VM part checks the `env()` and pending-field seams, but nothing checks the `arch`, `pointer_bytes` and `endian` values through the VM any more. Those checks went with the evaluator.
- **Results.**
  - `test_50_3` now passes. At HEAD it failed at `res.is_ok()`, because the snapshot's separately built tables gave `AnalysisPipeline` errors on a core file; the parser's canonical tables don't.
  - The only failures left are those that fail at HEAD: `milestone02` `test_m2_3` and `milestone50` `test_50_4_and_50_5`. The other seven harnesses pass.
  - The seed suite passes 533 with 0 failures. The gate shows only GATE003.
- **Pilot cleanup in the same change.**
  - Deleted the `comptime.rs` exemption in `selfhost/gate/seed_audit.pilot` and its file cap in `selfhost/gate/manifest.pilot`.
  - Corrected the `wiring.pilot` text.
  - `stdlib/core/bootstrap.pilot` now has no runner: only `milestone50` reads its text. Its header says so, and deleting it is left for an owner decision.
---

## `PROP-011` — Delete the Unrun Bootstrap Stage List `stdlib/core/bootstrap.pilot` (Rust Test Edit Only)

**Status: APPROVED BY OWNER 2026-11-15 ("Approve PROP-011").** The owner asked for this proposal ("Yes, propose it") after `PROP-010`.

**Why.**
- `stdlib/core/bootstrap.pilot` (62 lines) lists the core files to bootstrap, in order (`bootstrap_stage_count`, `bootstrap_stage_path`, `bootstrap_stage_ring`, `bootstrap_stage_at`).
- Its only runner was `BootstrapSnapshot` in `crates/pilot-analysis/src/comptime.rs`, which `PROP-010` deleted. The parser bootstraps from its own list (`BOOTSTRAP_SOURCES` in `crates/pilot-parser/src/tables.rs`).
- So the file is a second, unread copy of the bootstrap order, which AGENTS.md §1.1 says to delete on sight.
- The deletion itself is Pilot. It needs approval only because a Rust test reads the file's text.

**Rust file list** (test only; `src` LOC unchanged):

| File | Change |
| :--- | :--- |
| `crates/pilot-analysis/tests/milestone50_lossless_bridge_and_reflection.rs` | Remove the `FileAuditRule` for `stdlib/core/bootstrap.pilot` (it requires the four stage functions and the `.len` marker). Remove the `test_50_3` text check (`.len`, not `return 8;`). Drop `stdlib/core/bootstrap.pilot` from the list of core files `test_50_3` analyzes; `naming_attr.pilot`, `mem.pilot` and `meta.pilot` stay. Drop contract 1 from the header comment. The test count stays 4. |

**Pilot side** (same change):
- Delete `stdlib/core/bootstrap.pilot`.
- Delete the `Pilot.Core.Bootstrap` → path branches in `selfhost/lsp/handlers.pilot` and `selfhost/pkg/manifest.pilot`.
- Remove its row from `pilot.md` and its path from `docs/concept_matrix.md` C02, and correct the `wiring.pilot` F1 text.

**Acceptance.**
- `cargo test -p pilot-analysis --test milestone50_lossless_bridge_and_reflection` still passes `test_50_3` and fails only `test_50_4_and_50_5`, as now.
- The seed suite and `pilot gate` are unchanged.
- `git grep bootstrap.pilot` finds only history (`MILESTONES_100.md`, `proposals.md`).

**As built (2026-11-15).** As listed. `milestone50` passes 3 tests (`test_50_3` among them) and fails only `test_50_4_and_50_5`, as before. The seed suite passes 533 with 0 failures. `pilot gate` shows only GATE003 (20,826 `src` LOC, unchanged). The remaining mentions of `bootstrap.pilot` are history (`MILESTONES_100.md`, `.agents/AI_CODEBASE_INDEX.md`, this file) and the `wiring.pilot` F1 note that records the deletion.

---

## `PROP-012` — Delete Test-Only Rust: the AST S-Expression Printer, the Rust Module Header Scanner and `TokenKind::is_keyword`

**Status: APPROVED BY OWNER 2026-11-15 ("Approve PROP-012").** Logged under Rule 3. These three pieces of `src` are called by no binary, only by tests. Found by searching every `pub fn` in `crates/*/src` (outside `#[cfg(test)]`) for callers in other `src` code.

| Piece | Where | Why it is dead | LOC |
| :--- | :--- | :--- | ---: |
| **A.** `AstArena::to_sexpr` / `write_sexpr` | `crates/pilot-ast/src/lib.rs` | Only the crate's own unit test calls it. Pilot has its own printer (`sh_ast_emit_sexpr` in `selfhost/ast.pilot`). | −53 |
| **B.** `parse_header`, `ModuleDecl`, `at_module_decl`, `after_group` | `crates/pilot-parser/src/modules.rs` (+ the `pub use` in `lib.rs`) | It's a Rust module header scanner, which AGENTS.md §1.1 forbids, and `PROP-003` already lists `parse_header` for removal. Nothing in `src` calls it: Pilot reads `pkg`/`use` headers itself (`selfhost/loader/module_graph.pilot`), and `pilot-cli` links with `parse_unit`, which stays. | about −115 |
| **C.** `TokenKind::is_keyword` | `crates/pilot-lexer/src/token.rs` | Only `milestone52_seed_lexer_and_preprocessor.rs` calls it, and every call follows an `assert_eq!` on the exact `TokenKind`, so it adds nothing. | −5 |

**Rust file list.**

| File | Change |
| :--- | :--- |
| `crates/pilot-ast/src/lib.rs` | delete `to_sexpr` and `write_sexpr`; in the unit test, delete the arena setup and the `to_sexpr` assertion (the size and precedence assertions stay) |
| `crates/pilot-parser/src/modules.rs` | delete `ModuleDecl`, `parse_header`, `at_module_decl`, `after_group`; `parse_unit` and `parse_module_decl` stay |
| `crates/pilot-parser/src/lib.rs` | `pub use modules::parse_unit;` only |
| `crates/pilot-parser/tests/module_header.rs` | **delete** (it tests only `parse_header`) |
| `crates/pilot-parser/tests/damaged_source.rs` | drop the `parse_header(&text)` call; the crash check still runs `parse(&text)` on every damaged input |
| `crates/pilot-lexer/src/token.rs` | delete `is_keyword` (and the `impl` block if it is left empty, keeping `ColonColon`) |
| `crates/pilot-lexer/tests/milestone52_seed_lexer_and_preprocessor.rs` | delete the four `is_keyword` assertions |

**Net.** About −173 `src` LOC (20,826 → about 20,653).

**Acceptance.**
- `cargo build`.
- `cargo test -p pilot-ast`, `-p pilot-lexer` and `-p pilot-parser`: the same results as now, minus the deleted tests. `milestone51`'s 4 known failures are unchanged.
- `damaged_source --include-ignored` passes.
- The seed suite and `pilot gate` are unchanged apart from GATE003's number.

**Not included.**
- `ByteSpan::merge`: two `pilot-analysis` harnesses use it.
- `PilVirtualMachine::set_hook`: four harnesses drive the VM debug hook through it, and it is the seam the Pilot DAP work needs.
- `bootstrap_diagnostics`: its test guards that every bootstrap source parses.

**As built (2026-11-15).** Exactly the files listed. Net −186 `src` LOC (20,826 → 20,640, measured by GATE003; the budget of 20,408 is still exceeded).
- `token.rs`: `impl TokenKind` keeps only `ColonColon`.
- The `pilot-ast` unit test is renamed `ast_node_is_strictly_16_bytes`.
- The `damaged_source` doc comment no longer mentions `parse_header`.
- **Results.**
  - `pilot-ast`, `pilot-lexer` and `pilot-parser` tests pass, except `milestone51`'s 4 known failures.
  - `damaged_source --include-ignored` passes.
  - The seed suite passes 533 with 0 failures; `pilot gate` shows only GATE003.
- **Budget raised by the owner (2026-11-15).** After this proposal the owner raised `paused_max_total_rust_loc()` from 20,408 to 21,000 (`selfhost/gate/manifest.pilot`). At 20,640, `pilot gate` passes all 4 gates.

---

## `PROP-013` — Link a Run's Extra Toolchain Files in the Loader's One Pass

**Status: APPROVED BY OWNER 2026-11-15 ("Approve PROP-013") and built.** The owner chose this route on 2026-11-15 ("Rust proposal, one pass") after the `PROP-002b` `Map` slice failed C2.

**Problem (measured 2026-11-15, release build, best of 11 runs of `fn main() { print(1); }`).**
- `pilot run` with no extra files linked: 427,503 µs. With `stdlib/core/primitives/map.pilot` in `cli_run_links()`: 458,915 µs. With an empty `pkg` file in its place: 454,137 µs.
- So the cost (about 27–31 ms per run) does not depend on the file. It comes from `compile_entry` (`crates/pilot-cli/src/run.rs`), which calls `link_plan` once for the program and then once more for each extra file (`tail_links`). Each `link_plan` call compiles and runs the Pilot module loader (`LOADER`, `mg_link_plan`) again, which re-indexes the search roots (`mg_build_index` walks every `.pilot` file under them).
- That puts five of the eight C2 benchmarks over the owner's 5% limit (see the `PROP-002b` `Map` slice, "C2 result").

**Change.** The loader takes the extra files as more entry points of the same graph, so a run with extra files costs one loader pass, as a run without them does.

| File | Change |
| :--- | :--- |
| `selfhost/loader/module_graph.pilot` (Pilot) | New `mg_link_plan_with(entry_file, toolchain_root, extras: List<str>)`: the graph starts from the prelude, then each file in `extras`, then the entry, so each extra and what it imports come before the program, and a file the program already links is linked once (units are keyed by path). `mg_link_plan(entry, root)` becomes `mg_link_plan_with(entry, root, [])`; `mg_build_graph_sharing` gains the `extras` parameter, and its other caller (`link_lines.pilot`) passes `[]`. About +10 lines. |
| `crates/pilot-cli/src/run.rs` | `link_plan(path)` becomes `link_plan_with(path, &[])`. New `link_plan_with(path, extras)` builds the call `mg_link_plan_with("<path>", "<root>", ["<extra>", ..])`. In `compile_entry`, the extras are joined to the toolchain root as now and passed to one `link_plan_with` call; the second `link_plan` loop, the `seen`/`canonical` de-duplication and the entry re-ordering are deleted (the loader orders and de-duplicates). A `tail`, when given, is still pushed last. |

**Net Rust.** About −11 `src` lines (about −26, +15), in one file. No new Rust names: `mg_link_plan_with` is a loader function name of the same kind as the `mg_link_plan` the file already calls.

**Acceptance.**
- `cargo build`, `cargo fmt --check`, `cargo test -p pilot-cli`.
- Start-up: `pilot run` of the one-line program with `map.pilot` in `cli_run_links()` is within 2% of the run without it (427,503 µs measured).
- **C2:** `target/release/pilot run benchmarks/native/record.pilot -- --check` passes with the `Map` slice in place.
- The seed suite, `--ir` and `--emit c` give the same results as before this change (seed 542 passed, 0 failed, 14 known bugs reproduced). `pilot gate` passes.
- Link order: `collections/map_owner_methods` and `collections/map_declared_property` pass on the seed (they need `map.pilot` linked before the program), and `pilot run --coverage` (which links `coverage.pilot` the same way) still reports.

**Not included.** A `.pilot` file at the repository root makes `pilot run` spin at 100% CPU (seen 2026-11-15, not investigated; the loader's root index walks the whole tree). It is a separate finding.

**As built (2026-11-15).** Exactly the files listed, plus `selfhost/loader/link_lines.pilot` (its one call of `mg_build_graph_sharing` passes `[]`; the plan named it). Rust `run.rs` 431 → 420 (−11: +18, −29).
- **Start-up** (best of 11, `fn main() { print(1); }`, with `map.pilot` linked): 409,007 µs; without the link: 420,389 µs (both measured while `cargo test` ran; the 30 ms second pass is gone).
- **C2** (`record.pilot -- --check`, exit 0): fib 639,964 (0.90x), numeric_loop 1,751,318 (0.96x), string_build 416,793 (0.89x), map_workload 469,970 (0.94x), records 1,746,513 (0.99x), string_compare 1,286,734 (0.94x), string_find 1,101,946 (0.96x), string_map 495,007 (0.92x). Part of the gain over the baseline is likely run-to-run variance.
- **Suites:** seed 542 passed, 0 failed, 14 known bugs reproduced. `--ir` and `--emit c` match the run before this change, directory by directory. `pilot gate`: all 4 gates pass.
- **De-duplication:** a program that writes `use Pilot.Core.Primitives.Map;` itself, with `map.pilot` also linked, runs (`m.clear(); m.count` prints `0`).
- **`cargo test --release -p pilot-cli --no-fail-fast`:** 7 tests fail, and the same 7 fail with `run.rs` restored to its previous text: `a_built_binary_reads_its_arguments_and_standard_input`, `an_unlowered_construct_stops_at_its_seam`, `bare_coverage_turns_on_every_mode`, `coverage_on_a_struct_measures_its_methods`, `inherited_steps_follow_their_policy` (DECL001 `Coalesce`), `the_benchmarks_build_natively_and_match_their_c_references`, `well_typed_programs_get_no_diagnostic`. Not yet split into HEAD failures and failures from this session's uncommitted work.
- **Not run:** `cargo fmt --check` (the `rustfmt` component is not installed for the stable toolchain). The new lines are at most 106 columns; `run.rs` already had lines up to 125.

---

## `PROP-014` — Update the Benchmark Baseline Test for the Seed Column (Rust Test Edit Only)

**Status: APPROVED BY OWNER 2026-11-15 ("Approve PROP-014") and built.**

**Problem (found 2026-11-15).** `PROP-002a` C2 (owner threshold "5% slower max") added the seed's time to `benchmarks/native/baseline.tsv`, which `record.pilot -- --check` needs. Each row went from 4 fields to 6: `benchmark seed_us pilot_us c_us seed_over_c pilot_over_c`. The Rust test `the_benchmarks_build_natively_and_match_their_c_references` (`crates/pilot-cli/tests/cli_f6.rs:181`) still requires exactly 4 fields per row, so it fails since commit `03ee3ab`. It passes at `2a82f59`, the commit before. No `src` file is involved.

**Change.**

| File | Change |
| :--- | :--- |
| `crates/pilot-cli/tests/cli_f6.rs` | In the baseline check, expect 6 fields per row, not 4, and say in the comment which they are (benchmark, seed µs, native Pilot µs, C µs, seed/C, Pilot/C). Add `records` to the list of benchmarks it looks for; the file has a row for it and the list left it out. |

**Net Rust.** `0` `src` lines; the test changes by about 2 lines.

**Acceptance.** `cargo test --release -p pilot-cli --test cli_f6 the_benchmarks_build_natively_and_match_their_c_references` passes, and the rest of `cli_f6` is unchanged (`a_built_binary_reads_its_arguments_and_standard_input` already failed at `2a82f59`).

**As built (2026-11-15).** Exactly the file listed (+2 lines: the comment line and the field count; `records` added to the list). `cargo test --release -p pilot-cli --test cli_f6`: 9 passed, 1 failed; the failure is `a_built_binary_reads_its_arguments_and_standard_input`, as at `2a82f59`.

---

## `PROP-015` — Compile an `=> expr` Function Body Against Its Return Type, Delete the Dead `ProbeArg::Engine`, and (C) Let a Block's Last `for` Collect on the Seed

**Status: DRAFT, awaiting owner approval.** Found 2026-11-15 while clearing the `pilot-cli` test failures that predate this session's work (they fail at `2a82f59` too).

**A. Known bug `seed.expr_body_variant_hint`.**
- An `=> expr` function body is compiled with plain `compile_node`, without the declared return type as the expected type. `return expr;` uses `compile_with_hint(.., cur_ret_type)` (`lower_stmt.rs` `compile_return`), and so does a typed `let`.
- So a target-typed variant whose name two enums share is DECL001 "undefined" in an `=> expr` body only. `fn p() -> InheritPolicy => ::Coalesce;` fails because `Coalesce` is also an operator level (`stdlib/core/operators/levels.pilot`, PROP-008). `crates/pilot-cli/tests/cli_attrs.rs` `inherited_steps_follow_their_policy` fails on it.
- Repro: `tests/conformance/known_bugs/expr_body_variant_hint.pilot` (prints `1`, `1`, `1` by hand; today DECL001 on all three engines), registered in `bug_registry()`.

**B. Dead Rust: `ProbeArg::Engine`.** The coverage weaver no longer passes an engine operation as a probe argument: the `::Speed` entry points read the clock themselves (`Clock.now_ns` in `stdlib/core/coverage.pilot`). The old argument named `time_now_us`, an operation that no longer exists, so every `::Speed` run stopped with "cannot apply `-` to null and null". The Pilot side (`ProbeArgKind::Engine`, its branch in `selfhost/ir/lower_probe.pilot`) is deleted. The Rust variant and its two arms have no producer now.

**C. Known bug `check.collecting_for_in_block`, seed half** (added 2026-11-15).
- A collecting `for` that is a block's value (`let big = { for x in xs { yield x * 10; } };`, or the branches of `let c = if .. { for .. } else { for .. };`) must give the list. The Pilot half is built: `selfhost/check/generators.pilot` `for_collects` counts a block's last `for` as its value when the block's value is used (a `let`/`var` initializer, an argument, a returned or assigned value, a branch of an `if` whose value is used; never a function, lambda or loop body), and `selfhost/ir/lower.pilot` `lower_block` takes that `for` as the block's value. `known_bugs/block_value_collecting_for` passes on `--ir` and `--emit c`.
- The seed still fails: `compile_block` (`lower_stmt.rs`) sets `stmt_for` on every `for` item, so the last one never collects, its `yield` makes `main` a generator, and nothing runs.
- Change: `compile_block` leaves `stmt_for` unset for its last item when the block's value is wanted. That needs a `want_value` flag on `BytecodeCompiler` (`lib.rs`), set while `compile_let` (`lower_stmt.rs`) compiles its initializer and while an `if` whose value is wanted compiles its branches (`lower_expr.rs`, `IfExpr`), and cleared for loop and function bodies. Sized when the owner approves the direction; about +15.
- Acceptance: `known_bugs/block_value_collecting_for` passes on all three engines, moves to `tests/conformance/generators/` with its expected output unchanged, and its `bug_registry()` entry is deleted; `generators/*` and the seed suite are otherwise unchanged.

**Rust file list.**

| File | Change |
| :--- | :--- |
| `crates/pilot-vm/src/lower_stmt.rs` | In the function-body compile, an `=> expr` body (not a `BlockExpr`) is compiled with `compile_with_hint(arena, body_node, self.cur_ret_type.clone())`; a block body is unchanged. About +3. Part C: `compile_block` leaves `stmt_for` unset for its last item while `want_value` is set; `compile_let` sets it around its initializer. |
| `crates/pilot-vm/src/lib.rs` | Part C: the `want_value: bool` field on `BytecodeCompiler`, saved and restored with the other per-function state. |
| `crates/pilot-vm/src/lower_expr.rs` | Part C: an `IfExpr` passes `want_value` on to its branch blocks. |
| `crates/pilot-vm/src/lower_probe.rs` | Delete `ProbeArg::Engine(String)` (and its doc line), the `5 => ProbeArg::Engine(..)` arm of `from_code`, and the `ProbeArg::Engine(op) => self.emit_intrinsic(op, &[])` arm. About −4. |

**Net Rust.** About −1 `src` line for A and B; about +15 more with C.

**Acceptance.**
- `cargo build`; `cargo test -p pilot-vm` and `-p pilot-cli` give the results of today minus `inherited_steps_follow_their_policy`, which passes.
- `known_bugs/expr_body_variant_hint` is fixed on all three engines: it moves to `tests/conformance/types/` with its expected output unchanged, and its `bug_registry()` entry is deleted.
- The seed suite, `--ir`, `--emit c` and `pilot gate` are otherwise unchanged; `types/attr_coverage` and `cli_coverage` still pass.

**Not included.**
- `pilot ir run` reports LOOP005 ("`continue a..b` is only supported inside a `for` loop") for a `continue 5..6;` that is inside a `for` (`tests/ir/unlowered_continue_range.pilot`; `cli_f5` `an_unlowered_construct_stops_at_its_seam` fails on it). The check is in the legacy `selfhost/analyzer.pilot`, which walks `ast_parent` links built by `crates/pilot-analysis/src/infer.rs`; since `for` became a `syntax` rule (PROP-009 step 2) that walk no longer reaches the `for`. `pilot check` and `pilot run` report the right SEED001. Not yet diagnosed far enough to name a fix. The same analyzer warns LOOP005 "Potential infinite loop ... no 'break'" for `while true { guard some x = f() else { break; }; .. }`: it does not see a `break` inside a `guard`'s `else` (also a `syntax`-rule form since PROP-009). `runtime/stdin_lines` is written with `if some .. else` so `cli_f3` `check_passes_a_valid_program_without_running_it` stays clean.
- The checker accepts a `T?` where a `T` is expected (`f() + 1` with `f() -> i64?`). That is its own design question (a new diagnostic code).

---

## `PROP-003` through `PROP-006` — Subsequent Contraction Waves (Summary)

Each wave below will be expanded into an itemized, closed-file-list proposal before requesting owner approval:
1. **`PROP-003` (Wave 3):** Migrate remaining domain string/collection/cast helpers, `checked.rs`, `parse_header`, and parser semantic passes (`query.rs`, `comptime_if.rs`, `fstring.rs`, `pilot_impls.rs`, `attr_hooks.rs`, `num_suffix.rs`, `loud.rs`, `contracts.rs`, `with_scope.rs`) into `stdlib/` and `selfhost/`, and finish moving every remaining raw `__engine` call onto its owner (`PROP-002a` Amendment A1: one op, one owner; there is no `bridge_os.pilot` and no 2-file ceiling). Moving `attr_hooks.rs` must keep the one-call `comptime fn` expansion at bare-name call sites that A1's genesis-hook owners rely on (`attr_hooks.rs:55`), proven by a conformance program before the Rust file is deleted.
2. **`PROP-004` (Wave 4):** Delete `crates/pilot-analysis` (`6` `src` files, `821` LOC) after migrating remaining `crates/pilot-analysis/tests/milestone*.rs` checks into `.pilot` conformance suites.
3. **`PROP-005` (Wave 5):** Implement `selfhost/vm/ir_to_pilc.pilot` (`PilIR` $\rightarrow$ `.pilc`) and delete all `24` `crates/pilot-vm/src/lower_*.rs` files, `vm_gen.rs`, `codes.rs`, and `BytecodeCompiler` in `crates/pilot-vm/src/lib.rs:60-1015` (`-7,833` Rust `src` LOC). It lowers from PilIR after the `PROP-007` inlining pass, so the temporary seed inlining in `lower_call.rs` can go with the rest.
4. **`PROP-006` (Wave 6):** Ship precompiled `toolchain.pilc` booted directly by `PilVirtualMachine` and delete `crates/pilot-{lexer,parser,ast,span,intern}` + `crates/pilot-cli/src/{link,check}.rs` (`~ -5,950` Rust `src` LOC), reaching the **`~1,800` LOC irreducible VM + `bridge.rs` seed floor**.
