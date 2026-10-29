//! Milestone 35.2: SPDX License Compatibility Verification.

pub const PERMISSIVE_SPDX_LICENSES: &[&str] = &[
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "CC0-1.0",
    "Unlicense",
    "0BSD",
];

pub struct LicenseVerifier;

impl LicenseVerifier {
    pub fn is_viable_license(spdx_id: &str) -> bool {
        let trimmed = spdx_id.trim();
        PERMISSIVE_SPDX_LICENSES.iter().any(|&l| l.eq_ignore_ascii_case(trimmed))
    }
}
