using System;
using System.Collections.Generic;
using System.IO;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;

namespace Pilot;

public class Program
{
    public static int Main(string[] args)
    {
        Console.OutputEncoding = System.Text.Encoding.UTF8;

        if (args.Length > 0 && args[0] == "--all")
        {
            return RunAllTests();
        }

        bool showTokens = false;
        bool showTree = false;
        string? targetFile = null;

        for (int i = 0; i < args.Length; i++)
        {
            if (args[i] == "--tokens") showTokens = true;
            else if (args[i] == "--tree") showTree = true;
            else targetFile = args[i];
        }

        if (string.IsNullOrEmpty(targetFile))
        {
            Console.WriteLine("==================================================================");
            Console.WriteLine("                 PILOT LANGUAGE RUNNER & VALIDATOR                ");
            Console.WriteLine("==================================================================");
            Console.WriteLine("Usage:");
            Console.WriteLine("  dotnet run                        # Runs default validation");
            Console.WriteLine("  dotnet run -- <file.pilot>        # Parse and validate a file");
            Console.WriteLine("  dotnet run -- --tokens <file>     # Dump lexical token stream");
            Console.WriteLine("  dotnet run -- --tree <file>       # Print hierarchical AST");
            Console.WriteLine("  dotnet run -- --all               # Run all test scripts in tests/");
            Console.WriteLine("  dotnet test                       # Run full xUnit automated test suite");
            Console.WriteLine("==================================================================\n");

            targetFile = "C:\\Users\\jnjohnson\\Documents\\dev\\Pilot\\examples\\overview.pilot";
        }

        if (!File.Exists(Path.GetFullPath(targetFile)))
        {
            Console.ForegroundColor = ConsoleColor.Red;
            Console.WriteLine($"[ERROR] File not found: '{targetFile}'");
            Console.ResetColor();
            return 1;
        }

        return ParseAndReport(targetFile, showTokens, showTree);
    }

    public static int ParseAndReport(string filePath, bool showTokens, bool showTree)
    {
        Console.WriteLine($"\n--> Parsing: {filePath}");
        string source = File.ReadAllText(filePath);

        var inputStream = new AntlrInputStream(source);
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
            PrintParseTree(tree, parser, 0);
        }

        return 0;
    }

    private static int RunAllTests()
    {
        Console.WriteLine("==================================================================");
        Console.WriteLine("                 PILOT TEST SUITE RUNNER                          ");
        Console.WriteLine("==================================================================");

        string testsDir = "tests";
        if (!Directory.Exists(testsDir))
        {
            Console.WriteLine($"Tests directory '{testsDir}' not found.");
            return 1;
        }

        var files = Directory.GetFiles(testsDir, "*.pilot");
        Array.Sort(files);

        int passed = 0;
        int failed = 0;

        foreach (var file in files)
        {
            string fileName = Path.GetFileName(file);
            bool expectFailure = fileName.Contains("error");

            string source = File.ReadAllText(file);
            var inputStream = new AntlrInputStream(source);
            var lexer = new PilotLexer(inputStream);
            var tokenStream = new CommonTokenStream(lexer);
            var parser = new PilotParser(tokenStream);

            var errorTracker = new ErrorTracker();
            parser.RemoveErrorListeners();
            parser.AddErrorListener(errorTracker);

            parser.program();

            if (expectFailure)
            {
                if (errorTracker.HasErrors)
                {
                    Console.ForegroundColor = ConsoleColor.Green;
                    Console.WriteLine($"  [PASS] (Expected Negative Test) {fileName} - Caught {errorTracker.Errors.Count} error(s)");
                    Console.ResetColor();
                    passed++;
                }
                else
                {
                    Console.ForegroundColor = ConsoleColor.Red;
                    Console.WriteLine($"  [FAIL] {fileName} - Expected errors but parsed with 0 errors!");
                    Console.ResetColor();
                    failed++;
                }
            }
            else
            {
                if (!errorTracker.HasErrors)
                {
                    Console.ForegroundColor = ConsoleColor.Green;
                    Console.WriteLine($"  [PASS] {fileName}");
                    Console.ResetColor();
                    passed++;
                }
                else
                {
                    Console.ForegroundColor = ConsoleColor.Red;
                    Console.WriteLine($"  [FAIL] {fileName} ({errorTracker.Errors.Count} errors)");
                    foreach (var err in errorTracker.Errors)
                    {
                        Console.WriteLine($"         {err}");
                    }
                    Console.ResetColor();
                    failed++;
                }
            }
        }

        Console.WriteLine("------------------------------------------------------------------");
        Console.WriteLine($"Results: {passed} Passed, {failed} Failed, Total: {files.Length}");
        Console.WriteLine("==================================================================");
        return failed == 0 ? 0 : 1;
    }

    private static void PrintParseTree(IParseTree node, Parser parser, int depth)
    {
        string indent = new string(' ', depth * 2);
        string nodeText = Trees.GetNodeText(node, parser);

        if (node is ITerminalNode)
        {
            if (nodeText != "\n" && nodeText != "\r\n" && nodeText.Length > 0)
            {
                Console.WriteLine($"{indent}\u001b[90mterminal:\u001b[0m '{nodeText}'");
            }
        }
        else
        {
            Console.WriteLine($"{indent}\u001b[36m{nodeText}\u001b[0m");
            for (int i = 0; i < node.ChildCount; i++)
            {
                PrintParseTree(node.GetChild(i), parser, depth + 1);
            }
        }
    }
}

public class ErrorTracker : BaseErrorListener
{
    public List<string> Errors { get; } = new List<string>();
    public bool HasErrors => Errors.Count > 0;

    public override void SyntaxError(
        IRecognizer recognizer,
        IToken offendingSymbol,
        int line,
        int charPositionInLine,
        string msg,
        RecognitionException e)
    {
        Errors.Add($"Line {line}:{charPositionInLine} - {msg} (symbol: '{offendingSymbol?.Text}')");
    }
}
