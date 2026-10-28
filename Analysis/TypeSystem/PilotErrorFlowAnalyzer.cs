using System;
using System.Collections.Generic;
using System.Linq;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Runtime.Values;

namespace Pilot.Analysis.TypeSystem;

/// <summary>
/// Milestone 13: Values-First Error System Analyzer (`ERR001`–`ERR005`).
/// Enforces:
/// - `ERR001`: Must-use fallible returns (`T ! E` / `T!`) at statement level unless handled by `?`, `catch`, `match`, assignment, or `ignore_err(...)`.
/// - `ERR002`: Type-safe postfix `?` error propagation with automatic `adapter E1 -> E2` verification (and `T?` propagation inside `U?` functions).
/// - `ERR003`: Call-site `catch err => match err` exhaustiveness and partial `catch` error-union narrowing (`T ! (E1 | E2)` -> `T ! E2`).
/// - `ERR004`: Fallible return contract enforcement (`fail(...)` inside non-fallible functions or undeclared error variants).
/// - `ERR005`: Error stratification lint scoping legacy `try`/`except`/`raise` strictly to FFI boundaries and `isolate` fault supervisors.
/// </summary>
public sealed class PilotErrorFlowAnalyzer : PilotBaseVisitor<string>
{
    public const string UnhandledErrorReturnCode = "ERR001";
    public const string IncompatibleErrorPropagationCode = "ERR002";
    public const string NonExhaustiveCatchCode = "ERR003";
    public const string InvalidFailReturnCode = "ERR004";
    public const string ErrorStratificationLintCode = "ERR005";

    private sealed class FallibleFunctionInfo
    {
        public string Name { get; init; } = "";
        public string RawReturnType { get; set; } = "void";
        public bool IsFallible { get; set; }
        public bool HasInferredErrorSet { get; set; }
        public string OkType { get; set; } = "void";
        public List<string> ErrorTypes { get; } = new();
        public bool AllowExceptions { get; set; }

        public string EffectiveReturnType =>
            !IsFallible
                ? RawReturnType
                : ErrorTypes.Count > 0
                    ? $"{OkType} ! {string.Join(" | ", ErrorTypes)}"
                    : $"{OkType}!";
    }

    private sealed class ErrorScope
    {
        public ErrorScope? Parent { get; }
        public Dictionary<string, string> VariableTypes { get; } = new(StringComparer.Ordinal);
        public FallibleFunctionInfo? EnclosingFunction { get; set; }
        public bool InIsolate { get; set; }
        public bool InForeignBoundary { get; set; }

        public ErrorScope(ErrorScope? parent)
        {
            Parent = parent;
            if (parent != null)
            {
                EnclosingFunction = parent.EnclosingFunction;
                InIsolate = parent.InIsolate;
                InForeignBoundary = parent.InForeignBoundary;
            }
        }

        public string? ResolveVariable(string name)
        {
            for (var s = this; s != null; s = s.Parent)
            {
                if (s.VariableTypes.TryGetValue(name, out var t))
                    return t;
            }
            return null;
        }
    }

    private readonly Dictionary<string, FallibleFunctionInfo> _functions = new(StringComparer.Ordinal);
    private readonly Dictionary<string, List<string>> _adtVariants = new(StringComparer.Ordinal);
    private readonly Dictionary<string, string> _variantToAdt = new(StringComparer.Ordinal);
    private readonly HashSet<string> _records = new(StringComparer.Ordinal);
    private readonly List<(string FromType, string ToType)> _adapters = new();
    private readonly List<PilotAnalysisDiagnostic> _diagnostics = new();
    private ErrorScope _currentScope = new(null);

    public IReadOnlyList<PilotAnalysisDiagnostic> Diagnostics => _diagnostics;

    public IReadOnlyDictionary<string, string> InferredFunctionReturnTypes =>
        _functions.ToDictionary(kvp => kvp.Key, kvp => kvp.Value.EffectiveReturnType, StringComparer.Ordinal);

    public void CheckProgram(PilotParser.ProgramContext program)
    {
        PrescanDeclarations(program);
        ResolveInferredErrorSets(program);
        VisitProgram(program);
    }

    private void PushScope() => _currentScope = new ErrorScope(_currentScope);
    private void PopScope()
    {
        if (_currentScope.Parent != null)
            _currentScope = _currentScope.Parent;
    }

    #region Pass 1: Pre-Scan ADTs, Records, Adapters & Function Signatures

    private void PrescanDeclarations(IParseTree node)
    {
        switch (node)
        {
            case PilotParser.Type_alias_stmtContext ta:
                RegisterTypeAlias(ta);
                break;
            case PilotParser.Enum_defContext en:
                RegisterEnum(en);
                break;
            case PilotParser.Record_defContext rec:
                _records.Add(rec.identifier().GetText());
                break;
            case PilotParser.Struct_defContext st:
                _records.Add(st.identifier().GetText());
                break;
            case PilotParser.Adapter_defContext ad:
                RegisterAdapter(ad);
                break;
            case PilotParser.Func_defContext fn:
                RegisterFunction(fn);
                break;
        }

        for (int i = 0; i < node.ChildCount; i++)
        {
            PrescanDeclarations(node.GetChild(i));
        }
    }

    private void RegisterTypeAlias(PilotParser.Type_alias_stmtContext ctx)
    {
        string name = ctx.identifier().GetText();
        var variants = new List<string>();
        if (ctx.adt_sum_body() is { } sum)
        {
            for (int i = 0; i < sum.ChildCount; i++)
            {
                var ch = sum.GetChild(i);
                string? vName = ch switch
                {
                    PilotParser.IdentifierContext id => id.GetText(),
                    PilotParser.Adt_variantContext vc => vc.identifier().GetText(),
                    PilotParser.Adt_variant_payloadContext vpc => vpc.identifier().GetText(),
                    _ => null
                };
                if (!string.IsNullOrEmpty(vName))
                {
                    variants.Add(vName!);
                    _variantToAdt[vName!] = name;
                }
            }
            if (variants.Count > 0)
                _adtVariants[name] = variants;
            return;
        }

        var branches = ctx.type_spec()?.union_type()?.intersection_type();
        if (branches != null && branches.Length >= 2)
        {
            foreach (var b in branches)
            {
                string t = b.GetText().Trim();
                if (!string.IsNullOrEmpty(t) && char.IsUpper(t[0]))
                {
                    variants.Add(t);
                    if (!_variantToAdt.ContainsKey(t))
                        _variantToAdt[t] = name;
                }
            }
            if (variants.Count > 0)
                _adtVariants[name] = variants;
        }
    }

    private void RegisterEnum(PilotParser.Enum_defContext ctx)
    {
        string name = ctx.identifier().GetText();
        var variants = new List<string>();
        if (ctx.enum_suite()?.enum_member() != null)
        {
            foreach (var m in ctx.enum_suite().enum_member())
            {
                string vName = m.identifier().GetText();
                variants.Add(vName);
                _variantToAdt[vName] = name;
            }
        }
        _adtVariants[name] = variants;
    }

    private void RegisterAdapter(PilotParser.Adapter_defContext ctx)
    {
        if (ctx.type_spec() != null && ctx.type_spec().Length >= 2)
        {
            string fromType = ctx.type_spec(0).GetText().Trim();
            string toType = ctx.type_spec(1).GetText().Trim();
            _adapters.Add((fromType, toType));
        }
    }

    private void RegisterFunction(PilotParser.Func_defContext fn)
    {
        string name = fn.identifier().GetText();
        string rawRet = fn.type_spec()?.GetText().Trim() ?? "void";
        bool allowExceptions = fn.decorators()?.decorator()?.Any(d =>
        {
            string dn = d.dotted_name().GetText();
            return dn.Equals("ffi_boundary", StringComparison.OrdinalIgnoreCase) ||
                   dn.Equals("allow_exceptions", StringComparison.OrdinalIgnoreCase) ||
                   dn.Equals("supervisor", StringComparison.OrdinalIgnoreCase);
        }) == true;

        var info = new FallibleFunctionInfo
        {
            Name = name,
            RawReturnType = rawRet,
            AllowExceptions = allowExceptions
        };

        if (PilotAdtMemoryLayout.TrySplitFallibleTypeSpec(rawRet, out string okSpec, out string errSpec))
        {
            info.IsFallible = true;
            info.OkType = okSpec;
            if (string.IsNullOrEmpty(errSpec))
            {
                info.HasInferredErrorSet = true;
            }
            else
            {
                foreach (var part in SplitUnionTypes(errSpec))
                {
                    if (!info.ErrorTypes.Contains(part, StringComparer.Ordinal))
                        info.ErrorTypes.Add(part);
                }
            }
        }
        else
        {
            info.OkType = rawRet;
        }

        _functions[name] = info;
    }

    private void ResolveInferredErrorSets(PilotParser.ProgramContext program)
    {
        // Collect `fail(...)` and `?` error types inside functions with `-> T!`
        var funcContexts = new List<PilotParser.Func_defContext>();
        CollectFuncContexts(program, funcContexts);

        bool changed = true;
        int pass = 0;
        while (changed && pass++ < 4)
        {
            changed = false;
            foreach (var fnCtx in funcContexts)
            {
                string name = fnCtx.identifier().GetText();
                if (!_functions.TryGetValue(name, out var info) || !info.HasInferredErrorSet || fnCtx.suite() == null)
                    continue;

                var discovered = new List<string>();
                CollectBodyErrors(fnCtx.suite(), discovered);
                foreach (var err in discovered)
                {
                    if (!string.IsNullOrEmpty(err) && err != "any" && !info.ErrorTypes.Contains(err, StringComparer.Ordinal))
                    {
                        info.ErrorTypes.Add(err);
                        changed = true;
                    }
                }
            }
        }
    }

    private static void CollectFuncContexts(IParseTree node, List<PilotParser.Func_defContext> list)
    {
        if (node is PilotParser.Func_defContext fn)
            list.Add(fn);
        for (int i = 0; i < node.ChildCount; i++)
            CollectFuncContexts(node.GetChild(i), list);
    }

    private void CollectBodyErrors(IParseTree node, List<string> errors)
    {
        if (node is PilotParser.Atom_exprContext atomExpr)
        {
            string? atomId = atomExpr.atom()?.identifier()?.GetText();
            var trailers = atomExpr.trailer();
            if (atomId == "fail" && trailers != null && trailers.Length > 0 && trailers[0].LPAR() != null)
            {
                var argTest = trailers[0].arglist()?.argument().FirstOrDefault()?.test();
                if (argTest != null)
                {
                    string errType = InferErrorConstructorType(argTest.GetText().Trim());
                    if (!string.IsNullOrEmpty(errType))
                        errors.Add(errType);
                }
            }
            if (trailers != null && trailers.Any(t => t.QUESTION() != null) && atomId != null &&
                _functions.TryGetValue(atomId, out var callee) && callee.IsFallible)
            {
                errors.AddRange(callee.ErrorTypes);
            }
        }

        for (int i = 0; i < node.ChildCount; i++)
            CollectBodyErrors(node.GetChild(i), errors);
    }

    private string InferErrorConstructorType(string exprText)
    {
        if (string.IsNullOrWhiteSpace(exprText)) return "any";
        string t = exprText.Trim();
        if (t.StartsWith("\"", StringComparison.Ordinal) || t.StartsWith("f\"", StringComparison.Ordinal) || t.StartsWith("'", StringComparison.Ordinal))
            return "str";
        if (long.TryParse(t, out _))
            return "int";

        int paren = t.IndexOf('(');
        string head = (paren > 0 ? t[..paren] : t).Trim();
        if (head.Contains('.'))
        {
            string prefix = head[..head.IndexOf('.')].Trim();
            return prefix;
        }
        if (_variantToAdt.TryGetValue(head, out string? parentAdt))
            return head; // Preserve exact variant name for fine-grained union inference (`FileMissing | BadPort`), which also matches `parentAdt`!
        if (_records.Contains(head) || _adtVariants.ContainsKey(head))
            return head;
        if (_currentScope.ResolveVariable(head) is { } varType && varType != "any")
            return varType;

        return char.IsUpper(head[0]) ? head : "any";
    }

    #endregion

    #region Pass 2: AST Walk & Error Rule Verification

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
            string last = "void";
            foreach (var s in context.stmt())
                last = Visit(s);
            return last;
        }
        return "void";
    }

    public override string VisitFunc_def(PilotParser.Func_defContext context)
    {
        string name = context.identifier().GetText();
        _functions.TryGetValue(name, out var fnInfo);

        PushScope();
        _currentScope.EnclosingFunction = fnInfo;
        if (fnInfo?.AllowExceptions == true)
            _currentScope.InForeignBoundary = true;

        try
        {
            if (context.parameters()?.parameter() != null)
            {
                foreach (var p in context.parameters().parameter())
                {
                    string pName = p.identifier()?.GetText() ?? "_";
                    string pType = p.type_annotation()?.type_spec()?.GetText().Trim() ?? "any";
                    _currentScope.VariableTypes[pName] = pType;
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

    public override string VisitIsolate_stmt(PilotParser.Isolate_stmtContext context)
    {
        PushScope();
        _currentScope.InIsolate = true;
        try
        {
            return base.VisitIsolate_stmt(context);
        }
        finally
        {
            PopScope();
        }
    }

    public override string VisitInline_foreign_block(PilotParser.Inline_foreign_blockContext context)
    {
        PushScope();
        _currentScope.InForeignBoundary = true;
        try
        {
            if (context.stmt() != null)
            {
                foreach (var s in context.stmt())
                    Visit(s);
            }
        }
        finally
        {
            PopScope();
        }
        return "any";
    }

    public override string VisitTry_stmt(PilotParser.Try_stmtContext context)
    {
        // Check if the try block directly wraps an inline foreign block (`python { ... }`, `csharp { ... }`, etc.)
        bool wrapsForeign = ContainsForeignConstruct(context.suite(0));
        if (!_currentScope.InIsolate && !_currentScope.InForeignBoundary && !wrapsForeign)
        {
            Report(
                ErrorStratificationLintCode,
                "Error stratification lint: legacy 'try/except' is discouraged in native Pilot code. Use values-first 'T ! E' return errors with '?' / 'catch' for recoverable errors, 'panic(...)' for invariant bugs, or scope 'try/except' to FFI/isolate boundaries.",
                context.Start.Line,
                context.Start.Column,
                isError: false);
        }

        PushScope();
        _currentScope.InForeignBoundary = true;
        try
        {
            return base.VisitTry_stmt(context);
        }
        finally
        {
            PopScope();
        }
    }

    private static bool ContainsForeignConstruct(IParseTree? node)
    {
        if (node == null) return false;
        if (node is PilotParser.Inline_foreign_blockContext or PilotParser.Extern_defContext or PilotParser.Foreign_import_stmtContext)
            return true;
        for (int i = 0; i < node.ChildCount; i++)
        {
            if (ContainsForeignConstruct(node.GetChild(i)))
                return true;
        }
        return false;
    }

    public override string VisitLet_stmt(PilotParser.Let_stmtContext context)
    {
        HandleVarDecl(
            context.binding_pattern()?.identifier()?.GetText(),
            context.type_annotation()?.type_spec()?.GetText().Trim(),
            context.test());
        return "void";
    }

    public override string VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        HandleVarDecl(
            context.binding_pattern()?.identifier()?.GetText(),
            context.type_annotation()?.type_spec()?.GetText().Trim(),
            context.test());
        return "void";
    }

    public override string VisitConst_stmt(PilotParser.Const_stmtContext context)
    {
        HandleVarDecl(
            context.binding_pattern()?.identifier()?.GetText(),
            context.type_annotation()?.type_spec()?.GetText().Trim(),
            context.test());
        return "void";
    }

    private void HandleVarDecl(string? name, string? explicitType, PilotParser.TestContext? rhs)
    {
        string rhsType = rhs != null ? Visit(rhs) : "any";
        if (!string.IsNullOrEmpty(name))
        {
            _currentScope.VariableTypes[name!] = !string.IsNullOrWhiteSpace(explicitType) ? explicitType! : rhsType;
        }
    }

    public override string VisitGuard_stmt(PilotParser.Guard_stmtContext context)
    {
        var condTest = context.test();
        string condType = "any";
        string errVarName = context.identifier()?.GetText() ?? "err";

        // Handle grammar disambiguation where trailing `catch err { ... }` attaches to `coalesce_expr`
        if (condTest != null &&
            context.ELSE() == null &&
            context.CATCH() == null &&
            TryExtractGuardCatch(condTest, out string? innerExprText, out string? inlineCatchId, out var inlineCatchBlock))
        {
            condType = EvaluateSimpleExpressionType(innerExprText!);
            string errType = ExtractErrorType(condType);
            if (inlineCatchBlock != null)
            {
                PushScope();
                if (!string.IsNullOrEmpty(inlineCatchId))
                    _currentScope.VariableTypes[inlineCatchId!] = errType;
                Visit(inlineCatchBlock);
                PopScope();
            }
        }
        else if (condTest != null)
        {
            condType = Visit(condTest);
        }

        if (context.suite() != null)
        {
            PushScope();
            if (context.CATCH() != null && context.identifier() != null)
            {
                _currentScope.VariableTypes[errVarName] = ExtractErrorType(condType);
            }
            Visit(context.suite());
            PopScope();
        }

        if (context.pattern() != null)
        {
            string unwrapped = UnwrapOkType(condType);
            string patName = context.pattern().GetText().Trim();
            if (IsIdentifier(patName))
            {
                _currentScope.VariableTypes[patName] = unwrapped;
            }
        }

        return "void";
    }

    private static bool TryExtractGuardCatch(
        PilotParser.TestContext testCtx,
        out string? lhsText,
        out string? catchId,
        out PilotParser.BlockContext? catchBlock)
    {
        lhsText = null;
        catchId = null;
        catchBlock = null;

        var coal = testCtx.pipe_expr(0)?.set_query_expr(0)?.coalesce_expr(0);
        var catches = coal?.catch_clause();
        if (coal == null || catches == null || catches.Length == 0)
            return false;

        var lastCatch = catches[^1];
        if (lastCatch.FAT_ARROW() != null)
            return false;

        if (lastCatch.block() != null)
        {
            lhsText = coal.or_test(0).GetText().Trim();
            catchId = null;
            catchBlock = lastCatch.block();
            return true;
        }

        // Or `catch err { ... }` parsed as `or_test` with trailing block or suite
        return false;
    }

    public override string VisitExpr_stmt(PilotParser.Expr_stmtContext context)
    {
        bool hasEqual = context.EQUAL() != null && context.EQUAL().Length > 0;
        bool hasAug = context.augassign() != null && context.augassign().Length > 0;

        if (!hasEqual && !hasAug)
        {
            var tests = context.testlist()?.test();
            if (tests != null)
            {
                foreach (var t in tests)
                {
                    string exprType = Visit(t) ?? "any";
                    // Milestone 13.3 ERR001: Must-Use enforcement on statement-level `T ! E` expressions!
                    if (PilotAdtMemoryLayout.TrySplitFallibleTypeSpec(exprType, out _, out _))
                    {
                        Report(
                            UnhandledErrorReturnCode,
                            $"Must-use error return: expression '{t.GetText()}' returns fallible type '{exprType}' and cannot be ignored. Handle the error with '?', 'catch', 'guard ... catch', 'match', assignment, or explicit 'ignore_err(...)'.",
                            t.Start.Line,
                            t.Start.Column,
                            isError: true);
                    }
                }
            }
            return "void";
        }

        if (context.testlist() != null)
            Visit(context.testlist());
        if (context.test() != null)
        {
            foreach (var r in context.test())
                Visit(r);
        }
        return "void";
    }

    public override string VisitFlow_stmt(PilotParser.Flow_stmtContext context)
    {
        if (context.RAISE() != null)
        {
            if (!_currentScope.InIsolate && !_currentScope.InForeignBoundary)
            {
                Report(
                    ErrorStratificationLintCode,
                    "Error stratification lint: 'raise' is discouraged in native Pilot code. Return 'fail(err)' from a 'T ! E' function for recoverable errors or call 'panic(...)' for unrecoverable bugs.",
                    context.Start.Line,
                    context.Start.Column,
                    isError: false);
            }
        }

        if (context.RETURN() != null && context.testlist()?.test() is { Length: > 0 } retTests)
        {
            var retExpr = retTests[0];
            string retType = Visit(retExpr) ?? "any";
            var fn = _currentScope.EnclosingFunction;

            if (fn != null)
            {
                bool isFailCall = retType.StartsWith("fail:", StringComparison.Ordinal);
                if (isFailCall)
                {
                    string failErrType = retType["fail:".Length..].Trim();
                    if (!fn.IsFallible)
                    {
                        Report(
                            InvalidFailReturnCode,
                            $"Cannot return '{retExpr.GetText()}' from non-fallible function '{fn.Name}' (declared return type '{fn.RawReturnType}'). Change return type to '{fn.RawReturnType} ! {failErrType}' or '{fn.RawReturnType}!'.",
                            retExpr.Start.Line,
                            retExpr.Start.Column,
                            isError: true);
                    }
                    else if (!fn.HasInferredErrorSet && fn.ErrorTypes.Count > 0 && !IsErrorCompatibleOrAdaptable(failErrType, fn.ErrorTypes))
                    {
                        Report(
                            InvalidFailReturnCode,
                            $"Error value '{retExpr.GetText()}' of type '{failErrType}' is not compatible with declared error set '{string.Join(" | ", fn.ErrorTypes)}' of function '{fn.Name}'.",
                            retExpr.Start.Line,
                            retExpr.Start.Column,
                            isError: true);
                    }
                }
                else if (fn.IsFallible && !PilotAdtMemoryLayout.TrySplitFallibleTypeSpec(retType, out _, out _))
                {
                    // Check success value compatibility with `fn.OkType`
                    if (!AreTypesBroadlyCompatible(fn.OkType, retType))
                    {
                        Report(
                            InvalidFailReturnCode,
                            $"Cannot return value of type '{retType}' as success value of function '{fn.Name}' returning '{fn.EffectiveReturnType}' (expected '{fn.OkType}').",
                            retExpr.Start.Line,
                            retExpr.Start.Column,
                            isError: true);
                    }
                }
            }
            return "void";
        }

        return base.VisitFlow_stmt(context);
    }

    public override string VisitCoalesce_expr(PilotParser.Coalesce_exprContext context)
    {
        var orTests = context.or_test();
        if (orTests == null || orTests.Length == 0) return "any";

        string currentType = Visit(orTests[0]) ?? "any";
        var catches = context.catch_clause();

        if (catches != null && catches.Length > 0)
        {
            string okType = UnwrapOkType(currentType);
            string errSpec = ExtractErrorType(currentType);
            var activeErrors = ExpandErrorDomain(errSpec);

            foreach (var cc in catches)
            {
                PushScope();
                string? errId = cc.identifier()?.GetText();
                if (!string.IsNullOrEmpty(errId))
                {
                    _currentScope.VariableTypes[errId!] = errSpec;
                }

                // Check `catch err => match err { ... }` for ERR003 exhaustiveness and partial `catch` narrowing!
                var matchCtx = TryFindCatchMatch(cc);
                if (matchCtx != null && activeErrors.Count > 0)
                {
                    AnalyzeCatchMatch(matchCtx, orTests[0].GetText(), errSpec, activeErrors, out var remainingAfterCatch);
                    activeErrors = remainingAfterCatch;
                }

                Visit(cc);
                PopScope();

                // If the catch was a total fallback (`catch 8080` or exhaustive `match`), activeErrors becomes empty
                if (matchCtx == null)
                {
                    activeErrors.Clear();
                }
            }

            if (activeErrors.Count == 0)
            {
                return okType;
            }

            // Partial catch narrowing: returns `OkType ! RemainingErrorUnion`!
            return $"{okType} ! {string.Join(" | ", activeErrors)}";
        }

        for (int i = 1; i < orTests.Length; i++)
        {
            Visit(orTests[i]);
            if (currentType.EndsWith("?", StringComparison.Ordinal))
                currentType = currentType[..^1].Trim();
        }

        return currentType;
    }

    private static PilotParser.Match_stmtContext? TryFindCatchMatch(PilotParser.Catch_clauseContext cc)
    {
        if (cc.test() != null)
        {
            return FindFirstMatchStmt(cc.test());
        }
        if (cc.block() != null)
        {
            return FindFirstMatchStmt(cc.block());
        }
        return null;
    }

    private static PilotParser.Match_stmtContext? FindFirstMatchStmt(IParseTree node)
    {
        if (node is PilotParser.Match_stmtContext m) return m;
        for (int i = 0; i < node.ChildCount; i++)
        {
            var found = FindFirstMatchStmt(node.GetChild(i));
            if (found != null) return found;
        }
        return null;
    }

    private void AnalyzeCatchMatch(
        PilotParser.Match_stmtContext matchCtx,
        string sourceExprText,
        string declaredErrorSpec,
        List<string> inputErrorVariants,
        out List<string> remainingUnhandledErrors)
    {
        remainingUnhandledErrors = new List<string>();
        var arms = matchCtx.match_suite()?.match_arm();
        if (arms == null || arms.Length == 0)
        {
            Report(
                NonExhaustiveCatchCode,
                $"Non-exhaustive 'catch' error recovery on '{sourceExprText}' (error set '{declaredErrorSpec}'): missing cases for {string.Join(", ", inputErrorVariants)}.",
                matchCtx.Start.Line,
                matchCtx.Start.Column,
                isError: true);
            return;
        }

        var handled = new HashSet<string>(StringComparer.Ordinal);
        bool hasCatchAllRecovery = false;
        bool hasCatchAllRepropagation = false;

        foreach (var arm in arms)
        {
            bool armRepropagates = ArmBodyProducesFailure(arm.suite());
            if (arm.pattern() == null)
            {
                if (armRepropagates) hasCatchAllRepropagation = true;
                else hasCatchAllRecovery = true;
                continue;
            }

            var pat = arm.pattern();
            if (pat == null) continue;

            var matchedNames = new List<string>();
            bool isWildcard = ExtractMatchedErrorNames(pat, matchedNames);
            if (isWildcard)
            {
                if (armRepropagates) hasCatchAllRepropagation = true;
                else hasCatchAllRecovery = true;
            }
            else
            {
                foreach (var name in matchedNames)
                {
                    if (_adtVariants.TryGetValue(name, out var childVariants))
                    {
                        foreach (var cv in childVariants)
                        {
                            if (!armRepropagates) handled.Add(cv);
                        }
                        if (!armRepropagates) handled.Add(name);
                    }
                    else if (!armRepropagates)
                    {
                        handled.Add(name);
                    }
                }
            }
        }

        if (hasCatchAllRecovery)
        {
            return;
        }

        var unhandled = inputErrorVariants.Where(v => !handled.Contains(v) &&
            (!_variantToAdt.TryGetValue(v, out var parentAdt) || !handled.Contains(parentAdt))).ToList();

        if (hasCatchAllRepropagation)
        {
            // Partial `catch` narrowing! Handled variants are eliminated; unhandled variants remain in the narrowed error union.
            remainingUnhandledErrors.AddRange(unhandled);
            return;
        }

        if (unhandled.Count > 0)
        {
            Report(
                NonExhaustiveCatchCode,
                $"Non-exhaustive 'catch' error recovery on '{sourceExprText}' (error set '{declaredErrorSpec}'): unhandled error variant(s): {string.Join(", ", unhandled)}.",
                matchCtx.Start.Line,
                matchCtx.Start.Column,
                isError: true);
        }
    }

    private static bool ArmBodyProducesFailure(PilotParser.SuiteContext? body)
    {
        if (body == null) return false;
        string txt = body.GetText().Trim();
        return txt.Contains("fail(", StringComparison.Ordinal);
    }

    private bool ExtractMatchedErrorNames(PilotParser.PatternContext pat, List<string> names)
    {
        var orBranches = pat.or_pattern()?.and_pattern();
        if (orBranches == null) return true;

        foreach (var andBr in orBranches)
        {
            var notParts = andBr.not_pattern();
            if (notParts == null || notParts.Length == 0) continue;
            var prim = notParts[0].primary_pattern();
            if (prim == null || prim.UNDERSCORE() != null)
                return true;

            var tv = prim.type_or_var_pattern();
            if (tv == null) continue;
            if ((tv.LET() != null || tv.VAR() != null) && tv.dotted_name() == null)
                return true;

            string rawName = tv.dotted_name()?.GetText() ?? "";
            if (rawName.Contains('.'))
                rawName = rawName[(rawName.LastIndexOf('.') + 1)..];

            bool isKnownVariantOrType =
                _variantToAdt.ContainsKey(rawName) ||
                _adtVariants.ContainsKey(rawName) ||
                _records.Contains(rawName) ||
                tv.LPAR() != null ||
                tv.property_pattern() != null ||
                (rawName.Length > 0 && char.IsUpper(rawName[0]));

            if (!isKnownVariantOrType)
            {
                return true; // Bare lowercase binding `case other => ...` is a wildcard
            }

            names.Add(rawName);
        }

        return false;
    }

    public override string VisitAtom_expr(PilotParser.Atom_exprContext context)
    {
        var atom = context.atom();
        string currentType = Visit(atom) ?? "any";
        string? atomId = atom.identifier()?.GetText();
        var trailers = context.trailer();

        if (trailers == null || trailers.Length == 0)
            return currentType;

        string exprPrefix = atom.GetText();
        for (int i = 0; i < trailers.Length; i++)
        {
            var tr = trailers[i];
            if (tr.LPAR() != null)
            {
                if (tr.arglist()?.argument() != null)
                {
                    foreach (var arg in tr.arglist().argument())
                    {
                        if (arg.test() != null) Visit(arg.test());
                    }
                }

                if (i == 0 && atomId != null)
                {
                    if (atomId == "fail")
                    {
                        var firstArg = tr.arglist()?.argument().FirstOrDefault()?.test();
                        string errT = firstArg != null ? InferErrorConstructorType(firstArg.GetText().Trim()) : "any";
                        currentType = $"fail:{errT}";
                        exprPrefix += $"({tr.arglist()?.GetText() ?? ""})";
                        continue;
                    }
                    if (atomId == "ok")
                    {
                        var firstArg = tr.arglist()?.argument().FirstOrDefault()?.test();
                        string okT = firstArg != null ? (Visit(firstArg) ?? "any") : "any";
                        currentType = okT;
                        exprPrefix += $"({tr.arglist()?.GetText() ?? ""})";
                        continue;
                    }
                    if (atomId == "ignore_err")
                    {
                        currentType = "void";
                        exprPrefix += $"({tr.arglist()?.GetText() ?? ""})";
                        continue;
                    }
                    if (_functions.TryGetValue(atomId, out var fnInfo))
                    {
                        currentType = fnInfo.EffectiveReturnType;
                        exprPrefix += $"({tr.arglist()?.GetText() ?? ""})";
                        continue;
                    }
                }

                exprPrefix += $"({tr.arglist()?.GetText() ?? ""})";
            }
            else if (tr.DOT() != null && tr.identifier() != null)
            {
                string member = tr.identifier().GetText();
                bool nextIsCall = i + 1 < trailers.Length && trailers[i + 1].LPAR() != null;
                if (member is "or_panic" or "OrPanic" or "unwrap_or" or "UnwrapOr" or "ok_value" or "OkValue")
                {
                    currentType = UnwrapOkType(currentType);
                    if (nextIsCall)
                    {
                        Visit(trailers[i + 1]);
                        i++;
                    }
                    continue;
                }
                if (member is "is_ok" or "IsOk" or "is_err" or "IsErr" or "is_fail" or "IsFail")
                {
                    currentType = "bool";
                    continue;
                }
                if (nextIsCall && member is "parse_int" or "ParseInt")
                {
                    currentType = "int!ParseError";
                    Visit(trailers[i + 1]);
                    exprPrefix += $".{member}()";
                    i++;
                    continue;
                }
                if (nextIsCall && member is "parse_float" or "ParseFloat")
                {
                    currentType = "float!ParseError";
                    Visit(trailers[i + 1]);
                    exprPrefix += $".{member}()";
                    i++;
                    continue;
                }
                if (nextIsCall && member is "to_int" or "ToInt")
                {
                    currentType = "int?";
                    Visit(trailers[i + 1]);
                    exprPrefix += $".{member}()";
                    i++;
                    continue;
                }
                if (nextIsCall && member is "to_float" or "ToFloat")
                {
                    currentType = "float?";
                    Visit(trailers[i + 1]);
                    exprPrefix += $".{member}()";
                    i++;
                    continue;
                }
                exprPrefix += $".{member}";
            }
            else if (tr.QUESTION() != null && tr.LSQB() == null && tr.LPAR() == null)
            {
                var qTok = tr.QUESTION().Symbol;
                VerifyQuestionPropagation(exprPrefix, currentType, qTok);
                currentType = UnwrapOkType(currentType);
                if (currentType.EndsWith("?", StringComparison.Ordinal) && currentType.Length > 1)
                    currentType = currentType[..^1].Trim();
                exprPrefix += "?";
            }
            else
            {
                Visit(tr);
            }
        }

        return currentType;
    }

    public override string VisitAtom(PilotParser.AtomContext context)
    {
        if (context.literal() is { } lit)
        {
            if (lit.INTEGER() != null || lit.HEX_INTEGER() != null || lit.BIN_INTEGER() != null || lit.OCT_INTEGER() != null) return "int";
            if (lit.FLOAT_NUMBER() != null) return "float";
            if (lit.STRING().Length > 0 || lit.FSTRING().Length > 0 || lit.TRIPLE_STRING().Length > 0) return "str";
            if (lit.TRUE() != null || lit.FALSE() != null) return "bool";
            if (lit.NULL() != null || lit.NONE() != null) return "null";
        }

        if (context.identifier() != null)
        {
            string id = context.identifier().GetText();
            if (_currentScope.ResolveVariable(id) is { } varT)
                return varT;
        }

        if (context.LPAR() != null && context.testlist_comp()?.test() is { Length: 1 } singleParen)
        {
            return Visit(singleParen[0]) ?? "any";
        }

        return base.VisitAtom(context) ?? "any";
    }

    private void VerifyQuestionPropagation(string exprText, string exprType, IToken questionToken)
    {
        var fn = _currentScope.EnclosingFunction;
        if (PilotAdtMemoryLayout.TrySplitFallibleTypeSpec(exprType, out _, out string errSpec))
        {
            if (fn == null)
                return; // Top-level script propagation is allowed

            if (!fn.IsFallible)
            {
                Report(
                    IncompatibleErrorPropagationCode,
                    $"Cannot propagate error with '?' on '{exprText}' (type '{exprType}') inside non-fallible function '{fn.Name}' (return type '{fn.RawReturnType}').",
                    questionToken.Line,
                    questionToken.Column,
                    isError: true);
                return;
            }

            if (!fn.HasInferredErrorSet && fn.ErrorTypes.Count > 0 && !string.IsNullOrEmpty(errSpec))
            {
                var sourceVariants = ExpandErrorDomain(errSpec);
                foreach (var srcErr in sourceVariants)
                {
                    if (!IsErrorCompatibleOrAdaptable(srcErr, fn.ErrorTypes))
                    {
                        Report(
                            IncompatibleErrorPropagationCode,
                            $"Cannot propagate error type '{srcErr}' from '{exprText}?' into function '{fn.Name}' returning '{fn.EffectiveReturnType}': no matching error variant or 'adapter {srcErr} -> {string.Join(" | ", fn.ErrorTypes)}' found.",
                            questionToken.Line,
                            questionToken.Column,
                            isError: true);
                        break;
                    }
                }
            }
            return;
        }

        if (exprType.EndsWith("?", StringComparison.Ordinal))
        {
            if (fn != null && !fn.OkType.EndsWith("?", StringComparison.Ordinal) && fn.RawReturnType != "any")
            {
                Report(
                    IncompatibleErrorPropagationCode,
                    $"Cannot propagate optional 'none' with '?' on '{exprText}' (type '{exprType}') inside function '{fn.Name}' with non-optional return type '{fn.RawReturnType}'.",
                    questionToken.Line,
                    questionToken.Column,
                    isError: true);
            }
        }
    }

    private bool IsErrorCompatibleOrAdaptable(string sourceError, IReadOnlyList<string> targetErrors)
    {
        if (string.IsNullOrEmpty(sourceError) || sourceError == "any") return true;

        foreach (var target in targetErrors)
        {
            if (target == "any" || string.Equals(sourceError, target, StringComparison.Ordinal))
                return true;

            // Source is a variant of Target ADT (`FileMissing` in `ConfigError`)
            if (_variantToAdt.TryGetValue(sourceError, out string? parentAdt) &&
                string.Equals(parentAdt, target, StringComparison.Ordinal))
            {
                return true;
            }

            // Target is a variant of Source ADT? No, only if Target is an ADT containing Source, or declared adapter exists.
            // Check declared `adapter Source -> Target` or `adapter SourceParentAdt -> Target` or `adapter Source -> TargetVariant`
            foreach (var (fromT, toT) in _adapters)
            {
                bool fromMatches = string.Equals(fromT, sourceError, StringComparison.Ordinal) ||
                    (_variantToAdt.TryGetValue(sourceError, out var srcParent) && string.Equals(fromT, srcParent, StringComparison.Ordinal));
                bool toMatches = string.Equals(toT, target, StringComparison.Ordinal) ||
                    (_variantToAdt.TryGetValue(toT, out var dstParent) && string.Equals(dstParent, target, StringComparison.Ordinal));
                if (fromMatches && toMatches)
                    return true;
            }
        }

        return false;
    }

    private List<string> ExpandErrorDomain(string errSpec)
    {
        var result = new List<string>();
        if (string.IsNullOrWhiteSpace(errSpec) || errSpec == "any") return result;

        foreach (var part in SplitUnionTypes(errSpec))
        {
            if (_adtVariants.TryGetValue(part, out var variants) && variants.Count > 0)
            {
                foreach (var v in variants)
                {
                    if (!result.Contains(v, StringComparer.Ordinal))
                        result.Add(v);
                }
            }
            else if (!result.Contains(part, StringComparer.Ordinal))
            {
                result.Add(part);
            }
        }
        return result;
    }

    private string EvaluateSimpleExpressionType(string exprText)
    {
        int paren = exprText.IndexOf('(');
        if (paren > 0 && exprText.EndsWith(")", StringComparison.Ordinal))
        {
            string fnName = exprText[..paren].Trim();
            if (_functions.TryGetValue(fnName, out var fnInfo))
                return fnInfo.EffectiveReturnType;
        }
        if (_currentScope.ResolveVariable(exprText) is { } varT)
            return varT;
        return "any";
    }

    private static string UnwrapOkType(string typeSpec) =>
        PilotAdtMemoryLayout.TrySplitFallibleTypeSpec(typeSpec, out string okPart, out _) ? okPart : typeSpec;

    private static string ExtractErrorType(string typeSpec) =>
        PilotAdtMemoryLayout.TrySplitFallibleTypeSpec(typeSpec, out _, out string errPart) && !string.IsNullOrEmpty(errPart)
            ? errPart
            : "any";

    private static List<string> SplitUnionTypes(string spec) =>
        spec.Split('|', StringSplitOptions.RemoveEmptyEntries).Select(p => p.Trim()).Where(p => p.Length > 0).ToList();

    private static bool AreTypesBroadlyCompatible(string expectedOk, string actual)
    {
        if (string.IsNullOrEmpty(expectedOk) || string.IsNullOrEmpty(actual)) return true;
        if (expectedOk is "any" or "dynamic" or "unknown" || actual is "any" or "dynamic" or "unknown") return true;
        if (string.Equals(expectedOk, actual, StringComparison.OrdinalIgnoreCase)) return true;
        if (expectedOk.EndsWith("?", StringComparison.Ordinal))
        {
            if (actual is "null" or "none") return true;
            return AreTypesBroadlyCompatible(expectedOk[..^1].Trim(), actual);
        }
        if (IsNumeric(expectedOk) && IsNumeric(actual)) return true;
        return false;
    }

    private static bool IsNumeric(string t) =>
        t is "int" or "i8" or "i16" or "i32" or "i64" or "u8" or "u16" or "u32" or "u64" or "uint"
            or "float" or "f32" or "f64" or "double";

    private static bool IsIdentifier(string s) =>
        !string.IsNullOrEmpty(s) && s.All(c => char.IsLetterOrDigit(c) || c == '_');

    private void Report(string code, string message, int line, int column, bool isError)
    {
        if (_diagnostics.Any(d => d.Code == code && d.Line == line))
            return;
        _diagnostics.Add(new PilotAnalysisDiagnostic(code, message, line, column, isError));
    }

    protected override string DefaultResult => "any";
    protected override string AggregateResult(string aggregate, string nextResult) =>
        !string.IsNullOrEmpty(nextResult) && nextResult != "any" ? nextResult : aggregate;

    #endregion
}
