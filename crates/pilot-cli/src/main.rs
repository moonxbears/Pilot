//! Milestone 3: Minimal Pure-Pilot CLI Launcher (<= 35 LOC).
use std::{env, fs};
use pilot_analysis::{AnalysisPipeline, PhysicsRegistry};
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine, PilcBinary};

fn read_src(p: &str) -> Option<String> { [p.into(), format!("../{p}"), format!("../../{p}")].into_iter().find_map(|f| fs::read_to_string(f).ok()) }
fn eval_sh(paths: &[&str], tail: &str) -> String {
    let mut b = String::new(); for &d in paths { if let Some(s) = read_src(d) { b.push_str(&s); b.push('\n'); } }
    let arena = parse(&format!("{b}\n{tail}"));
    let (flat, syms) = pilot_analysis::flatten_ast(&arena, arena.root);
    let (mut vm, mut phy) = (PilVirtualMachine::new(), PhysicsRegistry::new(flat, syms));
    let reg_ptr = &mut phy as *mut PhysicsRegistry;
    vm.engine_handler = Some(Box::new(move |op, a| pilot_analysis::handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, a)));
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root)); vm.run().to_string_repr()
}
fn run_f(p: &str) {
    if p.ends_with(".pilc") { if let Ok(b) = fs::read(p) { if let Ok(bin) = PilcBinary::load_from_bytes(&b) { let mut vm = PilVirtualMachine::new(); vm.load_binary(bin); if let Some(i) = vm.run().as_int().filter(|&i| i != 0) { println!("RESULT: {i}"); } } } return; }
    let (arena, mut phy) = AnalysisPipeline::load_program(p); let (mut vm, reg_ptr) = (PilVirtualMachine::new(), &mut phy as *mut PhysicsRegistry);
    vm.engine_handler = Some(Box::new(move |op, a| pilot_analysis::handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, a)));
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root)); if let Some(i) = vm.run().as_int().filter(|&i| i != 0) { println!("RESULT: {i}"); }
}
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let spec = args.join("\x1f").replace('\\', "\\\\").replace('"', "\\\"");
    let out = eval_sh(&["selfhost/cli/spec.pilot", "selfhost/cli/parser.pilot", "selfhost/cli/help_and_comp.pilot", "selfhost/cli/main.pilot"], &format!("run_cli_entry(\"{spec}\");"));
    let (act, p) = out.split_once(':').unwrap_or((&out, ""));
    if act == "RUN" { run_f(p); }
    else if act == "GATE" { let r = eval_sh(&["stdlib/crypto/hash.pilot", "selfhost/lexer.pilot", "selfhost/ast.pilot", "selfhost/parser.pilot", "selfhost/pilotc.pilot", "selfhost/gate/manifest.pilot", "selfhost/gate/seed_audit.pilot", "selfhost/gate/rename_litmus.pilot", "selfhost/gate/bootstrap_parity.pilot", "selfhost/gate/runner.pilot"], &format!("run_pilot_gate_cli(\".\", \"{p}\");")); let (c, b) = r.split_once('\n').unwrap_or(("1", &r)); println!("{b}"); if let Ok(n) = c.parse::<i32>() { if n != 0 { std::process::exit(n); } } }
    else if act == "EMIT" { let (l, f) = p.split_once('|').unwrap_or((p, "")); let s = fs::read_to_string(f).unwrap_or_default(); let (flat, syms) = pilot_analysis::flatten_ast(&parse(&s), parse(&s).root); let mut phy = PhysicsRegistry::new(flat, syms); phy.source_text = s; let arena = parse(&format!("{}\n{}\ntranspile_arena_to_{l}();", read_src("selfhost/transpile/ir.pilot").unwrap_or_default(), read_src(&format!("selfhost/transpile/emit_{l}.pilot")).unwrap_or_default())); let (mut vm, reg_ptr) = (PilVirtualMachine::new(), &mut phy as *mut PhysicsRegistry); vm.engine_handler = Some(Box::new(move |op, a| pilot_analysis::handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, a))); vm.load_binary(BytecodeCompiler::compile(&arena, arena.root)); print!("{}", vm.run().to_string_repr()); }
    else if act == "PRINT" { let (msg, code) = p.split_once("\nEXIT:").unwrap_or((p, "0")); println!("{msg}"); if let Ok(n) = code.parse::<i32>() { if n != 0 { std::process::exit(n); } } }
    else if let Ok(n) = act.parse::<i32>() { if n != 0 { std::process::exit(n); } }
}
