//! Name-blind diagnostic container and LSP severity codes.

use pilot_span::ByteSpan;

pub type Diagnostic = AnalysisDiagnostic;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity { Error, Warning, Info, Hint }

impl DiagnosticSeverity {
    #[inline]
    pub const fn as_lsp_code(self) -> u8 {
        match self { Self::Error => 1, Self::Warning => 2, Self::Info => 3, Self::Hint => 4 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisDiagnostic {
    pub code: String, pub message: String, pub span: ByteSpan, pub severity: DiagnosticSeverity,
}

impl AnalysisDiagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>, span: ByteSpan) -> Self {
        Self { code: code.into(), message: message.into(), span, severity: DiagnosticSeverity::Error }
    }
    pub fn warning(code: impl Into<String>, message: impl Into<String>, span: ByteSpan) -> Self {
        Self { code: code.into(), message: message.into(), span, severity: DiagnosticSeverity::Warning }
    }
    pub fn is_error(&self) -> bool { self.severity == DiagnosticSeverity::Error }
}
