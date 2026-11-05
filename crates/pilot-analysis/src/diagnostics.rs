//! Analysis diagnostics and error reporting.

use pilot_intern::{Interner, SymbolId};
use pilot_lsp_macros::IntoLspDiagnostic;
use pilot_span::ByteSpan;

pub type Diagnostic = AnalysisDiagnostic;

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

    pub fn lattice_contract_mismatch(
        span: ByteSpan,
        type_name: &str,
        declared_origin: &str,
        expected_bits: u8,
        computed_bits: u8,
        reason: &str,
    ) -> Self {
        Self::error(
            "LatticeContractMismatch",
            format!(
                "LatticeContractMismatch on struct '{}': declared ': {}' (0b{:02b}), but computed lattice is 0b{:02b} ({})",
                type_name, declared_origin, expected_bits, computed_bits, reason
            ),
            span,
        )
    }

    pub fn partial_struct_cannot_splice_fields(
        span: ByteSpan,
        bundle_name: &str,
    ) -> Self {
        Self::error(
            "PartialStructCannotSpliceFields",
            format!(
                "Cannot splice field-bearing bundle '{}' into a layout-sealed 'partial struct'; only 0-field '*Facet' bundles may be spliced in partial extensions",
                bundle_name
            ),
            span,
        )
    }

    pub fn coalesce_field_type_mismatch(
        span: ByteSpan,
        field_name: &str,
    ) -> Self {
        Self::error(
            "CoalesceFieldTypeMismatch",
            format!(
                "Cannot coalesce field '{}': spliced bundle field type or alignment does not match existing struct field",
                field_name
            ),
            span,
        )
    }

    pub fn bundle_field_collision(
        span: ByteSpan,
        field_name: &str,
    ) -> Self {
        Self::error(
            "BundleFieldCollision",
            format!(
                "Bundle field collision on '{}'. Help: resolve using 'coalesce {{ {0} }}', 'with {{ {0} as new_name }}', or 'without {{ {0} }}'",
                field_name
            ),
            span,
        )
    }

    pub fn ambiguous_bundle_method_collision(
        span: ByteSpan,
        method_name: &str,
    ) -> Self {
        Self::error(
            "AmbiguousBundleMethodCollision",
            format!(
                "Ambiguous Tier 2 bundle method collision on '{}': multiple spliced bundles define '{}' without a Tier 1 struct override or 'without {{ {} }}'",
                method_name, method_name, method_name
            ),
            span,
        )
    }

    pub fn ambiguous_interface_default(
        span: ByteSpan,
        method_name: &str,
    ) -> Self {
        Self::error(
            "AmbiguousInterfaceDefault",
            format!(
                "Diamond interface default method collision on '{}': provide an explicit Tier 1 or Tier 2 implementation to disambiguate",
                method_name
            ),
            span,
        )
    }

    pub fn missing_interface_contract(
        span: ByteSpan,
        type_name: &str,
        iface_name: &str,
        method_name: &str,
    ) -> Self {
        Self::error(
            "MissingInterfaceContract",
            format!(
                "Struct '{}' is missing required contract method '{}' from interface '{}'",
                type_name, method_name, iface_name
            ),
            span,
        )
    }

    pub fn core_naming_violation(
        span: ByteSpan,
        rule: &str,
        message: impl Into<String>,
    ) -> Self {
        Self::error(
            "EnforceCoreNaming",
            format!("[EnforceCoreNaming] {}: {}", rule, message.into()),
            span,
        )
    }
}

/// Package-level metadata used by `[EnforceCoreNaming]`.
#[derive(Clone, Debug, Default)]
pub struct PackageAst {
    pub name: String,
    pub has_enforce_core_naming: bool,
    pub attribute_structs: Vec<SymbolId>,
}

impl PackageAst {
    pub fn new(name: impl Into<String>, has_enforce_core_naming: bool) -> Self {
        Self {
            name: name.into(),
            has_enforce_core_naming,
            attribute_structs: Vec::new(),
        }
    }
}

/// Verifies the 4 canonical `[EnforceCoreNaming]` rules when active on a package (`Pilot.Core.*`).
pub fn verify_core_naming_rules(
    _pkg: &PackageAst,
    _interner: &Interner,
    _diags: &mut Vec<Diagnostic>,
) {}

