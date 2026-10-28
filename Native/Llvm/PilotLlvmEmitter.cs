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
using Pilot.Runtime.Values;

namespace Pilot.Native.Llvm;

/// <summary>
/// Compiles Pilot AST suites into valid, SSA-form LLVM IR assembly (.ll).
/// Generates strongly-typed LLVM IR with stack allocations, compact unboxed ADT tagged unions,
/// O(1) switch jump tables for pattern matching, hardware arithmetic, structured control-flow
/// basic blocks, function definitions, and C standard library bindings.
/// </summary>
public sealed class PilotLlvmEmitter : PilotBaseVisitor<string>
{
    private readonly StringBuilder _sb = new();
    private readonly Dictionary<string, (string LlvmType, string PtrRegister)> _locals = new(StringComparer.Ordinal);
    private readonly Dictionary<string, string> _globalStrings = new(StringComparer.Ordinal);
    private readonly List<string> _globalStringDefs = new();

    private readonly Dictionary<string, PilotAdtDefinition> _knownAdts = new(StringComparer.Ordinal);
    private readonly Dictionary<string, PilotAdtVariantDefinition> _knownVariants = new(StringComparer.Ordinal);
    private readonly Dictionary<string, PilotRecordDefinition> _knownRecords = new(StringComparer.Ordinal);
    private readonly Dictionary<string, string> _functionReturnTypes = new(StringComparer.Ordinal);
    private readonly List<string> _llvmTypeDefs = new();

    private int _regCounter = 0;
    private int _blockCounter = 0;
    private int _stringCounter = 0;
    private string _currentFunction = "";
    private string _currentFunctionRetType = "i64";
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

        // 0. Pre-scan ADTs, Records, and function signatures to emit compact unboxed LLVM struct types
        PrescanTypesAndFunctions(context);

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
        _currentFunctionRetType = "i32";

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

        if (_llvmTypeDefs.Count > 0)
        {
            module.AppendLine("; --- Algebraic Data Types & Unboxed Value Records ---");
            foreach (var td in _llvmTypeDefs)
            {
                module.AppendLine(td);
            }
            module.AppendLine();
        }

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

    private void PrescanTypesAndFunctions(PilotParser.ProgramContext program)
    {
        var stmts = program.stmt();
        if (stmts == null) return;

        foreach (var stmt in stmts)
        {
            if (stmt.compound_stmt() is { } comp)
            {
                if (comp.record_def() is { } recCtx)
                {
                    string name = recCtx.identifier().GetText();
                    var gps = ExtractGenericNames(recCtx.generic_params());
                    var fields = ExtractAdtFields(recCtx.adt_field_list(), recCtx.struct_member());
                    _knownRecords[name] = new PilotRecordDefinition(name, fields, gps);
                }
                else if (comp.enum_def() is { } enumCtx)
                {
                    string name = enumCtx.identifier().GetText();
                    var gps = ExtractGenericNames(enumCtx.generic_params());
                    var adt = new PilotAdtDefinition(name, gps);
                    if (enumCtx.enum_suite()?.enum_member() != null)
                    {
                        foreach (var m in enumCtx.enum_suite().enum_member())
                        {
                            var v = adt.AddVariant(m.identifier().GetText(), ExtractAdtFields(m.adt_field_list(), m.struct_member()));
                            _knownVariants[v.Name] = v;
                        }
                    }
                    _knownAdts[name] = adt;
                }
            }
            else if (stmt.simple_stmt()?.small_stmt() is { } smalls)
            {
                foreach (var sm in smalls)
                {
                    if (sm.type_alias_stmt() is { } ta && ta.adt_sum_body() != null)
                    {
                        string name = ta.identifier().GetText();
                        var gps = ExtractGenericNames(ta.generic_params());
                        var adt = new PilotAdtDefinition(name, gps);
                        var body = ta.adt_sum_body();
                        for (int i = 0; i < body.ChildCount; i++)
                        {
                            var ch = body.GetChild(i);
                            if (ch is PilotParser.IdentifierContext uid)
                            {
                                var v = adt.AddVariant(uid.GetText(), Array.Empty<PilotAdtFieldDefinition>());
                                _knownVariants[v.Name] = v;
                            }
                            else if (ch is PilotParser.Adt_variantContext vc)
                            {
                                var v = adt.AddVariant(vc.identifier().GetText(), ExtractAdtFields(vc.adt_field_list(), vc.struct_member()));
                                _knownVariants[v.Name] = v;
                            }
                            else if (ch is PilotParser.Adt_variant_payloadContext vpc)
                            {
                                var v = adt.AddVariant(vpc.identifier().GetText(), ExtractAdtFields(vpc.adt_field_list(), vpc.struct_member()));
                                _knownVariants[v.Name] = v;
                            }
                        }
                        _knownAdts[name] = adt;
                    }
                }
            }
        }

        // Compute exact memory layouts and emit LLVM identified struct types
        foreach (var rec in _knownRecords.Values)
        {
            rec.Layout = PilotAdtMemoryLayout.ComputeForRecord(rec, _knownAdts, _knownRecords);
            var vLayout = rec.Layout.Variants[0];
            if (vLayout.Fields.Count == 0)
            {
                _llvmTypeDefs.Add($"%{rec.Name} = type {{ i8 }}");
            }
            else
            {
                string fieldList = string.Join(", ", vLayout.Fields.Select(f =>
                    f.IsBoxedIndirection ? "ptr" : MapType(f.TypeName, rec.GenericParameters)));
                _llvmTypeDefs.Add($"%{rec.Name} = type {{ {fieldList} }}");
            }
        }

        foreach (var adt in _knownAdts.Values)
        {
            adt.Layout = PilotAdtMemoryLayout.ComputeForAdt(adt, _knownAdts, _knownRecords);
            string tagLlvm = adt.Layout.TagBytes <= 1 ? "i8" : "i16";
            if (adt.Layout.PayloadSizeBytes == 0)
            {
                _llvmTypeDefs.Add($"%{adt.Name} = type {{ {tagLlvm} }}");
            }
            else
            {
                string payloadCarrier;
                if (adt.Layout.AlignmentBytes >= 8)
                {
                    int words = (adt.Layout.PayloadSizeBytes + 7) / 8;
                    payloadCarrier = $"[{words} x i64]";
                }
                else if (adt.Layout.AlignmentBytes >= 4)
                {
                    int dwords = (adt.Layout.PayloadSizeBytes + 3) / 4;
                    payloadCarrier = $"[{dwords} x i32]";
                }
                else
                {
                    payloadCarrier = $"[{adt.Layout.PayloadSizeBytes} x i8]";
                }

                _llvmTypeDefs.Add($"; ADT {adt.Name}: tag={tagLlvm}, payload={adt.Layout.PayloadSizeBytes}B, total={adt.Layout.TotalSizeBytes}B, recursive={adt.Layout.IsRecursive.ToString().ToLowerInvariant()}");
                _llvmTypeDefs.Add($"%{adt.Name} = type {{ {tagLlvm}, {payloadCarrier} }}");

                foreach (var vLayout in adt.Layout.Variants)
                {
                    if (vLayout.Fields.Count == 0) continue;
                    string vFields = string.Join(", ", vLayout.Fields.Select(f =>
                        f.IsBoxedIndirection ? "ptr" : MapType(f.TypeName, adt.GenericParameters)));
                    _llvmTypeDefs.Add($"%{adt.Name}.{vLayout.VariantName} = type {{ {vFields} }}");
                }
            }
        }

        // Record function return types for call sites
        foreach (var stmt in stmts)
        {
            if (stmt.compound_stmt()?.func_def() is { } fd)
            {
                string fnName = fd.identifier()?.GetText() ?? "func";
                _functionReturnTypes[fnName] = MapType(fd.type_spec()?.GetText());
            }
        }
    }

    private static IReadOnlyList<string> ExtractGenericNames(PilotParser.Generic_paramsContext? gp) =>
        gp == null ? Array.Empty<string>() : gp.generic_param().Select(p => p.identifier().GetText()).ToArray();

    private static List<PilotAdtFieldDefinition> ExtractAdtFields(
        PilotParser.Adt_field_listContext? fieldList,
        PilotParser.Struct_memberContext[]? structMembers)
    {
        var list = new List<PilotAdtFieldDefinition>();
        int idx = 0;
        if (fieldList?.adt_field() != null)
        {
            foreach (var f in fieldList.adt_field())
            {
                list.Add(new PilotAdtFieldDefinition(f.identifier()?.GetText(), f.type_spec()?.GetText() ?? "i64", idx++, f.test()));
            }
        }
        if (structMembers != null)
        {
            foreach (var sm in structMembers)
            {
                list.Add(new PilotAdtFieldDefinition(sm.identifier()?.GetText(), sm.type_annotation()?.type_spec()?.GetText() ?? "i64", idx++, sm.test()));
            }
        }
        return list;
    }

    private string CompileFunction(PilotParser.Func_defContext context)
    {
        string funcName = context.identifier()?.GetText() ?? "func";
        _currentFunction = funcName;
        var genericParams = ExtractGenericNames(context.generic_params());

        var paramNames = new List<string>();
        var paramTypes = new List<string>();
        var parameters = context.parameters()?.parameter();

        if (parameters != null)
        {
            foreach (var p in parameters)
            {
                paramNames.Add(p.identifier()?.GetText() ?? "arg");
                paramTypes.Add(MapType(p.type_annotation()?.type_spec()?.GetText(), genericParams));
            }
        }

        string retType = MapType(context.type_spec()?.GetText(), genericParams);
        _currentFunctionRetType = retType;

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
                sb.AppendLine($"    ret {retType} {DefaultZeroLiteral(retType)}");
            }
        }

        sb.AppendLine("}");
        return sb.ToString();
    }

    private static string DefaultZeroLiteral(string llvmType) => llvmType switch
    {
        "double" or "float" => "0.0",
        "ptr" => "null",
        _ when llvmType.StartsWith("%", StringComparison.Ordinal) || llvmType.StartsWith("{", StringComparison.Ordinal) => "zeroinitializer",
        _ => "0"
    };

    #region Statements

    public override string VisitStmt(PilotParser.StmtContext context)
    {
        if (context.compound_stmt() != null) return Visit(context.compound_stmt());
        if (context.simple_stmt() != null) return Visit(context.simple_stmt());
        return "";
    }

    public override string VisitCompound_stmt(PilotParser.Compound_stmtContext context)
    {
        if (context.if_stmt() != null) return Visit(context.if_stmt());
        if (context.while_stmt() != null) return Visit(context.while_stmt());
        if (context.for_stmt() != null) return Visit(context.for_stmt());
        if (context.match_stmt() != null) return VisitMatch_stmt(context.match_stmt());
        // Type declarations (record_def, enum_def) are emitted in the module header
        if (context.record_def() != null || context.enum_def() != null) return "";
        return base.VisitCompound_stmt(context) ?? "";
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
        string? explicitType = context.type_annotation()?.type_spec()?.GetText();
        return CompileVariableDecl(name, explicitType, context.test());
    }

    public override string VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "v";
        string? explicitType = context.type_annotation()?.type_spec()?.GetText();
        return CompileVariableDecl(name, explicitType, context.test());
    }

    public override string VisitConst_stmt(PilotParser.Const_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "v";
        string? explicitType = context.type_annotation()?.type_spec()?.GetText();
        return CompileVariableDecl(name, explicitType, context.test());
    }

    private string CompileVariableDecl(string name, string? explicitPilotType, PilotParser.TestContext? initExpr)
    {
        var sb = new StringBuilder();
        string ptr = $"%{name}.addr";

        if (initExpr != null)
        {
            var (valReg, valType, valCode) = CompileExpression(initExpr);
            string type = !string.IsNullOrWhiteSpace(explicitPilotType) ? MapType(explicitPilotType) : valType;

            if (!_locals.ContainsKey(name))
            {
                sb.AppendLine($"    {ptr} = alloca {type}, align 8");
                _locals[name] = (type, ptr);
            }
            sb.Append(valCode);

            if (valType != type && type == "i64" && valType == "i32")
            {
                string extReg = NextReg();
                sb.AppendLine($"    {extReg} = sext i32 {valReg} to i64");
                valReg = extReg;
            }
            else if (valType != type && type == "double" && valType == "i64")
            {
                string fpReg = NextReg();
                sb.AppendLine($"    {fpReg} = sitofp i64 {valReg} to double");
                valReg = fpReg;
            }
            sb.AppendLine($"    store {type} {valReg}, ptr {ptr}, align 8");
        }
        else
        {
            string type = MapType(explicitPilotType);
            if (!_locals.ContainsKey(name))
            {
                sb.AppendLine($"    {ptr} = alloca {type}, align 8");
                _locals[name] = (type, ptr);
            }
            sb.AppendLine($"    store {type} {DefaultZeroLiteral(type)}, ptr {ptr}, align 8");
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

                bool isFloat = loc.LlvmType is "double" or "float";
                string op = context.augassign(0).GetText();
                string inst = op switch
                {
                    "+=" => isFloat ? "fadd" : "add",
                    "-=" => isFloat ? "fsub" : "sub",
                    "*=" => isFloat ? "fmul" : "mul",
                    "/=" => isFloat ? "fdiv" : "sdiv",
                    "%=" => isFloat ? "frem" : "srem",
                    "&=" => "and",
                    "|=" => "or",
                    "^=" => "xor",
                    "<<=" => "shl",
                    ">>=" => "ashr",
                    _ => isFloat ? "fadd" : "add"
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
                if (_currentFunction != "main" && valType != _currentFunctionRetType &&
                    _currentFunctionRetType == "double" && valType == "i64")
                {
                    string fpReg = NextReg();
                    sb.AppendLine($"    {fpReg} = sitofp i64 {valReg} to double");
                    valReg = fpReg;
                    valType = "double";
                }
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

    // =========================================================================
    // Milestone 11: O(1) LLVM `switch` Jump-Table Lowering for `match`
    // =========================================================================

    public override string VisitMatch_stmt(PilotParser.Match_stmtContext context)
    {
        var (_, _, code) = CompileMatchInternal(context, isExpression: false);
        return code;
    }

    private (string Reg, string Type, string Code) CompileMatchInternal(
        PilotParser.Match_stmtContext context,
        bool isExpression)
    {
        var sb = new StringBuilder();
        var arms = context.match_suite()?.match_arm() ?? Array.Empty<PilotParser.Match_armContext>();

        var (scrutReg, scrutType, scrutCode) = CompileExpression(context.test());
        sb.Append(scrutCode);

        string endBlock = NextBlock("match.end");
        string defaultBlock = NextBlock("match.default");

        string resPtr = "";
        string resType = "i64";
        if (isExpression)
        {
            resType = InferMatchResultType(arms);
            resPtr = $"%match.res.{_blockCounter}";
            sb.AppendLine($"    {resPtr} = alloca {resType}, align 8");
            sb.AppendLine($"    store {resType} {DefaultZeroLiteral(resType)}, ptr {resPtr}, align 8");
        }

        // Case 1: ADT Tagged-Union Scrutinee (%Shape) -> O(1) extractvalue tag + LLVM `switch` jump table
        if (scrutType.StartsWith("%", StringComparison.Ordinal) &&
            _knownAdts.TryGetValue(scrutType[1..], out var adtDef))
        {
            string scrutPtr = $"%match.scrut.{_blockCounter}";
            sb.AppendLine($"    {scrutPtr} = alloca {scrutType}, align 8");
            sb.AppendLine($"    store {scrutType} {scrutReg}, ptr {scrutPtr}, align 8");

            string tagLlvm = adtDef.Layout.TagBytes <= 1 ? "i8" : "i16";
            string tagReg = NextReg();
            sb.AppendLine($"    {tagReg} = extractvalue {scrutType} {scrutReg}, 0");

            var armLabels = Enumerable.Range(0, arms.Length).Select(i => NextBlock($"match.arm.{i}")).ToArray();
            var tagBlocks = Enumerable.Range(0, adtDef.Variants.Count)
                .Select(t => NextBlock($"match.{adtDef.Name}.{adtDef.Variants[t].Name}"))
                .ToArray();

            sb.AppendLine($"    switch {tagLlvm} {tagReg}, label %{defaultBlock} [");
            for (int t = 0; t < adtDef.Variants.Count; t++)
            {
                sb.AppendLine($"        {tagLlvm} {t}, label %{tagBlocks[t]}");
            }
            sb.AppendLine("    ]");

            // Find catch-all arm label if present
            string fallbackLabel = endBlock;
            for (int i = 0; i < arms.Length; i++)
            {
                if (IsWildcardOrCatchAllArm(arms[i], out _))
                {
                    fallbackLabel = armLabels[i];
                    break;
                }
            }

            // Emit per-variant-tag dispatch blocks (outer tag is tested ONCE via `switch`!)
            for (int t = 0; t < adtDef.Variants.Count; t++)
            {
                var variant = adtDef.Variants[t];
                sb.AppendLine($"{tagBlocks[t]}:");

                var matchingArmIndices = new List<(int ArmIdx, PilotParser.Type_or_var_patternContext? TvPat, string? WholeAlias)>();
                for (int i = 0; i < arms.Length; i++)
                {
                    if (TryMatchArmToVariant(arms[i], adtDef.Name, variant.Name, out var tvPat, out string? wholeAlias))
                    {
                        matchingArmIndices.Add((i, tvPat, wholeAlias));
                        if (arms[i].test() == null) break;
                    }
                    else if (IsWildcardOrCatchAllArm(arms[i], out string? catchVar))
                    {
                        matchingArmIndices.Add((i, null, catchVar));
                        if (arms[i].test() == null) break;
                    }
                }

                if (matchingArmIndices.Count == 0)
                {
                    sb.AppendLine($"    br label %{fallbackLabel}");
                    continue;
                }

                for (int m = 0; m < matchingArmIndices.Count; m++)
                {
                    var (armIdx, tvPat, wholeAlias) = matchingArmIndices[m];
                    if (!string.IsNullOrEmpty(wholeAlias) && wholeAlias != "_")
                    {
                        _locals[wholeAlias] = (scrutType, scrutPtr);
                    }

                    if (tvPat != null && variant.Fields.Count > 0 && adtDef.Layout.PayloadSizeBytes > 0)
                    {
                        string payloadPtr = NextReg();
                        sb.AppendLine($"    {payloadPtr} = getelementptr inbounds {scrutType}, ptr {scrutPtr}, i32 0, i32 1");
                        string varStructType = $"%{adtDef.Name}.{variant.Name}";
                        var patArgs = tvPat.pattern_arg();
                        if (patArgs != null)
                        {
                            for (int f = 0; f < patArgs.Length && f < variant.Fields.Count; f++)
                            {
                                string boundName = ExtractSimpleBindingName(patArgs[f].pattern());
                                if (string.IsNullOrEmpty(boundName) || boundName == "_") continue;

                                var fLayout = adtDef.Layout.Variants[t].Fields[f];
                                string fieldLlvmType = fLayout.IsBoxedIndirection ? "ptr" : MapType(variant.Fields[f].TypeName, adtDef.GenericParameters);
                                string fPtr = NextReg();
                                sb.AppendLine($"    {fPtr} = getelementptr inbounds {varStructType}, ptr {payloadPtr}, i32 0, i32 {f}");

                                if (fLayout.IsBoxedIndirection)
                                {
                                    string boxPtrReg = NextReg();
                                    sb.AppendLine($"    {boxPtrReg} = load ptr, ptr {fPtr}, align 8");
                                    string targetType = MapType(variant.Fields[f].TypeName, adtDef.GenericParameters);
                                    _locals[boundName] = (targetType, boxPtrReg);
                                }
                                else
                                {
                                    _locals[boundName] = (fieldLlvmType, fPtr);
                                }
                            }
                        }
                    }

                    if (arms[armIdx].test() != null)
                    {
                        string nextTryLabel = m + 1 < matchingArmIndices.Count
                            ? NextBlock($"match.{variant.Name}.next")
                            : fallbackLabel;
                        var (gReg, gType, gCode) = CompileExpression(arms[armIdx].test());
                        sb.Append(gCode);
                        string gi1 = EnsureI1(gReg, gType, sb);
                        sb.AppendLine($"    br i1 {gi1}, label %{armLabels[armIdx]}, label %{nextTryLabel}");
                        if (m + 1 < matchingArmIndices.Count)
                        {
                            sb.AppendLine($"{nextTryLabel}:");
                        }
                    }
                    else
                    {
                        sb.AppendLine($"    br label %{armLabels[armIdx]}");
                        break;
                    }
                }
            }

            sb.AppendLine($"{defaultBlock}:");
            sb.AppendLine($"    br label %{fallbackLabel}");

            // Emit shared arm bodies once (DAG)
            EmitLlvmMatchArmBodies(sb, arms, armLabels, endBlock, isExpression, resPtr, resType);
        }
        else
        {
            // Case 2: Integer / Bool Scrutinee -> LLVM `switch` jump table!
            string discType = scrutType == "i1" ? "i1" : "i64";
            string discReg = scrutReg;
            if (scrutType != discType && scrutType == "i32")
            {
                discReg = NextReg();
                sb.AppendLine($"    {discReg} = sext i32 {scrutReg} to i64");
            }

            var armLabels = Enumerable.Range(0, arms.Length).Select(i => NextBlock($"match.arm.{i}")).ToArray();
            var switchCases = new List<(long Val, string Label)>();
            var emittedCaseValues = new HashSet<long>();
            string fallbackLabel = endBlock;

            for (int i = 0; i < arms.Length; i++)
            {
                if (IsWildcardOrCatchAllArm(arms[i], out string? catchVar))
                {
                    if (!string.IsNullOrEmpty(catchVar) && catchVar != "_")
                    {
                        string cPtr = $"%{catchVar}.addr";
                        sb.AppendLine($"    {cPtr} = alloca {discType}, align 8");
                        sb.AppendLine($"    store {discType} {discReg}, ptr {cPtr}, align 8");
                        _locals[catchVar] = (discType, cPtr);
                    }
                    fallbackLabel = armLabels[i];
                    break;
                }

                foreach (long caseVal in ExtractSwitchIntegersFromPattern(arms[i].pattern()))
                {
                    if (emittedCaseValues.Add(caseVal))
                    {
                        switchCases.Add((caseVal, armLabels[i]));
                    }
                }
            }

            sb.AppendLine($"    switch {discType} {discReg}, label %{defaultBlock} [");
            foreach (var (val, lbl) in switchCases)
            {
                sb.AppendLine($"        {discType} {val}, label %{lbl}");
            }
            sb.AppendLine("    ]");

            sb.AppendLine($"{defaultBlock}:");
            sb.AppendLine($"    br label %{fallbackLabel}");

            EmitLlvmMatchArmBodies(sb, arms, armLabels, endBlock, isExpression, resPtr, resType);
        }

        sb.AppendLine($"{endBlock}:");
        _hasReturned = false;

        if (isExpression)
        {
            string finalReg = NextReg();
            sb.AppendLine($"    {finalReg} = load {resType}, ptr {resPtr}, align 8");
            return (finalReg, resType, sb.ToString());
        }

        return ("0", "i64", sb.ToString());
    }

    private void EmitLlvmMatchArmBodies(
        StringBuilder sb,
        PilotParser.Match_armContext[] arms,
        string[] armLabels,
        string endBlock,
        bool isExpression,
        string resPtr,
        string resType)
    {
        for (int i = 0; i < arms.Length; i++)
        {
            sb.AppendLine($"{armLabels[i]}:");
            _hasReturned = false;

            if (isExpression)
            {
                var exprNode = TryExtractArmResultExpr(arms[i].suite());
                if (exprNode != null)
                {
                    // Emit any preceding statements in block arm before final expression
                    if (arms[i].suite().stmt() is { Length: > 1 } stmts)
                    {
                        for (int s = 0; s < stmts.Length - 1; s++)
                        {
                            sb.Append(Visit(stmts[s]));
                        }
                    }

                    var (armValReg, armValType, armCode) = CompileExpression(exprNode);
                    sb.Append(armCode);
                    if (armValType != resType && resType == "double" && armValType == "i64")
                    {
                        string fp = NextReg();
                        sb.AppendLine($"    {fp} = sitofp i64 {armValReg} to double");
                        armValReg = fp;
                    }
                    sb.AppendLine($"    store {resType} {armValReg}, ptr {resPtr}, align 8");
                }
                else
                {
                    sb.Append(VisitSuite(arms[i].suite()));
                }
            }
            else
            {
                sb.Append(VisitSuite(arms[i].suite()));
            }

            if (!_hasReturned)
            {
                sb.AppendLine($"    br label %{endBlock}");
            }
        }
    }

    private string InferMatchResultType(PilotParser.Match_armContext[] arms)
    {
        foreach (var arm in arms)
        {
            var expr = TryExtractArmResultExpr(arm.suite());
            if (expr != null)
            {
                string txt = expr.GetText();
                if (txt.Contains('.')) return "double";
                if (txt.StartsWith("\"") || txt.StartsWith("'")) return "ptr";
            }
        }
        return _currentFunction != "main" && _currentFunctionRetType != "void" ? _currentFunctionRetType : "i64";
    }

    private static PilotParser.TestContext? TryExtractArmResultExpr(PilotParser.SuiteContext suite)
    {
        if (suite.simple_stmt()?.small_stmt() is { Length: > 0 } smalls)
        {
            return smalls[^1].expr_stmt()?.testlist()?.test(0);
        }
        if (suite.stmt() is { Length: > 0 } stmts)
        {
            return stmts[^1].simple_stmt()?.small_stmt()?.LastOrDefault()?.expr_stmt()?.testlist()?.test(0);
        }
        return null;
    }

    private bool IsWildcardOrCatchAllArm(PilotParser.Match_armContext arm, out string? boundVar)
    {
        boundVar = null;
        var np = arm.pattern()?.or_pattern()?.and_pattern(0)?.not_pattern(0);
        if (np == null || arm.pattern().or_pattern().and_pattern().Length != 1) return false;
        if (np.primary_pattern()?.UNDERSCORE() != null) return true;
        if (np.primary_pattern()?.type_or_var_pattern() is { } tv &&
            tv.dotted_name()?.identifier().Length == 1 &&
            tv.LPAR() == null &&
            tv.generic_pattern_suffix() == null)
        {
            string id = tv.dotted_name().GetText();
            if (!_knownVariants.ContainsKey(id) && !_knownAdts.ContainsKey(id) && !_knownRecords.ContainsKey(id))
            {
                boundVar = id;
                return true;
            }
        }
        return false;
    }

    private static bool TryMatchArmToVariant(
        PilotParser.Match_armContext arm,
        string adtName,
        string variantName,
        out PilotParser.Type_or_var_patternContext? matchedTv,
        out string? wholeAlias)
    {
        matchedTv = null;
        wholeAlias = null;
        var branches = arm.pattern()?.or_pattern()?.and_pattern();
        if (branches == null) return false;

        foreach (var b in branches)
        {
            var np = b.not_pattern(0);
            if (np == null) continue;
            string? alias = null;
            while (np.AT() != null && np.not_pattern() != null)
            {
                alias = np.identifier()?.GetText();
                np = np.not_pattern();
            }

            var tv = np.primary_pattern()?.type_or_var_pattern();
            if (tv?.dotted_name() == null) continue;

            var parts = tv.dotted_name().identifier();
            string vCandidate = parts.Length == 2 ? parts[1].GetText() : tv.dotted_name().GetText();
            if (parts.Length == 2 && !string.Equals(parts[0].GetText(), adtName, StringComparison.Ordinal))
                continue;

            if (string.Equals(vCandidate, variantName, StringComparison.Ordinal))
            {
                matchedTv = tv;
                wholeAlias = alias;
                return true;
            }
        }
        return false;
    }

    private static string ExtractSimpleBindingName(PilotParser.PatternContext pat)
    {
        var np = pat.or_pattern()?.and_pattern(0)?.not_pattern(0);
        if (np == null) return "";
        if (np.AT() != null && np.identifier() != null) return np.identifier().GetText();
        return np.primary_pattern()?.type_or_var_pattern()?.dotted_name()?.GetText()
            ?? np.primary_pattern()?.type_or_var_pattern()?.identifier()?.GetText()
            ?? "";
    }

    private static IEnumerable<long> ExtractSwitchIntegersFromPattern(PilotParser.PatternContext? pat)
    {
        var branches = pat?.or_pattern()?.and_pattern();
        if (branches == null) yield break;

        foreach (var b in branches)
        {
            var prim = b.not_pattern(0)?.primary_pattern();
            if (prim == null) continue;

            if (prim.literal() is { } lit)
            {
                if (lit.TRUE() != null) yield return 1;
                else if (lit.FALSE() != null) yield return 0;
                else if (lit.INTEGER() != null && long.TryParse(lit.INTEGER().GetText().Replace("_", ""), out long lv))
                    yield return lv;
            }
            else if (prim.signed_number_pattern()?.INTEGER() is { } sInt &&
                     long.TryParse(sInt.GetText().Replace("_", ""), out long nv))
            {
                yield return -nv;
            }
            else if (prim.range_pattern() is { } rp)
            {
                var bounds = rp.range_bound();
                if (bounds.Length == 2 &&
                    bounds[0].INTEGER() != null && bounds[1].INTEGER() != null &&
                    long.TryParse(bounds[0].INTEGER().GetText().Replace("_", ""), out long lo) &&
                    long.TryParse(bounds[1].INTEGER().GetText().Replace("_", ""), out long hi))
                {
                    if (bounds[0].MINUS() != null) lo = -lo;
                    if (bounds[1].MINUS() != null) hi = -hi;
                    long inclusiveHi = rp.DOTDOT_LESS() != null ? hi - 1 : hi;
                    if (inclusiveHi >= lo && inclusiveHi - lo <= 256)
                    {
                        for (long v = lo; v <= inclusiveHi; v++) yield return v;
                    }
                }
            }
        }
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
            if (leftType is "double" or "float" || rightType is "double" or "float")
            {
                string fPred = op switch
                {
                    "==" => "oeq",
                    "!=" => "une",
                    "<" => "olt",
                    "<=" => "ole",
                    ">" => "ogt",
                    ">=" => "oge",
                    _ => "oeq"
                };
                string fRes = NextReg();
                sb.AppendLine($"    {fRes} = fcmp {fPred} {leftType} {leftReg}, {rightReg}");
                return (fRes, "i1", sb.ToString());
            }

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
            bool isFloat = leftType is "double" or "float" || rhsType is "double" or "float";
            if (isFloat)
            {
                if (leftType == "i64")
                {
                    string conv = NextReg();
                    sb.AppendLine($"    {conv} = sitofp i64 {leftReg} to double");
                    leftReg = conv;
                    leftType = "double";
                }
                if (rhsType == "i64")
                {
                    string conv = NextReg();
                    sb.AppendLine($"    {conv} = sitofp i64 {rhsReg} to double");
                    rhsReg = conv;
                }
            }
            string op = context.GetChild(2 * i - 1).GetText();
            string inst = op == "+" ? (isFloat ? "fadd" : "add") : (isFloat ? "fsub" : "sub");
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
            bool isFloat = leftType is "double" or "float" || rhsType is "double" or "float";
            if (isFloat)
            {
                if (leftType == "i64")
                {
                    string conv = NextReg();
                    sb.AppendLine($"    {conv} = sitofp i64 {leftReg} to double");
                    leftReg = conv;
                    leftType = "double";
                }
                if (rhsType == "i64")
                {
                    string conv = NextReg();
                    sb.AppendLine($"    {conv} = sitofp i64 {rhsReg} to double");
                    rhsReg = conv;
                }
            }
            string op = context.GetChild(2 * i - 1).GetText();
            string inst = op switch
            {
                "*" => isFloat ? "fmul" : "mul",
                "/" => isFloat ? "fdiv" : "sdiv",
                "//" => "sdiv",
                "%" => isFloat ? "frem" : "srem",
                _ => isFloat ? "fmul" : "mul"
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
        if (context.MINUS() != null)
        {
            var (innerReg, innerType, innerCode) = CompileFactor(context.factor());
            sb.Append(innerCode);
            string resReg = NextReg();
            if (innerType is "double" or "float")
            {
                sb.AppendLine($"    {resReg} = fneg {innerType} {innerReg}");
            }
            else
            {
                sb.AppendLine($"    {resReg} = sub {innerType} 0, {innerReg}");
            }
            return (resReg, innerType, sb.ToString());
        }
        if (context.PLUS() != null)
        {
            return CompileFactor(context.factor());
        }
        if (context.unary_op() != null)
        {
            var (innerReg, innerType, innerCode) = CompileFactor(context.factor());
            sb.Append(innerCode);
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

        // Match expression: `let area = match shape { ... }`
        if (atom.match_stmt() != null)
        {
            return CompileMatchInternal(atom.match_stmt(), isExpression: true);
        }

        // Check if this is a call: identifier(args...) or ADT/Record constructor
        if (atom.identifier() != null && trailers?.Length > 0 && trailers[0].LPAR() != null)
        {
            string funcName = atom.identifier().GetText();
            var argList = trailers[0].arglist()?.argument();

            // 1. ADT Variant Constructor -> inline stack tagged-union construction
            if (_knownVariants.TryGetValue(funcName, out var variantDef))
            {
                return EmitAdtVariantConstruction(variantDef, argList);
            }

            // 2. Record Product Type Constructor -> inline stack struct construction
            if (_knownRecords.TryGetValue(funcName, out var recDef))
            {
                return EmitRecordConstruction(recDef, argList);
            }

            // 3. Special built-in: print(...)
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

            // 4. General function call
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
            string retLlvmType = _functionReturnTypes.TryGetValue(funcName, out var knownRet) ? knownRet : "i64";
            if (retLlvmType == "void")
            {
                sb.AppendLine($"    call void @{funcName}({callArgs})");
                return ("0", "void", sb.ToString());
            }
            string retReg = NextReg();
            sb.AppendLine($"    {retReg} = call {retLlvmType} @{funcName}({callArgs})");
            return (retReg, retLlvmType, sb.ToString());
        }

        // Parenthesized expression: (expr)
        if (atom.LPAR() != null && atom.testlist_comp()?.test()?.Length > 0)
        {
            return CompileExpression(atom.testlist_comp().test(0));
        }

        // Identifier lookup, Unit ADT Variant, or Record `.field` projection
        if (atom.identifier() != null)
        {
            string id = atom.identifier().GetText();

            // Qualified unit/payload variant: `Shape.Origin` or `Shape.Circle(5.0)`
            if (_knownAdts.TryGetValue(id, out var qualAdt) && trailers?.Length > 0 && trailers[0].DOT() != null)
            {
                string vName = trailers[0].identifier().GetText();
                if (qualAdt.TryGetVariant(vName, out var qv) && qv != null)
                {
                    var callArgs = trailers.Length > 1 && trailers[1].LPAR() != null
                        ? trailers[1].arglist()?.argument()
                        : null;
                    return EmitAdtVariantConstruction(qv, callArgs);
                }
            }

            // Unqualified unit variant: `Origin`
            if (_knownVariants.TryGetValue(id, out var unitVar) && unitVar.IsUnit)
            {
                return EmitAdtVariantConstruction(unitVar, null);
            }

            if (_locals.TryGetValue(id, out var loc))
            {
                string loadReg = NextReg();
                sb.AppendLine($"    {loadReg} = load {loc.LlvmType}, ptr {loc.PtrRegister}, align 8");
                string curReg = loadReg;
                string curType = loc.LlvmType;

                // Record field access: `p.x`
                if (trailers != null)
                {
                    foreach (var tr in trailers)
                    {
                        if (tr.DOT() != null && curType.StartsWith("%", StringComparison.Ordinal) &&
                            _knownRecords.TryGetValue(curType[1..], out var rd))
                        {
                            int fIdx = rd.GetFieldIndex(tr.identifier().GetText());
                            if (fIdx >= 0)
                            {
                                string fType = MapType(rd.Fields[fIdx].TypeName, rd.GenericParameters);
                                string extReg = NextReg();
                                sb.AppendLine($"    {extReg} = extractvalue {curType} {curReg}, {fIdx}");
                                curReg = extReg;
                                curType = fType;
                            }
                        }
                    }
                }

                return (curReg, curType, sb.ToString());
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

    private (string Reg, string Type, string Code) EmitAdtVariantConstruction(
        PilotAdtVariantDefinition variant,
        PilotParser.ArgumentContext[]? args)
    {
        var sb = new StringBuilder();
        var adt = variant.ParentAdt;
        string adtType = $"%{adt.Name}";
        string tagLlvm = adt.Layout.TagBytes <= 1 ? "i8" : "i16";
        string tmpPtr = $"%adt.init.{_regCounter++}";

        sb.AppendLine($"    {tmpPtr} = alloca {adtType}, align 8");
        string tagPtr = NextReg();
        sb.AppendLine($"    {tagPtr} = getelementptr inbounds {adtType}, ptr {tmpPtr}, i32 0, i32 0");
        sb.AppendLine($"    store {tagLlvm} {variant.Tag}, ptr {tagPtr}, align 1");

        if (variant.Fields.Count > 0 && args != null && adt.Layout.PayloadSizeBytes > 0)
        {
            string payloadPtr = NextReg();
            sb.AppendLine($"    {payloadPtr} = getelementptr inbounds {adtType}, ptr {tmpPtr}, i32 0, i32 1");
            string varStructType = $"%{adt.Name}.{variant.Name}";
            var vLayout = adt.Layout.Variants[variant.Tag];

            for (int i = 0; i < args.Length && i < variant.Fields.Count; i++)
            {
                var (aReg, aType, aCode) = CompileExpression(args[i].test());
                sb.Append(aCode);

                string fieldPtr = NextReg();
                sb.AppendLine($"    {fieldPtr} = getelementptr inbounds {varStructType}, ptr {payloadPtr}, i32 0, i32 {i}");

                if (vLayout.Fields[i].IsBoxedIndirection)
                {
                    int allocBytes = Math.Max(8, adt.Layout.TotalSizeBytes);
                    string heapPtr = NextReg();
                    sb.AppendLine($"    {heapPtr} = call ptr @malloc(i64 {allocBytes})");
                    sb.AppendLine($"    store {aType} {aReg}, ptr {heapPtr}, align 8");
                    sb.AppendLine($"    store ptr {heapPtr}, ptr {fieldPtr}, align 8");
                }
                else
                {
                    string expectedType = MapType(variant.Fields[i].TypeName, adt.GenericParameters);
                    if (expectedType == "double" && aType == "i64")
                    {
                        string fp = NextReg();
                        sb.AppendLine($"    {fp} = sitofp i64 {aReg} to double");
                        aReg = fp;
                        aType = "double";
                    }
                    sb.AppendLine($"    store {expectedType} {aReg}, ptr {fieldPtr}, align 8");
                }
            }
        }

        string valReg = NextReg();
        sb.AppendLine($"    {valReg} = load {adtType}, ptr {tmpPtr}, align 8");
        return (valReg, adtType, sb.ToString());
    }

    private (string Reg, string Type, string Code) EmitRecordConstruction(
        PilotRecordDefinition rec,
        PilotParser.ArgumentContext[]? args)
    {
        var sb = new StringBuilder();
        string recType = $"%{rec.Name}";
        string tmpPtr = $"%rec.init.{_regCounter++}";
        sb.AppendLine($"    {tmpPtr} = alloca {recType}, align 8");

        if (args != null)
        {
            for (int i = 0; i < args.Length && i < rec.Fields.Count; i++)
            {
                var (aReg, aType, aCode) = CompileExpression(args[i].test());
                sb.Append(aCode);
                string expectedType = MapType(rec.Fields[i].TypeName, rec.GenericParameters);
                if (expectedType == "double" && aType == "i64")
                {
                    string fp = NextReg();
                    sb.AppendLine($"    {fp} = sitofp i64 {aReg} to double");
                    aReg = fp;
                }
                string fPtr = NextReg();
                sb.AppendLine($"    {fPtr} = getelementptr inbounds {recType}, ptr {tmpPtr}, i32 0, i32 {i}");
                sb.AppendLine($"    store {expectedType} {aReg}, ptr {fPtr}, align 8");
            }
        }

        string valReg = NextReg();
        sb.AppendLine($"    {valReg} = load {recType}, ptr {tmpPtr}, align 8");
        return (valReg, recType, sb.ToString());
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
            string formatted = val.ToString("G", CultureInfo.InvariantCulture);
            if (!formatted.Contains('.') && !formatted.Contains('E') && !formatted.Contains('e'))
            {
                formatted += ".0";
            }
            return (formatted, "double", "");
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
        sb.Append("c\"");
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

    private string MapType(string? pilotType, IReadOnlyList<string>? genericParams = null)
    {
        if (string.IsNullOrWhiteSpace(pilotType)) return "i64";
        string trimmed = pilotType.Trim();
        if (genericParams != null && genericParams.Contains(trimmed))
        {
            return "i64";
        }

        if (trimmed.EndsWith("[]", StringComparison.Ordinal))
        {
            return "{ ptr, i64 }";
        }

        string baseName = PilotAdtMemoryLayout.StripGenericsAndModifiers(trimmed);
        if (_knownAdts.ContainsKey(baseName) || _knownRecords.ContainsKey(baseName))
        {
            return $"%{baseName}";
        }

        return baseName.ToLowerInvariant() switch
        {
            "int" or "i64" or "long" or "uint" or "u64" => "i64",
            "i32" or "u32" => "i32",
            "i16" or "u16" => "i16",
            "i8" or "u8" or "byte" => "i8",
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
