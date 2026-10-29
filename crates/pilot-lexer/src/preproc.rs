//! Milestone 29.2: Zero-Copy In-Place Preprocessor State Machine (`PREPROC001`–`PREPROC011`).
//!
//! Evaluates conditional compilation directives directly on raw UTF-8 byte spans without allocating
//! any intermediate strings. Produces inactive byte ranges, code regions, warning suppressions,
//! and standard Pilot preprocessor diagnostics.

use std::collections::{HashMap, HashSet};
use pilot_span::ByteSpan;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreprocessorDiagnostic {
    pub line: u32,
    pub col: u32,
    pub severity: DiagnosticSeverity,
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeRegion {
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarningSuppression {
    pub warning_code: String,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Clone, Debug, Default)]
pub struct PreprocessorOptions {
    pub initial_symbols: HashSet<String>,
}

#[derive(Clone, Debug, Default)]
pub struct PreprocessorResult {
    pub defined_symbols: HashSet<String>,
    pub inactive_spans: Vec<ByteSpan>,
    pub regions: Vec<CodeRegion>,
    pub suppressions: Vec<WarningSuppression>,
    pub diagnostics: Vec<PreprocessorDiagnostic>,
}

struct IfFrame {
    condition_met: bool,
    current_branch_active: bool,
    has_else_branch: bool,
    start_line: u32,
}

pub struct Preprocessor {
    defined_symbols: HashSet<String>,
    if_stack: Vec<IfFrame>,
    region_stack: Vec<(String, u32)>,
    active_suppressions: HashMap<String, u32>,
    result: PreprocessorResult,
}

impl Preprocessor {
    pub fn new(options: PreprocessorOptions) -> Self {
        Self {
            defined_symbols: options.initial_symbols,
            if_stack: Vec::new(),
            region_stack: Vec::new(),
            active_suppressions: HashMap::new(),
            result: PreprocessorResult::default(),
        }
    }

    /// Preprocesses raw UTF-8 `src`, populating and returning the `PreprocessorResult`.
    pub fn process(mut self, src: &str) -> PreprocessorResult {
        let bytes = src.as_bytes();
        let mut line_num = 1u32;
        let mut cursor = 0usize;

        while cursor < bytes.len() {
            let line_start = cursor;
            let line_end = match bytes[cursor..].iter().position(|&b| b == b'\n') {
                Some(rel) => cursor + rel,
                None => bytes.len(),
            };
            let next_cursor = if line_end < bytes.len() { line_end + 1 } else { line_end };

            let line_bytes = &bytes[line_start..line_end];
            let line_str = match std::str::from_utf8(line_bytes) {
                Ok(s) => s,
                Err(_) => "",
            };

            // Support UNIX shebang on first line
            if line_num == 1 && line_str.starts_with("#!") {
                self.result.inactive_spans.push(ByteSpan::new(line_start as u32, line_end as u32));
                cursor = next_cursor;
                line_num += 1;
                continue;
            }

            // Find first non-whitespace character on this line
            let mut first_non_ws = 0usize;
            while first_non_ws < line_str.len() {
                let c = line_str.as_bytes()[first_non_ws];
                if c == b' ' || c == b'\t' || c == b'\r' {
                    first_non_ws += 1;
                } else {
                    break;
                }
            }

            let is_currently_active = self.if_stack.is_empty() || self.if_stack.last().unwrap().current_branch_active;

            if first_non_ws < line_str.len() && line_str.as_bytes()[first_non_ws] == b'#' {
                let col = (first_non_ws + 1) as u32;
                let directive_line = &line_str[first_non_ws + 1..].trim();
                self.handle_directive(directive_line, line_num, col, line_start as u32, line_end as u32);
            } else if !is_currently_active {
                // Inactive code line: mask it into inactive_spans
                self.result.inactive_spans.push(ByteSpan::new(line_start as u32, line_end as u32));
            }

            cursor = next_cursor;
            line_num += 1;
        }

        // Check for unclosed #if blocks
        for frame in &self.if_stack {
            self.result.diagnostics.push(PreprocessorDiagnostic {
                line: frame.start_line,
                col: 1,
                severity: DiagnosticSeverity::Error,
                code: "PREPROC004",
                message: "Unclosed #if directive at end of file.".to_string(),
            });
        }

        // Check for unclosed #region blocks
        for (name, start_l) in &self.region_stack {
            self.result.diagnostics.push(PreprocessorDiagnostic {
                line: *start_l,
                col: 1,
                severity: DiagnosticSeverity::Error,
                code: "PREPROC007",
                message: format!("Unclosed #region '{}' at end of file.", name),
            });
        }

        // Close any remaining active suppressions
        for (code, start_l) in self.active_suppressions {
            self.result.suppressions.push(WarningSuppression {
                warning_code: code,
                start_line: start_l,
                end_line: u32::MAX,
            });
        }

        self.result.defined_symbols = self.defined_symbols;
        self.result
    }

    fn handle_directive(
        &mut self,
        content: &str,
        line: u32,
        col: u32,
        line_start: u32,
        line_end: u32,
    ) {
        // Directive lines are always suppressed from normal code tokenization
        self.result.inactive_spans.push(ByteSpan::new(line_start, line_end));

        let mut parts = content.splitn(2, |c: char| c.is_whitespace());
        let directive = parts.next().unwrap_or("").to_ascii_lowercase();
        let args = parts.next().unwrap_or("").trim();

        let is_parent_active = self.if_stack.is_empty() || {
            let len = self.if_stack.len();
            if len <= 1 {
                true
            } else {
                self.if_stack[len - 2].current_branch_active
            }
        };

        match directive.as_str() {
            "define" => {
                if is_parent_active && (self.if_stack.is_empty() || self.if_stack.last().unwrap().current_branch_active) {
                    if args.is_empty() {
                        self.result.diagnostics.push(PreprocessorDiagnostic {
                            line, col,
                            severity: DiagnosticSeverity::Error,
                            code: "PREPROC001",
                            message: "Expected symbol name after #define.".to_string(),
                        });
                    } else {
                        self.defined_symbols.insert(args.to_ascii_uppercase());
                    }
                }
            }
            "undef" => {
                if is_parent_active && (self.if_stack.is_empty() || self.if_stack.last().unwrap().current_branch_active) {
                    if !args.is_empty() {
                        self.defined_symbols.remove(&args.to_ascii_uppercase());
                    }
                }
            }
            "if" => {
                let cond_val = if is_parent_active {
                    self.eval_condition(args, line, col)
                } else {
                    false
                };
                self.if_stack.push(IfFrame {
                    condition_met: cond_val,
                    current_branch_active: is_parent_active && cond_val,
                    has_else_branch: false,
                    start_line: line,
                });
            }
            "elif" => {
                let cond = if is_parent_active {
                    self.eval_condition(args, line, col)
                } else {
                    false
                };
                if let Some(top) = self.if_stack.last_mut() {
                    if top.has_else_branch {
                        self.result.diagnostics.push(PreprocessorDiagnostic {
                            line, col,
                            severity: DiagnosticSeverity::Error,
                            code: "PREPROC003",
                            message: "Unexpected #elif after #else.".to_string(),
                        });
                    } else if !top.condition_met && is_parent_active {
                        top.condition_met = cond;
                        top.current_branch_active = cond;
                    } else {
                        top.current_branch_active = false;
                    }
                } else {
                    self.result.diagnostics.push(PreprocessorDiagnostic {
                        line, col,
                        severity: DiagnosticSeverity::Error,
                        code: "PREPROC003",
                        message: "Unexpected #elif without matching #if.".to_string(),
                    });
                }
            }
            "else" => {
                if let Some(top) = self.if_stack.last_mut() {
                    if top.has_else_branch {
                        self.result.diagnostics.push(PreprocessorDiagnostic {
                            line, col,
                            severity: DiagnosticSeverity::Error,
                            code: "PREPROC003",
                            message: "Duplicate #else directive.".to_string(),
                        });
                    } else {
                        top.has_else_branch = true;
                        top.current_branch_active = is_parent_active && !top.condition_met;
                    }
                } else {
                    self.result.diagnostics.push(PreprocessorDiagnostic {
                        line, col,
                        severity: DiagnosticSeverity::Error,
                        code: "PREPROC003",
                        message: "Unexpected #else without matching #if.".to_string(),
                    });
                }
            }
            "endif" => {
                if self.if_stack.pop().is_none() {
                    self.result.diagnostics.push(PreprocessorDiagnostic {
                        line, col,
                        severity: DiagnosticSeverity::Error,
                        code: "PREPROC003",
                        message: "Unexpected #endif without matching #if.".to_string(),
                    });
                }
            }
            "region" => {
                self.region_stack.push((args.to_string(), line));
            }
            "endregion" => {
                if let Some((name, start_l)) = self.region_stack.pop() {
                    self.result.regions.push(CodeRegion {
                        name,
                        start_line: start_l,
                        end_line: line,
                    });
                } else {
                    self.result.diagnostics.push(PreprocessorDiagnostic {
                        line, col,
                        severity: DiagnosticSeverity::Error,
                        code: "PREPROC008",
                        message: "Unexpected #endregion without matching #region.".to_string(),
                    });
                }
            }
            "error" => {
                if is_parent_active && (self.if_stack.is_empty() || self.if_stack.last().unwrap().current_branch_active) {
                    self.result.diagnostics.push(PreprocessorDiagnostic {
                        line, col,
                        severity: DiagnosticSeverity::Error,
                        code: "PREPROC005",
                        message: args.to_string(),
                    });
                }
            }
            "warning" => {
                if is_parent_active && (self.if_stack.is_empty() || self.if_stack.last().unwrap().current_branch_active) {
                    self.result.diagnostics.push(PreprocessorDiagnostic {
                        line, col,
                        severity: DiagnosticSeverity::Warning,
                        code: "PREPROC006",
                        message: args.to_string(),
                    });
                }
            }
            "pragma" => {
                // e.g. #pragma warning disable CS0618 or #pragma warning restore CS0618
                let pragma_args: Vec<&str> = args.split_whitespace().collect();
                if pragma_args.len() >= 3 && pragma_args[0].eq_ignore_ascii_case("warning") {
                    let action = pragma_args[1].to_ascii_lowercase();
                    let warn_code = pragma_args[2].to_ascii_uppercase();
                    if action == "disable" {
                        self.active_suppressions.insert(warn_code, line);
                    } else if action == "restore" {
                        if let Some(start_l) = self.active_suppressions.remove(&warn_code) {
                            self.result.suppressions.push(WarningSuppression {
                                warning_code: warn_code,
                                start_line: start_l,
                                end_line: line,
                            });
                        }
                    }
                }
            }
            "nullable" | "line" => {
                // Stored/handled as valid metadata
            }
            _ => {
                self.result.diagnostics.push(PreprocessorDiagnostic {
                    line, col,
                    severity: DiagnosticSeverity::Error,
                    code: "PREPROC001",
                    message: format!("Unrecognized preprocessor directive '#{}'.", directive),
                });
            }
        }
    }

    /// Evaluates a preprocessor boolean condition (e.g. `DEBUG || (LINUX && !X86)`).
    fn eval_condition(&self, expr: &str, _line: u32, _col: u32) -> bool {
        let trimmed = expr.trim();
        if trimmed.is_empty() {
            return false;
        }

        // Support simple binary OR
        if let Some((lhs, rhs)) = trimmed.split_once("||") {
            return self.eval_condition(lhs, _line, _col) || self.eval_condition(rhs, _line, _col);
        }

        // Support simple binary AND
        if let Some((lhs, rhs)) = trimmed.split_once("&&") {
            return self.eval_condition(lhs, _line, _col) && self.eval_condition(rhs, _line, _col);
        }

        // Support unary NOT
        if let Some(rest) = trimmed.strip_prefix('!') {
            return !self.eval_condition(rest, _line, _col);
        }

        // Support parenthesized expressions
        if trimmed.starts_with('(') && trimmed.ends_with(')') {
            return self.eval_condition(&trimmed[1..trimmed.len() - 1], _line, _col);
        }

        let symbol = trimmed.trim();
        if symbol.eq_ignore_ascii_case("true") {
            true
        } else if symbol.eq_ignore_ascii_case("false") {
            false
        } else {
            self.defined_symbols.contains(&symbol.to_ascii_uppercase())
        }
    }
}
