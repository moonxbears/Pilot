//! Formatter options and configuration for `pilot fmt`.

/// Configuration options for Pilot formatting.
#[derive(Clone, Debug)]
pub struct FormatOptions {
    /// String to use for one level of indentation (default: 4 spaces).
    pub indent_string: String,
    /// Suite style:
    /// - `Some(true)`: force colon suites (`def foo():`)
    /// - `Some(false)`: force brace suites (`def foo() { ... }`)
    /// - `None`: preserve whatever style was present in the source.
    pub use_colon_suites: Option<bool>,
    /// Whether to emit trailing semicolons on statements (default: true).
    pub emit_semicolons: bool,
    /// Whether to preserve developer comments (default: true).
    pub preserve_comments: bool,
    /// Whether to preserve blank lines between declarations and statements (default: true).
    pub preserve_blank_lines: bool,
    /// Maximum consecutive blank lines allowed (default: 1).
    pub max_consecutive_blank_lines: usize,
    /// Maximum target line width before line-breaking groups (default: 100).
    pub max_line_width: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent_string: "    ".to_string(),
            use_colon_suites: None,
            emit_semicolons: true,
            preserve_comments: true,
            preserve_blank_lines: true,
            max_consecutive_blank_lines: 1,
            max_line_width: 100,
        }
    }
}

/// Result of formatting a Pilot source string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatResult {
    pub success: bool,
    pub formatted_code: String,
    pub was_modified: bool,
    pub diagnostics: Vec<String>,
}
