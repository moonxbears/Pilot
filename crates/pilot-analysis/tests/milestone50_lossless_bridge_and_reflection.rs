// ==============================================================================
// crates/pilot-analysis/tests/milestone50_lossless_bridge_and_reflection.rs
// Milestone 50 — Uncheatable Adversarial Verification Suite
//
// Required `.pilot` Contracts (all tested simultaneously on randomized inputs):
//   3. `selfhost/attr_weaver.pilot`:
//      - `pub fn highest_weight_attr_index(nid: i64) -> i64`
//      - `pub fn weave_sorted_attr_names(nid: i64) -> str`
//        Returns comma-separated attribute names attached to `nid` sorted in strictly
//        DESCENDING order of `weight: i32` (last arg, default `10` if 0 args), preserving
//        original attachment index order on equal weights.
//      - `pub fn weave_unit_attributes() -> i64`
//        Returns total Pillar-7 (`[Rule]` / `IEcsRule`) systems in unit and emits `ATTR001`
//        (reactive/formula attr on `interface`) and `ATTR002` (interceptor attr on field/param).
// ==============================================================================

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_analysis::{flatten_ast, handle_engine_intrinsic, PhysicsRegistry};
use pilot_parser::parse;
use pilot_vm::PilotVal;

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 50,
    test_file_rel: "crates/pilot-analysis/tests/milestone50_lossless_bridge_and_reflection.rs",
    expected_test_count: 1,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/diagnostics.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &["verify_unit_diagnostics"],
            required_code_markers: &["__engine", "EnforceCoreNaming", "LatticeContractMismatch"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "selfhost/attr_weaver.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[
                "highest_weight_attr_index",
                "weave_sorted_attr_names",
                "weave_unit_attributes",
            ],
            required_code_markers: &["__engine", "ATTR001", "ATTR002"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        let seed = nanos ^ ((std::process::id() as u64).rotate_left(17)) ^ salt;
        Self(if seed == 0 {
            0xCAFE_BABE_DEAD_BEEF
        } else {
            seed
        })
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn range(&mut self, lo: i64, hi_inclusive: i64) -> i64 {
        let span = (hi_inclusive - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }

    fn alpha_tag(&mut self, len: usize) -> String {
        const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
        (0..len)
            .map(|_| LOWER[(self.next_u64() as usize) % LOWER.len()] as char)
            .collect()
    }
}

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

#[test]
pub fn test_50_2_open_keyword_reflection_and_randomized_user_categories() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_50_2_open_keyword_reflection_and_randomized_user_categories",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5002_BEEF);
    let empty_arena = parse("");
    let (flat, syms) = flatten_ast(&empty_arena, empty_arena.root);
    let mut reg = PhysicsRegistry::new(flat, syms);

    let kw_path = workspace_root()
        .join("stdlib/core/keywords.pilot")
        .to_string_lossy()
        .to_string();
    let mounted = handle_engine_intrinsic(&mut reg, "mount_unit", &[PilotVal::from_str(&kw_path)])
        .unwrap()
        .as_int()
        .unwrap();
    assert!(mounted > 50);

    // Verify core categories 0..=14 are dynamically reflected
    for cat_id in [0i64, 1, 2, 5, 8] {
        let scope = handle_engine_intrinsic(
            &mut reg,
            "query_registered_symbols",
            &[PilotVal::from_int(cat_id), PilotVal::from_int(1)],
        )
        .unwrap()
        .to_string_repr();
        let badge = handle_engine_intrinsic(
            &mut reg,
            "query_registered_symbols",
            &[PilotVal::from_int(cat_id), PilotVal::from_int(2)],
        )
        .unwrap()
        .to_string_repr();
        assert!(
            scope.ends_with(".pilot") && !badge.is_empty(),
            "Category {} must reflect valid scope/badge, got {:?}/{:?}",
            cat_id,
            scope,
            badge
        );
    }

    // Register 6 random user categories >= 100 and verify strict isolation
    for idx in 0..6 {
        let cid = 100 + idx * 37 + rng.range(1, 20);
        let scope = format!("keyword.dsl.{}.pilot", rng.alpha_tag(6));
        let badge = format!("DSL-{}", rng.alpha_tag(5));
        handle_engine_intrinsic(
            &mut reg,
            "register_keyword_category",
            &[
                PilotVal::from_int(cid),
                PilotVal::from_str(&scope),
                PilotVal::from_str(&badge),
                PilotVal::from_int(14),
            ],
        )
        .unwrap();

        let word_cnt = rng.range(3, 6) as usize;
        let mut expected_words = Vec::new();
        for w_i in 0..word_cnt {
            let kw = format!("kw_{}_{}_{}", idx, w_i, rng.alpha_tag(5));
            let doc = format!("Doc {} {}", cid, rng.alpha_tag(10));
            handle_engine_intrinsic(
                &mut reg,
                "register_keyword",
                &[
                    PilotVal::from_int(cid),
                    PilotVal::from_str(&kw),
                    PilotVal::from_str(&doc),
                ],
            )
            .unwrap();
            expected_words.push((kw, doc));
        }

        let got_scope = handle_engine_intrinsic(
            &mut reg,
            "query_registered_symbols",
            &[PilotVal::from_int(cid), PilotVal::from_int(1)],
        )
        .unwrap()
        .to_string_repr();
        let got_badge = handle_engine_intrinsic(
            &mut reg,
            "query_registered_symbols",
            &[PilotVal::from_int(cid), PilotVal::from_int(2)],
        )
        .unwrap()
        .to_string_repr();
        assert_eq!(got_scope, scope);
        assert_eq!(got_badge, badge);

        let words_joined = handle_engine_intrinsic(
            &mut reg,
            "query_registered_symbols",
            &[PilotVal::from_int(cid)],
        )
        .unwrap()
        .to_string_repr();
        let got_list: Vec<&str> = words_joined.split('|').collect();
        assert_eq!(got_list.len(), expected_words.len());
        for (kw, doc) in &expected_words {
            assert!(got_list.contains(&kw.as_str()));
            let got_doc = handle_engine_intrinsic(
                &mut reg,
                "query_registered_symbols",
                &[
                    PilotVal::from_int(cid),
                    PilotVal::from_int(3),
                    PilotVal::from_str(kw),
                ],
            )
            .unwrap()
            .to_string_repr();
            assert_eq!(&got_doc, doc);
        }
    }
    guard.finish();
}
