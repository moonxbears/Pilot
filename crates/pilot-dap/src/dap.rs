//! Milestone 43.2: Debug Adapter Protocol (DAP) Protocol Server.

use std::io::{BufRead, Write};
use crate::debugger::{DebuggerState, PilotDebugger};

/// Standalone DAP protocol dispatcher.
pub struct DapServer {
    pub debugger: PilotDebugger,
    pub program_path: Option<String>,
    pub is_initialized: bool,
    pub stop_on_entry: bool,
    pub should_exit: bool,
    pub pending_events: Vec<String>,
}

impl Default for DapServer {
    fn default() -> Self {
        Self::new()
    }
}

impl DapServer {
    pub fn new() -> Self {
        Self {
            debugger: PilotDebugger::new(),
            program_path: None,
            is_initialized: false,
            stop_on_entry: false,
            should_exit: false,
            pending_events: Vec::new(),
        }
    }

    /// Handles an incoming DAP request string and returns the JSON-RPC response string.
    pub fn handle_message(&mut self, raw_json: &str) -> Option<String> {
        let trimmed = raw_json.trim();
        let command = extract_json_string(trimmed, "command")?;
        let seq = extract_json_number(trimmed, "seq").unwrap_or(1.0) as i64;

        match command.as_str() {
            "initialize" => {
                self.is_initialized = true;
                let body = r#"{
                    "supportsConfigurationDoneRequest": true,
                    "supportsConditionalBreakpoints": true,
                    "supportsHitConditionalBreakpoints": true,
                    "supportsLogPoints": true,
                    "supportsEvaluateForHovers": true,
                    "supportsSetVariable": true,
                    "supportsTerminateRequest": true
                }"#;
                let resp = format_dap_response(seq, "initialize", true, body);
                self.pending_events.push(format_dap_event("initialized", "{}"));
                Some(resp)
            }
            "launch" => {
                if let Some(prog) = extract_json_string(trimmed, "program") {
                    self.program_path = Some(prog.clone());
                    self.debugger.push_frame("main", &prog, 1, 1);
                }
                if let Some(stop) = extract_json_bool(trimmed, "stopOnEntry") {
                    self.stop_on_entry = stop;
                }
                let resp = format_dap_response(seq, "launch", true, "{}");
                if self.stop_on_entry {
                    self.pending_events.push(format_dap_event(
                        "stopped",
                        r#"{"reason":"entry","threadId":1,"allThreadsStopped":true}"#,
                    ));
                }
                Some(resp)
            }
            "setBreakpoints" => {
                let file = extract_json_string(trimmed, "path").unwrap_or_default();
                self.debugger.clear_breakpoints_for_file(&file);

                let mut lines = Vec::new();
                if let Some(bp_idx) = trimmed.find("\"breakpoints\"") {
                    let after_bp = &trimmed[bp_idx..];
                    if let Some(lbracket) = after_bp.find('[') {
                        if let Some(rbracket) = after_bp.find(']') {
                            let array_content = &after_bp[lbracket + 1..rbracket];
                            for obj in array_content.split('}') {
                                if let Some(l_num) = extract_json_number(obj, "line") {
                                    lines.push(l_num as u32);
                                }
                            }
                        }
                    }
                }

                if lines.is_empty() {
                    if let Some(lines_idx) = trimmed.find("\"lines\"") {
                        let after_l = &trimmed[lines_idx..];
                        if let Some(lbracket) = after_l.find('[') {
                            if let Some(rbracket) = after_l.find(']') {
                                for part in after_l[lbracket + 1..rbracket].split(',') {
                                    if let Ok(l) = part.trim().parse::<u32>() {
                                        lines.push(l);
                                    }
                                }
                            }
                        }
                    }
                }

                let mut verified_bps = Vec::new();
                for l in lines {
                    let bp = self.debugger.set_breakpoint(&file, l, None, None, None);
                    verified_bps.push(format!(
                        r#"{{"id":{},"verified":true,"line":{}}}"#,
                        bp.id, bp.line
                    ));
                }

                let body = format!(r#"{{"breakpoints":[{}]}}"#, verified_bps.join(","));
                Some(format_dap_response(seq, "setBreakpoints", true, &body))
            }
            "configurationDone" => {
                let resp = format_dap_response(seq, "configurationDone", true, "{}");
                Some(resp)
            }
            "threads" => {
                let body = r#"{"threads":[{"id":1,"name":"main"}]}"#;
                Some(format_dap_response(seq, "threads", true, body))
            }
            "stackTrace" => {
                let mut frames_json = Vec::new();
                for frame in self.debugger.call_stack.iter().rev() {
                    frames_json.push(format!(
                        r#"{{"id":{},"name":"{}","source":{{"path":"{}"}},"line":{},"column":{}}}"#,
                        frame.id, frame.name, frame.file, frame.line, frame.column
                    ));
                }
                let body = format!(
                    r#"{{"stackFrames":[{}],"totalFrames":{}}}"#,
                    frames_json.join(","),
                    frames_json.len()
                );
                Some(format_dap_response(seq, "stackTrace", true, &body))
            }
            "scopes" => {
                let body = r#"{"scopes":[{"name":"Locals","variablesReference":1,"expensive":false}]}"#;
                Some(format_dap_response(seq, "scopes", true, body))
            }
            "variables" => {
                let mut vars_json = Vec::new();
                if let Some(frame) = self.debugger.current_frame() {
                    for (k, v) in &frame.locals {
                        let ty = match v.tag() {
                            pilot_rt::value::TAG_INT => "i64",
                            pilot_rt::value::TAG_FLOAT => "f64",
                            pilot_rt::value::TAG_BOOL => "bool",
                            pilot_rt::value::TAG_SSO_STR | pilot_rt::value::TAG_HEAP_STR => "str",
                            _ => "any",
                        };
                        vars_json.push(format!(
                            r#"{{"name":"{}","value":"{}","type":"{}","variablesReference":0}}"#,
                            k, v.to_string_repr(), ty
                        ));
                    }
                }
                let body = format!(r#"{{"variables":[{}]}}"#, vars_json.join(","));
                Some(format_dap_response(seq, "variables", true, &body))
            }
            "continue" => {
                self.debugger.continue_exec();
                Some(format_dap_response(seq, "continue", true, r#"{"allThreadsContinued":true}"#))
            }
            "next" => {
                self.debugger.step_over();
                self.pending_events.push(format_dap_event(
                    "stopped",
                    r#"{"reason":"step","threadId":1,"allThreadsStopped":true}"#,
                ));
                Some(format_dap_response(seq, "next", true, "{}"))
            }
            "stepIn" => {
                self.debugger.step_into();
                self.pending_events.push(format_dap_event(
                    "stopped",
                    r#"{"reason":"step","threadId":1,"allThreadsStopped":true}"#,
                ));
                Some(format_dap_response(seq, "stepIn", true, "{}"))
            }
            "stepOut" => {
                self.debugger.step_out();
                self.pending_events.push(format_dap_event(
                    "stopped",
                    r#"{"reason":"step","threadId":1,"allThreadsStopped":true}"#,
                ));
                Some(format_dap_response(seq, "stepOut", true, "{}"))
            }
            "pause" => {
                self.debugger.pause();
                self.pending_events.push(format_dap_event(
                    "stopped",
                    r#"{"reason":"pause","threadId":1,"allThreadsStopped":true}"#,
                ));
                Some(format_dap_response(seq, "pause", true, "{}"))
            }
            "evaluate" => {
                let expr = extract_json_string(trimmed, "expression").unwrap_or_default();
                let frame_id = extract_json_number(trimmed, "frameId").map(|n| n as u32);
                match self.debugger.evaluate(&expr, frame_id) {
                    Ok((val, ty)) => {
                        let body = format!(
                            r#"{{"result":"{}","type":"{}","variablesReference":0}}"#,
                            val.to_string_repr(),
                            ty
                        );
                        Some(format_dap_response(seq, "evaluate", true, &body))
                    }
                    Err(e) => {
                        let body = format!(r#"{{"message":"{}"}}"#, e);
                        Some(format_dap_response(seq, "evaluate", false, &body))
                    }
                }
            }
            "disconnect" => {
                self.debugger.state = DebuggerState::Terminated;
                self.should_exit = true;
                Some(format_dap_response(seq, "disconnect", true, "{}"))
            }
            _ => {
                Some(format_dap_response(seq, &command, false, r#"{"message":"Command not supported"}"#))
            }
        }
    }
}

fn format_dap_response(seq: i64, command: &str, success: bool, body_json: &str) -> String {
    format!(
        r#"{{"seq":{},"type":"response","request_seq":{},"command":"{}","success":{},"body":{}}}"#,
        seq + 100,
        seq,
        command,
        if success { "true" } else { "false" },
        body_json
    )
}

fn format_dap_event(event_name: &str, body_json: &str) -> String {
    format!(
        r#"{{"seq":1,"type":"event","event":"{}","body":{}}}"#,
        event_name, body_json
    )
}

fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let key_idx = json.find(&pattern)?;
    let after = &json[key_idx + pattern.len()..];
    let colon_idx = after.find(':')?;
    let rest = after[colon_idx + 1..].trim();
    if rest.starts_with('"') {
        let end_quote = rest[1..].find('"')?;
        Some(rest[1..=end_quote].to_string())
    } else {
        None
    }
}

fn extract_json_number(json: &str, key: &str) -> Option<f64> {
    let pattern = format!("\"{}\"", key);
    let key_idx = json.find(&pattern)?;
    let after = &json[key_idx + pattern.len()..];
    let colon_idx = after.find(':')?;
    let rest = after[colon_idx + 1..].trim();
    let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '-' || *c == '.').collect();
    num_str.parse::<f64>().ok()
}

fn extract_json_bool(json: &str, key: &str) -> Option<bool> {
    let pattern = format!("\"{}\"", key);
    let key_idx = json.find(&pattern)?;
    let after = &json[key_idx + pattern.len()..];
    let colon_idx = after.find(':')?;
    let rest = after[colon_idx + 1..].trim();
    if rest.starts_with("true") {
        Some(true)
    } else if rest.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

/// Runs the DAP JSON-RPC stdio event loop.
pub fn run_dap_stdio_server<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> std::io::Result<()> {
    let mut server = DapServer::new();
    loop {
        if server.should_exit {
            break;
        }

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

            // Read headers until empty line
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

            if let Some(resp) = server.handle_message(&raw_json) {
                let out = format!("Content-Length: {}\r\n\r\n{}", resp.len(), resp);
                writer.write_all(out.as_bytes())?;
            }

            for event in std::mem::take(&mut server.pending_events) {
                let out = format!("Content-Length: {}\r\n\r\n{}", event.len(), event);
                writer.write_all(out.as_bytes())?;
            }

            writer.flush()?;
        }
    }
    Ok(())
}
