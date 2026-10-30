//! Milestone 22 & Milestone 43.2: Native Debug Adapter Protocol (DAP) Server & Interactive Debugger (`pilot-dap`).
//!
//! Enforces:
//! - Complete DAP JSON-RPC stdio protocol implementation (`run_dap_stdio_server`).
//! - Conditional, hit-count, and line breakpoints.
//! - Call stack unwinding, locals inspection, and variable expression evaluation.
//! - Interactive CLI debugger for terminal sessions.

pub mod json;
pub mod debugger;
pub mod dap;

pub use debugger::{DebuggerState, PilotBreakpoint, PilotDebugger, PilotStackFrame, StepMode};
pub use dap::{run_dap_stdio_server, DapServer};

use std::io::{self, BufRead, Write};

/// Runs an interactive terminal CLI debugging session on a target Pilot source file.
pub fn run_cli_debugger(file_path: &str, mut on_continue: impl FnMut()) -> io::Result<()> {
    let mut debugger = PilotDebugger::new();
    debugger.push_frame("main", file_path, 1, 1);

    println!("========================================================");
    println!(" Pilot Native Interactive Debugger v1.1.0");
    println!(" Attached to: {}", file_path);
    println!(" Type 'help' for command list, 'c' to run, 'q' to quit.");
    println!("========================================================");

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut line = String::new();

    loop {
        print!("pilot-dbg> ");
        io::stdout().flush()?;
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("").to_ascii_lowercase();

        match cmd.as_str() {
            "help" | "h" | "?" => {
                println!("Debugger Commands:");
                println!("  b <line> [if <cond>]  - Set a breakpoint at line");
                println!("  d <id>                - Delete a breakpoint by ID");
                println!("  info b                - List active breakpoints");
                println!("  c | continue          - Continue execution");
                println!("  s | step              - Step into next line");
                println!("  n | next              - Step over next line");
                println!("  o | out               - Step out of current function");
                println!("  p <expr>              - Print variable or expression value");
                println!("  bt | stack            - Print backtrace / call stack");
                println!("  q | quit              - Exit debugging session");
            }
            "b" | "break" => {
                if let Some(line_str) = parts.next() {
                    if let Ok(l) = line_str.parse::<u32>() {
                        let cond = if parts.next() == Some("if") {
                            Some(parts.collect::<Vec<_>>().join(" "))
                        } else {
                            None
                        };
                        let bp = debugger.set_breakpoint(file_path, l, cond.clone(), None, None);
                        if let Some(c) = cond {
                            println!("Breakpoint {} set at {}:{} (condition: {})", bp.id, file_path, l, c);
                        } else {
                            println!("Breakpoint {} set at {}:{}", bp.id, file_path, l);
                        }
                    } else {
                        println!("Invalid line number: {}", line_str);
                    }
                } else {
                    println!("Usage: b <line_number> [if <condition>]");
                }
            }
            "d" | "delete" => {
                if let Some(id_str) = parts.next() {
                    if let Ok(id) = id_str.parse::<u32>() {
                        if debugger.remove_breakpoint(id) {
                            println!("Breakpoint {} deleted.", id);
                        } else {
                            println!("Breakpoint {} not found.", id);
                        }
                    }
                }
            }
            "info" => {
                if parts.next() == Some("b") {
                    println!("Num  File                Line  Hits  Condition");
                    for bp in &debugger.breakpoints {
                        println!(
                            "{:<4} {:<19} {:<5} {:<5} {}",
                            bp.id,
                            bp.file,
                            bp.line,
                            bp.hit_count,
                            bp.condition.as_deref().unwrap_or("-")
                        );
                    }
                }
            }
            "c" | "continue" => {
                println!("Continuing execution...");
                debugger.continue_exec();
                on_continue();
                break;
            }
            "s" | "step" => {
                debugger.step_into();
                println!("Stepping into next call frame...");
                break;
            }
            "n" | "next" => {
                debugger.step_over();
                println!("Stepping over current line...");
                break;
            }
            "o" | "out" => {
                debugger.step_out();
                println!("Stepping out of current function...");
                break;
            }
            "p" | "print" => {
                let expr = parts.collect::<Vec<_>>().join(" ");
                if expr.is_empty() {
                    println!("Usage: p <variable_or_expression>");
                } else {
                    match debugger.evaluate(&expr, None) {
                        Ok((val, ty)) => println!("{} = ({}) {}", expr, ty, val.to_string_repr()),
                        Err(e) => println!("Error: {}", e),
                    }
                }
            }
            "bt" | "stack" => {
                println!("Call Stack:");
                for (idx, frame) in debugger.call_stack.iter().enumerate() {
                    println!("  #{} {}() at {}:{}", idx, frame.name, frame.file, frame.line);
                }
            }
            "q" | "quit" | "exit" => {
                println!("Debugger session terminated.");
                break;
            }
            other => println!("Unknown debugger command '{}'. Type 'help' for available commands.", other),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pilot_rt::value::PilotVal;

    #[test]
    fn test_debugger_breakpoints_and_evaluation() {
        let mut dbg = PilotDebugger::new();
        let bp1 = dbg.set_breakpoint("main.pilot", 10, None, None, None);
        assert_eq!(bp1.id, 1);
        assert_eq!(dbg.breakpoints.len(), 1);

        let hit = dbg.check_breakpoint("main.pilot", 10);
        assert!(hit.is_some());
        assert_eq!(dbg.breakpoints[0].hit_count, 1);

        // Frame and local evaluation
        dbg.push_frame("compute", "main.pilot", 10, 4);
        dbg.set_local("answer", PilotVal::from_int(42), "i64");

        let (val, ty) = dbg.evaluate("answer", None).unwrap();
        assert_eq!(val.as_int(), Some(42));
        assert_eq!(ty, "i64");

        // Literal evaluation
        let (lit_val, _) = dbg.evaluate("100", None).unwrap();
        assert_eq!(lit_val.as_int(), Some(100));

        assert!(dbg.remove_breakpoint(1));
        assert_eq!(dbg.breakpoints.len(), 0);
    }

    #[test]
    fn test_dap_server_initialize_and_launch_protocol() {
        let mut dap = DapServer::new();

        let init_msg = r#"{"seq":1,"type":"request","command":"initialize","arguments":{"clientID":"vscode"}}"#;
        let init_resp = dap.handle_message(init_msg).expect("Expected DAP initialize response");
        assert!(init_resp.contains("supportsConditionalBreakpoints"));
        assert!(dap.is_initialized);

        let launch_msg = r#"{"seq":2,"type":"request","command":"launch","arguments":{"program":"suite.pilot","stopOnEntry":true}}"#;
        let launch_resp = dap.handle_message(launch_msg).expect("Expected DAP launch response");
        assert!(launch_resp.contains("success"));
        assert_eq!(dap.program_path, Some("suite.pilot".to_string()));

        let threads_msg = r#"{"seq":3,"type":"request","command":"threads"}"#;
        let threads_resp = dap.handle_message(threads_msg).unwrap();
        assert!(threads_resp.contains("main"));

        let stack_msg = r#"{"seq":4,"type":"request","command":"stackTrace","arguments":{"threadId":1}}"#;
        let stack_resp = dap.handle_message(stack_msg).unwrap();
        assert!(stack_resp.contains("stackFrames"));
    }

    #[test]
    fn test_dap_full_stepping_and_variables_flow() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_debug_script.pilot");
        let script = "let x = 10\nlet y = 20\nlet z = x + y\n";
        std::fs::write(&test_file, script).unwrap();
        let file_path = test_file.to_str().unwrap().to_string();

        let mut dap = DapServer::new();
        dap.handle_message(r#"{"seq":1,"type":"request","command":"initialize","arguments":{}}"#).unwrap();

        // Set breakpoint on line 2 (let y = 20)
        let bp_req = format!(
            r#"{{"seq":2,"type":"request","command":"setBreakpoints","arguments":{{"source":{{"path":"{}"}},"breakpoints":[{{"line":2}}]}}}}"#,
            file_path
        );
        let bp_resp = dap.handle_message(&bp_req).unwrap();
        assert!(bp_resp.contains("\"verified\":true"));

        // Launch
        let launch_req = format!(
            r#"{{"seq":3,"type":"request","command":"launch","arguments":{{"program":"{}","stopOnEntry":false}}}}"#,
            file_path
        );
        dap.handle_message(&launch_req).unwrap();

        // ConfigurationDone starts execution
        dap.handle_message(r#"{"seq":4,"type":"request","command":"configurationDone"}"#).unwrap();

        // Check that stopped event was emitted for breakpoint
        let events = dap.drain_events();
        let stopped = events.iter().any(|e| e.contains("\"event\":\"stopped\"") && e.contains("\"breakpoint\""));
        assert!(stopped, "Expected stopped event on breakpoint");

        // Inspect call stack
        let stack_resp = dap.handle_message(r#"{"seq":5,"type":"request","command":"stackTrace","arguments":{"threadId":1}}"#).unwrap();
        assert!(stack_resp.contains("test_debug_script.pilot"));
        assert!(stack_resp.contains("\"line\":2"));

        // Inspect scopes
        let scopes_resp = dap.handle_message(r#"{"seq":6,"type":"request","command":"scopes","arguments":{"frameId":1}}"#).unwrap();
        assert!(scopes_resp.contains("Locals"));

        // Inspect variables (x should be 10)
        let vars_resp = dap.handle_message(r#"{"seq":7,"type":"request","command":"variables","arguments":{"variablesReference":1}}"#).unwrap();
        assert!(vars_resp.contains("\"name\":\"x\""));
        assert!(vars_resp.contains("\"value\":\"10\""));

        // Evaluate expression
        let eval_resp = dap.handle_message(r#"{"seq":8,"type":"request","command":"evaluate","arguments":{"expression":"x","frameId":1}}"#).unwrap();
        assert!(eval_resp.contains("\"result\":\"10\""));

        // Step over (next)
        dap.handle_message(r#"{"seq":9,"type":"request","command":"next","arguments":{"threadId":1}}"#).unwrap();
        let step_events = dap.drain_events();
        assert!(step_events.iter().any(|e| e.contains("\"event\":\"stopped\"")));

        // After stepping over line 2, line is 3, and y should be 20
        let vars_resp2 = dap.handle_message(r#"{"seq":10,"type":"request","command":"variables","arguments":{"variablesReference":1}}"#).unwrap();
        assert!(vars_resp2.contains("\"name\":\"y\""));
        assert!(vars_resp2.contains("\"value\":\"20\""));

        // Continue to completion
        dap.handle_message(r#"{"seq":11,"type":"request","command":"continue","arguments":{"threadId":1}}"#).unwrap();
        let final_events = dap.drain_events();
        assert!(final_events.iter().any(|e| e.contains("\"event\":\"terminated\"")));

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_function_debugging_and_call_stack() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_calc_suite.pilot");
        let script = "fn calculate(a: int, b: int) -> int {\n    let temp = a * 2\n    let result = temp + b\n    return result\n}\n\nlet x = 5\nlet y = 12\nlet ans = calculate(x, y)\n";
        std::fs::write(&test_file, script).unwrap();
        let file_path = test_file.to_str().unwrap().to_string();

        let mut dap = DapServer::new();
        dap.handle_message(r#"{"seq":1,"type":"request","command":"initialize","arguments":{}}"#).unwrap();

        // Set breakpoint on line 3 (inside calculate)
        let bp_req = format!(
            r#"{{"seq":2,"type":"request","command":"setBreakpoints","arguments":{{"source":{{"path":"{}"}},"breakpoints":[{{"line":3}}]}}}}"#,
            file_path
        );
        let bp_resp = dap.handle_message(&bp_req).unwrap();
        assert!(bp_resp.contains("\"verified\":true"));

        // Launch
        let launch_req = format!(
            r#"{{"seq":3,"type":"request","command":"launch","arguments":{{"program":"{}","stopOnEntry":false}}}}"#,
            file_path
        );
        dap.handle_message(&launch_req).unwrap();

        // ConfigurationDone
        dap.handle_message(r#"{"seq":4,"type":"request","command":"configurationDone"}"#).unwrap();

        // Stopped at line 3 inside calculate
        let events = dap.drain_events();
        assert!(events.iter().any(|e| e.contains("\"event\":\"stopped\"") && e.contains("\"breakpoint\"")));

        // Verify call stack: frame 2 is calculate at line 3, frame 1 is main at line 9
        let stack_resp = dap.handle_message(r#"{"seq":5,"type":"request","command":"stackTrace","arguments":{"threadId":1}}"#).unwrap();
        assert!(stack_resp.contains("\"name\":\"calculate\""));
        assert!(stack_resp.contains("\"line\":3"));

        // Verify locals in calculate
        let vars_resp = dap.handle_message(r#"{"seq":6,"type":"request","command":"variables","arguments":{"variablesReference":1000}}"#).unwrap();
        assert!(vars_resp.contains("\"name\":\"temp\""));
        assert!(vars_resp.contains("\"value\":\"10\""));

        // Step over
        dap.handle_message(r#"{"seq":7,"type":"request","command":"next","arguments":{"threadId":1}}"#).unwrap();
        let step_evs = dap.drain_events();
        assert!(step_evs.iter().any(|e| e.contains("\"event\":\"stopped\"") && e.contains("\"step\"")));

        // Verify result variable computed
        let vars_resp2 = dap.handle_message(r#"{"seq":8,"type":"request","command":"variables","arguments":{"variablesReference":1000}}"#).unwrap();
        assert!(vars_resp2.contains("\"name\":\"result\""));
        assert!(vars_resp2.contains("\"value\":\"22\""));

        // Continue to completion
        dap.handle_message(r#"{"seq":9,"type":"request","command":"continue","arguments":{"threadId":1}}"#).unwrap();
        let term_evs = dap.drain_events();
        assert!(term_evs.iter().any(|e| e.contains("\"event\":\"terminated\"")));

        let _ = std::fs::remove_file(test_file);
    }
}
