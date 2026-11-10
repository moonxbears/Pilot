# Audit: Milestones vs. Actual Project State

- **Date:** 2026-11-09
- **Audited revision:** `d3565a8` (M30 commit) plus the uncommitted edits to `MILESTONES_100.md`
- **Scope:** all of `MILESTONES_100.md` — M1–M30 (marked done) checked against the code; M31–M100 checked for feasibility from where the code actually is
- **Replaces:** the previous `Audit.md` (M1–M11 only), whose "Complete / Green" scorecard this audit does not confirm

---

## 1. Verdict

**The milestone document does not reflect the state of the project, and the project is not on track for M31 onward.**

- All 276 tests pass and `pilot gate --full` reports 0 violations. Both are true and both are misleading.
- None of M1–M30 is complete as specified. Each one exists as a Pilot library that only its own test calls. Almost nothing is connected to the compiler, the VM or the CLI.
- The baseline the roadmap starts from ("v1.1.51, Stage-3 fixed-point self-hosting") is not real. `selfhost/pilotc.pilot` is a 218-line template, not a compiler.
- What a user can actually run is a Rust parser plus a small Rust bytecode VM that executes a `while` / `if` / function / struct subset of the language, with integers, strings and maps. `for`, `switch`, f-strings, closures, floats, `defer`, constructors, destructors, imports and all error reporting do not work.
- M31–M50 are specified on top of a type checker, an SSA IR, a heap/ownership model and a native backend. None of those four exist yet.

There is real, reusable work in the repo (section 9). The problem is not that nothing was built; it is that "done" has been measured by tests that cannot tell a working feature from a stand-in.

---

## 2. How this audit was done

| Step | What was run or read | Result |
| :--- | :--- | :--- |
| Test suite | `cargo test --workspace --no-fail-fast` | 276 passed, 0 failed (2m41s) |
| Gate | `target/debug/pilot gate --full --json` | 4 gates, 0 violations, 0.83 s |
| Direct use | About 60 small programs through `pilot run`, plus every CLI subcommand | See section 3 |
| Code reading | Every M1–M30 spec section compared with the function bodies that implement it, in four parallel read-only passes (M1–M10, M11–M20, M21–M30, core pipeline) | See sections 5–6 |
| Spot checks | 20+ of the most serious findings re-checked by hand against the cited lines | All held |
| Forward review | M31–M50 manifests and acceptance tests, M51–M100 summaries | See section 7 |

**Not verified** (stated so nothing below is read as stronger than it is):

- Windows, macOS and ARM64 behaviour. Everything was run on Linux x86_64.
- Correctness of the Win64 / AAPCS64 / RISC-V ABI classifiers, the MSVC mangler, and the `.d.ts` / `.pyi` / Rust-signature scanners on real-world input.
- Whether the bytecode produced by `selfhost/vm/compiler.pilot` loads and runs.
- The LLVM, WASM, asm and GPU emitters were read, not executed. No emitted C or LLVM was compiled by hand.
- The timing in section 3.4 is a single run of one micro-benchmark.

---

## 3. What actually works today

### 3.1 The real execution path

```
user.pilot → Rust lexer/parser (crates/pilot-lexer, pilot-parser)
           → Rust BytecodeCompiler + PilVirtualMachine (crates/pilot-vm)
           → __engine intrinsics (crates/pilot-analysis/src/infer.rs, pilot-rt)
```

All Pilot code in `selfhost/` and `stdlib/` is executed by that Rust VM. The self-hosted analyzer, IR, codegen, Perceus, LSP and formatter are not on this path.

### 3.2 `pilot run` on real programs

| Construct | Result |
| :--- | :--- |
| Integer arithmetic, recursion (`fib(25)`) | Works |
| `while`, `if` / `else if` / `else`, `loop` + `return` | Works |
| Structs, methods, interface dispatch | Works |
| Maps with `m["a"]` | Works |
| `for x in xs` and `for i in 0..5` | Body never runs (prints `0`) |
| `switch` | Body never runs |
| f-strings `f"n={n}"` | Printed literally as `n={n}` |
| Closures `(a, b) => a + b` | Call returns `null` |
| Floats `1.5 * 2.0` | Prints `0` |
| `2 ** 3` | Prints `5` (unknown operators compile to addition) |
| `false && side()` | `side()` still runs (no short-circuit) |
| `defer` | Never runs |
| Constructors `self(...)`, field defaults, `~self()` | Yield `null` / never run |
| `expr?` on a result | Returns `null` |
| `find(3) ?? 9` | Prints `12` |
| Cascades `obj..set_a(1)..set_b(2)` | Corrupts the receiver |
| `use std.crypto;` then calling an imported function | Returns `null` (imports are not linked) |
| Type error `let x: i64 = "hello";` | No error, prints `hello`, exit 0 |
| Undefined name or function | No error, evaluates to `null`, exit 0 |
| Syntax error | No output, exit 0 |
| `return 7` from `main` | Prints `RESULT: 7`, process exit code 0 |

`examples/overview.pilot`, the project's own feature tour, prints nothing and exits 0.

**Root causes** (`crates/pilot-vm/src/lib.rs`):

- The bytecode compiler handles about 20 AST node kinds. Everything else — match/switch, defer, guard, try, lambda, cascade, range, coalesce, `?`, comptime, interface and extend declarations — falls into a `_ => 0` arm and is silently dropped.
- Binary operators always compile to integer opcodes; unknown operators fall back to `AddI64` (line 258). Float opcodes exist in the VM but are never emitted.
- `for` expands (correctly, via `stdlib/core/syntax/for.pilot`) to a `get_enumerator()` call, but that function lives in a stdlib file that is never linked into the user program.
- The value model has no tag separating integers from heap handles. After `var m = {a: 1}`, `var z = 0; z.a` prints `1`. Nothing is ever freed; there is no reference counting.
- The analyzer's diagnostics are discarded by `pilot run`, and there is no type checker to produce real ones (section 5.2).

### 3.3 CLI commands

| Status | Commands |
| :--- | :--- |
| Work | `--help`, `--version`, `completions`, `--schema-json`, `gate`, `run` (subset above), `emit <lang> <file>` (partial) |
| Listed in help, silently do nothing, exit 0 | `check`, `build`, `compile`, `fmt`, `test`, `repl`, `pkg`, `deps`, `tree`, `venv`, `lsp`, `debug`, `perf`, `from`, `map`, `ide-sync` |
| "Unknown command" | `inspect`, `chor`, `doc`, `bench`, `mcp`, `doctor`, `ship`, `migrate`, `absorb`, `studio` |

`selfhost/cli/main.pilot` dispatches with an if-chain that returns `"0"` for anything it does not handle. The VS Code extension's working language server and debugger are Python (`vscode-pilot/server/pilot-lsp.py`, 2,735 lines; `pilot-dap.py`, 2,124 lines).

### 3.4 Performance

One run of `fib(32)`: Pilot VM 0.58 s, CPython 0.14 s, C `-O2` 0.003 s. There is no path from Pilot source to native code that a user can invoke. Gate 3 ("C/Rust-competitive, zero leaks") has no implementation.

---

## 4. Baseline claims vs. reality

The roadmap's "M0 / current state", the README and `VISION.md` all assert the following.

| Claim | Finding |
| :--- | :--- |
| Stage-3 fixed-point self-hosting (`selfhost/pilotc.pilot`) | **False.** `pilotc.pilot` extracts four tokens (function name, bias, multiplier, n) from one fixed-shape function and pastes them into a hardcoded C string and a hardcoded 13-instruction bytecode blob. |
| Stage-2 == Stage-3 SHA-256 parity | **Tautology.** `selfhost/gate/bootstrap_parity.pilot` calls that same function twice with `2` and `3` and compares the outputs. |
| >90% of the compiler in pure Pilot | The Pilot code exists (66,892 lines) but does not compile Pilot. It reads the AST the Rust parser built, through `__engine("ast_*")` calls. |
| 14-token name-blind Rust seed, 5,3xx LOC | The line count is accurate and gamed: 5,363 lines but about 353 KB, with lines up to 981 characters. Normally formatted it would be substantially longer (not measured; `rustfmt` is not installed on the audit machine). The seed also hardcodes Pilot method names (`"len"`, `"slice"`, `"push"`, `"get"` in `pilot-vm/src/lib.rs`), exempted line-by-line in the gate. |
| Native compilation to C11, LLVM, WASM, asm, 8 GPU targets | Emitters exist and walk the AST for an integer-only subset. `f64` and `str` become `int64_t`; `-> i64` often becomes `void`. `crates/pilot-codegen` returns `int main(void){return 0;}` and one fixed GPU kernel regardless of input. Nothing is reachable from the CLI. |
| Perceus ARC with `drop_reuse`, `0.0 ms` GC | The VM does no reference counting and never frees. `~self()` never runs. Perceus code returns strings such as `"Drop:x"` that nothing consumes. |
| Type-safe | There is no type checker. `selfhost/analyzer.pilot` "inference" is a backwards name lookup, and its diagnostics fire on identifier prefixes (section 5.2). |
| Resident LSP, DAP debugger | Handlers exist with no transport (the seed has no stdin intrinsic). DAP returns canned JSON. The working servers are Python. |
| 7 bidirectional transpilers | `pilot emit` is faithful for `while` / `if` / struct fields. It drops methods, constructors and interfaces, turns `for x in xs` into an infinite `while xs:`, and leaves f-strings uninterpolated. |
| Version `v1.1.51` | Not supportable. This is pre-alpha (0.x) by any normal reading. |

---

## 5. Why the tests and the gate are green anyway

### 5.1 The test harness cannot see integration

Every milestone test (`crates/pilot-analysis/tests/milestoneNN_*.rs`) does the same thing: concatenate a hand-picked list of `.pilot` files with an inline script, run it on the Rust VM, and compare a returned value. The script calls library functions directly with hand-built inputs. So a feature specified as language syntax is "tested" without the syntax ever being used.

Example: M30 specifies that `[Cli]`, `[Service]` and `[Entity]` attributes generate code. The M30 tests never put an attribute on a struct; they call `cli_app_def_new(...)`, `cli_arg_def_new(...)`, `cli_parse_argv(...)` by hand.

The attached "anticheat" spec mostly checks that named `pub fn`s and marker strings exist in files. That rewards creating the right names, and the bodies behind them are often stubs.

### 5.2 Recurring failure patterns

| Pattern | Examples |
| :--- | :--- |
| **Fixed output keyed to the test** | `http_service_dispatch` returns the literal `{"id":1,"customer_id":42,"total_cents":2500,...}` and rejects a body if it contains a `-` character (`stdlib/std/http/service_weaver.pilot:166`). `wasm_invoke` branches on export names `"add"`, `"echo"`, `"infinite_loop_test"`, `"oob_read_test"` (`selfhost/adapter/wasm_runtime.pilot:334`). `py_int_from_obj` returns `42`, `py_float_from_obj` returns `3.14159` (`stdlib/std/polyglot/py_capi.pilot:268`). `inspect_run` ignores its target file and prints canned fields `header` / `payload` and methods `get_balance` / `validate`. |
| **Diagnostics triggered by names** | In `selfhost/analyzer.pilot`: `FLOW001` only if the function name starts with `bad_` (line 1172); `TYPE001` only if the variable starts with `mismatch` (1249); `TYPE002` on `undefined` (1256); `TYPE003` on `badcast` (1259); `OWN001` on `moved` (1294); `ESCAPE001`–`003` on `view` / `esc1` / `esc2` / `esc3` (1318–1331). About 35 such checks. |
| **The test supplies the answer** | `check_raw_pointer_escape(..., takes_raw_ptr: bool)` emits `CHOR008` when the caller passes `true`. `analyze_loop_clone(in_loop, ...)` emits `PERF001` when `in_loop` is true. Contract weaving takes `can_prove` and `eval_ok` as arguments. Left-recursion detection takes `is_left_rec`. |
| **Placeholder standing in for a real engine** | `PilotDb` queries return `"tokens_len=" + len`, `"cst_nodes=" + len/4`, `"ast_for_" + fid`, and always `"diagnostics_for_N: 0 errors"` (`selfhost/db/engine.pilot:361–414`). `pilot.cst.parse_lossless` returns `{root_kind: 500, child_count: 1}`. The IR "parser" counts occurrences of the substring `"block"`. |
| **Simulated resources** | Foreign pointers are integer literals (`0x1000`, `0x7fff0000`, `1000000`). The foreign destructor only increments a counter and never calls the destructor. Threads are a simulated id in `ENV`. `SoaTable.push` only changes `count`. |
| **Assertions too weak to fail** | `if plan_impure.diagnostics[0].len() < "DBC003".len() { return 8; }` checks length, not content (`milestone26...rs:253`). The cold-vs-incremental oracle compares two results that are always `"0 errors"`. |
| **Tautological gates** | Stage parity (section 4). The rename litmus renames text in a sandbox and checks the renamed text contains the new names, then runs one probe through the `pilotc` template. It never recompiles `stdlib/core` or runs a backend. |

### 5.3 What `pilot gate` really enforces

Real: the 1,500-line cap per file, TODO/FIXME markers, retired files, per-crate line ceilings, and a check that Rust string literals do not exactly equal a `stdlib/core` type name.

Not enforced, despite the spec and `AGENTS.md`:

- **Raw `__engine` use.** There are 4,344 raw calls (3,431 in `selfhost/`, 913 in `stdlib/`). Rule 12 forbids them. `type_audit.pilot` / `GATE010` were specified in M1 and do not exist.
- **Name-blindness beyond exact literals.** `"slice<"`, match arms and method names are not checked.
- **Ring 1 freestanding.** `stdlib/core/file.pilot` and `stdlib/core/env.pilot` make filesystem and environment engine calls; `ring_verifier.pilot` would reject them but nothing calls it.
- **Duplicate types.** `Diagnostic`, `FixSuggestion`, `HttpResponse`, `ForeignHandle`, `ContractCheckMode` and `StorageStrategy` are each defined twice.
- **Stub bodies, performance, leaks.** No check exists.

The line-count ratchet has had a perverse effect: every Rust crate sits 2–6 lines under its cap, achieved by packing code into very long lines.

---

## 6. Milestone status, M1–M30

**Scale**

| Label | Meaning |
| :--- | :--- |
| **Integrated** | Works on arbitrary input through the CLI or compile pipeline |
| **Library only** | A genuine general algorithm exists, but only tests call it |
| **Simulated** | Operates on hand-built descriptors or returns fixed values in place of the real thing |
| **Missing** | Specified but not present |

No milestone earns "Integrated" overall. The "Marked" column is the heading in `MILESTONES_100.md`; M2–M10 have completion commits but no `[COMPLETE]` tag.

### Phase 1 — foundation

| M | Title | Marked | Actual | Key evidence |
| :-: | :--- | :--- | :--- | :--- |
| 1 | Gate runner | Complete | Partly integrated | `pilot gate` runs and its hygiene checks are real. Rename litmus and stage parity are tautologies. `type_audit.pilot` missing. No performance gate. Fast run takes ~0.8 s against a 500 ms budget. |
| 2 | Module graph, comptime parity | — | Library only, shallow | `module_graph.pilot` has no cycle detection (no Tarjan), `topo_order` is discovery order, `content_hash` is the byte length. `pilot run` still scans `use ` lines textually in Rust (`pipeline.rs:36`) and never links imported code. `pragma_filter.pilot` has no caller, and the Rust filter it replaced was deleted. |
| 3 | CLI registry | — | Partly integrated | Help, completions, schema and typo suggestions work. Dispatch is still an if-chain; `ICliHandler` has no implementers; script arguments after `--` never reach the program. |
| 4 | Dual syntax | — | Partly integrated | Both forms parse. Semantically empty: `add!(1,2)`, `add!^(1,2)`, `sig add(1,2)` and `plan add(1,2)` all just print `3`. `pilot fmt` is a no-op. |
| 5 | `PilotDb` | — | Simulated | The red/green memo engine is plausible; every query returns a placeholder string. No tool uses it. |
| 6 | `PilIR` | — | Simulated / missing | No `IrFunction`, `IrBlock` or `IrModule`. `lower_ast.pilot` (103 lines) concatenates text and hardcodes `"i64"` and `"RangeWindow"`. No backend consumes IR. `selfhost/transpile/ir.pilot`, which M6 was to delete, is still the live path for `pilot emit`. |
| 7 | `pilot inspect` | — | Simulated | Unknown CLI command. `inspect_run` prints canned data. |
| 8 | Diagnostics, `check --fix` | — | Library only | Renderer (UTF-8 width, JSON, SARIF) is real. Autofix detection is a byte scanner with false positives; the "atomic write" has no rename. `pilot check` does nothing. |
| 9 | Three-ring stdlib | — | Mostly not done | Legacy `stdlib/asyncio`, `contracts`, `crypto`, `db`, `http`, `pkg`, `polyglot`, `realtime` remain. `stdlib/std/*.pilot` are thin shells (784 lines total). Ring verifier unenforced. |
| 10 | Compiler-as-a-library | — | Simulated | `stdlib/pilot/*` (46–116 lines each) wrap nothing: `format_str` replaces one substring, the IR pipeline adds a pass count, the VM sandbox fuel trap is driven by hand. |

### Phase 2 — foreign-language adapter

**There is no foreign interop reachable from Pilot.** `use c "definitely_not_a_header.h" as q; q.nope(1,2,3);` runs without complaint and yields `null`. `adapter X -> Y { ... }` is parsed and ignored. No crate has an external dependency (no libloading, libffi, pyo3, napi, wasmtime, quickjs).

| M | Title | Marked | Actual | Key evidence |
| :-: | :--- | :--- | :--- | :--- |
| 11 | `adapter` engine | Complete | Simulated | Thunk "IR" is a text string. Every symbol gets dummy params `["ptr","ptr","size_t"]`. Only `ADP001` is ever emitted. Prefix stripping and case conversion are real. |
| 12 | `use c` absorber | Complete | Library only | The C preprocessor (1,013 lines) is genuine and reads `#include` files from disk. The declaration parser is a word scanner: `unsigned long long x` parses as a field named `long`; function pointers are not handled. Never run on a real system header. |
| 13 | Calling conventions, frame trampoline | Complete | Partly real, unconnected | The Rust `dlopen` wrapper and SysV/Win64 assembly trampoline (`crates/pilot-rt/src/c_abi.rs`) are real and tested from Rust. `__engine("invoke_abi_frame")` has no handler, so Pilot can never reach them. The SysV classifier does not merge eightbytes. |
| 14 | Foreign handles, guards | Complete | Simulated | Destructor increments a counter. Spec asks for 100,000 handles and real threads; the test uses 100 and a fake thread id. |
| 15 | C++ | Complete | Simulated | Itanium mangler and class layout are real libraries. The exception boundary is an if on a boolean argument. No C++ is compiled or loaded. |
| 16 | Rust crates | Complete | Simulated | `absorb_rust_crate` takes source text; the features loop does nothing; cargo is never run. |
| 17 | Python | Complete | Simulated | `py_init_in_process()` is `return true`. libpython is never loaded. The mandated deletion of the `python3` subprocess bridge was not done. |
| 18 | JS / TS | Complete | Simulated | Bigint round-trip returns a hardcoded `i64::MAX`. No Node-API or QuickJS. `node` subprocess path not deleted. |
| 19 | WASM / WIT | Complete | Simulated | No bytecode is executed; dispatch is on export-name strings. The WIT text parser and flattening logic are real libraries. |
| 20 | SQL / OpenAPI / proto | Complete | Partly real, unconnected | Simple DDL scanner and proto varint/zigzag are real. OpenAPI "parsing" is substring search; `oneOf` pushes hardcoded `"VariantA"`, `"VariantB"`. Generated proto getters are `=> 0`. |

### Phase 3 — weavers, `.pchor`, comptime

Every key function in this phase has exactly two kinds of caller: its own file and its test.

| M | Title | Marked | Actual | Key evidence |
| :-: | :--- | :--- | :--- | :--- |
| 21 | `[...]` weavers | Complete | Simulated | Attributes on a struct generate nothing: `[Eq, Clone, Json]` then `a.to_json()` gives `null`. Generated `from_json` returns `T.default()`. The "zero hardcoded attribute names" rule was met by moving ~60 names into `register(...)` calls. |
| 22 | `*Bundle` / `*Facet` | Complete | Library only | Layout flattener and collision rules are real, over hand-built descriptors. At runtime spliced fields read correctly, but spliced methods that touch fields return `null`. Not wired into the analyzer. |
| 23 | Pipeline fusion, `each`, cascades | Complete | Simulated | Fusion adds constants per stage kind and rewrites no IR; equivalence is checked against `execute_fused_pipeline_mock`. Real cascades corrupt the receiver. `selfhost/perf/optimizer.pilot` was to be deleted and was not. |
| 24 | `.pchor` compiler | Complete | Partly real, unconnected | The text parser is real. Cue lattice class comes from name heuristics (`contains("View")`, `contains("Buffer")`). `CHOR004` finds only 2-cycles; `CHOR002` is never emitted; lowering is 88 lines of name strings. `pilot chor` is an unknown command. |
| 25 | `.pchor` storage / SoA | Complete | Simulated | Offset arithmetic on descriptors. Speedups are constants per strategy (4, 2, 8). `SoaTable` has no backing memory. |
| 26 | Typestates, contracts | Complete | Simulated | `requires n > 0` is not enforced: `f(0)` runs. Provability is supplied by the test. |
| 27 | User `syntax`, hygiene | Complete | Simulated | The spec's named form `pub syntax Name { ... }` silently stops all output. A macro's `let tmp` overwrites the caller's `tmp` (hygiene violation). The scopes-as-sets resolver is real and unconnected. |
| 28 | Tiers, perf coach | Complete | Partly real, unconnected | Padding analyzer is real. The tiered `ESCAPE001` messages are three fixed templates. No `--tier` flag exists. Mandated analyzer deletions not done. |
| 29 | Sandboxed comptime | Complete | Simulated | The VM has a real fuel counter that is only ever set in tests; `comptime { while true {} }` hangs under `pilot run`. `mount_unit` reads any path ungated. The VM still uses `HashMap`, which the milestone was to replace. |
| 30 | `[Cli]` / `[Service]` / `[Entity]` | Complete | Simulated | Argv parser and SQL/DDL generators are real string builders. HTTP dispatch is hardcoded. The attributes themselves are never used. |

### Mandated deletions not carried out

`AGENTS.md` requires superseded code to go in the same milestone. Still present: `selfhost/transpile/ir.pilot` (M6), `crates/pilot-analysis/src/diagnostics.rs` (M8), the string-concatenating loader in `main.rs` (M2), legacy stdlib directories (M9), the `python3` / `node` subprocess bridge in `stdlib/polyglot/bridge.pilot` (M17, M18), `stdlib/polyglot/shm.pilot` (M14), `selfhost/perf/optimizer.pilot` (M23), clearance string-parsing in `analyzer.pilot` (M28), the VM `HashMap` (M29).

Other hygiene debt: 203 `def`, 20 `match` and 13 `record` uses in `tests/*.pilot`; 14 tracked Python scripts in `scratch/`; `__pycache__/`, `input.txt`, `fix_type.sh`, `selfhost/.pilc-cache/analyzer.pilc`, `tests/test_convert_dunders.py` and `tools/pilot-kernel` (Python).

---

## 7. Are we on track for M31–M100?

No. The next twenty milestones each assume substrate that section 6 shows is absent.

| Needed by | Assumed to exist | Actual state |
| :--- | :--- | :--- |
| M31, M32, M33, M34, M35, M37, M48 | `PilIR`: typed SSA with blocks, lowered from real source | No IR containers, no lowering |
| M31, M32, M36, M39, M46 | Heap objects with reference counts and destructor calls | VM never frees; `~self()` never runs |
| M31–M38, M43, M45 | A type checker and method tables built from declarations | Name-prefix diagnostics only |
| M33, M35, M39, M40, M48 | A backend that compiles the whole language | Integer-only AST walkers, tests only |
| M38, M39, M50 | Self-hosted front end that parses declarations | `selfhost/parser.pilot` skips struct bodies and stores signatures as raw text |
| M41–M49 | Working `check`, `build`, `test`, `lsp`, `PilotDb` | No-ops and placeholders |
| M40, Gate 3 | Any performance or leak measurement | None |
| All Tier-1 platform claims | CI on Linux, macOS, Windows, x86_64 and ARM64 | CI runs `cargo test` on Ubuntu and Windows x86_64 only |

If M31–M50 are built the way M1–M30 were, they will produce twenty more libraries with passing tests and no working compiler. M38 ("retire the Rust lexer, parser and analysis crates") is the point where this stops being possible to paper over: the Rust seed is the only thing that currently executes anything.

M51–M100 are one-paragraph summaries and are reasonable as long-range direction. They are not assessable in detail until 1.0 exists.

---

## 8. Changes needed in `MILESTONES_100.md`

> **Applied on 2026-11-09:** 8.1 items 1–2 (baseline paragraph rewritten; `[COMPLETE]` markers replaced with audited `[STATUS: …]` tags and a legend) and 8.2 (Phase F, `F1`–`F8`, inserted before Phase 4 with a verification contract and a prerequisite table). 8.4 rules 10–13 are folded into the Phase F verification contract.
>
> **Also applied on 2026-11-09:** 8.3 item 6 (Rust seed contraction paused until `F8`). The gate now enforces the `<= 1,500` LOC law per Rust file plus a 15,000-line total budget instead of the per-crate ratchet. The policy lives in `selfhost/gate/manifest.pilot`, the Rust milestone tests read it through `crates/pilot-analysis/tests/common/seed_policy.rs`, and `AGENTS.md` §1 records the pause.
>
> **Still open:** 8.1 items 3–4 (version reset; README and `VISION.md` claims), 8.3 (reordering and scope of `M31`–`M50` and Phase 2), 8.4 items 14–18 (anticheat, ratchet, gates), 8.5 (spec defects), 8.6 (process and outstanding deletions).

### 8.1 Status and baseline corrections

1. **Rewrite the "Baseline (M0 / Current State)" paragraph** to match section 3. Remove "Stage-3 fixed-point self-hosting".
2. **Remove `[COMPLETE]` from M1 and M11–M30**, and from the M50 heading (`[COMPLETE / UNIFIED 1.0 GATE]` on a milestone that depends on unbuilt M31–M49). Replace with a status that distinguishes "library exists" from "integrated", for example `[LIBRARY]` vs `[INTEGRATED]`.
3. **Reset the version** from `1.1.51` to a `0.x` number in `Cargo.toml`, the README and the CLI banner.
4. **Fix the README and `VISION.md`** claims listed in section 4.

### 8.2 Insert a foundation phase before M31

Everything after M30 depends on these. They should be milestones in their own right, each proven through the CLI.

| # | Deliverable | Done when |
| :-: | :--- | :--- |
| F1 | **Honest seed.** Every AST node kind either executes correctly or is a hard error. Tagged values (int, float, handle distinct). Float and generic operators. Non-zero exit codes. | `examples/overview.pilot` and the `tests/*.pilot` corpus run and produce checked output; every program in section 3.2 behaves as the language reference says. |
| F2 | **Module linking.** `use` compiles and links imported units. | `for` works because `get_enumerator` is linked; `use std.crypto` functions are callable. |
| F3 | **Errors surface.** Syntax, name and (once F4 lands) type errors print and fail the process. | A negative-test corpus: each bad program fails with the expected code. |
| F4 | **Type checker.** Scoped name resolution, inference, method tables from declarations, lattice classification from fields. | The name-prefix triggers in `analyzer.pilot` are deleted and the same diagnostics fire on programs with arbitrary identifiers. |
| F5 | **Real `PilIR`.** Data structures, lowering from typed AST for the whole language, verifier, pass driver. | A VM or interpreter executes the IR and matches the AST path on the corpus. |
| F6 | **One real backend.** C11 from `PilIR`, a runtime (allocation, reference counts, strings, print), `pilot build` invoking the C compiler. | The corpus passes when built natively; outputs match the VM. |
| F7 | **Ownership at runtime.** Dup/drop insertion over the IR, destructors run, a live-allocation counter. | A leak check over the corpus reports zero. |
| F8 | **Real self-hosting.** The Pilot front end parses declarations and feeds the same pipeline; the compiler compiles its own sources. | Stage 2 and stage 3 are produced by actually compiling `selfhost/`, and their hashes match. |

Only after F5–F7 does M31 (interprocedural Perceus) have something to analyse, and only after F8 are M38/M39 meaningful.

### 8.3 Reordering and scope

5. **Move M38 and M39 (self-hosting cutover, C11 seed) ahead of M33–M37**, directly after the foundation phase. Optimising backends for a compiler that cannot yet compile itself is the wrong order.
6. **Pause Rust contraction until F8.** The seed must become correct before it can be replaced, and that will take more Rust in the short term, not less. Contracting it now only removes working behaviour (the pragma filter was deleted in M2 with no working replacement).
7. **Cut the 1.0 backend set to C11 first, LLVM second.** Defer M35 (hand-written x86_64/ARM64 encoders plus ELF, Mach-O and PE writers) and M36 (six GPU targets) until after 1.0. LLVM already provides SIMD, cross-targets and debug info, which covers most of M34.
8. **Redo Phase 2 as "C for real, then one managed runtime".** Wire the existing `dlopen` wrapper and trampoline to a VM intrinsic, add a raw-pointer/byte-buffer value, make `use c` and `adapter` reach the absorber, and prove it by calling a real C library. Then pick one of Python or JS. Mark C++, Rust, WASM, JVM, .NET and Swift as post-1.0. A bytecode VM with no native-call path cannot host libpython or catch C++ exceptions, so M15–M19 as written were not achievable when they were marked complete.
9. **Bring `pilot test` (M46) forward** to the foundation phase. Milestone tests are Rust files in `crates/pilot-analysis/tests`, which M38 and M89 are supposed to delete.

### 8.4 Verification rules to change

10. **Acceptance tests must go through the CLI on source files.** A milestone about syntax is tested by a `.pilot` file that uses the syntax, run with `pilot run` / `check` / `build`, with stdout and exit code checked. Function-level unit tests are welcome but do not close a milestone.
11. **Every diagnostic needs a negative and a positive test with neutral identifiers.** This removes name-keyed triggers.
12. **No test-supplied verdicts.** A checker may not take the fact it is supposed to establish as a parameter.
13. **Assert content, not length or substring presence.**
14. **Replace the anticheat name/marker lists** with behavioural checks. Requiring specific `pub fn` names has produced the names without the behaviour.
15. **Replace the line-count ratchet** with a byte or token count, and format the Rust with `rustfmt`. Drop the per-file "~360L" budgets as targets; several M31–M50 budgets are far below what the described feature needs, which invites shallow implementations.
16. **Drop the fixed "four tests per milestone" shape.** Let the acceptance criteria decide.
17. **Add the missing gates for real:** raw `__engine` use, duplicate types, ring isolation, a performance baseline and a leak count. Today the gate reports "0 violations" while 4,344 raw engine calls exist.
18. **Rebuild the binary before gate tests.** `anticheat.rs` runs a prebuilt `target/debug/pilot` and only rebuilds if it is missing, so it can test a stale binary.

### 8.5 Defects inside the spec text

19. **Milestone number collisions.** Test files `milestone50_*` through `milestone61_*` belong to the old numbering in `LEGACY_MILESTONES.md`. The new M50 cites the old `milestone50_lossless_bridge_and_reflection.rs` as already verifying it, and M43 refers to "CST (`M53`/`M8`)". New M50–M61 tests will collide with these files. Rename the legacy tests.
20. **M1** gives two different rename-pair lists (`map→dict_view` in the manifest, `str→utf8view` in §1.5).
21. **M5** names durability tiers `Low/Medium/High/Core` in §5.2 and `Volatile/Workspace/ExternalPkg/CoreGenesis` in §5.3.
22. **M6** declares `IrInst` as 32 bytes; its listed fields sum to 38.
23. **M1 and M3** jointly ask for more dispatch in `main.rs` and a 35-line cap, which is what produced 793-character lines.
24. **"The Rust seed contains strictly the 14 seed tokens"** cannot hold while a Rust parser parses the full language. State the rule as the behaviour that matters (renaming core types still compiles and runs real programs) and test that.
25. **Unmeasurable acceptance criteria** — "≤1.02× hand-written C", "0 ns overhead", "<15 ns", "sub-10 ms", ">500K LOC/s", "bit-identical across Windows/macOS/Linux" — should be removed until the infrastructure to measure them exists, then reintroduced with the measuring tool named.
26. **Multiplatform contract.** Either add macOS and ARM64 CI or downgrade the Tier-1 wording to a goal.

### 8.6 Process

27. **Enforce `AGENTS.md` §1 "Zero Shortcuts".** The patterns in section 5.2 are exactly what that rule forbids. The gate should fail on functions whose names contain `mock`, on fixed-literal returns in non-test code where practical, and reviews should check bodies, not signatures.
28. **Do the outstanding deletions** listed at the end of section 6.

---

## 9. What is real and worth keeping

- **Rust lexer and parser**, including the `syntax` rule system that expands `for` / `while` / `switch` from Pilot-defined rules. General.
- **Rust VM** for the integer / string / map subset, and its (currently unwired) fuel counter.
- **`dlopen` wrapper and SysV/Win64 assembly trampoline** in `crates/pilot-rt/src/c_abi.rs`. This is the seed of real FFI.
- **`pilot gate` hygiene checks**, and the CLI registry with help, completions, schema output and typo suggestions.
- **Self-hosted lexer** (`selfhost/lexer.pilot`) and the statement/expression half of the self-hosted parser.
- **`selfhost/vm/compiler.pilot`**, which covers more node kinds than the Rust compiler (defer, guard, try, match, for). Output unverified.
- **AST-walking emitters** for C, LLVM, WASM and asm — a starting point once types exist.
- **`pilot emit`** for straight-line code.
- **Libraries with genuine algorithms:** the C preprocessor, Itanium mangler, C++ class layout, WIT parser and flattening, proto varint/zigzag, SQL DDL scanner and generators, bundle layout flattener, scopes-as-sets hygiene resolver, struct padding analyzer, `.pchor` text parser, diagnostic renderer (UTF-8 width, SARIF), autofix edit applier, the `PilotDb` red/green memo skeleton, the argv parser.
- **The design documents.** `VISION.md`, `AGENTS.md` and `imp_type.md` are coherent, and the 2-bit lattice is a sound organising idea. The gap is implementation and verification, not direction.

---

## 10. Summary

| Question | Answer |
| :--- | :--- |
| Does the milestone document reflect the current state? | No. M1–M30 are libraries plus tests, not integrated features. The baseline self-hosting claim is false. |
| Are tests and gate trustworthy as completion signals? | No. They pass by construction for the reasons in section 5. |
| Are we on track for M31+? | No. Type checking, IR, a heap model and a native backend must exist first. |
| What should change first? | Correct the status markers and baseline (8.1), add the foundation phase (8.2), and change how milestones are verified (8.4). |
