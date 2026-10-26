using System;
using System.IO;
using System.Threading.Tasks;
using Pilot.Compiler;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Pilot.Scripting;

namespace Pilot.Debugging;

/// <summary>
/// Interactive CLI debugger REPL for Pilot scripts.
/// Supports stepping, breakpoints, call stack inspection, locals, and dynamic expression evaluation.
/// </summary>
public sealed class PilotCliDebugger
{
    private readonly IPilotDebugger _debugger;
    private string[] _sourceLines = Array.Empty<string>();
    private string _filePath = string.Empty;

    public PilotCliDebugger(IPilotDebugger? debugger = null)
    {
        _debugger = debugger ?? new PilotDebugger { WaitOnPause = true };
    }

    public int Run(string filePath, string[]? scriptArgs = null)
    {
        if (!File.Exists(filePath))
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[DEBUG ERROR] File not found: '{filePath}'");
            Console.ResetColor();
            return 1;
        }

        _filePath = Path.GetFullPath(filePath);
        _sourceLines = File.ReadAllLines(_filePath);

        Console.ForegroundColor = ConsoleColor.Cyan;
        Console.WriteLine("==================================================================");
        Console.WriteLine("                 PILOT INTERACTIVE DEBUGGER                       ");
        Console.WriteLine("==================================================================");
        Console.WriteLine($"Debugging: {_filePath}");
        Console.WriteLine("Type 'h' or 'help' for available debugger commands.\n");
        Console.ResetColor();

        // Pause initially on the first statement (StepInto)
        _debugger.StepInto();

        _debugger.Paused += (sender, e) =>
        {
            HandlePause(e);
        };

        var options = new PilotExecutionOptions()
            .WithDebugger(_debugger);

        var execTask = Task.Run(() =>
        {
            try
            {
                return PilotScript.RunFile(_filePath, scriptArgs ?? Array.Empty<string>());
            }
            catch (Exception ex)
            {
                return PilotExecutionResult.CreateFailure(ex.Message, string.Empty, string.Empty, TimeSpan.Zero);
            }
        });

        execTask.Wait();

        Console.ForegroundColor = ConsoleColor.Cyan;
        Console.WriteLine("\n[DEBUGGER] Program execution finished.");
        Console.ResetColor();

        return execTask.Result.Success ? 0 : 1;
    }

    private void HandlePause(DebuggerPauseEventArgs e)
    {
        Console.ForegroundColor = ConsoleColor.Yellow;
        string reasonStr = e.Reason switch
        {
            DebuggerPauseReason.Breakpoint => $"[BREAKPOINT #{e.Breakpoint?.Id}]",
            DebuggerPauseReason.Step => "[STEPPED]",
            DebuggerPauseReason.Exception => $"[EXCEPTION: {e.Exception?.Message}]",
            _ => "[PAUSED]"
        };

        var frame = e.Frame;
        Console.WriteLine($"\n{reasonStr} in {frame.FunctionName}() at line {frame.Line}:{frame.Column}");
        Console.ResetColor();

        PrintSourceWindow(frame.Line);

        // Command loop
        while (_debugger.State == DebuggerState.Paused)
        {
            Console.ForegroundColor = ConsoleColor.Green;
            Console.Write("(pilot-db) ");
            Console.ResetColor();

            string? input = Console.ReadLine()?.Trim();
            if (string.IsNullOrEmpty(input)) continue;

            string[] parts = input.Split(' ', 2, StringSplitOptions.RemoveEmptyEntries);
            string cmd = parts[0].ToLowerInvariant();
            string rest = parts.Length > 1 ? parts[1].Trim() : string.Empty;

            switch (cmd)
            {
                case "c" or "continue":
                    _debugger.Continue();
                    return;

                case "s" or "step" or "stepin":
                    _debugger.StepInto();
                    return;

                case "n" or "next" or "stepover":
                    _debugger.StepOver();
                    return;

                case "out" or "finish" or "stepout":
                    _debugger.StepOut();
                    return;

                case "b" or "break":
                    if (int.TryParse(rest, out int lineNum))
                    {
                        var bp = _debugger.SetBreakpoint(_filePath, lineNum);
                        Console.WriteLine($"Breakpoint {bp.Id} set at line {lineNum}");
                    }
                    else
                    {
                        Console.WriteLine("Usage: b <line_number>");
                    }
                    break;

                case "del" or "delete":
                    if (int.TryParse(rest, out int bpId))
                    {
                        bool removed = _debugger.RemoveBreakpoint(bpId);
                        Console.WriteLine(removed ? $"Breakpoint {bpId} deleted." : $"Breakpoint {bpId} not found.");
                    }
                    else
                    {
                        Console.WriteLine("Usage: del <breakpoint_id>");
                    }
                    break;

                case "l" or "locals":
                    PrintLocals();
                    break;

                case "bt" or "stack" or "where":
                    PrintCallStack();
                    break;

                case "p" or "eval" or "print":
                    if (!string.IsNullOrEmpty(rest))
                    {
                        try
                        {
                            var evalResult = _debugger.Evaluate(rest);
                            Console.WriteLine($"= {evalResult} ({evalResult.Kind})");
                        }
                        catch (Exception ex)
                        {
                            Console.ForegroundColor = ConsoleColor.Red;
                            Console.WriteLine($"Evaluation error: {ex.Message}");
                            Console.ResetColor();
                        }
                    }
                    else
                    {
                        Console.WriteLine("Usage: p <expression>");
                    }
                    break;

                case "set":
                    HandleSetVariable(rest);
                    break;

                case "list":
                    PrintSourceWindow(frame.Line, 5);
                    break;

                case "q" or "quit" or "exit":
                    _debugger.Stop();
                    return;

                case "h" or "help":
                default:
                    PrintHelp();
                    break;
            }
        }
    }

    private void PrintSourceWindow(int currentLine, int radius = 2)
    {
        if (_sourceLines.Length == 0) return;

        int start = Math.Max(1, currentLine - radius);
        int end = Math.Min(_sourceLines.Length, currentLine + radius);

        for (int l = start; l <= end; l++)
        {
            string marker = l == currentLine ? "-->" : "   ";
            string lineText = _sourceLines[l - 1];

            if (l == currentLine)
            {
                Console.ForegroundColor = ConsoleColor.White;
                Console.WriteLine($"  {marker} {l,4} | {lineText}");
                Console.ResetColor();
            }
            else
            {
                Console.ForegroundColor = ConsoleColor.DarkGray;
                Console.WriteLine($"  {marker} {l,4} | {lineText}");
                Console.ResetColor();
            }
        }
    }

    private void PrintLocals()
    {
        var locals = _debugger.GetLocals();
        Console.WriteLine($"--- Locals ({locals.Count}) ---");
        if (locals.Count == 0)
        {
            Console.WriteLine("  <no local variables>");
            return;
        }

        foreach (var kvp in locals)
        {
            Console.WriteLine($"  {kvp.Key} = {kvp.Value} ({kvp.Value.Kind})");
        }
    }

    private void PrintCallStack()
    {
        var frames = _debugger.CallStack.Frames;
        Console.WriteLine($"--- Call Stack ({frames.Count} frames) ---");
        for (int i = 0; i < frames.Count; i++)
        {
            var f = frames[i];
            string marker = i == 0 ? "-->" : "   ";
            Console.WriteLine($"  {marker} #{i,2} {f.FunctionName}() at {f.SourceFile}:{f.Line}:{f.Column}");
        }
    }

    private void HandleSetVariable(string args)
    {
        // Format: varName = expression
        string[] parts = args.Split('=', 2);
        if (parts.Length != 2)
        {
            Console.WriteLine("Usage: set <varName> = <expression>");
            return;
        }

        string varName = parts[0].Trim();
        string expr = parts[1].Trim();

        try
        {
            var value = _debugger.Evaluate(expr);
            bool success = _debugger.SetVariable(varName, value);
            if (success)
            {
                Console.WriteLine($"Set '{varName}' = {value}");
            }
            else
            {
                Console.WriteLine($"Variable '{varName}' not found in current frame.");
            }
        }
        catch (Exception ex)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"Failed to set variable: {ex.Message}");
            Console.ResetColor();
        }
    }

    private static void PrintHelp()
    {
        Console.WriteLine("Available commands:");
        Console.WriteLine("  c, continue        - Resume normal execution until next breakpoint");
        Console.WriteLine("  s, step            - Step into next statement or function call");
        Console.WriteLine("  n, next            - Step over next statement without entering functions");
        Console.WriteLine("  out, finish        - Step out of current function to its caller");
        Console.WriteLine("  b, break <line>    - Set a breakpoint on a line number");
        Console.WriteLine("  del <id>           - Delete a breakpoint by ID");
        Console.WriteLine("  l, locals          - List all local variables in the current frame");
        Console.WriteLine("  bt, stack, where   - Print the current call stack");
        Console.WriteLine("  p, eval <expr>     - Evaluate an expression dynamically in current scope");
        Console.WriteLine("  set <var> = <expr> - Modify a variable value in the current frame");
        Console.WriteLine("  list               - Show source code lines around current location");
        Console.WriteLine("  q, quit            - Terminate debugging session");
        Console.WriteLine("  h, help            - Show this help message");
    }
}
