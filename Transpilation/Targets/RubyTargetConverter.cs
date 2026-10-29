using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text;
using Pilot.Analysis;
using Pilot.Transpilation.Ingesters;
using Pilot.Transpilation.Ir;

namespace Pilot.Transpilation.Targets;

/// <summary>
/// Milestone 20.3 &amp; 20.4: AST-driven bidirectional code converter between Pilot and Ruby 3.2+.
/// Emits directly from <see cref="PilotIrModule"/> with precedence-aware parenthesization,
/// <c>Struct.new</c> ADTs, open-class <c>extend</c> blocks, <c>.. / ...</c> ranges,
/// <c>.select / .map</c> comprehensions, <c>#{}</c> string interpolation, and AST-scoped <see cref="PilotLibraryMapper"/> calls.
/// </summary>
public sealed class RubyTargetConverter : ITargetLanguageConverter
{
    public string Language { get; }
    public string TargetLanguage => Language;
    public string SourceLanguage => Language;

    public RubyTargetConverter(string alias = "ruby")
    {
        Language = alias;
    }

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
            sb.AppendLine("# frozen_string_literal: true");
            sb.AppendLine("# Code transpiled & optimized from Pilot to Ruby 3.2+");
            if (module.UsesJsonPrimitives)
                sb.AppendLine("require \"json\"");
            sb.AppendLine();
        }
        else if (module.UsesJsonPrimitives)
        {
            sb.AppendLine("require \"json\"");
            sb.AppendLine();
        }

        var emitter = new RubyIrEmitter(module, options);
        string body = emitter.EmitModule();

        if (!string.IsNullOrWhiteSpace(options.TargetNamespaceOrModule))
        {
            sb.AppendLine($"module {options.TargetNamespaceOrModule}");
            foreach (var line in body.TrimEnd().Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None))
            {
                sb.AppendLine(string.IsNullOrWhiteSpace(line) ? "" : options.IndentString + line);
            }
            sb.AppendLine("end");
        }
        else
        {
            sb.Append(body);
        }

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options) =>
        ForeignAstIngester.Ingest(foreignSource, SourceLanguage, options);

    private sealed class RubyIrEmitter
    {
        private readonly PilotIrModule _module;
        private readonly TranspilationOptions _options;
        private readonly HashSet<string> _knownClassesAndVariants = new(StringComparer.Ordinal);
        private readonly HashSet<string> _knownFunctions = new(StringComparer.Ordinal);
        private int _indent;
        private bool _inMethod;

        public RubyIrEmitter(PilotIrModule module, TranspilationOptions options)
        {
            _module = module;
            _options = options;

            foreach (var c in module.Statements.OfType<PilotIrClassDefStmt>())
                _knownClassesAndVariants.Add(c.Name);

            foreach (var adt in module.Statements.OfType<PilotIrAdtDefStmt>())
            {
                foreach (var v in adt.Variants)
                    _knownClassesAndVariants.Add(v.Name);
            }

            foreach (var fn in module.Statements.OfType<PilotIrFuncDefStmt>())
                _knownFunctions.Add(fn.Name);
        }

        private string Indent => string.Concat(Enumerable.Repeat(_options.IndentString, _indent));

        public string EmitModule()
        {
            var sb = new StringBuilder();

            foreach (var imp in _module.Imports)
            {
                if (imp.IsMappedNativeEquivalent)
                    continue;
                string line = EmitImport(imp);
                if (!string.IsNullOrWhiteSpace(line))
                    sb.AppendLine(line);
            }

            for (int i = 0; i < _module.Statements.Count; i++)
            {
                var stmt = _module.Statements[i];
                EmitLeadingComments(sb, stmt);
                string emitted = EmitStmt(stmt);
                if (!string.IsNullOrWhiteSpace(emitted))
                {
                    sb.AppendLine(emitted);
                    if (stmt is PilotIrFuncDefStmt or PilotIrClassDefStmt or PilotIrAdtDefStmt or
                                PilotIrEnumDefStmt or PilotIrExtensionDefStmt &&
                        i + 1 < _module.Statements.Count)
                    {
                        sb.AppendLine();
                    }
                }
            }

            return sb.ToString();
        }

        private void EmitLeadingComments(StringBuilder sb, PilotIrNode node)
        {
            foreach (var t in node.LeadingTrivia)
            {
                if (t.Kind is PilotTriviaKind.LineComment or PilotTriviaKind.DocComment)
                {
                    string clean = t.Text.TrimStart('/');
                    sb.AppendLine($"{Indent}#{clean}");
                }
            }
        }

        private string EmitImport(PilotIrImportStmt imp)
        {
            if (string.IsNullOrWhiteSpace(imp.ModulePath))
                return "";
            return $"{Indent}require \"{imp.ModulePath.Replace('.', '/')}\"";
        }

        public string EmitStmt(PilotIrStmt stmt) => stmt switch
        {
            PilotIrCommentStmt c => $"{Indent}#{c.CommentText.TrimStart('/')}",
            PilotIrImportStmt imp => imp.IsMappedNativeEquivalent ? "" : EmitImport(imp),
            PilotIrVarDeclStmt v => EmitVarDecl(v),
            PilotIrAssignStmt a => EmitAssignStmt(a),
            PilotIrFuncDefStmt f => EmitFuncDef(f, isMethod: false, isStaticMethod: false),
            PilotIrClassDefStmt c => EmitClassDef(c),
            PilotIrAdtDefStmt adt => EmitAdtDef(adt),
            PilotIrEnumDefStmt en => EmitEnumDef(en),
            PilotIrTypeAliasStmt ta => $"{Indent}# type {ta.Name} = {ta.TargetType}",
            PilotIrExtensionDefStmt ext => EmitExtensionDef(ext),
            PilotIrIfStmt ifs => EmitIfStmt(ifs),
            PilotIrGuardStmt g => EmitGuardStmt(g),
            PilotIrWhileStmt w => EmitWhileStmt(w),
            PilotIrForStmt f => EmitForStmt(f),
            PilotIrMatchStmt m => EmitMatchStmt(m),
            PilotIrTryStmt t => EmitTryStmt(t),
            PilotIrDeferStmt d => EmitDeferStmt(d),
            PilotIrReturnStmt r => r.Value != null ? $"{Indent}return {EmitExpr(r.Value)}" : $"{Indent}return",
            PilotIrYieldStmt y => y.Value != null ? $"{Indent}yield {EmitExpr(y.Value)}" : $"{Indent}yield",
            PilotIrRaiseStmt rs => rs.Value != null ? $"{Indent}raise {EmitExpr(rs.Value)}" : $"{Indent}raise RuntimeError",
            PilotIrBreakStmt b => b.Value != null ? $"{Indent}break {EmitExpr(b.Value)}" : $"{Indent}break",
            PilotIrContinueStmt => $"{Indent}next",
            PilotIrPassStmt => $"{Indent}# pass",
            PilotIrAssertStmt a => EmitAssertStmt(a),
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
                return $"{Indent}{lhs} = {rhs}";
            }

            string rhsVal = v.ComptimeFoldedLiteral ?? (v.Initializer != null ? EmitExpr(v.Initializer) : "nil");
            string varName = v.Name;
            if (_inMethod && varName.Length > 0 && char.IsUpper(varName[0]))
            {
                varName = char.ToLowerInvariant(varName[0]) + varName[1..];
            }
            return $"{Indent}{varName} = {rhsVal}";
        }

        private string EmitAssignStmt(PilotIrAssignStmt a)
        {
            if (a.Operator == "//=")
            {
                string lhs = EmitExpr(a.Target);
                return $"{Indent}{lhs} = ({lhs}).div({EmitExpr(a.Value)})";
            }
            return $"{Indent}{EmitExpr(a.Target)} {a.Operator} {EmitExpr(a.Value)}";
        }

        private string EmitFuncDef(PilotIrFuncDefStmt f, bool isMethod, bool isStaticMethod)
        {
            var sb = new StringBuilder();
            var paramList = new List<string>();

            foreach (var p in f.Parameters)
            {
                if (p.Name is "self" or "this")
                    continue;

                string pStr = (p.IsVariadic ? "*" : "") + p.Name;
                if (p.DefaultValue != null)
                    pStr += $" = {EmitExpr(p.DefaultValue)}";
                paramList.Add(pStr);
            }

            string fnName = isMethod && f.Name is "__init__" or "init" ? "initialize" : f.Name;
            string receiverPrefix = isStaticMethod ? "self." : "";
            string paramSuffix = paramList.Count > 0 ? $"({string.Join(", ", paramList)})" : "";

            sb.AppendLine($"{Indent}def {receiverPrefix}{fnName}{paramSuffix}");
            _indent++;
            bool prevInMethod = _inMethod;
            _inMethod = true;
            try
            {
                foreach (var req in f.RequiresClauses)
                {
                    sb.AppendLine($"{Indent}raise ArgumentError, \"Contract requires failed\" unless {EmitExpr(req)}");
                }
                foreach (var stmt in f.Body)
                {
                    EmitLeadingComments(sb, stmt);
                    string line = EmitStmt(stmt);
                    if (!string.IsNullOrWhiteSpace(line))
                        sb.AppendLine(line);
                }
            }
            finally
            {
                _inMethod = prevInMethod;
            }
            _indent--;
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        private string EmitClassDef(PilotIrClassDefStmt c)
        {
            if (c.Kind == PilotTypeDeclKind.Record && c.Methods.Count == 0 && c.Fields.Count > 0)
            {
                string syms = string.Join(", ", c.Fields.Select(f => ":" + f.Name));
                return $"{Indent}{c.Name} = Struct.new({syms})";
            }

            var sb = new StringBuilder();
            string inheritance = c.BaseTypes.Count > 0 ? $" < {c.BaseTypes[0]}" : "";
            sb.AppendLine($"{Indent}class {c.Name}{inheritance}");
            _indent++;

            if (c.Fields.Count > 0)
            {
                sb.AppendLine($"{Indent}attr_accessor {string.Join(", ", c.Fields.Select(f => ":" + f.Name))}");
                if (!c.Methods.Any(m => m.Name is "__init__" or "init" or "initialize"))
                {
                    var ctorParams = c.Fields.Select(f =>
                        f.DefaultValue != null
                            ? $"{f.Name} = {EmitExpr(f.DefaultValue)}"
                            : $"{f.Name} = nil");
                    sb.AppendLine($"{Indent}def initialize({string.Join(", ", ctorParams)})");
                    _indent++;
                    foreach (var f in c.Fields)
                        sb.AppendLine($"{Indent}@{f.Name} = {f.Name}");
                    _indent--;
                    sb.AppendLine($"{Indent}end");
                }
            }

            foreach (var m in c.Methods)
            {
                sb.AppendLine(EmitFuncDef(m, isMethod: true, isStaticMethod: m.IsStatic));
            }

            _indent--;
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        private string EmitAdtDef(PilotIrAdtDefStmt adt)
        {
            var sb = new StringBuilder();
            foreach (var v in adt.Variants)
            {
                if (v.Fields.Count == 0)
                {
                    sb.AppendLine($"{Indent}{v.Name} = Class.new");
                }
                else
                {
                    string syms = string.Join(", ", v.Fields.Select(f => ":" + f.Name));
                    sb.AppendLine($"{Indent}{v.Name} = Struct.new({syms})");
                }
            }

            if (adt.Variants.Count > 0)
            {
                sb.Append($"{Indent}{adt.Name} = [{string.Join(", ", adt.Variants.Select(v => v.Name))}].freeze");
            }

            return sb.ToString().TrimEnd();
        }

        private string EmitEnumDef(PilotIrEnumDefStmt en)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}module {en.Name}");
            _indent++;
            for (int i = 0; i < en.Members.Count; i++)
            {
                var (mName, mVal) = en.Members[i];
                string valStr = mVal != null ? EmitExpr(mVal) : i.ToString(CultureInfo.InvariantCulture);
                sb.AppendLine($"{Indent}{mName} = {valStr}");
            }
            _indent--;
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        private string EmitExtensionDef(PilotIrExtensionDefStmt ext)
        {
            var sb = new StringBuilder();
            string rubyClass = MapExtensionTargetToRubyClass(ext.TargetType);
            sb.AppendLine($"{Indent}class {rubyClass}");
            _indent++;
            foreach (var m in ext.Methods)
            {
                sb.AppendLine(EmitFuncDef(m, isMethod: true, isStaticMethod: false));
            }
            _indent--;
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        private static string MapExtensionTargetToRubyClass(string pilotType)
        {
            string t = pilotType.Trim().TrimEnd('?');
            if (t.StartsWith("list[") || t.StartsWith("List[") || t.StartsWith("["))
                return "Array";
            if (t.StartsWith("map[") || t.StartsWith("Map["))
                return "Hash";

            return t switch
            {
                "i8" or "i16" or "i32" or "i64" or "u8" or "u16" or "u32" or "u64" or "int" or "uint" => "Integer",
                "f32" or "f64" or "float" or "double" => "Float",
                "str" or "string" => "String",
                "bool" => "Object",
                "list" or "List" => "Array",
                "map" or "Map" or "dict" or "Dict" => "Hash",
                _ => t
            };
        }

        private string EmitIfStmt(PilotIrIfStmt ifs)
        {
            var sb = new StringBuilder();
            if (!string.IsNullOrEmpty(ifs.OptionalBindingVar))
            {
                sb.AppendLine($"{Indent}{ifs.OptionalBindingVar} = {EmitExpr(ifs.Condition)}");
                sb.AppendLine($"{Indent}if !{ifs.OptionalBindingVar}.nil?");
            }
            else
            {
                sb.AppendLine($"{Indent}if {EmitExpr(ifs.Condition)}");
            }
            EmitSuite(sb, ifs.ThenBody);

            foreach (var (cond, body) in ifs.ElifBranches)
            {
                sb.AppendLine($"{Indent}elsif {EmitExpr(cond)}");
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
                sb.AppendLine($"{Indent}{g.BoundVariable} = {EmitExpr(g.ConditionOrSource)}");
                string cond = $"{g.BoundVariable}.nil?";
                if (g.PatternGuard != null)
                    cond += $" || !({EmitExpr(g.PatternGuard)})";
                sb.AppendLine($"{Indent}if {cond}");
            }
            else
            {
                sb.AppendLine($"{Indent}unless {EmitExpr(g.ConditionOrSource)}");
            }
            EmitSuite(sb, g.ElseBody);
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        private string EmitWhileStmt(PilotIrWhileStmt w)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}while {EmitExpr(w.Condition)}");
            EmitSuite(sb, w.Body);
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        private string EmitForStmt(PilotIrForStmt f)
        {
            var sb = new StringBuilder();
            string target = f.TargetVars.Count > 0 ? string.Join(", ", f.TargetVars) : "_item";
            string iterable = f.Iterable is PilotIrRangeExpr r ? EmitRangeRaw(r) : EmitExpr(f.Iterable);

            sb.AppendLine($"{Indent}for {target} in {iterable}");
            _indent++;
            if (f.WhereFilter != null)
            {
                sb.AppendLine($"{Indent}next unless {EmitExpr(f.WhereFilter)}");
            }
            foreach (var s in f.Body)
            {
                EmitLeadingComments(sb, s);
                string line = EmitStmt(s);
                if (!string.IsNullOrWhiteSpace(line))
                    sb.AppendLine(line);
            }
            _indent--;
            sb.AppendLine($"{Indent}end");

            foreach (var post in f.CompletionThenBody)
                sb.AppendLine(EmitStmt(post));

            return sb.ToString().TrimEnd();
        }

        private string EmitRangeRaw(PilotIrRangeExpr r)
        {
            string op = r.IsInclusive ? ".." : "...";
            string core = $"{EmitExpr(r.Start, PilotIrPrecedence.Range)}{op}{EmitExpr(r.End, PilotIrPrecedence.Range)}";
            if (r.Step != null)
                return $"({core}).step({EmitExpr(r.Step)})";
            return core;
        }

        private string EmitMatchStmt(PilotIrMatchStmt m)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}case {EmitExpr(m.Scrutinee)}");
            foreach (var arm in m.Arms)
            {
                if (arm.Pattern is PilotIrWildcardPattern && arm.Guard == null)
                {
                    sb.AppendLine($"{Indent}else");
                }
                else if (arm.Pattern is PilotIrVariantPattern vp && vp.Arguments.Count > 0)
                {
                    string subPats = string.Join(", ", vp.Arguments.Select(a => EmitRubyPattern(a.SubPattern)));
                    string guard = arm.Guard != null ? $" if {EmitExpr(arm.Guard)}" : "";
                    sb.AppendLine($"{Indent}in {vp.VariantName}[{subPats}]{guard}");
                }
                else if (arm.Pattern is PilotIrBindingPattern bp)
                {
                    string guard = arm.Guard != null ? $" if {EmitExpr(arm.Guard)}" : "";
                    sb.AppendLine($"{Indent}in {bp.Name}{guard}");
                }
                else
                {
                    string whenVal = EmitRubyWhenClause(arm.Pattern);
                    sb.AppendLine($"{Indent}when {whenVal}");
                }

                if (arm.ExpressionBody != null && arm.Body.Count == 0)
                {
                    _indent++;
                    sb.AppendLine($"{Indent}{EmitExpr(arm.ExpressionBody)}");
                    _indent--;
                }
                else
                {
                    EmitSuite(sb, arm.Body);
                }
            }
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        private string EmitRubyPattern(PilotIrPattern pat) => pat switch
        {
            PilotIrWildcardPattern => "_",
            PilotIrLiteralPattern lit => EmitExpr(lit.Literal),
            PilotIrBindingPattern b => b.Name,
            PilotIrVariantPattern vp => vp.Arguments.Count > 0
                ? $"{vp.VariantName}[{string.Join(", ", vp.Arguments.Select(a => EmitRubyPattern(a.SubPattern)))}]"
                : vp.VariantName,
            _ => "_"
        };

        private string EmitRubyWhenClause(PilotIrPattern pat) => pat switch
        {
            PilotIrLiteralPattern lit => EmitExpr(lit.Literal),
            PilotIrOptionPattern opt => opt.IsSome ? "Object" : "nil",
            PilotIrVariantPattern vp => vp.VariantName,
            PilotIrRangePattern rp => EmitRangeRaw(new PilotIrRangeExpr { Start = rp.Start, End = rp.End, IsInclusive = rp.IsInclusive }),
            PilotIrOrPattern orp => string.Join(", ", orp.Alternatives.Select(EmitRubyWhenClause)),
            _ => "Object"
        };

        private string EmitTryStmt(PilotIrTryStmt t)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}begin");
            EmitSuite(sb, t.TryBody);
            foreach (var c in t.Catches)
            {
                string exType = !string.IsNullOrEmpty(c.ExceptionType) && c.ExceptionType is not ("Error" or "Exception")
                    ? $" {c.ExceptionType}"
                    : " StandardError";
                string varPart = !string.IsNullOrEmpty(c.VariableName) ? $" => {c.VariableName}" : "";
                sb.AppendLine($"{Indent}rescue{exType}{varPart}");
                EmitSuite(sb, c.Body);
            }
            if (t.FinallyBody.Count > 0)
            {
                sb.AppendLine($"{Indent}ensure");
                EmitSuite(sb, t.FinallyBody);
            }
            sb.Append($"{Indent}end");
            return sb.ToString();
        }

        private string EmitDeferStmt(PilotIrDeferStmt d)
        {
            if (d.Body.Count == 0)
                return $"{Indent}# defer";
            return string.Join(Environment.NewLine, d.Body.Select(EmitStmt));
        }

        private string EmitAssertStmt(PilotIrAssertStmt a)
        {
            string msg = a.Message != null ? EmitExpr(a.Message) : "\"Assertion failed\"";
            return $"{Indent}raise {msg} unless {EmitExpr(a.Condition)}";
        }

        private void EmitSuite(StringBuilder sb, List<PilotIrStmt> stmts)
        {
            _indent++;
            foreach (var s in stmts)
            {
                EmitLeadingComments(sb, s);
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
                    "true" or "True" => "true",
                    "false" or "False" => "false",
                    "null" or "none" or "None" or "undefined" => "nil",
                    "self" or "this" => "self",
                    _ => id.Name
                },
                PilotIrUnaryExpr u => EmitUnary(u),
                PilotIrBinaryExpr b => EmitBinary(b),
                PilotIrRangeExpr r => $"({EmitRangeRaw(r)})",
                PilotIrCallExpr c => EmitCall(c),
                PilotIrMemberAccessExpr m => EmitMemberAccess(m),
                PilotIrIndexExpr ix => EmitIndex(ix),
                PilotIrPostfixTryExpr pt => EmitPostfixTry(pt),
                PilotIrCollectionExpr col => EmitCollection(col),
                PilotIrLambdaExpr lam => EmitLambda(lam),
                PilotIrForExpr fe => EmitForExpr(fe),
                PilotIrIfExpr ie => $"{EmitExpr(ie.Condition, PilotIrPrecedence.Conditional)} ? {EmitExpr(ie.ThenBranch, PilotIrPrecedence.Conditional)} : {EmitExpr(ie.ElseBranch, PilotIrPrecedence.Conditional)}",
                PilotIrMatchExpr me => EmitMatchExpr(me),
                PilotIrSqlQueryExpr sq => EmitSqlQueryExpr(sq),
                _ => "nil"
            };

            if (parentPrec != PilotIrPrecedence.Lowest && expr.Precedence < parentPrec)
                return $"({core})";
            return core;
        }

        private static string EmitLiteral(PilotIrLiteralExpr lit) => lit.Kind switch
        {
            PilotLiteralKind.Boolean => lit.RawValue is "true" or "True" ? "true" : "false",
            PilotLiteralKind.Null or PilotLiteralKind.None => "nil",
            PilotLiteralKind.Keyword => lit.RawValue.StartsWith(":") ? lit.RawValue : $":{lit.RawValue}",
            PilotLiteralKind.DimensionalUnit => (lit.NormalizedNumericValue ?? 0.0).ToString(CultureInfo.InvariantCulture),
            _ => lit.RawValue
        };

        private string EmitInterpolatedString(PilotIrInterpolatedStringExpr fstr)
        {
            var sb = new StringBuilder("\"");
            foreach (var part in fstr.Parts)
            {
                if (!part.IsInterpolation)
                {
                    sb.Append(part.LiteralText.Replace("#{", "\\#{"));
                }
                else
                {
                    string inner = part.Expression != null ? EmitExpr(part.Expression) : part.LiteralText;
                    if (!string.IsNullOrEmpty(part.FormatSpec))
                        sb.Append($"#{{sprintf(\"%{part.FormatSpec}\", {inner})}}");
                    else
                        sb.Append($"#{{{inner}}}");
                }
            }
            sb.Append('"');
            return sb.ToString();
        }

        private string EmitUnary(PilotIrUnaryExpr u)
        {
            string op = u.Operator is "not" or "!" ? "!" : u.Operator;
            return $"{op}{EmitExpr(u.Operand, u.Precedence)}";
        }

        private string EmitBinary(PilotIrBinaryExpr b)
        {
            if (b.Operator == "|>")
                return $"{EmitExpr(b.Right, PilotIrPrecedence.Postfix)}({EmitExpr(b.Left)})";
            if (b.Operator == "<|")
                return $"{EmitExpr(b.Left, PilotIrPrecedence.Postfix)}({EmitExpr(b.Right)})";
            if (b.Operator == "//")
                return $"({EmitExpr(b.Left, PilotIrPrecedence.Postfix)}).div({EmitExpr(b.Right)})";
            if (b.Operator == "??")
            {
                string l = EmitExpr(b.Left, PilotIrPrecedence.Postfix);
                string r = EmitExpr(b.Right, PilotIrPrecedence.Conditional);
                return $"({l}.nil? ? {r} : {l})";
            }

            if (b.Operator is "in" or "not in")
            {
                string inc = $"{EmitExpr(b.Right, PilotIrPrecedence.Postfix)}.include?({EmitExpr(b.Left)})";
                return b.Operator == "not in" ? $"!{inc}" : inc;
            }

            string rbOp = b.Operator switch
            {
                "and" or "&&" => "&&",
                "or" or "||" => "||",
                "is" => "==",
                "is not" => "!=",
                _ => b.Operator
            };

            string left = EmitExpr(b.Left, b.Precedence);
            var rightMinPrec = b.Operator is "-" or "/" or "%"
                ? (PilotIrPrecedence)((int)b.Precedence + 1)
                : b.Precedence;
            string right = EmitExpr(b.Right, rightMinPrec);
            return $"{left} {rbOp} {right}";
        }

        private string EmitCall(PilotIrCallExpr c)
        {
            var renderedArgs = c.Arguments.Select(a =>
                !string.IsNullOrEmpty(a.Name) ? $"{a.Name}: {EmitExpr(a.Value)}" : EmitExpr(a.Value)).ToList();

            if (PilotLibraryMapper.TryFormatAstLibraryCall(c.LibraryCallKind, renderedArgs, "ruby", out string mapped, c))
                return mapped;

            if (c.Callee is PilotIrMemberAccessExpr mem && mem.MemberName is "append" or "Add" or "push_back")
            {
                return $"{EmitExpr(mem.Target, PilotIrPrecedence.Postfix)}.push({string.Join(", ", renderedArgs)})";
            }

            string callee = EmitExpr(c.Callee, PilotIrPrecedence.Postfix);
            if (_knownClassesAndVariants.Contains(callee) ||
                (callee.Length > 0 && char.IsUpper(callee[0]) && !callee.Contains('.') && !_knownFunctions.Contains(callee)))
            {
                return renderedArgs.Count > 0
                    ? $"{callee}.new({string.Join(", ", renderedArgs)})"
                    : $"{callee}.new";
            }

            return $"{callee}({string.Join(", ", renderedArgs)})";
        }

        private string EmitMemberAccess(PilotIrMemberAccessExpr m)
        {
            string target = EmitExpr(m.Target, PilotIrPrecedence.Postfix);
            if (PilotLibraryMapper.TryFormatAstMemberAccess(target, m.MemberName, "ruby", out string mapped))
                return mapped;
            if (m.MemberName is "Length" or "Count")
                return m.IsSafeNavigation ? $"{target}&.length" : $"{target}.length";
            string op = m.IsSafeNavigation ? "&." : ".";
            return $"{target}{op}{m.MemberName}";
        }

        private string EmitIndex(PilotIrIndexExpr ix)
        {
            string target = EmitExpr(ix.Target, PilotIrPrecedence.Postfix);
            if (ix.IsSlice)
            {
                string start = EmitExpr(ix.Index);
                string end = ix.SliceEnd != null ? EmitExpr(ix.SliceEnd) : "-1";
                string rangeOp = ix.SliceEnd != null ? "..." : "..";
                return $"{target}[{start}{rangeOp}{end}]";
            }
            return $"{target}[{EmitExpr(ix.Index)}]";
        }

        private string EmitPostfixTry(PilotIrPostfixTryExpr pt)
        {
            string op = EmitExpr(pt.Operand, PilotIrPrecedence.Postfix);
            if (!pt.IsQuestionPropagate && pt.CatchFallbackExpr != null)
            {
                string fb = EmitExpr(pt.CatchFallbackExpr, PilotIrPrecedence.Conditional);
                return $"({op}.nil? ? {fb} : {op})";
            }
            return op;
        }

        private string EmitCollection(PilotIrCollectionExpr col) => col.Kind switch
        {
            PilotCollectionKind.List or PilotCollectionKind.Tuple =>
                $"[{string.Join(", ", col.Elements.Select(e => EmitExpr(e)))}]",
            PilotCollectionKind.Set =>
                $"[{string.Join(", ", col.Elements.Select(e => EmitExpr(e)))}].uniq",
            PilotCollectionKind.Map =>
                $"{{{string.Join(", ", col.MapEntries.Select(kv => $"{EmitExpr(kv.Key)} => {EmitExpr(kv.Value)}"))}}}",
            _ => "[]"
        };

        private string EmitLambda(PilotIrLambdaExpr lam)
        {
            string parms = string.Join(", ", lam.Parameters.Select(p => p.Name));
            string body = lam.BodyExpr != null ? EmitExpr(lam.BodyExpr) : "nil";
            return string.IsNullOrEmpty(parms) ? $"-> {{ {body} }}" : $"->({parms}) {{ {body} }}";
        }

        private string EmitForExpr(PilotIrForExpr fe)
        {
            string target = fe.TargetVars.Count > 0 ? string.Join(", ", fe.TargetVars) : "x";
            string iter = fe.Iterable is PilotIrRangeExpr r ? $"({EmitRangeRaw(r)})" : EmitExpr(fe.Iterable, PilotIrPrecedence.Postfix);
            string chain = iter;
            if (fe.FilterCondition != null)
                chain += $".select {{ |{target}| {EmitExpr(fe.FilterCondition)} }}";
            chain += $".map {{ |{target}| {EmitExpr(fe.YieldExpr)} }}";
            return chain;
        }

        private string EmitMatchExpr(PilotIrMatchExpr me)
        {
            if (me.Arms.Count == 0)
                return "nil";
            var parts = new List<string>();
            foreach (var arm in me.Arms)
            {
                string val = arm.ExpressionBody != null ? EmitExpr(arm.ExpressionBody) : "nil";
                if (arm.Pattern is PilotIrWildcardPattern or PilotIrBindingPattern)
                    parts.Add($"else {val}");
                else
                    parts.Add($"when {EmitRubyWhenClause(arm.Pattern)} then {val}");
            }
            return $"(case {EmitExpr(me.Scrutinee)}; {string.Join("; ", parts)}; end)";
        }

        private string EmitSqlQueryExpr(PilotIrSqlQueryExpr sq)
        {
            string chain = EmitExpr(sq.Source, PilotIrPrecedence.Postfix);
            if (sq.WherePredicate != null)
                chain += $".select {{ |{sq.RangeVariable}| {EmitExpr(sq.WherePredicate)} }}";
            if (sq.OrderByKey != null)
            {
                chain += $".sort_by {{ |{sq.RangeVariable}| {EmitExpr(sq.OrderByKey)} }}";
                if (sq.OrderDescending)
                    chain += ".reverse";
            }
            chain += $".map {{ |{sq.RangeVariable}| {EmitExpr(sq.SelectProjection)} }}";
            if (sq.LimitCount != null)
                chain += $".take({EmitExpr(sq.LimitCount)})";
            return chain;
        }
    }
}
