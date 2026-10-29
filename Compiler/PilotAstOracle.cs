using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text;
using System.Text.Json;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;

namespace Pilot.Compiler;

/// <summary>
/// Managed snapshot of a 16-byte <see cref="NativeAstNode"/> arena produced by lowering an ANTLR
/// <see cref="PilotParser.ProgramContext"/> parse tree or inspecting a native Rust arena.
/// </summary>
public sealed class PilotAstArenaSnapshot
{
    public List<NativeAstNode> Nodes { get; } = new();
    public List<NativeByteSpan> Spans { get; } = new();
    public List<uint> ExtraChildren { get; } = new();
    public List<long> I64Pool { get; } = new();
    public List<double> F64Pool { get; } = new();
    public List<string> SymbolPool { get; } = new();
    private readonly Dictionary<string, uint> _symbolLookup = new(StringComparer.Ordinal);
    public uint RootId { get; set; } = NativeAstNode.NoneId;

    public uint Intern(string text)
    {
        if (_symbolLookup.TryGetValue(text, out uint id))
        {
            return id;
        }
        id = (uint)SymbolPool.Count;
        SymbolPool.Add(text);
        _symbolLookup[text] = id;
        return id;
    }

    public string ResolveSymbol(uint id) =>
        id < (uint)SymbolPool.Count ? SymbolPool[(int)id] : string.Empty;

    public uint AllocNode(NativeNodeKind kind, ushort flags, NativeByteSpan span, uint lhs, uint rhs)
    {
        uint id = (uint)Nodes.Count;
        Nodes.Add(new NativeAstNode(kind, flags, span.Start, lhs, rhs));
        Spans.Add(span);
        return id;
    }

    public (uint Start, uint Len) AllocChildSlice(IReadOnlyList<uint> children)
    {
        uint start = (uint)ExtraChildren.Count;
        uint len = (uint)children.Count;
        for (int i = 0; i < children.Count; i++)
        {
            ExtraChildren.Add(children[i]);
        }
        return (start, len);
    }

    public uint AllocListNode(NativeNodeKind kind, ushort flags, NativeByteSpan span, IReadOnlyList<uint> children)
    {
        var (start, len) = AllocChildSlice(children);
        return AllocNode(kind, flags, span, start, len);
    }

    public uint AllocInt(long val, NativeByteSpan span)
    {
        uint idx = (uint)I64Pool.Count;
        I64Pool.Add(val);
        return AllocNode(NativeNodeKind.IntLit, 0, span, idx, 0);
    }

    public uint AllocFloat(double val, NativeByteSpan span)
    {
        uint idx = (uint)F64Pool.Count;
        F64Pool.Add(val);
        return AllocNode(NativeNodeKind.FloatLit, 0, span, idx, 0);
    }

    public uint AllocIdent(string name, NativeByteSpan span)
    {
        uint sym = Intern(name);
        return AllocNode(NativeNodeKind.IdentExpr, 0, span, sym, 0);
    }

    public uint AllocStr(string value, NativeByteSpan span)
    {
        uint sym = Intern(value);
        return AllocNode(NativeNodeKind.StrLit, 0, span, sym, 0);
    }

    public uint AllocKeyword(string rawKw, NativeByteSpan span)
    {
        string normalized = rawKw.StartsWith(':') ? rawKw[1..] : rawKw;
        uint sym = Intern(normalized);
        return AllocNode(NativeNodeKind.KeywordLit, 0, span, sym, 0);
    }
}

/// <summary>
/// Milestone 28.1 &amp; 28.3: Canonical AST Normalizer and Differential Conformance Oracle.
/// Lowers ANTLR <see cref="PilotParser.ProgramContext"/> trees into the 16-byte <see cref="NativeAstNode"/>
/// arena representation and serializes canonical S-expressions / JSON for differential verification against
/// the Rust <c>pilot-ast</c> / <c>pilot-ffi</c> front-end crates.
/// </summary>
public static class PilotAstOracle
{
    /// <summary>
    /// Parses <paramref name="source"/> via the two-stage <c>SLL -&gt; LL</c> fast path (<see cref="PilotParser.ParseProgramFast"/>)
    /// and lowers the resulting ANTLR <see cref="PilotParser.ProgramContext"/> into a 16-byte <see cref="PilotAstArenaSnapshot"/>.
    /// </summary>
    public static PilotAstArenaSnapshot ParseAndLowerToArena(string source, out int syntaxErrors)
    {
        var input = new AntlrInputStream(source);
        var lexer = new PilotLexer(input);
        lexer.RemoveErrorListeners();
        var tokens = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokens);
        parser.RemoveErrorListeners();

        var program = parser.ParseProgramFast();
        syntaxErrors = parser.NumberOfSyntaxErrors;
        return LowerProgramToArena(program, source);
    }

    /// <summary>
    /// Lowers an ANTLR <see cref="PilotParser.ProgramContext"/> into a 16-byte <see cref="PilotAstArenaSnapshot"/>.
    /// </summary>
    public static PilotAstArenaSnapshot LowerProgramToArena(PilotParser.ProgramContext program, string source)
    {
        var arena = new PilotAstArenaSnapshot();
        var topChildren = new List<uint>();

        var stmts = program.stmt();
        if (stmts != null)
        {
            foreach (var stmt in stmts)
            {
                uint id = LowerStmt(arena, stmt);
                if (id != NativeAstNode.NoneId)
                {
                    topChildren.Add(id);
                }
            }
        }

        uint byteLen = (uint)Encoding.UTF8.GetByteCount(source);
        arena.RootId = arena.AllocListNode(
            NativeNodeKind.Program,
            0,
            new NativeByteSpan(0, byteLen),
            topChildren);
        return arena;
    }

    /// <summary>
    /// Formats a <see cref="PilotAstArenaSnapshot"/> as a canonical S-Expression identical in layout
    /// to <c>pilot_ast::AstArena::to_sexpr</c>.
    /// </summary>
    public static string ToCanonicalSExpr(PilotAstArenaSnapshot arena)
    {
        var sb = new StringBuilder();
        WriteNodeSExpr(arena, arena.RootId, sb);
        return sb.ToString();
    }

    /// <summary>
    /// Formats a <see cref="PilotAstArenaSnapshot"/> as canonical deterministic JSON for golden AST conformance snapshots.
    /// </summary>
    public static string ToCanonicalJson(PilotAstArenaSnapshot arena)
    {
        using var ms = new System.IO.MemoryStream();
        using (var writer = new Utf8JsonWriter(ms, new JsonWriterOptions { Indented = false }))
        {
            WriteNodeJson(arena, arena.RootId, writer);
        }
        return Encoding.UTF8.GetString(ms.ToArray());
    }

    private static uint LowerStmt(PilotAstArenaSnapshot arena, PilotParser.StmtContext stmt)
    {
        if (stmt.simple_stmt() is { } simple)
        {
            var smallStmts = simple.small_stmt();
            if (smallStmts != null && smallStmts.Length > 0)
            {
                return LowerSmallStmt(arena, smallStmts[0], GetSpan(stmt));
            }
        }

        if (stmt.compound_stmt() is { } compound)
        {
            return LowerCompoundStmt(arena, compound, GetSpan(stmt));
        }

        return arena.AllocNode(NativeNodeKind.ExprStmt, 0, GetSpan(stmt), 0, 0);
    }

    private static uint LowerSmallStmt(
        PilotAstArenaSnapshot arena,
        PilotParser.Small_stmtContext small,
        NativeByteSpan span)
    {
        if (small.import_stmt() is { } imp)
        {
            if (imp.import_from() is { } impFrom)
            {
                string fromMod = impFrom.dotted_name()?.GetText() ?? string.Empty;
                uint sym = arena.Intern(fromMod);
                return arena.AllocNode(NativeNodeKind.FromImportDecl, 0, span, sym, 0);
            }

            string modPath = imp.import_name()?.dotted_as_names()?.GetText()
                ?? imp.foreign_import_stmt()?.GetText()
                ?? string.Empty;
            uint modSym = arena.Intern(modPath);
            return arena.AllocNode(NativeNodeKind.ImportDecl, 0, span, modSym, 0);
        }

        if (small.let_stmt() is { } letStmt)
        {
            string name = letStmt.binding_pattern()?.GetText() ?? "_";
            uint sym = arena.Intern(name);
            uint rhs = letStmt.test() != null ? LowerTest(arena, letStmt.test()) : NativeAstNode.NoneId;
            return arena.AllocNode(NativeNodeKind.LetStmt, 0, span, sym, rhs);
        }

        if (small.var_stmt() is { } varStmt)
        {
            string name = varStmt.binding_pattern()?.GetText() ?? "_";
            uint sym = arena.Intern(name);
            uint rhs = varStmt.test() != null ? LowerTest(arena, varStmt.test()) : NativeAstNode.NoneId;
            return arena.AllocNode(NativeNodeKind.VarStmt, 0, span, sym, rhs);
        }

        if (small.const_stmt() is { } constStmt)
        {
            string name = constStmt.binding_pattern()?.GetText() ?? "_";
            uint sym = arena.Intern(name);
            uint rhs = constStmt.test() != null ? LowerTest(arena, constStmt.test()) : NativeAstNode.NoneId;
            return arena.AllocNode(NativeNodeKind.ConstDecl, 0, span, sym, rhs);
        }

        if (small.type_alias_stmt() is { } typeAlias)
        {
            string name = typeAlias.identifier()?.GetText() ?? "Type";
            uint sym = arena.Intern(name);
            return arena.AllocNode(NativeNodeKind.TypeAdtDecl, 0, span, sym, 0);
        }

        if (small.flow_stmt() is { } flow)
        {
            if (flow.RETURN() != null)
            {
                var tests = flow.testlist()?.test();
                uint exprId = (tests != null && tests.Length > 0)
                    ? LowerTest(arena, tests[0])
                    : NativeAstNode.NoneId;
                return arena.AllocNode(NativeNodeKind.ReturnStmt, 0, span, exprId, 0);
            }

            if (flow.BREAK() != null)
            {
                return arena.AllocNode(NativeNodeKind.BreakStmt, 0, span, 0, 0);
            }

            if (flow.CONTINUE() != null)
            {
                return arena.AllocNode(NativeNodeKind.ContinueStmt, 0, span, 0, 0);
            }

            if (flow.YIELD() != null)
            {
                var tests = flow.testlist()?.test();
                uint exprId = (tests != null && tests.Length > 0)
                    ? LowerTest(arena, tests[0])
                    : NativeAstNode.NoneId;
                return arena.AllocNode(NativeNodeKind.YieldStmt, 0, span, exprId, 0);
            }
        }

        if (small.guard_stmt() is { } guardStmt)
        {
            uint condId = guardStmt.test() != null ? LowerTest(arena, guardStmt.test()) : NativeAstNode.NoneId;
            return arena.AllocNode(NativeNodeKind.GuardStmt, 0, span, condId, 0);
        }

        if (small.defer_stmt() is { })
        {
            return arena.AllocNode(NativeNodeKind.DeferStmt, 0, span, 0, 0);
        }

        if (small.expr_stmt() is { } exprStmt)
        {
            var tests = exprStmt.testlist()?.test();
            var rhsTests = exprStmt.test();
            if (rhsTests != null && rhsTests.Length > 0)
            {
                uint lhsId = (tests != null && tests.Length > 0) ? LowerTest(arena, tests[0]) : NativeAstNode.NoneId;
                uint rhsId = LowerTest(arena, rhsTests[0]);
                return arena.AllocNode(NativeNodeKind.AssignStmt, 0, span, lhsId, rhsId);
            }

            uint exprId = (tests != null && tests.Length > 0) ? LowerTest(arena, tests[0]) : NativeAstNode.NoneId;
            return arena.AllocNode(NativeNodeKind.ExprStmt, 0, span, exprId, 0);
        }

        return arena.AllocNode(NativeNodeKind.ExprStmt, 0, span, 0, 0);
    }

    private static uint LowerCompoundStmt(
        PilotAstArenaSnapshot arena,
        PilotParser.Compound_stmtContext compound,
        NativeByteSpan span)
    {
        if (compound.func_def() is { } fnDef)
        {
            string name = fnDef.identifier()?.GetText() ?? "anon";
            uint sym = arena.Intern(name);
            return arena.AllocNode(NativeNodeKind.FnDecl, 0, span, sym, 0);
        }

        if (compound.record_def() is { } recDef)
        {
            string name = recDef.identifier()?.GetText() ?? "Record";
            uint sym = arena.Intern(name);
            return arena.AllocNode(NativeNodeKind.RecordDecl, 0, span, sym, 0);
        }

        if (compound.struct_def() is { } structDef)
        {
            string name = structDef.identifier()?.GetText() ?? "Struct";
            uint sym = arena.Intern(name);
            return arena.AllocNode(NativeNodeKind.StructDecl, 0, span, sym, 0);
        }

        if (compound.class_def() is { } classDef)
        {
            string name = classDef.identifier()?.GetText() ?? "Class";
            uint sym = arena.Intern(name);
            return arena.AllocNode(NativeNodeKind.ClassDecl, 0, span, sym, 0);
        }

        if (compound.interface_def() is { } ifaceDef)
        {
            string name = ifaceDef.identifier()?.GetText() ?? "Interface";
            uint sym = arena.Intern(name);
            return arena.AllocNode(NativeNodeKind.InterfaceDecl, 0, span, sym, 0);
        }

        if (compound.extension_def() is { } extDef)
        {
            string name = extDef.type_spec()?.GetText() ?? "Extend";
            uint sym = arena.Intern(name);
            return arena.AllocNode(NativeNodeKind.ExtendDecl, 0, span, sym, 0);
        }

        if (compound.guard_stmt() is { } guardStmt)
        {
            uint condId = guardStmt.test() != null ? LowerTest(arena, guardStmt.test()) : NativeAstNode.NoneId;
            return arena.AllocNode(NativeNodeKind.GuardStmt, 0, span, condId, 0);
        }

        if (compound.defer_stmt() is { })
        {
            return arena.AllocNode(NativeNodeKind.DeferStmt, 0, span, 0, 0);
        }

        if (compound.while_stmt() is { } whileStmt)
        {
            var tests = whileStmt.test();
            uint condId = (tests != null && tests.Length > 0) ? LowerTest(arena, tests[0]) : NativeAstNode.NoneId;
            return arena.AllocNode(NativeNodeKind.WhileStmt, 0, span, condId, 0);
        }

        if (compound.for_stmt() is { } forStmt)
        {
            string iterName = forStmt.for_target()?.GetText() ?? "_";
            uint sym = arena.Intern(iterName);
            return arena.AllocNode(NativeNodeKind.ForExpr, 0, span, sym, 0);
        }

        if (compound.if_stmt() is { } ifStmt)
        {
            var tests = ifStmt.test();
            uint condId = (tests != null && tests.Length > 0) ? LowerTest(arena, tests[0]) : NativeAstNode.NoneId;
            return arena.AllocNode(NativeNodeKind.IfExpr, 0, span, condId, 0);
        }

        if (compound.match_stmt() is { } matchStmt)
        {
            uint scrutinee = matchStmt.test() != null ? LowerTest(arena, matchStmt.test()) : NativeAstNode.NoneId;
            return arena.AllocNode(NativeNodeKind.MatchExpr, 0, span, scrutinee, 0);
        }

        return arena.AllocNode(NativeNodeKind.ExprStmt, 0, span, 0, 0);
    }

    private static uint LowerTest(PilotAstArenaSnapshot arena, PilotParser.TestContext test)
    {
        var span = GetSpan(test);
        string raw = test.GetText().Trim();

        // Check top-level additive/multiplicative binary expressions
        if (TryLowerSimpleBinary(arena, raw, span, out uint binId))
        {
            return binId;
        }

        if (long.TryParse(raw, NumberStyles.Integer, CultureInfo.InvariantCulture, out long iv))
        {
            return arena.AllocInt(iv, span);
        }

        if (double.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out double fv))
        {
            return arena.AllocFloat(fv, span);
        }

        if (raw is "true" or "false")
        {
            return arena.AllocNode(NativeNodeKind.BoolLit, 0, span, raw == "true" ? 1u : 0u, 0);
        }

        if (raw is "none" or "null")
        {
            return arena.AllocNode(NativeNodeKind.NoneLit, 0, span, 0, 0);
        }

        if (raw.StartsWith(':') && raw.Length > 1 && !raw.StartsWith(":="))
        {
            return arena.AllocKeyword(raw, span);
        }

        if (raw.Length >= 2 && raw.StartsWith('"') && raw.EndsWith('"'))
        {
            return arena.AllocStr(raw[1..^1], span);
        }

        return arena.AllocIdent(raw, span);
    }

    private static bool TryLowerSimpleBinary(
        PilotAstArenaSnapshot arena,
        string exprText,
        NativeByteSpan span,
        out uint nodeId)
    {
        int plusIdx = FindTopLevelOperator(exprText, '+');
        if (plusIdx > 0)
        {
            string left = exprText[..plusIdx].Trim();
            string right = exprText[(plusIdx + 1)..].Trim();
            uint lhs = LowerAtomicText(arena, left, span);
            uint rhs = LowerAtomicText(arena, right, span);
            nodeId = arena.AllocNode(NativeNodeKind.BinaryExpr, (ushort)NativeBinaryOp.Add, span, lhs, rhs);
            return true;
        }

        int mulIdx = FindTopLevelOperator(exprText, '*');
        if (mulIdx > 0)
        {
            string left = exprText[..mulIdx].Trim();
            string right = exprText[(mulIdx + 1)..].Trim();
            uint lhs = LowerAtomicText(arena, left, span);
            uint rhs = LowerAtomicText(arena, right, span);
            nodeId = arena.AllocNode(NativeNodeKind.BinaryExpr, (ushort)NativeBinaryOp.Mul, span, lhs, rhs);
            return true;
        }

        nodeId = NativeAstNode.NoneId;
        return false;
    }

    private static uint LowerAtomicText(PilotAstArenaSnapshot arena, string text, NativeByteSpan span)
    {
        if (TryLowerSimpleBinary(arena, text, span, out uint bin))
        {
            return bin;
        }
        if (long.TryParse(text, NumberStyles.Integer, CultureInfo.InvariantCulture, out long iv))
        {
            return arena.AllocInt(iv, span);
        }
        if (double.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture, out double fv))
        {
            return arena.AllocFloat(fv, span);
        }
        if (text is "true" or "false")
        {
            return arena.AllocNode(NativeNodeKind.BoolLit, 0, span, text == "true" ? 1u : 0u, 0);
        }
        if (text is "none" or "null")
        {
            return arena.AllocNode(NativeNodeKind.NoneLit, 0, span, 0, 0);
        }
        if (text.StartsWith(':') && text.Length > 1)
        {
            return arena.AllocKeyword(text, span);
        }
        if (text.Length >= 2 && text.StartsWith('"') && text.EndsWith('"'))
        {
            return arena.AllocStr(text[1..^1], span);
        }
        return arena.AllocIdent(text, span);
    }

    private static int FindTopLevelOperator(string s, char op)
    {
        int depth = 0;
        bool inQuotes = false;
        for (int i = 0; i < s.Length; i++)
        {
            char c = s[i];
            if (c == '"' && (i == 0 || s[i - 1] != '\\'))
            {
                inQuotes = !inQuotes;
                continue;
            }
            if (inQuotes)
            {
                continue;
            }
            if (c is '(' or '[' or '{')
            {
                depth++;
            }
            else if (c is ')' or ']' or '}')
            {
                depth = Math.Max(0, depth - 1);
            }
            else if (depth == 0 && c == op)
            {
                // Avoid matching ** or +=
                if (i + 1 < s.Length && (s[i + 1] == op || s[i + 1] == '='))
                {
                    i++;
                    continue;
                }
                return i;
            }
        }
        return -1;
    }

    private static NativeByteSpan GetSpan(ParserRuleContext ctx)
    {
        uint start = ctx.Start != null && ctx.Start.StartIndex >= 0 ? (uint)ctx.Start.StartIndex : 0u;
        uint stop = ctx.Stop != null && ctx.Stop.StopIndex >= (int)start ? (uint)(ctx.Stop.StopIndex + 1) : start;
        return new NativeByteSpan(start, stop);
    }

    private static void WriteNodeSExpr(PilotAstArenaSnapshot arena, uint id, StringBuilder sb)
    {
        if (id == NativeAstNode.NoneId || id >= (uint)arena.Nodes.Count)
        {
            sb.Append("nil");
            return;
        }

        var n = arena.Nodes[(int)id];
        var kind = n.NodeKind;
        switch (kind)
        {
            case NativeNodeKind.Program:
            case NativeNodeKind.BlockExpr:
            case NativeNodeKind.TupleExpr:
            case NativeNodeKind.ListExpr:
                sb.Append('(').Append(kind);
                for (int i = 0; i < (int)n.Rhs; i++)
                {
                    int idx = (int)n.Lhs + i;
                    if (idx < arena.ExtraChildren.Count)
                    {
                        sb.Append(' ');
                        WriteNodeSExpr(arena, arena.ExtraChildren[idx], sb);
                    }
                }
                sb.Append(')');
                break;

            case NativeNodeKind.IdentExpr:
                sb.Append("(Ident ").Append(arena.ResolveSymbol(n.Lhs)).Append(')');
                break;

            case NativeNodeKind.IntLit:
                long iv = n.Lhs < (uint)arena.I64Pool.Count ? arena.I64Pool[(int)n.Lhs] : 0L;
                sb.Append("(Int ").Append(iv.ToString(CultureInfo.InvariantCulture)).Append(')');
                break;

            case NativeNodeKind.FloatLit:
                double fv = n.Lhs < (uint)arena.F64Pool.Count ? arena.F64Pool[(int)n.Lhs] : 0.0;
                sb.Append("(Float ").Append(fv.ToString("G", CultureInfo.InvariantCulture)).Append(')');
                break;

            case NativeNodeKind.BoolLit:
                sb.Append(n.Lhs != 0 ? "(Bool true)" : "(Bool false)");
                break;

            case NativeNodeKind.StrLit:
                sb.Append("(Str \"").Append(arena.ResolveSymbol(n.Lhs)).Append("\")");
                break;

            case NativeNodeKind.NoneLit:
                sb.Append("(None)");
                break;

            case NativeNodeKind.KeywordLit:
                sb.Append("(Kw :").Append(arena.ResolveSymbol(n.Lhs)).Append(')');
                break;

            case NativeNodeKind.LetStmt:
            case NativeNodeKind.VarStmt:
            case NativeNodeKind.ConstDecl:
                sb.Append('(').Append(kind).Append(' ').Append(arena.ResolveSymbol(n.Lhs)).Append(' ');
                WriteNodeSExpr(arena, n.Rhs, sb);
                sb.Append(')');
                break;

            case NativeNodeKind.ReturnStmt:
                sb.Append("(ReturnStmt ");
                WriteNodeSExpr(arena, n.Lhs, sb);
                sb.Append(')');
                break;

            case NativeNodeKind.BinaryExpr:
                sb.Append("(BinaryExpr op=").Append(n.Flags).Append(' ');
                WriteNodeSExpr(arena, n.Lhs, sb);
                sb.Append(' ');
                WriteNodeSExpr(arena, n.Rhs, sb);
                sb.Append(')');
                break;

            case NativeNodeKind.ImportDecl:
            case NativeNodeKind.FromImportDecl:
            case NativeNodeKind.FnDecl:
            case NativeNodeKind.TypeAdtDecl:
            case NativeNodeKind.RecordDecl:
            case NativeNodeKind.StructDecl:
            case NativeNodeKind.ClassDecl:
            case NativeNodeKind.InterfaceDecl:
            case NativeNodeKind.ExtendDecl:
                sb.Append('(').Append(kind).Append(' ').Append(arena.ResolveSymbol(n.Lhs)).Append(')');
                break;

            default:
                sb.Append('(').Append(kind);
                if (n.Lhs != NativeAstNode.NoneId)
                {
                    sb.Append(' ');
                    WriteNodeSExpr(arena, n.Lhs, sb);
                }
                if (n.Rhs != NativeAstNode.NoneId && n.Rhs != 0)
                {
                    sb.Append(' ');
                    WriteNodeSExpr(arena, n.Rhs, sb);
                }
                sb.Append(')');
                break;
        }
    }

    private static void WriteNodeJson(PilotAstArenaSnapshot arena, uint id, Utf8JsonWriter writer)
    {
        if (id == NativeAstNode.NoneId || id >= (uint)arena.Nodes.Count)
        {
            writer.WriteNullValue();
            return;
        }

        var n = arena.Nodes[(int)id];
        var kind = n.NodeKind;
        writer.WriteStartObject();
        writer.WriteString("kind", kind.ToString());
        writer.WriteNumber("flags", n.Flags);

        switch (kind)
        {
            case NativeNodeKind.Program:
            case NativeNodeKind.BlockExpr:
            case NativeNodeKind.TupleExpr:
            case NativeNodeKind.ListExpr:
                writer.WriteStartArray("children");
                for (int i = 0; i < (int)n.Rhs; i++)
                {
                    int idx = (int)n.Lhs + i;
                    if (idx < arena.ExtraChildren.Count)
                    {
                        WriteNodeJson(arena, arena.ExtraChildren[idx], writer);
                    }
                }
                writer.WriteEndArray();
                break;

            case NativeNodeKind.IdentExpr:
            case NativeNodeKind.StrLit:
            case NativeNodeKind.KeywordLit:
            case NativeNodeKind.ImportDecl:
            case NativeNodeKind.FromImportDecl:
            case NativeNodeKind.FnDecl:
            case NativeNodeKind.TypeAdtDecl:
            case NativeNodeKind.RecordDecl:
            case NativeNodeKind.StructDecl:
            case NativeNodeKind.ClassDecl:
            case NativeNodeKind.InterfaceDecl:
            case NativeNodeKind.ExtendDecl:
                writer.WriteString("symbol", arena.ResolveSymbol(n.Lhs));
                break;

            case NativeNodeKind.IntLit:
                writer.WriteNumber("value", n.Lhs < (uint)arena.I64Pool.Count ? arena.I64Pool[(int)n.Lhs] : 0L);
                break;

            case NativeNodeKind.FloatLit:
                writer.WriteNumber("value", n.Lhs < (uint)arena.F64Pool.Count ? arena.F64Pool[(int)n.Lhs] : 0.0);
                break;

            case NativeNodeKind.BoolLit:
                writer.WriteBoolean("value", n.Lhs != 0);
                break;

            case NativeNodeKind.LetStmt:
            case NativeNodeKind.VarStmt:
            case NativeNodeKind.ConstDecl:
                writer.WriteString("symbol", arena.ResolveSymbol(n.Lhs));
                writer.WritePropertyName("init");
                WriteNodeJson(arena, n.Rhs, writer);
                break;

            case NativeNodeKind.ReturnStmt:
            case NativeNodeKind.ExprStmt:
                writer.WritePropertyName("expr");
                WriteNodeJson(arena, n.Lhs, writer);
                break;

            case NativeNodeKind.BinaryExpr:
                writer.WritePropertyName("lhs");
                WriteNodeJson(arena, n.Lhs, writer);
                writer.WritePropertyName("rhs");
                WriteNodeJson(arena, n.Rhs, writer);
                break;
        }

        writer.WriteEndObject();
    }
}
