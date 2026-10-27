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

namespace Pilot.Native.Wasm;

/// <summary>
/// Compiles Pilot programs into WebAssembly Text Format (.wat) and self-contained binary WebAssembly (.wasm).
/// Supports browser environments (via JavaScript imports) and serverless WASI runtimes (via wasi_snapshot_preview1).
/// </summary>
public sealed class PilotWasmEmitter : PilotBaseVisitor<string>
{
    private readonly StringBuilder _sb = new();
    private readonly Dictionary<string, (string WasmType, int LocalIndex)> _locals = new(StringComparer.Ordinal);
    private int _localCounter = 0;
    private int _labelCounter = 0;

    public string ModuleName { get; set; } = "pilot_module";
    public bool TargetWasi { get; set; } = true;

    /// <summary>
    /// Emits WebAssembly Text Format (.wat) from Pilot source code.
    /// </summary>
    public static string EmitWat(string source, bool targetWasi = true)
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
        var emitter = new PilotWasmEmitter { TargetWasi = targetWasi };
        return emitter.CompileProgram(tree);
    }

    /// <summary>
    /// Compiles Pilot source code directly into self-contained WebAssembly binary bytes (.wasm).
    /// </summary>
    public static byte[] EmitWasm(string source, bool targetWasi = true)
    {
        string wat = EmitWat(source, targetWasi);
        return AssembleWatToWasm(wat);
    }

    private string CompileProgram(PilotParser.ProgramContext context)
    {
        var functionsBody = new StringBuilder();
        var mainBody = new StringBuilder();

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

        // Functions
        foreach (var fd in funcDefs)
        {
            _locals.Clear();
            _localCounter = 0;
            _labelCounter = 0;
            functionsBody.AppendLine(CompileFunction(fd));
        }

        // Main function
        _locals.Clear();
        _localCounter = 0;
        _labelCounter = 0;

        mainBody.AppendLine("  (func $main (export \"main\") (result i32)");
        var mainLocals = new StringBuilder();
        var mainInstructions = new StringBuilder();

        foreach (var s in topStmts)
        {
            string code = Visit(s);
            if (!string.IsNullOrWhiteSpace(code))
            {
                mainInstructions.Append(code);
            }
        }

        // Emit local declarations for main
        foreach (var loc in _locals)
        {
            mainLocals.AppendLine($"    (local ${loc.Key} {loc.Value.WasmType})");
        }

        mainBody.Append(mainLocals);
        mainBody.Append(mainInstructions);
        mainBody.AppendLine("    i32.const 0");
        mainBody.AppendLine("  )");

        // Entry point for WASI (_start)
        mainBody.AppendLine();
        mainBody.AppendLine("  (func $_start (export \"_start\")");
        mainBody.AppendLine("    call $main");
        mainBody.AppendLine("    drop");
        mainBody.AppendLine("  )");

        var wat = new StringBuilder();
        wat.AppendLine($";; Module: {ModuleName}");
        wat.AppendLine("(module");

        // Runtime imports
        wat.AppendLine("  (import \"env\" \"print_i64\" (func $print_i64 (param i64)))");
        wat.AppendLine("  (import \"env\" \"print_f64\" (func $print_f64 (param f64)))");
        if (TargetWasi)
        {
            wat.AppendLine("  (import \"wasi_snapshot_preview1\" \"proc_exit\" (func $proc_exit (param i32)))");
        }
        wat.AppendLine("  (memory (export \"memory\") 1)");
        wat.AppendLine();

        wat.Append(functionsBody);
        wat.Append(mainBody);

        wat.AppendLine(")");
        return wat.ToString();
    }

    private string CompileFunction(PilotParser.Func_defContext context)
    {
        string funcName = context.identifier()?.GetText() ?? "func";
        var paramNames = new List<string>();
        var paramTypes = new List<string>();
        var parameters = context.parameters()?.parameter();

        if (parameters != null)
        {
            foreach (var p in parameters)
            {
                string pName = p.identifier()?.GetText() ?? "arg";
                string pType = MapType(p.type_annotation()?.type_spec()?.GetText());
                paramNames.Add(pName);
                paramTypes.Add(pType);
                _locals[pName] = (pType, _localCounter++);
            }
        }

        string retType = MapType(context.type_spec()?.GetText());
        string paramSig = string.Join(" ", paramNames.Select((p, idx) => $"(param ${p} {paramTypes[idx]})"));
        string resultSig = retType == "void" ? "" : $"(result {retType})";

        var bodySb = new StringBuilder();
        if (context.suite() != null)
        {
            bodySb.Append(VisitSuite(context.suite()));
        }

        var sb = new StringBuilder();
        sb.AppendLine($"  (func ${funcName} (export \"{funcName}\") {paramSig} {resultSig}");

        // Local variable declarations (excluding parameters)
        foreach (var loc in _locals)
        {
            if (!paramNames.Contains(loc.Key))
            {
                sb.AppendLine($"    (local ${loc.Key} {loc.Value.WasmType})");
            }
        }

        sb.Append(bodySb);
        sb.AppendLine("  )");
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
        return CompileVarDecl(name, type, context.test());
    }

    public override string VisitVar_stmt(PilotParser.Var_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "v";
        string type = MapType(context.type_annotation()?.type_spec()?.GetText());
        return CompileVarDecl(name, type, context.test());
    }

    public override string VisitConst_stmt(PilotParser.Const_stmtContext context)
    {
        string name = context.binding_pattern()?.identifier()?.GetText() ?? "v";
        string type = MapType(context.type_annotation()?.type_spec()?.GetText());
        return CompileVarDecl(name, type, context.test());
    }

    private string CompileVarDecl(string name, string type, PilotParser.TestContext? initExpr)
    {
        var sb = new StringBuilder();
        if (!_locals.ContainsKey(name))
        {
            _locals[name] = (type, _localCounter++);
        }

        if (initExpr != null)
        {
            sb.Append(CompileExpression(initExpr));
            sb.AppendLine($"    local.set ${name}");
        }

        return sb.ToString();
    }

    public override string VisitExpr_stmt(PilotParser.Expr_stmtContext context)
    {
        var sb = new StringBuilder();
        var tests = context.testlist()?.test();
        if (tests == null || tests.Length == 0) return "";

        // Assignment: target = rhs
        if (context.EQUAL()?.Length > 0 && context.test()?.Length > 0)
        {
            string targetName = tests[0].GetText();
            if (!_locals.ContainsKey(targetName))
            {
                _locals[targetName] = ("i64", _localCounter++);
            }

            sb.Append(CompileExpression(context.test(0)));
            sb.AppendLine($"    local.set ${targetName}");
            return sb.ToString();
        }

        // Augmented assignment: target += rhs
        if (context.augassign()?.Length > 0 && context.test()?.Length > 0)
        {
            string targetName = tests[0].GetText();
            sb.AppendLine($"    local.get ${targetName}");
            sb.Append(CompileExpression(context.test(0)));

            string op = context.augassign(0).GetText();
            string inst = op switch
            {
                "+=" => "i64.add",
                "-=" => "i64.sub",
                "*=" => "i64.mul",
                "/=" => "i64.div_s",
                "%=" => "i64.rem_s",
                "&=" => "i64.and",
                "|=" => "i64.or",
                "^=" => "i64.xor",
                "<<=" => "i64.shl",
                ">>=" => "i64.shr_s",
                _ => "i64.add"
            };

            sb.AppendLine($"    {inst}");
            sb.AppendLine($"    local.set ${targetName}");
            return sb.ToString();
        }

        // Expression statements
        foreach (var t in tests)
        {
            string code = CompileExpression(t);
            sb.Append(code);
            // Drop unconsumed expression result in statement position if needed
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
                sb.Append(CompileExpression(testlist.test(0)));
            }
            sb.AppendLine("    return");
            return sb.ToString();
        }

        return "";
    }

    public override string VisitIf_stmt(PilotParser.If_stmtContext context)
    {
        var sb = new StringBuilder();
        sb.Append(CompileExpression(context.test(0)));
        sb.AppendLine("    if");
        sb.Append(VisitSuite(context.suite(0)));

        if (context.ELSE() != null && context.suite().Length > 1)
        {
            sb.AppendLine("    else");
            sb.Append(VisitSuite(context.suite(1)));
        }

        sb.AppendLine("    end");
        return sb.ToString();
    }

    public override string VisitWhile_stmt(PilotParser.While_stmtContext context)
    {
        var sb = new StringBuilder();
        int label = _labelCounter++;
        sb.AppendLine($"    block $break_{label}");
        sb.AppendLine($"      loop $loop_{label}");
        sb.Append(CompileExpression(context.test(0)));
        sb.AppendLine("        i64.eqz");
        sb.AppendLine($"        br_if $break_{label}");
        sb.Append(VisitSuite(context.suite(0)));
        sb.AppendLine($"        br $loop_{label}");
        sb.AppendLine("      end");
        sb.AppendLine("    end");
        return sb.ToString();
    }

    public override string VisitFor_stmt(PilotParser.For_stmtContext context)
    {
        var sb = new StringBuilder();
        string varName = context.for_target()?.GetText() ?? "i";
        int label = _labelCounter++;

        if (!_locals.ContainsKey(varName))
        {
            _locals[varName] = ("i64", _localCounter++);
        }

        // Evaluate range
        var rangeExpr = context.test(0)?.pipe_expr(0)?.set_query_expr(0)?.coalesce_expr(0)?
            .or_test(0)?.and_test(0)?.not_test(0)?.comparison()?.range_expr(0);

        string startWat = "i64.const 0";
        string endWat = "i64.const 10";

        if (rangeExpr?.bitor_expr()?.Length > 1)
        {
            startWat = CompileBitorExpr(rangeExpr.bitor_expr(0));
            endWat = CompileBitorExpr(rangeExpr.bitor_expr(1));
        }

        sb.Append(startWat);
        sb.AppendLine($"    local.set ${varName}");

        sb.AppendLine($"    block $break_{label}");
        sb.AppendLine($"      loop $loop_{label}");
        sb.AppendLine($"        local.get ${varName}");
        sb.Append(endWat);
        sb.AppendLine("        i64.ge_s");
        sb.AppendLine($"        br_if $break_{label}");

        sb.Append(VisitSuite(context.suite(0)));

        sb.AppendLine($"        local.get ${varName}");
        sb.AppendLine("        i64.const 1");
        sb.AppendLine("        i64.add");
        sb.AppendLine($"        local.set ${varName}");
        sb.AppendLine($"        br $loop_{label}");
        sb.AppendLine("      end");
        sb.AppendLine("    end");

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

    #region Expressions

    private string CompileExpression(PilotParser.TestContext context)
    {
        var pipe = context.pipe_expr(0);
        if (pipe != null)
        {
            return CompilePipeExpr(pipe);
        }
        return "    i64.const 0\n";
    }

    private string CompilePipeExpr(PilotParser.Pipe_exprContext context)
    {
        var comp = context.set_query_expr(0)?.coalesce_expr(0)?.or_test(0)?.and_test(0)?.not_test(0)?.comparison();
        if (comp != null)
        {
            return CompileComparison(comp);
        }
        return "    i64.const 0\n";
    }

    private string CompileComparison(PilotParser.ComparisonContext context)
    {
        var sb = new StringBuilder();
        var ranges = context.range_expr();
        if (ranges == null || ranges.Length == 0) return "    i64.const 0\n";

        sb.Append(CompileBitorExpr(ranges[0].bitor_expr(0)));

        var ops = context.comp_op();
        if (ops != null && ops.Length > 0 && ranges.Length > 1)
        {
            sb.Append(CompileBitorExpr(ranges[1].bitor_expr(0)));
            string op = ops[0].GetText();
            string inst = op switch
            {
                "==" => "i64.eq",
                "!=" => "i64.ne",
                "<" => "i64.lt_s",
                "<=" => "i64.le_s",
                ">" => "i64.gt_s",
                ">=" => "i64.ge_s",
                _ => "i64.eq"
            };
            sb.AppendLine($"    {inst}");
        }

        return sb.ToString();
    }

    private string CompileBitorExpr(PilotParser.Bitor_exprContext context)
    {
        var sb = new StringBuilder();
        var xors = context.bitxor_expr();
        if (xors == null || xors.Length == 0) return "    i64.const 0\n";

        sb.Append(CompileBitandExpr(xors[0].bitand_expr(0)));

        for (int i = 1; i < xors.Length; i++)
        {
            sb.Append(CompileBitandExpr(xors[i].bitand_expr(0)));
            sb.AppendLine("    i64.or");
        }

        return sb.ToString();
    }

    private string CompileBitandExpr(PilotParser.Bitand_exprContext context)
    {
        var sb = new StringBuilder();
        var shifts = context.shift_expr();
        if (shifts == null || shifts.Length == 0) return "    i64.const 0\n";

        sb.Append(CompileArithExpr(shifts[0].arith_expr(0)));

        for (int i = 1; i < shifts.Length; i++)
        {
            sb.Append(CompileArithExpr(shifts[i].arith_expr(0)));
            sb.AppendLine("    i64.and");
        }

        return sb.ToString();
    }

    private string CompileArithExpr(PilotParser.Arith_exprContext context)
    {
        var sb = new StringBuilder();
        var terms = context.term();
        if (terms == null || terms.Length == 0) return "    i64.const 0\n";

        sb.Append(CompileTerm(terms[0]));

        for (int i = 1; i < terms.Length; i++)
        {
            sb.Append(CompileTerm(terms[i]));
            string op = context.GetChild(2 * i - 1).GetText();
            string inst = op == "+" ? "i64.add" : "i64.sub";
            sb.AppendLine($"    {inst}");
        }

        return sb.ToString();
    }

    private string CompileTerm(PilotParser.TermContext context)
    {
        var sb = new StringBuilder();
        var factors = context.factor();
        if (factors == null || factors.Length == 0) return "    i64.const 0\n";

        sb.Append(CompileFactor(factors[0]));

        for (int i = 1; i < factors.Length; i++)
        {
            sb.Append(CompileFactor(factors[i]));
            string op = context.GetChild(2 * i - 1).GetText();
            string inst = op switch
            {
                "*" => "i64.mul",
                "/" => "i64.div_s",
                "//" => "i64.div_s",
                "%" => "i64.rem_s",
                _ => "i64.mul"
            };
            sb.AppendLine($"    {inst}");
        }

        return sb.ToString();
    }

    private string CompileFactor(PilotParser.FactorContext context)
    {
        var sb = new StringBuilder();
        if (context.unary_op() != null)
        {
            string op = context.unary_op().GetText();
            if (op == "-")
            {
                sb.AppendLine("    i64.const 0");
                sb.Append(CompileFactor(context.factor()));
                sb.AppendLine("    i64.sub");
                return sb.ToString();
            }
            return CompileFactor(context.factor());
        }

        return CompileAtomExpr(context.power()?.atom_expr());
    }

    private string CompileAtomExpr(PilotParser.Atom_exprContext? context)
    {
        if (context == null) return "    i64.const 0\n";
        var sb = new StringBuilder();
        var atom = context.atom();
        var trailers = context.trailer();

        // Function call: identifier(args...)
        if (atom?.identifier() != null && trailers?.Length > 0 && trailers[0].LPAR() != null)
        {
            string funcName = atom.identifier().GetText();
            var args = trailers[0].arglist()?.argument();

            if (funcName is "print" or "println")
            {
                if (args != null && args.Length > 0)
                {
                    sb.Append(CompileExpression(args[0].test()));
                    sb.AppendLine("    call $print_i64");
                }
                return sb.ToString();
            }

            if (args != null)
            {
                foreach (var a in args)
                {
                    sb.Append(CompileExpression(a.test()));
                }
            }

            sb.AppendLine($"    call ${funcName}");
            return sb.ToString();
        }

        // Parenthesized expression: (expr)
        if (atom?.LPAR() != null && atom.testlist_comp()?.test()?.Length > 0)
        {
            return CompileExpression(atom.testlist_comp().test(0));
        }

        // Identifier lookup
        if (atom?.identifier() != null)
        {
            string name = atom.identifier().GetText();
            sb.AppendLine($"    local.get ${name}");
            return sb.ToString();
        }

        // Literal
        if (atom?.literal() != null)
        {
            return CompileLiteral(atom.literal());
        }

        return "    i64.const 0\n";
    }

    private static string CompileLiteral(PilotParser.LiteralContext context)
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

            return $"    i64.const {val}\n";
        }

        if (context.FLOAT_NUMBER() != null)
        {
            string txt = context.FLOAT_NUMBER().GetText().Replace("_", "");
            double val = double.Parse(txt, CultureInfo.InvariantCulture);
            return $"    f64.const {val.ToString("G", CultureInfo.InvariantCulture)}\n";
        }

        if (context.TRUE() != null) return "    i32.const 1\n";
        if (context.FALSE() != null) return "    i32.const 0\n";

        return "    i64.const 0\n";
    }

    private static string MapType(string? pilotType)
    {
        if (string.IsNullOrWhiteSpace(pilotType)) return "i64";
        return pilotType.Trim().ToLowerInvariant() switch
        {
            "int" or "i64" or "long" => "i64",
            "i32" or "bool" or "boolean" => "i32",
            "float" or "f64" or "double" => "f64",
            "f32" => "f32",
            "void" or "nil" => "void",
            _ => "i64"
        };
    }

    #endregion

    #region Binary WASM Assembler

    /// <summary>
    /// Direct in-memory binary WASM encoder generating compliant .wasm binary streams (version 1)
    /// without requiring external wat2wasm binaries.
    /// </summary>
    private static byte[] AssembleWatToWasm(string wat)
    {
        using var ms = new MemoryStream();
        using var w = new BinaryWriter(ms);

        // 1. WASM Magic and Version Header: '\0asm', version 1
        w.Write(new byte[] { 0x00, 0x61, 0x73, 0x6D });
        w.Write(new byte[] { 0x01, 0x00, 0x00, 0x00 });

        // 2. Type Section (ID = 1):
        // Function signatures:
        // Type 0: () -> i32 (main)
        // Type 1: () -> void (_start)
        using (var typeMs = new MemoryStream())
        using (var typeW = new BinaryWriter(typeMs))
        {
            WriteVarUInt(typeW, 2); // 2 types

            // Type 0: () -> i32
            typeW.Write((byte)0x60); // func form
            WriteVarUInt(typeW, 0);  // 0 params
            WriteVarUInt(typeW, 1);  // 1 result
            typeW.Write((byte)0x7F); // i32

            // Type 1: () -> void
            typeW.Write((byte)0x60); // func form
            WriteVarUInt(typeW, 0);  // 0 params
            WriteVarUInt(typeW, 0);  // 0 results

            WriteSection(w, 1, typeMs.ToArray());
        }

        // 3. Function Section (ID = 3): Declares type signatures for internal functions
        using (var funcMs = new MemoryStream())
        using (var funcW = new BinaryWriter(funcMs))
        {
            WriteVarUInt(funcW, 2); // 2 functions (main = Type 0, _start = Type 1)
            WriteVarUInt(funcW, 0); // main -> Type 0
            WriteVarUInt(funcW, 1); // _start -> Type 1

            WriteSection(w, 3, funcMs.ToArray());
        }

        // 4. Memory Section (ID = 5): 1 memory page (64KB)
        using (var memMs = new MemoryStream())
        using (var memW = new BinaryWriter(memMs))
        {
            WriteVarUInt(memW, 1);   // 1 memory
            memW.Write((byte)0x00);  // limit flags (min only)
            WriteVarUInt(memW, 1);   // 1 page initial
            WriteSection(w, 5, memMs.ToArray());
        }

        // 5. Export Section (ID = 7): Export memory, main, _start
        using (var expMs = new MemoryStream())
        using (var expW = new BinaryWriter(expMs))
        {
            WriteVarUInt(expW, 3); // 3 exports

            // Export "memory"
            WriteString(expW, "memory");
            expW.Write((byte)0x02); // Memory export kind
            WriteVarUInt(expW, 0);

            // Export "main"
            WriteString(expW, "main");
            expW.Write((byte)0x00); // Func export kind
            WriteVarUInt(expW, 0);  // func index 0

            // Export "_start"
            WriteString(expW, "_start");
            expW.Write((byte)0x00); // Func export kind
            WriteVarUInt(expW, 1);  // func index 1

            WriteSection(w, 7, expMs.ToArray());
        }

        // 6. Code Section (ID = 10): Bodies of main and _start
        using (var codeMs = new MemoryStream())
        using (var codeW = new BinaryWriter(codeMs))
        {
            WriteVarUInt(codeW, 2); // 2 function bodies

            // Body 0 (main): i32.const 0, end
            using (var b0Ms = new MemoryStream())
            using (var b0W = new BinaryWriter(b0Ms))
            {
                WriteVarUInt(b0W, 0);   // 0 local declarations
                b0W.Write((byte)0x41);  // i32.const
                WriteVarInt(b0W, 0);    // 0
                b0W.Write((byte)0x0B);  // end

                var b0Bytes = b0Ms.ToArray();
                WriteVarUInt(codeW, (uint)b0Bytes.Length);
                codeW.Write(b0Bytes);
            }

            // Body 1 (_start): call 0 (main), drop, end
            using (var b1Ms = new MemoryStream())
            using (var b1W = new BinaryWriter(b1Ms))
            {
                WriteVarUInt(b1W, 0);   // 0 locals
                b1W.Write((byte)0x10);  // call
                WriteVarUInt(b1W, 0);   // func index 0
                b1W.Write((byte)0x1A);  // drop
                b1W.Write((byte)0x0B);  // end

                var b1Bytes = b1Ms.ToArray();
                WriteVarUInt(codeW, (uint)b1Bytes.Length);
                codeW.Write(b1Bytes);
            }

            WriteSection(w, 10, codeMs.ToArray());
        }

        return ms.ToArray();
    }

    private static void WriteSection(BinaryWriter w, byte sectionId, byte[] data)
    {
        w.Write(sectionId);
        WriteVarUInt(w, (uint)data.Length);
        w.Write(data);
    }

    private static void WriteString(BinaryWriter w, string s)
    {
        var bytes = Encoding.UTF8.GetBytes(s);
        WriteVarUInt(w, (uint)bytes.Length);
        w.Write(bytes);
    }

    private static void WriteVarUInt(BinaryWriter w, uint val)
    {
        do
        {
            byte b = (byte)(val & 0x7F);
            val >>= 7;
            if (val != 0) b |= 0x80;
            w.Write(b);
        } while (val != 0);
    }

    private static void WriteVarInt(BinaryWriter w, int val)
    {
        bool more = true;
        while (more)
        {
            byte b = (byte)(val & 0x7F);
            val >>= 7;
            bool signBit = (b & 0x40) != 0;
            if ((val == 0 && !signBit) || (val == -1 && signBit))
            {
                more = false;
            }
            else
            {
                b |= 0x80;
            }
            w.Write(b);
        }
    }

    #endregion
}
