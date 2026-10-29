//! Standalone `pilot-dap` server binary.
//! Runs the Debug Adapter Protocol on stdio for IDE integrations (VS Code, etc.).

use std::io;
use pilot_dap::run_dap_stdio_server;

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let reader = io::BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let writer = io::BufWriter::new(stdout.lock());
    run_dap_stdio_server(reader, writer)
}
