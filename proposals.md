# `proposals.md` — Mandatory Owner-Approval Ledger for All Rust Changes

> **Binding Governance Rules ([`AGENTS.md`](file:///home/moony/Documents/dev/projects/Pilot/AGENTS.md) §1):**
> 1. **Zero Unapproved Rust Changes:** **No** Rust file (`crates/**`, `Cargo.toml`) may ever be created, modified, or deleted until a proposal below is explicitly marked **`[STATUS: APPROVED BY OWNER]`**.
> 2. **Strict File-Scope Confinement:** When executing an approved proposal, edits, creations, and deletions are **strictly restricted to ONLY the exact Rust file paths listed in that proposal**. If an additional Rust file needs to be touched, stop immediately and amend the proposal for owner approval first.
> 3. **Immediate Dead / Legacy Rust Registration:** Whenever any agent or developer discovers dead, unreferenced, or superseded Rust code during any task, they MUST halt before moving on and log its exact file paths here for immediate owner-approved deletion.
> 4. **Permanent 3-File Airlock Immutability Freeze:** Once `PROP-002` is approved and [`stdlib/core/bridge_lang.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/bridge_lang.pilot), [`stdlib/core/bridge_os.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/bridge_os.pilot), and [`crates/pilot-vm/src/bridge.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm/src/bridge.rs) are written, **zero future proposals may ever modify, delete, or work around any of those 3 files**.
> 5. **Mandatory Same-Change Map & Matrix Sync:** Every executed proposal MUST update [`rust.md`](file:///home/moony/Documents/dev/projects/Pilot/rust.md), [`pilot.md`](file:///home/moony/Documents/dev/projects/Pilot/pilot.md), [`docs/keyword_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/keyword_matrix.md), [`docs/concept_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/concept_matrix.md), and [`selfhost/gate/manifest.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/manifest.pilot) in the exact same commit.
> 6. **Break Loud & Seam Over Rust Retention:** Shrinking Rust takes absolute priority over keeping legacy Rust-backed `.pilot` paths temporarily green. When deleting misplaced Rust in an approved proposal breaks `.pilot` callers or unbuilt passes, **leave them loudly broken in Pilot via `seam(...)` (`SEAM001`, registered in `seams.pilot` / `known_bugs`)** and downgrade affected rows in `wiring.pilot`, `keyword_matrix.md`, and `concept_matrix.md`—never keep legacy Rust shims alive to avoid breaking Pilot callers.

---

## Proposal Index

| ID | Title | Net Rust `src` Delta | Status |
| :--- | :--- | ---: | :--- |
| **`PROP-001`** | Purge Discovered Dead Rust Crates/Modules, Remove Hardcoded String Leaks & Split Packed `pilot-parser/src/lib.rs` (`MOD001`) | `-399` net (`20,812` $\rightarrow$ `20,413` `src` LOC; `-1,439` unpacked) | **`COMPLETED (APPROVED BY OWNER)`** |
| **`PROP-002`** | Define & Permanently Freeze the 3-File Airlock (`bridge_lang.pilot`, `bridge_os.pilot`, `bridge.rs`) & Consolidate Host Ops | `~ -450` LOC | **`DRAFT — AWAITING DESIGN SIGN-OFF BEFORE FREEZE`** |
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

## `PROP-002` — Define & Permanently Freeze the 3-File Airlock (`bridge_lang.pilot`, `bridge_os.pilot`, `bridge.rs`)

- **Status:** **`DRAFT — AWAITING OWNER DESIGN SIGN-OFF BEFORE PERMANENT FREEZE`**
- **CRITICAL FREEZE WARNING:** Per owner policy, the moment these 3 files (`stdlib/core/bridge_lang.pilot`, `stdlib/core/bridge_os.pilot`, `crates/pilot-vm/src/bridge.rs`) are approved and written, **they can NEVER be modified, deleted, or worked around**. Every `HostOp` required across Pilot 1.0 and Pilot 2.0 (until `M95` native self-hosting retires Rust completely) is therefore specified below for owner review prior to creation.

### Proposed Closed `#[repr(u8)] pub enum HostOp` Table (`crates/pilot-vm/src/bridge.rs` — `42` Primitives)

| Group | `HostOp` Variants (1–5 Line Raw OS / Slab / Slot Primitives Only — Zero Domain Logic) |
| :--- | :--- |
| **0. Raw Memory Slabs & Pointers (`9`)** | `SlabAlloc(size, align) -> usize`, `SlabRealloc(ptr, old_sz, new_sz, align) -> usize`, `SlabFree(ptr, size, align)`, `MemCopy(dst, src, len)`, `MemSet(dst, byte, len)`, `MemCmp(a, b, len) -> i64`, `LoadInt(ptr, width_signed) -> i64`, `StoreInt(ptr, val, width)`, `LoadF64(ptr) / StoreF64(ptr, val)` |
| **1. VM Bootstrap Heap Slots (`8`)** | `StrBytesLen(s) -> i64`, `StrByteAt(s, i) -> i64`, `StrSliceBytes(s, a, b) -> str`, `StrFromBytes(ptr_or_list) -> str?`, `SlotListOps(new/len/get/set/push/pop/trunc)`, `SlotMapOps(new/len/get/set/has/rem/keys)` *(backs Ring-1 collections during `PilVM` bootstrap)* |
| **2. Process Streams, Faults & Seams (`4`)** | `WriteStdout(bytes_or_str)`, `WriteStderr(bytes_or_str)`, `ReadStdin(max_bytes) -> bytes`, `TrapFault(code_kind, id, msg) -> !` *(handles both `fault` and `SEAM001` `seam` traps)* |
| **3. Filesystem Syscalls (`9`)** | `FsReadBytes(path) -> res`, `FsWriteBytes(path, data, append) -> res`, `FsStat(path) -> (exists, kind, size, mtime_ns)`, `FsReadDir(path) -> list<str>`, `FsMkdir(path, parents) -> bool`, `FsRemove(path, recursive) -> bool`, `FsRename(src, dst) -> bool`, `FsCanonicalize(path) -> str?`, `FsCwdChdir(op, path) -> str?` |
| **4. Subprocess, Env & Clock (`7`)** | `ProcSpawn(cmd, args, cwd, env, stdin) -> handle`, `ProcPollWait(handle, block_ms) -> (done, code, out, err)`, `EnvGetList(key_or_all)`, `ProcArgsExePid(which)`, `ClockNowNs(monotonic: bool) -> i64`, `SleepNs(ns)`, `OsRandomBytes(len) -> bytes` |
| **5. TCP/UDP Sockets & Dynamic C-ABI (`5`)** | `SockOpenConnectBind(kind, addr, port) -> fd`, `SockReadWriteClose(fd, op, buf_or_len)`, `DlOpen(path) -> usize`, `DlSym(lib, name) -> usize`, `FfiCallFrame(fn_ptr, abi_desc_ptr, args_ptr, ret_ptr) -> i64` |

### Exact Rust Files Scoped for `PROP-002` (Subject to Final Confirmation Upon Sign-Off)
- **CREATE (`1` file — Permanently Frozen Once Written):** `crates/pilot-vm/src/bridge.rs`
- **DELETE (`4` files — Superseded by `bridge.rs` & `pilot-vm/src/value.rs`):** `crates/pilot-vm/src/heap.rs`, `crates/pilot-vm/src/host.rs`, `crates/pilot-rt/Cargo.toml`, `crates/pilot-rt/src/{lib,value,stdlib,c_abi,time}.rs` *(after moving `PilotVal` cleanly to `crates/pilot-vm/src/value.rs`)*
- **CREATE (`1` file — Moved `PilotVal`):** `crates/pilot-vm/src/value.rs`
- **MODIFY (Closed List):** `Cargo.toml`, `crates/pilot-vm/{Cargo.toml,src/lib.rs,src/vm.rs,src/ops.rs,src/collect.rs,src/pilc.rs}`, `crates/pilot-cli/{Cargo.toml,src/host.rs,src/run.rs,src/check.rs,src/tool.rs}`

---

## `PROP-003` through `PROP-006` — Subsequent Contraction Waves (Summary)

Each wave below will be expanded into an itemized, closed-file-list proposal before requesting owner approval:
1. **`PROP-003` (Wave 3):** Migrate remaining domain string/collection/cast helpers, `checked.rs`, `parse_header`, and parser semantic passes (`query.rs`, `comptime_if.rs`, `fstring.rs`, `pilot_impls.rs`, `attr_hooks.rs`, `num_suffix.rs`, `loud.rs`, `contracts.rs`, `with_scope.rs`) into `stdlib/` and `selfhost/`, and complete the 115-file `.pilot` migration onto `bridge_lang.pilot` + `bridge_os.pilot` (`GATE021` ceiling = `2` files).
2. **`PROP-004` (Wave 4):** Delete `crates/pilot-analysis` (`6` `src` files, `821` LOC) after migrating remaining `crates/pilot-analysis/tests/milestone*.rs` checks into `.pilot` conformance suites.
3. **`PROP-005` (Wave 5):** Implement `selfhost/vm/ir_to_pilc.pilot` (`PilIR` $\rightarrow$ `.pilc`) and delete all `24` `crates/pilot-vm/src/lower_*.rs` files, `vm_gen.rs`, `codes.rs`, and `BytecodeCompiler` in `crates/pilot-vm/src/lib.rs:60-1015` (`-7,833` Rust `src` LOC).
4. **`PROP-006` (Wave 6):** Ship precompiled `toolchain.pilc` booted directly by `PilVirtualMachine` and delete `crates/pilot-{lexer,parser,ast,span,intern}` + `crates/pilot-cli/src/{link,check}.rs` (`~ -5,950` Rust `src` LOC), reaching the **`~1,800` LOC irreducible VM + `bridge.rs` seed floor**.
