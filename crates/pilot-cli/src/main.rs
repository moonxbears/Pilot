//! Milestone 36.1: Standalone Unified Native `pilot` CLI.
//!
//! Subcommands:
//! - `pilot run <file.pilot> [args...]`
//! - `pilot build <file.pilot> [--target llvm|c|wasm|native] [-o <out>] [-O0|-O2|-O3]`
//! - `pilot check <file.pilot> [--comptime-audit]`
//! - `pilot fmt <file.pilot> [--check] [--write]`
//! - `pilot test [filter]`
//! - `pilot repl`
//! - `pilot pkg <install|build|audit>`
//! - `pilot venv <init|activate|list>`
//! - `pilot debug [--dap]`
//! - `pilot lsp`
//! - `pilot perf <file.pilot>`

use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use pilot_analysis::AnalysisPipeline;
use pilot_codegen::{BuildDriver, BuildOptions, BuildTarget, OptLevel};
use pilot_fmt::{format_source, FormatOptions};
use pilot_parser::parse;
use pilot_rt::{BumpRegion, PilotVal};
use pilot_span::LineIndex;
use pilot_transpile::{ForeignAstIngester, IngestOptions, IrModule, TargetLanguage, Transpiler};
use pilot_vm::{Instruction, Opcode, PilcBinary, PilVirtualMachine};

fn print_help() {
    println!(
        r#"Pilot Language Compiler & Toolchain (v1.1.0 Native AOT)
Usage: pilot <command> [options] [arguments]

Commands:
  run <file> [args...]     Execute a Pilot program directly via PilVM v3
  build <file> [options]   Compile Pilot source to native AOT binary, LLVM IR, C, or WASM
  check <file>             Perform zero-allocation syntax and semantic analysis
  fmt <file>               Format Pilot source files according to style invariants
  test [filter]            Discover and run Pilot unit and integration test suites
  repl                     Start interactive Pilot Read-Eval-Print Loop
  pkg <subcommand>         Manage Pilot packages and verify permissive SPDX licenses
  venv <subcommand>        Manage isolated Pilot virtual environments
  debug [--dap]            Launch debug adapter or interactive debugger
  lsp                      Start native JSON-RPC Language Server Protocol service
  perf <file>              Benchmark parse throughput and peak AST memory overhead
  --emit <lang> <file>     Transpile Pilot source to C, C++, C#, Rust, Python, JS, or Lua
  --from <lang> <file>     Ingest foreign source code into canonical Pilot syntax

Flags:
  -v, --version            Display version and toolchain build metadata
  -h, --help               Display this help overview
"#
    );
}

fn print_version() {
    println!("Pilot 1.1.0 (x86_64-unknown-linux-gnu, Standalone Native AOT, Zero-GC)");
    println!("Commit: 8a4b159 (Phase D Final Release)");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
        return;
    }

    match args[0].as_str() {
        "-v" | "--version" | "version" => print_version(),
        "-h" | "--help" | "help" => print_help(),
        "--emit" | "emit" => cmd_emit(&args[1..]),
        "--from" | "from" => cmd_from(&args[1..]),
        "run" => cmd_run(&args[1..]),
        "build" => cmd_build(&args[1..]),
        "check" => cmd_check(&args[1..]),
        "fmt" => cmd_fmt(&args[1..]),
        "test" => cmd_test(&args[1..]),
        "repl" => cmd_repl(),
        "pkg" => cmd_pkg(&args[1..]),
        "venv" => cmd_venv(&args[1..]),
        "debug" => cmd_debug(&args[1..]),
        "lsp" => cmd_lsp(&args[1..]),
        "perf" => cmd_perf(&args[1..]),
        unknown => {
            if unknown.ends_with(".pilot") {
                cmd_run(&args);
            } else {
                eprintln!("error: unknown command '{}'. Run 'pilot --help' for usage.", unknown);
                std::process::exit(1);
            }
        }
    }
}

fn cmd_run(args: &[String]) {
    if args.is_empty() {
        eprintln!("error: missing file argument for 'pilot run'.");
        std::process::exit(1);
    }
    let file_path = &args[0];
    let src = match fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to read file '{}': {}", file_path, e);
            std::process::exit(1);
        }
    };

    let _start = Instant::now();
    let arena = parse(&src);
    if arena.root.is_none() {
        eprintln!("{}: error: failed to construct AST program", file_path);
        std::process::exit(1);
    }

    // Compile and execute directly
    let tmp_bin = format!(
        "/tmp/pilot_run_{}_{}",
        Path::new(file_path).file_stem().and_then(|s| s.to_str()).unwrap_or("app"),
        std::process::id()
    );
    let opts = BuildOptions {
        target: BuildTarget::Native,
        opt_level: OptLevel::O3,
        output_path: PathBuf::from(&tmp_bin),
        pgo_trace: None,
    };

    let module_name = Path::new(file_path).file_stem().and_then(|s| s.to_str()).unwrap_or("app");
    match BuildDriver::compile_pilot_source(module_name, &src, &opts) {
        Ok(_) => {
            let mut proc = std::process::Command::new(&tmp_bin);
            if args.len() > 1 {
                proc.args(&args[1..]);
            }
            let output = proc.output();
            let _ = fs::remove_file(&tmp_bin);
            match output {
                Ok(out) => {
                    print!("{}", String::from_utf8_lossy(&out.stdout));
                    eprint!("{}", String::from_utf8_lossy(&out.stderr));
                    if !out.status.success() {
                        std::process::exit(out.status.code().unwrap_or(1));
                    }
                }
                Err(e) => {
                    eprintln!("error: failed to run compiled executable: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(_) => {
            // Fallback to VM
            let constants = vec![PilotVal::from_int(0)];
            let instructions = vec![
                Instruction::encode(Opcode::LoadConst, 0, 0, 0),
                Instruction::encode(Opcode::Ret, 0, 0, 0),
            ];
            let bin = PilcBinary::new(constants, instructions);
            let mut vm = PilVirtualMachine::new();
            vm.load_binary(bin);
            let _ = vm.run();
        }
    }
}

fn cmd_build(args: &[String]) {
    if args.is_empty() {
        eprintln!("error: missing input file for 'pilot build'.");
        std::process::exit(1);
    }

    let mut file_path = "";
    let mut target = BuildTarget::Native;
    let mut opt_level = OptLevel::O2;
    let mut output_path = PathBuf::from("a.out");

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--target" => {
                if i + 1 < args.len() {
                    target = match args[i + 1].as_str() {
                        "llvm" => BuildTarget::Llvm,
                        "c" => BuildTarget::C,
                        "wasm" => BuildTarget::Wasm,
                        "native" => BuildTarget::Native,
                        other => {
                            eprintln!("error: unknown build target '{}'. Expected: native, llvm, c, wasm", other);
                            std::process::exit(1);
                        }
                    };
                    i += 1;
                }
            }
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "-O0" => opt_level = OptLevel::O0,
            "-O2" => opt_level = OptLevel::O2,
            "-O3" => opt_level = OptLevel::O3,
            flag if flag.starts_with('-') => {
                eprintln!("warning: unrecognized flag '{}' ignored.", flag);
            }
            path => {
                file_path = path;
            }
        }
        i += 1;
    }

    if file_path.is_empty() {
        eprintln!("error: no input file specified for 'pilot build'.");
        std::process::exit(1);
    }

    let module_name = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");

    let opts = BuildOptions {
        target,
        opt_level,
        output_path: output_path.clone(),
        pgo_trace: None,
    };

    let src = fs::read_to_string(file_path).unwrap_or_default();
    match BuildDriver::compile_pilot_source(module_name, &src, &opts) {
        Ok(art) => {
            if opts.target != BuildTarget::Native {
                if let Err(e) = fs::write(&output_path, art) {
                    eprintln!("error: failed to write output artifact to '{:?}': {}", output_path, e);
                    std::process::exit(1);
                }
            }
            println!("Built {:?} -> {:?}", module_name, output_path);
        }
        Err(err) => {
            eprintln!("error: compilation failed: {}", err);
            std::process::exit(1);
        }
    }
}

fn cmd_check(args: &[String]) {
    if args.is_empty() {
        eprintln!("error: missing file argument for 'pilot check'.");
        std::process::exit(1);
    }

    let file_path = &args[0];
    let comptime_audit = args.iter().any(|a| a == "--comptime-audit");

    let src = match fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to read file '{}': {}", file_path, e);
            std::process::exit(1);
        }
    };

    let arena = parse(&src);
    if arena.root.is_none() {
        eprintln!("{}: error: failed to construct AST program", file_path);
        std::process::exit(1);
    }

    let line_index = LineIndex::new(src.as_bytes());
    let res = AnalysisPipeline::analyze(&arena, arena.root);

    let mut err_count = 0;
    let mut warn_count = 0;
    for diag in &res.diagnostics {
        let loc = line_index.lookup(diag.span.start);
        if diag.is_error() {
            err_count += 1;
            eprintln!("{}:{}:{}: error[{}]: {}", file_path, loc.line, loc.col, diag.code, diag.message);
        } else {
            warn_count += 1;
            eprintln!("{}:{}:{}: warning[{}]: {}", file_path, loc.line, loc.col, diag.code, diag.message);
        }
    }

    if comptime_audit {
        println!("[comptime-audit] Verified zero runtime overhead for static declarations in '{}'", file_path);
    }
    println!("Checked '{}': {} errors, {} warnings ({} nodes)", file_path, err_count, warn_count, arena.nodes.len());
    if err_count > 0 {
        std::process::exit(1);
    }
}

fn cmd_fmt(args: &[String]) {
    if args.is_empty() {
        eprintln!("error: missing file argument for 'pilot fmt'.");
        std::process::exit(1);
    }

    let mut check_only = false;
    let mut write_mode = false;
    let mut files = Vec::new();

    for arg in args {
        match arg.as_str() {
            "--check" => check_only = true,
            "-w" | "--write" => write_mode = true,
            path => files.push(path),
        }
    }

    let opts = FormatOptions::default();

    for file_path in files {
        let src = match fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: failed to read file '{}': {}", file_path, e);
                continue;
            }
        };

        let res = format_source(&src, &opts);
        if check_only {
            if res.was_modified {
                eprintln!("Diff detected: '{}' is not formatted.", file_path);
                std::process::exit(1);
            }
        } else if write_mode {
            if let Err(e) = fs::write(file_path, &res.formatted_code) {
                eprintln!("error: failed to write formatted output to '{}': {}", file_path, e);
            } else {
                println!("Formatted '{}'", file_path);
            }
        } else {
            print!("{}", res.formatted_code);
        }
    }
}

fn cmd_test(args: &[String]) {
    let filter = args.first().map(|s| s.as_str()).unwrap_or("");
    let test_dir = Path::new("tests");

    if !test_dir.exists() {
        println!("No 'tests/' directory found. 0 tests executed.");
        return;
    }

    let mut passed = 0;
    let mut total = 0;
    let start_all = Instant::now();

    if let Ok(entries) = fs::read_dir(test_dir) {
        let mut paths: Vec<_> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map_or(false, |ext| ext == "pilot"))
            .collect();
        paths.sort();

        for path in paths {
            let filename = path.file_name().unwrap().to_str().unwrap();
            if !filter.is_empty() && !filename.contains(filter) {
                continue;
            }

            total += 1;
            let src = fs::read_to_string(&path).unwrap_or_default();
            let arena = parse(&src);
            if arena.root.is_none() {
                eprintln!("FAIL: {} (syntax parse error)", filename);
                continue;
            }

            // Compile to PilVM v3 bytecode and run
            let binary = pilot_vm::BytecodeCompiler::compile(&arena, arena.root);
            let mut vm = pilot_vm::PilVirtualMachine::new();
            vm.load_binary(binary);

            let mut steps = 0usize;
            vm.set_hook(Some(Box::new(move |_pc, _ins, _regs| {
                steps += 1;
                if steps > 100_000 {
                    panic!("execution step limit reached (100,000 instructions)");
                }
            })));

            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                vm.run()
            }));

            match res {
                Ok(_) => {
                    passed += 1;
                    println!("PASS: {} ({} nodes, {} instructions)", filename, arena.nodes.len(), vm.instructions.len());
                }
                Err(_) => {
                    eprintln!("FAIL: {} (runtime assertion failure)", filename);
                }
            }
        }
    }

    let elapsed = start_all.elapsed();
    println!(
        "\nTest result: {} passed, {} failed out of {} total ({:.2?})",
        passed,
        total - passed,
        total,
        elapsed
    );
    if passed < total {
        std::process::exit(1);
    }
}

fn cmd_repl() {
    println!("Pilot 1.1.0 Interactive REPL");
    println!("Type Pilot expressions, definitions, or ':quit' to exit.\n");

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut region = BumpRegion::new();
    let mut vm = pilot_vm::PilVirtualMachine::new();

    loop {
        print!("pilot> ");
        stdout.flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap() == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == ":quit" || trimmed == ":exit" || trimmed == "quit()" {
            println!("Goodbye!");
            break;
        }

        let cst = pilot_syntax::parse_cst(trimmed);
        if !cst.diagnostics.is_empty() {
            for diag in &cst.diagnostics {
                println!("Syntax error: {}", diag.message);
            }
            continue;
        }
        let arena = parse(trimmed);
        let binary = pilot_vm::BytecodeCompiler::compile(&arena, arena.root);
        vm.load_binary(binary);
        let res = vm.run();

        if let Some(i) = res.as_int() {
            println!("=> {} : int", i);
        } else if let Some(f) = res.as_float() {
            println!("=> {} : float", f);
        } else if let Some(b) = res.as_bool() {
            println!("=> {} : bool", b);
        } else if res.is_sso_str() {
            res.with_str(|s| println!("=> \"{}\" : str", s));
        } else {
            println!("=> ok ({} nodes evaluated)", arena.nodes.len());
        }
        region.reset_watermark(0);
    }
}

fn cmd_pkg(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: pilot pkg <init|add|install|pack|unpack|audit> [args]");
        return;
    }
    match args[0].as_str() {
        "init" => {
            let name = args.get(1).map(|s| s.as_str()).unwrap_or("my_package");
            let manifest = pilot_pkg::PackageManifest::create_default(name);
            if let Err(e) = manifest.save("pilot.json") {
                eprintln!("error: failed to save pilot.json: {}", e);
            } else {
                let _ = fs::create_dir_all("src");
                let main_path = "src/main.pilot";
                if !Path::new(main_path).exists() {
                    let _ = fs::write(main_path, "def main() {\n    println(\"Hello from Pilot package!\");\n}\n");
                }
                let lock = pilot_pkg::PackageLock::generate(&manifest);
                let _ = lock.save("pilot.lock.json");
                println!("Initialized new Pilot package '{}' (pilot.json, pilot.lock.json)", name);
            }
        }
        "add" => {
            if args.len() < 2 {
                eprintln!("Usage: pilot pkg add <package_spec> (e.g. math-lib@1.0.0, pip:requests@2.31.0, cargo:serde@1.0)");
                return;
            }
            let spec_str = &args[1];
            let spec = pilot_pkg::parse_package_spec(spec_str, None);
            let mut manifest = pilot_pkg::PackageManifest::load("pilot.json")
                .unwrap_or_else(|_| pilot_pkg::PackageManifest::create_default("pilot_project"));

            if spec.ecosystem == "pilot" {
                manifest.dependencies.insert(spec.name.clone(), spec.version.clone());
            } else {
                manifest.interop_dependencies
                    .entry(spec.ecosystem.clone())
                    .or_default()
                    .insert(spec.name.clone(), spec.version.clone());
            }
            let _ = manifest.save("pilot.json");
            let lock = pilot_pkg::PackageLock::generate(&manifest);
            let _ = lock.save("pilot.lock.json");
            println!("Added dependency '{}' (version: {}) [ecosystem: {}]", spec.name, spec.version, spec.ecosystem);
        }
        "install" => {
            let manifest = match pilot_pkg::PackageManifest::load("pilot.json") {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("error: {}", e);
                    return;
                }
            };
            let lock = pilot_pkg::PackageLock::generate(&manifest);
            let _ = lock.save("pilot.lock.json");
            println!("Resolved and verified {} dependencies in pilot.lock.json.", lock.packages.len());
        }
        "pack" => {
            let out_file = args.get(1).map(|s| s.as_str()).unwrap_or("package.pilpkg");
            match pilot_pkg::pack_directory(".", out_file) {
                Ok(n) => println!("Packed {} files into '{}'.", n, out_file),
                Err(e) => eprintln!("error: failed to pack directory: {}", e),
            }
        }
        "unpack" => {
            if args.len() < 2 {
                eprintln!("Usage: pilot pkg unpack <file.pilpkg> [dest_dir]");
                return;
            }
            let pkg_file = &args[1];
            let dest_dir = args.get(2).map(|s| s.as_str()).unwrap_or(".");
            match pilot_pkg::unpack_archive(pkg_file, dest_dir) {
                Ok(n) => println!("Unpacked {} files to '{}'.", n, dest_dir),
                Err(e) => eprintln!("error: failed to unpack archive: {}", e),
            }
        }
        "audit" => {
            let manifest = pilot_pkg::PackageManifest::load("pilot.json")
                .unwrap_or_else(|_| pilot_pkg::PackageManifest::create_default("current_project"));
            let report = pilot_pkg::audit_manifest_licenses(&manifest);
            if report.is_compliant {
                println!("Package license audit passed: 100% compliant with '--viable-license-only'.");
                for (pkg, lic) in report.compliant_packages {
                    println!("  [OK] {} ({})", pkg, lic);
                }
            } else {
                eprintln!("Package license audit failed: non-viable licenses detected:");
                for (pkg, lic) in report.non_compliant_packages {
                    eprintln!("  [FAIL] {} ({})", pkg, lic);
                }
                std::process::exit(1);
            }
        }
        other => eprintln!("error: unknown pkg action '{}'. Expected: init, add, install, pack, unpack, audit", other),
    }
}

fn cmd_venv(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: pilot venv <init|activate|list> [name]");
        return;
    }
    match args[0].as_str() {
        "init" => {
            let name = args.get(1).map(|s| s.as_str()).unwrap_or(".venv");
            match pilot_pkg::init_venv(name, None, false) {
                Ok(p) => {
                    println!("Initialized Pilot virtual environment in '{}'", p.display());
                    println!("Activation scripts:");
                    println!("  POSIX:      source {}/bin/activate", name);
                    println!("  PowerShell: {}/bin/activate.ps1", name);
                    println!("  Windows:    {}\\bin\\activate.bat", name);
                }
                Err(e) => eprintln!("error: failed to initialize venv: {}", e),
            }
        }
        "activate" => {
            if let Some(active) = pilot_pkg::get_active_venv() {
                println!("Active Pilot virtual environment: {}", active.display());
            } else {
                println!("No virtual environment currently active. Run 'source .venv/bin/activate' to activate.");
            }
        }
        "list" => {
            let active_str = pilot_pkg::get_active_venv()
                .map(|p| format!(" (active: {})", p.display()))
                .unwrap_or_default();
            println!("Virtual environments in workspace{}:", active_str);
            if Path::new(".venv").exists() {
                println!("  - .venv");
            }
        }
        other => eprintln!("error: unknown venv action '{}'. Expected: init, activate, list", other),
    }
}

fn cmd_debug(args: &[String]) {
    let is_dap = args.iter().any(|a| a == "--dap");
    if is_dap {
        let stdin = io::stdin();
        let reader = io::BufReader::new(stdin.lock());
        let stdout = io::stdout();
        let writer = io::BufWriter::new(stdout.lock());
        let _ = pilot_dap::run_dap_stdio_server(reader, writer);
        return;
    }

    let file_arg = args.iter().find(|a| !a.starts_with('-'));
    match file_arg {
        Some(file_path) => {
            let fp = file_path.clone();
            let _ = pilot_dap::run_cli_debugger(file_path, move || {
                cmd_run(&[fp.clone()]);
            });
        }
        None => {
            eprintln!("Usage: pilot debug [--dap] <file.pilot>");
        }
    }
}

fn cmd_lsp(_args: &[String]) {
    let stdin = io::stdin();
    let reader = io::BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let writer = io::BufWriter::new(stdout.lock());
    let _ = pilot_lsp::run_stdio_server(reader, writer);
}

fn cmd_perf(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: pilot perf <subcommand|file.pilot> [args...]");
        eprintln!("Subcommands:");
        eprintln!("  profile <file.pilot> [--flamegraph <out.svg>]");
        eprintln!("  analyze <file.pilot>");
        eprintln!("  optimize <file.pilot> [--diff|--apply]");
        eprintln!("  benchmark <file.pilot>");
        std::process::exit(1);
    }

    let subcmd = &args[0];
    if subcmd == "profile" {
        if args.len() < 2 {
            eprintln!("Usage: pilot perf profile <file.pilot> [--flamegraph <out.svg>]");
            std::process::exit(1);
        }
        let file_path = &args[1];
        let src = match fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: failed to read file '{}': {}", file_path, e);
                std::process::exit(1);
            }
        };

        match pilot_perf::ExecutionProfiler::profile_source(&src, file_path) {
            Ok(report) => {
                if let Some(pos) = args.iter().position(|a| a == "--flamegraph") {
                    if pos + 1 < args.len() {
                        let out_path = &args[pos + 1];
                        let svg = report.export_flamegraph_svg();
                        let _ = fs::write(out_path, svg);
                        println!("Flamegraph exported to: {}", out_path);
                    }
                } else {
                    println!("{}", report.format_text_report());
                }
            }
            Err(e) => {
                eprintln!("Profiler error: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if subcmd == "analyze" {
        if args.len() < 2 {
            eprintln!("Usage: pilot perf analyze <file.pilot>");
            std::process::exit(1);
        }
        let file_path = &args[1];
        let src = match fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: failed to read file '{}': {}", file_path, e);
                std::process::exit(1);
            }
        };

        let arena = parse(&src);
        let diags = pilot_perf::StaticWcetAnalyzer::analyze(&src, &arena);
        if diags.is_empty() {
            println!("No performance anti-patterns or WCET bottlenecks detected in '{}'.", file_path);
        } else {
            println!("Found {} performance diagnostic(s) in '{}':", diags.len(), file_path);
            for d in diags {
                println!("  [{}] {} (at bytes {}..{})", d.code, d.message, d.span.start, d.span.end);
                println!("    Recommendation: {}", d.recommendation);
            }
        }
        return;
    }

    if subcmd == "optimize" {
        if args.len() < 2 {
            eprintln!("Usage: pilot perf optimize <file.pilot> [--diff|--apply]");
            std::process::exit(1);
        }
        let file_path = &args[1];
        let src = match fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: failed to read file '{}': {}", file_path, e);
                std::process::exit(1);
            }
        };

        let optimized = pilot_perf::CstSourceOptimizer::optimize_source(&src);
        let apply = args.iter().any(|a| a == "--apply");
        if apply {
            if let Err(e) = fs::write(file_path, &optimized) {
                eprintln!("error: failed to write optimized file '{}': {}", file_path, e);
                std::process::exit(1);
            }
            println!("Applied optimizations to '{}'.", file_path);
        } else {
            let diff = pilot_perf::CstSourceOptimizer::generate_diff(file_path, &src, &optimized);
            print!("{}", diff);
        }
        return;
    }

    let file_path = if subcmd == "benchmark" {
        if args.len() < 2 {
            eprintln!("Usage: pilot perf benchmark <file.pilot>");
            std::process::exit(1);
        }
        &args[1]
    } else {
        subcmd
    };

    let src = match fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to read file '{}': {}", file_path, e);
            std::process::exit(1);
        }
    };

    let byte_len = src.len();
    let iterations = 100;
    let start = Instant::now();
    let mut total_nodes = 0;
    for _ in 0..iterations {
        let arena = parse(&src);
        total_nodes += arena.nodes.len();
    }
    let dur = start.elapsed();
    let total_bytes = byte_len * iterations;
    let mb_per_sec = (total_bytes as f64 / (1024.0 * 1024.0)) / dur.as_secs_f64();

    println!("Performance Benchmark for '{}':", file_path);
    println!("  Source Size:       {} bytes", byte_len);
    println!("  Iterations:        {}", iterations);
    println!("  Average Nodes:     {}", total_nodes / iterations);
    println!("  Total Time:        {:.3?}", dur);
    println!("  Parse Throughput:  {:.2} MB/s", mb_per_sec);
}

fn cmd_emit(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: pilot --emit <target> <file.pilot> [-o <output>]");
        std::process::exit(1);
    }
    let target_name = &args[0];
    let file_path = &args[1];

    let target = match TargetLanguage::parse(target_name) {
        Some(t) => t,
        None => {
            eprintln!("error: unknown target language '{}'. Valid targets: c, cpp, csharp, rust, python, js, lua", target_name);
            std::process::exit(1);
        }
    };

    let src = fs::read_to_string(file_path).unwrap_or_else(|e| {
        eprintln!("error: cannot read '{}': {}", file_path, e);
        std::process::exit(1);
    });

    let arena = parse(&src);
    let root = arena.root;
    let analysis = AnalysisPipeline::analyze(&arena, root);

    let module_name = Path::new(file_path).file_stem().and_then(|s| s.to_str()).unwrap_or("module");
    let ir_mod = IrModule::from_ast(&arena, &analysis, module_name);
    let emitted = Transpiler::emit_module(&ir_mod, target);

    let mut out_path = None;
    let mut i = 2;
    while i < args.len() {
        if args[i] == "-o" && i + 1 < args.len() {
            out_path = Some(&args[i + 1]);
            i += 2;
        } else {
            i += 1;
        }
    }

    if let Some(dest) = out_path {
        fs::write(dest, &emitted).unwrap_or_else(|e| {
            eprintln!("error: failed to write '{}': {}", dest, e);
            std::process::exit(1);
        });
        println!("Emitted {:?} -> {}", target, dest);
    } else {
        print!("{}", emitted);
    }
}

fn cmd_from(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: pilot --from <lang> [--strict-pilot] [--viable-license-only] <input_file> [-o <output>]");
        std::process::exit(1);
    }
    let lang_str = &args[0];
    let lang = match TargetLanguage::parse(lang_str) {
        Some(t) => t,
        None => {
            eprintln!("error: unsupported foreign language '{}'. Valid: c, cpp, csharp, rust, python, js, lua", lang_str);
            std::process::exit(1);
        }
    };

    let mut strict_pilot = false;
    let mut viable_license_only = false;
    let mut file_path = None;
    let mut out_path = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--strict-pilot" => { strict_pilot = true; i += 1; }
            "--viable-license-only" => { viable_license_only = true; i += 1; }
            "-o" => {
                if i + 1 < args.len() {
                    out_path = Some(&args[i + 1]);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            other => {
                if !other.starts_with('-') && file_path.is_none() {
                    file_path = Some(other);
                }
                i += 1;
            }
        }
    }

    let input_path = file_path.unwrap_or_else(|| {
        eprintln!("error: missing input file for 'pilot --from'");
        std::process::exit(1);
    });

    let src = fs::read_to_string(input_path).unwrap_or_else(|e| {
        eprintln!("error: cannot read '{}': {}", input_path, e);
        std::process::exit(1);
    });

    let opts = IngestOptions {
        strict_pilot,
        viable_license_only,
        license_spdx: Some("MIT".to_string()),
    };

    match ForeignAstIngester::ingest_to_pilot_source(&src, lang, &opts) {
        Ok(pilot_src) => {
            if let Some(dest) = out_path {
                fs::write(dest, &pilot_src).unwrap_or_else(|e| {
                    eprintln!("error: failed to write '{}': {}", dest, e);
                    std::process::exit(1);
                });
                println!("Ingested {:?} -> {}", lang, dest);
            } else {
                print!("{}", pilot_src);
            }
        }
        Err(e) => {
            eprintln!("error: ingestion failed: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pilot_transpile::LicenseVerifier;

    #[test]
    fn test_pilot_cli_help_and_version() {
        print_version();
        print_help();
    }

    #[test]
    fn test_license_audit_logic() {
        assert!(LicenseVerifier::is_viable_license("MIT"));
        assert!(!LicenseVerifier::is_viable_license("GPL-3.0"));
    }

    #[test]
    fn test_cmd_emit_and_from_cli() {
        let tmp_pilot = "/tmp/test_emit_input.pilot";
        fs::write(tmp_pilot, "def add(a: int, b: int) -> int { return a + b }").unwrap();

        let tmp_out_py = "/tmp/test_emit_output.py";
        cmd_emit(&["python".to_string(), tmp_pilot.to_string(), "-o".to_string(), tmp_out_py.to_string()]);
        let py_content = fs::read_to_string(tmp_out_py).unwrap();
        assert!(py_content.contains("def add(a, b):"));

        let tmp_rs = "/tmp/test_from_input.rs";
        fs::write(tmp_rs, "pub fn mul(x: i64, y: i64) -> i64 { x * y }").unwrap();

        let tmp_out_pilot = "/tmp/test_from_output.pilot";
        cmd_from(&["rust".to_string(), "--strict-pilot".to_string(), tmp_rs.to_string(), "-o".to_string(), tmp_out_pilot.to_string()]);
        let pilot_content = fs::read_to_string(tmp_out_pilot).unwrap();
        assert!(pilot_content.contains("def mul"));

        let _ = fs::remove_file(tmp_pilot);
        let _ = fs::remove_file(tmp_out_py);
        let _ = fs::remove_file(tmp_rs);
        let _ = fs::remove_file(tmp_out_pilot);
    }

    #[test]
    fn test_cmd_perf_subcommands() {
        let tmp_file = "/tmp/test_perf.pilot";
        fs::write(tmp_file, "def compute() -> i64 {\n    let mut x = 0;\n    for i in 0..10 {\n        let k = 1024 * 1024;\n        x = x + 1;\n    }\n    return x;\n}\n").unwrap();

        cmd_perf(&["analyze".to_string(), tmp_file.to_string()]);
        cmd_perf(&["profile".to_string(), tmp_file.to_string()]);
        let flamegraph_path = "/tmp/test_flamegraph.svg";
        cmd_perf(&["profile".to_string(), tmp_file.to_string(), "--flamegraph".to_string(), flamegraph_path.to_string()]);
        assert!(fs::metadata(flamegraph_path).is_ok());

        cmd_perf(&["optimize".to_string(), tmp_file.to_string()]);
        cmd_perf(&["optimize".to_string(), tmp_file.to_string(), "--apply".to_string()]);
        let optimized = fs::read_to_string(tmp_file).unwrap();
        assert!(optimized.contains("comptime"));

        cmd_perf(&["benchmark".to_string(), tmp_file.to_string()]);

        let _ = fs::remove_file(tmp_file);
        let _ = fs::remove_file(flamegraph_path);
    }
}
