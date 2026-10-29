using System;
using System.IO;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Text;
using Antlr4.Runtime;
using Pilot.Compiler;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Milestone 28 verification suite:
/// - 28.1: Transitional C# <c>SLL -&gt; LL</c> fast path and canonical AST differential oracle (<see cref="PilotAstOracle"/>).
/// - 28.2: Native Rust workspace (<c>pilot-arena</c>, <c>pilot-span</c>, <c>pilot-intern</c>) &amp; SIMD/SWAR line index.
/// - 28.3: 16-byte cache-packed <see cref="NativeAstNode"/> ABI and grammar invariant precedence freeze.
/// - 28.4: Zero-copy C-ABI bridge (<see cref="NativeAstBridge"/>) reading <see cref="ReadOnlySpan{T}"/> directly over native memory.
/// </summary>
public class PilotPhaseDFoundationTests
{
    [Fact]
    public void SllToLlFastPath_HitsStage1ForValidPrograms_AndFallsBackToStage2OnSyntaxError()
    {
        PilotParser.ResetFastParseCountersForTesting();

        string validSource = """
            import std.math;
            def compute(x: i64, y: i64) -> i64 {
                let z = x + y * 2;
                return z;
            }
            let answer = compute(20, 11);
            """;

        var validInput = new AntlrInputStream(validSource);
        var validLexer = new PilotLexer(validInput);
        var validTokens = new CommonTokenStream(validLexer);
        var validParser = new PilotParser(validTokens);

        var tree = validParser.ParseProgramFast();
        Assert.NotNull(tree);
        Assert.Equal(0, validParser.NumberOfSyntaxErrors);
        Assert.True(PilotParser.SllFastPathHits >= 1, "Expected Stage 1 SLL fast path hit for valid program.");
        Assert.Equal(0, PilotParser.LlFallbackRetries);

        // Now feed an intentionally malformed program to verify Stage 2 LL fallback and full error listener fidelity
        string invalidSource = "let broken = (1 + * 2;\n";
        var invalidInput = new AntlrInputStream(invalidSource);
        var invalidLexer = new PilotLexer(invalidInput);
        invalidLexer.RemoveErrorListeners();
        var invalidTokens = new CommonTokenStream(invalidLexer);
        var invalidParser = new PilotParser(invalidTokens);
        invalidParser.RemoveErrorListeners();

        var errTree = invalidParser.ParseProgramFast();
        Assert.NotNull(errTree);
        Assert.True(invalidParser.NumberOfSyntaxErrors > 0, "Expected syntax errors to be reported via LL fallback.");
        Assert.True(PilotParser.LlFallbackRetries >= 1, "Expected Stage 2 LL fallback retry on syntax error.");
    }

    [Fact]
    public void NativeAstNode_HardwareLayoutAndGrammarInvariants_MatchRustSpec()
    {
        // Invariant #9: Strictly 16-byte AstNode and 8-byte ByteSpan
        Assert.Equal(16, Unsafe.SizeOf<NativeAstNode>());
        Assert.Equal(8, Unsafe.SizeOf<NativeByteSpan>());

        // Invariant #6: Type-spec binding powers T? (30) > & (20) > | (15) > -> (10) > ! (5)
        Assert.True(NativeTypePrecedence.PostfixOptional > NativeTypePrecedence.Intersection);
        Assert.True(NativeTypePrecedence.Intersection > NativeTypePrecedence.Union);
        Assert.True(NativeTypePrecedence.Union > NativeTypePrecedence.FnArrow);
        Assert.True(NativeTypePrecedence.FnArrow > NativeTypePrecedence.Fallible);

        // Invariant #8: Expression binding powers Cascade (1) < Assign (2) < Pipe (8) < Range (22) < PostfixLed (40)
        Assert.True(NativeExprPrecedence.Cascade < NativeExprPrecedence.Assign);
        Assert.True(NativeExprPrecedence.Assign < NativeExprPrecedence.Pipe);
        Assert.True(NativeExprPrecedence.Pipe < NativeExprPrecedence.Range);
        Assert.True(NativeExprPrecedence.Range < NativeExprPrecedence.PostfixLed);
    }

    [Fact]
    public void ZeroCopyCAbiBridge_ReadsNativeRustAstNodeAndByteSpanSlicesDirectly()
    {
        Assert.Equal(NativeAstBridge.ExpectedAbiVersion, NativeAstBridge.GetAbiVersion());
        Assert.Equal(16u, NativeAstBridge.GetNativeSizeOfAstNode());

        string source =
            "import std.math;\n" +
            "let x: i64 = 40 + 2;\n" +
            "var flag = true;\n" +
            "const tag = :core/ready;\n" +
            "return x;\n";

        using var nativeView = NativeAstBridge.ParseUtf8ToArena(source);
        ReadOnlySpan<NativeAstNode> nodes = nativeView.Nodes;
        ReadOnlySpan<NativeByteSpan> spans = nativeView.Spans;

        Assert.True(nodes.Length >= 8);
        Assert.Equal(nodes.Length, spans.Length);

        var root = nodes[(int)nativeView.RootId];
        Assert.Equal(NativeNodeKind.Program, root.NodeKind);

        var topChildren = nativeView.GetChildrenSlice(root.Lhs, root.Rhs);
        Assert.Equal(5, topChildren.Length);

        Assert.Equal(NativeNodeKind.ImportDecl, nodes[(int)topChildren[0]].NodeKind);
        Assert.Equal(NativeNodeKind.LetStmt, nodes[(int)topChildren[1]].NodeKind);
        Assert.Equal(NativeNodeKind.VarStmt, nodes[(int)topChildren[2]].NodeKind);
        Assert.Equal(NativeNodeKind.ConstDecl, nodes[(int)topChildren[3]].NodeKind);
        Assert.Equal(NativeNodeKind.ReturnStmt, nodes[(int)topChildren[4]].NodeKind);

        // Verify SIMD/SWAR LineIndex 1-based (Line, Col) lookup over C-ABI
        Assert.Equal(6u, nativeView.LineCount);
        var (line2, col2) = nativeView.LookupLineCol(17); // start of line 2 ("let x: i64 = 40 + 2;")
        Assert.Equal((2u, 1u), (line2, col2));
    }

    [Fact]
    public void DifferentialOracle_MatchesNativeRustArenaAndNormalizesStdlibCorpus()
    {
        string snippet =
            "import std.math;\n" +
            "let x: i64 = 40 + 2;\n" +
            "return x;\n";

        var oracleSnapshot = PilotAstOracle.ParseAndLowerToArena(snippet, out int syntaxErrors);
        Assert.Equal(0, syntaxErrors);

        string sexpr = PilotAstOracle.ToCanonicalSExpr(oracleSnapshot);
        Assert.Equal(
            "(Program (ImportDecl std.math) (LetStmt x (BinaryExpr op=1 (Int 40) (Int 2))) (ReturnStmt (Ident x)))",
            sexpr);

        string json = PilotAstOracle.ToCanonicalJson(oracleSnapshot);
        Assert.Contains("\"kind\":\"Program\"", json);
        Assert.Contains("\"kind\":\"BinaryExpr\"", json);

        // Compare C# oracle node kinds against native Rust arena node kinds
        using var nativeArena = NativeAstBridge.ParseUtf8ToArena(snippet);
        Assert.Equal(oracleSnapshot.Nodes.Count, nativeArena.NodeCount);
        for (int i = 0; i < oracleSnapshot.Nodes.Count; i++)
        {
            Assert.Equal(oracleSnapshot.Nodes[i].NodeKind, nativeArena.Nodes[i].NodeKind);
            Assert.Equal(oracleSnapshot.Nodes[i].Flags, nativeArena.Nodes[i].Flags);
        }

        // Verify that the two-stage SLL -> LL fast path + Oracle cleanly parses and normalizes stdlib modules
        string? repoRoot = FindRepoRoot();
        Assert.NotNull(repoRoot);
        var stdlibFiles = Directory.GetFiles(Path.Combine(repoRoot!, "stdlib"), "*.pilot", SearchOption.AllDirectories)
            .OrderBy(p => p, StringComparer.Ordinal)
            .ToArray();
        Assert.True(stdlibFiles.Length >= 15, $"Expected at least 15 stdlib .pilot modules, found {stdlibFiles.Length}.");

        foreach (string file in stdlibFiles)
        {
            string code = File.ReadAllText(file, Encoding.UTF8);
            var snapshot = PilotAstOracle.ParseAndLowerToArena(code, out int errs);
            Assert.True(errs == 0, $"Expected 0 syntax errors in stdlib module '{Path.GetFileName(file)}', got {errs}.");
            Assert.True(snapshot.Nodes.Count > 1);
            Assert.Equal(NativeNodeKind.Program, snapshot.Nodes[(int)snapshot.RootId].NodeKind);
        }
    }

    private static string? FindRepoRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir != null)
        {
            if (File.Exists(Path.Combine(dir.FullName, "Pilot.csproj")) &&
                Directory.Exists(Path.Combine(dir.FullName, "stdlib")))
            {
                return dir.FullName;
            }
            dir = dir.Parent;
        }
        return null;
    }
}
