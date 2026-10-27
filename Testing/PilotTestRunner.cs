using System;
using System.IO;
using System.Linq;
using Antlr4.Runtime;
using Pilot.Diagnostics;
using Pilot.Preprocessing;

namespace Pilot.Testing;

/// <summary>
/// Executes integration test suites located in the tests/ directory.
/// </summary>
public static class PilotTestRunner
{
    public static int RunAll(string testsDir = "tests")
    {
        Console.WriteLine("==================================================================");
        Console.WriteLine("                 PILOT TEST SUITE RUNNER                          ");
        Console.WriteLine("==================================================================");

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
            var preproc = PilotPreprocessor.Preprocess(source, file);
            var errorTracker = new ErrorTracker();
            if (preproc.HasErrors)
            {
                foreach (var err in preproc.Diagnostics.Where(d => d.Severity == PreprocessorDiagnosticSeverity.Error))
                {
                    errorTracker.Errors.Add($"[Preproc] {err.Message}");
                }
            }

            var inputStream = new AntlrInputStream(preproc.PreprocessedSource);
            var lexer = new PilotLexer(inputStream);
            var tokenStream = new CommonTokenStream(lexer);
            var parser = new PilotParser(tokenStream);

            parser.RemoveErrorListeners();
            parser.AddErrorListener(errorTracker);

            parser.ParseProgramFast();

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
}
