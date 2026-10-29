using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Threading;
using Antlr4.Runtime;
using Antlr4.Runtime.Atn;
using Antlr4.Runtime.Misc;

namespace Pilot;

/// <summary>
/// Milestone 28.1: High-performance two-stage parsing (<c>PredictionMode.SLL</c> with automatic
/// <c>PredictionMode.LL</c> fallback) and expression AST caching extensions for <see cref="PilotParser"/>.
/// Serves as the transitional C# differential conformance oracle until ANTLR removal in M30.5.
/// </summary>
public partial class PilotParser
{
    private static readonly ConcurrentDictionary<string, TestContext> CachedExpressions = new(StringComparer.Ordinal);
    private static readonly ConcurrentDictionary<string, ProgramContext> CachedSnippets = new(StringComparer.Ordinal);

    private static long _sllFastPathHits;
    private static long _llFallbackRetries;

    /// <summary>
    /// Number of parses that succeeded directly in Stage 1 (<see cref="PredictionMode.SLL"/>) without full-context ATN overhead.
    /// </summary>
    public static long SllFastPathHits => Interlocked.Read(ref _sllFastPathHits);

    /// <summary>
    /// Number of parses that encountered an SLL conflict or syntax error and rewound to Stage 2 (<see cref="PredictionMode.LL"/>).
    /// </summary>
    public static long LlFallbackRetries => Interlocked.Read(ref _llFallbackRetries);

    public static void ResetFastParseCountersForTesting()
    {
        Interlocked.Exchange(ref _sllFastPathHits, 0);
        Interlocked.Exchange(ref _llFallbackRetries, 0);
    }

    /// <summary>
    /// Parses a complete Pilot program using ANTLR4's two-stage <c>SLL -&gt; LL</c> parsing strategy.
    /// Valid programs parse in <see cref="PredictionMode.SLL"/> mode with <see cref="BailErrorStrategy"/>;
    /// if any syntax error or SLL ambiguity occurs, the token stream is rewound and re-parsed in
    /// <see cref="PredictionMode.LL"/> mode with all original error listeners restored.
    /// </summary>
    public ProgramContext ParseProgramFast()
    {
        var tokenStream = _input;
        if (tokenStream == null)
        {
            return program();
        }

        // Ensure BufferedTokenStream has initialized its initial token position (otherwise Index is -1)
        _ = tokenStream.La(1);
        int startTokenIndex = Math.Max(0, tokenStream.Index);
#pragma warning disable CS0618
        var savedListeners = new List<IAntlrErrorListener<IToken>>(ErrorListeners);
#pragma warning restore CS0618
        var savedErrorHandler = ErrorHandler;
        var savedPredictionMode = Interpreter.PredictionMode;

        try
        {
            RemoveErrorListeners();
            ErrorHandler = new BailErrorStrategy();
            Interpreter.PredictionMode = PredictionMode.Sll;

            var tree = program();
            Interlocked.Increment(ref _sllFastPathHits);
            RestoreParserState(savedListeners, savedErrorHandler, savedPredictionMode);
            return tree;
        }
        catch (Exception ex) when (ex is ParseCanceledException or RecognitionException)
        {
            Interlocked.Increment(ref _llFallbackRetries);
            RewindTokenStream(tokenStream, startTokenIndex);
            RestoreParserState(savedListeners, savedErrorHandler, savedPredictionMode);
            Interpreter.PredictionMode = PredictionMode.Ll;
            return program();
        }
    }

    /// <summary>
    /// Parses a single Pilot expression (<see cref="TestContext"/>) using two-stage <c>SLL -&gt; LL</c> prediction.
    /// </summary>
    public TestContext ParseTestFast()
    {
        var tokenStream = _input;
        if (tokenStream == null)
        {
            return test();
        }

        // Ensure BufferedTokenStream has initialized its initial token position (otherwise Index is -1)
        _ = tokenStream.La(1);
        int startTokenIndex = Math.Max(0, tokenStream.Index);
#pragma warning disable CS0618
        var savedListeners = new List<IAntlrErrorListener<IToken>>(ErrorListeners);
#pragma warning restore CS0618
        var savedErrorHandler = ErrorHandler;
        var savedPredictionMode = Interpreter.PredictionMode;

        try
        {
            RemoveErrorListeners();
            ErrorHandler = new BailErrorStrategy();
            Interpreter.PredictionMode = PredictionMode.Sll;

            var expr = test();
            Interlocked.Increment(ref _sllFastPathHits);
            RestoreParserState(savedListeners, savedErrorHandler, savedPredictionMode);
            return expr;
        }
        catch (Exception ex) when (ex is ParseCanceledException or RecognitionException)
        {
            Interlocked.Increment(ref _llFallbackRetries);
            RewindTokenStream(tokenStream, startTokenIndex);
            RestoreParserState(savedListeners, savedErrorHandler, savedPredictionMode);
            Interpreter.PredictionMode = PredictionMode.Ll;
            return test();
        }
    }

    private void RestoreParserState(
        List<IAntlrErrorListener<IToken>> savedListeners,
        IAntlrErrorStrategy savedErrorHandler,
        PredictionMode savedPredictionMode)
    {
        RemoveErrorListeners();
        foreach (var listener in savedListeners)
        {
            AddErrorListener(listener);
        }

        ErrorHandler = savedErrorHandler is BailErrorStrategy
            ? new DefaultErrorStrategy()
            : savedErrorHandler;
        Interpreter.PredictionMode = savedPredictionMode;
    }

    private static void RewindTokenStream(ITokenStream tokenStream, int startTokenIndex)
    {
        if (startTokenIndex == 0 && tokenStream is BufferedTokenStream buffered)
        {
            buffered.Reset();
        }
        else
        {
            tokenStream.Seek(startTokenIndex);
        }
    }

    /// <summary>
    /// Parses and caches an expression string into a reusable <see cref="TestContext"/> AST.
    /// Used for hot-path dynamic expression evaluation such as f-string interpolations and contracts.
    /// </summary>
    public static TestContext GetOrParseExpression(string expressionSource)
    {
        if (CachedExpressions.TryGetValue(expressionSource, out var cached))
        {
            return cached;
        }

        var inputStream = new AntlrInputStream(expressionSource);
        var lexer = new PilotLexer(inputStream);
        lexer.RemoveErrorListeners();
        var tokenStream = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokenStream);
        parser.RemoveErrorListeners();

        var tree = parser.ParseTestFast();
        if (parser.NumberOfSyntaxErrors == 0 && CachedExpressions.Count < 8192)
        {
            CachedExpressions.TryAdd(expressionSource, tree);
        }

        return tree;
    }

    /// <summary>
    /// Parses and caches a statement/program snippet into a reusable <see cref="ProgramContext"/> AST.
    /// </summary>
    public static ProgramContext GetOrParseSnippet(string snippetSource)
    {
        if (CachedSnippets.TryGetValue(snippetSource, out var cached))
        {
            return cached;
        }

        var inputStream = new AntlrInputStream(snippetSource);
        var lexer = new PilotLexer(inputStream);
        lexer.RemoveErrorListeners();
        var tokenStream = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokenStream);
        parser.RemoveErrorListeners();

        var tree = parser.ParseProgramFast();
        if (parser.NumberOfSyntaxErrors == 0 && CachedSnippets.Count < 4096)
        {
            CachedSnippets.TryAdd(snippetSource, tree);
        }

        return tree;
    }

    /// <summary>
    /// Returns the number of trailers on <paramref name="atomExpr"/> that belong directly to <paramref name="atomExpr"/>,
    /// excluding any trailing <c>..cascade</c> suffix that belongs to an enclosing unparenthesized <c>..prop = rhs</c>
    /// or <c>..[idx] = rhs</c> cascade target.
    /// </summary>
    public static int GetEffectiveTrailerCount(Atom_exprContext atomExpr)
    {
        var trailers = atomExpr.trailer();
        if (trailers == null || trailers.Length == 0)
        {
            return 0;
        }

        int suffixStart = trailers.Length;
        while (suffixStart > 0 && trailers[suffixStart - 1].cascade_trailer() != null)
        {
            suffixStart--;
        }

        if (suffixStart == trailers.Length)
        {
            return trailers.Length;
        }

        return IsRightmostUnparenthesizedLeafOfCascadeAssign(atomExpr) ? suffixStart : trailers.Length;
    }

    /// <summary>
    /// Extracts any trailing <see cref="Cascade_trailerContext"/> nodes that were syntactically attached to the
    /// rightmost unparenthesized leaf of a <c>..prop = rhs</c> or <c>..[idx] = rhs</c> assignment so they execute
    /// against the outer cascade receiver.
    /// </summary>
    public static IReadOnlyList<Cascade_trailerContext> ExtractHoistedCascades(TestContext? rhs)
    {
        if (rhs == null)
        {
            return Array.Empty<Cascade_trailerContext>();
        }

        Antlr4.Runtime.Tree.IParseTree cur = rhs;
        while (cur is ParserRuleContext rc && rc is not Atom_exprContext)
        {
            if (rc.ChildCount == 0)
            {
                return Array.Empty<Cascade_trailerContext>();
            }
            cur = rc.GetChild(rc.ChildCount - 1);
        }

        if (cur is not Atom_exprContext atomExpr)
        {
            return Array.Empty<Cascade_trailerContext>();
        }

        var trailers = atomExpr.trailer();
        if (trailers == null || trailers.Length == 0)
        {
            return Array.Empty<Cascade_trailerContext>();
        }

        int suffixStart = trailers.Length;
        while (suffixStart > 0 && trailers[suffixStart - 1].cascade_trailer() != null)
        {
            suffixStart--;
        }

        if (suffixStart == trailers.Length)
        {
            return Array.Empty<Cascade_trailerContext>();
        }

        var hoisted = new List<Cascade_trailerContext>(trailers.Length - suffixStart);
        for (int i = suffixStart; i < trailers.Length; i++)
        {
            hoisted.Add(trailers[i].cascade_trailer());
        }
        return hoisted;
    }

    private static bool IsRightmostUnparenthesizedLeafOfCascadeAssign(Atom_exprContext atomExpr)
    {
        ParserRuleContext cur = atomExpr;
        while (cur.Parent is ParserRuleContext parent)
        {
            if (parent is Cascade_targetContext ct &&
                ct.EQUAL() != null &&
                ct.LPAR() == null &&
                ct.test() == cur)
            {
                return true;
            }

            if (parent is AtomContext || parent.ChildCount == 0 || parent.GetChild(parent.ChildCount - 1) != cur)
            {
                return false;
            }

            cur = parent;
        }

        return false;
    }
}
