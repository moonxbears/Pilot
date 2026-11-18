# `proposals.md` — Mandatory Owner-Approval Ledger for All Rust Changes

> **Binding Governance Rules ([`AGENTS.md`](file:///home/moony/Documents/dev/projects/Pilot/AGENTS.md) §1):**
> 1. **Zero Unapproved Rust Changes:** **No** Rust file (`crates/**`, `Cargo.toml`) may ever be created, modified, or deleted until a proposal below is explicitly marked **`[STATUS: APPROVED BY OWNER]`**.
> 2. **Strict File-Scope Confinement:** When executing an approved proposal, edits, creations, and deletions are **strictly restricted to ONLY the exact Rust file paths listed in that proposal**. If an additional Rust file needs to be touched, stop immediately and amend the proposal for owner approval first.
> 3. **Immediate Dead / Legacy Rust Registration:** Whenever any agent or developer discovers dead, unreferenced, or superseded Rust code during any task, they MUST halt before moving on and log its exact file paths here for immediate owner-approved deletion.
> 4. **Airlock Freeze (Model B+, owner decision 2026-11-14; airlock contents per `PROP-002a` Amendment A1):** Once `PROP-002a` lands, [`stdlib/core/bridge_lang.pilot`](file:///home/moony/Documents/dev/projects/Pilot/stdlib/core/bridge_lang.pilot), [`crates/pilot-vm/src/bridge.rs`](file:///home/moony/Documents/dev/projects/Pilot/crates/pilot-vm/src/bridge.rs), and the ABI snapshot with its owner registry (`selfhost/gate/airlock_abi.pilot`) are frozen under B+. Every existing `HostOp` ID, sub-code, `PilotErr` code, `arch_kind` value, struct field offset, and hook signature is **frozen forever**: it is never deleted, renumbered or given a new meaning. The owner registry is checked, not frozen: each op always has exactly one owner, but an owner may move (to a `*Bundle`, out of `core.*` for `RING001`, or away entirely when `F6` implements the op in Pilot) through an owner-approved entry here that moves it in one change. Additions are **append-only**, each through its own owner-approved entry here. **One op, one owner:** each engine op appears exactly once, as the body of one `[EngineOp]` method or `comptime fn` of its owning type. Workarounds that route around the owner stay forbidden. `GATE021` enforces this.
> 5. **Mandatory Same-Change Map & Matrix Sync:** Every executed proposal MUST update [`rust.md`](file:///home/moony/Documents/dev/projects/Pilot/rust.md), [`pilot.md`](file:///home/moony/Documents/dev/projects/Pilot/pilot.md), [`docs/keyword_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/keyword_matrix.md), [`docs/concept_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/concept_matrix.md), and [`selfhost/gate/manifest.pilot`](file:///home/moony/Documents/dev/projects/Pilot/selfhost/gate/manifest.pilot) in the exact same commit.
> 6. **Break Loud & Seam Over Rust Retention:** Shrinking Rust takes absolute priority over keeping legacy Rust-backed `.pilot` paths temporarily green. When deleting misplaced Rust in an approved proposal breaks `.pilot` callers or unbuilt passes, **leave them loudly broken in Pilot via `seam(...)` (`SEAM001`, registered in `seams.pilot` / `known_bugs`)** and downgrade affected rows in `wiring.pilot`, `keyword_matrix.md`, and `concept_matrix.md`—never keep legacy Rust shims alive to avoid breaking Pilot callers. **One exception** (owner goal: run-time speed first): native bindings listed in `GATE021`'s shrink-only `TRANSITIONAL_NATIVE_BINDINGS` table (`PROP-002a` Amendment A1) stay until their owner method is at least as fast on the seed (the C2 threshold, after `PROP-007`). The table may only shrink.

---

## Work Queue (owner, 2026-11-17: "approve them and queue them up for working on")

**Execution order of every proposal, by phase and track, with dependencies: [`docs/roadmap.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/roadmap.md).** This queue is the near-term part of it.

The path to `PROP-022`, in order. Each item is built and verified on its own (three engines, `pilot gate --full`) before the next starts.

| # | Item | Approved 2026-11-17 | Still needs, before its Rust is touched |
| :-- | :-- | :-- | :-- |
| 1 | `PROP-005` 5B.3, finish — **DONE 2026-11-17** (engine bindings typed; see 5B.3 progress) | — | nothing |
| 2 | `PROP-005` 5B.4 | yes | decision **D4**, per file: rewrite each legacy `__engine` file onto owner types or delete it; Rust link-list edits (`run.rs`) itemized |
| 3 | `PROP-005` 5B.5, 5B.6 | yes | their Rust file lists, if any, itemized |
| 4 | `PROP-005` 5C | yes | its Rust file list, if any, itemized |
| 5 | `PROP-005` 5D (bytecode from PilIR, `pilot run --via-ir`) — **Pilot side built 2026-11-18** (build order 1–5; 6–7 pass without lowering of their own; see 5D "As built") | yes | owner's last look at the 5D Rust list, cut to the minimum (4 files: `pilc.rs` debug trailer +35, `run.rs` locations +20, delete the `milestone55` test −454, its `anticheat.rs` row −1; net about −400); the direction was approved 2026-11-18 on "Pilot first" terms |
| 6 | `PROP-008` step 2, remaining work: operator tokens from the declared tables — **PILOT PART BUILT 2026-11-18; RUST LIST APPROVED 2026-11-18 WITHOUT `expr.rs` AND BUILT IN PART (−80); RESULT APPROVED "Pilot first"; GRAM002 BUILT IN PILOT** (see "Step 2 Rust as built"; final Rust file list there for the last look before merging) | yes | approval of Amendment 008-C (`crates/pilot-parser/src/modules.rs`, so the program's own symbols can be lexed), or the owner's choice to move that lex to `PROP-022` with the member call |
| 7 | `PROP-022`, the rest (§3): delete `map_binop_sym`, `binop_code`, `prefix_flag`, the `TryPropagateExpr` special case, the native index parse, `BinaryOp` | yes | its Rust file list fixed when sequenced (§4) |

- **What the approval covers:** the direction and order of every item above, and all of its Pilot work, which may start as each item comes up.
- **What it does not cover (rules 1 and 2 above):** a Rust file not yet named. When an item reaches its Rust, its exact file list is written into its entry here and shown to the owner for a one-word approval before any Rust edit.
- **Owner decisions 2026-11-18** (answers to the open-decisions list, in its numbering):
  1. `PROP-035` Amendment A1: **approved** (built; see `PROP-035` §5).
  2. `PROP-005` 5B.4 group 3, the CLI host services: **"Pilot first"**. Each of the 20 calls gets a Pilot `[EngineOp]` owner (one `ToolHost` owner type for the tool natives, the five `Map` natives on `Map`), and every caller goes through the owner method. Before any Rust or the owner registry (`airlock_host_ops()`) is touched, each call is first checked for whether it can be rebuilt in Pilot over existing owners and deleted instead; only the calls that cannot are given owners, and that list comes back for approval.
  3. Delete `crates/pilot-analysis/src/diagnostics.rs`: **approved** (deleted).
  4. `PROP-033`: **withdrawn**.
  5. `PROP-019`: **approved** (roadmap B2).
  6. `PROP-025`: **approved** (roadmap B5, B7).
  7. and 8. The 5D Rust list and the `PROP-008` step 2 result, including the GRAM002 strictness question: **approved on "Pilot first" terms**. Whatever can be done in Pilot is done in Pilot; the Rust part is cut to the irreducible minimum, and the GRAM002 option that keeps the check in Pilot is taken. The itemized Rust lists still come back for a final look before they are merged.
- **Not in this queue:** `PROP-005` 5E (bootstrap **T1** decided by the owner 2026-11-18; its Rust list is not itemized yet) and 5F. `PROP-022` needs only 5D.

---

## Proposal Index

| ID | Title | Net Rust `src` Delta | Status |
| :--- | :--- | ---: | :--- |
| **`PROP-001`** | Purge Discovered Dead Rust Crates/Modules, Remove Hardcoded String Leaks & Split Packed `pilot-parser/src/lib.rs` (`MOD001`) | `-399` net (`20,812` $\rightarrow$ `20,413` `src` LOC; `-1,439` unpacked) | **`COMPLETED (APPROVED BY OWNER)`** |
| **`PROP-002a`** | Define the 3-File Airlock Under the B+ Freeze (51 scalar/pointer `HostOp`s), Retire `pilot-rt`, Consolidate Stage-A Natives & Arm `GATE021` | `<= 0` net (`9` $\rightarrow$ `8` crates; `-683` `pilot-rt`) | **`APPROVED; A1 MERGED (d055851); C2 RESOLVED 2026-11-15 (BASELINE MEASURED; OWNER THRESHOLD 5% SLOWER MAX; CHECKED BY record.pilot --check)`** |
| **`PROP-002b`** | Bucket B/D `.pilot` Migration to Owner Methods (`~1,875` `map_*` + `~800` `ast_*` sites), Retire Transitional Native Bindings & `MapBuffer` Duplicate, Retire `heap.rs`/`host.rs` | To be itemized | `DRAFT — MAP SLICE ITEMIZED 2026-11-15 (PILOT ONLY, 0 RUST; RUN BEFORE PROP-007 STEP 4 UNDER OPTION M) — APPROVED 2026-11-15, BUILT; C2 PASSES AFTER `PROP-013` (2026-11-15); PilIR STILL USES NATIVE BINDINGS FOR `Map`; OTHER TYPES AND BUCKETS B/D STILL TO ITEMIZE` |
| **`PROP-007`** | Implement `[Inline]`: PilIR Pass (Pilot), Forced Mode for `[EngineOp]` Owners, Seed Lowering (Rust) | Seed part to be itemized; offset by native-binding dispatch removal | `DRAFT — PILIR PART (STEPS 1–3) BUILT; SEED PART (STEP 4): OWNER CHOSE OPTION M 2026-11-15 (MEASURE ON PROP-002b's MAP SLICE FIRST; BUILD OPTION F ONLY IF C2 FAILS)` |
| **`PROP-008`** | Every Operator Declared by an Attribute on Its Interface or `syntax` Rule (Named Levels, Closed Parse Shapes), and Comptime User-Defined Operators | Lexer/parser operator code removed; to be itemized | `STEP 1 + AMENDMENT 008-A APPROVED 2026-11-15 — IN PROGRESS; STEP 2 (OPERATOR TOKENS FROM TABLES) APPROVED 2026-11-17 AND QUEUED (WORK QUEUE ITEM 6); STEP 2 PILOT PART BUILT 2026-11-18; ITS RUST LIST APPROVED 2026-11-18 ("Approve, drop expr.rs") AND BUILT IN PART (−80); RESULT APPROVED 2026-11-18 ON "PILOT FIRST" TERMS, GRAM002 BUILT IN PILOT; AMENDMENT 008-C (`modules.rs`) AWAITING APPROVAL; STEP 3 NEEDS AN ITEMIZED LIST` |
| **`PROP-009`** | Every Receptor Word and `__ssa` Defined by a `syntax` Rule with a `quote` Template (Sequence Rule Heads, `asm` Machine Ops, Core Forms) | Step 1a must be a net deletion (`query.rs`, `with_scope.rs`); step 1b about +60 | `STEPS 1A + 1B BUILT 2026-11-15; OPTION (C) BUILT — STOPPED: STEP 1A IS ABOUT +56 RUST, NOT YET A NET DELETION (§9); STEP 1C (SYNTAX CLASSES + RULEARM DELETION, §10) BUILT 2026-11-15 (−60 RUST; PROP-009 TOTAL STILL +66; `milestone51` RUST TEST EDIT AWAITS APPROVAL); STEP 2 `CoreForm` PHASES 2A + 2B APPROVED TOGETHER 2026-11-15 AND BUILT (§11; +186 RUST, OVER THE −15..−40 ESTIMATE, ACCEPTED BY THE OWNER 2026-11-15; `true`/`false`/`none` NOT MOVED); STEP 2C GROUP 1 (`errdefer`/`become`, §12, about −30 RUST) APPROVED 2026-11-15 AND BUILT; STEP 2C GROUP 2 (UNBUILT KEYWORDS, §13, about −90 RUST) APPROVED 2026-11-15 AND BUILT (−63 RUST); STEP 2C GROUP 3 (STRAY `else`, DEAD `select` ARM, §14, about −9 RUST) APPROVED 2026-11-15 AND BUILT WITH AMENDMENT `RejectWord` (−5 RUST); §15 (A FUNCTION'S `where` CLAUSE IS DROPPED SILENTLY) OPTION B APPROVED 2026-11-15 AND BUILT (±0 RUST); STEP 3: OPTION (b) CHOSEN BY OWNER 2026-11-15 (FILE LIST TO ITEMIZE); `milestone51` TEST EDIT APPROVED 2026-11-15 AND MADE; `emit_ssa_*` HOOKS REMOVED (§16); §17 (INLINE FOREIGN BLOCKS) OPTION R APPROVED 2026-11-15 AND BUILT (−8 RUST, NOT −18; `Rest` FIX, TEST DELETION AND `milestone51` LINE APPROVED AS EXTRAS)` |
| **`PROP-010`** | Purge Dead Rust Found 2026-11-15: the Unused Rust `comptime` Evaluator and Unreferenced Items | `-191` | `APPROVED BY OWNER 2026-11-15 AND BUILT` |
| **`PROP-011`** | Delete the Unrun Bootstrap Stage List `stdlib/core/bootstrap.pilot` (Rust Test Edit Only) | `0` (`src` unchanged) | `APPROVED BY OWNER 2026-11-15 AND BUILT` |
| **`PROP-012`** | Delete Test-Only Rust: the AST S-Expression Printer, the Rust Module Header Scanner and `TokenKind::is_keyword` | `-186` | `APPROVED BY OWNER 2026-11-15 AND BUILT` |
| **`PROP-013`** | Link a Run's Extra Toolchain Files in the Loader's One Pass (`run.rs` `compile_entry`) | `-11` | `APPROVED BY OWNER 2026-11-15 ("Approve PROP-013") AND BUILT; C2 PASSES` |
| **`PROP-014`** | Update the Benchmark Baseline Test for the Seed Column `PROP-002a` C2 Added (Rust Test Edit Only) | `0` (`src` unchanged) | `APPROVED BY OWNER 2026-11-15 ("Approve PROP-014") AND BUILT` |
| **`PROP-015`** | Compile an `=> expr` Function Body Against Its Return Type, Delete the Dead `ProbeArg::Engine`, and (C) Let a Block's Last `for` Collect on the Seed | `+23` as built (`+34`, `-11`) | `APPROVED, BUILT` |
| **`PROP-016`** | Format All Rust With `rustfmt` and Count the Seed's True Size (147 files; split `pilot-parser/src/decl.rs`) | `+7,177` `src` lines of formatting and file headers, `0` code | `APPROVED BY OWNER 2026-11-15 (D1 (a): CEILING 27,800) AND BUILT` |
| **`PROP-017`** | Update Three Rust Tests for the Package Loader: New Never-Linking Case and the Moved Inspect Selector (Rust Test Edits Only) | `0` (`src` unchanged) | `APPROVED BY OWNER 2026-11-15 ("Approve PROP-017") AND BUILT` |
| **`PROP-018`** | Operator Fixity From Holes in the Symbol (`op _+_`, `op -_`, `op _?`, `op \|_\|`, `op _[_]`), Optional Where the Parameter Count Decides; the Unpack Operators Become `op *`, `op **`, `op *..` | `+54` net in `crates/pilot-parser` (`decl.rs`, `tables.rs`; estimated `+20`); deletions follow in `PROP-008` step 2 and `PROP-005` | `APPROVED BY OWNER 2026-11-16 ("Approve as recommended": §6 decisions 1–4 as recommended) — BUILT 2026-11-16 (§7)` |
| **`PROP-019`** | Type-Position Operators Declared in Pilot: Their Levels, Associativity and Prefix Symbols Leave Rust; `op` Symbols Take Values Only | about `-25` to `0` in `crates/pilot-parser` (`tables.rs`, `stmt.rs`, `expr.rs`) and `crates/pilot-ast/src/lib.rs` | `APPROVED 2026-11-18 (owner: "approve"); QUEUED (roadmap B2)` |
| **`PROP-020`** | Read `const` Generic Parameters Written After the Colon (`N: const usize`), Closing Known Bug `parser.const_generic_struct` | `+5` in `crates/pilot-parser/src/generic_params.rs` (estimated `+3`; `rustfmt` wraps the condition) | `APPROVED BY OWNER 2026-11-16 ("Approve PROP-020") AND BUILT` |
| **`PROP-021`** | The Lexer Declared in Pilot: Token Patterns Compiled by Pilot Into a State Table the Seed Only Runs (numbers, strings, comments, names, operators; `LIT001`/`LIT002`, `LEX001`–`LEX003`; correctly rounded float literals in Pilot) | about `-900` to `-1,000` (`crates/pilot-lexer` from 1,120 to about 200; parser literal handling); one appended `bridge_lang.pilot` hook | `21A–21H BUILT (21H 2026-11-17); AMENDMENT A1 APPROVED 2026-11-17` |
| **`PROP-022`** | Operator Meaning Declared in Pilot: Every Operator Use Lowers to Its Member Call or Its Rule's Template | about `-150` or more in `crates/pilot-parser` and `crates/pilot-ast`; sequenced after `PROP-008` step 2 and `PROP-005` | `DRAFT 2026-11-16 — DESIGN; SLICE 22A (STRUCT PREFIX OPERATORS) APPROVED 2026-11-16 ("Approve 22A") AND BUILT (`lower_expr.rs` +20); THE REST APPROVED 2026-11-17 AND QUEUED AFTER PROP-008 STEP 2 AND PROP-005 5D (WORK QUEUE ITEM 7)` |
| **`PROP-023`** | The Language Section of `toolchain.pilc`: Pilot's Lexical, Grammar and Meaning Tables Precompiled, Read by Number; a Genesis File | design only; deletes the Rust lexer and parser when built | `DRAFT 2026-11-16 — DESIGN, NOT FOR BUILDING YET` |
| **`PROP-024`** | `//` Always Starts a Comment: Delete the `//=` and `//` Operator Tokens | about `-6` in `crates/pilot-lexer` (`lexer.rs`, `token.rs`); built `-4` | `APPROVED BY OWNER 2026-11-16 ("Approve PROP-024") AND BUILT` |
| **`PROP-025`** | The Parser in Pilot: One Grammar-Table-Driven Pilot Parser Builds the AST the Checker and PilIR Read; the Rust Parser Is Deleted | Pilot work first (no Rust change); at the cutover deletes `crates/pilot-parser` (9,165 lines) and `crates/pilot-ast` (917) under `PROP-006` | `APPROVED 2026-11-18 (owner: "approve"); QUEUED (roadmap B5, B7); EACH RUST LIST STILL COMES BACK` |
| **`PROP-026`** | Seed: a Struct's Operator Applies to the Result of Another Operator or a Method Call (`a + b + c`), Closing Known Bug `seed.chained_struct_operator` | about `+20` in `crates/pilot-vm/src/lower_expr.rs`; deleted with the rest of the seed bytecode compiler by `PROP-005`; built `+28` | `APPROVED BY OWNER 2026-11-16 ("Approve PROP-026") AND BUILT` |
| **`PROP-027`** | Operators a Type Gains Through `ext` Apply to Literals (`"0" ..= "9"` as a Byte-Range Pattern); a Struct's `op _[_]` Takes a Range Index (`hex[1 ..= 6]`) | small, in `crates/pilot-vm/src/lower_expr.rs`; plus Pilot lowering in `selfhost/ir/` | `APPROVED BY OWNER 2026-11-16 ("Approve PROP-027")` |
| **`PROP-028`** | Fill the `bridge.proc_signal` Seam's `signal_handle` Sub-Op, So `pilot test --timeout` Can Stop a Hung Case | `+4` in `crates/pilot-vm/src/bridge.rs` | `APPROVED BY OWNER 2026-11-16 ("Approve PROP-028") AND BUILT` |
| **`PROP-029`** | Seed Lexer: `!` Ends No Operand, So `!.on` Reads as `!self.on`, Closing Known Bug `seed.not_prefix_dot` | `0` (one argument in `crates/pilot-lexer/src/lexer.rs`) | `SUPERSEDED 2026-11-17 by PROP-021 21H` |
| **`PROP-030`** | Seed: A `const` List or Map of Any Length Compiles (`materialize_cmap` Releases Each Element's Register), Closing Known Bug `seed.const_list_registers` | `+6` in `crates/pilot-vm/src/lib.rs` | `APPROVED BY OWNER 2026-11-16 ("Approve PROP-030") AND BUILT` |
| **`PROP-032`** | Enums With Text Values (`enum CacheField { Key = "key", .. }`): `as str`, printing, `ENUM001`–`ENUM003` | about `+12` in `crates/pilot-vm/src/lib.rs` (`register_enum`) | `APPROVED BY OWNER 2026-11-16 ("Approve PROP-032")` |
| **`PROP-033`** | The Analyzer Skips Units Whose Diagnostics Are Thrown Away (`ast_unit_is_user`) | about `+10` in `crates/pilot-cli/src/run.rs`, `crates/pilot-analysis/src/{pipeline,infer}.rs` | `WITHDRAWN 2026-11-18 (owner: "withdraw"; the analyzer was deleted by PROP-035)` |
| **`PROP-034`** | A String Built From Many Parts Is Allocated Once: f-Strings and `+` Chains Lower to One `str_build`, and a Unique String Appends in Place (Perceus) | about `+15` for the seed VM's n-part build (opcode or appended `HostOp`), itemized when sequenced; the rest is Pilot and C | `DRAFT 2026-11-17 — DESIGN; SEQUENCED AFTER PROP-025 STEPS 1–3` |
| **`PROP-035`** | Stop Running the Retired `selfhost/analyzer.pilot`: Delete `run::analyze`, `cli_analyze` and the Analyzer Loader | about `-140` (`crates/pilot-cli`, `crates/pilot-analysis`) | `APPROVED 2026-11-17; BUILT 2026-11-17; A1 APPROVED AND BUILT 2026-11-18; diagnostics.rs DELETED 2026-11-18` |
| **`PROP-003`** | Wave 3, itemized 2026-11-15: slices 3A–3F (struct `invariant` rule, dead f-string desugarer, `op_checked` to Pilot, contract rules, f-string scanner dedupe, small trims) | about `-270` now (most of the planned `-2,850` waits for `PROP-005`/`006`) | `3A + 3B BUILT 2026-11-15 (−29 RUST); 3C DEFERRED TO PROP-005; 3D–3F DRAFT` |
| **`PROP-004`** | Retire Legacy `crates/pilot-analysis` Crate (`1,591` formatted `src` LOC; `647` packed) & Migrate Milestone Harnesses to `.pilot` | `-1,591` LOC (formatted) | `PLANNED (WAVE 4)` |
| **`PROP-005`** | Replace Rust `BytecodeCompiler` (`24` `lower_*.rs` + `vm_gen.rs` + `codes.rs`) with `selfhost/vm/ir_to_pilc.pilot` | about `-7,950` LOC packed, about `-8,770` formatted (PROP-016) | `ITEMIZED 2026-11-15: 5A APPROVED AND BUILT (PILOT ONLY); 5B ITEMIZED (SLICES 5B.1–5B.6); D1/D2/D3/D5 DECIDED 2026-11-15, D4 DECIDED 2026-11-17; 5B.1 AND 5B.2 APPROVED AND BUILT (PILOT ONLY); 5B.3 APPROVED AND DONE 2026-11-17 (RUST AMENDMENTS 5B.3a AND 5B.3b BUILT); 5B.4–5B.6, 5C AND 5D APPROVED 2026-11-17 AND QUEUED (WORK QUEUE ITEMS 2–5; EACH RUST LIST STILL NEEDED); 5E–5F NOT APPROVED; BOOTSTRAP T1 DECIDED 2026-11-18 ("T1: toolchain.pilc")` |
| **`PROP-006`** | Precompiled `toolchain.pilc` Stage-3 Cutover — Delete `pilot-{lexer,parser,ast,span,intern}` | `~ -5,950` LOC packed (the five crates are `11,386` formatted, PROP-016) | `PLANNED (WAVE 6 -> ~1,800 LOC FLOOR)` |

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

**Step 2 re-surveyed after `PROP-021` 21H (2026-11-18, read and run on `948eec2`).** Step 2 was written before the lexer moved into Pilot. What it still has to do, from the code:
- **Built-in operator spellings are already table-driven.** The seed lexer (`crates/pilot-lexer/src/lexer.rs`, 421 lines) has no operator byte arms left: it runs `stdlib/core/lex/toolchain.lex`, whose operator families are every `op` symbol piece and grammar-attribute symbol the standard library declares (`declared_operator_symbols`, `selfhost/check/grammar_attrs.pilot`, through `stdlib/core/lex/operators.pilot`) plus the 20 spellings and 6 call-quote openers of the shrink-only `stdlib/core/lex/seed_forms.pilot`. Longest match is the DFA's. So "a longest-match loop over the declared symbols" exists for the standard library's operators; none of the seed-form spellings is declared yet, so `seed_forms.pilot` cannot shrink without giving them a meaning (`PROP-022`).
- **Every operator spelling still has its own `TokenKind`.** `crates/pilot-lexer/src/token.rs` has 73 operator kinds. Each family's number is copied by hand into `SEED_FAMILY_KINDS` (`stdlib/core/lex/seed_kinds.pilot`), so a new standard-library operator still needs a new Rust variant (or LEX008). The parser names 43 of the 73; **30 are never named in Rust**: `Slash`, `Percent`, `ShiftLeft`, `EqEqual`, `NotEqual`, `LessEqual`, `GreaterEqual`, `Walrus`, `DoubleStarEqual`, `PipeBackward`, `Coalesce`, `JsonGetText`, `JsonContains`, `JsonContained`, `DoubleAmper`, `DoublePipe`, `Spaceship`, `PlusPipe`, `MinusPipe`, `StarPipe`, `PlusPercent`, `MinusPercent`, `StarPercent`, `PlusQuestion`, `MinusQuestion`, `StarQuestion`, `DoublePercent`, `TildeGreater`, `GreaterDotDotLess`, `SwapEqual`. The parser reads those by text only.
- **The parser reads infix operators by text from the grammar.** `expr.rs` looks every infix symbol and word up in `infix_ops` (filled from `grammar_entry` by `tables.rs` `apply_grammar_entry`, keyed by symbol text), postfix ones in `postfix_ops`, and a prefix operator's power in `prefix_ops`. The 43 kinds it names are parse-shape logic still in Rust: prefix flags (`prefix_flag`: `-`, `+`, `!`, `~`, `&`, `<-`, `*`, `**`, `*..`), ranges and cascades (`..`, `..=`, `..<`, `>..`, `?..`), safe navigation (`?.`), call quotes, compound assignments, type syntax (`<`, `>`, `>>`, `*`, `@`, `?`, `^`, `|`, `&`, `->`), `$` holes and `...`. Those are `PROP-022`'s and `PROP-025`'s, not step 2's.
- **A program's own grammar attributes already reach the seed's Pratt table**, for the file that declares them: `parse_program` ingests each item of a file whose text holds `[`, so `[Infix(::Multiply, ::Left)]` on a user `op` adds an entry while the file parses. An imported user package's entries (and its `syntax` rules) are not visible in the importing file.
- **Probes (run 2026-11-18):**
  - `[Infix(::Multiply, ::Left)] op <.>` used as `a <.> b`: `error[SYN001]: Expected RParen, found Ident`. The table lexes `<.>` as `<`, `.`, `>`.
  - The same with `@>` (one token already, a seed form with no meaning): parses through the user's entry, then the seed compiler stops with `error[SYN001]: unknown binary operator` on `pilot run`, `pilot ir run` and `pilot check` alike: `binop_code("@>")` is 0, and a `BinaryExpr` carries only a native code.
  - `op %%` on a struct with no attribute: works (`%%` is the standard library's; PROP-026 dispatch).
  - `op +-` declared, `a +-b` written: silently read as `a + (-b)` (it failed only because the struct had no prefix `-`).
  - A library's `quote { 5 +-3 }` read in a file that declares `+-` keeps the library's grouping (binary, unary, int): templates are parsed in their defining file.
  - A `syntax` rule of an imported user package: `error[DECL001]: no function named plus_neg`.

**Step 2, Pilot part, as built (2026-11-18; no Rust).**
- `selfhost/check/grammar_attrs.pilot`: **GRAM001** (an operator attribute in the program declares a built-in symbol or readable word: declared outside the user's files, or a family the seed's table already lexes as one token), **GRAM003** (a later declaration of one symbol in one position disagrees in level, associativity or shape with the first), and seam **`grammar.user_ops_seed`** on every other operator a program declares (an attribute, or an infix/prefix/postfix `op` whose symbol the seed's table has no family for), so `a +-b` can no longer silently mean `a + (-b)`. The checker now links `stdlib/core/lex/seed_kinds.pilot` and the `stdlib/core/operators/` declarations (`selfhost/cli/host.pilot`). `GRAM001`/`GRAM003` are in `selfhost/diagnostics/errors.pilot`.
- Tests: `operators/err_gram001`, `operators/err_gram003`, `operators/err_user_op_seed_seam` (three engines); known bugs `seed.user_operator` (`known_bugs/user_infix_operator`: `<.>` at `Multiply`, `Left`; grouping worked by hand, prints 24, 123, 45) and `seed.operator_hygiene` (`known_bugs/operator_hygiene` with `known_bugs/lib/hygiene_ops.pilot`: a library's quote and rule keep the library's grammar; prints binary, unary, int, 2, -2 5).
- **Not built:** **GRAM002** (needs the seed to lex a program's own symbols first; its check re-lexes each file with and without the new symbol in Pilot, over the table `stdlib/pilot/lex/modes.pilot` already runs, and reports a match of the new symbol whose ends do not fall on the old token boundaries; whether a file that only imports the symbol may have any old split change at all is an owner decision) and **GRAM004** (needs `tighter_than` / `looser_than` placement, which the attributes do not take yet; the seed reads levels only as positions in `OpLevel`). Neither has a catalog entry until it is built.

**Step 2 Rust file list (2026-11-18) — APPROVED BY OWNER 2026-11-18 WITH ONE CHANGE ("Approve, drop expr.rs"): every row below except `crates/pilot-parser/src/expr.rs`. The member call for an operator with no native code stays with `PROP-022`.** Strictly these files. Scope: one generic operator token, the program's own infix operator symbols lexed and parsed on the seed, and the dead Rust found while surveying (rule 3). Prefix and postfix user operators, `map_binop_sym`, `prefix_flag` and the remaining 43 operator kinds stay for `PROP-022` / `PROP-025`.

| File | Change | Lines |
| :--- | :--- | ---: |
| `crates/pilot-lexer/src/token.rs` | Add `Operator = 269`, the one kind of an operator family the parser does not name. Delete the 30 operator variants above, which nothing in `crates/` names. Delete `RawDslBlock` (no table family has kind 29, so it is never produced) and the unused `TokenTape::slice_str`. | about −34 |
| `crates/pilot-lexer/src/lexer.rs` | `run(t, src, extra)`: in the base mode, where one of `extra` (the program's own operator symbols) matches at `pos` longer than the table's token, emit `Operator` for it (role plain). No symbol, character or word is written in Rust. | about +14 |
| `crates/pilot-lexer/src/lib.rs` | `tokenize(table, source, extra)` passes the symbols through. | 0 |
| `crates/pilot-parser/src/lib.rs` | `tokenize(src)` passes no symbols. `parse_with_tables` / `parse_and_ingest`: when parsing a file added grammar entries whose symbol does not lex as one token, parse the file again with those symbols lexed (`extra`) and with the collected entries already in its tables, so a use may come before the declaration. | about +22 |
| ~~`crates/pilot-parser/src/expr.rs`~~ **(dropped by the owner, 2026-11-18; `PROP-022`)** | An infix use whose entry has no native code (`bin_op == 0`, a program's own operator) becomes the call of the declaring `op`'s member, `lhs.(S)(rhs)` (`MemberExpr` + `CallExpr`; the member name is `op_member_name`'s), as `PROP-022` §2 rule 1 does for every operator later. Nothing changes for an operator with a native code. | about +12 |
| `crates/pilot-parser/src/stmt.rs` | Delete the `TokenKind::RawDslBlock` arm (dead with the variant) and the no-op `if let Some((rk, _)) = self.receptor() { match rk { _ => {} } }` (5 lines that do nothing). | about −13 |
| `crates/pilot-parser/src/inline_block.rs` | `report_stray_characters`: delete the `PolyglotBlock` span filter; no such node is ever built. | about −8 |
| `crates/pilot-ast/src/lib.rs` | Delete `alloc_polyglot_block` (its one caller is the dead arm) and `NodeKind::PolyglotBlock` (its variant, `from_u16` arm and name). | about −14 |
| `crates/pilot-vm/src/lower_expr.rs` | Delete the `NodeKind::PolyglotBlock` arm. | about −4 |
| `crates/pilot-analysis/src/infer.rs` | Delete the `NodeKind::PolyglotBlock` arm. | about −1 |

- **Net:** about **−26** `src` lines (+48, −74), a net deletion as step 2 requires. `rustfmt` may move it by a few lines either way; it is measured before and after, and `paused_max_total_rust_loc()` ratchets down by the measured amount.
- **Pilot in the same change (no approval needed):**
  - `stdlib/core/lex/seed_kinds.pilot`: the 30 families above leave `SEED_FAMILY_KINDS`; an operator family (`operator_tokens`, `seed_form_tokens`) not listed gets `SEED_OPERATOR_KIND = 269`, so a new standard-library operator needs no Rust and no hand-copied number. LEX008 stays for every other family. `toolchain.lex` is regenerated (`pilot lex --write-table`, GATE024).
  - `selfhost/check/grammar_attrs.pilot`: `seed_lexes` counts the generic kind; the seam `grammar.user_ops_seed` narrows to prefix and postfix user operators and to user words (infix new symbols run); **GRAM002** is built over the re-lex.
  - `known_bugs/user_infix_operator` moves to `operators/` (it must pass on three engines); `seed.user_operator` is retired. `seed.operator_hygiene` stays until imported `syntax` rules are visible (its quote half must pass).
  - `known_bugs/compound_pow_assign`: `**=` becomes `Operator`, so `x **= 3;` turns from a silent no-op into a parse error; its actual output in `bug_registry()` is updated (the expected output does not change).
- **Acceptance:** `operators/user_infix_operator` (from `known_bugs`) passes on `pilot test`, `--ir` and `--emit c`; every other conformance program unchanged on the three engines; `cargo test -p pilot-lexer`, `cargo test -p pilot-parser --test damaged_source -- --include-ignored` (the runner and the re-parse must not panic on any input), `cargo fmt --check`; `pilot gate --full` (GATE024 with the regenerated table).
- **Risks:** a quote of a user operator's use shows a member call, not a binary node, until `PROP-022` gives every operator that shape; the re-parse doubles parse time only for a file that declares a new symbol.

**Step 2 Rust as built (2026-11-18).** Measured `crates/**/src/*.rs`: **27,290 → 27,210 (−80)**; `paused_max_total_rust_loc()` lowered to 27,210. `cargo fmt --check` is clean.
- **Final Rust file list (for the owner's last look before merging; `git diff --numstat 948eec2`, lines added / removed / net):**

| File | Change | + | − | Net |
| :--- | :--- | ---: | ---: | ---: |
| `crates/pilot-lexer/src/token.rs` | `Operator = 269`; 30 never-named operator kinds, `RawDslBlock`, `TokenTape::slice_str` deleted | 4 | 39 | −35 |
| `crates/pilot-parser/src/stmt.rs` | `RawDslBlock` arm and the no-op receptor `match` deleted | 0 | 13 | −13 |
| `crates/pilot-parser/src/inline_block.rs` | `PolyglotBlock` span filter and its two imports deleted | 1 | 14 | −13 |
| `crates/pilot-ast/src/lib.rs` | `alloc_polyglot_block`, `NodeKind::PolyglotBlock` deleted | 0 | 14 | −14 |
| `crates/pilot-vm/src/lower_expr.rs` | `NodeKind::PolyglotBlock` arm deleted | 0 | 4 | −4 |
| `crates/pilot-analysis/src/infer.rs` | `NodeKind::PolyglotBlock` arm deleted | 0 | 1 | −1 |
| **Total** | | **5** | **85** | **−80** |

  No other Rust file is touched; `lexer.rs`, `pilot-lexer/src/lib.rs`, `pilot-parser/src/lib.rs` (approved) and `expr.rs` (dropped) are unchanged. In Pilot: `seed_kinds.pilot` drops the 30 families and gives every other operator spelling `SEED_OPERATOR_KIND` (`is_spelling`); `toolchain.lex` regenerated with the old binary (the bootstrap order of 21H), GATE024 current with the new one. `selfhost/vm/compiler.pilot` loses its dead `PolyglotBlock` branch (9 `__engine` calls on the legacy ratchet). `seed_lexes` (`grammar_attrs.pilot`) counts only the families `seed_kinds.pilot` names and the seed forms (`seed_forms.pilot`, now linked into the checker), not the generic kind, so `<.>` and `+-` still stop at the seam and `@>`/`%%` are still GRAM001.
- **Not built: `lexer.rs`, `pilot-lexer/src/lib.rs` and `pilot-parser/src/lib.rs` (the program's own symbols).** A user program is not parsed through `parse_with_tables` / `parse_and_ingest`: `pilot-cli/src/link.rs` calls `parse_unit` in `crates/pilot-parser/src/modules.rs`, which builds its own `Parser` with `tokenize(src)`. The re-parse therefore has to go in `modules.rs`, which the list does not name; per Rule 2 those three rows were written, then reverted, and not committed. Without `expr.rs` the lex alone would turn `a <.> b` from `SYN001` into "unknown binary operator", so nothing a program can run changes either way.
- **Consequences, kept loud:** seam `grammar.user_ops_seed` is unchanged (every user operator with a new symbol, or with no native code, stops there); `known_bugs/user_infix_operator` still reproduces with the same actual (`error[SYN001]: Expected RParen, found Ident`), and its registry entry now names 008-C and `PROP-022`. `known_bugs/compound_pow_assign` is unchanged (still prints 2): `**=` is a seed form, so it still lexes as one token, now `Operator`, and `assign_op_of` still has no entry for it. **GRAM004** stays unbuilt.
- **GRAM002 built in Pilot (owner 2026-11-18: "Pilot first"; no Rust).** `selfhost/check/grammar_lex.pilot` reads the table the seed runs (`read_seed_table` in `stdlib/pilot/lex/seed.pilot`, over the new `written_key` in `cache.pilot`; `SEED_TABLE_FILE` moved there from `selfhost/lex/seed_table.pilot`), lexes the file that declares each new symbol with `lex_modal`, and reports GRAM002 where the symbol, matched from an operator token's start and longer than it, would end inside another token (`operators/err_gram002`: `=:` before `::Low`). A symbol that only joins whole tokens (`+` `-` into `+-`) is what the declaring file asks for and is not reported. Importing files are not lexed: on the seed an imported user package's operators are not visible, and a quote or rule keeps its defining file's grammar (`known_bugs/operator_hygiene`), so a non-importing file must not be reported. **Strictness still open for the owner:** once imports carry operators, should a file that imports a symbol be allowed to have whole tokens joined by it (lenient, the declaring file's rule) or not (strict)? The check takes either in Pilot. The checker now links `stdlib/core/{generator,os_abi,host_raw,file}.pilot` and `stdlib/pilot/lex/{modes,cache,seed}.pilot`; `pilot check` on a program declaring a new symbol takes about 0.65 s. Seam `grammar.seed_table_read` stops GRAM002 loudly when the table cannot be read. Found while linking: a private function of one linked file still collides with a public one of the same name in another (`cache.pilot`'s private `joined` replaced `selfhost/transpile/source.pilot`'s `joined` in the C emitter's link, so `--emit c` printed parameter lists as `[...]`); `cache.pilot`'s private `joined` and `word` are renamed `all_of` and `one_word`, and the duplicate entries in `cli_lex_files` and `check_fix` that the checker now links are removed.
- **Trade-off of the generic kind:** a family named by an operator spelling that no list names gets `Operator` instead of LEX008. A new delimiter (bracket-like) spelling added to `seed_forms.pilot` would therefore lex as a plain operator rather than fail the table build; brackets are named in `SEED_FAMILY_KINDS`, so this only bites a new one.

**Amendment 008-C (2026-11-18) — AWAITING OWNER APPROVAL.** Strictly these files, for the "program's own symbols" rows above:

| File | Change | Lines |
| :--- | :--- | ---: |
| `crates/pilot-lexer/src/lexer.rs` | As in the step 2 list: `run(t, src, extra)` emits `Operator` where one of `extra` matches longer than the table's token, in the base mode. | about +14 |
| `crates/pilot-lexer/src/lib.rs` | `tokenize(table, source, extra)`. | 0 |
| `crates/pilot-parser/src/lib.rs` | `tokenize(src, extra)`; `parse_with_tables` / `parse_and_ingest` re-parse as in the step 2 list. | about +22 |
| `crates/pilot-parser/src/modules.rs` | **New in this amendment.** `parse_unit`: when parsing the file added grammar entries whose symbol does not lex as one token, re-tokenize with those symbols and parse again with the entries already in its tables (the user-program path; `link.rs` calls only this). | about +12 |

- **Net:** about +48. **Value without the `PROP-022` member call:** a user infix use parses and stops at the seed compiler ("unknown binary operator") instead of `SYN001`; the checker seam still stops it first. GRAM002 becomes reachable.
- **Alternative (recommended):** do not approve 008-C now; fold the four rows into `PROP-022`, which adds the member call, so the Rust lands only when a user operator can actually run.
- **Why this part cannot move to Pilot:** the seed lexes every `.pilot` file in Rust from a table fixed before any program is read; a program's symbols are known only after its attributes are parsed, so taking them needs the seed's lexer to accept them (or a table rebuilt per program, which is a 10 s+ DFA build per check). Everything else of step 2 (the families, the generic kind, GRAM001/002/003, the seam) is Pilot.


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

**Status: APPROVED by the owner 2026-11-15 (parts A, B and C); BUILT 2026-11-15.** Found 2026-11-15 while clearing the `pilot-cli` test failures that predate this session's work (they fail at `2a82f59` too).

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

**As built (2026-11-15).** Only the five listed files changed.

| File | Change |
| :--- | :--- |
| `crates/pilot-vm/src/lower_stmt.rs` | A: an `=> expr` body is compiled with `compile_with_hint(.., cur_ret_type)`. C: `compile_block` takes `want_value`, leaves `stmt_for` unset for its last item when the value is wanted, and gives that `for`'s list as the block's value; `compile_let` sets `want_value` before an initializer that is a block or `if`; `compile_if` passes it on to its branches; new `carries_value` (a block or `if`, bare or as a statement). +26 / −3. |
| `crates/pilot-vm/src/lib.rs` | C: the `want_value: bool` field (the compiler derives `Default`, so no initializer). +4. |
| `crates/pilot-vm/src/lower_probe.rs` | B: `ProbeArg::Engine` and both arms deleted. −4. |
| `crates/pilot-vm/src/lower_expr.rs` | Not touched: `compile_if` lives in `lower_stmt.rs`. |

- Net +23 Rust lines (+34, −11), above the +16 estimate: C needed the block-value arm in `compile_block` and a helper. The flag is set only right before compiling a node that is itself a block or `if`, which takes it at entry, so a lambda or loop body never sees it.
- `types/expr_body_variant_hint` and `generators/block_value_collecting_for` (moved out of `known_bugs`, expected outputs unchanged) pass on all three engines; both `bug_registry()` entries are deleted.

**Not included.**
- `pilot ir run` reports LOOP005 ("`continue a..b` is only supported inside a `for` loop") for a `continue 5..6;` that is inside a `for` (`tests/ir/unlowered_continue_range.pilot`; `cli_f5` `an_unlowered_construct_stops_at_its_seam` fails on it). The check is in the legacy `selfhost/analyzer.pilot`, which walks `ast_parent` links built by `crates/pilot-analysis/src/infer.rs`; since `for` became a `syntax` rule (PROP-009 step 2) that walk no longer reaches the `for`. `pilot check` and `pilot run` report the right SEED001. Not yet diagnosed far enough to name a fix. The same analyzer warns LOOP005 "Potential infinite loop ... no 'break'" for `while true { guard some x = f() else { break; }; .. }`: it does not see a `break` inside a `guard`'s `else` (also a `syntax`-rule form since PROP-009). `runtime/stdin_lines` is written with `if some .. else` so `cli_f3` `check_passes_a_valid_program_without_running_it` stays clean.
- The checker accepts a `T?` where a `T` is expected (`f() + 1` with `f() -> i64?`). That is its own design question (a new diagnostic code).

---

## `PROP-016` — Format All Rust With `rustfmt` and Count the Seed's True Size

**Status: APPROVED by the owner 2026-11-15 ("Approve, raise ceiling": D1 (a)); BUILT 2026-11-15.** Found 2026-11-15, when `cargo fmt --check` first ran on this machine (rustfmt was not installed until then).

**As built.**
- The trailing spaces on `lower_expr.rs` lines 387 and 555 were removed, then `cargo fmt` ran. `cargo fmt --check` exits 0.
- `cargo fmt` changed **147** distinct files, not 154. rustfmt's report names `pilot-analysis/tests/common/{anticheat,seed_policy,test_metrics}.rs` again through `pilot-{cli,lexer,parser}/tests/../../` paths, so 7 entries in the list below are duplicates. `git diff --name-only` equals the de-duplicated list.
- `decl.rs` is now 1,305 lines. `decl_syntax.rs` (202 lines) holds `parse_syntax_decl`, `parse_syntax_class` and `rule_body_is_quote`; `decl_bind.rs` (158 lines) holds `parse_bindings` and `parse_binding_init`. The code moved unchanged, and `lib.rs` has the two `mod` lines.
- `crates/*/src` is 27,724 lines across 88 files: 20,547 packed, plus 14 lines of new file headers. `paused_max_total_rust_loc()` is `27,800` (`selfhost/gate/manifest.pilot`).
- `rust.md` counts are refreshed, with a note that counts are now of formatted source.
- **D2, re-measured on formatted source:**

  | Estimate | Packed (HEAD) | Formatted |
  | :--- | ---: | ---: |
  | `PROP-005`: `pilot-vm` `lower_*.rs` + `vm_gen.rs` + `codes.rs` (26 files) | 6,999 | 7,770 |
  | `PROP-005`: plus `pilot-vm/src/lib.rs`'s compiler part (lib.rs whole: 1,022 → 1,074) | about −7,950 | about −8,770 |
  | `PROP-006`: `pilot-{lexer,parser,ast,span,intern}/src` | 7,058 | 11,386 |
  | `PROP-004`: `pilot-analysis/src` | 647 | 1,591 |
  | Rust test files (`crates/*/tests`, 70 files) | 28,415 (stated) | 31,051 |

  The `~1,800` LOC floor was an estimate for code not yet written. It was not re-measured, and should be read as packed lines.
- **Verification (2026-11-15):**
  - `cargo fmt --check` passes, and `cargo build` gives no warnings.
  - The three engine suites match the results from before formatting: `pilot test` 554/0; `--ir` fails only `check_fix_borrows` and `proc_large_output`; `--emit c` fails `errdefer`, the 7 `err_bridge_*` tests and the same two.
  - `pilot gate` passes 4/4, and `damaged_source --include-ignored` passes.
  - `cargo test --workspace --no-fail-fast` fails 19 tests. To sort them, the same tests were run on a copy of this tree that kept HEAD's unformatted crates:
    - These fail on both trees, so they predate this change: `milestone02` m2_3, `milestone04` (4), `milestone08` m8_1, `milestone12` m12_2, `milestone16` m16_4, `milestone57` 57_1, `milestone58` (6), `milestone61` 61_1 and 61_3, `cli_f5` `an_unlowered_construct_stops_at_its_seam`. `milestone58` also hangs on both trees: its binary sits idle until it is killed.
    - `milestone01` m1_1, m1_2 and m1_4 pass when rerun. Their failures came from a run that started before the ceiling was raised, or overlapped with the `--emit c` suite.
    - `cli_changed` `a_documentation_commit_runs_only_the_cases_that_never_link` and `a_lowerer_change_reaches_every_ir_case_but_no_seed_case` come from PROP-005 5A, not from formatting. The first now reaches 3 never-linking cases instead of 2, because 5A added `modules/err_package_cycle`; its expectation is in Rust. In the second, `selfhost/cli/test_changed.pilot` sees that `wiring.pilot`'s package declaration differs from commit `95f85ad`'s and runs every case. Both are left for a follow-up.

**Why.**
- `cargo fmt --check` fails on 154 of the workspace's Rust files. AGENTS.md requires surviving Rust to pass it and forbids packing statements onto long lines to keep the line count down (§1.1).
- Formatting changes no behaviour, but it shows the seed's real size:

  | Measured on a scratch copy, 2026-11-15 | Before | After `cargo fmt` |
  | :--- | ---: | ---: |
  | `crates/*/src/**/*.rs` (what GATE003 counts) | 20,547 | 27,710 (+7,163) |
  | all workspace `.rs` (with tests) | 48,645 | 58,761 (+10,116) |
  | `src` files over 1,500 lines (MOD001) | 0 | 1 (`pilot-parser/src/decl.rs`, 1,653) |

- So formatting by itself would fail the gate twice: the total goes over `paused_max_total_rust_loc()` (21,000), and `decl.rs` goes over the per-file cap.
- rustfmt cannot format one spot. `crates/pilot-vm/src/lower_expr.rs` lines 387 and 555 end with a space after `return self.error(crate::codes::SEED,`, and rustfmt stops with `left behind trailing whitespace`.

**Change (Rust: formatting only, plus one split).**
1. Delete the trailing space on `crates/pilot-vm/src/lower_expr.rs` lines 387 and 555.
2. Run `cargo fmt` on the workspace. This rewrites exactly the 154 files listed below and nothing else.
3. Split `crates/pilot-parser/src/decl.rs` (1,653 lines once formatted) at function boundaries, moving code unchanged:
   - new `crates/pilot-parser/src/decl_syntax.rs`: `parse_syntax_decl`, `parse_syntax_class`, `rule_body_is_quote` (about 200 lines);
   - new `crates/pilot-parser/src/decl_bind.rs`: `parse_bindings`, `parse_binding_init` (about 150 lines);
   - `crates/pilot-parser/src/lib.rs` gains the two `mod` lines.
   - `decl.rs` keeps about 1,300 lines.
4. Add `rustfmt.toml`? **No.** Default settings, so `cargo fmt --check` means what AGENTS.md says.

**Pilot side (no approval needed, listed for completeness).**
- `selfhost/gate/manifest.pilot` `paused_max_total_rust_loc()`: raise to the formatted count, rounded up (proposed `27,800`), with a comment that the raise is formatting only, not growth. **This is an owner decision (D1 below).**
- `rust.md` line counts refreshed.
- `wiring.pilot`/`concept_matrix.md` rows that quote the seed's line count are updated to the formatted figure.

**Owner decisions.**
- **D1, the ceiling.**
  - (a) Raise `paused_max_total_rust_loc()` to `27,800`, so that it measures formatted lines from now on.
  - (b) Keep `21,000`, and fail the gate until enough Rust is deleted (`PROP-005` alone is about −7,950 unformatted, so roughly −10,000 formatted).
  - **Recommendation: (a).** The 21,000 was set on packed lines, so it never measured what it was meant to. (b) blocks every other change behind PROP-005.
- **D2, earlier estimates.** Proposal estimates (PROP-005's −7,950, PROP-006's ~−5,950, the `~1,800` LOC floor) were measured on packed lines. Re-measure them after formatting? **Recommendation:** yes, as part of this change.

**Acceptance.**
- `cargo fmt --check` exits 0.
- `cargo build` and `cargo test` give the same results as before (apart from the pre-existing `cli_f5 an_unlowered_construct_stops_at_its_seam` failure).
- `pilot test` on all three engines gives the same results as before.
- `pilot gate` and `pilot gate --full` pass with the new ceiling.
- No `src` file is over 1,500 lines.
- `git diff --stat` touches only the 154 listed files, plus `decl_syntax.rs`, `decl_bind.rs`, `manifest.pilot`, `rust.md` and the ledger rows.

**Exact Rust files to modify (154, all by `cargo fmt`; `lower_expr.rs` and `decl.rs`/`lib.rs` also by hand as above).**
  - **`pilot-analysis`** (53: 5 `src`, 48 tests and other): `src/diagnostics.rs`, `src/infer.rs`, `src/lib.rs`, `src/pipeline.rs`, `src/types.rs`, `tests/common/anticheat.rs`, `tests/common/seed_policy.rs`, `tests/common/test_metrics.rs`, `tests/milestone01_seed_unlock_and_pilot_gate.rs`, `tests/milestone02_module_graph_and_comptime_parity.rs`, `tests/milestone03_declarative_cli_registry_and_comptime_parity.rs`, `tests/milestone04_dual_syntax_pairings_and_bidirectional_formatter.rs`, `tests/milestone05_shared_incremental_compiler_query_database.rs`, `tests/milestone06_first_class_typed_ssa_mid_level_ir.rs`, `tests/milestone07_zero_black_box_transparency_tool.rs`, `tests/milestone08_progressive_diagnostics_and_cst_autofix.rs`, `tests/milestone09_three_ring_standard_library_personality.rs`, `tests/milestone10_compiler_as_a_library.rs`, `tests/milestone11_adapter_blueprint_engine.rs`, `tests/milestone12_headerless_c_semantic_absorber.rs`, `tests/milestone13_adaptive_hardware_calling_conventions.rs`, `tests/milestone14_perceus_foreign_resource_and_scope_guards.rs`, `tests/milestone15_cpp_class_raii_vtable_and_exceptions.rs`, `tests/milestone16_rust_crate_and_zero_copy_adapter.rs`, `tests/milestone17_python_capi_and_zero_copy_tensor.rs`, `tests/milestone18_javascript_napi_and_typescript_dts.rs`, `tests/milestone19_wasm_component_model_and_wit.rs`, `tests/milestone20_live_schema_and_wire_protocol_absorber.rs`, `tests/milestone21_boilerplate_weaver_and_structural_delegation.rs`, `tests/milestone22_bundle_facet_algebraic_composition.rs`, `tests/milestone23_pipeline_fusion_each_and_cascade.rs`, `tests/milestone24_pchor_sublanguage_choreographer.rs`, `tests/milestone25_storage_and_memory_layout_choreography.rs`, `tests/milestone26_typestates_and_design_by_contract.rs`, `tests/milestone27_scoped_user_sublanguages_and_hygiene.rs`, `tests/milestone28_progressive_learning_profiles_and_coach.rs`, `tests/milestone29_capability_sandboxed_comptime_execution.rs`, `tests/milestone30_self_generating_blueprints.rs`, `tests/milestone50_lossless_bridge_and_reflection.rs`, `tests/milestone52_pilot_preprocessor_and_receptors.rs`, `tests/milestone53_cst_recovery_and_formatter.rs`, `tests/milestone54_perceus_ownership_and_escape.rs`, `tests/milestone55_selfhost_vm_compiler_and_debug_table.rs`, `tests/milestone56_layout_driven_codegen_and_gpu_backends.rs`, `tests/milestone57_bidirectional_transpilers_and_ingest.rs`, `tests/milestone58_runtime_crypto_wire_db_and_polyglot.rs`, `tests/milestone59_persistent_pilvm_lsp_and_dap.rs`, `tests/milestone60_selfhost_cli_pkg_perf_and_bench.rs`, `tests/milestone61_stage3_fixed_point_and_universal_rename_gate.rs`, `tests/phase1_ironclad_killswitch.rs`, `tests/phase1_rename_torture_gate.rs`, `tests/polyglot_transport_and_generative_embedding.rs`, `tests/seed_wide_operands.rs`.
  - **`pilot-ast`** (1: 1 `src`, 0 tests and other): `src/lib.rs`.
  - **`pilot-cli`** (19: 6 `src`, 13 tests and other): `src/check.rs`, `src/host.rs`, `src/link.rs`, `src/main.rs`, `src/run.rs`, `src/tool.rs`, `tests/cli_attrs.rs`, `tests/cli_bench.rs`, `tests/cli_changed.rs`, `tests/cli_consts.rs`, `tests/cli_coverage.rs`, `tests/cli_emit.rs`, `tests/cli_f3.rs`, `tests/cli_f4.rs`, `tests/cli_f5.rs`, `tests/cli_f6.rs`, `tests/cli_f7.rs`, `tests/cli_keyword_matrix.rs`, `tests/../../pilot-analysis/tests/common/test_metrics.rs`.
  - **`pilot-lexer`** (8: 4 `src`, 4 tests and other): `src/lexer.rs`, `src/lib.rs`, `src/simd.rs`, `src/token.rs`, `tests/milestone52_seed_lexer_and_preprocessor.rs`, `tests/../../pilot-analysis/tests/common/anticheat.rs`, `tests/../../pilot-analysis/tests/common/seed_policy.rs`, `tests/../../pilot-analysis/tests/common/test_metrics.rs`.
  - **`pilot-parser`** (33: 25 `src`, 8 tests and other): `src/accessors.rs`, `src/attr_hooks.rs`, `src/comptime_if.rs`, `src/contracts.rs`, `src/declared_names.rs`, `src/decl.rs`, `src/expr.rs`, `src/forms.rs`, `src/fstring.rs`, `src/generic_receiver.rs`, `src/inline_block.rs`, `src/int_lit.rs`, `src/lib.rs`, `src/loud.rs`, `src/modules.rs`, `src/num_suffix.rs`, `src/open_range.rs`, `src/param_mods.rs`, `src/pilot_impls.rs`, `src/ret_type.rs`, `src/rule_arms.rs`, `src/stmt.rs`, `src/tables.rs`, `src/template.rs`, `src/tuple_member.rs`, `tests/bootstrap_sources.rs`, `tests/damaged_source.rs`, `tests/milestone51_homoiconic_syntax_and_contraction.rs`, `tests/native_calls.rs`, `tests/../../pilot-analysis/tests/common/anticheat.rs`, `tests/../../pilot-analysis/tests/common/seed_policy.rs`, `tests/../../pilot-analysis/tests/common/test_metrics.rs`, `tests/register_layouts.rs`.
  - **`pilot-vm`** (40: 38 `src`, 2 tests and other): `src/bridge.rs`, `src/checked.rs`, `src/collect.rs`, `src/heap.rs`, `src/host.rs`, `src/instruction.rs`, `src/lib.rs`, `src/lower_args.rs`, `src/lower_call.rs`, `src/lower_cascade.rs`, `src/lower_closure.rs`, `src/lower_contract.rs`, `src/lower_defer.rs`, `src/lower_destructure.rs`, `src/lower_dispatch.rs`, `src/lower_drop.rs`, `src/lower_expr.rs`, `src/lower_for.rs`, `src/lower_global.rs`, `src/lower_members.rs`, `src/lower_native.rs`, `src/lower_pipe.rs`, `src/lower_probe.rs`, `src/lower_program.rs`, `src/lower_stmt.rs`, `src/lower_struct.rs`, `src/lower_switch.rs`, `src/lower_try.rs`, `src/lower_unit.rs`, `src/lower_variant.rs`, `src/lower_yield.rs`, `src/ops.rs`, `src/pilc.rs`, `src/stage_a_natives.rs`, `src/strings.rs`, `src/value.rs`, `src/vm_gen.rs`, `src/vm.rs`, `tests/linked_interface_dispatch.rs`, `tests/runtime_memory.rs`.

**Exact Rust files to create (2):** `crates/pilot-parser/src/decl_syntax.rs`, `crates/pilot-parser/src/decl_bind.rs`.

**Rust files to delete:** none.

---

## `PROP-017` — Update Three Rust Tests for the Package Loader: New Never-Linking Case and the Moved Inspect Selector (Rust Test Edits Only)

**Status: APPROVED by the owner 2026-11-15 ("Approve PROP-017"); BUILT 2026-11-15.** Only the listed lines changed. `cargo test -p pilot-cli --test cli_changed` passes 9/9, `milestone07` passes 4/4, and `cargo fmt --check` passes. Found while fixing the two `cli_changed` failures PROP-016's verification turned up.

**Pilot side, already done (no approval needed).**
- `selfhost/cli/test_changed.pilot`: for `--since A..B`, `repackaged` compared each file's package at `A` with the working tree, not with `B`. Since PROP-005 5A renamed every selfhost package, any historical range that touched a selfhost file counted as a package change, and `--changed` ran every case. It now compares `A` with `B` (`later_rev`, `write_at_rev`), and uses the working tree only when `--since` names one revision.
- The seed CLI linked all of `selfhost/ir`, so any lowerer change reached every seed case. Cause: under the package loader, `selfhost/cli/inspect_cmd.pilot`'s `use selfhost.inspect.*` loaded every lens, and `layout_lens.pilot` imports `selfhost.ir.*`. The CLI used one function from that package (`parse_inspect_selector`), and nothing else uses the selector. So `selfhost/inspect/selector.pilot` moved to `selfhost/cli/inspect_selector.pilot` (package `selfhost.cli`), and the import is gone. Its duplicate `nameof` shim was dropped, since `inspect_cmd.pilot` defines the same one in the same package.
- Results:
  - `pilot test --changed --since 95f85ad~1..95f85ad` now reaches only the never-linking cases on the seed engine.
  - Under `--ir` it still runs every case.
  - `pilot test` is 554 passed, 0 failed.

**Rust test edits requested (no `src` change).**
1. `crates/pilot-cli/tests/cli_changed.rs`:
   - The comment at lines 230–232 and `a_documentation_commit_runs_only_the_cases_that_never_link`, lines 242 and 247–250: the never-linking cases are now 3. PROP-005 5A added `tests/conformance/modules/err_package_cycle.pilot`, which stops at MOD002 before linking. `"... 2 of "` becomes `"... 3 of "`, and the expected list gains `"tests/conformance/modules/err_package_cycle.pilot"` after `err_import_cycle`.
   - `a_lowerer_change_reaches_every_ir_case_but_no_seed_case`, line 260: `picked(&stdout).len()` goes from `2` to `3`, for the same case.
2. `crates/pilot-analysis/tests/milestone07_zero_black_box_transparency_tool.rs`, lines 22 and 100: `"selfhost/inspect/selector.pilot"` becomes `"selfhost/cli/inspect_selector.pilot"`. Its required functions (`target_kind_name`, `parse_int`, `parse_inspect_selector`) and markers are unchanged. Until this edit, all 4 of its tests fail on the missing file (`milestone07...rs:111`).

**Acceptance.** `cargo test -p pilot-cli --test cli_changed` and `cargo test -p pilot-analysis --test milestone07_zero_black_box_transparency_tool` pass, and `cargo fmt --check` passes.

**Not included.** `pilot inspect` is not registered as a command (`CLI001: Unknown command 'inspect'`). It was not registered before this change either. And 119 selfhost files each define `pub pure fn nameof(sym: str) -> str => sym;`, which nothing calls because `nameof(..)` parses as the language construct. Both are left for follow-ups.

---

## `PROP-018` — Operator Fixity From Holes in the Symbol, Optional Where the Parameter Count Decides; the Unpack Operators as `op`s

**Status: APPROVED BY OWNER 2026-11-16 ("Approve as recommended"): the §3 Rust files and the four §6 decisions as recommended. BUILT 2026-11-16 (§7).** Third wording, after these owner directions on 2026-11-16:
- the unpack interfaces should declare operators (`*`, `**`, `*..`), not named methods;
- the fixities are `infix`, `prefix`, `postfix`, `circumfix`, `mixfix` and `matchfix`;
- fixity comes from **holes in the symbol** (`_` where an operand goes), and the holes are **optional until a specific fixity is needed**.

### 1. Holes give the fixity

An `_` written in an `op`'s symbol marks an operand position. The pattern decides the fixity; no keyword is added.

| Fixity | Pattern | Use site | Example |
| :--- | :--- | :--- | :--- |
| `infix` | `_S_` | `a S b` | `op _+_ (rhs: T) -> T` |
| `prefix` | `S_` | `S a` | `op -_ () -> T`, `op *_ () -> TSeq` |
| `postfix` | `_S` | `a S` | `op _? () -> TOk` |
| `circumfix` | `S_E` | `S a E` | `op \|_\| () -> T` (`\|v\|`) |
| `matchfix` | `S_, ..E` | `S a, b, .. E` | `op <\|_, ..\|> (items: slice<T>) -> Self` |
| `mixfix` | any other pattern of symbols and holes | | `op _[_] (idx: TIdx) -> TOut`, `op _[_]=_ (idx: TIdx, val: TVal)` |

The receiver fills the first hole, and the parameters fill the rest in order (a `matchfix` list fills its one list hole). So the number of holes is the parameter count plus one, except for `matchfix`, which has one list parameter and no receiver hole.

**Holes are optional.** An `op` written without holes keeps today's meaning, decided by its parameter count:
- **one parameter:** `infix`, so `op + (rhs)` is `op _+_ (rhs)`;
- **none:** `prefix`, so `op - ()` is `op -_ ()`, and `op * ()` is the splat operator;
- **`op []` and `op []=`:** the `mixfix` index forms `_[_]` and `_[_]=_`, as the parser reads them today.

Holes are needed only where the default is wrong: `postfix` (`op _? ()`), `circumfix`, `matchfix` and any other `mixfix`. Anyone may also write them for clarity.

**`_` inside symbols.** A hole is a standalone `_` token. Because `_` lexes as an identifier character, there is no conflict:
- a word operator that contains an underscore (`not_in`) stays one identifier token and is never a hole;
- a hole beside a word operator needs a space: `op _ and _`, since `_and_` would lex as one word;
- `_` on its own can never be an operator symbol. It is already the wildcard pattern. `GRAM005` gains this case.

Checked on the seed 2026-11-16, with no change: a struct declaring `op _+_ (rhs)`, `op -_ ()`, `op _? ()` and `op |_| ()` parses, checks and runs (`parse_op_decl` joins the tokens up to `(`).

### 2. Member names

Today the seed parser names an `op` method by its text (`crates/pilot-parser/src/decl.rs:1000` `parse_op_decl`). The seed (`method_owners`, `{owner}.{name}`) and the Pilot checker and lowerer (`prog_member(p, t, "==")`, `"[]"`) look it up under that name. So `op * (rhs)` (multiply) and `op * ()` (splat) get one entry, and one replaces the other; `**` collides the same way. `-` already collides unnoticed: a struct with `op - ()` and `op - (rhs)` fails on the seed with `runtime error: cannot negate record` at `-a`, and `pilot ir run` stops at `SEAM001 ir.lower_construct` on `a - b`.

The member name becomes the **canonical pattern**:
- `infix` and the two index forms keep today's bare names (`+`, `==`, `[]`, `[]=`), whether written with holes or without, so no existing lookup changes;
- every other fixity keeps its holes: `-_`, `*_`, `**_`, `*.._`, `_?`, `|_|`. A no-hole, no-parameter `op S ()` is named `S_`.

The fixity names (`infix` … `matchfix`) are used by diagnostics, `pilot inspect` and the docs. They are not keywords.

### 3. Rust (needs approval). Strictly these files:

| File | Change |
| :--- | :--- |
| `crates/pilot-parser/src/decl.rs` | `parse_op_decl` (`:1000`) forms the canonical name from §2: `_S_`, `_[_]` and `_[_]=_` become `S`, `[]` and `[]=`; a pattern without holes and without parameters gets a trailing `_`; other patterns are kept as written. A hole is recognised as a token whose text is exactly `_` (structural, not a word). |
| `crates/pilot-parser/src/tables.rs` | The grammar ingest of an `op` whose attribute gives no symbol (`:365-379`, `ingest_decl_attrs`) takes the `op`'s name with its holes removed, so `[Prefix(::Splat)]` on `op *_` still declares `*`. |

Expected size: about `+20` lines. `paused_max_total_rust_loc()` holds flat only if the owner accepts that or an offsetting deletion is found in the same change. Nothing in `crates/pilot-vm` changes: once the parser names the methods apart, `method_owners`, the function entries and `compile_unpack_operand` (which already takes the method name from Pilot) work as they are.

**3.2 as built (2026-11-17).** Where it differs from the table above, and why:
- **Rust, strictly the six listed files.** `crates/pilot-lexer` went from 1,026 lines (`lexer.rs` 535, `simd.rs` 217, `token.rs` 245, `lib.rs` 90, `keywords.rs` 29) to 707 (`lexer.rs` 421, `token.rs` 266, `lib.rs` 20); with the parser's `lib.rs` (+28) and `expr.rs` (+5), net −376 Rust lines, not the estimated −640. The runner itself is about 120 lines; the rest of `lexer.rs` reads and checks the table file (every count, state, class and kind is checked on load, so a damaged table is a load error, never a panic while lexing).
- **`load_table(text)`, not `load_table(path)`:** `pilot-lexer` reads no file. `crates/pilot-parser/src/lib.rs` reads `stdlib/core/lex/toolchain.lex` from the source root once (`LEX_TABLE_FILE`, a `OnceLock` like the bootstrap tables) and stops with a message naming the file and `pilot lex --write-table` when it is missing or unreadable. The parser's own `tokenize(src)` runs it, so `modules.rs`, `fstring.rs` and `tables.rs` are unchanged.
- **`TokenKind::from_u16`** is generated with the enum by one `macro_rules!`, so the list of numbers is written once.
- **The f-string merge is name-blind:** a token that enters a mode from the base mode starts one token that runs until the stack is back to the base mode, with that token's kind and flags. A byte no token starts inside it ends the f-string there (the parser reports it unclosed); a token of the error kind inside it ends it and is an `ErrorToken`.
- **Pilot additions found by the comparison** (all declarations, no Rust):
  - a hole's format spec is one token (`format_spec`, `strings.pilot`): `:` and a run with no space, quote, bracket, brace or `:`, before the hole's `}`. Retires known bug `lex.fstring_format_spec`. A hole has no `#` line or shebang (`LINE_START_TRIVIA`, `trivia.pilot`);
  - a number with a misplaced separator (`1__000`, `1000_`, `0x_FF`, `1_.5`, `100_u8`) is one token (`*_misplaced`, `numbers.pilot misplaced_separator_tokens`), read by the seed as the number kind the old lexer gave, so the checker still reports LIT001 with its message (the five `numbers/err_lit001_*` tests failed with SYN001 without it);
  - a string never closed is one token (`unclosed_string`), which the seed reads as a string (its parser reports it unclosed, as before) and the tape reports as LEX001;
  - `\"` in a hole (owner: "Reject it") is its own family, `hole_escaped_quote`, read by the seed as an error token and reported by the tape as **LEX009** with the fix-it (`REJECTED_FORMS` in `lexicon.pilot`, used by `build_tape`). The 15 files that used it were rewritten to plain quotes with their expected output unchanged.
  - The hole mode now differs from the main mode, so it is no longer written `shares 0`: the table is 246 kB (380 families).
- **Comparison** (scratch dumper outside `crates/`, both lexers over the same 1,299 files): 1,298 identical in span, kind and flags, `Newline` tokens included. The other is `operators/not_prefix_dot`, where the table is right (known bug `seed.not_prefix_dot` retired; `PROP-029` superseded).
- **Lexing rate** (release build, 5.3 MB of `.pilot`, best of 3 rounds per run, two runs): the table runner 165–188 MB/s, the old SIMD lexer 338–397 MB/s, so about 2.1× slower. A report, not a gate.
- **Tests:** `lex/seed_words`, `lex/call_quote_tokens`, `lex/dimensional_numbers` (from the deleted unit tests), `lex/fstring_format_spec`, `lex/err_lex009_escaped_quote_in_hole`, `operators/not_prefix_dot` (promoted).
- **Not done, needs a new approval (`PROP-021` Amendment A1 below).**

**PROP-021 Amendment A1 (2026-11-17) — APPROVED BY OWNER 2026-11-17 ("Approve A1").** Strictly the files below. Rust that 21H left broken or dead, outside the files 21H may touch:
| File | Change | Why |
| :--- | :--- | :--- |
| `crates/pilot-lexer/tests/milestone52_seed_lexer_and_preprocessor.rs` | Delete. | It calls `tokenize(src)` with one argument and reads `keywords.rs`/`simd.rs`, so `cargo test -p pilot-lexer` no longer compiles. What it checks of the lexer is now `lex/seed_words`, `lex/call_quote_tokens` and `lex/dimensional_numbers` (AGENTS.md §9.1: language behaviour is tested in Pilot). |
| `crates/pilot-analysis/tests/common/anticheat.rs` | Allow `stdlib/core/lex/toolchain.lex` beside `stdlib/pilot.json`. | Since 21G (already committed) every test using the anti-cheat guard fails with `UNEXPECTED_DATA_FIXTURE` for the generated table (e.g. `pilot-parser`'s `milestone51`). The file is generated from the declarations and checked by GATE024. |
| `crates/pilot-parser/src/fstring.rs` | Delete `unescape_hole_quotes` and the `ESCAPED_QUOTE` handling in `fstring_hole_end`. | Dead: the table no longer lets `\"` into a hole (owner: "Reject it"). |
| `crates/pilot-lexer/src/token.rs` | Delete `PRECEDED_BY_WS`, `PRECEDED_BY_NEWLINE`, `FOLLOWED_BY_LPAREN`. | Dead: nothing sets or reads them since the runner. |
| `crates/pilot-lexer/Cargo.toml`, `crates/pilot-cli/Cargo.toml`, `crates/pilot-analysis/Cargo.toml` | Drop the unused `pilot-intern` dependency of `pilot-lexer`, and the unused `pilot-lexer` dependency of `pilot-cli` and `pilot-analysis`. | Dead dependencies (no `use` of them). |

Estimate: about −560 lines (the milestone test is about 520).

**A1 as built (2026-11-17).** All five rows done. In `anticheat.rs`, besides the allow-list line, the deleted test's entry in `MILESTONE_TEST_MANIFEST` is removed: the guard requires every listed file to exist, so keeping it would fail every guarded test (`MISSING_MILESTONE_TEST`). The empty `crates/pilot-lexer/tests/` directory is gone.

### 4. Pilot (no approval needed; listed so the slice is whole)

1. **Unpack interfaces** (`stdlib/core/operator.pilot`). No holes are needed, because no parameters already means `prefix`:
   ```pilot
   ISplatOperator<TSeq: IAnyValue> : IAnyValue {
       [Prefix(::Splat)]
       pub pure op * () -> TSeq;
   }
   IPairsOperator<TPairs: IAnyValue> : IAnyValue {
       [Prefix(::Splat)]
       pub pure op ** () -> TPairs;
   }
   IFlattenOperator<TLeaf: IAnyValue> : IAnyValue {
       [Prefix(::Splat)]
       pub pure op *.. () -> slice<TLeaf>;
   }
   ```
   `stdlib/core/syntax/expr_sugar.pilot` drops its three `[Prefix(::Splat, ..)]` attributes, so each operator is declared once, on its `op`. The readable rewrites `splat(x)`, `pairs(x)` and `flat(x)` stay.
2. **The one existing `op` whose default is wrong.** `op ? ()` (`ITryOperator` and its implementers) becomes `op _? ()`. No other declaration changes; `op - ()`, `op ~ ()` and `op >.. ()` are `prefix` by default.
3. **Checker** (`selfhost/check/grammar_attrs.pilot`, codes in `selfhost/diagnostics/errors.pilot`):
   - `GRAM005` gains: `_` alone as a symbol.
   - `GRAM008`: a pattern that is no fixity. That covers two holes side by side (`_ _`), a pattern with no symbol, and a list hole outside a matched pair.
   - `GRAM009`: the hole count disagrees with the parameter count, or the pattern's position disagrees with its grammar attribute (`[Prefix]` on `op _?`).
   - The program tables (`selfhost/check/program.pilot`) name members the way the seed does (§2).
   - Every lookup of a no-parameter `op` uses its canonical name: `check_unpack` (`selfhost/check/rules.pilot:897`), `selfhost/ir/lower_members.pilot:593,624`, `selfhost/ir/lower_spread.pilot:50-51`, and the existing lookups of `?`, `-`, `~` and `>..` (inventoried when built).
4. **Seed bindings.** `stdlib/core/builtins/native_calls.pilot:120-122` bind `"*_"`, `"**_"`, `"*.._"` instead of `"unpack"`, `"unpack_pairs"`, `"flat"`.
   - Those three are member names, not Rust natives (`crates/pilot-vm/src/stage_a_natives.rs` has no arm for them).
   - So they leave `transitional_native_bindings()` (`selfhost/gate/airlock_abi.pilot`) and `selfhost/gate/manifest.pilot:687`, which is a shrink.
   - If `GATE021` would reject the new strings as natives, these bindings move to a non-native form (a Pilot gate change, checked when built).
5. **Seams for the parts not built yet:**
   - `circumfix`, `matchfix` and any `mixfix` other than `_[_]` / `_[_]=_` are accepted and checked (`GRAM008`/`GRAM009`), but no use site parses: their parse shapes are an append to `PROP-008` §4 (a Rust change for a later entry). A program that reaches one stops at `seam("grammar.circumfix_shape", ..)`, `seam("grammar.matchfix_shape", ..)` or `seam("grammar.mixfix_shape", ..)`, registered in `seams.pilot`.
   - Dispatching a user `prefix`/`postfix` `op` other than the unpack three (`-a` on a struct), and lowering a struct's `infix` `op` on PilIR: their own entries.
6. **Records.** `docs/concept_matrix.md` (a fixity row), `docs/keyword_matrix.md` for the `splat`/`pairs`/`flat` rows, `pilot.md`, `wiring.pilot`, `seams.pilot`.

**Not in this proposal (deletions it makes possible, owned elsewhere):**
- `prefix_flag`'s three unpack arms and the `StarDotDot`/`DoubleStar` handling: `PROP-008` step 2.
- `UNARY_OP_SPLAT`/`UNARY_OP_DOUBLE_SPLAT`/`UNARY_OP_DEEP_FLATTEN` and the VM unpack code in `lower_expr.rs`, `lower_args.rs`, `lower_yield.rs` and `lower_members.rs`: `PROP-005`.

### 5. Acceptance (written before the code)

- `collections/pairs_map_struct` and `modules/k2_limits` rewritten to `op *`, `op **` and `op *..`, with the same `.stdout`, on all three engines.
- New `collections/prefix_op_beside_infix`: a struct with both `op * (rhs)` and `op * ()`, and both `op ** (rhs)` and `op ** ()`. The expected output is worked by hand in its comment: `a * b`, `[*a]` and `splat(a)` each give their own result.
- New `operators/hole_patterns`: the same type declared once with holes (`op _+_`, `op -_`) and once without (`op +`, `op - ()`) gives the same results.
- New `errors/err_gram008_hole_pattern` (`op _ _ (rhs)`) and `errors/err_gram009_hole_arity` (`op -_ (rhs: T)`), and a `GRAM005` case for `op _`.
- New `errors/err_unpack_infix_only`: a type with only `op * (rhs)` used as `*x` gives `TYPE001` at the `*`, not a call of multiply with no argument.
- `circumfix`, `matchfix` and other `mixfix`: one program each that stops at its registered seam (`SEAM001`).
- `cargo fmt --check`; `cargo test -p pilot-parser` and `cargo test -p pilot-parser --test damaged_source -- --include-ignored` (parser edit); `cargo test -p pilot-vm`; `cargo test -p pilot-cli`; `pilot test` / `--ir` / `--emit c`; `pilot gate --full`; `pilot run tools/keyword_matrix/main.pilot -- --check`.

### 6. Decisions for the owner

1. **The `matchfix` list hole.** Recommended: `_, ..` inside the pair (`op <|_, ..|>`), read as "one or more operands, comma separated". Alternatives: `_..` (shorter, but `..` touching `_` reads like a range), or a separate list mark.
2. **`circumfix` versus `matchfix`.** This proposal reads `circumfix` as one operand between two symbols (`|v|`) and `matchfix` as a comma list inside a matched pair. Some languages use the two words as synonyms. Confirm or redefine.
3. **Canonical names.** Recommended: `infix` and the two index forms keep their bare names, and every other fixity keeps its holes, so no existing lookup changes. The alternative is to keep holes in every name (`_+_`, `_[_]`). That changes every operator lookup in Rust (`crates/pilot-vm/src/lower_expr.rs:516-519` and the `INDEX_GET_OP`/`INDEX_SET_OP` users in `lower_members.rs`) and in Pilot, so it would add `crates/pilot-vm` files to §3.
4. **Default for no parameters and no holes.** Recommended: `prefix`, so `op - ()` and `op * ()` need no holes and only `?` changes. The alternative, requiring holes on every no-parameter `op`, adds `op -_`, `op ~_` and `op >.._` edits but makes every unary position explicit.

### 7. As built (2026-11-16)

- **Rust (only the two approved files):** `crates/pilot-parser/src/decl.rs` (`parse_op_decl` keeps a space between two words of a symbol; `is_word_char`, `hole_positions`, `strip_holes`, `op_member_name`) and `crates/pilot-parser/src/tables.rs` (an attribute on a holed `op` declares the symbol without its holes). **+61 / −7 lines (+54 net), against the `+20` estimated**: the hole scan and member-name rule took more lines than planned. `pilot gate` passes with it, so `paused_max_total_rust_loc()` still holds. Verified 2026-11-16 (capped): `pilot test` 576 passed + 12 known bugs reproduced, `--ir` 566, `--emit c` 558 (the 8 earlier failures only), `pilot gate` 4/4, `pilot gate --full` 5 passed + 1 at seams, keyword matrix current, `cargo fmt --check` clean, `damaged_source` and `pilot-vm` pass.
- **Pilot:** `stdlib/core/operator.pilot` (`[Prefix(::Splat)] op * ()`, `op ** ()`, `op *.. ()`; `[Postfix] op _? ()`), `result.pilot`, `option.pilot`, `syntax/try_catch.pilot` (`"_?"`), `syntax/expr_sugar.pilot` (the three `[Prefix(::Splat, ..)]` attributes deleted; the rules stay), `builtins/native_calls.pilot` (`unpack_op` binds `*_`, `**_`, `*.._`), `selfhost/check/infer.pilot` (`op_unwrap`, `op_splat`, `op_pairs`, `op_flatten`, `unpacked_element`), `check/rules.pilot` (`check_unpack` also checks structs; `check_prefix_call`), `check/grammar_attrs.pilot` (`Fixity`, `check_holes`: GRAM005 hole-only, GRAM008, GRAM009, the three shape seams), `check/snapshot.pilot` (`unary_neg`, `unary_bit_not`), `ir/lower_members.pilot`, `ir/lower_spread.pilot`, `diagnostics/errors.pilot` (GRAM008, GRAM009), `gate/ledger/seams.pilot`, and `gate/airlock_abi.pilot` + `gate/manifest.pilot` (`unpack`, `unpack_pairs`, `flat` were member names, not natives: deleted from both lists; check 4 skips a binding whose target is a symbol).
- **Found while building, fixed in Pilot:** `[*x]` typed its item as `x` itself, so `let xs: List<i64> = [*a]` was TYPE001 even for a list `a`; `check_unpack` only looked at `[Bits]` primitives, so `*s` on a struct with only `op * (rhs)` was never reported.
- **Owner direction 2026-11-16 (changes §2 and §6 decision 3):** "infix should be the default when no _ is present, prefix when no arguments are provided, and everything else needs _". So a holeless `op` is infix with one parameter and prefix with none; indexing has no holeless form any more. Every `op []` / `op []=` in the tree became `op _[_]` / `op _[_]=_` (stdlib/core/operator.pilot, mem.pilot, primitives/{list,slice,stride,grid,ptr,map,map_view,ring,str,array,simd}.pilot, stdlib/std/col.pilot, tests/conformance/types/index_operators, contracts/roles_clean), and GRAM011 reports a holeless `op` with two or more parameters or a structural symbol (`tests/conformance/operators/err_gram011`). Member names are unchanged (`_[_]` is still `[]`), so no Rust changes; the seed's holeless index rule in `op_member_name` is now unreachable from checked code, and is deleted when `decl.rs` is next touched under an approved proposal.
- **Amendment A1 (needs owner approval; not applied): three Rust test edits, `src` unchanged.** Each test pins a name or form PROP-018 changed:
  1. `crates/pilot-parser/tests/milestone51_homoiconic_syntax_and_contraction.rs:91` requires `stdlib/core/syntax/expr_sugar.pilot` to contain `[Prefix(`; the unpack operators' `[Prefix(::Splat)]` moved to `stdlib/core/operator.pilot` and the call-form rules that carried it were deleted (the sequence rules stay). Edit: `syntax_rule!("stdlib/core/syntax/expr_sugar.pilot", &["syntax", "quote"])`. Fails 4 tests until applied.
  2. `crates/pilot-parser/tests/native_calls.rs:41` expects the try protocol's unwrap member to be `"?"`; it is `"_?"` now. Edit: `("is_success", "_?", "failure")`. Fails 1 test.
  3. `crates/pilot-cli/tests/cli_f4.rs:291-292` (`a_role_gives_its_lattice`) declares `op []` / `op []=` in its fixture, which GRAM011 now reports. Edit: `op _[_]` / `op _[_]=_`. Fails 1 test.
- **Found, fixed by PROP-020 (2026-11-16): `parser.const_generic_struct` (known bug, `tests/conformance/known_bugs/const_generic_struct`).** The seed does not parse a `const` generic parameter (`N: const usize`). In `stdlib/core/primitives/array.pilot` and `grid.pilot` the members after the header were read as statements all along, and the old `op [] (` was parsed as an expression and dropped without a word; with `op _[_]` the bootstrap reports it, so `crates/pilot-parser/tests/bootstrap_sources.rs` fails until the parser reads `const` parameters. That fix is in `crates/pilot-parser` (struct header generics) and needs its own approved file list; it is not hidden by going back to `op []`.
- **Not built (seams):** a use of a circumfix, matchfix or non-index mixfix operator (`grammar.circumfix_shape`, `grammar.matchfix_shape`, `grammar.mixfix_shape`, reported at the declaration); `-x` / `~x` on a struct calling its prefix `op -_` / `op ~_`, which the seed never dispatched (it trapped with "cannot negate record"; now `grammar.prefix_op_call` at build time).

---

## `PROP-019` — Type-Position Operators Declared in Pilot; `op` Symbols Take Values Only

**Status: APPROVED BY OWNER 2026-11-18 ("approve"); queued as roadmap step B2. DRAFT 2026-11-16.** Owner direction 2026-11-16 ("lets go with your recommendation"), after asking whether operators can take things other than expressions. The recommendation:

1. An `op` is a member of a type, so every operand is a value and its holes (`_`, PROP-018) stay untyped. Anything taking a type, block, name or pattern is a `syntax` rule with typed holes (`$x:Type`, `$b:Block`, `stdlib/core/syntax/holes.pilot`). Pilot only; no approval needed (see §4).
2. Type-position operators (`syntax type_construct`) declare their levels the way expression operators do since PROP-008, so no binding power, associativity or operator token is left in Rust. Needs this proposal.

### 1. What is in Rust today (read 2026-11-16)

- `crates/pilot-parser/src/tables.rs:896-901` (`compile_type_rule`): the binding power and associativity of a type infix rule come from its token kind: `&` 20, `|` 15, `=>`/`->` 10 right-associative, `!` 5. Any other infix symbol is dropped (`return None`), so a new type operator cannot be added from Pilot.
- `crates/pilot-ast/src/lib.rs:328-335` (`TypePrecedence`: `Lowest 0`, `Fallible 5`, `FnArrow 10`, `Union 15`, `Intersection 20`, `PostfixOptional 30`) and its ordering assertion (`:909`).
- `crates/pilot-ast/src/lib.rs:299-326` `ExprPrecedence` is read only by that same test (`:913`): dead vocabulary, deleted with it (found by `docs/vocabulary_in_rust.md`).
- `crates/pilot-parser/src/stmt.rs:15` (only `*` and `@` can start a prefix type rule), `:18` and `:121` (`PostfixOptional`), `:167` (`31`); `crates/pilot-parser/src/expr.rs:300` (`30`).
- The levels are otherwise only written in the header comment of `stdlib/core/syntax/type_construct.pilot`.

### 2. Pilot (no approval needed; listed so the slice is whole)

- `stdlib/core/operators/levels.pilot`: `enum TypeLevel : u8 { Fallible, Arrow, Union, Intersect, Postfix }`, lowest binding first (the order is the precedence; append-only like `OpLevel`).
- `stdlib/core/syntax/type_construct.pilot`: each rule carries its shape and level, e.g. `[Infix(::Union, ::Left)] (T1 | T2) => ..`, `[Infix(::Arrow, ::Right)] (A -> R) => ..`, `[Prefix(::Postfix)] (*T) => ..`, `[Postfix(::Postfix)] (T?) => ..`. The level argument is read against `TypeLevel` on a `type_construct` rule and `OpLevel` elsewhere.
- `selfhost/check/grammar_attrs.pilot`: GRAM005-GRAM009 also check `type_construct` rules (a missing attribute is GRAM006's shape check failing loudly, never a silently dropped rule).
- `selfhost/check/grammar_attrs.pilot`: **GRAM010** (recommendation 1): an `op` symbol that marks a hole with a kind (`op $t:Type ?`, `op _:Block`) is an error that names the `syntax` rule form instead.
- Acceptance tests first: `tests/conformance/types/type_levels.pilot` (`A | B & C` is `union<A, intersect<B, C>>`; `T? | U`; `A -> B -> C` is right-associative; `T ! E | F` binds `|` first; worked by hand from the declared order), `types/err_type_rule_no_level.pilot`, `operators/err_gram010.pilot`.

### 3. Rust (needs approval). Strictly these files:

| File | Change |
| :--- | :--- |
| `crates/pilot-parser/src/tables.rs` | `compile_type_rule` takes the level ordinal and associativity from the rule's `grammar_entry` (the hook PROP-008 already feeds, routed to `type_rules` when its target is a `type_construct` rule) instead of matching token kinds; a second `register_grammar` call names `TypeLevel`, kept in its own `grammar_enums` slot. Power = the same `level_bp` formula. Delete the token match. |
| `crates/pilot-parser/src/stmt.rs` | A prefix type rule starts at any symbol the table holds (not only `*`/`@`); the postfix power comes from the table's `Postfix` level, not `TypePrecedence`/`31`. |
| `crates/pilot-parser/src/expr.rs` | `:300` reads the same postfix power instead of `30`. |
| `crates/pilot-ast/src/lib.rs` | Delete `TypePrecedence`, the dead `ExprPrecedence` and their assertion. |

No new hook: `register_grammar` and `grammar_entry` in `stdlib/core/bridge_lang.pilot` are reused, so the frozen airlock does not change. If the owner prefers a separate hook (`register_type_levels`), it is an append to `bridge_lang.pilot` and `tables.rs` `apply_hook`, and this proposal gains that line.

### 4. Estimate and risk

- Rust: the token match (`~8`), `TypePrecedence` (`~12` with the assertion) and the literals go; routing type entries adds `~10-20`. About `-25` to `0` net; reported as built.
- Risk: the Stage-A bootstrap reads `type_construct` before or after the grammar entries depending on load order (PROP-008's two-pass bootstrap in `lib.rs`); if the order is wrong, types parse with no levels. The acceptance test fails loudly in that case, and `lib.rs` would have to join §3, which needs a fresh approval rather than a quiet edit.
- Recommendation 1 (GRAM010) does not depend on this proposal and is built in Pilot (2026-11-16): `selfhost/check/grammar_attrs.pilot` `check_holes`, `selfhost/diagnostics/errors.pilot`, `tests/conformance/operators/err_gram010` (`op _:Type?`; `op $t:Type ?` reports the same).

### 5. Decision for the owner

1. Reuse `register_grammar`/`grammar_entry` (recommended, no airlock change) or add a `register_type_levels` hook (append-only, one more frozen item).

---

## `PROP-020` — Read `const` Generic Parameters Written After the Colon (`N: const usize`)

**Status: APPROVED BY OWNER 2026-11-16 ("Approve PROP-020") AND BUILT 2026-11-16: `generic_params.rs` +5 lines (a comment, the check and `rustfmt`'s line break); `types/const_generic_struct` passes and its known-bug entry is deleted; `bootstrap_sources` passes. Original draft below.** Found while building PROP-018 (known bug `parser.const_generic_struct`, `tests/conformance/known_bugs/const_generic_struct`).

### 1. The bug

`stdlib/core/primitives/array.pilot` (`array<T: IAnyValue, N: const usize>`) and `grid.pilot` (`grid<T, N: const usize, M: const usize>`) write the `const` mark after the parameter's colon. `crates/pilot-parser/src/generic_params.rs:25` only takes `const` *before* the name (`<const N = 4>`), so after `N:` the word `const` reaches `parse_type`:

- in a user file the header is SYN001 ("operator `>` is missing an operand");
- in the stdlib bootstrap the members after the header were parsed as statements all along. `op [] (` was read as an expression and dropped without a word; since PROP-018's `op _[_]` the bootstrap reports it, and `crates/pilot-parser/tests/bootstrap_sources.rs` fails.

### 2. Rust (needs approval). Strictly this file:

| File | Change |
| :--- | :--- |
| `crates/pilot-parser/src/generic_params.rs` | After `eat(TokenKind::Colon)`, skip a word whose modifier action is `ModAct::Const` (the same table lookup line 25 already does, no word written in Rust) before `parse_type`. About `+3` lines. |

Whether `const` marks a value parameter is not recorded today either way (the prefix form drops it too); recording it belongs to the checker's const-generic work, not to this fix.

### 3. Acceptance

- `tests/conformance/known_bugs/const_generic_struct` (prints `12`) moves to `tests/conformance/types/const_generic_struct` and passes on `pilot test`, `--ir`, `--emit c`; its `bug_registry()` entry is deleted.
- `cargo test -p pilot-parser --test bootstrap_sources` passes; `damaged_source` with `--include-ignored` passes.

---

## `PROP-021` — The Lexer Declared in Pilot: Token Patterns Compiled by Pilot Into a State Table the Seed Only Runs

**Status: DRAFT 2026-11-16, rewritten 2026-11-16 at the owner's direction ("yes, lets rewrite prop-021"); the Rust part (§3, refreshed 2026-11-16 as slices 21G Pilot and 21H Rust): slice 21H APPROVED BY OWNER 2026-11-17 ("PROP-021 21H"), strictly the files of §3.2, and BUILT 2026-11-17, see 3.2 as built; Amendment A1 (dead and broken Rust 21H left outside its files) approved 2026-11-17 ("Approve A1"); slice 21G BUILT 2026-11-16 at the owner's request ("Start 21G only"), see 3.1 as built. Slice 21A (Pilot only, no Rust) BUILT 2026-11-16 at the owner's request ("can we start of the first proposal of the lex proposals?"), see §7. Slice 21B (Pilot only) BUILT 2026-11-16 (owner: "lets focus on prop-021+"), see §8. Slice 21C (Pilot only) BUILT 2026-11-16 (owner: "Start 21C"), see §9. Slice 21D (literal values, Pilot only) BUILT 2026-11-16 (owner: "lets continue with the next part"), see §10. Slice 21E (the token tape, Pilot only) BUILT 2026-11-16 in the compact 6-byte shape (owner: "Zig-style (6 bytes)", "Start 21E"), see §11. Slice 21F (float values, correctly rounded, Pilot only) BUILT 2026-11-16 (owner: "do the next prop"), see §12.** Owner direction: "I would like pilot to have recognition over its own behavior" (about `1_000_000` meaning `1000000`), and "can you define what numbers are in pilot and then compile it to the lex file?". The first draft (per-literal tables the seed reads) is replaced. The lexer is no longer hand-written in either language. Pilot declares every token as a pattern, Pilot compiles the patterns into a state machine, and the seed only runs that machine.

### 1. What exists today (read 2026-11-16)

- **Two hand-written lexers.**
  - `crates/pilot-lexer` (`lexer.rs`, `simd.rs`, `keywords.rs`, `token.rs`; 1,120 lines) is the one that runs.
  - `selfhost/lexer.pilot` (346 lines) is a hand-written Pilot copy. It knows less: its numbers are plain digit runs. It feeds a self-hosted parser that stops at a seam.
- **Rules written more than once** (`docs/vocabulary_in_rust.md`):
  - the number grammar three times (`lexer.rs:300-367`, `num_suffix.rs:115-164`, `int_lit.rs:23-41`);
  - the f-string rules three times (`lexer.rs`, `simd.rs:101-217`, `fstring.rs`);
  - the seed words twice (`keywords.rs`, `lib.rs` `is_seed_kw`).
- **Silent acceptance and silent fallback.**
  - Separator placement is never checked: `1__000`, `1000_` and `0x_FF` are all accepted (run 2026-11-16).
  - `expr.rs:376` reads a float with `unwrap_or(0.0)`, so an unreadable float silently becomes `0.0`.

### 2. Design

**2.1 Token patterns are Pilot values.** `stdlib/core/lex/patterns.pilot` provides the pattern builders, evaluated at `comptime`:
- `lit(text)` and `class(chars)`;
- `range(lo, hi)`;
- `seq(..)`, `alt(..)`, `opt(p)`, `many(p)` (zero or more), `some_of(p)` (one or more);
- `not_followed_by(p)`, used sparingly, for rules like "`.` but not `..`";
- `repeat(p, min, max)`, a bounded count (`\x` and exactly two hex digits, `\u{..}` and one to six), which costs about one state per count;
- `byte(n)` and `byte_range(lo, hi)`, a byte named by its value, for the bytes the bootstrap subset cannot write in a string (`"` is `byte(34)`, `\` is `byte(92)`).

Patterns compose: a pattern may use any other pattern, so `float` reuses `digits` and a suffixed number is `seq(alt(integer, float), name_run())`. Composing patterns makes **one** token. Several tokens in a row (`100[m]`, `a..b`) are a `syntax` rule, never a pattern. Glue pieces into one token only when they must touch with no space and mean one thing together (`0xFF`, `1.5e3`, `200u8`).

They build a `Pattern` tree (`0b10` owned, `enum PatternKind : u8`). Every token family is a declaration with a `[Token(..)]` attribute naming its family and, where it has a value, the Pilot function that builds the value from the text. For example (the syntax is settled during building, under the constraints of 2.6):

```pilot
fn digits(base: Radix) -> Pattern =>
    seq(digit(base), many(seq(opt(lit("_")), digit(base))));

[Token(::Number, nameof(int_from_text))]
let integer = alt(seq(lit("0x"), digits(::Hex)),
                  seq(lit("0b"), digits(::Bin)),
                  seq(lit("0o"), digits(::Oct)),
                  digits(::Dec));

[Token(::Number, nameof(float_from_text))]
let float = seq(digits(::Dec), lit("."), digits(::Dec),
                opt(seq(class("eE"), opt(class("+-")), digits(::Dec))));
```

Placement rules need no separate check. `_` appears only between two digits because that is the only place the pattern allows it, so `1__000`, `1000_` and `0x_FF` are not numbers. A lone `_` then lexes as a name or the wildcard, as it does today.

**2.2 Every token comes from a declaration.**

| Family | Declared in |
| :--- | :--- |
| Numbers and the dimensional suffix | `stdlib/core/lex/numbers.pilot` |
| Strings: quotes, triple quotes, prefixes, escapes, `f"..{}.."` hole marks | `stdlib/core/lex/strings.pilot` |
| Comments, whitespace, newlines, shebang | `stdlib/core/lex/trivia.pilot` |
| Names (start and continue classes) | `stdlib/core/lex/names.pilot` |
| The 13 structural delimiters | `stdlib/core/lex/delimiters.pilot` |
| Operator symbols, call-quote forms, the dot rules | collected from every `op` declaration and operator attribute (PROP-008/018), the operator-token half of PROP-008 step 2 |
| Seed words and registered keywords | collected from `register_keyword` / `bind_receptor` |

**2.3 Pilot compiles the patterns into the lexer.** `selfhost/lex/build.pilot` takes every `[Token]` declaration, builds a nondeterministic automaton, makes it deterministic and minimizes it. It picks the longest match. When two families match exactly the same text, file order never decides: the declarations must say which wins (an explicit `priority:` on `[Token]`; keywords outrank names that way), and a tie with no stated priority is `LEX002`, naming both declarations. A pattern that refers to itself, directly or through other patterns, cannot become a state table and is `LEX003` (nesting is a mode, 2.4). The result is a `LexTable`:
- states × byte classes → next state;
- for each accepting state, its token family and the receptor or value function it names;
- the byte-class map that collapses 256 bytes into the few classes the patterns use.

`pilot inspect lex` prints it, with the pattern that produced each accepting state.

**2.4 A few context rules can't be a plain DFA**, so each is a small named mode in the table, not Rust logic:
- **f-string holes**: a brace depth stack, pushed and popped by `{`/`}` inside an `f"..."`;
- **nested block comments**;
- **infix-dot vs prefix-dot**: whether whitespace or which token kind came before;
- **the `:name` keyword literal**, only after whitespace.

The patterns declare the modes, and the seed keeps a mode stack. Each of these rules exists only because of a declaration.

**2.5 The seed only runs the table.** The Rust lexer becomes a byte loop:
1. look up the byte's class, then the next state;
2. at a dead end, emit the last accepting state's token;
3. push or pop the mode stack when a token says so.

That is about 80 lines of mechanism with no character, word or symbol in it. `scan_number_or_dimension`, `scan_string_literal`, `scan_operator_or_delimiter`, `keywords.rs`, the simd scanners and the per-kind `TokenKind` spellings are deleted.

The parser's re-derivations go too:
- `int_lit.rs`, `num_suffix.rs` `suffix_start` and `literal_value`, and `expr.rs:359-379` become calls of the declared value function, through the existing receptor path;
- the float fallback becomes an error (`LIT002`).

`selfhost/lexer.pilot` is deleted: the self-hosted toolchain runs the same table.

**2.6 Bootstrap.** Two constraints:
1. The pattern files are read by a lexer before their table exists. So `stdlib/core/lex/*.pilot` uses only what today's Rust lexer already reads the same way: plain decimal numbers, plain `"..."` strings with no escapes, and the 14 seed words plus registered words.
2. The first `LexTable` is built by today's Rust lexer, reading the pattern files, and written next to `toolchain.pilc`. From then on, each `LexTable` is built by the toolchain that the previous one lexed.

The gate rebuilds the table and compares it byte for byte (`PROP-023`). Until `PROP-023`'s `LANG` section exists, the table ships as its own file, `toolchain.lex`, in a format `selfhost/lex/build.pilot` writes and the seed reads by number.

**2.7 Errors.** Lexing errors are reported in Pilot terms: the longest valid prefix, plus the declaration whose pattern the text failed to finish.
- `LIT001`: "`1000_` is not a number: `_` must stand between two digits (`stdlib/core/lex/numbers.pilot` `digits`)".
- `LEX001`: no pattern matches at all, or a byte no pattern allows (a raw control byte inside a string), with the byte offset.
- `LEX002`: two families tie on the same text with no stated priority (reported when the table is built).
- `LEX003`: a pattern refers to itself (reported when the table is built).
- `LIT002`: the text is a well-formed token but its value does not fit the type it lands in (2.9).

**2.8 Bytes, not characters.** The table steps over raw bytes (0–255). The seed knows nothing about Unicode. Non-ASCII names are spelled in `names.pilot` as the valid UTF-8 byte sequences (`utf8_letter()`), so what counts as a letter is declared in Pilot. Each family declares which bytes may start it and continue it; any other byte ends it or is `LEX001`.

**2.9 The lexer decides shape; the type decides range.** A pattern says only that text is number-shaped. It never encodes "at most 255": the bytes `256` look like `255`, and `let x: u8 = 300` has no suffix for the lexer to see. The value function and the type the literal lands in (its suffix, its context, else the default) decide whether the value fits, using that type's `[Bits]` / `[Layout]` width, so "`u8` holds 0–255" is stated once, on `u8`. Length caps on names or strings, if any are wanted, are checks over the token's span, not patterns.
- **Integers:** a value outside the type's range is `LIT002`.
- **Floats: overflow is an error, rounding is not.** A value past the type's largest finite value is `LIT002`, never a silent `inf` (`1.0000000000034e50` written out in full does not fit `f32`, whose largest value is about `3.4e38`). A value in range becomes the nearest representable value, as every decimal float does (`0.1` is not exact either). An optional warning-level lint may report a literal with more significant digits than the type keeps.
- **Correctly rounded, in Pilot.** `float_from_text` converts decimal text to the nearest binary value itself (a fast path plus a big-decimal fallback, or a published algorithm such as Eisel–Lemire). It never hands the text to the host's `strtod` or Rust's `parse`, since then a host library would define what a Pilot literal means. Its expected results come from published conversion test vectors, not from running it.

**2.10 What is standard and what is chosen for ownership, not speed** (owner direction 2026-11-16: "lets make sure we add it to the prop and that we are aware of that distinction"). This proposal makes two choices that have to be kept apart:
- **Token as a span, value computed afterwards: the standard approach, and efficient.** rustc, clang, Go, Swift and Roslyn all do it. The lexer records a token's family and its start and end offsets, with no copy and no allocation. The value function reads those bytes once at compile time and builds the number directly. A float needs every digit before it can round, so it must work this way anyway. By the time the program runs, a literal is a constant, so none of this costs run time.
- **The lexer as a generated state table: correct and linear, but not the fastest way to lex.** This is the lex/flex approach. Most fast production compilers use hand-written lexers instead: rustc, clang, Go, V8 and Swift do, as does today's `crates/pilot-lexer` with its SIMD scanners. The fastest *generated* lexers (re2c) emit the DFA as straight-line code rather than a table, which avoids one memory lookup per byte. **The table is chosen because it is the simplest form in which Pilot's patterns are the only definition and the seed holds no vocabulary**: a hand-written lexer in either language writes the rules in its code again. Expect the table runner to lex more slowly than today's SIMD lexer. §5 measures and reports it. It is not gated, since compile speed is ungated (owner, 2026-11-10), and run time is unaffected.
- **Where the speed comes back: a direct-code emitter, later.** Because the patterns are the source of truth, `selfhost/lex/` can later emit the same minimized DFA as straight-line Pilot (and through it C) instead of a table, as re2c does. Nothing is hand-written again, and the gate's byte-identical check still applies to the generated code. It pays off once the toolchain itself runs native code (`--emit c` builds of the toolchain, or the native toolchain after `M46`); while the seed interprets the toolchain, a table is the better fit. **This is a follow-up step, not part of this proposal's build.** It needs its own approval when it is sequenced, and its acceptance is lexing the tree byte-identically to the table runner and faster than it.

### 3. The seed runs the table (refreshed 2026-11-16, owner: "PROP-021 §3: seed runs table"; the Rust part awaits approval)

**3.0 Where the two lexers stand (measured 2026-11-16).** A one-off comparison lexed 1,256 of the tree's 1,279 `.pilot` files with both lexers: `crates/pilot-lexer` (through a scratch binary outside `crates/`), and the Pilot table with the standard library's declared operators (on the seed, `pilot run`). The comparison ignores the Rust `Newline` tokens. It merges each f-string's pieces into one token, and the Rust lexer's `IS_DIMENSIONAL` number (`250ms`) into the two table tokens it covers.
- **1,164 files (93 %) give identical token spans.**
- **92 files differ.** At the first difference in each:
  - **`$` (49 files; 689 LEX001):** `syntax`-rule holes. No token declares `$`.
  - **`\"` inside f-string holes (18 files):** the owner's open question (§11.6). The table rejects the form and the Rust lexer accepts it.
  - **Missing operator tokens (about 20 files):** the compound assignments (`+=`, `-=`, `*=`, `/=`, `%=`, `**=`, `&=`, `|=`, `^=`, `<<=`, `>>=`, `??=`), `?.`, `?..`, `...`, `@`, and the call-quote opener `!@(`. The Rust lexer has them all; no Pilot declaration does.
  - **Intended differences (5 files):** error tests whose bytes neither lexer should accept the same way (`errors/err_stray_character`, `errors/err_string_unterminated`, `numbers/err_lit001_before_dot`, `tool_fixtures/lex/stray_byte`).
- This compares spans only. Mapping each family to the kind the parser reads is 3.1's job.

**3.1 Slice 21G (Pilot only, no approval needed): the table lexes what the seed lexes.**
- **Declare the missing tokens**, so the comparison above has no difference except the intended ones:
  - `$` in `stdlib/core/lex/delimiters.pilot`;
  - the compound assignments, `?.`, `?..`, `...` and `@` where operator tokens are declared (`stdlib/core/lex/operators.pilot` or the `syntax` rules' attributes);
  - the call-quote openers `!(`, `!!(`, `!&(`, `!^(`, `!@(`, `!?(`, as tokens that start only `Glued` (touching a callee) and end before the `(`. That is what the Rust lexer emits today.
  - **`\"` in holes waits on the owner's decision** (the open question in §11.6). Until then those 18 files are listed as an expected difference, not hidden.
- **Split number families by what the parser reads.** Today the parser picks the base from the kind (`IntLit`, `HexIntLit`, `BinIntLit`, `OctIntLit`). So `number_tokens()` declares one family per radix in `RADIXES`. It also declares a suffixed number (`seq(number, name_run)`), which is the `IS_DIMENSIONAL` token the parser reads (`250ms`, `200u8`); that gives each `250ms` one token again.
- **Give each family the kind the seed reads, declared in Pilot.** New file `stdlib/core/lex/seed_kinds.pilot`: a `const` table from family to the `TokenKind` number (`crates/pilot-lexer/src/token.rs`'s `#[repr(u16)]` values) and the flag bits (`IS_RAW_STRING`, `IS_INTERPOLATED`, `IS_DIMENSIONAL`). A family it does not list is LEX008 when the table is written. Rust then receives numbers only, never a Pilot name.
- **Write the table the seed reads.** A checked-in, generated file, `stdlib/core/lex/toolchain.lex`, in the cache's format (`pilot-lex-cache 2`, run-length lines) plus one line per family for 3.1's kind and flags.
  - `pilot lex --write-table` regenerates it from the declarations.
  - A gate rule (GATE024, `selfhost/gate/`) rebuilds it in memory and fails when the checked-in file differs, so the file can never drift from the declarations.
  - The f-string hole mode has the same tokens as the main mode. The writer shares one table between them instead of writing it twice (the main table is about 1,526 states × 149 classes).
- **Acceptance (21G):**
  - a Pilot tool, `tools/lex_compare/main.pilot`, runs the comparison of 3.0 in the tree. It compares the kinds and flags of 3.1 as well as the spans. It reports every file that differs; the only differences allowed are the listed intended ones;
  - conformance tests on three engines: the new tokens (`lex/compound_assign_tokens`, `lex/call_quote_tokens`), and `err_*` cases for a family with no seed kind (LEX008) and a stale table (GATE024).

**3.1 as built (2026-11-16, Pilot only).** Where it differs from the plan above, and why:
- **The missing spellings live in one shrink-only list, `stdlib/core/lex/seed_forms.pilot`** (a `[StandIn("lex.token_attribute", ..)]`), not in `delimiters.pilot` or as operator/`syntax` declarations. Declaring them as operators would hand them a meaning in the bootstrap tables, which is PROP-022's job. A spelling leaves the list when a declaration declares it.
- **The call-quote openers start in any context, not only `Glued`.** The Rust lexer emits `!(` whether or not it touches a callee (`if !(t)` is a call quote to it), so the table does too. Each opener is matched only before a `(` that it leaves for the next token: fixed trailing context (`trailed_by` in `patterns.pilot`, `trails` in each `TokenAction`, cache format 3).
- **Strings are split per quoting as well.** The seed reads `"…"` and `"""…"""` as different kinds (`StrLit`, `TripleStrLit`), so each of string, raw string, byte string and the four f-string openers is its own family (`string`, `string_triple`, `fstring_open_raw`, …).
- **Numbers:** `number_tokens()` stays the bare numbers, because the checker's LIT001 table reads them. `suffixed_number_tokens()` adds the `*_suffixed` families and, so that a seed word is never a suffix (`1if`), each number before a seed word, per word length, at a higher priority.
- **An f-string's pieces** are listed in `seed_kinds.pilot` as `Inner`: the seed reads the whole f-string as the opening's token. A space that is not a newline is `Skip`.
- **The table file** is the cache format (now 4) with a short key, plus `seed_special` and `seed_kinds` lines. A mode whose automaton is an earlier mode's is written `shares <mode>`, which is how the hole mode shares the main table: 364 families, 90 kB.
- **GATE024 runs in a `lex_table` gate with `--full` only.** Building the lexer from nothing takes about two minutes on the seed; it runs in about one second with `target/cache/lex`.
- **`tools/lex_compare` was dropped.** No CLI hands the Rust lexer's tokens to a Pilot program. The comparison was rerun with scratch tools (a Rust dumper outside `crates/`, and a Pilot dumper over the checked-in `toolchain.lex`), now comparing kinds and flags as well as spans:
  - 1,257 of 1,288 files are identical;
  - 15 differ by `\"` in f-string holes. **Decided 2026-11-17 (owner: "Reject it"):** a hole is code, so a quote in it is just a quote (`f"{m["k"]}"`); `\"` in a hole is an error with a fix-it, and the 15 files are rewritten with their expected output unchanged;
  - 3 differ by a `#` format spec in a hole (`{n:#x}`): in the hole mode `#` starts a directive line. This is new and needs a decision on how a hole's format spec is lexed;
  - 1 is `known_bugs/not_prefix_dot`, where the table is right and the Rust lexer is wrong;
  - 12 are error tests, including `0x_FF`, which the table reads as `0` with the suffix `x_FF` and the Rust lexer as one hex number. Either way it is LIT001.
- **Tests:**
  - `lex/seed_forms` (compound assignments and call quotes);
  - `lex/suffixed_numbers`;
  - `lex/seed_table`;
  - `lex/err_lex008_no_seed_kind`.

  On `--emit c` the seed form `??=` first came out as the C trigraph for `#`. The emitted C now writes each `?` that follows a `?` as `\?` (`without_trigraphs` in `selfhost/transpile/c_family.pilot`), tested by `strings/trigraph_text`.

  GATE024's failure path was checked by hand (`--check-table` against an edited file), not by a conformance program, because that would mean editing the checked-in table.

**3.2 Slice 21H (Rust; APPROVED BY OWNER 2026-11-17, "PROP-021 21H"). Strictly these files:**

| File | Change |
| :--- | :--- |
| `crates/pilot-lexer/src/lexer.rs` | Replace the hand-written scanners with the table runner. The runner reads `toolchain.lex` (format above) and keeps a mode stack and the context (start, spaced, glued, other). It steps byte class → state, takes the longest match, and emits the family's `TokenKind` and flags from 3.1. A newline space token is emitted as `Newline` and other spaces are skipped, as today. An f-string's pieces become one `StrLit` token (with its flags), so `fstring.rs` is unchanged in this slice. A byte no token starts is one `ErrorToken`, as today. About 180 lines. |
| `crates/pilot-lexer/src/simd.rs`, `crates/pilot-lexer/src/keywords.rs` | Delete. |
| `crates/pilot-lexer/src/token.rs` | Add a checked `TokenKind::from_u16`. An unknown number is a load error naming the family, never a guess. |
| `crates/pilot-lexer/src/lib.rs` | `tokenize` runs the loaded table. Add `load_table(path)`. The three unit tests that list seed words and symbols move to Pilot conformance tests (`lex/seed_words`, `lex/call_quote_tokens`, `lex/dimensional_numbers`) and are deleted here. |
| `crates/pilot-parser/src/lib.rs` | `set_source_root` also loads `stdlib/core/lex/toolchain.lex`. A missing or unreadable table stops with an error that names the file and `pilot lex --write-table`. There is no fallback lexer. |
| `crates/pilot-parser/src/expr.rs` | A float the table accepted but `parse` cannot read is `LIT002` instead of `unwrap_or(0.0)`. |

- **Not in 21H, and why:**
  - `int_lit.rs`, `num_suffix.rs`: the seed still turns literal text into values in Rust, because the Rust `BytecodeCompiler` needs constants before any Pilot runs. They leave with PROP-005.
  - `fstring.rs`'s hole scanner (about 100 lines): the parser reads f-string pieces in a later slice.
  - `TokenKind` itself, until PROP-022.
  - `run.rs` and the `bridge_lang.pilot` hook in the old list are no longer needed: the parser loads the table from the source root, as it already loads the bootstrap tables. So no frozen file changes.
- **Bootstrap:** the seed lexes the toolchain's own sources with the checked-in table. Changing a declaration means running `pilot lex --write-table` with the old table first, as a compiler builds itself with the previous release. GATE024 catches a table that was not regenerated.
- **Estimate:** `crates/pilot-lexer` goes from 1,116 lines to about 470: `token.rs` (245, plus `from_u16`), the runner (about 180) and `lib.rs` (about 40). With the `expr.rs` change, net about −640 Rust lines. That is less than the old estimate of −900 to −1,000, because the number and f-string handling stays until PROP-005 and the f-string slice. The ceiling in `selfhost/gate/manifest.pilot` ratchets down by the measured amount.
- **Acceptance (21H):**
  - every conformance program passes unchanged on `pilot test`, `--ir` and `--emit c`;
  - `cargo test -p pilot-parser --test damaged_source -- --include-ignored` passes: the runner must not panic on any input;
  - `cargo test -p pilot-lexer` and `cargo fmt --check` pass;
  - the 3.1 comparison, rerun against the runner, is identical for every file;
  - non-ASCII names (`lex/utf8_names`) now work on the seed;
  - the lexing rate of the runner on the tree is reported against today's SIMD lexer. It is a report, not a gate (compile speed is not gated, 2026-11-10).

### 4. Pilot (no approval needed; listed so the slice is whole)

- `stdlib/core/lex/{patterns,numbers,strings,trivia,names,delimiters}.pilot`.
- `selfhost/lex/build.pilot` (pattern → NFA → DFA → minimized table) and `selfhost/lex/table_writer.pilot`.
- `stdlib/core/lex/literal_value.pilot` (`int_from_text`, `float_from_text`, string unescape), used by the checker and the self-hosted IR. `float_from_text` is correctly rounded (2.9) and is the largest single piece of Pilot work in this proposal; it may land as its own slice.
- `LIT001`, `LIT002`, `LEX001`, `LEX002`, `LEX003` in `selfhost/diagnostics/errors.pilot`.
- `pilot inspect lex`.
- `selfhost/lexer.pilot` deleted, with its row in `pilot.md`.

### 5. Acceptance (written before the code)

- `tests/conformance/numbers/digit_separators.pilot`: `1_000_000 == 1000000`, `0xFFFF_FFFF`, `10_00_000`, `1_0.5_0`, `2.5e1_0`, with values worked by hand in comments.
- `numbers/err_digit_separator_{double,trailing,after_prefix,before_dot,before_suffix}.pilot` (`LIT001`), `numbers/err_float_unreadable.pilot`, `numbers/err_int_out_of_range.pilot` (`300` into `u8`), `numbers/err_float_overflow_f32.pilot` (`LIT002`), `lex/err_no_token.pilot` and `lex/err_control_byte_in_string.pilot` (`LEX001`), `lex/err_tie_without_priority.pilot` (`LEX002`), `lex/err_self_referential_pattern.pilot` (`LEX003`).
- `numbers/float_rounding.pilot`: long and borderline decimal literals (halfway cases, values just under the largest finite value, subnormals) printed as their exact bits, with expected bits taken from published conversion test vectors.
- `lex/utf8_names.pilot`: non-ASCII names lex as one name; a malformed UTF-8 sequence is `LEX001`. The `0x_FF` in `text/string_literal_forms` becomes `0xFF`.
- Every existing conformance program passes unchanged on `pilot test`, `--ir` and `--emit c`. Lexing is byte-identical for every file in the tree (compared by a one-off dump: the old lexer's tokens against the table's), apart from the `LIT001` cases.
- `cargo test -p pilot-parser --test damaged_source -- --include-ignored` passes; the table runner must not panic on any input.
- **Litmus** (`pilot gate --litmus`), Pilot edits only, with no Rust change. Each builds and runs on all three engines:
  - change the separator in `numbers.pilot` to `'` (`1'000'000`);
  - add a `0t` ternary prefix;
  - respell the line comment.
- **Performance:** the lexing rate of the table runner on the tree is measured against today's lexer and reported. The owner has said compile speed is not gated (2026-11-10), so this is a report, not a gate. The report states the ratio against today's hand-written SIMD lexer, so the cost of the ownership choice in 2.10 is on record when the direct-code emitter is sequenced.

### 7. Slice 21A, as built 2026-11-16 (Pilot only)

- **Built:**
  - `stdlib/core/lex/patterns.pilot` (191 lines): `Pattern` as a postfix program of byte-range ops. Builders: `byte`, `byte_range`, `range`, `lit`, `class`, `seq`, `alt`, `opt`, `many`, `some_of`, `repeat`, plus `token` / `token_over` (with a priority).
  - `stdlib/core/lex/numbers.pilot` (64 lines): the number tokens, with `_` only between two digits.
  - `stdlib/pilot/lex/build.pilot` (566 lines): byte classes → Thompson NFA → DFA → Moore minimization → merged byte classes, `LEX002` ties, `LEX004` empty match, and `lex_all`, a table runner in Pilot.
- **Tests** (`tests/conformance/lex/`): `dfa_minimal` (state and class counts worked by hand), `digit_separators`, `token_priority`, `err_lex002_tie` and `err_lex004_empty`. All pass on `pilot test`, `--ir` and `--emit c`.
- **Changed from §2 and §4:**
  - **The builder lives in Ring 3** (`pilot.lex.build`, `stdlib/pilot/lex/`), not `selfhost/lex/`, so programs, tests and tools can import it. That is Ring 3's job (AGENTS.md §3, compiler as a library).
  - **A pattern is a postfix program**, not a tree. That makes self-reference impossible within a pattern. `LEX003` is left to `[Token]` collection, where declarations refer to each other by name.
  - **New code `LEX004`:** a token that matches the empty text.
  - **Patterns use Pilot's own operators** (owner, 2026-11-16: "is it possible to get as close to pilot syntax use as possible for the lexical definitions?"). `a + b` is sequence, `a | b` is alternation, and `.opt()`, `.many()`, `.some()` and `.repeat(min, max)` are methods. That became possible with `PROP-026`. `seq([..])` and `alt([..])` remain for lists. `p * n` repeats exactly `n` times. `"0" ..= "9"` and `hex[1 ..= 6]` wait on `PROP-027`.
  - **`not_followed_by` is not built yet.** It lands with the modes slice (comments, dots), which needs it.
- **Stand-in:** `number_tokens()` lists the declarations by hand in place of `[Token]` collection (`lex.token_attribute` in `seams.pilot`). The wiring row stays `DRAFT` until that is replaced.
- **Not yet:**
  - The seed runs no table: `crates/pilot-lexer` is unchanged, and that is the §3 Rust part, awaiting approval.
  - Strings, comments, names, delimiters and operator symbols are not declared yet.
  - No modes.
  - No value functions (`int_from_text`, `float_from_text`).
  - `LIT001`, `LIT002`, `LEX001` (the runner marks an unmatched byte as family -1, but no caller reports it yet) and `LEX003`.
  - `pilot inspect lex`: `pilot inspect` itself stops at the seam `cli.command_handler`.
- **Found while building:** `m.get(k) ?? x` stops on PilIR and the emitted C (the seed is right). `known_bugs/` runs on the seed only, so it cannot hold a PilIR-only bug. The repro is `tests/conformance/collections/map_get_coalesce`, which fails on `--ir` and `--emit c` until it is fixed. The builder uses `has` and indexing instead.
- **Duplicates to retire when the seed runs the table:** besides `crates/pilot-lexer` and `selfhost/lexer.pilot`, there are hand-written tokenizers in `stdlib/pilot/lex.pilot` (`token_cursor_next`), `selfhost/fmt/fmt.pilot` (`fmt_tokenize`) and `selfhost/cli/repl.pilot` (`repl_tokenize`). All of them should run the one table. *(2026-11-16: `fmt_tokenize` is gone; `pilot fmt` reads the token tape. `repl_tokenize` was deleted with `selfhost/cli/repl.pilot`, a calculator that returned `0` for unknown names and for division by zero, reachable from no command. `pilot repl` stops at `cli.command_handler` until a real REPL runs lines through the pipeline.)*

### 8. Slice 21B, as built 2026-11-16 (Pilot only)

**Scope:** every token family that needs neither a mode nor declaration collection, declared to read exactly as today's seed lexer does.

- **Built** (`stdlib/core/lex/`):
  - `words.pilot`: the 14 seed words and the 13 structural tokens, declared once. It uses nothing else, so the checker links it alone (`cli_checker_files()` in `selfhost/cli/host.pilot`). The private copies in `selfhost/check/grammar_attrs.pilot` (GRAM005) are deleted; it imports this file.
  - `names.pilot`: `name_start`, `name_pattern` (ASCII letters, `_`, digits after the first byte) and `name_tokens()`. Each seed word is a token over the name pattern with priority 1.
  - `trivia.pilot`: spaces (space, tab, CR), newline, `//` and `#` lines (not including the newline), and `/* .. */` ending at the first `*/` (no nesting, as today).
  - `strings.pilot`: quoted strings (`"` and `'`, `\` takes the next byte, no newline), triple-quoted strings (end at the first three quotes), and the `r` and `b` prefixes as families `raw_string` and `byte_string`.
  - `delimiters.pilot`: one token per structural token.
  - `lexicon.pilot`: `core_tokens()`, all 38 families declared so far. They build into one table with no `LEX002`.
  - `patterns.pilot` gains `any_byte()`, `any_but([..])` (the complement of a byte set) and `byte_of(text)`.
- **Tests** (`tests/conformance/lex/`): `names_and_words`, `trivia`, `strings`, `delimiters` and `lexicon`, with expected output worked by hand. All 11 lex programs pass on `pilot test`, `--ir` and `--emit c`.
- **Found while building:** a struct's operator on a local bound without a type annotation runs the machine operator on the seed (PilIR and C are right). It is filed as `known_bugs/unannotated_local_operator` (`seed.unannotated_local_operator`, owner `PROP-005`). `trivia.pilot` and `strings.pilot` annotate those locals and are marked `[Workaround]`.
- **Where the table differs from today's lexer, on purpose:**
  - A string missing its closing quote, or a block comment missing `*/`, matches no token. Each of its bytes then starts no token, where the seed lexer instead ran to the end of the line or file. That will be `LEX001` once a caller reports it.
- **Not yet:**
  - **Operators, the dots, `:name`, f-strings, `#!` and UTF-8 names.** Operators are to be collected from the `op` declarations; the others need modes. That is slice 21C.
  - **No byte-identical comparison with the seed lexer yet.** Dumping the seed's tokens needs Rust, so it comes with §3.
  - **`[Token]` collection.** Every `*_tokens()` function is the stand-in `lex.token_attribute`.

### 9. Slice 21C, as built 2026-11-16 (Pilot only)

**Scope:** contexts, modes, the operators the program declares, and `pilot lex` (owner: "Start 21C").

- **Contexts.** A token may be limited to the context it starts in (`TokenWhen`: `Always`, `Spaced` (after a space or at the start), `Glued` (touching the end of an operand), `AtStart`). The builder gives the table one start state per context (`LexContextKind`, an `enum : u8`). Each start reaches only the declarations allowed there, so `LEX002` fires only for tokens that can meet. A token's `TokenRole` (`Plain`, `Operand`, `Space`, `Trivia`) sets the next context. As in today's lexer, the text starts spaced, and comments, `#` lines and the shebang leave the context as it was.
  - `.` is two families: `infix_dot` (`Glued`, priority 1) and `prefix_dot`. Names, numbers, strings, `)`, `]`, `}` and declared postfix operators (`_?`) end an operand.
  - `keyword_literal` (`:name`, `:ns/name`) starts only after a space or at the start, so `x:y` stays a `:`.
  - `shebang` (`#!..`) is `AtStart`; anywhere else `#!` is a `#` line.
- **Modes** (`stdlib/pilot/lex/modes.pilot`): a lexicon is a list of `LexMode`s, each built into its own table. A token that `enters` a mode pushes it, and one that `leaves` pops it. An `enters` naming no declared mode is `LEX005`.
- **F-strings:**
  - The 8 openers (`f`/`F`, `rf`/`fr` in either case, `"` or `'`, single or triple) each enter their own mode. That mode holds text, `{{`, an escape unless raw, and in the triple modes one or two quotes as text.
  - A `{` enters `fstring_hole`, which lexes the main mode's tokens; there `{` nests and `}` leaves.
  - **This differs from the seed on purpose:** the seed reads an f-string as one token and rescans its holes. Here the hole's expression is real tokens.
- **Operators come from the program.** `declared_operator_symbols` and `declared_postfix_symbols` (`selfhost/check/grammar_attrs.pilot`) read every `op` pattern and grammar attribute. `stdlib/core/lex/operators.pilot` makes one token per symbol. Nothing is listed by hand.
- **Lexicon:** `core_tokens()` has 41 families (it was 38; the new ones are `keyword_literal`, `shebang` and the split dot). `core_tokens_with(operators)` adds operators. `core_modes(operators)` gives the main mode, the 8 f-string modes and the hole mode.
- **Tables are typed:** each declaration's effect on the lexer is one `TokenAction` (role, mode to enter, whether it leaves), not parallel `i64`/`str`/`bool` lists.
- **Data is constant and tokens are lazy (imp_type.md Rules 13 and 14):** the named bytes (`TAB`, `NEWLINE`, `CARRIAGE_RETURN`, `DOUBLE_QUOTE`, `SINGLE_QUOTE`, `BACKSLASH`), `SEED_WORDS`, `STRUCTURAL_TOKENS`, `DOT`, `HOLE_MODE`, `LEX_CONTEXT_KINDS` and `LEX_CONTEXT_START` are `const`s, not functions returning a literal. The checker, which links `words.pilot` alone, reads them (the GRAM005 tests pass). `lex_all` and `lex_modal` are generators (`Generator<Lexeme>`), so a token is matched only when the caller asks for it; `pilot lex` links `stdlib/core/generator.pilot` for that. The pattern and `*_tokens()` functions stay functions: they build values with operators, and they are the stand-in for `[Token]` collection, which replaces them.
- **`pilot lex <file>`** (`selfhost/lex/lex_tool.pilot`, `cmd_lex`):
  - It prints `family:text` per token, leaving out tokens whose role is `Space`.
  - A byte that starts no token is `LEX001`, with exit 1. No file, or a file that does not exist, is a usage error with exit 2.
  - It finds no `LEX001` on `selfhost/cli/commands.pilot` (3,134 tokens), `stdlib/pilot/lex/build.pilot` (4,193) or `stdlib/core/operators/levels.pilot`.
- **Tests** (`tests/conformance/lex/`):
  - New: `dots`, `keyword_literal`, `shebang`, `fstrings`, `err_lex005_undeclared_mode` and `cli_lex`, worked by hand.
  - Updated: `delimiters` (the dot split) and `lexicon` (41).
  - All 17 lex programs pass on `pilot test`, `--ir` and `--emit c`.
- **Found while building:**
  - **Seed tool linking.** Inside `pilot lex`, a call to the public `is_space(ref ModalLexer, Lexeme)` from `pilot.lex.modes` ran the private one-argument `is_space(b: u8)` of `selfhost/check/attr_text.pilot`, with no error. That ignores both visibility and arity. A plain program resolves a similar clash (a local function against a private one of an imported package) correctly, so the bug is in how `cli_program_tool` links a tool's files. The lexer function is named `is_space_lexeme`. No repro is filed yet: it needs a tool whose files clash, and none of the fixed tool file lists can be pointed at a test.
- **Needs approval (Rust test, one file): `crates/pilot-cli/tests/cli_f3.rs`.** `every_listed_command_works_or_says_it_is_not_implemented` keeps its own list of the commands that have a handler (`working`). `lex` now has one, so the test runs `pilot lex` with no file and expects the "not implemented" seam, but gets the usage error (exit 2). The change is to add `("lex", &["lex", "tests/tool_fixtures/lex/sample.pilot"])` to `working`, and nothing else. Until it is approved, that test fails. Because cargo stops at the first failing test binary, it also hides whether `cli_f4` runs.
- **Rename litmus (GATE008) caught an f-string hole.** The litmus renamer treats an f-string as one string, so a renamed name inside a hole (`f"{src.slice(..)}"`) is left as it was and the renamed toolchain fails. `modal_lexeme_text` slices outside the f-string, as `lexeme_text` already did. The renamer will read holes as code once it uses the Pilot lexer. `tests/tool_fixtures/lex` is added to the litmus sandbox (`rl_sandbox_trees`).
- **The C test runner mixed up two tests with the same file name.** `pilot test --emit` wrote every emitted program to `target/test-emit/<lang>/<file name>`. So `lex/fstrings` and `text/fstrings` overwrote each other in a full run, and `lex/fstrings` failed under `--emit c` only when the whole corpus ran. `modules/start_order` and `modules/lib/start_order` had the same latent clash. `selfhost/cli/test_emit.pilot` now names each emitted file after the test's whole path (`emitted_name`, `/` written as `__`).
- **Spellings the seed lexes as one token that no Pilot declaration provides (20).** `pilot lex` splits each into its pieces:
  - `$`, `@`, `@>`, `<@`, `?.`, `?..`, `??=`, `...`, `<-` (used only in `stdlib/asyncio/select_syntax.pilot`);
  - the compound assignments `+=` `-=` `*=` `**=` `/=` `%=` `&=` `|=` `^=` `<<=` `>>=`.
  - Their meaning belongs to `PROP-022`. They are to be declared there, never listed in the lexer by hand.
- **Not yet:**
  - The seed still runs `crates/pilot-lexer` (§3).
  - No byte-identical comparison with the seed lexer.
  - `[Token]` collection (stand-in `lex.token_attribute`), UTF-8 names, value functions, and `LIT001`/`LIT002`/`LEX003`.
- **Recorded for speed, not built:**
  - `next`, `class_of` and `accept` as `u16`/`u8` buffers instead of `List<i64>`.
  - A 12-byte `[Layout(c_abi)] Lexeme`.
  - Numbered token handles in place of family-name strings.
  - The lexer runs at compile time, which is ungated, so these wait for the parser that consumes the stream (`PROP-005`) or for `PROP-021.direct_code`.

### 10. Slice 21D, as built 2026-11-16 (Pilot only)

**Scope:** what an integer literal is worth, and `LIT001`/`LIT002` (owner: "lets continue with the next part", choosing "Literal values").

- **The numbers are data** (`stdlib/core/lex/numbers.pilot`). Each item is a `const`, not a function:
  - `RADIXES` (radix, prefix, base);
  - `DIGIT_RANGES` (byte range and value);
  - `DIGIT_SEPARATOR`.
  - The digit patterns and `integer()` are built from those consts, and the DFA is unchanged. Respelling the separator or adding a radix is a one-line edit, and the value code reads the same consts.
- **Values** (`stdlib/core/lex/literal_value.pilot`):
  - `int_from_text` turns an integer token into an `IntLiteral`: a magnitude of any size, in 16-bit limbs.
  - Size is never decided there. `fits(bits, signed, negated)` decides it against the landing type's `[Bits]` width and register class.
  - `range_text` writes a type's range in decimal, for the message.
- **`LIT001`** (`selfhost/check/literals.pilot`, `check_number_text`): the literal's text is matched against the number table (`Infer.numbers`). The match must take the whole text, or stop where only a scalar type suffix follows. Otherwise it reports the number it read, what follows it, and the rule that `_` stands only between two digits. This covers `1__000`, `1000_`, `0x_FF`, `1_.5` and `1_u8`.
- **`LIT002`:**
  - **What is checked:** an integer literal's value, negated under unary `-`, against its destination type.
  - **Where it is checked:**
    - in `assignable`, with every `TYPE001` assignable failure going through `report_unfit` (call arguments, binding, return, parameter default, closure, delegation);
    - in an unannotated `let`/`var`/`const`, against the literal slot type;
    - in `check_assign`.
  - **Owner decision 2026-11-16: a hex, binary or octal literal is a value, not a bit pattern** ("Reject: LIT002"), as in Rust, Go and C#. `let s: i64 = 0xFFFFFFFFFFFFFFFF;` is `LIT002`, and the bit pattern is written as a cast (`top() as i64` in `numbers/u64_values`, changed accordingly).
- **Tests:**
  - `lex/int_values`.
  - `numbers/digit_separator_values`.
  - `numbers/int_range_edges`.
  - `numbers/err_lit001_{double,trailing,after_prefix,before_dot,before_suffix}`.
  - `numbers/err_lit002_{let,negative_unsigned,signed,hex_signed,arg,return,assign,default}`.
  - Every expected value and range is worked by hand in the test's comments.
  - `text/string_literal_forms` wrote `0x_FF`, which is `LIT001` now. It reads `0xFF`, the same value (255), as §5 said it would.
  - The numbers (36) and lex (18) directories pass on `pilot test`, `--ir` and `--emit c`.
- **Found while building:**
  - **Seed tool linking, second time.** The checker now links the lexer files, so the private `joined`, `op` and `listed` in `patterns.pilot` took over same-named functions in `selfhost/transpile/source.pilot`, `selfhost/check/asm.pilot` and `selfhost/check/methods.pilot` inside the C emitter (`--emit c` stopped with "`map_get` list index must be an integer"). It is the same bug as §9's `is_space`: `check::run_on_program` (`crates/pilot-cli`) links a tool's files by bare name, ignoring visibility and arity. The private helpers are renamed (`pat_op`, `joined_pattern`, `byte_listed`, `wrapped_pattern`). The fix is in Rust and needs a proposal. A plain program resolves the same clash correctly (`modules/private_fn_clash`), so no conformance program can reproduce it.
  - **Rename litmus, f-string hole again.** The `LIT002` message read a local named `range` inside an f-string hole. The renamer leaves holes alone, so in the renamed toolchain the name was undefined and every program stopped (1,012 `GATE008` violations). The local is now `values`. This keeps happening until the renamer reads holes as code (§9).
  - **`::Variant` with two enums sharing a name** (`seed.variant_compare_untyped`): `asm.pilot`'s `::Operand` broke once the lexer's `TokenRole::Operand` was linked into the checker. The variants are qualified there, tagged `[Workaround]`.
- **Not yet:**
  - **String unescape:** not built. Corrected 2026-11-16 during 21E: core does have a constructor from bytes to `str` (`from_utf8`, bound in `stdlib/core/builtins/native_calls.pilot`), so nothing blocks it. The seed silently keeps an unknown escape (`\q`), which is an open decision.
  - **Float values:** slice 21F, correctly rounded. Built, see §12.
  - **`LIT002` gaps:**
    - field assignment (`obj.f = 300`);
    - comparisons;
    - an `opt` destination (`u8?`), which still reports `TYPE001`.
    - A suffixed literal (`300u8`) is still range-checked by Rust until §3.
  - **Cost:** building the number table in `infer_new` adds a table build to every check. It is not measured; §11 makes the table a constant.

### 11. Slice 21E, planned 2026-11-16: the token tape (Pilot only)

**Status: BUILT 2026-11-16 (§11.6); shape decided 2026-11-16 (§11.2).** Owner request 2026-11-16: "lets plan for this. having rust as the guideline will help with implementation for building the same thing in pilot", after asking whether the lexer stores its important facts the way `crates/pilot-lexer` does. Correctly rounded floats, planned earlier as 21E, move to **21F**. This slice changes no Rust: `crates/pilot-lexer` is the guide, not something this slice edits.

#### 11.1 The Rust guide, read 2026-11-16

| What | `crates/pilot-lexer` | Pilot today (`stdlib/pilot/lex/build.pilot`) |
| :--- | :--- | :--- |
| Token kind | `#[repr(u16)] enum TokenKind`: 2 bytes | `Lexeme.decl: i64`, an index into `LexTable.names`: 8 bytes |
| Position | `ByteSpan { start: u32, end: u32 }`: 8 bytes, with a `size_of == 8` assertion | `start: i64`, `end: i64`, plus `mode: i64`: 24 bytes |
| Per-token flags | `TokenFlags(u8)`: 6 bits | none |
| Matching brackets | `bracket_links: Vec<u32>`, filled while lexing (`emit`, `lexer.rs:486-520`) | none |
| Storage | `TokenTape`: one `Vec` per field, reserved at `source.len() / 4` tokens | a `yield` stream of whole `Lexeme`s; nothing kept |
| Byte classes | `const [bool; 256]` tables (`simd.rs`) | `class_of: List<i64>`, rebuilt by `build_table` on every run, including in the checker since 21D |

What a Rust token costs: 2 + 1 + 8 + 4 = **15 bytes**. A Pilot `Lexeme` costs 32.

**What the Rust parser reads (grep 2026-11-16):**
- `matching_bracket`: about 15 call sites, in `expr.rs`, `decl.rs`, `rule_arms.rs`, `comptime_if.rs`, `inline_block.rs`, `decl_syntax.rs`, `ret_type.rs` and `loud.rs`.
- `IS_RAW_STRING` and `IS_INTERPOLATED`: read in `fstring.rs`.
- `IS_DIMENSIONAL`: read in `expr.rs:366`.
- `PRECEDED_BY_WS`, `PRECEDED_BY_NEWLINE` and `FOLLOWED_BY_LPAREN`: set by the lexer, **never read**.
- Mismatched brackets: the Rust lexer leaves them unlinked, with no error.

So the guide is followed where the Rust parser depends on it, and not copied where it isn't.

#### 11.2 Decided: the compact shape, 6 bytes a token

**Owner decision 2026-11-16: "Zig-style (6 bytes)".** This is the shape of the Zig compiler's tokenizer, which keeps a tag and a start offset per token and re-runs the tokenizer when it needs a token's text. Rust's 15-byte tape (§11.1) stays the guide for *what* the parser must be able to ask; the compact tape answers the same questions by recomputing instead of storing.

| Question | Rust stores | The compact tape |
| :--- | :--- | :--- |
| Which family? | `kinds` (2 bytes) | `kinds: List<u16>` (2 bytes) |
| Where does it start? | `spans[i].start` | `starts: List<u32>` (4 bytes) |
| Where does it end? | `spans[i].end` | **recomputed:** `end(i)` runs the token's own mode table from `starts[i]` (below) |
| Raw, interpolated? | flag bits | the family's `FamilyInfo`, once per family in the constant table |
| Space or newline before? | flag bits (never read) | **recomputed:** the bytes between `end(i - 1)` and `starts[i]` |
| Glued to the next token? | `FOLLOWED_BY_LPAREN` (never read) | **recomputed:** `end(i) == starts[i + 1]` |
| Matching bracket? | `bracket_links` | **not on the tape:** the parser keeps its own stack, as Zig's does; `bracket_links(ref tape, src)` builds the list on demand for a caller that wants random access (Rule 14) |

- **Why the end can be recomputed exactly.** A token is the longest match from its start, among the families allowed in its mode and context. If its own family's pattern matched further, that longer match would have won. So the token's end is its own family's longest match from `starts[i]`. `end(i)` therefore needs only the family's mode, never the neighbours. Each family gets one id across all modes (`kinds` indexes the flattened list of mode and declaration), so the kind names the mode. A one-pattern table per family is not needed: running the mode's table and taking the last accepting state for this family gives the same answer.
- **The tape** (`pkg pilot.lex`, replacing `stdlib/pilot/lex.pilot`):
  - `TokenTape : IObject { kinds: List<u16>, starts: List<u32> }`, reserved at `source.len() / 4` tokens, as Rust reserves.
  - `len`, `kind`, `start`, `end(src)`, `span(src) -> ByteSpan`, `text(src)`, `newline_before(src)`, `space_before(src)` and `glued(src)`. The `[Layout(c_abi)] ByteSpan { start: u32, end: u32 }` is kept as the value `span` returns.
  - Trivia (comments, `#` lines, the shebang) stays on the tape, so `pilot.cst`, `fmt` and `--fix` stay lossless. Runs of spaces are not tokens.
- **Brackets are still checked while the tape is built.** `delimiters.pilot` gains `const BRACKET_PAIRS`. The builder keeps a stack, which it drops when it is done. A mismatched or unclosed bracket is `LEX006` (Rust leaves it unlinked without a word). The f-string hole's braces count as a pair.
- **Width limits fail loudly:** a source longer than `u32` holds, or more than 65,535 families, is `LEX007`. Rust casts without checking.
- **Tables become constants** (Rule 13): `next`, `class_of`, `accept` and `families` are computed once at `comptime` into `u16`/`u8` lists, and the checker (`Infer.numbers`) and `pilot lex` read them. If the seed's `comptime` cannot build a `LexTable`, that goes in `known_bugs` with a repro, and the run-time build stays as `[StandIn("lex.table_comptime", ...)]`.
- **Producing:** `lex_tape(ref lexer: ModalLexer, src: str) -> TokenTape ! LexError` drains `lex_modal`, which stays the stream for tools that read tokens once.
- **What it costs:** asking for a token's end, text or neighbours re-reads that token's bytes through the table. The parser reads each token's text about once, so the extra work is about one more pass over the bytes it looks at, and in exchange it moves 6 bytes a token instead of 15. The speed report (§11.4) measures both sides.

#### 11.3 Probes, run 2026-11-16

1. **`List<u16>` and `List<u32>` are not compact on any engine.** The values keep their type (`65535` and `4294967295` round-trip), but the emitted C stores every element as a boxed dynamic value (`pilot_list_push(.., pilot_v_int(..))`), and the seed VM does the same. Core has no compact typed buffer to use instead: `SliceBuffer<T>` (`stdlib/core/mem.pilot`) cannot grow (seam `mem.slice_buffer_grow`), and its `~self()` frees nothing. So the 6 bytes of §11.2 need a buffer that does not exist yet.
2. **A `comptime` `const` can hold the lexer table.** `const T: LexTable = comptime { build_table(core_tokens()).table };` gives 159 states. A scalar `comptime` is folded into the output (`144`), but a structured value runs once when the program starts. That is the registered stand-in `ir.comptime_at_start`, so a constant table is built once per run, not once per check. Rows that rely on it list that stand-in in `blocked_by`.
3. **No test can read a struct's size.** There is no `size_of`, and `pilot inspect` stops at the seam `cli.command_handler`. `lex/tape_layout` is dropped from §11.4 until one of them exists.

#### 11.4 Acceptance (written before the code; expected values worked by hand)

- `lex/tape_basic`: for a short source, each token's family, start and recomputed end, worked out by hand from the byte offsets.
- `lex/tape_ends`: tokens whose end depends on the longest match (`1_000.5e3`, `..=`, `>>=` against `>>`, a triple-quoted string, an f-string's text inside a hole's mode), each end worked by hand.
- `lex/tape_neighbours`: `newline_before`, `space_before` and `glued` on hand-chosen tokens.
- `lex/tape_brackets`: `bracket_links` on nested `( [ { } ] )` and an f-string hole, both ways.
- `lex/err_lex006_mismatched` (`( ]`) and `lex/err_lex006_unclosed` (`{` at the end of the file).
- **Whole-tree check:** for every `.pilot` file in the tree, every recomputed `end(i)` equals the end the yield stream gave. This is the evidence that the recomputation is exact, run on all three engines.
- `pilot lex` prints from the tape, and its existing tests pass unchanged.
- All lex and numbers programs pass on `pilot test`, `--ir` and `--emit c`, and `pilot gate --full` passes, rename litmus included.
- **Speed report (not a gate):** bytes per token, the time to build the tape, and the time to read every token's text, on the seed VM and in emitted C, against the yield stream.

#### 11.5 Files

- **Replaced:** `stdlib/pilot/lex.pilot`. Its hand-written `token_cursor_next`, `TokenKind`, `Token` and the `token_kind_*()` constant functions are deleted; `ByteSpan` is kept. Its `pilot.md` row is rewritten, and wiring row `M10` ("pilot.lex tokenizer") is re-pointed at the tape.
- **Changed:**
  - `stdlib/pilot/lex/build.pilot`: `FamilyInfo`, the `u16`/`u8` table fields, and the end-of-one-token run.
  - `stdlib/pilot/lex/modes.pilot`: one family id across modes, and `lex_tape`.
  - `stdlib/core/lex/delimiters.pilot`: `BRACKET_PAIRS`.
  - `selfhost/lex/lex_tool.pilot`.
  - `selfhost/check/infer.pilot`: the constant table.
  - `selfhost/diagnostics/errors.pilot`: `LEX006`, `LEX007`.
- **Not in this slice:** `fmt_tokenize` (`selfhost/fmt/fmt.pilot`) and `repl_tokenize` (`selfhost/cli/repl.pilot`) switch to the tape in a later slice (§8 lists them). *(Done 2026-11-16, see §8.)* The parser that consumes the tape is `PROP-005`.

#### 11.6 As built 2026-11-16 (Pilot only; owner: "Start 21E")

- **Compact storage.** Probe 1 showed that a `List` holds every element as one engine value. The owner chose a new core type ("Build PackedBuffer<T>"):
  - `stdlib/core/packed.pilot`: `PackedBuffer<T> : IObject`, integers side by side in one `RawMem` block at their own width (1, 2, 4 or 8 bytes);
  - it grows by doubling from 8, frees its block in `~self()`, and has `len`, `byte_len`, `reserve`, `push`, `op _[_]` and `op _[_]=_`;
  - an index out of range is `PACK001`, a width other than 1, 2, 4 or 8 is `PACK002` (both stop the program through `fault`);
  - the caller passes the width (`packed_buffer<u16>(sizeof(u16), false)`), because the seed cannot lower `sizeof(T)` in a generic body (SEED001). That is stand-in `mem.packed_width`.
- **The tape** (`stdlib/pilot/lex.pilot`, `pkg pilot.lex`; the hand-written cursor, `Token`, `TokenKind` and the `token_kind_*()` constant functions are deleted, `ByteSpan` kept):
  - `TokenTape { kinds: PackedBuffer<u16>, starts: PackedBuffer<u32> }`. A kind is `family * 4 + context`, where the family is numbered across every mode (`ModalLexer.bases`, `mode_of`, `family_of`, `decl_of`, `family_name` in `stdlib/pilot/lex/modes.pilot`).
  - `end(i)` re-runs `match_at` in the token's mode and start context: the same call that found the token, so it is exact by construction.
  - `span`, `text`, `newline_before` and `touches_next` are recomputed.
  - `bracket_links` builds partners on demand into a `PackedBuffer<u32>` and answers `partner(i) -> i64?`.
  - `build_tape` reports LEX001 (a byte that starts no token), LEX006 (unbalanced brackets) and LEX007 (a source past a `u32` offset). `build_modes` reports LEX007 when families times contexts pass 65,536.
  - The bracket pairs are data: `BRACKET_PAIRS` in `stdlib/core/lex/lexicon.pilot`, with `HOLE_OPEN` (`stdlib/core/lex/strings.pilot`) pairing an f-string hole with its `}`.
- **The checker's number table is a constant:** `NUMBER_TABLE` in `selfhost/check/literals.pilot` is a `comptime` `const`; the `Infer.numbers` field is gone. Probe 2: a structured `comptime` value is built once when the program starts (stand-in `ir.comptime_at_start`), so once per check.
- **`pilot lex`** prints from the tape. Its output and `cli_lex` are unchanged.
- **Found while building, and fixed in Pilot:**
  - **The keyword registry shadowed `sizeof` and `__engine`.** `stdlib/core/keywords.pilot` declares an empty struct per keyword (`sizeof {}`, `__engine {}`). `span.pilot` glob-imports it, and `patterns.pilot` imports `span.pilot`, so in a program reaching the lexer:
    - `sizeof(u16)` was checked as building a `sizeof` (TYPE001);
    - PilIR lowered a library's `__engine(..)` as building an `__engine` (seam `ir.lower_construct`).
  - **The fixes:**
    - `call_type` (`selfhost/check/infer.pilot`) does not read a bound type question as a construction (`prog_is_type_query`, `selfhost/check/program.pilot`);
    - `lower_call` (`selfhost/ir/lower.pilot`) never reads the seed token `__engine` as a type.
    - Test: `modules/keyword_registry_glob`.
    - Left open: why a glob import of a library lets its names win over intrinsics. Name resolution is in the Rust snapshot.
  - **Two PilIR limits met on the way, both loud seams:** a generic `v as T` cast, and `let _ = ..` inside `~self()`, stop at `ir.lower_construct`. `PackedBuffer` avoids both.
- **Tests** (expected values worked by hand):
  - `collections/packed_buffer`, `errors/err_pack001_index`, `errors/err_pack002_width`;
  - `lex/tape_basic`, `lex/tape_ends`, `lex/tape_neighbours`, `lex/tape_brackets`, `lex/tape_matches_stream`, `lex/err_lex006_brackets`;
  - `modules/keyword_registry_glob`.
  - All pass on `pilot test`, `--ir` and `--emit c`.
- **Whole-tree check and speed report** (a scratch program built with `pilot build --release`, so emitted C, over all 1,232 `.pilot` files):
  - **Ends:** 880,253 tokens, and every recomputed end, start and family equals the yield stream's.
  - **Size:** the tapes take 5,281,518 bytes, 6 a token. A `Lexeme` is 32, and `crates/pilot-lexer` keeps 15.
  - **Time:** lexing every file with the stream took 0.79 s, building every tape 1.01 s, and building every tape and reading every token's text 1.42 s. Reading every text, which recomputes every end, costs about half of one more lexing pass, as §11.2 expected.
  - **Seed and PilIR:** `tape_matches_stream` runs two smaller files on those engines, because PilIR took 303 s on the large ones (seed 5.4 s, C 5.6 s).
- **The tree does not all lex yet (1,053 errors, none from the tape):**
  - **941 LEX001,** mostly:
    - `$` in `syntax` rules (one of the 20 seed spellings with no declaration, §9, PROP-022);
    - `\"` inside f-string holes (`f"{m[\"a\"]}"`). The seed accepts that form, but AGENTS.md says holes take unescaped quotes, so whether the escaped form stays valid was an open decision for the owner; decided 2026-11-17 ("Reject it"), see 3.1 as built.
  - **112 LEX006,** from the same two causes and from format specs. A `#` in a hole's format spec (`{n:#_###}`) starts a `#` line that swallows the hole's `}`. That is a 21C gap, filed as known bug `lex.fstring_format_spec` (`tests/conformance/known_bugs/fstring_format_spec_lex.pilot`, true output `true`).
- **Not yet:**
  - the lexer tables (`next`, `class_of`, `accept`) are still `List<i64>`, not `u16`/`u8` buffers;
  - LEX007 has no test (it needs a 4 GiB source or 16,384 families);
  - `--live-allocations` does not count `RawMem` blocks, so there is no leak test for `PackedBuffer`. The C runtime (`runtime/c/pilot_rt_bridge.h`) would need to count them;
  - `fmt_tokenize` and `repl_tokenize` still have their own tokenizers.
- **Built lexers are cached (2026-11-16, owner: "Yes, A1 then A3").** `stdlib/pilot/lex/cache.pilot`: `build_modes_cached` / `build_table_cached` keep a built lexer in `target/cache/lex`, keyed by the declarations and the hash of the builder's sources, the whole key stored in the file. The slow lex cases and `pilot lex` use it. Test: `lex/lex_cache` on all three engines.

### 12. Slice 21F, as built 2026-11-16 (Pilot only): float values, correctly rounded

**Scope:** what a float literal is worth, LIT002 for floats (§2.9), and the same rounding for text conversion (owner: "do the next prop").

- **One routine, two callers** (`stdlib/core/convert/decimal_round.pilot`, `Pilot.Core.Convert.DecimalRound`):
  - `round_decimal(digits, exp10, fmt) -> u64?` gives the encoding of the nearest value of an IEEE format to `digits * 10^exp10`, ties to even, with subnormals.
  - It returns none past the largest finite value.
  - **Method:** the plain big-integer algorithm (Go's and glibc's fallback).
    - The value is an exact fraction, and its binary order fixes the quantum.
    - The quotient (at most p bits) is found bit by bit.
    - The remainder decides the rounding.
  - Bounds from `10^k >= 2^(3k)` stop absurd exponents before any big power is built.
  - `read_decimal_digits` keeps every digit, and `DecimalDigits.encoding(fmt)` rounds it.
  - Float literals use it through `float_from_text` (`stdlib/core/lex/literal_value.pilot`).
  - `str.to_f64()` / `to_f32()` use it outside their exact fast path (`stdlib/core/convert/convert.pilot`). That **closes the seam `convert.float_parse_slow_path`**, and an f32 is now rounded once, straight to binary32.
  - No host `strtod` or Rust `parse` is involved.
- **Magnitudes** (`stdlib/core/convert/limbs.pilot`): the 16-bit limb arithmetic that 21D kept privately in `literal_value.pilot` moved there, with shift, compare, subtract and powers of ten added. `literal_value.pilot` now uses it, so there is one copy.
- **Data, not repeated spellings:**
  - The decimal notation (`DECIMAL_POINT`, `EXPONENT_MARKS`, `EXPONENT_SIGNS`) is declared once, as consts in `decimal_round.pilot`.
  - The number patterns (`numbers.pilot` `float()`, `exponent()`) and the reader both use them.
  - The formats are the existing `FloatFormat`s of `stdlib/core/float_bits.pilot`. `format_of_width(bits)` picks one by a type's `[Bits]` width, so the checker names no type.
- **LIT002** (`selfhost/check/literals.pilot` `float_range_problem`):
  - A float literal landing in a float type is refused only when it rounds past the largest finite value. The message gives that value exactly as `(2 - 2^-f) * 2^bias`.
  - With no type around it, a literal lands in its own literal slot's type (`default_literal_type`): float slot for floats, integer slot for integers.
  - The checker's tool link list (`selfhost/cli/host.pilot`) gained `float_bits.pilot`, `limbs.pilot` and `decimal_round.pilot`.
- **Tests** (expected values worked by hand in each test's comments, and checked against Python's correctly rounded `float()` with `struct`, plus an exact rational rounding for binary32):
  - `lex/float_values`: 0.1, 1e23, ties to even at 2^53 and 2^24, the largest double/float and the halfway points past them, the least normal, the largest subnormal (`2.2250738585072011e-308`), half the least subnormal on both sides, 1e±400, separators, and four texts that are not floats.
  - `numbers/err_lit002_float_f32` and `numbers/err_lit002_float_f64` (under a minus, with no type).
  - `numbers/convert_float_parse_slow`, which replaces `convert_float_parse_seam`: a tie, 36 digits, 30 digits with an exponent, Overflow, and an f32 rounded once.
  - All pass on `pilot test`, `--ir` and `--emit c`.
- **Found and fixed on the way:** §9's "rename litmus, f-string hole" problem is gone. Since 2026-11-16, `selfhost/gate/rename_litmus.pilot` renames inside f-string holes, which are code.
- **Not yet:**
  - The seed still reads float literals itself (`expr.rs`, `unwrap_or(0.0)`) until §3. A literal's value at run time is the seed's, which is correctly rounded but not computed by this routine.
  - `f16`/`f128`: there is no `FloatFormat` for them, so `format_of_width` has none and their literals are not checked.
  - The optional lint for digits beyond the type's precision.
  - String unescape. Built, see §12.2.
  - LIT002 gaps of §10: field assignment, comparisons, `opt` destinations. Closed, see §12.1.
- **Modern alternative, recorded:**
  - Eisel–Lemire (used by Rust's `dec2flt`, Go 1.16+ and fast_float) answers almost every input with one 128-bit multiply and a table of powers of five. It falls back to big integers only in rare ambiguous cases.
  - It is many times faster, but needs 128-bit multiplication and a ~650-entry table.
  - It costs only compile time here (ungated) and, for `to_f64()`, run time only outside the fast path. It is worth adding when `to_f64()` on long text shows up in a benchmark.

#### 12.1 The LIT002 gaps of §10, closed 2026-11-16 (owner: "continue with the next item or prop")

- **Field assignment** (`p.a = 300`): `check_assign` checked only plain names. It used to pass and store 300 mod 256 = 44.
- **Struct literal fields** (`P { a: 300 }`): field values were not type-checked at all. `P { a: "x" }` passed too.
  - Now `check_struct_literal_fields` (`selfhost/check/rules.pilot`) checks each value against the field's declared type, over the literal's generic arguments.
  - A wrong type is TYPE001; a number outside the range is LIT002.
- **Comparisons** (`x == 300`, `256 > x`, `y < 1e39`): `check_compared_literal` (`selfhost/check/literals.pilot`) requires the literal, on either side, to fit the other side's type. A comparison with a value no such type holds is decided before it runs.
- **Optional and result destinations** (`u8? = 300`): the literal lands in the type that the destination's `op ?` gives. This was TYPE001 "found `i64`", which named the wrong problem.
- **Unknown field reads** (the known bug `check.field_read_unknown`, found by the rename litmus): `s.b` on a struct without `b` is now METH001. The check is `check_member_read`.
  - It used to pass, and the seed read the field as none.
  - `[Lang]` primitives are skipped, because the engine answers some of their members. Open generic types are skipped too.
  - The repro became `errors/err_meth001_field_read`.
- **Tests:** `numbers/err_lit002_{field,struct_field,compare,opt}`, `errors/err_struct_field_type` and the passing edge program `numbers/literal_fit_places`. Expected values are worked by hand. All fail before the change and pass after, on `pilot test`, `--ir` and `--emit c`.

#### 12.2 String values and LIT003, built 2026-11-16 (owner: "Reject it")

- **Decision 2026-11-16:** an escape not in the set is an error, as in Rust, Go and C#. The seed used to keep `\q` as its two bytes.
- **The escapes are data:** `ESCAPES` in `stdlib/core/lex/literal_value.pilot`.
  - It holds exactly the seed's set (`crates/pilot-parser` `unescape_str`): `\n`, `\t`, `\r`, `\\`, `\"` and `\'`.
  - Each entry is the byte after the backslash and the byte it stands for.
  - The prefix letters (`RAW_MARKS`, `BYTE_MARKS`, `FSTRING_MARKS`) became consts in `stdlib/core/lex/strings.pilot`, which its patterns now use.
  - A stray doc comment with no item under it there was deleted.
- **Values:** `string_from_text(text) -> str ! BadEscape` gives a plain, byte or triple-quoted string's value, or its first unknown escape.
  - A raw string keeps every byte.
  - An f-string has no single value, so asking for one faults.
  - Nothing here writes an escape in a string literal, so the lex files stay readable by today's seed lexer (§2, rule 1). Bytes come from `from_utf8`.
- **LIT003** (`selfhost/check/literals.pilot` `check_string_escapes`): every string token in the program's own files is read from the source.
  - That covers plain, byte and triple-quoted strings, and an f-string's text outside its holes.
  - `string_escape_problems` lists each unknown escape, and the message lists the declared set.
  - A hole is code, so a string inside one is checked as a token of its own.
- **Tests** (expected bytes worked by hand from ASCII):
  - `lex/string_values` (values, raw, triple, byte, unknown escapes, f-string text and holes) and `errors/err_lit003_escape` pass on `pilot test`, `--ir` and `--emit c`.
  - The whole corpus still passes on the seed (642), so no test program relied on a kept unknown escape.
- **Not yet:**
  - The seed still builds string values itself (`unescape_str`) until §3. A program LIT003 does not stop gets the same value either way, because the escape sets are equal.
  - `\"` inside f-string holes (`f"{m[\"a\"]}"`): rejected by the owner 2026-11-17 ("Reject it"), see 3.1 as built.

#### 12.3 UTF-8 names, built 2026-11-16 (owner: "goto next unimplemented"; letters: "Unicode XID")

- **The letters are Unicode's (UAX #31):** a name starts with XID_Start or `_` and goes on with XID_Continue.
  - **The data:** `stdlib/core/lex/unicode_xid.pilot` holds the non-ASCII ranges as `const` data: 665 XID_Start ranges, and 376 that are XID_Continue but not XID_Start.
  - **Where it comes from:** `tools/unicode_xid/main.pilot` generates the file from Unicode 15.1 `DerivedCoreProperties.txt` (`/usr/share/unicode/` here).
  - **Checked against an outside reference:** the generated ranges equal the file's. Against Python 3.12's tables (Unicode 15.0), the only differences are 15.1 additions: CJK Extension I U+2EBF0..U+2EE5D in XID_Start, and U+200C, U+200D, U+30FB, U+FF65 in XID_Continue.
- **Bytes, as §2.8 says.** `stdlib/core/lex/utf8.pilot` splits each range into UTF-8 byte-range sequences (the RE2 / regex-syntax construction) and joins them as a trie, grouped by byte range position by position. The name table is 453 states and 98 byte classes.
- **One report per character.** A character that starts no token is one unmatched lexeme over its whole UTF-8 sequence (`utf8_char_length`, `build.pilot` `match_at`), so it is one LEX001 rather than one per byte.
  - A Pilot `str` is always well-formed (`from_utf8` refuses anything else), so the malformed case can only come from a file. What `File.read_text` does with malformed bytes is not checked yet.
- **The builder had to get faster.** As first written, the XID name table took 116 s to build even as emitted C, and timed out after 300 s on PilIR. Four changes in `stdlib/pilot/lex/build.pilot`, standard ones, bring the C build to about 4 s and the seed build to 6.5 s; the ASCII-only tables come out the same:
  - the trie in place of one flat alternation of about 1,800 sequences;
  - `closure` sorts the states it reached instead of scanning every NFA state;
  - each transition is added once to the byte intervals its range spans, instead of every interval testing every state;
  - state keys, minimization signatures and class columns are joined from integer lists by halves, instead of strings grown one piece at a time.
- **Seed bug found and fixed:** a `const` list past about 120 items did not compile on the seed (PROP-030, approved and built).
- **Tests:**
  - **`lex/utf8_names`:** classes from Unicode's data, byte offsets from the UTF-8 lengths.
  - **`collections/const_list_long`.**
  - **The `lex/` folder:** passes on the seed (29 with the new test; 3 min 41 s cold, every table rebuilt).
- **Still open:**
  - the seed's Rust lexer still reads ASCII names only, so a program cannot use a non-ASCII name until the seed runs the table (§3);
  - NFC normalization of names;
  - a lint for confusable names;
  - the malformed-file case above.

### 6. Decided

- **`//=` does not exist** (owner 2026-11-16: "leave it meaning nothing unless it was supposed to mean something but that would conflict with the comment"). It was floor-division assignment, from a `//` floor-division operator that can never be written, since `//` starts a comment. The comment pattern in `trivia.pilot` has no exception: `//` and everything after it on the line is a comment. Fixed in today's lexer by `PROP-024`.
---

## `PROP-022` — Operator Meaning Declared in Pilot: Every Operator Use Lowers to Its Member Call or Its Rule's Template

**Status: DRAFT 2026-11-16, design for owner approval; the exact file list is fixed when it is sequenced (after `PROP-008` step 2 and `PROP-005` phase 5D).** **2026-11-16, owner: "do prop-022": slice 22A (§5) itemized for approval; the rest stays sequenced as in §4.** Owner question 2026-11-16 about `op *`, `op :=:`, `op _?` and `op []=`: "I want pilot to understand itself completely without help outside".

### 1. What Rust decides today (read 2026-11-16)

| Operator | Pilot declares | Rust supplies the meaning |
| :--- | :--- | :--- |
| `*x`, `**x`, `*..x` | `[Prefix(::Splat)] op * ()` (PROP-018) | `prefix_flag` (`crates/pilot-parser/src/expr.rs:1003`) maps tokens to unary flags 6/7/8; engines call `*_` only through the `unpack_op` binding. |
| `a :=: b` and the other binary operators | `[Infix(..)] op :=: (rhs)` | `map_binop_sym` / `binop_code` (`crates/pilot-parser/src/tables.rs:646-700`, 42 symbols) map text to `BinaryOp` (49 variants, `crates/pilot-ast/src/lib.rs:339`); engines run the variant natively, not the `op` body. |
| `x?` | `[Postfix] op _? ()` and `bind_try_protocol` | `expr.rs:151` builds `TryPropagateExpr`; each engine implements the early return. |
| `a[i]`, `a[i] = v` | `op _[_]`, `op _[_]=_` (PROP-018 owner direction) | parsed at `TokenKind::LBracket` (`expr.rs:118`); `INDEX_GET_OP`/`INDEX_SET_OP` (`crates/pilot-parser/src/accessors.rs:15-16`). |

### 2. The rule

An operator use means exactly one of:

1. **its member call**: `a + b` is `a.(+)(b)`, `-a` is `a.(-_)()`, `a[i] = v` is `a.(_[_]=_)(i, v)`. A primitive's `op` body is its machine op (`asm(MachineOp::Add, ..)` / `__ssa`), so `i64 + i64` is no longer special;
2. **its rule's template**, when it is more than a call: `x?` is the `try` rule over the declared protocol, `a && b` short-circuits through its `syntax` rule, `a :=: b` swaps through a template over two `ref` operands;
3. a **core form** (`CoreForm`) or **machine op** (`MachineOp`), the only fixed lists.

The checker resolves which (`selfhost/check/`), `pilot inspect` shows it, and the engines only lower calls, templates, core forms and machine ops. This also closes the `grammar.prefix_op_call` seam (user prefix operators) and the `operators.rewrite_template` seam (PROP-008).

### 3. What goes from Rust (fixed when sequenced)

- `crates/pilot-parser`: `map_binop_sym`, `binop_code`, `prefix_flag`, the `TryPropagateExpr` special case, `INDEX_GET_OP`/`INDEX_SET_OP` and the native index parse, after PROP-008 step 2 makes operator tokens table-driven.
- `crates/pilot-ast`: `BinaryOp` and the unary flag constants, once no parser or engine reads them.
- **Pilot, in the same change:** the `prefix_op` table in `stdlib/core/builtins/native_calls.pilot` (stand-in `parse.prefix_flag_table`, added by slice 22A), with `PREFIX_OP_TABLE`/`prefix_member_name` in `selfhost/check/infer.pilot` and the seed's `prefix_member` lookup. A prefix use then carries its operator, and the member is that operator's `op` declaration. Until then the table only maps the Rust parser's unary flags to member names.
- `crates/pilot-vm`: nothing here; its operator lowering goes with the Rust `BytecodeCompiler` in PROP-005.

### 4. Order and acceptance

- Sequenced after PROP-008 step 2 (operator tokens from tables) and PROP-005 phase 5D (bytecode from PilIR), so the change lands in Pilot passes, not in Rust lowering that is about to be deleted.
- Acceptance: every `tests/conformance/operators/*` program on three engines; a user struct's `-x` runs its `op -_` (`err_prefix_op_call_seam` becomes a passing `prefix_op_call` test); a litmus that renames an operator symbol in `stdlib/core/operator.pilot` and its uses with no Rust change.

### 5. Slice 22A — a struct's prefix operator runs its member (`-m` is `m.(-_)()`), closing seam `grammar.prefix_op_call`

**Status: APPROVED 2026-11-16 (owner: "Approve 22A"); BUILT 2026-11-16.**

**As built:**
- **Rust:** `crates/pilot-vm/src/lower_expr.rs` grew net `+20`, not `+12`. Besides `compile_unary` and the shared `prefix_member` helper, `static_type` gained a `UnaryExpr` arm (the member's declared return type). Without it, `-(-m)` stopped with `cannot negate record`. It is the same kind of arm PROP-026 added for binary operators, in the approved file.
- **Pilot:**
  - The `prefix_op` table is in `native_calls.pilot`.
  - `selfhost/check/infer.pilot` has `prefix_member`; the type of `-g` is the member's return type.
  - `check_prefix_call` gives `METH001` for a struct with no member.
  - PilIR uses `lower_prefix_member`.
  - The seam `grammar.prefix_op_call` is retired.
- **Runs:** `operators/{prefix_op_call,err_prefix_op_missing,hole_patterns}` pass on the seed. `operators/` and `collections/` pass on `--ir` and `--emit c` (44 each; `collections/map_declared_property` stops at its existing `ir.lower_construct` seam).

**Why now, not after PROP-005:** this is the one part of §2 rule 1 with a user-visible hole today. A struct may declare `op -_`, `op ~_` or `op !_`, and the checker accepts the declaration. Every use of one stops the build at `grammar.prefix_op_call` (`operators/err_prefix_op_call_seam`), on all three engines, because the checker is shared. Infix struct operators already run their members on all three (PROP-026 on the seed, `lower_binary` in PilIR). The rest of §3 (deleting `map_binop_sym`, `BinaryOp`, …) still waits for PROP-008 step 2 and PROP-005 5D.

**Read 2026-11-16:**
- **Seed:** `compile_unary` (`crates/pilot-vm/src/lower_expr.rs:722`) maps flags `0`/`4`/`5` straight to `Neg`/`BitNot`/`Not`. Unpacking (`6`/`7`/`8`) already finds its member name through a Pilot binding, `native_member("unpack_op", flag)`, set by `bind_native_member("unpack_op", "6", "*_")` in `stdlib/core/builtins/native_calls.pilot`. So the seed spells no operator name.
- **PilIR:** `lower_unary` (`selfhost/ir/lower.pilot:880`) emits `sub`/`bit_xor`/`cmp` for every operand.
- **The checker:** `check_prefix_call` (`selfhost/check/rules.pilot:1017`) raises the seam.
- **Interfaces** (`INegateOperator`, `IBitwiseNot`) declare `op - ()`/`op ~ ()` without bodies. Primitives keep their machine op: dispatch is only for a user struct whose member has a body, as for infix.

**Design:**
- **One table, in Pilot.** `bind_native_member("unpack_op", ..)` becomes `bind_native_member("prefix_op", ..)`, and gains `"0" -> "-_"`, `"4" -> "~_"`, `"5" -> "!_"`. It is the one place that says which member a prefix flag calls. Both engines read it, and neither spells `-_`.
- **Seed:** `compile_unary`, for flags `0`/`4`/`5`, first asks the table for the member. If the operand's static type is a user struct (`self.structs`) declaring it, the seed emits the method call, as `compile_unpack_operand` does. `compile_unpack_operand` reads the renamed key.
- **PilIR (Pilot):** `lower_unary` does the same through `bound(lw, native_member_hook(), ["prefix_op", flag])` and `runs_as_code`/`member_call`. The emitted C follows PilIR.
- **Checker (Pilot):**
  - `check_prefix_call` and its seam are deleted; seam `grammar.prefix_op_call` is retired.
  - The type of `-m` is the member's return type.
  - A struct with no such member is an error, `METH001` ("`P` declares no prefix `op -_`"). Checked 2026-11-16: today that use, too, only stops at the seam. So a struct is never negated by the machine op.

**Rust (needs approval). Strictly this file:**

| File | Change |
| :--- | :--- |
| `crates/pilot-vm/src/lower_expr.rs` | `compile_unary`: flags `UNARY_NEG`/`UNARY_BIT_NOT`/`UNARY_NOT` call the struct's prefix member when the `prefix_op` table names one the operand's struct declares. `compile_unpack_operand`: the key `"unpack_op"` becomes `"prefix_op"`; the lookup is shared through one small helper. |

**Estimate:** about `+12` lines. The whole seed bytecode compiler goes with PROP-005.

**Acceptance:**
- **New `operators/prefix_op_call`** (expected values hand-worked) runs on `pilot test`, `--ir` and `--emit c`:
  - `-m` on `Money` runs `op -_`;
  - `~f` on a flag set runs `op ~_`;
  - `!s` runs `op !_`;
  - `-5` and `!true` stay machine ops.
- **`err_prefix_op_call_seam` is deleted.** `hole_patterns` gains its prefix calls.
- **Misuse case `operators/err_prefix_op_missing`:** `-p` on a struct without `op -_` reports `METH001` and exits 1.
- `operators/*`, `collections/*` (unpacking) and the whole suite pass on all three engines. `cargo test -p pilot-vm` and `cargo fmt --check` pass.

---

## `PROP-023` — The Language Section of `toolchain.pilc`: Pilot's Tables Precompiled, Read by Number

**Status: DRAFT 2026-11-16, design only; not for building until PROP-021, PROP-022 and PROP-005 land.** Owner questions 2026-11-16: "how would you suggest pilot define its entire code like literals, meaning, everything" and "what would the non .pilot text look like?". The layered design is written up in `docs/self_description.md`.

### 1. The section

`selfhost/vm/pilc_writer.pilot` writes a `LANG` section into `toolchain.pilc` (`PIL3`), compiled from the `.pilot` declarations, never edited by hand:

```
section LANG
  strings    one pool; every name below is an index into it
  lex        the LexTable: byte classes, DFA states, accepting families, modes (PROP-021)
  operators  (symbol, fixity, level, assoc, member or rule)                   (PROP-008/018/022)
  type_ops   (symbol, fixity, type level, assoc, template)                     (PROP-019)
  rules      (sequence head of words and hole kinds, template AST)             (PROP-009)
  core_forms the fixed list, by ordinal
  receptors  (token family, TypeId, from-text function)                        (PROP-021)
```

`pilot inspect lang` prints it with names filled in.

### 2. Properties

1. **Derived, never edited**: the gate rebuilds the section and compares it byte for byte (as Gate 2 compares Stage 2 with Stage 3); a stale or hand-patched file fails.
2. **The seed sees only numbers**: ordinals, pool indices and bitmaps; it compares bytes it is handed and holds no language word.
3. **Meanings are references** to Pilot functions and templates in the `CODE` section.
4. **Versioned**: a table layout change bumps the `.pilc` version, append-only like the airlock, so an old seed refuses a new file.

### 3. The genesis file and the end state

`stdlib/core/genesis.pilot` is written in the 14 seed words and plain decimal numbers only and holds the token patterns the rest of the bootstrap needs (`PROP-021` §2.6); every other file is read with the tables before it (the multi-pass bootstrap PROP-008 introduced). After the first `toolchain.pilc`, each one is built by the last; `crates/pilot-lexer` and the text-reading half of `crates/pilot-parser` are deleted, and the seed only loads `.pilc`.

---

## `PROP-024` — `//` Always Starts a Comment: Delete the `//=` and `//` Operator Tokens

**Status: APPROVED BY OWNER 2026-11-16 ("Approve PROP-024") AND BUILT 2026-11-16: `lexer.rs` 537 → 535, `token.rs` 247 → 245 (net −4; `rustfmt` realigned the comments of the compound-assignment variants); the repro moved to `tests/conformance/text/comment_double_slash_equal` and its known-bug entry is deleted. Original draft below.** Owner direction 2026-11-16 about `//=`: "I thought it was a comment because of the // if it means nothing then leave it meaning nothing unless it was supposed to mean something but that would conflict with the comment".

### 1. What exists today (read and run 2026-11-16)

- `crates/pilot-lexer/src/lexer.rs:78` starts a line comment at `//` only when the next byte is not `=` (`b2 != b'='`).
- `lexer.rs:425-426` then makes `//=` the token `DoubleSlashEqual` and `//` the token `DoubleSlash`. The `//` arm can never be reached: every `//` not followed by `=` is already a comment.
- Neither token means anything. No `op //` is declared, the parser's assignment table has no `//=` entry, and nothing else in `crates/` or the `.pilot` tree names either kind.
- **Result:** `x //= 2;` silently does nothing (run: `var x = 7; x //= 2; print(x);` prints `7`), and a comment that begins `//=`, such as `//=== heading ===`, is parsed as code and rejected with `SYN001`. That is known bug `lexer.comment_double_slash_equal` (`tests/conformance/known_bugs/comment_double_slash_equal.pilot`, expected `7` then `8`).
- They come from a `//` floor-division operator described in `docs/lexical-reference.md`, which can never be written because `//` starts a comment.

### 2. Rust (needs approval). Strictly these files:

| File | Change |
| :--- | :--- |
| `crates/pilot-lexer/src/lexer.rs` | Drop `&& b2 != b'='` from the line-comment test (line 78). Delete the two arms at lines 425-426. |
| `crates/pilot-lexer/src/token.rs` | Delete `DoubleSlash = 277` and `DoubleSlashEqual = 306`. Other variants keep their numbers. |

**Estimate:** about `-6` lines. `paused_max_total_rust_loc()` holds or ratchets down.

### 3. Pilot and docs (no approval needed)

- The repro moves from `known_bugs` to `tests/conformance/text/comment_double_slash_equal.pilot` and its known-bug entry is deleted.
- `docs/lexical-reference.md` loses the `//` floor-division and `//=` rows. `docs/vocabulary_in_rust.md` drops `//=` from its findings.

### 4. Acceptance

- `comment_double_slash_equal` prints `7` then `8` on `pilot test`, `--ir` and `--emit c`.
- Every existing conformance program passes unchanged, and `cargo test -p pilot-parser --test damaged_source -- --include-ignored` passes.

---

## `PROP-025` — The Parser in Pilot: One Grammar-Table-Driven Pilot Parser Builds the AST the Checker and PilIR Read; the Rust Parser Is Deleted

**Status: APPROVED BY OWNER 2026-11-18 ("approve"); queued as roadmap steps B5 (steps 1–3) and B7 (the rest). Pilot work needs no further approval; every Rust change it reaches still comes back as an itemized list. DRAFT 2026-11-16.** Owner direction 2026-11-16: "is structural grammar parsing in pilot, or is that also rust? If not then it should be part of pilot and not rust. The only thing that should exist in rust is the absolute necessary rust engine needs for compilation and memory management".

### 1. What exists today (read 2026-11-16)

- **Every `.pilot` file is parsed by Rust.** `crates/pilot-parser` (9,165 lines) builds the AST, and `crates/pilot-ast` (917 lines) holds it. `crates/pilot-lexer` (1,116 lines) is covered by `PROP-021`.
- **The Pilot checker reads the Rust parser's arena.** `selfhost/check/snapshot.pilot` takes it whole through the host operation `chk_snapshot` (`crates/pilot-cli/src/check.rs`). The PilIR lowering also starts from that arena.
- **Pilot has two partial parsers, and neither one is used for real compilation.**
  - `selfhost/parser.pilot` (1,021 lines) runs on the hand-written `selfhost/lexer.pilot`, keeps its AST in `__engine("map_new")` maps, and stops at the seam `parser.operator_grammar` on the first binary operator.
  - `selfhost/cst/cst_parser.pilot` (491 lines) builds the lossless tree for `fmt`.
- **`PROP-006` (Wave 6) already plans to delete** `crates/pilot-{lexer,parser,ast,span,intern}` once `toolchain.pilc` is precompiled. No proposal itemizes the Pilot parser that has to exist before then. This proposal does.

### 2. Design

- **One parser, in Pilot, driven by declared tables.** It reads the token stream from the `PROP-021` table. It takes every operator's level, associativity and fixity from the declared grammar (`OpLevel`, `PROP-018` holes, `PROP-019` type levels), and every `syntax` rule from its declaration. It hardcodes only the mechanisms: a Pratt loop, sequence matching for rules, and the 13 structural delimiters, which are themselves token families. The AST is a flat Pilot arena (`0b10` owned, `List`-backed nodes with spans), not engine maps.
- **It replaces all three.** The Rust parser goes, `selfhost/parser.pilot` is rewritten in place, and `cst_parser.pilot` becomes a mode of the same parser that keeps trivia, so `fmt` and the compiler can never disagree about the grammar.
- **The checker and PilIR read the Pilot AST.** `snapshot.pilot`'s `chk_snapshot` and the arena reads in `selfhost/ir/` move onto the Pilot arena. `chk_snapshot` then has no caller, and its Rust side is deleted under `PROP-006`.
- **Diagnostics** keep their codes and messages. The `damaged_source` corpus becomes a Pilot conformance set: every input must produce diagnostics without a trap.

### 3. Bootstrap: why Rust parsing can go entirely

The seed needs a parser only to read `.pilot` text, and after `PROP-023` / `PROP-006` it never does. It loads `toolchain.pilc`, which already contains this Pilot parser compiled. Only the first `toolchain.pilc` is built with today's Rust parser. From then on each one is built by the previous one, and the gate checks the fixed point (Stage-2 == Stage-3, AGENTS.md §9). After that, Rust keeps only the `.pilc` loader, the dispatch loop, memory slabs and the frozen `HostOp`s, as the owner directed.

### 4. Order

1. `PROP-021` builds the lexer table, which the parser consumes.
2. The Pilot parser is built on the side and checked against the Rust parser: both ASTs are dumped for every file in the tree and compared by a one-off dump, the same method as `PROP-021` §5. A difference is a bug in one of them, found and fixed, never accepted.
3. `snapshot.pilot` and `selfhost/ir/` switch to the Pilot AST behind a flag, then by default once all three engines pass.
4. `PROP-005` (bytecode from PilIR in Pilot) and `PROP-023` (precompiled tables), then the `PROP-006` cutover deletes the Rust parser and AST crates.

Steps 1–3 are Pilot only and need no Rust change. Step 4's Rust deletions are itemized in `PROP-006` when it is expanded.

### 5. Acceptance

- Identical ASTs from both parsers for every `.pilot` file in the tree, compared by a one-off dump.
- `pilot test`, `--ir` and `--emit c` pass unchanged with the checker reading the Pilot AST.
- The `damaged_source` inputs as Pilot conformance programs: diagnostics, never a trap.
- **Litmus:** respell an operator, add an operator level, and add a `syntax` rule, each a Pilot-only edit, with the program parsing and running on all three engines.

---

## `PROP-026` — Seed: a Struct's Operator Applies to the Result of Another Operator or a Method Call (`a + b + c`)

**Status: APPROVED BY OWNER 2026-11-16 ("Approve PROP-026") AND BUILT 2026-11-16 (§5).** Owner request 2026-11-16 ("Draft the seed fix"), so that token patterns can be written with Pilot's own operators (`digit + (lit("_").opt() + digit).many()`, `PROP-021`) instead of nested function calls.

### 1. What exists today (read and run 2026-11-16)

- **The bug:** known bug `seed.chained_struct_operator` (`tests/conformance/known_bugs/chained_struct_operator.pilot`, expected `(e | f)`, `([a] b)`, `((a b) c)`, worked by hand). On the seed, `a + b + c` and `a.wrap() + b` on a struct that declares `op +` stop with `runtime error: cannot apply + to record and record`. PilIR and the emitted C print the expected output.
- **The cause:** the seed calls a struct's operator only when it can name the left operand's type (`compile_declared_operator`, `crates/pilot-vm/src/lower_expr.rs:498`, through `static_type` at `:556`). `static_type` knows a variable's declared type, a plain function call's return type and a struct construction. It returns `None` for an operator result (`BinaryExpr`) and for a method call (a `CallExpr` whose callee is a `MemberExpr`). The seed then falls back to the machine `+`, which fails on two records.
- **The return types are already collected.** `crates/pilot-vm/src/lower_stmt.rs:177-182` records every declared return type in `fn_ret_types`, keyed `Owner.name` for members, operators included (`Pat.+`). Nothing new needs collecting.

### 2. Rust (needs approval). Strictly this file:

| File | Change |
| :--- | :--- |
| `crates/pilot-vm/src/lower_expr.rs` | `static_type` gains two arms. (1) `BinaryExpr`: the left operand's static type, when it is a struct that declares the operator's symbol (`declares`), gives `fn_ret_types["{ty}.{symbol}"]`. (2) `CallExpr` whose callee is a `MemberExpr`: the receiver's static type gives `fn_ret_types["{ty}.{method}"]`. Both use only what the seed already has: `BinaryOp::symbol`, `declares` and `fn_ret_types`. No name is hardcoded. |

**Estimate:** about `+20` lines. `lower_expr.rs` goes from 769 to about 790. The seed's total stays under `paused_max_total_rust_loc()` (27,800). The whole seed bytecode compiler, this file included, is deleted by `PROP-005`. The fix is worth it now because the `PROP-021` token files are read by the seed until then.

### 3. Pilot (no approval needed)

- The repro moves from `known_bugs/` to `tests/conformance/operators/chained_struct_operator.pilot` and its known-bug entry is deleted.
- Once this is built, `stdlib/core/lex/numbers.pilot` is rewritten with `+`, `|` and `.opt()` / `.many()` / `.some()` / `.repeat()` on `Pattern`. `seq` and `alt` stay for lists of parts.

### 4. Acceptance

- `operators/chained_struct_operator` prints `(e | f)`, `([a] b)`, `((a b) c)` on `pilot test`, `--ir` and `--emit c`.
- Every existing conformance program passes unchanged on all three engines. `cargo test -p pilot-vm` and `cargo fmt --check` pass.
- **Must not change:** a struct that declares no operator still gets the machine operator, and integers, floats and strings are untouched. That is checked by the existing `operators/*` and `numbers/*` programs.

### 5. As built 2026-11-16

- **Only the approved file changed:** `crates/pilot-vm/src/lower_expr.rs`, 769 → 797 lines (`+30 −2`; estimated `+20`). The draft's claim that the seed stays under the 27,800 ceiling was wrong: the seed came to 27,807 (GATE003). The owner raised `paused_max_total_rust_loc()` to 28,000 on 2026-11-16 ("raise it to 28,000").
  - The two arms are as drafted.
  - Building found one more case the draft missed: the left operand can be a call to a function of an *imported* unit, such as `lit("ab") | lit("ac")` in a program that imports `Pilot.Core.Lex.Patterns`. The seed compiles those only when they are first called, so their return type was not recorded yet when `static_type` asked for it. A helper, `declared_ret`, now reads the declared return type from the imported declaration (`lazy_decls`) when it is not compiled yet. It serves plain calls, method calls and operators alike.
- **Tests:**
  - The repro is `tests/conformance/operators/chained_struct_operator`, and its known-bug entry is deleted.
  - The lex patterns now use the operators: `stdlib/core/lex/numbers.pilot` is written with `+`, `|`, `.opt()` and `.many()`, and `Pattern` declares `op +`, `op |`, `.opt()`, `.many()`, `.some()` and `.repeat(min, max)`. The free functions `opt`, `many`, `some_of` and `repeat` are deleted, so there is one way to write each. `seq` and `alt` stay for lists of parts.

---

## `PROP-027` — Operators a Type Gains Through `ext` Apply to Literals (`"0" ..= "9"`); a Struct's `op _[_]` Takes a Range Index (`hex[1 ..= 6]`)

**Status: APPROVED BY OWNER 2026-11-16 ("Approve PROP-027"); BUILT 2026-11-16 (see §5).** Owner question 2026-11-16: "Can we use "A".."B" for range and something for repeat?"

### 1. What exists today (run 2026-11-16)

- **`..` excludes its end and `..=` includes it.** `for i in 0 .. 3` runs 3 times and `0 ..= 3` runs 4. A byte range includes both ends, so the Pilot spelling is `"0" ..= "9"`. `"0" .. "9"` would leave out `9`.
- **`p * 3` (exactly three times) works today** on all three engines. It is built (`Pattern` declares `op *`, tested in `lex/dfa_minimal`). `.repeat(min, max)` covers a bounded count.
- **`"0" ..= "9"` does not work yet.** Giving `str` that meaning is an `ext str { op ..= (hi: str) -> Pattern }` in `Pilot.Core.Lex.Patterns`. A probe found two gaps:
  - The seed: `compile_declared_operator` (`crates/pilot-vm/src/lower_expr.rs:498`) calls a declared operator only for a struct, and `static_type` has no type for a literal. A range operator then falls through to `SEED001` "range values are only lowered as `for` iterables and slice indices by the seed".
  - PilIR stops at the seam `ir.lower_construct`.
- **`hex[1 ..= 6]` does not work on any engine.** An index that is a range is always lowered as a string slice (`compile_index_value`, `lower_expr.rs:267-292`, which calls `str_slice` whatever the receiver is). PilIR and the emitted C do the same. This is known bug `seed.struct_range_index` (`tests/conformance/known_bugs/struct_range_index.pilot`, expected `d[1..6]`).

### 2. Design

- **A literal has the type its receptor binds** (`bind_receptor`, the string family). The seed reads that type name from the bootstrap tables, so nothing is hardcoded. `compile_declared_operator` calls an operator for any type that declares it, not only structs: `declares` already knows `ext` owners. That covers `"0" ..= "9"` and any other operator an `ext` gives a literal's type.
- **An index by a range** on a receiver whose static type declares `op _[_]` calls that operator with the range value. The string slice stays for receivers with no such operator. The range value is built the way `compile_operator_fn` builds any operator value (the function Pilot binds to `..=`). If no function is bound for the operand type, the case stops at a registered seam, never at a wrong slice.
- **PilIR:** the same two rules in `selfhost/ir/` (Pilot, no approval needed). Closing `ir.lower_construct` for these node kinds is part of the slice.

### 3. Rust (needs approval). Strictly this file:

| File | Change |
| :--- | :--- |
| `crates/pilot-vm/src/lower_expr.rs` | `static_type`: a string literal's type from the receptor tables. `compile_declared_operator`: any declaring owner, not only structs. `compile_index_value`: a declared `op _[_]` with a range argument before the string slice. |

**Estimate:** about `+25` lines. The seed stays under the 28,000 ceiling (about 27,830). All of it is deleted with the seed bytecode compiler by `PROP-005`.

### 4. Acceptance

- `lex/byte_range_literal.pilot`: `"0" ..= "9"`, `"a" ..= "f" | "A" ..= "F"` and `hex[1 ..= 6]` give the same tables as the function forms (state and class counts worked by hand), on all three engines.
- `seed.struct_range_index` passes and moves to `operators/`.
- Every existing conformance program passes unchanged. In particular, string slicing (`s[1 ..= 3]`) is unchanged.
- Once it passes, the lex files use `"0" ..= "9"` and `range("0", "9")` is deleted, so there is one spelling.

### 5. As built (2026-11-16)

**Two places differ from §2. Both are recorded here.**
- **There is no receptor that names a literal's type.** No `bind_receptor` binds the string family to a type, so the seed cannot read one from the bootstrap tables. Only the checker knows a literal's type (`literal_slot`). The seed rule was changed to one that needs no literal type:
  - An operator the seed has no lowering of its own for (no instruction, no `bind_operator_fn`) that exactly one type declares as an instance member is a call of that type's operator (`sole_owner`).
  - With the lex files loaded, that one type is `str` (its `ext` block), so `"0" ..= "9"` calls `str...=`.
  - If a second type declares the same operator, the seed stops at the old `SEED001` again. It never guesses.
  - `static_type` gives such a call the declared return type, so `"a" ..= "f" | "A" ..= "F"` runs `Pattern`'s `op |`.
- **The range value for an index is built from its two ends, not by a bound function.** No function is bound to `..=`. The seed and PilIR build a value of the operator's declared parameter type (`span<i64>`) from the two ends, in its first two fields. This is the structural bounds-pair rule (field anatomy), with no type name in the code.
- **Rust: `+73` lines, not `+25`** (`lower_expr.rs` 797 → 870: `sole_owner`, `first_param_type`, the index branch, the fallback call). The workspace is about 27,880, under the 28,000 ceiling.

**Pilot (no approval needed).**
- `selfhost/check/infer.pilot`:
  - `binary_type`: any operator not typed above gives the return type its left operand's type declares (`i64`'s `..=` gives `span<i64>`, `ext str`'s gives `Pattern`).
  - New `index_takes_range`: a type's `op []` whose parameter is a non-register named type takes a range. `x[a ..= b]` on such a type has the operator's return type. Strings and lists keep slicing.
- `selfhost/check/snapshot.pilot`: `binary_op_symbols` moved here from `selfhost/ir/lower.pilot`, so the checker and the lowering share one table.
- `selfhost/ir/lower_members.pilot`:
  - New `lower_bounds_record`: a range value whose type is a record of its two ends.
  - `lower_index`: calls the type's `op []` when `index_takes_range`.
- `selfhost/ir/lower.pilot`: `lower_binary` builds range values with `lower_bounds_record`.
- `stdlib/core/lex/patterns.pilot`:
  - Adds `op _[_] (counts: span<i64>)` and `ext str { op ..= }`.
  - Deletes `range()` and `.repeat()`.
  - Imports `Pilot.Core.Primitives.Span` explicitly. Without the import, the seed has no fields for `span`.
- `stdlib/core/lex/numbers.pilot` and the lex tests use `"0" ..= "9"` and `p[a ..= b]`. `p * n` stays. A range binds looser than `+` and `*`, so `("0" ..= "9") * 3` needs its brackets; it binds tighter than `|`.

**Tests.**
- New `lex/byte_range_literal` passes on all three engines.
- `seed.struct_range_index` passes on all three. It moved to `operators/struct_range_index` (with the `Span` import added) and its `bug_registry` entry is deleted.

---

## `PROP-028` — Fill the `bridge.proc_signal` Seam's `signal_handle` Sub-Op, So `pilot test --timeout` Can Stop a Hung Case

**Status: APPROVED 2026-11-16 (owner: "Approve PROP-028"); BUILT 2026-11-16.** Owner request 2026-11-16: "Yes, A1 then A3" (A1: a hung case must fail).

**As built 2026-11-16:** `crates/pilot-vm/src/bridge.rs` `34 =>` gained `0 if i_at(2) == 9` (`Child::kill`, `-11` for an unknown handle), net `+4` lines; the seam now names `signal_pid` and signals other than 9. In Pilot: `ProcessHandle.kill()` (`stdlib/std/proc.pilot`), and `JobPool.time_out` closes the pipes, kills and waits. `runtime/proc_job_timeout` gained the spinning job (`spinner: gone`); `test_timeout`'s sleeper spins. Both pass on `pilot test`, `--ir` and `--emit c` (39–40 s each run), and no fixture process is left afterwards.

### 1. What exists today (run 2026-11-16)

- **`pilot test --timeout S` is built in Pilot.** `stdlib/std/proc.pilot` `JobPool` gives each job a limit (`set_limit`), and a job still running past it is handed back as timed out. The runners report it as `FAIL … timed out after S s` and go on. Tests: `tests/conformance/runtime/{proc_job_timeout,test_timeout}` pass on `pilot test`, `--ir` and `--emit c`.
- **The hung child keeps running.** HostOp 34 `ProcSignal` (frozen sub-ops `0=signal_handle, 1=signal_pid, 2=self_pid, 3=exit`, §3 table) answers only `2` and `3` in the seed VM. Every other sub-op stops at seam `bridge.proc_signal` (`crates/pilot-vm/src/bridge.rs`, the `34 =>` arm). The runner itself runs on the seed VM, so `JobPool` cannot call `Process.send_signal(0, handle, 9)`: the call would trap. The emitted-C runtime already implements it (`runtime/c/pilot_rt_bridge.h`, `case 34`: `kill(a1, a2)`).
- So a case that loops forever is reported as failed but goes on using a CPU until the machine stops it. For a sleeping case this costs nothing; for a spinning one it costs a core until it ends.

### 2. Design

- **Sub-op `0` (`signal_handle`) with signal 9** kills the child the handle names: `vm.bridge.procs.get_mut(&h)` then `Child::kill()`. That is SIGKILL on Unix and `TerminateProcess` on Windows. The handle stays in the table, so the caller's `Process.wait` reaps it as usual.
- **Any other signal, and sub-op `1` (`signal_pid`),** stay at the seam: nothing needs them yet, and `std::process` has no portable way to send them.
- **No ABI change:** the op, its sub-op numbers and its signature are already frozen; this fills one of them (Model B+, append-only meaning unchanged). A handle that does not exist returns `-11` (`-E_BADF`), as op 33 does.
- **Then, in Pilot (no approval needed):** `JobPool.time_out` calls `Process.send_signal(0, id, 9)` and waits for the child, and the `test_timeout` fixture's sleeper can spin instead of sleep.

### 3. Rust (needs approval). Strictly this file:

| File | Change |
| :--- | :--- |
| `crates/pilot-vm/src/bridge.rs` | The `34 =>` arm: `0 if i_at(2) == 9 =>` kill the child named by handle `i_at(1)` (`Child::kill`), `-11` for an unknown handle; the seam message names what is still missing (`signal_pid`, other signals). |

**Estimate:** about `+6` lines. To hold the Rust total flat (`paused_max_total_rust_loc()`), the slice deletes as many lines elsewhere in the seed or records the `+6` against the ceiling with the owner's say.

### 4. Acceptance

- `tests/conformance/runtime/proc_job_timeout.pilot` gains a job that spins (`while true {}` in `sh`) with a 2 s limit: it is timed out, and afterwards `kill -0 <pid>` reports it gone.
- `runtime/test_timeout`'s sleeper spins, and the run still ends under 40 s.
- `cargo test -p pilot-vm`, `cargo fmt --check`, and every conformance program on the three engines pass unchanged.

## `PROP-029` — Seed Lexer: `!` Ends No Operand, So `!.on` Reads as `!self.on`

**Status: SUPERSEDED 2026-11-17 by `PROP-021` slice 21H** (the seed runs the table built from the token declarations, where `!` ends no operand; `crates/pilot-lexer/src/lexer.rs` no longer has the hand-written `!` branch). Known bug `seed.not_prefix_dot` is retired and its repro is `tests/conformance/operators/not_prefix_dot`. Owner decision 2026-11-16: "Read as !self.x".

### 1. What happens today (run 2026-11-16)

- AGENTS.md §5.1: a `.` glued to an operand is `InfixDot`; anywhere else it is `PrefixDot`, which reads as `self.<name>`. So `-.level` is `-self.level`, and the seed reads it that way (prints `-3`).
- `!.on` is not read that way. The seed lexer emits `!` (`TokenKind::Exclamation`, `crates/pilot-lexer/src/lexer.rs`, the plain-`!` `emit` at the end of the `b0 == b'!'` branch) with `can_precede_infix_dot = true`, so the `.` after it is infix, and `!.on` becomes a member read with nothing on its left. `pilot check` passes; at run time the seed stops with `cannot apply ! to null`, and PilIR stops inside the checker snapshot (`list index -9`).
- `!` never ends an operand in Pilot: postfix `x!` is not Pilot. The seed parser reads it only to report `NULL001`. No `.pilot` file in the tree writes `x!.y`.
- The Pilot lexer already gets it right. `stdlib/core/lex/operators.pilot` lets only declared postfix operators (`_?`) end an operand, so `pilot lex` gives `! prefix_dot name`. Test: `tests/conformance/lex/dots` (passes).
- Repro: `tests/conformance/known_bugs/not_prefix_dot.pilot` (true output `false -3`), registered as `seed.not_prefix_dot` in `bug_registry()`.

### 2. Design

- Emit plain `!` with `can_precede_infix_dot = false`, as every other prefix operator (`-`, `~`, `*`) already is. A `.` touching `!` is then prefix, and `!.on` reads `!self.on`.
- `x!.y` (postfix `!`, then a member) would then read `x ! self.y`. That is a parse error, which is still loud, and `x!` is already NULL001.
- The call-quote tokens (`!(`, `!!(`, `!&(`, …) and `!=` are unchanged.

### 3. Rust (needs approval). Strictly this file:

| File | Change |
| :--- | :--- |
| `crates/pilot-lexer/src/lexer.rs` | The plain-`!` `self.emit(TokenKind::Exclamation, …)`: last argument `true` becomes `false`. |

**Estimate:** `0` lines net.

### 4. Acceptance

- `known_bugs/not_prefix_dot` prints `false -3` on the seed. It then moves to `tests/conformance/syntax/` (or an equivalent case folder), passes on `pilot test`, `--ir` and `--emit c`, and its `bug_registry()` entry is retired.
- `lex/dots` still passes. `cargo test -p pilot-lexer`, `cargo test -p pilot-parser --test damaged_source -- --include-ignored` and `cargo fmt --check` pass.
- Every conformance program passes unchanged on the three engines.

## `PROP-030` — Seed: A `const` List or Map of Any Length Compiles

**Status: APPROVED 2026-11-16 (owner: "Approve PROP-030"); BUILT 2026-11-16.** As built: `crates/pilot-vm/src/lib.rs` `materialize_cmap` takes a mark and releases to it after each `list_push`/`map_set`, net `+6` (with a comment). `collections/const_list_long` (the repro, promoted) passes on the seed; known bug `seed.const_list_registers` is retired. Found while building UTF-8 names (PROP-021 §2.8; owner chose Unicode XID 2026-11-16). Unicode's identifier letters are 1,041 ranges, kept as `const` data in `stdlib/core/lex/unicode_xid.pilot` (generated by `tools/unicode_xid/main.pilot`). imp_type.md Rule 11 says known data is constant data.

### 1. What happens today (run 2026-11-16)

- A `const` (or `let`) list whose items are all constants is folded at compile time into a `CMap` (`eval_cmap`). Every read of the name rebuilds it with `materialize_cmap` (`crates/pilot-vm/src/lib.rs:980`).
- For each item, `materialize_cmap` keeps the item's register until the whole list is built. A list literal (`compile_list_literal`, `lower_expr.rs:836`) instead releases it after each `list_push`. So a constant list past about 120 items stops the build: `SEED001: expression needs more than 127 scratch registers`, reported once per item past the limit.
- The same holds for a constant map.
- PilIR and the emitted C read the same program correctly.
- Repro: `tests/conformance/known_bugs/const_list_registers.pilot`: 300 items; true output `300`, `44850`. It is a reproduced known bug on the seed and passes on `--ir`.

### 2. Design

- **The fix:** in `materialize_cmap`'s `List` and `Map` arms, take `let mark = self.next_reg;` before each item and `self.release_to(mark);` after its push or set, as `compile_list_literal` does.
- **Not fixed here:** the seed still rebuilds the constant at every read, which costs time, not correctness. It goes with the bytecode compiler in PROP-005. Static data in the output is stand-in `ir.const_static_data` on the other engines.

### 3. Rust (needs approval). Strictly this file:

| File | Change |
| :--- | :--- |
| `crates/pilot-vm/src/lib.rs` | `materialize_cmap`: release each item's registers after it is pushed (`List`) or set (`Map`). |

**Estimate:** about `+4` lines.

### 4. Acceptance

- `known_bugs/const_list_registers` passes on the seed. It moves to `tests/conformance/collections/const_list_long`, passes on all three engines, and its `bug_registry()` entry is retired.
- Every conformance program passes unchanged. `cargo test -p pilot-vm` and `cargo fmt --check` pass.
- Then UTF-8 names (`lex/utf8_names`) can be finished on the seed.

## `PROP-031` — Rust Milestone Tests That Name Deleted Pilot Code

**Status: APPROVED 2026-11-16 (owner: "Approve PROP-031").**

Retiring the hand-written tokenizers (PROP-021 §8) deleted Pilot code that three Rust test files still read by name. AGENTS.md §9.1 says language and library behaviour is tested in Pilot, and `runtime/fmt_command` now covers `pilot fmt` through the CLI. These Rust tests were not run for this change.

| File | What it names | Proposed change |
| :--- | :--- | :--- |
| `crates/pilot-analysis/tests/milestone04_dual_syntax_pairings_and_bidirectional_formatter.rs` | `format_pilot_with_width`, `format_pilot_source`, `format_pilot_with_style` in `selfhost/fmt/fmt.pilot` (4 of its tests already failed before this change, see PROP-016) | Delete the formatter tests; `tests/conformance/runtime/fmt_command` replaces them. |
| `crates/pilot-analysis/tests/milestone10_compiler_as_a_library.rs` | `fmt_style_compact`, `fmt_style_readable` in `stdlib/pilot/fmt.pilot` (deleted: constant functions, rule 11) | Drop those markers and the `format_str(.., fmt_style_readable())` case. |
| `crates/pilot-analysis/tests/milestone60_selfhost_cli_pkg_perf_and_bench.rs` | `selfhost/cli/repl.pilot`, `repl_session_new`, `repl_eval_line` (deleted: a fake) | Delete the REPL part of `test_60_3_and_60_4_..`. |

**Acceptance:** `cargo test -p pilot-analysis` names none of the deleted items; no other file changes.

**As built (2026-11-16):** `milestone10` and `milestone60` edited as above (the `milestone60` test is renamed `test_60_3_and_60_4_selfhost_cli_test_runner_diff_test_and_crate_purge`). `milestone04` is **not** done: deleting `test_m4_3` trips the anti-cheat test-count pin, so every `pilot-analysis` milestone test fails. It was put back unchanged. Failures that remain and predate this change: `test_m10_1` (`FAULT: no function named tokenize`, `stdlib/pilot/lex.pilot` was replaced in PROP-021 21E), `test_60_3` (`selfhost/cli/build_cmd.pilot` does not exist), and `test_m4_3` (it calls the deleted `format_pilot_with_style`).

**Amendment A1 (APPROVED 2026-11-16, owner: "Approve A1"):** also modify `crates/pilot-analysis/tests/common/anticheat.rs` line 221: the `MILESTONE_TEST_MANIFEST` count for `milestone04` goes from `4` to `3`. Then delete `test_m4_3`, `eval_fmt` and the `selfhost/fmt/fmt.pilot` audit rule in `milestone04` as planned, and set its `expected_test_count` to `3`. *Built 2026-11-16: the count pin passes; no `pilot-analysis` test names the deleted code. The three test files still fail, each on older missing items: `milestone04` on `stdlib/core/syntax/call_quote.pilot` lacking `pub syntax` (anti-cheat pass 9), `milestone10` on `tokenize` in `stdlib/pilot/lex.pilot`, `milestone60` on `selfhost/cli/diff_test.pilot` and `build_cmd.pilot`.*

## `PROP-032` — Enums With Text Values (`enum CacheField { Key = "key", ModeOf = "mode_of", }`)

**Status: APPROVED 2026-11-16 (owner: "Approve PROP-032").** Owner request (2026-11-16): "make enums available to have constant string values".

### 1. What happens today (run 2026-11-16)

- A variant's written value must be an integer literal. `enum Field : str { Key = "key", }` stops the seed with ``SEED001: the value of `Field.Key` must be an integer literal`` (`register_enum`, `crates/pilot-vm/src/lib.rs:1046`). The error is reported at `stdlib/core/operators/levels.pilot:1:1`, not at the variant: a location bug of its own, not fixed here.
- The checker (`variant_values`, `selfhost/check/layout_attrs.pilot`) and PilIR (`variant_tag`, `selfhost/ir/layouts.pilot`) read values as integers only.
- Printing an enum value prints its value on all three engines (`High = 9` prints `9`).
- So code that needs a word per variant writes a `switch` by hand: `cache_label` (`stdlib/pilot/lex/cache.pilot`), `token_tag_name` (`selfhost/lexer.pilot`), `wiring_level_label` (`selfhost/gate/ledger/wiring.pilot`).

### 2. Design

- **Declaring:** an enum whose variants write string literals is a *text enum*. Every variant writes its text, and the texts are distinct. A base (`: u8`) is optional and sets the tag's width as today.
- **Representation (PilIR, C):** variants are integer tags `0 .. n-1` in declaration order, exactly as an enum with no values. `==`, `!=` and `switch` stay integer operations. Each text enum has one constant table of its texts, read by tag.
- **Reading the text:** `v as str` gives the variant's text. Printing a text enum value, alone or in an f-string hole, prints its text.
- **What it costs (the owner's aim, 2026-11-16: "the call would be efficient as a const enum"):**
  - A variant named in the source (`CacheField::Key as str`) folds to the string constant `"key"` at compile time. There is no lookup at run time.
  - A value known only at run time costs one indexed load from a static table: `static const` data in the emitted C, never rebuilt per read.
  - Comparing and `switch` cost the same as an integer enum's.
  - The texts are written once, on their variants, so the declaration shows what each value stands for. No separate table or `switch` can fall out of step.
- **Seed:** `register_enum` records each variant's string as its compile-time value (`CMap::Val`, which already holds strings: `eval_cmap`). `as str` is then the value itself, and printing prints it. The seed stays name-blind: an enum is text-valued because its written values are string literals, never by its base type's name.
- **What keeps the two representations unobservable (checker):**
  - `ENUM001`: an integer cast (`as i64`, `as u8`, ..) of a text enum value.
  - `ENUM002`: an enum mixing text and integer values, or a text enum with a variant that writes no text.
  - `ENUM003`: two variants of a text enum with the same text.
  - `<`, `<=`, `>`, `>=` on a text enum have no operator: `METH001`, as for any type without one.
- **Not in this proposal (possible follow-ups):**
  - A variant with no text taking its own name. The seed would need the enum's base type, which it cannot know without naming a type.
  - A lookup from text back to variant (`CacheField.from_text(s) -> CacheField?`).

### 3. Rust (needs approval). Strictly this file:

| File | Change |
| :--- | :--- |
| `crates/pilot-vm/src/lib.rs` | `register_enum`: a written value that is a string literal is stored as that string. An enum mixing string and integer values, or a text enum with a variant without a value, is `SEED001` (the checker reports `ENUM002` for the same program). |

**Estimate:** about `+12` lines. This file has uncommitted changes from another session's approved work. Build after those land, or in coordination with that session.

### 4. Pilot (no approval needed; listed for scope)

| File | Change |
| :--- | :--- |
| `selfhost/check/layout_attrs.pilot` (or a new `selfhost/check/enum_text.pilot`) | Recognise text enums; `ENUM002`, `ENUM003`. |
| `selfhost/check/infer.pilot` | Type `v as str` on a text enum as `str`; `ENUM001` for integer casts. |
| `selfhost/diagnostics/errors.pilot` | Register `ENUM001`–`ENUM003`. |
| `selfhost/ir/lower.pilot` (`lower_cast`) and the print/format lowering | `as str`, printing and f-string holes of a text enum read its constant text table by tag. |
| `selfhost/ir/layouts.pilot` | `variant_tag` gives a text enum's variants `0 .. n-1`. |
| `docs/keyword_matrix.md` (`enum` row), `docs/concept_matrix.md`, `pilot.md`, `rust.md`, `selfhost/gate/ledger/wiring.pilot` | Same-change sync. |

`selfhost/ir/*` and `selfhost/check/infer.pilot` currently have uncommitted changes from the session working on the PilIR interpreter. That part is sequenced after it, or done in coordination.

### 5. Acceptance (tests written first, expected output worked by hand)

- `tests/conformance/types/enum_text_values.pilot`: `as str`, printing, an f-string hole, `==`/`!=`, an exhaustive `switch`, and a `: u8` text enum. All three engines agree.
- `tests/conformance/errors/err_enum001_text_int_cast`, `err_enum002_mixed_values`, `err_enum002_missing_text`, `err_enum003_duplicate_text`.
- `types/enum_explicit_values` and every other conformance program pass unchanged. `cargo test -p pilot-vm` and `cargo fmt --check` pass.
- Then the three hand-written `switch` mappings above become text enums, and their callers use `as str`.

### 6. As built (2026-11-16)

- **Rust:** `register_enum` (`crates/pilot-vm/src/lib.rs`, `+10` lines, `1,080` → `1,090`): a written value whose compile-time value is a string is stored as that string. Mixed enums are left to the checker (`ENUM002`), which runs on every program the seed accepts; the seed adds no `SEED001` of its own for them.
- **Checker:** `is_text_enum` and `check_text_enum` (`ENUM002`, `ENUM003`) in `selfhost/check/layout_attrs.pilot`; `ENUM001` in `selfhost/check/rules.pilot` `check_cast`; `ENUM001`–`ENUM003` registered in `selfhost/diagnostics/errors.pilot`.
- **PilIR:** `variant_tag` gives a text enum's variants `0 .. n-1`, and `variant_texts` lists their texts (`selfhost/ir/layouts.pilot`). `lower_enum_text` (`selfhost/ir/lower_switch.pilot`) folds `Light::Red as str` to the constant `"stop"`. Any other value reads its text through one `switch_table` of string constants by tag (O(1)). `engine_value` does the same for arguments of native calls (`print`) and operands of arithmetic (`"[" + f`, the f-string holes). Comparisons and `switch` stay integer operations.
- **Difference kept:** a text-enum value passed to a raw `__engine(..)` call keeps its tag on PilIR, and is the text on the seed (where the variant is a string). Inside one engine a value stored and read back through engine ops is consistent; only `Pilot.Core.*` may make such calls (`ENGINE001`).
- **Seed cost:** on the seed a text-enum comparison compares strings, not integers. `TokenTagKind` is compared in the seed-run self-hosted parser, so its hot path pays that on the seed only.
- **Converted:** `CacheField` (`stdlib/pilot/lex/cache.pilot`), `TokenTagKind` (`selfhost/lexer.pilot`), `WiringLevel` (`selfhost/gate/ledger/wiring.pilot`). `CACHE_LABELS`/`cache_label`, `TOKEN_TAG_NAMES`/`token_tag_name` and `WIRING_LEVEL_LABELS`/`wiring_level_label` are deleted, and their callers use `as str` or print the value.

## `PROP-033` — The Analyzer Skips Units Whose Diagnostics Are Thrown Away

**Status: WITHDRAWN BY OWNER 2026-11-18 ("withdraw"). `PROP-035` deleted the analyzer this was meant to speed up. No Rust was touched.**

On every `pilot run` / `pilot check`, `analyze` (`crates/pilot-cli/src/run.rs` ~443-470) runs `selfhost/analyzer.pilot` over the whole linked program, then keeps only the diagnostics whose file is a user file (`if !user.contains(&file) { return None; }`). `analyze_program` (`selfhost/analyzer.pilot` ~1306-1331) runs `analyze_unit` (every `verify_*` pass, Pass 7, Pass 8, `is_suppressed` parent walks) over every unit, stdlib included, and the stdlib is usually most of a linked program's nodes. Passes 1-3 (layouts and inference over all nodes) must stay whole-program, because user code needs stdlib layouts. Only the per-unit checking can be skipped.

The analyzer cannot tell a user unit from a stdlib unit today. The only unit query is `ast_node_unit`, and no query gives a unit's path.

| File | Change |
| :--- | :--- |
| `crates/pilot-cli/src/run.rs` | Pass the unit indices of the user files (the `user` list `analyze` already builds) to the pipeline. |
| `crates/pilot-analysis/src/pipeline.rs` | `AnalysisPipeline::analyze` takes that list and hands it to the analyzer registry. |
| `crates/pilot-analysis/src/infer.rs` | One new read-only op, `ast_unit_is_user(unit) -> 0/1`, next to `ast_node_unit`. |
| `selfhost/analyzer.pilot` (Pilot) | In `analyze_program`, call `analyze_unit` only when `ast_unit_is_user(unit)` is 1. |

**Behaviour:** `pilot run` and `pilot check` give the same diagnostics, because the skipped ones were filtered out anyway. One difference: an analyzer fault raised inside a stdlib unit would no longer be reported. Other callers of `AnalysisPipeline::analyze`, such as the milestone tests, pass "every unit is user" and keep today's behaviour.

**Rust LOC:** about +10 lines, an addition to the seed. It is weighed against a per-case saving on every conformance case on all engines. Measure before/after with `pilot test tests/conformance` before landing.

**Acceptance:** `pilot test`, `--ir` and `--emit c` keep their results. `cargo test -p pilot-analysis` and `-p pilot-cli` show no new failures. `cargo fmt --check` passes. `paused_max_total_rust_loc()` is updated only with the owner's approval.

---

## `PROP-034` — A String Built From Many Parts Is Allocated Once: f-Strings and `+` Chains Lower to One Join, and a Unique String Appends in Place

**Status: DRAFT 2026-11-17, design only; not for building until `PROP-025` steps 1–3 land.** Owner direction 2026-11-17: "lets add this as a proposal after the building blocks of the pilot parser are built from that proposal". The question it came from: does `a + b + c` allocate once for the final string, or once per `+`?

### 1. What happens today (read 2026-11-17)

- **The parser keeps every `+`.** `crates/pilot-parser` builds `((a + b) + c) + d`. `f"x{a}y{b}"` is rewritten into the same left-nested chain (`crates/pilot-parser/src/fstring.rs` `lower_fstring`). `selfhost/ir/pass_fold.pilot` folds integers only, so even `"a" + "b"` reaches run time.
- **Seed VM (`pilot run`, and `--ir`, which runs on the same VM):** each string `+` renders both operands into a Rust `String`, then copies it into a new VM string (`crates/pilot-vm/src/checked.rs` `binary_op`). It also calls `collect_if_due` (`vm.rs`). A chain of k parts makes k − 1 strings and copies the growing prefix each time, which is O(n²) bytes.
- **C backend (`--emit c`):** `pilot_v_concat` (`runtime/c/pilot_rt.h`) is still one `+` at a time, but:
  - strings of 15 bytes or less need no allocation;
  - when the left string ends at its buffer's written end, the right one is appended in place;
  - a growing buffer is reallocated at twice what it needs.
  
  Chains and `s = s + x` loops therefore copy O(n) bytes, amortized, without being sized once.
- **`std.text.join_text` also builds pairwise:** it joins neighbours with `+` round after round, O(n log n). No engine op builds one string from a list of parts.

### 2. What other languages do

- **One many-part concatenation, sized once:**
  - Go: `a + b + c` becomes one `concatstrings` call, on the stack when small and not escaping.
  - C#: `String.Concat(a, b, c, d)`; interpolation goes through a pooled handler; constant pieces are folded at compile time.
  - Java 9+: `invokedynamic` `StringConcatFactory` (JEP 280).
  - Python: f-strings use the `BUILD_STRING n` opcode.
  - Lua: `CONCAT` over consecutive registers.
- **Loops of `s = s + x`:** a builder type, or an in-place append when the string has one owner (CPython refcount 1, Rust `String + &str`, Koka and Lean 4 under Perceus reuse).

### 3. Design

1. **One node in the Pilot parser.** When `PROP-025`'s parser lowers `f"..."`, it emits one `StrBuild` node holding the parts in order. It does not emit a `+` chain. Neighbouring literal parts are joined at parse time. A hole with a format pattern stays a call of the bound format function, as today.
2. **`+` chains join the same node in a Pilot pass, conservatively.**
   - The chain is flattened only from the first operand the checker knows is `str`, so `1 + 2 + "a"` stays `(1 + 2) + "a"`, which gives `"3a"`.
   - Operands of unknown or generic type keep plain `+`.
   - Chains of fewer than 3 parts keep plain `+`.
   - Evaluation stays left to right, and each part is shown exactly as `+` shows it today.
3. **PilIR:** one op, `str_build` (n operands). The verifier, printer, text parser, `pilot inspect` and const-eval learn it. Const-eval folds it when every part is constant.
4. **Engines:**
   - **C (`runtime/c/pilot_rt.h`, not Rust):**
     - `pilot_v_build(n, parts)` adds up the lengths and allocates once.
     - Results of 15 bytes or less stay inline.
     - When the first part is uniquely held and has room, the call appends in place, as `pilot_v_concat` does today.
   - **Seed VM (Rust, frozen airlock):**
     - It needs one appended way to build a string from n registers: either an opcode or one appended `HostOp` with its `[EngineOp]` owner in Pilot.
     - Which one, and the exact files, are itemized when this proposal is sequenced. The choice depends on what `PROP-005` leaves of the bytecode compiler.
     - This is the only Rust in the proposal: about +15 lines. It replaces the per-`+` render-and-copy for the most common string pattern in the selfhost tree.
   - **JS / Python / Lua transpilers:** emit the target's own join (`[...].join("")`, `"".join(...)`, `table.concat`).
5. **In-place append for loops, decided by Perceus.** `selfhost/ir/pass_perceus.pilot` already proves when a value is unique. For `s = s + x` with `s` unique, it emits an append-in-place form (a `drop_reuse` of the string buffer). This makes the C runtime's run-time check a static decision on every engine, visible in `pilot inspect`. A `StringBuilder`-style type is not added.
6. **`join_text` uses `str_build`** once it exists, so library joins also allocate once.

### 4. Order

1. `PROP-025` steps 1–3: the Pilot parser exists and the checker and PilIR read its AST.
2. Steps 3.1–3.3 and the C and transpiler parts of 3.4 (Pilot and C only).
3. The seed VM part of 3.4, itemized for approval. Until it lands, the seed VM lowers `str_build` to today's `+` chain, so behaviour is identical.
4. Step 3.5 (Perceus in place) and step 3.6.

### 5. Acceptance

- **Output:** every program prints exactly what it prints today on `pilot test`, `--ir` and `--emit c`. New conformance cases, with expectations worked out by hand, cover:
  - a numeric prefix (`1 + 2 + "a"` → `3a`);
  - non-string parts (lists, records, `opt`);
  - evaluation order with side-effecting parts;
  - the empty f-string, an all-literal f-string, and format patterns.
- **Allocation:** on the C backend, an f-string of k parts makes at most one allocation. This is checked by a leak and allocation count in `pilot bench`. A 10,000-step `s = s + x` loop copies O(n) bytes on every engine.
- **Speed:** `pilot test tests/conformance` time, measured before and after on the seed and `--ir`.
- **Litmus:** renaming `str` keeps all of this working; the pass finds strings by `TypeId`, never by name.

## `PROP-035` — Stop Running the Retired `selfhost/analyzer.pilot`: Delete `run::analyze`, `cli_analyze` and the Analyzer Loader

**Status: APPROVED BY OWNER 2026-11-17 ("Approve PROP-035"). BUILT 2026-11-17 (see §5); Amendment A1 (the anti-cheat test counts and `test_m27_4`) APPROVED BY OWNER 2026-11-18 ("approve") AND BUILT; `diagnostics.rs` deletion APPROVED 2026-11-18 AND BUILT.**

### 1. Why
`selfhost/analyzer.pilot` (1,336 lines, 394 raw `__engine` calls) reads the Rust AST to report about 35 diagnostics. 5B.4 group 2 moves each check into `selfhost/check` and deletes it from the analyzer in the same change, so no diagnostic is reported twice. When the last check has moved, the analyzer reports nothing, but the CLI still parses the program a second time and runs it on every `pilot check`, `pilot run`, `pilot ir` and `pilot emit`.

### 2. Pilot (no approval needed; done in the same change)
- `selfhost/cli/commands.pilot` `semantic_diags` stops calling `host_analyze`; `in_source_order` and `report_pending_diags` take the checker's list only.
- `selfhost/cli/host.pilot`: `host_analyze` deleted (12 → 11 raw calls).
- `selfhost/analyzer.pilot` deleted; its row leaves `legacy_engine_ratchet()`, `pilot.md` and `wiring.pilot`; `selfhost/.pilc-cache/analyzer.pilc` stays on the retired list.

### 3. Rust (needs approval). Strictly these files:
- `crates/pilot-cli/src/run.rs`: delete `pub fn analyze` and `ANALYZER_FAULT`.
- `crates/pilot-cli/src/host.rs`: delete the `cli_analyze` op and its doc row; `cli_checked_tool` reports the checker's diagnostics alone (`report_pending` loses its `analyzed` argument).
- `crates/pilot-analysis/src/infer.rs`: delete `load_or_compile_analyzer_pilc`, `run_pilot_analyzer`, `analyze_physics_with_source`'s analyzer call and `PhysicsRegistry.analyzer_error`.
- `crates/pilot-analysis/src/pipeline.rs`, `crates/pilot-analysis/src/lib.rs`: drop the analyzer from `AnalysisPipeline` and the re-exports.
- About 10 `crates/pilot-analysis/tests/milestone*.rs` files call `AnalysisPipeline`, `analyze_physics` or `run_pilot_analyzer`; each is itemized when this is sequenced, or the deletion waits for `PROP-004`, which retires that crate.
  - **Itemized at build time (2026-11-17)**, the ten test files that call them: `crates/pilot-analysis/tests/{milestone14_perceus_foreign_resource_and_scope_guards,milestone50_lossless_bridge_and_reflection,milestone54_perceus_ownership_and_escape,milestone56_layout_driven_codegen_and_gpu_backends,milestone58_runtime_crypto_wire_db_and_polyglot,milestone59_persistent_pilvm_lsp_and_dap,milestone60_selfhost_cli_pkg_perf_and_bench,milestone61_stage3_fixed_point_and_universal_rename_gate,phase1_ironclad_killswitch,phase1_rename_torture_gate}.rs`. Only the test functions and helpers that run the analyzer are deleted; the rest of each file stays.
- Expected size: about `-80` lines in `crates/pilot-cli` and `-60` in `crates/pilot-analysis`; `paused_max_total_rust_loc()` goes down.

### 4. To verify before approval
- `AnalysisPipeline::analyze` also passes on the Rust parser's diagnostics (`arena.diagnostics`). `run.rs` `compile_entry` already reports those and stops (`parse_diags`), so `pilot run` loses nothing; `pilot check`'s path must be shown to do the same, with a test (a syntax error under `pilot check`) run before and after.
- `pilot check selfhost/cli/main.pilot` gives the same per-code counts before and after.

### 5. As built (2026-11-17)
- **Pilot:** `selfhost/analyzer.pilot` and `selfhost/.pilc-cache/analyzer.pilc` deleted. `selfhost/cli/commands.pilot`: `semantic_diags` and `report_pending_diags` take the checker's diagnostics alone; `in_source_order` now only sorts that list into source order (it used to merge the analyzer's in too, and `cli_f4` `unused_and_shadowed_bindings_come_from_scoped_resolution` pins the order). `selfhost/cli/host.pilot`: `host_analyze` deleted; `host_pending_diags` gives one list (12 → 11 raw calls, ratchet lowered). `selfhost/gate/manifest.pilot`: the analyzer's paths leave `GM_ALLOWED_INFER_STRINGS`; `paused_max_total_rust_loc()` 27,585 → 27,290 (measured).
- **Rust `src`, the approved files only (−215 lines):** `pilot-cli/src/run.rs` (`analyze`, `ANALYZER_FAULT`; −57), `pilot-cli/src/host.rs` (`cli_analyze`; `report_pending(checked)`, `cli_pending_diags` hands one list; −4), `pilot-analysis/src/infer.rs` (`load_or_compile_analyzer_pilc`, `run_pilot_analyzer`, `analyze_physics_with_source`, `analyze_physics`, `PhysicsRegistry.analyzer_error`; −82), `pilot-analysis/src/pipeline.rs` deleted (its only `src` caller was `run::analyze`; −69), `pilot-analysis/src/lib.rs` (re-exports; −3).
- **Rust tests (the ten files itemized in §3; −4,807 lines):** every test function and helper that reached the analyzer API was deleted, with the imports and helpers that became unused. Whole files deleted: `milestone54`, `milestone56`, `phase1_rename_torture_gate` (no test left). Tests deleted: `test_m14_3`; `test_50_1`, `test_50_3`, `test_50_4_and_50_5`; `test_58_1`; `test_59_1`, `test_59_3_and_59_4`; `test_60_2`, `test_60_3_and_60_4`; `test_61_3_and_61_4`; ten of eleven `phase1_ironclad_killswitch` tests (only `test_diagnostic_catalogs_parsed_cleanly` stays). What they checked now runs as conformance programs in `selfhost/check` (`lint_*`), or was the analyzer's layout binding, which nothing in `pilot` read.
- **§4 checks:** a SYN001 under `pilot check`, `pilot run`, `pilot ir run` and `pilot emit c` prints the same before and after (parser errors come from `cli_check`, which runs before the checker). `pilot check selfhost/cli/main.pilot` gives the same count for every code except ENGINE001 27 → 26 (the deleted `host_analyze`).
- **Verified:** VM 703 passed (19 known bugs reproduced); IR and C 695 passed, 0 failed, 1 seam each (`map_declared_property`); `pilot gate --full` 6 passed, `bootstrap_parity` at its seam; `cargo fmt --check` clean; `cargo test -p pilot-analysis --no-run` 0 warnings. `milestone50`'s `FileAuditRule` for `selfhost/analyzer.pilot` and its contract comments were removed (an approved file).
- **`cargo test -p pilot-cli --no-fail-fast`:** 89 passed, 5 failed; the same 5 fail on HEAD `b9ba25f` built in a separate worktree, so none comes from this change: `cli_emit` `a_wrong_emitted_output_is_reported` (the runner's `[n/m] PASS` progress lines), `cli_f3` `every_listed_command_works_or_says_it_is_not_implemented` (`pilot fmt`/`pilot lex` print usage) and `test_runner_runs_marked_functions_only`, `cli_f4` `a_role_gives_its_lattice` (GRAM011 on `op []`/`op []=` with `_index`), `cli_keyword_matrix` `the_matrix_is_regenerated_from_the_repository`.
- **Not yet green: `cargo test -p pilot-analysis` fails everywhere until Amendment A1.** `crates/pilot-analysis/tests/common/anticheat.rs` `MILESTONE_TEST_MANIFEST` holds a minimum `#[test]` count per milestone file, and every guarded test checks it, so the deleted analyzer tests make all 130 guarded tests panic (`PASS 3: DELETED_TEST_FN`). `milestone27` `test_m27_4` also reads the deleted `selfhost/analyzer.pilot`. Neither file is on §3's list.
- **Amendment A1 (APPROVED BY OWNER 2026-11-18: "approve"; BUILT 2026-11-18). Strictly these files:**
  - `crates/pilot-analysis/tests/common/anticheat.rs`: in `MILESTONE_TEST_MANIFEST`, `milestone14` 4 → 3, `milestone50` 4 → 1, `milestone58` 7 → 6, `milestone59` 3 → 1, `milestone60` 3 → 1, `milestone61` 3 → 2; delete the `milestone54` and `milestone56` rows (files deleted). Nothing else in the file changes.
  - `crates/pilot-analysis/tests/milestone27_scoped_user_sublanguages_and_hygiene.rs`: in `test_m27_4_zero_hardcoded_keywords_in_analyzer_and_cst_recovery`, delete step 1 (the `selfhost/analyzer.pilot` audit); step 2 (`cst_recovery.pilot`) stays.
  - Then `cargo test -p pilot-analysis --no-fail-fast`, reporting every failure that was already failing before (`milestone01 test_m1_3`, `milestone02 test_m2_3`, `milestone57 test_57_1`, `milestone58`, `milestone60`) apart from any new one.
- **A1 as built (2026-11-18):** the six counts and two row deletions above, and step 1 of `test_m27_4`. Each of the six test files also carries its own `expected_test_count`, which the guard asserts equals the manifest (`PASS 3: TEST_COUNT_DRIFT`); those six lines changed to match (14: 3, 50: 1, 58: 6, 59: 1, 60: 1, 61: 2). The six files were already on `PROP-035` §3's list.
- **Dead after this, APPROVED FOR DELETION 2026-11-18 (owner: "approve") AND DELETED (AGENTS.md rule 4):** `crates/pilot-analysis/src/diagnostics.rs` (55 lines: `Diagnostic`, `DiagnosticSeverity`, `AnalysisDiagnostic`) has no user left in `src` or tests; `lib.rs` still re-exports it. Proposed: delete the file and its `pub mod`/`pub use` lines in `crates/pilot-analysis/src/lib.rs`.

---

## `PROP-003` — Wave 3, Itemized: Parser Desugarings and VM Helpers That Can Leave Rust Now

**Status: itemized 2026-11-15. Slices 3A, 3B and 3C APPROVED by the owner 2026-11-15 ("Approve 3A–3C"). 3A and 3B BUILT 2026-11-15; 3C DEFERRED TO `PROP-005` BY THE OWNER 2026-11-15 (larger than itemized; see below); 3D–3F still drafts.** Owner asked to start PROP-003 ("lets do prop-003"). Each slice is approved and built on its own.

### What the survey found (2026-11-15)

The wave summary planned about −2,850 Rust lines by moving `heap.rs`/`ops.rs`/`checked.rs`, `parse_header` and nine parser files to Pilot. **Most of that cannot leave Rust before `PROP-005`/`PROP-006`.** About −250 to −300 can go now, in the slices below.

- `parse_header` is already gone (`PROP-012`). `query.rs` and `with_scope.rs` are already gone (`PROP-009`).
- **Irreducible until `PROP-006`** (the Rust Stage-A parser must still read the stdlib and selfhost, and `pilot-parser` cannot call Pilot code because `pilot-vm` depends on it):
  - `fstring.rs` (330): selfhost's ~923 f-strings are parsed and lowered by it. A `syntax` rule cannot split a string token, and `selfhost/parser.pilot:247` drops interpolation, so there is no Pilot front end to hand it to.
  - `attr_hooks.rs` (133): `[Infix]`/`[Prefix]`/`[Layout]` and every bare-name genesis call reach the bootstrap tables only through it. It is how the Rust Pratt parser learns the operators it needs to parse the stdlib.
  - `pilot_impls.rs` (286): the data channel from `bind_*` hooks to the Rust parser's desugarings and to `pilot-vm`. `selfhost/check` and `selfhost/ir` already re-derive the same bindings in Pilot; the Rust copy goes with `PROP-005`/`006`.
- **Blocked by `PROP-005`** (the names are emitted by `lower_*.rs`, so the Rust arms cannot go while the seed lowering exists): the domain helpers in `heap.rs` (`display`, `value_clone`, `collect_elements`, `contains`, `slice`, `sorted_map_keys`, `char_floor`; about 103 lines), and `ops::fits`/`num_fits` (only `lower_members.rs` emits it, so it dies with that file for free).
- **Irreducible VM core (stays):** the `ops.rs` ALU (`arith`, `bitwise`, `compare`, `equals`, `unary`, `truth`, about 291 lines) and the `heap.rs` slot primitives (about 165). The casts (`wrap`, `convert`, `scalar_from`, 85) need char-construct, float-truncate and f32-round primitives before they can move.
- **Needs its own design:** `comptime_if.rs` (245). It folds the condition and skips the untaken branch while parsing; no rule can do that. The alternative (parse both branches, let `selfhost/check` choose and hand the seed a run-plan row) changes semantics (the untaken branch gets parsed) and costs about +30 to +60 Rust of plumbing for about −245. To be a separate proposal, together with closing the seam `comptime_if.condition`.

### Slices

| Slice | What | Rust files | Pilot files | Net Rust | Risk |
| :--- | :--- | :--- | :--- | ---: | :--- |
| **3A** | Struct `invariant` becomes a `RejectWord` `syntax` rule | `crates/pilot-parser/src/contracts.rs`, `decl.rs` | `stdlib/contracts/contracts_syntax.pilot` | about −25 | low |
| **3B** | Delete the dead Pilot f-string desugarer `stdlib/core/syntax/fstring.pilot` (230 Pilot lines) | `crates/pilot-parser/src/tables.rs` (one `BOOTSTRAP_SOURCES` line); tests `crates/pilot-analysis/tests/milestone52_pilot_preprocessor_and_receptors.rs`, `milestone61_stage3_fixed_point_and_universal_rename_gate.rs` | delete the file; `pilot.md` | −1 `src` | low |
| **3C** | `op_checked` moves to Pilot: new `selfhost/ir/interp_ops.pilot` pre-checks the seed's failures and maps the interpreter's engine-op names to static calls; `binary_op` moves into `vm.rs` (it is dispatch) | `crates/pilot-vm/src/checked.rs` (deleted), `vm.rs`, `stage_a_natives.rs`, `lib.rs` | `selfhost/ir/{interp_ops,fault,interp}.pilot`; `manifest.pilot` ratchet (`fault.pilot` 3 → 2) | about −70 | medium |
| **3D** | `requires`/`ensures`/`old` become `syntax` rules | `crates/pilot-parser/src/{contracts,decl,forms,rule_arms,loud}.rs`, `lib.rs` | `stdlib/core/syntax/forms.pilot`, `contracts_syntax.pilot`, `selfhost/check/contract_clauses.pilot` | about −55 | medium |
| **3E** | `fstring.rs` reuses the lexer's hole scanner instead of its own copy | `crates/pilot-parser/src/fstring.rs`, `crates/pilot-lexer/src/simd.rs` | — | about −50 | medium |
| **3F** | Small trims | `loud.rs`, `num_suffix.rs`, `attr_hooks.rs`, `tables.rs` | `selfhost/check/*` | about −70 | low–medium |

**Slice details.**

- **3A.**
  - Add to `contracts_syntax.pilot` two rules: `invariant ( $c:Expr )` and `invariant ( )`. Each expands to `form(CoreForm::RejectWord, "<the SEAM001 contracts.invariant message today>", ..)`. Use an `Expr` hole: `Rest` cannot start at a prefix dot (`.balance`).
  - Delete `bind_receptor(nameof(invariant), 33, 20)`.
  - Rust deleted: `at_invariant`, `parse_invariant`, `INVARIANT_SUB`, `INVARIANT_UNCONNECTED`, and the check in `decl.rs` `parse_item`.
  - Acceptance: `contracts/invariant_seam` unchanged (SEAM001 at 5:5, from the word); the seed suite, `--ir`, `--emit c` and `pilot gate` unchanged.
- **3B.**
  - The file is never called. It is read on every bootstrap but has no hooks. It also diverges from the real f-string behaviour (no `:pattern`, raw or triple-quoted forms; it lowers holes to text).
  - Deleting it is required by AGENTS.md (dead Pilot code), but its references are a Rust `src` line and two Rust test files, so it needs this approval.
  - A Pilot f-string front end for `PROP-006` is written fresh against `selfhost/parser.pilot`, not from this file.
- **3C.**
  - `op_checked` today runs any intrinsic by name and returns the seed's own failure messages. The Pilot side checks the failing cases first, with the same messages: division by zero, type errors, NaN comparison, a non-bool condition, an out-of-bounds list index, pop from an empty list, not-a-record.
  - `num_wrap`/`num_convert` become ordinary Pilot casts.
  - Acceptance: every `err_*` case on `pilot test --ir` keeps its `.stderr`; `pass_const_eval` folds as before; `cli_f5`, `cli_f7`, `cli_attrs`, `cli_changed` unchanged.
  - Risk: message parity. If one failure cannot be matched without a new primitive, the slice stops and reports it.
- **3D.**
  - Needs three small mechanisms, each append-only:
    - a signature-position rule hook in `parse_fn` (about +6; also opens a later route for a function's `where`);
    - `=>` allowed inside a bracketed rule head (about +3);
    - `CoreForm::Requires`, `Ensures` (optional binder) and `Old`, building today's exact nodes (about +18).
  - Then `with_contracts` stays (it wraps the body), and the receptor-33 parsing goes.
  - New checker errors replace what was DECL001: `old(..)` outside `ensures`, and a contract clause that is not a leading statement of a function body.
  - Acceptance: `contracts/*`, the contract `err_*` cases, `err_fn_where_unchecked` and `milestone51` `test_51_3` unchanged on all three engines. Nothing in `pilot-vm`, `selfhost/check` or `selfhost/ir` changes shape.
- **3E.**
  - Delete `is_fstring_prefix` (an exact copy of the lexer's `string_prefix_flags`), `skip_nested_fstring` and `fstring_hole_end`; make the lexer's scanner `pub` and give it the `\"`-in-hole quote mode.
  - This also fixes a latent bug: the lexer and parser scanners disagree on `\"` inside a hole, so `f"{m[\"}\"]}"` is cut at different `}`s.
  - A conformance case for that literal is written first, with its expected output worked by hand.
- **3F.**
  - Move the non-literal parameter default and the `pub`-on-a-local checks from `loud.rs` to `selfhost/check` (about −30). The checker's spans differ from today's pinned columns, so `err_param_default_not_literal` and `err_pub_local` need an approved `.stderr` column edit, or a snapshot addition.
  - Move the `num_suffix.rs` range, fraction and over-64-bit checks to the checker, which can spot a suffix cast by its shared span (about −20; positions unchanged).
  - Merge the three or four copies of the `HookArg` → value mapping in `attr_hooks.rs`/`tables.rs` (about −20).

### As built: 3A and 3B (2026-11-15)

**3A, struct `invariant` (built).**
- `stdlib/contracts/contracts_syntax.pilot`: two rules, `invariant ( $c:Expr )` and `invariant ( )`, each expanding to `form(CoreForm::RejectWord, "<the old SEAM001 message>", ..)`; `bind_receptor(nameof(invariant), 33, 20)` deleted.
- Rust deleted: `INVARIANT_SUB`, `INVARIANT_UNCONNECTED`, `at_invariant`, `parse_invariant` (`contracts.rs`, −24) and the check in `decl.rs` `parse_item` (−4). Only the listed files changed.
- `contracts/invariant_seam` is unchanged on all three engines (SEAM001 at 5:5, reported from the word).
- **Found while building, and fixed in Pilot.** A file that declares a function named like a rule's word turns that rule off (`expand_seq_rule` `file_declares_fn`), so in such a file `invariant(..)` in a struct body fell through to a statement among the struct's members. The parser keeps such a statement, and nothing ever reported it: `print(5);` inside a struct body was silently ignored on every engine, before this change too.
  - `selfhost/check/rules.pilot` `check_type_body_item` now reports SYN001 for an expression statement whose owner (through member groups) is a struct, interface or `ext` body. A `*Bundle` splice (a splat statement, with its `with`/`without`/`coalesce` modifiers) is a member and is not reported.
  - New test `errors/err_type_body_statement` (expected output by hand: SYN001, exit 1), passing on all three engines.

**3B, dead `stdlib/core/syntax/fstring.pilot` (built).**
- The file (230 Pilot lines) is deleted, with its `BOOTSTRAP_SOURCES` line in `tables.rs` (−1 `src`), its audit rule and the f-string section of `test_52_3` in `milestone52`, its entry in `milestone61`'s file list, and its `pilot.md` row.
- `milestone52` passes. `milestone61` `test_61_1` and `test_61_3_and_61_4` fail, but they fail the same way at HEAD (`ddfb872`, checked in a separate worktree): the selfhost frontend returns `null`, and a kernel parse lacks the operator declarations. Not caused by this slice.

**Net Rust (3A + 3B):** −29 `src` lines (contracts.rs −24, decl.rs −4, tables.rs −1); Rust tests −83 net.

### 3C stopped before building (2026-11-15): larger than itemized

Replacing `op_checked` needs three things the itemization did not count:

1. **Telling a value's kind without a new `__engine` call.** The interpreter's values are `any`. Its pre-checks must know whether an operand is an integer, a float, a boolean or a string (for "cannot apply `+` to boolean and integer", "integer division by zero", and similar).
   - The seed compiles `x is T` only from a checker plan row (`lower_expr.rs` `planned_type_test`; with no row it traps), and the toolchain's `any`-typed code gets no rows (`any` is declared nowhere; the toolchain is not checked as user code).
   - The engine's own kind test, `is_kind`, could be reached only by a new `__engine` call, which AGENTS.md forbids, and `fault.pilot` may only go down on the ratchet.
   - Possible way round: take each operand's register class from the PilIR instruction's layout (the interpreter has it) instead of from the value. Not tried.
2. **About 40 engine operations dispatched by name** (`interp_engine_ops()`: `str_*`, `list_*`, `map_*`, `to_str`, `scalar_from`, `value_clone`, `rt_print`, ...). Each needs a static Pilot call with the seed's failure behaviour.
3. **`host_op` with any of the 51 frozen `HostOp` IDs.** The interpreter passes raw IDs and arguments through. A Pilot replacement needs a 51-entry table onto the `[EngineOp]` owners' public methods, whose signatures and marshalling are not the raw op's (`GATE021` lets only the public methods call the owners).

The pinned messages are few (about 12 `.stderr` files carry seed runtime messages). But items 1–3 are several hundred Pilot lines with fidelity risk for every unpinned failure, for −70 Rust.

**Owner decision 2026-11-15: option (b), defer 3C to `PROP-005`** ("Defer 3C to PROP-005"). `checked.rs`, the `op_checked` arm and `fault.pilot`'s `__engine("op_checked", ..)` stay until then.

**Options that were offered:**
- **(a) Build it as above**: layout-based kind checks, a by-name table for the 40 operations, a 51-entry host-op table. Several hundred Pilot lines; −70 Rust.
- **(b) Defer 3C to `PROP-005`.** When PilIR becomes the `.pilc` path, the interpreter no longer runs on the seed's values, and `op_checked` can go with the seed lowering.
- **(c) Narrow it**: move only the binary-operator and condition pre-checks to Pilot (the IR interpreter's operator faults) and keep `op_checked` for engine operations. Deletes little Rust (`BINARY`, `CONDITION`, about −30) but no `__engine` site.

### Order and total

- Suggested order: **3A, 3B** (smallest, sure), then **3C**, then **3D**, **3E**, **3F**.
- Total about −270 Rust `src` lines and −230 dead Pilot lines, against the −2,850 the wave summary planned. The rest moves to `PROP-005`/`PROP-006`, or to a separate `comptime if` proposal.

---

## `PROP-005` — PilIR Becomes the Bytecode Path: Replace the Rust `BytecodeCompiler` With `selfhost/vm/ir_to_pilc.pilot`

**Status: itemized 2026-11-15 (owner: "develop 005 then"). Phase 5A APPROVED by the owner 2026-11-15 ("Approve 5A") and BUILT (Pilot only, no Rust touched). 5B–5F not approved.** The phases below are built and approved one at a time.

### Scope (what is deleted at the end)
- `crates/pilot-vm/src/lower_*.rs` (24 files), `vm_gen.rs`, `codes.rs`, and `BytecodeCompiler` in `crates/pilot-vm/src/lib.rs` (lines 60–1015). Measured 2026-11-15: 6,999 lines in the 26 files plus about 955 in `lib.rs`, so about −7,950 (the summary below says −7,833).
- `vm_gen.rs` is VM runtime, not compiler code: generator frames are reached through the `gen_intrinsic` arm in `stage_a_natives.rs:21-33`. It goes together with that arm, because PilIR lowers generators to state machines (`selfhost/ir/lower_generator.pilot`) and emitted code never calls `gen_*`.
- `codes.rs` is also used by the CLI (`run.rs` reads `codes::SYNTAX`). Its constants need a new home before it is deleted.
- Inherited from `PROP-003` slice 3C: `crates/pilot-vm/src/checked.rs`, the `op_checked` arm, and `selfhost/ir/fault.pilot:96` `__engine("op_checked", ..)`.
- Inherited from `PROP-003`'s blocked list: the domain helpers in `heap.rs` (about 103 lines) and `ops::fits`/`num_fits`.
- The temporary `PROP-007` seed inlining in `lower_call.rs`.

### The bootstrap problem this wave has to solve (owner decision needed before phase 5D)
Today every `pilot` invocation compiles the toolchain itself, meaning the `CLI` list in `crates/pilot-cli/src/main.rs` and the `link_tool` lists, with the Rust `BytecodeCompiler`. Once that compiler is deleted, something else has to turn the toolchain into bytecode, and the only Pilot path (snapshot → `selfhost/check` → `selfhost/ir` → `ir_to_pilc`) is itself toolchain code. There are two ways out:
- **(T1) Checked-in `toolchain.pilc`, produced by the Pilot path, pulled forward from `PROP-006`.** The last Rust-compiled toolchain runs the Pilot path once to write `toolchain.pilc`. That file then rebuilds itself until stage 2 equals stage 3 (Gate 2), and the VM boots it directly. The Rust parser stays (the checker still reads `chk_snapshot`) until `PROP-006`.
- **(T2) Keep the Rust compiler for the toolchain only; route user programs through PilIR.** This deletes nothing until T1 lands anyway, so it is only a staging step.
- **Decided 2026-11-18: T1** (owner: "T1: toolchain.pilc"). It is the only way to reach the deletion. `PROP-006` then shrinks to deleting the Rust lexer, parser and AST once the Pilot parser feeds the checker.

### Findings 2026-11-15 (scratch copy, nothing kept)
Can the Pilot pipeline read the toolchain at all? `pilot check selfhost/ir/main.pilot` was run on a scratch copy:
1. **Package names do not resolve.** Toolchain files declare `pkg Pilot.SelfHost.X.Y` but import `use selfhost.x.y`. 24 of the 228 selfhost files declare no package. The loader's search roots (`mg_default_search_roots`: the entry directory and `<toolchain>/stdlib`) do not include `selfhost/`. Result: 12 MOD001.
2. **One package per file gives genuine cycles.** With path-named packages per file there are 78 MOD002 cross-file cycles, e.g. `lower` ↔ `lower_members` ↔ `lower_variant`.
3. **One package per directory gives no cycles between directories.** The directory-level import graph has 0 strongly connected components larger than 1. The loader, however, treats every file as a graph node, and it reports a package's files using each other, or a file importing its own package, as cycles (8,952 MOD002). Sibling files are also not loaded unless something imports them.
4. With a two-part scratch loader change, the toolchain gets through the loader: load a package's sibling files with any of its files, and treat same-package edges as non-edges in the topological visit (about 12 lines in `selfhost/loader/module_graph.pilot`). Two more scratch fixes were needed: drop self-imports, and remove one trailing comma in a parameter list (`selfhost/types/delegation.pilot:84`, SYN001; that file is reachable only from a wiring row).
5. **Next blocker: spec-only stdlib core files.** The toolchain's `use Pilot.Core.*` imports pull in core files such as `stdlib/core/mem.pilot` (`let RawStorageBundle<T> = { .. }` bundle literals, SEED001 `ParamDecl` not lowered). They are pulled through `task`, `meta`, `primitives/{stride,pairs,expand,str,map_view}` and `pilot/abi/aapcs64`. The CLI today links a hand-picked list that avoids them.
6. **Not yet measured:** the checker's type errors on the toolchain (behind 5), and whether PilIR lowers all of it. The toolchain also uses `any`, which nothing declares; it has never been checked as user code, so the checker plans no `is` rows for it.

### Phases
- **5A — The toolchain loads through the Pilot loader (Pilot only, no Rust).**
  - Packages become one per directory, `selfhost.<dir>`, declared in every selfhost file.
  - Imports are rewritten to `use selfhost.<dir>.*;` and self-imports are dropped.
  - `mg_default_search_roots` adds the toolchain root, so `selfhost.*` resolves.
  - The loader loads a package's files together and does not count same-package edges as cycles. A cycle between packages stays MOD002.
  - Each toolchain file states the stdlib core imports it really uses, in place of the flat link list.
  - The spec-only core files the toolchain reaches are dealt with, not left in the import graph: given seams, cut from the imports, or made to compile.
  - Done when `pilot check selfhost/cli/main.pilot` gets past the loader with 0 MOD001/MOD002, plus new conformance tests under `tests/conformance/modules/`: a package whose files use each other runs; a real cycle between packages is still `err_*` MOD002.
  - The Rust `CLI` list is untouched in 5A.
  - **As built 2026-11-15:**
    - **Package names.** All 228 selfhost files declare `selfhost.<dir>`; the top-level files are `selfhost`. 204 old names were replaced (`Pilot.SelfHost.*`, `Pilot.Diagnostics`, `Pilot.Loader`) and 24 declarations were added. Imports are collapsed to `use selfhost.<dir>.*;`, with self-imports dropped. `stdlib/pilot/chor.pilot` follows the rename. `selfhost/loader/ring_verifier.pilot` classifies `selfhost.*` as the SelfHost ring.
    - **Loader** (`selfhost/loader/module_graph.pilot`, 562 → 635 lines):
      - `selfhost` is the third search root.
      - Roots are indexed lazily, in order, only as far as a lookup needs. A package under an earlier root hides one with the same name under a later root.
      - A package is every file declaring it under one root: those files load together and are visited before the file that pulled them in, so the entry stays last. A file importing its own package makes no edge.
      - MOD002 names a cycle by package, each package once.
      - Without the lazy indexing, `pilot run` of a one-line program went from 0.39 s to 0.50 s; with it, it stays at 0.40 s.
    - **Fixes found on the way:**
      - A trailing comma in a parameter list (`selfhost/types/delegation.pilot:84`).
      - Dead imports of the spec-only `Pilot.Core.Memory`/`Pilot.Core.Meta` in `stdlib/core/reflect.pilot`, `selfhost/lsp/handlers.pilot` and `selfhost/ide/vscode_gen.pilot`; they used none of their names. This kept `mem.pilot`, `builtins.pilot` and `type_construct.pilot` (SEED001) out of the toolchain's graph.
      - `stdlib/crypto/hash.pilot` declared no package, so the gate's `use stdlib.crypto.hash.*` never resolved. It now declares `std.crypto.hash`.
    - **Tests:** `tests/conformance/modules/package_peer_files` (fails on the previous loader with a self-import MOD002) and `err_package_cycle`.
    - **Result:** `pilot check` from one entry in each of the 21 selfhost directories (`selfhost/cli/main.pilot` plus 20) reports 0 MOD001/MOD002/SYN/SEED.
    - **The 5B baseline:** `selfhost/cli/main.pilot` gives about 7,500 checker errors: OWN001 6,240, TYPE001 640, TYPE002 219, ENGINE001 171, OWN004 108, MOD003 68, COMP005 51, PURE001 43, and smaller counts. `selfhost/vm/compiler.pilot` calls a function that no longer exists (`vmc_add_int_const`, DECL001).
- **5B — The toolchain type-checks as user code (Pilot only).**
  - Declare `any` (or replace its uses) so `is` tests get planned rows.
  - Fix every checker error the toolchain shows, or file it as a `known_bugs` entry with its true output.
  - Done when `pilot check` on each toolchain entry (`selfhost/cli/main.pilot` plus the `link_tool` entries) reports 0 errors.
- **5B itemized 2026-11-15 (owner: "Itemize 5B"). Not approved.**
  - **Baseline.** `pilot check` was run from the 21 directory entries and the results deduplicated by location: about 13,160 diagnostics, 12,097 of them errors in files the real commands link.
    - A file counts as "live" when it is named in a link list: `crates/pilot-cli/src`, `selfhost/cli`, `selfhost/loader` or `selfhost/gate`. That is 155 of the 228 selfhost files, plus the stdlib files they link.
    - The other 73 files are not linked by any command. 70 of them are used only by `crates/pilot-analysis/tests/milestone*.rs`, which is `PROP-004`'s retirement list; they have 825 errors. The other three are `cli/deps_cmd.pilot`, `pkg/version.pilot` (both reached only through imports) and `diagnostics/errors.pilot` (read by `crates/pilot-vm/src/codes.rs`).
  - **Live errors by class:**

    | Class | Count | Cause |
    |---|---|---|
    | OWN001 | 6,583 | Shared context handles passed into plain parameters, which move them: `lw` 1,975, `s` 1,833, `p` 767, `f` 270, `inf` 171, `sink` 127, ... |
    | ENGINE001 | 2,366 | Raw `__engine` calls in 50 legacy files |
    | SEAM001 | 1,043 | The same raw `__engine` sites, read as calls to a function with no body |
    | TYPE001 | 1,155 | Mostly `i64` passed where `usize` is expected: `str.slice` 594, `str.byte_at` 521 |
    | MOD003 | 268 | 145 are `__engine` again; 53 are a checker bug: a loop variable `op` (`selfhost/ir/interp.pilot:784`) resolves to another package's private `op` instead of shadowing it; the rest are items that now cross a directory package without `pub` |
    | TYPE002 | 238 | `any` 199 (declared nowhere); `ref` 89 (parameters written `x: ref<T>`; the tested form is `ref x: T`, `memory/moves_clean`) |
    | METH001 | 151 | All in `analyzer.pilot` |
    | OWN004 | 129 | |
    | PURE001 | 75 | `cli/spec.pilot` and `gate/seed_audit.pilot` |
    | COMP005 | 48 | `: IValue` structs with fields that borrow |
    | ESCAPE002 | 19 | |
    | NULL005/006 | 17 | |
    | Other | 5 | TYPE003, FLOW005, DECL001, DECL004 |

  - **Owner decisions needed before the slices they gate:**
    - **D1 (gates 5B.1): may a plain `ref` parameter change what it borrows?**
      - Today the checker allows `c.items.push(v)` and `c.n = ..` through a `ref` parameter. Only rebinding the parameter is DECL004, whose message calls `ref` "a read-only borrow". The toolchain changes `lw`, `sink`, `out` and others through their parameters.
      - (a) `ref` may change fields; the toolchain uses `ref` everywhere.
      - (b) `ref` is read-only; changing needs `var ref`. A new checker rule enforces this, and parameters that change are marked `var ref`. On the seed, `var ref` adds a write-back on return.
      - **Recommendation: (b).** It matches the DECL004 wording and the `var ref` keyword's own description.
    - **D2 (gates 5B.2): the index type of `str`.**
      - (a) `str.slice`/`byte_at` take `i64`, matching the toolchain and what `len()` gives in practice.
      - (b) The 1,115 call sites cast.
      - (c) The checker accepts `i64` for `usize`.
      - **Recommendation: (a).** (c) would hide real sign errors.
    - **D3 (gates 5B.3): `any`.** It is used 202 times in 34 live files: `List<any>` 145, `Map<any, any>` 15, bare `any` 31.
      - (a) Declare `any` as a `[Lang]` top type, with `is` tests, a keyword-matrix row and a passing and an `err_*` program.
      - (b) Replace each use with a concrete type or a record. The `List<any>` lists handed back to the Rust host (for example `mg_link_plan`) become typed records the host reads by field.
      - **Recommendation: (b)** for the toolchain, plus a separate proposal if users need a top type.
    - **D4 (gates 5B.4): the legacy files that call `__engine` directly and are still linked.**
      - `analyzer.pilot` (393 calls; linked by `crates/pilot-cli/src/run.rs` on every `pilot run` for the `host_analyze` diagnostics).
      - The gate's `lexer`/`ast`/`parser`/`pilotc` (64 + 101 + 178 + ...; the gate links them for `bootstrap_parity`, which stops at a seam anyway).
      - `cli/host.pilot` 12, `gate/*` about 80, `ir/fault.pilot` 2 (`op_checked`, which stays until 5F), `check/snapshot.pilot` 1 (`chk_snapshot`, which needs an owner method).
      - For each file, either rewrite it onto owner types (`Map`/`List`/`File` methods and typed structs; this is `PROP-003` bucket B), or delete it when its work is duplicated. The first thing to check is whether `analyzer.pilot`'s diagnostics are already produced by `selfhost/check`.
      - Removing a file from the Rust link list (`run.rs`) is a Rust edit and needs its own proposal.
    - **D5: the 70 harness-only files.** Either wait for `PROP-004` to delete them along with their Rust harnesses, or leave them outside 5B's done condition. **Recommendation:** leave them out of 5B and itemize `PROP-004` next.
  - **Slices** (each approved and built on its own; after each one, `pilot test` on all three engines, `pilot gate --full`, and the error count from every live entry):
    - **5B.1 Parameter borrowing (about 6,750: OWN001, OWN004, ESCAPE002, DECL004).**
      - A Pilot codemod run over the live files: every owned-type parameter its function never moves becomes `ref` (or `var ref` if it changes, under D1 (b)).
      - Parameters the function does move are listed and fixed by hand.
      - Under D1 (b), first add the checker rule for changes through a `ref` parameter, with a passing and an `err_*` program.
      - **5B.1 as built (approved 2026-11-15, Pilot only):**
        - **The rule.** `selfhost/check/consts.pilot` reports DECL004 for a change made through a plain `ref` parameter (`readonly_ref_param`). `own.pilot` reads parameter words through `flow_param_flags`/`by_value`/`may_change_param`, and an expression body (`=> b`) moves its value when the return type owns. `aliasing.pilot` no longer counts a `0b00` value passed by value as a second way into a `var ref` argument (`copied_in`): it is copied before the call. Tests: `errors/err_ref_param_change`, `memory/var_ref_change`, `memory/var_ref_copied_arg` (all three engines); `memory/drop_element_replace` now uses `var ref`.
        - **The codemod.** `pilot check --fix` runs `selfhost/check/fix_borrows.pilot`. Every plain owned parameter of a free function in the user's files starts as `ref`; those that still move their value go back to by value, in rounds until none does. This makes functions that call each other in a cycle borrow together. Then `var` is added where the value changes, including on parameters already written `ref`. Only the word is inserted. `kept:` lines show where each by-value parameter really moves. Test: `runtime/check_fix_borrows` (fixture `tests/tool_fixtures/check_fix/borrows.pilot`).
        - **Applied** from `selfhost/cli/main.pilot` and from one file in each selfhost package, until a run changed nothing. Shared context fields (`Walk.p`, `Flow.inf`, `Lowerer.inf`, `IrFrame.m` and others) were made `ref` fields by hand. `ty_subst`, `index_members` and five `ty_named` callers read a name out before lending the type table (ALIAS001).
        - **Result** (`pilot check selfhost/cli/main.pilot`): OWN001 6,240 → 266, OWN004 108 → 31, ESCAPE002 19 → 15, DECL004 0, ALIAS001 3.
        - **Left:**
          - The 266 OWN001 are locals moved and then used again (`p.built.push(bt)` then `bt.decl`, `methods.pilot:322`), not parameters. Each needs the code reordered, or a clone.
          - The 3 ALIAS001 lend a container and one of its elements to the same call (`check/stack.pilot:104`, `ir/lower_global.pilot:429`, `ir/pass_perceus.pilot:67`).
          - The MOD003 local-shadowing bug also hits `words` in `fix_borrows.pilot` (5B.5).
          - `stdlib/core` `file_walk_rec(ref out)` pushes through a `ref` parameter. That breaks D1, but it is not reported because stdlib files are not user files.
          - Returning a `ref` parameter by value is not reported as an escape.
        - **Rust deletion (approved by the owner 2026-11-15, done):** `crates/pilot-cli/tests/cli_f3.rs` `check_fix_stops_at_its_seam` is deleted. It expected the retired `cli.check_fix` seam, and it ran `pilot check --fix` on `tests/conformance/runtime/stdin_lines.pilot`, which could have rewritten that file. `runtime/check_fix_borrows` replaces it. Only that one test function was touched. `cargo test -p pilot-cli --test cli_f3`: 12 passed. `cargo fmt --check` was not run because rustfmt is not installed.
    - **5B.2 Index types (about 1,155 TYPE001).** D2 (a), then fix the remaining width and option mismatches (`emit` u16/u32, `Map.set` keys, `binding_type` options) by hand.
      - **5B.2 as built (approved 2026-11-15, Pilot only):**
        - **Re-measured first** (`pilot check selfhost/cli/main.pilot`, after 5B.1): 1,000 errors, TYPE001 341 (`str.slice` 226, `str.byte_at` 46, the rest widths and options).
        - **D2 (a).** `stdlib/core/primitives/str.pilot`: `byte_at` and `slice` take `i64` and clamp to the text (below 0 reads as 0, past the end as the length; `byte_at` outside gives 0 as before). Tests: `strings/str_i64_index` (hand-worked bytes, all three engines) and `errors/err_str_index_type`. The unused `str_len_pure`, `str_byte_at_pure`, `str_slice_pure`, `str_index_of_pure` and `str_contains_pure` are deleted.
        - **Positions held as `usize` became `i64`:** `diagnostics/renderer.pilot` (every byte position and width), the CLI argument layer (`cli/spec.pilot`, `cli/parser.pilot`, `cli/commands.pilot`, `cli/help_and_comp.pilot`), `check/quote_value.pilot`, `diagnostics/model.pilot` `str_replace`, `cli/test_ir.pilot`. `List.len()` (still `usize`) is cast where it is stored as an `i64` count or index (`methods`, `program`, `resolve`, `snapshot`, `types`).
        - **Bug found:** `check/resolve.pilot` `lookup_position` counted down a `usize` with `while i >= 0`, which is always true; it is `i64` now.
        - **Options:** a value read from a `Map` key just set or tested is read with `m[k]` instead of `m.get(k)`; a value tested against `none` and then used is bound with `if some` / `guard some` (`attr_chain`, `attr_inherit`, `comptime_tool`). Reading `bw.roots[b]` and `p.gen_memo[memo]` showed `binding_roots`/`binding_assignments` handing out a list owned by a map (OWN004, hidden by the option before); they return a copy now.
        - **Dead code deleted:** `selfhost/cli/diff_test.pilot` (no caller anywhere; `format_diff_report` printed a fixed "all backends OK").
        - **Result:** 1,000 → 675 errors; TYPE001 341 → 17, OWN004 18 → 17, nothing else rose. The 17 TYPE001 left are the CLI's `Map` handles declared `i64` (`cli_parse_args`, `cli_tokenize`, `cli_handlers`, `cli_suggest_typo`): the untyped-handle problem of 5B.3.
        - **The rest of the tree.** The other entries (adapters, `chor`, `cst`, loader, `tools/keyword_matrix`, `stdlib/std`) had 177 `as usize` casts inside `slice`/`byte_at` arguments and 690 lines passing `usize` positions. The casts are removed; locals, parameters and returns holding positions became `i64` in 24 files (struct fields declared `usize` were left, except `CppEnv.eval_pos`, a position); `str.index_of` returns `i64` (its two callers in `chor/parser.pilot` passed an `isize` to `slice`); byte sizes passed to `cpp_field_decl_new` stay `usize` with a cast. Checking these packages also showed `opt<str>::some(..)` / `opt<str>::none` in `adapter/c_sysroot.pilot`, which is not Pilot; it is `return cand;` / `return none;` now. After this, no `slice`, `byte_at` or `index_of` call in the tree has a type error.
        - **Width and option pass (2026-11-16, approved "Yes, do them"):** 351 TYPE001 → 52 across `selfhost/ir`, `selfhost/adapter`, `selfhost/loader` and `selfhost/chor`. `selfhost/ir/model.pilot` gained checked narrowings `ir_u16`/`ir_u32` (fault, never truncate, when a value does not fit) and `ir_sub`/`ir_operand` on top; `ir_emit`, `emit`, `emit_void` and `make_inst` take the sub-code as `i64` and narrow it once; pool indexes, span ids and new value ids go through `ir_operand`; `opcode_of` through `ir_u16`. The lowerer's value and slot maps are `Map<str, u32>`. `ir/text.pilot` parses numbers with a range check (`number_u32`/`number_u16`/`number_u8`). Reads of keys just set or tested use `m[k]` (47 sites); byte predicates take `u8` (10 files); `loader/module_graph.pilot` casts unit ids.
        - **Collection counts and indexes are `i64` (owner decision 2026-11-16, "Make them i64", then "All collections"):** extends D2 (a) from `str` positions to every collection. `stdlib/core/operator.pilot`: `IReadOnlyCollection.count` and `IReadOnlyDictionary.count` are `i64`, `IReadOnlyList` is `IIndexGet<i64, T>`, `IWritableList` is `IIndexSet<i64, T>`, `IList.remove_at` takes `i64`. `List` (`len`, `count`, `[]`, `[]=`, `remove_at`, its enumerator), `array`, `slice`, `stride`, `str` (`[]`), `SliceBuffer` (`[]`, `[]=`, `remove_at`, which now also refuses a negative index), `ContiguousViewFacet.count`, `Map.count` and `map.count` follow. Stored layout fields stay `usize` (`slice`/`str`/`SliceBuffer` `len`, `cap`; `byte_len`), since they are the `{ ptr, usize }` layout; the getters convert. `u64` was considered and not taken: it brings back casts against `i64` `str` positions and literals, and an empty list's `len() - 1` would wrap. Tests: `collections/count_index_i64` (hand-worked, all three engines), `errors/err_list_index_type`; `collections/map_declared_property` binds `count` to an `i64`. Callers fixed: `chor/storage_plan` drop columns `List<i64>`, `types/typestate` state indexes `i64`, `adapter/cpp_abi` field loop index, IR operand counts through `ir_operand`, `loader/module_graph` unit id.
        - **Bug found (not fixed, recorded):** inside a `while` or `for` body, the checker reads the left operand of arithmetic as unknown, so `x + 7` takes the literal's `i64` type. Known bug `check.loop_arith_literal` (`tests/conformance/known_bugs/loop_arith_literal.pilot`, true output kept). It accounts for one site left in `chor/storage_plan.pilot`; one in `adapter/cpp_abi.pilot` is the `ref<T>` gap of 5B.3.
        - **Lossless integer widening is implicit (owner decision 2026-11-16, "Allow implicit widening"):** `selfhost/check/program.pilot` `prog_widens` (name-blind, from `[Bits]` class and width) lets an unsigned integer fit a strictly wider unsigned or signed type and a signed integer a strictly wider signed type, in arguments, `let`/`var` annotations, assignments and returns (`rules.pilot` `assignable`). Never signed into unsigned, never the same width, never `bool` or the character type (found by their literal slots). `view_cast.pilot`'s `[Bits]` width reader moved to `program.pilot` as `prog_bits` (shared). Tests: `numbers/implicit_widening` (hand-worked, sign extension included, all three engines) and `errors/err_widen_signed_to_unsigned`, `err_widen_same_width`, `err_widen_bool`. The engines needed no change: each already extends the value correctly.
        - **Result:** TYPE001 in `selfhost/ir`, `adapter`, `loader`, `chor` 351 → 2: one in `adapter/cpp_abi.pilot` (a field read through a `ref<T>` parameter, 5B.3) and one in `chor/storage_plan.pilot` (known bug `check.loop_arith_literal`). The CLI's 17 are its `Map` handles (5B.3). No width or option mismatch is left.
        - **Verified (after the width and option pass, 2026-11-16, all runs memory- and task-capped):** `pilot test` 558 passed, `--ir` 548 passed, `--emit c` 540 passed with the 8 earlier failures (`errdefer`, 7 `err_bridge_*`); `pilot gate` 4/4, `pilot gate --full` 5 plus a seam; `cli_f3` 12/12; keyword matrix regenerated and `--check` clean. `pilot check selfhost/cli/main.pilot` unchanged at 675 errors, TYPE001 17.
    - **5B.3 `any` and `ref<T>` (238 TYPE002).**
      - `x: ref<T>` becomes `ref x: T` (102 sites, mechanical).
      - `any` per D3.
      - **5B.3 progress (approved 2026-11-16, "Approve 5B.3"; Pilot only so far):**
        - **Re-measured first:** 259 TYPE002 from the live entries (`any` 195, `ref` 63, one stale `Snapshot`), plus the CLI's 17 TYPE001 from `Map` handles declared `i64`.
        - **`ref<T>` parameters (done):** a codemod rewrote all 150 `name: ref<T>` / `name: var ref<T>` parameters in `selfhost/`, `stdlib/` and `tools/` to `ref name: T` / `var ref name: T` (48 files, including `stdlib/core/operator.pilot` `:=:` and `stdlib/core/meta.pilot`'s `IComputed`/`IRule` methods); `stdlib/std/serde/derive_bin.pilot` now generates the new form. Left: the three `fn<ref<T>, R>` function-type fields in `stdlib/core/meta.pilot`, which no live entry reaches. No `ref` TYPE002 is left in live files.
        - **CLI argument layer (done):** `selfhost/cli/spec.pilot` holds typed records: `CommandSpec : IObject` with `flags: List<FlagSpec>` and `positionals: List<PositionalSpec>`; `ParsedArgs` with `flag_values: Map<str, List<str>>`, `positionals`/`passthrough_args: List<str>`; `CommandRegistry` with `List<CommandSpec>`, `Map<str, i64>`, `Map<str, str>`; the parse result is a `CliParse` record (`cli_parse_ok`/`cli_parse_error`) instead of a map keyed `"ok"`/`"message"`/...; `cli_handlers()` is a `Map<str, (ParsedArgs, CommandRegistry) => i64>` and all 12 handlers take `(ref p, ref reg)`. A flag's values are read, extended and stored back (`parsed_add_flag`) instead of relying on the seed sharing a list held in a map. Deleted as dead or replaced: the `cli_list_*`/`cli_map_*` helpers, the `ParsedArgs`/`CommandRegistry` methods, `parsed_get_int`, `parsed_get_bool`, `spec_registry_get` (callers index `reg.commands`), `ICliHandler`, `resolve_cli_subcommand`. `pilot check selfhost/cli/main.pilot`: 675 → 443 errors; the CLI's TYPE001 17 → 0. Usage errors print the same messages with exit code 2 (checked by hand: unknown command with suggestion, unknown flag, missing value, bad choice).
        - **Left in 5B.3: 166 `any` in live files.** 56 are the PilIR interpreter's runtime values (`selfhost/ir/interp.pilot`); about 110 are rows exchanged with the Rust host. Fixed-shape host rows can be typed as tuples with no Rust change (the seed keeps a tuple as a list and reads `.0` by position): diagnostic rows `(str, str, i64, i64, i64)`, attribute rows `(i64, i64, List<i64>)`, file rows `(str, bool, i64, str, bool)`. Two parts need a decision:
          - **The run plan's drop rows** mix two shapes in one list, which `crates/pilot-cli/src/check.rs` `decode_plan` reads from `parts[0]`: ordinary rows `(action, start, end, kind, a, b)` (ints at 1 and 2) and `Glue`/`CopyGlue` rows `(action, key, owner, [fields], elem, len_op, get_op, seq_op, payload, absent_op)` (strings at 1 and 2). No single type fits both.
          - **The interpreter's values** need a value type. A recursive payload enum (`enum IrVal { Int(v: i64), Float(v: f64), Text(v: str), Rec(items: List<IrVal>), .. }`) checks and runs on the seed and on PilIR (probe, 2026-11-16).
        - **Rust amendment 5B.3a (APPROVED AND BUILT 2026-11-16):** in `crates/pilot-cli/src/check.rs` only, `decode_plan` reads `Glue`/`CopyGlue` rows from a third plan part (`parts[2]`) instead of picking them out of `parts[0]`; `selfhost/check/drops.pilot` and `selfhost/check/main.pilot` put glue rows in that list. Each list then has one row type (`DropRow`, `GlueRow` tuples). Expected size: about the same number of Rust lines (the loop moves; no new logic), so `paused_max_total_rust_loc()` holds flat. Verification: `cargo fmt --check`, `cargo test -p pilot-cli`, the three engines, `pilot gate --full`.
          - **Built:** `decode_plan` walks plan parts 0 and 2 (`crates/pilot-cli/src/check.rs` stays at 697 lines). In Pilot, `selfhost/check/drops.pilot` declares `DropRow`, `GlueRow`, `AtRow` and `FieldRow` tuple aliases; `DropPlanner.rows: List<DropRow>`, `glue: List<GlueRow>`, `at: Map<str, List<AtRow>>` (written by read, push, store back instead of relying on a shared handle); `stored_fields -> List<FieldRow>`; glue and copy-glue rows go to `pl.glue` (copy-glue padded to ten); the five other row writers (receivers, numeric, type tests, view casts, quotes) take `List<DropRow>`; `selfhost/ir/lower_drop.pilot` and `Lowerer.releases` read `AtRow`. `plan_of` returns `[rows, probes, glue]`. Dead `drop_plan()` in `selfhost/check/main.pilot` deleted (Rust evaluates `run_plan();`).
        - **Host rows typed (2026-11-17, Pilot only):** the snapshot's attribute and file rows are `AttrRow = (i64, i64, List<i64>)` and `FileRow = (str, bool, i64, str, bool)` (`selfhost/check/snapshot.pilot`, read by `.0`..`.4` in eight files); diagnostics are `DiagRow = (str, str, i64, i64, i64)`; the run plan is `RunPlan = (List<DropRow>, List<ProbeRow>, List<GlueRow>)`; `check_with_steps` gives `(List<DiagRow>, RunPlan?, List<List<str>>?)`; `selfhost/cli/host.pilot` reads `HostDiagRow` tuples and step rows `List<List<str>>?`. Inside Pilot a probe is a `PlannedProbe` record with `ProbeArg` arguments (`selfhost/check/probes.pilot`), read by PilIR (`selfhost/ir/lower_probe.pilot`, deferred calls in `Lowerer.later_probes`) and turned into the host's `ProbeRow` only in `plan_of`. Flags-enum rows are `(str, i64)`, coverage entries `(i64, bool)`, size parameters `(i64, str)`, splice rows `(i64, str, str)`, `Program.built` is `List<BuiltType>` (and `build_type` stores the finished type back instead of relying on the list sharing it). `pilot check selfhost/cli/main.pilot`: TYPE002 69 → 0, all errors 501 → 409.
        - **Rust amendment 5B.3b (APPROVED 2026-11-17, owner: "Approve 5B.3b"; BUILT):** a probe argument is the tuple `(code, text, flag)`, one type for every argument kind. Today `crates/pilot-cli/src/check.rs` `planned_probe` reads both the text and the flag from slot 1 (`pair.get(1)`), which a single tuple type cannot fill (a text in one row, a `bool` in another). The change, in `crates/pilot-cli/src/check.rs` only: `let flag = matches!(pair.get(2), Some(HostValue::Bool(true)));` (slot 2 instead of slot 1). One line changed, net `0` Rust lines; `paused_max_total_rust_loc()` holds. Without it every `Flag` argument reads as false on the seed (`[Debug(changed_only: true)]` and the before-value of an assignment would stop showing), so the Pilot change above is not finished until it lands. Verification: `cargo fmt --check`, `cargo test -p pilot-cli`, the `[Debug]`/`[Coverage]` conformance programs on three engines, `pilot gate --full`.
        - **PilIR interpreter values typed (2026-11-17, owner: "Build IrVal + Pilot ops now"; Pilot only):**
          - **Values:** a value is `enum IrVal { Nil, Bool, Int, UInt, Flt, Chr, Text, Seq, Rec }` (`selfhost/ir/interp_value.pilot`). Lists and records hold the seed's own `List<IrVal>` / `Map<str, IrVal>`, so they are shared handles as before and the seed's collector frees them. This replaces the planned index heap, which would never shrink.
          - **Operators and conversions:** `selfhost/ir/interp_ops.pilot` (`enum IrOp`).
          - **Engine operations:** `selfhost/ir/interp_engine.pilot`, a typed `Map<str, IrEngineOp>`. They are Pilot code following the seed's rules and messages (`seed_ops_spec`, worked from `crates/pilot-vm`). Handler and preparer tables are `List<IrStepFn>`; `ir_eval` gives `IrEvalResult`.
          - **`any` and `__engine`:** `any` is left only at the host boundary `host_checked` in `selfhost/ir/fault.pilot`, where values cross to the seed's untyped `op_checked` for the raw-memory host operations. `fresh_copy`, the `__engine("value_clone")` site, is deleted, and the ratchet for `fault.pilot` goes 3 → 1. In the interpreter files, `pilot check` errors went 96 → 41, and TYPE002 65 → 4, all 4 at that boundary.
          - **Seed parity fixed along the way:**
            - A fault's report caps its trace at 32 calls and counts the rest; new test `errors/err_deep_trace`, which passes on 3 engines. `selfhost/cli/test_emit.pilot` also leaves the count line unchecked for emitted programs.
            - A place outside every file prints `<unknown>`.
            - The program-args host op returns the argument count, plus `-12` for a missing buffer.
            - `stdin_read_*`, which the seed does not implement, now stop at `ir.interp_hook` instead of giving null.
          - **Gaps kept:**
            - The seed's `<list #N>` slot names print as `<list>`.
            - `str_contains` on a list cannot match the very same nested list or record by handle.
            - The seed's 10,000-frame call limit is not reproduced.
          - **Verification:** 3 engines on `tests/conformance`. VM: 670 passed, 0 failed (13 known bugs reproduced). IR: 660 passed, 0 failed, 3 seams. C: 660 passed, 0 failed, 3 seams.
          - **Speed (user CPU):** a tight arithmetic loop takes 2.6 → 3.2 s, and `control/become_tail` 3.4 → 2.1 s (direct `Int` handlers for `+ - *` and comparisons). The `--ir` lex cases take about 15% longer; they were already over 60 s, and that is the lexer speed work.
        - **Other loose typing (2026-11-17, owner: "find any instances of any or loose typing"):**
          - **stdlib:** `ast<any>` is gone; magic `u8`/`str` codes became enums (`LogLevel`, `JsonKind`, `OpenMode`, `PatchPart`, `AnatomyKind`, `CaseStyle`, `HttpMethod`, `DiagSeverity`, `FixApplicability`, `ForeignLang`, `NamingRule`); the wire decoders return records.
          - **selfhost:** `airlock_abi` tables are tuple aliases; `mg_link_plan` gives `LinkPlan`; comptime step rows are `StepRow` with `enum StepAction`; `File.walk_files`/`list_dirs` give `List<str>`.
          - **Left:**
            - The `glist_*` helpers (mixed lists behind raw `__engine`; 5B.4).
            - Duplicate diagnostic and naming enums in `selfhost/diagnostics/model.pilot` and `selfhost/adapter/rules.pilot`. Rust file lists and milestone tests link those files, so merging them needs a Rust proposal.
        - **Engine bindings typed (2026-11-17, owner: "start the next unimplemented stage or proposal"; Pilot only):** a binding is `EngineBinding { hook: str, args: List<str> }` (`selfhost/check/program.pilot`) instead of a `[hook, args..]` text row; `prog_engine_bindings`, `Program.engine_memo`, `Flow.engine` and `Lowerer.bindings` hold `List<EngineBinding>`, and the 20 readers in `check/{program,quote_value,effects,infer,own,copies}.pilot` and `ir/{lower,lower_destructure,lower_try,lower_unit,lower_members}.pilot` read `.hook` and `.args`. `pilot check selfhost/cli/main.pilot` unchanged at 409 errors, same codes. Three engines: VM 672 passed (16 known bugs reproduced), IR 664 passed and C 664 passed (1 seam each, `map_declared_property`), 0 failed. With this, 5B.3 has no `any` or `ref<T>` left in live files except the two kept by design: the `glist_*` helpers in `gate/manifest.pilot` and `loader/pragma_filter.pilot` (raw `__engine`, 5B.4) and `ir/fault.pilot` `host_checked` (the host boundary, until 5F). **5B.3 is done.**
    - **5B.4 Raw `__engine` in live files (about 3,550 diagnostics on the same sites).**
      - Per D4, file by file, smallest first.
      - `__engine` calls that must stay (owner ops) move to `[EngineOp]` owners.
      - The legacy ratchet in the manifest goes down with each file.
      - **D4 survey (2026-11-17, owner: "Survey 5B.4 files"). D4 DECIDED 2026-11-17 (owner: "Approve, start group 1"): the recommendation of each group below is approved; group 1 is built first.** Group 3's owner question and group 4's Rust test edits still come back to the owner before their Rust or owner registry is touched.** The 67 files on `legacy_engine_ratchet()`, with `__engine(` counted outside comments and strings, and reach worked out from every link list (`crates/pilot-cli/src`, `crates/pilot-vm/src`, `selfhost/{cli,loader,gate}`) plus `use` imports and package peers; user reach from `stdlib/core/prelude.pilot` and the imports of `tests/` and `examples/`. 3,306 calls in all:

        | Group | Files | Calls | What the calls are | Recommended |
        |---|---:|---:|---|---|
        | **1. Linked, raw `Map`** | 11 | 375 | `map_new`/`map_get`/`map_set` only (`pkg/manifest.pilot` also reads the AST): `crypto/hash` 156, `preprocessor` 45, `pkg/resolver` 45, `pkg/manifest` 27, `gate/manifest` 21, `gate/runner` 18 (+1 `get_env`), `loader/pragma_filter` 18 (the `glist_*` helpers), `pkg/exclusions` 18, `gate/rename_litmus` 13, `pkg/archive` 9, `transpile/ingest` 5 | **Rewrite** onto `Map<K, V>`/`List<T>`, as the `PROP-002b` `Map` slice did. Pilot only, no Rust. Smallest first. |
        | **2. Linked, reads the Rust AST** | 9 | 601 | `ast_*`, `sym_str`, `mount_unit` and other Stage-A natives: `analyzer` 394, `ffi_bindgen` 56, `core/reflect` 52 (user programs reach it), `attr_weaver` 26, `core/primitives/ast` 21 (in the prelude: every program), `diagnostics` 20, `ast_bridge` 17, `core/keywords` 8, `cli/test_runner` 7 | `analyzer.pilot` is **not** duplicated: 27 of its 29 codes (KW001, OPT001/002, COND001/002, LOOP001–005, UNIMP001–004, DIV001, FLOW002–004, NULL002, MATCH002, UNREACH001, ENGINE001, ...) come from nowhere in `selfhost/check`, and `pilot run` (`run.rs` `analyze`) reports them. **Port** its checks into `selfhost/check` over `Program`, then drop it from the link list (a Rust edit). The rest move onto `selfhost/check`'s snapshot when their callers are ported, or wait for `PROP-025` (the Rust AST goes). |
        | **3. Linked, CLI host services** | 5 | 20 | `cli/host` 12 (`cli_run`, `cli_check`, `cli_analyze`, ...), `check/main` 1 (`chk_report`), `ir/main` 1 (`tool_input_text`), `ir/fault` 1 (`op_checked`, kept until 5F), `core/primitives/map` 5 (the `Map` type's own natives) | **Give them owners**: one `[EngineOp]` owner type for the tool host (`ToolHost`) and the `Map` natives on `Map`. Needs an owner decision on whether Stage-A tool natives get owners in `airlock_host_ops()` (they are not `HostOp`s). **Decided 2026-11-18: "Pilot first"** (see the Work Queue, decision 2). |
        | **4. Old frontend, linked only for `bootstrap_parity`** | 3 | 343 | `lexer` 64, `ast` 101, `parser` 178 (all `map_*`); `selfhost/cli/commands.pilot` links them with `pilotc.pilot` for the gate, which stops at seam `bootstrap.stage_compile` before using them | **Delete**: `PROP-021` (lexer) and `PROP-025` (parser) replace them, and the gate already stops at its seam. 7 Rust milestone tests name them, so the deletion goes with `PROP-004` or a small Rust test-edit proposal. |
        | **5. Not linked by any command** | 35 | 1,967 | `codegen/*` (8 files, 746), `vm/compiler` 341, `lsp/*`, `perceus/*`, `perf/*`, `dap/*`, `db/*`, `ide/vscode_gen`, `cst/cst`, `bench/bench`, `vm/pilc_writer`, and the unpackaged `stdlib/{core/effects,core/reactive,crypto/ed25519,db/sql_engine,http/http,pkg/licensing,polyglot/marshal}` | Outside 5B (D5). Every one is named by a `crates/pilot-analysis` milestone test, so deleting them is `PROP-004`. |
        | **Stale ratchet rows** | 4 | 0 | `check/program` (ratchet 2), `check/snapshot` (2), `ir/lower` (2), `gate/seed_audit` (12) have no call left; others are well under their ceiling (`lsp/handlers` 72 of 228, `transpile/ingest` 5 of 30) | **Tighten** the ratchet to the measured counts now (Pilot only). |

      - **Group 1 as built (2026-11-17, Pilot only, no Rust):**
        - **Zero `__engine` left** in `crypto/hash` (156 → 0), `preprocessor` (45 → 0), `pkg/resolver` (45 → 0), `transpile/ingest` (29 → 0; the survey's 5 was a miscount), `gate/manifest` (22 → 0), `gate/runner` (20 → 0, the `get_env` was a "no report" placeholder and is `return none;`), `loader/pragma_filter` (20 → 0), `pkg/exclusions` (18 → 0), `gate/rename_litmus` (15 → 0), `pkg/archive` (9 → 0). `pkg/manifest` 27 → 6: the six left read the Rust AST (`mount_unit`, `ast_*`, `sym_str`; group 2).
        - **Types:** raw handles became records and typed collections: `PkgArchive`/`PkgArchiveEntry`, `SourceExclusions`/`MemberExclusion`, `DepResolution`/`ResolvedDep`/`DepConflict` with `enum DepKind`, `PkgManifest`/`ManifestDep`, the preprocessor's `List<IfFrame>` stack (one shared `apply_conditional_directive` replaces two copies), `PragmaInterval`/`PragmaOpen`, `SeedTokenRegistry`, `Map<str, CrateCeilingRule>`, `GateEntry`, `RenamePair`/`RenameMap`; hash buffers, schedules and states are `List<i64>` and the round constants `const` tables (`SHA256_K`, `SHA512_K_HI`/`LO`, `BLAKE3_IV`, `BLAKE3_MSG_PERM`). The gate's banned-token and retired-file lists are `const`s (`RETIRED_FILES`, `BANNED_*`). `build_gate_registry` and `build_canonical_litmus_pairs` stay functions because `milestone01` calls them by name; the gate's `glist_*`/`gset_*` helpers stay as typed wrappers over `List<T>`/`Map<str, bool>` (no `__engine`, no `any`) because six gate files and `milestone01` call them.
        - **Caller fixes:** `stdlib/crypto/ed25519.pilot` hashes through `hash_words_be_bytes(sha512_words(..))` over `List<i64>` buffers (168 → 147 calls); `cli/pkg_cmd.pilot` follows the new `pkg` types; `gate/seed_audit.pilot` reads `crate_reg.keys()` and the `const` lists (12 → 9 calls, all `map_get` on registries that are real `Map`s now). Dead code deleted on the way: `ArchiveFormat` and its fake detectors, `ManifestField`/`manifest_get_field` (ignored its argument), `IngestedPilotModule` and three unused emitters, the preprocessor's unused `PreprocDiagnostic`/`CodeRegion`/`WarningSuppression`, `is_core_module_excluded`, `blake3_g_mix`. `resolve_package_dependencies` lost its unread second parameter.
        - **Ratchet:** `legacy_engine_ratchet()` lowered to the counts the gate measures: 67 → 55 files, 668 off the ceilings (stale rows included: `check/program`, `ir/lower`, `lsp/handlers` 228 → 222).
        - **New test:** `crypto/hash_vectors` (FIPS 180-4 SHA-256/512, RFC 4231 HMAC, official BLAKE3 vectors at 0–4,097 bytes; values from the published vectors and Python `hashlib`, never from Pilot). Hashing is about 25% faster on the VM (274 KB through all four hashes: 2.4 → 1.8 s net of startup).
        - **Verified:** VM 673 passed (16 known bugs reproduced), IR 665 passed and C 665 passed (1 seam each, `map_declared_property`), 0 failed; `pilot gate --full` 6 passed, `bootstrap_parity` at its seam, rename litmus included.
        - **Found, not fixed:**
          - Reading a `const` list on the VM copies the list on every read (2M reads: 3.29 s vs 0.62 s from a local); `hash.pilot` copies each table into a local once.
          - The checker reports ESCAPE002 when a function returns a `str` built from `ref` parameters (`explain_package_resolution`, now by value).
          - A target-typed `::Variant` of a selfhost enum stops at `ir.lower_construct` (node kind 300) in `pkg/resolver`; written `DepKind::X` instead.
          - Rust milestone tests already failing before this work and unchanged by it: `milestone60` (needs the deleted `cli/diff_test.pilot`), `milestone02 test_m2_3`, `milestone57 test_57_1`, `milestone58` (the `http.pilot` `tcp_connect` audit), `milestone01 test_m1_3` (expects 6 gates; there are 7).
      - **`stdlib/crypto/ed25519.pilot` finished (2026-11-17, owner: "not all __engine calls are replaced in ed25519.pilot"):** 147 → 0 calls, ratchet row removed. The scratch state is one `List<i64>` of `ED_SCRATCH_WORDS` (3,072) words passed `var ref` to every `ed_*` helper; the init flag lives in word `ED_INITED_SLOT`; 21 helpers that only wrote the state are `void`. New test `crypto/ed25519_vectors` checks RFC 8032 §7.1 TEST 1 and TEST 2 (public keys, signatures, verify, wrong-message and tampered-signature verify; values from the RFC). `ed25519_public_key_hex` copies its result (`[Workaround("check.escape_through_ref_arg", ...)]`), filed as `known_bugs/escape_str_from_ref_list`.
      - **Group 2 as built (2026-11-17, owner: "Start group 2", Pilot only, no Rust):** every check in `selfhost/analyzer.pilot` moved to `selfhost/check` or was deleted as a duplicate or fake, each in the same change as its port, so nothing is reported twice. The analyzer then held only layout binding and inference that nothing in `pilot` read (the CLI keeps only its diagnostics), so it is now an empty file: 1,336 → 7 lines, 394 → 0 calls, ratchet row removed (owner: "analyzer.pilot has __engine calls"). It stays only because `crates/pilot-analysis/src/infer.rs` embeds it (`include_str!`) and `run.rs` `analyze` runs it; `PROP-035` deletes both. The `pilot-analysis` milestone tests that read its bound layouts (`milestone14`, `27`, `50`, `54`, `56`, `phase1_rename_torture_gate`, `phase1_ironclad_killswitch`) now fail; `PROP-035` deletes them with the loader (AGENTS.md rule 6). The removed layout code is in git history (`b9ba25f`).
        - **`check/lint_flow.pilot`** (202 lines, `check_flow_lints`): NULL002, ERR005, LOOP001–003, LOOP005 (`continue a..b`), MATCH002.
        - **`check/lint_decls.pilot`** (309 lines, `check_decl_lints`, first in `check_all`): `[EnforceCoreNaming]` (7 rules), KW001, FLOW002–004, DECL002, DECL004 (`let`), VAR001 at uses.
        - **`check/lint_hints.pilot`** (356 lines, `check_lint_hints`) and **`check/suppress.pilot`** (94 lines, `[Suppress]`/`[Allow]`): UNIMP001/004, UNREACH001, LOOP005 (constant `while`), COND001/002, OPT001, FLOAT001, DIV001, VAR001, INLINE001, ENGINE001, HARD001. `check/diag.pilot` gains `sink_info`.
        - **Deleted, not ported:** LatticeContractMismatch (duplicate of COMP005), PILOT104 (duplicate of NOALLOC001), UNIMP003 (duplicate of TYPE006; `modules/analyzer_linked.stderr` updated), UNIMP002 (duplicate of FLOW001), OPT002 (fake: it fired on every `opt` read).
        - **Same output:** `pilot check selfhost/cli/main.pilot` before and after gives the same count for every code except OPT002 (deleted) and drops in the group-1 files (ENGINE001 120 → 27, KW001 84 → 68, TYPE001 9 → 5, UNUSED001/004).
        - **New tests and known bugs:** `control/{guard_else_leaves,if_conditions_used,keyword_contextual_names,loop_exits,placeholder_names,while_true_exits}`, `contracts/no_hardcoding_consts` and their `err_*`/`warn_*` pairs; known bugs `seed.continue_bounds_masks_check` and `seed.ctor_bare_return`.
        - **Verified (with the analyzer emptied):** VM 703 passed (19 known bugs reproduced); IR and C 695 passed, 0 failed, 1 seam each (`map_declared_property`); `pilot gate --full` 6 passed, `bootstrap_parity` at its seam; keyword matrix `--check` clean. Rust milestone tests not run (the seven `pilot-analysis` ones above are expected to fail until `PROP-035`).
        - **Found, not fixed (owner calls marked):**
          - **Owner:** ERR005 warns on every `try`, which the `try_catch` syntax file contradicts.
          - **Owner:** `[HardRealtime]` heap use is no longer checked (the analyzer's version was PILOT104, a duplicate of NOALLOC001 on `noalloc` only).
          - LOOP005 now names two rules (`continue a..b` bounds, constant `while`); one should get a new code.
          - OPT001 is an info diagnostic and the test runner cannot pin infos.
          - FLOW003/004 are masked by the seed's own check (`seed.continue_bounds_masks_check`).
          - VAR001 is now reported at uses as well as declarations.
          - `selfhost/diagnostics.pilot` repeats the naming messages; only Rust tests read it (`PROP-004`).
          - Not filed yet: on PilIR and C a `List<i64?>` literal holding `none` faults, and a closure inside a constructor fails.
    - **5B.5 Checker bugs found on the toolchain.**
      - The loop-variable shadowing bug (MOD003 on `op`), and any others: each gets a `known_bugs` repro first, then a fix in `selfhost/check`.
      - Then the visibility fixes (`pub` on items used across directory packages).
    - **5B.6 The rest.** METH001 (analyzer, per D4), PURE001, COMP005 (declare each struct's real lattice), NULL005/006, TYPE003, FLOW005.
  - **Owner decisions 2026-11-15:** D1 (b), `ref` is read-only and changes need `var ref`. D2 (a), `str` takes `i64` indexes. D3 (b), replace `any` with real types. D5: exclude the harness-only files and do `PROP-004` next. D4 is still open, per file. The slices themselves are not yet approved.
  - **Done when** `pilot check` reports 0 errors from every live entry. A new `pilot gate --full` rule, `toolchain_check`, keeps it at 0. Warnings (UNUSED004 217 and others) are listed, not gated.
- **5C — PilIR lowers and runs the whole toolchain (Pilot only).**
  - `pilot ir --mode run` on the toolchain entries: every `ir.lower_construct` seam it hits is closed in `selfhost/ir/`.
  - Done when `pilot test` driven by the IR-interpreted CLI gives the same results as the seed-run CLI. Slow is acceptable here.
- **5D — `selfhost/vm/ir_to_pilc.pilot` (Pilot only).**
  - Emits `.pilc` from optimised PilIR, after the `PROP-007` inline pass. Estimated 1,400–1,800 lines, so it is split across files: encoding, register allocation, op lowering, aggregates, closures, driver.
  - **Mapping (survey 2026-11-15):**
    - Arithmetic, `cmp`, `cast`, constants, `call_direct`, `return`, `jump`/`cond_br`, `unreachable` and `call_intrinsic` map 1:1 onto VM opcodes and intrinsics, so error messages stay identical to the PilIR interpreter's.
    - `switch_table` becomes a `CmpEq` chain. Block parameters become parallel moves.
    - `stack_slot`/`load`/`store` become registers. Globals become `global_get`/`global_set`.
    - Narrow integer ops add `num_wrap`.
    - Perceus ops are not emitted (only the C emitter runs Perceus).
    - The never-emitted ops (`simd_lane_op`, `arena_alloc`, `make_view` and the like) stop at a seam.
  - **Hard parts:**
    - (1) `call_indirect`: function records are untagged aggregates with layout "?". The proposed fix is a small IR change that tags them in `lower_closure`/`lower_generator`. They then map to `closure_new` + `CallClosure` through per-function trampolines.
    - (2) `extract_field`/`insert_field` on layout "?" needs a run-time `record_type` dispatch helper.
    - (3) Register pressure: about 238 usable registers, args in r1..r16. Linear-scan allocation with spilling; never allocate r0..r17.
    - (4) Source positions: every VM instruction carries its IR instruction's span. The `Call`/`EngineIntrinsic`/`Trap` is the last instruction with that span, so traces match the seed's byte for byte.
  - **Rust needed in 5D (a separate itemized proposal):** `.pilc` v5 stores no spans (`crates/pilot-vm/src/pilc.rs:35-86`), so a loaded `.pilc` reports `<program>`. Either add a v6 debug section, or hand the spans to the VM in memory.
  - **Reuse:**
    - `selfhost/vm/compiler.pilot` (1,125 lines) compiles from the Rust AST through `PhysicsRegistry`, which retires in `PROP-004`. Its input side is dead and its opcode list stops at 36. Keep only the encode/decode and patch helpers (about 150 lines), delete the rest when 5D lands, and port the `milestone55` checks to Pilot.
    - `pilc_writer.pilot` (227 lines) has the current byte layout, but keeps its state in a seed map through `__engine("map_get")` and lacks the UINT and CHAR tags. Keep the encoders and replace the state model.
  - **Build order inside 5D:** each step is compared against `pilot ir run` on `tests/conformance`.
    1. scalars, calls and spans;
    2. control flow and globals;
    3. register allocation;
    4. aggregates;
    5. closures;
    6. generators;
    7. try/defer/drop sweeps.
  - New CLI path: `pilot run --via-ir` runs a user program from PilIR bytecode on the existing VM.
  - Done when the conformance suite passes on `--via-ir` with the same results as the seed, and C2 (`pilot bench`) holds at seed speed or better.
  - **As built (2026-11-18, Pilot only; no Rust touched):**
    - **Done, build order 1–5:** `stdlib/pilot/vm/pilc.pilot` (the v5 container: build, write with `PackedBuffer<u8>` + `File.write_raw`, read back, disassemble; a `PDBG` trailer after the code holds the program's files and one source offset per instruction, which the loader skips today), `selfhost/vm/pilc_regs.pilot` (liveness, linear scan over r18..r254, cells in registers of their own), `pilc_lower.pilot` (scalars, narrow wrapping, compares, casts, cells and globals, direct calls, engine operations on an allow-list of the VM's natives, jumps with parallel moves, switch tables, returns), `pilc_aggregates.pilot` (records as VM maps marked with their layout id, other aggregates as lists; a field of a value whose layout is not known statically is found at run time through a start-up table of every record layout's field names), `pilc_closures.pilot` (`call_indirect` through `CallClosure`: one thunk per (function, captured count) unpacks the record's captured values and jumps to the function; a start-up table maps the pair to the thunk), `ir_to_pilc.pilot` (driver; address 0 calls the start-up code, then falls into function 0). The legacy `selfhost/vm/compiler.pilot` and `pilc_writer.pilot` are deleted (their ratchet rows removed from `selfhost/gate/manifest.pilot`).
    - **CLI:** `pilot compile <file> [-o out.pilc] [--disasm]`; `pilot run --via-ir <file> -- args` (compiles into a fresh temporary directory, runs the `.pilc` on the VM through `host_run_with`, deletes it); `pilot test --via-ir [paths]` runs every conformance case that way and judges it as `--ir` does.
    - **Result:** `pilot test --via-ir`: 689 passed, 9 failed, 3 stopped at a seam (the programs' own: vm.call_quote twice, ir.lower_construct once), 7 not eligible (2026-11-18, before the merge with main). Every failure is a case whose expected stderr names a source location (`file:line:col`): the VM prints `<program>` because it does not read the debug trailer (Rust item 1). The seams are the programs' own (`own.capture`, `own.collection_drop`, `vm.call_quote`, `ir.lower_construct`, ...), the same as `--ir`.
    - **Steps 6–7 (generators, try/defer/drop):** need no lowering of their own. PilIR already lowers generators to records plus `call_indirect`, and `try`/`defer`/drops to ordinary blocks and calls, so with steps 4 and 5 their conformance cases pass on `--via-ir` (`generators/*`, `memory/drop_*`, `control/try_catch`, `control/try_records`). No separate sweep was written.
    - **PilIR fix found by 5D:** `recv?..step` applied the step to the optional's own record, and its "absent" test was the engine's null, which an `opt` record never is. The IR interpreter printed the expected output only by accident. Now `selfhost/ir/lower_members.pilot` tests presence with the try protocol, unwraps before the step, and unwraps for a member read on an optional (`control/cascade_optional_member`, on all engines).
    - **Not done:**
      - runtime-error locations of a compiled program (Rust item 1);
      - spilling when live values outgrow the registers (`vm.register_spill`);
      - more than 16 arguments, or 15 through a function value (`vm.call_args`);
      - a slot's address as a value (`vm.address_value`);
      - engine operations the VM lacks (`vm.intrinsic`, e.g. `proc_args`);
      - C2 (`pilot bench` at seed speed) is not measured;
      - `--via-ir` is not yet the default `pilot run` path (5E/5F).
    - **Rust needed, itemized (awaiting owner approval; nothing below has been edited):**

      | # | File | Change | Why | Net Rust LOC |
      | :-- | :-- | :-- | :-- | --: |
      | 1 | `crates/pilot-vm/src/pilc.rs` | In `load_from_bytes`, after the code: when the bytes that remain start with `PDBG`, read the file count, each file's base (`i64`) and path (`u32` length + bytes), then the span count (equal to the instruction count) and one `i64` offset per instruction (`-1` for none). Keep the offsets in `pc_spans` (`ByteSpan { start, end: start }`) and the files in a new `debug_files: Vec<(u32, String)>` field. A file without the trailer loads as today. | So a fault in a compiled program has a span (`span_for_pc`). | +35 |
      | 2 | `crates/pilot-cli/src/run.rs` | In `compile_entry`'s `.pilc` branch and `execute`: when the loaded binary has `debug_files`, read each file and locate spans with the same (path, text, base) lookup `link.rs` `locate` does, instead of `<program>`. | So stderr names `file:line:col` and the `called from` lines exactly as `pilot run` (`errors/err_deep_trace`, `err_seam`, `err_pending`, ...). | +20 |
      | 3 | `crates/pilot-analysis/tests/milestone55_selfhost_vm_compiler_and_debug_table.rs` | Delete. | It reads the deleted `selfhost/vm/compiler.pilot` and `pilc_writer.pilot`. Its container checks are now `tests/conformance/vm/pilc_container` and `err_pilc_operand`, and its compile-and-run checks are `pilot test --via-ir`. | −454 |
      | 4 | `crates/pilot-analysis/tests/common/anticheat.rs` | Remove the `milestone55_selfhost_vm_compiler_and_debug_table.rs` row (line 253) from `MILESTONE_TEST_MANIFEST`. | Item 3 deletes that file (as in `PROP-031` A1). | −1 |

      Expected net: about **−400** Rust lines (+55 −455).
      - **Owner decision 2026-11-18 (relayed by the coordinator): approved on "Pilot first" terms.** The list was cut to the minimum. The `crates/pilot-cli/src/host.rs` drop-plan skip is dropped: `run --via-ir` already avoids the directory scan in Pilot by running from a fresh, empty directory. A Pilot-side span table cannot replace items 1–2. The VM gives a fault a source span only through `pc_spans`, which only the loader fills for a `.pilc`, and `pilot run x.pilc` reports the fault in Rust (`run.rs` `execute`) before any Pilot code sees it. So the trailer must be read into the existing `pc_spans` (1), and `execute` must locate those spans against the trailer's files (2). No new field beyond the file list, no new opcode, no new `HostOp`.
      - **Not yet edited:** this worktree changed no Rust. The edit waits for the owner's last look at this exact list. **No VM gap was found:** every PilIR opcode the interpreter runs maps onto existing opcodes and natives (`CallClosure`, `closure_new`, `record_set_type`/`record_type`, `global_get`/`global_set`, `IntrinsicArgs`/`IntrinsicKey`). So no new opcode or `HostOp` is proposed, and running emitted bytes goes through the existing `cli_run` on a `.pilc` path.
- **5E — Bootstrap (T1, decided 2026-11-18; needs its Rust list approved).**
  - Proposal to be itemized with its exact file list: the VM boots a checked-in `toolchain.pilc`, and Gate 2 checks stage 2 equals stage 3.
- **5F — Deletion (Rust).**
  - The scope list above, plus `paused_max_total_rust_loc()` ratcheted down by the measured amount, `must_be_absent` entries in `selfhost/gate/manifest.pilot`, and updates to `rust.md`, `pilot.md` and the matrices.
  - Depends on `PROP-002b` for native-binding dispatch now done in `lower_native.rs`/`lower_members.rs`, or on 5D emitting owner calls directly. To be settled in 5D.

---

## `PROP-003` through `PROP-006` — Subsequent Contraction Waves (Summary)

Each wave below will be expanded into an itemized, closed-file-list proposal before requesting owner approval:
1. **`PROP-003` (Wave 3):** Migrate remaining domain string/collection/cast helpers, `checked.rs`, `parse_header`, and parser semantic passes (`query.rs`, `comptime_if.rs`, `fstring.rs`, `pilot_impls.rs`, `attr_hooks.rs`, `num_suffix.rs`, `loud.rs`, `contracts.rs`, `with_scope.rs`) into `stdlib/` and `selfhost/`, and finish moving every remaining raw `__engine` call onto its owner (`PROP-002a` Amendment A1: one op, one owner; there is no `bridge_os.pilot` and no 2-file ceiling). Moving `attr_hooks.rs` must keep the one-call `comptime fn` expansion at bare-name call sites that A1's genesis-hook owners rely on (`attr_hooks.rs:55`), proven by a conformance program before the Rust file is deleted.
2. **`PROP-004` (Wave 4):** Delete `crates/pilot-analysis` (`6` `src` files, `821` LOC) after migrating remaining `crates/pilot-analysis/tests/milestone*.rs` checks into `.pilot` conformance suites.
3. **`PROP-005` (Wave 5):** (Also takes `PROP-003` slice 3C: delete `crates/pilot-vm/src/checked.rs` and the `op_checked` engine operation once the PilIR interpreter no longer runs on the seed's values; owner, 2026-11-15.) Implement `selfhost/vm/ir_to_pilc.pilot` (`PilIR` $\rightarrow$ `.pilc`) and delete all `24` `crates/pilot-vm/src/lower_*.rs` files, `vm_gen.rs`, `codes.rs`, and `BytecodeCompiler` in `crates/pilot-vm/src/lib.rs:60-1015` (`-7,833` Rust `src` LOC). It lowers from PilIR after the `PROP-007` inlining pass, so the temporary seed inlining in `lower_call.rs` can go with the rest.
4. **`PROP-006` (Wave 6):** Ship precompiled `toolchain.pilc` booted directly by `PilVirtualMachine` and delete `crates/pilot-{lexer,parser,ast,span,intern}` + `crates/pilot-cli/src/{link,check}.rs` (`~ -5,950` Rust `src` LOC), reaching the **`~1,800` LOC irreducible VM + `bridge.rs` seed floor**.
