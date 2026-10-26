using System;
using System.IO;
using System.Linq;
using System.Text;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Pilot.Transpilation;

namespace Pilot.Cli;

/// <summary>
/// Interactive Read-Eval-Print Loop (REPL) for the Pilot language.
/// Provides persistent state, multi-line editing, scope inspection, and transpilation preview.
/// </summary>
public sealed class PilotRepl
{
    private readonly PilotEngine _engine = new();
    private readonly PilotExecutionOptions _options;
    private PilotExecutionContext _context;

    public PilotRepl()
    {
        _options = new PilotExecutionOptions();
        _context = new PilotExecutionContext(_engine, _options);

        // Populate standard built-ins into REPL global scope
        foreach (var kvp in _options.Builtins.GetAll())
        {
            _context.Environment.GlobalScope.Declare(kvp.Key, PilotValue.FromFunction(kvp.Value));
        }
    }

    public int Run()
    {
        Console.OutputEncoding = Encoding.UTF8;
        Console.ForegroundColor = ConsoleColor.Cyan;
        Console.WriteLine("==================================================================");
        Console.WriteLine("                    PILOT INTERACTIVE REPL                        ");
        Console.WriteLine("     \"Beautiful code you can make your own without sacrificing speed\"  ");
        Console.WriteLine("==================================================================");
        Console.ForegroundColor = ConsoleColor.DarkGray;
        Console.WriteLine("Commands: :help, :scope, :reset, :emit <python|csharp|rust>, :exit");
        Console.WriteLine("Type any expression or statement and press Enter.\n");
        Console.ResetColor();

        var multiLineBuffer = new StringBuilder();

        while (true)
        {
            bool isContinuation = multiLineBuffer.Length > 0;
            Console.ForegroundColor = isContinuation ? ConsoleColor.DarkYellow : ConsoleColor.Magenta;
            Console.Write(isContinuation ? "....> " : "pilot> ");
            Console.ResetColor();

            string? line = Console.ReadLine();
            if (line == null) break; // EOF (Ctrl+Z / Ctrl+D)

            string trimmed = line.Trim();

            // Commands
            if (!isContinuation && trimmed.StartsWith(":"))
            {
                if (HandleCommand(trimmed))
                {
                    continue;
                }
                else
                {
                    break; // Exit command
                }
            }

            if (!isContinuation && (trimmed == "exit" || trimmed == "quit" || trimmed == "exit()"))
            {
                break;
            }

            multiLineBuffer.AppendLine(line);
            string fullInput = multiLineBuffer.ToString().Trim();

            // Determine if more input is expected (unmatched braces/parentheses)
            if (NeedsMoreInput(fullInput))
            {
                continue;
            }

            // Execute input
            ExecuteInput(fullInput);
            multiLineBuffer.Clear();
        }

        Console.ForegroundColor = ConsoleColor.Cyan;
        Console.WriteLine("\nGoodbye!");
        Console.ResetColor();
        return 0;
    }

    private void ExecuteInput(string input)
    {
        if (string.IsNullOrWhiteSpace(input)) return;

        try
        {
            // First try evaluating as expression
            try
            {
                var exprVal = _engine.EvaluateExpression(input, _context.Environment.CurrentScope);
                if (!exprVal.IsUndefined)
                {
                    Console.ForegroundColor = ConsoleColor.Green;
                    Console.Write("=> ");
                    Console.ForegroundColor = ConsoleColor.White;
                    Console.Write(exprVal.ToString());
                    Console.ForegroundColor = ConsoleColor.DarkGray;
                    Console.WriteLine($" ({exprVal.Kind})");
                    Console.ResetColor();
                    return;
                }
            }
            catch
            {
                // If not standalone expression, execute as statements
            }

            var visitor = new PilotInterpreterVisitor(_context);
            var inputStream = new Antlr4.Runtime.AntlrInputStream(input);
            var lexer = new PilotLexer(inputStream);
            var tokenStream = new Antlr4.Runtime.CommonTokenStream(lexer);
            var parser = new PilotParser(tokenStream);

            var errorTracker = new Diagnostics.ErrorTracker();
            parser.RemoveErrorListeners();
            parser.AddErrorListener(errorTracker);

            var tree = parser.program();
            if (errorTracker.HasErrors)
            {
                Console.ForegroundColor = ConsoleColor.Red;
                foreach (var err in errorTracker.Errors)
                {
                    Console.WriteLine($"  {err}");
                }
                Console.ResetColor();
                return;
            }

            var val = visitor.VisitProgram(tree);
            if (!val.IsUndefined && !val.IsNull)
            {
                Console.ForegroundColor = ConsoleColor.Green;
                Console.Write("=> ");
                Console.ForegroundColor = ConsoleColor.White;
                Console.Write(val.ToString());
                Console.ForegroundColor = ConsoleColor.DarkGray;
                Console.WriteLine($" ({val.Kind})");
                Console.ResetColor();
            }
        }
        catch (Exception ex)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"Error: {ex.Message}");
            Console.ResetColor();
        }
    }

    private bool HandleCommand(string cmd)
    {
        var parts = cmd.Split(' ', 2, StringSplitOptions.RemoveEmptyEntries);
        string name = parts[0].ToLowerInvariant();
        string arg = parts.Length > 1 ? parts[1].Trim() : "";

        switch (name)
        {
            case ":help":
                Console.ForegroundColor = ConsoleColor.Yellow;
                Console.WriteLine("Available REPL commands:");
                Console.WriteLine("  :help                       Show this help menu");
                Console.WriteLine("  :scope, :vars               List variables declared in current session");
                Console.WriteLine("  :reset                      Reset environment and variables");
                Console.WriteLine("  :clear                      Clear the terminal screen");
                Console.WriteLine("  :emit <python|csharp|rust>  Transpile code snippet to target");
                Console.WriteLine("  :exit, :quit                Exit the REPL");
                Console.ResetColor();
                return true;

            case ":scope":
            case ":vars":
                Console.ForegroundColor = ConsoleColor.Yellow;
                Console.WriteLine("Variables in active REPL scope:");
                foreach (var kvp in _context.Environment.CurrentScope.GetLocalBindings())
                {
                    Console.WriteLine($"  {kvp.Key,-16} : {kvp.Value.Value.Kind,-10} = {kvp.Value.Value}");
                }
                Console.ResetColor();
                return true;

            case ":clear":
                Console.Clear();
                return true;

            case ":reset":
                _context = new PilotExecutionContext(_engine, _options);
                foreach (var kvp in _options.Builtins.GetAll())
                {
                    _context.Environment.GlobalScope.Declare(kvp.Key, PilotValue.FromFunction(kvp.Value));
                }
                Console.ForegroundColor = ConsoleColor.Yellow;
                Console.WriteLine("REPL session reset.");
                Console.ResetColor();
                return true;

            case ":emit":
                if (string.IsNullOrEmpty(arg))
                {
                    Console.WriteLine("Usage: :emit <python|csharp|rust>");
                    return true;
                }
                Console.WriteLine($"Enter Pilot code to transpile to {arg} (end with empty line):");
                var sb = new StringBuilder();
                while (true)
                {
                    string? l = Console.ReadLine();
                    if (string.IsNullOrEmpty(l)) break;
                    sb.AppendLine(l);
                }
                var res = TranspilationRegistry.Default.Transpile(sb.ToString(), arg);
                if (res.Success)
                {
                    Console.ForegroundColor = ConsoleColor.Cyan;
                    Console.WriteLine($"--- Transpiled to {arg} ---");
                    Console.WriteLine(res.Code);
                    Console.ResetColor();
                }
                else
                {
                    Console.ForegroundColor = ConsoleColor.Red;
                    Console.WriteLine(string.Join("; ", res.Diagnostics));
                    Console.ResetColor();
                }
                return true;

            case ":exit":
            case ":quit":
                return false;

            default:
                Console.ForegroundColor = ConsoleColor.Red;
                Console.WriteLine($"Unknown command '{name}'. Type :help for list.");
                Console.ResetColor();
                return true;
        }
    }

    private static bool NeedsMoreInput(string input)
    {
        int braces = 0;
        int brackets = 0;
        int parens = 0;
        bool inString = false;
        char quoteChar = '\0';

        for (int i = 0; i < input.Length; i++)
        {
            char c = input[i];
            if (inString)
            {
                if (c == quoteChar && (i == 0 || input[i - 1] != '\\'))
                {
                    inString = false;
                }
                continue;
            }

            if (c == '"' || c == '\'')
            {
                inString = true;
                quoteChar = c;
            }
            else if (c == '{') braces++;
            else if (c == '}') braces--;
            else if (c == '[') brackets++;
            else if (c == ']') brackets--;
            else if (c == '(') parens++;
            else if (c == ')') parens--;
        }

        return braces > 0 || brackets > 0 || parens > 0;
    }
}
