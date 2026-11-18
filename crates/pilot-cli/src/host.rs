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
//! | `cli_run` | path, args, tail, checker files, declaration files, tail links | `[exit_code, diagnostics]`; not run when there are diagnostics; the files `tail links` names are linked before the tail |
//! | `cli_typecheck` | path, checker files, declaration files, entry call | `[diagnostics, rest]`: the type checker's diagnostics in the program's own files, and what else the entry gave (`check.rs`); a run plan it made is kept for the `cli_run` that follows |
//! | `cli_program_tool` | path, tool files, declaration files, entry call | the tool's exit code; the tool reads the program's snapshot (`check::run_on_program`) |
//! | `cli_program_eval` | path, tool files, declaration files, entry call | the value the entry call returns (`check::eval_on_program`), or `none` when the program does not link |
//! | `cli_functions` | path | `[name, [attributes]]` per top-level function, or `none` |
//! | `cli_tool` | files, entry call | the tool's result as text |
//! | `cli_tool_on` | files, entry call, input path | the same, its AST queries seeing the input file |

use crate::check;
use crate::run::{self, Diag, EXIT_FAILURE};
use crate::tool;
use pilot_vm::{codes, HostOpHandler, HostValue};

fn diags(list: &[Diag]) -> HostValue {
    HostValue::List(list.iter().map(Diag::to_host).collect())
}

/// The CLI's host operations.
pub fn cli_host_ops() -> HostOpHandler {
    Box::new(|op, args| {
        let text = |i: usize| {
            args.get(i)
                .and_then(HostValue::as_str)
                .unwrap_or("")
                .to_string()
        };
        let texts = |i: usize| -> Vec<String> {
            args.get(i)
                .and_then(HostValue::as_list)
                .map(|l| {
                    l.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default()
        };
        let result = match op {
            "cli_check" => {
                // A seed limit the checker's run plan lifts (a method-call
                // receiver whose type only the checker knows) is compiled
                // again with that plan, given the checker's sources (1) and
                // declarations (2), as `cli_run` compiles it.
                let checker = texts(1);
                let compiled = run::compile(&text(0), None).or_else(|d| {
                    if checker.is_empty() || !d.iter().any(|x| x.code == codes::SEED) {
                        return Err(d);
                    }
                    let plan = check::drop_plan(&text(0), &checker, &texts(2));
                    run::compile_planned(&text(0), None, Some(&plan))
                });
                match compiled {
                    Ok(_) => HostValue::List(Vec::new()),
                    Err(d) => diags(&d),
                }
            }
            "cli_run" => {
                let tail = text(2);
                let tail = (!tail.is_empty()).then_some(tail.as_str());
                // With the checker's sources (3) and declarations (4), owned
                // values are dropped where its drop plan says (F7).
                let checker = texts(3);
                let plan =
                    (!checker.is_empty()).then(|| check::drop_plan(&text(0), &checker, &texts(4)));
                match run::compile_entry(&text(0), tail, &texts(5), plan.as_ref()) {
                    Ok(program) => {
                        let code = run::execute(program, texts(1));
                        HostValue::List(vec![
                            HostValue::Int(i64::from(code)),
                            HostValue::List(Vec::new()),
                        ])
                    }
                    Err(d) => {
                        HostValue::List(vec![HostValue::Int(i64::from(EXIT_FAILURE)), diags(&d)])
                    }
                }
            }
            "cli_typecheck" => {
                let (found, rest) = check::check(&text(0), &texts(1), &texts(2), &text(3));
                HostValue::List(vec![diags(&found), rest])
            }
            "cli_program_tool" => HostValue::Int(i64::from(check::run_on_program(
                &text(0),
                &texts(1),
                &texts(2),
                &text(3),
            ))),
            "cli_program_eval" => check::eval_on_program(&text(0), &texts(1), &texts(2), &text(3)),
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
