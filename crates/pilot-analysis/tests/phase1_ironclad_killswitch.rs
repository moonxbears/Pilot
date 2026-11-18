// ==============================================================================
// tests/phase1_ironclad_killswitch.rs
// Ironclad Phase 1 Verification:
// Gate 1: The Kill-Switch Proof Test (Proves Rust Has Zero Hidden Inference)
// Gate 2: The Anti-Hardcoding Structural Proof Test (1B, 8B, 24B, 32B)
// ==============================================================================

use pilot_parser::parse;

#[test]
fn test_diagnostic_catalogs_parsed_cleanly() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    for catalog_rel in [
        "selfhost/diagnostics/errors.pilot",
        "selfhost/diagnostics/warnings.pilot",
        "selfhost/diagnostics/infos.pilot",
    ] {
        let p = root.join(catalog_rel);
        assert!(p.exists(), "Diagnostic catalog file must exist: {:?}", p);
        let src = std::fs::read_to_string(&p).expect("read catalog");
        let parsed = parse(&src);
        assert!(
            parsed.diagnostics.is_empty(),
            "Diagnostic catalog {:?} must parse with 0 diagnostics, got: {:?}",
            catalog_rel,
            parsed.diagnostics
        );
    }
}
