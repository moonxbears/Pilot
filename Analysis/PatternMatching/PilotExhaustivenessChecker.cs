using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using Antlr4.Runtime.Tree;
using Pilot.Analysis.TypeSystem;
using Pilot.Runtime.Values;

namespace Pilot.Analysis.PatternMatching;

/// <summary>
/// Implements Luc Maranget's ("Warnings for Pattern Matching", JFP 2007) usefulness matrix
/// exhaustiveness and redundancy verifier over Pilot algebraic data types (ADTs), records,
/// booleans, bounded &amp; unbounded integer intervals, character ranges, tuples, fixed/slice arrays,
/// Clojure-style `:kw` keywords, nullables, and typestates.
///
/// Diagnostics emitted:
/// - `MATCH001` (Error): Non-exhaustive `match` expression/statement with exact missing witness counterexamples.
/// - `MATCH002` (Warning): Unreachable / subsumed `case` arm completely covered by prior unguarded arms.
/// </summary>
public sealed class PilotExhaustivenessChecker : PilotBaseVisitor<object?>
{
    public const string NonExhaustiveDiagnosticCode = "MATCH001";
    public const string RedundantArmDiagnosticCode = "MATCH002";

    private readonly PilotTypeInferenceEngine _typeEngine;
    private readonly List<PilotAnalysisDiagnostic> _diagnostics = new();

    private readonly Dictionary<string, PilotAdtDefinition> _adts = new(StringComparer.Ordinal);
    private readonly Dictionary<string, PilotAdtVariantDefinition> _variants = new(StringComparer.Ordinal);
    private readonly Dictionary<string, PilotRecordDefinition> _records = new(StringComparer.Ordinal);
    private readonly Dictionary<string, List<string>> _classTypestates = new(StringComparer.Ordinal);
    private readonly Dictionary<string, PilotType> _functionReturnTypes = new(StringComparer.Ordinal);
    private readonly HashSet<string> _visitedImports = new(StringComparer.Ordinal);
    private readonly Stack<Dictionary<string, PilotType>> _localScopes = new();

    public IReadOnlyList<PilotAnalysisDiagnostic> Diagnostics => _diagnostics;
    public IReadOnlyDictionary<string, PilotAdtDefinition> AdtDefinitions => _adts;
    public IReadOnlyDictionary<string, PilotRecordDefinition> RecordDefinitions => _records;

    public PilotExhaustivenessChecker(PilotTypeInferenceEngine? typeEngine = null)
    {
        _typeEngine = typeEngine ?? new PilotTypeInferenceEngine();
        _localScopes.Push(new Dictionary<string, PilotType>(StringComparer.Ordinal));
    }

    public static IReadOnlyList<PilotAnalysisDiagnostic> CheckProgram(
        PilotParser.ProgramContext program,
        PilotTypeInferenceEngine? typeEngine = null)
    {
        if (typeEngine == null)
        {
            typeEngine = new PilotTypeInferenceEngine();
            typeEngine.VisitProgram(program);
        }
        var checker = new PilotExhaustivenessChecker(typeEngine);
        checker.CollectDeclarations(program);
        checker.VisitProgram(program);
        return checker.Diagnostics;
    }

    #region Pass 1: Collect ADT, Enum, Record & Typestate Declarations

    public void CollectDeclarations(IParseTree node)
    {
        switch (node)
        {
            case PilotParser.Type_alias_stmtContext typeAlias:
                RegisterTypeAliasAdt(typeAlias);
                break;
            case PilotParser.Enum_defContext enumDef:
                RegisterEnumAdt(enumDef);
                break;
            case PilotParser.Record_defContext recDef:
                RegisterRecord(recDef);
                break;
            case PilotParser.Struct_defContext structDef:
                RegisterStructAsRecord(structDef);
                break;
            case PilotParser.Class_defContext classDef:
                RegisterClassTypestates(classDef);
                break;
            case PilotParser.Func_defContext funcDef:
                if (funcDef.identifier() != null && funcDef.type_spec() != null)
                {
                    _functionReturnTypes[funcDef.identifier().GetText()] = ParseRichType(funcDef.type_spec().GetText());
                }
                break;
            case PilotParser.Import_stmtContext impStmt:
                CollectImportedDeclarations(impStmt);
                break;
        }

        for (int i = 0; i < node.ChildCount; i++)
        {
            CollectDeclarations(node.GetChild(i));
        }

        RefreshLayouts();
    }

    private void CollectImportedDeclarations(PilotParser.Import_stmtContext imp)
    {
        if (imp.import_name()?.dotted_as_names()?.dotted_as_name() is { } specs)
        {
            foreach (var spec in specs)
            {
                string dotted = spec.dotted_name().GetText();
                if (Pilot.Runtime.Stdlib.PilotStdlibResolver.TryGetParsedModuleAst(dotted, 0, null, out var modAst, out string modId) &&
                    modAst != null && _visitedImports.Add(modId))
                {
                    CollectDeclarations(modAst);
                }
            }
        }
        else if (imp.import_from() is { } fromCtx)
        {
            int dots = fromCtx.DOT()?.Length ?? 0;
            string dotted = fromCtx.dotted_name()?.GetText() ?? string.Empty;
            if (Pilot.Runtime.Stdlib.PilotStdlibResolver.TryGetParsedModuleAst(dotted, dots, null, out var modAst, out string modId) &&
                modAst != null && _visitedImports.Add(modId))
            {
                CollectDeclarations(modAst);
            }
        }
    }

    private void RegisterTypeAliasAdt(PilotParser.Type_alias_stmtContext ctx)
    {
        string name = ctx.identifier().GetText();
        var gps = ExtractGenericParams(ctx.generic_params());

        if (ctx.adt_sum_body() != null)
        {
            var adt = new PilotAdtDefinition(name, gps);
            var sum = ctx.adt_sum_body();
            for (int i = 0; i < sum.ChildCount; i++)
            {
                var ch = sum.GetChild(i);
                if (ch is PilotParser.IdentifierContext uid)
                    adt.AddVariant(uid.GetText(), Array.Empty<PilotAdtFieldDefinition>());
                else if (ch is PilotParser.Adt_variantContext vc)
                    adt.AddVariant(vc.identifier().GetText(), ExtractFields(vc.adt_field_list(), vc.struct_member()));
                else if (ch is PilotParser.Adt_variant_payloadContext vpc)
                    adt.AddVariant(vpc.identifier().GetText(), ExtractFields(vpc.adt_field_list(), vpc.struct_member()));
            }
            StoreAdt(adt);
            return;
        }

        // Also detect unit-only PascalCase union sum types like `type Status = Pending | Running | Done`
        var branches = ctx.type_spec()?.union_type()?.intersection_type();
        if (branches != null && branches.Length >= 2)
        {
            var names = new List<string>();
            foreach (var b in branches)
            {
                string t = b.GetText().Trim();
                if (string.IsNullOrEmpty(t) || !char.IsUpper(t[0]) || !t.All(c => char.IsLetterOrDigit(c) || c == '_') || IsBuiltinTypeName(t))
                    return;
                names.Add(t);
            }
            var adt = new PilotAdtDefinition(name, gps);
            foreach (var n in names) adt.AddVariant(n, Array.Empty<PilotAdtFieldDefinition>());
            StoreAdt(adt);
        }
    }

    private void RegisterEnumAdt(PilotParser.Enum_defContext ctx)
    {
        string name = ctx.identifier().GetText();
        var gps = ExtractGenericParams(ctx.generic_params());
        var adt = new PilotAdtDefinition(name, gps);
        if (ctx.enum_suite()?.enum_member() != null)
        {
            foreach (var m in ctx.enum_suite().enum_member())
            {
                adt.AddVariant(m.identifier().GetText(), ExtractFields(m.adt_field_list(), m.struct_member()));
            }
        }
        StoreAdt(adt);
    }

    private void RegisterRecord(PilotParser.Record_defContext ctx)
    {
        string name = ctx.identifier().GetText();
        var gps = ExtractGenericParams(ctx.generic_params());
        var fields = ExtractFields(ctx.adt_field_list(), ctx.struct_member());
        _records[name] = new PilotRecordDefinition(name, fields, gps);
    }

    private void RegisterStructAsRecord(PilotParser.Struct_defContext ctx)
    {
        string name = ctx.identifier().GetText();
        if (_records.ContainsKey(name)) return;
        var gps = ExtractGenericParams(ctx.generic_params());
        var fields = ExtractFields(null, ctx.struct_member());
        _records[name] = new PilotRecordDefinition(name, fields, gps);
    }

    private void RegisterClassTypestates(PilotParser.Class_defContext ctx)
    {
        string className = ctx.identifier().GetText();
        if (ctx.suite()?.stmt() == null) return;
        var states = new List<string>();
        foreach (var stmt in ctx.suite().stmt())
        {
            if (stmt.compound_stmt()?.state_def() is { } sd)
            {
                states.Add(sd.identifier().GetText());
            }
        }
        if (states.Count > 0)
        {
            _classTypestates[className] = states;
        }
    }

    private void StoreAdt(PilotAdtDefinition adt)
    {
        _adts[adt.Name] = adt;
        foreach (var v in adt.Variants)
        {
            _variants[v.Name] = v;
        }
    }

    private void RefreshLayouts()
    {
        foreach (var adt in _adts.Values)
            adt.Layout = PilotAdtMemoryLayout.ComputeForAdt(adt, _adts, _records);
        foreach (var rec in _records.Values)
            rec.Layout = PilotAdtMemoryLayout.ComputeForRecord(rec, _adts, _records);
    }

    private static IReadOnlyList<string> ExtractGenericParams(PilotParser.Generic_paramsContext? gp) =>
        gp == null ? Array.Empty<string>() : gp.generic_param().Select(p => p.identifier().GetText()).ToArray();

    private static IReadOnlyList<PilotAdtFieldDefinition> ExtractFields(
        PilotParser.Adt_field_listContext? fl,
        PilotParser.Struct_memberContext[]? members)
    {
        var list = new List<PilotAdtFieldDefinition>();
        if (fl?.adt_field() != null)
        {
            var af = fl.adt_field();
            for (int i = 0; i < af.Length; i++)
            {
                list.Add(new PilotAdtFieldDefinition(af[i].identifier()?.GetText(), af[i].type_spec().GetText(), i, af[i].test()));
            }
        }
        else if (members != null)
        {
            for (int i = 0; i < members.Length; i++)
            {
                list.Add(new PilotAdtFieldDefinition(
                    members[i].identifier().GetText(),
                    members[i].type_annotation()?.type_spec()?.GetText() ?? "any",
                    i,
                    members[i].test()));
            }
        }
        return list;
    }

    #endregion

    #region Pass 2: AST Walk & Scope Type Tracking

    public override object? VisitFunc_def(PilotParser.Func_defContext context)
    {
        var scope = new Dictionary<string, PilotType>(StringComparer.Ordinal);
        if (context.parameters()?.parameter() != null)
        {
            foreach (var p in context.parameters().parameter())
            {
                string? pName = p.identifier()?.GetText();
                string? pType = p.type_annotation()?.type_spec()?.GetText();
                if (!string.IsNullOrEmpty(pName))
                {
                    scope[pName!] = !string.IsNullOrEmpty(pType) ? ParseRichType(pType!) : PilotType.Any;
                }
            }
        }
        _localScopes.Push(scope);
        try
        {
            if (context.suite() != null) Visit(context.suite());
        }
        finally
        {
            _localScopes.Pop();
        }
        return null;
    }

    public override object? VisitLet_stmt(PilotParser.Let_stmtContext context)
    {
        string? name = context.binding_pattern()?.identifier()?.GetText();
        if (!string.IsNullOrEmpty(name))
        {
            var t = ResolveDeclType(context.type_annotation()?.type_spec()?.GetText(), context.test());
            _localScopes.Peek()[name!] = t;
        }
        if (context.test() != null) Visit(context.test());
        return null;
    }

    public override object? VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        string? name = context.binding_pattern()?.identifier()?.GetText();
        if (!string.IsNullOrEmpty(name))
        {
            var t = ResolveDeclType(context.type_annotation()?.type_spec()?.GetText(), context.test());
            _localScopes.Peek()[name!] = t;
        }
        if (context.test() != null) Visit(context.test());
        return null;
    }

    public override object? VisitMatch_stmt(PilotParser.Match_stmtContext context)
    {
        VerifyMatch(context);
        return base.VisitMatch_stmt(context);
    }

    public override object? VisitCoalesce_expr(PilotParser.Coalesce_exprContext context)
    {
        var catches = context.catch_clause();
        if (catches == null || catches.Length == 0)
            return base.VisitCoalesce_expr(context);

        var orTests = context.or_test();
        if (orTests != null && orTests.Length > 0)
            Visit(orTests[0]);

        var lhsType = orTests != null && orTests.Length > 0
            ? InferExpressionTextType(orTests[0].GetText().Trim())
            : PilotType.Any;
        var errType = lhsType.IsResult ? lhsType.TypeArguments[1] : PilotType.Any;

        foreach (var cc in catches)
        {
            var catchScope = new Dictionary<string, PilotType>(_localScopes.Peek(), StringComparer.Ordinal);
            if (cc.identifier() != null)
            {
                catchScope[cc.identifier().GetText()] = errType;
            }
            _localScopes.Push(catchScope);
            try
            {
                Visit(cc);
            }
            finally
            {
                _localScopes.Pop();
            }
        }
        return null;
    }

    private PilotType InferExpressionTextType(string txt)
    {
        foreach (var sc in _localScopes)
        {
            if (sc.TryGetValue(txt, out var localT) && localT.Kind != PilotTypeKind.Any)
                return localT;
        }
        int parenIdx = txt.IndexOf('(');
        if (parenIdx > 0 && txt.EndsWith(")", StringComparison.Ordinal))
        {
            string callee = txt[..parenIdx].Trim();
            int genericBr = callee.IndexOf('[');
            string baseCallee = genericBr > 0 ? callee[..genericBr] : callee;
            if (_functionReturnTypes.TryGetValue(baseCallee, out var fnRet))
                return fnRet;
        }
        return PilotType.Any;
    }

    private PilotType ResolveDeclType(string? explicitType, PilotParser.TestContext? rhs)
    {
        if (!string.IsNullOrWhiteSpace(explicitType))
            return ParseRichType(explicitType!);
        if (rhs != null)
            return InferScrutineeType(rhs);
        return PilotType.Any;
    }

    private PilotType InferScrutineeType(PilotParser.TestContext? expr)
    {
        if (expr == null) return PilotType.Any;
        string txt = expr.GetText().Trim();

        // Simple identifier lookup in local / engine scopes
        foreach (var sc in _localScopes)
        {
            if (sc.TryGetValue(txt, out var localT) && localT.Kind != PilotTypeKind.Any)
                return localT;
        }
        if (_typeEngine.InferredTypes.TryGetValue(txt, out var engT) && engT.Kind != PilotTypeKind.Any && engT.Kind != PilotTypeKind.TypeVariable)
        {
            return engT;
        }

        // Unit variant identifier: `Origin` or `Shape.Origin`
        if (_variants.TryGetValue(txt, out var uv))
            return new PilotType(PilotTypeKind.Primitive, uv.ParentAdt.Name);

        int dotIdx = txt.IndexOf('.');
        if (dotIdx > 0)
        {
            string prefix = txt[..dotIdx];
            if (_adts.ContainsKey(prefix))
                return new PilotType(PilotTypeKind.Primitive, prefix);
        }

        // Constructor call: `Circle(5.0)` or `Shape.Circle(5.0)` or `Point(1.0, 2.0)`
        int parenIdx = txt.IndexOf('(');
        if (parenIdx > 0 && txt.EndsWith(")", StringComparison.Ordinal))
        {
            string callee = txt[..parenIdx].Trim();
            int genericBr = callee.IndexOf('[');
            string baseCallee = genericBr > 0 ? callee[..genericBr] : callee;
            if (baseCallee.Contains('.'))
            {
                string adtPrefix = baseCallee[..baseCallee.IndexOf('.')];
                if (_adts.ContainsKey(adtPrefix))
                    return ParseRichType(adtPrefix);
            }
            if (_variants.TryGetValue(baseCallee, out var vDef))
                return new PilotType(PilotTypeKind.Primitive, vDef.ParentAdt.Name);
            if (_records.ContainsKey(baseCallee))
                return new PilotType(PilotTypeKind.Primitive, baseCallee);
            if (_functionReturnTypes.TryGetValue(baseCallee, out var fnRet))
                return fnRet;
        }

        // Tuple literal: `(a, b)`
        if (expr.pipe_expr()?.Length == 1)
        {
            var atom = TryExtractAtom(expr);
            if (atom?.LPAR() != null && atom.testlist_comp()?.test() is { Length: > 1 } tupleElems)
            {
                return PilotType.TupleOf(tupleElems.Select(InferScrutineeType).ToArray());
            }
            if (atom?.collection_expr() is { } col)
            {
                var elems = col.collection_element();
                var elemT = elems != null && elems.Length > 0 ? InferScrutineeType(elems[0].test()) : PilotType.Any;
                return PilotType.ListOf(elemT);
            }
            if (atom?.symbol_literal() != null)
            {
                return new PilotType(PilotTypeKind.Primitive, "keyword");
            }
            if (atom?.literal() is { } lit)
            {
                if (lit.TRUE() != null || lit.FALSE() != null) return PilotType.Bool;
                if (lit.INTEGER() != null || lit.HEX_INTEGER() != null || lit.BIN_INTEGER() != null || lit.OCT_INTEGER() != null) return PilotType.Int;
                if (lit.FLOAT_NUMBER() != null) return PilotType.Float;
                if (lit.STRING().Length > 0 || lit.FSTRING().Length > 0 || lit.TRIPLE_STRING().Length > 0) return PilotType.String;
            }
        }

        return PilotType.Any;
    }

    private static PilotParser.AtomContext? TryExtractAtom(PilotParser.TestContext test)
    {
        var atomExpr = test.pipe_expr(0)?.set_query_expr(0)?.coalesce_expr(0)?.or_test(0)?
            .and_test(0)?.not_test(0)?.comparison()?.range_expr(0)?.bitor_expr(0)?
            .bitxor_expr(0)?.bitand_expr(0)?.shift_expr(0)?.arith_expr(0)?.term(0)?
            .factor(0)?.power()?.atom_expr();
        return atomExpr != null && (atomExpr.trailer() == null || atomExpr.trailer().Length == 0) ? atomExpr.atom() : null;
    }

    public PilotType ParseRichType(string typeStr)
    {
        if (string.IsNullOrWhiteSpace(typeStr)) return PilotType.Any;
        string s = typeStr.Trim();
        if (s.StartsWith("ref ", StringComparison.Ordinal)) s = s[4..].Trim();

        // Milestone 13.1: Fallible return type `T ! E` or inferred `T!`
        if (PilotAdtMemoryLayout.TrySplitFallibleTypeSpec(s, out string okSpec, out string errSpec))
        {
            var okType = ParseRichType(okSpec);
            var errType = string.IsNullOrEmpty(errSpec) ? PilotType.Any : ParseRichType(errSpec);
            return PilotType.ResultOf(okType, errType);
        }

        // Top-level union `IoError | ParseError`
        var unionParts = SplitTopLevel(s, '|');
        if (unionParts.Count > 1)
        {
            return PilotType.UnionOf(unionParts.Select(ParseRichType).ToArray());
        }

        if (s.EndsWith("?", StringComparison.Ordinal))
            return PilotType.OptionOf(ParseRichType(s[..^1]));

        // Postfix slice array: `Point[]`, `int[][]`
        if (s.EndsWith("[]", StringComparison.Ordinal))
            return PilotType.ListOf(ParseRichType(s[..^2]));

        // Fixed-size or slice array: `[int; 4]` or `[int]`
        if (s.StartsWith("[", StringComparison.Ordinal) && s.EndsWith("]", StringComparison.Ordinal))
        {
            string inner = s[1..^1].Trim();
            int semi = inner.IndexOf(';');
            if (semi > 0 && int.TryParse(inner[(semi + 1)..].Trim(), out int fixedLen))
            {
                return new PilotType(PilotTypeKind.Collection, $"FixedArray[{fixedLen}]", new[] { ParseRichType(inner[..semi].Trim()) });
            }
            return PilotType.ListOf(ParseRichType(inner));
        }

        // Tuple `(T1, T2)`
        if (s.StartsWith("(", StringComparison.Ordinal) && s.EndsWith(")", StringComparison.Ordinal) && !s.Contains("->"))
        {
            var parts = SplitTopLevel(s[1..^1], ',').Select(ParseRichType).ToArray();
            return PilotType.TupleOf(parts);
        }

        // `List[T]` or `List<T>`
        if ((s.StartsWith("List[", StringComparison.OrdinalIgnoreCase) && s.EndsWith("]", StringComparison.Ordinal)) ||
            (s.StartsWith("List<", StringComparison.OrdinalIgnoreCase) && s.EndsWith(">", StringComparison.Ordinal)) ||
            (s.StartsWith("Array[", StringComparison.OrdinalIgnoreCase) && s.EndsWith("]", StringComparison.Ordinal)))
        {
            int open = s.IndexOfAny(new[] { '[', '<' });
            return PilotType.ListOf(ParseRichType(s[(open + 1)..^1]));
        }

        // Generic ADT / Record `Expr[int]`
        int genOpen = s.IndexOf('[');
        if (genOpen > 0 && s.EndsWith("]", StringComparison.Ordinal))
        {
            string baseName = s[..genOpen].Trim();
            var args = SplitTopLevel(s[(genOpen + 1)..^1], ',').Select(ParseRichType).ToArray();
            return new PilotType(PilotTypeKind.Primitive, baseName, args);
        }

        // Preserve exact integer width names (`u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `int`)
        string lower = s.ToLowerInvariant();
        if (lower is "u8" or "byte") return new PilotType(PilotTypeKind.Primitive, "u8");
        if (lower == "i8") return new PilotType(PilotTypeKind.Primitive, "i8");
        if (lower == "u16") return new PilotType(PilotTypeKind.Primitive, "u16");
        if (lower == "i16") return new PilotType(PilotTypeKind.Primitive, "i16");
        if (lower == "u32") return new PilotType(PilotTypeKind.Primitive, "u32");
        if (lower == "i32") return new PilotType(PilotTypeKind.Primitive, "i32");
        if (lower is "bool" or "boolean") return PilotType.Bool;
        if (lower is "int" or "i64" or "long") return PilotType.Int;
        if (lower is "float" or "f64" or "f32" or "double") return PilotType.Float;
        if (lower is "str" or "string") return PilotType.String;
        if (lower == "char") return new PilotType(PilotTypeKind.Primitive, "char");
        if (lower is "symbol" or "keyword") return new PilotType(PilotTypeKind.Primitive, "keyword");
        if (lower is "any" or "dynamic") return PilotType.Any;

        return PilotTypeInferenceEngine.ParseTypeString(s);
    }

    #endregion

    #region Maranget Pattern Algebra & Usefulness Matrix

    private abstract record MatchPat;
    private sealed record WildPat : MatchPat
    {
        public static readonly WildPat Instance = new();
    }
    private sealed record OrPat(IReadOnlyList<MatchPat> Branches) : MatchPat;
    private sealed record CtorPat(CtorDesc Ctor, IReadOnlyList<MatchPat> Args) : MatchPat;

    private enum CtorFamily
    {
        AdtVariant,
        Record,
        Tuple,
        Bool,
        IntInterval,
        CharInterval,
        ListExact,
        ListAtLeast,
        Keyword,
        Typestate,
        Null,
        NonNull,
        ResultOk,
        ResultErr,
        OpenValue
    }

    private sealed record CtorDesc(
        CtorFamily Family,
        string Name,
        int Arity,
        long Lo = 0,
        long Hi = 0,
        string? OwnerType = null);

    private void VerifyMatch(PilotParser.Match_stmtContext matchCtx)
    {
        var arms = matchCtx.match_suite()?.match_arm();
        if (arms == null || arms.Length == 0)
        {
            _diagnostics.Add(new PilotAnalysisDiagnostic(
                NonExhaustiveDiagnosticCode,
                "Non-exhaustive 'match': match block has no arms. Missing pattern: _",
                matchCtx.Start.Line,
                matchCtx.Start.Column,
                isError: true));
            return;
        }

        var scrutineeType = InferScrutineeType(matchCtx.test());
        var parsedArmPatterns = new List<MatchPat>(arms.Length);
        for (int i = 0; i < arms.Length; i++)
        {
            parsedArmPatterns.Add(LowerPattern(arms[i].pattern(), scrutineeType));
        }

        // If scrutinee type was unannotated (`any` or type variable), refine it from unambiguous arm constructors!
        if (scrutineeType.Kind is PilotTypeKind.Any or PilotTypeKind.TypeVariable)
        {
            scrutineeType = RefineTypeFromPatterns(parsedArmPatterns, scrutineeType);
            if (matchCtx.test() == null && scrutineeType.Kind is PilotTypeKind.Any or PilotTypeKind.TypeVariable)
            {
                return;
            }
            // Re-lower patterns with refined scrutinee type so positional/named fields map cleanly
            for (int i = 0; i < arms.Length; i++)
            {
                parsedArmPatterns[i] = LowerPattern(arms[i].pattern(), scrutineeType);
            }
        }

        var colTypes = new[] { scrutineeType };
        var effectiveMatrix = new List<MatchPat[]>();

        for (int i = 0; i < arms.Length; i++)
        {
            var arm = arms[i];
            var pat = parsedArmPatterns[i];
            var guardExpr = arm.test();
            bool? staticGuard = TryEvaluateStaticBoolGuard(guardExpr);

            if (staticGuard == false)
            {
                _diagnostics.Add(new PilotAnalysisDiagnostic(
                    RedundantArmDiagnosticCode,
                    $"Unreachable match arm: guard condition `{guardExpr!.GetText()}` is statically false.",
                    arm.Start.Line,
                    arm.Start.Column,
                    isError: false));
                continue;
            }

            bool useful = IsUseful(effectiveMatrix, new[] { pat }, colTypes);
            if (!useful)
            {
                _diagnostics.Add(new PilotAnalysisDiagnostic(
                    RedundantArmDiagnosticCode,
                    $"Unreachable match arm: pattern `{arm.pattern().GetText()}` is completely subsumed by earlier arms.",
                    arm.Start.Line,
                    arm.Start.Column,
                    isError: false));
            }

            // Per Maranget & Milestone 11.2: arms with arbitrary runtime `when`/`if` guards do NOT
            // contribute to exhaustiveness or subsume subsequent arms unless statically proven true!
            if (guardExpr == null || staticGuard == true)
            {
                effectiveMatrix.Add(new[] { pat });
            }
        }

        // Check exhaustiveness: is WildPat useful against `effectiveMatrix`?
        // For shorthand `match { ... }` lambdas without an explicit scrutinee, only enforce MATCH001 on closed finite ADT/bool domains.
        bool enforceExhaustiveness = matchCtx.test() != null ||
            scrutineeType.Equals(PilotType.Bool) ||
            scrutineeType.Kind == PilotTypeKind.Option ||
            scrutineeType.Name.StartsWith("Result[", StringComparison.Ordinal) ||
            _adts.ContainsKey(scrutineeType.Name);
        if (enforceExhaustiveness && IsUseful(effectiveMatrix, new MatchPat[] { WildPat.Instance }, colTypes))
        {
            var witnesses = FindMissingWitnesses(effectiveMatrix, colTypes, maxWitnesses: 4);
            string witnessText = witnesses.Count > 0 ? string.Join(" | ", witnesses) : "_";
            string scrutineeLabel = matchCtx.test()?.GetText() ?? "<match>";
            _diagnostics.Add(new PilotAnalysisDiagnostic(
                NonExhaustiveDiagnosticCode,
                $"Non-exhaustive match on `{scrutineeLabel}` (type `{scrutineeType}`). Missing pattern: {witnessText}",
                matchCtx.Start.Line,
                matchCtx.Start.Column,
                isError: true));
        }
    }

    private PilotType RefineTypeFromPatterns(IReadOnlyList<MatchPat> patterns, PilotType current)
    {
        foreach (var p in patterns)
        {
            var inferred = TryInferTypeFromSinglePat(p);
            if (inferred != null && inferred.Kind != PilotTypeKind.Any)
                return inferred;
        }
        return current;
    }

    private PilotType? TryInferTypeFromSinglePat(MatchPat pat)
    {
        switch (pat)
        {
            case CtorPat cp:
                return cp.Ctor.Family switch
                {
                    CtorFamily.AdtVariant when cp.Ctor.OwnerType != null => new PilotType(PilotTypeKind.Primitive, cp.Ctor.OwnerType),
                    CtorFamily.Record when cp.Ctor.OwnerType != null => new PilotType(PilotTypeKind.Primitive, cp.Ctor.OwnerType),
                    CtorFamily.Bool => PilotType.Bool,
                    CtorFamily.IntInterval => PilotType.Int,
                    CtorFamily.CharInterval => new PilotType(PilotTypeKind.Primitive, "char"),
                    CtorFamily.ListExact or CtorFamily.ListAtLeast => PilotType.ListOf(PilotType.Any),
                    CtorFamily.Tuple => PilotType.TupleOf(Enumerable.Repeat(PilotType.Any, cp.Ctor.Arity).ToArray()),
                    CtorFamily.Typestate when cp.Ctor.OwnerType != null => new PilotType(PilotTypeKind.Primitive, cp.Ctor.OwnerType),
                    CtorFamily.Keyword => new PilotType(PilotTypeKind.Primitive, "keyword"),
                    CtorFamily.NonNull when cp.Ctor.Arity == 1 => PilotType.OptionOf(
                        cp.Args.Count == 1 ? (TryInferTypeFromSinglePat(cp.Args[0]) ?? PilotType.Any) : PilotType.Any),
                    CtorFamily.ResultOk when cp.Ctor.Arity == 1 => PilotType.ResultOf(
                        cp.Args.Count == 1 ? (TryInferTypeFromSinglePat(cp.Args[0]) ?? PilotType.Any) : PilotType.Any,
                        PilotType.Any),
                    CtorFamily.ResultErr when cp.Ctor.Arity == 1 => PilotType.ResultOf(
                        PilotType.Any,
                        cp.Args.Count == 1 ? (TryInferTypeFromSinglePat(cp.Args[0]) ?? PilotType.Any) : PilotType.Any),
                    _ => null
                };
            case OrPat op:
                foreach (var b in op.Branches)
                {
                    var sub = TryInferTypeFromSinglePat(b);
                    if (sub != null) return sub;
                }
                return null;
            default:
                return null;
        }
    }

    private static bool? TryEvaluateStaticBoolGuard(PilotParser.TestContext? guard)
    {
        if (guard == null) return true;
        string txt = guard.GetText().Trim();
        if (txt is "true" or "True") return true;
        if (txt is "false" or "False") return false;
        return null;
    }

    private MatchPat LowerPattern(PilotParser.PatternContext? ctx, PilotType expectedType)
    {
        if (ctx == null) return WildPat.Instance;
        return LowerOrPattern(ctx.or_pattern(), expectedType);
    }

    private MatchPat LowerOrPattern(PilotParser.Or_patternContext ctx, PilotType expectedType)
    {
        var branches = ctx.and_pattern();
        if (branches.Length == 1) return LowerAndPattern(branches[0], expectedType);
        return new OrPat(branches.Select(b => LowerAndPattern(b, expectedType)).ToArray());
    }

    private MatchPat LowerAndPattern(PilotParser.And_patternContext ctx, PilotType expectedType)
    {
        var parts = ctx.not_pattern();
        if (parts.Length == 1) return LowerNotPattern(parts[0], expectedType);

        // Intersect integer intervals in conjunctive patterns like `>= 0 and <= 255`!
        var lowered = parts.Select(p => LowerNotPattern(p, expectedType)).ToList();
        if (lowered.All(p => p is CtorPat { Ctor.Family: CtorFamily.IntInterval }))
        {
            long lo = long.MinValue;
            long hi = long.MaxValue;
            foreach (var cp in lowered.Cast<CtorPat>())
            {
                lo = Math.Max(lo, cp.Ctor.Lo);
                hi = Math.Min(hi, cp.Ctor.Hi);
            }
            if (lo <= hi)
            {
                return MakeIntInterval(lo, hi);
            }
        }

        // For general conjunctions (`not null and > 0`, etc.), return the most specific constructor pattern
        return lowered.FirstOrDefault(p => p is not WildPat) ?? WildPat.Instance;
    }

    private MatchPat LowerNotPattern(PilotParser.Not_patternContext ctx, PilotType expectedType)
    {
        if (ctx.NOT() != null)
        {
            var inner = LowerNotPattern(ctx.not_pattern(), expectedType);
            return NegatePattern(inner, expectedType);
        }

        // `@` whole-value binding `sq @ Rectangle(w, h)` has identical coverage to `Rectangle(w, h)`
        if (ctx.AT() != null)
        {
            return LowerNotPattern(ctx.not_pattern(), expectedType);
        }

        return LowerPrimaryPattern(ctx.primary_pattern(), expectedType);
    }

    private MatchPat NegatePattern(MatchPat inner, PilotType expectedType)
    {
        if (inner is CtorPat cp)
        {
            if (cp.Ctor.Family == CtorFamily.Null)
                return new CtorPat(new CtorDesc(CtorFamily.NonNull, "not null", 0), Array.Empty<MatchPat>());
            if (cp.Ctor.Family == CtorFamily.NonNull)
                return new CtorPat(new CtorDesc(CtorFamily.Null, "null", 0), Array.Empty<MatchPat>());
            if (cp.Ctor.Family == CtorFamily.Bool)
            {
                bool neg = cp.Ctor.Name != "true";
                return new CtorPat(new CtorDesc(CtorFamily.Bool, neg ? "true" : "false", 0), Array.Empty<MatchPat>());
            }
            if (cp.Ctor.Family == CtorFamily.IntInterval)
            {
                var (dMin, dMax) = GetIntDomainBounds(expectedType);
                var branches = new List<MatchPat>();
                if (cp.Ctor.Lo > dMin) branches.Add(MakeIntInterval(dMin, cp.Ctor.Lo - 1));
                if (cp.Ctor.Hi < dMax) branches.Add(MakeIntInterval(cp.Ctor.Hi + 1, dMax));
                if (branches.Count == 1) return branches[0];
                if (branches.Count > 1) return new OrPat(branches);
            }
            if (cp.Ctor.Family == CtorFamily.AdtVariant && cp.Ctor.OwnerType != null &&
                _adts.TryGetValue(cp.Ctor.OwnerType, out var adt))
            {
                var others = adt.Variants
                    .Where(v => v.Name != cp.Ctor.Name)
                    .Select(v => new CtorPat(
                        new CtorDesc(CtorFamily.AdtVariant, v.Name, v.Arity, OwnerType: adt.Name),
                        Enumerable.Repeat<MatchPat>(WildPat.Instance, v.Arity).ToArray()))
                    .Cast<MatchPat>()
                    .ToList();
                if (others.Count == 1) return others[0];
                if (others.Count > 1) return new OrPat(others);
            }
        }
        return new CtorPat(new CtorDesc(CtorFamily.OpenValue, $"not({inner})", 0), Array.Empty<MatchPat>());
    }

    private MatchPat LowerPrimaryPattern(PilotParser.Primary_patternContext? ctx, PilotType expectedType)
    {
        if (ctx == null || ctx.UNDERSCORE() != null)
            return WildPat.Instance;

        if (ctx.range_pattern() != null)
            return LowerRangePattern(ctx.range_pattern(), expectedType);

        if (ctx.signed_number_pattern() != null)
        {
            var sn = ctx.signed_number_pattern();
            if (sn.FLOAT_NUMBER() != null)
                return new CtorPat(new CtorDesc(CtorFamily.OpenValue, sn.GetText(), 0), Array.Empty<MatchPat>());
            long v = -ParseUnsignedLong(sn.INTEGER()?.GetText(), sn.HEX_INTEGER()?.GetText(), sn.BIN_INTEGER()?.GetText(), sn.OCT_INTEGER()?.GetText());
            return MakeIntInterval(v, v);
        }

        if (ctx.literal() != null)
            return LowerLiteralPattern(ctx.literal());

        if (ctx.keyword_pattern() != null)
        {
            var kwCtx = ctx.keyword_pattern();
            string kwName = kwCtx.symbol_literal().GetText();
            if (!kwName.StartsWith(":", StringComparison.Ordinal)) kwName = ":" + kwName;
            if (kwCtx.LPAR() == null)
                return new CtorPat(new CtorDesc(CtorFamily.Keyword, kwName, 0), Array.Empty<MatchPat>());

            var pArgs = kwCtx.pattern_arg();
            var subPats = pArgs != null
                ? pArgs.Select(a => LowerPattern(a.pattern(), PilotType.Any)).ToArray()
                : Array.Empty<MatchPat>();
            return new CtorPat(new CtorDesc(CtorFamily.Keyword, kwName, subPats.Length), subPats);
        }

        if (ctx.relational_pattern() != null)
            return LowerRelationalPattern(ctx.relational_pattern(), expectedType);

        if (ctx.collection_pattern() != null)
            return LowerCollectionPattern(ctx.collection_pattern(), expectedType);

        if (ctx.tuple_pattern_match() != null)
        {
            var pats = ctx.tuple_pattern_match().pattern();
            bool hasComma = ctx.tuple_pattern_match().COMMA()?.Length > 0;
            if (pats.Length == 1 && !hasComma)
                return LowerPattern(pats[0], expectedType);

            var elemTypes = expectedType.Kind == PilotTypeKind.Tuple && expectedType.TypeArguments.Count == pats.Length
                ? expectedType.TypeArguments
                : Enumerable.Repeat(PilotType.Any, pats.Length).ToArray();
            var loweredElems = new MatchPat[pats.Length];
            for (int i = 0; i < pats.Length; i++)
                loweredElems[i] = LowerPattern(pats[i], elemTypes[i]);

            return new CtorPat(new CtorDesc(CtorFamily.Tuple, "Tuple", pats.Length), loweredElems);
        }

        if (ctx.property_pattern() != null)
            return LowerPropertyPattern(ctx.property_pattern(), expectedType);

        if (ctx.type_or_var_pattern() != null)
            return LowerTypeOrVarPattern(ctx.type_or_var_pattern(), expectedType);

        return WildPat.Instance;
    }

    private MatchPat LowerRangePattern(PilotParser.Range_patternContext ctx, PilotType expectedType)
    {
        var bounds = ctx.range_bound();
        bool inclusive = ctx.DOTDOT_LESS() == null;
        if (bounds[0].STRING() != null && bounds[1].STRING() != null)
        {
            string s0 = StripQuotes(bounds[0].STRING().GetText());
            string s1 = StripQuotes(bounds[1].STRING().GetText());
            if (s0.Length == 1 && s1.Length == 1)
            {
                long lo = s0[0];
                long hi = inclusive ? s1[0] : s1[0] - 1;
                return new CtorPat(new CtorDesc(CtorFamily.CharInterval, $"'{s0}'..'{s1}'", 0, lo, hi), Array.Empty<MatchPat>());
            }
            return new CtorPat(new CtorDesc(CtorFamily.OpenValue, ctx.GetText(), 0), Array.Empty<MatchPat>());
        }

        if (bounds[0].FLOAT_NUMBER() != null || bounds[1].FLOAT_NUMBER() != null)
        {
            return new CtorPat(new CtorDesc(CtorFamily.OpenValue, ctx.GetText(), 0), Array.Empty<MatchPat>());
        }

        long l = ParseBoundInt(bounds[0]);
        long h = ParseBoundInt(bounds[1]);
        if (!inclusive) h -= 1;
        return MakeIntInterval(l, h);
    }

    private static long ParseBoundInt(PilotParser.Range_boundContext b)
    {
        long v = ParseUnsignedLong(b.INTEGER()?.GetText(), b.HEX_INTEGER()?.GetText(), b.BIN_INTEGER()?.GetText(), b.OCT_INTEGER()?.GetText());
        return b.MINUS() != null ? -v : v;
    }

    private static long ParseUnsignedLong(string? dec, string? hex, string? bin, string? oct)
    {
        if (hex != null) return Convert.ToInt64(hex.Replace("_", "")[2..], 16);
        if (bin != null) return Convert.ToInt64(bin.Replace("_", "")[2..], 2);
        if (oct != null) return Convert.ToInt64(oct.Replace("_", "")[2..], 8);
        if (dec != null) return long.Parse(dec.Replace("_", ""), CultureInfo.InvariantCulture);
        return 0;
    }

    private MatchPat LowerLiteralPattern(PilotParser.LiteralContext lit)
    {
        if (lit.TRUE() != null)
            return new CtorPat(new CtorDesc(CtorFamily.Bool, "true", 0), Array.Empty<MatchPat>());
        if (lit.FALSE() != null)
            return new CtorPat(new CtorDesc(CtorFamily.Bool, "false", 0), Array.Empty<MatchPat>());
        if (lit.NULL() != null || lit.NONE() != null)
            return new CtorPat(new CtorDesc(CtorFamily.Null, "null", 0), Array.Empty<MatchPat>());
        if (lit.INTEGER() != null || lit.HEX_INTEGER() != null || lit.BIN_INTEGER() != null || lit.OCT_INTEGER() != null)
        {
            long v = ParseUnsignedLong(lit.INTEGER()?.GetText(), lit.HEX_INTEGER()?.GetText(), lit.BIN_INTEGER()?.GetText(), lit.OCT_INTEGER()?.GetText());
            return MakeIntInterval(v, v);
        }
        if (lit.STRING().Length == 1)
        {
            string raw = lit.STRING(0).GetText();
            if (raw.StartsWith("'", StringComparison.Ordinal))
            {
                string unq = StripQuotes(raw);
                if (unq.Length == 1)
                    return new CtorPat(new CtorDesc(CtorFamily.CharInterval, raw, 0, unq[0], unq[0]), Array.Empty<MatchPat>());
            }
        }
        return new CtorPat(new CtorDesc(CtorFamily.OpenValue, lit.GetText(), 0), Array.Empty<MatchPat>());
    }

    private MatchPat LowerRelationalPattern(PilotParser.Relational_patternContext rel, PilotType expectedType)
    {
        string rhsTxt = rel.shift_expr().GetText().Trim();
        if (long.TryParse(rhsTxt.Replace("_", ""), NumberStyles.Integer, CultureInfo.InvariantCulture, out long k))
        {
            var (dMin, dMax) = GetIntDomainBounds(expectedType);
            if (rel.GREATER() != null) return MakeIntInterval(k == long.MaxValue ? k : k + 1, dMax);
            if (rel.GREATEREQUAL() != null) return MakeIntInterval(k, dMax);
            if (rel.LESS() != null) return MakeIntInterval(dMin, k == long.MinValue ? k : k - 1);
            if (rel.LESSEQUAL() != null) return MakeIntInterval(dMin, k);
            if (rel.EQEQUAL() != null) return MakeIntInterval(k, k);
            if (rel.NOTEQUAL() != null) return NegatePattern(MakeIntInterval(k, k), expectedType);
        }
        return new CtorPat(new CtorDesc(CtorFamily.OpenValue, rel.GetText(), 0), Array.Empty<MatchPat>());
    }

    private MatchPat LowerCollectionPattern(PilotParser.Collection_patternContext colCtx, PilotType expectedType)
    {
        var elemType = expectedType.Kind == PilotTypeKind.Collection && expectedType.TypeArguments.Count > 0
            ? expectedType.TypeArguments[0]
            : PilotType.Any;

        var items = colCtx.slice_pattern_element();
        if (items == null || items.Length == 0)
        {
            return new CtorPat(new CtorDesc(CtorFamily.ListExact, "[]", 0), Array.Empty<MatchPat>());
        }

        int sliceIdx = -1;
        for (int i = 0; i < items.Length; i++)
        {
            if (items[i].DOTDOT() != null || items[i].ELLIPSIS() != null || items[i].STAR() != null)
            {
                sliceIdx = i;
                break;
            }
        }

        if (sliceIdx == -1)
        {
            var exactArgs = items.Select(it => LowerPattern(it.pattern(), elemType)).ToArray();
            return new CtorPat(new CtorDesc(CtorFamily.ListExact, $"List[{exactArgs.Length}]", exactArgs.Length), exactArgs);
        }

        int prefixLen = sliceIdx;
        int suffixLen = items.Length - 1 - sliceIdx;
        var sliceArgs = new List<MatchPat>(prefixLen + suffixLen);
        for (int i = 0; i < prefixLen; i++)
            sliceArgs.Add(LowerPattern(items[i].pattern(), elemType));
        for (int i = 0; i < suffixLen; i++)
            sliceArgs.Add(LowerPattern(items[sliceIdx + 1 + i].pattern(), elemType));

        // Encode prefixLen in Lo and suffixLen in Hi!
        return new CtorPat(
            new CtorDesc(CtorFamily.ListAtLeast, $"List[>={prefixLen + suffixLen}]", prefixLen + suffixLen, Lo: prefixLen, Hi: suffixLen),
            sliceArgs);
    }

    private MatchPat LowerPropertyPattern(PilotParser.Property_patternContext propCtx, PilotType expectedType)
    {
        string baseName = StripTypeArgs(expectedType.Name);
        if (_records.TryGetValue(baseName, out var rec))
        {
            var args = Enumerable.Repeat<MatchPat>(WildPat.Instance, rec.Fields.Count).ToArray();
            foreach (var mem in propCtx.property_pattern_member())
            {
                int idx = rec.GetFieldIndex(mem.identifier().GetText());
                if (idx >= 0)
                {
                    var fType = SubstituteGenericType(rec.Fields[idx].TypeName, rec.GenericParameters, expectedType.TypeArguments);
                    args[idx] = LowerPattern(mem.pattern(), fType);
                }
                else
                {
                    return new CtorPat(new CtorDesc(CtorFamily.OpenValue, propCtx.GetText(), 0), Array.Empty<MatchPat>());
                }
            }
            return new CtorPat(new CtorDesc(CtorFamily.Record, rec.Name, rec.Fields.Count, OwnerType: rec.Name), args);
        }

        return new CtorPat(new CtorDesc(CtorFamily.OpenValue, propCtx.GetText(), 0), Array.Empty<MatchPat>());
    }

    private MatchPat LowerTypeOrVarPattern(PilotParser.Type_or_var_patternContext tvCtx, PilotType expectedType)
    {
        // `let x` or `var x` is always an irrefutable wildcard binding
        if ((tvCtx.LET() != null || tvCtx.VAR() != null) && tvCtx.dotted_name() == null)
        {
            return WildPat.Instance;
        }

        if (tvCtx.dotted_name() == null) return WildPat.Instance;

        var parts = tvCtx.dotted_name().identifier();
        string fullDotted = tvCtx.dotted_name().GetText();
        var genSuffix = tvCtx.generic_pattern_suffix();
        bool hasParens = tvCtx.LPAR() != null;
        var patArgs = tvCtx.pattern_arg();
        var propPat = tvCtx.property_pattern();
        var trailingId = tvCtx.identifier();

        string? explicitOwner = null;
        string targetName = fullDotted;
        if (genSuffix?.identifier() != null)
        {
            explicitOwner = fullDotted;
            targetName = genSuffix.identifier().GetText();
        }
        else if (parts.Length == 2)
        {
            explicitOwner = parts[0].GetText();
            targetName = parts[1].GetText();
        }

        // Built-in Option `some(pat)` / `some x` constructor pattern (Milestone 12)
        if ((explicitOwner == null || string.Equals(explicitOwner, "Option", StringComparison.Ordinal)) &&
            (string.Equals(targetName, "some", StringComparison.Ordinal) ||
             (string.Equals(targetName, "Some", StringComparison.Ordinal) && !_variants.ContainsKey("Some"))) &&
            (hasParens || trailingId != null))
        {
            var innerType = expectedType.Kind == PilotTypeKind.Option && expectedType.TypeArguments.Count == 1
                ? expectedType.TypeArguments[0]
                : expectedType;
            var innerPat = hasParens && patArgs != null && patArgs.Length == 1
                ? LowerPattern(patArgs[0].pattern(), innerType)
                : WildPat.Instance;
            return new CtorPat(new CtorDesc(CtorFamily.NonNull, "some", 1), new[] { innerPat });
        }

        // Built-in Result `ok(pat)` / `ok x` and `fail(pat)` / `err(pat)` constructor patterns (Milestone 13)
        if ((explicitOwner == null || string.Equals(explicitOwner, "Result", StringComparison.Ordinal)) &&
            (string.Equals(targetName, "ok", StringComparison.Ordinal) ||
             (string.Equals(targetName, "Ok", StringComparison.Ordinal) && !_variants.ContainsKey("Ok"))) &&
            (hasParens || trailingId != null))
        {
            var innerType = expectedType.IsResult ? expectedType.TypeArguments[0] : PilotType.Any;
            var innerPat = hasParens && patArgs != null && patArgs.Length == 1
                ? LowerPattern(patArgs[0].pattern(), innerType)
                : WildPat.Instance;
            return new CtorPat(new CtorDesc(CtorFamily.ResultOk, "ok", 1), new[] { innerPat });
        }

        if ((explicitOwner == null || string.Equals(explicitOwner, "Result", StringComparison.Ordinal)) &&
            (string.Equals(targetName, "fail", StringComparison.Ordinal) ||
             string.Equals(targetName, "err", StringComparison.Ordinal) ||
             (string.Equals(targetName, "Err", StringComparison.Ordinal) && !_variants.ContainsKey("Err"))) &&
            (hasParens || trailingId != null))
        {
            var innerType = expectedType.IsResult ? expectedType.TypeArguments[1] : PilotType.Any;
            var innerPat = hasParens && patArgs != null && patArgs.Length == 1
                ? LowerPattern(patArgs[0].pattern(), innerType)
                : WildPat.Instance;
            return new CtorPat(new CtorDesc(CtorFamily.ResultErr, "fail", 1), new[] { innerPat });
        }

        // Typestate pattern: `Conn[Open]`
        if (genSuffix != null && genSuffix.identifier() == null && !hasParens)
        {
            string stateName = genSuffix.type_arguments().GetText().Trim();
            if (_classTypestates.ContainsKey(fullDotted))
            {
                return new CtorPat(
                    new CtorDesc(CtorFamily.Typestate, $"{fullDotted}[{stateName}]", 0, OwnerType: fullDotted),
                    Array.Empty<MatchPat>());
            }
        }

        // ADT Variant Lookup (qualified `Shape.Circle(r)` / `Shape.Origin` or unqualified `Circle(r)` / `Origin`)
        PilotAdtVariantDefinition? resolvedVariant = null;
        if (explicitOwner != null && _adts.TryGetValue(explicitOwner, out var ownerAdt))
        {
            ownerAdt.TryGetVariant(targetName, out resolvedVariant);
        }
        else
        {
            string expectedBase = StripTypeArgs(expectedType.Name);
            if (_adts.TryGetValue(expectedBase, out var expAdt) && expAdt.TryGetVariant(targetName, out var ev))
            {
                resolvedVariant = ev;
            }
            else if (_variants.TryGetValue(targetName, out var anyV))
            {
                resolvedVariant = anyV;
            }
        }

        if (resolvedVariant != null && (explicitOwner != null || hasParens || resolvedVariant.IsUnit || propPat != null))
        {
            var adt = resolvedVariant.ParentAdt;
            var subPats = Enumerable.Repeat<MatchPat>(WildPat.Instance, resolvedVariant.Arity).ToArray();
            if (hasParens && patArgs != null)
            {
                int posIdx = 0;
                foreach (var a in patArgs)
                {
                    int fIdx = a.identifier() != null ? resolvedVariant.GetFieldIndex(a.identifier().GetText()) : posIdx++;
                    if (fIdx >= 0 && fIdx < resolvedVariant.Arity)
                    {
                        var fType = SubstituteGenericType(resolvedVariant.Fields[fIdx].TypeName, adt.GenericParameters, expectedType.TypeArguments);
                        subPats[fIdx] = LowerPattern(a.pattern(), fType);
                    }
                }
            }
            else if (propPat != null)
            {
                foreach (var mem in propPat.property_pattern_member())
                {
                    int fIdx = resolvedVariant.GetFieldIndex(mem.identifier().GetText());
                    if (fIdx >= 0 && fIdx < resolvedVariant.Arity)
                    {
                        var fType = SubstituteGenericType(resolvedVariant.Fields[fIdx].TypeName, adt.GenericParameters, expectedType.TypeArguments);
                        subPats[fIdx] = LowerPattern(mem.pattern(), fType);
                    }
                }
            }
            return new CtorPat(
                new CtorDesc(CtorFamily.AdtVariant, resolvedVariant.Name, resolvedVariant.Arity, OwnerType: adt.Name),
                subPats);
        }

        // Whole-ADT type match `case Shape s =>` matches all variants of `Shape`
        if (!hasParens && propPat == null && trailingId != null &&
            string.Equals(targetName, StripTypeArgs(expectedType.Name), StringComparison.Ordinal))
        {
            return WildPat.Instance;
        }

        // ADT type match inside a union error set (`case IoError e =>` on `IoError | ParseError`)
        if (!hasParens && propPat == null && trailingId != null &&
            expectedType.Kind == PilotTypeKind.Union &&
            _adts.TryGetValue(targetName, out var unionSubAdt) &&
            unionSubAdt.Variants.Count > 0)
        {
            var variantPats = unionSubAdt.Variants
                .Select(v => (MatchPat)new CtorPat(
                    new CtorDesc(CtorFamily.AdtVariant, v.Name, v.Arity, OwnerType: unionSubAdt.Name),
                    Enumerable.Repeat<MatchPat>(WildPat.Instance, v.Arity).ToArray()))
                .ToArray();
            return variantPats.Length == 1 ? variantPats[0] : new OrPat(variantPats);
        }

        // Record / Struct destructuring `Point(x, y)` or `Point { x: 1, y: 2 }` or unit record `TimeoutError`
        if (_records.TryGetValue(targetName, out var recDef) &&
            (hasParens || propPat != null || trailingId != null || recDef.Fields.Count == 0))
        {
            var subPats = Enumerable.Repeat<MatchPat>(WildPat.Instance, recDef.Fields.Count).ToArray();
            if (hasParens && patArgs != null)
            {
                int posIdx = 0;
                foreach (var a in patArgs)
                {
                    int fIdx = a.identifier() != null ? recDef.GetFieldIndex(a.identifier().GetText()) : posIdx++;
                    if (fIdx >= 0 && fIdx < recDef.Fields.Count)
                    {
                        var fType = SubstituteGenericType(recDef.Fields[fIdx].TypeName, recDef.GenericParameters, expectedType.TypeArguments);
                        subPats[fIdx] = LowerPattern(a.pattern(), fType);
                    }
                }
            }
            if (propPat != null)
            {
                foreach (var mem in propPat.property_pattern_member())
                {
                    int fIdx = recDef.GetFieldIndex(mem.identifier().GetText());
                    if (fIdx >= 0 && fIdx < recDef.Fields.Count)
                    {
                        var fType = SubstituteGenericType(recDef.Fields[fIdx].TypeName, recDef.GenericParameters, expectedType.TypeArguments);
                        subPats[fIdx] = LowerPattern(mem.pattern(), fType);
                    }
                }
            }
            return new CtorPat(new CtorDesc(CtorFamily.Record, recDef.Name, recDef.Fields.Count, OwnerType: recDef.Name), subPats);
        }

        // Type test / subclass pattern (`int x`, `Circle c` where `Circle` is a class, etc.)
        if (hasParens || propPat != null || trailingId != null || IsBuiltinTypeName(targetName))
        {
            // If type test matches the static scrutinee type itself (e.g. `int x` on `int`), it is irrefutable!
            if (!hasParens && propPat == null && string.Equals(targetName, expectedType.Name, StringComparison.OrdinalIgnoreCase))
                return WildPat.Instance;
            return new CtorPat(new CtorDesc(CtorFamily.OpenValue, tvCtx.GetText(), 0), Array.Empty<MatchPat>());
        }

        // Bare single identifier capture (`x`, `fallback`, `head`) -> irrefutable wildcard!
        if (parts.Length == 1 && explicitOwner == null && genSuffix == null)
        {
            return WildPat.Instance;
        }

        return new CtorPat(new CtorDesc(CtorFamily.OpenValue, tvCtx.GetText(), 0), Array.Empty<MatchPat>());
    }

    private PilotType SubstituteGenericType(string fieldTypeSpec, IReadOnlyList<string> typeParams, IReadOnlyList<PilotType> typeArgs)
    {
        if (typeParams.Count > 0 && typeArgs.Count == typeParams.Count)
        {
            string replaced = fieldTypeSpec;
            for (int i = 0; i < typeParams.Count; i++)
            {
                if (replaced == typeParams[i])
                    return typeArgs[i];
                replaced = System.Text.RegularExpressions.Regex.Replace(
                    replaced,
                    $@"\b{System.Text.RegularExpressions.Regex.Escape(typeParams[i])}\b",
                    typeArgs[i].ToString());
            }
            return ParseRichType(replaced);
        }
        return ParseRichType(fieldTypeSpec);
    }

    private static CtorPat MakeIntInterval(long lo, long hi) =>
        new(new CtorDesc(CtorFamily.IntInterval, lo == hi ? lo.ToString(CultureInfo.InvariantCulture) : $"{lo}..{hi}", 0, lo, hi), Array.Empty<MatchPat>());

    private static (long Min, long Max) GetIntDomainBounds(PilotType type)
    {
        return type.Name.ToLowerInvariant() switch
        {
            "u8" or "byte" => (0L, 255L),
            "i8" => (-128L, 127L),
            "u16" => (0L, 65535L),
            "i16" => (-32768L, 32767L),
            "u32" => (0L, uint.MaxValue),
            "i32" => (int.MinValue, int.MaxValue),
            _ => (long.MinValue, long.MaxValue)
        };
    }

    #endregion

    #region Maranget Algorithm U(P, q) & Witness Reconstruction I(P, n)

    private bool IsUseful(IReadOnlyList<MatchPat[]> matrix, MatchPat[] query, PilotType[] colTypes)
    {
        // Base case: 0 columns! Useful iff matrix has 0 rows.
        if (query.Length == 0)
        {
            return matrix.Count == 0;
        }

        var q0 = query[0];
        var qRest = query[1..];
        var t0 = colTypes[0];
        var tRest = colTypes[1..];

        // Or-pattern expansion on query head: U(P, (p1 | p2) :: rest) = U(P, p1 :: rest) || U(P, p2 :: rest)
        if (q0 is OrPat orQ)
        {
            foreach (var branch in orQ.Branches)
            {
                var nextQ = new MatchPat[query.Length];
                nextQ[0] = branch;
                Array.Copy(qRest, 0, nextQ, 1, qRest.Length);
                if (IsUseful(matrix, nextQ, colTypes))
                    return true;
            }
            return false;
        }

        // Flatten top-level OrPat in column 0 of matrix for clean specialization
        var flatMatrix = ExpandColumnZeroOrs(matrix);

        if (q0 is CtorPat cq)
        {
            // Interval constructor (IntInterval or CharInterval): split into elementary disjoint intervals!
            if (cq.Ctor.Family is CtorFamily.IntInterval or CtorFamily.CharInterval)
            {
                var elemIntervals = PartitionInterval(cq.Ctor.Lo, cq.Ctor.Hi, flatMatrix, cq.Ctor.Family);
                foreach (var (elo, ehi) in elemIntervals)
                {
                    var elemCtor = new CtorDesc(cq.Ctor.Family, FormatInterval(elo, ehi, cq.Ctor.Family), 0, elo, ehi);
                    var specP = SpecializeMatrix(flatMatrix, elemCtor);
                    if (IsUseful(specP, qRest, tRest))
                        return true;
                }
                return false;
            }

            // List slice constructor `ListAtLeast(a + b)` in query: check both exact lengths >= a+b up to N and `ListAtLeast(N)`!
            if (cq.Ctor.Family == CtorFamily.ListAtLeast)
            {
                int minLen = cq.Ctor.Arity;
                int maxSplit = ComputeListSplitBound(flatMatrix, cq);
                var elemType = GetListElementType(t0);

                for (int len = minLen; len < maxSplit; len++)
                {
                    var exactCtor = new CtorDesc(CtorFamily.ListExact, $"List[{len}]", len);
                    var expandedQ = ExpandSliceQueryArgs(cq, len, qRest);
                    var expandedTypes = PrependRepeated(elemType, len, tRest);
                    if (IsUseful(SpecializeMatrix(flatMatrix, exactCtor), expandedQ, expandedTypes))
                        return true;
                }

                int tailN = Math.Max(minLen, maxSplit);
                var tailCtor = new CtorDesc(CtorFamily.ListAtLeast, $"List[>={tailN}]", tailN, Lo: tailN, Hi: 0);
                var tailQ = ExpandSliceQueryArgs(cq, tailN, qRest);
                var tailTypes = PrependRepeated(elemType, tailN, tRest);
                return IsUseful(SpecializeMatrix(flatMatrix, tailCtor), tailQ, tailTypes);
            }

            var subTypes = GetCtorSubTypes(cq.Ctor, t0);
            var specMatrix = SpecializeMatrix(flatMatrix, cq.Ctor);
            var specQuery = ConcatPatterns(cq.Args, qRest);
            var specColTypes = ConcatTypes(subTypes, tRest);
            return IsUseful(specMatrix, specQuery, specColTypes);
        }

        // q0 is WildPat (`_`)!
        var (isComplete, completeCtors) = CheckSignatureCompleteness(flatMatrix, t0);
        if (isComplete)
        {
            foreach (var ctor in completeCtors)
            {
                var subTypes = GetCtorSubTypes(ctor, t0);
                var specP = SpecializeMatrix(flatMatrix, ctor);
                var specQ = PrependWildcards(ctor.Arity, qRest);
                var specT = ConcatTypes(subTypes, tRest);
                if (IsUseful(specP, specQ, specT))
                    return true;
            }
            return false;
        }
        else
        {
            var defP = DefaultMatrix(flatMatrix);
            return IsUseful(defP, qRest, tRest);
        }
    }

    private List<string> FindMissingWitnesses(IReadOnlyList<MatchPat[]> matrix, PilotType[] colTypes, int maxWitnesses)
    {
        var results = new List<string>();
        var witnessTuple = FindWitnessVector(ExpandColumnZeroOrs(matrix), colTypes);
        if (witnessTuple != null && witnessTuple.Length > 0)
        {
            results.Add(witnessTuple[0]);

            // For top-level closed ADTs/enums/typestates, collect up to maxWitnesses distinct missing variants for crystal-clear diagnostics!
            if (colTypes.Length == 1)
            {
                var flat = ExpandColumnZeroOrs(matrix);
                var (isComplete, allCtors) = GetFullClosedDomainConstructors(flat, colTypes[0]);
                if (isComplete && allCtors.Count > 1)
                {
                    results.Clear();
                    foreach (var ctor in allCtors)
                    {
                        if (results.Count >= maxWitnesses) break;
                        var subTypes = GetCtorSubTypes(ctor, colTypes[0]);
                        var specP = SpecializeMatrix(flat, ctor);
                        var subWitness = FindWitnessVector(ExpandColumnZeroOrs(specP), subTypes);
                        if (subWitness != null)
                        {
                            results.Add(FormatConstructorWitness(ctor, subWitness, colTypes[0]));
                        }
                    }
                }
            }
        }
        return results;
    }

    private string[]? FindWitnessVector(List<MatchPat[]> flatMatrix, PilotType[] colTypes)
    {
        if (colTypes.Length == 0)
        {
            return flatMatrix.Count == 0 ? Array.Empty<string>() : null;
        }

        if (flatMatrix.Count == 0)
        {
            return Enumerable.Repeat("_", colTypes.Length).ToArray();
        }

        var t0 = colTypes[0];
        var tRest = colTypes[1..];

        // If column 0 contains only wildcards (zero constructor constraints), step directly via DefaultMatrix
        // so recursive / mutually-recursive ADTs never expand infinitely on unconstrained subtrees!
        if (!flatMatrix.Any(r => r.Length > 0 && r[0] is CtorPat))
        {
            var defP = DefaultMatrix(flatMatrix);
            var restWitness = FindWitnessVector(ExpandColumnZeroOrs(defP), tRest);
            if (restWitness == null) return null;

            var full = new string[1 + tRest.Length];
            full[0] = "_";
            Array.Copy(restWitness, 0, full, 1, tRest.Length);
            return full;
        }

        var (hasClosedDomain, domainCtors) = GetFullClosedDomainConstructors(flatMatrix, t0);

        if (hasClosedDomain)
        {
            foreach (var ctor in domainCtors)
            {
                var subTypes = GetCtorSubTypes(ctor, t0);
                var specP = SpecializeMatrix(flatMatrix, ctor);
                var restWitness = FindWitnessVector(ExpandColumnZeroOrs(specP), ConcatTypes(subTypes, tRest));
                if (restWitness != null)
                {
                    string headStr = FormatConstructorWitness(ctor, restWitness[..ctor.Arity], t0);
                    var full = new string[1 + tRest.Length];
                    full[0] = headStr;
                    Array.Copy(restWitness, ctor.Arity, full, 1, tRest.Length);
                    return full;
                }
            }
            return null;
        }
        else
        {
            var defP = DefaultMatrix(flatMatrix);
            var restWitness = FindWitnessVector(ExpandColumnZeroOrs(defP), tRest);
            if (restWitness == null) return null;

            var full = new string[1 + tRest.Length];
            full[0] = "_";
            Array.Copy(restWitness, 0, full, 1, tRest.Length);
            return full;
        }
    }

    private (bool IsComplete, List<CtorDesc> Ctors) CheckSignatureCompleteness(List<MatchPat[]> flatMatrix, PilotType colType)
    {
        // When column 0 has no constructor patterns at all (empty matrix or purely wildcard column),
        // DefaultMatrix(P) is exact and guarantees termination on recursive ADTs.
        if (!flatMatrix.Any(r => r.Length > 0 && r[0] is CtorPat))
            return (false, new List<CtorDesc>());

        var (hasClosedDomain, domainCtors) = GetFullClosedDomainConstructors(flatMatrix, colType);
        if (!hasClosedDomain)
            return (false, new List<CtorDesc>());

        return (true, domainCtors);
    }

    private (bool HasClosedDomain, List<CtorDesc> Ctors) GetFullClosedDomainConstructors(List<MatchPat[]> flatMatrix, PilotType colType)
    {
        string baseName = StripTypeArgs(colType.Name);

        // 0. Fallible Result `T ! E` (`{ ok(x), fail(e) }`)
        bool hasResultPat = flatMatrix.Any(r => r[0] is CtorPat { Ctor.Family: CtorFamily.ResultOk or CtorFamily.ResultErr });
        if (colType.IsResult || hasResultPat)
        {
            return (true, new List<CtorDesc>
            {
                new(CtorFamily.ResultOk, "ok", 1),
                new(CtorFamily.ResultErr, "fail", 1)
            });
        }

        // 0b. Closed Union of ADTs / Records / Variants (`IoError | ParseError`)
        if (colType.Kind == PilotTypeKind.Union && colType.TypeArguments.Count > 0)
        {
            var unionCtors = new List<CtorDesc>();
            bool allClosed = true;
            foreach (var memberType in colType.TypeArguments)
            {
                string mName = StripTypeArgs(memberType.Name);
                if (_adts.TryGetValue(mName, out var mAdt) && mAdt.Variants.Count > 0)
                {
                    foreach (var v in mAdt.Variants)
                    {
                        var desc = new CtorDesc(CtorFamily.AdtVariant, v.Name, v.Arity, OwnerType: mAdt.Name);
                        if (!unionCtors.Contains(desc)) unionCtors.Add(desc);
                    }
                }
                else if (_records.TryGetValue(mName, out var mRec))
                {
                    var desc = new CtorDesc(CtorFamily.Record, mRec.Name, mRec.Fields.Count, OwnerType: mRec.Name);
                    if (!unionCtors.Contains(desc)) unionCtors.Add(desc);
                }
                else if (_variants.TryGetValue(mName, out var mVar))
                {
                    var desc = new CtorDesc(CtorFamily.AdtVariant, mVar.Name, mVar.Arity, OwnerType: mVar.ParentAdt.Name);
                    if (!unionCtors.Contains(desc)) unionCtors.Add(desc);
                }
                else
                {
                    allClosed = false;
                    break;
                }
            }
            if (allClosed && unionCtors.Count > 0)
            {
                return (true, unionCtors);
            }
        }

        // 1. Nullable `T?` or `{ null, not null }` / `{ some(x), none }` pair
        bool hasSomePat = flatMatrix.Any(r => r[0] is CtorPat { Ctor.Family: CtorFamily.NonNull, Ctor.Arity: 1 });
        bool hasNullAndNonNull =
            flatMatrix.Any(r => r[0] is CtorPat { Ctor.Family: CtorFamily.Null }) &&
            flatMatrix.Any(r => r[0] is CtorPat { Ctor.Family: CtorFamily.NonNull });
        if (hasSomePat || hasNullAndNonNull || (colType.Kind == PilotTypeKind.Option && colType.TypeArguments.Count == 1))
        {
            bool useSomeConstructor = hasSomePat ||
                (colType.Kind == PilotTypeKind.Option &&
                 !flatMatrix.Any(r => r[0] is CtorPat { Ctor.Family: CtorFamily.NonNull, Ctor.Arity: 0 }));
            return (true, new List<CtorDesc>
            {
                useSomeConstructor ? new(CtorFamily.NonNull, "some", 1) : new(CtorFamily.NonNull, "not null", 0),
                new(CtorFamily.Null, useSomeConstructor ? "none" : "null", 0)
            });
        }

        // 2. Closed ADT / Enum
        if (_adts.TryGetValue(baseName, out var adt) && adt.Variants.Count > 0)
        {
            var ctors = adt.Variants
                .Select(v => new CtorDesc(CtorFamily.AdtVariant, v.Name, v.Arity, OwnerType: adt.Name))
                .ToList();
            return (true, ctors);
        }

        // 3. Record / Struct product type
        if (_records.TryGetValue(baseName, out var rec) &&
            flatMatrix.All(r => r[0] is WildPat or CtorPat { Ctor.Family: CtorFamily.Record }))
        {
            return (true, new List<CtorDesc>
            {
                new(CtorFamily.Record, rec.Name, rec.Fields.Count, OwnerType: rec.Name)
            });
        }

        // 4. Tuple
        if (colType.Kind == PilotTypeKind.Tuple)
        {
            int arity = colType.TypeArguments.Count;
            return (true, new List<CtorDesc> { new(CtorFamily.Tuple, "Tuple", arity) });
        }

        // 5. Bool
        if (colType == PilotType.Bool || baseName.Equals("bool", StringComparison.OrdinalIgnoreCase))
        {
            return (true, new List<CtorDesc>
            {
                new(CtorFamily.Bool, "true", 0),
                new(CtorFamily.Bool, "false", 0)
            });
        }

        // 6. Class with declared Typestates (`Conn[Open]`, `Conn[Closed]`)
        if (_classTypestates.TryGetValue(baseName, out var states) && states.Count > 0 &&
            flatMatrix.Any(r => r[0] is CtorPat { Ctor.Family: CtorFamily.Typestate }))
        {
            var ctors = states
                .Select(st => new CtorDesc(CtorFamily.Typestate, $"{baseName}[{st}]", 0, OwnerType: baseName))
                .ToList();
            return (true, ctors);
        }

        // 7. Bounded & Unbounded Integer Intervals (`u8` 0..255, `i8` -128..127, `int`, etc.)
        if (IsIntegerType(baseName))
        {
            // Only partition integer intervals if at least one arm tests an IntInterval (otherwise single `_` missing witness is cleaner!)
            if (flatMatrix.Any(r => r[0] is CtorPat { Ctor.Family: CtorFamily.IntInterval }))
            {
                var (dMin, dMax) = GetIntDomainBounds(colType);
                var elem = PartitionInterval(dMin, dMax, flatMatrix, CtorFamily.IntInterval);
                var ctors = elem
                    .Select(iv => new CtorDesc(CtorFamily.IntInterval, FormatInterval(iv.Lo, iv.Hi, CtorFamily.IntInterval), 0, iv.Lo, iv.Hi))
                    .ToList();
                return (true, ctors);
            }
            if (baseName.Equals("u8", StringComparison.OrdinalIgnoreCase))
            {
                return (true, new List<CtorDesc> { new(CtorFamily.IntInterval, "0..255", 0, 0, 255) });
            }
        }

        // 8. Char intervals (`'\\0'..'\\uFFFF'`) when char ranges are matched
        if (baseName.Equals("char", StringComparison.OrdinalIgnoreCase) &&
            flatMatrix.Any(r => r[0] is CtorPat { Ctor.Family: CtorFamily.CharInterval }))
        {
            var elem = PartitionInterval(0, char.MaxValue, flatMatrix, CtorFamily.CharInterval);
            var ctors = elem
                .Select(iv => new CtorDesc(CtorFamily.CharInterval, FormatInterval(iv.Lo, iv.Hi, CtorFamily.CharInterval), 0, iv.Lo, iv.Hi))
                .ToList();
            return (true, ctors);
        }

        // 9. Fixed-length array `[T; N]` or Dynamic List/Slice `T[]` / `List[T]`
        if (colType.Kind == PilotTypeKind.Collection && colType.Name.StartsWith("FixedArray[", StringComparison.Ordinal))
        {
            int len = int.Parse(colType.Name["FixedArray[".Length..^1], CultureInfo.InvariantCulture);
            return (true, new List<CtorDesc> { new(CtorFamily.ListExact, $"List[{len}]", len) });
        }

        if ((colType.Kind == PilotTypeKind.Collection && colType.Name.Equals("List", StringComparison.OrdinalIgnoreCase)) ||
            flatMatrix.Any(r => r[0] is CtorPat { Ctor.Family: CtorFamily.ListExact or CtorFamily.ListAtLeast }))
        {
            int splitN = Math.Max(1, ComputeListSplitBound(flatMatrix, null));
            var listCtors = new List<CtorDesc>(splitN + 1);
            for (int k = 0; k < splitN; k++)
            {
                listCtors.Add(new CtorDesc(CtorFamily.ListExact, k == 0 ? "[]" : $"List[{k}]", k));
            }
            listCtors.Add(new CtorDesc(CtorFamily.ListAtLeast, $"List[>={splitN}]", splitN, Lo: splitN, Hi: 0));
            return (true, listCtors);
        }

        return (false, new List<CtorDesc>());
    }

    private static List<MatchPat[]> SpecializeMatrix(List<MatchPat[]> flatMatrix, CtorDesc targetCtor)
    {
        var result = new List<MatchPat[]>();
        foreach (var row in flatMatrix)
        {
            var p0 = row[0];
            var rest = row[1..];

            if (p0 is WildPat)
            {
                result.Add(PrependWildcards(targetCtor.Arity, rest));
                continue;
            }

            if (p0 is CtorPat cp)
            {
                // Interval containment: since targetCtor is an elementary interval, cp covers targetCtor iff cp.Lo <= target.Lo && target.Hi <= cp.Hi!
                if ((targetCtor.Family == CtorFamily.IntInterval && cp.Ctor.Family == CtorFamily.IntInterval) ||
                    (targetCtor.Family == CtorFamily.CharInterval && cp.Ctor.Family == CtorFamily.CharInterval))
                {
                    if (cp.Ctor.Lo <= targetCtor.Lo && targetCtor.Hi <= cp.Ctor.Hi)
                    {
                        result.Add(rest);
                    }
                    continue;
                }

                // List specialization!
                if (targetCtor.Family == CtorFamily.ListExact)
                {
                    int targetLen = targetCtor.Arity;
                    if (cp.Ctor.Family == CtorFamily.ListExact && cp.Ctor.Arity == targetLen)
                    {
                        result.Add(ConcatPatterns(cp.Args, rest));
                    }
                    else if (cp.Ctor.Family == CtorFamily.ListAtLeast && targetLen >= cp.Ctor.Arity)
                    {
                        result.Add(ExpandSliceRowArgs(cp, targetLen, rest));
                    }
                    continue;
                }

                if (targetCtor.Family == CtorFamily.ListAtLeast)
                {
                    int targetMin = targetCtor.Arity;
                    if (cp.Ctor.Family == CtorFamily.ListAtLeast && cp.Ctor.Arity <= targetMin)
                    {
                        result.Add(ExpandSliceRowArgs(cp, targetMin, rest));
                    }
                    continue;
                }

                // Null constructor (`null` and `none` are aliases for CtorFamily.Null)
                if (targetCtor.Family == CtorFamily.Null)
                {
                    if (cp.Ctor.Family == CtorFamily.Null)
                    {
                        result.Add(rest);
                    }
                    continue;
                }

                // NonNull (`some(pat)` with Arity 1 or `not null` with Arity 0)
                if (targetCtor.Family == CtorFamily.NonNull)
                {
                    if (cp.Ctor.Family == CtorFamily.NonNull)
                    {
                        if (cp.Ctor.Arity == targetCtor.Arity)
                        {
                            result.Add(ConcatPatterns(cp.Args, rest));
                        }
                        else if (cp.Ctor.Arity == 0 && targetCtor.Arity == 1)
                        {
                            result.Add(PrependWildcards(1, rest));
                        }
                        else if (cp.Ctor.Arity == 1 && targetCtor.Arity == 0 && cp.Args[0] is WildPat)
                        {
                            result.Add(rest);
                        }
                    }
                    continue;
                }

                // NonNull covers concrete non-null constructors
                if (cp.Ctor.Family == CtorFamily.NonNull && targetCtor.Family != CtorFamily.Null)
                {
                    if (cp.Ctor.Arity == 0 || (cp.Ctor.Arity == 1 && cp.Args[0] is WildPat))
                    {
                        result.Add(PrependWildcards(targetCtor.Arity, rest));
                    }
                    continue;
                }

                if (cp.Ctor.Family == targetCtor.Family &&
                    cp.Ctor.Arity == targetCtor.Arity &&
                    string.Equals(cp.Ctor.Name, targetCtor.Name, StringComparison.Ordinal))
                {
                    result.Add(ConcatPatterns(cp.Args, rest));
                }
            }
        }
        return result;
    }

    private static List<MatchPat[]> DefaultMatrix(List<MatchPat[]> flatMatrix)
    {
        var result = new List<MatchPat[]>();
        foreach (var row in flatMatrix)
        {
            if (row[0] is WildPat)
            {
                result.Add(row[1..]);
            }
        }
        return result;
    }

    private static List<MatchPat[]> ExpandColumnZeroOrs(IReadOnlyList<MatchPat[]> matrix)
    {
        var expanded = new List<MatchPat[]>(matrix.Count);
        foreach (var row in matrix)
        {
            if (row.Length > 0 && row[0] is OrPat orP)
            {
                ExpandOrRow(orP, row[1..], expanded);
            }
            else
            {
                expanded.Add(row);
            }
        }
        return expanded;
    }

    private static void ExpandOrRow(OrPat orPat, MatchPat[] rest, List<MatchPat[]> dest)
    {
        foreach (var b in orPat.Branches)
        {
            if (b is OrPat nested)
            {
                ExpandOrRow(nested, rest, dest);
            }
            else
            {
                var next = new MatchPat[1 + rest.Length];
                next[0] = b;
                Array.Copy(rest, 0, next, 1, rest.Length);
                dest.Add(next);
            }
        }
    }

    private static List<(long Lo, long Hi)> PartitionInterval(long minBound, long maxBound, List<MatchPat[]> matrix, CtorFamily family)
    {
        if (minBound > maxBound) return new List<(long, long)>();

        var points = new SortedSet<long> { minBound };
        foreach (var row in matrix)
        {
            if (row[0] is CtorPat cp && cp.Ctor.Family == family)
            {
                long a = Math.Clamp(cp.Ctor.Lo, minBound, maxBound);
                long b = Math.Clamp(cp.Ctor.Hi, minBound, maxBound);
                if (a <= b)
                {
                    points.Add(a);
                    if (b < maxBound) points.Add(b + 1);
                }
            }
        }

        var sorted = points.ToArray();
        var intervals = new List<(long Lo, long Hi)>(sorted.Length);
        for (int i = 0; i < sorted.Length; i++)
        {
            long lo = sorted[i];
            long hi = (i + 1 < sorted.Length) ? sorted[i + 1] - 1 : maxBound;
            if (lo <= hi) intervals.Add((lo, hi));
        }
        return intervals;
    }

    private static int ComputeListSplitBound(List<MatchPat[]> flatMatrix, CtorPat? querySlice)
    {
        int maxBound = querySlice != null ? querySlice.Ctor.Arity : 0;
        foreach (var row in flatMatrix)
        {
            if (row[0] is CtorPat cp)
            {
                if (cp.Ctor.Family == CtorFamily.ListExact)
                    maxBound = Math.Max(maxBound, cp.Ctor.Arity + 1);
                else if (cp.Ctor.Family == CtorFamily.ListAtLeast)
                    maxBound = Math.Max(maxBound, cp.Ctor.Arity);
            }
        }
        return maxBound;
    }

    private static MatchPat[] ExpandSliceRowArgs(CtorPat slicePat, int targetSlots, MatchPat[] rest)
    {
        int prefix = (int)slicePat.Ctor.Lo;
        int suffix = (int)slicePat.Ctor.Hi;
        int middleWildcards = Math.Max(0, targetSlots - prefix - suffix);

        var row = new MatchPat[targetSlots + rest.Length];
        int pos = 0;
        for (int i = 0; i < prefix; i++) row[pos++] = slicePat.Args[i];
        for (int i = 0; i < middleWildcards; i++) row[pos++] = WildPat.Instance;
        for (int i = 0; i < suffix; i++) row[pos++] = slicePat.Args[prefix + i];
        Array.Copy(rest, 0, row, pos, rest.Length);
        return row;
    }

    private static MatchPat[] ExpandSliceQueryArgs(CtorPat slicePat, int targetSlots, MatchPat[] qRest) =>
        ExpandSliceRowArgs(slicePat, targetSlots, qRest);

    private PilotType[] GetCtorSubTypes(CtorDesc ctor, PilotType parentType)
    {
        if (ctor.Arity == 0) return Array.Empty<PilotType>();

        switch (ctor.Family)
        {
            case CtorFamily.AdtVariant when ctor.OwnerType != null && _adts.TryGetValue(ctor.OwnerType, out var adt) &&
                                             adt.TryGetVariant(ctor.Name, out var vd) && vd != null:
                return vd.Fields.Select(f => SubstituteGenericType(f.TypeName, adt.GenericParameters, parentType.TypeArguments)).ToArray();

            case CtorFamily.Record when ctor.OwnerType != null && _records.TryGetValue(ctor.OwnerType, out var rec):
                return rec.Fields.Select(f => SubstituteGenericType(f.TypeName, rec.GenericParameters, parentType.TypeArguments)).ToArray();

            case CtorFamily.Tuple:
                return parentType.Kind == PilotTypeKind.Tuple && parentType.TypeArguments.Count == ctor.Arity
                    ? parentType.TypeArguments.ToArray()
                    : Enumerable.Repeat(PilotType.Any, ctor.Arity).ToArray();

            case CtorFamily.ListExact or CtorFamily.ListAtLeast:
                var elemT = GetListElementType(parentType);
                return Enumerable.Repeat(elemT, ctor.Arity).ToArray();

            case CtorFamily.NonNull when ctor.Arity == 1:
                var innerOpt = parentType.Kind == PilotTypeKind.Option && parentType.TypeArguments.Count == 1
                    ? parentType.TypeArguments[0]
                    : parentType;
                return new[] { innerOpt };

            case CtorFamily.ResultOk when ctor.Arity == 1:
                return new[] { parentType.IsResult ? parentType.TypeArguments[0] : PilotType.Any };

            case CtorFamily.ResultErr when ctor.Arity == 1:
                return new[] { parentType.IsResult ? parentType.TypeArguments[1] : PilotType.Any };

            default:
                return Enumerable.Repeat(PilotType.Any, ctor.Arity).ToArray();
        }
    }

    private static PilotType GetListElementType(PilotType listType) =>
        listType.Kind == PilotTypeKind.Collection && listType.TypeArguments.Count > 0
            ? listType.TypeArguments[0]
            : PilotType.Any;

    private static string FormatConstructorWitness(CtorDesc ctor, string[] argWitnesses, PilotType colType)
    {
        return ctor.Family switch
        {
            CtorFamily.AdtVariant => ctor.Arity == 0 ? ctor.Name : $"{ctor.Name}({string.Join(", ", argWitnesses)})",
            CtorFamily.Record => $"{ctor.Name}({string.Join(", ", argWitnesses)})",
            CtorFamily.Tuple => $"({string.Join(", ", argWitnesses)})",
            CtorFamily.Bool => ctor.Name,
            CtorFamily.Null => ctor.Name,
            CtorFamily.NonNull => ctor.Arity == 1 ? $"some({string.Join(", ", argWitnesses)})" : "not null",
            CtorFamily.ResultOk => $"ok({string.Join(", ", argWitnesses)})",
            CtorFamily.ResultErr => $"fail({string.Join(", ", argWitnesses)})",
            CtorFamily.Typestate => ctor.Name,
            CtorFamily.Keyword => ctor.Arity == 0 ? ctor.Name : $"{ctor.Name}({string.Join(", ", argWitnesses)})",
            CtorFamily.ListExact => ctor.Arity == 0 ? "[]" : $"[{string.Join(", ", argWitnesses)}]",
            CtorFamily.ListAtLeast => ctor.Arity == 0 ? "[..]" : $"[{string.Join(", ", argWitnesses)}, ..]",
            CtorFamily.IntInterval => FormatIntWitness(ctor.Lo, ctor.Hi, colType),
            CtorFamily.CharInterval => FormatInterval(ctor.Lo, ctor.Hi, CtorFamily.CharInterval),
            _ => ctor.Name
        };
    }

    private static string FormatIntWitness(long lo, long hi, PilotType colType)
    {
        var (dMin, dMax) = GetIntDomainBounds(colType);
        if (lo == dMin && hi == dMax && dMin == long.MinValue) return "_";
        if (lo == hi) return lo.ToString(CultureInfo.InvariantCulture);
        if (lo == long.MinValue) return $"<= {hi}";
        if (hi == long.MaxValue) return $">= {lo}";
        return $"{lo}..{hi}";
    }

    private static string FormatInterval(long lo, long hi, CtorFamily family)
    {
        if (family == CtorFamily.CharInterval)
        {
            char c0 = (char)lo;
            char c1 = (char)hi;
            return c0 == c1 ? $"'{c0}'" : $"'{c0}'..'{c1}'";
        }
        return lo == hi ? lo.ToString(CultureInfo.InvariantCulture) : $"{lo}..{hi}";
    }

    private static MatchPat[] PrependWildcards(int count, MatchPat[] rest)
    {
        if (count == 0) return rest;
        var arr = new MatchPat[count + rest.Length];
        for (int i = 0; i < count; i++) arr[i] = WildPat.Instance;
        Array.Copy(rest, 0, arr, count, rest.Length);
        return arr;
    }

    private static MatchPat[] ConcatPatterns(IReadOnlyList<MatchPat> head, MatchPat[] tail)
    {
        if (head.Count == 0) return tail;
        var arr = new MatchPat[head.Count + tail.Length];
        for (int i = 0; i < head.Count; i++) arr[i] = head[i];
        Array.Copy(tail, 0, arr, head.Count, tail.Length);
        return arr;
    }

    private static PilotType[] ConcatTypes(PilotType[] head, PilotType[] tail)
    {
        if (head.Length == 0) return tail;
        var arr = new PilotType[head.Length + tail.Length];
        Array.Copy(head, 0, arr, 0, head.Length);
        Array.Copy(tail, 0, arr, head.Length, tail.Length);
        return arr;
    }

    private static PilotType[] PrependRepeated(PilotType elem, int count, PilotType[] tail)
    {
        if (count == 0) return tail;
        var arr = new PilotType[count + tail.Length];
        for (int i = 0; i < count; i++) arr[i] = elem;
        Array.Copy(tail, 0, arr, count, tail.Length);
        return arr;
    }

    private static bool IsIntegerType(string name) =>
        name.ToLowerInvariant() is "int" or "i8" or "i16" or "i32" or "i64" or "u8" or "u16" or "u32" or "u64" or "uint" or "long" or "byte";

    private static bool IsBuiltinTypeName(string name) =>
        name.ToLowerInvariant() is "int" or "i8" or "i16" or "i32" or "i64" or "u8" or "u16" or "u32" or "u64" or "uint"
            or "float" or "f8" or "f16" or "f32" or "f64" or "double" or "decimal" or "complex"
            or "str" or "string" or "bool" or "boolean" or "char" or "bytes" or "any" or "dynamic" or "void"
            or "list" or "tuple" or "map" or "dict" or "set" or "range" or "matrix" or "object" or "symbol" or "keyword";

    private static string StripTypeArgs(string name)
    {
        int br = name.IndexOfAny(new[] { '[', '<' });
        return br > 0 ? name[..br].Trim() : name.Trim();
    }

    private static string StripQuotes(string s) =>
        s.Length >= 2 && (s[0] == '"' || s[0] == '\'') ? s[1..^1] : s;

    private static List<string> SplitTopLevel(string input, char sep)
    {
        var res = new List<string>();
        int depth = 0, start = 0;
        for (int i = 0; i < input.Length; i++)
        {
            char c = input[i];
            if (c is '[' or '(' or '<' or '{') depth++;
            else if (c is ']' or ')' or '>' or '}') depth--;
            else if (c == sep && depth == 0)
            {
                res.Add(input[start..i].Trim());
                start = i + 1;
            }
        }
        if (start <= input.Length)
        {
            string rem = input[start..].Trim();
            if (rem.Length > 0) res.Add(rem);
        }
        return res;
    }

    #endregion
}
