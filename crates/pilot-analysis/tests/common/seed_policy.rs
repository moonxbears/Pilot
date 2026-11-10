//! ==============================================================================
//! crates/pilot-analysis/tests/common/seed_policy.rs
//! Rust Seed Contraction Policy for milestone tests.
//!
//! Single source of truth: `selfhost/gate/manifest.pilot`
//! (`seed_contraction_paused()`, `paused_max_total_rust_loc()`,
//! `paused_max_rust_file_loc()`). Milestone tests that assert a Rust LOC ceiling
//! pass their pre-pause ceiling through these helpers so that the gate and the
//! tests always apply the same rule.
//!
//! While contraction is paused (MILESTONES_100.md Phase F, until F8) the
//! pre-pause per-file and per-crate ceilings are suspended: a single Rust file
//! is held to the `<= 1,500` LOC law and a crate to the paused total budget.
//! When contraction resumes the caller's pre-pause ceiling applies again.
//! ==============================================================================

// Each test binary uses only part of this module.
#![allow(dead_code)]

use std::path::Path;

const MANIFEST_REL: &str = "selfhost/gate/manifest.pilot";

/// Ceiling for one Rust source file.
pub fn seed_rust_file_loc_cap(root: &Path, pre_pause_cap: usize) -> usize {
    seed_cap(root, pre_pause_cap, "paused_max_rust_file_loc")
}

/// Ceiling for a whole crate or for the whole Rust workspace.
pub fn seed_rust_crate_loc_cap(root: &Path, pre_pause_cap: usize) -> usize {
    seed_cap(root, pre_pause_cap, "paused_max_total_rust_loc")
}

fn seed_cap(root: &Path, pre_pause_cap: usize, paused_budget_fn: &str) -> usize {
    let manifest = std::fs::read_to_string(root.join(MANIFEST_REL))
        .unwrap_or_else(|e| panic!("SEED POLICY: cannot read {MANIFEST_REL}: {e}"));
    match manifest_pure_fn_value(&manifest, "seed_contraction_paused").as_str() {
        "true" => manifest_pure_fn_value(&manifest, paused_budget_fn)
            .parse()
            .unwrap_or_else(|e| panic!("SEED POLICY: {paused_budget_fn}() is not a usize literal: {e}")),
        "false" => pre_pause_cap,
        other => panic!("SEED POLICY: seed_contraction_paused() must be `true` or `false`, found `{other}`"),
    }
}

/// Reads the literal body of a one-line `pub pure fn name() -> T => literal;`.
/// Panics instead of guessing if the declaration is missing or reshaped.
fn manifest_pure_fn_value(src: &str, fn_name: &str) -> String {
    let decl = format!("pub pure fn {fn_name}() ->");
    let line = src
        .lines()
        .find(|l| l.trim_start().starts_with(&decl))
        .unwrap_or_else(|| panic!("SEED POLICY: `{decl} ... => <literal>;` not found in {MANIFEST_REL}"));
    let body = line
        .split_once("=>")
        .unwrap_or_else(|| panic!("SEED POLICY: `{fn_name}` in {MANIFEST_REL} must be an expression-bodied `=>` fn"))
        .1;
    body.trim().trim_end_matches(';').trim().to_string()
}
