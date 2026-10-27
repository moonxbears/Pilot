using System;
using System.Collections.Generic;
using System.Globalization;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Runtime.Bytecode;
using Pilot.Runtime.Values;

namespace Pilot.Compiler;

/// <summary>
/// Compiles Pilot ANTLR parse trees into optimized <see cref="PilotBytecodeChunk"/> binaries.
/// Translates AST statements, expressions, control flow, functions, and collections into
/// flat linear instruction streams with local slot registers and fast-path arithmetic.
/// </summary>
public sealed class PilotBytecodeCompiler : PilotBaseVisitor<object?>
{
    private readonly PilotBytecodeChunk _chunk;
    private readonly PilotBytecodeCompiler? _parent;
    private readonly Stack<LoopContext> _loops = new();
    private readonly bool _isFunction;
    private bool _discardExpressionResult = true;

    private sealed class LoopContext
    {
        public int LoopStartIp { get; }
        public List<int> BreakJumps { get; } = new();
        public List<int> ContinueJumps { get; } = new();

        public LoopContext(int loopStartIp)
        {
            LoopStartIp = loopStartIp;
        }
    }

    public PilotBytecodeCompiler(string name = "<main>", PilotBytecodeCompiler? parent = null, bool isFunction = false)
    {
        _chunk = new PilotBytecodeChunk(name);
        _parent = parent;
        _isFunction = isFunction;
    }

    /// <summary>
    /// Compiles a complete Pilot program AST into a bytecode chunk.
    /// </summary>
    public static PilotBytecodeChunk Compile(PilotParser.ProgramContext program, string? source = null)
    {
        var compiler = new PilotBytecodeCompiler("<program>");
        compiler.VisitProgram(program);
        compiler._chunk.Emit(PilotOpcode.Halt);
        return compiler._chunk;
    }

    /// <summary>
    /// Compiles a function suite into a bytecode chunk.
    /// </summary>
    public static PilotBytecodeChunk CompileSuite(
        PilotParser.SuiteContext suite,
        IReadOnlyList<string>? parameters = null,
        string name = "<func>",
        PilotBytecodeCompiler? parent = null)
    {
        var compiler = new PilotBytecodeCompiler(name, parent, isFunction: true);
        if (parameters != null)
        {
            foreach (var p in parameters)
            {
                compiler._chunk.ParameterNames.Add(p);
                compiler._chunk.GetOrAllocateLocalSlot(p);
            }
        }

        compiler.VisitSuite(suite);

        // Ensure every function path returns
        if (compiler._chunk.Instructions.Count == 0 ||
            (compiler._chunk.Instructions[^1].Opcode != PilotOpcode.Return &&
             compiler._chunk.Instructions[^1].Opcode != PilotOpcode.ReturnValue))
        {
            if (!compiler._discardExpressionResult)
            {
                compiler._chunk.Emit(PilotOpcode.ReturnValue);
            }
            else
            {
                compiler._chunk.Emit(PilotOpcode.Return);
            }
        }

        return compiler._chunk;
    }

    #region Programs & Statements

    public override object? VisitProgram(PilotParser.ProgramContext context)
    {
        var stmts = context.stmt();
        if (stmts != null && stmts.Length > 0)
        {
            for (int i = 0; i < stmts.Length; i++)
            {
                _discardExpressionResult = (i < stmts.Length - 1);
                Visit(stmts[i]);
            }
        }
        return null;
    }

    public override object? VisitStmt(PilotParser.StmtContext context)
    {
        if (context.compound_stmt() != null)
            return Visit(context.compound_stmt());
        if (context.simple_stmt() != null)
            return Visit(context.simple_stmt());
        return null;
    }

    public override object? VisitSimple_stmt(PilotParser.Simple_stmtContext context)
    {
        var smalls = context.small_stmt();
        if (smalls != null && smalls.Length > 0)
        {
            bool parentDiscard = _discardExpressionResult;
            for (int i = 0; i < smalls.Length; i++)
            {
                _discardExpressionResult = parentDiscard || (i < smalls.Length - 1);
                Visit(smalls[i]);
            }
        }
        return null;
    }

    public override object? VisitSmall_stmt(PilotParser.Small_stmtContext context)
    {
        if (context.expr_stmt() != null) return Visit(context.expr_stmt());
        if (context.let_stmt() != null) return Visit(context.let_stmt());
        if (context.var_stmt() != null) return Visit(context.var_stmt());
        if (context.const_stmt() != null) return Visit(context.const_stmt());
        if (context.flow_stmt() != null) return Visit(context.flow_stmt());
        return base.VisitSmall_stmt(context);
    }

    public override object? VisitCompound_stmt(PilotParser.Compound_stmtContext context)
    {
        if (context.if_stmt() != null) return Visit(context.if_stmt());
        if (context.while_stmt() != null) return Visit(context.while_stmt());
        if (context.for_stmt() != null) return Visit(context.for_stmt());
        if (context.func_def() != null) return Visit(context.func_def());
        return base.VisitCompound_stmt(context);
    }

    public override object? VisitSuite(PilotParser.SuiteContext context)
    {
        bool parentDiscard = _discardExpressionResult;
        if (context.simple_stmt() != null)
        {
            _discardExpressionResult = parentDiscard;
            Visit(context.simple_stmt());
            return null;
        }

        var stmts = context.stmt();
        if (stmts != null && stmts.Length > 0)
        {
            for (int i = 0; i < stmts.Length; i++)
            {
                _discardExpressionResult = parentDiscard || (i < stmts.Length - 1);
                Visit(stmts[i]);
            }
        }
        return null;
    }

    public override object? VisitLet_stmt(PilotParser.Let_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "";
        if (!string.IsNullOrEmpty(name))
        {
            CompileVariableDeclaration(name, context.test());
        }
        else
        {
            int fallback = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fallback);
            _chunk.Emit(PilotOpcode.Pop);
        }
        return null;
    }

    public override object? VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "";
        if (!string.IsNullOrEmpty(name))
        {
            CompileVariableDeclaration(name, context.test());
        }
        else
        {
            int fallback = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fallback);
            _chunk.Emit(PilotOpcode.Pop);
        }
        return null;
    }

    public override object? VisitConst_stmt(PilotParser.Const_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "";
        if (!string.IsNullOrEmpty(name))
        {
            CompileVariableDeclaration(name, context.test());
        }
        else
        {
            int fallback = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fallback);
            _chunk.Emit(PilotOpcode.Pop);
        }
        return null;
    }

    private void CompileVariableDeclaration(string name, PilotParser.TestContext? initExpr)
    {
        if (initExpr != null)
        {
            Visit(initExpr);
        }
        else
        {
            _chunk.Emit(PilotOpcode.ConstUndefined);
        }

        if (_isFunction)
        {
            int slot = _chunk.GetOrAllocateLocalSlot(name);
            _chunk.Emit(PilotOpcode.StoreLocal, slot);
        }
        else
        {
            int idIndex = _chunk.AddIdentifier(name);
            _chunk.Emit(PilotOpcode.DeclareNamed, idIndex);
        }
    }

    public override object? VisitExpr_stmt(PilotParser.Expr_stmtContext context)
    {
        var testList = context.testlist();
        if (testList == null) return null;

        var tests = testList.test();
        if (tests == null || tests.Length == 0) return null;

        // Simple assignment: target = expr
        if (context.EQUAL()?.Length > 0 && context.test()?.Length > 0)
        {
            Visit(context.test(0)); // Evaluate RHS
            AssignToTarget(tests[0]);
            return null;
        }

        // Augmented assignment: target += expr, etc.
        if (context.augassign()?.Length > 0 && context.test()?.Length > 0)
        {
            CompileAugmentedAssign(tests[0], context.test(0), context.augassign(0));
            return null;
        }

        // Standalone expression statement
        for (int i = 0; i < tests.Length; i++)
        {
            Visit(tests[i]);
            if (_discardExpressionResult || i < tests.Length - 1)
            {
                _chunk.Emit(PilotOpcode.Pop);
            }
        }
        return null;
    }

    private void AssignToTarget(PilotParser.TestContext targetNode)
    {
        // Simple identifier target: x = rhs
        if (IsSimpleIdentifier(targetNode, out string idName))
        {
            if (_isFunction && _chunk.TryGetLocalSlot(idName, out int targetSlot))
            {
                _chunk.Emit(PilotOpcode.StoreLocal, targetSlot);
            }
            else if (_isFunction)
            {
                int newSlot = _chunk.GetOrAllocateLocalSlot(idName);
                _chunk.Emit(PilotOpcode.StoreLocal, newSlot);
            }
            else
            {
                int idIndex = _chunk.AddIdentifier(idName);
                _chunk.Emit(PilotOpcode.StoreNamed, idIndex);
            }
            return;
        }

        // Member or index access assignment: a.b = rhs or a[i] = rhs
        var atomExpr = GetAtomExpr(targetNode);
        if (atomExpr != null && atomExpr.trailer()?.Length > 0)
        {
            var trailers = atomExpr.trailer();
            var lastTrailer = trailers[^1];

            if (lastTrailer.DOT() != null)
            {
                // receiver.member = rhs
                string member = lastTrailer.identifier().GetText();
                int memberId = _chunk.AddIdentifier(member);
                CompileAtomExprPrefix(atomExpr, trailers.Length - 1);
                _chunk.Emit(PilotOpcode.Swap);
                _chunk.Emit(PilotOpcode.SetMember, memberId);
                return;
            }

            if (lastTrailer.LSQB() != null)
            {
                // receiver[index] = rhs
                var subCtx = lastTrailer.subscriptlist()?.subscript(0);
                if (subCtx != null)
                {
                    CompileAtomExprPrefix(atomExpr, trailers.Length - 1);
                    Visit(subCtx.test(0));
                    int tempSlot = _chunk.GetOrAllocateLocalSlot("<temp_rhs>");
                    _chunk.Emit(PilotOpcode.StoreLocal, tempSlot);
                    _chunk.Emit(PilotOpcode.LoadLocal, tempSlot);
                    _chunk.Emit(PilotOpcode.SetIndex);
                    return;
                }
            }
        }

        // Fallback for complex pattern matching target
        int fallback = _chunk.AddAstFallback(targetNode);
        _chunk.Emit(PilotOpcode.EvalAst, fallback);
        _chunk.Emit(PilotOpcode.Pop);
    }

    private void CompileAugmentedAssign(PilotParser.TestContext targetNode, PilotParser.TestContext rhsNode, PilotParser.AugassignContext aug)
    {
        if (IsSimpleIdentifier(targetNode, out string idName))
        {
            if (_isFunction && _chunk.TryGetLocalSlot(idName, out int slot))
            {
                _chunk.Emit(PilotOpcode.LoadLocal, slot);
            }
            else
            {
                int idIndex = _chunk.AddIdentifier(idName);
                _chunk.Emit(PilotOpcode.LoadNamed, idIndex);
            }

            Visit(rhsNode);

            string op = aug.GetText();
            switch (op)
            {
                case "+=": _chunk.Emit(PilotOpcode.Add); break;
                case "-=": _chunk.Emit(PilotOpcode.Sub); break;
                case "*=": _chunk.Emit(PilotOpcode.Mul); break;
                case "/=": _chunk.Emit(PilotOpcode.Div); break;
                case "%=": _chunk.Emit(PilotOpcode.Mod); break;
                case "&=": _chunk.Emit(PilotOpcode.BitAnd); break;
                case "|=": _chunk.Emit(PilotOpcode.BitOr); break;
                case "^=": _chunk.Emit(PilotOpcode.BitXor); break;
                case "<<=": _chunk.Emit(PilotOpcode.ShiftLeft); break;
                case ">>=": _chunk.Emit(PilotOpcode.ShiftRight); break;
                default: _chunk.Emit(PilotOpcode.Add); break;
            }

            if (_isFunction && _chunk.TryGetLocalSlot(idName, out int storeSlot))
            {
                _chunk.Emit(PilotOpcode.StoreLocal, storeSlot);
            }
            else
            {
                int idIndex = _chunk.AddIdentifier(idName);
                _chunk.Emit(PilotOpcode.StoreNamed, idIndex);
            }
            return;
        }

        // Fallback
        int fallback = _chunk.AddAstFallback(targetNode);
        _chunk.Emit(PilotOpcode.EvalAst, fallback);
        _chunk.Emit(PilotOpcode.Pop);
    }

    public override object? VisitFlow_stmt(PilotParser.Flow_stmtContext context)
    {
        if (context.RETURN() != null)
        {
            var testlist = context.testlist();
            if (testlist?.test()?.Length > 0)
            {
                Visit(testlist.test(0));
                _chunk.Emit(PilotOpcode.ReturnValue);
            }
            else
            {
                _chunk.Emit(PilotOpcode.Return);
            }
            return null;
        }

        if (context.BREAK() != null)
        {
            if (_loops.Count > 0)
            {
                int jmp = _chunk.EmitJump(PilotOpcode.Jump);
                _loops.Peek().BreakJumps.Add(jmp);
            }
            return null;
        }

        if (context.CONTINUE() != null)
        {
            if (_loops.Count > 0)
            {
                int jmp = _chunk.EmitJump(PilotOpcode.Jump);
                _loops.Peek().ContinueJumps.Add(jmp);
            }
            return null;
        }

        // Fallback (e.g. raise, yield)
        int fallback = _chunk.AddAstFallback(context);
        _chunk.Emit(PilotOpcode.EvalAst, fallback);
        return null;
    }

    public override object? VisitIf_stmt(PilotParser.If_stmtContext context)
    {
        // if condition
        Visit(context.test(0));
        int falseJump = _chunk.EmitJump(PilotOpcode.JumpIfFalse);

        Visit(context.suite(0));

        var endJumps = new List<int>();
        int endJump = _chunk.EmitJump(PilotOpcode.Jump);
        endJumps.Add(endJump);

        _chunk.PatchJump(falseJump);

        // elif clauses
        var elifTests = context.ELIF();
        int suiteIndex = 1;
        if (elifTests?.Length > 0)
        {
            for (int i = 0; i < elifTests.Length; i++)
            {
                Visit(context.test(i + 1));
                int elifFalseJump = _chunk.EmitJump(PilotOpcode.JumpIfFalse);
                Visit(context.suite(suiteIndex++));
                endJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));
                _chunk.PatchJump(elifFalseJump);
            }
        }

        // else clause
        if (context.ELSE() != null && suiteIndex < context.suite().Length)
        {
            Visit(context.suite(suiteIndex));
        }

        // Patch all end jumps to point here
        foreach (var ej in endJumps)
        {
            _chunk.PatchJump(ej);
        }

        return null;
    }

    public override object? VisitWhile_stmt(PilotParser.While_stmtContext context)
    {
        int loopStart = _chunk.Instructions.Count;
        var loopCtx = new LoopContext(loopStart);
        _loops.Push(loopCtx);

        // Evaluate condition
        Visit(context.test(0));
        int exitJump = _chunk.EmitJump(PilotOpcode.JumpIfFalse);

        // Body
        bool prevDiscard = _discardExpressionResult;
        _discardExpressionResult = true;
        Visit(context.suite(0));
        _discardExpressionResult = prevDiscard;

        // Jump back to condition
        _chunk.Emit(PilotOpcode.Jump, loopStart);

        // Patch exit jump
        _chunk.PatchJump(exitJump);

        // Patch break & continue jumps
        foreach (var bj in loopCtx.BreakJumps)
        {
            _chunk.PatchJump(bj);
        }
        foreach (var cj in loopCtx.ContinueJumps)
        {
            _chunk.PatchJump(cj, loopStart);
        }

        _loops.Pop();
        return null;
    }

    public override object? VisitFor_stmt(PilotParser.For_stmtContext context)
    {
        // 1. Evaluate collection/range
        Visit(context.test(0));
        _chunk.Emit(PilotOpcode.GetIter);

        int iterSlot = _chunk.GetOrAllocateLocalSlot($"<iter_{_chunk.Instructions.Count}>");
        _chunk.Emit(PilotOpcode.StoreLocal, iterSlot);

        int loopStart = _chunk.Instructions.Count;
        var loopCtx = new LoopContext(loopStart);
        _loops.Push(loopCtx);

        _chunk.Emit(PilotOpcode.LoadLocal, iterSlot);
        int exitJump = _chunk.EmitJump(PilotOpcode.ForIter);

        // 2. Assign item to target
        var forTarget = context.for_target();
        string targetName = forTarget.GetText();
        if (_isFunction)
        {
            int slot = _chunk.GetOrAllocateLocalSlot(targetName);
            _chunk.Emit(PilotOpcode.StoreLocal, slot);
        }
        else
        {
            int idIndex = _chunk.AddIdentifier(targetName);
            _chunk.Emit(PilotOpcode.StoreNamed, idIndex);
        }

        // 3. Body suite
        bool prevDiscard = _discardExpressionResult;
        _discardExpressionResult = true;
        Visit(context.suite(0));
        _discardExpressionResult = prevDiscard;

        // 4. Jump back
        _chunk.Emit(PilotOpcode.Jump, loopStart);

        // 5. Patch exit and breaks
        _chunk.PatchJump(exitJump);
        foreach (var bj in loopCtx.BreakJumps)
        {
            _chunk.PatchJump(bj);
        }
        foreach (var cj in loopCtx.ContinueJumps)
        {
            _chunk.PatchJump(cj, loopStart);
        }

        _loops.Pop();
        return null;
    }

    public override object? VisitFunc_def(PilotParser.Func_defContext context)
    {
        string funcName = context.identifier().GetText();

        var paramNames = new List<string>();
        var parameters = context.parameters()?.parameter();
        if (parameters != null)
        {
            foreach (var p in parameters)
            {
                paramNames.Add(p.identifier()?.GetText() ?? "arg");
            }
        }

        var nestedChunk = CompileSuite(context.suite(), paramNames, funcName, this);
        int chunkIndex = _chunk.AddNestedChunk(nestedChunk);

        _chunk.Emit(PilotOpcode.MakeClosure, chunkIndex);

        if (_isFunction)
        {
            int slot = _chunk.GetOrAllocateLocalSlot(funcName);
            _chunk.Emit(PilotOpcode.StoreLocal, slot);
        }
        else
        {
            int idIndex = _chunk.AddIdentifier(funcName);
            _chunk.Emit(PilotOpcode.DeclareNamed, idIndex);
        }

        return null;
    }

    #endregion

    #region Expressions & Operators

    public override object? VisitTest(PilotParser.TestContext context)
    {
        var pipeExprs = context.pipe_expr();
        if (pipeExprs?.Length > 0)
        {
            if (context.IF() != null && pipeExprs.Length > 1 && context.test() != null)
            {
                // Ternary: val IF condition ELSE alt
                Visit(pipeExprs[1]); // condition
                int falseJump = _chunk.EmitJump(PilotOpcode.JumpIfFalse);
                Visit(pipeExprs[0]); // val
                int endJump = _chunk.EmitJump(PilotOpcode.Jump);
                _chunk.PatchJump(falseJump);
                Visit(context.test()); // alt
                _chunk.PatchJump(endJump);
                return null;
            }

            return Visit(pipeExprs[0]);
        }

        // Fallback
        int fallback = _chunk.AddAstFallback(context);
        _chunk.Emit(PilotOpcode.EvalAst, fallback);
        return null;
    }

    public override object? VisitPipe_expr(PilotParser.Pipe_exprContext context)
    {
        var queries = context.set_query_expr();
        if (queries == null || queries.Length == 0) return null;

        Visit(queries[0]);

        for (int i = 1; i < queries.Length; i++)
        {
            Visit(queries[i]);
            _chunk.Emit(PilotOpcode.Swap);
            _chunk.Emit(PilotOpcode.Call, 1);
        }

        return null;
    }

    public override object? VisitSet_query_expr(PilotParser.Set_query_exprContext context)
    {
        if (context.coalesce_expr()?.Length > 0)
        {
            return Visit(context.coalesce_expr(0));
        }
        return null;
    }

    public override object? VisitCoalesce_expr(PilotParser.Coalesce_exprContext context)
    {
        var orTests = context.or_test();
        if (orTests == null || orTests.Length == 0) return null;

        Visit(orTests[0]);

        for (int i = 1; i < orTests.Length; i++)
        {
            _chunk.Emit(PilotOpcode.Dup);
            int skipJump = _chunk.EmitJump(PilotOpcode.JumpIfNotNull);
            _chunk.Emit(PilotOpcode.Pop);
            Visit(orTests[i]);
            _chunk.PatchJump(skipJump);
        }

        return null;
    }

    public override object? VisitOr_test(PilotParser.Or_testContext context)
    {
        var andTests = context.and_test();
        if (andTests == null || andTests.Length == 0) return null;

        Visit(andTests[0]);

        for (int i = 1; i < andTests.Length; i++)
        {
            int skipJump = _chunk.EmitJump(PilotOpcode.JumpIfTrueOrPop);
            Visit(andTests[i]);
            _chunk.PatchJump(skipJump);
        }

        return null;
    }

    public override object? VisitAnd_test(PilotParser.And_testContext context)
    {
        var notTests = context.not_test();
        if (notTests == null || notTests.Length == 0) return null;

        Visit(notTests[0]);

        for (int i = 1; i < notTests.Length; i++)
        {
            int skipJump = _chunk.EmitJump(PilotOpcode.JumpIfFalseOrPop);
            Visit(notTests[i]);
            _chunk.PatchJump(skipJump);
        }

        return null;
    }

    public override object? VisitNot_test(PilotParser.Not_testContext context)
    {
        if (context.NOT() != null)
        {
            Visit(context.not_test());
            _chunk.Emit(PilotOpcode.Not);
            return null;
        }

        return Visit(context.comparison());
    }

    public override object? VisitComparison(PilotParser.ComparisonContext context)
    {
        var ranges = context.range_expr();
        if (ranges == null || ranges.Length == 0) return null;

        Visit(ranges[0]);

        var ops = context.comp_op();
        if (ops != null)
        {
            for (int i = 0; i < ops.Length; i++)
            {
                Visit(ranges[i + 1]);
                string op = ops[i].GetText();
                switch (op)
                {
                    case "==": _chunk.Emit(PilotOpcode.Eq); break;
                    case "!=": _chunk.Emit(PilotOpcode.Ne); break;
                    case "<": _chunk.Emit(PilotOpcode.Lt); break;
                    case "<=": _chunk.Emit(PilotOpcode.Le); break;
                    case ">": _chunk.Emit(PilotOpcode.Gt); break;
                    case ">=": _chunk.Emit(PilotOpcode.Ge); break;
                    case "is": _chunk.Emit(PilotOpcode.Identical); break;
                    case "is not": _chunk.Emit(PilotOpcode.NotIdentical); break;
                    default: _chunk.Emit(PilotOpcode.Eq); break;
                }
            }
        }

        return null;
    }

    public override object? VisitRange_expr(PilotParser.Range_exprContext context)
    {
        var bitorExprs = context.bitor_expr();
        if (bitorExprs == null || bitorExprs.Length == 0) return null;

        Visit(bitorExprs[0]);

        if (bitorExprs.Length > 1)
        {
            Visit(bitorExprs[1]);
            bool isExclusive = context.DOTDOT_EQUAL() == null;
            _chunk.Emit(PilotOpcode.BuildRange, isExclusive ? 1 : 0);
        }

        return null;
    }

    public override object? VisitBitor_expr(PilotParser.Bitor_exprContext context)
    {
        var xorExprs = context.bitxor_expr();
        if (xorExprs == null || xorExprs.Length == 0) return null;

        Visit(xorExprs[0]);
        for (int i = 1; i < xorExprs.Length; i++)
        {
            Visit(xorExprs[i]);
            _chunk.Emit(PilotOpcode.BitOr);
        }
        return null;
    }

    public override object? VisitBitxor_expr(PilotParser.Bitxor_exprContext context)
    {
        var andExprs = context.bitand_expr();
        if (andExprs == null || andExprs.Length == 0) return null;

        Visit(andExprs[0]);
        for (int i = 1; i < andExprs.Length; i++)
        {
            Visit(andExprs[i]);
            _chunk.Emit(PilotOpcode.BitXor);
        }
        return null;
    }

    public override object? VisitBitand_expr(PilotParser.Bitand_exprContext context)
    {
        var shiftExprs = context.shift_expr();
        if (shiftExprs == null || shiftExprs.Length == 0) return null;

        Visit(shiftExprs[0]);
        for (int i = 1; i < shiftExprs.Length; i++)
        {
            Visit(shiftExprs[i]);
            _chunk.Emit(PilotOpcode.BitAnd);
        }
        return null;
    }

    public override object? VisitShift_expr(PilotParser.Shift_exprContext context)
    {
        var arithExprs = context.arith_expr();
        if (arithExprs == null || arithExprs.Length == 0) return null;

        Visit(arithExprs[0]);
        for (int i = 1; i < arithExprs.Length; i++)
        {
            Visit(arithExprs[i]);
            var opToken = context.GetChild(2 * i - 1).GetText();
            _chunk.Emit(opToken == "<<" ? PilotOpcode.ShiftLeft : PilotOpcode.ShiftRight);
        }
        return null;
    }

    public override object? VisitArith_expr(PilotParser.Arith_exprContext context)
    {
        var terms = context.term();
        if (terms == null || terms.Length == 0) return null;

        Visit(terms[0]);
        for (int i = 1; i < terms.Length; i++)
        {
            Visit(terms[i]);
            var opToken = context.GetChild(2 * i - 1).GetText();
            _chunk.Emit(opToken == "+" ? PilotOpcode.Add : PilotOpcode.Sub);
        }
        return null;
    }

    public override object? VisitTerm(PilotParser.TermContext context)
    {
        var factors = context.factor();
        if (factors == null || factors.Length == 0) return null;

        Visit(factors[0]);
        for (int i = 1; i < factors.Length; i++)
        {
            Visit(factors[i]);
            var opToken = context.GetChild(2 * i - 1).GetText();
            switch (opToken)
            {
                case "*": _chunk.Emit(PilotOpcode.Mul); break;
                case "/": _chunk.Emit(PilotOpcode.Div); break;
                case "//": _chunk.Emit(PilotOpcode.FloorDiv); break;
                case "%": _chunk.Emit(PilotOpcode.Mod); break;
                default: _chunk.Emit(PilotOpcode.Mul); break;
            }
        }
        return null;
    }

    public override object? VisitFactor(PilotParser.FactorContext context)
    {
        if (context.unary_op() != null)
        {
            Visit(context.factor());
            string op = context.unary_op().GetText();
            switch (op)
            {
                case "-": _chunk.Emit(PilotOpcode.Neg); break;
                case "+": _chunk.Emit(PilotOpcode.Pos); break;
                case "~": _chunk.Emit(PilotOpcode.BitNot); break;
                default: break;
            }
            return null;
        }

        return Visit(context.power());
    }

    public override object? VisitPower(PilotParser.PowerContext context)
    {
        Visit(context.atom_expr());

        if (context.factor() != null)
        {
            Visit(context.factor());
            _chunk.Emit(PilotOpcode.Pow);
        }

        return null;
    }

    public override object? VisitAtom_expr(PilotParser.Atom_exprContext context)
    {
        Visit(context.atom());

        var trailers = context.trailer();
        if (trailers != null)
        {
            for (int i = 0; i < trailers.Length; i++)
            {
                var trailer = trailers[i];

                // Method / Function Call: (args)
                if (trailer.LPAR() != null)
                {
                    var args = trailer.arglist()?.argument();
                    int argCount = args?.Length ?? 0;
                    if (args != null)
                    {
                        for (int a = 0; a < args.Length; a++)
                        {
                            Visit(args[a].test());
                        }
                    }
                    _chunk.Emit(PilotOpcode.Call, argCount);
                }
                // Member Access: .member
                else if (trailer.DOT() != null)
                {
                    string member = trailer.identifier().GetText();
                    // Peek ahead: is next trailer a call?
                    if (i + 1 < trailers.Length && trailers[i + 1].LPAR() != null)
                    {
                        // Direct method call: receiver.method(args)
                        i++;
                        var callArgs = trailers[i].arglist()?.argument();
                        int argCount = callArgs?.Length ?? 0;
                        if (callArgs != null)
                        {
                            for (int a = 0; a < callArgs.Length; a++)
                            {
                                Visit(callArgs[a].test());
                            }
                        }
                        int memberId = _chunk.AddIdentifier(member);
                        _chunk.Emit(PilotOpcode.CallMethod, memberId, extra: argCount);
                    }
                    else
                    {
                        int memberId = _chunk.AddIdentifier(member);
                        _chunk.Emit(PilotOpcode.GetMember, memberId);
                    }
                }
                // Subscript Access: [index]
                else if (trailer.LSQB() != null)
                {
                    var subCtx = trailer.subscriptlist()?.subscript(0);
                    if (subCtx != null)
                    {
                        Visit(subCtx.test(0));
                        _chunk.Emit(PilotOpcode.GetIndex);
                    }
                }
            }
        }

        return null;
    }

    public override object? VisitAtom(PilotParser.AtomContext context)
    {
        if (context.identifier() != null)
        {
            string id = context.identifier().GetText();
            if (_isFunction && _chunk.TryGetLocalSlot(id, out int slot))
            {
                _chunk.Emit(PilotOpcode.LoadLocal, slot);
            }
            else
            {
                int idIndex = _chunk.AddIdentifier(id);
                _chunk.Emit(PilotOpcode.LoadNamed, idIndex);
            }
            return null;
        }

        if (context.literal() != null)
        {
            return Visit(context.literal());
        }

        if (context.collection_expr() != null)
        {
            return Visit(context.collection_expr());
        }

        if (context.LPAR() != null && context.testlist_comp() != null)
        {
            var tests = context.testlist_comp().test();
            int count = tests?.Length ?? 0;
            if (tests != null)
            {
                for (int i = 0; i < tests.Length; i++)
                {
                    Visit(tests[i]);
                }
            }
            if (count > 1)
            {
                _chunk.Emit(PilotOpcode.BuildTuple, count);
            }
            return null;
        }

        if (context.LBRACE() != null && context.dictorsetmaker() != null)
        {
            var maker = context.dictorsetmaker();
            var dictEntries = maker.dict_entry();
            if (dictEntries != null && dictEntries.Length > 0)
            {
                for (int i = 0; i < dictEntries.Length; i++)
                {
                    Visit(dictEntries[i].test(0)); // Key
                    Visit(dictEntries[i].test(1)); // Value
                }
                _chunk.Emit(PilotOpcode.BuildMap, dictEntries.Length);
                return null;
            }

            var colElements = maker.collection_element();
            if (colElements != null && colElements.Length > 0)
            {
                for (int i = 0; i < colElements.Length; i++)
                {
                    Visit(colElements[i].test());
                }
                _chunk.Emit(PilotOpcode.BuildSet, colElements.Length);
                return null;
            }
        }

        // Fallback
        int fallback = _chunk.AddAstFallback(context);
        _chunk.Emit(PilotOpcode.EvalAst, fallback);
        return null;
    }

    public override object? VisitLiteral(PilotParser.LiteralContext context)
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
                if (txt.StartsWith("0x", StringComparison.OrdinalIgnoreCase))
                    val = Convert.ToInt64(txt[2..], 16);
                else if (txt.StartsWith("0b", StringComparison.OrdinalIgnoreCase))
                    val = Convert.ToInt64(txt[2..], 2);
                else if (txt.StartsWith("0o", StringComparison.OrdinalIgnoreCase))
                    val = Convert.ToInt64(txt[2..], 8);
                else
                    val = long.Parse(txt, CultureInfo.InvariantCulture);
            }

            if (val >= int.MinValue && val <= int.MaxValue)
            {
                _chunk.Emit(PilotOpcode.ConstInt, (int)val);
            }
            else
            {
                int idx = _chunk.AddConstant(PilotValue.FromInt(val));
                _chunk.Emit(PilotOpcode.Const, idx);
            }
            return null;
        }

        if (context.FLOAT_NUMBER() != null)
        {
            string txt = context.FLOAT_NUMBER().GetText().Replace("_", "");
            double val = double.Parse(txt, CultureInfo.InvariantCulture);
            int idx = _chunk.AddConstant(PilotValue.FromFloat(val));
            _chunk.Emit(PilotOpcode.Const, idx);
            return null;
        }

        if (context.STRING()?.Length > 0)
        {
            string raw = context.STRING(0).GetText();
            string unescaped = StripQuotes(raw);
            int idx = _chunk.AddConstant(PilotValue.FromString(unescaped));
            _chunk.Emit(PilotOpcode.Const, idx);
            return null;
        }

        if (context.TRUE() != null)
        {
            _chunk.Emit(PilotOpcode.ConstTrue);
            return null;
        }

        if (context.FALSE() != null)
        {
            _chunk.Emit(PilotOpcode.ConstFalse);
            return null;
        }

        if (context.NULL() != null || context.NONE() != null)
        {
            _chunk.Emit(PilotOpcode.ConstNull);
            return null;
        }

        if (context.UNDEFINED() != null)
        {
            _chunk.Emit(PilotOpcode.ConstUndefined);
            return null;
        }

        // F-strings or complex literals fallback to AST
        int fallback = _chunk.AddAstFallback(context);
        _chunk.Emit(PilotOpcode.EvalAst, fallback);
        return null;
    }

    public override object? VisitCollection_expr(PilotParser.Collection_exprContext context)
    {
        var elements = context.collection_element();
        int count = elements?.Length ?? 0;
        if (elements != null)
        {
            for (int i = 0; i < elements.Length; i++)
            {
                Visit(elements[i].test());
            }
        }
        _chunk.Emit(PilotOpcode.BuildList, count);
        return null;
    }

    #endregion

    #region Helper Methods

    private static bool IsSimpleIdentifier(PilotParser.TestContext test, out string name)
    {
        name = string.Empty;
        var atomExpr = GetAtomExpr(test);
        if (atomExpr != null && (atomExpr.trailer() == null || atomExpr.trailer().Length == 0))
        {
            var idNode = atomExpr.atom()?.identifier();
            if (idNode != null)
            {
                name = idNode.GetText();
                return true;
            }
        }
        return false;
    }

    private static PilotParser.Atom_exprContext? GetAtomExpr(PilotParser.TestContext test)
    {
        return test?.pipe_expr(0)?.set_query_expr(0)?.coalesce_expr(0)?.or_test(0)?
            .and_test(0)?.not_test(0)?.comparison()?.range_expr(0)?.bitor_expr(0)?
            .bitxor_expr(0)?.bitand_expr(0)?.shift_expr(0)?.arith_expr(0)?.term(0)?
            .factor(0)?.power()?.atom_expr();
    }

    private void CompileAtomExprPrefix(PilotParser.Atom_exprContext atomExpr, int count)
    {
        Visit(atomExpr.atom());
        var trailers = atomExpr.trailer();
        for (int i = 0; i < count; i++)
        {
            var trailer = trailers[i];
            if (trailer.DOT() != null)
            {
                string member = trailer.identifier().GetText();
                int memberId = _chunk.AddIdentifier(member);
                _chunk.Emit(PilotOpcode.GetMember, memberId);
            }
            else if (trailer.LSQB() != null)
            {
                var subCtx = trailer.subscriptlist()?.subscript(0);
                if (subCtx != null)
                {
                    Visit(subCtx.test(0));
                    _chunk.Emit(PilotOpcode.GetIndex);
                }
            }
        }
    }

    private static string StripQuotes(string s)
    {
        if (string.IsNullOrEmpty(s)) return s;
        if ((s.StartsWith("\"\"\"") && s.EndsWith("\"\"\"")) || (s.StartsWith("'''") && s.EndsWith("'''")))
        {
            return s.Length >= 6 ? s[3..^3] : "";
        }
        if ((s.StartsWith("\"") && s.EndsWith("\"")) || (s.StartsWith("'") && s.EndsWith("'")))
        {
            return s.Length >= 2 ? s[1..^1] : "";
        }
        return s;
    }

    #endregion
}
