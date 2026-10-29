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
/// Milestone 20.3 &amp; 20.4: AST-driven bidirectional code converter between Pilot and Java 21+.
/// Emits directly from <see cref="PilotIrModule"/> with precedence-aware parenthesization,
/// Java 21 <c>sealed interface</c> + <c>record</c> ADTs, <c>LongStream</c>/<c>IntStream</c> comprehensions,
/// boxed generic type arguments, and AST-scoped <see cref="PilotLibraryMapper"/> calls compilable by <c>javac</c>.
/// </summary>
public sealed class JavaTargetConverter : ITargetLanguageConverter
{
    public string Language { get; }
    public string TargetLanguage => Language;
    public string SourceLanguage => Language;

    public JavaTargetConverter(string alias = "java")
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
        if (!string.IsNullOrWhiteSpace(options.TargetNamespaceOrModule))
        {
            sb.AppendLine($"package {options.TargetNamespaceOrModule};");
            sb.AppendLine();
        }

        if (options.EmitHeaderComments)
        {
            sb.AppendLine("// Code transpiled & optimized from Pilot to Java 21+");
        }

        sb.AppendLine("import java.io.*;");
        sb.AppendLine("import java.nio.charset.StandardCharsets;");
        sb.AppendLine("import java.nio.file.*;");
        sb.AppendLine("import java.util.*;");
        sb.AppendLine("import java.util.function.*;");
        sb.AppendLine("import java.util.stream.*;");
        sb.AppendLine();

        var emitter = new JavaIrEmitter(module, options);
        sb.Append(emitter.EmitModule());

        return TranspilationResult.CreateSuccess(sb.ToString().TrimEnd() + Environment.NewLine, TargetLanguage);
    }

    public TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options) =>
        ForeignAstIngester.Ingest(foreignSource, SourceLanguage, options);

    private sealed class JavaIrEmitter
    {
        private readonly PilotIrModule _module;
        private readonly TranspilationOptions _options;
        private readonly HashSet<string> _knownTypeDecls = new(StringComparer.Ordinal);
        private readonly Dictionary<string, string> _extensionMethods = new(StringComparer.Ordinal);
        private readonly Dictionary<string, string> _localJavaTypes = new(StringComparer.Ordinal);
        private HashSet<string> _activeMutatedNames = new(StringComparer.Ordinal);
        private string _currentReturnType = "void";
        private int _indent;
        private bool _inExtensionMethod;

        public JavaIrEmitter(PilotIrModule module, TranspilationOptions options)
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

            foreach (var ext in module.Statements.OfType<PilotIrExtensionDefStmt>())
            {
                string extClass = GetExtensionClassName(ext);
                foreach (var m in ext.Methods)
                    _extensionMethods[m.Name] = extClass;
            }
        }

        private string Indent => string.Concat(Enumerable.Repeat(_options.IndentString, _indent));

        private static string GetExtensionClassName(PilotIrExtensionDefStmt ext)
        {
            string raw = ext.ExtensionName ?? ext.TargetType;
            int bracket = raw.IndexOf('[');
            if (bracket > 0)
                raw = raw[..bracket];
            raw = raw.TrimEnd('?').Trim();
            return $"{raw}PilotExtensions";
        }

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

            sb.AppendLine("class ProgramModule");
            sb.AppendLine("{");
            _indent = 1;

            foreach (var t in topTypes)
            {
                EmitLeadingComments(sb, t);
                string ts = t is PilotIrExtensionDefStmt ext
                    ? EmitExtensionClass(ext)
                    : EmitStmt(t);
                if (!string.IsNullOrWhiteSpace(ts))
                {
                    sb.AppendLine(ts);
                    sb.AppendLine();
                }
            }

            foreach (var m in moduleMembers)
            {
                EmitLeadingComments(sb, m);
                string ms = EmitStmt(m);
                if (!string.IsNullOrWhiteSpace(ms))
                {
                    sb.AppendLine(ms);
                    sb.AppendLine();
                }
            }

            if (topExecutableStmts.Count > 0)
            {
                _localJavaTypes.Clear();
                _currentReturnType = "void";
                _activeMutatedNames = PilotIngestValidator.CollectMutatedVariableNames(topExecutableStmts);
                foreach (var mv in _module.MutatedVariables)
                    _activeMutatedNames.Add(mv);

                string throwsClause = ContainsCheckedIo(topExecutableStmts) ? " throws Exception" : "";
                sb.AppendLine($"{Indent}public static void main(String[] args){throwsClause}");
                sb.AppendLine($"{Indent}{{");
                _indent++;
                foreach (var s in topExecutableStmts)
                {
                    EmitLeadingComments(sb, s);
                    string line = EmitStmt(s);
                    if (!string.IsNullOrWhiteSpace(line))
                        sb.AppendLine(line);
                }
                _indent--;
                sb.AppendLine($"{Indent}}}");
            }

            _indent = 0;
            sb.AppendLine("}");
            return sb.ToString();
        }

        private void EmitLeadingComments(StringBuilder sb, PilotIrNode node)
        {
            foreach (var t in node.LeadingTrivia)
            {
                if (t.Kind is PilotTriviaKind.LineComment or PilotTriviaKind.DocComment)
                {
                    string clean = t.Text.TrimStart('/');
                    sb.AppendLine($"{Indent}//{clean}");
                }
            }
        }

        public string EmitStmt(PilotIrStmt stmt) => stmt switch
        {
            PilotIrCommentStmt c => $"{Indent}//{c.CommentText.TrimStart('/')}",
            PilotIrImportStmt => "",
            PilotIrVarDeclStmt v => EmitVarDecl(v),
            PilotIrAssignStmt a => EmitAssignStmt(a),
            PilotIrFuncDefStmt f => EmitFuncDef(f, isInstanceMethod: false, extensionTargetType: null),
            PilotIrClassDefStmt c => EmitClassDef(c),
            PilotIrAdtDefStmt adt => EmitAdtDef(adt),
            PilotIrEnumDefStmt en => EmitEnumDef(en),
            PilotIrTypeAliasStmt ta => $"{Indent}// type {ta.Name} = {ta.TargetType}",
            PilotIrExtensionDefStmt ext => EmitExtensionClass(ext),
            PilotIrIfStmt ifs => EmitIfStmt(ifs),
            PilotIrGuardStmt g => EmitGuardStmt(g),
            PilotIrWhileStmt w => EmitWhileStmt(w),
            PilotIrForStmt f => EmitForStmt(f),
            PilotIrMatchStmt m => EmitMatchStmt(m),
            PilotIrTryStmt t => EmitTryStmt(t),
            PilotIrDeferStmt d => string.Join(Environment.NewLine, d.Body.Select(EmitStmt)),
            PilotIrReturnStmt r => r.Value != null ? $"{Indent}return {EmitExpr(r.Value)};" : $"{Indent}return;",
            PilotIrYieldStmt y => y.Value != null ? $"{Indent}// yield {EmitExpr(y.Value)};" : $"{Indent}// yield;",
            PilotIrRaiseStmt rs => rs.Value != null
                ? $"{Indent}throw new RuntimeException(String.valueOf({EmitExpr(rs.Value)}));"
                : $"{Indent}throw new RuntimeException();",
            PilotIrBreakStmt => $"{Indent}break;",
            PilotIrContinueStmt => $"{Indent}continue;",
            PilotIrPassStmt => $"{Indent}// pass",
            PilotIrAssertStmt a => a.Message != null
                ? $"{Indent}if (!({EmitExpr(a.Condition)})) throw new AssertionError(String.valueOf({EmitExpr(a.Message)}));"
                : $"{Indent}if (!({EmitExpr(a.Condition)})) throw new AssertionError(\"Assertion failed\");",
            PilotIrComptimeStmt ct => string.Join(Environment.NewLine, ct.Body.Select(EmitStmt)),
            PilotIrExprStmt e => $"{Indent}{EmitExpr(e.Expression)};",
            _ => ""
        };

        private string EmitVarDecl(PilotIrVarDeclStmt v)
        {
            if (v.TupleDestructureNames.Count > 1)
            {
                string rhs = v.Initializer != null ? EmitExpr(v.Initializer) : "List.of()";
                var sb = new StringBuilder();
                string tmpName = $"__tuple_{v.Line}";
                sb.AppendLine($"{Indent}var {tmpName} = {rhs};");
                for (int i = 0; i < v.TupleDestructureNames.Count; i++)
                {
                    sb.Append($"{Indent}var {v.TupleDestructureNames[i]} = {tmpName}.get({i});");
                    if (i + 1 < v.TupleDestructureNames.Count)
                        sb.AppendLine();
                }
                return sb.ToString();
            }

            bool hasExplicitType = !string.IsNullOrEmpty(v.DeclaredType) &&
                                   v.DeclaredType is not ("auto" or "any");
            string typeStr = hasExplicitType ? MapType(v.DeclaredType!) : "var";

            string rhsVal;
            if (v.ComptimeFoldedLiteral != null)
            {
                rhsVal = v.ComptimeFoldedLiteral;
            }
            else if (v.Initializer != null)
            {
                if (v.Initializer is PilotIrCollectionExpr { Kind: PilotCollectionKind.List, Elements.Count: 0 } && typeStr == "var")
                {
                    rhsVal = "new ArrayList<Object>()";
                }
                else
                {
                    rhsVal = EmitExpr(v.Initializer);
                    if (typeStr == "Long" && rhsVal != "null" && int.TryParse(rhsVal, out _))
                        rhsVal = $"{rhsVal}L";
                    else if (typeStr == "Double" && rhsVal != "null" && int.TryParse(rhsVal, out _))
                        rhsVal = $"{rhsVal}.0";
                }
            }
            else
            {
                rhsVal = DefaultJavaValue(typeStr == "var" ? "Object" : typeStr);
                if (typeStr == "var")
                    typeStr = "Object";
            }

            bool isMut = v.Kind == PilotVarDeclKind.Var || v.IsMutated || _activeMutatedNames.Contains(v.Name);
            string finalPrefix = !isMut ? "final " : "";

            string recordedType = typeStr;
            if (typeStr == "var" && v.Initializer is PilotIrLiteralExpr lit)
            {
                recordedType = lit.Kind switch
                {
                    PilotLiteralKind.Integer => "int",
                    PilotLiteralKind.Float => "double",
                    PilotLiteralKind.Boolean => "boolean",
                    PilotLiteralKind.String => "String",
                    _ => "Object"
                };
            }
            _localJavaTypes[v.Name] = recordedType;

            return $"{Indent}{finalPrefix}{typeStr} {v.Name} = {rhsVal};";
        }

        private string EmitAssignStmt(PilotIrAssignStmt a)
        {
            if (a.Operator == "=" && a.Target is PilotIrIndexExpr ix)
            {
                return $"{Indent}{EmitExpr(ix.Target, PilotIrPrecedence.Postfix)}.set((int)({EmitExpr(ix.Index)}), {EmitExpr(a.Value)});";
            }

            string op = a.Operator == "//=" ? "/=" : a.Operator;
            if (op == "**=")
            {
                string lhs = EmitExpr(a.Target);
                return $"{Indent}{lhs} = Math.pow({lhs}, {EmitExpr(a.Value)});";
            }
            return $"{Indent}{EmitExpr(a.Target)} {op} {EmitExpr(a.Value)};";
        }

        private string EmitFuncDef(PilotIrFuncDefStmt f, bool isInstanceMethod, string? extensionTargetType)
        {
            var savedLocals = new Dictionary<string, string>(_localJavaTypes, StringComparer.Ordinal);
            var savedMutated = _activeMutatedNames;
            string savedRet = _currentReturnType;

            _localJavaTypes.Clear();
            _activeMutatedNames = PilotIngestValidator.CollectMutatedVariableNames(f.Body);
            foreach (var ml in f.MutatedLocals)
                _activeMutatedNames.Add(ml);

            var paramList = new List<string>();
            for (int i = 0; i < f.Parameters.Count; i++)
            {
                var p = f.Parameters[i];
                if (p.Name is "self" or "this")
                {
                    if (extensionTargetType != null)
                    {
                        string extJavaType = MapType(extensionTargetType);
                        paramList.Add($"{extJavaType} self_val");
                        _localJavaTypes["self_val"] = extJavaType;
                    }
                    continue;
                }

                string pType = !string.IsNullOrEmpty(p.TypeAnnotation) ? MapType(p.TypeAnnotation) : "long";
                if (p.IsVariadic)
                    pType += "...";
                paramList.Add($"{pType} {p.Name}");
                _localJavaTypes[p.Name] = pType;
            }

            if (extensionTargetType != null && (paramList.Count == 0 || !paramList[0].EndsWith(" self_val", StringComparison.Ordinal)))
            {
                string extJavaType = MapType(extensionTargetType);
                paramList.Insert(0, $"{extJavaType} self_val");
                _localJavaTypes["self_val"] = extJavaType;
            }

            string retType = !string.IsNullOrWhiteSpace(f.DeclaredReturnType)
                ? MapType(f.DeclaredReturnType!)
                : MapType(f.EffectiveReturnType);
            _currentReturnType = retType;

            string staticMod = isInstanceMethod ? "" : "static ";
            string genPart = f.GenericParams.Count > 0 ? $"<{string.Join(", ", f.GenericParams)}> " : "";
            string throwsClause = ContainsCheckedIo(f.Body) ? " throws Exception" : "";

            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}public {staticMod}{genPart}{retType} {f.Name}({string.Join(", ", paramList)}){throwsClause}");
            sb.AppendLine($"{Indent}{{");
            _indent++;
            bool prevExt = _inExtensionMethod;
            _inExtensionMethod = extensionTargetType != null;
            try
            {
                foreach (var req in f.RequiresClauses)
                {
                    sb.AppendLine($"{Indent}if (!({EmitExpr(req)})) throw new IllegalArgumentException(\"Contract requires failed\");");
                }
                foreach (var s in f.Body)
                {
                    EmitLeadingComments(sb, s);
                    string line = EmitStmt(s);
                    if (!string.IsNullOrWhiteSpace(line))
                        sb.AppendLine(line);
                }
            }
            finally
            {
                _inExtensionMethod = prevExt;
                _localJavaTypes.Clear();
                foreach (var kv in savedLocals)
                    _localJavaTypes[kv.Key] = kv.Value;
                _activeMutatedNames = savedMutated;
                _currentReturnType = savedRet;
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitClassDef(PilotIrClassDefStmt c)
        {
            string genPart = c.GenericParams.Count > 0 ? $"<{string.Join(", ", c.GenericParams)}>" : "";
            if (c.Kind == PilotTypeDeclKind.Record && c.Methods.Count == 0)
            {
                string recFields = string.Join(", ", c.Fields.Select(f => $"{MapType(f.TypeAnnotation)} {f.Name}"));
                return $"{Indent}public record {c.Name}{genPart}({recFields}) {{}}";
            }

            var sb = new StringBuilder();
            string inheritance = c.BaseTypes.Count > 0 ? $" extends {MapType(c.BaseTypes[0])}" : "";
            sb.AppendLine($"{Indent}public static class {c.Name}{genPart}{inheritance}");
            sb.AppendLine($"{Indent}{{");
            _indent++;

            foreach (var f in c.Fields)
            {
                string fType = MapType(f.TypeAnnotation);
                string defVal = f.DefaultValue != null ? $" = {EmitExpr(f.DefaultValue)}" : "";
                sb.AppendLine($"{Indent}public {fType} {f.Name}{defVal};");
            }

            if (c.Fields.Count > 0 && !c.Methods.Any(m => m.Name == c.Name || m.Name is "__init__" or "init"))
            {
                sb.AppendLine($"{Indent}public {c.Name}() {{}}");
                var ctorParams = c.Fields.Select(f => $"{MapType(f.TypeAnnotation)} {f.Name}");
                sb.AppendLine($"{Indent}public {c.Name}({string.Join(", ", ctorParams)})");
                sb.AppendLine($"{Indent}{{");
                _indent++;
                foreach (var f in c.Fields)
                    sb.AppendLine($"{Indent}this.{f.Name} = {f.Name};");
                _indent--;
                sb.AppendLine($"{Indent}}}");
            }

            foreach (var m in c.Methods)
            {
                if (m.Name is "__init__" or "init")
                {
                    var ctorParams = m.Parameters
                        .Where(p => p.Name is not ("self" or "this"))
                        .Select(p => $"{(!string.IsNullOrEmpty(p.TypeAnnotation) ? MapType(p.TypeAnnotation) : "Object")} {p.Name}");
                    sb.AppendLine($"{Indent}public {c.Name}({string.Join(", ", ctorParams)})");
                    EmitBlock(sb, m.Body);
                    continue;
                }
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
            if (adt.Variants.Count == 0)
            {
                sb.Append($"{Indent}public interface {adt.Name}{genPart} {{}}");
                return sb.ToString();
            }

            string permitsList = string.Join(", ", adt.Variants.Select(v => v.Name));
            sb.AppendLine($"{Indent}public sealed interface {adt.Name}{genPart} permits {permitsList} {{}}");
            foreach (var v in adt.Variants)
            {
                string fields = string.Join(", ", v.Fields.Select(f => $"{MapType(f.Type)} {f.Name}"));
                sb.AppendLine($"{Indent}public record {v.Name}{genPart}({fields}) implements {adt.Name}{genPart} {{}}");
            }
            return sb.ToString().TrimEnd();
        }

        private string EmitEnumDef(PilotIrEnumDefStmt en)
        {
            var sb = new StringBuilder();
            bool hasValues = en.Members.Any(m => m.Value != null);
            sb.AppendLine($"{Indent}public enum {en.Name}");
            sb.AppendLine($"{Indent}{{");
            _indent++;

            if (!hasValues)
            {
                for (int i = 0; i < en.Members.Count; i++)
                {
                    string comma = i + 1 < en.Members.Count ? "," : "";
                    sb.AppendLine($"{Indent}{en.Members[i].Name}{comma}");
                }
            }
            else
            {
                for (int i = 0; i < en.Members.Count; i++)
                {
                    var (mName, mVal) = en.Members[i];
                    string valStr = mVal != null ? EmitExpr(mVal) : i.ToString(CultureInfo.InvariantCulture);
                    string term = i + 1 < en.Members.Count ? "," : ";";
                    sb.AppendLine($"{Indent}{mName}({valStr}){term}");
                }
                sb.AppendLine();
                sb.AppendLine($"{Indent}public final long value;");
                sb.AppendLine($"{Indent}{en.Name}(long value) {{ this.value = value; }}");
            }

            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitExtensionClass(PilotIrExtensionDefStmt ext)
        {
            var sb = new StringBuilder();
            string className = GetExtensionClassName(ext);
            sb.AppendLine($"{Indent}public static final class {className}");
            sb.AppendLine($"{Indent}{{");
            _indent++;
            foreach (var m in ext.Methods)
            {
                sb.AppendLine(EmitFuncDef(m, isInstanceMethod: false, extensionTargetType: ext.TargetType));
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string EmitIfStmt(PilotIrIfStmt ifs)
        {
            var sb = new StringBuilder();
            if (!string.IsNullOrEmpty(ifs.OptionalBindingVar))
            {
                sb.AppendLine($"{Indent}var {ifs.OptionalBindingVar} = {EmitExpr(ifs.Condition)};");
                sb.AppendLine($"{Indent}if ({ifs.OptionalBindingVar} != null)");
            }
            else
            {
                sb.AppendLine($"{Indent}if ({EmitExpr(ifs.Condition)})");
            }
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
                if (!_localJavaTypes.ContainsKey(g.BoundVariable))
                {
                    sb.AppendLine($"{Indent}var {g.BoundVariable} = {EmitExpr(g.ConditionOrSource)};");
                    _localJavaTypes[g.BoundVariable] = "Object";
                }
                string cond = $"{g.BoundVariable} == null";
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
                string iterType = InferRangeVarType(r);
                _localJavaTypes[target] = iterType;
                string cmp = r.IsInclusive ? "<=" : "<";
                string step = r.Step != null ? $"{target} += {EmitExpr(r.Step)}" : $"++{target}";
                sb.AppendLine($"{Indent}for ({iterType} {target} = {EmitExpr(r.Start)}; {target} {cmp} {EmitExpr(r.End)}; {step})");
            }
            else
            {
                sb.AppendLine($"{Indent}for (var {target} : {EmitExpr(f.Iterable)})");
            }

            sb.AppendLine($"{Indent}{{");
            _indent++;
            if (f.WhereFilter != null)
            {
                sb.AppendLine($"{Indent}if (!({EmitExpr(f.WhereFilter)})) continue;");
            }
            foreach (var s in f.Body)
            {
                EmitLeadingComments(sb, s);
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

        private string InferRangeVarType(PilotIrRangeExpr r)
        {
            if (r.End is PilotIrIdentifierExpr endId &&
                _localJavaTypes.TryGetValue(endId.Name, out var endJavaType) &&
                endJavaType is "int" or "Integer")
            {
                return "int";
            }
            if (_currentReturnType is "int" or "Integer")
                return "int";
            return "long";
        }

        private string EmitMatchStmt(PilotIrMatchStmt m)
        {
            bool usePatternSwitch = m.Arms.Count > 0 && m.Arms.All(a =>
                a.Pattern is PilotIrVariantPattern or PilotIrWildcardPattern or PilotIrOptionPattern);

            if (usePatternSwitch)
            {
                var sb = new StringBuilder();
                sb.AppendLine($"{Indent}switch ({EmitExpr(m.Scrutinee)})");
                sb.AppendLine($"{Indent}{{");
                _indent++;
                foreach (var arm in m.Arms)
                {
                    string pat = EmitSwitchPattern(arm.Pattern);
                    string guard = arm.Guard != null ? $" when {EmitExpr(arm.Guard)}" : "";
                    string header = pat == "_" ? "default ->" : $"case {pat}{guard} ->";
                    sb.AppendLine($"{Indent}{header}");
                    EmitBlock(sb, arm.Body);
                }
                _indent--;
                sb.Append($"{Indent}}}");
                return sb.ToString();
            }

            var ifSb = new StringBuilder();
            string scrut = EmitExpr(m.Scrutinee, PilotIrPrecedence.Comparison);
            for (int i = 0; i < m.Arms.Count; i++)
            {
                var arm = m.Arms[i];
                string cond = BuildPatternCondition(scrut, arm.Pattern);
                if (arm.Guard != null)
                    cond = cond == "true" ? EmitExpr(arm.Guard) : $"({cond}) && ({EmitExpr(arm.Guard)})";

                if (cond == "true" && i > 0)
                    ifSb.AppendLine($"{Indent}else");
                else if (i == 0)
                    ifSb.AppendLine($"{Indent}if ({cond})");
                else
                    ifSb.AppendLine($"{Indent}else if ({cond})");

                EmitBlock(ifSb, arm.Body);
            }
            return ifSb.ToString().TrimEnd();
        }

        private string EmitSwitchPattern(PilotIrPattern pat) => pat switch
        {
            PilotIrWildcardPattern => "_",
            PilotIrBindingPattern b => !string.IsNullOrEmpty(b.TypeName) ? $"{MapBoxedType(b.TypeName)} {b.Name}" : $"var {b.Name}",
            PilotIrOptionPattern opt => opt.IsSome
                ? (opt.InnerPattern is PilotIrBindingPattern bp ? $"Object {bp.Name}" : "Object _opt")
                : "null",
            PilotIrVariantPattern vp => $"{vp.VariantName}({string.Join(", ", vp.Arguments.Select(a => EmitSwitchPattern(a.SubPattern)))})",
            _ => "_"
        };

        private string BuildPatternCondition(string scrut, PilotIrPattern pat) => pat switch
        {
            PilotIrWildcardPattern or PilotIrBindingPattern => "true",
            PilotIrLiteralPattern lit => lit.Literal.Kind == PilotLiteralKind.String
                ? $"Objects.equals({scrut}, {EmitExpr(lit.Literal)})"
                : $"{scrut} == {EmitExpr(lit.Literal)}",
            PilotIrOptionPattern opt => opt.IsSome ? $"{scrut} != null" : $"{scrut} == null",
            PilotIrRangePattern rp => rp.IsInclusive
                ? $"{scrut} >= {EmitExpr(rp.Start)} && {scrut} <= {EmitExpr(rp.End)}"
                : $"{scrut} >= {EmitExpr(rp.Start)} && {scrut} < {EmitExpr(rp.End)}",
            PilotIrRelationalPattern rel => $"{scrut} {rel.Operator} {EmitExpr(rel.Value)}",
            PilotIrOrPattern orp => string.Join(" || ", orp.Alternatives.Select(a => $"({BuildPatternCondition(scrut, a)})")),
            _ => "true"
        };

        private string EmitTryStmt(PilotIrTryStmt t)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}try");
            EmitBlock(sb, t.TryBody);
            foreach (var c in t.Catches)
            {
                string exType = !string.IsNullOrEmpty(c.ExceptionType) && c.ExceptionType != "Error"
                    ? c.ExceptionType
                    : "Exception";
                string varName = !string.IsNullOrEmpty(c.VariableName) ? c.VariableName : "err";
                sb.AppendLine($"{Indent}catch ({exType} {varName})");
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
                EmitLeadingComments(sb, s);
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
                    "true" or "True" => "true",
                    "false" or "False" => "false",
                    "self" or "this" => _inExtensionMethod ? "self_val" : "this",
                    _ => id.Name
                },
                PilotIrUnaryExpr u => EmitUnary(u),
                PilotIrBinaryExpr b => EmitBinary(b),
                PilotIrRangeExpr r => EmitRangeStreamBoxedList(r),
                PilotIrCallExpr c => EmitCall(c),
                PilotIrMemberAccessExpr m => EmitMemberAccess(m),
                PilotIrIndexExpr ix => EmitIndexExpr(ix),
                PilotIrPostfixTryExpr pt => EmitPostfixTry(pt),
                PilotIrCollectionExpr col => EmitCollection(col),
                PilotIrLambdaExpr lam => EmitLambda(lam),
                PilotIrForExpr fe => EmitForExpr(fe),
                PilotIrIfExpr ie => $"{EmitExpr(ie.Condition, PilotIrPrecedence.Conditional)} ? {EmitExpr(ie.ThenBranch, PilotIrPrecedence.Conditional)} : {EmitExpr(ie.ElseBranch, PilotIrPrecedence.Conditional)}",
                PilotIrMatchExpr me => EmitMatchExpr(me),
                PilotIrSqlQueryExpr sq => EmitSqlQueryExpr(sq),
                _ => "null"
            };

            if (parentPrec != PilotIrPrecedence.Lowest && expr.Precedence < parentPrec)
                return $"({core})";
            return core;
        }

        private static string EmitLiteral(PilotIrLiteralExpr lit) => lit.Kind switch
        {
            PilotLiteralKind.Null or PilotLiteralKind.None => "null",
            PilotLiteralKind.Boolean => lit.RawValue is "true" or "True" ? "true" : "false",
            PilotLiteralKind.Keyword => $"\"{lit.RawValue.TrimStart(':')}\"",
            PilotLiteralKind.DimensionalUnit => (lit.NormalizedNumericValue ?? 0.0).ToString(CultureInfo.InvariantCulture),
            _ => lit.RawValue
        };

        private string EmitInterpolatedString(PilotIrInterpolatedStringExpr fstr)
        {
            if (!fstr.Parts.Any(p => p.IsInterpolation))
            {
                string plain = string.Concat(fstr.Parts.Select(p => p.LiteralText));
                return $"\"{plain}\"";
            }

            var fmtBuilder = new StringBuilder();
            var args = new List<string>();
            foreach (var part in fstr.Parts)
            {
                if (!part.IsInterpolation)
                {
                    fmtBuilder.Append(part.LiteralText.Replace("%", "%%"));
                }
                else
                {
                    string spec = !string.IsNullOrEmpty(part.FormatSpec) ? $"%{part.FormatSpec}" : "%s";
                    fmtBuilder.Append(spec);
                    args.Add(part.Expression != null ? EmitExpr(part.Expression) : part.LiteralText);
                }
            }

            return $"String.format(\"{fmtBuilder}\", {string.Join(", ", args)})";
        }

        private string EmitUnary(PilotIrUnaryExpr u)
        {
            string op = u.Operator is "not" ? "!" : u.Operator;
            return $"{op}{EmitExpr(u.Operand, u.Precedence)}";
        }

        private string EmitBinary(PilotIrBinaryExpr b)
        {
            if (b.Operator == "**")
                return $"Math.pow({EmitExpr(b.Left)}, {EmitExpr(b.Right)})";
            if (b.Operator == "|>")
                return $"{EmitExpr(b.Right, PilotIrPrecedence.Postfix)}({EmitExpr(b.Left)})";
            if (b.Operator == "<|")
                return $"{EmitExpr(b.Left, PilotIrPrecedence.Postfix)}({EmitExpr(b.Right)})";
            if (b.Operator == "??")
            {
                string l = EmitExpr(b.Left, PilotIrPrecedence.Comparison);
                string r = EmitExpr(b.Right, PilotIrPrecedence.Conditional);
                return $"({l} != null ? {l} : {r})";
            }

            if (b.Operator is "==" or "!=" &&
                (b.Left is PilotIrLiteralExpr { Kind: PilotLiteralKind.String } ||
                 b.Right is PilotIrLiteralExpr { Kind: PilotLiteralKind.String }))
            {
                string eqCall = $"Objects.equals({EmitExpr(b.Left)}, {EmitExpr(b.Right)})";
                return b.Operator == "!=" ? $"!{eqCall}" : eqCall;
            }

            if (b.Operator is "in" or "not in")
            {
                string containsCall = $"{EmitExpr(b.Right, PilotIrPrecedence.Postfix)}.contains({EmitExpr(b.Left)})";
                return b.Operator == "not in" ? $"!{containsCall}" : containsCall;
            }

            string javaOp = b.Operator switch
            {
                "and" => "&&",
                "or" => "||",
                "//" => "/",
                "is" => "==",
                "is not" => "!=",
                _ => b.Operator
            };

            string left = EmitExpr(b.Left, b.Precedence);
            var rightMinPrec = b.Operator is "-" or "/" or "//" or "%"
                ? (PilotIrPrecedence)((int)b.Precedence + 1)
                : b.Precedence;
            string right = EmitExpr(b.Right, rightMinPrec);
            return $"{left} {javaOp} {right}";
        }

        private string EmitRangeStreamBoxedList(PilotIrRangeExpr r)
        {
            string fn = r.IsInclusive ? "rangeClosed" : "range";
            return $"LongStream.{fn}({EmitExpr(r.Start)}, {EmitExpr(r.End)}).boxed().collect(Collectors.toList())";
        }

        private string EmitCall(PilotIrCallExpr c)
        {
            var renderedArgs = c.Arguments.Select(a => EmitExpr(a.Value)).ToList();

            if (PilotLibraryMapper.TryFormatAstLibraryCall(c.LibraryCallKind, renderedArgs, "java", out string mapped, c))
                return mapped;

            if (c.Callee is PilotIrMemberAccessExpr mem)
            {
                string recv = EmitExpr(mem.Target, PilotIrPrecedence.Postfix);
                if (mem.MemberName is "append" or "push" or "Add" or "push_back")
                    return $"{recv}.add({string.Join(", ", renderedArgs)})";

                if (_extensionMethods.TryGetValue(mem.MemberName, out var extClass))
                {
                    var extArgs = new List<string> { recv };
                    extArgs.AddRange(renderedArgs);
                    return $"{extClass}.{mem.MemberName}({string.Join(", ", extArgs)})";
                }
            }

            string callee = EmitExpr(c.Callee, PilotIrPrecedence.Postfix);
            if (_knownTypeDecls.Contains(callee) ||
                (callee.Length > 0 && char.IsUpper(callee[0]) && !callee.Contains('.')))
            {
                return $"new {callee}({string.Join(", ", renderedArgs)})";
            }

            return $"{callee}({string.Join(", ", renderedArgs)})";
        }

        private string EmitMemberAccess(PilotIrMemberAccessExpr m)
        {
            string target = EmitExpr(m.Target, PilotIrPrecedence.Postfix);
            if (PilotLibraryMapper.TryFormatAstMemberAccess(target, m.MemberName, "java", out string mapped))
                return mapped;
            if (m.MemberName is "Length" or "Count")
                return $"{target}.size()";
            if (m.IsSafeNavigation)
                return $"({target} != null ? {target}.{m.MemberName} : null)";
            return $"{target}.{m.MemberName}";
        }

        private string EmitIndexExpr(PilotIrIndexExpr ix)
        {
            string target = EmitExpr(ix.Target, PilotIrPrecedence.Postfix);
            if (ix.IsSlice)
            {
                string start = EmitExpr(ix.Index);
                string end = ix.SliceEnd != null ? EmitExpr(ix.SliceEnd) : $"{target}.size()";
                return $"{target}.subList((int)({start}), (int)({end}))";
            }
            return $"{target}.get((int)({EmitExpr(ix.Index)}))";
        }

        private string EmitPostfixTry(PilotIrPostfixTryExpr pt)
        {
            string op = EmitExpr(pt.Operand, PilotIrPrecedence.Comparison);
            if (!pt.IsQuestionPropagate && pt.CatchFallbackExpr != null)
            {
                string fb = EmitExpr(pt.CatchFallbackExpr, PilotIrPrecedence.Conditional);
                return $"({op} != null ? {op} : {fb})";
            }
            return op;
        }

        private string EmitCollection(PilotIrCollectionExpr col) => col.Kind switch
        {
            PilotCollectionKind.List => col.Elements.Count == 0
                ? "new ArrayList<>()"
                : $"new ArrayList<>(Arrays.asList({string.Join(", ", col.Elements.Select(e => EmitExpr(e)))}))",
            PilotCollectionKind.Tuple => $"List.of({string.Join(", ", col.Elements.Select(e => EmitExpr(e)))})",
            PilotCollectionKind.Set => col.Elements.Count == 0
                ? "new HashSet<>()"
                : $"new HashSet<>(Arrays.asList({string.Join(", ", col.Elements.Select(e => EmitExpr(e)))}))",
            PilotCollectionKind.Map => col.MapEntries.Count == 0
                ? "new HashMap<>()"
                : $"new HashMap<>(Map.ofEntries({string.Join(", ", col.MapEntries.Select(kv => $"Map.entry({EmitExpr(kv.Key)}, {EmitExpr(kv.Value)})"))}))",
            _ => "new ArrayList<>()"
        };

        private string EmitLambda(PilotIrLambdaExpr lam)
        {
            string parms = string.Join(", ", lam.Parameters.Select(p => p.Name));
            string body = lam.BodyExpr != null ? EmitExpr(lam.BodyExpr) : "{}";
            return $"({parms}) -> {body}";
        }

        private string EmitForExpr(PilotIrForExpr fe)
        {
            string target = fe.TargetVars.Count > 0 ? fe.TargetVars[0] : "x";
            if (fe.Iterable is PilotIrRangeExpr r)
            {
                bool useIntStream = _currentReturnType == "List<Integer>";
                string streamCls = useIntStream ? "IntStream" : "LongStream";
                string rangeFn = r.IsInclusive ? "rangeClosed" : "range";
                string chain = $"{streamCls}.{rangeFn}({EmitExpr(r.Start)}, {EmitExpr(r.End)})";
                if (fe.FilterCondition != null)
                    chain += $".filter({target} -> {EmitExpr(fe.FilterCondition)})";
                chain += $".map({target} -> {EmitExpr(fe.YieldExpr)}).boxed().collect(Collectors.toList())";
                return chain;
            }

            string src = EmitExpr(fe.Iterable, PilotIrPrecedence.Postfix);
            string colChain = $"{src}.stream()";
            if (fe.FilterCondition != null)
                colChain += $".filter({target} -> {EmitExpr(fe.FilterCondition)})";
            colChain += $".map({target} -> {EmitExpr(fe.YieldExpr)}).collect(Collectors.toList())";
            return colChain;
        }

        private string EmitMatchExpr(PilotIrMatchExpr me)
        {
            if (me.Arms.Count == 0)
                return "null";
            string scrut = EmitExpr(me.Scrutinee, PilotIrPrecedence.Comparison);
            string expr = "null";
            for (int i = me.Arms.Count - 1; i >= 0; i--)
            {
                var arm = me.Arms[i];
                string val = arm.ExpressionBody != null ? EmitExpr(arm.ExpressionBody, PilotIrPrecedence.Conditional) : "null";
                string cond = BuildPatternCondition(scrut, arm.Pattern);
                if (cond == "true")
                    expr = val;
                else
                    expr = $"({cond} ? {val} : {expr})";
            }
            return expr;
        }

        private string EmitSqlQueryExpr(PilotIrSqlQueryExpr sq)
        {
            string chain = $"{EmitExpr(sq.Source, PilotIrPrecedence.Postfix)}.stream()";
            if (sq.WherePredicate != null)
                chain += $".filter({sq.RangeVariable} -> {EmitExpr(sq.WherePredicate)})";
            chain += $".map({sq.RangeVariable} -> {EmitExpr(sq.SelectProjection)})";
            if (sq.LimitCount != null)
                chain += $".limit({EmitExpr(sq.LimitCount)})";
            chain += ".collect(Collectors.toList())";
            return chain;
        }

        private static bool ContainsCheckedIo(IEnumerable<PilotIrStmt> stmts)
        {
            foreach (var s in stmts)
            {
                switch (s)
                {
                    case PilotIrVarDeclStmt vd when vd.Initializer != null && ExprContainsCheckedIo(vd.Initializer):
                    case PilotIrAssignStmt ass when ExprContainsCheckedIo(ass.Value):
                    case PilotIrExprStmt es when ExprContainsCheckedIo(es.Expression):
                    case PilotIrReturnStmt ret when ret.Value != null && ExprContainsCheckedIo(ret.Value):
                        return true;
                    case PilotIrIfStmt ifs when ContainsCheckedIo(ifs.ThenBody) ||
                                                ifs.ElifBranches.Any(e => ContainsCheckedIo(e.Body)) ||
                                                ContainsCheckedIo(ifs.ElseBody):
                    case PilotIrWhileStmt ws when ContainsCheckedIo(ws.Body):
                    case PilotIrForStmt fs when ContainsCheckedIo(fs.Body):
                        return true;
                }
            }
            return false;
        }

        private static bool ExprContainsCheckedIo(PilotIrExpr expr) => expr switch
        {
            PilotIrCallExpr c => c.LibraryCallKind is PilotLibraryCallKind.FileReadText or PilotLibraryCallKind.FileWriteText ||
                                 ExprContainsCheckedIo(c.Callee) ||
                                 c.Arguments.Any(a => ExprContainsCheckedIo(a.Value)),
            PilotIrBinaryExpr b => ExprContainsCheckedIo(b.Left) || ExprContainsCheckedIo(b.Right),
            PilotIrUnaryExpr u => ExprContainsCheckedIo(u.Operand),
            _ => false
        };

        private static string DefaultJavaValue(string javaType) => javaType switch
        {
            "byte" or "short" or "int" => "0",
            "long" => "0L",
            "float" => "0.0f",
            "double" => "0.0",
            "boolean" => "false",
            _ => "null"
        };

        private string MapType(string pilotType)
        {
            string t = pilotType.Trim();
            if (_options.TypeMappings.TryGetValue(t, out var mapped))
                return mapped;

            if (t.EndsWith("?"))
                return MapBoxedType(t[..^1]);
            if (t.Contains('!'))
                return "Object";
            if ((t.StartsWith("list[") || t.StartsWith("List[")) && t.EndsWith("]"))
                return $"List<{MapBoxedType(t[(t.IndexOf('[') + 1)..^1])}>";
            if (t.StartsWith("[") && t.EndsWith("]"))
                return $"List<{MapBoxedType(t[1..^1])}>";
            if ((t.StartsWith("set[") || t.StartsWith("Set[")) && t.EndsWith("]"))
                return $"Set<{MapBoxedType(t[(t.IndexOf('[') + 1)..^1])}>";
            if ((t.StartsWith("map[") || t.StartsWith("Map[")) && t.EndsWith("]"))
            {
                var parts = t[(t.IndexOf('[') + 1)..^1].Split(',', 2);
                return parts.Length == 2
                    ? $"Map<{MapBoxedType(parts[0])}, {MapBoxedType(parts[1])}>"
                    : "Map<Object, Object>";
            }

            return t switch
            {
                "i8" => "byte",
                "u8" or "i16" => "short",
                "u16" or "i32" or "int" => "int",
                "u32" or "i64" or "u64" => "long",
                "f32" or "float" => "float",
                "f64" or "double" => "double",
                "decimal" => "java.math.BigDecimal",
                "str" or "string" => "String",
                "bool" => "boolean",
                "void" => "void",
                "any" or "auto" or "object" or "dynamic" => "Object",
                "list" or "List" => "List<Object>",
                "map" or "Map" or "dict" or "Dict" => "Map<Object, Object>",
                "set" or "Set" => "Set<Object>",
                "Matrix" => "PilotMatrix",
                _ => t
            };
        }

        private string MapBoxedType(string pilotType)
        {
            string t = pilotType.Trim().TrimEnd('?');
            if (_options.TypeMappings.TryGetValue(t, out var mapped))
                return mapped;

            if ((t.StartsWith("list[") || t.StartsWith("List[")) && t.EndsWith("]"))
                return $"List<{MapBoxedType(t[(t.IndexOf('[') + 1)..^1])}>";
            if (t.StartsWith("[") && t.EndsWith("]"))
                return $"List<{MapBoxedType(t[1..^1])}>";
            if ((t.StartsWith("set[") || t.StartsWith("Set[")) && t.EndsWith("]"))
                return $"Set<{MapBoxedType(t[(t.IndexOf('[') + 1)..^1])}>";
            if ((t.StartsWith("map[") || t.StartsWith("Map[")) && t.EndsWith("]"))
            {
                var parts = t[(t.IndexOf('[') + 1)..^1].Split(',', 2);
                return parts.Length == 2
                    ? $"Map<{MapBoxedType(parts[0])}, {MapBoxedType(parts[1])}>"
                    : "Map<Object, Object>";
            }

            return t switch
            {
                "i8" => "Byte",
                "u8" or "i16" => "Short",
                "u16" or "i32" or "int" => "Integer",
                "u32" or "i64" or "u64" => "Long",
                "f32" or "float" => "Float",
                "f64" or "double" => "Double",
                "decimal" => "java.math.BigDecimal",
                "str" or "string" => "String",
                "bool" => "Boolean",
                "any" or "auto" or "object" or "dynamic" => "Object",
                "list" or "List" => "List<Object>",
                "map" or "Map" or "dict" or "Dict" => "Map<Object, Object>",
                "set" or "Set" => "Set<Object>",
                "Matrix" => "PilotMatrix",
                _ => t
            };
        }
    }
}
