using System;
using System.Collections.Generic;
using System.IO;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Analysis.TypeSystem;
using Pilot.Runtime.Bytecode;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Analysis.Comptime;

/// <summary>
/// First-class compile-time (`comptime`) execution engine &amp; `.rodata` constant materializer (Milestone 19.2).
/// Evaluates `comptime { ... }` blocks, `comptime(...)` expressions, pure `const` initializers,
/// `comptime assert` checks, and `comptime` function calls inside a deterministic sandboxed context,
/// freezing resulting arrays, records, and maps (<see cref="PilotFrozenMap"/> perfect-hash tables)
/// directly into the constant pool.
/// </summary>
public sealed class PilotComptimeEvaluator : PilotBaseVisitor<object?>
{
    public const int DefaultStepBudget = 1_000_000;

    private readonly PilotEngine _sandboxEngine;
    private readonly PilotExecutionContext _sandboxContext;
    private readonly Dictionary<IParseTree, PilotValue> _precomputedValues = new(ReferenceEqualityComparer.Instance);
    private readonly Dictionary<string, PilotValue> _constValues = new(StringComparer.Ordinal);
    private readonly Dictionary<string, PilotParser.Func_defContext> _funcDefs = new(StringComparer.Ordinal);
    private readonly HashSet<string> _localParametersInScope = new(StringComparer.Ordinal);
    private readonly List<PilotAnalysisDiagnostic> _diagnostics = new();
    private readonly int _stepBudget;

    public IReadOnlyDictionary<IParseTree, PilotValue> PrecomputedValues => _precomputedValues;
    public IReadOnlyDictionary<string, PilotValue> ConstValues => _constValues;
    public IReadOnlyList<PilotAnalysisDiagnostic> Diagnostics => _diagnostics;

    private static readonly HashSet<string> ImpureBuiltins = new(StringComparer.OrdinalIgnoreCase)
    {
        "print", "println", "read_file", "write_file", "append_file", "delete_file",
        "http_get", "http_post", "exec_shell", "spawn_process", "input", "sleep", "shared_buffer"
    };

    public PilotComptimeEvaluator(int stepBudget = DefaultStepBudget)
    {
        _stepBudget = stepBudget;
        var writer = new StringWriter();
        var options = new PilotExecutionOptions
        {
            StandardOutput = writer,
            StandardError = writer,
            MaxCallDepth = 256
        };
        _sandboxEngine = new PilotEngine();
        _sandboxContext = new PilotExecutionContext(_sandboxEngine, options)
        {
            IsComptimeEvaluation = true,
            ComptimeStepBudget = stepBudget
        };
    }

    public void EvaluateProgram(PilotParser.ProgramContext program)
    {
        if (program?.stmt() == null) return;

        // Pass 1: Register imports, pure/comptime type definitions, and function declarations in sandbox
        foreach (var stmt in program.stmt())
        {
            if (stmt.simple_stmt()?.small_stmt() is { } smalls)
            {
                foreach (var sm in smalls)
                {
                    if (sm.import_stmt() is { } imp && imp.foreign_import_stmt() == null)
                    {
                        TryExecuteInSandbox(imp);
                    }
                    else if (sm.type_alias_stmt() != null)
                    {
                        TryExecuteInSandbox(sm);
                    }
                }
            }

            var comp = stmt.compound_stmt();
            if (comp == null) continue;

            if (comp.record_def() != null || comp.enum_def() != null || comp.struct_def() != null || comp.class_def() != null)
            {
                TryExecuteInSandbox(comp);
            }
            else if (comp.func_def() is { } fnDef)
            {
                string fnName = fnDef.identifier().GetText();
                _funcDefs[fnName] = fnDef;
                TryExecuteInSandbox(fnDef);
            }
        }

        // Pass 2: Evaluate top-level constants, comptime blocks, comptime assertions, and function bodies
        foreach (var stmt in program.stmt())
        {
            Visit(stmt);
        }
    }

    public override object? VisitFunc_def(PilotParser.Func_defContext context)
    {
        var savedLocals = new HashSet<string>(_localParametersInScope, StringComparer.Ordinal);
        var paramList = context.parameters()?.parameter();
        if (paramList != null)
        {
            foreach (var p in paramList)
            {
                string? pName = p.identifier()?.GetText();
                if (!string.IsNullOrEmpty(pName))
                {
                    if (p.COMPTIME() == null)
                    {
                        _localParametersInScope.Add(pName);
                    }
                }
            }
        }

        if (context.suite() != null)
        {
            Visit(context.suite());
        }

        _localParametersInScope.Clear();
        foreach (var s in savedLocals) _localParametersInScope.Add(s);
        return null;
    }

    public override object? VisitConst_stmt(PilotParser.Const_stmtContext context)
    {
        string? name = context.binding_pattern()?.identifier()?.GetText();
        var rhs = context.test();
        if (!string.IsNullOrEmpty(name) && rhs != null)
        {
            Visit(rhs);
            PilotValue existingFrozen = default;
            if (TryUnwrapSingleAtomExpr(rhs) is { } innerAtom &&
                innerAtom.atom()?.comptime_expr() is { } innerCtExpr &&
                _precomputedValues.TryGetValue(innerCtExpr, out existingFrozen))
            {
                _constValues[name] = existingFrozen;
                _sandboxContext.Environment.CurrentScope.Declare(name, existingFrozen);
            }
            else if (!ContainsImpureCall(rhs) && !ReferencesDynamicRuntimeLocal(rhs))
            {
                if (TryEvaluateNodeInSandbox(rhs, out var val, out _))
                {
                    var frozen = val.Freeze();
                    _precomputedValues[rhs] = frozen;
                    _constValues[name] = frozen;
                    _sandboxContext.Environment.CurrentScope.Declare(name, frozen);
                }
            }
        }
        return null;
    }

    public override object? VisitLet_stmt(PilotParser.Let_stmtContext context)
    {
        string? name = context.binding_pattern()?.identifier()?.GetText();
        var rhs = context.test();
        if (rhs != null)
        {
            Visit(rhs);
            if (!string.IsNullOrEmpty(name) && _precomputedValues.TryGetValue(rhs, out var preVal))
            {
                _constValues[name] = preVal;
                _sandboxContext.Environment.CurrentScope.Declare(name, preVal);
            }
            else if (!string.IsNullOrEmpty(name))
            {
                _localParametersInScope.Add(name);
            }
        }
        return null;
    }

    public override object? VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        string? name = context.binding_pattern()?.identifier()?.GetText();
        if (!string.IsNullOrEmpty(name))
        {
            _localParametersInScope.Add(name);
        }
        if (context.test() != null)
        {
            Visit(context.test());
        }
        return null;
    }

    public override object? VisitComptime_stmt(PilotParser.Comptime_stmtContext context)
    {
        if (!ValidateComptimePurity(context))
        {
            return null;
        }

        if (TryEvaluateNodeInSandbox(context, out var result, out string? errorMsg))
        {
            _precomputedValues[context] = result.Freeze();
        }
        else if (!string.IsNullOrEmpty(errorMsg))
        {
            ReportComptimeFailure(context, errorMsg);
        }

        return null;
    }

    public override object? VisitComptime_expr(PilotParser.Comptime_exprContext context)
    {
        if (!ValidateComptimePurity(context))
        {
            return null;
        }

        if (TryEvaluateNodeInSandbox(context, out var result, out string? errorMsg))
        {
            _precomputedValues[context] = result.Freeze();
        }
        else if (!string.IsNullOrEmpty(errorMsg))
        {
            ReportComptimeFailure(context, errorMsg);
        }

        return null;
    }

    public override object? VisitAssert_stmt(PilotParser.Assert_stmtContext context)
    {
        if (context.COMPTIME() != null)
        {
            var tests = context.test();
            if (tests != null && tests.Length > 0)
            {
                if (!ValidateComptimePurity(tests[0]))
                {
                    return null;
                }

                if (TryEvaluateNodeInSandbox(tests[0], out var condVal, out string? err))
                {
                    if (!condVal.IsTrue)
                    {
                        string customMsg = tests.Length > 1 && TryEvaluateNodeInSandbox(tests[1], out var msgVal, out _)
                            ? msgVal.ToString()
                            : $"Compile-time assertion failed: 'comptime assert {tests[0].GetText()}' evaluated to false.";
                        _diagnostics.Add(new PilotAnalysisDiagnostic(
                            "COMPTIME008",
                            customMsg,
                            context.Start.Line,
                            context.Start.Column,
                            isError: true));
                    }
                    else
                    {
                        _precomputedValues[context] = PilotValue.True;
                    }
                }
                else if (!string.IsNullOrEmpty(err))
                {
                    ReportComptimeFailure(context, err);
                }
            }
            return null;
        }

        return base.VisitAssert_stmt(context);
    }

    public override object? VisitAtom_expr(PilotParser.Atom_exprContext context)
    {
        string? calleeName = context.atom()?.identifier()?.GetText();
        var trailers = context.trailer();
        if (!string.IsNullOrEmpty(calleeName) && trailers is { Length: > 0 } && trailers[0].LPAR() != null)
        {
            if (_funcDefs.TryGetValue(calleeName, out var fnDef))
            {
                var parameters = fnDef.parameters()?.parameter();
                var args = trailers[0].arglist()?.argument();
                bool allComptimeArgsValid = true;

                if (parameters != null)
                {
                    for (int i = 0; i < parameters.Length; i++)
                    {
                        if (parameters[i].COMPTIME() != null)
                        {
                            var argExpr = args != null && i < args.Length ? args[i].test() : parameters[i].test();
                            if (argExpr == null || !IsComptimeConstantExpression(argExpr))
                            {
                                allComptimeArgsValid = false;
                                string pName = parameters[i].identifier()?.GetText() ?? $"#{i}";
                                _diagnostics.Add(new PilotAnalysisDiagnostic(
                                    "COMPTIME010",
                                    $"Argument for 'comptime' parameter '{pName}' of '{calleeName}' must be a compile-time constant expression.",
                                    (argExpr ?? (ParserRuleContext)context).Start.Line,
                                    (argExpr ?? (ParserRuleContext)context).Start.Column,
                                    isError: true));
                            }
                        }
                    }
                }

                // If function is declared `comptime def` (or all args are constant and it has a comptime parameter), fold at compile time
                if (allComptimeArgsValid &&
                    (fnDef.COMPTIME() != null || (parameters != null && Array.Exists(parameters, p => p.COMPTIME() != null))) &&
                    AreAllCallArgumentsConstant(args))
                {
                    if (TryEvaluateNodeInSandbox(context, out var foldedVal, out string? err))
                    {
                        _precomputedValues[context] = foldedVal.Freeze();
                    }
                    else if (!string.IsNullOrEmpty(err))
                    {
                        ReportComptimeFailure(context, err);
                    }
                }
            }
        }

        if (context.atom()?.comptime_expr() is { } ce)
        {
            VisitComptime_expr(ce);
            if (trailers == null || trailers.Length == 0)
            {
                if (_precomputedValues.TryGetValue(ce, out var val))
                {
                    _precomputedValues[context] = val;
                }
                return null;
            }
        }

        return base.VisitAtom_expr(context);
    }

    public override object? VisitTest(PilotParser.TestContext context)
    {
        base.VisitTest(context);
        // Propagate single-atom precomputed value up to enclosing TestContext so let/const/arg sites can look it up directly
        if (!_precomputedValues.ContainsKey(context))
        {
            var atomExpr = TryUnwrapSingleAtomExpr(context);
            if (atomExpr != null)
            {
                if (_precomputedValues.TryGetValue(atomExpr, out var val))
                {
                    _precomputedValues[context] = val;
                }
                else if (atomExpr.trailer() == null || atomExpr.trailer().Length == 0)
                {
                    if (atomExpr.atom()?.comptime_expr() is { } ce && _precomputedValues.TryGetValue(ce, out var ceVal))
                    {
                        _precomputedValues[context] = ceVal;
                    }
                }
            }
        }
        return null;
    }

    private bool ValidateComptimePurity(ParserRuleContext node)
    {
        if (ContainsImpureCall(node, out string? impureName))
        {
            _diagnostics.Add(new PilotAnalysisDiagnostic(
                "COMPTIME009",
                $"Impure host or I/O operation '{impureName}' is forbidden inside a 'comptime' evaluation context.",
                node.Start.Line,
                node.Start.Column,
                isError: true));
            return false;
        }
        return true;
    }

    private void ReportComptimeFailure(ParserRuleContext node, string errorMsg)
    {
        string code = errorMsg.Contains("[COMPTIME008]", StringComparison.Ordinal) ? "COMPTIME008" : "COMPTIME009";
        _diagnostics.Add(new PilotAnalysisDiagnostic(
            code,
            errorMsg,
            node.Start.Line,
            node.Start.Column,
            isError: true));
    }

    private bool IsComptimeConstantExpression(PilotParser.TestContext expr)
    {
        if (_precomputedValues.ContainsKey(expr)) return true;
        if (ReferencesDynamicRuntimeLocal(expr)) return false;
        if (ContainsImpureCall(expr)) return false;
        return TryEvaluateNodeInSandbox(expr, out _, out _);
    }

    private bool AreAllCallArgumentsConstant(PilotParser.ArgumentContext[]? args)
    {
        if (args == null || args.Length == 0) return true;
        foreach (var a in args)
        {
            if (a.test() == null || !IsComptimeConstantExpression(a.test()))
            {
                return false;
            }
        }
        return true;
    }

    private bool ReferencesDynamicRuntimeLocal(IParseTree node)
    {
        if (node is PilotParser.AtomContext atom && atom.identifier() != null && atom.DOT() == null)
        {
            string id = atom.identifier().GetText();
            if (_localParametersInScope.Contains(id) && !_constValues.ContainsKey(id))
            {
                return true;
            }
        }
        for (int i = 0; i < node.ChildCount; i++)
        {
            if (ReferencesDynamicRuntimeLocal(node.GetChild(i)))
            {
                return true;
            }
        }
        return false;
    }

    private static bool ContainsImpureCall(IParseTree node) => ContainsImpureCall(node, out _);

    private static bool ContainsImpureCall(IParseTree node, out string? impureName)
    {
        if (node is PilotParser.Inline_foreign_blockContext)
        {
            impureName = "inline_foreign_block";
            return true;
        }
        if (node is PilotParser.Atom_exprContext atomExpr &&
            atomExpr.trailer() is { Length: > 0 } &&
            atomExpr.trailer(0).LPAR() != null)
        {
            string? id = atomExpr.atom()?.identifier()?.GetText();
            if (!string.IsNullOrEmpty(id) && ImpureBuiltins.Contains(id))
            {
                impureName = id;
                return true;
            }
        }
        for (int i = 0; i < node.ChildCount; i++)
        {
            if (ContainsImpureCall(node.GetChild(i), out impureName))
            {
                return true;
            }
        }
        impureName = null;
        return false;
    }

    private void TryExecuteInSandbox(IParseTree node)
    {
        try
        {
            _sandboxContext.ComptimeStepsExecuted = 0;
            _sandboxContext.ComptimeStepBudget = _stepBudget;
            var visitor = _sandboxContext.GetOrCreateVisitor();
            visitor.Visit(node);
        }
        catch
        {
            // Ignore sandbox registration errors on partial declarations
        }
    }

    private bool TryEvaluateNodeInSandbox(IParseTree node, out PilotValue result, out string? errorMessage)
    {
        try
        {
            _sandboxContext.ComptimeStepsExecuted = 0;
            _sandboxContext.ComptimeStepBudget = _stepBudget;
            _sandboxContext.PendingFlow = PilotFlowSignalKind.None;

            if (node is PilotParser.Comptime_stmtContext cs && cs.stmt() is { Length: > 0 } stmts)
            {
                result = ExecuteComptimeBlockViaPilVm(stmts);
                errorMessage = null;
                return true;
            }
            if (node is PilotParser.Comptime_exprContext ce && ce.stmt() is { Length: > 0 } exprStmts)
            {
                result = ExecuteComptimeBlockViaPilVm(exprStmts);
                errorMessage = null;
                return true;
            }

            var visitor = _sandboxContext.GetOrCreateVisitor();
            result = visitor.Visit(node);
            errorMessage = null;
            return true;
        }
        catch (Exception ex)
        {
            result = PilotValue.Undefined;
            errorMessage = ex.Message;
            return false;
        }
    }

    private PilotValue ExecuteComptimeBlockViaPilVm(PilotParser.StmtContext[] stmts)
    {
        // Compile statements into a temporary PilVM chunk and execute under the deterministic step budget
        var compiler = new Pilot.Compiler.PilotBytecodeCompiler("<comptime>", isFunction: true);
        compiler.DiscardExpressionResult = false;
        for (int i = 0; i < stmts.Length; i++)
        {
            compiler.DiscardExpressionResult = (i < stmts.Length - 1);
            compiler.Visit(stmts[i]);
        }
        compiler.Chunk.Emit(PilotOpcode.Halt);
        _sandboxContext.Environment.PushScope("comptime_vm");
        try
        {
            return PilVirtualMachine.Execute(compiler.Chunk, _sandboxContext).Freeze();
        }
        finally
        {
            _sandboxContext.Environment.PopScope();
        }
    }

    private static PilotParser.Atom_exprContext? TryUnwrapSingleAtomExpr(PilotParser.TestContext test)
    {
        if (test.IF() != null || test.pipe_expr()?.Length != 1) return null;
        var pipe = test.pipe_expr(0);
        if (pipe.set_query_expr()?.Length != 1) return null;
        var sq = pipe.set_query_expr(0);
        if (sq.coalesce_expr()?.Length != 1) return null;
        var coal = sq.coalesce_expr(0);
        if (coal.ChildCount != 1 || coal.or_test()?.Length != 1) return null;
        var orT = coal.or_test(0);
        if (orT.and_test()?.Length != 1) return null;
        var andT = orT.and_test(0);
        if (andT.not_test()?.Length != 1 || andT.not_test(0).NOT() != null) return null;
        var comp = andT.not_test(0).comparison();
        if (comp == null || comp.range_expr()?.Length != 1 || comp.comp_op()?.Length > 0) return null;
        var rng = comp.range_expr(0);
        if (rng.bitor_expr()?.Length != 1) return null;
        var bo = rng.bitor_expr(0);
        if (bo.bitxor_expr()?.Length != 1) return null;
        var bx = bo.bitxor_expr(0);
        if (bx.bitand_expr()?.Length != 1) return null;
        var ba = bx.bitand_expr(0);
        if (ba.shift_expr()?.Length != 1) return null;
        var sh = ba.shift_expr(0);
        if (sh.arith_expr()?.Length != 1) return null;
        var ar = sh.arith_expr(0);
        if (ar.term()?.Length != 1) return null;
        var tm = ar.term(0);
        if (tm.factor()?.Length != 1) return null;
        var fc = tm.factor(0);
        if (fc.power() == null || fc.power().factor() != null) return null;
        return fc.power().atom_expr();
    }
}
