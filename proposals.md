# `proposals.md` — Mandatory Owner-Approval Ledger for All Rust Changes

> **Binding Governance Rules ([`AGENTS.md`](file:///home/moony/Documents/dev/projects/Pilot/AGENTS.md) §1):**
> 1. **Zero Unapproved Rust Changes:** **No** Rust file (`crates/**`, `Cargo.toml`) may ever be created, modified, or deleted until a proposal below is explicitly marked **`[STATUS: APPROVED BY OWNER]`**.
> 2. **Strict File-Scope Confinement:** When executing an approved proposal, edits, creations, and deletions are **strictly restricted to ONLY the exact Rust file paths listed in that proposal**. If an additional Rust file needs to be touched, stop immediately and amend the proposal for owner approval first.
> 3. **Immediate Dead / Legacy Rust Registration:** Whenever any agent or developer discovers dead, unreferenced, or superseded Rust code during any task, they MUST halt before moving on and log its exact file paths here for immediate owner-approved deletion.
> 4. **3-File Airlock Freeze (Model B+, owner decision 2026-11-14):** Once `PROP-002a` is approved and [`stdlib/core/bridge_lang.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/bridge_lang.pilot), [`stdlib/core/bridge_os.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/bridge_os.pilot), and [`crates/pilot-vm/src/bridge.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm/src/bridge.rs) are written, every existing `HostOp` ID, sub-code, `PilotErr` code, `arch_kind` value, struct field offset and hook signature is **frozen forever**: it is never deleted, renumbered or given a new meaning. Additions (new IDs, sub-codes, error codes, arch kinds, hooks or protocol keys) are **append-only**, each through its own owner-approved entry here. Workarounds that route around the airlock stay forbidden. `GATE021` enforces this against the checked-in snapshot `selfhost/gate/airlock_abi.pilot`.
> 5. **Mandatory Same-Change Map & Matrix Sync:** Every executed proposal MUST update [`rust.md`](file:///home/moony/Documents/dev/projects/Pilot/rust.md), [`pilot.md`](file:///home/moony/Documents/dev/projects/Pilot/pilot.md), [`docs/keyword_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/keyword_matrix.md), [`docs/concept_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/concept_matrix.md), and [`selfhost/gate/manifest.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/manifest.pilot) in the exact same commit.
> 6. **Break Loud & Seam Over Rust Retention:** Shrinking Rust takes absolute priority over keeping legacy Rust-backed `.pilot` paths temporarily green. When deleting misplaced Rust in an approved proposal breaks `.pilot` callers or unbuilt passes, **leave them loudly broken in Pilot via `seam(...)` (`SEAM001`, registered in `seams.pilot` / `known_bugs`)** and downgrade affected rows in `wiring.pilot`, `keyword_matrix.md`, and `concept_matrix.md`—never keep legacy Rust shims alive to avoid breaking Pilot callers.

---

## Proposal Index

| ID | Title | Net Rust `src` Delta | Status |
| :--- | :--- | ---: | :--- |
| **`PROP-001`** | Purge Discovered Dead Rust Crates/Modules, Remove Hardcoded String Leaks & Split Packed `pilot-parser/src/lib.rs` (`MOD001`) | `-399` net (`20,812` $\rightarrow$ `20,413` `src` LOC; `-1,439` unpacked) | **`COMPLETED (APPROVED BY OWNER)`** |
| **`PROP-002a`** | Define the 3-File Airlock Under the B+ Freeze (51 scalar/pointer `HostOp`s), Retire `pilot-rt`, Consolidate Stage-A Natives & Arm `GATE021` | `<= 0` net (`9` $\rightarrow$ `8` crates; `-683` `pilot-rt`) | **`[STATUS: APPROVED BY OWNER]`** |
| **`PROP-002b`** | Bucket B/D `.pilot` Migration (`~1,875` `map_*` + `~800` `ast_*` sites) & Retire `heap.rs`/`host.rs` | To be itemized | `DRAFT — ITEMIZE BEFORE APPROVAL` |
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

- **Status:** **`[STATUS: APPROVED BY OWNER]`** *(approved 2026-11-14)*
- **Origin:** replaces the original `PROP-002` draft, following the Claude ↔ Gemini review in [`conversations.md`](file:///home/moony/Documents/dev/projects/Pilot/conversations.md) (points P1–P9 closed; the owner chose freeze model **B+** on 2026-11-14). The "Final Consensus" section of that file is the reasoning behind every item below.
- **Behaviour change:** none. Conformance counts measured on `f4` at `f8dca46` (488 seed, 480 `--ir`, 480 `--emit c`) must not drop.

### 1. Freeze Model B+ (Owner Decision)

This applies to [`stdlib/core/bridge_lang.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/bridge_lang.pilot), [`stdlib/core/bridge_os.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/bridge_os.pilot) and [`crates/pilot-vm/src/bridge.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm/src/bridge.rs).

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
- `register_pratt_op` takes a numeric `BinaryOp` code. That removes the six hardcoded English operator words (`"and"`, `"or"`, `"is"`, `"union"`, `"intersect"`, `"except"`) from `crates/pilot-parser/src/tables.rs` and their exemption in `seed_audit.pilot`.

`host_spawn_eval` gets **no** hook. It becomes pure Pilot over `ProcSpawn` + `ProcWait` + `FdRead`.

### 3. `bridge_os.pilot` ↔ `bridge.rs` (51 Scalar/Pointer `HostOp`s, `HOST_OP_COUNT = 51`)

`[EnforceCoreNaming] pub lvl(100) pkg Pilot.Core.Bridge.Os;` Every argument and return is an `i64` or `f64` scalar or pointer. There is no `VmHostCall` and no VM value-slot op: list, map and string slots stay in the non-frozen Stage-A table (§4). Struct layouts live in a new non-frozen-by-name file, `stdlib/core/os_abi.pilot`, whose offsets are frozen through the snapshot.

**Conventions:**
- **Fallible ops** return `>= 0` on success and `-(PilotErr)` on failure.
- **`[Infallible]` ops** never return `-PilotErr`: 0, 1, 2, 4–11, 49 and 50.
- **Address outputs:** ops producing a 64-bit address (`MemMap`, `DlSym`, `FfiCallbackAlloc`) write it to `*out_ptr` and return `0` or `-PilotErr`.
- **Variable-length outputs** return `-44` (`E_RANGE`) and write the required length to `*out_len_ptr` when the buffer is short.
- **Descriptors:** `0`, `1` and `2` are stdin, stdout and stderr on every OS. Numbers `>= 3` index `bridge.rs`'s unified table of files, pipes and sockets.
- **Unsupported targets:** an op the target cannot provide returns `-43` (`E_NOSYS`). It never traps.
- **No unwind catching:** `os_ffi_call` performs no unwind catching. Foreign exceptions and hardware faults in foreign code end the process. Foreign errors are caught inside each `std.polyglot.prog.*` absorber's C-ABI shim and lifted to `res<T, ForeignErr>`.

| ID | Op | `bridge_os.pilot` signature | Sub-codes / notes |
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
- **Ring-1/Ring-2 OS callers** move to `bridge_os.pilot`: `file.pilot`, `env.pilot`, `std/{io,proc,time,env}.pilot`, `http.pilot` and `polyglot/bridge.pilot`. The domain algorithms (`walk_files`, sorted `list_dirs`, `str_find`, `proc_first_done`, `unit_header`) become pure Pilot.
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
- **Parser, for `bridge_lang` hook resolution, non-bootstrap hook ingestion, and numeric `register_pratt_op` (4):** `crates/pilot-parser/src/tables.rs`, `crates/pilot-parser/src/decl.rs`, `crates/pilot-parser/src/attr_hooks.rs`, `crates/pilot-parser/src/pilot_impls.rs`.
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
- **`pilot-cli` (3):** `crates/pilot-cli/src/main.rs` (prepend `os_abi.pilot` and `bridge_os.pilot` to `CLI`), `crates/pilot-cli/src/run.rs` (prepend `os_abi.pilot` and `bridge_os.pilot` to `LOADER`; `PilotVal` import), `crates/pilot-cli/src/tool.rs`.
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
- New files: `stdlib/core/bridge_lang.pilot`, `stdlib/core/bridge_os.pilot`, `stdlib/core/os_abi.pilot` and `selfhost/gate/airlock_abi.pilot`.
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

---

## `PROP-002b` — Bucket B/D `.pilot` Migration & Retire `heap.rs`/`host.rs`

- **Status:** **`DRAFT — MUST BE ITEMIZED BEFORE OWNER APPROVAL`** *(depends on `PROP-002a`)*
- **Scope:**
  1. **Bucket B:** a mechanical migration of about 1,875 raw `map_*` call sites (`map_set` 939, `map_get` 842, `map_new` 94) to `Map()`/`.get()`/`.set()`.
  2. **Bucket D:** about 800 `ast_*` call sites across 38 files, migrated to pure-Pilot `selfhost/ast.pilot` + `selfhost/check/`, or stopped at registered seams (rule 6).
  3. Delete `crates/pilot-vm/src/heap.rs` and `crates/pilot-vm/src/host.rs`. `stage_a_natives.rs` survives.
- **Required before approval:**
  - the exact Rust file list (CREATE / DELETE / MODIFY)
  - an itemized table of every conformance or milestone case allowed to become a `SEAM001`, with its seam ID and `wiring.pilot` downgrade
  - before and after conformance counts
- **Not in scope:** `Reflect`/`PhysicsRegistry` lives in `crates/pilot-analysis` and retires in `PROP-004`. `PROP-004` must name the pure-Pilot successor for layout and symbol reflection (`AGENTS.md` §6, `pilot inspect`), and register a `reflect.type_meta` seam plus a `::Spec` wiring row for anything not built.

---

## `PROP-003` through `PROP-006` — Subsequent Contraction Waves (Summary)

Each wave below will be expanded into an itemized, closed-file-list proposal before requesting owner approval:
1. **`PROP-003` (Wave 3):** Migrate remaining domain string/collection/cast helpers, `checked.rs`, `parse_header`, and parser semantic passes (`query.rs`, `comptime_if.rs`, `fstring.rs`, `pilot_impls.rs`, `attr_hooks.rs`, `num_suffix.rs`, `loud.rs`, `contracts.rs`, `with_scope.rs`) into `stdlib/` and `selfhost/`, and complete the 115-file `.pilot` migration onto `bridge_lang.pilot` + `bridge_os.pilot` (`GATE021` ceiling = `2` files).
2. **`PROP-004` (Wave 4):** Delete `crates/pilot-analysis` (`6` `src` files, `821` LOC) after migrating remaining `crates/pilot-analysis/tests/milestone*.rs` checks into `.pilot` conformance suites.
3. **`PROP-005` (Wave 5):** Implement `selfhost/vm/ir_to_pilc.pilot` (`PilIR` $\rightarrow$ `.pilc`) and delete all `24` `crates/pilot-vm/src/lower_*.rs` files, `vm_gen.rs`, `codes.rs`, and `BytecodeCompiler` in `crates/pilot-vm/src/lib.rs:60-1015` (`-7,833` Rust `src` LOC).
4. **`PROP-006` (Wave 6):** Ship precompiled `toolchain.pilc` booted directly by `PilVirtualMachine` and delete `crates/pilot-{lexer,parser,ast,span,intern}` + `crates/pilot-cli/src/{link,check}.rs` (`~ -5,950` Rust `src` LOC), reaching the **`~1,800` LOC irreducible VM + `bridge.rs` seed floor**.
