# Pilot Describes Itself

**Status: design, 2026-11-16.** It records the owner's direction ("I want pilot to understand itself completely without help outside") and the plan to reach it. Proposals: `PROP-019`, `PROP-020`, `PROP-021`, `PROP-022`, `PROP-023` in [`proposals.md`](../proposals.md). What is still in Rust is listed in [`vocabulary_in_rust.md`](vocabulary_in_rust.md).

## The rule

**The seed knows mechanisms; Pilot declares vocabulary.** The seed (Rust today, C11 from `M46`) scans bytes, matches tables, builds nodes and runs a small fixed core. Every character class, word, symbol, precedence, literal form and meaning is declared in Pilot. The seed only reads those declarations.

## Four layers

Every construct is declared once in each layer that applies.

| Layer | Pilot declares | Home | Proposal |
| :--- | :--- | :--- | :--- |
| **1. Spelling** | every token as a **pattern** (`lit`, `class`, `seq`, `alt`, `opt`, `many`) with a `[Token]` attribute: numbers (the digit separator is legal only where the pattern puts it), strings, comments, names, the 13 structural delimiters, and operator symbols collected from `op` declarations. Pilot compiles the patterns into a state table (DFA plus a few declared modes, such as f-string holes), and the seed only runs the table | `stdlib/core/lex/`, `selfhost/lex/build.pilot` | `PROP-021` (subsumes `PROP-008` step 2) |
| **2. Shape** | operator fixity from `_` holes (`_+_`, `-_`, `_?`, `_[_]`, `_[_]=_`; without holes one parameter is infix and none prefix, anything else must write its holes); levels and associativity as ordered enums (`OpLevel`, `TypeLevel`); `syntax` rules whose sequence heads give each hole a kind (`HoleKind`); the built-in statements as `CoreForm` | `stdlib/core/operators/`, `stdlib/core/syntax/` | `PROP-008`, `PROP-009`, `PROP-018`, `PROP-019` |
| **3. Meaning** | every construct lowers to exactly one of: its **member call** (`a + b` calls `+`, `*x` calls `*_`, `a[i] = v` calls `_[_]=_`); its **rule's template** (`x?` through `try` over the declared protocol); a **core form** or **machine op** (`CoreForm`, `MachineOp`). A literal means a call too: a type declares the literal family it accepts and the Pilot function that builds a value from the text | `stdlib/core/operator.pilot`, `stdlib/core/syntax/`, `stdlib/core/asm/` | `PROP-021`, `PROP-022` |
| **4. Checking** | diagnostics (`selfhost/diagnostics/errors.pilot`), checker passes (`selfhost/check/`), seams and known bugs (`selfhost/gate/ledger/`) | already Pilot | — |

### What goes where

- **An operator whose operands are all values is an `op`.** It is a member of its first operand's type. Its holes are plain `_` and have no kind (`GRAM010`).
- **A construct that takes a type, block, name or pattern is a `syntax` rule.** Its sequence head gives each hole its kind (`$t:Type`, `$b:Block`).
- **Type-position operators** (`T?`, `T | U`, `A -> R`) are `syntax type_construct` rules, with levels from `TypeLevel` (`PROP-019`).
- **`_` means an operand, never part of a symbol.** A `_` that touches a word character belongs to that word (`max_len`, `1_000`). Every other `_` is a hole.

## Bootstrap

Something has to be read before any table exists, so the bootstrap runs in stages.

1. **Genesis.** The token pattern files, `stdlib/core/lex/*.pilot` (later `stdlib/core/genesis.pilot`), use only the 14 seed words, plain decimal numbers and plain strings. Today's Rust lexer reads them once to build the first state table; after that, each table is built by the toolchain the previous one lexed.
2. **Multi-pass build.** Each later file is read with the tables collected before it. This is the bootstrap `PROP-008` introduced (`build_bootstrap_tables` in `crates/pilot-parser/src/lib.rs`).
3. **Precompiled tables.** The `LANG` section of `toolchain.pilc` (`PROP-023`) carries every table, already compiled. The seed reads data by number and never reads `.pilot` text, so what remains of the Rust parser is deleted.

## How ownership is proved

1. **Litmus.** Each change below is made in Pilot only, and the tree must build and give the same output on `pilot test`, `--ir` and `--emit c` with no Rust change. `pilot gate --litmus` extends to each one.
   - Rename a primitive (AGENTS.md §1.4, already gated).
   - Change the digit separator (`1'000'000`, `PROP-021`).
   - Respell an operator symbol (`PROP-022`).
   - Swap a keyword (keyword matrix).
2. **Ratchet.** A gate counts the language-vocabulary literals in Rust (`b'_'`, `"[]"`, `"?"`, token match arms, binding powers), the rows of [`vocabulary_in_rust.md`](vocabulary_in_rust.md). The count may only go down, as `paused_max_total_rust_loc()` and the `__engine` call sites do. Its target is 0.
3. **Byte-identical tables.** The gate rebuilds the `LANG` section and compares it with the shipped one (`PROP-023`).

## Order

1. Inventory: [`vocabulary_in_rust.md`](vocabulary_in_rust.md).
2. `PROP-020`: a parser bug PROP-018 surfaced (`const` generic parameters). It is small, and the bootstrap test needs it.
3. `PROP-019`: type-operator levels.
4. `PROP-021`: the lexer as Pilot token patterns compiled to a state table. This includes operator symbols, which were `PROP-008` step 2. Both hand-written lexers are deleted (`crates/pilot-lexer`'s scanners and `selfhost/lexer.pilot`).
5. `PROP-005` phase 5D: bytecode from PilIR, so operator lowering leaves `crates/pilot-vm`.
   - `PROP-025`, in parallel: the parser in Pilot, driven by the declared grammar and reading `PROP-021`'s tokens. The checker and PilIR switch to its AST.
6. `PROP-022`: operator meaning as member calls and templates.
7. `PROP-023`: the genesis file and the precompiled `LANG` section. With `PROP-025` done, the Rust lexer, parser and AST crates are deleted (`PROP-006`). What remains in Rust is the `.pilc` loader, the dispatch loop, memory and the frozen `HostOp`s.

8. Later, after `M46`: a direct-code lexer. The same token patterns are emitted as straight-line code instead of a table (`PROP-021` §2.10).

Each step needs owner approval before any Rust file changes (`proposals.md`).

## Ownership first, then speed

Some choices here are made so that Pilot owns its definition, not because they are fastest. Each one is labelled so, together with how its speed comes back. Example: the lexer is a generated state table (`PROP-021`). That is linear but slower than a hand-written lexer, and it is the simplest form that keeps every rule in Pilot. The fast form, straight-line code generated from the same patterns, comes later and is generated, never hand-written. Recording a token as a span and computing its value afterwards is not one of these trade-offs: it is the standard approach, and already efficient.
