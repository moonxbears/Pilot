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
}
