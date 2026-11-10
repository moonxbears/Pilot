//! Milestone 3: Minimal Pure-Pilot CLI Launcher (<= 35 LOC).
mod link;
mod run;
mod tool;
use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let spec = args.join("\x1f").replace('\\', "\\\\").replace('"', "\\\"");
    let out = tool::eval(&["stdlib/core/file.pilot", "stdlib/core/env.pilot", "stdlib/core/primitives/map.pilot", "selfhost/cli/spec.pilot", "selfhost/cli/parser.pilot", "selfhost/cli/help_and_comp.pilot", "selfhost/cli/main.pilot"], &format!("run_cli_entry(\"{spec}\");"));
    let (act, p) = out.split_once(':').unwrap_or((&out, ""));
    if act == "RUN" { let mut lines = p.split('\n'); let file = lines.next().unwrap_or(""); std::process::exit(run::run_file(file, lines.map(str::to_string).collect())); }
    else if act == "GATE" { let r = tool::eval(&["stdlib/crypto/hash.pilot", "selfhost/lexer.pilot", "selfhost/ast.pilot", "selfhost/parser.pilot", "selfhost/pilotc.pilot", "selfhost/gate/manifest.pilot", "selfhost/gate/seed_audit.pilot", "selfhost/gate/rename_litmus.pilot", "selfhost/gate/bootstrap_parity.pilot", "selfhost/gate/runner.pilot"], &format!("run_pilot_gate_cli(\".\", \"{p}\");")); let (c, b) = r.split_once('\n').unwrap_or(("1", &r)); println!("{b}"); if let Ok(n) = c.parse::<i32>() { if n != 0 { std::process::exit(n); } } }
    else if act == "EMIT" { let (l, f) = p.split_once('|').unwrap_or((p, "")); let ir = "selfhost/transpile/ir.pilot".to_string(); let emitter = format!("selfhost/transpile/emit_{l}.pilot"); print!("{}", tool::eval_on_input(&[&ir, &emitter], &format!("transpile_arena_to_{l}();"), f)); }
    else if act == "PRINT" { let (msg, code) = p.split_once("\nEXIT:").unwrap_or((p, "0")); println!("{msg}"); if let Ok(n) = code.parse::<i32>() { if n != 0 { std::process::exit(n); } } }
    else if let Ok(n) = act.parse::<i32>() { if n != 0 { std::process::exit(n); } }
}
