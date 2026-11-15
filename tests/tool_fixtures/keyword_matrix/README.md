# keyword_matrix fixture

A small repository for the acceptance test of `tools/keyword_matrix`
(`crates/pilot-cli/tests/cli_keyword_matrix.rs`). The test copies this
folder, runs the tool on the copy with `--root`, and compares the result with
`expected_keyword_matrix.md`.

The files are not Pilot or Rust programs that build; they only have the
shapes the tool reads. `expected_keyword_matrix.md` is worked by hand from
them; the working is in the comments of the test.
