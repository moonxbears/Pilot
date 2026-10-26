using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Execution;

/// <summary>
/// Core AST visitor for executing Pilot statements, evaluating expressions,
/// and dispatching language constructs according to semantic specifications.
/// </summary>
public sealed class PilotInterpreterVisitor : PilotBaseVisitor<PilotValue>
{
    public PilotExecutionContext Context { get; }
    private PilotClassDefinition? _currentClassDef;

    public PilotInterpreterVisitor(PilotExecutionContext context)
    {
        Context = context ?? throw new ArgumentNullException(nameof(context));
    }

    #region Program & Statements

    public override PilotValue VisitProgram(PilotParser.ProgramContext context)
    {
        PilotValue lastValue = PilotValue.Undefined;
        Pilot.Debugging.PilotStackFrame? rootFrame = null;
        if (Context.Debugger != null)
        {
            rootFrame = Context.Debugger.OnFunctionEnter(
                "<global>",
                Array.Empty<PilotValue>(),
                Context.Environment.GlobalScope,
                Context);
        }

        try
        {
            foreach (var stmt in context.stmt())
            {
                lastValue = Visit(stmt);
            }
        }
        catch (PilotReturnSignal ret)
        {
            lastValue = ret.Value;
        }
        finally
        {
            Context.ExecuteDefers();
            if (Context.Debugger != null && rootFrame != null)
            {
                Context.Debugger.OnFunctionExit(rootFrame, lastValue, Context);
            }
        }

        return lastValue;
    }

    public override PilotValue VisitStmt(PilotParser.StmtContext context)
    {
        if (Context.Debugger != null)
        {
            Context.Debugger.BeforeExecuteStatement(context.Start.Line, context.Start.Column, Context);
        }

        if (context.compound_stmt() != null)
            return Visit(context.compound_stmt());
        if (context.simple_stmt() != null)
            return Visit(context.simple_stmt());
        return PilotValue.Undefined;
    }

    public override PilotValue VisitSimple_stmt(PilotParser.Simple_stmtContext context)
    {
        PilotValue lastVal = PilotValue.Undefined;
        var smalls = context.small_stmt();
        for (int i = 0; i < smalls.Length; i++)
        {
            if (i > 0 && Context.Debugger != null)
            {
                Context.Debugger.BeforeExecuteStatement(smalls[i].Start.Line, smalls[i].Start.Column, Context);
            }
            lastVal = Visit(smalls[i]);
        }
        return lastVal;
    }

    public override PilotValue VisitSuite(PilotParser.SuiteContext context)
    {
        if (context.simple_stmt() != null)
        {
            return Visit(context.simple_stmt());
        }

        PilotValue lastVal = PilotValue.Undefined;
        if (context.stmt() != null)
        {
            foreach (var stmt in context.stmt())
            {
                lastVal = Visit(stmt);
            }
        }
        return lastVal;
    }

    #endregion

    #region Variable Declarations & Assignments

    public override PilotValue VisitLet_stmt(PilotParser.Let_stmtContext context)
    {
        PilotValue initialVal = context.test() != null ? Visit(context.test()) : PilotValue.Undefined;
        DeclareBinding(context.binding_pattern(), initialVal, isConst: false);
        return initialVal;
    }

    public override PilotValue VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        PilotValue initialVal = context.test() != null ? Visit(context.test()) : PilotValue.Undefined;
        DeclareBinding(context.binding_pattern(), initialVal, isConst: false);
        return initialVal;
    }

    public override PilotValue VisitConst_stmt(PilotParser.Const_stmtContext context)
    {
        PilotValue initialVal = context.test() != null ? Visit(context.test()) : PilotValue.Undefined;
        DeclareBinding(context.binding_pattern(), initialVal, isConst: true);
        return initialVal;
    }

    private void DeclareBinding(PilotParser.Binding_patternContext context, PilotValue val, bool isConst)
    {
        if (context.identifier() != null)
        {
            string name = context.identifier().GetText();
            if (!Context.Environment.CurrentScope.Assign(name, val))
            {
                Context.Environment.CurrentScope.Declare(name, val, isConst);
            }
            return;
        }

        if (context.tuple_pattern() != null && (val.Kind == PilotValueKind.Tuple || val.Kind == PilotValueKind.List))
        {
            var items = val.Kind == PilotValueKind.Tuple ? val.AsTuple() : val.AsList();
            var patternItems = context.tuple_pattern().binding_pattern_item();
            for (int i = 0; i < Math.Min(items.Count, patternItems.Length); i++)
            {
                DeclareBinding(patternItems[i].binding_pattern(), items[i], isConst);
            }
        }
        else if (context.list_pattern() != null && (val.Kind == PilotValueKind.List || val.Kind == PilotValueKind.Tuple))
        {
            var items = val.Kind == PilotValueKind.List ? val.AsList() : val.AsTuple();
            var patternItems = context.list_pattern().binding_pattern_item();
            for (int i = 0; i < Math.Min(items.Count, patternItems.Length); i++)
            {
                DeclareBinding(patternItems[i].binding_pattern(), items[i], isConst);
            }
        }
    }

    public override PilotValue VisitExpr_stmt(PilotParser.Expr_stmtContext context)
    {
        var testList = context.testlist();
        if (testList == null) return PilotValue.Undefined;

        var tests = testList.test();
        if (context.EQUAL() != null && context.EQUAL().Length > 0 && context.test() != null && context.test().Length > 0)
        {
            // Simple assignment: a = val
            var rhsVal = Visit(context.test(0));
            AssignTarget(tests[0], rhsVal, context.test(0).GetText());
            return rhsVal;
        }

        if (context.augassign() != null && context.augassign().Length > 0 && context.test() != null && context.test().Length > 0)
        {
            // Augmented assignment: a += val
            var lhsVal = Visit(tests[0]);
            var rhsVal = Visit(context.test(0));
            var op = context.augassign(0);
            PilotValue computed = lhsVal;

            if (op.PLUSEQUAL() != null) computed = lhsVal + rhsVal;
            else if (op.MINEQUAL() != null) computed = lhsVal - rhsVal;
            else if (op.STAREQUAL() != null) computed = lhsVal * rhsVal;
            else if (op.SLASHEQUAL() != null) computed = lhsVal / rhsVal;
            else if (op.PERCENTEQUAL() != null) computed = lhsVal % rhsVal;

            AssignTarget(tests[0], computed);
            return computed;
        }

        // Standalone expression statement
        PilotValue result = PilotValue.Undefined;
        foreach (var t in tests)
        {
            result = Visit(t);
        }
        return result;
    }

    private void AssignTarget(PilotParser.TestContext targetContext, PilotValue value, string? rhsText = null)
    {
        string targetText = targetContext.GetText();

        // Contract forwarding assignment: IList.Contracts = target or self.IList.Contracts = target
        if (targetText.EndsWith(".Contracts", StringComparison.OrdinalIgnoreCase))
        {
            int cIdx = targetText.LastIndexOf(".Contracts", StringComparison.OrdinalIgnoreCase);
            string contractPart = targetText[..cIdx];
            string normContract = Pilot.Runtime.Contracts.PilotContractRegistry.NormalizeContractName(contractPart);
            string targetMember = !string.IsNullOrEmpty(rhsText)
                ? rhsText
                : (value.Kind == PilotValueKind.String ? value.AsString() : targetText);

            if (targetMember.StartsWith("self.", StringComparison.OrdinalIgnoreCase))
            {
                targetMember = targetMember[5..];
            }

            if (_currentClassDef != null)
            {
                _currentClassDef.DelegateContract(normContract, targetMember, Context);
                return;
            }

            if (Context.Environment.TryResolve("self", out var selfVal) && selfVal.Kind == PilotValueKind.Object)
            {
                var obj = selfVal.AsObject();
                obj.ContractSubstitutes[normContract] = value;
                obj.ClassDef?.DelegateContract(normContract, targetMember, Context);
                return;
            }
        }

        int dotIdx = targetText.LastIndexOf('.');
        if (dotIdx > 0 && !targetText.EndsWith("."))
        {
            string receiverExpr = targetText[..dotIdx];
            string member = targetText[(dotIdx + 1)..];
            try
            {
                var receiverVal = Context.Engine.EvaluateExpression(receiverExpr, Context.Environment.CurrentScope);
                if (receiverVal.Kind == PilotValueKind.Object)
                {
                    receiverVal.AsObject().SetMember(member, value);
                    return;
                }
            }
            catch
            {
                // Fallback to scope
            }
        }

        int bracketIdx = targetText.IndexOf('[');
        if (bracketIdx > 0 && targetText.EndsWith("]"))
        {
            string receiverExpr = targetText[..bracketIdx];
            string indexExpr = targetText[(bracketIdx + 1)..^1];
            try
            {
                var receiverVal = Context.Engine.EvaluateExpression(receiverExpr, Context.Environment.CurrentScope);
                var indexVal = Context.Engine.EvaluateExpression(indexExpr, Context.Environment.CurrentScope);
                receiverVal[indexVal] = value;
                return;
            }
            catch
            {
                // Fallback to scope
            }
        }

        // Fallback assign to scope by text
        if (!Context.Environment.CurrentScope.Assign(targetText, value))
        {
            Context.Environment.CurrentScope.Declare(targetText, value);
        }
    }

    #endregion

    #region Functions & Classes

    public override PilotValue VisitFunc_def(PilotParser.Func_defContext context)
    {
        string funcName = context.identifier().GetText();
        var paramList = new List<PilotParameter>();

        if (context.parameters()?.parameter() != null)
        {
            foreach (var p in context.parameters().parameter())
            {
                string pName = p.identifier()?.GetText() ?? p.splice_expr()?.GetText() ?? "arg";
                string? pType = p.type_annotation()?.type_spec()?.GetText();
                PilotValue? defaultVal = p.test() != null ? Visit(p.test()) : default(PilotValue?);
                bool isParams = p.PARAMS() != null || p.STAR() != null;
                bool isKwargs = p.DOUBLESTAR() != null;
                paramList.Add(new PilotParameter(pName, pType, defaultVal, isParams, isKwargs));
            }
        }

        bool isGen = ContainsYield(context.suite());
        var function = new PilotFunction(
            funcName,
            paramList,
            context.suite(),
            Context.Environment.CurrentScope,
            isAsync: context.ASYNC() != null,
            isGenerator: isGen);

        if (Context.Environment.CurrentScope.ContainsLocal(funcName))
        {
            var existing = Context.Environment.CurrentScope.Resolve(funcName);
            if (existing.Kind == PilotValueKind.MethodGroup)
            {
                var mg = existing.AsMethodGroup();
                mg.AddOverload(function);
                return existing;
            }
            else if (existing.Kind == PilotValueKind.Function)
            {
                var oldFunc = existing.AsFunction();
                var mg = new PilotMethodGroup(funcName, new[] { oldFunc, function });
                var mgVal = PilotValue.FromMethodGroup(mg);
                Context.Environment.CurrentScope.Assign(funcName, mgVal);
                return mgVal;
            }
        }

        Context.Environment.CurrentScope.Declare(funcName, PilotValue.FromFunction(function));
        return PilotValue.FromFunction(function);
    }

    private static bool ContainsYield(Antlr4.Runtime.Tree.IParseTree? tree)
    {
        if (tree == null) return false;
        if (tree is PilotParser.Func_defContext) return false;
        if (tree is PilotParser.Flow_stmtContext flow && flow.YIELD() != null) return true;
        for (int i = 0; i < tree.ChildCount; i++)
        {
            if (ContainsYield(tree.GetChild(i))) return true;
        }
        return false;
    }

    public override PilotValue VisitClass_def(PilotParser.Class_defContext context)
    {
        string className = context.identifier().GetText();

        // 1. Algebraic Class Composition: class Admin = BaseUser | AdminPrivileges
        if (context.EQUAL() != null && context.test() != null)
        {
            var rhsVal = Visit(context.test());
            if (rhsVal.Kind == PilotValueKind.Function && rhsVal.AsFunction().ClassDef != null)
            {
                var baseDef = rhsVal.AsFunction().ClassDef!;
                var namedDef = baseDef.CloneWithName(className);
                var ctor = PilotFunction.CreateNative(className, (ctx, args) => PilotValue.FromObject(namedDef.Instantiate(ctx, args)));
                ctor.ClassDef = namedDef;
                Context.Environment.CurrentScope.Declare(className, PilotValue.FromFunction(ctor));
                return PilotValue.FromFunction(ctor);
            }
            throw new PilotTypeException($"Right-hand side of class composition must evaluate to a class, got {rhsVal.Kind}", context.Start.Line, context.Start.Column);
        }

        string? baseName = context.class_bases()?.type_spec()?.FirstOrDefault()?.GetText();

        PilotClassDefinition classDef;
        if (Context.Environment.TryResolve(className, out var existingVal) &&
            existingVal.Kind == PilotValueKind.Function &&
            existingVal.AsFunction().ClassDef != null)
        {
            // Partial class: merge into existing definition
            classDef = existingVal.AsFunction().ClassDef!;
            if (baseName != null && string.IsNullOrEmpty(classDef.BaseClassName))
            {
                classDef.BaseClassName = baseName;
            }
        }
        else
        {
            classDef = new PilotClassDefinition(className, baseName);
        }

        // Enter temporary class definition scope to extract methods
        var prevClass = _currentClassDef;
        _currentClassDef = classDef;
        Context.Environment.PushScope($"ClassDef:{className}");
        try
        {
            if (context.suite()?.stmt() != null)
            {
                foreach (var stmt in context.suite().stmt())
                {
                    if (stmt.compound_stmt()?.state_def() != null)
                    {
                        VisitState_def(stmt.compound_stmt().state_def());
                    }
                    else if (stmt.compound_stmt()?.macro_def() != null)
                    {
                        var mVal = VisitMacro_def(stmt.compound_stmt().macro_def());
                        if (mVal.Kind == PilotValueKind.Function)
                        {
                            classDef.GenerativeMacros.Add(mVal.AsFunction());
                        }
                    }
                    else if (stmt.compound_stmt()?.func_def() != null)
                    {
                        var funcDef = stmt.compound_stmt().func_def();
                        var funcVal = VisitFunc_def(funcDef);
                        if (funcVal.Kind == PilotValueKind.MethodGroup)
                        {
                            var mg = funcVal.AsMethodGroup();
                            classDef.MethodGroups[mg.Name] = mg;
                        }
                        else
                        {
                            var func = funcVal.AsFunction();
                            classDef.Methods[func.Name] = func;
                        }
                    }
                    else if (stmt.simple_stmt()?.small_stmt() != null)
                    {
                        foreach (var s in stmt.simple_stmt().small_stmt())
                        {
                            if (s.steal_stmt() != null) VisitSteal_stmt(s.steal_stmt());
                            else if (s.delegate_stmt() != null) VisitDelegate_stmt(s.delegate_stmt());
                            else if (s.contract_forward_stmt() != null) VisitContract_forward_stmt(s.contract_forward_stmt());
                            else if (s.annassign_stmt() != null)
                            {
                                var ann = s.annassign_stmt();
                                string fieldName = ann.test(0)?.GetText() ?? "";
                                string fieldType = ann.type_spec()?.GetText() ?? "any";
                                PilotValue defVal = ann.test().Length > 1 ? Visit(ann.test(1)) : PilotValue.Null;
                                if (!string.IsNullOrEmpty(fieldName))
                                {
                                    classDef.StaticFields[fieldName] = defVal;
                                }
                            }
                            else if (s.let_stmt() != null)
                            {
                                var val = VisitLet_stmt(s.let_stmt());
                                string varName = s.let_stmt().binding_pattern()?.identifier()?.GetText() ?? "";
                                if (!string.IsNullOrEmpty(varName)) classDef.StaticFields[varName] = val;
                            }
                            else if (s.var_stmt() != null)
                            {
                                var val = VisitVar_stmt(s.var_stmt());
                                string varName = s.var_stmt().binding_pattern()?.identifier()?.GetText() ?? "";
                                if (!string.IsNullOrEmpty(varName)) classDef.StaticFields[varName] = val;
                            }
                            else Visit(s);
                        }
                    }
                    else
                    {
                        Visit(stmt);
                    }
                }
            }
        }
        finally
        {
            _currentClassDef = prevClass;
            Context.Environment.PopScope();
        }

        // Inherit generative macros from base classes
        if (!string.IsNullOrEmpty(classDef.BaseClassName) &&
            Context.Environment.TryResolve(classDef.BaseClassName, out var baseVal) &&
            baseVal.Kind == PilotValueKind.Function && baseVal.AsFunction().ClassDef != null)
        {
            foreach (var gm in baseVal.AsFunction().ClassDef!.GenerativeMacros)
            {
                if (!classDef.GenerativeMacros.Contains(gm))
                {
                    classDef.GenerativeMacros.Add(gm);
                }
            }
        }

        // Execute compile-time generative macros on class definition
        if (classDef.GenerativeMacros.Count > 0)
        {
            var classNode = new Pilot.Runtime.Generative.PilotClassNode(classDef);
            var classNodeVal = PilotValue.FromObject(classNode.ToPilotObject(Context));
            foreach (var macro in classDef.GenerativeMacros)
            {
                macro.Invoke(Context, new[] { classNodeVal });
            }
        }

        // Register class constructor function
        var ctorFunction = PilotFunction.CreateNative(
            className,
            (ctx, args) => PilotValue.FromObject(classDef.Instantiate(ctx, args)));
        ctorFunction.ClassDef = classDef;

        if (Context.Environment.CurrentScope.ContainsLocal(className))
        {
            Context.Environment.CurrentScope.Assign(className, PilotValue.FromFunction(ctorFunction));
        }
        else
        {
            Context.Environment.CurrentScope.Declare(className, PilotValue.FromFunction(ctorFunction));
        }

        return PilotValue.FromFunction(ctorFunction);
    }

    public override PilotValue VisitState_def(PilotParser.State_defContext context)
    {
        string stateName = context.identifier().GetText();
        if (_currentClassDef == null)
        {
            throw new PilotTypeException($"Typestate '{stateName}' can only be defined inside a class.", context.Start.Line, context.Start.Column);
        }

        var stateDef = new PilotClassDefinition($"{_currentClassDef.Name}[{stateName}]");
        var prevClass = _currentClassDef;
        _currentClassDef = stateDef;
        Context.Environment.PushScope($"StateDef:{stateName}");
        try
        {
            if (context.suite()?.stmt() != null)
            {
                foreach (var stmt in context.suite().stmt())
                {
                    var funcDef = stmt.compound_stmt()?.func_def();
                    if (funcDef != null)
                    {
                        var funcVal = VisitFunc_def(funcDef);
                        if (funcVal.Kind == PilotValueKind.MethodGroup)
                        {
                            stateDef.MethodGroups[funcVal.AsMethodGroup().Name] = funcVal.AsMethodGroup();
                        }
                        else
                        {
                            stateDef.Methods[funcVal.AsFunction().Name] = funcVal.AsFunction();
                        }
                    }
                    else
                    {
                        Visit(stmt);
                    }
                }
            }
        }
        finally
        {
            _currentClassDef = prevClass;
            Context.Environment.PopScope();
        }

        _currentClassDef.States[stateName] = stateDef;
        if (_currentClassDef.InitialState == null)
        {
            _currentClassDef.InitialState = stateName;
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitMacro_def(PilotParser.Macro_defContext context)
    {
        string macroName = context.identifier().GetText();
        var paramList = new List<PilotParameter>();

        if (context.parameters()?.parameter() != null)
        {
            foreach (var p in context.parameters().parameter())
            {
                string pName = p.identifier()?.GetText() ?? p.splice_expr()?.GetText() ?? "arg";
                string? pType = p.type_annotation()?.type_spec()?.GetText();
                PilotValue? defaultVal = p.test() != null ? Visit(p.test()) : default(PilotValue?);
                paramList.Add(new PilotParameter(pName, pType, defaultVal));
            }
        }

        var macroFunction = new PilotFunction(
            macroName,
            paramList,
            context.suite(),
            Context.Environment.CurrentScope,
            isAsync: false,
            isGenerator: false);

        Context.Environment.CurrentScope.Declare(macroName, PilotValue.FromFunction(macroFunction));
        return PilotValue.FromFunction(macroFunction);
    }

    public override PilotValue VisitExtension_def(PilotParser.Extension_defContext context)
    {
        string extName = context.identifier()?.GetText() ?? "AnonymousExtension";
        string targetType = context.type_spec().GetText();

        var ext = new Pilot.Runtime.Extensions.PilotLexicalExtension(extName, targetType);
        Context.Environment.PushScope($"ExtensionDef:{extName}");
        try
        {
            if (context.suite()?.stmt() != null)
            {
                foreach (var stmt in context.suite().stmt())
                {
                    var funcDef = stmt.compound_stmt()?.func_def();
                    if (funcDef != null)
                    {
                        var funcVal = VisitFunc_def(funcDef);
                        if (funcVal.Kind == PilotValueKind.MethodGroup)
                        {
                            ext.MethodGroups[funcVal.AsMethodGroup().Name] = funcVal.AsMethodGroup();
                        }
                        else
                        {
                            ext.Methods[funcVal.AsFunction().Name] = funcVal.AsFunction();
                        }
                    }
                }
            }
        }
        finally
        {
            Context.Environment.PopScope();
        }

        Context.RegisteredExtensions[extName] = ext;
        Context.Environment.CurrentScope.Declare(extName, PilotValue.FromForeign(ext));
        return PilotValue.FromForeign(ext);
    }

    public override PilotValue VisitSteal_stmt(PilotParser.Steal_stmtContext context)
    {
        string sourceClassName = context.dotted_name().GetText();
        PilotClassDefinition? sourceClass = null;

        if (Context.Environment.TryResolve(sourceClassName, out var val))
        {
            if (val.Kind == PilotValueKind.Function && val.AsFunction().ClassDef != null)
            {
                sourceClass = val.AsFunction().ClassDef;
            }
            else if (val.Kind == PilotValueKind.Object && val.AsObject().ClassDef != null)
            {
                sourceClass = val.AsObject().ClassDef;
            }
        }

        if (sourceClass == null)
        {
            throw new PilotRuntimeException($"Cannot steal methods: class or type '{sourceClassName}' not found.");
        }

        List<string>? specific = null;
        if (context.identifier() != null && context.identifier().Length > 0)
        {
            specific = context.identifier().Select(id => id.GetText()).ToList();
        }

        if (_currentClassDef != null)
        {
            _currentClassDef.StealFrom(sourceClass, specific);
        }
        else if (Context.Environment.TryResolve("self", out var selfVal) && selfVal.Kind == PilotValueKind.Object)
        {
            selfVal.AsObject().ClassDef?.StealFrom(sourceClass, specific);
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitDelegate_stmt(PilotParser.Delegate_stmtContext context)
    {
        string typeName = context.type_spec().GetText();
        string targetMember = context.test().GetText();
        var targetVal = Visit(context.test());

        if (_currentClassDef != null)
        {
            _currentClassDef.DelegateContract(typeName, targetMember, Context);
        }
        else if (Context.Environment.TryResolve("self", out var selfVal) && selfVal.Kind == PilotValueKind.Object)
        {
            var obj = selfVal.AsObject();
            string norm = Pilot.Runtime.Contracts.PilotContractRegistry.NormalizeContractName(typeName);
            obj.ContractSubstitutes[norm] = targetVal;
            obj.ClassDef?.DelegateContract(norm, targetMember, Context);
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitContract_forward_stmt(PilotParser.Contract_forward_stmtContext context)
    {
        string typeName = context.type_spec().GetText();
        string targetMember = context.test().GetText();
        var targetVal = Visit(context.test());

        if (_currentClassDef != null)
        {
            _currentClassDef.DelegateContract(typeName, targetMember, Context);
        }
        else if (Context.Environment.TryResolve("self", out var selfVal) && selfVal.Kind == PilotValueKind.Object)
        {
            var obj = selfVal.AsObject();
            string norm = Pilot.Runtime.Contracts.PilotContractRegistry.NormalizeContractName(typeName);
            obj.ContractSubstitutes[norm] = targetVal;
            obj.ClassDef?.DelegateContract(norm, targetMember, Context);
        }

        return PilotValue.Undefined;
    }

    #endregion

    #region Control Flow (If, While, For, Match, Guard, Defer)

    public override PilotValue VisitIf_stmt(PilotParser.If_stmtContext context)
    {
        var tests = context.test();
        var suites = context.suite();

        // Main if condition
        if (Visit(tests[0]).IsTrue)
        {
            return Visit(suites[0]);
        }

        // Elif / Else If branches
        int elifCount = tests.Length - 1;
        for (int i = 0; i < elifCount; i++)
        {
            if (Visit(tests[i + 1]).IsTrue)
            {
                return Visit(suites[i + 1]);
            }
        }

        // Else branch
        if (suites.Length > tests.Length)
        {
            return Visit(suites[^1]);
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitWhile_stmt(PilotParser.While_stmtContext context)
    {
        PilotValue lastVal = PilotValue.Undefined;
        while (Visit(context.test(0)).IsTrue)
        {
            try
            {
                lastVal = Visit(context.suite(0));
            }
            catch (PilotBreakSignal)
            {
                break;
            }
            catch (PilotContinueSignal)
            {
                continue;
            }
        }
        return lastVal;
    }

    public override PilotValue VisitFor_stmt(PilotParser.For_stmtContext context)
    {
        var iterable = Visit(context.test(0));
        IEnumerable<PilotValue> collection;

        if (iterable.Kind == PilotValueKind.List)
        {
            collection = iterable.AsList();
        }
        else if (iterable.Kind == PilotValueKind.Tuple)
        {
            collection = iterable.AsTuple();
        }
        else if (iterable.Kind == PilotValueKind.Range)
        {
            collection = iterable.AsRange().Select(i => PilotValue.FromInt(i));
        }
        else if (iterable.Kind == PilotValueKind.Set)
        {
            collection = iterable.AsSet();
        }
        else if (iterable.Kind == PilotValueKind.Generator)
        {
            collection = iterable.AsGenerator();
        }
        else if (iterable.Kind == PilotValueKind.String)
        {
            collection = iterable.AsString().Select(c => PilotValue.FromChar(c));
        }
        else if (iterable.Kind == PilotValueKind.Map)
        {
            collection = iterable.AsMap().Keys;
        }
        else
        {
            collection = Array.Empty<PilotValue>();
        }

        var forTarget = context.for_target();
        PilotValue lastVal = PilotValue.Undefined;

        foreach (var item in collection)
        {
            AssignForTarget(forTarget, item);
            try
            {
                lastVal = Visit(context.suite(0));
            }
            catch (PilotBreakSignal)
            {
                break;
            }
            catch (PilotContinueSignal)
            {
                continue;
            }
        }

        return lastVal;
    }

    public override PilotValue VisitWith_stmt(PilotParser.With_stmtContext context)
    {
        var pushedExtensions = new List<Pilot.Runtime.Extensions.PilotLexicalExtension>();

        foreach (var item in context.with_item())
        {
            if (item.EXTENSION() != null && item.identifier() != null)
            {
                string extName = item.identifier().GetText();
                if (Context.RegisteredExtensions.TryGetValue(extName, out var ext))
                {
                    Context.ActiveExtensions.Push(ext);
                    pushedExtensions.Add(ext);
                }
                else if (Context.Environment.TryResolve(extName, out var extVal) &&
                         extVal.AsForeign() is Pilot.Runtime.Extensions.PilotLexicalExtension foreignExt)
                {
                    Context.ActiveExtensions.Push(foreignExt);
                    pushedExtensions.Add(foreignExt);
                }
            }
            else if (item.test() != null)
            {
                var val = Visit(item.test());
                if (val.AsForeign() is Pilot.Runtime.Extensions.PilotLexicalExtension ext)
                {
                    Context.ActiveExtensions.Push(ext);
                    pushedExtensions.Add(ext);
                }
                if (item.identifier() != null)
                {
                    Context.Environment.CurrentScope.Declare(item.identifier().GetText(), val);
                }
            }
        }

        try
        {
            return Visit(context.suite());
        }
        finally
        {
            for (int i = 0; i < pushedExtensions.Count; i++)
            {
                Context.ActiveExtensions.Pop();
            }
        }
    }

    private void AssignForTarget(PilotParser.For_targetContext target, PilotValue value)
    {
        if (target.binding_pattern()?.Length > 0)
        {
            var bp = target.binding_pattern(0);
            if (bp.identifier() != null)
            {
                string name = bp.identifier().GetText();
                if (!Context.Environment.CurrentScope.Assign(name, value))
                {
                    Context.Environment.CurrentScope.Declare(name, value, isConst: false);
                }
                return;
            }
            DeclareBinding(bp, value, isConst: false);
        }
        else if (target.exprlist() != null)
        {
            string name = target.exprlist().GetText();
            if (!Context.Environment.CurrentScope.Assign(name, value))
            {
                Context.Environment.CurrentScope.Declare(name, value);
            }
        }
    }

    public override PilotValue VisitMatch_stmt(PilotParser.Match_stmtContext context)
    {
        var subject = Visit(context.test());
        var matchSuite = context.match_suite();
        if (matchSuite?.match_arm() == null) return PilotValue.Undefined;

        foreach (var arm in matchSuite.match_arm())
        {
            Context.Environment.PushScope("MatchArm");
            try
            {
                if (PilotPatternMatcher.Match(arm.pattern(), subject, Context.Environment.CurrentScope, this))
                {
                    // Check optional when/if guard: case pat when x > 10 => suite
                    if (arm.test() != null && !Visit(arm.test()).IsTrue)
                    {
                        continue;
                    }

                    return Visit(arm.suite());
                }
            }
            finally
            {
                Context.Environment.PopScope();
            }
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitGuard_stmt(PilotParser.Guard_stmtContext context)
    {
        PilotValue condVal;
        if (context.LET() != null && context.pattern() != null)
        {
            var rhs = Visit(context.test());
            bool matched = PilotPatternMatcher.Match(context.pattern(), rhs, Context.Environment.CurrentScope, this);
            condVal = PilotValue.FromBool(matched);
        }
        else
        {
            condVal = Visit(context.test());
        }

        if (!condVal.IsTrue)
        {
            // Guard precondition failed: execute else exit suite
            return Visit(context.suite());
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitDefer_stmt(PilotParser.Defer_stmtContext context)
    {
        var capturedScope = Context.Environment.CurrentScope;
        Context.PushDefer(() =>
        {
            var prevScope = Context.Environment.CurrentScope;
            try
            {
                if (context.suite() != null) Visit(context.suite());
                else if (context.small_stmt() != null) Visit(context.small_stmt());
                else if (context.test() != null) Visit(context.test());
            }
            finally
            {
                // Restore scope
            }
        });

        return PilotValue.Undefined;
    }

    private static readonly HashSet<string> KnownExceptionNames = new(StringComparer.OrdinalIgnoreCase)
    {
        "Exception",
        "Error",
        "RuntimeError",
        "TypeError",
        "DivideByZeroError",
        "IndexError",
        "KeyError",
        "NullReferenceError",
        "AssertionError",
        "RecursionError",
        "ContractError",
        "TimeoutError",
        "UserError",
        "PilotTypeException",
        "PilotDivideByZeroException",
        "PilotIndexOutOfRangeException",
        "PilotKeyNotFoundException",
        "PilotNullReferenceException",
        "PilotAssertionException",
        "PilotRecursionException",
        "PilotContractException",
        "PilotTimeoutException",
        "PilotUserException",
        "PilotRuntimeException"
    };

    private static bool IsKnownExceptionTypeName(string name)
    {
        if (KnownExceptionNames.Contains(name))
            return true;
        if (name.EndsWith("Error", StringComparison.OrdinalIgnoreCase) ||
            name.EndsWith("Exception", StringComparison.OrdinalIgnoreCase))
            return true;
        return false;
    }

    public override PilotValue VisitTry_stmt(PilotParser.Try_stmtContext context)
    {
        bool executedWithoutException = false;
        PilotValue lastVal = PilotValue.Undefined;

        try
        {
            try
            {
                lastVal = Visit(context.suite(0));
                executedWithoutException = true;
            }
            catch (PilotControlFlowSignal)
            {
                // Flow control signals (return, break, continue, yield) bypass except clauses
                throw;
            }
            catch (Exception ex)
            {
                bool handled = false;
                var exceptClauses = context.except_clause();
                if (exceptClauses != null && exceptClauses.Length > 0)
                {
                    foreach (var clause in exceptClauses)
                    {
                        string? typeSpecText = clause.type_spec()?.GetText();
                        string? varName = clause.identifier()?.GetText();
                        string? expectedType = typeSpecText;

                        if (string.IsNullOrEmpty(varName) && !string.IsNullOrEmpty(typeSpecText))
                        {
                            if (!IsKnownExceptionTypeName(typeSpecText))
                            {
                                varName = typeSpecText;
                                expectedType = null;
                            }
                        }

                        if (IsExceptionMatching(ex, expectedType))
                        {
                            handled = true;
                            Context.Environment.PushScope("ExceptHandler");
                            try
                            {
                                if (!string.IsNullOrEmpty(varName))
                                {
                                    PilotValue errPayload = ex is PilotUserException pue
                                        ? pue.UserPayload
                                        : (ex is PilotRuntimeException pre
                                            ? PilotValue.FromString(pre.Message)
                                            : PilotValue.FromString(ex.Message));
                                    Context.Environment.CurrentScope.Declare(varName, errPayload);
                                }
                                lastVal = Visit(clause.suite());
                            }
                            finally
                            {
                                Context.Environment.PopScope();
                            }
                            break;
                        }
                    }
                }

                if (!handled)
                {
                    throw;
                }
            }

            // Execute else suite if try block succeeded without any exception
            if (executedWithoutException && context.ELSE() != null)
            {
                var suites = context.suite();
                if (suites.Length > 1)
                {
                    lastVal = Visit(suites[1]);
                }
            }
        }
        finally
        {
            // Execute finally suite if present (always runs regardless of try/except/else outcome)
            if (context.FINALLY() != null)
            {
                var suites = context.suite();
                int finallyIdx = context.ELSE() != null ? 2 : 1;
                if (finallyIdx < suites.Length)
                {
                    Visit(suites[finallyIdx]);
                }
                else if (suites.Length > 1)
                {
                    Visit(suites[suites.Length - 1]);
                }
            }
        }

        return lastVal;
    }

    private static bool IsExceptionMatching(Exception ex, string? expectedType)
    {
        if (string.IsNullOrWhiteSpace(expectedType) ||
            expectedType.Equals("Exception", StringComparison.OrdinalIgnoreCase) ||
            expectedType.Equals("Error", StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        if (ex is PilotRuntimeException pre)
        {
            if (pre.TypeName.Equals(expectedType, StringComparison.OrdinalIgnoreCase)) return true;
            if (pre.GetType().Name.Equals(expectedType, StringComparison.OrdinalIgnoreCase)) return true;
            if (expectedType.Equals("RuntimeError", StringComparison.OrdinalIgnoreCase)) return true;
        }

        if (ex.GetType().Name.Equals(expectedType, StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        return false;
    }

    public override PilotValue VisitAssert_stmt(PilotParser.Assert_stmtContext context)
    {
        var tests = context.test();
        var cond = Visit(tests[0]);
        if (!cond.IsTrue)
        {
            string message = tests.Length > 1
                ? Visit(tests[1]).ToString()
                : $"Assertion failed: '{tests[0].GetText()}' evaluated to false.";
            throw new PilotAssertionException(message, context.Start.Line, context.Start.Column);
        }
        return PilotValue.True;
    }

    public override PilotValue VisitFlow_stmt(PilotParser.Flow_stmtContext context)
    {
        if (context.RETURN() != null)
        {
            PilotValue retVal = context.testlist() != null ? VisitTestlist(context.testlist()) : PilotValue.Undefined;
            throw new PilotReturnSignal(retVal);
        }

        if (context.BREAK() != null)
        {
            string? label = context.identifier()?.GetText();
            throw new PilotBreakSignal(label);
        }

        if (context.CONTINUE() != null)
        {
            string? label = context.identifier()?.GetText();
            throw new PilotContinueSignal(label);
        }

        if (context.YIELD() != null)
        {
            PilotValue yieldVal = context.testlist() != null ? VisitTestlist(context.testlist()) : PilotValue.Undefined;
            if (Context.YieldHandler != null)
            {
                return Context.YieldHandler(yieldVal);
            }
            throw new PilotYieldSignal(yieldVal);
        }

        if (context.RAISE() != null)
        {
            var tests = context.test();
            if (tests != null && tests.Length > 0)
            {
                var errVal = Visit(tests[0]);
                if (errVal.Kind == PilotValueKind.Object)
                {
                    var msg = errVal.AsObject().GetMember("message");
                    string messageStr = msg != PilotValue.Undefined ? msg.ToString() : errVal.ToString();
                    throw new PilotUserException(messageStr, errVal.AsObject().TypeName, errVal, context.Start.Line, context.Start.Column);
                }
                throw new PilotUserException(errVal.ToString(), "UserError", errVal, context.Start.Line, context.Start.Column);
            }
            throw new PilotUserException("Error raised.", "UserError", null, context.Start.Line, context.Start.Column);
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitTestlist(PilotParser.TestlistContext context)
    {
        var tests = context.test();
        if (tests.Length == 1) return Visit(tests[0]);

        var list = new List<PilotValue>(tests.Length);
        foreach (var t in tests)
        {
            list.Add(Visit(t));
        }
        return PilotValue.FromTuple(list);
    }

    #endregion

    #region Expressions & Operators

    public override PilotValue VisitTest(PilotParser.TestContext context)
    {
        if (context.lambda_expr() != null)
        {
            return Visit(context.lambda_expr());
        }

        var pipeExprs = context.pipe_expr();
        if (pipeExprs?.Length > 0)
        {
            var primaryVal = Visit(pipeExprs[0]);

            // Ternary expression: expr IF condition ELSE alt
            if (context.IF() != null && pipeExprs.Length > 1 && context.test() != null)
            {
                var conditionVal = Visit(pipeExprs[1]);
                return conditionVal.IsTrue ? primaryVal : Visit(context.test());
            }

            return primaryVal;
        }

        return PilotValue.Undefined;
    }

    public override PilotValue VisitPipe_expr(PilotParser.Pipe_exprContext context)
    {
        var queries = context.set_query_expr();
        var val = Visit(queries[0]);

        for (int i = 1; i < queries.Length; i++)
        {
            var next = Visit(queries[i]);
            if (next.Kind == PilotValueKind.Function)
            {
                val = next.AsFunction().Invoke(Context, new[] { val });
            }
        }

        return val;
    }

    public override PilotValue VisitSet_query_expr(PilotParser.Set_query_exprContext context)
    {
        if (context.coalesce_expr()?.Length > 0)
        {
            return Visit(context.coalesce_expr(0));
        }
        return PilotValue.Undefined;
    }

    public override PilotValue VisitCoalesce_expr(PilotParser.Coalesce_exprContext context)
    {
        var tests = context.or_test();
        var val = Visit(tests[0]);

        for (int i = 1; i < tests.Length; i++)
        {
            if (val.IsNull || val.IsUndefined)
            {
                val = Visit(tests[i]);
            }
            else
            {
                break;
            }
        }

        return val;
    }

    public override PilotValue VisitOr_test(PilotParser.Or_testContext context)
    {
        var tests = context.and_test();
        var val = Visit(tests[0]);

        for (int i = 1; i < tests.Length; i++)
        {
            if (val.IsTrue) return val;
            val = Visit(tests[i]);
        }

        return val;
    }

    public override PilotValue VisitAnd_test(PilotParser.And_testContext context)
    {
        var tests = context.not_test();
        var val = Visit(tests[0]);

        for (int i = 1; i < tests.Length; i++)
        {
            if (!val.IsTrue) return val;
            val = Visit(tests[i]);
        }

        return val;
    }

    public override PilotValue VisitNot_test(PilotParser.Not_testContext context)
    {
        if (context.NOT() != null)
        {
            return !Visit(context.not_test());
        }
        return Visit(context.comparison());
    }

    public override PilotValue VisitComparison(PilotParser.ComparisonContext context)
    {
        var ranges = context.range_expr();
        var left = Visit(ranges[0]);

        if (ranges.Length == 1 && context.pattern()?.Length == 0)
        {
            return left;
        }

        // IS / IS NOT pattern
        if (context.IS()?.Length > 0 && context.pattern()?.Length > 0)
        {
            bool matched = PilotPatternMatcher.Match(context.pattern(0), left, Context.Environment.CurrentScope, this);
            return context.NOT()?.Length > 0 ? PilotValue.FromBool(!matched) : PilotValue.FromBool(matched);
        }

        // Comparison operators: ==, !=, <, <=, >, >=, in, not in
        if (context.comp_op()?.Length > 0 && ranges.Length > 1)
        {
            var op = context.comp_op(0);
            var right = Visit(ranges[1]);

            if (op.EQEQUAL() != null) return PilotValue.FromBool(left == right);
            if (op.NOTEQUAL() != null) return PilotValue.FromBool(left != right);
            if (op.LESS() != null) return PilotValue.FromBool(left < right);
            if (op.LESSEQUAL() != null) return PilotValue.FromBool(left <= right);
            if (op.GREATER() != null) return PilotValue.FromBool(left > right);
            if (op.GREATEREQUAL() != null) return PilotValue.FromBool(left >= right);

            if (op.IN() != null)
            {
                if (right.Kind == PilotValueKind.List) return PilotValue.FromBool(right.AsList().Contains(left));
                if (right.Kind == PilotValueKind.Set) return PilotValue.FromBool(right.AsSet().Contains(left));
                if (right.Kind == PilotValueKind.Map) return PilotValue.FromBool(right.AsMap().ContainsKey(left));
                if (right.Kind == PilotValueKind.String) return PilotValue.FromBool(right.AsString().Contains(left.ToString()));
                return PilotValue.False;
            }
        }

        return left;
    }

    public override PilotValue VisitRange_expr(PilotParser.Range_exprContext context)
    {
        var bitor = context.bitor_expr();
        var startVal = Visit(bitor[0]);

        if (bitor.Length > 1)
        {
            var endVal = Visit(bitor[1]);
            bool inclusive = context.DOTDOT_EQUAL() != null;
            return PilotValue.FromRange(new PilotRange(startVal.AsInt(), endVal.AsInt(), 1, inclusive));
        }

        return startVal;
    }

    public override PilotValue VisitBitor_expr(PilotParser.Bitor_exprContext context)
    {
        var xors = context.bitxor_expr();
        var val = Visit(xors[0]);

        for (int i = 1; i < xors.Length; i++)
        {
            var rhs = Visit(xors[i]);
            val = val | rhs;
        }

        return val;
    }

    public override PilotValue VisitBitxor_expr(PilotParser.Bitxor_exprContext context)
    {
        var ands = context.bitand_expr();
        var val = Visit(ands[0]);

        for (int i = 1; i < ands.Length; i++)
        {
            var rhs = Visit(ands[i]);
            val = PilotValue.FromInt(val.AsInt() ^ rhs.AsInt());
        }

        return val;
    }

    public override PilotValue VisitBitand_expr(PilotParser.Bitand_exprContext context)
    {
        var shifts = context.shift_expr();
        var val = Visit(shifts[0]);

        for (int i = 1; i < shifts.Length; i++)
        {
            var rhs = Visit(shifts[i]);
            val = val & rhs;
        }

        return val;
    }

    public override PilotValue VisitShift_expr(PilotParser.Shift_exprContext context)
    {
        var ariths = context.arith_expr();
        var val = Visit(ariths[0]);

        for (int i = 1; i < ariths.Length; i++)
        {
            var rhs = Visit(ariths[i]);
            var op = context.GetChild(2 * i - 1).GetText();
            if (op == "<<") val = PilotValue.FromInt(val.AsInt() << (int)rhs.AsInt());
            else if (op == ">>") val = PilotValue.FromInt(val.AsInt() >> (int)rhs.AsInt());
        }

        return val;
    }

    public override PilotValue VisitArith_expr(PilotParser.Arith_exprContext context)
    {
        var terms = context.term();
        var val = Visit(terms[0]);

        for (int i = 1; i < terms.Length; i++)
        {
            var rhs = Visit(terms[i]);
            var op = context.GetChild(2 * i - 1).GetText();
            if (op == "+") val = val + rhs;
            else if (op == "-") val = val - rhs;
        }

        return val;
    }

    public override PilotValue VisitTerm(PilotParser.TermContext context)
    {
        var factors = context.factor();
        var val = Visit(factors[0]);

        for (int i = 1; i < factors.Length; i++)
        {
            var op = context.GetChild(2 * i - 1).GetText();
            if (op == "->")
            {
                // Structural Projection: source -> {field1, field2}
                val = EvaluateProjection(val, factors[i]);
                continue;
            }

            var rhs = Visit(factors[i]);
            if (op == "*") val = val * rhs;
            else if (op == "/")
            {
                if ((rhs.Kind == PilotValueKind.Int || rhs.Kind == PilotValueKind.UInt) && rhs.AsInt() == 0)
                {
                    throw new PilotDivideByZeroException("Division by zero.", context.Start.Line, context.Start.Column);
                }
                else if (rhs.Kind == PilotValueKind.Float && rhs.AsFloat() == 0.0)
                {
                    throw new PilotDivideByZeroException("Division by zero.", context.Start.Line, context.Start.Column);
                }
                val = val / rhs;
            }
            else if (op == "%")
            {
                if ((rhs.Kind == PilotValueKind.Int || rhs.Kind == PilotValueKind.UInt) && rhs.AsInt() == 0)
                {
                    throw new PilotDivideByZeroException("Modulo by zero.", context.Start.Line, context.Start.Column);
                }
                val = val % rhs;
            }
            else if (op == "//")
            {
                long divisor = rhs.AsInt();
                if (divisor == 0)
                {
                    throw new PilotDivideByZeroException("Integer division by zero.", context.Start.Line, context.Start.Column);
                }
                val = PilotValue.FromInt(val.AsInt() / divisor);
            }
        }

        return val;
    }

    private PilotValue EvaluateProjection(PilotValue source, PilotParser.FactorContext factorCtx)
    {
        var fieldNames = new List<string>();

        var atom = factorCtx.power()?.atom_expr()?.atom();
        if (atom?.LBRACE() != null && atom.dictorsetmaker() != null)
        {
            var dmaker = atom.dictorsetmaker();
            if (dmaker.collection_element() != null)
            {
                foreach (var elem in dmaker.collection_element())
                {
                    string fName = elem.test()?.GetText()?.Trim('"', '\'') ?? "";
                    if (!string.IsNullOrEmpty(fName)) fieldNames.Add(fName);
                }
            }
            if (dmaker.dict_entry() != null)
            {
                foreach (var entry in dmaker.dict_entry())
                {
                    string fName = entry.test(0)?.GetText()?.Trim('"', '\'') ?? "";
                    if (!string.IsNullOrEmpty(fName)) fieldNames.Add(fName);
                }
            }
        }
        else if (atom?.collection_expr() != null)
        {
            foreach (var el in atom.collection_expr().collection_element())
            {
                string fName = el.test()?.GetText()?.Trim('"', '\'') ?? "";
                if (!string.IsNullOrEmpty(fName)) fieldNames.Add(fName);
            }
        }
        else
        {
            var rhsVal = Visit(factorCtx);
            if (rhsVal.Kind == PilotValueKind.List)
            {
                foreach (var item in rhsVal.AsList()) fieldNames.Add(item.AsString());
            }
            else if (rhsVal.Kind == PilotValueKind.Set)
            {
                foreach (var item in rhsVal.AsSet()) fieldNames.Add(item.AsString());
            }
            else if (rhsVal.Kind == PilotValueKind.String)
            {
                fieldNames.Add(rhsVal.AsString());
            }
        }

        var map = new Dictionary<PilotValue, PilotValue>();
        foreach (var field in fieldNames)
        {
            var fVal = source.GetMember(field, Context);
            map[PilotValue.FromString(field)] = fVal;
        }

        return PilotValue.FromMap(map);
    }

    public override PilotValue VisitFactor(PilotParser.FactorContext context)
    {
        if (context.PLUS() != null) return Visit(context.factor());
        if (context.MINUS() != null) return -Visit(context.factor());
        if (context.power() != null) return Visit(context.power());
        if (context.unary_op() != null) return Visit(context.unary_op());
        return PilotValue.Undefined;
    }

    public override PilotValue VisitUnary_op(PilotParser.Unary_opContext context)
    {
        if (context.AWAIT() != null)
        {
            var val = Visit(context.factor());
            if (val.Kind == PilotValueKind.Future)
            {
                return val.AsFuture().Await();
            }
            return val;
        }

        if (context.SPAWN() != null)
        {
            var factor = context.factor();
            var future = Pilot.Runtime.Concurrency.PilotFuture.Run(() =>
            {
                var subContext = new PilotExecutionContext(
                    Context.Engine,
                    Context.Options,
                    new PilotEnvironment(Context.Environment.CurrentScope));
                var subVisitor = new PilotInterpreterVisitor(subContext);
                return subVisitor.Visit(factor);
            });
            return PilotValue.FromFuture(future);
        }

        if (context.ARROW_LEFT() != null)
        {
            return Visit(context.factor());
        }

        if (context.quote_expr() != null) return Visit(context.quote_expr());
        if (context.quasiquote_expr() != null) return Visit(context.quasiquote_expr());
        if (context.splice_expr() != null) return Visit(context.splice_expr());

        return PilotValue.Undefined;
    }

    public override PilotValue VisitPower(PilotParser.PowerContext context)
    {
        var baseVal = Visit(context.atom_expr());
        if (context.DOUBLESTAR() != null && context.factor() != null)
        {
            var expVal = Visit(context.factor());
            return PilotValue.FromFloat(Math.Pow(baseVal.AsFloat(), expVal.AsFloat()));
        }
        return baseVal;
    }

    #endregion

    #region Atom Expressions, Cascade Chaining, & Member Access

    public override PilotValue VisitAtom_expr(PilotParser.Atom_exprContext context)
    {
        var current = Visit(context.atom());

        foreach (var trailer in context.trailer())
        {
            // Multi-cast Broadcast Call: *(...) or .*(...)
            if (trailer.STAR() != null && trailer.LPAR() != null)
            {
                var (posArgs, namedArgs) = EvaluateCallArguments(trailer.arglist());
                if (current.Kind == PilotValueKind.MethodGroup)
                {
                    current = current.AsMethodGroup().Invoke(Context, posArgs, namedArgs, isBroadcast: true);
                }
                else if (current.Kind == PilotValueKind.Function)
                {
                    var boundArgs = BindArgumentsToFunction(current.AsFunction(), posArgs, namedArgs);
                    var res = current.AsFunction().Invoke(Context, boundArgs);
                    current = PilotValue.FromList(new List<PilotValue> { res });
                }
                else if (current.IsNumber && posArgs.Count > 0)
                {
                    current = current * posArgs[0];
                }
                else
                {
                    throw new PilotTypeException($"Cannot broadcast invoke non-callable value: {current}", context.Start.Line, context.Start.Column);
                }
            }
            // Method / Function Call: (...)
            else if (trailer.LPAR() != null)
            {
                var (posArgs, namedArgs) = EvaluateCallArguments(trailer.arglist());
                if (current.Kind == PilotValueKind.MethodGroup)
                {
                    current = current.AsMethodGroup().Invoke(Context, posArgs, namedArgs, isBroadcast: false);
                }
                else if (current.Kind == PilotValueKind.Function)
                {
                    var boundArgs = BindArgumentsToFunction(current.AsFunction(), posArgs, namedArgs);
                    current = current.AsFunction().Invoke(Context, boundArgs);
                }
                else
                {
                    throw new PilotTypeException($"Attempted to invoke non-function value: {current}", context.Start.Line, context.Start.Column);
                }
            }
            // Dot Member Access: .member
            else if (trailer.DOT() != null || trailer.SAFE_NAV() != null)
            {
                if (trailer.SAFE_NAV() != null && (current.IsNull || current.IsUndefined))
                {
                    current = PilotValue.Undefined;
                    continue;
                }

                if (trailer.DOT() != null && (current.IsNull || current.IsUndefined))
                {
                    string mName = trailer.identifier()?.GetText() ?? "unknown";
                    throw new PilotNullReferenceException($"Cannot access member '{mName}' on null or undefined value.", context.Start.Line, context.Start.Column);
                }

                string memberName = trailer.identifier().GetText();
                current = current.GetMember(memberName, Context);
            }
            // Indexing: [index]
            else if (trailer.LSQB() != null)
            {
                var subCtx = trailer.subscriptlist()?.subscript(0);
                if (subCtx != null)
                {
                    var idxVal = Visit(subCtx.test(0));
                    current = current[idxVal];
                }
            }
            // Cascade Operator: ..method() or ..prop = val
            else if (trailer.cascade_trailer() != null)
            {
                current = EvaluateCascade(current, trailer.cascade_trailer());
            }
        }

        return current;
    }

    private PilotValue EvaluateCascade(PilotValue receiver, PilotParser.Cascade_trailerContext cascadeCtx)
    {
        var target = cascadeCtx.cascade_target();
        if (target == null) return receiver;

        // Cascade method call: ..func(args) or ..let res = func(args) or ..func(args) -> res
        if (target.dotted_name() != null && target.LPAR() != null)
        {
            string methodName = target.dotted_name().GetText();
            var member = receiver.GetMember(methodName, Context);
            var (posArgs, namedArgs) = EvaluateCallArguments(target.arglist());

            PilotValue callResult = PilotValue.Undefined;
            if (member.Kind == PilotValueKind.Function)
            {
                var boundArgs = BindArgumentsToFunction(member.AsFunction(), posArgs, namedArgs);
                callResult = member.AsFunction().Invoke(Context, boundArgs);
            }
            else if (member.Kind == PilotValueKind.MethodGroup)
            {
                callResult = member.AsMethodGroup().Invoke(Context, posArgs, namedArgs, isBroadcast: false);
            }

            // Bind intermediate return value if requested: (let res = func()) or func() -> res
            var binding = target.binding_pattern();
            if (binding?.Length > 0)
            {
                DeclareBinding(binding[0], callResult, isConst: false);
            }

            // In cascade chaining, return receiver self!
            return receiver;
        }

        // Cascade property assignment: ..prop = val
        if (target.dotted_name() != null && target.EQUAL() != null && target.test() != null)
        {
            string propName = target.dotted_name().GetText();
            var propVal = Visit(target.test());
            if (receiver.Kind == PilotValueKind.Object)
            {
                receiver.AsObject().SetMember(propName, propVal);
            }
            return receiver;
        }

        return receiver;
    }

    private (List<PilotValue> Positional, Dictionary<string, PilotValue> Named) EvaluateCallArguments(PilotParser.ArglistContext? arglist)
    {
        var positional = new List<PilotValue>();
        var named = new Dictionary<string, PilotValue>(StringComparer.Ordinal);
        if (arglist?.argument() == null) return (positional, named);

        foreach (var arg in arglist.argument())
        {
            if (arg.identifier() != null && arg.EQUAL() != null && arg.test() != null)
            {
                string key = arg.identifier().GetText();
                var val = Visit(arg.test());
                named[key] = val;
            }
            else if (arg.test() != null)
            {
                positional.Add(Visit(arg.test()));
            }
        }
        return (positional, named);
    }

    private IReadOnlyList<PilotValue> EvaluateArglist(PilotParser.ArglistContext? arglist)
    {
        return EvaluateCallArguments(arglist).Positional;
    }

    private static IReadOnlyList<PilotValue> BindArgumentsToFunction(
        PilotFunction function,
        IReadOnlyList<PilotValue> posArgs,
        IReadOnlyDictionary<string, PilotValue> namedArgs)
    {
        if (namedArgs.Count == 0) return posArgs;

        var full = new List<PilotValue>();
        int posIdx = 0;
        for (int i = 0; i < function.Parameters.Count; i++)
        {
            var p = function.Parameters[i];
            if (namedArgs.TryGetValue(p.Name, out var nVal))
            {
                full.Add(nVal);
            }
            else if (posIdx < posArgs.Count)
            {
                full.Add(posArgs[posIdx++]);
            }
            else if (p.DefaultValue.HasValue)
            {
                full.Add(p.DefaultValue.Value);
            }
            else
            {
                full.Add(PilotValue.Undefined);
            }
        }
        while (posIdx < posArgs.Count)
        {
            full.Add(posArgs[posIdx++]);
        }
        return full;
    }

    #endregion

    #region Atoms, Literals & Collections

    public override PilotValue VisitAtom(PilotParser.AtomContext context)
    {
        if (context.literal() != null) return Visit(context.literal());
        if (context.identifier() != null) return ResolveIdentifier(context.identifier().GetText());
        if (context.collection_expr() != null) return VisitCollection_expr(context.collection_expr());
        if (context.LPAR() != null && context.testlist_comp() != null) return VisitTestlist_comp(context.testlist_comp());
        if (context.LBRACE() != null && context.dictorsetmaker() != null) return VisitDictorsetmaker(context.dictorsetmaker());
        if (context.nameof_expr() != null) return PilotValue.FromString(context.nameof_expr().dotted_name()?.GetText() ?? context.nameof_expr().test()?.GetText() ?? "");
        if (context.inline_foreign_block() != null) return VisitInline_foreign_block(context.inline_foreign_block());
        if (context.quote_expr() != null) return PilotValue.FromString(context.quote_expr().GetText());
        if (context.quasiquote_expr() != null) return PilotValue.FromString(context.quasiquote_expr().GetText());
        if (context.splice_expr() != null) return ResolveIdentifier(context.splice_expr().GetText().TrimStart('$', '@'));

        return PilotValue.Undefined;
    }

    private PilotValue ResolveIdentifier(string name)
    {
        if (name == "true") return PilotValue.True;
        if (name == "false") return PilotValue.False;
        if (name == "null" || name == "none") return PilotValue.Null;
        if (name == "undefined") return PilotValue.Undefined;

        if (Context.Environment.TryResolve(name, out var val))
        {
            return val;
        }

        if (Context.Options.Builtins.TryGet(name, out var builtin))
        {
            return PilotValue.FromFunction(builtin);
        }

        throw new PilotRuntimeException($"Undefined variable '{name}'.");
    }

    public override PilotValue VisitLiteral(PilotParser.LiteralContext context)
    {
        if (context.INTEGER() != null)
        {
            string text = context.INTEGER().GetText().Replace("_", "");
            if (text.StartsWith("0x", StringComparison.OrdinalIgnoreCase))
                return PilotValue.FromInt(Convert.ToInt64(text[2..], 16));
            if (text.StartsWith("0b", StringComparison.OrdinalIgnoreCase))
                return PilotValue.FromInt(Convert.ToInt64(text[2..], 2));
            return PilotValue.FromInt(long.Parse(text, CultureInfo.InvariantCulture));
        }

        if (context.FLOAT_NUMBER() != null)
        {
            string text = context.FLOAT_NUMBER().GetText().Replace("_", "");
            return PilotValue.FromFloat(double.Parse(text, CultureInfo.InvariantCulture));
        }

        if (context.FSTRING()?.Length > 0)
        {
            string raw = context.FSTRING(0).GetText();
            return EvaluateFString(raw);
        }

        if (context.STRING()?.Length > 0)
        {
            string raw = context.STRING(0).GetText();
            if ((raw.StartsWith("f\"", StringComparison.OrdinalIgnoreCase) || raw.StartsWith("f'", StringComparison.OrdinalIgnoreCase)) &&
                raw.Length >= 3 && (raw.EndsWith("\"") || raw.EndsWith("'")))
            {
                return EvaluateFString(raw);
            }
            return PilotValue.FromString(UnescapeString(raw));
        }

        if (context.TRIPLE_STRING()?.Length > 0)
        {
            string raw = context.TRIPLE_STRING(0).GetText();
            if (raw.Length >= 6) raw = raw[3..^3];
            return PilotValue.FromString(raw);
        }

        if (context.TRUE() != null) return PilotValue.True;
        if (context.FALSE() != null) return PilotValue.False;
        if (context.NULL() != null || context.NONE() != null) return PilotValue.Null;
        if (context.UNDEFINED() != null) return PilotValue.Undefined;

        return PilotValue.Undefined;
    }

    private PilotValue EvaluateFString(string raw)
    {
        string content = raw.Substring(2, raw.Length - 3);
        var sb = new StringBuilder();
        int i = 0;
        while (i < content.Length)
        {
            if (content[i] == '{')
            {
                int endBrace = content.IndexOf('}', i);
                if (endBrace > i)
                {
                    string expr = content.Substring(i + 1, endBrace - i - 1).Trim();
                    try
                    {
                        var inputStream = new AntlrInputStream(expr);
                        var lexer = new PilotLexer(inputStream);
                        var parser = new PilotParser(new CommonTokenStream(lexer));
                        parser.RemoveErrorListeners();
                        var exprTree = parser.test();
                        var evaluated = Visit(exprTree);
                        sb.Append(evaluated.ToString());
                    }
                    catch
                    {
                        sb.Append(expr);
                    }
                    i = endBrace + 1;
                    continue;
                }
            }
            sb.Append(content[i++]);
        }
        return PilotValue.FromString(sb.ToString());
    }

    private static string UnescapeString(string str)
    {
        if (str.Length >= 2 && ((str.StartsWith("\"") && str.EndsWith("\"")) || (str.StartsWith("'") && str.EndsWith("'"))))
        {
            str = str[1..^1];
        }
        return str.Replace("\\n", "\n").Replace("\\r", "\r").Replace("\\t", "\t").Replace("\\\"", "\"").Replace("\\\\", "\\");
    }

    public override PilotValue VisitCollection_expr(PilotParser.Collection_exprContext context)
    {
        var elements = context.collection_element();
        if (elements == null || elements.Length == 0)
        {
            return PilotValue.FromList(new List<PilotValue>());
        }

        var list = new List<PilotValue>(elements.Length);
        foreach (var el in elements)
        {
            var val = Visit(el.test());
            // Spread operator: ..sublist or *sublist
            if (el.DOTDOT() != null || el.STAR() != null)
            {
                if (val.Kind == PilotValueKind.List) list.AddRange(val.AsList());
                else if (val.Kind == PilotValueKind.Tuple) list.AddRange(val.AsTuple());
                else if (val.Kind == PilotValueKind.Set) list.AddRange(val.AsSet());
                else if (val.Kind == PilotValueKind.Range) list.AddRange(val.AsRange().Select(PilotValue.FromInt));
                else if (val.Kind == PilotValueKind.Generator) list.AddRange(val.AsGenerator());
                else list.Add(val);
            }
            else
            {
                list.Add(val);
            }
        }

        return PilotValue.FromList(list);
    }

    public override PilotValue VisitDictorsetmaker(PilotParser.DictorsetmakerContext context)
    {
        var dictEntries = context.dict_entry();
        if (dictEntries != null && dictEntries.Length > 0)
        {
            var map = new Dictionary<PilotValue, PilotValue>();
            foreach (var e in dictEntries)
            {
                if (e.DOTDOT() != null || e.DOUBLESTAR() != null)
                {
                    var spreadVal = Visit(e.test(0));
                    if (spreadVal.Kind == PilotValueKind.Map)
                    {
                        foreach (var kvp in spreadVal.AsMap())
                        {
                            map[kvp.Key] = kvp.Value;
                        }
                    }
                    else if (spreadVal.Kind == PilotValueKind.Object)
                    {
                        foreach (var kvp in spreadVal.AsObject().Fields)
                        {
                            map[PilotValue.FromString(kvp.Key)] = kvp.Value;
                        }
                    }
                }
                else
                {
                    var tests = e.test();
                    if (tests.Length >= 2)
                    {
                        map[Visit(tests[0])] = Visit(tests[1]);
                    }
                }
            }
            return PilotValue.FromMap(map);
        }

        var colElements = context.collection_element();
        if (colElements != null && colElements.Length > 0)
        {
            var set = new HashSet<PilotValue>();
            foreach (var el in colElements)
            {
                var val = Visit(el.test());
                if (el.DOTDOT() != null || el.STAR() != null)
                {
                    if (val.Kind == PilotValueKind.Set)
                    {
                        foreach (var item in val.AsSet()) set.Add(item);
                    }
                    else if (val.Kind == PilotValueKind.List)
                    {
                        foreach (var item in val.AsList()) set.Add(item);
                    }
                    else if (val.Kind == PilotValueKind.Tuple)
                    {
                        foreach (var item in val.AsTuple()) set.Add(item);
                    }
                    else if (val.Kind == PilotValueKind.Range)
                    {
                        foreach (var item in val.AsRange()) set.Add(PilotValue.FromInt(item));
                    }
                    else if (val.Kind == PilotValueKind.Generator)
                    {
                        foreach (var item in val.AsGenerator()) set.Add(item);
                    }
                    else
                    {
                        set.Add(val);
                    }
                }
                else
                {
                    set.Add(val);
                }
            }
            return PilotValue.FromSet(set);
        }

        return PilotValue.FromMap(new Dictionary<PilotValue, PilotValue>());
    }

    public override PilotValue VisitTestlist_comp(PilotParser.Testlist_compContext context)
    {
        var tests = context.test();
        if (tests == null || tests.Length == 0)
        {
            return PilotValue.FromTuple(Array.Empty<PilotValue>());
        }

        var items = new List<PilotValue>(tests.Length);
        foreach (var t in tests)
        {
            items.Add(Visit(t));
        }
        return items.Count == 1 ? items[0] : PilotValue.FromTuple(items);
    }

    public override PilotValue VisitInline_foreign_block(PilotParser.Inline_foreign_blockContext context)
    {
        string lang = context.PYTHON() != null ? "python" : context.CSHARP() != null ? "csharp" : "rust";
        string code = (context.RAW_DSL_BLOCK()?.GetText()) ??
                      (context.stmt() != null ? string.Join("\n", context.stmt().Select(s => s.GetText())) : string.Empty);

        if (Context.Options.Polyglot.TryGet(lang, out var provider))
        {
            return provider.ExecuteBlock(code, Context);
        }

        return PilotValue.FromString($"[{lang} block: {code}]");
    }

    #endregion
}
