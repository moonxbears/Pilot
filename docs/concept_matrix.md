# Concept Matrix

The verified implementation status, per-engine parity, and Pilot vs. Rust ownership of every structural, symbolic, memory, runtime, polyglot, and toolchain concept in Pilot that is broader than (or orthogonal to) a single registered word in [`docs/keyword_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/keyword_matrix.md).

Per [`AGENTS.md`](file:///home/moony/Documents/dev/projects/Pilot/AGENTS.md) §11 Rule 8:
- Check both [`docs/keyword_matrix.md`](file:///home/moony/Documents/dev/projects/Pilot/docs/keyword_matrix.md) and this file before relying on or claiming any language, runtime, or toolchain capability.
- Update the matching row(s) in the **exact same change** whenever a concept's engine parity, conformance tests, diagnostic codes, or Pilot/Rust ownership changes.
- **Never mark a row higher than real `pilot` CLI runs demonstrate** (`pilot test`, `pilot test --ir`, `pilot test --emit c`, `pilot gate`).

---

## Columns & Verdicts

| Column | Meaning |
| :--- | :--- |
| **ID & Concept** | Canonical concept identifier (`C01`–`C40`) and architectural name. |
| **Surface / Mechanism** | Symbolic syntax, readable pairing ([`AGENTS.md`](file:///home/moony/Documents/dev/projects/Pilot/AGENTS.md) §2.10), or compiler invariant. |
| **S / I / C** | Verified behavior on **S**eed (`pilot run`), **I**R (`pilot test --ir`), and emitted **C** (`pilot test --emit c`): `ok` = verified passing, `part` = core subset works with documented limits, `seam` = traps loudly at `SEAM001`, `tool` = CLI/compiler-host capability rather than per-expression codegen. |
| **Verdict** | `REAL` (end-to-end across engines), `PARTIAL` (working core with tracked gaps/limits), `SEAM` (explicit `SEAM001` boundary), or `MIGRATING` (works today while legacy Rust shims are queued in [`proposals.md`](file:///home/moony/Documents/dev/projects/Pilot/proposals.md)). |
| **Owning Pilot Modules ([`pilot.md`](file:///home/moony/Documents/dev/projects/Pilot/pilot.md))** | Primary `.pilot` files implementing the concept. |
| **Legacy Rust Residual ([`rust.md`](file:///home/moony/Documents/dev/projects/Pilot/rust.md))** | Remaining Stage-A / `BytecodeCompiler` Rust code (`0` when already pure Pilot, or target proposal ID in [`proposals.md`](file:///home/moony/Documents/dev/projects/Pilot/proposals.md)). |
| **Verified Conformance, Codes & Notes** | Passing/failing `tests/conformance/*` programs, diagnostic codes, and open `known_bugs` or seams. |

---

## Summary (`40` Tracked Concepts)

| Verdict | Count | Meaning |
| :--- | ---: | :--- |
| `REAL` | `26` | Proven end-to-end across engines (`ok / ok / ok` or verified CLI tool gate) with passing + failing tests. |
| `PARTIAL` | `9` | Real working implementation with explicit, documented scope limits or open `known_bugs`. |
| `MIGRATING` | `3` | Enforced in part today, with Airlock / Stage-3 cutover queued in [`proposals.md`](file:///home/moony/Documents/dev/projects/Pilot/proposals.md). |
| `SEAM` | `2` | Intentionally stops at a registered `SEAM001` boundary until scheduled milestone wiring. |

---

## 1. Airlock, Bootstrap & Name-Blind Physics (`C01`–`C04`)

| ID & Concept | Surface / Mechanism | S | I | C | Verdict | Owning Pilot Modules | Legacy Rust Residual | Verified Conformance, Codes & Notes |
| :--- | :--- | :-: | :-: | :-: | :--- | :--- | :--- | :--- |
| **C01. 14-Token Seed Ceiling** | Only `__engine`..`asm` recognized in Stage-A lexer | ok | ok | ok | `REAL` | `stdlib/core/keywords.pilot`, `selfhost/gate/seed_audit.pilot` | `pilot-lexer/src/keywords.rs` (retires Wave 6) | Enforced by `pilot gate`; all other `126` keywords bind from Pilot (`core.pilot` / `syntax`). |
| **C02. Universal Name-Blind Contract** | Zero hardcoded user-space type/op names in Rust; Dual-Pass Rename Litmus | ok | ok | ok | `MIGRATING` | `stdlib/core/bootstrap.pilot`, `selfhost/gate/rename_litmus.pilot` | Minor cast/query strings in `ops.rs`/`query.rs` (`PROP-003`) | `[Bits]`, `[Layout]`, `[Lattice]`, `MethodTable`, and `bind_receptor` drive types name-blindly (`GATE001`–`GATE010`). |
| **C03. 3-File Frozen `__engine` Airlock** | `bridge_lang.pilot` + `bridge_os.pilot` $\leftrightarrow$ `bridge.rs` (`ENGINE001`) | ok | ok | ok | `MIGRATING` | `selfhost/check/resolve.pilot`, `selfhost/gate/seed_audit.pilot` | `115` `.pilot` files + `heap.rs`/`host.rs` consolidating in `PROP-002` | User `__engine` rejected (`errors/err___engine_user`, `ENGINE001`); `PROP-002` freezes the 3 airlock files permanently. |
| **C04. 3-Ring Namespace Isolation** | `core.*` (`0` OS imports, `RING001`), `std.*`, `pilot.*` | ok | ok | ok | `REAL` | `stdlib/core/prelude.pilot`, `selfhost/gate/seed_audit.pilot` | `0` | Freestanding `core.*` verified by `RING001` and `[EnforceCoreNaming]`. |

---

## 2. Lexical, Symbolic & Dual-Readability Syntax (`C05`–`C13`)

| ID & Concept | Surface / Mechanism | S | I | C | Verdict | Owning Pilot Modules | Legacy Rust Residual | Verified Conformance, Codes & Notes |
| :--- | :--- | :-: | :-: | :-: | :--- | :--- | :--- | :--- |
| **C05. Two-Way Dot & Target Enums** | `InfixDot` (`a.b`), `PrefixDot` (`.field` $\rightarrow$ `self.field`), `::Variant` | ok | ok | ok | `REAL` | `selfhost/lexer.pilot`, `selfhost/parser.pilot`, `selfhost/check/infer.pilot` | Stage-A `lexer.rs` / `parser` | Verified by `types/enum_payload`, `types/shared_variant_name`, `types/method_self_field_option`; `known_bugs/variant_compare_untyped` tracked. |
| **C06. Fluid Cascades** | `recv..m1()..field = v` / `?..` (`Precedence::Cascade = 1`) | ok | ok | ok | `REAL` | `selfhost/parser.pilot`, `selfhost/ir/lower_expr.pilot` | `lower_cascade.rs` (retires `PROP-005`) | Verified on all 3 engines by `control/cascade`. |
| **C07. Nested `f"..."` Interpolation** | `f"a {d[f"{k["x"]}"]}"` & `{x:spec}` format holes | ok | ok | ok | `REAL` | `stdlib/core/format.pilot`, `selfhost/lexer.pilot`, `selfhost/parser.pilot` | `fstring.rs` (retires `PROP-003`/`006`) | Balanced brace/string hole scanning with unescaped inner quotes: `strings/fstring_nested_quotes`, `text/fstrings`, `text/fstring_format_patterns`. |
| **C08. Symbolic Type Rewrites** | `T?`$\leftrightarrow$`opt<T>`, `T!E`$\leftrightarrow$`res<T,E>`, `T[]`, `T[N]`, `T[..]`, `T@N`, `(A)=>R` | ok | ok | ok | `REAL` | `stdlib/core/syntax/type_construct.pilot`, `selfhost/check/types.pilot` | Stage-A `TypeConstructRule` | Verified by `types/type_construct_forms`, `types/res_short_form`, `types/tuple_values`, `types/union_values`. |
| **C09. Sequence & Map Unpacking** | `*x`/`splat(x)`, `**x`/`pairs(x)`, `*..x`/`flat(x)` | ok | ok | ok | `REAL` | `stdlib/core/interfaces/unpack.pilot`, `selfhost/check/infer.pilot`, `selfhost/ir/lower_expr.pilot` | `heap.rs` `list_extend` (moves to Pilot `PROP-003`) | `ISplatOperator`, `IPairsOperator`, `IFlattenOperator` verified by `collections/splat_list`, `collections/pairs_map_struct`, `collections/flat_leaves`, `collections/spread_call_args`. |
| **C10. 6 Call-Quoting Projections** | `f!(x)`/`call`, `f!!(~x)`/`splice`, `f!&(x)`/`frame`, `f!^(x)`/`sig`, `f!@(x)`/`grad`, `f!?(x)`/`plan` | part | part | part | `PARTIAL` | `stdlib/core/call_expr.pilot`, `stdlib/core/signal.pilot`, `stdlib/core/dual.pilot`, `stdlib/core/effect_plan.pilot`, `selfhost/check/quote_value.pilot` | Stage-A receptor binding | `quote { e }` + `$x` splicing (`quote/quote_value`, `quote/quote_splice`, `QUOTE001`–`003`) are `REAL`; full `grad`/`sig`/`plan` receptor projection traps at `errors/err_call_quote` until M32–M35. |
| **C11. Numeric & Dimensional Suffixes** | Scalar `200u8`, `1.5f32` + SI/unit `250ms`, `100[m]` | ok | ok | ok | `REAL` | `stdlib/std/units.pilot`, `selfhost/check/infer.pilot`, `selfhost/ir/lower_expr.pilot` | `num_suffix.rs`, `lower_unit.rs` (`PROP-003`/`005`) | Range-checked scalar suffixes and `bind_unit_literal` verified across all 3 engines (`types/local_named_unit`). |
| **C12. Distributive Headers & Trailing Commas** | `pub:`, `pub static pure:`, `let T { a = 1, b = 2, };`, collection trailing `,` | ok | ok | ok | `REAL` | `selfhost/parser.pilot`, `selfhost/cst/parser.pilot` | Stage-A `ScopeHeader` | Verified by `types/field_group_defaults`; `known_bugs/struct_field_scope_label` tracked. |
| **C13. `#` Preprocessor Directives** | `#if`, `#elif`, `#else`, `#endif`, `#define`, `#region`, `#pragma` | ok | ok | ok | `PARTIAL` | `selfhost/preprocessor.pilot` | `pilot-lexer/src/lexer.rs` | Conditional compilation and `#pragma` handled before lexing (`MOD001` clean). |

---

## 3. 2-Bit Hardware Lattice, Composition & Dispatch (`C14`–`C20`)

| ID & Concept | Surface / Mechanism | S | I | C | Verdict | Owning Pilot Modules | Legacy Rust Residual | Verified Conformance, Codes & Notes |
| :--- | :--- | :-: | :-: | :-: | :--- | :--- | :--- | :--- |
| **C14. 2-Bit Hardware Type Lattice** | `0b00` `IValue`, `0b01` `IReference`, `0b10` `IObject`, `0b11` `IGuard` | ok | ok | ok | `REAL` | `stdlib/core/interfaces/lattice.pilot`, `selfhost/types/lattice.pilot`, `selfhost/check/types.pilot` | `0` (computed in Pilot) | Bitwise-OR field lattice inference verified by `memory/value_copy`, `memory/value_copy_nested`, `types/struct_copy_independent`. |
| **C15. `*Bundle` & `*Facet` Splicing** | `*Bundle` state/methods, `*Facet` methods, `coalesce`, `with { a as b }`, `without` | ok | ok | ok | `REAL` | `selfhost/check/methods.pilot`, `selfhost/types/method_table.pilot`, `selfhost/ir/lower.pilot` | `lower_members.rs` (`PROP-005`) | Verified by `types/spliced_methods`, `types/partial_facets`, `types/coalesce_bundle_field`, `types/splice_with_renamed_field`, `types/without_hidden`, `modules/err_without_member`. |
| **C16. 4-Tier `MethodTable` & `override`/`open`** | Explicit struct > `*Bundle`/`*Facet` > Sub-`interface` > Base `interface` | ok | ok | ok | `REAL` | `selfhost/types/method_table.pilot`, `selfhost/check/methods.pilot`, `selfhost/check/access.pilot` | `lower_dispatch.rs` (`PROP-005`) | Verified by `types/interface_defaults`, `types/generic_interface_calls`, `types/delegation`, `modules/open_override`, `modules/err_override_not_open` (`METH001`, `VIS001`). |
| **C17. Extensions (`ext` & `use ext`)** | `ext fn T.m()`, `ext T { ... }`, `use ext Name { ... }` | ok | ok | ok | `PARTIAL` | `selfhost/check/methods.pilot`, `selfhost/check/resolve.pilot` | `lower_members.rs` (`PROP-005`) | Verified by `types/ext_fn_single`, `types/ext_receiver_inferred`, `modules/err_priv_ext`; `known_bugs/ext_bases_dropped` tracked. |
| **C18. Variation B Properties & Indexers** | Implicit `x: T { get => field; set => { field = value; } }`, guarded `set(v) if c`, `self[i]` | ok | ok | ok | `REAL` | `selfhost/check/methods.pilot`, `selfhost/ir/lower_expr.pilot` | `accessors.rs`, `lower_members.rs` | Verified by `types/property_get`, `types/property_getters`, `types/property_setters`, `types/guarded_setters`, `types/indexer_properties`, `types/index_operators`. |
| **C19. Dunder-Free Lifecycle** | `self(params...)`, `~self()`, `scope self { enter: ... exit: ... }` | ok | ok | ok | `PARTIAL` | `selfhost/check/drops.pilot`, `selfhost/ir/lower.pilot` | `lower_struct.rs`, `lower_drop.rs` | `self(...)` and `~self()` verified across all 3 engines (`memory/drop_*`, `types/target_constructors`); `scope self` enter/exit clause weaving is `PARTIAL`. |
| **C20. Two-Axis Visibility & `lvl(N)` Clearance** | `priv`/`file`/`prot`/`pkg`/`int`/`pub`/`open`/`export` + `lvl(N)` rings | ok | ok | ok | `REAL` | `selfhost/check/access.pilot`, `selfhost/check/clearance.pilot` | `param_mods.rs` (28L) | Verified by `modules/visibility`, `modules/err_file_visibility`, `modules/prot_visibility`, `modules/k2_limits`, `types/lvl_same_or_lower` (`VIS001`, `LVL001`, `MOD003`). |

---

## 4. Memory, Arenas, Scopes, Perceus ARC & Static Proofs (`C21`–`C26`)

| ID & Concept | Surface / Mechanism | S | I | C | Verdict | Owning Pilot Modules | Legacy Rust Residual | Verified Conformance, Codes & Notes |
| :--- | :--- | :-: | :-: | :-: | :--- | :--- | :--- | :--- |
| **C21. Perceus ARC & `drop_reuse`** | Deterministic last-use drop, move-by-default (`0b10`), in-place slot reuse | ok | ok | ok | `REAL` | `selfhost/check/drops.pilot`, `selfhost/check/own.pilot`, `selfhost/ir/pass_perceus.pilot`, `selfhost/perceus/reuse.pilot` | `lower_drop.rs` consumes Pilot's `DropPlan` (`PROP-005`) | Verified across all 3 engines by 15 `memory/drop_*` & `memory/moves_clean` suites (`OWN001`). |
| **C22. Scoped Bump Arenas (`ArenaScope`)** | `with arena a { ... }`, `arena(cap)`, `0b11` non-escaping reset/free | ok | ok | ok | `REAL` | `stdlib/core/arena.pilot`, `selfhost/check/escape.pilot`, `selfhost/check/drops.pilot` | `0` (`pilot-arena` & `memory.rs` deleted in `PROP-001`) | Verified by `memory/arena_scope` and `errors/err_arena_escape` (`ESCAPE001`–`004`). |
| **C23. Lexical Borrow, Aliasing & `iso` Proofs** | `ref` (`in`), `var ref` writeback, disjoint aliasing (`ALIAS001`), `iso` uniqueness | ok | ok | ok | `REAL` | `selfhost/check/borrows.pilot`, `selfhost/check/aliasing.pilot`, `selfhost/perceus/escape.pilot` | `0` (100% checked in Pilot) | Verified by `memory/ref_fields`, `memory/var_ref_writeback`, `memory/const_ref_read`, `memory/alias_iso_params`, `memory/escape_views_clean`, `errors/err_alias_var_ref`. |
| **C24. `pure` & `noalloc` Static Effect Proofs** | Transitive call-graph verification (`PURE001`, `NOALLOC001`) | ok | ok | ok | `REAL` | `stdlib/core/native_effects.pilot`, `selfhost/check/purity.pilot`, `selfhost/check/noalloc.pilot`, `selfhost/check/effects.pilot` | `0` (100% checked in Pilot) | Verified by `contracts/noalloc_fns`, `contracts/const_effects`, and `errors/err_noalloc_*` / `errors/err_pure_*`. |
| **C25. Zero-Copy Bit Views & Checked Conversions** | `x.as_u64()` (`view.pilot`), `x.to_u8()` $\rightarrow$ `u8 ! ConvertError` (`convert.pilot`) | ok | ok | ok | `REAL` | `stdlib/core/view.pilot`, `stdlib/core/convert/convert.pilot`, `stdlib/core/convert/convertible.pilot` | Float string parse seam (`convert_float_parse_seam`) | Verified by `numbers/as_view_widths`, `numbers/as_view_slices`, `modules/convert_int`, `modules/convert_float`, `modules/convert_bool_char`. |
| **C26. Packed Struct Layouts & `[Bits]`/`[Layout]`** | Bit-level field packing, alignment (`ALIGN001`), `sizeof`/`offsetof` | ok | ok | ok | `PARTIAL` | `selfhost/ir/layouts.pilot`, `selfhost/codegen/physics.pilot`, `selfhost/check/types.pilot` | `pilot-analysis/src/types.rs` (dead crate `PROP-004`) | Verified by `types/packed_fields`, `types/layout_attrs`, `types/layout_align_wide`, `types/sizeof_struct`; `alignof`/`offsetof`/`fields_of` queued in K4 (`errors/err_alignof_query`). |

---

## 5. Control Flow, Failure Values, Generators & Metaprogramming (`C27`–`C33`)

| ID & Concept | Surface / Mechanism | S | I | C | Verdict | Owning Pilot Modules | Legacy Rust Residual | Verified Conformance, Codes & Notes |
| :--- | :--- | :-: | :-: | :-: | :--- | :--- | :--- | :--- |
| **C27. Values-First Errors & Postfix `?`** | `ITryOperator` (`opt<T>` / `res<T, E>`), `??`, `guard`, `try`/`catch`, `errdefer` | ok | ok | ok | `REAL` | `stdlib/core/option.pilot`, `stdlib/core/result.pilot`, `stdlib/core/interfaces/try_op.pilot`, `selfhost/ir/lower_expr.pilot` | `lower_try.rs`, `lower_defer.rs` (`PROP-005`) | Verified by `control/try_catch`, `control/catch_failure`, `control/guard_try`, `control/coalesce`, `control/errdefer`, `memory/drop_try_exit`. |
| **C28. Exhaustive `switch` & Destructuring** | Tuple/record/slice/enum payload patterns, `if` guards, `let { a, b } = ...` | ok | ok | ok | `REAL` | `selfhost/check/exhaustiveness.pilot`, `selfhost/check/patterns.pilot`, `selfhost/ir/lower_stmt.pilot` | `lower_switch.rs`, `lower_destructure.rs` (`PROP-005`) | Verified by `control/switch`, `control/switch_case_list`, `control/default_arms`, `collections/destructuring`. |
| **C29. 3-Tier Dynamic `for` & Collecting `for`** | Scalar/stepped ranges, `get_enumerator()`, `for ... { yield x; }`, `then` fallback | ok | ok | ok | `REAL` | `stdlib/core/range_window.pilot`, `selfhost/ir/lower_for.pilot`, `selfhost/ir/lower_generator.pilot` | `lower_for.rs`, `lower_yield.rs` (`PROP-005`) | Verified by `control/for_loops`, `control/for_range_step`, `control/for_enumerator`, `control/for_then`, `generators/collecting_for`. |
| **C30. Resumable `yield` Generators & `become` Tail Calls** | `Generator<TItem, TSent>`, `yield*`/`yield from`, constant-stack `become f(...)` | ok | ok | ok | `PARTIAL` | `stdlib/core/generator.pilot`, `selfhost/ir/lower_generator.pilot`, `selfhost/check/tail.pilot` | `vm_gen.rs` (`PROP-005`) | Generators (`generators/generator_*`) & `become` (`control/become_tail`, `TAIL001`–`003`) pass on all 3 engines; ledger `DRAFT` for full bidirectional `send` state machines. |
| **C31. Design-by-Contract & Compiler Hints** | `requires(c)`, `ensures(r => c)`, `old(e)`, `invariant`, `assume(c)` | ok | ok | ok | `PARTIAL` | `stdlib/contracts/contracts.pilot`, `stdlib/core/hint.pilot`, `selfhost/check/probes.pilot` | `contracts.rs`, `lower_contract.rs` (`PROP-003`/`005`) | `requires`, `ensures`, `old`, `assume` verified (`contracts/contracts`, `contracts/ensures_holds`, `contracts/assume_facts`); struct `invariant` stops at `contracts/invariant_seam`. |
| **C32. `[...]` Weighted Attribute Pipeline** | `[Attr1, Attr2]` sorted by `weight: i32` (`IStepAttribute`: `::Continue`/`::SkipNext`/`::Stop`) | ok | ok | ok | `REAL` | `stdlib/core/attr_flow.pilot`, `stdlib/core/attributes.pilot`, `selfhost/check/probes.pilot` | `0` (planned in `selfhost/check/probes.pilot`) | Verified on all 3 engines by `types/attr_coverage`, `types/attr_debug`, `types/attr_inherit_debug`. |
| **C33. Homoiconic `syntax` Macros & `quote { $x }`** | `syntax name(...) => quote { ... }`, typed `ast<T>` values, LINQ `from..select` | ok | ok | ok | `PARTIAL` | `stdlib/core/primitives/ast.pilot`, `selfhost/syntax/macro_expander.pilot`, `selfhost/check/quote_value.pilot`, `selfhost/ir/lower_quote.pilot` | `rule_arms.rs`, `template.rs`, `query.rs` (`PROP-003`/`006`) | `control/syntax_macro_expr`, `quote/quote_value`, `quote/quote_splice`, `collections/query_*` pass across engines; hygiene bug `known_bugs/rule_template_capture` tracked. |

---

## 6. Universal Foreign Adapter, Transport DOMs & `.pchor` Choreography (`C34`–`C37`)

| ID & Concept | Surface / Mechanism | S | I | C | Verdict | Owning Pilot Modules | Legacy Rust Residual | Verified Conformance, Codes & Notes |
| :--- | :--- | :-: | :-: | :-: | :--- | :--- | :--- | :--- |
| **C34. Foreign Absorbers & `adapter A -> B`** | `use c`/`cpp`/`rust`/`py`/`ts`/`wasm`/`sql`/`proto`/`openapi`, `AdapterBlueprint` | part | part | part | `SEAM` | `selfhost/adapter/*` (`28` files), `stdlib/polyglot/*`, `stdlib/std/polyglot/prog/*` | `c_abi.rs` `dlopen`/`dlsym` moving to `bridge.rs` (`PROP-002`) | Pure-Pilot absorbers & `AdapterBlueprint` planners exist (`selfhost/adapter/`); surface `adapter` keyword traps at `errors/err_adapter_unbuilt` until parser-to-`selfhost/adapter` wiring lands. |
| **C35. Transport DOMs (JSON, YAML, XML, TOML)** | Bidirectional `JsonValue`, `YamlNode`, `XmlNode`, `TomlValue` (`0b00`/`0b10`) | ok | ok | ok | `REAL` | `stdlib/std/polyglot/transport/{json,yaml,xml,toml}.pilot`, `stdlib/std/serde/*` | `0` (100% pure Pilot) | RFC 8259 JSON & Unicode escapes verified (`text/json_rfc8259`, `text/json_unicode_escapes`) + full DOM query/mutate/emit in `std.polyglot.transport.*`. |
| **C36. `.pchor` Sublanguage & Layout Choreographer** | `SoA<T>`, Bit-Packed/Arena rewrites, `cue` **2-Bit Boundary Law** (`CHOR001`) | tool | tool | tool | `PARTIAL` | `selfhost/chor/{lexer,parser,pass,verifier}.pilot`, `stdlib/pilot/chor.pilot` | `0` (100% pure Pilot) | Implemented in `selfhost/chor/` (`pilot chor`); end-to-end automatic `.pchor` rewrite on `pilot build` tracked in `wiring.pilot`. |
| **C37. Hard Real-Time (`[Wcet]`, `deadline`) & `async` Tasks** | Cycle-bounded `noalloc` paths, work-stealing `Task<T>`, channels (`std.chan`) | seam | seam | seam | `SEAM` | `stdlib/asyncio/*`, `stdlib/realtime/*`, `stdlib/std/{chan,sync}.pilot`, `selfhost/perf/wcet.pilot` | Dead `pilot-rt/src/concurrency.rs` deleted in `PROP-001` | Library modules exist in Pilot; surface `async`/`await`/`spawn`/`deadline` keywords trap loudly at `SEAM001` until M36/M40 state-machine lowering lands. |

---

## 7. Compiler-as-a-Library (`PilotDb`, Lossless CST) & Unified Toolchain (`C38`–`C40`)

| ID & Concept | Surface / Mechanism | S | I | C | Verdict | Owning Pilot Modules | Legacy Rust Residual | Verified Conformance, Codes & Notes |
| :--- | :--- | :-: | :-: | :-: | :--- | :--- | :--- | :--- |
| **C38. Incremental `PilotDb` & Module Loader** | Red-green content-addressed query memoization, `pkg`/`use` DAG | tool | tool | tool | `MIGRATING` | `selfhost/db/{query_db,persistence}.pilot`, `selfhost/loader/module_graph.pilot`, `stdlib/pilot/db.pilot` | `parse_header` in `modules.rs`/`host.rs` (`PROP-003`) | Multi-unit linking verified across all 3 engines (`modules/modules`, `modules/start_order`, `modules/err_import_cycle`). |
| **C39. Lossless Green/Red CST, `fmt`, `--fix` & `migrate`** | 100% trivia/comment conservation, Wadler-Lindig formatter, `[Migrate]` codemods | tool | tool | tool | `REAL` | `selfhost/cst/{tree,lexer,parser,autofix}.pilot`, `selfhost/fmt/*`, `selfhost/migrate/migrator.pilot` | `0` (100% pure Pilot) | Executed via `pilot fmt`, `pilot check --fix`, and `pilot migrate` with comment-conservation checks. |
| **C40. Unified `pilot` CLI, LSP, DAP, `inspect`, `mcp` & `gate`** | Open `CommandRegistry`, JSON-RPC IDE/AI servers, 3-Gate repository verifier | tool | tool | tool | `REAL` | `selfhost/cli/*`, `selfhost/lsp/*`, `selfhost/dap/*`, `selfhost/inspect/*`, `selfhost/mcp/*`, `selfhost/gate/*`, `tools/*` | Thin `.pilc` bootstrap in `crates/pilot-cli/` (`6` files) | Every subcommand routes through `selfhost/cli/main.pilot`; `pilot gate --full` enforces `MOD001`, `ENGINE001`, `SEAM001`, and `keyword_matrix.md`. |
