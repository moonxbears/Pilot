//! Milestone 8, 27 & 43.3: Native Pilot Package & Virtual Environment Manager (`pilot-pkg`).
//!
//! Enforces:
//! - Manifest (`pilot.json`) & deterministic lockfile (`pilot.lock.json`) management.
//! - Self-contained `.pilpkg` archive packing and extraction.
//! - Virtual environment sandbox generation with cross-platform activation scripts.
//! - Foreign package adoption (`pip`, `npm`, `cargo`, `nuget`, `maven`, `luarocks`) and license compliance.

pub mod manifest;
pub mod archive;
pub mod venv;
pub mod interop;

pub use manifest::{LockedPackage, PackageLock, PackageManifest};
pub use archive::{pack_directory, unpack_archive};
pub use venv::{get_active_venv, init_venv, PilotVenvConfig, PILOT_VENV_CFG, PILOT_VENV_ENV_VAR};
pub use interop::{audit_manifest_licenses, parse_package_spec, AuditReport, ParsedPackageSpec};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_and_lockfile_generation() {
        let mut manifest = PackageManifest::create_default("math_suite");
        manifest.dependencies.insert("vector".to_string(), "1.2.0".to_string());

        let mut pip_pkgs = std::collections::HashMap::new();
        pip_pkgs.insert("numpy".to_string(), "1.24.0".to_string());
        manifest.interop_dependencies.insert("pip".to_string(), pip_pkgs);

        let json = manifest.to_json();
        assert!(json.contains("math_suite"));
        assert!(json.contains("vector"));
        assert!(json.contains("numpy"));

        let loaded = PackageManifest::from_json(&json).expect("Parse failed");
        assert_eq!(loaded.name, "math_suite");
        assert_eq!(loaded.dependencies.get("vector"), Some(&"1.2.0".to_string()));

        let lock = PackageLock::generate(&manifest);
        assert_eq!(lock.name, "math_suite");
        assert!(lock.packages.contains_key("vector"));
        assert!(lock.packages.contains_key("pip:numpy"));
    }

    #[test]
    fn test_package_spec_parsing() {
        let p1 = parse_package_spec("math-lib@1.0.0", None);
        assert_eq!(p1.ecosystem, "pilot");
        assert_eq!(p1.name, "math-lib");
        assert_eq!(p1.version, "1.0.0");

        let p2 = parse_package_spec("pip:requests@2.31.0", None);
        assert_eq!(p2.ecosystem, "pip");
        assert_eq!(p2.name, "requests");
        assert_eq!(p2.version, "2.31.0");

        let p3 = parse_package_spec("cargo:serde", Some("1.0"));
        assert_eq!(p3.ecosystem, "cargo");
        assert_eq!(p3.name, "serde");
        assert_eq!(p3.version, "1.0");
    }

    #[test]
    fn test_venv_initialization_and_scripts() {
        let tmp_dir = format!("/tmp/test_pilot_venv_{}", std::process::id());
        let _ = std::fs::remove_dir_all(&tmp_dir);

        let venv_path = init_venv(&tmp_dir, Some("myenv"), false).expect("Init venv failed");
        assert!(venv_path.join("pilotvenv.cfg").exists());
        assert!(venv_path.join("bin").join("activate").exists());
        assert!(venv_path.join("bin").join("activate.ps1").exists());
        assert!(venv_path.join("bin").join("activate.bat").exists());

        let cfg_content = std::fs::read_to_string(venv_path.join("pilotvenv.cfg")).unwrap();
        let parsed = PilotVenvConfig::parse(&cfg_content);
        assert_eq!(parsed.prompt, "myenv");

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_pilpkg_archive_pack_and_unpack() {
        let src_dir = format!("/tmp/test_pilpkg_src_{}", std::process::id());
        let dest_dir = format!("/tmp/test_pilpkg_dest_{}", std::process::id());
        let pkg_file = format!("/tmp/test_{}.pilpkg", std::process::id());

        let _ = std::fs::create_dir_all(&src_dir);
        std::fs::write(format!("{}/main.pilot", src_dir), "println(\"hello\");").unwrap();
        std::fs::write(format!("{}/pilot.json", src_dir), "{\"name\":\"test\"}").unwrap();

        let count = pack_directory(&src_dir, &pkg_file).expect("Pack failed");
        assert_eq!(count, 2);

        let unpack_count = unpack_archive(&pkg_file, &dest_dir).expect("Unpack failed");
        assert_eq!(unpack_count, 2);

        let read_main = std::fs::read_to_string(format!("{}/main.pilot", dest_dir)).unwrap();
        assert_eq!(read_main, "println(\"hello\");");

        let _ = std::fs::remove_dir_all(&src_dir);
        let _ = std::fs::remove_dir_all(&dest_dir);
        let _ = std::fs::remove_file(&pkg_file);
    }
}
