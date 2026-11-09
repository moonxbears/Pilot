//! ==============================================================================
//! crates/pilot-analysis/tests/common/anticheat.rs
//! Universal 10-Pass Anti-Cheat, Tamper-Seal & Architectural Integrity Engine
//!
//! Shared across `pilot-analysis`, `pilot-parser`, and `pilot-lexer` integration
//! suites via `#[path = ...]` with zero external crate dependencies (`std` only).
//! Every `#[test]` across all 13 milestone suites arms an `AntiCheatGuard` on
//! entry and must complete `guard.finish()` on exit (enforced both by RAII `Drop`
//! and by static self-inspection of all 13 test files).
//! ==============================================================================

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub struct FileAuditRule {
    pub path: &'static str,
    pub max_lines: usize,
    pub name_blind: bool,
    pub required_pub_fns: &'static [&'static str],
    pub required_code_markers: &'static [&'static str],
    pub forbidden_code_markers: &'static [&'static str],
    pub forbidden_string_substrings: &'static [&'static str],
}

pub struct MilestoneAuditSpec {
    pub milestone_id: u32,
    pub test_file_rel: &'static str,
    pub expected_test_count: usize,
    pub files: &'static [FileAuditRule],
    pub required_group_pub_fns: &'static [(&'static str, &'static [&'static str])],
}

pub struct AntiCheatGuard<'a> {
    test_name: &'static str,
    spec: &'a MilestoneAuditSpec,
    root: PathBuf,
    finished: bool,
}

impl<'a> AntiCheatGuard<'a> {
    pub fn arm(test_name: &'static str, spec: &'a MilestoneAuditSpec) -> Self {
        let root = workspace_root();
        // Pre-flight architectural gate & crate-tree integrity check before running test code.
        verify_live_pilot_gate_and_crate_tree(&root, test_name, spec.milestone_id);
        Self {
            test_name,
            spec,
            root,
            finished: false,
        }
    }

    pub fn finish(mut self) {
        self.finished = true;
        run_all_ten_anticheat_passes(&self.root, self.test_name, self.spec);
    }
}

impl<'a> Drop for AntiCheatGuard<'a> {
    fn drop(&mut self) {
        if !self.finished && !std::thread::panicking() {
            panic!(
                "ANTI-CHEAT EARLY-RETURN BYPASS [M{}::{}]: Test exited without executing `guard.finish()`!",
                self.spec.milestone_id, self.test_name
            );
        }
    }
}

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

// ==============================================================================
// Pure-Rust FIPS 180-4 SHA-256 Implementation (Zero External Crate Dependencies)
// ==============================================================================

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

pub fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let bit_len = (data.len() as u64) * 8;
    let mut padded = Vec::with_capacity(((data.len() + 9 + 63) / 64) * 64);
    padded.extend_from_slice(data);
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = String::with_capacity(64);
    for word in h {
        out.push_str(&format!("{:08x}", word));
    }
    out
}

// ==============================================================================
// Pass 1 Manifest: Surviving Bootstrap Crates & Live Pure-Pilot Gate Seal
// ==============================================================================

const SURVIVING_CRATES: &[&str] = &[
    "pilot-analysis", "pilot-arena", "pilot-ast", "pilot-cli", "pilot-codegen", "pilot-ffi",
    "pilot-intern", "pilot-lexer", "pilot-parser", "pilot-rt", "pilot-span", "pilot-vm",
];

const RETIRED_PYTHON_GATE_SCRIPTS: &[&str] = &[
    "scripts/gate1_check.py", "scripts/gate2_check.py", "scripts/gate3_check.py",
    "scripts/gate4_selfhost_contraction.py", "scripts/convert_dunders.py",
];

const MILESTONE_TEST_MANIFEST: &[(&str, usize)] = &[
    ("crates/pilot-analysis/tests/milestone01_seed_unlock_and_pilot_gate.rs", 4),
    ("crates/pilot-analysis/tests/milestone02_module_graph_and_comptime_parity.rs", 4),
    ("crates/pilot-analysis/tests/milestone03_declarative_cli_registry_and_comptime_parity.rs", 4),
    ("crates/pilot-analysis/tests/milestone04_dual_syntax_pairings_and_bidirectional_formatter.rs", 4),
    ("crates/pilot-analysis/tests/milestone05_shared_incremental_compiler_query_database.rs", 4),
    ("crates/pilot-analysis/tests/milestone50_lossless_bridge_and_reflection.rs", 4),
    ("crates/pilot-parser/tests/milestone51_homoiconic_syntax_and_contraction.rs", 4),
    ("crates/pilot-lexer/tests/milestone52_seed_lexer_and_preprocessor.rs", 4),
    ("crates/pilot-analysis/tests/milestone52_pilot_preprocessor_and_receptors.rs", 3),
    ("crates/pilot-analysis/tests/milestone53_cst_recovery_and_formatter.rs", 3),
    ("crates/pilot-analysis/tests/milestone54_perceus_ownership_and_escape.rs", 2),
    ("crates/pilot-analysis/tests/milestone55_selfhost_vm_compiler_and_debug_table.rs", 3),
    ("crates/pilot-analysis/tests/milestone56_layout_driven_codegen_and_gpu_backends.rs", 3),
    ("crates/pilot-analysis/tests/milestone57_bidirectional_transpilers_and_ingest.rs", 2),
    ("crates/pilot-analysis/tests/milestone58_runtime_crypto_wire_db_and_polyglot.rs", 7),
    ("crates/pilot-analysis/tests/milestone59_persistent_pilvm_lsp_and_dap.rs", 3),
    ("crates/pilot-analysis/tests/milestone60_selfhost_cli_pkg_perf_and_bench.rs", 3),
    ("crates/pilot-analysis/tests/milestone61_stage3_fixed_point_and_universal_rename_gate.rs", 3),
    ("crates/pilot-analysis/tests/mega_all_milestones_simultaneous_gate.rs", 1),
];

const GLOBAL_BANNED_LEAK_SUBSTRINGS: &[&str] = &[
    // Known cryptographic digest / RFC 8032 test-vector outputs (never valid in real algorithmic code)
    "e3b0c44298fc1c14", "ba7816bf8f01cfea", "cf83e1357eefb8bd", "af1349b9f5f9a1a6",
    "d75a980182b10ab7", "e5564300c360ac72", "3d4017c3e843895a", "92a009a9f0d4cab8",
    "72bad3de6140207b",
    // Test-harness magic delimiters
    "@@SEP@@", "@@H@@", "@@LIT@@", "@@WASM@@", "@@SQL@@", "@@ED@@", "@@DBG@@", "@@TR@@",
    "@@OPT@@", "@@DIFF@@", "@@WIRE@@", "@@SPLIT@@", "@@LSP@@", "@@DAP@@", "@@PKG@@",
    "@@PERF@@", "@@CLI@@", "@@FE@@", "@@BS@@", "<<<SEP>>>", "<<<DBG>>>", "<<<W>>>",
    "<<<T>>>", "===HDR===", "===SEL===",
    // Test-only adversarial decoy & probe identifiers
    "dsp_audio_synth", "litmus_compute", "litmus_shader", "PacketFrame_", "pilot_ffi_step_",
    "dead_branch_unreachable", "bench_target_", "WholeNum", "TruthBit", "Utf8Run",
];

const BANNED_STUB_MARKERS: &[&str] = &[
    "TODO", "FIXME", "UNIMPLEMENTED", "PLACEHOLDER",
    "HACK_BYPASS", "MOCK_IMPL", "todo!(", "unimplemented!(",
];

const BANNED_NAME_BLIND_TYPE_STRINGS: &[&str] = &[
    "i64", "u8", "f64", "char", "bool", "usize", "slice", "stride", "option", "result",
    "range", "SliceBuffer", "RangeWindow", "Dual", "Signal", "EffectPlan",
];

const DELETED_LEGACY_PILOT_FILES: &[&str] = &[
    "selfhost/scope_header.pilot",
    "selfhost/emitter.pilot",
    "selfhost/ide_sync.pilot",
];

// ==============================================================================
// Pass 1..10 Master Orchestrator
// ==============================================================================

fn run_all_ten_anticheat_passes(root: &Path, test_name: &str, spec: &MilestoneAuditSpec) {
    // Pass 1..4 static workspace-wide sweeps cached once per test process
    static WORKSPACE_SWEEPS_OK: OnceLock<()> = OnceLock::new();
    WORKSPACE_SWEEPS_OK.get_or_init(|| {
        verify_live_pilot_gate_and_crate_tree(root, test_name, spec.milestone_id);
        verify_no_smuggled_foreign_or_binary_artifacts(root, test_name, spec.milestone_id);
        verify_deleted_legacy_and_global_pilot_caps(root, test_name, spec.milestone_id);
    });

    // Pass 3: Test-Suite Self-Integrity & Anti-Skip Guard (spec-checked + cached manifest scan)
    verify_test_suite_self_integrity(root, test_name, spec);

    // Passes 5–10: Cached per `(milestone_id, test_file_rel)` so multi-test suites audit each file once
    static SPEC_CACHE: OnceLock<Mutex<HashSet<(u32, &'static str)>>> = OnceLock::new();
    let cache = SPEC_CACHE.get_or_init(|| Mutex::new(HashSet::new()));
    let key = (spec.milestone_id, spec.test_file_rel);
    let already_done = cache.lock().unwrap_or_else(|e| e.into_inner()).contains(&key);
    if !already_done {
        for rule in spec.files {
            audit_single_pilot_file(root, test_name, spec.milestone_id, rule);
        }
        for &(req_fn, candidate_paths) in spec.required_group_pub_fns {
            verify_group_pub_fn(root, test_name, spec.milestone_id, req_fn, candidate_paths);
        }
        cache.lock().unwrap_or_else(|e| e.into_inner()).insert(key);
    }
}

fn verify_live_pilot_gate_and_crate_tree(root: &Path, test_name: &str, milestone_id: u32) {
    static PASS1_OK: OnceLock<()> = OnceLock::new();
    PASS1_OK.get_or_init(|| {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "Internal anti-cheat SHA-256 self-check failed"
        );
        for retired_py in RETIRED_PYTHON_GATE_SCRIPTS {
            assert!(
                !root.join(retired_py).exists(),
                "ANTI-CHEAT [PASS 1: RETIRED_SCRIPT_RESURRECTED] [M{}::{}]: Legacy Python script '{}' reappeared!",
                milestone_id, test_name, retired_py
            );
        }
        let allowed_crates: HashSet<&str> = SURVIVING_CRATES.iter().copied().collect();
        for entry in std::fs::read_dir(root.join("crates")).expect("read crates/") {
            let path = entry.unwrap().path();
            if path.is_dir() {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                assert!(
                    allowed_crates.contains(name.as_str()),
                    "ANTI-CHEAT [PASS 1: UNAUTHORIZED_CRATE] [M{}::{}]: Illegal or resurrected crate directory 'crates/{}'!",
                    milestone_id, test_name, name
                );
                assert!(
                    !path.join("build.rs").exists(),
                    "ANTI-CHEAT [PASS 1: ILLEGAL_BUILD_RS] [M{}::{}]: Unauthorized build.rs in 'crates/{}'!",
                    milestone_id, test_name, name
                );
            }
        }
        let bin = root.join("target/debug/pilot");
        if !bin.exists() {
            let build_st = std::process::Command::new("cargo")
                .args(["build", "-q", "-p", "pilot-cli"])
                .current_dir(root)
                .status()
                .expect("cargo build -p pilot-cli");
            assert!(build_st.success(), "ANTI-CHEAT [PASS 1]: Failed building pilot-cli");
        }
        let out = std::process::Command::new(&bin)
            .args(["gate", "--json"])
            .current_dir(root)
            .output()
            .expect("spawn target/debug/pilot gate --json");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success()
                && stdout.contains("\"passed\":true")
                && stdout.contains("\"total_violations\":0"),
            "ANTI-CHEAT [PASS 1: LIVE_PILOT_GATE_FAILED] [M{}::{}]: Pure-Pilot `pilot gate --json` failed!\n{}{}",
            milestone_id,
            test_name,
            stdout,
            String::from_utf8_lossy(&out.stderr)
        );
    });
}

fn verify_no_smuggled_foreign_or_binary_artifacts(root: &Path, test_name: &str, milestone_id: u32) {
    let banned_exts: HashSet<&str> = [
        "py", "pyc", "js", "mjs", "cjs", "ts", "sh", "bash", "so", "dylib", "dll", "a", "o",
        "wasm", "exe", "c", "h", "cpp", "rs", "db", "sqlite", "sqlite3", "sql", "svg",
    ].into_iter().collect();

    for top in ["stdlib", "selfhost"] {
        let mut stack = vec![root.join(top)];
        while let Some(dir) = stack.pop() {
            if !dir.exists() { continue; }
            for entry in std::fs::read_dir(&dir).unwrap() {
                let p = entry.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                    assert!(
                        !banned_exts.contains(ext.as_str()),
                        "ANTI-CHEAT [PASS 2: SMUGGLED_ARTIFACT] [M{}::{}]: Illegal foreign script/binary '{}' found in {}/!",
                        milestone_id, test_name, rel, top
                    );
                    if ext == "pilc" {
                        let bytes = std::fs::read(&p).unwrap_or_default();
                        assert!(
                            bytes.len() >= 4 && &bytes[0..4] == b"PIL3",
                            "ANTI-CHEAT [PASS 2: CORRUPT_OR_FAKE_PILC] [M{}::{}]: '{}' does not begin with PIL3 magic header!",
                            milestone_id, test_name, rel
                        );
                    } else if ext != "pilot" && ext != "tmp" {
                        assert!(
                            rel == "stdlib/pilot.json" || rel == "stdlib/THIRD_PARTY_NOTICES.md",
                            "ANTI-CHEAT [PASS 2: UNEXPECTED_DATA_FIXTURE] [M{}::{}]: Unauthorized non-.pilot file '{}' in {}/!",
                            milestone_id, test_name, rel, top
                        );
                    }
                }
            }
        }
    }
}

fn verify_test_suite_self_integrity(root: &Path, test_name: &str, spec: &MilestoneAuditSpec) {
    let matched_current = MILESTONE_TEST_MANIFEST.iter().any(|&(rel_path, min_tests)| {
        if rel_path == spec.test_file_rel {
            assert_eq!(
                min_tests, spec.expected_test_count,
                "ANTI-CHEAT [PASS 3: TEST_COUNT_DRIFT] [M{}::{}]: Spec expected_test_count mismatch for {}",
                spec.milestone_id, test_name, rel_path
            );
            true
        } else {
            false
        }
    });
    assert!(
        matched_current,
        "ANTI-CHEAT [PASS 3: UNREGISTERED_SPEC] [M{}::{}]: '{}' missing from MILESTONE_TEST_MANIFEST",
        spec.milestone_id, test_name, spec.test_file_rel
    );

    static PASS3_MANIFEST_OK: OnceLock<()> = OnceLock::new();
    PASS3_MANIFEST_OK.get_or_init(|| {
        for &(rel_path, min_tests) in MILESTONE_TEST_MANIFEST {
            let raw = std::fs::read_to_string(root.join(rel_path)).unwrap_or_else(|e| {
                panic!("ANTI-CHEAT [PASS 3: MISSING_MILESTONE_TEST] [M{}::{}]: '{}' missing: {}", spec.milestone_id, test_name, rel_path, e)
            });
            let code_only: String = raw.lines().map(|l| l.split("//").next().unwrap_or("")).collect::<Vec<_>>().join("\n");

            for forbidden_attr in ["#[ignore", "#[should_panic", "process::exit("] {
                assert!(
                    !code_only.contains(forbidden_attr),
                    "ANTI-CHEAT [PASS 3: TEST_BYPASS_ATTR] [M{}::{}]: Forbidden '{}' detected in '{}'!",
                    spec.milestone_id, test_name, forbidden_attr, rel_path
                );
            }

            let test_attr_cnt = code_only.matches("#[test]").count();
            assert!(
                test_attr_cnt >= min_tests,
                "ANTI-CHEAT [PASS 3: DELETED_TEST_FN] [M{}::{}]: '{}' has only {} #[test] functions (expected >= {})!",
                spec.milestone_id, test_name, rel_path, test_attr_cnt, min_tests
            );

            let (arm_cnt, fin_cnt) = (code_only.matches("AntiCheatGuard::arm(").count(), code_only.matches(".finish();").count());
            assert!(
                arm_cnt >= min_tests && fin_cnt >= min_tests,
                "ANTI-CHEAT [PASS 3: UNHOOKED_ANTICHEAT_GUARD] [M{}::{}]: '{}' has {} #[test]s but arm={} / finish={}!",
                spec.milestone_id, test_name, rel_path, min_tests, arm_cnt, fin_cnt
            );

            if rel_path == "crates/pilot-analysis/tests/mega_all_milestones_simultaneous_gate.rs" {
                for mod_prefix in [
                    "m01::test_", "m50::test_", "m51::test_", "m52_lexer::test_", "m52_analysis::test_",
                    "m53::test_", "m54::test_", "m55::test_", "m56::test_", "m57::test_",
                    "m58::test_", "m59::test_", "m60::test_", "m61::test_",
                ] {
                    assert!(
                        code_only.contains(mod_prefix),
                        "ANTI-CHEAT [PASS 3: GUTTED_MEGA_TEST] [M{}::{}]: Mega test is missing invocation of '{}'!",
                        spec.milestone_id, test_name, mod_prefix
                    );
                }
                let mega_calls = code_only.matches("::test_").count();
                assert!(
                    mega_calls >= 48 && code_only.matches("snapshot_all_pilot_sha256(").count() >= 2,
                    "ANTI-CHEAT [PASS 3: GUTTED_MEGA_TEST] [M{}::{}]: Mega test must invoke all 48 milestone tests (found {}) and verify pre/post SHA-256 .pilot snapshots!",
                    spec.milestone_id, test_name, mega_calls
                );
            }
        }
        verify_governance_docs_integrity(root, test_name, spec.milestone_id);
    });
}

fn verify_governance_docs_integrity(root: &Path, test_name: &str, milestone_id: u32) {
    for doc in ["AGENTS.md", "VISION.md", "MILESTONES_100.md"] {
        let meta = std::fs::metadata(root.join(doc)).unwrap_or_else(|e| {
            panic!(
                "ANTI-CHEAT [PASS 3B: MISSING_GOVERNANCE_DOC] [M{}::{}]: '{}' missing at workspace root: {}",
                milestone_id, test_name, doc, e
            )
        });
        assert!(
            meta.len() > 256,
            "ANTI-CHEAT [PASS 3B: EMPTY_GOVERNANCE_DOC] [M{}::{}]: '{}' is unexpectedly empty!",
            milestone_id, test_name, doc
        );
    }
}

fn verify_deleted_legacy_and_global_pilot_caps(root: &Path, test_name: &str, milestone_id: u32) {
    for legacy in DELETED_LEGACY_PILOT_FILES {
        assert!(
            !root.join(legacy).exists(),
            "ANTI-CHEAT [PASS 4: RESURRECTED_LEGACY_FILE] [M{}::{}]: Deleted legacy file '{}' reappeared on disk!",
            milestone_id, test_name, legacy
        );
    }

    for top in ["selfhost", "stdlib"] {
        let mut stack = vec![root.join(top)];
        while let Some(dir) = stack.pop() {
            if !dir.exists() {
                continue;
            }
            for entry in std::fs::read_dir(&dir).unwrap() {
                let p = entry.unwrap().path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|e| e.to_str()) == Some("pilot") {
                    let rel = p
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    let lines = std::fs::read_to_string(&p).unwrap().lines().count();
                    assert!(
                        lines <= 1500,
                        "ANTI-CHEAT [PASS 4: GLOBAL_PILOT_LINE_CAP] [M{}::{}]: '{}' has {} lines (hard cap is 1,500)!",
                        milestone_id, test_name, rel, lines
                    );
                }
            }
        }
    }
}

// ==============================================================================
// Stateful Pilot Lexical & Function-Body Scanner (Passes 5, 6, 7, 8, 9, 10)
// ==============================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
enum TokKind {
    Ident,
    Number,
    Punct,
    StrLit,
    FStrLit,
    CharLit,
}

#[derive(Clone, Debug)]
struct PilotTok {
    kind: TokKind,
    text: String,
    line: usize,
}

#[derive(Clone, Debug)]
struct ExtractedFn {
    is_pub: bool,
    is_extern: bool,
    params: Vec<String>,
    body_tokens: Vec<PilotTok>,
}

struct ScannedPilotFile {
    code_only_text: String,
    tokens: Vec<PilotTok>,
    string_literals: Vec<(usize, String)>,
    functions: HashMap<String, Vec<ExtractedFn>>,
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn scan_pilot_source(rel_path: &str, raw: &str, milestone_id: u32, test_name: &str) -> ScannedPilotFile {
    let bytes = raw.as_bytes();
    let n = bytes.len();
    let mut i = 0usize;
    let mut line = 1usize;
    let mut at_line_start = true;
    let mut code_only = String::with_capacity(n);
    let mut tokens = Vec::new();
    let mut string_literals = Vec::new();

    let valid_preproc: HashSet<&str> = [
        "define", "undef", "if", "elif", "else", "endif", "region", "endregion", "error",
        "warning", "pragma",
    ]
    .into_iter()
    .collect();

    while i < n {
        let b = bytes[i];
        if b == b'\n' {
            line += 1;
            at_line_start = true;
            code_only.push('\n');
            i += 1;
            continue;
        }
        if b == b' ' || b == b'\t' || b == b'\r' {
            code_only.push(b as char);
            i += 1;
            continue;
        }

        // Line comment `//`
        if b == b'/' && i + 1 < n && bytes[i + 1] == b'/' {
            while i < n && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // Block comment `/* ... */`
        if b == b'/' && i + 1 < n && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < n && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                if bytes[i] == b'\n' {
                    line += 1;
                    code_only.push('\n');
                }
                i += 1;
            }
            i = (i + 2).min(n);
            continue;
        }

        // Check line-leading `#` (Pythonic `#` comments banned by AGENTS.md §2.8)
        if at_line_start && b == b'#' {
            at_line_start = false;
            if i + 1 < n && bytes[i + 1] == b'!' {
                while i < n && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            let mut j = i + 1;
            while j < n && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            let w_start = j;
            while j < n && is_ident_continue(bytes[j]) {
                j += 1;
            }
            let directive = &raw[w_start..j];
            assert!(
                valid_preproc.contains(directive),
                "ANTI-CHEAT [PASS 5: PROHIBITED_HASH_COMMENT] [M{}::{}]: Illegal '#' comment at {}:{} (use '//' per AGENTS.md §2.8)!",
                milestone_id, test_name, rel_path, line
            );
            while i < n && bytes[i] != b'\n' {
                code_only.push(bytes[i] as char);
                i += 1;
            }
            continue;
        }
        at_line_start = false;

        // Ban legacy `$"..."` string interpolation in Normal state (AGENTS.md §2.4)
        assert!(
            !(b == b'$' && i + 1 < n && bytes[i + 1] == b'"'),
            "ANTI-CHEAT [PASS 5: LEGACY_DOLLAR_INTERP] [M{}::{}]: Prohibited `$\"...\"` string interpolation at {}:{} (use `f\"...\"`)!",
            milestone_id, test_name, rel_path, line
        );

        // `f"..."` interpolated string literal (with `{ ... }` nested brace & inner string tracking)
        if b == b'f' && i + 1 < n && bytes[i + 1] == b'"' {
            let start_line = line;
            let raw_start = i;
            i += 2;
            let mut brace_depth = 0usize;
            while i < n {
                let c = bytes[i];
                if c == b'\n' {
                    line += 1;
                }
                if c == b'\\' {
                    i = (i + 2).min(n);
                    continue;
                }
                if brace_depth == 0 {
                    if c == b'"' {
                        i += 1;
                        break;
                    }
                    if c == b'{' {
                        if i + 1 < n && bytes[i + 1] == b'{' {
                            i += 2;
                            continue;
                        }
                        brace_depth = 1;
                    }
                } else if c == b'"' {
                    i += 1;
                    while i < n {
                        if bytes[i] == b'\\' {
                            i = (i + 2).min(n);
                            continue;
                        }
                        if bytes[i] == b'\n' {
                            line += 1;
                        }
                        if bytes[i] == b'"' {
                            i += 1;
                            break;
                        }
                        i += 1;
                    }
                    continue;
                } else if c == b'{' {
                    brace_depth += 1;
                } else if c == b'}' {
                    brace_depth -= 1;
                }
                i += 1;
            }
            let f_txt = raw[raw_start..i].to_string();
            code_only.push_str(&f_txt);
            string_literals.push((start_line, f_txt.clone()));
            tokens.push(PilotTok {
                kind: TokKind::FStrLit,
                text: f_txt,
                line: start_line,
            });
            continue;
        }

        // Standard `"..."` string literal
        if b == b'"' {
            let start_line = line;
            i += 1;
            let mut val = String::new();
            while i < n {
                let c = bytes[i];
                if c == b'\n' {
                    line += 1;
                }
                if c == b'\\' && i + 1 < n {
                    val.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                if c == b'"' {
                    i += 1;
                    break;
                }
                val.push(c as char);
                i += 1;
            }
            code_only.push('"');
            code_only.push_str(&val);
            code_only.push('"');
            string_literals.push((start_line, val.clone()));
            tokens.push(PilotTok {
                kind: TokKind::StrLit,
                text: val,
                line: start_line,
            });
            continue;
        }

        // Character literal `'a'`
        if b == b'\'' {
            let start_line = line;
            let s = i;
            i += 1;
            while i < n {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(n);
                    continue;
                }
                if bytes[i] == b'\'' || bytes[i] == b'\n' {
                    if bytes[i] == b'\'' {
                        i += 1;
                    }
                    break;
                }
                i += 1;
            }
            let ch_txt = raw[s..i].to_string();
            code_only.push_str(&ch_txt);
            tokens.push(PilotTok {
                kind: TokKind::CharLit,
                text: ch_txt,
                line: start_line,
            });
            continue;
        }

        // Identifier
        if is_ident_start(b) {
            let s = i;
            i += 1;
            while i < n && is_ident_continue(bytes[i]) {
                i += 1;
            }
            let ident = raw[s..i].to_string();
            code_only.push_str(&ident);
            tokens.push(PilotTok {
                kind: TokKind::Ident,
                text: ident,
                line,
            });
            continue;
        }

        // Number
        if b.is_ascii_digit() {
            let s = i;
            i += 1;
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'.') {
                if bytes[i] == b'.' && i + 1 < n && bytes[i + 1] == b'.' {
                    break;
                }
                i += 1;
            }
            let num = raw[s..i].to_string();
            code_only.push_str(&num);
            tokens.push(PilotTok {
                kind: TokKind::Number,
                text: num,
                line,
            });
            continue;
        }

        // Two-char punctuation (`=>`, `->`, `::`, `..`, `==`, `!=`, `<=`, `>=`, `&&`, `||`, `<<`, `>>`)
        if i + 1 < n {
            let two = &raw[i..i + 2];
            if matches!(
                two,
                "=>" | "->" | "::" | ".." | "==" | "!=" | "<=" | ">=" | "&&" | "||" | "<<" | ">>"
            ) {
                code_only.push_str(two);
                tokens.push(PilotTok {
                    kind: TokKind::Punct,
                    text: two.to_string(),
                    line,
                });
                i += 2;
                continue;
            }
        }

        let one = (b as char).to_string();
        code_only.push(b as char);
        tokens.push(PilotTok {
            kind: TokKind::Punct,
            text: one,
            line,
        });
        i += 1;
    }

    let functions = extract_functions(&tokens);
    ScannedPilotFile {
        code_only_text: code_only,
        tokens,
        string_literals,
        functions,
    }
}

fn extract_functions(tokens: &[PilotTok]) -> HashMap<String, Vec<ExtractedFn>> {
    let mut out: HashMap<String, Vec<ExtractedFn>> = HashMap::new();
    let len = tokens.len();
    let mut section_pub_stack: Vec<bool> = vec![false];
    let mut idx = 0usize;

    while idx < len {
        let t = &tokens[idx];
        if t.kind == TokKind::Punct && t.text == "{" {
            let mut block_pub = *section_pub_stack.last().unwrap_or(&false);
            // Check if `{` opens a distributive modifier block (`pub { ... }` / `pub static pure { ... }`)
            let mut back = idx;
            let mut saw_mod_vis: Option<bool> = None;
            while back > 0 {
                let prev = &tokens[back - 1];
                if prev.kind == TokKind::Ident
                    && matches!(
                        prev.text.as_str(),
                        "pub"
                            | "export"
                            | "open"
                            | "priv"
                            | "prot"
                            | "pkg"
                            | "file"
                            | "int"
                            | "pure"
                            | "static"
                            | "partial"
                            | "noalloc"
                            | "comptime"
                            | "async"
                            | "unsafe"
                    )
                {
                    if matches!(prev.text.as_str(), "pub" | "export" | "open") {
                        saw_mod_vis = Some(true);
                    } else if matches!(prev.text.as_str(), "priv" | "prot" | "pkg" | "file" | "int") {
                        saw_mod_vis = Some(false);
                    }
                    back -= 1;
                } else {
                    break;
                }
            }
            if let Some(v) = saw_mod_vis {
                let at_stmt_start = back == 0
                    || (tokens[back - 1].kind == TokKind::Punct
                        && matches!(tokens[back - 1].text.as_str(), "{" | "}" | ";" | ":"));
                if at_stmt_start {
                    block_pub = v;
                }
            }
            section_pub_stack.push(block_pub);
            idx += 1;
            continue;
        }
        if t.kind == TokKind::Punct && t.text == "}" {
            if section_pub_stack.len() > 1 {
                section_pub_stack.pop();
            }
            idx += 1;
            continue;
        }

        // Distributive visibility section labels (`pub:`, `priv:`, `pub partial static pure:`, etc.)
        if t.kind == TokKind::Ident
            && idx + 1 < len
            && tokens[idx + 1].kind == TokKind::Punct
            && tokens[idx + 1].text == ":"
            && matches!(
                t.text.as_str(),
                "pub"
                    | "export"
                    | "open"
                    | "priv"
                    | "prot"
                    | "pkg"
                    | "file"
                    | "int"
                    | "pure"
                    | "static"
                    | "partial"
                    | "noalloc"
                    | "comptime"
                    | "async"
                    | "unsafe"
            )
        {
            let mut back = idx + 1;
            let mut label_vis: Option<bool> = None;
            while back > 0 {
                let prev = &tokens[back - 1];
                if prev.kind == TokKind::Ident
                    && matches!(
                        prev.text.as_str(),
                        "pub"
                            | "export"
                            | "open"
                            | "priv"
                            | "prot"
                            | "pkg"
                            | "file"
                            | "int"
                            | "pure"
                            | "static"
                            | "partial"
                            | "noalloc"
                            | "comptime"
                            | "async"
                            | "unsafe"
                    )
                {
                    if matches!(prev.text.as_str(), "pub" | "export" | "open") {
                        label_vis = Some(true);
                    } else if matches!(prev.text.as_str(), "priv" | "prot" | "pkg" | "file" | "int") {
                        label_vis = Some(false);
                    }
                    back -= 1;
                } else {
                    break;
                }
            }
            let at_stmt_start = back == 0
                || (tokens[back - 1].kind == TokKind::Punct
                    && matches!(tokens[back - 1].text.as_str(), "{" | "}" | ";" | ":"));
            if at_stmt_start {
                if let (Some(v), Some(top)) = (label_vis, section_pub_stack.last_mut()) {
                    *top = v;
                }
                idx += 2;
                continue;
            }
        }

        if t.kind == TokKind::Ident && t.text == "fn" {
            // Inspect preceding modifiers on the current declaration header
            let mut has_pub = *section_pub_stack.last().unwrap_or(&false);
            let mut is_extern = false;
            let mut back = idx;
            while back > 0 {
                let prev = &tokens[back - 1];
                if prev.kind == TokKind::Ident
                    && matches!(
                        prev.text.as_str(),
                        "pub"
                            | "export"
                            | "open"
                            | "priv"
                            | "prot"
                            | "pkg"
                            | "file"
                            | "int"
                            | "pure"
                            | "static"
                            | "noalloc"
                            | "comptime"
                            | "ext"
                            | "extern"
                            | "async"
                            | "unsafe"
                    )
                {
                    if matches!(prev.text.as_str(), "pub" | "export" | "open") {
                        has_pub = true;
                    }
                    if matches!(prev.text.as_str(), "priv" | "prot" | "pkg" | "file" | "int") {
                        has_pub = false;
                    }
                    if prev.text == "extern" {
                        is_extern = true;
                    }
                    back -= 1;
                } else {
                    break;
                }
            }

            // Advance to function name (supporting `fn name(...)` and `ext fn Receiver.name(...)`)
            let mut cur = idx + 1;
            if cur < len && tokens[cur].kind == TokKind::Ident {
                let mut fn_name = tokens[cur].text.clone();
                cur += 1;
                // Skip optional `<T>` on receiver
                if cur < len && tokens[cur].kind == TokKind::Punct && tokens[cur].text == "<" {
                    let mut ad = 1usize;
                    cur += 1;
                    while cur < len && ad > 0 {
                        if tokens[cur].kind == TokKind::Punct && tokens[cur].text == "<" {
                            ad += 1;
                        } else if tokens[cur].kind == TokKind::Punct && tokens[cur].text == ">" {
                            ad -= 1;
                        } else if tokens[cur].kind == TokKind::Punct && tokens[cur].text == ">>" {
                            ad = ad.saturating_sub(2);
                        }
                        cur += 1;
                    }
                }
                if cur + 1 < len
                    && tokens[cur].kind == TokKind::Punct
                    && tokens[cur].text == "."
                    && tokens[cur + 1].kind == TokKind::Ident
                {
                    fn_name = tokens[cur + 1].text.clone();
                    cur += 2;
                }
                // Skip optional generic parameter list `<T, U>` on function
                if cur < len && tokens[cur].kind == TokKind::Punct && tokens[cur].text == "<" {
                    let mut ad = 1usize;
                    cur += 1;
                    while cur < len && ad > 0 {
                        if tokens[cur].kind == TokKind::Punct && tokens[cur].text == "<" {
                            ad += 1;
                        } else if tokens[cur].kind == TokKind::Punct && tokens[cur].text == ">" {
                            ad -= 1;
                        } else if tokens[cur].kind == TokKind::Punct && tokens[cur].text == ">>" {
                            ad = ad.saturating_sub(2);
                        }
                        cur += 1;
                    }
                }

                // Parse parameter list `(...)`
                if cur < len && tokens[cur].kind == TokKind::Punct && tokens[cur].text == "(" {
                    let p_start = cur + 1;
                    let mut pd = 1usize;
                    cur += 1;
                    while cur < len && pd > 0 {
                        if tokens[cur].kind == TokKind::Punct && tokens[cur].text == "(" {
                            pd += 1;
                        } else if tokens[cur].kind == TokKind::Punct && tokens[cur].text == ")" {
                            pd -= 1;
                        }
                        cur += 1;
                    }
                    let p_end = cur.saturating_sub(1);
                    let params = parse_param_names(&tokens[p_start..p_end]);

                    // Advance past return type / contracts (`requires` / `ensures`) to `{`, `=>`, or `;`
                    let after_params_idx = cur;
                    let mut paren_d = 0usize;
                    let mut bracket_d = 0usize;
                    let mut saw_ret_arrow = false;
                    while cur < len {
                        let tk = &tokens[cur];
                        if tk.kind == TokKind::Punct {
                            match tk.text.as_str() {
                                "->" if paren_d == 0 && bracket_d == 0 => saw_ret_arrow = true,
                                "(" => paren_d += 1,
                                ")" => paren_d = paren_d.saturating_sub(1),
                                "[" => bracket_d += 1,
                                "]" => bracket_d = bracket_d.saturating_sub(1),
                                ";" if paren_d == 0 && bracket_d == 0 => {
                                    out.entry(fn_name).or_default().push(ExtractedFn {
                                        is_pub: has_pub,
                                        is_extern: true,
                                        params,
                                        body_tokens: Vec::new(),
                                    });
                                    break;
                                }
                                "=>" if paren_d == 0 && bracket_d == 0 => {
                                    // Distinguish function return type `-> (A) => R` from inline body `=> expr;`
                                    let is_fn_ret_type_arrow = saw_ret_arrow
                                        && cur > after_params_idx
                                        && tokens[cur - 1].kind == TokKind::Punct
                                        && tokens[cur - 1].text == ")";
                                    if !is_fn_ret_type_arrow {
                                        let b_start = cur + 1;
                                        let mut b_cur = b_start;
                                        let mut bd = 0usize;
                                        while b_cur < len {
                                            if tokens[b_cur].kind == TokKind::Punct {
                                                if matches!(tokens[b_cur].text.as_str(), "{" | "(" | "[") {
                                                    bd += 1;
                                                } else if matches!(tokens[b_cur].text.as_str(), "}" | ")" | "]") {
                                                    bd = bd.saturating_sub(1);
                                                } else if tokens[b_cur].text == ";" && bd == 0 {
                                                    break;
                                                }
                                            }
                                            b_cur += 1;
                                        }
                                        out.entry(fn_name).or_default().push(ExtractedFn {
                                            is_pub: has_pub,
                                            is_extern,
                                            params,
                                            body_tokens: tokens[b_start..b_cur].to_vec(),
                                        });
                                        break;
                                    }
                                }
                                "{" if paren_d == 0 && bracket_d == 0 => {
                                    let b_start = cur + 1;
                                    let mut b_cur = b_start;
                                    let mut bd = 1usize;
                                    while b_cur < len && bd > 0 {
                                        if tokens[b_cur].kind == TokKind::Punct {
                                            if tokens[b_cur].text == "{" {
                                                bd += 1;
                                            } else if tokens[b_cur].text == "}" {
                                                bd -= 1;
                                            }
                                        }
                                        b_cur += 1;
                                    }
                                    let b_end = b_cur.saturating_sub(1);
                                    out.entry(fn_name).or_default().push(ExtractedFn {
                                        is_pub: has_pub,
                                        is_extern,
                                        params,
                                        body_tokens: tokens[b_start..b_end].to_vec(),
                                    });
                                    break;
                                }
                                _ => {}
                            }
                        }
                        cur += 1;
                    }
                }
            }
        }
        idx += 1;
    }
    out
}

fn parse_param_names(param_tokens: &[PilotTok]) -> Vec<String> {
    let mut names = Vec::new();
    let mut seg_start = 0usize;
    let mut depth = 0usize;
    for (i, t) in param_tokens.iter().enumerate() {
        if t.kind == TokKind::Punct {
            match t.text.as_str() {
                "<" | "(" | "[" | "{" => depth += 1,
                ">" | ")" | "]" | "}" => depth = depth.saturating_sub(1),
                ">>" => depth = depth.saturating_sub(2),
                "," if depth == 0 => {
                    if let Some(nm) = extract_single_param_name(&param_tokens[seg_start..i]) {
                        names.push(nm);
                    }
                    seg_start = i + 1;
                }
                _ => {}
            }
        }
    }
    if seg_start < param_tokens.len() {
        if let Some(nm) = extract_single_param_name(&param_tokens[seg_start..]) {
            names.push(nm);
        }
    }
    names
}

fn extract_single_param_name(seg: &[PilotTok]) -> Option<String> {
    // Find identifier immediately preceding `:` (or last non-modifier identifier if untyped comptime param)
    let mut colon_idx = None;
    let mut depth = 0usize;
    for (i, t) in seg.iter().enumerate() {
        if t.kind == TokKind::Punct {
            match t.text.as_str() {
                "<" | "(" | "[" => depth += 1,
                ">" | ")" | "]" => depth = depth.saturating_sub(1),
                ":" if depth == 0 => {
                    colon_idx = Some(i);
                    break;
                }
                _ => {}
            }
        }
    }
    let slice = match colon_idx {
        Some(ci) => &seg[..ci],
        None => seg,
    };
    slice
        .iter()
        .rev()
        .find(|t| {
            t.kind == TokKind::Ident
                && !matches!(t.text.as_str(), "var" | "let" | "ref" | "iso" | "const")
        })
        .map(|t| t.text.clone())
}

fn is_trivial_constant_stub(body: &[PilotTok]) -> bool {
    if body.is_empty() {
        return true;
    }
    let stripped: Vec<&PilotTok> = if body.first().map(|t| t.text.as_str()) == Some("return") {
        body[1..]
            .iter()
            .take_while(|t| !(t.kind == TokKind::Punct && t.text == ";"))
            .collect()
    } else {
        body.iter()
            .take_while(|t| !(t.kind == TokKind::Punct && t.text == ";"))
            .collect()
    };
    match stripped.as_slice() {
        [] => true,
        [only] => matches!(
            only.kind,
            TokKind::Number | TokKind::StrLit | TokKind::CharLit
        ) || (only.kind == TokKind::Ident && matches!(only.text.as_str(), "true" | "false" | "none")),
        [sign, num] if sign.kind == TokKind::Punct && sign.text == "-" && num.kind == TokKind::Number => true,
        _ => false,
    }
}

fn audit_single_pilot_file(root: &Path, test_name: &str, milestone_id: u32, rule: &FileAuditRule) {
    let full = root.join(rule.path);
    let raw = std::fs::read_to_string(&full).unwrap_or_else(|e| {
        panic!(
            "ANTI-CHEAT [PASS 5: MISSING_PILOT_DELIVERABLE] [M{}::{}]: Required file '{}' missing: {}",
            milestone_id, test_name, rule.path, e
        )
    });

    let line_cnt = raw.lines().count();
    assert!(
        (1..=rule.max_lines).contains(&line_cnt),
        "ANTI-CHEAT [PASS 5: LINE_CAP_VIOLATION] [M{}::{}]: '{}' has {} lines (allowed 1..={})!",
        milestone_id, test_name, rule.path, line_cnt, rule.max_lines
    );

    // Pass 6a: Stub/TODO/Placeholder marker ban
    for marker in BANNED_STUB_MARKERS {
        assert!(
            !raw.contains(marker),
            "ANTI-CHEAT [PASS 6: STUB_OR_TODO_MARKER] [M{}::{}]: Banned placeholder marker '{}' found in '{}'!",
            milestone_id, test_name, marker, rule.path
        );
    }

    let scanned = scan_pilot_source(rule.path, &raw, milestone_id, test_name);
    let toks = &scanned.tokens;
    assert!(
        toks.len() >= 12,
        "ANTI-CHEAT [PASS 6: HOLLOW_SHELL_FILE] [M{}::{}]: '{}' contains only {} non-comment tokens (cannot leave module as an empty/comment-only shell)!",
        milestone_id,
        test_name,
        rule.path,
        toks.len()
    );

    // Pass 5b: Legacy prohibited Pilot keywords & `nameof("...")` string-literal bypass check
    for w in 0..toks.len() {
        let tk = &toks[w];
        if tk.kind == TokKind::Ident {
            match tk.text.as_str() {
                "def" if w + 1 < toks.len() && toks[w + 1].kind == TokKind::Ident => {
                    panic!(
                        "ANTI-CHEAT [PASS 5: LEGACY_DEF_KEYWORD] [M{}::{}]: Prohibited 'def' function syntax at {}:{}!",
                        milestone_id, test_name, rule.path, tk.line
                    );
                }
                "mut"
                    if w > 0
                        && toks[w - 1].kind == TokKind::Ident
                        && matches!(toks[w - 1].text.as_str(), "let" | "var" | "ref") =>
                {
                    panic!(
                        "ANTI-CHEAT [PASS 5: LEGACY_MUT_KEYWORD] [M{}::{}]: Prohibited 'mut' modifier at {}:{} (use 'var')!",
                        milestone_id, test_name, rule.path, tk.line
                    );
                }
                "lambda" | "elif" => {
                    panic!(
                        "ANTI-CHEAT [PASS 5: LEGACY_KEYWORD] [M{}::{}]: Prohibited '{}' keyword at {}:{}!",
                        milestone_id, test_name, tk.text, rule.path, tk.line
                    );
                }
                "nameof"
                    if w + 2 < toks.len()
                        && toks[w + 1].kind == TokKind::Punct
                        && toks[w + 1].text == "("
                        && toks[w + 2].kind == TokKind::StrLit =>
                {
                    panic!(
                        "ANTI-CHEAT [PASS 5: NAMEOF_STRING_CHEAT] [M{}::{}]: `nameof(\"{}\")` called with a string literal instead of an unquoted identifier at {}:{}!",
                        milestone_id, test_name, toks[w + 2].text, rule.path, tk.line
                    );
                }
                _ => {}
            }
        }
    }

    // Pass 6b: Required `pub fn` existence, non-extern body, non-trivial logic & parameter liveness
    for &req_fn in rule.required_pub_fns {
        let defs = scanned.functions.get(req_fn).unwrap_or_else(|| {
            panic!(
                "ANTI-CHEAT [PASS 6: MISSING_REQUIRED_PUB_FN] [M{}::{}]: Required function '{}' is not defined in '{}'!",
                milestone_id, test_name, req_fn, rule.path
            )
        });
        let real_def = defs
            .iter()
            .find(|d| d.is_pub && !d.is_extern && !d.body_tokens.is_empty())
            .unwrap_or_else(|| {
                panic!(
                    "ANTI-CHEAT [PASS 6: EXTERN_OR_PRIVATE_STUB] [M{}::{}]: '{}' in '{}' must be a `pub fn` with a concrete Pilot body (never `extern fn` or empty)!",
                    milestone_id, test_name, req_fn, rule.path
                )
            });

        verify_fn_body_and_params(milestone_id, test_name, rule.path, req_fn, real_def);
    }

    // Pass 7: Global hardcoded test-vector, cryptographic digest, fixture & delimiter leak scan
    for leak in GLOBAL_BANNED_LEAK_SUBSTRINGS {
        assert!(
            !scanned.code_only_text.contains(leak),
            "ANTI-CHEAT [PASS 7: HARDCODED_TEST_VECTOR_OR_FIXTURE] [M{}::{}]: '{}' hardcodes banned test vector / fixture / delimiter '{}'!",
            milestone_id, test_name, rule.path, leak
        );
    }

    // Pass 8: Milestone-specific shortcut / external-oracle / bypass prohibitions
    for &forbidden_code in rule.forbidden_code_markers {
        assert!(
            !scanned.code_only_text.contains(forbidden_code),
            "ANTI-CHEAT [PASS 8: FORBIDDEN_SHORTCUT_OR_BYPASS] [M{}::{}]: '{}' contains banned shortcut/bypass '{}'!",
            milestone_id, test_name, rule.path, forbidden_code
        );
    }
    for &forbidden_str in rule.forbidden_string_substrings {
        for &(ln, ref lit) in &scanned.string_literals {
            assert!(
                !lit.contains(forbidden_str),
                "ANTI-CHEAT [PASS 8: FORBIDDEN_STRING_LITERAL] [M{}::{}]: String literal \"{}\" at {}:{} contains banned substring '{}'!",
                milestone_id, test_name, lit, rule.path, ln, forbidden_str
            );
        }
    }

    // Pass 9: Expected architectural mechanism proof
    for &req_marker in rule.required_code_markers {
        assert!(
            scanned.code_only_text.contains(req_marker),
            "ANTI-CHEAT [PASS 9: MISSING_ARCHITECTURAL_MECHANISM] [M{}::{}]: '{}' does not use required mechanism '{}'!",
            milestone_id, test_name, rule.path, req_marker
        );
    }

    // Pass 10: Universal Name-Blind Contract on pure compiler / IDE / Perceus / VM / REPL modules
    if rule.name_blind {
        let banned_exact: HashSet<&str> = BANNED_NAME_BLIND_TYPE_STRINGS.iter().copied().collect();
        for &(ln, ref lit) in &scanned.string_literals {
            assert!(
                !banned_exact.contains(lit.as_str()),
                "ANTI-CHEAT [PASS 10: NAME_BLIND_VIOLATION] [M{}::{}]: Name-Blind module '{}' hardcodes Pilot type name \"{}\" at line {}!",
                milestone_id, test_name, rule.path, lit, ln
            );
        }
    }
}

fn verify_fn_body_and_params(
    milestone_id: u32,
    test_name: &str,
    rel_path: &str,
    req_fn: &str,
    real_def: &ExtractedFn,
) {
    assert!(
        real_def.body_tokens.len() >= 3 && !is_trivial_constant_stub(&real_def.body_tokens),
        "ANTI-CHEAT [PASS 6: TRIVIAL_CONSTANT_RETURN_STUB] [M{}::{}]: `pub fn {}` in '{}' returns a trivial constant/stub instead of computing!",
        milestone_id, test_name, req_fn, rel_path
    );

    for param in &real_def.params {
        if param == "self" || param.starts_with('_') {
            continue;
        }
        let used_in_body = real_def.body_tokens.iter().any(|bt| {
            (bt.kind == TokKind::Ident && &bt.text == param)
                || (bt.kind == TokKind::FStrLit && bt.text.contains(param))
        });
        assert!(
            used_in_body,
            "ANTI-CHEAT [PASS 6: UNUSED_PARAMETER_FAKE_IMPL] [M{}::{}]: Parameter '{}' of `pub fn {}` in '{}' is completely ignored by the function body!",
            milestone_id, test_name, param, req_fn, rel_path
        );
    }
}

fn verify_group_pub_fn(
    root: &Path,
    test_name: &str,
    milestone_id: u32,
    req_fn: &str,
    candidate_paths: &[&str],
) {
    for &rel_path in candidate_paths {
        if let Ok(raw) = std::fs::read_to_string(root.join(rel_path)) {
            let scanned = scan_pilot_source(rel_path, &raw, milestone_id, test_name);
            if let Some(defs) = scanned.functions.get(req_fn) {
                if let Some(real_def) = defs
                    .iter()
                    .find(|d| d.is_pub && !d.is_extern && !d.body_tokens.is_empty())
                {
                    verify_fn_body_and_params(milestone_id, test_name, rel_path, req_fn, real_def);
                    return;
                }
            }
        }
    }
    panic!(
        "ANTI-CHEAT [PASS 6: MISSING_GROUP_PUB_FN] [M{}::{}]: Required `pub fn {}` with concrete body was not found in any module of {:?}!",
        milestone_id, test_name, req_fn, candidate_paths
    );
}
