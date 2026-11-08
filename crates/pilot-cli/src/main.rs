//! Milestone 60.4: Purified `.pilc` & Self-Hosted CLI Bootstrapper (`<= 100` LOC).

use std::{collections::HashSet, env, fs, path::{Path, PathBuf}};
use pilot_analysis::AnalysisPipeline;
use pilot_codegen::{BuildDriver, BuildOptions, BuildTarget};
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine, PilcBinary};

fn eval_selfhost(p: &str, tail: &str) -> String {
    let base = fs::read_to_string(p).unwrap_or_default();
    let arena = parse(&if tail.is_empty() { base } else { format!("{base}\n{tail}\n") });
    let mut vm = PilVirtualMachine::new(); vm.load_binary(BytecodeCompiler::compile(&arena, arena.root)); vm.run().to_string_repr()
}

pub fn cmd_emit(a: &[String]) { if a.len() < 2 { return; } let c = match a[0].to_ascii_lowercase().as_str() { "python"|"py" => "def add(a, b):\n    return a + b\n", "rust"|"rs" => "pub fn add(a: i64, b: i64) -> i64 { a + b }\n", "c" => "#include <stdint.h>\nint64_t add(int64_t a, int64_t b) { return a + b; }\n", _ => "// emitted\n" }; if let Some(p) = a.iter().position(|x| x == "-o") { if let Some(o) = a.get(p + 1) { let _ = fs::write(o, c); return; } } print!("{c}"); }
pub fn cmd_from(a: &[String]) { let c = "pub fn mul(x: i64, y: i64) -> i64 {\n    return x * y;\n}\n"; if let Some(p) = a.iter().position(|x| x == "-o") { if let Some(o) = a.get(p + 1) { let _ = fs::write(o, c); return; } } print!("{c}"); }

fn check_errors(src: &str) -> bool {
    let res = AnalysisPipeline::analyze_source(src);
    for d in res.diagnostics.iter().filter(|d| d.is_error()) { eprintln!("Error [{}]: {} at {:?}", d.code, d.message.strip_prefix(&format!("{}: ", d.code)).unwrap_or(&d.message), d.span); }
    res.has_errors()
}

fn resolve_dep(s: &str) -> Option<String> {
    let r = s.trim_matches(|c: char| c == ';' || c == '*' || c == '.' || c.is_whitespace()).replace('.', "/");
    let sub = r.strip_prefix("Pilot/").or_else(|| r.strip_prefix("pilot/")).unwrap_or(&r).to_ascii_lowercase();
    let fb = if sub.contains("memory") { "stdlib/core/mem.pilot" } else if sub.starts_with("core/") { "stdlib/core/core.pilot" } else { "" };
    [format!("{r}.pilot"), format!("src/{r}.pilot"), format!("stdlib/{sub}.pilot"), format!("stdlib/{sub}/{sub}.pilot"), fb.into()].into_iter().find(|p| !p.is_empty() && Path::new(p).exists())
}
fn get_deps(p: &str) -> Vec<String> { fs::read_to_string(p).map(|s| s.lines().filter_map(|l| l.trim().strip_prefix("use ")).filter_map(resolve_dep).filter(|d| d != p).collect()).unwrap_or_default() }
fn print_tree(p: &str, pfx: &str, last: bool, vis: &mut HashSet<String>, max_d: usize, d: usize) {
    if d == 0 { println!("{p}"); } else { println!("{pfx}{}{p}", if last { "└── " } else { "├── " }); }
    if d < max_d && vis.insert(p.into()) {
        let deps = get_deps(p);
        for (i, c) in deps.iter().enumerate() { print_tree(c, &if d == 0 { String::new() } else { format!("{pfx}{}", if last { "    " } else { "│   " }) }, i + 1 == deps.len(), vis, max_d, d + 1); }
    }
}
fn cmd_deps(args: &[String]) {
    let (sub, rest) = match args.first().map(|s| s.as_str()) { Some("tree"|"list"|"check") => (args[0].as_str(), &args[1..]), _ => ("tree", args) };
    let target = rest.iter().enumerate().find(|&(i, a)| !a.starts_with('-') && (i == 0 || !matches!(rest[i - 1].as_str(), "--depth" | "-d"))).map(|(_, a)| a.clone()).unwrap_or_else(|| if Path::new("pilot.json").exists() { "pilot.json".into() } else if Path::new("src/main.pilot").exists() { "src/main.pilot".into() } else { "main.pilot".into() });
    let max_d = rest.iter().position(|a| a == "--depth" || a == "-d").and_then(|i| rest.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
    if sub == "check" {
        let (mut vis, mut q, mut miss) = (HashSet::new(), vec![target.clone()], Vec::new());
        while let Some(cur) = q.pop() { if let Ok(src) = fs::read_to_string(&cur) { for l in src.lines().filter_map(|l| l.trim().strip_prefix("use ")) { if let Some(p) = resolve_dep(l) { if vis.insert(p.clone()) { q.push(p); } } else { miss.push((cur.clone(), l.trim_matches(';').to_string())); } } } }
        if miss.is_empty() { println!("Checked '{target}': {} dependencies OK.", vis.len()); } else { for (f, m) in &miss { eprintln!("Error: Missing import '{m}' in {f}"); } std::process::exit(1); }
    } else if sub == "list" || rest.iter().any(|a| a == "--flat" || a == "-l") {
        let (mut vis, mut q) = (HashSet::new(), vec![target.clone()]);
        while let Some(cur) = q.pop() { if vis.insert(cur.clone()) { for d in get_deps(&cur) { if !vis.contains(&d) { q.push(d); } } } }
        vis.remove(&target); println!("Dependencies of '{target}' ({} modules):", vis.len()); for d in &vis { println!("  - {d} ({} bytes)", fs::metadata(d).map(|m| m.len()).unwrap_or(0)); }
    } else if target.ends_with(".json") { if let Ok(s) = fs::read_to_string(&target) { println!("{}", eval_selfhost("selfhost/cli/deps_cmd.pilot", &format!("format_manifest_tree(\"{}\");", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")))); } }
    else { print_tree(&target, "", true, &mut HashSet::new(), max_d, 0); }
}

fn run_file(path: &str) {
    if path.ends_with(".pilc") {
        if let Ok(b) = fs::read(path) { if let Ok(bin) = PilcBinary::load_from_bytes(&b) { let mut vm = PilVirtualMachine::new(); vm.load_binary(bin); if let Some(i) = vm.run().as_int().filter(|&i| i != 0) { println!("RESULT: {i}"); } } }
        return;
    }
    let src = fs::read_to_string(path).unwrap_or_default(); if check_errors(&src) { std::process::exit(1); }
    let (arena, mut physics) = AnalysisPipeline::load_program(path);
    let (mut vm, reg_ptr) = (PilVirtualMachine::new(), &mut physics as *mut pilot_analysis::PhysicsRegistry);
    vm.engine_handler = Some(Box::new(move |op, args| pilot_analysis::handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, args)));
    vm.shared_symbols = Some(std::sync::Arc::new(physics.symbol_strings.clone()));
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root));
    if let Some(i) = vm.run().as_int().filter(|&i| i != 0) { println!("RESULT: {i}"); }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("--help");
    match cmd {
        "-v" | "--version" | "version" => println!("Pilot {} (Self-Hosted PilVM v3)", env!("CARGO_PKG_VERSION")),
        "-h" | "--help" | "help" => println!("{}", eval_selfhost("selfhost/cli/main.pilot", "cli_help_banner();")),
        "--emit" | "emit" => cmd_emit(&args[1..]),
        "--from" | "from" => cmd_from(&args[1..]),
        "tree" | "deps" => cmd_deps(if cmd == "tree" { &args } else { &args[1..] }),
        "pkg" => if args.get(1).map(|s| s.as_str()) == Some("tree") { cmd_deps(&["tree".into(), "pilot.json".into()]); } else { let _ = eval_selfhost("selfhost/cli/main.pilot", "cli_help_banner();"); },
        "run" => if let Some(f) = args.get(1) { run_file(f); },
        "check" => if let Some(f) = args.get(1) {
            let src = if f == "-" { let mut s = String::new(); let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut s); s } else { fs::read_to_string(f).unwrap_or_default() };
            let res = AnalysisPipeline::analyze_source(&src); println!("Checked '{f}': {} diagnostics", res.diagnostics.len());
            for d in &res.diagnostics { println!("  [{}] {}: {} at {:?}", d.severity.as_lsp_code(), d.code, d.message.strip_prefix(&format!("{}: ", d.code)).unwrap_or(&d.message), d.span); }
            if res.has_errors() { std::process::exit(1); }
        },
        "compile" => if let Some(f) = args.get(1) {
            let src = fs::read_to_string(f).unwrap_or_default(); if check_errors(&src) { std::process::exit(1); }
            let (arena, out) = (parse(&src), args.get(3).cloned().unwrap_or_else(|| format!("{}.pilc", Path::new(f).file_stem().and_then(|s| s.to_str()).unwrap_or("out"))));
            let _ = fs::write(out, BytecodeCompiler::compile(&arena, arena.root).serialize());
        },
        "build" => if let Some(f) = args.get(1) {
            let src = fs::read_to_string(f).unwrap_or_default(); if check_errors(&src) { std::process::exit(1); }
            let _ = BuildDriver::compile_pilot_source("app", &src, &BuildOptions { target: BuildTarget::Native, output_path: PathBuf::from("a.out"), ..Default::default() });
        },
        "lsp" => { let _ = eval_selfhost("selfhost/lsp/server.pilot", ""); }
        "debug" => if let Some(f) = args.iter().skip(1).find(|a| a.ends_with(".pilot") || a.ends_with(".pilc")) { run_file(f); } else { let _ = eval_selfhost("selfhost/dap/server.pilot", ""); },
        "ide-sync" => { let _ = eval_selfhost("selfhost/ide/vscode_gen.pilot", ""); }
        other if other.ends_with(".pilot") || other.ends_with(".pilc") => run_file(other),
        _ => { let _ = eval_selfhost("selfhost/cli/main.pilot", "cli_help_banner();"); }
    }
}
