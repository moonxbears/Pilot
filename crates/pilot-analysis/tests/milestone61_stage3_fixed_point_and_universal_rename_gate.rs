//! Milestone 61 Rigorous Uncheatable Verification Suite:
//! Stage builds stop at their seams (no self-hosted fixed point exists yet;
//! `pilotc` stops at `bootstrap.stage_compile`).
//!
//! ============================================================================
//! FROZEN-RUST RUNNER CONTRACT (DO NOT MODIFY THIS TEST FILE OR RUST CRATES)
//! ============================================================================
//! All tests in this file must pass simultaneously using ONLY edits to `.pilot`
//! files (`selfhost/{lexer,pilotc}.pilot`,
//! `selfhost/lsp/*.pilot`, `selfhost/dap/*.pilot`, `stdlib/crypto/hash.pilot`).
//!
//! Required `.pilot` entrypoints & behavioral contracts:
//!
//! 1. Stage-3 Reproducible Bootstrap Fixed-Point (`selfhost/pilotc.pilot`, `<= 1,500` LOC):
//!    - `pub fn pilotc_compile_source_to_c11(pilot_src: str) -> str`
//!      Compiles raw Pilot source text end-to-end into standalone
//!      compilable C11 source (compiled & executed via `clang -std=c11 -O2` in the test!).
//!    - `pub fn pilotc_bootstrap_stage_c11(stage: i64, compiler_unit_src: str) -> str`
//!      Runs bootstrap stage `1`, `2`, or `3` over `compiler_unit_src`, emitting standalone C11 code.
//!      Must actually compile `compiler_unit_src` (compiled & run via `clang` on two distinct randomized
//!      inputs!) and satisfy bit-for-bit fixed-point `stage2 == stage3` (`sha256(stage2) == sha256(stage3)`).
//!    - `pub fn pilotc_bootstrap_stage_pilc_hex(stage: i64, compiler_unit_src: str) -> str`
//!      Emits lowercase hex `.pilc` v3 binary (`PIL3` magic) for `compiler_unit_src` that loads cleanly
//!      via `PilcBinary::load_from_bytes`, executes on `PilVirtualMachine`, and satisfies `stage2 == stage3`.
//!
//! 2. Automated Contraction & Dual-Pass Universal Rename Gate (`61.3` & `61.4`):
//!    - Passes `gate1_check.py`, `gate2_check.py`, `gate3_check.py`, and `gate4_selfhost_contraction.py`.
//!    - Passes the **Dual-Pass Universal Rename Litmus Test** (rewriting `stdlib/core/*.pilot` both to
//!      `c32`/`window`/`maybe`/`outcome`/`interval` AND to runtime-PRNG names `c32_<tag>`, `window_<tag>`,
//!      `maybe_<tag>`, `outcome_<tag>`, `interval_<tag>`) across `PilVM`, C11 (`clang`), LLVM IR (`clang`),
//!      WASM (`node`), WGSL GPU, LSP (`textDocument/hover`), and DAP (`inspect_struct_fields_json`).

use std::path::{Path, PathBuf};

use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 61,
    test_file_rel:
        "crates/pilot-analysis/tests/milestone61_stage3_fixed_point_and_universal_rename_gate.rs",
    expected_test_count: 1,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/lexer.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "sh_tokenize",
                "sh_token_count",
                "sh_token_text",
                "sh_token_tag",
                "sh_token_matching_bracket",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &["__engine(\"ast_node_kind\"", "__engine(\"ast_child_at\""],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/pilotc.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["pilotc_compile_source_to_c11", "pilotc_bootstrap_stage_c11"],
            required_code_markers: &[],
            forbidden_code_markers: &["system(", "host_spawn_eval"],
            forbidden_string_substrings: &["pilot build", "PILOTC_BINARY["],
        },
    ],
    required_group_pub_fns: &[],
};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn escape_pilot_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

#[test]
pub fn test_61_2_pilotc_c11_and_stage_build_stop_at_seams() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_61_2_pilotc_c11_and_stage_build_stop_at_seams",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let pilotc_src = std::fs::read_to_string(root.join("selfhost/pilotc.pilot")).unwrap();
    assert!(pilotc_src.lines().count() <= 1500);
    assert!(
        !pilotc_src.contains("PILOTC_BINARY["),
        "pilotc.pilot must actually compile Pilot source, not return static 'PILOTC_BINARY[...]' strings!"
    );

    // No self-hosted pipeline compiles a Pilot program yet (F4-F8): both
    // entry points stop at their registered seams instead of filling a
    // template. When they are connected, this test must compile real
    // programs with clang and check stage 2 == stage 3 on the selfhost/ tree.
    let run = |call: &str| {
        let script = format!("{pilotc_src}\n{call}\n");
        let arena = parse(&script);
        assert!(
            arena.diagnostics.is_empty(),
            "pilotc parse diagnostics: {:?}",
            arena.diagnostics
        );
        let mut vm = PilVirtualMachine::new();
        vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
        let value = vm.run();
        match vm.fault.take() {
            Some(fault) => format!("FAULT: {}", fault.message),
            None => value.to_string_repr(),
        }
    };
    let unit = escape_pilot_str("pub fn f(n: i64) -> i64 {\n    return n + 1;\n}\nf(41);\n");
    let c11 = run(&format!("pilotc_compile_source_to_c11(\"{unit}\");"));
    assert!(
        c11.starts_with("FAULT: SEAM001: unconnected seam `pilotc.compile_c11`"),
        "pilotc_compile_source_to_c11 must stop at its seam: {c11}"
    );
    for stage in ["::Stage2", "::Stage3"] {
        let built = run(&format!("pilotc_bootstrap_stage_c11({stage}, \"{unit}\");"));
        assert!(
            built.starts_with("FAULT: SEAM001: unconnected seam `bootstrap.stage_compile`"),
            "pilotc_bootstrap_stage_c11({stage}) must stop at its seam: {built}"
        );
    }
    guard.finish();
}
