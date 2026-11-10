Fixtures for the `pilot test` runner itself (`crates/pilot-cli/tests/cli_f3.rs`).
Programs under `failing/` and `units.pilot`'s `breaks` are meant to fail, and
`fixed_bug/known_bugs/now_works.pilot` is meant to pass so the runner reports
it as fixed. `pilot test` with no path runs `tests/conformance`, never these.
