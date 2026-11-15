# Fixture keyword matrix

Text before the generated part is kept as written.

<!-- keyword_matrix: generated part. Edit the hand columns (Bound by, S, I, C, Verdict, Complexity, Notes) in place, then run tools/keyword_matrix. -->

## Summary

| Verdict | Keywords |
|---|---|
| REAL | 2 |
| FALSE CLAIM | 1 |
| NOT AUDITED | 1 |
| PARTIAL | 1 |
| **Total** | **5** |

Rows with no evidence gap: 1.

| Complexity | Keywords |
|---|---|
| 1 | 1 |
| 2 | 1 |
| 3 | 1 |
| 4 | 1 |
| 5 | 0 |

Pilot share of the code that implements a keyword (4 keywords with code found): median 66%, lowest 0%, highest 87%.

## Keywords by registry category

### 0. Seed

| Keyword | Registry says | Bound by | Pair | S | I | C | Uses pass / fail | Named tests | Codes | Ledger | Known bugs | Verdict | Complexity | Pilot / Rust | Evidence gaps | Notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `tether` | Binds a satellite to its host. | ? | `~>` | ? | ? | ? | 0 / 1 | `errors/err_tether_loose` | TET001 | Core | — | NOT AUDITED | ? | 0% / 100% (0 / 84) | binding, named passing program, an engine | Not checked yet. |

### 1. Flow

| Keyword | Registry says | Bound by | Pair | S | I | C | Uses pass / fail | Named tests | Codes | Ledger | Known bugs | Verdict | Complexity | Pilot / Rust | Evidence gaps | Notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `orbit` | Repeats a block around a centre \| with a pipe. | receptor | — | ok | ok | ok | 2 / 1 | `control/orbit_basic`, `control/err_orbit_empty` | ORB001 | Draft, Wired | `seed.drift_overshoot` | REAL | 2 | 71% / 29% (149 / 62) | known bug open | Fixture row. |
| `drift` | Moves a value slowly. | receptor | — | ok | ok | – | 1 / 0 | `control/drift_steps` | — | Draft | `seed.drift_overshoot` | PARTIAL | 3 | 87% / 13% (69 / 10) | named failing program, per-engine run, known bug open | C emitter not run. |
| `lull` | Waits without doing anything for as long as the block around it says, then lets the next s… | none | — | err | err | err | 0 / 0 | — | — | — | — | FALSE CLAIM | 1 | — | binding, named passing program, named failing program, ledger row | Registered with no grammar. |

### 35. Domain

| Keyword | Registry says | Bound by | Pair | S | I | C | Uses pass / fail | Named tests | Codes | Ledger | Known bugs | Verdict | Complexity | Pilot / Rust | Evidence gaps | Notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `unit` | Attaches a dimension to a number literal. | syntax rule | — | ok | ok | ok | 0 / 0 | `misc/unit_scale`, `errors/err_unit_mismatch` | UNIT001 | Wired | — | REAL | 4 | 61% / 39% (47 / 30) | none | Fixture row. |

<!-- keyword_matrix: end of generated part -->

## Text after the generated part

Kept as written too.
