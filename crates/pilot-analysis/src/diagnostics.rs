//! Analysis diagnostics and error reporting.

use pilot_lsp_macros::IntoLspDiagnostic;
use pilot_span::ByteSpan;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl DiagnosticSeverity {
    /// Converts compiler diagnostic severity directly into the LSP `DiagnosticSeverity` integer code.
    #[inline]
    pub const fn as_lsp_code(self) -> u8 {
        match self {
            DiagnosticSeverity::Error => 1,
            DiagnosticSeverity::Warning => 2,
            DiagnosticSeverity::Info => 3,
            DiagnosticSeverity::Hint => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, IntoLspDiagnostic)]
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
