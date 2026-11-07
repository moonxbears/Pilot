use std::path::Path;
use pilot_analysis::{eval_file_with_tables, BootstrapSnapshot};
use pilot_rt::PilotVal;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

fn run_pilot_script(modules: &[&str], main_src: &str) -> PilotVal {
    let mut combined = String::new();
    for m in modules {
        combined.push_str(m);
        combined.push('\n');
    }
    combined.push_str(main_src);
    let arena = pilot_parser::parse(&combined);
    assert!(
        arena.diagnostics.is_empty(),
        "Pilot script parse diagnostics: {:?}",
        arena.diagnostics
    );
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    vm.run()
}

#[test]
fn milestone54_1_lattice_arc_and_drop_reuse_execute_on_vm() {
    let arc_src = include_str!("../../../selfhost/perceus/arc.pilot");
    let opt_src = include_str!("../../../selfhost/perceus/optimizer.pilot");
    let rt_src = include_str!("../../../selfhost/perceus/realtime.pilot");

    let snap = BootstrapSnapshot::global();
    for (name, src) in [
        ("selfhost/perceus/arc.pilot", arc_src),
        ("selfhost/perceus/optimizer.pilot", opt_src),
    ] {
        let mut tables = snap.tables.clone();
        let mut vm = snap.vm.clone();
        let res = eval_file_with_tables(src, &mut tables, &mut vm);
        assert!(res.is_ok(), "{} failed analysis: {:?}", name, res.err());
    }

    let val = run_pilot_script(
        &[arc_src, rt_src, opt_src],
        r#"
fn main() -> i64 {
    // 1. 2-bit lattice classification (`0b00..0b11`)
    if classify_perceus_action(0) != ownership_trivial_bit_copy() { return 10; }
    if classify_perceus_action(1) != ownership_lexical_borrow_only() { return 11; }
    if classify_perceus_action(2) != ownership_move_and_drop() { return 12; }
    if classify_perceus_action(3) != ownership_scoped_guard_move_and_drop() { return 13; }

    // 2. Binding lifecycle tracking by lattice
    let val_state = track_binding_by_lattice("val_x", 0);
    if !val_state.elided_retain || val_state.needs_scope_exit_drop { return 20; }

    let ref_state = track_binding_by_lattice("ref_y", 1);
    if !ref_state.elided_retain || ref_state.needs_scope_exit_drop { return 21; }

    let obj_state = track_binding_by_lattice("obj_z", 2);
    if !obj_state.allow_early_last_use_drop || !obj_state.needs_scope_exit_drop { return 22; }

    let guard_state = track_binding_by_lattice("guard_g", 3);
    if guard_state.allow_early_last_use_drop || !guard_state.needs_scope_exit_drop { return 23; }

    // 3. Guard (`0b11`) must never drop early before scope exit
    if emit_drop_op_for_lattice("guard_g", 3, false) != "" { return 30; }
    if emit_drop_op_for_lattice("guard_g", 3, true) != "ScopeExitDrop:guard_g" { return 31; }
    if emit_drop_op_for_lattice("obj_z", 2, false) != "Drop:obj_z" { return 32; }

    // 4. DropReuse in-place memory recycling when layout sizes match
    if emit_drop_reuse_op("old_buf", 2, 64, "new_buf", 64) != "DropReuse:old_buf->new_buf" { return 40; }
    if emit_drop_reuse_op("old_buf", 2, 64, "other_buf", 128) != "Drop:old_buf" { return 41; }

    // 5. Reverse-order (LIFO) scope-exit drops for `0b11` scoped guards
    if emit_reverse_guard_drops("g1,g2,g3") != "ScopeExitDrop:g3;ScopeExitDrop:g2;ScopeExitDrop:g1" { return 50; }

    return 1;
}
"#,
    );
    assert_eq!(val.as_int(), Some(1));
}

#[test]
fn milestone54_2_borrow_escape_and_hard_realtime_verifiers_execute_on_vm() {
    let esc_src = include_str!("../../../selfhost/perceus/escape.pilot");
    let rt_src = include_str!("../../../selfhost/perceus/realtime.pilot");
    let opt_src = include_str!("../../../selfhost/perceus/optimizer.pilot");

    let snap = BootstrapSnapshot::global();
    for (name, src) in [
        ("selfhost/perceus/escape.pilot", esc_src),
        ("selfhost/perceus/realtime.pilot", rt_src),
    ] {
        let mut tables = snap.tables.clone();
        let mut vm = snap.vm.clone();
        let res = eval_file_with_tables(src, &mut tables, &mut vm);
        assert!(res.is_ok(), "{} failed analysis: {:?}", name, res.err());
    }

    let val = run_pilot_script(
        &[esc_src, rt_src, opt_src],
        r#"
fn main() -> i64 {
    // 1. ESCAPE001: Borrow-tracked (`0b01` or `0b11`) crossing `.pchor` cue boundary
    if verify_borrow_escape_by_lattice("view_v", 1, true, -1, true) != "ESCAPE001" { return 10; }
    if verify_borrow_escape_by_lattice("guard_g", 3, true, -1, true) != "ESCAPE001" { return 11; }

    // 2. ESCAPE002: Borrow-tracked stored inside `0b10` (`IObject`) heap container
    if verify_borrow_escape_by_lattice("view_v", 1, true, 2, false) != "ESCAPE002" { return 20; }

    // 3. ESCAPE003: Borrow-tracked escaping function return without input `ref` tie
    if verify_borrow_escape_by_lattice("view_v", 1, false, -1, false) != "ESCAPE003" { return 30; }
    if verify_borrow_escape_by_lattice("view_v", 1, true, -1, false) != "" { return 31; }

    // 4. PILOT105: `.pchor` cue transfer allows `0b00` (`IValue`) and `iso 0b10` (`IObject`) only
    if verify_cue_transfer_by_lattice("audio_cue", 0, false) != "" { return 40; }
    if verify_cue_transfer_by_lattice("audio_cue", 2, true) != "" { return 41; }
    if verify_cue_transfer_by_lattice("audio_cue", 2, false) != "PILOT105" { return 42; }
    if verify_cue_transfer_by_lattice("audio_cue", 3, true) != "PILOT105" { return 43; }

    // 5. PILOT104: `[HardRealtime]` & `noalloc` contract verification (zero "dsp" name sniffing!)
    let rt_fn = "[HardRealtime]\npub fn process_frame(x: f64) -> f64 { return x; }";
    let noalloc_fn = "pub noalloc fn step(x: f64) -> f64 { return x; }";
    let dsp_named_fn = "pub fn dsp_helper(x: f64) -> f64 { return x; }";

    if analyze_perceus_realtime_source(rt_fn, true) != "PILOT104" { return 50; }
    if analyze_perceus_realtime_source(noalloc_fn, true) != "PILOT104" { return 51; }
    // Function named `dsp_helper` without `[HardRealtime]` or `noalloc` must NOT trigger PILOT104!
    if analyze_perceus_realtime_source(dsp_named_fn, true) != "" { return 52; }

    return 1;
}
"#,
    );
    assert_eq!(val.as_int(), Some(1));

    // Deliverable 54.3: Verify crates/pilot-perceus is completely deleted from disk
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    assert!(
        !root.join("crates/pilot-perceus").exists(),
        "crates/pilot-perceus must be deleted from disk"
    );
}
