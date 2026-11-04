//! Analysis diagnostics and error reporting.

use pilot_intern::{Interner, SymbolId};
use pilot_lsp_macros::IntoLspDiagnostic;
use pilot_span::ByteSpan;
use crate::types::TypeArena;

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
    pkg: &PackageAst,
    type_arena: &mut TypeArena,
    interner: &Interner,
    diags: &mut Vec<Diagnostic>,
) {
    if !pkg.has_enforce_core_naming {
        return;
    }

    // Rule 1 & Rule 4: Structs (`[Lang("...")]` primitives vs non-`[Lang]` composite structs & `IAttribute`)
    let mut newly_registered_attrs: Vec<String> = Vec::new();
    let iattr_iface_id = type_arena.interfaces_by_name.get("IAttribute").copied();

    for type_def in type_arena.type_defs.values() {
        // Skip internal synthetic builtins that were not user/stdlib-declared
        if type_def.name.0 == 0 {
            continue;
        }
        let name_str = interner.resolve(type_def.name);
        if name_str.is_empty() {
            continue;
        }

        if type_def.is_lang_primitive {
            // Rule 1: `[Lang("...")]` Built-In Primitives MUST Be Single-Word `lowercase`
            let valid_lower = name_str
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                && !name_str.contains('_');
            if !valid_lower {
                diags.push(AnalysisDiagnostic::core_naming_violation(
                    ByteSpan::default(),
                    "Rule1_LangPrimitiveLowercase",
                    format!(
                        "[Lang] primitive struct '{}' must be single-word lowercase without underscores",
                        name_str
                    ),
                ));
            }
        } else {
            // Rule 4: Non-`[Lang]` Composite Structs & Attributes MUST Be `PascalCase`
            let starts_upper = name_str
                .chars()
                .next()
                .map_or(false, |c| c.is_ascii_uppercase());
            if !starts_upper || name_str.contains('_') {
                diags.push(AnalysisDiagnostic::core_naming_violation(
                    ByteSpan::default(),
                    "Rule4_CompositeStructPascalCase",
                    format!(
                        "Non-[Lang] composite struct '{}' must use PascalCase",
                        name_str
                    ),
                ));
            }

            let implements_iattr = pkg.attribute_structs.contains(&type_def.name)
                || iattr_iface_id.map_or(false, |id| {
                    type_def.implemented_interfaces.contains_interface(id)
                        || type_def.declared_interfaces.contains(&id)
                });
            if implements_iattr {
                if !name_str.ends_with("Attr") || name_str.len() <= 4 {
                    diags.push(AnalysisDiagnostic::core_naming_violation(
                        ByteSpan::default(),
                        "Rule4_AttributeSuffix",
                        format!(
                            "Attribute struct '{}' implementing 'IAttribute' must end with 'Attr'",
                            name_str
                        ),
                    ));
                } else {
                    let short_name = &name_str[..name_str.len() - 4];
                    newly_registered_attrs.push(name_str.to_string());
                    newly_registered_attrs.push(short_name.to_string());
                }
            }
        }
    }

    for attr_name in newly_registered_attrs {
        type_arena.registered_attributes.insert(attr_name);
    }

    // Rule 2: `interface` Declarations MUST Use `I` + `PascalCase`
    for iface_def in type_arena.interface_defs.values() {
        if iface_def.name.0 == 0 {
            continue;
        }
        let name_str = interner.resolve(iface_def.name);
        if name_str.is_empty() {
            continue;
        }
        let mut chars = name_str.chars();
        let first_is_i = chars.next() == Some('I');
        let second_is_upper = chars.next().map_or(false, |c| c.is_ascii_uppercase());
        if !first_is_i || !second_is_upper || name_str.contains('_') {
            diags.push(AnalysisDiagnostic::core_naming_violation(
                iface_def.span,
                "Rule2_InterfaceIPascalCase",
                format!(
                    "Interface '{}' must start with 'I' followed by PascalCase (e.g. 'I{}')",
                    name_str, name_str
                ),
            ));
        }
    }

    // Rule 3: Code Bundles MUST Use `...Bundle` or `...Facet` Suffixes
    for bundle_def in type_arena.bundle_defs.values() {
        if bundle_def.name.0 == 0 {
            continue;
        }
        let name_str = interner.resolve(bundle_def.name);
        if name_str.is_empty() {
            continue;
        }
        let is_pascal = name_str
            .chars()
            .next()
            .map_or(false, |c| c.is_ascii_uppercase())
            && !name_str.contains('_');
        if bundle_def.is_method_only_facet() {
            if !is_pascal || !name_str.ends_with("Facet") {
                diags.push(AnalysisDiagnostic::core_naming_violation(
                    bundle_def.span,
                    "Rule3_MethodOnlyFacetSuffix",
                    format!(
                        "Zero-field code bundle '{}' must use PascalCase and end with 'Facet'",
                        name_str
                    ),
                ));
            }
        } else {
            if !is_pascal || !name_str.ends_with("Bundle") {
                diags.push(AnalysisDiagnostic::core_naming_violation(
                    bundle_def.span,
                    "Rule3_FieldBearingBundleSuffix",
                    format!(
                        "Field-bearing code bundle '{}' ({} field(s)) must use PascalCase and end with 'Bundle' (not 'Facet')",
                        name_str,
                        bundle_def.fields.len()
                    ),
                ));
            }
        }
    }
}

