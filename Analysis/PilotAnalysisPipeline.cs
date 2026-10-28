using System;
using System.Collections.Generic;
using Antlr4.Runtime;
using Pilot.Analysis.ContractVerification;
using Pilot.Analysis.DimensionalAnalysis;
using Pilot.Analysis.Loops;
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
    public IReadOnlyDictionary<string, string> InferredFunctionReturnTypes { get; }
    public IReadOnlyDictionary<PilotParser.For_stmtContext, PilotForLoopInfo> LoopInfos { get; }
    public PilotTypeInferenceEngine? TypeEngine { get; }
    public IReadOnlyList<PilotTypeConstraint> TypeConstraints => TypeEngine?.Constraints ?? Array.Empty<PilotTypeConstraint>();
    public IReadOnlyList<PilotMonomorphizedCallSite> MonomorphizedCallSites => TypeEngine?.MonomorphizedCallSites ?? Array.Empty<PilotMonomorphizedCallSite>();
    public IReadOnlyList<PilotInlayHint> InlayHints => TypeEngine?.InlayHints ?? Array.Empty<PilotInlayHint>();

    public PilotStaticAnalysisResult(
        IEnumerable<PilotAnalysisDiagnostic> diagnostics,
        IReadOnlyDictionary<string, PilotType> inferredTypes,
        IReadOnlyDictionary<string, string>? inferredFunctionReturnTypes = null,
        IReadOnlyDictionary<PilotParser.For_stmtContext, PilotForLoopInfo>? loopInfos = null,
        PilotTypeInferenceEngine? typeEngine = null)
    {
        Diagnostics.AddRange(diagnostics);
        InferredTypes = inferredTypes;
        InferredFunctionReturnTypes = inferredFunctionReturnTypes ?? new Dictionary<string, string>(StringComparer.Ordinal);
        LoopInfos = loopInfos ?? new Dictionary<PilotParser.For_stmtContext, PilotForLoopInfo>();
        TypeEngine = typeEngine;
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

        // 6. Concise Modern Null-Safety Verifier (Milestone 12: NULL001–NULL005)
        var nullSafety = new PilotNullSafetyAnalyzer();
        nullSafety.CheckProgram(program);
        diagnostics.AddRange(nullSafety.Diagnostics);

        // 7. Values-First Error System Verifier (Milestone 13: ERR001–ERR005)
        var errorFlow = new PilotErrorFlowAnalyzer();
        errorFlow.CheckProgram(program);
        diagnostics.AddRange(errorFlow.Diagnostics);

        // 8. Dynamic-Schedule & Expression-Oriented For Loop Analyzer (Milestone 15: LOOP001–LOOP004)
        var loopAnalyzer = new PilotForLoopAnalyzer();
        loopAnalyzer.CheckProgram(program);
        diagnostics.AddRange(loopAnalyzer.Diagnostics);

        return new PilotStaticAnalysisResult(diagnostics, typeEngine.InferredTypes, errorFlow.InferredFunctionReturnTypes, loopAnalyzer.LoopInfos, typeEngine);
    }

    /// <summary>
    /// Parses and analyzes raw Pilot source code.
    /// </summary>
    public static PilotStaticAnalysisResult AnalyzeSource(string source)
    {
        var preprocessed = Pilot.Preprocessing.PilotPreprocessor.Preprocess(source);
        var inputStream = new AntlrInputStream(preprocessed.PreprocessedSource);
        var lexer = new PilotLexer(inputStream);
        lexer.RemoveErrorListeners();
        var tokenStream = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokenStream);
        var bangDiagnostics = new List<PilotAnalysisDiagnostic>();
        parser.RemoveErrorListeners();
        parser.AddErrorListener(new NullAssertionSyntaxListener(bangDiagnostics));
        var tree = parser.ParseProgramFast();

        var result = Analyze(tree);
        foreach (var bd in bangDiagnostics)
        {
            if (!result.Diagnostics.Exists(d => d.Code == "NULL005" && d.Line == bd.Line))
            {
                result.Diagnostics.Add(bd);
            }
        }
        return result;
    }

    private sealed class NullAssertionSyntaxListener : BaseErrorListener
    {
        private readonly List<PilotAnalysisDiagnostic> _diagnostics;
        public NullAssertionSyntaxListener(List<PilotAnalysisDiagnostic> diagnostics) => _diagnostics = diagnostics;

        public override void SyntaxError(
            IRecognizer recognizer,
            IToken offendingSymbol,
            int line,
            int charPositionInLine,
            string msg,
            RecognitionException e)
        {
            if (offendingSymbol?.Text == "!" || (msg != null && msg.Contains("'!'", StringComparison.Ordinal)))
            {
                _diagnostics.Add(new PilotAnalysisDiagnostic(
                    "NULL005",
                    "Unchecked postfix non-null assertion '!' is removed in Pilot. Use 'guard some x else', '??', or explicit '.OrPanic(\"reason\")'.",
                    line,
                    charPositionInLine,
                    isError: true));
            }
        }
    }
}
