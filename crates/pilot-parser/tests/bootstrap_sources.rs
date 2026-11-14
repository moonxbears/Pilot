//! The stdlib files the parser builds its tables from must parse without a
//! diagnostic. Those diagnostics are not shown when the tables are built, so
//! a declaration that fails to parse there (a rule template holding a node
//! templates cannot carry, a keyword used before the file that binds it)
//! would otherwise be lost without a word.

#[test]
fn the_bootstrap_sources_parse_without_diagnostics() {
    let found = pilot_parser::bootstrap_diagnostics();
    assert!(found.is_empty(), "bootstrap sources reported:\n{}", found.join("\n"));
}
