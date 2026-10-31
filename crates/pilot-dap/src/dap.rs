//! Milestone 43.2: Debug Adapter Protocol (DAP) Protocol Server.

use std::io::{BufRead, Write};
use crate::debugger::{DebuggerState, PilotDebugger};
use crate::json::JsonParser;

/// Standalone DAP protocol dispatcher.
pub struct DapServer {
    pub debugger: PilotDebugger,
    pub program_path: Option<String>,
    pub is_initialized: bool,
    pub stop_on_entry: bool,
    pub should_exit: bool,
    pub pending_events: Vec<String>,
    pub next_var_ref: u64,
    pub var_ref_cache: std::collections::HashMap<u64, Vec<(String, String, String, u64)>>,
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
            next_var_ref: 3000,
            var_ref_cache: std::collections::HashMap::new(),
        }
    }

    /// Handles an incoming DAP request string and returns the response string.
    pub fn drain_events(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_events)
    }

    pub fn handle_message(&mut self, raw_json: &str) -> Option<String> {
        let msg = match JsonParser::parse(raw_json) {
            Ok(v) => v,
            Err(_) => return None,
        };

        let command = msg.get("command").and_then(|c| c.as_str())?.to_string();
        let seq = msg.get("seq").and_then(|s| s.as_u64()).unwrap_or(1) as i64;
        let args = msg.get("arguments");

        match command.as_str() {
            "initialize" => {
                self.is_initialized = true;
                self.var_ref_cache.clear();
                self.next_var_ref = 3000;
                let body = r#"{
                    "supportsConfigurationDoneRequest": true,
                    "supportsConditionalBreakpoints": true,
                    "supportsHitConditionalBreakpoints": true,
                    "supportsLogPoints": false,
                    "supportsEvaluateForHovers": true,
                    "supportsSetVariable": false,
                    "supportsTerminateRequest": true
                }"#;
                let resp = format_dap_response(seq, "initialize", true, body);
                self.pending_events.push(format_dap_event("initialized", "{}"));
                Some(resp)
            }
            "launch" => {
                let prog = args.and_then(|a| a.get("program")).and_then(|p| p.as_str()).unwrap_or("");
                let stop = args.and_then(|a| a.get("stopOnEntry")).and_then(|s| s.as_bool()).unwrap_or(false);

                self.stop_on_entry = stop;
                self.var_ref_cache.clear();
                self.next_var_ref = 3000;
                if !prog.is_empty() {
                    self.program_path = Some(prog.to_string());
                    let _ = self.debugger.load_program(prog);
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
                let file = args.and_then(|a| a.get("source")).and_then(|s| s.get("path")).and_then(|p| p.as_str()).unwrap_or("");
                let mut lines = Vec::new();

                if let Some(bps) = args.and_then(|a| a.get("breakpoints")).and_then(|b| b.as_array()) {
                    for bp in bps {
                        if let Some(l) = bp.get("line").and_then(|n| n.as_u64()) {
                            let cond = bp.get("condition").and_then(|c| c.as_str()).map(|s| s.to_string());
                            lines.push((l as u32, cond));
                        }
                    }
                } else if let Some(line_nums) = args.and_then(|a| a.get("lines")).and_then(|l| l.as_array()) {
                    for l in line_nums {
                        if let Some(num) = l.as_u64() {
                            lines.push((num as u32, None));
                        }
                    }
                }

                self.debugger.clear_breakpoints_for_file(file);

                let mut verified = Vec::new();
                for (line, cond) in lines {
                    let bp = self.debugger.set_breakpoint(file, line, cond, None, None);
                    verified.push(format!(
                        r#"{{"id":{},"verified":true,"line":{}}}"#,
                        bp.id, bp.line
                    ));
                }

                let body = format!(r#"{{"breakpoints":[{}]}}"#, verified.join(","));
                Some(format_dap_response(seq, "setBreakpoints", true, &body))
            }
            "configurationDone" => {
                let resp = format_dap_response(seq, "configurationDone", true, "{}");
                if !self.stop_on_entry && self.debugger.state == DebuggerState::Running {
                    self.debugger.continue_exec();
                    self.drain_outputs();
                    self.check_execution_state();
                }
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
                let frame_id = args.and_then(|a| a.get("frameId")).and_then(|f| f.as_u64()).unwrap_or(0);
                let current_id = self.debugger.call_stack.last().map(|f| f.id as u64).unwrap_or(1);
                let locals_ref = if frame_id == 0 || frame_id == current_id {
                    1000
                } else {
                    10000 + frame_id
                };
                let globals_ref = 2000;
                let body = format!(
                    r#"{{"scopes":[{{"name":"Locals","variablesReference":{},"expensive":false}},{{"name":"Globals","variablesReference":{},"expensive":false}}]}}"#,
                    locals_ref, globals_ref
                );
                Some(format_dap_response(seq, "scopes", true, &body))
            }
            "variables" => {
                let var_ref = args.and_then(|a| a.get("variablesReference")).and_then(|v| v.as_u64()).unwrap_or(0);
                let mut vars = Vec::new();

                if let Some(children) = self.var_ref_cache.get(&var_ref).cloned() {
                    for (c_name, c_val, c_ty, child_ref) in children {
                        vars.push(format!(
                            r#"{{"name":{},"value":{},"type":"{}","variablesReference":{}}}"#,
                            escape_json_str(&c_name),
                            escape_json_str(&c_val),
                            c_ty,
                            child_ref
                        ));
                    }
                } else if var_ref == 2 || var_ref == 2000 {
                    // Globals
                    let raw_globals: Vec<(String, String, String)> = {
                        let mut g: Vec<_> = self.debugger.globals.iter().map(|(k, v)| {
                            let ty = self.debugger.global_types.get(k).cloned().unwrap_or_else(|| "any".to_string());
                            (k.clone(), v.to_string_repr(), ty)
                        }).collect();
                        g.sort_by(|a, b| a.0.cmp(&b.0));
                        g
                    };
                    for (k, val_str, ty) in raw_globals {
                        let child_ref = self.resolve_child_ref(&k, &val_str);
                        vars.push(format!(
                            r#"{{"name":{},"value":{},"type":"{}","variablesReference":{}}}"#,
                            escape_json_str(&k),
                            escape_json_str(&val_str),
                            ty,
                            child_ref
                        ));
                    }
                } else {
                    // Locals
                    let raw_locals: Vec<(String, String, String)> = {
                        let frame = if var_ref >= 10000 {
                            let target_fid = (var_ref - 10000) as u32;
                            self.debugger.call_stack.iter().find(|f| f.id == target_fid)
                        } else {
                            self.debugger.call_stack.last()
                        };

                        if let Some(frame) = frame {
                            let mut l: Vec<_> = frame.locals.iter().map(|(k, v)| {
                                let ty = frame.local_types.get(k)
                                    .or_else(|| self.debugger.global_types.get(k))
                                    .cloned()
                                    .unwrap_or_else(|| {
                                        if v.is_int() { "int".to_string() }
                                        else if v.is_float() { "float".to_string() }
                                        else if v.is_bool() { "bool".to_string() }
                                        else { "any".to_string() }
                                    });
                                (k.clone(), v.to_string_repr(), ty)
                            }).collect();
                            l.sort_by(|a, b| a.0.cmp(&b.0));
                            l
                        } else {
                            Vec::new()
                        }
                    };

                    for (k, val_str, ty) in raw_locals {
                        let child_ref = self.resolve_child_ref(&k, &val_str);
                        vars.push(format!(
                            r#"{{"name":{},"value":{},"type":"{}","variablesReference":{}}}"#,
                            escape_json_str(&k),
                            escape_json_str(&val_str),
                            ty,
                            child_ref
                        ));
                    }
                }

                let body = format!(r#"{{"variables":[{}]}}"#, vars.join(","));
                Some(format_dap_response(seq, "variables", true, &body))
            }
            "continue" => {
                self.var_ref_cache.clear();
                self.next_var_ref = 3000;
                self.debugger.continue_exec();
                self.drain_outputs();
                let resp = format_dap_response(seq, "continue", true, r#"{"allThreadsContinued":true}"#);
                self.check_execution_state();
                Some(resp)
            }
            "next" => {
                self.var_ref_cache.clear();
                self.next_var_ref = 3000;
                self.debugger.step_over();
                self.drain_outputs();
                let resp = format_dap_response(seq, "next", true, "{}");
                self.check_execution_state();
                Some(resp)
            }
            "stepIn" => {
                self.var_ref_cache.clear();
                self.next_var_ref = 3000;
                self.debugger.step_into();
                self.drain_outputs();
                let resp = format_dap_response(seq, "stepIn", true, "{}");
                self.check_execution_state();
                Some(resp)
            }
            "stepOut" => {
                self.var_ref_cache.clear();
                self.next_var_ref = 3000;
                self.debugger.step_out();
                self.drain_outputs();
                let resp = format_dap_response(seq, "stepOut", true, "{}");
                self.check_execution_state();
                Some(resp)
            }
            "pause" => {
                self.debugger.pause();
                let resp = format_dap_response(seq, "pause", true, "{}");
                self.pending_events.push(format_dap_event(
                    "stopped",
                    r#"{"reason":"pause","threadId":1,"allThreadsStopped":true}"#,
                ));
                Some(resp)
            }
            "evaluate" => {
                let expr = args.and_then(|a| a.get("expression")).and_then(|e| e.as_str()).unwrap_or("");
                let frame_id = args.and_then(|a| a.get("frameId")).and_then(|f| f.as_u64()).map(|n| n as u32);
                match self.debugger.evaluate(expr, frame_id) {
                    Ok((val, ty)) => {
                        let body = format!(
                            r#"{{"result":{},"type":"{}","variablesReference":0}}"#,
                            escape_json_str(&val.to_string_repr()),
                            ty
                        );
                        Some(format_dap_response(seq, "evaluate", true, &body))
                    }
                    Err(e) => {
                        let body = format!(r#"{{"message":{}}}"#, escape_json_str(&e));
                        Some(format_dap_response(seq, "evaluate", false, &body))
                    }
                }
            }
            "disconnect" | "terminate" => {
                self.should_exit = true;
                let resp = format_dap_response(seq, &command, true, "{}");
                self.pending_events.push(format_dap_event("terminated", "{}"));
                Some(resp)
            }
            _ => {
                Some(format_dap_response(seq, &command, false, r#"{"message":"Command not supported"}"#))
            }
        }
    }

    fn drain_outputs(&mut self) {
        for out in std::mem::take(&mut self.debugger.pending_outputs) {
            let ev = format!(
                r#"{{"category":"stdout","output":{}}}"#,
                escape_json_str(&out)
            );
            self.pending_events.push(format_dap_event("output", &ev));
        }
    }

    fn resolve_child_ref(&mut self, name: &str, val_str: &str) -> u64 {
        if let Some(compound) = self.debugger.compound_values.get(val_str).or_else(|| self.debugger.compound_values.get(name)).cloned() {
            self.cache_compound_children(&compound, 0)
        } else {
            0
        }
    }

    fn cache_compound_children(&mut self, compound: &crate::debugger::DebugCompoundVal, depth: usize) -> u64 {
        if depth > 10 {
            return 0;
        }
        let children = compound.to_children();
        if children.is_empty() {
            return 0;
        }
        let ref_id = self.next_var_ref;
        self.next_var_ref += 1;

        let mut cached_children = Vec::new();
        for (c_name, c_val, c_ty) in children {
            let child_ref = if let Some(child_compound) = self.debugger.compound_values.get(&c_val).or_else(|| self.debugger.compound_values.get(&c_name)).cloned() {
                self.cache_compound_children(&child_compound, depth + 1)
            } else {
                0
            };
            cached_children.push((c_name, c_val, c_ty, child_ref));
        }

        self.var_ref_cache.insert(ref_id, cached_children);
        ref_id
    }

    fn check_execution_state(&mut self) {
        match &self.debugger.state {
            DebuggerState::Paused(reason) => {
                let ev = format!(
                    r#"{{"reason":"{}","threadId":1,"allThreadsStopped":true}}"#,
                    reason
                );
                self.pending_events.push(format_dap_event("stopped", &ev));
            }
            DebuggerState::Terminated => {
                self.pending_events.push(format_dap_event("exited", r#"{"exitCode":0}"#));
                self.pending_events.push(format_dap_event("terminated", "{}"));
                self.should_exit = true;
            }
            DebuggerState::Running => {}
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

fn escape_json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
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
