# Pilot Documentation

Pilot's design is ahead of its implementation, so this page says which files describe the design and which record what actually runs. When they disagree, the records win.

## What runs today (records, kept current with every change)

| File | What it records |
| :--- | :--- |
| [`keyword_matrix.md`](keyword_matrix.md) | Every registered keyword: its grammar binding, tests, and status on the seed VM, PilIR and generated C. Checked by `pilot run tools/keyword_matrix/main.pilot -- --check`. |
| [`concept_matrix.md`](concept_matrix.md) | Every language, runtime and toolchain concept beyond single keywords, with per-engine status and whether Pilot or Rust owns it. |
| `selfhost/gate/ledger/wiring.pilot` | Capability maturity (SPEC, DRAFT, CORE, WIRED, COMPLETE) with the run that proves each level. |
| `selfhost/gate/ledger/seams.pilot` | Every registered seam (unbuilt piece that traps with `SEAM001`), stand-in and known bug. |
| [`../pilot.md`](../pilot.md), [`../rust.md`](../rust.md) | Maps of every Pilot file and every remaining Rust seed file. |
| `tests/conformance/` | The executable specification: each `<name>.pilot` with its expected `.stdout` / `.stderr` / `.exit`, run by `pilot test` on every engine. The best source for real, current syntax. |

## Plans and design

| File | What it holds |
| :--- | :--- |
| [`roadmap.md`](roadmap.md) | The order of all proposals, the Rust each step deletes, open owner decisions and the to-fix-later list. |
| [`../proposals.md`](../proposals.md) | Every proposal and the owner approval each Rust change needs. |
| [`../VISION.md`](../VISION.md) | The target design. Describes the language as it is meant to be, not as it is. |
| [`../MILESTONES_100.md`](../MILESTONES_100.md) | The long-range milestone plan (M1 to M100). |
| [`../imp_type.md`](../imp_type.md) | How to choose types: the 2-bit lattice and the type selection rules. |
| [`self_description.md`](self_description.md) | Design for moving language vocabulary out of the Rust seed into Pilot declarations. |
| [`keyword_roadmap.md`](keyword_roadmap.md) | The order in which unbuilt keywords get built. |
| [`vocabulary_in_rust.md`](vocabulary_in_rust.md) | Inventory (2026-11-16) of Pilot vocabulary still written in Rust. |

## Rules

[`../AGENTS.md`](../AGENTS.md) holds the rules for every contributor and AI coder: syntax rules (section 2), the Rust approval gate, the definition of done (section 9.1) and the owner's working rules (section 12).

## Syntax in brief

There is no separate syntax reference yet; the old tutorial, syntax and lexical references and `docs/concepts/` were deleted on 2026-11-18 because they described an earlier design (`def`, `int`, `except`). Use AGENTS.md section 2, the declarations in `stdlib/core/keywords.pilot` and `stdlib/core/syntax/`, and the conformance tests.
