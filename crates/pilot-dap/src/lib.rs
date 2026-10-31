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

    #[test]
    fn test_mytest_pilot_debugging() {
        let mut dap = DapServer::new();
        dap.handle_message(r#"{"seq":1,"type":"request","command":"initialize","arguments":{}}"#).unwrap();

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let test_src = "for x in 15..20:\n    println(x)\n\nfor j in [1, 2, 3, 4, 5]{\n    println(j)\n}\n\nvar myvar : int = 0\nwhile (true) {\n    if (myvar >= 20){\n        break;\n    }\n    println(f\"true - {myvar}\")\n    myvar += 1\n}\n";
        let path = format!("{}/../../examples/dap_myvar_test.pilot", manifest_dir);
        let _ = std::fs::write(&path, test_src);
        let path = std::fs::canonicalize(&path).unwrap().to_str().unwrap().to_string();

        // Set breakpoint on line 13: println(f"true - {myvar}")
        let bp_req = format!(
            r#"{{"seq":2,"type":"request","command":"setBreakpoints","arguments":{{"source":{{"path":"{}"}},"breakpoints":[{{"line":13}}]}}}}"#,
            path
        );
        let bp_resp = dap.handle_message(&bp_req).unwrap();
        assert!(bp_resp.contains("\"verified\":true"));

        let launch_req = format!(
            r#"{{"seq":3,"type":"request","command":"launch","arguments":{{"program":"{}","stopOnEntry":false}}}}"#,
            path
        );
        dap.handle_message(&launch_req).unwrap();

        if let Some(frame) = dap.debugger.current_frame() {
            println!("Initial frame stmts count: {}", frame.stmts.len());
            for (nid, l, c) in &frame.stmts {
                println!("  stmt nid: {:?}, line: {}, col: {}", nid, l, c);
            }
        }

        dap.handle_message(r#"{"seq":4,"type":"request","command":"configurationDone"}"#).unwrap();

        let events = dap.drain_events();
        println!("Events after configurationDone: {:?}", events);
        let stopped = events.iter().any(|e| e.contains("\"event\":\"stopped\"") && e.contains("\"breakpoint\""));
        assert!(stopped, "Expected stopped event on breakpoint at line 13. Events: {:?}", events);

        // Inspect variables in Locals
        let vars_resp = dap.handle_message(r#"{"seq":5,"type":"request","command":"variables","arguments":{"variablesReference":1000}}"#).unwrap();
        
        // Assert that myvar is NOT null and has actual value "0" and type "int"
        assert!(vars_resp.contains("\"name\":\"myvar\""), "Expected myvar in variables: {}", vars_resp);
        assert!(vars_resp.contains("\"value\":\"0\""), "Expected myvar value to be '0', got: {}", vars_resp);
        assert!(vars_resp.contains("\"type\":\"int\""), "Expected myvar type to be 'int', got: {}", vars_resp);

        // Step over line 13 (println)
        dap.handle_message(r#"{"seq":6,"type":"request","command":"next","arguments":{"threadId":1}}"#).unwrap();
        dap.drain_events();

        // Step over line 14 (myvar += 1)
        dap.handle_message(r#"{"seq":7,"type":"request","command":"next","arguments":{"threadId":1}}"#).unwrap();
        dap.drain_events();

        // Check variables again: myvar should now be 1
        let vars_resp2 = dap.handle_message(r#"{"seq":8,"type":"request","command":"variables","arguments":{"variablesReference":1000}}"#).unwrap();
        assert!(vars_resp2.contains("\"value\":\"1\""), "Expected myvar to be 1 after += 1, got: {}", vars_resp2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_compound_data_types_and_destructuring() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_compounds.pilot");
        let script = "record Point(x: float, y: float)\n\nlet pt = Point(1.5, 2.5)\nlet numbers = [10, 20, 30]\nlet mytup = (100, \"pilot\")\nlet myunit = 250ms\nlet mykw = :ready\nlet (px, py) = (11, 22)\nlet [head, ..tail] = [1, 2, 3, 4]\nlet myvar = 42\nprintln(f\"done - {myvar}\")\n";
        std::fs::write(&test_file, script).unwrap();
        let file_path = test_file.to_str().unwrap().to_string();

        let mut dap = DapServer::new();
        dap.handle_message(r#"{"seq":1,"type":"request","command":"initialize","arguments":{}}"#).unwrap();

        // Set breakpoint on line 11: println(f"done - {myvar}")
        let bp_req = format!(
            r#"{{"seq":2,"type":"request","command":"setBreakpoints","arguments":{{"source":{{"path":"{}"}},"breakpoints":[{{"line":11}}]}}}}"#,
            file_path
        );
        dap.handle_message(&bp_req).unwrap();

        let launch_req = format!(
            r#"{{"seq":3,"type":"request","command":"launch","arguments":{{"program":"{}","stopOnEntry":false}}}}"#,
            file_path
        );
        dap.handle_message(&launch_req).unwrap();
        dap.handle_message(r#"{"seq":4,"type":"request","command":"configurationDone"}"#).unwrap();

        let events = dap.drain_events();
        let stopped = events.iter().any(|e| e.contains("\"event\":\"stopped\"") && e.contains("\"breakpoint\""));
        assert!(stopped, "Expected stopped event on breakpoint at line 11. Events: {:?}", events);

        // Request locals
        let vars_resp = dap.handle_message(r#"{"seq":5,"type":"request","command":"variables","arguments":{"variablesReference":1000}}"#).unwrap();

        // 1. Verify myvar
        assert!(vars_resp.contains("\"name\":\"myvar\""));
        assert!(vars_resp.contains("\"value\":\"42\""));
        assert!(vars_resp.contains("\"type\":\"int\""));

        // 2. Verify record pt
        assert!(vars_resp.contains("\"name\":\"pt\""));
        assert!(vars_resp.contains("\"type\":\"Point\""));
        assert!(vars_resp.contains("Point(x: 1.5, y: 2.5)"));

        // 3. Verify list numbers
        assert!(vars_resp.contains("\"name\":\"numbers\""));
        assert!(vars_resp.contains("[10, 20, 30]"));

        // 4. Verify tuple mytup
        assert!(vars_resp.contains("\"name\":\"mytup\""));
        assert!(vars_resp.contains("(100, pilot)"));

        // 5. Verify unit literal
        assert!(vars_resp.contains("\"name\":\"myunit\""));
        assert!(vars_resp.contains("\"value\":\"250ms\""));
        assert!(vars_resp.contains("\"type\":\"Duration\""));

        // 6. Verify keyword literal
        assert!(vars_resp.contains("\"name\":\"mykw\""));
        assert!(vars_resp.contains("\"value\":\":ready\""));
        assert!(vars_resp.contains("\"type\":\"keyword\""));

        // 7. Verify destructuring (px, py)
        assert!(vars_resp.contains("\"name\":\"px\""));
        assert!(vars_resp.contains("\"value\":\"11\""));
        assert!(vars_resp.contains("\"name\":\"py\""));
        assert!(vars_resp.contains("\"value\":\"22\""));

        // 8. Verify slice destructuring [head, ..tail]
        assert!(vars_resp.contains("\"name\":\"head\""));
        assert!(vars_resp.contains("\"value\":\"1\""));
        assert!(vars_resp.contains("\"name\":\"tail\""));
        assert!(vars_resp.contains("[2, 3, 4]"));

        // 9. Verify expand child variables of pt (Point)
        let pt_children_resp = dap.handle_message(r#"{"seq":6,"type":"request","command":"variables","arguments":{"variablesReference":3002}}"#).unwrap();
        assert!(pt_children_resp.contains("\"name\":\"x\""));
        assert!(pt_children_resp.contains("\"value\":\"1.5\""));
        assert!(pt_children_resp.contains("\"name\":\"y\""));
        assert!(pt_children_resp.contains("\"value\":\"2.5\""));

        // 10. Verify expand child variables of numbers (List)
        let num_children_resp = dap.handle_message(r#"{"seq":7,"type":"request","command":"variables","arguments":{"variablesReference":3001}}"#).unwrap();
        assert!(num_children_resp.contains("\"name\":\"[0]\""));
        assert!(num_children_resp.contains("\"value\":\"10\""));
        assert!(num_children_resp.contains("\"name\":\"[1]\""));
        assert!(num_children_resp.contains("\"value\":\"20\""));
        assert!(num_children_resp.contains("\"name\":\"[2]\""));
        assert!(num_children_resp.contains("\"value\":\"30\""));

        // 11. Verify evaluate expressions in debugger
        let eval_pt_x = dap.handle_message(r#"{"seq":8,"type":"request","command":"evaluate","arguments":{"expression":"pt.x","frameId":1}}"#).unwrap();
        assert!(eval_pt_x.contains("\"result\":\"1.5\""));

        let eval_list = dap.handle_message(r#"{"seq":9,"type":"request","command":"evaluate","arguments":{"expression":"numbers[1]","frameId":1}}"#).unwrap();
        assert!(eval_list.contains("\"result\":\"20\""));

        let eval_comp = dap.handle_message(r#"{"seq":10,"type":"request","command":"evaluate","arguments":{"expression":"px + py","frameId":1}}"#).unwrap();
        assert!(eval_comp.contains("\"result\":\"33\""));

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    #[ignore = "Showcase 20 assertion failure repro from request 7"]
    fn test_showcase_20_reproduce() {
        let mut dap = DapServer::new();
        dap.handle_message(r#"{"seq":1,"type":"request","command":"initialize","arguments":{}}"#).unwrap();
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = format!("{}/../../examples/showcase/20_dynamic_for_expressions_and_c_abi.pilot", manifest_dir);
        let path = std::fs::canonicalize(&path).unwrap().to_str().unwrap().to_string();

        dap.handle_message(&format!(
            r#"{{"seq":2,"type":"request","command":"launch","arguments":{{"program":"{}","stopOnEntry":false}}}}"#,
            path
        )).unwrap();
        dap.handle_message(r#"{"seq":3,"type":"request","command":"configurationDone"}"#).unwrap();

        let events = dap.drain_events();
        for ev in &events {
            println!("EV: {}", ev);
            assert!(!ev.contains("Assertion Error"), "Unexpected assertion failure in showcase 20: {}", ev);
        }

        let combined_output = events.join("\n");
        assert!(combined_output.contains("Cubes (1..5): [1, 8, 27, 64, 125]"));
        assert!(combined_output.contains("Pivoted (0..2 -> 8..10): [0, 1, 2, 8, 9, 10]"));
        assert!(combined_output.contains("Resumable LIFO Worklist: [1, 2, 100, 101, 3, 4]"));
        assert!(combined_output.contains("Port 8080 index: 2, Port 3000 fallback: -1"));
        assert!(combined_output.contains("24-byte C-ABI SliceBuffer: [10, 20, 30, 40, 50, 60], managed_lists=0"));
        assert!(combined_output.contains("Shared view 1 (@0x7fff1000): [115.5, 40.0, 33.25, 40.0, 50.0, 60.0]"));
        assert!(combined_output.contains("Shared view 2 (@0x7fff2000): [115.5, 40.0, 33.25, 40.0, 50.0, 60.0]"));
    }

    #[test]
    fn test_showcase_21_dap_inferred_variable_types() {
        let mut dap = DapServer::new();
        dap.handle_message(r#"{"seq":1,"type":"request","command":"initialize","arguments":{}}"#).unwrap();
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = format!("{}/../../examples/showcase/21_advanced_type_inference.pilot", manifest_dir);
        let path = std::fs::canonicalize(&path).unwrap().to_str().unwrap().to_string();

        dap.handle_message(&format!(
            r#"{{"seq":2,"type":"request","command":"setBreakpoints","arguments":{{"source":{{"path":"{}"}},"breakpoints":[{{"line":15}}]}}}}"#,
            path
        )).unwrap();

        dap.handle_message(&format!(
            r#"{{"seq":3,"type":"request","command":"launch","arguments":{{"program":"{}","stopOnEntry":false}}}}"#,
            path
        )).unwrap();
        dap.handle_message(r#"{"seq":4,"type":"request","command":"configurationDone"}"#).unwrap();

        // Execution should hit the breakpoint at line 15
        assert!(matches!(dap.debugger.state, DebuggerState::Paused(_)));

        // Get threads -> stackTrace -> scopes -> variables
        let stack_resp = dap.handle_message(r#"{"seq":5,"type":"request","command":"stackTrace","arguments":{"threadId":1}}"#).unwrap();
        assert!(stack_resp.contains("\"id\":1"));

        let scopes_resp = dap.handle_message(r#"{"seq":6,"type":"request","command":"scopes","arguments":{"frameId":1}}"#).unwrap();
        assert!(scopes_resp.contains("\"variablesReference\":1000"));

        let vars_resp = dap.handle_message(r#"{"seq":7,"type":"request","command":"variables","arguments":{"variablesReference":1000}}"#).unwrap();
        println!("Vars response at line 15: {}", vars_resp);

        // Check frame_area: value 2073600 (1920 * 1080), type "int"
        assert!(vars_resp.contains(r#"{"name":"frame_area","value":"2073600","type":"int","variablesReference":0}"#), "Expected frame_area to be int: {}", vars_resp);

        // Check canvas_area: value 100 (12.5 * 8.0), type "float"
        assert!(vars_resp.contains(r#"{"name":"canvas_area","value":"100","type":"float","variablesReference":0}"#), "Expected canvas_area to be float: {}", vars_resp);
    }

    #[test]
    fn test_heterogeneous_collection_dap() {
        let mut dap = DapServer::new();
        dap.handle_message(r#"{"seq":1,"type":"request","command":"initialize","arguments":{}}"#).unwrap();
        let test_file = "/tmp/test_heterogeneous_dap.pilot";
        std::fs::write(test_file, "let mylist = [\"apples\", 5, 2];\nlet x = 1;\n").unwrap();

        dap.handle_message(&format!(
            r#"{{"seq":2,"type":"request","command":"setBreakpoints","arguments":{{"source":{{"path":"{}"}},"breakpoints":[{{"line":2}}]}}}}"#,
            test_file
        )).unwrap();

        dap.handle_message(&format!(
            r#"{{"seq":3,"type":"request","command":"launch","arguments":{{"program":"{}","stopOnEntry":false}}}}"#,
            test_file
        )).unwrap();
        dap.handle_message(r#"{"seq":4,"type":"request","command":"configurationDone"}"#).unwrap();

        assert!(matches!(dap.debugger.state, DebuggerState::Paused(_)));

        let vars_resp = dap.handle_message(r#"{"seq":7,"type":"request","command":"variables","arguments":{"variablesReference":1000}}"#).unwrap();
        println!("DAP vars response: {}", vars_resp);

        assert!(vars_resp.contains("\"name\":\"mylist\""), "Expected mylist variable: {}", vars_resp);
        assert!(vars_resp.contains("List[str | int]") || vars_resp.contains("List[i64 | str]"),
            "Expected constrained union type for mylist: {}", vars_resp);

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_indexer_dap() {
        let mut dap = DapServer::new();
        dap.handle_message(r#"{"seq":1,"type":"request","command":"initialize","arguments":{}}"#).unwrap();
        let test_file = "/tmp/test_indexer_dap.pilot";
        std::fs::write(test_file, "let map = {\"alpha\": 10, \"beta\": 20};\nlet item = map[\"alpha\"];\nmap[\"alpha\"] = 99;\nlet updated = map[\"alpha\"];\n").unwrap();

        dap.handle_message(&format!(
            r#"{{"seq":2,"type":"request","command":"setBreakpoints","arguments":{{"source":{{"path":"{}"}},"breakpoints":[{{"line":4}}]}}}}"#,
            test_file
        )).unwrap();

        dap.handle_message(&format!(
            r#"{{"seq":3,"type":"request","command":"launch","arguments":{{"program":"{}","stopOnEntry":false}}}}"#,
            test_file
        )).unwrap();
        dap.handle_message(r#"{"seq":4,"type":"request","command":"configurationDone"}"#).unwrap();

        assert!(matches!(dap.debugger.state, DebuggerState::Paused(_)));

        let vars_resp = dap.handle_message(r#"{"seq":7,"type":"request","command":"variables","arguments":{"variablesReference":1000}}"#).unwrap();
        println!("DAP indexer vars response: {}", vars_resp);

        assert!(vars_resp.contains("\"name\":\"item\""), "Expected item variable: {}", vars_resp);
        assert!(vars_resp.contains("\"value\":\"10\""), "Expected item value 10: {}", vars_resp);

        let _ = std::fs::remove_file(test_file);
    }
}
