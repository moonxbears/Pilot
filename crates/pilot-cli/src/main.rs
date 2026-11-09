//! Milestone 1: Unlocked Shrinking CLI Bootstrapper & Pure-Pilot `pilot gate` (`<= 100` LOC).
use std::{collections::HashSet, env, fs, path::{Path, PathBuf}};
use pilot_analysis::AnalysisPipeline;
use pilot_codegen::{BuildDriver, BuildOptions, BuildTarget};
use pilot_parser::parse;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine, PilcBinary};
fn read_src(p: &str) -> Option<String> { [p.into(), format!("../{p}"), format!("../../{p}")].into_iter().find_map(|f| fs::read_to_string(f).ok()) }
fn eval_selfhost(p: &str, tail: &str) -> String {
    let mut base = String::new(); for d in get_deps(p) { if let Some(s) = read_src(&d) { base.push_str(&s); base.push('\n'); } }
    if let Some(s) = read_src(p) { base.push_str(&s); }
    let arena = parse(&if tail.is_empty() { base } else { format!("{base}\n{tail}\n") });
    let mut vm = PilVirtualMachine::new(); vm.load_binary(BytecodeCompiler::compile(&arena, arena.root)); vm.run().to_string_repr()
}
fn emit_or_from(is_emit: bool, a: &[String]) {
    if a.len() < 2 { return; }
    let (lang, sf, out) = (a[0].to_ascii_lowercase(), a.iter().skip(1).find(|x| !x.starts_with('-') && *x != "-o"), a.iter().position(|x| x == "-o").and_then(|p| a.get(p + 1)));
    if let Some(f) = sf { if let Ok(s) = fs::read_to_string(f) {
        let p_esc = s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
        let res = if is_emit {
            let ef = match lang.as_str() { "python"|"py" => ("emit_python", "transpile_arena_to_python()"), "rust"|"rs" => ("emit_rust", "transpile_arena_to_rust()"), "c" => ("emit_c", "transpile_arena_to_c()"), "cpp"|"c++" => ("emit_cpp", "transpile_arena_to_cpp()"), "csharp"|"cs" => ("emit_csharp", "transpile_arena_to_csharp()"), "js"|"javascript" => ("emit_js", "transpile_arena_to_js()"), "lua" => ("emit_lua", "transpile_arena_to_lua()"), _ => ("", "") };
            if ef.0.is_empty() { return; }
            let b = format!("{}\n{}\nlet _ = parse(\"{p_esc}\"); {};", read_src("selfhost/transpile/ir.pilot").unwrap_or_default(), read_src(&format!("selfhost/transpile/{}.pilot", ef.0)).unwrap_or_default(), ef.1);
            let arena = parse(&b); let mut vm = PilVirtualMachine::new(); vm.load_binary(BytecodeCompiler::compile(&arena, arena.root)); vm.run().to_string_repr()
        } else { eval_selfhost("selfhost/transpile/ingest.pilot", &format!("ingest_foreign_source(\"{lang}\", \"{p_esc}\");")) };
        if let Some(o) = out { let _ = fs::write(o, &res); } else { print!("{res}"); }
    }}
}
pub fn cmd_emit(a: &[String]) { emit_or_from(true, a); } pub fn cmd_from(a: &[String]) { emit_or_from(false, a); }
fn check_errors(src: &str) -> bool { let res = AnalysisPipeline::analyze_source(src); for d in res.diagnostics.iter().filter(|d| d.is_error()) { eprintln!("Error [{}]: {} at {:?}", d.code, d.message.strip_prefix(&format!("{}: ", d.code)).unwrap_or(&d.message), d.span); } res.has_errors() }
fn resolve_dep(s: &str) -> Option<String> {
    let r = s.trim_matches(|c: char| c == ';' || c == '*' || c == '.' || c.is_whitespace()).replace('.', "/");
    let sub = r.strip_prefix("Pilot/").or_else(|| r.strip_prefix("pilot/")).unwrap_or(&r).to_ascii_lowercase();
    let fb = if sub.contains("memory") { "stdlib/core/mem.pilot" } else if sub.starts_with("core/") { "stdlib/core/core.pilot" } else { "" };
    [format!("{r}.pilot"), format!("{sub}.pilot"), format!("src/{r}.pilot"), format!("stdlib/{sub}.pilot"), format!("stdlib/{sub}/{sub}.pilot"), fb.into()].into_iter().find(|p| !p.is_empty() && read_src(p).is_some())
}
fn get_deps(p: &str) -> Vec<String> { read_src(p).map(|s| s.lines().filter_map(|l| l.trim().strip_prefix("use ")).filter_map(resolve_dep).filter(|d| d != p).collect()).unwrap_or_default() }
fn print_tree(p: &str, pfx: &str, last: bool, vis: &mut HashSet<String>, max_d: usize, d: usize) {
    println!("{}{p}", if d == 0 { String::new() } else { format!("{pfx}{}", if last { "└── " } else { "├── " }) });
    if d < max_d && vis.insert(p.into()) { for (i, c) in get_deps(p).iter().enumerate() { print_tree(c, &if d == 0 { String::new() } else { format!("{pfx}{}", if last { "    " } else { "│   " }) }, i + 1 == get_deps(p).len(), vis, max_d, d + 1); } }
}
fn cmd_deps(args: &[String]) {
    let (sub, rest) = match args.first().map(|s| s.as_str()) { Some("tree"|"list"|"check") => (args[0].as_str(), &args[1..]), _ => ("tree", args) };
    let target = rest.iter().enumerate().find(|&(i, a)| !a.starts_with('-') && (i == 0 || !matches!(rest[i - 1].as_str(), "--depth" | "-d"))).map(|(_, a)| a.clone()).unwrap_or_else(|| ["pilot.json", "src/main.pilot", "main.pilot"].into_iter().find(|p| Path::new(p).exists()).unwrap_or("main.pilot").into());
    let max_d = rest.iter().position(|a| a == "--depth" || a == "-d").and_then(|i| rest.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
    if sub == "check" { let (mut vis, mut q, mut miss) = (HashSet::new(), vec![target.clone()], Vec::new());
        while let Some(cur) = q.pop() { if let Ok(src) = fs::read_to_string(&cur) { for l in src.lines().filter_map(|l| l.trim().strip_prefix("use ")) { if let Some(p) = resolve_dep(l) { if vis.insert(p.clone()) { q.push(p); } } else { miss.push((cur.clone(), l.trim_matches(';').to_string())); } } } }
        if miss.is_empty() { println!("Checked '{target}': {} dependencies OK.", vis.len()); } else { for (f, m) in &miss { eprintln!("Error: Missing import '{m}' in {f}"); } std::process::exit(1); }
    } else if sub == "list" || rest.iter().any(|a| a == "--flat" || a == "-l") { let (mut vis, mut q) = (HashSet::new(), vec![target.clone()]);
        while let Some(cur) = q.pop() { if vis.insert(cur.clone()) { for d in get_deps(&cur) { if !vis.contains(&d) { q.push(d); } } } }
        vis.remove(&target); println!("Dependencies of '{target}' ({} modules):", vis.len()); for d in &vis { println!("  - {d} ({} bytes)", fs::metadata(d).map(|m| m.len()).unwrap_or(0)); }
    } else if target.ends_with(".json") { if let Ok(s) = fs::read_to_string(&target) { println!("{}", eval_selfhost("selfhost/cli/deps_cmd.pilot", &format!("format_manifest_tree(\"{}\");", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")))); } }
    else { print_tree(&target, "", true, &mut HashSet::new(), max_d, 0); }
}
fn cmd_map(args: &[String]) {
    if args.iter().any(|a| a == "--help" || a == "-h") { println!("{}", eval_selfhost("selfhost/cli/map_cmd.pilot", "map_help_banner();")); return; }
    let (target, flags) = (args.iter().find(|a| !a.starts_with('-')).cloned().unwrap_or_else(|| "src/main.pilot".into()), args.iter().filter(|a| a.starts_with('-')).cloned().collect::<Vec<_>>().join(" "));
    let mut tgts: Vec<String> = if Path::new(&target).is_dir() { fs::read_dir(&target).map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path().to_string_lossy().to_string()).filter(|p| p.ends_with(".pilot")).collect()).unwrap_or_default() } else { vec![target] }; tgts.sort();
    for t in tgts { if let Some(s) = read_src(&t) { println!("{}", eval_selfhost("selfhost/cli/map_cmd.pilot", &format!("format_code_map(\"{t}\", \"{}\", \"{flags}\");", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")))); } }
}
fn cmd_pkg(a: &[String]) {
    let out = eval_selfhost("selfhost/cli/pkg_cmd.pilot", &format!("handle_pkg_cmd(\"{}\", \"{}\", \"{}\", \"{}\");", a.first().map(|s| s.as_str()).unwrap_or("help"), a.get(1..).unwrap_or(&[]).join(" "), fs::read_to_string("pilot.json").unwrap_or_default().replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"), fs::read_to_string("pilot.lock").unwrap_or_default().replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")));
    for p in out.split("@@WRITE:") { if let Some((path, c)) = p.split_once("@@\n") { let _ = fs::write(path.trim(), c); } }
    let d = out.split("@@WRITE:").next().unwrap_or(&out); if !d.trim().is_empty() { println!("{}", d.trim()); }
}
fn run_file(p: &str) {
    if p.ends_with(".pilc") { if let Ok(b) = fs::read(p) { if let Ok(bin) = PilcBinary::load_from_bytes(&b) { let mut vm = PilVirtualMachine::new(); vm.load_binary(bin); if let Some(i) = vm.run().as_int().filter(|&i| i != 0) { println!("RESULT: {i}"); } } } return; }
    let src = fs::read_to_string(p).unwrap_or_default(); if check_errors(&src) { std::process::exit(1); }
    let (arena, mut phy) = AnalysisPipeline::load_program(p);
    let (mut vm, reg_ptr) = (PilVirtualMachine::new(), &mut phy as *mut pilot_analysis::PhysicsRegistry);
    let (mut em, mut tm, mut am, mut sm) = (std::collections::HashMap::new(), std::collections::HashMap::new(), std::collections::HashMap::new(), std::collections::HashMap::new());
    for (k, v) in [("architecture", pilot_rt::PilotVal::from_str(env::consts::ARCH)), ("os", pilot_rt::PilotVal::from_str(env::consts::OS)), ("pointer_byte", pilot_rt::PilotVal::from_int(std::mem::size_of::<usize>() as i64)), ("endianess", pilot_rt::PilotVal::from_str(if cfg!(target_endian = "little") { "little" } else { "big" })), ("backend", pilot_rt::PilotVal::from_str("vm")), ("simd_width_bits", pilot_rt::PilotVal::from_int(128)), ("cache_line", pilot_rt::PilotVal::from_int(64)), ("cache_line_bytes", pilot_rt::PilotVal::from_int(64)), ("opt_level", pilot_rt::PilotVal::from_int(2))] { tm.insert(k.into(), v); }
    am.insert("node_count".into(), pilot_rt::PilotVal::from_int(phy.arena.nodes.len() as i64)); am.insert("root_node".into(), pilot_rt::PilotVal::from_int(0)); sm.insert("count".into(), pilot_rt::PilotVal::from_int(phy.symbol_strings.len() as i64));
    for (k, v) in [("is_comptime", 0), ("unit_count", 1), ("symbol_count", phy.symbol_strings.len() as i64), ("target", 1), ("ast", 2), ("symbols", 3)] { em.insert(k.into(), pilot_rt::PilotVal::from_int(v)); }
    vm.heap_maps.extend([em, tm, am, sm]); vm.shared_symbols = Some(std::sync::Arc::new(phy.symbol_strings.clone()));
    vm.engine_handler = Some(Box::new(move |op, args| pilot_analysis::handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, args)));
    vm.load_binary(BytecodeCompiler::compile(&arena, arena.root)); if let Some(i) = vm.run().as_int().filter(|&i| i != 0) { println!("RESULT: {i}"); }
}
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("--help");
    match cmd {
        "-v" | "--version" | "version" => println!("Pilot {} (Self-Hosted PilVM v3)", env!("CARGO_PKG_VERSION")), "-h" | "--help" | "help" => println!("{}", eval_selfhost("selfhost/cli/main.pilot", "cli_help_banner();")),
        "--emit" | "emit" => cmd_emit(&args[1..]), "--from" | "from" => cmd_from(&args[1..]), "tree" | "deps" => cmd_deps(if cmd == "tree" { &args } else { &args[1..] }), "map" | "outline" => cmd_map(&args[1..]), "pkg" => cmd_pkg(&args[1..]),
        "gate" => { let out = eval_selfhost("selfhost/gate/runner.pilot", &format!("run_pilot_gate_cli(\".\", \"{}\");", args[1..].join(" ").replace('\\', "\\\\").replace('"', "\\\"")));
            let (c, b) = out.split_once('\n').unwrap_or(("1", &out)); println!("{b}"); if let Ok(n) = c.parse::<i32>() { if n != 0 { std::process::exit(n); } } },
        "run" => if let Some(f) = args.get(1) { run_file(f); },
        "check" => if let Some(f) = args.get(1) { let src = if f == "-" { let mut s = String::new(); let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut s); s } else { fs::read_to_string(f).unwrap_or_default() };
            let res = AnalysisPipeline::analyze_source(&src); println!("Checked '{f}': {} diagnostics", res.diagnostics.len());
            for d in &res.diagnostics { println!("  [{}] {}: {} at {:?}", d.severity.as_lsp_code(), d.code, d.message.strip_prefix(&format!("{}: ", d.code)).unwrap_or(&d.message), d.span); } if res.has_errors() { std::process::exit(1); } },
        "compile" | "build" => if let Some(f) = args.get(1) {
            let src = fs::read_to_string(f).unwrap_or_default(); if check_errors(&src) { std::process::exit(1); }
            if cmd == "compile" { let a = parse(&src); let out = args.get(3).cloned().unwrap_or_else(|| format!("{}.pilc", Path::new(f).file_stem().and_then(|s| s.to_str()).unwrap_or("out"))); let _ = fs::write(out, BytecodeCompiler::compile(&a, a.root).serialize()); }
            else { let _ = BuildDriver::compile_pilot_source("app", &src, &BuildOptions { target: BuildTarget::Native, output_path: PathBuf::from("a.out"), ..Default::default() }); }
        },
        "lsp" | "ide-sync" => { let _ = eval_selfhost(if cmd == "lsp" { "selfhost/lsp/server.pilot" } else { "selfhost/ide/vscode_gen.pilot" }, ""); }, "debug" => if let Some(f) = args.iter().skip(1).find(|a| a.ends_with(".pilot") || a.ends_with(".pilc")) { run_file(f); } else { let _ = eval_selfhost("selfhost/dap/server.pilot", ""); },
        other if other.ends_with(".pilot") || other.ends_with(".pilc") => run_file(other),
        _ => { let _ = eval_selfhost("selfhost/cli/main.pilot", "cli_help_banner();"); }
    }
}
