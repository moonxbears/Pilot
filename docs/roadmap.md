# Pilot Rust-Contraction Roadmap

`proposals.md` lists proposals in the order they were thought of. This file lists them in the order they are **built**. Each step names what it needs before it can start, and what it unblocks. Statuses are as of 2026-11-18; `proposals.md` stays the record of approvals and "as built" notes.

The end state is `PROP-006`: the seed keeps only the VM, the slab/byte primitives and `bridge.rs` (about 1,800 Rust lines). Everything else — lexer, parser, checker, compiler, toolchain — is Pilot, booted from a checked-in `toolchain.pilc`.

There are three tracks. Track A (the compiler back end) and Track B (the front end) can run in parallel; Track C is cleanup that slots in whenever its blocker clears. All three meet at Phase 4.

```
Phase 0  close out what is in flight
            │
   ┌────────┴──────────────┐
Track A                  Track B                  Track C (cleanup, any time)
back end                 front end                PROP-004, PROP-003 3D–3F,
5B.4 → 5B.5/5B.6 → 5C    008 step 2 → 019 →       PROP-002b remainder
      → 5D → T1 → 5E     022 → 025 (1–3) → 034
      → 5F               → 025 (rest)
   └────────┬──────────────┘
Phase 4  PROP-023 (LANG section) → PROP-006 cutover (~1,800 Rust LOC floor)
```

## Rules every step follows (so no step grows Rust)

- **A step deletes; it does not add.** Each step below lists the Rust and old Pilot it removes. A step may add Rust only where its approved proposal names the exact lines and there is no Pilot way (an appended `HostOp`, a boot hook). If a step's Rust total goes up, stop and ask the owner.
- **The only reason to keep a Rust file is that the toolchain itself needs it to run** (the code that runs `pilot`, `pilot test` and `pilot gate`). Breaking a user feature is never a reason: the feature becomes a `seam(...)` or a `known_bugs` entry with its true expected output (AGENTS.md rule 6).
- **Feature code keeps working on the other engines.** `pilot test --ir` and `--emit c` run through PilIR in Pilot. When a Rust `lower_*.rs` file goes, only the seed VM loses that feature for user programs. When a Rust parser file goes, all three engines lose it, because all three still read the Rust parser's AST until `PROP-025`.
- **Rust tests are deleted, not kept or added.** What a Rust test checked that no Pilot conformance program checks is ported to `tests/conformance/` first. The one exception: `pilot-parser`'s `damaged_source` test stays until the Rust parser is deleted (AGENTS.md §11 rule 7).
- **Old Pilot code goes on sight.** A `.pilot` file that only works on the Rust AST bridge, or that a Pilot replacement has superseded, is deleted when its last user goes, not kept "for later".

Seed today: **27,054** `src` lines (the `paused_max_total_rust_loc()` ceiling) plus **25,486** lines of Rust tests (`pilot-analysis` 20,703; `pilot-cli` 3,266; `pilot-parser` 1,151; `pilot-vm` 366).

---

## What each step deletes

| Step | Rust deleted | Old Pilot deleted | What breaks (made loud) |
| :-- | :-- | :-- | :-- |
| **D0 — now** (new proposal; needs nothing else first) | All of `crates/pilot-analysis`: `src` 1,380 (`infer.rs` 1,080, `types.rs` 287, `lib.rs` 13) and its 46 test files, 20,703. In `pilot-cli`: the `pilot_analysis` uses in `tool.rs` and `run.rs` (the `ast_*`/`layout_*`/`mount_unit` hooks) and the `Cargo.toml` dependency. The 13 `pilot-cli` tests (3,266) and 2 `pilot-vm` tests (366), after porting what only they check. `pilot-parser`'s tests except `damaged_source` (948). **About −26,700.** | 5B.4 group 4: `selfhost/{lexer,ast,parser}.pilot` (343 `__engine` calls). Group 5, the 35 unlinked files (1,967 calls): `selfhost/vm/compiler.pilot` (the old Pilot bytecode compiler, superseded by `ir_to_pilc.pilot`), `vm/pilc_writer`, `codegen/*` (old emitters over the Rust AST; deleted 2026-11-18, nothing ran them; `--emit c` uses `selfhost/ir`), `perceus/*`, `perf/*`, `lsp/*`, `dap/*`, `db/*`, `ide/vscode_gen`, `cst/cst`, `bench/bench`, and each unpackaged `stdlib/*` file no test imports. | Everything that reads the Rust AST through `pilot-analysis`: `ffi_bindgen`, `core/reflect` (user reflection), `attr_weaver`, `core/primitives/ast` (`ast<T>` reads), `diagnostics`, `ast_bridge` (deleted 2026-11-18), `core/keywords` (the keyword-matrix tool). Each call becomes a seam; its rows drop to `SEAM`. **Rewrite first, in Pilot over `selfhost/check`'s snapshot:** the 7 AST reads in `cli/test_runner` and the 6 in `pkg/manifest`, because `pilot test` and package loading are toolchain. LSP, DAP and the Perceus/WCET passes are rebuilt on `selfhost/check` later. |
| **A1 — 5B.4 groups 3–4** | Each CLI host native that can be rebuilt in Pilot over existing owners leaves `pilot-cli/src/{host,tool}.rs` ("Pilot first", 2026-11-18). | `legacy_engine_ratchet()` rows go to 0 and are removed. | done 2026-11-18 |
| **A2–A4 — 5B.5, 5B.6, 5C** | None expected. A fix that seems to need Rust is first asked about. | Any check left duplicated between `selfhost/check` and an older pass. | — |
| **A5 — 5D** | None (adds only the approved minimum). | Whatever `ir_to_pilc.pilot` supersedes that D0 did not already remove. | — |
| **A7 — 5E (T1)** | The toolchain link lists in `pilot-cli/src/{link,run}.rs` that only existed to compile the toolchain from source (exact list in the 5E proposal). | — | — |
| **A8 — 5F** | `pilot-vm`: the 24 `lower_*.rs` files (7,683), the `BytecodeCompiler` in `lib.rs` (lines about 60–1015), `vm_gen.rs` (172), `codes.rs` (36), `checked.rs` (103, `PROP-003` 3C); the casts trim in `ops.rs`. **About −8,770.** | `ir/fault`'s `op_checked` call. | — (user programs already run through PilIR since 5D). |
| **Before A8, if wanted** | A `lower_*.rs` file for a feature the toolchain does not use can go early: the seed VM then reports a seam for that feature in user programs, and `--ir`/`--emit c` keep it. Candidates by toolchain use (lines in `selfhost`+`stdlib`): contracts (`lower_contract.rs` 195; 3 lines), cascades (`lower_cascade.rs` 86; 1 line), payload variants (`lower_variant.rs` 130; 2 lines), destructuring (`lower_destructure.rs` 213; 2 lines). Rewrite those few toolchain lines, then delete. | — | Seed-VM runs of those features. |
| **B1 — `PROP-008` step 2** | The 30 dead operator `TokenKind`s and their arms (exact count in the step's report). | — | — |
| **B2 — `PROP-019`** | `TypePrecedence` and its table (about −25). | — | — |
| **B3 — `PROP-022`** | `map_binop_sym`, `binop_code`, `prefix_flag`, the `TryPropagateExpr` case, the native index parse, `BinaryOp`. | — | — |
| **B5–B7 — `PROP-025`** | Parser desugarings as the Pilot parser takes each over: `fstring.rs` 344, `pilot_impls.rs` 337, `comptime_if.rs` 263, `attr_hooks.rs` 194, `loud.rs` 170, `num_suffix.rs` 164, `contracts.rs` 113, `declared_names.rs` 37, `unbuilt.rs` 28 (1,650). Rarely used ones can go earlier, the same way as the early `lower_*` files: `contracts.rs`, `num_suffix.rs`, `comptime_if.rs` (3, 3 and 5 toolchain lines). | — | Before B7, an early parser deletion breaks that feature on all three engines. |
| **P1 — `PROP-023`** | Whatever lexical, grammar or meaning table is still built in Rust. | — | — |
| **P2 — `PROP-006`** | `pilot-parser` (`src` 9,106 + `damaged_source` 203), `pilot-ast` 917, `pilot-lexer` 704, `pilot-intern` 162, `pilot-span` 81, `pilot-cli/src/{check,link}.rs` 768, `pilot-vm/src/stage_a_natives.rs` 360 (natives exist only for the Stage-A parser). **About −12,300.** | `stdlib/core/bridge_lang.pilot`'s Stage-A hooks; `transitional_native_bindings()` emptied. | — |
| **P3 — C11 seed (`M46`)** | The rest of the Rust seed, rewritten in C11: `pilot-vm`'s VM, values, slabs, `bridge.rs`, `.pilc` loader, and `pilot-cli`'s `main.rs`/`tool.rs`/`host.rs`. | — | — |

After P2, about 4,700 Rust lines remain (`pilot-vm`'s `bridge.rs` 1,245, `lib.rs` remainder, `vm.rs` 594, `ops.rs`, `heap.rs` 349, `value.rs` 290, `collect.rs` 274, `instruction.rs` 261, `pilc.rs` 176, `host.rs` 98, `strings.rs` 72, and `pilot-cli`'s `main.rs`/`tool.rs`/`host.rs`/`run.rs` remainder). `heap.rs` and `host.rs` fold into `bridge.rs` (`rust.md`: "CONSOLIDATE"); reaching the ~1,800 floor needs those and the `ops.rs`/`run.rs` remainders itemized before P3.

---

## Phase 0 — Close out what is in flight

| # | Item | State | Needs from the owner |
| :-- | :-- | :-- | :-- |
| 0.1 | `PROP-005` 5D — `selfhost/vm/ir_to_pilc.pilot`, `pilot run --via-ir` | Pilot part merged 2026-11-18 (`be4229f`); `--via-ir` fails the 9 cases that need `.pilc` source locations | final look at its 4-file Rust list (`pilc.rs` debug trailer, `run.rs` locations, the `milestone55` deletion, one `anticheat.rs` row) |
| 0.2 | `PROP-008` step 2 — generic `Operator` token, 30 dead operator kinds deleted | merged 2026-11-18 (`4b94e8c`) | — |
| 0.3 | `PROP-035` Amendment A1 — `anticheat.rs` test counts, `test_m27_4` step 1 | approved and built 2026-11-18 | — |
| 0.4 | PR #18 (`claude/const-read-ir-speed`) — PilIR interpreter about 2× faster on Ed25519; `PROP-036` built (a `const` list read no longer copies the list) | open | review and merge |
| 0.5 | `PROP-033` (analyzer skips stdlib units) | withdrawn 2026-11-18 | — |
| 0.6 | `PROP-007` step 4 (seed `[Inline]`) | Option M: built only if C2 fails; C2 passes | none while C2 holds |
| 0.7 | `crates/pilot-analysis/src/diagnostics.rs` (55 lines, no users since `PROP-035`) | deleted 2026-11-18 | — |

---

## Track A — The compiler back end (`PROP-005`)

Goal: Pilot (PilIR → `.pilc`) compiles everything, and the Rust `BytecodeCompiler` (24 `lower_*.rs`, `vm_gen.rs`, `codes.rs`; about −8,770 lines) is deleted.

| # | Item | Depends on | Unblocks | Needs from the owner |
| :-- | :-- | :-- | :-- | :-- |
| A1 | **5B.4 groups 3–4** — the CLI host services get `[EngineOp]` owners (group 3); delete the old `selfhost/{lexer,ast,parser}.pilot` (group 4). Group 1 and 2 are done. | — | 5B.5, PROP-004 | built 2026-11-18 (`PROP-038`, approved): 4 host services rebuilt in Pilot and their Rust deleted (−78 lines), the other 8 owned by `ToolHost`, old `selfhost/{ast,parser}.pilot` deleted; Amendments A1 (`test_61_1` deleted, milestone61 count 2 → 1 in `anticheat.rs`) and A2 (milestone01's gate bundle) approved and built the same day; left: `selfhost/lexer.pilot` goes with `PROP-004` |
| A2 | **5B.5** — checker bugs found on the toolchain (`known_bugs` repro first, then the fix), then the visibility fixes | A1 | A3 | Rust list only if a fix needs one |
| A3 | **5B.6** — the remaining rules: METH001, PURE001, COMP005 (each struct's real lattice), NULL005/006, TYPE003, FLOW005. Done when `pilot check` reports 0 errors on every live entry, kept at 0 by a new `toolchain_check` gate rule | A2 | A4 | as A2 |
| A4 | **5C** — PilIR lowers and runs the whole toolchain: every `ir.lower_construct` seam it hits is closed | A3 | A6 (T1 needs it) | as A2 |
| A5 | **5D** — Phase 0.1. Runs user programs through `--via-ir` | 0.1 | B3, A7 | (Phase 0.1) |
| A6 | **Bootstrap: T1, decided 2026-11-18** (`PROP-005` §5E) — a checked-in `toolchain.pilc` built by the Pilot path; the Rust compiler is not kept for the toolchain | A4, A5 | A7 | decided |
| A7 | **5E** — the VM boots `toolchain.pilc`; Gate 2 checks stage 2 == stage 3 | A6 | A8 | approve its itemized Rust list |
| A8 | **5F** — delete the Rust `BytecodeCompiler`; ratchet `paused_max_total_rust_loc()` down; `must_be_absent` entries | A7 | Phase 4 | approve its Rust list (the big deletion) |
| A9 | **`PROP-039`** — `.pilc` carries Pilot's Perceus `dup`/`drop`; the VM counts references and frees exactly where Pilot says; the tracing collector (`collect.rs`, 274) is deleted (AGENTS.md 12.4.5: Pilot decides, Rust acts) | A5 (Pilot part); A8 (deleting the collector) | — | draft 2026-11-18: opcodes or intrinsics; land the `+40` now or with A8 |
| A10 | **`PROP-040`** — `pilot lsp` and `pilot debug --dap` in Pilot: diagnostics, hover and definition from the checker, run in one resident process; the debugger steps the PilIR interpreter, and every pause decision is Pilot. Ends the Python editor tools and the `cli.command_handler` seam for these two commands | — | `milestone59_*.rs` test (−486) | approved 2026-11-18 (interpreter; diagnostics after typing pauses; LSP first). Part A (`pilot lsp`) built 2026-11-18, with Amendment 040-A (+1 Rust line); `milestone59` test deleted. Part B (debugger) not started |

`PROP-003` 3C (`op_checked` / `checked.rs`) was moved into this track and goes in A8.

---

## Track B — The front end (operators, then the parser)

Goal: every operator's spelling, level and meaning come from Pilot declarations, and a Pilot parser replaces the Rust one.

| # | Item | Depends on | Unblocks | Needs from the owner |
| :-- | :-- | :-- | :-- | :-- |
| B1 | **`PROP-008` step 2** — Phase 0.2 | — | B3 | (Phase 0.2); GRAM002 strictness question |
| B2 | **`PROP-019`** — type-position operators (`T?`, `A -> R`, `T1 | T2`) declare their levels in Pilot; `TypePrecedence` leaves Rust (about −25 to 0) | — (independent; best before B5) | B5 | approved 2026-11-18; built 2026-11-18 with Amendment 019-A (approved the same day) |
| B3 | **`PROP-022`, the rest** — every operator use lowers to its member call or rule template; delete `map_binop_sym`, `binop_code`, `prefix_flag`, the `TryPropagateExpr` case, the native index parse, `BinaryOp` | Pilot slices: B1, A5 (started 2026-11-18, slice 22B built; 22C–22F next). Rust deletions: A7 (5E), because the seed `BytecodeCompiler` that compiles the toolchain reads `BinaryOp` | B5, Phase 4 | its Rust list, itemized after 5E |
| B4 | **`PROP-008` step 3** — method-backed operators lower to owner methods; integer `+` lowers exactly as now on all three engines; gated on C2 | B3 | — | its Rust list |
| B5 | **`PROP-025` steps 1–3** — the Pilot parser's building blocks, checked against the Rust parser by dumping both ASTs for every file in the tree (any difference is a bug, never accepted) | B2 (B3 helps) | B6, B7 | approved 2026-11-18; Rust lists come back. Steps 1–2 built 2026-11-18 (Pilot only, `stdlib/pilot/parse/`, `PILOT_PARSE_COMPARE`); step 3 not started |
| B6 | **`PROP-034`** — a string built from many parts is allocated once (f-strings and `+` chains lower to one join) | B5 | — | its Rust/HostOp list (one appended op) |
| B7 | **`PROP-025`, the rest** — the Pilot parser builds the AST the checker and PilIR read | B5 | Phase 4 | its Rust list |

`PROP-003` 3D (`requires`/`ensures`/`old` as `syntax` rules) and 3F (small parser trims) shrink the Rust parser that B7 replaces. Build them only if they are a clear net deletion before B7 lands; otherwise drop them. 3E (`fstring.rs` reuses the lexer's hole scanner) no longer applies as written: since `PROP-021` 21H the lexer has no Rust hole scanner. It becomes "the parser reads f-string pieces from the table" inside B7.

---

## Track C — Cleanup (whenever its blocker clears)

| # | Item | Depends on | Needs from the owner |
| :-- | :-- | :-- | :-- |
| C1 | **`PROP-004`** — retire `crates/pilot-analysis` (now 1,437 `src` lines). Its tests are ported to Pilot conformance programs; `tool.rs`/`run.rs` stop serving `ast_*`/`layout_*` hooks once no Pilot tool reads the Rust AST | 0.3; A1 (5B.4 group 5: the unlinked files) | itemized file list |
| C2 | **`PROP-002b` remainder** — the other types and buckets B/D move onto owner methods (`map_*` slice done) | overlaps A1; PilIR still uses native bindings for `Map` | itemize per type |

---

## Phase 4 — The cutover

| # | Item | Depends on | Needs from the owner |
| :-- | :-- | :-- | :-- |
| P1 | **`PROP-023`** — `toolchain.pilc` gets a `LANG` section: the lexical, grammar and meaning tables compiled from the `.pilot` declarations, read by number; a genesis file | `PROP-021` (done), B3, A8 | approve when itemized |
| P2 | **`PROP-006`** — the VM boots `toolchain.pilc` directly; delete `crates/pilot-{lexer,parser,ast,span,intern}` and `pilot-cli/src/{link,check}.rs` (about −5,950) | A8, B7, P1, C1 | approve its Rust list |
| P3 | **C11 seed (`M46`)** — the remaining ~1,800-line Rust floor is rewritten in C11 | P2 | — |

---

## Done (for reference)

`PROP-001`, `002a`, `009` (all approved steps), `010`–`018`, `020`, `021` (21A–21H, A1), `022` slice 22A, `024`, `026`, `027`, `028`, `030`, `032`, `035`; `PROP-005` 5A, 5B.1–5B.3, 5B.4 groups 1–2; `PROP-003` 3A, 3B; `PROP-007` steps 1–3; `PROP-002b` `map_*` slice. `PROP-029` was superseded by `PROP-021` 21H.

## Decisions the owner holds

Decided 2026-11-18: A1 (approved), 5B.4 group 3 ("Pilot first"), `diagnostics.rs` (deleted), `PROP-033` (withdrawn), `PROP-019` and `PROP-025` (approved), the 5D Rust list and the `PROP-008` step 2 result with GRAM002 (approved on "Pilot first" terms; final lists still shown before merge), bootstrap T1. Details: the Work Queue in `proposals.md`.

Still open:
1. Merge PR #18 (`PROP-036`).
2. ERR005 warning on every `try`; `[HardRealtime]` heap checks.
3. The itemized Rust lists of later steps as each is reached (5E, 5F, `PROP-022`, `PROP-023`, `PROP-004`, `PROP-006`), and whether to keep `PROP-003` 3D/3F.

## Owner's to-fix-later list (from the checked-conversion work, 2026-11-12)

Kept for a follow-up pass. Check each item against `selfhost/gate/ledger/seams.pilot` before working on it.
1. Receiver-type planning (`selfhost/check/receivers.pilot`) covers user files only, not stdlib.
2. On a SEED001 retry, `pilot run` computes the run plan twice (`cli_check` retry, then `cli_run`).
3. Primitives can't declare `ext T : IToX` bases (known bug `seed.ext_bases_dropped`, a parser issue).
4. No `static const` members; limits are static functions such as `i32.max()`.
5. `-128i8`-style minimum literals fail (the unary minus applies after the suffixed literal).
6. Known bug `seed.ok_result_members`: `.is_err` / `.is_ok` on a successful result fails on the VM.
7. Known bugs `seed.struct_ok_unwrapped`, `seed.int_to_f32_double_round`; seam `convert.wide_scalar`; stand-in `convert.nan_bits`. (`seed.f32_not_rounded` and `convert.float_parse_slow_path` were no longer in the ledger on 2026-11-18: closed or renamed.)
8. The rename litmus fails on `tests/conformance/errors/err_pure_result_mutate.pilot` (the word `result` in its expected stderr).
