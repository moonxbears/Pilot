using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Antlr4.Runtime;
using Pilot.Compiler;
using Pilot.Diagnostics;
using Pilot.Preprocessing;
using Pilot.Scripting;
using Pilot.Testing;

namespace Pilot.Cli;

/// <summary>
/// Main CLI entry point for the Pilot language toolchain.
/// Supports both compilation and direct scripting execution.
/// </summary>
public static class Program
{
    public static int Main(string[] args)
    {
        Console.OutputEncoding = System.Text.Encoding.UTF8;

        if (args.Length > 0 && args[0] == "--all")
        {
            return PilotTestRunner.RunAll();
        }

        // 1. Direct inline script execution: pilot -c "code..." [args...]
        if (args.Length > 1 && (args[0] == "-c" || args[0] == "--command"))
        {
            string code = args[1];
            string[] scriptArgs = args.Length > 2 ? args.Skip(2).ToArray() : Array.Empty<string>();
            return ExecuteInlineScript(code, scriptArgs);
        }

        // 2. Compilation subcommand: pilot compile <file.pilot> [-o <out.pilc>]
        if (args.Length > 1 && (args[0] == "compile" || args[0] == "--compile"))
        {
            return ExecuteCompilationSubcommand(args.Skip(1).ToArray());
        }

        // 3. Script run subcommand: pilot run <file.pilot|file.pilc> [args...]
        if (args.Length > 1 && (args[0] == "run"))
        {
            string scriptFile = args[1];
            string[] scriptArgs = args.Length > 2 ? args.Skip(2).ToArray() : Array.Empty<string>();
            return RunScriptFile(scriptFile, scriptArgs);
        }

        // 4. Interactive debugger subcommand: pilot debug <file.pilot> [args...]
        if (args.Length > 1 && (args[0] == "debug" || args[0] == "--debug"))
        {
            string scriptFile = args[1];
            string[] scriptArgs = args.Length > 2 ? args.Skip(2).ToArray() : Array.Empty<string>();
            return new Pilot.Debugging.PilotCliDebugger().Run(scriptFile, scriptArgs);
        }

        // 4. Direct script execution if first argument is a file: pilot script.pilot [args...] or pilot script.pilc [args...]
        if (args.Length > 0 && !args[0].StartsWith("-") &&
            (args[0].EndsWith(".pilot", StringComparison.OrdinalIgnoreCase) || args[0].EndsWith(".pilc", StringComparison.OrdinalIgnoreCase)) &&
            File.Exists(args[0]))
        {
            string scriptFile = args[0];
            string[] scriptArgs = args.Length > 1 ? args.Skip(1).ToArray() : Array.Empty<string>();
            return RunScriptFile(scriptFile, scriptArgs);
        }

        bool showTokens = false;
        bool showTree = false;
        bool execute = false;
        bool startRepl = false;
        string? emitTarget = null;
        string? fromSource = null;
        string? targetFile = null;
        var definedSymbols = new List<string>();
        var scriptArgsList = new List<string>();

        for (int i = 0; i < args.Length; i++)
        {
            if (args[i] == "--tokens") showTokens = true;
            else if (args[i] == "--tree") showTree = true;
            else if (args[i] == "--exec" || args[i] == "--run") execute = true;
            else if (args[i] == "--repl" || args[i] == "-i") startRepl = true;
            else if ((args[i] == "--emit" || args[i] == "--target") && i + 1 < args.Length)
            {
                emitTarget = args[++i];
            }
            else if ((args[i] == "--from" || args[i] == "--ingest") && i + 1 < args.Length)
            {
                fromSource = args[++i];
            }
            else if ((args[i] == "--define" || args[i] == "-D") && i + 1 < args.Length)
            {
                definedSymbols.Add(args[++i]);
            }
            else if (!args[i].StartsWith("-"))
            {
                if (targetFile == null)
                {
                    targetFile = args[i];
                }
                else
                {
                    scriptArgsList.Add(args[i]);
                }
            }
        }

        if (startRepl)
        {
            return new PilotRepl().Run();
        }

        if (!string.IsNullOrEmpty(fromSource) && !string.IsNullOrEmpty(targetFile) && File.Exists(targetFile))
        {
            string foreignCode = File.ReadAllText(targetFile);
            var result = Pilot.Transpilation.TranspilationRegistry.Default.Ingest(foreignCode, fromSource);
            if (!result.Success)
            {
                Console.ForegroundColor = ConsoleColor.Red;
                Console.WriteLine($"[INGEST FAIL] {string.Join("; ", result.Diagnostics)}");
                Console.ResetColor();
                return 1;
            }
            Console.WriteLine(result.Code);
            return 0;
        }

        if (string.IsNullOrEmpty(targetFile))
        {
            PrintUsage();
            targetFile = Path.Combine(Directory.GetCurrentDirectory(), "examples", "overview.pilot");
            if (!File.Exists(targetFile))
            {
                return 0;
            }
        }

        if (!File.Exists(Path.GetFullPath(targetFile)))
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[ERROR] File not found: '{targetFile}'");
            Console.ResetColor();
            return 1;
        }

        // Running a pre-compiled binary (.pilc)
        if (targetFile.EndsWith(".pilc", StringComparison.OrdinalIgnoreCase))
        {
            return RunScriptFile(targetFile, scriptArgsList.ToArray());
        }

        if (!string.IsNullOrEmpty(emitTarget))
        {
            string pilotCode = File.ReadAllText(targetFile);
            var result = Pilot.Transpilation.TranspilationRegistry.Default.Transpile(pilotCode, emitTarget);
            if (!result.Success)
            {
                Console.ForegroundColor = ConsoleColor.Red;
                Console.WriteLine($"[TRANSPILATION FAIL] {string.Join("; ", result.Diagnostics)}");
                Console.ResetColor();
                return 1;
            }
            Console.WriteLine(result.Code);
            return 0;
        }

        return ParseAndReport(targetFile, showTokens, showTree, definedSymbols, execute, scriptArgsList);
    }

    private static int ExecuteInlineScript(string code, string[] scriptArgs)
    {
        var result = PilotScript.Run(code, scriptArgs);
        if (!result.Success)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[SCRIPT ERROR] {string.Join("; ", result.Diagnostics)}");
            Console.ResetColor();
            return 1;
        }
        return 0;
    }

    private static int ExecuteCompilationSubcommand(string[] args)
    {
        string? inputFile = null;
        string? outputFile = null;
        var symbols = new List<string>();

        for (int i = 0; i < args.Length; i++)
        {
            if ((args[i] == "-o" || args[i] == "--output") && i + 1 < args.Length)
            {
                outputFile = args[++i];
            }
            else if ((args[i] == "-D" || args[i] == "--define") && i + 1 < args.Length)
            {
                symbols.Add(args[++i]);
            }
            else if (!args[i].StartsWith("-"))
            {
                inputFile = args[i];
            }
        }

        if (string.IsNullOrEmpty(inputFile))
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine("[ERROR] Missing source file to compile. Usage: pilot compile <file.pilot> [-o <out.pilc>]");
            Console.ResetColor();
            return 1;
        }

        if (!File.Exists(inputFile))
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[ERROR] File not found: '{inputFile}'");
            Console.ResetColor();
            return 1;
        }

        outputFile ??= Path.ChangeExtension(inputFile, ".pilc");

        Console.WriteLine($"--> Compiling '{inputFile}' -> '{outputFile}'...");
        var options = new PilotCompilerOptions(symbols)
            .WithSourceFile(inputFile);

        var result = PilotCompiler.Default.CompileFile(inputFile, options);
        if (!result.Success || result.Program == null)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[COMPILATION FAILED] ({result.Elapsed.TotalMilliseconds:F2}ms):");
            foreach (var diag in result.Diagnostics)
            {
                Console.WriteLine($"  {diag}");
            }
            Console.ResetColor();
            return 1;
        }

        result.Program.Save(outputFile);

        Console.ForegroundColor = ConsoleColor.Green;
        Console.WriteLine($"[COMPILATION SUCCESS] Generated '{outputFile}' ({new FileInfo(outputFile).Length} bytes) in {result.Elapsed.TotalMilliseconds:F2}ms");
        Console.ResetColor();
        return 0;
    }

    private static int RunScriptFile(string filePath, string[] scriptArgs)
    {
        if (!File.Exists(filePath))
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[ERROR] Script file not found: '{filePath}'");
            Console.ResetColor();
            return 1;
        }

        var result = PilotScript.RunFile(filePath, scriptArgs);
        if (!result.Success)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[SCRIPT RUN ERROR] ({result.Elapsed.TotalMilliseconds:F2}ms):");
            foreach (var diag in result.Diagnostics)
            {
                Console.WriteLine($"  {diag}");
            }
            Console.ResetColor();
            return 1;
        }

        return 0;
    }

    public static void PrintUsage()
    {
        Console.WriteLine("==================================================================");
        Console.WriteLine("                 PILOT LANGUAGE RUNNER & VALIDATOR                ");
        Console.WriteLine("==================================================================");
        Console.WriteLine("Scripting & Execution Usage:");
        Console.WriteLine("  pilot <script.pilot> [args...]       # Run script file directly");
        Console.WriteLine("  pilot <script.pilc> [args...]        # Run precompiled binary directly");
        Console.WriteLine("  pilot run <file.pilot> [args...]     # Explicitly run script");
        Console.WriteLine("  pilot debug <file.pilot> [args...]   # Start interactive CLI debugger session");
        Console.WriteLine("  pilot -c \"<code>\" [args...]         # Execute inline code string");
        Console.WriteLine("  pilot compile <file> [-o <out.pilc>] # Precompile to binary format");
        Console.WriteLine("\nTooling & Analysis Usage:");
        Console.WriteLine("  pilot --tokens <file>                # Dump lexical token stream");
        Console.WriteLine("  pilot --tree <file>                  # Print hierarchical AST");
        Console.WriteLine("  pilot --repl                         # Start interactive REPL");
        Console.WriteLine("  pilot --emit <target> <file>         # Transpile to target (python, csharp, rust)");
        Console.WriteLine("  pilot --from <lang> <file>           # Ingest foreign code to Pilot");
        Console.WriteLine("  pilot -D <symbol> <file>             # Predefine compilation symbol");
        Console.WriteLine("  pilot --all                          # Run all test scripts in tests/");
        Console.WriteLine("==================================================================\n");
    }

    public static int ParseAndReport(
        string filePath,
        bool showTokens,
        bool showTree,
        IEnumerable<string>? definedSymbols = null,
        bool execute = false,
        IReadOnlyList<string>? scriptArgs = null)
    {
        Console.WriteLine($"\n--> Parsing: {filePath}");
        string source = File.ReadAllText(filePath);

        var preproc = PilotPreprocessor.Preprocess(
            source,
            filePath,
            new PilotPreprocessorOptions(definedSymbols ?? Array.Empty<string>()));

        foreach (var warn in preproc.Diagnostics.Where(d => d.Severity == PreprocessorDiagnosticSeverity.Warning))
        {
            Console.ForegroundColor = ConsoleColor.Yellow;
            Console.WriteLine($"  [PREPROC WARNING] {warn.Message} (line {warn.Line})");
            Console.ResetColor();
        }

        if (preproc.HasErrors)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"\n[FAIL] Found {preproc.Diagnostics.Count(d => d.Severity == PreprocessorDiagnosticSeverity.Error)} preprocessor error(s):");
            foreach (var err in preproc.Diagnostics.Where(d => d.Severity == PreprocessorDiagnosticSeverity.Error))
            {
                Console.WriteLine($"  [{err.Line}:{err.Column}] {err.Message}");
            }
            Console.ResetColor();
            return 1;
        }

        var inputStream = new AntlrInputStream(preproc.PreprocessedSource);
        var lexer = new PilotLexer(inputStream);

        if (showTokens)
        {
            Console.WriteLine("\n--- Token Stream ---");
            var tokens = lexer.GetAllTokens();
            foreach (var t in tokens)
            {
                string name = PilotLexer.DefaultVocabulary.GetSymbolicName(t.Type);
                string text = t.Text.Replace("\r", "\\r").Replace("\n", "\\n");
                Console.WriteLine($"  Line {t.Line,3}:{t.Column,-3} Token: {name,-22} Text: '{text}' (Channel: {t.Channel})");
            }
            lexer.Reset();
        }

        var tokenStream = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokenStream);

        var errorListener = new ErrorTracker();
        parser.RemoveErrorListeners();
        parser.AddErrorListener(errorListener);

        var tree = parser.program();

        if (errorListener.HasErrors)
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"\n[FAIL] Found {errorListener.Errors.Count} syntax error(s):");
            foreach (var err in errorListener.Errors)
            {
                Console.WriteLine($"  {err}");
            }
            Console.ResetColor();
            return 1;
        }

        Console.ForegroundColor = ConsoleColor.Green;
        Console.WriteLine($"[SUCCESS] Parsed successfully with 0 errors!");
        Console.ResetColor();
        Console.WriteLine($"  Root Context: {tree.GetType().Name}");
        Console.WriteLine($"  Top-level nodes: {tree.ChildCount}");

        if (showTree)
        {
            Console.WriteLine("\n--- Hierarchical Parse Tree ---");
            ParseTreeVisualizer.Print(tree, parser, 0);
        }

        if (execute)
        {
            Console.WriteLine("\n--> Executing with PilotCompiledProgram...");
            var program = new PilotCompiledProgram(preproc.PreprocessedSource, tree, filePath, DateTime.UtcNow, definedSymbols);
            var result = program.RunWithArgs(scriptArgs ?? Array.Empty<string>());
            if (!result.Success)
            {
                Console.ForegroundColor = ConsoleColor.Red;
                Console.WriteLine($"\n[RUNTIME FAIL] Execution failed ({result.Elapsed.TotalMilliseconds:F2}ms):");
                foreach (var err in result.Diagnostics)
                {
                    Console.WriteLine($"  {err}");
                }
                Console.ResetColor();
                return 1;
            }

            Console.ForegroundColor = ConsoleColor.Green;
            Console.WriteLine($"\n[RUNTIME SUCCESS] Executed in {result.Elapsed.TotalMilliseconds:F2}ms");
            Console.ResetColor();
            Console.WriteLine($"  Final Value: {result.Value}");
        }

        return 0;
    }
}
