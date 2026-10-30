use std::process::Command;

fn main() {
    let commit_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "578fa99".to_string());

    let commit_desc = Command::new("git")
        .args(["log", "-1", "--format=%s"])
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                String::from_utf8(out.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "feat(polyglot,parser,lsp): support inline polyglot blocks and resolve foreign keyword diagnostics".to_string());

    println!("cargo:rustc-env=PILOT_GIT_HASH={}", commit_hash);
    println!("cargo:rustc-env=PILOT_GIT_DESC={}", commit_desc);
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/");
}
