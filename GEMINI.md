# Pilot: instructions for Gemini / Antigravity agents

All rules for this repository live in AGENTS.md, imported below so they load with this file. The owner's working rules are its section 12 (capped runs, the 60-second test limit, git safety, the scorecard, priorities). Nothing here adds to or overrides it.

@AGENTS.md

## Where things stand and where they are going (read before choosing work)

This list only says where to look. It adds no rules.

1. **Direction:** [`VISION.md`](VISION.md) (what Pilot is for) and [`MILESTONES_100.md`](MILESTONES_100.md) (the milestones). Both describe the design the project is aiming for, not what works today (AGENTS.md 11.1).
2. **Order of work:** [`docs/roadmap.md`](docs/roadmap.md) orders every proposal, lists the Rust each step deletes, and holds the owner's to-fix-later list (AGENTS.md 12.4.4).
3. **Decisions and Rust changes:** [`proposals.md`](proposals.md). Its status column records what the owner has approved. A Rust file may change only under an approved proposal, and only the files it names.
4. **What actually works:** `selfhost/gate/ledger/wiring.pilot` (each capability's level, with run evidence), `selfhost/gate/ledger/seams.pilot` (every place that stops on purpose, and the known bugs), [`docs/concept_matrix.md`](docs/concept_matrix.md) and [`docs/keyword_matrix.md`](docs/keyword_matrix.md). Trust a real run over any of these, and fix a document that disagrees with the run.
5. **Where code lives:** [`pilot.md`](pilot.md) (every Pilot file and what it does) and [`rust.md`](rust.md) (every Rust file and why it still exists).
