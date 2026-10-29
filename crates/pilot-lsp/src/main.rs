//! Standalone zero-GC native `pilot-lsp` binary.
//! Communicates with editors over standard input / standard output using JSON-RPC LSP protocol.

use std::io;
use pilot_lsp::run_stdio_server;

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let reader = io::BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let writer = io::BufWriter::new(stdout.lock());
    run_stdio_server(reader, writer)
}
