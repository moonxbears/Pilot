//! The seed lexer (`pilot-lexer`): runs the table Pilot builds from its
//! token declarations (`stdlib/core/lex/toolchain.lex`, PROP-021 §3). The
//! caller reads the table's text and loads it once with `load_table`.

pub mod lexer;
pub mod token;

pub use lexer::SeedTable;
pub use token::{TokenFlags, TokenKind, TokenTape};

/// The table in `text` (a file written by `pilot lex --write-table`), or
/// why it cannot be read.
pub fn load_table(text: &str) -> Result<SeedTable, String> {
    SeedTable::parse(text)
}

/// Tokenizes `source` with `table` into a dense Struct-of-Arrays `TokenTape`.
pub fn tokenize(table: &SeedTable, source: &str) -> TokenTape {
    lexer::run(table, source)
}
