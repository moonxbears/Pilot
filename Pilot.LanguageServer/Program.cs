using System;
using System.IO;
using System.Threading.Tasks;
using Pilot.Diagnostics;
using Pilot.LanguageServer.Lsp;
using Pilot.LanguageServer.Semantics;

namespace Pilot.LanguageServer;

public static class Program
{
    public static async Task<int> Main(string[] args)
    {
        if (args.Length > 0 && (args[0] is "--version" or "-v" or "-V" or "version"))
        {
            Console.WriteLine($"Pilot Language Server v{PilotVersion.FullVersion} (commit #{PilotVersion.CommitCount}: {PilotVersion.ShortCommitSha} on {PilotVersion.Branch})");
            return 0;
        }

        if (args.Length >= 2 && args[0] == "--check")
        {
            string filePath = Path.GetFullPath(args[1]);
            if (!File.Exists(filePath))
            {
                Console.Error.WriteLine($"[ERROR] File not found: {filePath}");
                return 1;
            }

            string source = await File.ReadAllTextAsync(filePath);
            var result = PilotSemanticAnalyzer.Analyze(source, new Uri(filePath).AbsoluteUri);

            Console.WriteLine($"Analyzed '{Path.GetFileName(filePath)}':");
            Console.WriteLine($"  Top-level symbols : {result.TopLevelSymbols.Count}");
            Console.WriteLine($"  Semantic tokens   : {result.SemanticTokens.Count}");
            Console.WriteLine($"  Symbol references : {result.References.Count}");
            Console.WriteLine($"  Diagnostics       : {result.Diagnostics.Count}");

            foreach (var diag in result.Diagnostics)
            {
                Console.WriteLine($"    [{diag.Severity}] {diag.Code} (Line {diag.StartLine + 1}:{diag.StartColumn + 1}): {diag.Message}");
            }

            return result.Diagnostics.Exists(d => d.Severity == PilotDiagnosticSeverity.Error) ? 1 : 0;
        }

        // Default: Run Language Server Protocol (JSON-RPC 2.0) over standard input/output for VS Code
        var server = new PilotLspServer();
        using var stdin = Console.OpenStandardInput();
        using var stdout = Console.OpenStandardOutput();
        await server.RunAsync(stdin, stdout);
        return 0;
    }
}
