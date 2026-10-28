using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using Antlr4.Runtime;
using Antlr4.Runtime.Atn;

namespace Pilot;

/// <summary>
/// High-performance two-stage parsing (SLL with automatic LL fallback) and
/// expression AST caching extensions for <see cref="PilotParser"/>.
/// </summary>
public partial class PilotParser
{
    private static readonly ConcurrentDictionary<string, TestContext> CachedExpressions = new(StringComparer.Ordinal);
    private static readonly ConcurrentDictionary<string, ProgramContext> CachedSnippets = new(StringComparer.Ordinal);

    /// <summary>
    /// Parses a complete Pilot program using ANTLR4's two-stage SLL -> LL parsing strategy.
    /// Valid programs parse in SLL mode without full-context ATN overhead; if any syntax
    /// error or SLL conflict occurs, the stream is rewound and re-parsed with full LL mode
    /// and all original error listeners attached.
    /// </summary>
    public ProgramContext ParseProgramFast() => program();

    /// <summary>
    /// Parses a single Pilot expression (<see cref="TestContext"/>).
    /// </summary>
    public TestContext ParseTestFast() => test();

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
