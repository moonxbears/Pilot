//! ==============================================================================
//! crates/pilot-analysis/tests/milestone54_perceus_ownership_and_escape.rs
//! Milestone 54 — Uncheatable Lattice-Driven Perceus ARC, Escape & Realtime Gate
//!
//! Required `.pilot` Contracts (all tested simultaneously on randomized ASTs):
//!   All passes in `selfhost/perceus/{lattice,escape,realtime,optimizer}.pilot` MUST walk
//!   real parsed Pilot programs in `PhysicsRegistry` (`RawAstArena` + `LayoutTable` +
//!   `source_slice` spans) with ZERO caller-passed `has_alloc` or `lattice_bits` hints
//!   and ZERO function/type name sniffing (e.g., `"dsp"` is strictly prohibited!).
//!
//!   1. `selfhost/perceus/lattice.pilot`: every struct's 2-bit lattice (`0b00..0b11`)
//!      comes from `[Lattice(mask)]` attributes on its interfaces (plus `IValue`=0,
//!      `IReference`=1, `IObject`=2, `IGuard`=3), its `~self()` destructor (`| 2`) and
//!      the bitwise-OR of its stored field types' lattice bits.
//!      Test 54.1 checks these lattices on randomized programs. (It used to check the
//!      string-emitting ARC schedule `perceus_schedule_fn` of the former
//!      `selfhost/perceus/arc.pilot`, deleted by F7, which replaced it with the checker's
//!      drop plan and PilIR reference counts: see `crates/pilot-cli/tests/cli_f7.rs`.)
//!   2. `selfhost/perceus/escape.pilot`, `selfhost/perceus/realtime.pilot` & `optimizer.pilot`:
//!      - `pub fn verify_perceus_unit() -> i64`
//!        Walks all `FnDecl`s in `RawAstArena`, emits diagnostics via
//!        `__engine("ast_emit_diag", nid, 1, msg)` containing the exact diagnostic code,
//!        and returns total diagnostics emitted:
//!        * `ESCAPE001`: Passing a `0b01` or `0b11` variable as an argument to a `[Cue]` function.
//!        * `ESCAPE002`: Passing/storing a `0b01` or `0b11` variable into a `0b10` (`IObject`)
//!          struct constructor / initializer or field assignment.
//!        * `ESCAPE003`: `return`ing a local `0b01` or `0b11` variable that was NOT derived
//!          from a `ref` parameter of the enclosing function.
//!        * `PILOT105`: Passing a non-`iso` `0b10` variable to a `[Cue]` function (whereas
//!          passing an `iso`-qualified `0b10` parameter or any `0b00` value is legal!).
//!        * `PILOT104`: Inside a `[HardRealtime]` or `noalloc` function, constructing any
//!          `0b10` heap object (`[...]` list literal or `0b10` struct construction) or
//!          calling `spawn(...)`. Functions named `dsp_*` WITHOUT `[HardRealtime]` or
//!          `noalloc` MUST NOT emit `PILOT104`!
//! ==============================================================================

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_analysis::{
    eval_file_with_tables, flatten_ast, handle_engine_intrinsic, run_pilot_analyzer,
    BootstrapSnapshot, PhysicsRegistry,
};
use pilot_parser::parse;
use pilot_rt::PilotVal;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const PERCEUS_GROUP: &[&str] = &[
    "selfhost/perceus/lattice.pilot",
    "selfhost/perceus/escape.pilot",
    "selfhost/perceus/realtime.pilot",
    "selfhost/perceus/optimizer.pilot",
];

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 54,
    test_file_rel: "crates/pilot-analysis/tests/milestone54_perceus_ownership_and_escape.rs",
    expected_test_count: 2,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/perceus/lattice.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &["@hard_realtime"],
            forbidden_string_substrings: &["dsp", "@hard_realtime"],
        },
        anticheat::FileAuditRule {
            path: "selfhost/perceus/escape.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &["@hard_realtime"],
            forbidden_string_substrings: &["dsp", "@hard_realtime"],
        },
        anticheat::FileAuditRule {
            path: "selfhost/perceus/realtime.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &["@hard_realtime"],
            forbidden_string_substrings: &["dsp", "@hard_realtime"],
        },
        anticheat::FileAuditRule {
            path: "selfhost/perceus/optimizer.pilot",
            max_lines: 1500,
            name_blind: true,
            required_pub_fns: &[],
            required_code_markers: &[],
            forbidden_code_markers: &["@hard_realtime"],
            forbidden_string_substrings: &["dsp", "@hard_realtime"],
        },
    ],
    required_group_pub_fns: &[("verify_perceus_unit", PERCEUS_GROUP)],
};

struct Rng(u64);

impl Rng {
    fn new(salt: u64) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5454_5454_CAFE_F00D);
        let s = nanos ^ ((std::process::id() as u64).rotate_left(17)) ^ salt;
        Self(if s == 0 { 0xABCD_1234_EF56_7890 } else { s })
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn lower_ident(&mut self, prefix: &str) -> String {
        let chars = b"abcdefghijklmnopqrstuvwxyz";
        let mut out = format!("{}_", prefix);
        for _ in 0..5 {
            out.push(chars[(self.next_u64() as usize) % chars.len()] as char);
        }
        out
    }

    fn pascal_ident(&mut self, prefix: &str) -> String {
        let chars = b"abcdefghijklmnopqrstuvwxyz";
        let mut out = prefix.to_string();
        for _ in 0..5 {
            out.push(chars[(self.next_u64() as usize) % chars.len()] as char);
        }
        out
    }
}

fn ws_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read_ws(rel: &str) -> String {
    let p = ws_root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("Failed to read {}: {}", p.display(), e))
}

fn escape_pilot_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Parses `target_program`, runs `selfhost/analyzer.pilot` on `PhysicsRegistry` so every
/// struct/field has its real `LayoutTable` `byte_size` bound, then executes the Perceus
/// pass modules against that `PhysicsRegistry`!
fn run_perceus_on_program(target_program: &str, tail_expr: &str) -> (PilotVal, Vec<String>) {
    let target_arena = parse(target_program);
    assert!(
        target_arena.diagnostics.is_empty(),
        "Target program had parse errors: {:?}\nProgram:\n{}",
        target_arena.diagnostics,
        target_program
    );
    let (flat, syms) = flatten_ast(&target_arena, target_arena.root);
    let mut registry = PhysicsRegistry::new(flat, syms);
    registry.source_text = target_program.to_string();

    // Populate LayoutTable & node_layouts via selfhost/analyzer.pilot
    run_pilot_analyzer(&mut registry, None).expect("analyzer.pilot must run cleanly");
    registry.diagnostics.clear();

    let lat_src = read_ws("selfhost/perceus/lattice.pilot");
    let esc_src = read_ws("selfhost/perceus/escape.pilot");
    let rt_src = read_ws("selfhost/perceus/realtime.pilot");
    let opt_src = read_ws("selfhost/perceus/optimizer.pilot");

    let pass_script = format!(
        "{}\n{}\n{}\n{}\nlet __out = {};\n__out;\n",
        lat_src, esc_src, rt_src, opt_src, tail_expr
    );
    let pass_arena = parse(&pass_script);
    assert!(
        pass_arena.diagnostics.is_empty(),
        "Perceus pass modules failed to parse: {:?}",
        pass_arena.diagnostics
    );

    let reg_ptr = &mut registry as *mut PhysicsRegistry;
    let mut vm = PilVirtualMachine::new();
    vm.engine_handler = Some(Box::new(move |op, args| {
        handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, args)
    }));
    let binary = BytecodeCompiler::compile(&pass_arena, pass_arena.root);
    assert!(
        binary.diagnostics.is_empty(),
        "perceus pass failed to compile: {:?}",
        binary.diagnostics
    );
    vm.load_binary(binary);
    let val = vm.run();
    if let Some(fault) = &vm.fault {
        panic!("perceus pass faulted: {fault}");
    }
    let diags = registry.diagnostics.into_iter().map(|d| d.message).collect();
    (val, diags)
}

#[test]
pub fn test_54_1_real_ast_lattice_from_attributes_destructors_and_fields() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_54_1_real_ast_lattice_from_attributes_destructors_and_fields",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5401);
    let snap = BootstrapSnapshot::global();
    for rel in PERCEUS_GROUP {
        let src = read_ws(rel);
        let mut tables = snap.tables.clone();
        let mut vm = snap.vm.clone();
        eval_file_with_tables(&src, &mut tables, &mut vm)
            .unwrap_or_else(|e| panic!("{} failed bootstrap evaluation: {:?}", rel, e));
    }

    for trial in 0..6 {
        // Randomized names: the lattice comes from the declarations only.
        let sc64 = rng.lower_ident("u64x");
        let if_val = rng.pascal_ident("ICustVal");
        let if_ref = rng.pascal_ident("ICustRef");
        let if_guard = rng.pascal_ident("ICustGuard");
        let val_st = rng.pascal_ident("ValPair");
        let view_st = rng.pascal_ident("WinView");
        let obj_st = rng.pascal_ident("Buf16A");
        let lock_st = rng.pascal_ident("ScopeLock");
        let held_st = rng.pascal_ident("Holder");
        let lens_st = rng.pascal_ident("LensBox");
        let attr_guard_st = rng.pascal_ident("Lease");

        let program = format!(
            "[Bits(64, 2)]\npub struct {sc64} {{}}\n\
             [Lattice(0)]\npub interface {if_val} {{}}\n\
             [Lattice(1)]\npub interface {if_ref} {{}}\n\
             [Lattice(3)]\npub interface {if_guard} {{}}\n\
             pub struct {val_st} : {if_val} {{ pub a: {sc64}; pub b: {sc64}; }}\n\
             pub struct {view_st} : {if_ref} {{ pub p: {sc64}; pub len: {sc64}; }}\n\
             pub struct {obj_st} {{ pub x: {sc64}; ~self() {{}} }}\n\
             pub struct {lock_st} : {if_ref} {{ pub handle: {sc64}; ~self() {{}} }}\n\
             pub struct {held_st} : {if_val} {{ pub inner: {obj_st}; }}\n\
             pub struct {lens_st} : {if_val} {{ pub w: {view_st}; }}\n\
             pub struct {attr_guard_st} : {if_guard} {{ pub h: {sc64}; }}\n",
        );

        // Each struct's lattice as one base-4 digit, first struct lowest.
        let mut terms = Vec::new();
        for (i, name) in [&val_st, &view_st, &obj_st, &lock_st, &held_st, &lens_st, &attr_guard_st].iter().enumerate() {
            terms.push(format!(
                "{} * lat_deduce_type_lattice(__engine(\"sym_lookup\", {}), __engine(\"ast_node_count\", 0), __engine(\"map_new\"), 0)",
                4i64.pow(i as u32),
                escape_pilot_str(name)
            ));
        }
        let (res, _) = run_perceus_on_program(&program, &terms.join(" + "));

        // By the lattice rules, worked by hand: a value 0b00; a view 0b01
        // (its interface's [Lattice(1)]); ~self() alone 0b10; a view with
        // ~self() 0b01 | 0b10 = 0b11; a value holding an owned field 0b10;
        // a value holding a view 0b01; [Lattice(3)] on the interface 0b11.
        let expected = [0i64, 1, 2, 3, 2, 1, 3];
        let want: i64 = expected.iter().enumerate().map(|(i, l)| l * 4i64.pow(i as u32)).sum();
        assert_eq!(
            res.to_string_repr(),
            want.to_string(),
            "Trial {}: lattices (base 4, first struct lowest) of {:?}",
            trial,
            [&val_st, &view_st, &obj_st, &lock_st, &held_st, &lens_st, &attr_guard_st]
        );
    }
    guard.finish();
}

#[test]
pub fn test_54_2_and_54_3_real_ast_escape_realtime_cue_verifiers_and_dsp_decoy_trap() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_54_2_and_54_3_real_ast_escape_realtime_cue_verifiers_and_dsp_decoy_trap",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5402);

    // Static check: forbid `"dsp"` name sniffing or `@hard_realtime` in selfhost/perceus/*.pilot
    for rel in [
        "selfhost/perceus/lattice.pilot",
        "selfhost/perceus/escape.pilot",
        "selfhost/perceus/realtime.pilot",
        "selfhost/perceus/optimizer.pilot",
    ] {
        let content = read_ws(rel);
        for (ln, line) in content.lines().enumerate() {
            let code_part = line.split("//").next().unwrap_or("");
            assert!(
                !code_part.contains("\"dsp\"") && !code_part.contains("@hard_realtime"),
                "{}:{} contains prohibited legacy name-sniffing literal: {}",
                rel,
                ln + 1,
                line
            );
        }
    }

    for trial in 0..6 {
        let sc = rng.lower_ident("word64");
        let val_t = rng.pascal_ident("PlainVal");
        let view_t = rng.pascal_ident("BorrowWin");
        let obj_t = rng.pascal_ident("HeapBox");
        let guard_t = rng.pascal_ident("ArenaLease");
        let cue_fn = rng.lower_ident("dispatch_cue");

        // 1. Clean legal program — INCLUDING a decoy `dsp_*` function that allocates on the heap
        //    (without `[HardRealtime]` or `noalloc`), `ref`-tied view return, `iso` 0b10 cue transfer,
        //    and stack-only `[HardRealtime]` + `noalloc` functions -> MUST EMIT ZERO DIAGNOSTICS!
        let dsp_decoy = format!("dsp_audio_synth_{}", rng.lower_ident("ch"));
        let clean_prog = format!(
            "[Bits(64, 2)]\npub struct {sc} {{}}\n\
             [Lattice(0)]\npub interface IB0 {{}}\n\
             [Lattice(1)]\npub interface IB1 {{}}\n\
             [Lattice(2)]\npub interface IB2 {{}}\n\
             [Lattice(3)]\npub interface IB3 {{}}\n\
             pub struct {val_t} : IB0 {{ pub x: {sc}; }}\n\
             pub struct {view_t} : IB1 {{ pub ptr_v: {sc}; }}\n\
             pub struct {obj_t} : IB2 {{ pub data: {sc}; ~self() {{}} }}\n\
             pub struct {guard_t} : IB3 {{ pub lock: {sc}; ~self() {{}} }}\n\
             [Cue]\npub fn {cue_fn}(v: {val_t}, iso owned_pkt: {obj_t}) -> {sc} {{ return v.x; }}\n\
             pub fn {dsp_decoy}(seed: {sc}) -> {obj_t} {{\n\
                 let arr = [1, 2, 3];\n\
                 let b: {obj_t} = {obj_t} {{ data: seed }};\n\
                 return b;\n\
             }}\n\
             pub fn borrow_passthrough(ref inp: {view_t}) -> {view_t} {{\n\
                 let sub: {view_t} = inp;\n\
                 return sub;\n\
             }}\n\
             pub fn cue_ok(v: {val_t}, iso pkt: {obj_t}) -> {sc} {{\n\
                 return {cue_fn}(v, pkt);\n\
             }}\n\
             [HardRealtime]\npub fn rt_clean(a: {val_t}, ref w: {view_t}) -> {val_t} {{\n\
                 let local_v: {val_t} = a;\n\
                 return local_v;\n\
             }}\n\
             pub noalloc fn step_clean(a: {val_t}) -> {val_t} {{\n\
                 return a;\n\
             }}\n"
        );
        let (clean_cnt, clean_diags) = run_perceus_on_program(&clean_prog, "verify_perceus_unit()");
        assert_eq!(
            clean_cnt.as_int(),
            Some(0),
            "Trial {}: Clean program (with `{}` decoy) must emit 0 Perceus diagnostics, got {:?}",
            trial,
            dsp_decoy,
            clean_diags
        );
        assert!(clean_diags.is_empty());

        // 2. ESCAPE001: Passing 0b01 view to `[Cue]` function
        let esc1_prog = format!(
            "[Bits(64, 2)]\npub struct {sc} {{}}\n\
             [Lattice(1)]\npub interface IB1 {{}}\n\
             pub struct {view_t} : IB1 {{ pub ptr_v: {sc}; }}\n\
             [Cue]\npub fn {cue_fn}(w: {view_t}) -> {sc} {{ return w.ptr_v; }}\n\
             pub fn bad_cue_escape(ref w: {view_t}) -> {sc} {{\n\
                 return {cue_fn}(w);\n\
             }}\n"
        );
        let (_, esc1_diags) = run_perceus_on_program(&esc1_prog, "verify_perceus_unit()");
        assert!(
            esc1_diags.iter().any(|d| d.contains("ESCAPE001")),
            "Trial {}: Expected ESCAPE001 when passing 0b01 view across [Cue], got {:?}",
            trial,
            esc1_diags
        );

        // 3. ESCAPE002: Storing 0b01 view inside 0b10 heap object constructor
        let container_t = rng.pascal_ident("HeapHolder");
        let esc2_prog = format!(
            "[Bits(64, 2)]\npub struct {sc} {{}}\n\
             [Lattice(1)]\npub interface IB1 {{}}\n\
             pub struct {view_t} : IB1 {{ pub ptr_v: {sc}; }}\n\
             pub struct {container_t} {{ pub slot: {sc}; ~self() {{}} }}\n\
             pub fn bad_heap_store(ref w: {view_t}) -> {container_t} {{\n\
                 let c: {container_t} = {container_t} {{ slot: w }};\n\
                 return c;\n\
             }}\n"
        );
        let (_, esc2_diags) = run_perceus_on_program(&esc2_prog, "verify_perceus_unit()");
        assert!(
            esc2_diags.iter().any(|d| d.contains("ESCAPE002")),
            "Trial {}: Expected ESCAPE002 when storing 0b01 view into 0b10 object, got {:?}",
            trial,
            esc2_diags
        );

        // 4. ESCAPE003: Returning local 0b01 view NOT tied to a `ref` parameter
        let esc3_prog = format!(
            "[Bits(64, 2)]\npub struct {sc} {{}}\n\
             [Lattice(1)]\npub interface IB1 {{}}\n\
             pub struct {view_t} : IB1 {{ pub ptr_v: {sc}; }}\n\
             pub fn bad_return_escape(addr: {sc}) -> {view_t} {{\n\
                 let local_win: {view_t} = {view_t} {{ ptr_v: addr }};\n\
                 return local_win;\n\
             }}\n"
        );
        let (_, esc3_diags) = run_perceus_on_program(&esc3_prog, "verify_perceus_unit()");
        assert!(
            esc3_diags.iter().any(|d| d.contains("ESCAPE003")),
            "Trial {}: Expected ESCAPE003 when returning untied local 0b01 view, got {:?}",
            trial,
            esc3_diags
        );

        // 5. PILOT105: Passing non-`iso` 0b10 object to `[Cue]` function
        let p105_prog = format!(
            "[Bits(64, 2)]\npub struct {sc} {{}}\n\
             pub struct {obj_t} {{ pub data: {sc}; ~self() {{}} }}\n\
             [Cue]\npub fn {cue_fn}(pkt: {obj_t}) -> {sc} {{ return pkt.data; }}\n\
             pub fn bad_unisolated_cue(pkt: {obj_t}) -> {sc} {{\n\
                 return {cue_fn}(pkt);\n\
             }}\n"
        );
        let (_, p105_diags) = run_perceus_on_program(&p105_prog, "verify_perceus_unit()");
        assert!(
            p105_diags.iter().any(|d| d.contains("PILOT105")),
            "Trial {}: Expected PILOT105 when passing non-`iso` 0b10 object across [Cue], got {:?}",
            trial,
            p105_diags
        );

        // 6. PILOT104: Allocating 0b10 object or list literal inside `[HardRealtime]` and `noalloc`
        let p104_rt = format!(
            "[Bits(64, 2)]\npub struct {sc} {{}}\n\
             pub struct {obj_t} {{ pub data: {sc}; ~self() {{}} }}\n\
             [HardRealtime]\npub fn bad_rt_alloc(x: {sc}) -> {obj_t} {{\n\
                 let heap_obj: {obj_t} = {obj_t} {{ data: x }};\n\
                 return heap_obj;\n\
             }}\n"
        );
        let (_, p104_rt_diags) = run_perceus_on_program(&p104_rt, "verify_perceus_unit()");
        assert!(
            p104_rt_diags.iter().any(|d| d.contains("PILOT104")),
            "Trial {}: Expected PILOT104 on 0b10 allocation in [HardRealtime], got {:?}",
            trial,
            p104_rt_diags
        );

        let p104_noalloc = format!(
            "[Bits(64, 2)]\npub struct {sc} {{}}\n\
             pub noalloc fn bad_noalloc_list(x: {sc}) -> {sc} {{\n\
                 let xs = [10, 20, 30];\n\
                 return x;\n\
             }}\n"
        );
        let (_, p104_na_diags) = run_perceus_on_program(&p104_noalloc, "verify_perceus_unit()");
        assert!(
            p104_na_diags.iter().any(|d| d.contains("PILOT104")),
            "Trial {}: Expected PILOT104 on list allocation in `noalloc` fn, got {:?}",
            trial,
            p104_na_diags
        );
    }

    // Deliverable 54.3: Verify crates/pilot-perceus is completely deleted from disk & Cargo.toml
    let root = ws_root();
    assert!(!root.join("crates/pilot-perceus").exists(), "crates/pilot-perceus must be deleted");
    assert!(!read_ws("Cargo.toml").contains("pilot-perceus"));
    guard.finish();
}
