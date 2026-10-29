using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Pilot.Analysis;
using Pilot.Analysis.TypeSystem;
using Pilot.Transpilation.Ingesters;
using Pilot.Transpilation.Ir;

namespace Pilot.Transpilation.Targets;

/// <summary>
/// Milestone 20.3 &amp; 20.4: AST-driven bidirectional code converter between Pilot and Lua 5.4 / LuaJIT.
/// Emits register-allocated <c>local</c> bindings, native numeric <c>for</c> loops, metatable classes,
/// <c>string.format</c> interpolations, and IIFE comprehensions.
/// </summary>
public sealed class LuaTargetConverter : ITargetLanguageConverter
{
    public string Language => "lua";
    public string TargetLanguage => Language;
    public string SourceLanguage => Language;

    public TranspilationResult TranspileSource(string pilotSource, TranspilationOptions options)
    {
        var (module, errors) = PilotTranspileIrBuilder.BuildFromSource(pilotSource, options);
        if (module == null || errors.Count > 0)
        {
            return TranspilationResult.CreateFailure(errors, TargetLanguage);
        }

        return TranspileModule(module, options);
    }

    public TranspilationResult Transpile(PilotParser.ProgramContext programAst, TranspilationOptions options)
    {
        PilotStaticAnalysisResult? analysis = null;
        try { analysis = PilotAnalysisPipeline.Analyze(programAst); } catch { }
        var module = PilotTranspileIrBuilder.Build(programAst, null, analysis, options);
        return TranspileModule(module, options);
    }

    public TranspilationResult TranspileModule(PilotIrModule module, TranspilationOptions options)
    {
        var sb = new StringBuilder();
        if (options.EmitHeaderComments)
        {
            sb.AppendLine("-- Code transpiled & optimized from Pilot to Lua 5.4 / LuaJIT");
            sb.AppendLine("-- Optimizations: local register bindings, numeric range loops, constant folding");
            sb.AppendLine();
        }

        var emitter = new LuaIrEmitter(module, options);
        sb.Append(emitter.EmitModule());

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options) =>
        ForeignAstIngester.Ingest(foreignSource, SourceLanguage, options);

    private sealed class LuaIrEmitter
    {
        private readonly PilotIrModule _module;
        private readonly TranspilationOptions _options;
        private int _indent;

        public LuaIrEmitter(PilotIrModule module, TranspilationOptions options)
        {
            _module = module;
            _options = options;
        }

        private string Indent => string.Concat(Enumerable.Repeat(_options.IndentString, _indent));

        public string EmitModule()
        {
            var sb = new StringBuilder();
            foreach (var stmt in _module.Statements)
            {
                string s = EmitStmt(stmt);
                if (!string.IsNullOrWhiteSpace(s))
                {
                    sb.AppendLine(s);
                }
            }
            return sb.ToString();
        }

        public string EmitStmt(PilotIrStmt stmt) => stmt switch
        {
            PilotIrCommentStmt c => $"{Indent}--{c.CommentText.TrimStart('/')}",
            PilotIrVarDeclStmt v => EmitVarDecl(v),
            PilotIrAssignStmt a => EmitAssign(a),
            PilotIrFuncDefStmt f => EmitFuncDef(f, classOwner: null),
            PilotIrClassDefStmt c => EmitClassDef(c),
            PilotIrAdtDefStmt adt => EmitAdtDef(adt),
            PilotIrEnumDefStmt en => EmitEnumDef(en),
            PilotIrExtensionDefStmt ext => EmitExtensionDef(ext),
            PilotIrIfStmt ifs => EmitIfStmt(ifs),
            PilotIrGuardStmt g => EmitGuardStmt(g),
            PilotIrWhileStmt w => EmitWhileStmt(w),
            PilotIrForStmt f => EmitForStmt(f),
            PilotIrMatchStmt m => EmitMatchStmt(m),
            PilotIrDeferStmt d => $"-- [defer] {string.Join("; ", d.Body.Select(s => EmitStmt(s).Trim()))}",
            PilotIrReturnStmt r => r.Value != null ? $"{Indent}return {EmitExpr(r.Value)}" : $"{Indent}return",
            PilotIrRaiseStmt rs => rs.Value != null ? $"{Indent}error({EmitExpr(rs.Value)})" : $"{Indent}error(\"error\")",
            PilotIrBreakStmt => $"{Indent}break",
            PilotIrPassStmt => $"{Indent}-- pass",
            PilotIrAssertStmt a => a.Message != null
                ? $"{Indent}assert({EmitExpr(a.Condition)}, {EmitExpr(a.Message)})"
                : $"{Indent}assert({EmitExpr(a.Condition)})",
            PilotIrComptimeStmt ct => string.Join(Environment.NewLine, ct.Body.Select(EmitStmt)),
            PilotIrExprStmt e => $"{Indent}{EmitExpr(e.Expression)}",
            _ => ""
        };

        private string EmitVarDecl(PilotIrVarDeclStmt v)
        {
            if (v.TupleDestructureNames.Count > 1)
            {
                string lhs = string.Join(", ", v.TupleDestructureNames);
                string rhs = v.Initializer != null ? EmitExpr(v.Initializer) : "nil";
                return $"{Indent}local {lhs} = table.unpack({rhs})";
            }

            string rhsVal = v.ComptimeFoldedLiteral ?? (v.Initializer != null ? EmitExpr(v.Initializer) : "nil");
            if (v.Kind == PilotVarDeclKind.Const)
            {
                return $"{Indent}local {v.Name} <const> = {rhsVal}";
            }
            return $"{Indent}local {v.Name} = {rhsVal}";
        }

        private string EmitAssign(PilotIrAssignStmt a)
        {
            string target = EmitExpr(a.Target);
            string val = EmitExpr(a.Value);
            return a.Operator switch
            {
                "+=" => $"{Indent}{target} = {target} + ({val})",
                "-=" => $"{Indent}{target} = {target} - ({val})",
                "*=" => $"{Indent}{target} = {target} * ({val})",
                "/=" => $"{Indent}{target} = {target} / ({val})",
                _ => $"{Indent}{target} = {val}"
            };
        }

        private string EmitFuncDef(PilotIrFuncDefStmt f, string? classOwner)
        {
            var paramList = f.Parameters
                .Where(p => classOwner == null || p.Name is not ("self" or "this"))
                .Select(p => p.IsVariadic ? "..." : p.Name)
                .ToList();

            string header = classOwner != null
                ? $"function {classOwner}:{f.Name}({string.Join(", ", paramList)})"
                : $"local function {f.Name}({string.Join(", ", paramList)})";

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}{header}");
            _indent++;
            foreach (var req in f.RequiresClauses)
                sb.AppendLine($"{Indent}assert({EmitExpr(req)})");
            foreach (var s in f.Body)
            {
                string line = EmitStmt(s);
                if (!string.IsNullOrWhiteSpace(line))
                    sb.AppendLine(line);
            }
            _indent--;
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        private string EmitClassDef(PilotIrClassDefStmt c)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}local {c.Name} = {{}}");
            sb.AppendLine($"{Indent}{c.Name}.__index = {c.Name}");
            sb.AppendLine($"{Indent}function {c.Name}.new(init)");
            sb.AppendLine($"{Indent}{_options.IndentString}return setmetatable(init or {{}}, {c.Name})");
            sb.AppendLine($"{Indent}end");
            foreach (var m in c.Methods)
            {
                sb.AppendLine(EmitFuncDef(m, classOwner: c.Name));
            }
            return sb.ToString().TrimEnd();
        }

        private string EmitAdtDef(PilotIrAdtDefStmt adt)
        {
            var sb = new StringBuilder();
            foreach (var v in adt.Variants)
            {
                var fNames = v.Fields.Select(f => f.Name).ToList();
                var pairs = new List<string> { $"_tag = \"{v.Name}\"" };
                pairs.AddRange(fNames.Select(n => $"{n} = {n}"));
                sb.AppendLine($"{Indent}local function {v.Name}({string.Join(", ", fNames)})");
                sb.AppendLine($"{Indent}{_options.IndentString}return {{ {string.Join(", ", pairs)} }}");
                sb.AppendLine($"{Indent}end");
            }
            return sb.ToString().TrimEnd();
        }

        private string EmitEnumDef(PilotIrEnumDefStmt en)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}local {en.Name} = {{");
            _indent++;
            for (int i = 0; i < en.Members.Count; i++)
            {
                var (mName, mVal) = en.Members[i];
                string val = mVal != null ? EmitExpr(mVal) : i.ToString();
                sb.AppendLine($"{Indent}{mName} = {val},");
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitExtensionDef(PilotIrExtensionDefStmt ext)
        {
            var sb = new StringBuilder();
            foreach (var m in ext.Methods)
            {
                if (m.Parameters.Count == 0 || m.Parameters[0].Name is not ("self" or "this"))
                    m.Parameters.Insert(0, new PilotIrParameter("self"));
                sb.AppendLine(EmitFuncDef(m, classOwner: null));
            }
            return sb.ToString().TrimEnd();
        }

        private string EmitIfStmt(PilotIrIfStmt ifs)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}if {EmitExpr(ifs.Condition)} then");
            EmitSuite(sb, ifs.ThenBody);

            foreach (var (cond, body) in ifs.ElifBranches)
            {
                sb.AppendLine($"{Indent}elseif {EmitExpr(cond)} then");
                EmitSuite(sb, body);
            }
            if (ifs.ElseBody.Count > 0)
            {
                sb.AppendLine($"{Indent}else");
                EmitSuite(sb, ifs.ElseBody);
            }
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        private string EmitGuardStmt(PilotIrGuardStmt g)
        {
            var sb = new StringBuilder();
            if (g.IsSomeBinding && !string.IsNullOrEmpty(g.BoundVariable))
            {
                sb.AppendLine($"{Indent}local {g.BoundVariable} = {EmitExpr(g.ConditionOrSource)}");
                string cond = $"{g.BoundVariable} == nil";
                if (g.PatternGuard != null)
                    cond += $" or not ({EmitExpr(g.PatternGuard)})";
                sb.AppendLine($"{Indent}if {cond} then");
            }
            else
            {
                sb.AppendLine($"{Indent}if not ({EmitExpr(g.ConditionOrSource)}) then");
            }
            EmitSuite(sb, g.ElseBody);
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        private string EmitWhileStmt(PilotIrWhileStmt w)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}while {EmitExpr(w.Condition)} do");
            EmitSuite(sb, w.Body);
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        private string EmitForStmt(PilotIrForStmt f)
        {
            var sb = new StringBuilder();
            string target = f.TargetVars.Count > 0 ? f.TargetVars[0] : "_item";
            if (f.Iterable is PilotIrRangeExpr r)
            {
                string start = EmitExpr(r.Start);
                string end = r.IsInclusive ? EmitExpr(r.End) : $"{EmitExpr(r.End)} - 1";
                string step = r.Step != null ? $", {EmitExpr(r.Step)}" : "";
                sb.AppendLine($"{Indent}for {target} = {start}, {end}{step} do");
            }
            else
            {
                sb.AppendLine($"{Indent}for _, {target} in ipairs({EmitExpr(f.Iterable)}) do");
            }
            EmitSuite(sb, f.Body);
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        private string EmitMatchStmt(PilotIrMatchStmt m)
        {
            var sb = new StringBuilder();
            string matchVar = "__match_val";
            sb.AppendLine($"{Indent}local {matchVar} = {EmitExpr(m.Scrutinee)}");
            for (int i = 0; i < m.Arms.Count; i++)
            {
                var arm = m.Arms[i];
                string kw = i == 0 ? "if" : "elseif";
                if (arm.Pattern is PilotIrWildcardPattern or PilotIrBindingPattern)
                    sb.AppendLine(i == 0 ? $"{Indent}if true then" : $"{Indent}else");
                else if (arm.Pattern is PilotIrLiteralPattern lit)
                    sb.AppendLine($"{Indent}{kw} {matchVar} == {EmitExpr(lit.Literal)} then");
                else
                    sb.AppendLine(i == 0 ? $"{Indent}if true then" : $"{Indent}else");
                EmitSuite(sb, arm.Body);
            }
            if (m.Arms.Count > 0)
                sb.Append($"{Indent}end");
            return sb.ToString();
        }

        private void EmitSuite(StringBuilder sb, List<PilotIrStmt> stmts)
        {
            _indent++;
            foreach (var s in stmts)
            {
                string line = EmitStmt(s);
                if (!string.IsNullOrWhiteSpace(line))
                    sb.AppendLine(line);
            }
            _indent--;
        }

        public string EmitExpr(PilotIrExpr expr, PilotIrPrecedence parentPrec = PilotIrPrecedence.Lowest)
        {
            string core = expr switch
            {
                PilotIrLiteralExpr lit => EmitLiteral(lit),
                PilotIrInterpolatedStringExpr fstr => EmitInterpolatedString(fstr),
                PilotIrIdentifierExpr id => id.Name switch
                {
                    "null" or "none" or "None" or "undefined" => "nil",
                    "true" or "True" => "true",
                    "false" or "False" => "false",
                    "this" => "self",
                    _ => id.Name
                },
                PilotIrUnaryExpr u => u.Operator is "!" or "not"
                    ? $"not {EmitExpr(u.Operand, u.Precedence)}"
                    : $"{u.Operator}{EmitExpr(u.Operand, u.Precedence)}",
                PilotIrBinaryExpr b => EmitBinary(b),
                PilotIrCallExpr c => EmitCall(c),
                PilotIrMemberAccessExpr m => EmitMemberAccess(m),
                PilotIrIndexExpr ix => $"{EmitExpr(ix.Target, PilotIrPrecedence.Postfix)}[{EmitExpr(ix.Index)}]",
                PilotIrPostfixTryExpr pt => pt.CatchFallbackExpr != null
                    ? $"({EmitExpr(pt.Operand, PilotIrPrecedence.LogicalOr)} or {EmitExpr(pt.CatchFallbackExpr, PilotIrPrecedence.LogicalOr)})"
                    : EmitExpr(pt.Operand, parentPrec),
                PilotIrCollectionExpr col => EmitCollection(col),
                PilotIrLambdaExpr lam => $"function({string.Join(", ", lam.Parameters.Select(p => p.Name))}) return {(lam.BodyExpr != null ? EmitExpr(lam.BodyExpr) : "nil")} end",
                PilotIrForExpr fe => EmitForExpr(fe),
                PilotIrIfExpr ie => $"({EmitExpr(ie.Condition)} and {EmitExpr(ie.ThenBranch)} or {EmitExpr(ie.ElseBranch)})",
                _ => "nil"
            };

            if (parentPrec != PilotIrPrecedence.Lowest && expr.Precedence < parentPrec)
                return $"({core})";
            return core;
        }

        private static string EmitLiteral(PilotIrLiteralExpr lit) => lit.Kind switch
        {
            PilotLiteralKind.Null or PilotLiteralKind.None => "nil",
            PilotLiteralKind.Boolean => lit.RawValue is "true" or "True" ? "true" : "false",
            PilotLiteralKind.DimensionalUnit => (lit.NormalizedNumericValue ?? 0.0).ToString(System.Globalization.CultureInfo.InvariantCulture),
            _ => lit.RawValue
        };

        private string EmitInterpolatedString(PilotIrInterpolatedStringExpr fstr)
        {
            var fmt = new StringBuilder("\"");
            var args = new List<string>();
            foreach (var p in fstr.Parts)
            {
                if (!p.IsInterpolation)
                {
                    fmt.Append(p.LiteralText);
                }
                else
                {
                    fmt.Append("%s");
                    string inner = p.Expression != null ? EmitExpr(p.Expression) : p.LiteralText;
                    args.Add($"tostring({inner})");
                }
            }
            fmt.Append('"');
            return args.Count > 0
                ? $"string.format({fmt}, {string.Join(", ", args)})"
                : fmt.ToString();
        }

        private string EmitBinary(PilotIrBinaryExpr b)
        {
            if (b.Operator == "|>")
                return $"{EmitExpr(b.Right, PilotIrPrecedence.Postfix)}({EmitExpr(b.Left)})";
            if (b.Operator == "??")
                return $"({EmitExpr(b.Left, PilotIrPrecedence.LogicalOr)} or {EmitExpr(b.Right, PilotIrPrecedence.LogicalOr)})";

            bool isStringConcat = b.Operator == "+" &&
                (b.Left.ResolvedType == PilotType.String || b.Right.ResolvedType == PilotType.String ||
                 b.Left is PilotIrLiteralExpr { Kind: PilotLiteralKind.String } ||
                 b.Right is PilotIrLiteralExpr { Kind: PilotLiteralKind.String });

            string luaOp = isStringConcat ? ".." : b.Operator switch
            {
                "!=" => "~=",
                "&&" or "and" => "and",
                "||" or "or" => "or",
                "**" => "^",
                _ => b.Operator
            };

            string left = EmitExpr(b.Left, b.Precedence);
            var rightMinPrec = b.Operator is "-" or "/" or "%"
                ? (PilotIrPrecedence)((int)b.Precedence + 1)
                : b.Precedence;
            string right = EmitExpr(b.Right, rightMinPrec);
            return $"{left} {luaOp} {right}";
        }

        private string EmitCall(PilotIrCallExpr c)
        {
            var renderedArgs = c.Arguments.Select(a => EmitExpr(a.Value)).ToList();
            if (PilotLibraryMapper.TryFormatAstLibraryCall(c.LibraryCallKind, renderedArgs, "lua", out string mapped))
                return mapped;

            if (c.Callee is PilotIrMemberAccessExpr mem && mem.MemberName is "append" or "push")
                return $"table.insert({EmitExpr(mem.Target, PilotIrPrecedence.Postfix)}, {string.Join(", ", renderedArgs)})";

            string callee = EmitExpr(c.Callee, PilotIrPrecedence.Postfix);
            return $"{callee}({string.Join(", ", renderedArgs)})";
        }

        private string EmitMemberAccess(PilotIrMemberAccessExpr m)
        {
            string target = EmitExpr(m.Target, PilotIrPrecedence.Postfix);
            if (PilotLibraryMapper.TryFormatAstMemberAccess(target, m.MemberName, "lua", out string mapped))
                return mapped;
            return $"{target}.{m.MemberName}";
        }

        private string EmitCollection(PilotIrCollectionExpr col) => col.Kind switch
        {
            PilotCollectionKind.Map => $"{{ {string.Join(", ", col.MapEntries.Select(kv => $"[{EmitExpr(kv.Key)}] = {EmitExpr(kv.Value)}"))} }}",
            _ => $"{{ {string.Join(", ", col.Elements.Select(e => EmitExpr(e)))} }}"
        };

        private string EmitForExpr(PilotIrForExpr fe)
        {
            string target = fe.TargetVars.Count > 0 ? fe.TargetVars[0] : "x";
            string loopHeader = fe.Iterable is PilotIrRangeExpr r
                ? $"for {target} = {EmitExpr(r.Start)}, {(r.IsInclusive ? EmitExpr(r.End) : $"{EmitExpr(r.End)} - 1")} do"
                : $"for _, {target} in ipairs({EmitExpr(fe.Iterable)}) do";
            string filter = fe.FilterCondition != null
                ? $"if {EmitExpr(fe.FilterCondition)} then table.insert(__res, {EmitExpr(fe.YieldExpr)}) end"
                : $"table.insert(__res, {EmitExpr(fe.YieldExpr)})";
            return $"(function() local __res = {{}}; {loopHeader} {filter} end; return __res end)()";
        }
    }
}
