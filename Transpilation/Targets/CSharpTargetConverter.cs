using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Pilot.Analysis;
using Pilot.Transpilation.Ingesters;
using Pilot.Transpilation.Ir;

namespace Pilot.Transpilation.Targets;

/// <summary>
/// Milestone 20.3 &amp; 20.4: AST-driven bidirectional code converter between Pilot and C# (.NET 10).
/// Emits valid C# compilable by Roslyn (<c>CSharpCompilation</c>) and ingests C# via <see cref="CSharpAstIngester"/>.
/// </summary>
public sealed class CSharpTargetConverter : ITargetLanguageConverter
{
    public string Language => "csharp";
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
            sb.AppendLine("// Code transpiled from Pilot to C# (.NET 10)");
            sb.AppendLine("#nullable enable");
            sb.AppendLine("using System;");
            sb.AppendLine("using System.Collections.Generic;");
            sb.AppendLine("using System.IO;");
            sb.AppendLine("using System.Linq;");
            sb.AppendLine("using System.Text.Json;");
            sb.AppendLine();
        }

        string ns = options.TargetNamespaceOrModule ?? "PilotTranspiledApp";
        sb.AppendLine($"namespace {ns};");
        sb.AppendLine();

        var emitter = new CSharpIrEmitter(module, options);
        sb.Append(emitter.EmitModule());

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options) =>
        CSharpAstIngester.Ingest(foreignSource, options);

    private sealed class CSharpIrEmitter
    {
        private readonly PilotIrModule _module;
        private readonly TranspilationOptions _options;
        private readonly HashSet<string> _knownTypeDecls = new(StringComparer.Ordinal);
        private int _indent;
        private bool _inExtensionMethod;

        public CSharpIrEmitter(PilotIrModule module, TranspilationOptions options)
        {
            _module = module;
            _options = options;
            foreach (var c in module.Statements.OfType<PilotIrClassDefStmt>())
                _knownTypeDecls.Add(c.Name);
            foreach (var adt in module.Statements.OfType<PilotIrAdtDefStmt>())
            {
                _knownTypeDecls.Add(adt.Name);
                foreach (var v in adt.Variants)
                    _knownTypeDecls.Add(v.Name);
            }
        }

        private string Indent => string.Concat(Enumerable.Repeat(_options.IndentString, _indent));

        public string EmitModule()
        {
            var sb = new StringBuilder();
            var topTypes = new List<PilotIrStmt>();
            var moduleMembers = new List<PilotIrStmt>();
            var topExecutableStmts = new List<PilotIrStmt>();

            foreach (var stmt in _module.Statements)
            {
                if (stmt is PilotIrClassDefStmt or PilotIrAdtDefStmt or PilotIrEnumDefStmt or PilotIrExtensionDefStmt)
                    topTypes.Add(stmt);
                else if (stmt is PilotIrFuncDefStmt)
                    moduleMembers.Add(stmt);
                else
                    topExecutableStmts.Add(stmt);
            }

            sb.AppendLine("public static class ProgramModule");
            sb.AppendLine("{");
            _indent = 1;

            // Emit top-level classes inside ProgramModule as well or alongside so existing tests find them
            foreach (var t in topTypes)
            {
                if (t is PilotIrExtensionDefStmt)
                    continue;
                string ts = EmitStmt(t);
                if (!string.IsNullOrWhiteSpace(ts))
                {
                    sb.AppendLine(ts);
                    sb.AppendLine();
                }
            }

            foreach (var m in moduleMembers)
            {
                string ms = EmitStmt(m);
                if (!string.IsNullOrWhiteSpace(ms))
                {
                    sb.AppendLine(ms);
                    sb.AppendLine();
                }
            }

            if (topExecutableStmts.Count > 0)
            {
                sb.AppendLine($"{Indent}public static void RunMain()");
                sb.AppendLine($"{Indent}{{");
                _indent++;
                foreach (var s in topExecutableStmts)
                {
                    string line = EmitStmt(s);
                    if (!string.IsNullOrWhiteSpace(line))
                        sb.AppendLine(line);
                }
                _indent--;
                sb.AppendLine($"{Indent}}}");
            }

            _indent = 0;
            sb.AppendLine("}");

            foreach (var ext in topTypes.OfType<PilotIrExtensionDefStmt>())
            {
                sb.AppendLine();
                sb.AppendLine(EmitExtensionClass(ext));
            }

            return sb.ToString();
        }

        public string EmitStmt(PilotIrStmt stmt) => stmt switch
        {
            PilotIrCommentStmt c => $"{Indent}//{c.CommentText.TrimStart('/')}",
            PilotIrVarDeclStmt v => EmitVarDecl(v),
            PilotIrAssignStmt a => $"{Indent}{EmitExpr(a.Target)} {a.Operator} {EmitExpr(a.Value)};",
            PilotIrFuncDefStmt f => EmitFuncDef(f, isInstanceMethod: false, extensionTargetType: null),
            PilotIrClassDefStmt c => EmitClassDef(c),
            PilotIrAdtDefStmt adt => EmitAdtDef(adt),
            PilotIrEnumDefStmt en => EmitEnumDef(en),
            PilotIrIfStmt ifs => EmitIfStmt(ifs),
            PilotIrGuardStmt g => EmitGuardStmt(g),
            PilotIrWhileStmt w => EmitWhileStmt(w),
            PilotIrForStmt f => EmitForStmt(f),
            PilotIrMatchStmt m => EmitMatchStmt(m),
            PilotIrTryStmt t => EmitTryStmt(t),
            PilotIrDeferStmt d => string.Join(Environment.NewLine, d.Body.Select(EmitStmt)),
            PilotIrReturnStmt r => r.Value != null ? $"{Indent}return {EmitExpr(r.Value)};" : $"{Indent}return;",
            PilotIrYieldStmt y => y.Value != null ? $"{Indent}yield return {EmitExpr(y.Value)};" : $"{Indent}yield break;",
            PilotIrRaiseStmt rs => rs.Value != null ? $"{Indent}throw new Exception(Convert.ToString({EmitExpr(rs.Value)}));" : $"{Indent}throw new Exception();",
            PilotIrBreakStmt => $"{Indent}break;",
            PilotIrContinueStmt => $"{Indent}continue;",
            PilotIrPassStmt => $"{Indent}// pass",
            PilotIrAssertStmt a => a.Message != null
                ? $"{Indent}if (!({EmitExpr(a.Condition)})) throw new InvalidOperationException({EmitExpr(a.Message)});"
                : $"{Indent}if (!({EmitExpr(a.Condition)})) throw new InvalidOperationException(\"Assertion failed\");",
            PilotIrComptimeStmt ct => string.Join(Environment.NewLine, ct.Body.Select(EmitStmt)),
            PilotIrExprStmt e => $"{Indent}{EmitExpr(e.Expression)};",
            _ => ""
        };

        private string EmitVarDecl(PilotIrVarDeclStmt v)
        {
            if (v.TupleDestructureNames.Count > 1)
            {
                string lhs = $"var ({string.Join(", ", v.TupleDestructureNames)})";
                string rhs = v.Initializer != null ? EmitExpr(v.Initializer) : "default";
                return $"{Indent}{lhs} = {rhs};";
            }

            string rhsVal = v.ComptimeFoldedLiteral ?? (v.Initializer != null ? EmitExpr(v.Initializer) : "default!");
            string typeStr = !string.IsNullOrEmpty(v.DeclaredType) ? MapType(v.DeclaredType) : "var";
            if (v.Kind == PilotVarDeclKind.Const && typeStr != "var" &&
                typeStr is "int" or "long" or "double" or "float" or "bool" or "string")
            {
                return $"{Indent}const {typeStr} {v.Name} = {rhsVal};";
            }
            if (typeStr != "var" && v.Initializer == null)
            {
                return $"{Indent}{typeStr} {v.Name} = default!;";
            }
            return $"{Indent}{typeStr} {v.Name} = {rhsVal};";
        }

        private string EmitFuncDef(PilotIrFuncDefStmt f, bool isInstanceMethod, string? extensionTargetType)
        {
            var paramList = new List<string>();
            for (int i = 0; i < f.Parameters.Count; i++)
            {
                var p = f.Parameters[i];
                if (p.Name is "self" or "this")
                {
                    if (extensionTargetType != null)
                        paramList.Add($"this {MapType(extensionTargetType)} self_val");
                    continue;
                }
                string pType = !string.IsNullOrEmpty(p.TypeAnnotation) ? MapType(p.TypeAnnotation) : "dynamic";
                string defPart = p.DefaultValue != null ? $" = {EmitExpr(p.DefaultValue)}" : "";
                paramList.Add($"{pType} {p.Name}{defPart}");
            }

            if (extensionTargetType != null && (paramList.Count == 0 || !paramList[0].StartsWith("this ")))
            {
                paramList.Insert(0, $"this {MapType(extensionTargetType)} self_val");
            }

            string retType = MapType(f.EffectiveReturnType);
            string staticMod = isInstanceMethod ? "" : "static ";
            string genPart = f.GenericParams.Count > 0 ? $"<{string.Join(", ", f.GenericParams)}>" : "";

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}public {staticMod}{retType} {f.Name}{genPart}({string.Join(", ", paramList)})");
            sb.AppendLine($"{Indent}{{");
            _indent++;
            bool prevExt = _inExtensionMethod;
            _inExtensionMethod = extensionTargetType != null;
            try
            {
                foreach (var req in f.RequiresClauses)
                {
                    sb.AppendLine($"{Indent}if (!({EmitExpr(req)})) throw new ArgumentException(\"Contract requires failed\");");
                }
                foreach (var s in f.Body)
                {
                    string line = EmitStmt(s);
                    if (!string.IsNullOrWhiteSpace(line))
                    {
                        sb.AppendLine(line);
                    }
                }
            }
            finally
            {
                _inExtensionMethod = prevExt;
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitClassDef(PilotIrClassDefStmt c)
        {
            var sb = new StringBuilder();
            string inheritance = c.BaseTypes.Count > 0 ? $" : {string.Join(", ", c.BaseTypes.Select(MapType))}" : "";
            string genPart = c.GenericParams.Count > 0 ? $"<{string.Join(", ", c.GenericParams)}>" : "";
            sb.AppendLine($"{Indent}public class {c.Name}{genPart}{inheritance}");
            sb.AppendLine($"{Indent}{{");
            _indent++;
            foreach (var f in c.Fields)
            {
                string defVal = f.DefaultValue != null ? $" = {EmitExpr(f.DefaultValue)};" : "";
                sb.AppendLine($"{Indent}public {MapType(f.TypeAnnotation)} {f.Name} {{ get; set; }}{defVal}");
            }
            foreach (var m in c.Methods)
            {
                bool isInstance = !m.IsStatic;
                sb.AppendLine(EmitFuncDef(m, isInstanceMethod: isInstance, extensionTargetType: null));
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitAdtDef(PilotIrAdtDefStmt adt)
        {
            var sb = new StringBuilder();
            string genPart = adt.GenericParams.Count > 0 ? $"<{string.Join(", ", adt.GenericParams)}>" : "";
            sb.AppendLine($"{Indent}public abstract record {adt.Name}{genPart};");
            foreach (var v in adt.Variants)
            {
                string fields = string.Join(", ", v.Fields.Select(f => $"{MapType(f.Type)} {f.Name}"));
                sb.AppendLine($"{Indent}public sealed record {v.Name}{genPart}({fields}) : {adt.Name}{genPart};");
            }
            return sb.ToString().TrimEnd();
        }

        private string EmitEnumDef(PilotIrEnumDefStmt en)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}public enum {en.Name}");
            sb.AppendLine($"{Indent}{{");
            _indent++;
            foreach (var (mName, mVal) in en.Members)
            {
                string valPart = mVal != null ? $" = {EmitExpr(mVal)}" : "";
                sb.AppendLine($"{Indent}{mName}{valPart},");
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitExtensionClass(PilotIrExtensionDefStmt ext)
        {
            var sb = new StringBuilder();
            string className = $"{ext.ExtensionName ?? ext.TargetType}PilotExtensions";
            sb.AppendLine($"public static class {className}");
            sb.AppendLine("{");
            _indent = 1;
            foreach (var m in ext.Methods)
            {
                sb.AppendLine(EmitFuncDef(m, isInstanceMethod: false, extensionTargetType: ext.TargetType));
            }
            _indent = 0;
            sb.Append("}");
            return sb.ToString();
        }

        private string EmitIfStmt(PilotIrIfStmt ifs)
        {
            var sb = new StringBuilder();
            if (!string.IsNullOrEmpty(ifs.OptionalBindingVar))
                sb.AppendLine($"{Indent}if ({EmitExpr(ifs.Condition)} is {{}} {ifs.OptionalBindingVar})");
            else
                sb.AppendLine($"{Indent}if ({EmitExpr(ifs.Condition)})");
            EmitBlock(sb, ifs.ThenBody);

            foreach (var (cond, body) in ifs.ElifBranches)
            {
                sb.AppendLine($"{Indent}else if ({EmitExpr(cond)})");
                EmitBlock(sb, body);
            }
            if (ifs.ElseBody.Count > 0)
            {
                sb.AppendLine($"{Indent}else");
                EmitBlock(sb, ifs.ElseBody);
            }
            return sb.ToString().TrimEnd();
        }

        private string EmitGuardStmt(PilotIrGuardStmt g)
        {
            var sb = new StringBuilder();
            if (g.IsSomeBinding && !string.IsNullOrEmpty(g.BoundVariable))
            {
                sb.AppendLine($"{Indent}var {g.BoundVariable} = {EmitExpr(g.ConditionOrSource)};");
                string cond = $"{g.BoundVariable} is null";
                if (g.PatternGuard != null)
                    cond += $" || !({EmitExpr(g.PatternGuard)})";
                sb.AppendLine($"{Indent}if ({cond})");
            }
            else
            {
                sb.AppendLine($"{Indent}if (!({EmitExpr(g.ConditionOrSource)}))");
            }
            EmitBlock(sb, g.ElseBody);
            return sb.ToString().TrimEnd();
        }

        private string EmitWhileStmt(PilotIrWhileStmt w)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}while ({EmitExpr(w.Condition)})");
            EmitBlock(sb, w.Body);
            return sb.ToString().TrimEnd();
        }

        private string EmitForStmt(PilotIrForStmt f)
        {
            var sb = new StringBuilder();
            string target = f.TargetVars.Count > 0 ? f.TargetVars[0] : "_item";
            if (f.Iterable is PilotIrRangeExpr r)
            {
                string cmp = r.IsInclusive ? "<=" : "<";
                string step = r.Step != null ? $"{target} += {EmitExpr(r.Step)}" : $"++{target}";
                sb.AppendLine($"{Indent}for (long {target} = {EmitExpr(r.Start)}; {target} {cmp} {EmitExpr(r.End)}; {step})");
            }
            else
            {
                sb.AppendLine($"{Indent}foreach (var {target} in {EmitExpr(f.Iterable)})");
            }
            sb.AppendLine($"{Indent}{{");
            _indent++;
            if (f.WhereFilter != null)
            {
                sb.AppendLine($"{Indent}if (!({EmitExpr(f.WhereFilter)})) continue;");
            }
            foreach (var s in f.Body)
            {
                string line = EmitStmt(s);
                if (!string.IsNullOrWhiteSpace(line))
                    sb.AppendLine(line);
            }
            _indent--;
            sb.AppendLine($"{Indent}}}");
            foreach (var post in f.CompletionThenBody)
                sb.AppendLine(EmitStmt(post));
            return sb.ToString().TrimEnd();
        }

        private string EmitMatchStmt(PilotIrMatchStmt m)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}switch ({EmitExpr(m.Scrutinee)})");
            sb.AppendLine($"{Indent}{{");
            _indent++;
            foreach (var arm in m.Arms)
            {
                string pat = EmitPattern(arm.Pattern);
                string guard = arm.Guard != null ? $" when {EmitExpr(arm.Guard)}" : "";
                string caseHeader = pat == "_" ? $"default:" : $"case {pat}{guard}:";
                sb.AppendLine($"{Indent}{caseHeader}");
                sb.AppendLine($"{Indent}{{");
                _indent++;
                bool hasTerminator = false;
                foreach (var s in arm.Body)
                {
                    if (s is PilotIrReturnStmt or PilotIrBreakStmt or PilotIrContinueStmt or PilotIrRaiseStmt)
                        hasTerminator = true;
                    string line = EmitStmt(s);
                    if (!string.IsNullOrWhiteSpace(line))
                        sb.AppendLine(line);
                }
                if (!hasTerminator)
                    sb.AppendLine($"{Indent}break;");
                _indent--;
                sb.AppendLine($"{Indent}}}");
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitPattern(PilotIrPattern pat) => pat switch
        {
            PilotIrWildcardPattern => "_",
            PilotIrLiteralPattern lit => EmitExpr(lit.Literal),
            PilotIrBindingPattern b => !string.IsNullOrEmpty(b.TypeName) ? $"{MapType(b.TypeName)} {b.Name}" : $"var {b.Name}",
            PilotIrOptionPattern opt => opt.IsSome
                ? (opt.InnerPattern is PilotIrBindingPattern bp ? $"{{}} {bp.Name}" : "not null")
                : "null",
            PilotIrVariantPattern vp => $"{vp.VariantName}({string.Join(", ", vp.Arguments.Select(a => EmitPattern(a.SubPattern)))})",
            PilotIrRangePattern rp => $">= {EmitExpr(rp.Start)} and {(rp.IsInclusive ? "<=" : "<")} {EmitExpr(rp.End)}",
            PilotIrRelationalPattern rel => $"{rel.Operator} {EmitExpr(rel.Value)}",
            PilotIrOrPattern orp => string.Join(" or ", orp.Alternatives.Select(EmitPattern)),
            _ => "_"
        };

        private string EmitTryStmt(PilotIrTryStmt t)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}try");
            EmitBlock(sb, t.TryBody);
            foreach (var c in t.Catches)
            {
                string exType = !string.IsNullOrEmpty(c.ExceptionType) && c.ExceptionType != "Error" ? c.ExceptionType : "Exception";
                string varPart = !string.IsNullOrEmpty(c.VariableName) ? $" {c.VariableName}" : "";
                sb.AppendLine($"{Indent}catch ({exType}{varPart})");
                EmitBlock(sb, c.Body);
            }
            if (t.FinallyBody.Count > 0)
            {
                sb.AppendLine($"{Indent}finally");
                EmitBlock(sb, t.FinallyBody);
            }
            return sb.ToString().TrimEnd();
        }

        private void EmitBlock(StringBuilder sb, List<PilotIrStmt> stmts)
        {
            sb.AppendLine($"{Indent}{{");
            _indent++;
            foreach (var s in stmts)
            {
                string line = EmitStmt(s);
                if (!string.IsNullOrWhiteSpace(line))
                    sb.AppendLine(line);
            }
            _indent--;
            sb.AppendLine($"{Indent}}}");
        }

        public string EmitExpr(PilotIrExpr expr, PilotIrPrecedence parentPrec = PilotIrPrecedence.Lowest)
        {
            string core = expr switch
            {
                PilotIrLiteralExpr lit => EmitLiteral(lit),
                PilotIrInterpolatedStringExpr fstr => EmitInterpolatedString(fstr),
                PilotIrIdentifierExpr id => id.Name switch
                {
                    "null" or "none" or "None" or "undefined" => "null",
                    "self" or "this" => _inExtensionMethod ? "self_val" : "this",
                    _ => id.Name
                },
                PilotIrUnaryExpr u => EmitUnary(u),
                PilotIrBinaryExpr b => EmitBinary(b),
                PilotIrRangeExpr r => EmitRangeEnumerable(r),
                PilotIrCallExpr c => EmitCall(c),
                PilotIrMemberAccessExpr m => EmitMemberAccess(m),
                PilotIrIndexExpr ix => $"{EmitExpr(ix.Target, PilotIrPrecedence.Postfix)}[{EmitExpr(ix.Index)}]",
                PilotIrPostfixTryExpr pt => EmitPostfixTry(pt),
                PilotIrCollectionExpr col => EmitCollection(col),
                PilotIrLambdaExpr lam => $"({string.Join(", ", lam.Parameters.Select(p => p.Name))}) => {(lam.BodyExpr != null ? EmitExpr(lam.BodyExpr) : "{}")}",
                PilotIrForExpr fe => EmitForExpr(fe),
                PilotIrIfExpr ie => $"{EmitExpr(ie.Condition, PilotIrPrecedence.Conditional)} ? {EmitExpr(ie.ThenBranch, PilotIrPrecedence.Conditional)} : {EmitExpr(ie.ElseBranch, PilotIrPrecedence.Conditional)}",
                _ => "default"
            };

            if (parentPrec != PilotIrPrecedence.Lowest && expr.Precedence < parentPrec)
                return $"({core})";
            return core;
        }

        private static string EmitLiteral(PilotIrLiteralExpr lit) => lit.Kind switch
        {
            PilotLiteralKind.Null or PilotLiteralKind.None => "null",
            PilotLiteralKind.Boolean => lit.RawValue is "true" or "True" ? "true" : "false",
            PilotLiteralKind.DimensionalUnit => (lit.NormalizedNumericValue ?? 0.0).ToString(System.Globalization.CultureInfo.InvariantCulture),
            _ => lit.RawValue
        };

        private string EmitInterpolatedString(PilotIrInterpolatedStringExpr fstr)
        {
            var sb = new StringBuilder("$\"");
            foreach (var part in fstr.Parts)
            {
                if (!part.IsInterpolation)
                    sb.Append(part.LiteralText);
                else
                {
                    string inner = part.Expression != null ? EmitExpr(part.Expression) : part.LiteralText;
                    string spec = !string.IsNullOrEmpty(part.FormatSpec) ? $":{part.FormatSpec}" : "";
                    sb.Append($"{{{inner}{spec}}}");
                }
            }
            sb.Append('"');
            return sb.ToString();
        }

        private string EmitUnary(PilotIrUnaryExpr u)
        {
            string op = u.Operator is "not" ? "!" : u.Operator;
            return $"{op}{EmitExpr(u.Operand, u.Precedence)}";
        }

        private string EmitBinary(PilotIrBinaryExpr b)
        {
            if (b.Operator == "**")
                return $"Math.Pow({EmitExpr(b.Left)}, {EmitExpr(b.Right)})";
            if (b.Operator == "|>")
                return $"{EmitExpr(b.Right, PilotIrPrecedence.Postfix)}({EmitExpr(b.Left)})";

            string csOp = b.Operator switch
            {
                "and" => "&&",
                "or" => "||",
                "//" => "/",
                _ => b.Operator
            };
            string left = EmitExpr(b.Left, b.Precedence);
            var rightMinPrec = b.Operator is "-" or "/" or "//" or "%"
                ? (PilotIrPrecedence)((int)b.Precedence + 1)
                : b.Precedence;
            string right = EmitExpr(b.Right, rightMinPrec);
            return $"{left} {csOp} {right}";
        }

        private string EmitRangeEnumerable(PilotIrRangeExpr r)
        {
            string start = EmitExpr(r.Start);
            string end = EmitExpr(r.End);
            string count = r.IsInclusive ? $"({end} - {start} + 1)" : $"({end} - {start})";
            return $"Enumerable.Range((int)({start}), (int)({count}))";
        }

        private string EmitCall(PilotIrCallExpr c)
        {
            var renderedArgs = c.Arguments.Select(a =>
                !string.IsNullOrEmpty(a.Name) ? $"{a.Name}: {EmitExpr(a.Value)}" : EmitExpr(a.Value)).ToList();

            if (PilotLibraryMapper.TryFormatAstLibraryCall(c.LibraryCallKind, renderedArgs, "csharp", out string mapped))
                return mapped;

            if (c.Callee is PilotIrMemberAccessExpr mem && mem.MemberName is "append" or "push")
            {
                return $"{EmitExpr(mem.Target, PilotIrPrecedence.Postfix)}.Add({string.Join(", ", renderedArgs)})";
            }

            string callee = EmitExpr(c.Callee, PilotIrPrecedence.Postfix);
            if (_knownTypeDecls.Contains(callee) || (callee.Length > 0 && char.IsUpper(callee[0]) && !callee.Contains('.')))
            {
                return $"new {callee}({string.Join(", ", renderedArgs)})";
            }
            return $"{callee}({string.Join(", ", renderedArgs)})";
        }

        private string EmitMemberAccess(PilotIrMemberAccessExpr m)
        {
            string target = EmitExpr(m.Target, PilotIrPrecedence.Postfix);
            if (PilotLibraryMapper.TryFormatAstMemberAccess(target, m.MemberName, "csharp", out string mapped))
                return mapped;
            string op = m.IsSafeNavigation ? "?." : ".";
            return $"{target}{op}{m.MemberName}";
        }

        private string EmitPostfixTry(PilotIrPostfixTryExpr pt)
        {
            string op = EmitExpr(pt.Operand, PilotIrPrecedence.Coalesce);
            if (!pt.IsQuestionPropagate && pt.CatchFallbackExpr != null)
                return $"({op} ?? {EmitExpr(pt.CatchFallbackExpr, PilotIrPrecedence.Coalesce)})";
            return op;
        }

        private string EmitCollection(PilotIrCollectionExpr col) => col.Kind switch
        {
            PilotCollectionKind.List => $"new List<dynamic> {{ {string.Join(", ", col.Elements.Select(e => EmitExpr(e)))} }}",
            PilotCollectionKind.Tuple => $"({string.Join(", ", col.Elements.Select(e => EmitExpr(e)))})",
            PilotCollectionKind.Set => $"new HashSet<dynamic> {{ {string.Join(", ", col.Elements.Select(e => EmitExpr(e)))} }}",
            PilotCollectionKind.Map => $"new Dictionary<dynamic, dynamic> {{ {string.Join(", ", col.MapEntries.Select(kv => $"[{EmitExpr(kv.Key)}] = {EmitExpr(kv.Value)}"))} }}",
            _ => "new List<dynamic>()"
        };

        private string EmitForExpr(PilotIrForExpr fe)
        {
            string target = fe.TargetVars.Count > 0 ? fe.TargetVars[0] : "x";
            string src = EmitExpr(fe.Iterable, PilotIrPrecedence.Postfix);
            string chain = src;
            if (fe.FilterCondition != null)
                chain += $".Where({target} => {EmitExpr(fe.FilterCondition)})";
            chain += $".Select({target} => {EmitExpr(fe.YieldExpr)}).ToList()";
            return chain;
        }

        private string MapType(string pilotType)
        {
            string t = pilotType.Trim();
            if (_options.TypeMappings.TryGetValue(t, out var mapped))
                return mapped;

            if (t.EndsWith("?"))
                return $"{MapType(t[..^1])}?";
            if (t.Contains('!'))
                return MapType(t.Split('!', 2)[0].Trim());
            if (t.StartsWith("list[") && t.EndsWith("]"))
                return $"List<{MapType(t[5..^1])}>";
            if (t.StartsWith("set[") && t.EndsWith("]"))
                return $"HashSet<{MapType(t[4..^1])}>";
            if (t.StartsWith("map[") && t.EndsWith("]"))
            {
                var parts = t[4..^1].Split(',', 2);
                return parts.Length == 2
                    ? $"Dictionary<{MapType(parts[0])}, {MapType(parts[1])}>"
                    : "Dictionary<object, object>";
            }

            return t switch
            {
                "i8" => "sbyte",
                "u8" => "byte",
                "i16" => "short",
                "u16" => "ushort",
                "i32" or "int" => "int",
                "u32" or "uint" => "uint",
                "i64" => "long",
                "u64" => "ulong",
                "f32" or "float" => "float",
                "f64" or "double" => "double",
                "decimal" => "decimal",
                "str" or "string" => "string",
                "bool" => "bool",
                "void" => "void",
                "any" or "auto" => "dynamic",
                "list" => "List<object>",
                "map" => "Dictionary<object, object>",
                "set" => "HashSet<object>",
                _ => t
            };
        }
    }
}
