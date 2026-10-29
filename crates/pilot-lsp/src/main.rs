//! Standalone zero-GC native `pilot-lsp` binary.
//! Communicates with editors over standard input / standard output using JSON-RPC LSP protocol.

use std::io::{self, BufRead, Read, Write};
use pilot_lsp::LspServer;

fn main() -> io::Result<()> {
    let mut server = LspServer::new();
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());

    loop {
        if server.shutdown_requested {
            break;
        }

        // Read Content-Length header
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break; // EOF
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            let content_len: usize = match rest.trim().parse() {
                Ok(len) => len,
                Err(_) => continue,
            };

            // Read headers until empty line "\r\n"
            loop {
                line.clear();
                let n = reader.read_line(&mut line)?;
                if n == 0 || line.trim().is_empty() {
                    break;
                }
            }

            // Read exact payload bytes
            let mut payload = vec![0u8; content_len];
            reader.read_exact(&mut payload)?;
            let raw_json = String::from_utf8_lossy(&payload);

            // Handle message
            if let Some(resp) = server.handle_message(&raw_json) {
                let resp_bytes = resp.as_bytes();
                write!(writer, "Content-Length: {}\r\n\r\n", resp_bytes.len())?;
                writer.write_all(resp_bytes)?;
                writer.flush()?;
            }
        }
    }

    Ok(())
}
