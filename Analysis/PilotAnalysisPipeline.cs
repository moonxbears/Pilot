using System;
using System.Collections.Generic;
using Antlr4.Runtime;
using Pilot.Analysis.ContractVerification;
using Pilot.Analysis.DimensionalAnalysis;
using Pilot.Analysis.Ownership;
using Pilot.Analysis.PatternMatching;
using Pilot.Analysis.TypeSystem;
using Pilot.Compiler;

namespace Pilot.Analysis;

/// <summary>
/// Result of running the static analysis pipeline over a Pilot program AST.
/// </summary>
public sealed class PilotStaticAnalysisResult
{
    public bool HasErrors => Diagnostics.Exists(d => d.IsError);
    public List<PilotAnalysisDiagnostic> Diagnostics { get; } = new();
    public IReadOnlyDictionary<string, PilotType> InferredTypes { get; }

    public PilotStaticAnalysisResult(
        IEnumerable<PilotAnalysisDiagnostic> diagnostics,
        IReadOnlyDictionary<string, PilotType> inferredTypes)
    {
        Diagnostics.AddRange(diagnostics);
        InferredTypes = inferredTypes;
    }
}

/// <summary>
/// Primary coordinator for Milestone 3 &amp; Milestone 11 static analysis passes:
/// - Flow-sensitive bidirectional type inference and constraint solving
/// - Affine &amp; linear resource borrow checking (preserving Python/C#-like simplicity)
/// - Static physical units of measure dimensional algebra
/// - Design-by-contract compile-time predicate verification
/// - Maranget decision-tree exhaustiveness &amp; redundancy checking (`MATCH001` / `MATCH002`)
/// </summary>
public static class PilotAnalysisPipeline
{
    /// <summary>
    /// Analyzes a parsed Pilot program AST.
    /// </summary>
    public static PilotStaticAnalysisResult Analyze(PilotParser.ProgramContext program)
    {
        var diagnostics = new List<PilotAnalysisDiagnostic>();

        // 1. Bidirectional Type Inference
        var typeEngine = new PilotTypeInferenceEngine();
        typeEngine.VisitProgram(program);
        diagnostics.AddRange(typeEngine.Diagnostics);

        // 2. Affine & Linear Resource Borrow Checker
        var borrowChecker = new PilotBorrowChecker();
        borrowChecker.VisitProgram(program);
        diagnostics.AddRange(borrowChecker.Diagnostics);

        // 3. Static Dimensional Analysis
        var dimensionChecker = new PilotDimensionChecker();
        dimensionChecker.VisitProgram(program);
        diagnostics.AddRange(dimensionChecker.Diagnostics);

        // 4. Compile-Time Contract Verification
        var contractVerifier = new PilotContractVerifier();
        contractVerifier.VisitProgram(program);
        diagnostics.AddRange(contractVerifier.Diagnostics);

        // 5. Maranget Exhaustiveness & Redundancy Pattern Matching Verification (Milestone 11)
        var matchDiagnostics = PilotExhaustivenessChecker.CheckProgram(program, typeEngine);
        diagnostics.AddRange(matchDiagnostics);

        return new PilotStaticAnalysisResult(diagnostics, typeEngine.InferredTypes);
    }

    /// <summary>
    /// Parses and analyzes raw Pilot source code.
    /// </summary>
    public static PilotStaticAnalysisResult AnalyzeSource(string source)
    {
        var inputStream = new AntlrInputStream(source);
        var lexer = new PilotLexer(inputStream);
        var tokenStream = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokenStream);
        var tree = parser.ParseProgramFast();

        return Analyze(tree);
    }
}
