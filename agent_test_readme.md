# `agent_test_readme.md` — Pilot Self-Hosting Milestone Verification & Anti-Cheat Rules

This document defines the mandatory rules, cumulative progression protocol, anti-cheat invariants, and deliverable contracts for completing **Milestones 50 through 61** and the **Simultaneous Mega Gate (`MEGA`)**.

---

## 1. Immutable Boundaries (Strictly Enforced by Cryptographic SHA-256 Seals)

You are the **Pilot implementation runner**. Your job is to implement real, spec-compliant, canonical `.pilot` code in `stdlib/` and `selfhost/` so that all milestone verification tests pass genuinely.

### 1.1 Files You Are Permitted to Create or Edit
1. **`.pilot` source files** inside `stdlib/**/*.pilot` and `selfhost/**/*.pilot` (excluding the permanently retired legacy monoliths `selfhost/scope_header.pilot`, `selfhost/emitter.pilot`, and `selfhost/ide_sync.pilot`).
2. **The Milestone Progress Ledger checkboxes (`- [ ]` $\rightarrow$ `- [x]`) in §2 of this file (`agent_test_readme.md`)**, strictly following the cumulative completion rules below. Do not delete or rename any ledger line.

### 1.2 Files You Are Strictly Forbidden from Modifying (Zero Exceptions)
Every single `#[test]` across the workspace runs [`AntiCheatGuard`](crates/pilot-analysis/tests/common/anticheat.rs) on entry and exit, verifying **48 FIPS 180-4 SHA-256 digests** and cross-checking every test file:
- **DO NOT** modify, add, or delete any Rust file in `crates/*/src/**/*.rs`.
- **DO NOT** modify `Cargo.toml` or any `crates/*/Cargo.toml`, or recreate any retired Rust crate (`pilot-syntax`, `pilot-fmt`, `pilot-perceus`, `pilot-transpile`, `pilot-polyglot`, `pilot-dap`, `pilot-lsp`, `pilot-lsp-macros`, `pilot-db`, `pilot-incremental`, `pilot-pkg`, `pilot-perf`, `pilot-bench`).
- **DO NOT** modify any test file in `crates/*/tests/**/*.rs` or `crates/pilot-analysis/tests/common/anticheat.rs` (no `#[ignore]`, `#[should_panic]`, `process::exit`, `mem::forget`, early `return;` before `guard.finish()`, or weakened assertions).
- **DO NOT** modify `scripts/gate1_check.py`, `scripts/gate2_check.py`, `scripts/gate3_check.py`, or `scripts/gate4_selfhost_contraction.py`.
- **DO NOT** place non-`.pilot` files (`.py`, `.js`, `.mjs`, `.sh`, `.c`, `.cpp`, `.rs`, `.so`, `.dylib`, `.dll`, `.a`, `.o`, `.wasm`, `.pilc`, `.json`, `.sql`, etc.) or binary blobs anywhere inside `stdlib/` or `selfhost/`.

---

## 2. Official Cumulative Milestone Progress Ledger

> **RULE FOR MARKING A MILESTONE PASSED (`[x]`):**
> 1. You must implement **all** deliverables for Milestone `M<N>` in `.pilot` without shortcuts or stubs.
> 2. You must run **ALL tests in Milestone `M<N>` AND ALL tests in EVERY previous milestone (`M50` through `M<N-1>`) in a single cumulative test run**, and **100% of those tests must pass (`0 failed`)**.
> 3. **Monotonicity is machine-enforced on every test run (`Pass 3B`):** Marking `M<N>` as `[x]` while any earlier milestone `M50`..`M<N-1>` is still `[ ]` immediately triggers an anti-cheat panic (`ILLEGAL_NON_CUMULATIVE_CHECKMARK`). If an edit for `M<N>` breaks an earlier milestone, you must fix the regression (or uncheck `M<N>`) until the entire prefix `M50`..`M<N>` passes together.
> 4. You may mark **`MEGA`** as `[x]` **only** after `test_mega_all_milestones_50_through_61_simultaneously` passes (`44/44` tests across all 13 milestone suites passing simultaneously on a single SHA-256-verified `.pilot` codebase) and `M50`–`M61` are all `[x]`.

### Progress Checklist (Keep Exact Line Prefixes `- [ ] <ID>:` / `- [x] <ID>:`)
- [x] M50: Lossless AST/Physics Bridge, Open Keyword Reflection & Weight-Sorted Attribute Weaver (`milestone50_lossless_bridge_and_reflection`)
- [x] M51: Homoiconic Syntax Package Migration, `nameof(...)` Hygiene & Parser Contraction (`milestone51_homoiconic_syntax_and_contraction`)
- [x] M52: 14-Token Seed Lexer, Pilot Preprocessor & Literal/Unit Receptors (`milestone52_seed_lexer_and_preprocessor` + `milestone52_pilot_preprocessor_and_receptors`)
- [x] M53: Lossless GreenNode CST, Incremental Subtree Reuse, Live Typo Recovery & Wadler-Lindig Formatter (`milestone53_cst_recovery_and_formatter`)
- [ ] M54: Lattice-Driven Perceus ARC (`Dup`/`Drop`/`DropReuse`/`ScopeExitDrop`), Escape & Realtime Verifiers (`milestone54_perceus_ownership_and_escape`)
- [ ] M55: Self-Hosted PilVM v3 Bytecode Compiler, `.pilc` Binary Serializer & Debug Span/Register Tables (`milestone55_selfhost_vm_compiler_and_debug_table`)
- [ ] M56: Layout-Driven C11, LLVM IR, WASM, x86_64 ASM & 8-Backend GPU Codegen (`milestone56_layout_driven_codegen_and_gpu_backends`)
- [ ] M57: 7-Target Source Transpilers, 6-Language Foreign Ingesters & Boolean SPDX License Evaluator (`milestone57_bidirectional_transpilers_and_ingest`)
- [ ] M58: Purified Runtime FFI Bindgen, RFC Crypto (SHA-256/512, BLAKE3, HMAC, Ed25519), Glitch-Free Reactive, CoW Effects, TCP HTTP/1.1, Polyglot & Wire DB (`milestone58_runtime_crypto_wire_db_and_polyglot`)
- [ ] M59: Strict-JSON VS Code Generator, Persistent PilVM LSP & Live `VmHook`-Stepped DAP (`milestone59_persistent_pilvm_lsp_and_dap`)
- [ ] M60: Self-Hosted CLI, Package/Venv/Archive Manager, Flamegraph Profiler, AST WCET & Constant-Folding Optimizer (`milestone60_selfhost_cli_pkg_perf_and_bench`)
- [ ] M61: Standalone Self-Hosted Lexer/Parser, Stage-3 Fixed-Point (`pilotc2 == pilotc3`) & Dual-Pass Universal Rename Litmus (`milestone61_stage3_fixed_point_and_universal_rename_gate`)
- [ ] MEGA: Simultaneous Full-Suite Gate — All 13 Milestone Suites (44 Tests Across M50–M61) Passing Together (`mega_all_milestones_simultaneous_gate`)

---

## 3. What Every `.pilot` Implementation Must Do to Pass the 10-Pass Anti-Cheat Engine

At the end of **every** test (`guard.finish()`), `anticheat.rs` runs 10 static + structural verification passes in addition to the randomized runtime oracles inside each test:

1. **Canonical Pilot Syntax Only (`AGENTS.md` §2 — Checked Lexically in Pass 5):**
   - Use `fn` (never `def`), `(x) => expr` (never `lambda`), `var` (never `mut`), `switch` (never `match`), `else if` (never `elif`).
   - Use `//` and `/* ... */` comments (never `#` comments; `#` at line start is strictly reserved for `#if`, `#ifdef`, `#ifndef`, `#elif`, `#else`, `#endif`, `#define`, `#undef`, `#region`, `#endregion`, `#pragma`, `#error`, `#warning`, `#line`, and `#!` shebangs).
   - Use `f"..."` for string interpolation (never `$"..."`; `$` is strictly for homoiconic `quote { $node }` splicing).
   - Use `nameof(ident)` with an **unquoted identifier** — `nameof("ident")` with a string literal is caught and rejected (`NAMEOF_STRING_CHEAT`).
2. **Strict Per-File Line Caps (`Pass 4` & `Pass 5`):**
   - Every `.pilot` file in `stdlib/` and `selfhost/` must be **`<= 1,500` lines**.
   - `selfhost/ide/vscode_gen.pilot` and `selfhost/lsp/handlers.pilot` must each be **`<= 1,000` lines**.
   - Split large subsystems cleanly across modular `.pilot` files rather than cramming code onto single lines or exceeding caps.
3. **Zero Stubs, Placeholders, Dead Parameters, or Constant Returns (`Pass 5` & `Pass 6`):**
   - Never leave `TODO`, `FIXME`, `UNIMPLEMENTED`, `PLACEHOLDER`, `HACK_BYPASS`, or `MOCK_IMPL` anywhere in comments or code.
   - Every required `pub fn` must have a concrete Pilot body (`{ ... }` or `=> ...;`, never `extern fn` or empty `{}`), must perform real computation (never returning a single hardcoded constant literal), and must actually read every non-`_` parameter in its body (`UNUSED_PARAMETER_FAKE_IMPL`).
4. **Zero Hardcoded Test Vectors, Fixtures, or Harness Delimiters (`Pass 7`):**
   - All tests generate **runtime-randomized identifiers, types, constants, byte lengths, ports, SQL datasets, and Ed25519 keys** seeded from OS entropy (`SystemTime` + PID), and cross-check outputs against live external oracles (`clang`, `clang++`, `rustc`, `node`, `python3`, `sqlite3`, loopback TCP sockets, and `PilVirtualMachine`).
   - Known RFC/crypto test digests, test-harness framing strings (`@@WIRE@@`, `<<<DBG>>>`, `===HDR===`, etc.), and adversarial decoy names (`dsp_audio_synth`, `PacketFrame_`, etc.) are banned across all `.pilot` files.
5. **Zero External Shell-Out / Fake-Algorithm Bypasses (`Pass 8` & `Pass 9`):**
   - `stdlib/crypto/hash.pilot` and `selfhost/pkg/manifest.pilot` must implement real RFC 6234 SHA-256 / SHA-512, official spec multi-chunk Merkle BLAKE3, and RFC 2104 HMAC-SHA256 in Pilot — FNV-1a constants (`14695981039346656037`, `0xcbf29ce484222325`) and `host_spawn_eval` shell-outs to `python3`/`openssl` are banned.
   - `stdlib/crypto/ed25519.pilot` must implement full 255-bit $GF(2^{255}-19)$ twisted Edwards Curve25519 arithmetic + canonical $S < L$ malleability rejection — 31-bit toy primes (`2147483647`) and external `crypto.sign`/`openssl` shell-outs are banned.
   - `stdlib/http/http.pilot` must speak real HTTP/1.1 (`Content-Length` + `Transfer-Encoding: chunked`) over raw TCP engine hooks (`tcp_connect`, `tcp_send`, `tcp_recv`, `tcp_close`) — `curl`/`urllib` are banned.
   - `stdlib/db/sql_engine.pilot` and `stdlib/polyglot/marshal.pilot` must execute relational SQL and JSON parsing natively in Pilot — `host_spawn_eval` / `sqlite3` shell-outs are banned (`host_spawn_eval` is permitted *only* in `stdlib/polyglot/bridge.pilot` for foreign Python/Node execution).
   - `selfhost/ide/vscode_gen.pilot` must synthesize valid JSON artifacts in Pilot without calling `fs_read_text` (a live VM trap aborts if `fs_read_text` is called).
   - `selfhost/{lexer,ast,parser}.pilot` must tokenize and parse standalone Pilot source in pure Pilot memory without calling mounted Rust `ast_*` arena hooks.
6. **Universal Name-Blindness (`AGENTS.md` §1 — Checked in `Pass 10` & `M61` Dual-Pass Litmus):**
   - Pure compiler, CST, Perceus, VM, IDE, LSP, DAP, REPL, WCET, optimizer, and standalone frontend `.pilot` modules must deduce types strictly from `[Bits(width, reg_bank)]`, `LayoutTable` metrics/field offsets, `[Lattice(mask)]` + `~self()`, and `PhysicsRegistry` symbol queries — **never** hardcoding Pilot type names (`"i64"`, `"u8"`, `"f64"`, `"char"`, `"bool"`, `"usize"`, `"slice"`, `"stride"`, `"option"`, `"result"`, `"range"`, `"SliceBuffer"`, `"RangeWindow"`, `"Dual"`, `"Signal"`, `"EffectPlan"`).

---

## 4. Exact Milestone Test Commands & Cumulative Verification Workflow

Work sequentially from **M50** to **M61**. Before marking any milestone `M<N>` as `[x]` in §2, run its **Cumulative Verification Command** below and verify `0` failures and `0` compiler warnings (`RUSTFLAGS="-D warnings"`).

### Milestone 50 — Lossless Bridge, Open Keyword Reflection & Weaver
- **Deliverable:** `selfhost/analyzer.pilot` (`pub fn run_analyzer_pass() -> i64`, ` Enforces [EnforceCoreNaming]`, `[Bits]`/`[Layout]` binding, full weight-sorted `[Attr]` weaving).
- **Cumulative Command (M50):**
  ```bash
  cargo test -p pilot-analysis --test milestone50_lossless_bridge_and_reflection
  ```

### Milestone 51 — Homoiconic Syntax Migration & `nameof(...)` Hygiene
- **Deliverables:** `stdlib/core/syntax/{type_construct,visibility,lifecycle,properties,module_ext,attribute,for,while,until,switch,guard,try_catch,expr_sugar,fstring,unit_decl}.pilot`, `stdlib/db/sql_syntax.pilot`, `stdlib/polyglot/polyglot_syntax.pilot`, `stdlib/realtime/rt_syntax.pilot`, `stdlib/asyncio/select_syntax.pilot`, `stdlib/contracts/contracts_syntax.pilot` (all keyword/operator/receptor registrations must use `nameof(ident)` rather than raw keyword strings).
- **Cumulative Command (M50–M51):**
  ```bash
  cargo test -p pilot-analysis --test milestone50_lossless_bridge_and_reflection && \
  cargo test -p pilot-parser --test milestone51_homoiconic_syntax_and_contraction
  ```

### Milestone 52 — 14-Token Seed Lexer, Pilot Preprocessor & Receptors
- **Deliverables:** `selfhost/preprocessor.pilot` (`pub fn preprocess_source(src: str) -> str` supporting nested `#if`/`#ifdef`/`#ifndef`/`#elif`/`#else`/`#endif`, `#define`/`#undef`, boolean `&&`/`||`/`!`/`defined(...)`, `#pragma warning disable/restore`, `#error`/`#warning`/`#region`, string/comment immunity, and exact diagnostics `PREPROC001`–`PREPROC008`) + `stdlib/core/core.pilot` receptors.
- **Cumulative Command (M50–M52):**
  ```bash
  cargo test -p pilot-analysis --test milestone50_lossless_bridge_and_reflection && \
  cargo test -p pilot-parser --test milestone51_homoiconic_syntax_and_contraction && \
  cargo test -p pilot-lexer --test milestone52_seed_lexer_and_preprocessor && \
  cargo test -p pilot-analysis --test milestone52_pilot_preprocessor_and_receptors
  ```

### Milestone 53 — Lossless CST, Incremental Reuse, Typo Recovery & Formatter
- **Deliverables:**
  - `selfhost/cst/{cst,cst_parser}.pilot`: `parse_lossless_cst`, `cst_node_kind`, `cst_child_count`, `cst_child_at`, `cst_node_text`, `cst_detect_missing_ident`, `cst_detect_missing_rparen`, `cst_detect_missing_rbrace`.
  - `selfhost/cst/incremental.pilot`: `incremental_reparse_cst` (true `GreenNode` handle reuse on unaffected top-level sibling declarations).
  - `selfhost/cst/cst_recovery.pilot`: `suggest_keyword_typo` (queries live `__engine("query_registered_symbols", cat)` across `0..=37` and `100..=255` for Damerau-Levenshtein distance-1 fix-its), `is_stmt_sync_anchor`, `is_decl_sync_anchor`.
  - `selfhost/fmt/fmt.pilot`: `format_pilot_with_width`, `format_pilot_source`, `verify_format_idempotent` (width-sensitive Wadler-Lindig layout).
- **Cumulative Command (M50–M53):**
  ```bash
  cargo test -p pilot-parser --test milestone51_homoiconic_syntax_and_contraction && \
  cargo test -p pilot-lexer --test milestone52_seed_lexer_and_preprocessor && \
  cargo test -p pilot-analysis --test milestone50_lossless_bridge_and_reflection \
    --test milestone52_pilot_preprocessor_and_receptors \
    --test milestone53_cst_recovery_and_formatter
  ```

### Milestone 54 — Lattice-Driven Perceus ARC, Escape & Realtime Verifiers
- **Deliverables:** `selfhost/perceus/{arc,escape,realtime,optimizer}.pilot` (`perceus_schedule_fn` emitting `Dup:<v>`, `Drop:<v>`, size-matched `DropReuse:<old>-><new>`, and LIFO `ScopeExitDrop:<g>`; `verify_perceus_unit` emitting `ESCAPE001`, `ESCAPE002`, `ESCAPE003`, `PILOT105`, and `PILOT104` strictly from `[Lattice]`, `~self()`, `[Cue]`, `[HardRealtime]`, and `noalloc` — never `"dsp"` name-sniffing).
- **Cumulative Command (M50–M54):** Add `--test milestone54_perceus_ownership_and_escape` to the `pilot-analysis` command above.

### Milestone 55 — Self-Hosted PilVM v3 Bytecode Compiler & `.pilc` Serializer
- **Deliverables:** `selfhost/vm/{compiler,pilc_writer}.pilot` (`compile_arena_to_pilc_hex` emitting executable `PIL3` v3 binaries with SSO `<=14B` + heap `>14B` string pools and zero `"main"` hardcoding; `compile_arena_debug_manifest` emitting `SPANS:...` and `REGS:...`).
- **Cumulative Command (M50–M55):** Add `--test milestone55_selfhost_vm_compiler_and_debug_table`.

### Milestone 56 — Layout-Driven C11, LLVM IR, WASM, x86_64 ASM & 8-GPU Codegen
- **Deliverables:** `selfhost/codegen/{physics,c_types,c_header,c_emitter,llvm_emitter,wasm_emitter,asm_emitter,gpu_c_dialects,gpu_shaders,gpu_llvm}.pilot` (`emit_arena_c11`, `emit_arena_llvm_ir`, `emit_arena_wasm_hex`, `emit_arena_wat`, `emit_arena_wasm_mjs`, `emit_arena_wasm_dts`, `emit_arena_x86_64_asm`, `emit_arena_gpu_kernel` across `cuda`, `hip`, `opencl`, `openmp`, `wgsl`, `spirv`, `nvptx`, `amdgpu` — compiled and executed by `clang`, `clang++`, and `node` WebAssembly).
- **Cumulative Command (M50–M56):** Add `--test milestone56_layout_driven_codegen_and_gpu_backends`.

### Milestone 57 — 7-Target Transpilers, 6-Language Ingesters & SPDX Gate
- **Deliverables:** `selfhost/transpile/{ir,emit_rust,emit_c,emit_cpp,emit_csharp,emit_python,emit_js,emit_lua,ingest}.pilot` (`transpile_arena_to_{rust,c,cpp,csharp,python,js,lua}`, `ingest_foreign_source` for `python`/`rust`/`js`/`csharp`/`c`/`lua` into canonical executable Pilot) + `stdlib/pkg/licensing.pilot` (`is_viable_spdx_license` with `AND`/`OR`/`(...)` boolean algebra).
- **Cumulative Command (M50–M57):** Add `--test milestone57_bidirectional_transpilers_and_ingest`.

### Milestone 58 — FFI Bindgen, Real Crypto, Reactive/Effects, TCP HTTP/1.1, Polyglot & DB
- **Deliverables:** `selfhost/ffi_bindgen.pilot`, `stdlib/crypto/{hash,ed25519}.pilot`, `stdlib/core/{reactive,effects}.pilot`, `stdlib/http/http.pilot`, `stdlib/polyglot/{shm,marshal,bridge}.pilot`, `stdlib/db/{sql_engine,postgres_wire,mysql_wire}.pilot` (see doc header of [milestone58_runtime_crypto_wire_db_and_polyglot.rs](crates/pilot-analysis/tests/milestone58_runtime_crypto_wire_db_and_polyglot.rs) for all 33 required `pub fn` signatures).
- **Cumulative Command (M50–M58):** Add `--test milestone58_runtime_crypto_wire_db_and_polyglot`.

### Milestone 59 — Persistent PilVM LSP & Live `VmHook`-Stepped DAP
- **Deliverables:** `selfhost/ide/vscode_gen.pilot`, `selfhost/lsp/{handlers,ide_db,server}.pilot`, `selfhost/dap/{vm_controller,cli_dbg,inspector,repl_eval,server}.pilot` (see doc header of [milestone59_persistent_pilvm_lsp_and_dap.rs](crates/pilot-analysis/tests/milestone59_persistent_pilvm_lsp_and_dap.rs)).
- **Cumulative Command (M50–M59):** Add `--test milestone59_persistent_pilvm_lsp_and_dap`.

### Milestone 60 — Self-Hosted CLI, Package/Venv Manager, Profiler, WCET & Optimizer
- **Deliverables:** `selfhost/pkg/{manifest,archive,venv,interop}.pilot`, `selfhost/perf/{profiler,wcet,optimizer}.pilot`, `selfhost/bench/bench.pilot`, `selfhost/cli/{main,build_cmd,test_runner,diff_test,repl}.pilot`, `selfhost/pilotc.pilot` (see doc header of [milestone60_selfhost_cli_pkg_perf_and_bench.rs](crates/pilot-analysis/tests/milestone60_selfhost_cli_pkg_perf_and_bench.rs)).
- **Cumulative Command (M50–M60):** Add `--test milestone60_selfhost_cli_pkg_perf_and_bench`.

### Milestone 61 — Standalone Self-Hosted Frontend, Stage-3 Fixed-Point & Rename Gate
- **Deliverables:** `selfhost/{lexer,ast,parser,pilotc}.pilot` (`sh_tokenize`, `sh_parse_program` with homoiconic `syntax ... => quote { ... }` splicing and Pratt precedence, `pilotc_compile_source_to_c11`, `pilotc_bootstrap_stage_c11`, `pilotc_bootstrap_stage_pilc_hex` achieving bit-for-bit `stage2 == stage3` parity, plus all 4 workspace gates and the Dual-Pass Universal Rename Litmus across all 7 backends).
- **Cumulative Command (M50–M61):** Add `--test milestone61_stage3_fixed_point_and_universal_rename_gate`.

---

## 5. The Simultaneous Mega Test (`MEGA`)

Once `M50` through `M61` are complete and marked `[x]`, run the **Simultaneous Mega Gate** in [mega_all_milestones_simultaneous_gate.rs](crates/pilot-analysis/tests/mega_all_milestones_simultaneous_gate.rs):

```bash
RUSTFLAGS="-D warnings" cargo test -p pilot-analysis \
  --test mega_all_milestones_simultaneous_gate \
  -- --exact test_mega_all_milestones_50_through_61_simultaneously
```

And then run the full workspace verification sweep:

```bash
RUSTFLAGS="-D warnings" cargo test --workspace
```

`test_mega_all_milestones_50_through_61_simultaneously` executes **all 44 milestone test functions (`M50`–`M61`) sequentially inside a single atomic test** while verifying pre-run and post-run SHA-256 digests of every `.pilot` file on disk (proving zero mid-run file swapping) alongside all 10 anti-cheat passes. Only when both commands above exit `0` with `0` warnings may you mark `- [x] MEGA:` in §2.
