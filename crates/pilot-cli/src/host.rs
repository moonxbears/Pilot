//! Host operations of the Pilot CLI.
//!
//! The CLI itself is Pilot (`selfhost/cli/`): it parses arguments, dispatches
//! commands through its registry, renders diagnostics and runs tests. It
//! reaches the seed only through these operations, which do what Pilot cannot
//! do for itself yet: compile and run a program, read a file's declarations,
//! and run another self-hosted tool.
//!
//! | operation | arguments | result |
//! | --- | --- | --- |
//! | `cli_check` | path | diagnostics (`run::Diag::to_host`) |
//! | `cli_run` | path, args, tail | `[exit_code, diagnostics]`; not run when there are diagnostics |
//! | `cli_functions` | path | `[name, [attributes]]` per top-level function, or `none` |
//! | `cli_tool` | files, entry call | the tool's result as text |
//! | `cli_tool_on` | files, entry call, input path | the same, its AST queries seeing the input file |

use crate::run::{self, Diag, EXIT_FAILURE};
use crate::tool;
use pilot_vm::{HostOpHandler, HostValue};

fn diags(list: &[Diag]) -> HostValue {
    HostValue::List(list.iter().map(Diag::to_host).collect())
}

/// The CLI's host operations.
pub fn cli_host_ops() -> HostOpHandler {
    Box::new(|op, args| {
        let text = |i: usize| args.get(i).and_then(HostValue::as_str).unwrap_or("").to_string();
        let texts = |i: usize| -> Vec<String> {
            args.get(i)
                .and_then(HostValue::as_list)
                .map(|l| l.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                .unwrap_or_default()
        };
        let result = match op {
            "cli_check" => match run::compile(&text(0), None) {
                Ok(_) => HostValue::List(Vec::new()),
                Err(d) => diags(&d),
            },
            "cli_run" => {
                let tail = text(2);
                let tail = (!tail.is_empty()).then_some(tail.as_str());
                match run::compile(&text(0), tail) {
                    Ok(program) => {
                        let code = run::execute(program, texts(1));
                        HostValue::List(vec![HostValue::Int(i64::from(code)), HostValue::List(Vec::new())])
                    }
                    Err(d) => HostValue::List(vec![HostValue::Int(i64::from(EXIT_FAILURE)), diags(&d)]),
                }
            }
            "cli_functions" => run::declared_functions(&text(0)).unwrap_or(HostValue::Null),
            "cli_tool" | "cli_tool_on" => {
                let files = texts(0);
                let paths: Vec<&str> = files.iter().map(String::as_str).collect();
                let out = if op == "cli_tool" {
                    tool::eval(&paths, &text(1))
                } else {
                    tool::eval_on_input(&paths, &text(1), &text(2))
                };
                HostValue::Str(out)
            }
            _ => return None,
        };
        Some(Ok(result))
    })
}
