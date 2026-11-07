//! ==============================================================================
//! crates/pilot-analysis/tests/milestone54_perceus_ownership_and_escape.rs
//! Milestone 54 — Uncheatable Lattice-Driven Perceus ARC, Escape & Realtime Gate
//!
//! Required `.pilot` Contracts (all tested simultaneously on randomized ASTs):
//!   All passes in `selfhost/perceus/{arc,escape,realtime,optimizer}.pilot` MUST walk
//!   real parsed Pilot programs in `PhysicsRegistry` (`RawAstArena` + `LayoutTable` +
//!   `source_slice` spans) with ZERO caller-passed `has_alloc` or `lattice_bits` hints
//!   and ZERO function/type name sniffing (e.g., `"dsp"` is strictly prohibited!).
//!
//!   1. `selfhost/perceus/arc.pilot` & `selfhost/perceus/optimizer.pilot`:
//!      - Deduce every struct's 2-bit lattice (`0b00..0b11`) strictly from:
//!        * `[Lattice(mask)]` attributes on implemented `interface`s in the unit
//!          (plus `IValue`=0, `IReference`=1, `IObject`=2, `IGuard`=3),
//!        * `~self()` destructor flag (`(ast_decl_flags(struct_nid) & 1024) != 0` => `| 2`),
//!        * bitwise-OR of all stored field types' lattice bits (`kind_tag == 21`).
//!      - `pub fn perceus_schedule_fn(fn_name: str) -> str`
//!        Walks the `FnDecl` named `fn_name` in `RawAstArena` and returns `;`-separated
//!        ARC / lifecycle instructions in execution order:
//!        * `0b00` (`IValue`) & `0b01` (`IReference`): 0 `Dup`, 0 `Drop`, 0 `ScopeExitDrop`.
//!        * `0b10` (`IObject`):
//!          - When a binding `v` is read `K >= 2` times, emits `K - 1` `"Dup:<v>"` ops
//!            before the first `K - 1` reads and moves on the final read (no drop after move).
//!          - When a binding `v`'s last read (or dead binding) is in the statement
//!            immediately preceding `let new_v: NewObj = ...` where `NewObj` is ALSO `0b10`
//!            and its `LayoutTable` `byte_size` (`layout_metrics`) equals `v`'s `byte_size`,
//!            emits `"DropReuse:<v>-><new_v>"` instead of `"Drop:<v>"`!
//!          - Otherwise, if `v`'s last read occurs before subsequent statements (and is not
//!            returned/moved as the final result), or if `v` is never read, emits `"Drop:<v>"`
//!            at its last-use / dead point!
//!        * `0b11` (`IGuard`):
//!          - NEVER drops early at last use!
//!          - Emits `"ScopeExitDrop:<g>"` at block exit / `return` in strict **LIFO reverse
//!            declaration order** (`"ScopeExitDrop:g3;ScopeExitDrop:g2;ScopeExitDrop:g1"`).
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
    "selfhost/perceus/arc.pilot",
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
            path: "selfhost/perceus/arc.pilot",
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
    required_group_pub_fns: &[
        ("perceus_schedule_fn", PERCEUS_GROUP),
        ("verify_perceus_unit", PERCEUS_GROUP),
    ],
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

    let arc_src = read_ws("selfhost/perceus/arc.pilot");
    let esc_src = read_ws("selfhost/perceus/escape.pilot");
    let rt_src = read_ws("selfhost/perceus/realtime.pilot");
    let opt_src = read_ws("selfhost/perceus/optimizer.pilot");

    let pass_script = format!(
        "{}\n{}\n{}\n{}\nlet __out = {};\n__out;\n",
        arc_src, esc_src, rt_src, opt_src, tail_expr
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
    vm.load_binary(BytecodeCompiler::compile(&pass_arena, pass_arena.root));
    let val = vm.run();
    let diags = registry.diagnostics.into_iter().map(|d| d.message).collect();
    (val, diags)
}

#[test]
fn test_54_1_real_ast_perceus_arc_dup_last_use_drop_reuse_and_lifo_guards() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_54_1_real_ast_perceus_arc_dup_last_use_drop_reuse_and_lifo_guards",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5401);
    let snap = BootstrapSnapshot::global();
    for rel in [
        "selfhost/perceus/arc.pilot",
        "selfhost/perceus/escape.pilot",
        "selfhost/perceus/realtime.pilot",
        "selfhost/perceus/optimizer.pilot",
    ] {
        let src = read_ws(rel);
        let mut tables = snap.tables.clone();
        let mut vm = snap.vm.clone();
        eval_file_with_tables(&src, &mut tables, &mut vm)
            .unwrap_or_else(|e| panic!("{} failed bootstrap evaluation: {:?}", rel, e));
    }

    for trial in 0..6 {
        // Randomized scalar, interface, struct, variable, and function names!
        let sc64 = rng.lower_ident("u64x");
        let sc32 = rng.lower_ident("u32x");
        let if_val = rng.pascal_ident("ICustVal");
        let if_ref = rng.pascal_ident("ICustRef");
        let if_guard = rng.pascal_ident("ICustGuard");

        let val_st = rng.pascal_ident("ValPair");
        let view_st = rng.pascal_ident("WinView");
        let obj16_a = rng.pascal_ident("Buf16A"); // 2 * 64-bit = 16 bytes, has ~self() -> 0b10
        let obj16_b = rng.pascal_ident("Buf16B"); // 4 * 32-bit = 16 bytes, has ~self() -> 0b10 (SAME 16B size!)
        let obj24_c = rng.pascal_ident("Buf24C"); // 3 * 64-bit = 24 bytes, has ~self() -> 0b10 (DIFFERENT 24B size!)
        let guard_st = rng.pascal_ident("ScopeLock"); // : ICustRef + ~self() -> 0b01 | 0b10 = 0b11!

        let fn_dup = rng.lower_ident("fn_dup_test");
        let fn_reuse = rng.lower_ident("fn_reuse_match");
        let fn_noreuse = rng.lower_ident("fn_reuse_mismatch");
        let fn_guards = rng.lower_ident("fn_lifo_guards");

        let v_val = rng.lower_ident("v_copy");
        let v_view = rng.lower_ident("v_lens");
        let v_multi = rng.lower_ident("owned_multi");
        let v_unused = rng.lower_ident("owned_unused");
        let v_old = rng.lower_ident("old_pkt");
        let v_new = rng.lower_ident("new_pkt");
        let g1 = rng.lower_ident("g_alpha");
        let g2 = rng.lower_ident("g_beta");
        let g3 = rng.lower_ident("g_gamma");

        let program = format!(
            "[Bits(64, 2)]\npub struct {sc64} {{}}\n\
             [Bits(32, 2)]\npub struct {sc32} {{}}\n\
             [Lattice(0)]\npub interface {if_val} {{}}\n\
             [Lattice(1)]\npub interface {if_ref} {{}}\n\
             [Lattice(3)]\npub interface {if_guard} {{}}\n\
             pub struct {val_st} : {if_val} {{ pub a: {sc64}; pub b: {sc64}; }}\n\
             pub struct {view_st} : {if_ref} {{ pub p: {sc64}; pub len: {sc64}; }}\n\
             pub struct {obj16_a} {{ pub x: {sc64}; pub y: {sc64}; ~self() {{}} }}\n\
             pub struct {obj16_b} {{ pub p0: {sc32}; pub p1: {sc32}; pub p2: {sc32}; pub p3: {sc32}; ~self() {{}} }}\n\
             pub struct {obj24_c} {{ pub x: {sc64}; pub y: {sc64}; pub z: {sc64}; ~self() {{}} }}\n\
             pub struct {guard_st} : {if_ref} {{ pub handle: {sc64}; ~self() {{}} }}\n\
             \n\
             pub fn consume16({obj16_a}_in: {obj16_a}) -> {sc64} {{ return {obj16_a}_in.x; }}\n\
             pub fn touch_val(x: {val_st}, w: {view_st}) -> {sc64} {{ return x.a; }}\n\
             pub fn touch_guard(g: {sc64}) -> {sc64} {{ return g; }}\n\
             \n\
             // 1) Multi-use 0b10 emits 2 Dups + final move; unused 0b10 drops; 0b00 & 0b01 emit 0 ARC ops!\n\
             pub fn {fn_dup}(seed_v: {val_st}, seed_w: {view_st}, init_a: {obj16_a}, dead_a: {obj16_a}) -> {sc64} {{\n\
                 let {v_val}: {val_st} = seed_v;\n\
                 let {v_view}: {view_st} = seed_w;\n\
                 let {v_multi}: {obj16_a} = init_a;\n\
                 let {v_unused}: {obj16_a} = dead_a;\n\
                 let t0 = touch_val({v_val}, {v_view});\n\
                 let t1 = touch_val({v_val}, {v_view});\n\
                 let u1 = consume16({v_multi});\n\
                 let u2 = consume16({v_multi});\n\
                 let u3 = consume16({v_multi});\n\
                 return u1 + u2 + u3 + t0 + t1;\n\
             }}\n\
             \n\
             // 2) Same byte_size (16B -> 16B across distinct field counts 2x64b vs 4x32b!) -> DropReuse!\n\
             pub fn {fn_reuse}(src_a: {obj16_a}, c0: {sc32}) -> {obj16_b} {{\n\
                 let {v_old}: {obj16_a} = src_a;\n\
                 let extracted = consume16({v_old});\n\
                 let {v_new}: {obj16_b} = {obj16_b} {{ p0: c0, p1: c0, p2: c0, p3: c0 }};\n\
                 return {v_new};\n\
             }}\n\
             \n\
             // 3) Mismatched byte_size (16B -> 24B) -> plain Drop (MUST NOT emit DropReuse)!\n\
             pub fn {fn_noreuse}(src_a: {obj16_a}, c64: {sc64}) -> {obj24_c} {{\n\
                 let {v_old}: {obj16_a} = src_a;\n\
                 let extracted = consume16({v_old});\n\
                 let {v_new}: {obj24_c} = {obj24_c} {{ x: c64, y: c64, z: c64 }};\n\
                 return {v_new};\n\
             }}\n\
             \n\
             // 4) Three 0b11 guards (derived from 0b01 interface + ~self() 0b10 = 0b11!): LIFO ScopeExitDrop!\n\
             pub fn {fn_guards}(h: {sc64}) -> {sc64} {{\n\
                 let {g1}: {guard_st} = {guard_st} {{ handle: h }};\n\
                 let {g2}: {guard_st} = {guard_st} {{ handle: h }};\n\
                 let {g3}: {guard_st} = {guard_st} {{ handle: h }};\n\
                 let early = touch_guard(h);\n\
                 return early;\n\
             }}\n",
        );

        let tail = format!(
            "perceus_schedule_fn({f1}) + \"|||\" + perceus_schedule_fn({f2}) + \"|||\" + perceus_schedule_fn({f3}) + \"|||\" + perceus_schedule_fn({f4})",
            f1 = escape_pilot_str(&fn_dup),
            f2 = escape_pilot_str(&fn_reuse),
            f3 = escape_pilot_str(&fn_noreuse),
            f4 = escape_pilot_str(&fn_guards),
        );
        let (res, _) = run_perceus_on_program(&program, &tail);
        let schedules_str = res.to_string_repr();
        let schedules: Vec<&str> = schedules_str.split("|||").collect();
        assert_eq!(schedules.len(), 4, "Trial {}: unexpected schedule format `{}`", trial, schedules_str);

        let (s_dup, s_reuse, s_noreuse, s_guards) = (schedules[0], schedules[1], schedules[2], schedules[3]);

        // Verify 1: `v_multi` has exactly 2 `Dup:` and 0 `Drop:` (moved on 3rd use); `v_unused` has 1 `Drop:`;
        // `v_val` (0b00) and `v_view` (0b01) never appear in ARC ops!
        let dup_token = format!("Dup:{}", v_multi);
        assert_eq!(
            s_dup.matches(&dup_token).count(),
            2,
            "Trial {}: 3 reads of 0b10 `{}` must emit 2 `{}` ops, got `{}`",
            trial, v_multi, dup_token, s_dup
        );
        assert!(
            !s_dup.contains(&format!("Drop:{}", v_multi)),
            "Trial {}: `{}` moved on final read must NOT also Drop: `{}`",
            trial, v_multi, s_dup
        );
        assert!(
            s_dup.contains(&format!("Drop:{}", v_unused)),
            "Trial {}: unused 0b10 `{}` must emit `Drop:{}`: got `{}`",
            trial, v_unused, v_unused, s_dup
        );
        assert!(
            !s_dup.contains(&v_val) && !s_dup.contains(&v_view),
            "Trial {}: 0b00 `{}` and 0b01 `{}` must have 0 retain/drop ops, got `{}`",
            trial, v_val, v_view, s_dup
        );

        // Verify 2 & 3: Layout-size-verified DropReuse (16B == 16B) vs size mismatch (16B != 24B)
        let expected_reuse = format!("DropReuse:{}->{}", v_old, v_new);
        assert!(
            s_reuse.contains(&expected_reuse),
            "Trial {}: Same-size 16B->16B 0b10 allocation must emit `{}`, got `{}`",
            trial, expected_reuse, s_reuse
        );
        assert!(
            !s_noreuse.contains("DropReuse:"),
            "Trial {}: Different-size 16B->24B 0b10 allocation MUST NOT emit DropReuse, got `{}`",
            trial, s_noreuse
        );

        // Verify 4: Strict LIFO reverse guard order (g3, then g2, then g1) and zero early `Drop:`!
        let expected_lifo = format!(
            "ScopeExitDrop:{};ScopeExitDrop:{};ScopeExitDrop:{}",
            g3, g2, g1
        );
        assert!(
            !s_guards.contains(&format!("Drop:{}", g1)),
            "Trial {}: 0b11 guard `{}` must never drop early before scope exit: `{}`",
            trial, g1, s_guards
        );
        assert!(
            s_guards.contains(&expected_lifo),
            "Trial {}: 0b11 guards must drop in strict LIFO order `{}`, got `{}`",
            trial, expected_lifo, s_guards
        );
    }
    guard.finish();
}

#[test]
fn test_54_2_and_54_3_real_ast_escape_realtime_cue_verifiers_and_dsp_decoy_trap() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_54_2_and_54_3_real_ast_escape_realtime_cue_verifiers_and_dsp_decoy_trap",
        &ANTICHEAT_SPEC,
    );
    let mut rng = Rng::new(0x5402);

    // Static check: forbid `"dsp"` name sniffing or `@hard_realtime` in selfhost/perceus/*.pilot
    for rel in [
        "selfhost/perceus/arc.pilot",
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
