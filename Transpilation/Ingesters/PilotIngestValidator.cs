using System;
using System.Collections.Generic;
using System.Linq;
using Antlr4.Runtime;
using Pilot.Diagnostics;
using Pilot.Formatting;
using Pilot.Preprocessing;
using Pilot.Transpilation.Ir;
using Pilot.Transpilation.Licensing;

namespace Pilot.Transpilation.Ingesters;

/// <summary>
/// Milestone 20.4: Mandatory Post-Ingest Parse Gate (<c>INGEST001</c>–<c>INGEST010</c>)
/// and <c>--strict-pilot</c> semantic idiom upgrader (M12 Null-Safety, M13 Values-First Errors, M15 ForExpr, M19 Mutability).
/// </summary>
public static class PilotIngestValidator
{
    /// <summary>
    /// Applies <c>--strict-pilot</c> upgrades (when enabled), formats the <see cref="PilotIrModule"/>
    /// via <see cref="PilotFormatter"/>, and validates the resulting <c>.pilot</c> code through
    /// <see cref="PilotLexer"/> and <see cref="PilotParser"/>.
    /// </summary>
    public static TranspilationResult ValidateAndFinalize(
        PilotIrModule module,
        TranspilationOptions options,
        string sourceLang,
        PilotLicenseCheckResult? licenseInfo = null)
    {
        UpgradeToStrictPilotIdioms(module, options.StrictPilot);

        var fmtOptions = new PilotFormatterOptions
        {
            IndentString = options.IndentString,
            UseColonSuites = options.PreferColonSuites ? true : false,
            EmitSemicolons = !options.PreferColonSuites,
            PreserveComments = true,
            PreserveBlankLines = true
        };

        string pilotCode = PilotFormatter.FormatModule(module, fmtOptions);

        // Mandatory Post-Ingest Parse Gate (INGEST001–INGEST010)
        var parseErrors = VerifyGeneratedPilotSyntax(pilotCode);
        if (parseErrors.Count > 0)
        {
            return TranspilationResult.CreateFailure(parseErrors, "pilot", licenseInfo);
        }

        return TranspilationResult.CreateSuccess(pilotCode, "pilot", licenseInfo);
    }

    /// <summary>
    /// Validates raw generated <c>.pilot</c> code through <see cref="PilotPreprocessor"/>,
    /// <see cref="PilotLexer"/>, and <see cref="PilotParser"/>.
    /// </summary>
    public static List<string> VerifyGeneratedPilotSyntax(string pilotCode)
    {
        var diagnostics = new List<string>();
        var preproc = PilotPreprocessor.Preprocess(pilotCode, "<ingested.pilot>");
        if (preproc.HasErrors)
        {
            foreach (var d in preproc.Diagnostics.Where(x => x.Severity == PreprocessorDiagnosticSeverity.Error))
            {
                diagnostics.Add($"INGEST001: Preprocessor error in generated Pilot code (line {d.Line}): {d.Message}");
            }
            return diagnostics;
        }

        var inputStream = new AntlrInputStream(preproc.PreprocessedSource);
        var lexer = new PilotLexer(inputStream);
        var errorTracker = new ErrorTracker();
        lexer.RemoveErrorListeners();

        var tokenStream = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokenStream);
        parser.RemoveErrorListeners();
        parser.AddErrorListener(errorTracker);

        _ = parser.ParseProgramFast();
        if (errorTracker.HasErrors)
        {
            foreach (var err in errorTracker.Errors)
            {
                diagnostics.Add($"INGEST002: Post-ingest Pilot syntax gate failed: {err}");
            }
        }

        return diagnostics;
    }

    /// <summary>
    /// Upgrades a foreign-ingested <see cref="PilotIrModule"/> to idiomatic Pilot:
    /// 1. Mutability demotion (<c>let</c> vs <c>var</c> based on actual reassignment analysis, M19).
    /// 2. When <paramref name="strictPilot"/> is true:
    ///    - Lifts <c>raise</c>/<c>throw</c> into <c>return fail(...)</c> and annotates return type with <c>! Error</c> (M13).
    ///    - Rewrites early-return <c>if x == null</c> / <c>if x == none</c> checks into <c>guard some x else { ... }</c> (M12).
    ///    - Rewrites imperative list-append loops into <c>[expr for x in iter]</c> <see cref="PilotIrForExpr"/> (M15).
    /// </summary>
    public static void UpgradeToStrictPilotIdioms(PilotIrModule module, bool strictPilot)
    {
        var topMutated = CollectMutatedVariableNames(module.Statements);
        UpgradeStatementList(module.Statements, topMutated, strictPilot, enclosingFunc: null);
    }

    private static void UpgradeStatementList(
        List<PilotIrStmt> stmts,
        HashSet<string> mutatedNames,
        bool strictPilot,
        PilotIrFuncDefStmt? enclosingFunc)
    {
        for (int i = 0; i < stmts.Count; i++)
        {
            var stmt = stmts[i];

            // 1. Mutability analysis: let vs var
            if (stmt is PilotIrVarDeclStmt vd && vd.Kind != PilotVarDeclKind.Const)
            {
                bool isMut = mutatedNames.Contains(vd.Name);
                vd.IsMutated = isMut;
                vd.Kind = isMut ? PilotVarDeclKind.Var : PilotVarDeclKind.Let;
            }

            // 2. Function-level recursive upgrade
            if (stmt is PilotIrFuncDefStmt fn)
            {
                var fnMutated = CollectMutatedVariableNames(fn.Body);
                UpgradeStatementList(fn.Body, fnMutated, strictPilot, fn);

                if (strictPilot && ContainsFailReturn(fn.Body))
                {
                    string baseRet = !string.IsNullOrWhiteSpace(fn.DeclaredReturnType) ? fn.DeclaredReturnType! : "any";
                    if (!baseRet.Contains('!'))
                    {
                        fn.DeclaredReturnType = $"{baseRet} ! Error";
                    }
                }
            }
            else if (stmt is PilotIrClassDefStmt cls)
            {
                foreach (var m in cls.Methods)
                {
                    var mMut = CollectMutatedVariableNames(m.Body);
                    UpgradeStatementList(m.Body, mMut, strictPilot, m);
                    if (strictPilot && ContainsFailReturn(m.Body))
                    {
                        string baseRet = !string.IsNullOrWhiteSpace(m.DeclaredReturnType) ? m.DeclaredReturnType! : "any";
                        if (!baseRet.Contains('!'))
                            m.DeclaredReturnType = $"{baseRet} ! Error";
                    }
                }
            }
            else if (stmt is PilotIrExtensionDefStmt ext)
            {
                foreach (var m in ext.Methods)
                {
                    var mMut = CollectMutatedVariableNames(m.Body);
                    UpgradeStatementList(m.Body, mMut, strictPilot, m);
                }
            }
            else if (stmt is PilotIrIfStmt ifs)
            {
                // Strict-Pilot M12: if x == null / if x == none { return ... } -> guard some x else { return ... }
                if (strictPilot &&
                    ifs.ElifBranches.Count == 0 &&
                    ifs.ElseBody.Count == 0 &&
                    ifs.ThenBody.Count > 0 &&
                    ifs.ThenBody[^1] is PilotIrReturnStmt or PilotIrRaiseStmt &&
                    TryExtractNullCheckVariable(ifs.Condition, out string? checkedVar))
                {
                    UpgradeStatementList(ifs.ThenBody, mutatedNames, strictPilot, enclosingFunc);
                    if (enclosingFunc != null)
                    {
                        for (int pIdx = 0; pIdx < enclosingFunc.Parameters.Count; pIdx++)
                        {
                            var p = enclosingFunc.Parameters[pIdx];
                            if (p.Name == checkedVar && !string.IsNullOrEmpty(p.TypeAnnotation) && !p.TypeAnnotation.EndsWith("?"))
                            {
                                enclosingFunc.Parameters[pIdx] = p with { TypeAnnotation = p.TypeAnnotation + "?" };
                            }
                        }
                    }

                    var guardStmt = new PilotIrGuardStmt
                    {
                        Line = ifs.Line,
                        IsSomeBinding = true,
                        BoundVariable = checkedVar,
                        ConditionOrSource = new PilotIrIdentifierExpr { Name = checkedVar! }
                    };
                    guardStmt.ElseBody.AddRange(ifs.ThenBody);
                    stmts[i] = guardStmt;
                    continue;
                }

                UpgradeStatementList(ifs.ThenBody, mutatedNames, strictPilot, enclosingFunc);
                foreach (var (_, eb) in ifs.ElifBranches)
                    UpgradeStatementList(eb, mutatedNames, strictPilot, enclosingFunc);
                UpgradeStatementList(ifs.ElseBody, mutatedNames, strictPilot, enclosingFunc);
            }
            else if (stmt is PilotIrWhileStmt ws)
            {
                UpgradeStatementList(ws.Body, mutatedNames, strictPilot, enclosingFunc);
            }
            else if (stmt is PilotIrForStmt fs)
            {
                UpgradeStatementList(fs.Body, mutatedNames, strictPilot, enclosingFunc);
            }
            else if (stmt is PilotIrRaiseStmt rs && strictPilot && enclosingFunc != null)
            {
                // Strict-Pilot M13: lift raise ValueError("msg") -> return fail("msg")
                PilotIrExpr errPayload = UnwrapExceptionConstructor(rs.Value);
                var failCall = new PilotIrCallExpr
                {
                    Line = rs.Line,
                    Callee = new PilotIrIdentifierExpr { Name = "fail" },
                    LibraryCallKind = PilotLibraryCallKind.FailConstructor
                };
                failCall.Arguments.Add(new PilotIrCallArg(null, errPayload));
                stmts[i] = new PilotIrReturnStmt
                {
                    Line = rs.Line,
                    Value = failCall
                };
            }

            // Strict-Pilot M15: rewrite `let acc = []; for x in iter { acc.append(val); }` -> `let acc = [val for x in iter];`
            if (strictPilot &&
                i + 1 < stmts.Count &&
                stmts[i] is PilotIrVarDeclStmt emptyListDecl &&
                emptyListDecl.Initializer is PilotIrCollectionExpr { Kind: PilotCollectionKind.List, Elements.Count: 0 } &&
                stmts[i + 1] is PilotIrForStmt nextFor &&
                TryExtractAppendLoop(nextFor, emptyListDecl.Name, out var yieldExpr, out var filterExpr))
            {
                var forExpr = new PilotIrForExpr
                {
                    Line = nextFor.Line,
                    Iterable = nextFor.Iterable,
                    FilterCondition = filterExpr,
                    YieldExpr = yieldExpr!
                };
                forExpr.TargetVars.AddRange(nextFor.TargetVars);
                emptyListDecl.Initializer = forExpr;
                emptyListDecl.Kind = PilotVarDeclKind.Let;
                emptyListDecl.IsMutated = false;
                stmts.RemoveAt(i + 1);
            }
        }
    }

    private static bool TryExtractNullCheckVariable(PilotIrExpr cond, out string? varName)
    {
        varName = null;
        if (cond is PilotIrBinaryExpr bin && bin.Operator is "==" or "is")
        {
            if (bin.Left is PilotIrIdentifierExpr idL &&
                bin.Right is PilotIrLiteralExpr { Kind: PilotLiteralKind.Null or PilotLiteralKind.None })
            {
                varName = idL.Name;
                return true;
            }
            if (bin.Right is PilotIrIdentifierExpr idR &&
                bin.Left is PilotIrLiteralExpr { Kind: PilotLiteralKind.Null or PilotLiteralKind.None })
            {
                varName = idR.Name;
                return true;
            }
        }
        return false;
    }

    private static PilotIrExpr UnwrapExceptionConstructor(PilotIrExpr? expr)
    {
        if (expr == null)
            return new PilotIrLiteralExpr { Kind = PilotLiteralKind.String, RawValue = "\"Error\"", StringContent = "Error" };

        if (expr is PilotIrCallExpr call && call.Arguments.Count > 0)
        {
            return call.Arguments[0].Value;
        }
        return expr;
    }

    private static bool TryExtractAppendLoop(
        PilotIrForStmt forStmt,
        string listVarName,
        out PilotIrExpr? yieldExpr,
        out PilotIrExpr? filterExpr)
    {
        yieldExpr = null;
        filterExpr = null;

        if (forStmt.Body.Count != 1)
            return false;

        if (forStmt.Body[0] is PilotIrExprStmt es &&
            IsAppendOnVar(es.Expression, listVarName, out yieldExpr))
        {
            return true;
        }

        if (forStmt.Body[0] is PilotIrIfStmt ifs &&
            ifs.ElifBranches.Count == 0 &&
            ifs.ElseBody.Count == 0 &&
            ifs.ThenBody.Count == 1 &&
            ifs.ThenBody[0] is PilotIrExprStmt ies &&
            IsAppendOnVar(ies.Expression, listVarName, out yieldExpr))
        {
            filterExpr = ifs.Condition;
            return true;
        }

        return false;
    }

    private static bool IsAppendOnVar(PilotIrExpr expr, string listVarName, out PilotIrExpr? pushedVal)
    {
        pushedVal = null;
        if (expr is PilotIrCallExpr call &&
            call.Arguments.Count == 1 &&
            call.Callee is PilotIrMemberAccessExpr mem &&
            mem.Target is PilotIrIdentifierExpr id &&
            id.Name == listVarName &&
            mem.MemberName is "append" or "push" or "add" or "Add" or "push_back")
        {
            pushedVal = call.Arguments[0].Value;
            return true;
        }
        return false;
    }

    private static bool ContainsFailReturn(List<PilotIrStmt> stmts)
    {
        foreach (var s in stmts)
        {
            if (s is PilotIrReturnStmt ret &&
                ret.Value is PilotIrCallExpr call &&
                (call.LibraryCallKind == PilotLibraryCallKind.FailConstructor ||
                 (call.Callee is PilotIrIdentifierExpr id && id.Name == "fail")))
            {
                return true;
            }
            if (s is PilotIrIfStmt ifs)
            {
                if (ContainsFailReturn(ifs.ThenBody) ||
                    ifs.ElifBranches.Any(e => ContainsFailReturn(e.Body)) ||
                    ContainsFailReturn(ifs.ElseBody))
                    return true;
            }
            if (s is PilotIrGuardStmt gs && ContainsFailReturn(gs.ElseBody))
                return true;
            if (s is PilotIrWhileStmt ws && ContainsFailReturn(ws.Body))
                return true;
            if (s is PilotIrForStmt fs && ContainsFailReturn(fs.Body))
                return true;
        }
        return false;
    }

    public static HashSet<string> CollectMutatedVariableNames(IEnumerable<PilotIrStmt> stmts)
    {
        var mutated = new HashSet<string>(StringComparer.Ordinal);
        void Walk(IEnumerable<PilotIrStmt> list)
        {
            foreach (var s in list)
            {
                switch (s)
                {
                    case PilotIrAssignStmt ass:
                        if (ass.Target is PilotIrIdentifierExpr id)
                            mutated.Add(id.Name);
                        else if (ass.Target is PilotIrMemberAccessExpr mem && mem.Target is PilotIrIdentifierExpr objId)
                            mutated.Add(objId.Name);
                        else if (ass.Target is PilotIrIndexExpr idx && idx.Target is PilotIrIdentifierExpr arrId)
                            mutated.Add(arrId.Name);
                        break;
                    case PilotIrExprStmt es when es.Expression is PilotIrCallExpr call &&
                                                 call.Callee is PilotIrMemberAccessExpr m &&
                                                 m.Target is PilotIrIdentifierExpr receiver &&
                                                 m.MemberName is "append" or "push" or "pop" or "insert" or "remove" or "clear" or "add" or "Add" or "push_back":
                        mutated.Add(receiver.Name);
                        break;
                    case PilotIrIfStmt ifs:
                        Walk(ifs.ThenBody);
                        foreach (var (_, b) in ifs.ElifBranches) Walk(b);
                        Walk(ifs.ElseBody);
                        break;
                    case PilotIrWhileStmt ws:
                        Walk(ws.Body);
                        break;
                    case PilotIrForStmt fs:
                        Walk(fs.Body);
                        break;
                    case PilotIrGuardStmt gs:
                        Walk(gs.ElseBody);
                        break;
                    case PilotIrTryStmt ts:
                        Walk(ts.TryBody);
                        foreach (var c in ts.Catches) Walk(c.Body);
                        Walk(ts.FinallyBody);
                        break;
                }
            }
        }
        Walk(stmts);
        return mutated;
    }
}
