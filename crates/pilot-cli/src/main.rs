//! The `pilot` executable: runs the self-hosted CLI (`selfhost/cli/`) and
//! exits with the code it returns.
mod check;
mod host;
mod link;
mod run;
mod tool;

/// The sources of the self-hosted CLI, linked in this order.
const CLI: &[&str] = &[
    "stdlib/core/file.pilot",
    "stdlib/core/env.pilot",
    "stdlib/core/primitives/map.pilot",
    "stdlib/std/proc.pilot",
    "stdlib/std/text.pilot",
    "selfhost/diagnostics/model.pilot",
    "selfhost/diagnostics/renderer.pilot",
    "selfhost/cli/spec.pilot",
    "selfhost/cli/parser.pilot",
    "selfhost/cli/help_and_comp.pilot",
    "selfhost/cli/host.pilot",
    "selfhost/cli/report.pilot",
    "selfhost/cli/test_cmd.pilot",
    "selfhost/cli/commands.pilot",
    "selfhost/cli/main.pilot",
];

fn main() {
    // The seed's syntax, native bindings and primitives come from the stdlib
    // it runs with, never from copies compiled into the binary.
    pilot_parser::set_source_root(std::path::Path::new(&tool::toolchain_root()));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let line = run::pilot_str(&args.join("\x1f"));
    let code = tool::eval_cli(CLI, &format!("run_cli_entry(\"{line}\");"), host::cli_host_ops());
    std::process::exit(code);
}
