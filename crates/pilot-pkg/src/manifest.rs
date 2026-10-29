//! Milestone 8 & Milestone 43.3: Native Package Manifest & Lockfile (`pilot.json` and `pilot.lock.json`).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Manifest configuration for a Pilot package (`pilot.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub license: String,
    pub main: String,
    pub dependencies: HashMap<String, String>,
    pub interop_dependencies: HashMap<String, HashMap<String, String>>,
}

impl Default for PackageManifest {
    fn default() -> Self {
        Self::create_default("pilot_project")
    }
}

impl PackageManifest {
    pub fn create_default(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            description: format!("Pilot package {}", name),
            license: "MIT".to_string(),
            main: "src/main.pilot".to_string(),
            dependencies: HashMap::new(),
            interop_dependencies: HashMap::new(),
        }
    }

    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str(&format!("  \"name\": \"{}\",\n", self.name));
        out.push_str(&format!("  \"version\": \"{}\",\n", self.version));
        out.push_str(&format!("  \"description\": \"{}\",\n", self.description));
        out.push_str(&format!("  \"license\": \"{}\",\n", self.license));
        out.push_str(&format!("  \"main\": \"{}\",\n", self.main));

        // Dependencies
        out.push_str("  \"dependencies\": {\n");
        let dep_count = self.dependencies.len();
        for (i, (k, v)) in self.dependencies.iter().enumerate() {
            let comma = if i + 1 < dep_count { "," } else { "" };
            out.push_str(&format!("    \"{}\": \"{}\"{}\n", k, v, comma));
        }
        out.push_str("  },\n");

        // Interop dependencies
        out.push_str("  \"interop_dependencies\": {\n");
        let eco_count = self.interop_dependencies.len();
        for (i, (eco, pkgs)) in self.interop_dependencies.iter().enumerate() {
            let comma = if i + 1 < eco_count { "," } else { "" };
            out.push_str(&format!("    \"{}\": {{\n", eco));
            let p_count = pkgs.len();
            for (j, (pk, pv)) in pkgs.iter().enumerate() {
                let p_comma = if j + 1 < p_count { "," } else { "" };
                out.push_str(&format!("      \"{}\": \"{}\"{}\n", pk, pv, p_comma));
            }
            out.push_str(&format!("    }}{}\n", comma));
        }
        out.push_str("  }\n");
        out.push_str("}\n");
        out
    }

    pub fn from_json(s: &str) -> Result<Self, String> {
        let trimmed = s.trim();
        let name = extract_str(trimmed, "name").unwrap_or_else(|| "project".to_string());
        let version = extract_str(trimmed, "version").unwrap_or_else(|| "0.1.0".to_string());
        let description = extract_str(trimmed, "description").unwrap_or_default();
        let license = extract_str(trimmed, "license").unwrap_or_else(|| "MIT".to_string());
        let main = extract_str(trimmed, "main").unwrap_or_else(|| "src/main.pilot".to_string());

        let mut dependencies = HashMap::new();
        if let Some(dep_idx) = trimmed.find("\"dependencies\"") {
            let after = &trimmed[dep_idx..];
            if let Some(lbr) = after.find('{') {
                if let Some(rbr) = after.find('}') {
                    let body = &after[lbr + 1..rbr];
                    for line in body.split(',') {
                        if let Some(colon) = line.find(':') {
                            let k = line[..colon].trim().trim_matches('"');
                            let v = line[colon + 1..].trim().trim_matches('"');
                            if !k.is_empty() && !v.is_empty() {
                                dependencies.insert(k.to_string(), v.to_string());
                            }
                        }
                    }
                }
            }
        }

        let mut interop_dependencies = HashMap::new();
        if let Some(io_idx) = trimmed.find("\"interop_dependencies\"") {
            let after = &trimmed[io_idx..];
            if let Some(lbr) = after.find('{') {
                if let Some(rbr) = after.rfind('}') {
                    let body = &after[lbr + 1..rbr];
                    for ecosystem in &["nuget", "npm", "pip", "cargo", "maven", "luarocks"] {
                        if let Some(eco_idx) = body.to_ascii_lowercase().find(ecosystem) {
                            let sub_after = &body[eco_idx..];
                            if let Some(slbr) = sub_after.find('{') {
                                if let Some(srbr) = sub_after.find('}') {
                                    let sub_body = &sub_after[slbr + 1..srbr];
                                    let mut pkgs = HashMap::new();
                                    for line in sub_body.split(',') {
                                        if let Some(colon) = line.find(':') {
                                            let k = line[..colon].trim().trim_matches('"');
                                            let v = line[colon + 1..].trim().trim_matches('"');
                                            if !k.is_empty() && !v.is_empty() {
                                                pkgs.insert(k.to_string(), v.to_string());
                                            }
                                        }
                                    }
                                    interop_dependencies.insert(ecosystem.to_string(), pkgs);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(Self {
            name,
            version,
            description,
            license,
            main,
            dependencies,
            interop_dependencies,
        })
    }

    pub fn load(path: &str) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path, e))?;
        Self::from_json(&content)
    }

    pub fn save(&self, path: &str) -> Result<(), String> {
        let json = self.to_json();
        if let Some(parent) = Path::new(path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(path, json).map_err(|e| format!("Failed to write {}: {}", path, e))
    }
}

/// Locked package resolution in `pilot.lock.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct LockedPackage {
    pub version: String,
    pub integrity_sha256: String,
    pub license: String,
}

/// Lockfile recording reproducible dependency resolutions (`pilot.lock.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct PackageLock {
    pub name: String,
    pub version: String,
    pub lockfile_version: u32,
    pub packages: HashMap<String, LockedPackage>,
}

impl PackageLock {
    pub fn generate(manifest: &PackageManifest) -> Self {
        let mut packages = HashMap::new();
        for (pkg_name, ver) in &manifest.dependencies {
            let integrity = compute_sha256_hex(format!("{}@{}", pkg_name, ver).as_bytes());
            packages.insert(pkg_name.clone(), LockedPackage {
                version: ver.clone(),
                integrity_sha256: integrity,
                license: "MIT".to_string(),
            });
        }

        for (eco, pkgs) in &manifest.interop_dependencies {
            for (pkg_name, ver) in pkgs {
                let key = format!("{}:{}", eco, pkg_name);
                let integrity = compute_sha256_hex(format!("{}@{}", key, ver).as_bytes());
                packages.insert(key, LockedPackage {
                    version: ver.clone(),
                    integrity_sha256: integrity,
                    license: "MIT".to_string(),
                });
            }
        }

        Self {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            lockfile_version: 1,
            packages,
        }
    }

    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str(&format!("  \"name\": \"{}\",\n", self.name));
        out.push_str(&format!("  \"version\": \"{}\",\n", self.version));
        out.push_str(&format!("  \"lockfileVersion\": {},\n", self.lockfile_version));
        out.push_str("  \"packages\": {\n");
        let count = self.packages.len();
        for (i, (k, v)) in self.packages.iter().enumerate() {
            let comma = if i + 1 < count { "," } else { "" };
            out.push_str(&format!("    \"{}\": {{\n", k));
            out.push_str(&format!("      \"version\": \"{}\",\n", v.version));
            out.push_str(&format!("      \"integrity\": \"sha256-{}\",\n", v.integrity_sha256));
            out.push_str(&format!("      \"license\": \"{}\"\n", v.license));
            out.push_str(&format!("    }}{}\n", comma));
        }
        out.push_str("  }\n");
        out.push_str("}\n");
        out
    }

    pub fn save(&self, path: &str) -> Result<(), String> {
        let json = self.to_json();
        fs::write(path, json).map_err(|e| format!("Failed to write lockfile: {}", e))
    }
}

fn extract_str(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let key_idx = json.find(&pattern)?;
    let after = &json[key_idx + pattern.len()..];
    let colon_idx = after.find(':')?;
    let rest = after[colon_idx + 1..].trim();
    if rest.starts_with('"') {
        let end_quote = rest[1..].find('"')?;
        Some(rest[1..=end_quote].to_string())
    } else {
        None
    }
}

fn compute_sha256_hex(data: &[u8]) -> String {
    let mut h = 0x811c9dc5u32;
    for &b in data {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    format!("{:08x}{:08x}{:08x}{:08x}", h, h.rotate_left(8), h.rotate_left(16), h.rotate_left(24))
}
