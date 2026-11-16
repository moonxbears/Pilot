# Language Vocabulary Still in Rust

**Inventory, 2026-11-16.** This is every place where `crates/*/src` hard-codes language vocabulary that [`self_description.md`](self_description.md) moves into Pilot declarations. Each row was checked against the code.

It excludes:
- `crates/pilot-analysis`, which is being retired (`PROP-004`);
- tests;
- pure byte-scanning mechanics.

The VM's opcode dispatch is grouped per file, because it leaves with the Rust `BytecodeCompiler` in `PROP-005`.

**Layers:**
- **Spelling:** which characters make up a token or literal.
- **Shape:** which tokens or receptor codes form which construct, including binding powers.
- **Meaning:** which token, word or flag maps to which operation.

"Receptor N" is a numeric category code. The *word* comes from Pilot through `bind_receptor`, but what the code number means is fixed in Rust.

**Line counts:** "Lines" usually counts a whole function, so the totals are an upper bound. This is especially true for Shape. A gate that counts these rows and may only go down is planned (`self_description.md`, "How ownership is proved"); it is not built.

All paths are under `crates/`.

## Spelling

| File:lines | Hard-coded | Lines | Moves in |
| :--- | :--- | ---: | :--- |
| `pilot-lexer/src/lexer.rs:58-64` | newline is its own token | 7 | PROP-021 |
| `pilot-lexer/src/lexer.rs:66-89` | comments: `#` to end of line, `#!` shebang at offset 0, `//` (always, since PROP-024), `/* */` | 24 | PROP-021 |
| `pilot-lexer/src/lexer.rs:91-119` | string prefixes `rf`/`fr`/`f`/`r`/`b`, any case; quotes `"` and `'` | 29 | PROP-021 |
| `pilot-lexer/src/lexer.rs:267-298` | triple quote → `TripleStrLit` | 32 | PROP-021 |
| `pilot-lexer/src/lexer.rs:121-125, 300-367` | numbers: `0x`/`0b`/`0o`, separator `_`, fraction `.`, exponent `e`/`E[+-]` | 73 | PROP-021 |
| `pilot-lexer/src/lexer.rs:369-379` | dimensional suffix: an adjacent alphabetic run that is not a seed word | 11 | PROP-021 |
| `pilot-lexer/src/lexer.rs:127-171` | `::`, `:=`, `:=:`, and the `:name[/ns]` keyword literal (only after whitespace) | 45 | PROP-021 |
| `pilot-lexer/src/lexer.rs:173-183` | a name starts with a letter or `_`; which tokens may precede an infix dot | 11 | PROP-021 |
| `pilot-lexer/src/lexer.rs:185-231` | `!=`, the call-quote forms `!(` `!!(` `!&(` `!^(` `!@(` `!?(`, bare `!` | 47 | PROP-021 |
| `pilot-lexer/src/lexer.rs:233-257` | `..=`, `..<`, `...`, `..`; the infix-dot vs prefix-dot rule | 25 | PROP-021 |
| `pilot-lexer/src/lexer.rs:385-476` | `scan_operator_or_delimiter`: about 75 operator and delimiter arms | 92 | PROP-021 |
| `pilot-lexer/src/lexer.rs:508-535` | bracket pairs; which kinds are trivia | 28 | PROP-021 |
| `pilot-lexer/src/keywords.rs:11-29` | the 14 seed words | 19 | PROP-021 |
| `pilot-lexer/src/token.rs:10-147, 159-166` | `TokenKind`, one variant per spelled token; `TokenFlags` | 146 | PROP-021 |
| `pilot-lexer/src/simd.rs:3-26` | whitespace set; identifier class `[A-Za-z0-9_]` | 24 | PROP-021 |
| `pilot-lexer/src/simd.rs:57-99` | comment ends, `\` escape skip, string ends, triple-quote close | 41 | PROP-021 |
| `pilot-lexer/src/simd.rs:101-118` | `string_prefix_flags` (repeats the lexer's rule) | 18 | PROP-021 |
| `pilot-lexer/src/simd.rs:120-217` | f-string hole scanning: `{`…`}` depth, `{{`/`}}`, nested strings and comments | 98 | PROP-021 |
| `pilot-parser/src/int_lit.rs:23-41` | strips `_`; skips a 2-character prefix; bases 10/16/2/8 | 19 | PROP-021 |
| `pilot-parser/src/num_suffix.rs:115-164` | `suffix_start` and `literal_value` re-derive the number grammar | 49 | PROP-021 |
| `pilot-parser/src/expr.rs:359-379` | literal token → base; float `replace('_', "")` + `unwrap_or(0.0)` at :376 | 21 | PROP-021 |
| `pilot-parser/src/tuple_member.rs:15-40` | `t.0.1` lexed as float `0.1`, split on `.` | 26 | PROP-021 |
| `pilot-parser/src/fstring.rs:16-108, 145-338` | f-string prefix, body, `{{`/`}}`, holes, `:` format spec, pattern alphabet `# 0 , _ .` (third copy of the f-string rules) | 275 | PROP-021 |
| `pilot-parser/src/fstring.rs:341-366` | `unescape_str`: `\n \t \r \\ \" \'` | 26 | PROP-021 |
| `pilot-parser/src/stmt.rs:342-349, 690-699` | raw DSL block fences and default `"dsl"`; quoted record-pattern keys | 18 | PROP-021 |
| `pilot-parser/src/decl.rs:1314-1354` | `op` holes: `_` not part of a word; prefix member name `"{text}_"` (PROP-018) | 41 | PROP-022/023 |
| `pilot-parser/src/decl.rs:6-11` | a source is ingested if it contains `"__engine"`, `"comptime"` or `'['` | 6 | PROP-023 |
| `pilot-parser/src/lib.rs:386-399` | trivia kinds skipped by `next_non_nl` | 14 | PROP-023 |

**Totals:** 33 rows (some merged above), about 1,265 lines.

## Shape

| File:lines | Hard-coded | Lines | Moves in |
| :--- | :--- | ---: | :--- |
| `pilot-parser/src/tables.rs:92-109, 214-261` | shape ordinals (`PREFIX=1` … `ADJACENT_CALL=9`), `ASSOC_RIGHT=1`, `LEVEL_BASE=10`, `LEVEL_STEP=4`, `CLAUSE_BP=11`; `level_bp` | 66 | PROP-023 (data in `LANG`) |
| `pilot-parser/src/tables.rs:615-641` | `is_structural_symbol`: the structural kinds and seed words | 27 | PROP-023 |
| `pilot-parser/src/tables.rs:811-913` | `compile_type_rule`: type prefixes `*T`/`@T`, postfixes, and infix powers `&` 20, `\|` 15, `=>`/`->` 10 right, `!` 5 | 103 | PROP-019 |
| `pilot-ast/src/lib.rs:299-335` | `ExprPrecedence` (used only by a test at :913, so dead) and `TypePrecedence` | 36 | PROP-019 (delete) |
| `pilot-parser/src/stmt.rs:11-229`, `ret_type.rs:12-47` | type parsing: `$` holes, tuple types, paths, `<..>` arguments at `BitOr`'s power (:90), postfixes with literal `parse_type_bp(31)` (:167), infix map `& \| -> ! =>`, `ABOVE_FN_ARROW=11` | 252 | PROP-019 |
| `pilot-parser/src/expr.rs:14-31` | tokens that continue an expression across a newline (`.`, `?.`, `\|>`, `?..`, `..`) | 18 | PROP-023 |
| `pilot-parser/src/expr.rs:32-46, 923-967`, `generic_receiver.rs:24-44` | `<..>` in expressions: token whitelist, `>>` as two closes | 81 | PROP-023 |
| `pilot-parser/src/expr.rs:49-149` | struct literal; member access `.`, `?.`, `::`; calls and call quotes; newline breaks; index `[ ]` with open range | 123 | PROP-022 |
| `pilot-parser/src/expr.rs:178-235` | cascade `..`/`?..` (literal power `2`), cascade-vs-range heuristic; `catch` (receptor 19) | 90 | PROP-022 |
| `pilot-parser/src/expr.rs:316-341, 353-465` | Pratt infix loop by shape ordinal; `yield`, `quote`, `comptime`, `$name`, implied receiver `"_self"` (:442) | 115 | PROP-022 |
| `pilot-parser/src/expr.rs:466-522, 572-918` | prefix-operator start tokens; receptor-37 call-quote words; call arguments; lambda vs tuple; list, map and brace-map literals (receptors 15/16/21/22 excluded) | 364 | PROP-022 |
| `pilot-parser/src/stmt.rs:232-399` | statement starts: receptor 33; `return`, `yield`, `defer`, `loop`, `if`, `asm`; the "Unregistered keyword" rule (receptors 19/29/31) | 138 | PROP-022/023 |
| `pilot-parser/src/stmt.rs:456-495`, `comptime_if.rs:24, 107, 256-262` | `if`/`else` (`ELSE_RECEPTOR=3`), `comptime if` | 50 | PROP-022 |
| `pilot-parser/src/stmt.rs:527-721` | patterns: literal power `36`, ranges, `\|`, `"_"` wildcard, tuple/record/list (splat flag 2), variants (uppercase initial or receptor (31,2)), record fields | 192 | PROP-022 |
| `pilot-parser/src/decl.rs:42-300` | attribute syntax; modifier loop and its follow-token sets; header targets `fn`=1 `struct`=2 `interface`=3 receptor 27=4 | 259 | PROP-023 |
| `pilot-parser/src/decl.rs:393-481, 701-946` | type-body items by receptor (4 syntax, 7 extern, 26 adt, 27 enum, 25 op, 20 scope); parameters; `where` (28); scope enter/exit (21/22) | 240 | PROP-023 |
| `pilot-parser/src/decl.rs:1000-1151, 1275-1311`, `accessors.rs:14-93` | `op` declarations and `INDEX_GET_OP`/`INDEX_SET_OP`; `*Bundle` splices (receptors 29/30); `extern` (receptor 8); `get`/`set` (`GETTER_RECEPTOR=23`, `SETTER_RECEPTOR=24`) | 204 | PROP-022 |
| `pilot-parser/src/generic_params.rs:19-63` | generic parameters; `const` taken only before the name | 45 | PROP-020 |
| `pilot-parser/src/op_decl.rs:15-65`, `declared_names.rs:16-36` | `op` declaration lookahead; assignment token set (lacks `**=`) | 72 | PROP-022 |
| `pilot-parser/src/modules.rs:8, 62-100`, `contracts.rs:21-94`, `loud.rs:44-169` | `use` paths (receptors 29/30); `requires`/`ensures`/`old` (33 with 18/19/24); `*args`, `**kwargs`, `where` | 145 | PROP-022/023 |
| `pilot-parser/src/rule_arms.rs:14, 179-400, 543-1020`, `decl_syntax.rs:9-60`, `pilot_impls.rs:55, 161-163` | rule meta-syntax `$name`, `$( .. )?`, `$( .. ) sep *`, `$name:Kind`; hole kinds 0..7; `syntax` heads (receptors 5/6); symbol quote (33,25) | 181 | PROP-023 |
| `pilot-parser/src/lib.rs:194-217` | `is_seed_kw`, the 14 seed words a second time | 24 | PROP-023 |

**Totals:** 49 rows (merged above), about 2,815 lines. This layer is the most overstated.

## Meaning

| File:lines | Hard-coded | Lines | Moves in |
| :--- | :--- | ---: | :--- |
| `pilot-parser/src/tables.rs:643-703` | `binop_code`, `map_binop_sym`: 42 symbols → `BinaryOp` | 61 | PROP-022 |
| `pilot-ast/src/lib.rs:287-297, 337-513` | `CallQuoteKind`; `BinaryOp` (1–49) and `symbol()`, including the words `and or is union intersect except as` and `as view`; the unary flag constants | 185 | PROP-022 |
| `pilot-parser/src/expr.rs:150-177` | postfix `?` → `TryPropagateExpr`; postfix `!` → NULL001 | 28 | PROP-022 |
| `pilot-parser/src/expr.rs:236-315, 342-344` | `in` (receptor (10,0)); raw `23\|24` range codes (:255, :342); `as`/`as view` (receptor 29) at literal `parse_type_bp(30)` | 83 | PROP-022 |
| `pilot-parser/src/expr.rs:523-571, 630-644, 1003-1015` | readable prefix word → symbol → `prefix_flag`; receptor 1 bool, 2 none, `old`, `some`; call-quote token → kind | 73 | PROP-022 |
| `pilot-parser/src/stmt.rs:316-329, 428-455, 498-526, 726-742` | `loop` → while-true; `some x = v`; `assign_op_of` raw codes (`=` 0, `+=` 1, … `??=` 22, **no `**=`**) | 88 | PROP-022 |
| `pilot-parser/src/open_range.rs:14-30`, `loud.rs:147-157` | `..=` vs half-open; `a..b : step` → `BinaryOp::Step` | 28 | PROP-022 |
| `pilot-parser/src/fstring.rs:110-143` | f-string → chain of `BinaryOp::Add`; `:pattern` → the format-spec function | 34 | PROP-021/022 |
| `pilot-parser/src/num_suffix.rs:13-17, 27-96` | register classes `gpr_signed`, `gpr_unsigned`, `simd_float` decide a type-suffix cast vs a unit literal | 75 | PROP-021 |
| `pilot-parser/src/comptime_if.rs:26-29, 194-252` | unary flags again; constant folding of And/Or/Add/…/Ge | 63 | PROP-022 |
| `pilot-parser/src/forms.rs:11-135` | `CoreForm` ordinals → node builders | 125 | stays as the fixed core (by ordinal) |
| `pilot-parser/src/decl.rs:246-269, 728-747` | modifier actions → declaration and parameter flags | 44 | PROP-023 |
| `pilot-parser/src/tables.rs:39-52, 86-90, 415, 480-602`, `pilot_impls.rs:29-72`, `attr_hooks.rs:26, 122` | engine hook and action names (`register_modifier`, `bind_receptor`, `HOOK_*`, `"__engine"`) | 170 | stays: the frozen seed↔Pilot contract (airlock) |
| `pilot-vm/src/lower_expr.rs` (many sites) | range constants, unary flags, `direct_opcode`, `is`/`as`, short-circuit, `??`, pipes, operator-method dispatch by `op.symbol()` (`!=` falls back to `==` negated), splat | 180 | PROP-005 |
| `pilot-vm/src/lower_members.rs`, `lower_for.rs`, `lower_args.rs`, `lower_yield.rs`, `lower_contract.rs`, `lib.rs:1033` | splat, index names, range, step, `old` flags | 28 | PROP-005 |
| `pilot-vm/src/ops.rs:61-86` | opcode → symbol strings | 26 | PROP-005 |

**Totals:** 33 rows (merged above), about 1,289 lines: about 1,055 in the parser and AST, about 234 in the VM.

## Findings

- **Duplicated rules:**
  - The number grammar is written three times: `lexer.rs`, `num_suffix.rs` and `int_lit.rs`.
  - The f-string rules are written three times: `lexer.rs`, `simd.rs` and `fstring.rs`.
  - The seed-word list is written twice: `keywords.rs` and `lib.rs` `is_seed_kw`.
  - The unary flags are written three times: `expr.rs`, `comptime_if.rs` and `pilot-vm` `lower_expr.rs`.

  Declaring each rule once in Pilot removes the copies.
- **Known bug `seed.compound_pow_assign`:** `x **= 3` leaves `x` unchanged, because `assign_op_of` has no `**=` entry (`tests/conformance/known_bugs/compound_pow_assign`). `//=` no longer exists: `//` always starts a comment (owner, 2026-11-16; `PROP-024`, built).
- **Raw `BinaryOp` numbers in the parser:** `expr.rs:255` and `:342` write `23|24`, and `assign_op_of` writes codes, instead of naming the enum.
- **`ExprPrecedence` is dead:** only a test reads it (`pilot-ast/src/lib.rs:913`). It belongs on the deletion list of PROP-019.
- **The airlock hook names and the `CoreForm` builders stay by design:** the hooks are the frozen contract and are read by ordinal or name, and the core forms are the small fixed core.
