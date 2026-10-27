using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using Antlr4.Runtime;
using Pilot.Compiler;
using Pilot.Diagnostics;
using Pilot.Preprocessing;

namespace Pilot.Native.Llvm;

/// <summary>
/// Compiles Pilot AST suites into valid, SSA-form LLVM IR assembly (.ll).
/// Generates strongly-typed LLVM IR with stack allocations, hardware arithmetic,
/// structured control-flow basic blocks, function definitions, and C standard library bindings.
/// </summary>
public sealed class PilotLlvmEmitter : PilotBaseVisitor<string>
{
    private readonly StringBuilder _sb = new();
    private readonly Dictionary<string, (string LlvmType, string PtrRegister)> _locals = new(StringComparer.Ordinal);
    private readonly Dictionary<string, string> _globalStrings = new(StringComparer.Ordinal);
    private readonly List<string> _globalStringDefs = new();

    private int _regCounter = 0;
    private int _blockCounter = 0;
    private int _stringCounter = 0;
    private string _currentFunction = "";
    private bool _hasReturned = false;

    private readonly Stack<string> _loopCondBlocks = new();
    private readonly Stack<string> _loopExitBlocks = new();

    public string ModuleName { get; set; } = "pilot_module";
    public string TargetTriple { get; set; } = "x86_64-pc-windows-msvc";

    /// <summary>
    /// Emits LLVM IR assembly (.ll) from Pilot source code.
    /// </summary>
    public static string Emit(string source, string moduleName = "pilot_module")
    {
        var preproc = PilotPreprocessor.Preprocess(source, "");
        var inputStream = new AntlrInputStream(preproc.PreprocessedSource);
        var lexer = new PilotLexer(inputStream);
        var tokenStream = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokenStream);

        var errorTracker = new ErrorTracker();
        parser.RemoveErrorListeners();
        parser.AddErrorListener(errorTracker);

        var tree = parser.ParseProgramFast();
        return Emit(tree, moduleName);
    }

    /// <summary>
    /// Emits LLVM IR assembly (.ll) from a parsed Pilot program AST.
    /// </summary>
    public static string Emit(PilotParser.ProgramContext program, string moduleName = "pilot_module")
    {
        var emitter = new PilotLlvmEmitter { ModuleName = moduleName };
        return emitter.CompileProgram(program);
    }

    private string NextReg() => $"%{_regCounter++}";
    private string NextBlock(string prefix = "bb") => $"{prefix}.{_blockCounter++}";

    private string CompileProgram(PilotParser.ProgramContext context)
    {
        var functionsBody = new StringBuilder();
        var mainBody = new StringBuilder();

        // 1. Separate functions from top-level script statements
        var stmts = context.stmt();
        var topStmts = new List<PilotParser.StmtContext>();
        var funcDefs = new List<PilotParser.Func_defContext>();

        if (stmts != null)
        {
            foreach (var s in stmts)
            {
                var funcDef = s.compound_stmt()?.func_def();
                if (funcDef != null)
                {
                    funcDefs.Add(funcDef);
                }
                else
                {
                    topStmts.Add(s);
                }
            }
        }

        // 2. Emit function definitions
        foreach (var fd in funcDefs)
        {
            _locals.Clear();
            _regCounter = 0;
            _blockCounter = 0;
            _hasReturned = false;
            functionsBody.AppendLine(CompileFunction(fd));
            functionsBody.AppendLine();
        }

        // 3. Emit main function for top-level statements
        _locals.Clear();
        _regCounter = 0;
        _blockCounter = 0;
        _hasReturned = false;
        _currentFunction = "main";

        mainBody.AppendLine("define i32 @main(i32 %argc, ptr %argv) {");
        mainBody.AppendLine("entry:");

        foreach (var s in topStmts)
        {
            string code = Visit(s);
            if (!string.IsNullOrWhiteSpace(code))
            {
                mainBody.Append(code);
            }
        }

        if (!_hasReturned)
        {
            mainBody.AppendLine("    ret i32 0");
        }
        mainBody.AppendLine("}");

        // 4. Assemble final LLVM IR module
        var module = new StringBuilder();
        module.AppendLine($"; ModuleID = '{ModuleName}'");
        module.AppendLine($"source_filename = \"{ModuleName}.pilot\"");
        module.AppendLine($"target triple = \"{TargetTriple}\"");
        module.AppendLine();

        // Standard runtime declarations
        module.AppendLine("declare i32 @printf(ptr, ...)");
        module.AppendLine("declare i32 @puts(ptr)");
        module.AppendLine("declare ptr @malloc(i64)");
        module.AppendLine("declare void @free(ptr)");
        module.AppendLine("declare void @exit(i32)");
        module.AppendLine();

        // Standard format strings
        module.AppendLine("@.fmt_int = private unnamed_addr constant [6 x i8] c\"%lld\\0A\\00\", align 1");
        module.AppendLine("@.fmt_float = private unnamed_addr constant [4 x i8] c\"%g\\0A\\00\", align 1");
        module.AppendLine("@.fmt_str = private unnamed_addr constant [4 x i8] c\"%s\\0A\\00\", align 1");

        // Global string constants
        foreach (var strDef in _globalStringDefs)
        {
            module.AppendLine(strDef);
        }
        module.AppendLine();

        module.Append(functionsBody);
        module.Append(mainBody);

        return module.ToString();
    }

    private string CompileFunction(PilotParser.Func_defContext context)
    {
        string funcName = context.identifier()?.GetText() ?? "func";
        _currentFunction = funcName;

        var paramNames = new List<string>();
        var paramTypes = new List<string>();
        var parameters = context.parameters()?.parameter();

        if (parameters != null)
        {
            foreach (var p in parameters)
            {
                paramNames.Add(p.identifier()?.GetText() ?? "arg");
                paramTypes.Add(MapType(p.type_annotation()?.type_spec()?.GetText()));
            }
        }

        string retType = MapType(context.type_spec()?.GetText());

        var sb = new StringBuilder();
        var paramSig = string.Join(", ", paramNames.Select((p, idx) => $"{paramTypes[idx]} %arg.{p}"));
        sb.AppendLine($"define {retType} @{funcName}({paramSig}) {{");
        sb.AppendLine("entry:");

        // Alloca parameters and store
        for (int i = 0; i < paramNames.Count; i++)
        {
            string pName = paramNames[i];
            string pType = paramTypes[i];
            string ptr = $"%{pName}.addr";
            sb.AppendLine($"    {ptr} = alloca {pType}, align 8");
            sb.AppendLine($"    store {pType} %arg.{pName}, ptr {ptr}, align 8");
            _locals[pName] = (pType, ptr);
        }

        // Visit body suite
        var suite = context.suite();
        if (suite != null)
        {
            sb.Append(VisitSuite(suite));
        }

        if (!_hasReturned)
        {
            if (retType == "void")
            {
                sb.AppendLine("    ret void");
            }
            else
            {
                sb.AppendLine($"    ret {retType} 0");
            }
        }

        sb.AppendLine("}");
        return sb.ToString();
    }

    #region Statements

    public override string VisitStmt(PilotParser.StmtContext context)
    {
        if (context.compound_stmt() != null) return Visit(context.compound_stmt());
        if (context.simple_stmt() != null) return Visit(context.simple_stmt());
        return "";
    }

    public override string VisitSimple_stmt(PilotParser.Simple_stmtContext context)
    {
        var sb = new StringBuilder();
        var smalls = context.small_stmt();
        if (smalls != null)
        {
            foreach (var sm in smalls)
            {
                sb.Append(Visit(sm));
            }
        }
        return sb.ToString();
    }

    public override string VisitSmall_stmt(PilotParser.Small_stmtContext context)
    {
        if (context.expr_stmt() != null) return Visit(context.expr_stmt());
        if (context.let_stmt() != null) return Visit(context.let_stmt());
        if (context.var_stmt() != null) return Visit(context.var_stmt());
        if (context.const_stmt() != null) return Visit(context.const_stmt());
        if (context.flow_stmt() != null) return Visit(context.flow_stmt());
        return "";
    }

    public override string VisitLet_stmt(PilotParser.Let_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "v";
        string type = MapType(context.type_annotation()?.type_spec()?.GetText());
        return CompileVariableDecl(name, type, context.test());
    }

    public override string VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "v";
        string type = MapType(context.type_annotation()?.type_spec()?.GetText());
        return CompileVariableDecl(name, type, context.test());
    }

    public override string VisitConst_stmt(PilotParser.Const_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "v";
        string type = MapType(context.type_annotation()?.type_spec()?.GetText());
        return CompileVariableDecl(name, type, context.test());
    }

    private string CompileVariableDecl(string name, string type, PilotParser.TestContext? initExpr)
    {
        var sb = new StringBuilder();
        string ptr = $"%{name}.addr";

        if (!_locals.ContainsKey(name))
        {
            sb.AppendLine($"    {ptr} = alloca {type}, align 8");
            _locals[name] = (type, ptr);
        }

        if (initExpr != null)
        {
            var (valReg, valType, valCode) = CompileExpression(initExpr);
            sb.Append(valCode);
            if (valType != type && type == "i64" && valType == "i32")
            {
                string extReg = NextReg();
                sb.AppendLine($"    {extReg} = sext i32 {valReg} to i64");
                valReg = extReg;
            }
            sb.AppendLine($"    store {type} {valReg}, ptr {ptr}, align 8");
        }
        else
        {
            sb.AppendLine($"    store {type} 0, ptr {ptr}, align 8");
        }

        return sb.ToString();
    }

    public override string VisitExpr_stmt(PilotParser.Expr_stmtContext context)
    {
        var sb = new StringBuilder();
        var tests = context.testlist()?.test();
        if (tests == null || tests.Length == 0) return "";

        // Assignment: x = rhs
        if (context.EQUAL()?.Length > 0 && context.test()?.Length > 0)
        {
            string targetName = tests[0].GetText();
            var (rhsReg, rhsType, rhsCode) = CompileExpression(context.test(0));
            sb.Append(rhsCode);

            if (_locals.TryGetValue(targetName, out var loc))
            {
                sb.AppendLine($"    store {loc.LlvmType} {rhsReg}, ptr {loc.PtrRegister}, align 8");
            }
            else
            {
                string ptr = $"%{targetName}.addr";
                sb.AppendLine($"    {ptr} = alloca {rhsType}, align 8");
                sb.AppendLine($"    store {rhsType} {rhsReg}, ptr {ptr}, align 8");
                _locals[targetName] = (rhsType, ptr);
            }
            return sb.ToString();
        }

        // Augmented assignment: x += rhs
        if (context.augassign()?.Length > 0 && context.test()?.Length > 0)
        {
            string targetName = tests[0].GetText();
            if (_locals.TryGetValue(targetName, out var loc))
            {
                string loadReg = NextReg();
                sb.AppendLine($"    {loadReg} = load {loc.LlvmType}, ptr {loc.PtrRegister}, align 8");

                var (rhsReg, rhsType, rhsCode) = CompileExpression(context.test(0));
                sb.Append(rhsCode);

                string op = context.augassign(0).GetText();
                string inst = op switch
                {
                    "+=" => "add",
                    "-=" => "sub",
                    "*=" => "mul",
                    "/=" => "sdiv",
                    "%=" => "srem",
                    "&=" => "and",
                    "|=" => "or",
                    "^=" => "xor",
                    "<<=" => "shl",
                    ">>=" => "ashr",
                    _ => "add"
                };

                string resReg = NextReg();
                sb.AppendLine($"    {resReg} = {inst} {loc.LlvmType} {loadReg}, {rhsReg}");
                sb.AppendLine($"    store {loc.LlvmType} {resReg}, ptr {loc.PtrRegister}, align 8");
            }
            return sb.ToString();
        }

        // Standalone expression (e.g. print(...) or function call)
        foreach (var t in tests)
        {
            var (_, _, exprCode) = CompileExpression(t);
            sb.Append(exprCode);
        }

        return sb.ToString();
    }

    public override string VisitFlow_stmt(PilotParser.Flow_stmtContext context)
    {
        var sb = new StringBuilder();
        if (context.RETURN() != null)
        {
            var testlist = context.testlist();
            if (testlist?.test()?.Length > 0)
            {
                var (valReg, valType, valCode) = CompileExpression(testlist.test(0));
                sb.Append(valCode);
                sb.AppendLine($"    ret {valType} {valReg}");
            }
            else
            {
                sb.AppendLine("    ret void");
            }
            _hasReturned = true;
            return sb.ToString();
        }

        if (context.BREAK() != null && _loopExitBlocks.Count > 0)
        {
            sb.AppendLine($"    br label %{_loopExitBlocks.Peek()}");
            return sb.ToString();
        }

        if (context.CONTINUE() != null && _loopCondBlocks.Count > 0)
        {
            sb.AppendLine($"    br label %{_loopCondBlocks.Peek()}");
            return sb.ToString();
        }

        return "";
    }

    public override string VisitIf_stmt(PilotParser.If_stmtContext context)
    {
        var sb = new StringBuilder();
        string thenBlock = NextBlock("if.then");
        string elseBlock = NextBlock("if.else");
        string mergeBlock = NextBlock("if.end");

        // Condition
        var (condReg, condType, condCode) = CompileExpression(context.test(0));
        sb.Append(condCode);

        // Ensure i1 condition
        string i1Reg = EnsureI1(condReg, condType, sb);
        sb.AppendLine($"    br i1 {i1Reg}, label %{thenBlock}, label %{elseBlock}");

        // Then block
        sb.AppendLine($"{thenBlock}:");
        bool prevReturn = _hasReturned;
        _hasReturned = false;
        sb.Append(VisitSuite(context.suite(0)));
        if (!_hasReturned)
        {
            sb.AppendLine($"    br label %{mergeBlock}");
        }

        // Else block
        sb.AppendLine($"{elseBlock}:");
        _hasReturned = false;
        if (context.ELSE() != null && context.suite().Length > 1)
        {
            sb.Append(VisitSuite(context.suite(1)));
        }
        if (!_hasReturned)
        {
            sb.AppendLine($"    br label %{mergeBlock}");
        }

        sb.AppendLine($"{mergeBlock}:");
        _hasReturned = false;
        return sb.ToString();
    }

    public override string VisitWhile_stmt(PilotParser.While_stmtContext context)
    {
        var sb = new StringBuilder();
        string condBlock = NextBlock("while.cond");
        string bodyBlock = NextBlock("while.body");
        string endBlock = NextBlock("while.end");

        _loopCondBlocks.Push(condBlock);
        _loopExitBlocks.Push(endBlock);

        sb.AppendLine($"    br label %{condBlock}");
        sb.AppendLine($"{condBlock}:");

        var (condReg, condType, condCode) = CompileExpression(context.test(0));
        sb.Append(condCode);
        string i1Reg = EnsureI1(condReg, condType, sb);
        sb.AppendLine($"    br i1 {i1Reg}, label %{bodyBlock}, label %{endBlock}");

        sb.AppendLine($"{bodyBlock}:");
        _hasReturned = false;
        sb.Append(VisitSuite(context.suite(0)));
        if (!_hasReturned)
        {
            sb.AppendLine($"    br label %{condBlock}");
        }

        sb.AppendLine($"{endBlock}:");

        _loopCondBlocks.Pop();
        _loopExitBlocks.Pop();
        _hasReturned = false;

        return sb.ToString();
    }

    public override string VisitFor_stmt(PilotParser.For_stmtContext context)
    {
        var sb = new StringBuilder();
        string varName = context.for_target()?.GetText() ?? "i";
        string condBlock = NextBlock("for.cond");
        string bodyBlock = NextBlock("for.body");
        string stepBlock = NextBlock("for.step");
        string endBlock = NextBlock("for.end");

        _loopCondBlocks.Push(stepBlock);
        _loopExitBlocks.Push(endBlock);

        // Range expr: start..end
        var rangeExpr = context.test(0)?.pipe_expr(0)?.set_query_expr(0)?.coalesce_expr(0)?
            .or_test(0)?.and_test(0)?.not_test(0)?.comparison()?.range_expr(0);

        string startReg = "0";
        string endReg = "10";
        if (rangeExpr?.bitor_expr()?.Length > 1)
        {
            var (sReg, _, sCode) = CompileExpressionNode(rangeExpr.bitor_expr(0));
            var (eReg, _, eCode) = CompileExpressionNode(rangeExpr.bitor_expr(1));
            sb.Append(sCode);
            sb.Append(eCode);
            startReg = sReg;
            endReg = eReg;
        }

        string ptr = $"%{varName}.addr";
        sb.AppendLine($"    {ptr} = alloca i64, align 8");
        sb.AppendLine($"    store i64 {startReg}, ptr {ptr}, align 8");
        _locals[varName] = ("i64", ptr);

        sb.AppendLine($"    br label %{condBlock}");
        sb.AppendLine($"{condBlock}:");

        string curReg = NextReg();
        sb.AppendLine($"    {curReg} = load i64, ptr {ptr}, align 8");
        string cmpReg = NextReg();
        sb.AppendLine($"    {cmpReg} = icmp slt i64 {curReg}, {endReg}");
        sb.AppendLine($"    br i1 {cmpReg}, label %{bodyBlock}, label %{endBlock}");

        sb.AppendLine($"{bodyBlock}:");
        _hasReturned = false;
        sb.Append(VisitSuite(context.suite(0)));
        if (!_hasReturned)
        {
            sb.AppendLine($"    br label %{stepBlock}");
        }

        sb.AppendLine($"{stepBlock}:");
        string stepLoadReg = NextReg();
        sb.AppendLine($"    {stepLoadReg} = load i64, ptr {ptr}, align 8");
        string nextReg = NextReg();
        sb.AppendLine($"    {nextReg} = add i64 {stepLoadReg}, 1");
        sb.AppendLine($"    store i64 {nextReg}, ptr {ptr}, align 8");
        sb.AppendLine($"    br label %{condBlock}");

        sb.AppendLine($"{endBlock}:");

        _loopCondBlocks.Pop();
        _loopExitBlocks.Pop();
        _hasReturned = false;

        return sb.ToString();
    }

    public override string VisitSuite(PilotParser.SuiteContext context)
    {
        var sb = new StringBuilder();
        if (context.simple_stmt() != null)
        {
            sb.Append(Visit(context.simple_stmt()));
            return sb.ToString();
        }

        var stmts = context.stmt();
        if (stmts != null)
        {
            foreach (var s in stmts)
            {
                sb.Append(Visit(s));
            }
        }
        return sb.ToString();
    }

    #endregion

    #region Expressions & Calls

    private (string Reg, string Type, string Code) CompileExpression(PilotParser.TestContext context)
    {
        var sb = new StringBuilder();
        var pipe = context.pipe_expr(0);
        if (pipe != null)
        {
            return CompilePipeExpr(pipe);
        }
        return ("0", "i64", "");
    }

    private (string Reg, string Type, string Code) CompileExpressionNode(Antlr4.Runtime.Tree.IParseTree node)
    {
        if (node is PilotParser.TestContext tc) return CompileExpression(tc);
        if (node is PilotParser.Bitor_exprContext bc) return CompileBitorExpr(bc);
        if (node is PilotParser.Arith_exprContext ac) return CompileArithExpr(ac);
        return ("0", "i64", "");
    }

    private (string Reg, string Type, string Code) CompilePipeExpr(PilotParser.Pipe_exprContext context)
    {
        var query = context.set_query_expr(0)?.coalesce_expr(0)?.or_test(0)?.and_test(0)?.not_test(0);
        if (query?.comparison() != null)
        {
            return CompileComparison(query.comparison());
        }
        return ("0", "i64", "");
    }

    private (string Reg, string Type, string Code) CompileComparison(PilotParser.ComparisonContext context)
    {
        var sb = new StringBuilder();
        var ranges = context.range_expr();
        if (ranges == null || ranges.Length == 0) return ("0", "i64", "");

        var (leftReg, leftType, leftCode) = CompileRangeExpr(ranges[0]);
        sb.Append(leftCode);

        var ops = context.comp_op();
        if (ops != null && ops.Length > 0 && ranges.Length > 1)
        {
            var (rightReg, rightType, rightCode) = CompileRangeExpr(ranges[1]);
            sb.Append(rightCode);

            string op = ops[0].GetText();
            string predicate = op switch
            {
                "==" => "eq",
                "!=" => "ne",
                "<" => "slt",
                "<=" => "sle",
                ">" => "sgt",
                ">=" => "sge",
                _ => "eq"
            };

            string resReg = NextReg();
            sb.AppendLine($"    {resReg} = icmp {predicate} {leftType} {leftReg}, {rightReg}");
            return (resReg, "i1", sb.ToString());
        }

        return (leftReg, leftType, sb.ToString());
    }

    private (string Reg, string Type, string Code) CompileRangeExpr(PilotParser.Range_exprContext context)
    {
        var bitor = context.bitor_expr(0);
        return CompileBitorExpr(bitor);
    }

    private (string Reg, string Type, string Code) CompileBitorExpr(PilotParser.Bitor_exprContext context)
    {
        var sb = new StringBuilder();
        var xors = context.bitxor_expr();
        if (xors == null || xors.Length == 0) return ("0", "i64", "");

        var (leftReg, leftType, leftCode) = CompileBitandExpr(xors[0].bitand_expr(0));
        sb.Append(leftCode);

        for (int i = 1; i < xors.Length; i++)
        {
            var (rhsReg, rhsType, rhsCode) = CompileBitandExpr(xors[i].bitand_expr(0));
            sb.Append(rhsCode);
            string resReg = NextReg();
            sb.AppendLine($"    {resReg} = or {leftType} {leftReg}, {rhsReg}");
            leftReg = resReg;
        }

        return (leftReg, leftType, sb.ToString());
    }

    private (string Reg, string Type, string Code) CompileBitandExpr(PilotParser.Bitand_exprContext context)
    {
        var sb = new StringBuilder();
        var shifts = context.shift_expr();
        if (shifts == null || shifts.Length == 0) return ("0", "i64", "");

        var (leftReg, leftType, leftCode) = CompileArithExpr(shifts[0].arith_expr(0));
        sb.Append(leftCode);

        for (int i = 1; i < shifts.Length; i++)
        {
            var (rhsReg, rhsType, rhsCode) = CompileArithExpr(shifts[i].arith_expr(0));
            sb.Append(rhsCode);
            string resReg = NextReg();
            sb.AppendLine($"    {resReg} = and {leftType} {leftReg}, {rhsReg}");
            leftReg = resReg;
        }

        return (leftReg, leftType, sb.ToString());
    }

    private (string Reg, string Type, string Code) CompileArithExpr(PilotParser.Arith_exprContext context)
    {
        var sb = new StringBuilder();
        var terms = context.term();
        if (terms == null || terms.Length == 0) return ("0", "i64", "");

        var (leftReg, leftType, leftCode) = CompileTerm(terms[0]);
        sb.Append(leftCode);

        for (int i = 1; i < terms.Length; i++)
        {
            var (rhsReg, rhsType, rhsCode) = CompileTerm(terms[i]);
            sb.Append(rhsCode);
            string op = context.GetChild(2 * i - 1).GetText();
            string inst = op == "+" ? "add" : "sub";
            string resReg = NextReg();
            sb.AppendLine($"    {resReg} = {inst} {leftType} {leftReg}, {rhsReg}");
            leftReg = resReg;
        }

        return (leftReg, leftType, sb.ToString());
    }

    private (string Reg, string Type, string Code) CompileTerm(PilotParser.TermContext context)
    {
        var sb = new StringBuilder();
        var factors = context.factor();
        if (factors == null || factors.Length == 0) return ("0", "i64", "");

        var (leftReg, leftType, leftCode) = CompileFactor(factors[0]);
        sb.Append(leftCode);

        for (int i = 1; i < factors.Length; i++)
        {
            var (rhsReg, rhsType, rhsCode) = CompileFactor(factors[i]);
            sb.Append(rhsCode);
            string op = context.GetChild(2 * i - 1).GetText();
            string inst = op switch
            {
                "*" => "mul",
                "/" => "sdiv",
                "//" => "sdiv",
                "%" => "srem",
                _ => "mul"
            };
            string resReg = NextReg();
            sb.AppendLine($"    {resReg} = {inst} {leftType} {leftReg}, {rhsReg}");
            leftReg = resReg;
        }

        return (leftReg, leftType, sb.ToString());
    }

    private (string Reg, string Type, string Code) CompileFactor(PilotParser.FactorContext context)
    {
        var sb = new StringBuilder();
        if (context.unary_op() != null)
        {
            var (innerReg, innerType, innerCode) = CompileFactor(context.factor());
            sb.Append(innerCode);
            string op = context.unary_op().GetText();
            if (op == "-")
            {
                string resReg = NextReg();
                sb.AppendLine($"    {resReg} = sub {innerType} 0, {innerReg}");
                return (resReg, innerType, sb.ToString());
            }
            return (innerReg, innerType, sb.ToString());
        }

        return CompilePower(context.power());
    }

    private (string Reg, string Type, string Code) CompilePower(PilotParser.PowerContext context)
    {
        return CompileAtomExpr(context.atom_expr());
    }

    private (string Reg, string Type, string Code) CompileAtomExpr(PilotParser.Atom_exprContext context)
    {
        var sb = new StringBuilder();
        var atom = context.atom();
        var trailers = context.trailer();

        // Check if this is a function call: identifier(args...)
        if (atom.identifier() != null && trailers?.Length > 0 && trailers[0].LPAR() != null)
        {
            string funcName = atom.identifier().GetText();
            var argList = trailers[0].arglist()?.argument();

            // Special built-in: print(...)
            if (funcName is "print" or "println")
            {
                if (argList != null && argList.Length > 0)
                {
                    var (argReg, argType, argCode) = CompileExpression(argList[0].test());
                    sb.Append(argCode);

                    if (argType == "ptr")
                    {
                        sb.AppendLine($"    call i32 (ptr, ...) @printf(ptr @.fmt_str, ptr {argReg})");
                    }
                    else if (argType == "double")
                    {
                        sb.AppendLine($"    call i32 (ptr, ...) @printf(ptr @.fmt_float, double {argReg})");
                    }
                    else
                    {
                        sb.AppendLine($"    call i32 (ptr, ...) @printf(ptr @.fmt_int, i64 {argReg})");
                    }
                }
                return ("0", "i32", sb.ToString());
            }

            // General function call
            var compiledArgs = new List<(string Reg, string Type)>();
            if (argList != null)
            {
                foreach (var a in argList)
                {
                    var (aReg, aType, aCode) = CompileExpression(a.test());
                    sb.Append(aCode);
                    compiledArgs.Add((aReg, aType));
                }
            }

            string callArgs = string.Join(", ", compiledArgs.Select(a => $"{a.Type} {a.Reg}"));
            string retReg = NextReg();
            sb.AppendLine($"    {retReg} = call i64 @{funcName}({callArgs})");
            return (retReg, "i64", sb.ToString());
        }

        // Parenthesized expression: (expr)
        if (atom.LPAR() != null && atom.testlist_comp()?.test()?.Length > 0)
        {
            return CompileExpression(atom.testlist_comp().test(0));
        }

        // Identifier lookup
        if (atom.identifier() != null)
        {
            string id = atom.identifier().GetText();
            if (_locals.TryGetValue(id, out var loc))
            {
                string loadReg = NextReg();
                sb.AppendLine($"    {loadReg} = load {loc.LlvmType}, ptr {loc.PtrRegister}, align 8");
                return (loadReg, loc.LlvmType, sb.ToString());
            }
            return ("0", "i64", "");
        }

        // Literals
        if (atom.literal() != null)
        {
            return CompileLiteral(atom.literal());
        }

        return ("0", "i64", "");
    }

    private (string Reg, string Type, string Code) CompileLiteral(PilotParser.LiteralContext context)
    {
        if (context.INTEGER() != null || context.HEX_INTEGER() != null || context.BIN_INTEGER() != null || context.OCT_INTEGER() != null)
        {
            long val;
            if (context.HEX_INTEGER() != null)
            {
                string txt = context.HEX_INTEGER().GetText().Replace("_", "");
                val = Convert.ToInt64(txt[2..], 16);
            }
            else if (context.BIN_INTEGER() != null)
            {
                string txt = context.BIN_INTEGER().GetText().Replace("_", "");
                val = Convert.ToInt64(txt[2..], 2);
            }
            else if (context.OCT_INTEGER() != null)
            {
                string txt = context.OCT_INTEGER().GetText().Replace("_", "");
                val = Convert.ToInt64(txt[2..], 8);
            }
            else
            {
                string txt = context.INTEGER().GetText().Replace("_", "");
                val = long.Parse(txt, CultureInfo.InvariantCulture);
            }

            return (val.ToString(CultureInfo.InvariantCulture), "i64", "");
        }

        if (context.FLOAT_NUMBER() != null)
        {
            string txt = context.FLOAT_NUMBER().GetText().Replace("_", "");
            double val = double.Parse(txt, CultureInfo.InvariantCulture);
            return (val.ToString("G", CultureInfo.InvariantCulture), "double", "");
        }

        if (context.TRUE() != null) return ("1", "i1", "");
        if (context.FALSE() != null) return ("0", "i1", "");

        if (context.STRING()?.Length > 0)
        {
            string raw = context.STRING(0).GetText();
            string strVal = StripQuotes(raw);
            string globalName = GetOrAddGlobalString(strVal);
            return (globalName, "ptr", "");
        }

        return ("0", "i64", "");
    }

    private string GetOrAddGlobalString(string s)
    {
        if (_globalStrings.TryGetValue(s, out var existing))
        {
            return existing;
        }

        string globalName = $"@.str.{_stringCounter++}";
        _globalStrings[s] = globalName;

        var escapedBytes = Encoding.UTF8.GetBytes(s);
        int len = escapedBytes.Length + 1; // null terminator
        var sb = new StringBuilder();
        sb.Append($"c\"");
        foreach (byte b in escapedBytes)
        {
            if (b is >= 32 and <= 126 and not 34 and not 92)
            {
                sb.Append((char)b);
            }
            else
            {
                sb.Append($"\\{b:X2}");
            }
        }
        sb.Append("\\00\"");

        _globalStringDefs.Add($"{globalName} = private unnamed_addr constant [{len} x i8] {sb}, align 1");
        return globalName;
    }

    private string EnsureI1(string reg, string type, StringBuilder sb)
    {
        if (type == "i1") return reg;
        string i1Reg = NextReg();
        sb.AppendLine($"    {i1Reg} = icmp ne {type} {reg}, 0");
        return i1Reg;
    }

    private static string MapType(string? pilotType)
    {
        if (string.IsNullOrWhiteSpace(pilotType)) return "i64";
        return pilotType.Trim().ToLowerInvariant() switch
        {
            "int" or "i64" or "long" => "i64",
            "i32" => "i32",
            "i16" => "i16",
            "i8" or "byte" => "i8",
            "float" or "f64" or "double" => "double",
            "f32" => "float",
            "bool" or "boolean" => "i1",
            "str" or "string" => "ptr",
            "void" or "nil" => "void",
            _ => "i64"
        };
    }

    private static string StripQuotes(string s)
    {
        if (string.IsNullOrEmpty(s)) return s;
        if ((s.StartsWith("\"\"\"") && s.EndsWith("\"\"\"")) || (s.StartsWith("'''") && s.EndsWith("'''")))
            return s.Length >= 6 ? s[3..^3] : "";
        if ((s.StartsWith("\"") && s.EndsWith("\"")) || (s.StartsWith("'") && s.EndsWith("'")))
            return s.Length >= 2 ? s[1..^1] : "";
        return s;
    }

    #endregion
}
