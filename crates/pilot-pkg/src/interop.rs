//! Milestone 8 & Milestone 43.3: Foreign Ecosystem Interop Package Adapters & License Auditing.

use pilot_transpile::LicenseVerifier;
use crate::manifest::PackageManifest;

/// Parsed package specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPackageSpec {
    pub ecosystem: String,
    pub name: String,
    pub version: String,
}

/// Parses a dependency string into ecosystem, package name, and version.
/// Examples:
///   - `math-lib@1.0.0` -> `(pilot, math-lib, 1.0.0)`
///   - `pip:requests@2.31.0` -> `(pip, requests, 2.31.0)`
///   - `npm:lodash` -> `(npm, lodash, latest)`
pub fn parse_package_spec(spec: &str, default_ver: Option<&str>) -> ParsedPackageSpec {
    let (ecosystem, rest) = if let Some(colon_idx) = spec.find(':') {
        (spec[..colon_idx].to_ascii_lowercase(), &spec[colon_idx + 1..])
    } else {
        ("pilot".to_string(), spec)
    };

    let (name, version) = if let Some(at_idx) = rest.find('@') {
        (&rest[..at_idx], &rest[at_idx + 1..])
    } else {
        (rest, default_ver.unwrap_or("latest"))
    };

    ParsedPackageSpec {
        ecosystem,
        name: name.to_string(),
        version: version.to_string(),
    }
}

/// Result of package license compliance auditing.
#[derive(Debug, Clone)]
pub struct AuditReport {
    pub is_compliant: bool,
    pub compliant_packages: Vec<(String, String)>, // (pkg_name, license)
    pub non_compliant_packages: Vec<(String, String)>,
}

/// Audits package licenses against permissive open-source criteria.
pub fn audit_manifest_licenses(manifest: &PackageManifest) -> AuditReport {
    let mut compliant = Vec::new();
    let mut non_compliant = Vec::new();

    // Check project itself
    if LicenseVerifier::is_viable_license(&manifest.license) {
        compliant.push((manifest.name.clone(), manifest.license.clone()));
    } else {
        non_compliant.push((manifest.name.clone(), manifest.license.clone()));
    }

    AuditReport {
        is_compliant: non_compliant.is_empty(),
        compliant_packages: compliant,
        non_compliant_packages: non_compliant,
    }
}
