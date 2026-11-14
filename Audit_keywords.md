# Keyword implementation audit — 2026-11-13

## 1. Scope and method

**What was audited.** Every keyword in the registry `stdlib/core/keywords.pilot` (categories 0–16, 35–37: 148 words), plus every word the standard library binds to grammar that the registry does not list. Operators and primitive types are out of scope.

**How.**

1. *Binding.* For each keyword: which file binds it to grammar (`bind_receptor`, `register_modifier`, a `syntax` rule, a `Cfg.Tokens` entry, a seed token) and what the binding lowers to. Sources read: `stdlib/core/core.pilot`, `stdlib/core/syntax/*.pilot`, `stdlib/asyncio/select_syntax.pilot`, `stdlib/contracts/contracts_syntax.pilot`, `stdlib/db/sql_syntax.pilot`, `stdlib/polyglot/polyglot_syntax.pilot`, `stdlib/realtime/rt_syntax.pilot`, and the receptor dispatch in `crates/pilot-parser/src/lib.rs`.
2. *Behaviour.* 110 probe programs written for this audit (several behaviours per keyword, each with an expected output worked by hand from the registry's own description), run on the seed (`pilot test`), PilIR (`--ir`) and C (`--emit c`), then about 40 follow-up one-off programs to separate probe mistakes from defects. This is on top of the conformance corpus (312 programs).
3. *Claims.* The registry's description of each keyword and the wiring ledger's rows were compared with what the probes showed.

**Verdicts used below.**

| Verdict | Meaning |
|---|---|
| REAL | Does what its description says on the seed, PilIR and C, with its diagnostics. |
| PARTIAL | The main path is real; the listed gaps are not. |
| WRONG | Runs and gives a wrong answer with no error. |
| IGNORED | Accepted, has no effect. |
| SIMULATED | A rule that imitates the feature without doing it (a fake, in the terms of AGENTS.md section 1). |
| BROKEN | Bound to grammar but every use is rejected. |
| MISPARSED | Accepted as something else. |
| SEAM | Stops at a registered seam. |
| UNBOUND | In the registry (documented, highlighted by editors), no grammar at all. |

**State of the tree during the audit.** Uncommitted working tree of 2026-11-13: `pilot test` 312 passed / 0 failed; `--ir` 306 / 0 (1 seam); `--emit c` 299 / 0 (8 seams); `pilot gate --full` 4 passed, 1 at seams.

---

## 2. Findings, most serious first

### 2.1 Crashes

| # | Finding | Evidence |
|---|---|---|
| C1 | **`case 1, 2:` (a `switch` arm with several values) crashes the parser** (`crates/pilot-ast/src/lib.rs:401`, index out of bounds on a missing node). | `fn main() { switch 1 { case 1, 2: print("low"); default: print("hi"); } }` **Fixed 2026-11-13:** a comma list in a `case` now means `case a \| b:` (`crates/pilot-parser/src/switch.rs`; `tests/conformance/control/switch_case_list`). |
| C2 | **One file the parser cannot read breaks every program in its folder tree.** Running a program indexes every `.pilot` file under the program's folder, recursively (`selfhost/loader/module_graph.pilot` `mg_build_index`), so the crash of C1 in one file made all 110 probes beside it crash, and every one-off program in a parent folder too. The same walk makes each run parse files it never links. | Any program run from a folder that also holds the C1 program. **Fixed 2026-11-13:** the header reader stops before the first item that is not `pkg`/`use`, so indexing never parses a body (`crates/pilot-parser/src/modules.rs` `at_module_decl`). A parse-only fuzz pass over every `.pilot` file then found and fixed six more parser crashes (see the wiring row "a file the parser cannot read ..."). |

### 2.2 Silent wrong answers

| # | Finding | Seed | PilIR / C |
|---|---|---|---|
| W1 | **Stepped ranges ignore the step.** `for i in 0..10:3` visits 0, 1, 2, … 9. The parser reads the step and drops it (`lib.rs`, `if matches!(pe.bin_op, 23 \| 24) && self.eat(Colon) { let _ = self.parse_expr_bp(..) }`). | wrong | wrong |
| W2 | **A function's default parameter value is not used.** `fn add(a: i64, b: i64 = 4)` called as `add(3)` stops with "cannot apply `+` to integer and record". No conformance program covers default parameters. | runtime error | seam |
| W3 | **Indexer properties return nothing.** `pub self[i: i64] -> i64 { get => self.cells[i]; .. }` then `g[1]` prints `null`; the setter works. | wrong (`null`) | seam |
| W4 | **Two equal structs are not equal on the seed, and a declared `op ==` is not called.** `V { x: 4 } == V { x: 4 }` is `false` on the seed and `true` on PilIR, with or without `pub op ==`. | wrong | right |
| W5 | **An optional prints differently per engine.** `let o: i64? = 5; print(o)` prints `5` on the seed and `[5, true, false, false]` on PilIR and C; the same for `g.next()` of a generator. | `5` | record text |
| W6 | **Dimensions are not checked.** `1s + 64KB` prints `1000065536`. | wrong | wrong |
| W7 | **Visibility is not enforced through a module-qualified name.** With `use lib.vis;`, `vis.hidden_fn()` (`priv`) and `vis.file_fn()` (`file`) run. The unqualified form (`hidden_fn()`) is rejected with MOD003 as the ledger says, and a `priv` field is rejected with VIS001 either way. | runs | runs |
| W8 | **`lvl(N)` is not enforced.** `lvl0 fn low() => top_secret()` calling a `lvl(9)` function runs. (Ledger: clearance checks SPEC.) | runs | runs |

### 2.3 Fakes that no ledger row declares

AGENTS.md section 1 forbids rules that imitate a feature and requires legacy ones to be listed as `SIMULATED`. These are bound in the bootstrap tables today and have **no ledger row**:

| Keyword | What the rule does (`file`) | Verdict |
|---|---|---|
| `spawn` | Runs its body at once, then builds a record tagged `"__async_task_v1"` with `state: "PENDING"` (`stdlib/asyncio/select_syntax.pilot`). | SIMULATED and BROKEN |
| `await` | Reads `result` back out of that record. | SIMULATED and BROKEN |
| `sync`, `atomic`, `isolate` | Set a flag in the environment record around the body. No lock, no atomicity, no isolation. | SIMULATED and BROKEN |
| `select { .. }` (channel form) | Builds a map and runs its arms. | SIMULATED and BROKEN |
| `transaction`, `commit`, `rollback`, `savepoint` | Flip `global_tx_state.is_active`; nothing is rolled back (`stdlib/db/sql_syntax.pilot`). | SIMULATED and BROKEN |
| `deadline(b, body)`, `within(b, body)` | Bind the budget to an unused local and run the body (`stdlib/realtime/rt_syntax.pilot`). | IGNORED |
| `on_missed`, `fallback` | **Always** run their body: `deadline(10s, { .. }); on_missed({ print("missed"); })` prints `missed`. | SIMULATED |
| `periodic(p, body)` | A plain `loop` with no period. | SIMULATED |
| `region(pool, body)` | Binds anything as the pool and runs the body; no arena. `region(5, { .. })` runs. | IGNORED |
| `noalloc({ .. })` (block form) | Builds a `RealtimeScopeGuard` that is not in scope where it expands. | BROKEN |

**Replaced by seams, 2026-11-13 (keyword roadmap K0):** every rule in this table now stops at a registered seam (`concurrency.*`, `data.*`, `realtime.*`) before its body runs, with an `err_<word>_seam` program per word on all three engines; `TransactionState`, `tx_*` and `RealtimeScopeGuard` are deleted. The table describes the code before that change.

"BROKEN" for the first five rows: the rules expand to raw `__engine(..)` calls or to functions (`tx_begin`) that are not in scope in the user's file, so every use is rejected (ENGINE001 or DECL001). They cannot run, which is the only reason they do no harm.

### 2.4 Accepted as something else

**Changed 2026-11-13 (keyword roadmap K0):** `adapter`, `database`, `context`, `each`, `async`, `export` and `asm` now stop the build at the seam `keyword.unbuilt` (crates/pilot-parser/src/unbuilt.rs, bound from the library; `err_<word>_unbuilt` programs). `extern`/`lib` and `lvl` are not changed yet. The table describes the code before that change.

**Changed 2026-11-13 (keyword roadmap K0), `extern`, `lib`, `lvl`:** a call of a top-level function declared without a body now stops the build at the seam `abi.extern_call` (selfhost/check/clearance.pilot; `err_extern_call_seam`) instead of DECL001. `lvl(N)` is kept on its declaration as an attribute (crates/pilot-parser/src/param_mods.rs) and a use of a function of a higher level than the user's stops at the seam `lvl.clearance` (`err_lvl_upward_call`; same-level and downward uses run, `lvl_same_or_lower`); this closes W8 as a seam, not as LVL001. Package levels are not read: what they mean is owner decision D11 (docs/keyword_roadmap.md). The shorthand `lvl0`..`lvl100` stops at `keyword.unbuilt`, and a modifier registered with an action the parser does not carry out is no longer ignored (ModAct::Ring is deleted).

| Keyword | What happens |
|---|---|
| `adapter A -> B { .. }` | Parsed as **a struct declaration named `A`** whose body is the block (`parse_domain_decl`). No conversion is installed; `let c: B = a` is TYPE001. Ledger: SPEC ("parsed and discarded"). |
| `database Name { .. }`, `context Name { .. }` | Same path: a struct named `Name`. |
| `extern "C" fn f(..);` | A function with no body. Calling it is DECL001 ("no function named"). `lib "name"` is skipped inside it. |
| `async fn` | The modifier is read and has no effect: the function is ordinary. |
| `each x` (prefix) | The word is skipped: `dbl(each [1, 2, 3])` passes the list itself. |
| `asm { .. }` | Parsed as an inline foreign-language block; seed SEED001, PilIR seam. |

### 2.5 Registered but with no grammar (27)

`actor`, `alignof`, `ascending`, `blueprint`, `bounded`, `connect`, `cue`, `delete`, `descending`, `fields_of`, `insert`, `into`, `jitter`, `macro`, `module`, `offsetof`, `order`, `override`, `priority`, `role`, `route`, `state`, `timeout`, `transition`, `unquote`, `update`, `values`.

**Changed 2026-11-13 (keyword roadmap K0):** all 27, and `connect`, `order`, `values`, `ascending`/`descending` among them, are bound. Declaration and statement words stop at the seam `keyword.unbuilt` where a declaration or statement starts (receptor 40, place 1); query clause words stop there as a query clause (place 2); `override` is an unbuilt modifier; `alignof`, `offsetof`, `fields_of` are type queries the seed refuses with SEED001. Each stays an ordinary name elsewhere (`unbuilt_words_as_names_2`). Per D4 (decided 2026-11-13) the nine symbolic-form names (`call_quote_*`, `op_splat`, `op_pairs`, `op_expand`) are deleted from the registry: each symbol is the Pair of its readable twin. `unquote` keeps its row with Pair `~` and follows the deep splice's seam `vm.call_quote` (`err_unquote_seam`). `err_query_clause` now uses `skip`, which is no keyword, since `order` stops at the seam.

Using any of them is "Unregistered keyword or unexpected identifier" or "undefined name". `blueprint`, `cue`, `role` and `route` belong to `.pchor` files (ledger M24, CORE as a library parser); `state` / `transition` have a library lattice (M26, CORE) with no syntax. Editors highlight all 27 as keywords because the registry lists them.

### 2.6 Bound to grammar but not in the registry

- Long forms of the visibility words: `private`, `fileprivate`, `protected`, `package`, `internal`, `public`; and `namespace`, `extension`. AGENTS.md section 2 asks for one keyword per responsibility.
- `lvl0`, `lvl1`, `lvl2`, `lvl3`, `lvl80`, `lvl90`, `lvl100`.
- Word operators: `and`, `or`, `not`, `is`, `some`, `union`, `intersect`, `except`; `view`, `on`, `raw`.
- Realtime rule words: `on_missed`, `fallback`.
- Inline-language words: `c`, `cpp`, `rust`, `rs`, `python`, `py`, `js`, `javascript`, `ts`, `typescript`, `csharp`, `cs`, `lua`, `sql`, `postgres`, `postgresql`, `mysql`, `sqlite`, `wgsl`, `ptx`, `msl`, `java`.
- `open`, `until` and `splice` are each registered twice (two categories), which is intended (two roles) but means one description per role.
- `stdlib/core/reflect.pilot:503` repeats the registry's category lists as strings (`"as|async|await|spawn|.."`), a second source for the same facts.

---

## 3. Every keyword

Columns: **S** seed, **I** PilIR, **C** emitted C. `ok` = probe output correct; `seam` = registered seam; `err` = rejected; `—` = not applicable.

### 3.1 Seed tokens

| Keyword | Bound by | S | I | C | Verdict and notes |
|---|---|---|---|---|---|
| `__engine` | seed token | err | err | err | REAL by design: ENGINE001 outside core. |
| `struct` | seed token | ok | ok | ok | PARTIAL: defaults, nesting, value copy are right. Equality is W4; `static` fields are not supported (`P.count` is "undefined name"). |
| `interface` | seed token | ok | ok | ok | REAL: defaults, overriding, TYPE006 for a missing member; a call on a `T : I` parameter now runs on PilIR and C. |
| `fn` | seed token | ok | ok | ok | PARTIAL: named arguments, generics, closures are right; default values are W2. A closure captures by value (writing a captured variable is DECL004; a later change of the variable is not seen). |
| `let` | seed token | ok | ok | ok | REAL: assignment to a `let` is rejected. |
| `ref` | seed token | ok | ok | ok | REAL: `ref` lends, `var ref` writes back (ALIAS001). |
| `if` | seed token | ok | ok | ok | REAL, as statement and as value. |
| `loop` | seed token | ok | ok | ok | REAL. |
| `return` | seed token | ok | ok | ok | REAL; FLOW001 when a path returns nothing. |
| `yield` | seed token | ok | ok | ok | PARTIAL (ledger DRAFT): generators and collecting `for` work; a `next()` result prints per W5. |
| `defer` | seed token | ok | ok | ok | REAL: last in first out, on return and on a failing `?`. |
| `comptime` | seed token | ok | ok | ok | REAL: `comptime fn`, `comptime { }`, `comptime if`. |
| `quote` | seed token | ok | ok | ok | PARTIAL: templates of `syntax` rules work; expansion is not hygienic (ledger M27 SPEC); a quote held as a value has no use yet. |
| `asm` | seed token | err | seam | seam | MISPARSED as an inline foreign block (2.4). |

### 3.2 Control flow

| Keyword | Bound by | S | I | C | Verdict and notes |
|---|---|---|---|---|---|
| `for` | receptor 9 | ok | ok | ok | PARTIAL: ranges, lists, slices, generators. Step is W1. A `Map` and a `str` are "not iterable"; `Map.keys()` does not exist. |
| `in` | receptor 10 | ok | ok | ok | REAL in `for`; `x in xs` over sequences only. |
| `while`, `until` | `syntax` rules | ok | ok | ok | REAL. |
| `break`, `continue` | receptor 13 | ok | ok | ok | REAL; `break value` needs a `for .. then`. |
| `then` | receptor 11 | ok | ok | ok | REAL. |
| `switch`, `default` | receptors 14, 16 | ok | ok | ok | REAL: values, ranges, guards (`case n if ..`), variants with payloads, value form, FLOW005 exhaustiveness. |
| `case` | receptor 15 | ok | ok | ok | REAL (C1 fixed: `case 1, 2:` is an or-pattern). |
| `else` | receptor 3 | ok | ok | ok | REAL. |
| `guard` | receptor 17 | ok | ok | ok | REAL, boolean and `some` forms. |
| `try`, `catch` | receptors 18, 19 | ok | ok | ok | REAL, block and expression forms. |
| `errdefer` | receptor 38 | ok | ok | ok | REAL (FLOW006 when the function cannot fail). |
| `become` | receptor 39 | ok | seam | seam | Seed REAL (200,000 mutual calls run); PilIR needs a tail-call instruction. |

### 3.3 Declarations

| Keyword | Bound by | S | I | C | Verdict and notes |
|---|---|---|---|---|---|
| `var` | `Cfg.Tokens.mod_var` | ok | ok | ok | REAL. |
| `const` | modifier | ok | ok | ok | REAL: assignment rejected. |
| `enum` | receptor 27 | ok | ok | ok | REAL: explicit values, comparison, exhaustive `switch`. |
| `type` | receptor 26 | ok | ok | ok | PARTIAL: plain alias and variant types (`type S = A(..) \| B(..)`) work; a generic alias is TYPE004. |
| `state` | — | err | err | err | UNBOUND. |
| `actor` | — | err | err | err | UNBOUND. |

### 3.4 Visibility

| Keyword | Bound by | Verdict and notes |
|---|---|---|
| `priv`, `file`, `pkg`, `prot` | modifiers 0, 1, 4, 3 | PARTIAL: an item used from another package is MOD003 and a `priv` field is VIS001, for unqualified use; bypassed by a module-qualified call (W7). `prot` behaves as `pkg`: nothing gives derived types or extensions their own reach. |
| `pub`, `int`, `open`, `export` | modifiers 7, 6, 8, 9 | All four reach other packages of the program. `open` (overriding across packages) and `export` (foreign ABI) add nothing to `pub` yet. |
| `lvl` | parametric modifier | IGNORED (W8). |

### 3.5 Modifiers, properties, lifecycle

| Keyword | Bound by | S | I | C | Verdict and notes |
|---|---|---|---|---|---|
| `ext` | `Cfg.Tokens.mod_ext` | ok | ok | ok | REAL, block and one-line forms. A call on the wrong receiver type is not rejected (known bug `check.ext_wrong_receiver`). |
| `static` | modifier | ok | ok | ok | PARTIAL: static functions only. |
| `pure` | modifier | ok | ok | ok | REAL (PURE001). |
| `partial` | modifier | ok | ok | ok | REAL. |
| `iso` | modifier | ok | ok | ok | REAL (ISO001). |
| `noalloc` | modifier + rule | ok | ok | ok | Function modifier REAL (NOALLOC001); block form BROKEN (2.3). |
| `Align`, `Flags`, `Stack`, `packed` | attributes | ok | ok | ok | REAL per the corpus (`layout_attrs`, `flags_ops`, `stack_let`); not probed further here. |
| `get`, `set`, `field`, `value` | receptors 23, 24; receiver words | ok | ok | ok | REAL for properties, including a setter that rejects a value. Indexers are W3. |
| `self` | receiver word | ok | ok | ok | REAL; constructor `self(..)` and destructor `~self()`. An owned value is dropped after its last use, not at the end of its block. |
| `scope`, `enter`, `exit` | receptors 20–22 | seam | seam | seam | SEAM `lifecycle.scope_self`: what enters a value is undecided. |

### 3.6 Contracts and hints

| Keyword | Bound by | S | I | C | Verdict and notes |
|---|---|---|---|---|---|
| `requires`, `ensures`, `old` | receptor 33 / 18, 19, 24 | ok | ok | ok | REAL (DBC006, DBC007; DBC002/DBC003 for an impure clause). |
| `invariant` | receptor 33 / 20 | seam | seam | seam | SEAM `contracts.invariant`. |
| `assume`, `unreachable` | functions in `core/hint.pilot` | ok | ok | ok | REAL (HINT001, HINT002). |
| `assert` | native function | ok | ok | ok | REAL. |

### 3.7 Adapters, injection, composition

| Keyword | Bound by | Verdict and notes |
|---|---|---|
| `adapter` | receptor 33 / 21 | MISPARSED (2.4). |
| `context` | receptor 33 / 22 | MISPARSED at top level; as a parameter modifier or `with context` it is a syntax error. |
| `each` | receptor 33 / 23 | Parameter form: syntax error. Prefix form: IGNORED. |
| `as` | receptor 29 | REAL for casts on the seed and PilIR; `as view` is known bug `seed.view_reinterpret_identity`. |
| `with`, `without`, `coalesce` | receptor 30 | REAL (COMP003–COMP008). A member hidden by `without`, called in the same file, fails at run time ("no method"), not when checked. |
| `override` | — | UNBOUND. |

### 3.8 Concurrency and real time

| Keyword | Bound by | Verdict |
|---|---|---|
| `async` | modifier `effect_async` | IGNORED. |
| `await`, `spawn`, `select`, `sync`, `atomic`, `isolate` | `syntax` rules | SIMULATED and BROKEN (2.3). |
| `deadline`, `within`, `periodic`, `region` | `syntax` rules | IGNORED / SIMULATED (2.3). The real scoped arena is `with arena a { .. }` (ledger F7, DRAFT). |
| `bounded`, `jitter`, `priority`, `timeout` | — | UNBOUND. |

### 3.9 Database and queries

| Keyword | Bound by | Verdict |
|---|---|---|
| `from`, `where`, `select` (query) | receptors 33 / 6, 28, 33 / 7 | REAL over a sequence, on all three engines; any other clause is SYN001. |
| `transaction`, `commit`, `rollback`, `savepoint` | `syntax` rules | SIMULATED and BROKEN (2.3). |
| `database` | receptor 33 / 1 | MISPARSED (2.4). |
| `ascending`, `connect`, `delete`, `descending`, `insert`, `into`, `order`, `update`, `values` | — | UNBOUND. |

### 3.10 Foreign code

| Keyword | Bound by | Verdict |
|---|---|---|
| `extern`, `lib` | receptors 7, 8 | IGNORED: declaration with no body, call is DECL001. |
| `assembly` | receptor 32 / 17 | Bound as an inline-language word (same path as `asm`). |
| `module` | — | UNBOUND. |

### 3.11 Reflection and metaprogramming

| Keyword | Bound by | S | I | C | Verdict and notes |
|---|---|---|---|---|---|
| `nameof` | receptor 33 / 25 | ok | ok | ok | REAL. |
| `sizeof` | type query | ok | ok | ok | PARTIAL: primitives only; `sizeof(Pt)` is SEED001 on the seed and "undefined name `Pt`" from the checker. |
| `alignof`, `offsetof`, `fields_of` | — | err | err | err | UNBOUND. |
| `op` | receptor 25 | ok | ok | ok | PARTIAL: `op +`, `op []` work; `op ==` is W4. |
| `syntax` | receptor 4 | ok | ok | ok | REAL for rules, statement and expression templates; not hygienic. |
| `macro`, `unquote` | — | err | err | err | UNBOUND (`~x` inside call-quotes is the symbol form). |
| `unit` | `syntax` rule | ok | ok | ok | PARTIAL: literals convert to their base unit; W6. |
| `use` | module word | ok | ok | ok | REAL; a call through an alias (`use std.text as t; t.join_text(..)`) runs on the seed and stops at `ir.lower_construct` on PilIR and C. |
| `blueprint`, `cue`, `role`, `route` | — | err | err | err | UNBOUND in `.pilot` (2.5). |
| `transition` | — | err | err | err | UNBOUND. |

### 3.12 Unpacking and call-quoting

| Keyword | Bound by | S | I | C | Verdict and notes |
|---|---|---|---|---|---|
| `splat`, `flat` (`*x`, `*..x`) | receptor 35 + rules | ok | ok | ok | REAL in list literals; `f(*xs)` / `f(splat(xs))` REAL for a direct call. |
| `pairs` (`**x`) | receptor 35 + rule | err | seam | seam | Not implemented: SEED001 / `ir.lower_construct`. |
| `call`, `splice`, `frame`, `sig`, `grad`, `plan` and `! !! !& !^ !@ !?` | receptor 37 | seam | seam | seam | SEAM `vm.call_quote` (ledger M4 SPEC). Both spellings reach the same seam. |

---

## 4. Diagnostics that do work

Confirmed by failure-case probes: ENGINE001, TYPE001, TYPE002, TYPE003, TYPE006, FLOW001, FLOW005, FLOW006, MOD003 (unqualified), VIS001, PURE001, NOALLOC001, ISO001, ALIAS001, DBC006, DBC007, HINT001, HINT002, DECL004, COMP004, SYN001, CALL001.

Not raised where the registry or AGENTS.md implies one: a dimension mismatch (W6), LVL001 (W8), MOD003 through a qualified name (W7), a call of a member hidden by `without` in the same file, an extra argument to an `ext` method, arguments that bind one generic parameter to two types (known bug `check.generic_arg_unchecked`).

---

## 5. Ledger and registry against reality

| Item | Says | Found |
|---|---|---|
| Ledger, visibility row (F4) | MOD003 / VIS001 WIRED | True for the tested forms; W7 is not covered. |
| Ledger | no rows | The fakes of 2.3 (concurrency, transactions, real time). They need `SIMULATED` rows naming the files, or to be removed. |
| Ledger M9 "legacy stdlib directories removed" | SPEC: `stdlib/asyncio`, `contracts`, `db`, `polyglot`, `realtime` remain | These are the files that hold the fakes, and they are in the bootstrap ingest list (`crates/pilot-parser/src/lib.rs`), so they shape every program's grammar. |
| Registry, `async` | "Declares an asynchronous function returning a task." | No effect. |
| Registry, `spawn` | "Spawns a concurrent green task." | Runs its body in place; unusable. |
| Registry, `sync` / `atomic` / `isolate` | locks, atomic instructions, isolation | A flag. |
| Registry, `deadline` / `within` / `periodic` | real-time contracts | Nothing is timed. |
| Registry, `transaction` | "ACID database transaction" | A flag. |
| Registry, `region` | "explicit arena region" | No arena. |
| Registry, `adapter` | "implicit or structural conversion" | A struct declaration. |
| Registry, `step` operator `:` | "Stepped range progression" | Step dropped (W1). |
| Registry, `lvl` | "security clearance level ring" | Not checked. |

---

## 6. Test suite state found on the way

`cargo test --workspace -- --include-ignored` (run 2026-11-13 00:10): 300 passed, 4 failed.

| Test | Cause | Status |
|---|---|---|
| `cli_changed::a_documentation_commit_runs_only_the_cases_that_never_link` | The two cases are listed in another order since the conformance folders were split. | Fixed, passes. |
| `cli_f7::a_binary_built_to_count_allocations_reports_none_left` | A path left over from the folder split. | Fixed, passes. |
| `cli_f7::the_corpus_passes_natively_with_no_live_allocations` | Asserts "0 stopped at a seam" for `--emit c`. It has 8: `become` (which was already a seam at HEAD) and 7 programs that use `char`, whose C representation is not written (new with this work). | Open. |
| `milestone51::test_51_3_domain_syntax_packages_and_strict_nameof_hygiene` | Its source writes `requires len(users) >= 0` without parentheses; the contract grammar at HEAD requires them. Not caused by this work. | Open; the test or the grammar has to move, which is the owner's call. |

---

## 7. Suggested order of repair

1. **C1, C2** — a parser crash, and one bad file taking down a folder.
2. **W1–W4** — step, default parameters, indexers, struct equality: everyday code gives wrong answers.
3. **The fakes of 2.3** — delete the rules or stop them at seams, and add ledger rows; today they are bound in every program's grammar.
4. **2.4** — `adapter`, `database`, `context`, `extern`, `async`, `each` should be rejected or stopped at a seam until they are built, not accepted as something else.
5. **W5–W8** — optional display, dimension checks, qualified visibility, `lvl`.
6. **2.5** — either bind the 27 unbound keywords to a seam ("not implemented yet") or take them out of the registry until they exist.
7. `char` in emitted C and `become` in PilIR, to bring the native corpus test back to zero seams.

Probe programs and raw results: `/tmp/claude-1000/-home-moony-Documents-dev-projects-Pilot/e3ce282f-0a2c-4bb7-9773-d7a6563f2793/scratchpad/audit/` (`p/` the 110 probes, `panic/` the crashing one, `seed.txt`, `ir.txt`, `c.txt`, `results.json`).

## 2026-11-13 — `assert` made a keyword (uncommitted)
- `assert(cond)` / `assert(cond, note)` is a syntax rule (stdlib/core/assert.pilot) with one arm per comparison operator. A failure reports the expression text and, for a comparison, both sides. The native `rt_assert` is deleted on every engine.
- A condition of `if`, `while`, `!` or `assert` whose type is known and not `bool` is TYPE001 (err_condition_not_bool, err_assert_not_bool).
- Rule templates are not hygienic: known bug parser.rule_template_capture.
- Follow-up the same day: rule templates are hygienic (the names a template binds for itself are hidden from the caller; parser.rule_template_capture fixed, err_assert_hygiene). The assert message is built in the template, so a user function named like a helper cannot change it. `assert true;` and a wrong argument count are SYN001. `!` on a non-bool is TYPE001 with its own wording, and `if !x` is reported once. A template holding a node it cannot carry is SYN001 instead of becoming `none`. The bootstrap `for` and `scope` rules hold such nodes (AssignStmt, DeferStmt); their diagnostics are discarded at bootstrap.
- Second pass the same day:
  - A string side of a failed `assert` comparison is shown quoted and escaped.
  - Rule templates carry assignments and `defer`, so the bootstrap `for` and `scope` rules no longer lose nodes.
  - A parser test requires every bootstrap source to parse without a diagnostic (crates/pilot-parser/tests/bootstrap_sources.rs). To make it pass, module_ext.pilot moved before visibility.pilot, and the never-parsing `__bootstrap_syntax` in core.pilot was deleted.
  - PilIR lowers a method called on a method's result.
  - Open for the owner: the text of a struct differs between the seed and PilIR/C, and an `if` without `else` (so `let v = assert(..)`) gives none as a value.
- The owner decided (2026-11-13) that a struct prints with its field names. Every engine run by a gate now prints `{ name: value, .. }`, sorted by field name as the seed sorts a map: the seed, PilIR, C, JS and Python (types/struct_text). The Lua, C++, Rust and C# emitters are not changed.

## 2026-11-13 — K0 closed; the parser fails loudly (uncommitted)
- The `keywords` gate is on (`pilot gate`; GATE021–GATE023, selfhost/gate/keyword_gate.pilot). It was shown to fail on a row with no binding, a fake verdict and an empty matrix.
- Syntax the parser read and threw away is now reported (crates/pilot-parser/src/loud.rs): a step on a range (W1, `err_range_step`), a default parameter value (seam `fn.param_default`, W2, `err_param_default`), `*args`/`**kwargs`, generic parameters on an `enum`, `catch e: T`, a `where` clause on a type, parameters on `~self()`, `x[a, b]`, a pattern after `for`, `name?: T`, a modifier on one accessor, a stray word in an accessor block, `else` with no `if`, a second visibility word, a visibility word on a local, a missing type after a type operator, and a `nameof` that does not quote one name. Each has an `err_*` program.
- The checker now reports a wrong argument count for a plain function call (CALL001), `*`/`*..` on a scalar (TYPE001), a missing enum member (METH001), assigning a get-only property (DECL004) and a `coalesce`/`with`/`without` naming no member of the bundle (COMP009).
- A block `{ i = i + 1; }` passed as an argument is no longer read as a map (this made `until(.., { .. })` loop forever).
- Not done: generic parameters on a `let` binding are still dropped; the arity check skips method calls, pipes and calls inside type bodies; a get-only assignment is caught only when the receiver's type is known. New known bugs: `engine.struct_copy_aliases`, `engine.with_renamed_field_unseen`.

## 2026-11-13 — K1: W1 and W2 closed (uncommitted)
- W1: `for i in a..b : step` counts by the step on the seed, PilIR, C, JS and Python (`control/for_range_step`). A step of 0 or less stops the program before the first pass (`err_for_step_zero`). The parser builds `BinaryOp::Step` (49); only a `for` head reads it.
- W2: a literal parameter default fills an argument a call leaves out, on every engine (`control/fn_param_defaults`: trailing, named, method, loop-element and call-result receivers). A default must be a literal (`err_param_default_not_literal`), a parameter without one may not follow one with one (`err_param_default_gap`), the default must have the parameter's type (`err_param_default_type`), and the arity check allows the range (`err_call_arity_default`). The seam `fn.param_default` is deleted.
- Not done: a default that names an earlier parameter or a constant (where it is evaluated is not decided); the checker does not check a step's type against the range's bounds; examples/showcase/16 still fails, now on `enum Expr<T>` (generic enum, refused since K0).
- Found on the way: the PilIR interpreter stored the result of an instruction that has none (a call made for its effect), which crashed with an index out of bounds; fixed in selfhost/ir/interp.pilot `put`.

### K1: `become` runs on every engine (2026-11-13)

`become f(args);` no longer stops at `ir.lower_construct` on PilIR. The tail call becomes a jump: `selfhost/ir/lower_tail.pilot` copies the blocks of each function a tail call reaches into the caller and jumps to the copy. `control/become_tail` and the new `control/become_shapes` pass on the seed, PilIR, C, JavaScript and Python. Still stopping at the seam: a call through a function value, a `become` in a function with `var ref` parameters or in a generator, and a `become` with a value to release after the call.

### K1: known bug `engine.struct_copy_aliases` fixed (2026-11-13)

A struct that declares no lattice and holds only values (`struct P { pub a: i64; }`) is now copied where it is bound, passed, returned or stored, on every engine: `selfhost/check/copies.pilot` reads the lattice worked out from the fields. The repro moved to `tests/conformance/types/struct_copy_independent.pilot`, with `struct_copy_passed.pilot` beside it; the bug entry is deleted. An optional or a collection (types the engine holds itself) is not copied field by field.
