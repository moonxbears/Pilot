//! Analysis diagnostics and error reporting.

use pilot_span::ByteSpan;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisDiagnostic {
    pub code: String,
    pub message: String,
    pub span: ByteSpan,
    pub severity: DiagnosticSeverity,
}

impl AnalysisDiagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>, span: ByteSpan) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            span,
            severity: DiagnosticSeverity::Error,
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>, span: ByteSpan) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            span,
            severity: DiagnosticSeverity::Warning,
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == DiagnosticSeverity::Error
    }
}
