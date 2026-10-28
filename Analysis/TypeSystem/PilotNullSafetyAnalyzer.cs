using System;
using System.Collections.Generic;
using System.Linq;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Runtime.Values;

namespace Pilot.Analysis.TypeSystem;

/// <summary>
/// Milestone 12: Concise Modern Null-Safety Verifier (`NULL001`–`NULL005`).
/// Enforces non-nullable-by-default types (`T`), implicit lifting (`T -> T?`),
/// flow-sensitive presence narrowing (`guard some x`, `if some x`, `x != none`, `??`, `??=`),
/// safe navigation (`?.`), and FFI / SQL null-boundary hardening.
/// </summary>
public sealed class PilotNullSafetyAnalyzer : PilotBaseVisitor<string>
{
    private sealed class FunctionSignatureInfo
    {
        public string Name { get; init; } = "";
        public List<(string Name, string Type)> Parameters { get; } = new();
        public string ReturnType { get; set; } = "any";
        public bool HasExplicitNonNullReturn { get; set; }
    }

    private sealed class TypeMemberInfo
    {
        public string Name { get; init; } = "";
        public Dictionary<string, string> Fields { get; } = new(StringComparer.Ordinal);
        public Dictionary<string, FunctionSignatureInfo> Methods { get; } = new(StringComparer.Ordinal);
    }

    private sealed class NullSafetyScope
    {
        public NullSafetyScope? Parent { get; }
        public Dictionary<string, string> DeclaredTypes { get; } = new(StringComparer.Ordinal);
        public Dictionary<string, bool> ExplicitNonNullable { get; } = new(StringComparer.Ordinal);
        public Dictionary<string, string> NarrowedTypes { get; } = new(StringComparer.Ordinal);
        public string? EnclosingFunctionName { get; set; }
        public string? EnclosingReturnType { get; set; }
        public bool EnclosingReturnIsExplicitNonNullable { get; set; }

        public NullSafetyScope(NullSafetyScope? parent)
        {
            Parent = parent;
            if (parent != null)
            {
                EnclosingFunctionName = parent.EnclosingFunctionName;
                EnclosingReturnType = parent.EnclosingReturnType;
                EnclosingReturnIsExplicitNonNullable = parent.EnclosingReturnIsExplicitNonNullable;
            }
        }

        public string? ResolveEffectiveType(string name)
        {
            for (var s = this; s != null; s = s.Parent)
            {
                if (s.NarrowedTypes.TryGetValue(name, out var narrowed))
                    return narrowed;
                if (s.DeclaredTypes.TryGetValue(name, out var declared))
                    return declared;
            }
            return null;
        }

        public (string? DeclaredType, bool IsExplicitNonNull) ResolveDeclaration(string name)
        {
            for (var s = this; s != null; s = s.Parent)
            {
                if (s.DeclaredTypes.TryGetValue(name, out var declared))
                {
                    s.ExplicitNonNullable.TryGetValue(name, out bool exp);
                    return (declared, exp);
                }
            }
            return (null, false);
        }

        public void ClearNarrowing(string name)
        {
            for (var s = this; s != null; s = s.Parent)
            {
                s.NarrowedTypes.Remove(name);
                if (s.DeclaredTypes.ContainsKey(name))
                    break;
            }
        }
    }

    private static readonly HashSet<string> AllowedOptionalMembers = new(StringComparer.Ordinal)
    {
        "or_panic", "OrPanic",
        "unwrap_or", "UnwrapOr",
        "is_some", "IsSome",
        "is_none", "IsNone",
        "is_unboxed", "IsUnboxed",
        "has_niche", "HasNiche",
        "to_string", "ToString", "__str__",
        "type_of", "GetType",
        "equals", "Equals", "__eq__",
        "hash", "GetHashCode", "__hash__",
        "clone", "Clone", "__clone__"
    };

    private readonly Dictionary<string, FunctionSignatureInfo> _functions = new(StringComparer.Ordinal);
    private readonly Dictionary<string, TypeMemberInfo> _types = new(StringComparer.Ordinal);
    private readonly List<PilotAnalysisDiagnostic> _diagnostics = new();
    private NullSafetyScope _currentScope = new(null);

    public IReadOnlyList<PilotAnalysisDiagnostic> Diagnostics => _diagnostics;

    public void CheckProgram(PilotParser.ProgramContext program)
    {
        PrescanDeclarations(program);
        VisitProgram(program);
    }

    public static bool IsOptionalType(string? typeSpec)
    {
        if (string.IsNullOrWhiteSpace(typeSpec)) return false;
        string t = typeSpec.Trim();
        if (t is "null" or "none" or "None") return true;
        if (t.EndsWith("?", StringComparison.Ordinal)) return true;
        if ((t.StartsWith("Option[", StringComparison.OrdinalIgnoreCase) && t.EndsWith("]", StringComparison.Ordinal)) ||
            (t.StartsWith("Option<", StringComparison.OrdinalIgnoreCase) && t.EndsWith(">", StringComparison.Ordinal)))
        {
            return true;
        }
        return false;
    }

    public static bool IsDynamicOrAny(string? typeSpec)
    {
        if (string.IsNullOrWhiteSpace(typeSpec)) return true;
        string t = typeSpec.Trim();
        return t is "any" or "dynamic" or "unknown" or "void" or "object";
    }

    public static bool IsStrictNonNullableType(string? typeSpec)
    {
        if (string.IsNullOrWhiteSpace(typeSpec)) return false;
        string t = typeSpec.Trim();
        if (PilotAdtMemoryLayout.TrySplitFallibleTypeSpec(t, out string okPart, out _))
            return IsStrictNonNullableType(okPart);
        if (IsDynamicOrAny(t) || IsOptionalType(t)) return false;
        if (t.Contains('|'))
        {
            return !t.Split('|').Select(p => p.Trim()).Any(p => p is "null" or "none" or "None" || IsOptionalType(p));
        }
        return true;
    }

    public static string UnwrapFallibleType(string? typeSpec)
    {
        if (string.IsNullOrWhiteSpace(typeSpec)) return "any";
        string t = typeSpec.Trim();
        return PilotAdtMemoryLayout.TrySplitFallibleTypeSpec(t, out string okPart, out _) ? okPart : t;
    }

    public static string UnwrapOptionalType(string? typeSpec)
    {
        if (string.IsNullOrWhiteSpace(typeSpec)) return "any";
        string t = typeSpec.Trim();
        if (t.EndsWith("?", StringComparison.Ordinal) && t.Length > 1)
            return t[..^1].Trim();
        if ((t.StartsWith("Option[", StringComparison.OrdinalIgnoreCase) && t.EndsWith("]", StringComparison.Ordinal)) ||
            (t.StartsWith("Option<", StringComparison.OrdinalIgnoreCase) && t.EndsWith(">", StringComparison.Ordinal)))
        {
            return t[7..^1].Trim();
        }
        return t;
    }

    public static string EnsureOptionalType(string? typeSpec)
    {
        if (string.IsNullOrWhiteSpace(typeSpec) || typeSpec == "any") return "any?";
        string t = typeSpec.Trim();
        if (IsOptionalType(t)) return t;
        return $"{t}?";
    }

    private void PushScope() => _currentScope = new NullSafetyScope(_currentScope);
    private void PopScope()
    {
        if (_currentScope.Parent != null)
            _currentScope = _currentScope.Parent;
    }

    private void PrescanDeclarations(PilotParser.ProgramContext program)
    {
        if (program.stmt() == null) return;
        foreach (var stmt in program.stmt())
        {
            var comp = stmt.compound_stmt();
            if (comp == null) continue;

            if (comp.func_def() is { } fn)
            {
                RegisterFunctionSignature(fn, null);
            }
            else if (comp.class_def() is { } cls)
            {
                RegisterClassOrStruct(cls.identifier().GetText(), cls.suite());
            }
            else if (comp.struct_def() is { } st)
            {
                RegisterStructDef(st);
            }
            else if (comp.record_def() is { } rec)
            {
                RegisterRecordDef(rec);
            }
            else if (comp.model_def() is { } model)
            {
                RegisterModelDef(model);
            }
            else if (comp.extern_def() is { } ext)
            {
                RegisterExternDef(ext);
            }
        }
    }

    private FunctionSignatureInfo RegisterFunctionSignature(PilotParser.Func_defContext fn, TypeMemberInfo? ownerType)
    {
        string name = fn.identifier().GetText();
        string? retSpec = fn.type_spec()?.GetText().Trim();
        var sig = new FunctionSignatureInfo
        {
            Name = name,
            ReturnType = retSpec ?? "any",
            HasExplicitNonNullReturn = retSpec != null && IsStrictNonNullableType(retSpec)
        };

        if (fn.parameters()?.parameter() != null)
        {
            foreach (var p in fn.parameters().parameter())
            {
                string pName = p.identifier()?.GetText() ?? "_";
                if (pName == "self" && ownerType != null) continue;
                string pType = p.type_annotation()?.type_spec()?.GetText().Trim() ?? "any";
                sig.Parameters.Add((pName, pType));
            }
        }

        if (ownerType != null)
            ownerType.Methods[name] = sig;
        else
            _functions[name] = sig;

        return sig;
    }

    private void RegisterClassOrStruct(string typeName, PilotParser.SuiteContext? suite)
    {
        if (!_types.TryGetValue(typeName, out var info))
        {
            info = new TypeMemberInfo { Name = typeName };
            _types[typeName] = info;
        }
        if (suite?.stmt() == null) return;
        foreach (var s in suite.stmt())
        {
            if (s.compound_stmt()?.func_def() is { } fn)
            {
                RegisterFunctionSignature(fn, info);
            }
            if (s.simple_stmt()?.small_stmt() != null)
            {
                foreach (var sm in s.simple_stmt().small_stmt())
                {
                    if (sm.let_stmt() is { } l && l.binding_pattern()?.identifier() is { } lid)
                    {
                        info.Fields[lid.GetText()] = l.type_annotation()?.type_spec()?.GetText().Trim() ?? "any";
                    }
                    else if (sm.var_stmt() is { } v && v.binding_pattern()?.identifier() is { } vid)
                    {
                        info.Fields[vid.GetText()] = v.type_annotation()?.type_spec()?.GetText().Trim() ?? "any";
                    }
                    else if (sm.annassign_stmt() is { } ann)
                    {
                        info.Fields[ann.test(0).GetText().Trim()] = ann.type_spec().GetText().Trim();
                    }
                }
            }
        }
    }

    private void RegisterStructDef(PilotParser.Struct_defContext st)
    {
        string name = st.identifier().GetText();
        if (st.suite() != null)
        {
            RegisterClassOrStruct(name, st.suite());
            return;
        }
        var info = new TypeMemberInfo { Name = name };
        _types[name] = info;
        if (st.struct_member() != null)
        {
            foreach (var m in st.struct_member())
            {
                info.Fields[m.identifier().GetText()] = m.type_annotation()?.type_spec()?.GetText().Trim() ?? "any";
            }
        }
    }

    private void RegisterRecordDef(PilotParser.Record_defContext rec)
    {
        string name = rec.identifier().GetText();
        var info = new TypeMemberInfo { Name = name };
        _types[name] = info;
        var sig = new FunctionSignatureInfo { Name = name, ReturnType = name, HasExplicitNonNullReturn = true };
        if (rec.adt_field_list()?.adt_field() != null)
        {
            int idx = 0;
            foreach (var f in rec.adt_field_list().adt_field())
            {
                string fName = f.identifier()?.GetText() ?? $"_{idx}";
                string fType = f.type_spec().GetText().Trim();
                info.Fields[fName] = fType;
                sig.Parameters.Add((fName, fType));
                idx++;
            }
        }
        if (rec.struct_member() != null)
        {
            foreach (var m in rec.struct_member())
            {
                string fName = m.identifier().GetText();
                string fType = m.type_annotation()?.type_spec()?.GetText().Trim() ?? "any";
                info.Fields[fName] = fType;
                sig.Parameters.Add((fName, fType));
            }
        }
        _functions[name] = sig;
    }

    private void RegisterModelDef(PilotParser.Model_defContext model)
    {
        // Milestone 12.4: SQL Model / Table schema columns with nullable defaults or `T?` surface as `T?`
        string name = model.identifier().GetText();
        var info = new TypeMemberInfo { Name = name };
        _types[name] = info;
        if (model.suite()?.stmt() == null) return;
        foreach (var s in model.suite().stmt())
        {
            if (s.simple_stmt()?.small_stmt() == null) continue;
            foreach (var sm in s.simple_stmt().small_stmt())
            {
                if (sm.let_stmt() is { } l && l.binding_pattern()?.identifier() is { } lid)
                {
                    string fType = l.type_annotation()?.type_spec()?.GetText().Trim() ?? "any";
                    if (l.test()?.GetText().Trim() is "null" or "none" or "None")
                        fType = EnsureOptionalType(fType);
                    info.Fields[lid.GetText()] = fType;
                }
                else if (sm.var_stmt() is { } v && v.binding_pattern()?.identifier() is { } vid)
                {
                    string fType = v.type_annotation()?.type_spec()?.GetText().Trim() ?? "any";
                    if (v.test()?.GetText().Trim() is "null" or "none" or "None")
                        fType = EnsureOptionalType(fType);
                    info.Fields[vid.GetText()] = fType;
                }
                else if (sm.annassign_stmt() is { } ann)
                {
                    string fType = ann.type_spec().GetText().Trim();
                    if (ann.test().Length > 1 && ann.test(1).GetText().Trim() is "null" or "none" or "None")
                        fType = EnsureOptionalType(fType);
                    info.Fields[ann.test(0).GetText().Trim()] = fType;
                }
            }
        }
    }

    private void RegisterExternDef(PilotParser.Extern_defContext ext)
    {
        // Milestone 12.4: Foreign Interop Null-Boundary Hardening
        // Foreign pointers (`T*`, `ptr`) and unannotated FFI returns automatically surface as `T?` unless `@nonnull`
        if (ext.extern_member() == null) return;
        foreach (var mem in ext.extern_member())
        {
            if (mem.identifier() == null || (mem.DEF() == null && mem.FN() == null)) continue;
            string fnName = mem.identifier().GetText();
            bool hasNonNullAttr = mem.decorators()?.decorator()
                ?.Any(d => d.dotted_name().GetText().Equals("nonnull", StringComparison.OrdinalIgnoreCase)) == true;

            string rawRet = mem.type_spec()?.GetText().Trim() ?? "any?";
            string hardenedRet = HardenForeignReturnType(rawRet, hasNonNullAttr);

            var sig = new FunctionSignatureInfo
            {
                Name = fnName,
                ReturnType = hardenedRet,
                HasExplicitNonNullReturn = !IsOptionalType(hardenedRet)
            };

            if (mem.parameters()?.parameter() != null)
            {
                foreach (var p in mem.parameters().parameter())
                {
                    string pName = p.identifier()?.GetText() ?? "_";
                    string pType = p.type_annotation()?.type_spec()?.GetText().Trim() ?? "any";
                    sig.Parameters.Add((pName, pType));
                }
            }

            _functions[fnName] = sig;
        }
    }

    public static string HardenForeignReturnType(string rawType, bool hasNonNullAttribute = false)
    {
        if (hasNonNullAttribute) return rawType;
        string t = (rawType ?? "any").Trim();
        if (t is "void" or "nil" or "bool" or "int" or "i8" or "i16" or "i32" or "i64"
            or "uint" or "u8" or "u16" or "u32" or "u64" or "float" or "f32" or "f64" or "double")
        {
            return t;
        }
        // Nullable C pointers (`char*`, `User*`, `void*`, `ptr`) and foreign reference returns surface as `T?`
        if (t.EndsWith("*", StringComparison.Ordinal))
        {
            string pointee = t[..^1].Trim();
            if (pointee is "char" or "const char") return "str?";
            return EnsureOptionalType(pointee);
        }
        return EnsureOptionalType(t);
    }

    public override string VisitErrorNode(IErrorNode node)
    {
        if (node.Symbol?.Text == "!")
        {
            Report("NULL005",
                "Unchecked postfix non-null assertion '!' is removed in Pilot. Use 'guard some x else', '??', or explicit '.OrPanic(\"reason\")'.",
                node.Symbol.Line,
                node.Symbol.Column);
        }
        return "any";
    }

    public override string VisitProgram(PilotParser.ProgramContext context)
    {
        if (context.stmt() != null)
        {
            foreach (var s in context.stmt())
                Visit(s);
        }
        return "void";
    }

    public override string VisitSuite(PilotParser.SuiteContext context)
    {
        if (context.simple_stmt() != null)
            return Visit(context.simple_stmt());

        if (context.stmt() != null)
        {
            foreach (var s in context.stmt())
                Visit(s);
        }
        return "void";
    }

    public override string VisitFunc_def(PilotParser.Func_defContext context)
    {
        string fnName = context.identifier().GetText();
        string? retSpec = context.type_spec()?.GetText().Trim();
        bool strictNonNullRet = retSpec != null && IsStrictNonNullableType(retSpec);

        PushScope();
        _currentScope.EnclosingFunctionName = fnName;
        _currentScope.EnclosingReturnType = retSpec;
        _currentScope.EnclosingReturnIsExplicitNonNullable = strictNonNullRet;

        try
        {
            if (context.parameters()?.parameter() != null)
            {
                foreach (var p in context.parameters().parameter())
                {
                    string pName = p.identifier()?.GetText() ?? "_";
                    string? pType = p.type_annotation()?.type_spec()?.GetText().Trim();
                    _currentScope.DeclaredTypes[pName] = pType ?? "any";
                    _currentScope.ExplicitNonNullable[pName] = pType != null && IsStrictNonNullableType(pType);
                }
            }

            if (context.suite() != null)
                Visit(context.suite());
        }
        finally
        {
            PopScope();
        }

        return "void";
    }

    public override string VisitLet_stmt(PilotParser.Let_stmtContext context)
    {
        HandleVarDecl(
            context.binding_pattern(),
            context.type_annotation()?.type_spec()?.GetText().Trim(),
            context.test(),
            context.Start);
        return "void";
    }

    public override string VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        HandleVarDecl(
            context.binding_pattern(),
            context.type_annotation()?.type_spec()?.GetText().Trim(),
            context.test(),
            context.Start);
        return "void";
    }

    public override string VisitConst_stmt(PilotParser.Const_stmtContext context)
    {
        HandleVarDecl(
            context.binding_pattern(),
            context.type_annotation()?.type_spec()?.GetText().Trim(),
            context.test(),
            context.Start);
        return "void";
    }

    private void HandleVarDecl(
        PilotParser.Binding_patternContext? pat,
        string? explicitType,
        PilotParser.TestContext? rhs,
        IToken fallbackToken)
    {
        string? rhsType = rhs != null ? Visit(rhs) : null;
        string varName = pat?.GetText().Trim() ?? "_";
        var errTok = rhs?.Start ?? pat?.Start ?? fallbackToken;

        if (!string.IsNullOrWhiteSpace(explicitType))
        {
            bool isNonNull = IsStrictNonNullableType(explicitType);
            if (isNonNull && rhsType != null && IsOptionalType(rhsType))
            {
                Report("NULL001",
                    $"Cannot assign '{rhs!.GetText()}' (of optional/null type '{FormatDisplayType(rhsType)}') to non-nullable '{varName}: {explicitType}'. Change type to '{explicitType}?' or provide a fallback with '??'.",
                    errTok.Line,
                    errTok.Column);
            }
            if (pat?.identifier() != null)
            {
                _currentScope.DeclaredTypes[varName] = explicitType;
                _currentScope.ExplicitNonNullable[varName] = isNonNull;
                _currentScope.NarrowedTypes.Remove(varName);
            }
        }
        else if (pat?.identifier() != null)
        {
            string inferred = rhsType == "null" ? "any?" : (rhsType ?? "any");
            _currentScope.DeclaredTypes[varName] = inferred;
            _currentScope.ExplicitNonNullable[varName] = false;
            _currentScope.NarrowedTypes.Remove(varName);
        }
    }

    public override string VisitExpr_stmt(PilotParser.Expr_stmtContext context)
    {
        var lhsList = context.testlist();
        var rhsTests = context.test();
        bool hasEqual = context.EQUAL() != null && context.EQUAL().Length > 0;
        bool hasAug = context.augassign() != null && context.augassign().Length > 0;

        if (!hasEqual && !hasAug)
        {
            if (lhsList?.test() != null)
            {
                foreach (var t in lhsList.test())
                    Visit(t);
            }
            return "void";
        }

        var lhs = lhsList?.test().FirstOrDefault();
        string? rhsType = null;
        PilotParser.TestContext? lastRhs = null;
        if (rhsTests != null)
        {
            foreach (var r in rhsTests)
            {
                lastRhs = r;
                rhsType = Visit(r);
            }
        }

        if (lhs != null)
        {
            string lhsText = lhs.GetText().Trim();
            if (IsSimpleIdentifierText(lhsText))
            {
                // Null-coalescing assignment `x ??= fallback` narrows `x` to non-nullable if `fallback` is non-nullable!
                if (hasAug && context.augassign(0).COALESCE_EQUAL() != null)
                {
                    string? currentEff = _currentScope.ResolveEffectiveType(lhsText);
                    if (currentEff != null && rhsType != null && !IsOptionalType(rhsType))
                    {
                        _currentScope.NarrowedTypes[lhsText] = UnwrapOptionalType(currentEff);
                    }
                    return "void";
                }

                var (declType, isExplicitNonNull) = _currentScope.ResolveDeclaration(lhsText);
                if (isExplicitNonNull && declType != null && rhsType != null && IsOptionalType(rhsType))
                {
                    var tok = lastRhs?.Start ?? lhs.Start;
                    Report("NULL001",
                        $"Cannot assign '{lastRhs?.GetText() ?? "none"}' (of optional/null type '{FormatDisplayType(rhsType)}') to non-nullable variable '{lhsText}: {declType}'.",
                        tok.Line,
                        tok.Column);
                }
                else if (declType != null && IsOptionalType(declType))
                {
                    if (rhsType != null && !IsOptionalType(rhsType) && !IsDynamicOrAny(rhsType))
                    {
                        // Reassigning a non-null value narrows the optional variable until next null assignment
                        _currentScope.NarrowedTypes[lhsText] = UnwrapOptionalType(declType);
                    }
                    else
                    {
                        // Reassigning `none` / `T?` clears prior narrowing
                        _currentScope.ClearNarrowing(lhsText);
                    }
                }
            }
            else
            {
                Visit(lhs);
            }
        }

        return "void";
    }

    public override string VisitFlow_stmt(PilotParser.Flow_stmtContext context)
    {
        if (context.RETURN() != null && context.testlist()?.test() is { Length: > 0 } retTests)
        {
            var retExpr = retTests[0];
            string retType = Visit(retExpr);

            if (_currentScope.EnclosingReturnIsExplicitNonNullable &&
                _currentScope.EnclosingReturnType != null &&
                IsOptionalType(retType))
            {
                Report("NULL003",
                    $"Cannot return '{retExpr.GetText()}' (of optional/null type '{FormatDisplayType(retType)}') from function '{_currentScope.EnclosingFunctionName}' with non-nullable return type '{_currentScope.EnclosingReturnType}'.",
                    retExpr.Start.Line,
                    retExpr.Start.Column);
            }
        }
        else if (context.testlist() != null)
        {
            Visit(context.testlist());
        }
        return "void";
    }

    public override string VisitGuard_stmt(PilotParser.Guard_stmtContext context)
    {
        bool hasSome = context.SOME() != null;
        var pat = context.pattern();
        var condTest = context.test();

        string rhsType = condTest != null ? Visit(condTest) : "any";

        // Visit else suite first (before guard narrowing takes effect for the remainder of the enclosing scope)
        if (context.suite() != null)
        {
            PushScope();
            try
            {
                // If guard was `guard x == none else { ... }`, then inside else `x` is non-null!
                if (!hasSome && pat == null && condTest != null &&
                    TryExtractNullComparison(condTest, out string? eqVar, out bool isNotEqual) &&
                    !isNotEqual && eqVar != null)
                {
                    string? orig = _currentScope.ResolveEffectiveType(eqVar);
                    if (orig != null)
                        _currentScope.NarrowedTypes[eqVar] = UnwrapOptionalType(orig);
                }
                Visit(context.suite());
            }
            finally
            {
                PopScope();
            }
        }

        // Apply narrowing in the enclosing scope after `guard ... else { <exit> }`!
        if (pat != null && (context.LET() != null || context.VAR() != null || hasSome))
        {
            string unwrapped = UnwrapOptionalType(UnwrapFallibleType(rhsType));
            BindPatternVariablesInScope(pat, unwrapped, _currentScope);
        }
        else if (hasSome && condTest != null)
        {
            string varText = condTest.GetText().Trim();
            if (IsSimpleIdentifierText(varText))
            {
                string? orig = _currentScope.ResolveEffectiveType(varText);
                _currentScope.NarrowedTypes[varText] = UnwrapOptionalType(orig ?? rhsType);
            }
        }
        else if (condTest != null)
        {
            ApplyConditionNarrowing(condTest, _currentScope, whenTrue: true);
        }

        if (context.pattern_guard()?.test() != null)
        {
            Visit(context.pattern_guard().test());
        }

        return "void";
    }

    public override string VisitIf_stmt(PilotParser.If_stmtContext context)
    {
        var tests = context.test();
        var suites = context.suite();
        var patterns = context.pattern();
        var guards = context.pattern_guard();

        // Track variables where an `if x == none { return; }` branch guarantees early exit
        var postIfNarrowings = new Dictionary<string, string>(StringComparer.Ordinal);

        for (int i = 0; i < tests.Length; i++)
        {
            var prevSuite = i > 0 ? suites[i - 1] : null;
            var branchPat = FindBranchPattern(patterns, tests[i], prevSuite);
            bool hasSome = Pilot.Runtime.Execution.PilotInterpreterVisitor.HasBranchSomeKeyword(context, tests[i], prevSuite);
            var branchGuard = FindBranchGuard(guards, tests[i], suites[i]);

            string rhsType = Visit(tests[i]);

            PushScope();
            try
            {
                if (branchPat != null)
                {
                    string unwrapped = UnwrapOptionalType(rhsType);
                    BindPatternVariablesInScope(branchPat, unwrapped, _currentScope);
                }
                else if (hasSome)
                {
                    string varText = tests[i].GetText().Trim();
                    if (IsSimpleIdentifierText(varText))
                    {
                        string? orig = _currentScope.ResolveEffectiveType(varText);
                        _currentScope.NarrowedTypes[varText] = UnwrapOptionalType(orig ?? rhsType);
                    }
                }
                else
                {
                    ApplyConditionNarrowing(tests[i], _currentScope, whenTrue: true);
                }

                if (branchGuard?.test() != null)
                    Visit(branchGuard.test());

                Visit(suites[i]);
            }
            finally
            {
                PopScope();
            }

            // Early-exit narrowing: `if x == none { return; }` narrows `x` after the `if` statement!
            if (i == 0 && tests.Length == 1 && suites.Length == 1 && SuiteAlwaysExits(suites[0]))
            {
                if (!hasSome && branchPat == null &&
                    TryExtractNullComparison(tests[0], out string? cmpVar, out bool isNotEqual) &&
                    !isNotEqual && cmpVar != null)
                {
                    string? orig = _currentScope.ResolveEffectiveType(cmpVar);
                    if (orig != null && IsOptionalType(orig))
                    {
                        postIfNarrowings[cmpVar] = UnwrapOptionalType(orig);
                    }
                }
            }
        }

        if (suites.Length > tests.Length)
        {
            PushScope();
            try
            {
                // If `if x == none { ... } else { ... }` or `if some x { ... } else { ... }`:
                if (tests.Length == 1)
                {
                    ApplyConditionNarrowing(tests[0], _currentScope, whenTrue: false);
                }
                Visit(suites[^1]);
            }
            finally
            {
                PopScope();
            }
        }

        foreach (var kv in postIfNarrowings)
        {
            _currentScope.NarrowedTypes[kv.Key] = kv.Value;
        }

        return "void";
    }

    public override string VisitWhile_stmt(PilotParser.While_stmtContext context)
    {
        var cond = context.test(0);
        var pat = context.pattern();
        bool hasSome = context.SOME() != null;

        string rhsType = cond != null ? Visit(cond) : "any";

        PushScope();
        try
        {
            if (pat != null)
            {
                BindPatternVariablesInScope(pat, UnwrapOptionalType(rhsType), _currentScope);
            }
            else if (hasSome && cond != null)
            {
                string varText = cond.GetText().Trim();
                if (IsSimpleIdentifierText(varText))
                {
                    string? orig = _currentScope.ResolveEffectiveType(varText);
                    _currentScope.NarrowedTypes[varText] = UnwrapOptionalType(orig ?? rhsType);
                }
            }
            else if (cond != null)
            {
                ApplyConditionNarrowing(cond, _currentScope, whenTrue: true);
            }

            if (context.pattern_guard()?.test() != null)
                Visit(context.pattern_guard().test());

            if (context.suite() != null)
            {
                foreach (var s in context.suite())
                    Visit(s);
            }
        }
        finally
        {
            PopScope();
        }

        return "void";
    }

    public override string VisitMatch_stmt(PilotParser.Match_stmtContext context)
    {
        string scrutType = context.test() != null ? Visit(context.test()) : "any";
        if (context.match_suite()?.match_arm() != null)
        {
            foreach (var arm in context.match_suite().match_arm())
            {
                PushScope();
                try
                {
                    if (arm.pattern() != null)
                        BindPatternVariablesInScope(arm.pattern(), scrutType, _currentScope);
                    if (arm.test() != null)
                        Visit(arm.test());
                    if (arm.suite() != null)
                        Visit(arm.suite());
                }
                finally
                {
                    PopScope();
                }
            }
        }
        return "any";
    }

    public override string VisitCoalesce_expr(PilotParser.Coalesce_exprContext context)
    {
        var parts = context.or_test();
        if (parts == null || parts.Length == 0) return "any";
        string current = Visit(parts[0]);
        var catches = context.catch_clause();
        if (catches != null && catches.Length > 0)
        {
            foreach (var cc in catches)
            {
                PushScope();
                if (cc.identifier() != null)
                {
                    _currentScope.DeclaredTypes[cc.identifier().GetText()] = "any";
                }
                Visit(cc);
                PopScope();
            }
            current = UnwrapFallibleType(current);
        }
        for (int i = 1; i < parts.Length; i++)
        {
            string fallbackType = Visit(parts[i]);
            if (IsOptionalType(current))
            {
                // `opt ?? non_null_fallback` narrows `T?` -> `T`!
                current = IsOptionalType(fallbackType)
                    ? EnsureOptionalType(UnwrapOptionalType(current))
                    : UnwrapOptionalType(current);
            }
        }
        return current;
    }

    public override string VisitAtom_expr(PilotParser.Atom_exprContext context)
    {
        var atom = context.atom();
        string currentType = Visit(atom) ?? "any";
        string exprPrefixText = atom.GetText();
        var trailers = context.trailer();

        if (trailers == null || trailers.Length == 0)
            return currentType;

        // Check direct function call `fn(arg0, arg1, ...)`
        string? atomIdent = atom.identifier()?.GetText();
        for (int i = 0; i < trailers.Length; i++)
        {
            var tr = trailers[i];

            if (tr.QUESTION() != null && tr.LSQB() == null && tr.LPAR() == null)
            {
                currentType = UnwrapOptionalType(UnwrapFallibleType(currentType));
                exprPrefixText += "?";
                continue;
            }

            if (tr.DOT() != null && tr.identifier() != null)
            {
                string memberName = tr.identifier().GetText();
                bool isNextCall = i + 1 < trailers.Length && trailers[i + 1].LPAR() != null;

                if (AllowedOptionalMembers.Contains(memberName))
                {
                    if (memberName is "or_panic" or "OrPanic" or "unwrap_or" or "UnwrapOr")
                    {
                        currentType = UnwrapOptionalType(currentType);
                        if (isNextCall)
                        {
                            VisitTrailerArguments(trailers[i + 1], null, memberName);
                            exprPrefixText += $".{memberName}({trailers[i + 1].arglist()?.GetText() ?? ""})";
                            i++;
                        }
                        continue;
                    }
                    if (memberName is "is_some" or "IsSome" or "is_none" or "IsNone" or "is_unboxed" or "IsUnboxed" or "has_niche" or "HasNiche")
                    {
                        currentType = "bool";
                        continue;
                    }
                    if (memberName is "to_string" or "ToString" or "__str__" or "type_of" or "GetType")
                    {
                        currentType = "str";
                        if (isNextCall) i++;
                        continue;
                    }
                }

                if (IsOptionalType(currentType))
                {
                    var dotTok = tr.DOT().Symbol;
                    Report("NULL002",
                        $"Cannot access member '.{memberName}' on optional expression '{exprPrefixText}' of type '{FormatDisplayType(currentType)}' without narrowing ('guard some', 'if some', '!= none') or safe navigation ('?.').",
                        dotTok.Line,
                        dotTok.Column);
                }

                string unwrappedBase = UnwrapOptionalType(currentType);
                currentType = ResolveMemberType(unwrappedBase, memberName, isNextCall ? trailers[i + 1] : null);
                if (isNextCall)
                {
                    exprPrefixText += $".{memberName}({trailers[i + 1].arglist()?.GetText() ?? ""})";
                    i++;
                }
                else
                {
                    exprPrefixText += $".{memberName}";
                }
            }
            else if (tr.SAFE_NAV() != null && tr.identifier() != null)
            {
                string memberName = tr.identifier().GetText();
                bool isNextCall = i + 1 < trailers.Length && trailers[i + 1].LPAR() != null;
                string unwrappedBase = UnwrapOptionalType(currentType);
                string resolved = ResolveMemberType(unwrappedBase, memberName, isNextCall ? trailers[i + 1] : null);
                currentType = EnsureOptionalType(resolved);
                if (isNextCall)
                {
                    exprPrefixText += $"?.{memberName}({trailers[i + 1].arglist()?.GetText() ?? ""})";
                    i++;
                }
                else
                {
                    exprPrefixText += $"?.{memberName}";
                }
            }
            else if (tr.LSQB() != null)
            {
                bool isOptionalIndex = tr.SAFE_NAV() != null || tr.QUESTION() != null;
                if (!isOptionalIndex && IsOptionalType(currentType))
                {
                    var lsqbTok = tr.LSQB().Symbol;
                    Report("NULL002",
                        $"Cannot index into optional expression '{exprPrefixText}' of type '{FormatDisplayType(currentType)}' without narrowing ('guard some', 'if some', '!= none').",
                        lsqbTok.Line,
                        lsqbTok.Column);
                }
                if (tr.subscriptlist() != null)
                    Visit(tr.subscriptlist());
                string elemType = ExtractElementType(UnwrapOptionalType(currentType));
                currentType = isOptionalIndex ? EnsureOptionalType(elemType) : elemType;
                exprPrefixText += isOptionalIndex ? $"?[{tr.subscriptlist()?.GetText() ?? ""}]" : $"[{tr.subscriptlist()?.GetText() ?? ""}]";
            }
            else if (tr.LPAR() != null)
            {
                bool isOptionalCall = tr.SAFE_NAV() != null || tr.QUESTION() != null;
                if (!isOptionalCall && i == 0 && atomIdent != null)
                {
                    if (atomIdent is "some" or "Some")
                    {
                        var firstArg = tr.arglist()?.argument().FirstOrDefault()?.test();
                        string innerT = firstArg != null ? Visit(firstArg) : "any";
                        currentType = EnsureOptionalType(innerT);
                        exprPrefixText = $"{atomIdent}({tr.arglist()?.GetText() ?? ""})";
                        continue;
                    }

                    if (_functions.TryGetValue(atomIdent, out var fnSig))
                    {
                        VisitTrailerArguments(tr, fnSig, atomIdent);
                        currentType = fnSig.ReturnType;
                        exprPrefixText = $"{atomIdent}({tr.arglist()?.GetText() ?? ""})";
                        continue;
                    }
                }

                if (!isOptionalCall && IsOptionalType(currentType))
                {
                    var lparTok = tr.LPAR().Symbol;
                    Report("NULL002",
                        $"Cannot invoke optional expression '{exprPrefixText}' of type '{FormatDisplayType(currentType)}' without narrowing.",
                        lparTok.Line,
                        lparTok.Column);
                }
                VisitTrailerArguments(tr, null, exprPrefixText);
                currentType = isOptionalCall ? "any?" : "any";
            }
            else
            {
                Visit(tr);
            }
        }

        return currentType;
    }

    private string ResolveMemberType(string baseType, string memberName, PilotParser.TrailerContext? callTrailer)
    {
        string cleanBase = StripGenericSuffix(baseType);
        if (cleanBase is "str" or "string")
        {
            if (callTrailer != null)
                VisitTrailerArguments(callTrailer, null, $"{cleanBase}.{memberName}");
            return memberName switch
            {
                "length" or "Length" or "count" or "Count" or "code_point" or "CodePoint" => "int",
                "to_upper" or "ToUpper" or "to_lower" or "ToLower" or "trim" or "Trim"
                or "trim_start" or "TrimStart" or "trim_end" or "TrimEnd"
                or "substring" or "Substring" or "replace" or "Replace"
                or "pad_left" or "PadLeft" or "pad_right" or "PadRight" => "str",
                "to_int" or "ToInt" or "index_of" or "IndexOf" => "int?",
                "parse_int" or "ParseInt" => "int!ParseError",
                "to_float" or "ToFloat" => "float?",
                "parse_float" or "ParseFloat" => "float!ParseError",
                "to_bool" or "ToBool" or "parse_bool" or "ParseBool" => "bool?",
                "starts_with" or "StartsWith" or "ends_with" or "EndsWith" or "contains" or "Contains"
                or "is_digit" or "IsDigit" or "is_alpha" or "IsAlpha" or "is_whitespace" or "IsWhitespace"
                or "is_empty" or "IsEmpty" => "bool",
                "split" or "Split" or "lines" or "Lines" or "chars" or "Chars" => "str[]",
                _ => "any"
            };
        }

        if (_types.TryGetValue(cleanBase, out var typeInfo))
        {
            if (typeInfo.Methods.TryGetValue(memberName, out var methodSig))
            {
                if (callTrailer != null)
                    VisitTrailerArguments(callTrailer, methodSig, $"{cleanBase}.{memberName}");
                return methodSig.ReturnType;
            }
            if (typeInfo.Fields.TryGetValue(memberName, out var fieldType))
            {
                return fieldType;
            }
        }

        if (callTrailer != null)
            VisitTrailerArguments(callTrailer, null, $"{cleanBase}.{memberName}");

        return "any";
    }

    private void VisitTrailerArguments(PilotParser.TrailerContext callTrailer, FunctionSignatureInfo? sig, string callableName)
    {
        var args = callTrailer.arglist()?.argument();
        if (args == null) return;

        for (int idx = 0; idx < args.Length; idx++)
        {
            var arg = args[idx];
            var argExpr = arg.test();
            if (argExpr == null) continue;
            string argType = Visit(argExpr);

            if (sig != null)
            {
                (string Name, string Type)? matchedParam = null;
                if (arg.identifier() != null)
                {
                    string named = arg.identifier().GetText();
                    var found = sig.Parameters.FirstOrDefault(p => p.Name == named);
                    if (!string.IsNullOrEmpty(found.Name)) matchedParam = found;
                }
                else if (idx < sig.Parameters.Count)
                {
                    matchedParam = sig.Parameters[idx];
                }

                if (matchedParam is { } mp && IsStrictNonNullableType(mp.Type) && IsOptionalType(argType))
                {
                    Report("NULL004",
                        $"Cannot pass '{argExpr.GetText()}' (of optional/null type '{FormatDisplayType(argType)}') to non-nullable parameter '{mp.Name}: {mp.Type}' of '{callableName}'.",
                        argExpr.Start.Line,
                        argExpr.Start.Column);
                }
            }
        }
    }

    public override string VisitAtom(PilotParser.AtomContext context)
    {
        if (context.literal() is { } lit)
        {
            if (lit.NONE() != null || lit.NULL() != null) return "null";
            if (lit.INTEGER() != null || lit.HEX_INTEGER() != null || lit.BIN_INTEGER() != null || lit.OCT_INTEGER() != null) return "int";
            if (lit.FLOAT_NUMBER() != null) return "float";
            if (lit.STRING().Length > 0 || lit.TRIPLE_STRING().Length > 0 || lit.FSTRING().Length > 0) return "str";
            if (lit.TRUE() != null || lit.FALSE() != null) return "bool";
            return "any";
        }

        if (context.identifier() is { } idCtx)
        {
            string name = idCtx.GetText();
            if (name is "none" or "None" or "null") return "null";
            if (name is "true" or "True" or "false" or "False") return "bool";
            return _currentScope.ResolveEffectiveType(name) ?? "any";
        }

        if (context.LPAR() != null && context.testlist_comp()?.test() is { Length: 1 } parenTests &&
            context.testlist_comp().COMMA().Length == 0)
        {
            return Visit(parenTests[0]);
        }

        if (context.for_stmt() is { } forStmt)
        {
            Visit(forStmt);
            var loopInfo = Pilot.Analysis.Loops.PilotForLoopAnalyzer.AnalyzeSingleLoop(forStmt, isExpressionPositionOverride: true);
            if (loopInfo.ReturnMode == Pilot.Analysis.Loops.LoopReturnMode.ScalarBreak && !loopInfo.HasCompletionBranch)
            {
                return "any?";
            }
            return "any";
        }

        return base.VisitAtom(context) ?? "any";
    }

    private void ApplyConditionNarrowing(PilotParser.TestContext cond, NullSafetyScope targetScope, bool whenTrue)
    {
        // Walk AND/OR conjunctions for `x != none` / `x == none` / `x is not null` / `x is some(y)`
        var comps = FindAllComparisons(cond);
        foreach (var comp in comps)
        {
            if (TryExtractComparisonNarrowing(comp, out string? varName, out bool narrowsWhenTrue) && varName != null)
            {
                if (narrowsWhenTrue == whenTrue)
                {
                    string? orig = targetScope.ResolveEffectiveType(varName);
                    if (orig != null && IsOptionalType(orig))
                    {
                        targetScope.NarrowedTypes[varName] = UnwrapOptionalType(orig);
                    }
                }
            }
        }
    }

    private static bool TryExtractNullComparison(PilotParser.TestContext test, out string? varName, out bool isNotEqual)
    {
        var comps = FindAllComparisons(test);
        if (comps.Count == 1 && TryExtractComparisonNarrowing(comps[0], out varName, out isNotEqual))
            return true;
        varName = null;
        isNotEqual = false;
        return false;
    }

    private static List<PilotParser.ComparisonContext> FindAllComparisons(IParseTree node)
    {
        var list = new List<PilotParser.ComparisonContext>();
        CollectComparisons(node, list);
        return list;
    }

    private static void CollectComparisons(IParseTree node, List<PilotParser.ComparisonContext> list)
    {
        if (node is PilotParser.ComparisonContext c)
        {
            list.Add(c);
            return;
        }
        for (int i = 0; i < node.ChildCount; i++)
            CollectComparisons(node.GetChild(i), list);
    }

    private static bool TryExtractComparisonNarrowing(
        PilotParser.ComparisonContext comp,
        out string? varName,
        out bool narrowsWhenTrue)
    {
        varName = null;
        narrowsWhenTrue = false;

        var ranges = comp.range_expr();
        if (ranges == null || ranges.Length == 0) return false;

        // `x is not null` / `x is not none` / `x is some(...)`
        if (comp.IS() != null && comp.IS().Length > 0 && comp.pattern() != null && comp.pattern().Length > 0)
        {
            string lhs = ranges[0].GetText().Trim();
            if (!IsSimpleIdentifierText(lhs)) return false;
            string patText = comp.pattern(0).GetText().Trim();
            bool hasIsNot = comp.NOT() != null && comp.NOT().Length > 0;
            if (patText is "null" or "none" or "None")
            {
                varName = lhs;
                narrowsWhenTrue = hasIsNot;
                return true;
            }
            if (patText.StartsWith("not null", StringComparison.Ordinal) ||
                patText.StartsWith("not none", StringComparison.Ordinal) ||
                patText.StartsWith("some", StringComparison.Ordinal))
            {
                varName = lhs;
                narrowsWhenTrue = !hasIsNot;
                return true;
            }
        }

        // `x != none`, `x != null`, `none != x`, `x == none`, `x == null`
        if (ranges.Length == 2 && comp.comp_op() != null && comp.comp_op().Length == 1)
        {
            var op = comp.comp_op(0);
            if (op.NOTEQUAL() == null && op.EQEQUAL() == null) return false;
            bool isNe = op.NOTEQUAL() != null;

            string left = ranges[0].GetText().Trim();
            string right = ranges[1].GetText().Trim();

            if (right is "null" or "none" or "None" && IsSimpleIdentifierText(left))
            {
                varName = left;
                narrowsWhenTrue = isNe;
                return true;
            }
            if (left is "null" or "none" or "None" && IsSimpleIdentifierText(right))
            {
                varName = right;
                narrowsWhenTrue = isNe;
                return true;
            }
        }

        return false;
    }

    private void BindPatternVariablesInScope(PilotParser.PatternContext pat, string scrutineeType, NullSafetyScope scope)
    {
        var orBranches = pat.or_pattern()?.and_pattern();
        if (orBranches == null) return;
        foreach (var ab in orBranches)
        {
            foreach (var np in ab.not_pattern())
            {
                BindNotPatternInScope(np, scrutineeType, scope);
            }
        }
    }

    private void BindNotPatternInScope(PilotParser.Not_patternContext np, string typeSpec, NullSafetyScope scope)
    {
        if (np.AT() != null && np.identifier() != null)
        {
            string alias = np.identifier().GetText();
            if (alias != "_") scope.DeclaredTypes[alias] = typeSpec;
            if (np.not_pattern() != null)
                BindNotPatternInScope(np.not_pattern(), typeSpec, scope);
            return;
        }
        if (np.NOT() != null) return;

        var prim = np.primary_pattern();
        if (prim == null) return;

        if (prim.type_or_var_pattern() is { } tv)
        {
            if ((tv.LET() != null || tv.VAR() != null) && tv.dotted_name() == null && tv.identifier() != null)
            {
                string vName = tv.identifier().GetText();
                if (vName != "_") scope.DeclaredTypes[vName] = typeSpec;
                return;
            }

            string targetName = tv.dotted_name()?.GetText() ?? "";
            if (targetName is "some" or "Some" or "Option.some" or "Option.Some")
            {
                string innerType = UnwrapOptionalType(typeSpec);
                if (tv.pattern_arg() != null && tv.pattern_arg().Length == 1)
                {
                    BindPatternVariablesInScope(tv.pattern_arg(0).pattern(), innerType, scope);
                }
                if (tv.identifier() != null)
                {
                    string tid = tv.identifier().GetText();
                    if (tid != "_") scope.DeclaredTypes[tid] = innerType;
                }
                return;
            }

            if (tv.identifier() != null)
            {
                string tid = tv.identifier().GetText();
                if (tid != "_") scope.DeclaredTypes[tid] = targetName.Length > 0 ? targetName : typeSpec;
                return;
            }

            if (tv.LPAR() == null && tv.property_pattern() == null && tv.generic_pattern_suffix() == null &&
                tv.dotted_name()?.identifier().Length == 1)
            {
                string bare = tv.dotted_name().GetText();
                if (bare is not ("_" or "null" or "none" or "None" or "true" or "false"))
                {
                    scope.DeclaredTypes[bare] = typeSpec;
                }
            }
        }
    }

    private static bool SuiteAlwaysExits(PilotParser.SuiteContext suite)
    {
        if (suite.simple_stmt() != null)
            return SimpleStmtExits(suite.simple_stmt());

        var stmts = suite.stmt();
        if (stmts == null || stmts.Length == 0) return false;
        foreach (var s in stmts)
        {
            if (s.simple_stmt() != null && SimpleStmtExits(s.simple_stmt()))
                return true;
        }
        return false;
    }

    private static bool SimpleStmtExits(PilotParser.Simple_stmtContext simple)
    {
        if (simple.small_stmt() == null) return false;
        foreach (var sm in simple.small_stmt())
        {
            if (sm.flow_stmt() is { } fl &&
                (fl.RETURN() != null || fl.RAISE() != null || fl.BREAK() != null || fl.CONTINUE() != null))
            {
                return true;
            }
        }
        return false;
    }

    private static PilotParser.PatternContext? FindBranchPattern(
        PilotParser.PatternContext[]? patterns,
        PilotParser.TestContext branchTest,
        PilotParser.SuiteContext? prevSuite)
    {
        if (patterns == null || patterns.Length == 0) return null;
        int minTok = prevSuite != null ? prevSuite.Stop.TokenIndex : -1;
        int maxTok = branchTest.Start.TokenIndex;
        foreach (var p in patterns)
        {
            if (p.Start.TokenIndex > minTok && p.Start.TokenIndex < maxTok)
                return p;
        }
        return null;
    }

    private static PilotParser.Pattern_guardContext? FindBranchGuard(
        PilotParser.Pattern_guardContext[]? guards,
        PilotParser.TestContext branchTest,
        PilotParser.SuiteContext branchSuite)
    {
        if (guards == null || guards.Length == 0) return null;
        int minTok = branchTest.Stop.TokenIndex;
        int maxTok = branchSuite.Start.TokenIndex;
        foreach (var g in guards)
        {
            if (g.Start.TokenIndex > minTok && g.Start.TokenIndex < maxTok)
                return g;
        }
        return null;
    }

    private static string ExtractElementType(string collectionType)
    {
        if (collectionType.EndsWith("[]", StringComparison.Ordinal))
            return collectionType[..^2];
        int lb = collectionType.IndexOf('[');
        int rb = collectionType.LastIndexOf(']');
        if (lb > 0 && rb > lb)
            return collectionType[(lb + 1)..rb].Trim();
        return "any";
    }

    private static string StripGenericSuffix(string typeName)
    {
        string s = typeName.Trim();
        int lb = s.IndexOfAny(new[] { '[', '<' });
        return lb > 0 ? s[..lb].Trim() : s;
    }

    private static bool IsSimpleIdentifierText(string text)
    {
        if (string.IsNullOrEmpty(text)) return false;
        if (!char.IsLetter(text[0]) && text[0] != '_') return false;
        for (int i = 1; i < text.Length; i++)
        {
            if (!char.IsLetterOrDigit(text[i]) && text[i] != '_')
                return false;
        }
        return true;
    }

    private static string FormatDisplayType(string typeSpec) =>
        typeSpec is "null" or "none" ? "none" : typeSpec;

    private void Report(string code, string message, int line, int column)
    {
        _diagnostics.Add(new PilotAnalysisDiagnostic(code, message, line, column, isError: true));
    }
}
