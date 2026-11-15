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
| **`PROP-002a`** | Define the 3-File Airlock Under the B+ Freeze (51 scalar/pointer `HostOp`s), Retire `pilot-rt`, Consolidate Stage-A Natives & Arm `GATE021` | `<= 0` net (`9` $\rightarrow$ `8` crates; `-683` `pilot-rt`) | **`APPROVED — PAUSED FOR AMENDMENT A1 ("one op, one owner")`** |
| **`PROP-002b`** | Bucket B/D `.pilot` Migration to Owner Methods (`~1,875` `map_*` + `~800` `ast_*` sites), Retire Transitional Native Bindings & `MapBuffer` Duplicate, Retire `heap.rs`/`host.rs` | To be itemized | `DRAFT — ITEMIZE BEFORE APPROVAL (after PROP-007)` |
| **`PROP-007`** | Implement `[Inline]`: PilIR Pass (Pilot), Forced Mode for `[EngineOp]` Owners, Seed Lowering (Rust) | Seed part to be itemized; offset by native-binding dispatch removal | `DRAFT — PILIR PART MAY START; SEED PART NEEDS APPROVAL` |
| **`PROP-008`** | Every Operator Declared by an Attribute on Its Interface or `syntax` Rule (Named Levels, Closed Parse Shapes), and Comptime User-Defined Operators | Lexer/parser operator code removed; to be itemized | `DRAFT — HOOK SHAPE DECIDED IN A1, BEFORE THE FREEZE` |
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
  - `tests/conformance/runtime/bridge_os.pilot` failed with `TYPE001` (`str.byte_at` takes `usize`, returns `u8`) on `f4` before A1. It is renamed `runtime/host_ops.pilot` and casts explicitly. Not yet rerun.
  - `std/proc` `proc_make_pipe` declared its outputs `ref` instead of `var ref` (committed in `7920302`), so pipes were never wired and `pilot test` captured no output from any case. Fixed; the checker gap that allowed it is known bug `check.assign_readonly_ref_param`.
  - `bridge.rs` created pipes without close-on-exec, so a spawned child inherited the write end of its own stdin and never saw end-of-input (`runtime/stdin_lines` hung the suite). Both ends are now close-on-exec. `bridge.rs` is in this proposal's file list; this is an implementation fix, not a change to any frozen item.
  - The two pipe fixes above also explain the `pilot-cli` `cli_attrs` failures on `f4` (`the C compiler rejected …` with empty compiler output: `cc` ran but its output and exit code were lost) and the `pilot gate --full` litmus in `milestone01` `m1_3` (it runs `pilot test` in a sandbox). After the fixes, `debug_traces_natively` passed on its own, and six simultaneous `pilot build`s of `attr_debug.pilot` all succeeded. The full `cli_attrs` and `m1_3` runs have not been repeated.
  - `crates/pilot-vm/tests/runtime_memory.rs` (in this proposal's file list) probed `UnitHeader.read(path, 0)`, `h.package` and `h.imports`, which `selfhost/loader/module_graph.pilot` no longer has, so the raw VM read a null and faulted in `str_len`. The probe now uses `UnitHeader.read(path)`, `package_name()` and `decls`; the expected counts (2, then 3) are unchanged. Not yet rerun.
  - `File.list_dirs` / `File.walk_files` (`stdlib/core/file.pilot`) treated a symbolic link to a directory as a file (op 27 reports links as `stat_kind_symlink`), so `pilot gate` on a workspace whose crates are linked (`milestone01` `m1_2`) missed every seam in them and failed `GATE012` six times. A link now counts as what it points to. `pilot gate --json` on that overlay passes 4/4 after the fix.

**Two checks before resuming** (results recorded here):
1. **C1 — Constructor representation.** A conformance program that tests whether a constructor body can produce an engine handle as `self` on the seed (today `Map`'s constructor is a native binding to `map_new`).
   - If it can, owning types keep no constructor binding.
   - If not, each owning type keeps exactly one constructor binding, to its `priv new_internal`, listed in `TRANSITIONAL_NATIVE_BINDINGS`.
   - Either way, Stage-A `Map`/`List` declare no fields and carry `[StandIn(…, "F6")]` (precedent: `GeneratorFrameHandle`, `stdlib/core/generator.pilot:29`).
   - **Result (2026-11-14): no.** On the seed, `self = __engine("map_new")` in a constructor works: the program printed `7`, as expected. On PilIR (`pilot ir run`) the same program stops at the registered seam `ir.lower_construct` (node kind 102). So for now each data owner in `PROP-002b` keeps one constructor binding, listed in `transitional_native_bindings()`, until that seam is connected.
2. **C2 — Speed baseline.** `pilot bench` on the seed before and after, gated by a regression threshold the owner sets.

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
- **Not in scope:** `Reflect`/`PhysicsRegistry` lives in `crates/pilot-analysis` and retires in `PROP-004`. `PROP-004` must name the pure-Pilot successor for layout and symbol reflection (`AGENTS.md` §6, `pilot inspect`), and register a `reflect.type_meta` seam plus a `::Spec` wiring row for anything not built.

---

## `PROP-007` — Implement `[Inline]` (Required Before Owners Replace Native Bindings)

- **Status:** **`DRAFT`**. The PilIR part is pure Pilot and may start. The seed part changes Rust and needs an itemized file list and owner approval.
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

- **Status:** **`DRAFT — HOOK SHAPE MUST BE DECIDED BEFORE THE B+ FREEZE (fold into Amendment A1)`**. Step 1 is pure Pilot plus one hook. Steps 2 and 3 change Rust and need an itemized file list and owner approval.
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

---

## `PROP-003` through `PROP-006` — Subsequent Contraction Waves (Summary)

Each wave below will be expanded into an itemized, closed-file-list proposal before requesting owner approval:
1. **`PROP-003` (Wave 3):** Migrate remaining domain string/collection/cast helpers, `checked.rs`, `parse_header`, and parser semantic passes (`query.rs`, `comptime_if.rs`, `fstring.rs`, `pilot_impls.rs`, `attr_hooks.rs`, `num_suffix.rs`, `loud.rs`, `contracts.rs`, `with_scope.rs`) into `stdlib/` and `selfhost/`, and finish moving every remaining raw `__engine` call onto its owner (`PROP-002a` Amendment A1: one op, one owner; there is no `bridge_os.pilot` and no 2-file ceiling). Moving `attr_hooks.rs` must keep the one-call `comptime fn` expansion at bare-name call sites that A1's genesis-hook owners rely on (`attr_hooks.rs:55`), proven by a conformance program before the Rust file is deleted.
2. **`PROP-004` (Wave 4):** Delete `crates/pilot-analysis` (`6` `src` files, `821` LOC) after migrating remaining `crates/pilot-analysis/tests/milestone*.rs` checks into `.pilot` conformance suites.
3. **`PROP-005` (Wave 5):** Implement `selfhost/vm/ir_to_pilc.pilot` (`PilIR` $\rightarrow$ `.pilc`) and delete all `24` `crates/pilot-vm/src/lower_*.rs` files, `vm_gen.rs`, `codes.rs`, and `BytecodeCompiler` in `crates/pilot-vm/src/lib.rs:60-1015` (`-7,833` Rust `src` LOC). It lowers from PilIR after the `PROP-007` inlining pass, so the temporary seed inlining in `lower_call.rs` can go with the rest.
4. **`PROP-006` (Wave 6):** Ship precompiled `toolchain.pilc` booted directly by `PilVirtualMachine` and delete `crates/pilot-{lexer,parser,ast,span,intern}` + `crates/pilot-cli/src/{link,check}.rs` (`~ -5,950` Rust `src` LOC), reaching the **`~1,800` LOC irreducible VM + `bridge.rs` seed floor**.
