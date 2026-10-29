using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using Pilot.Transpilation;
using Pilot.Transpilation.Ir;

namespace Pilot.Formatting;

/// <summary>
/// Options controlling canonical Pilot CST/AST code formatting (<c>pilot fmt</c>).
/// </summary>
public sealed class PilotFormatterOptions
{
    public string IndentString { get; set; } = "    ";
    public bool? UseColonSuites { get; set; } = null;
    public bool EmitSemicolons { get; set; } = true;
    public bool PreserveComments { get; set; } = true;
    public bool PreserveBlankLines { get; set; } = true;
    public int MaxConsecutiveBlankLines { get; set; } = 1;
}

/// <summary>
/// Result of formatting a Pilot source string or file.
/// </summary>
public sealed class PilotFormatResult
{
    public bool Success { get; init; }
    public string FormattedCode { get; init; } = "";
    public bool WasModified { get; init; }
    public IReadOnlyList<string> Diagnostics { get; init; } = Array.Empty<string>();
}

/// <summary>
/// Milestone 20.2: Lossless Trivia-Preserving Pilot CST/AST Formatter (<c>pilot fmt</c>).
/// Pretty-prints <see cref="PilotIrModule"/> or raw <c>.pilot</c> source while preserving comments,
/// blank lines, and suite conventions (brace <c>{ ... }</c> or colon <c>: ...</c>).
/// </summary>
public static class PilotFormatter
{
    /// <summary>
    /// Formats raw Pilot source code into canonical, trivia-preserving Pilot syntax.
    /// </summary>
    public static PilotFormatResult FormatSource(string pilotSource, PilotFormatterOptions? options = null)
    {
        options ??= new PilotFormatterOptions();
        var transpileOpts = new TranspilationOptions
        {
            IndentString = options.IndentString,
            PreferColonSuites = options.UseColonSuites ?? false
        };

        var (module, errors) = PilotTranspileIrBuilder.BuildFromSource(pilotSource, transpileOpts);
        if (module == null || errors.Count > 0)
        {
            return new PilotFormatResult
            {
                Success = false,
                FormattedCode = pilotSource,
                WasModified = false,
                Diagnostics = errors
            };
        }

        if (options.UseColonSuites.HasValue)
        {
            module.PreferColonSuites = options.UseColonSuites.Value;
        }

        string formatted = FormatModule(module, options);
        bool modified = !string.Equals(
            NormalizeLineEndings(pilotSource).Trim(),
            NormalizeLineEndings(formatted).Trim(),
            StringComparison.Ordinal);

        return new PilotFormatResult
        {
            Success = true,
            FormattedCode = formatted,
            WasModified = modified
        };
    }

    /// <summary>
    /// Formats a <see cref="PilotIrModule"/> into canonical Pilot source code.
    /// Used by <c>pilot fmt</c>, <c>pilot --from</c> foreign ingesters, and <c>pilot perf optimize --apply</c>.
    /// </summary>
    public static string FormatModule(PilotIrModule module, PilotFormatterOptions? options = null)
    {
        options ??= new PilotFormatterOptions();
        bool colonSuites = options.UseColonSuites ?? module.PreferColonSuites;
        var writer = new PilotAstPrinter(options, colonSuites);
        return writer.PrintModule(module);
    }

    /// <summary>
    /// Formats a single <c>.pilot</c> file on disk (or checks formatting without writing when <paramref name="checkOnly"/> is true).
    /// </summary>
    public static PilotFormatResult FormatFile(string filePath, bool checkOnly = false, PilotFormatterOptions? options = null)
    {
        if (!File.Exists(filePath))
        {
            return new PilotFormatResult
            {
                Success = false,
                Diagnostics = new[] { $"File not found: '{filePath}'" }
            };
        }

        string original = File.ReadAllText(filePath);
        var res = FormatSource(original, options);
        if (res.Success && res.WasModified && !checkOnly)
        {
            File.WriteAllText(filePath, res.FormattedCode);
        }
        return res;
    }

    private static string NormalizeLineEndings(string s) =>
        s.Replace("\r\n", "\n").Replace("\r", "\n");

    private sealed class PilotAstPrinter
    {
        private readonly PilotFormatterOptions _options;
        private readonly bool _colonSuites;
        private int _indent;

        public PilotAstPrinter(PilotFormatterOptions options, bool colonSuites)
        {
            _options = options;
            _colonSuites = colonSuites;
        }

        private string Indent => string.Concat(Enumerable.Repeat(_options.IndentString, _indent));
        private string Semi => _options.EmitSemicolons && !_colonSuites ? ";" : "";

        public string PrintModule(PilotIrModule module)
        {
            var sb = new StringBuilder();

            if (_options.PreserveComments && module.HeaderComments.Count > 0)
            {
                foreach (var hc in module.HeaderComments)
                {
                    string c = hc.StartsWith("//") || hc.StartsWith("#") || hc.StartsWith("/*") ? hc : "// " + hc;
                    sb.AppendLine(c);
                }
                sb.AppendLine();
            }

            for (int i = 0; i < module.Statements.Count; i++)
            {
                var stmt = module.Statements[i];
                EmitLeadingTrivia(sb, stmt.LeadingTrivia, isFirstInBlock: i == 0 && module.HeaderComments.Count == 0);
                string rendered = PrintStmt(stmt);
                if (!string.IsNullOrEmpty(rendered))
                {
                    sb.AppendLine(rendered);
                }
            }

            if (_options.PreserveComments && module.EndOfFileTrivia.Count > 0)
            {
                foreach (var tr in module.EndOfFileTrivia)
                {
                    if (!string.IsNullOrWhiteSpace(tr.Text))
                        sb.AppendLine(NormalizeComment(tr.Text));
                }
            }

            return sb.ToString().TrimEnd() + Environment.NewLine;
        }

        private void EmitLeadingTrivia(StringBuilder sb, List<PilotTrivia> trivia, bool isFirstInBlock)
        {
            if (trivia.Count == 0)
                return;

            int blankCount = 0;
            foreach (var tr in trivia)
            {
                if (tr.Kind == PilotTriviaKind.BlankLine)
                {
                    if (_options.PreserveBlankLines && !isFirstInBlock && blankCount < _options.MaxConsecutiveBlankLines)
                    {
                        sb.AppendLine();
                        blankCount++;
                    }
                }
                else if (_options.PreserveComments && !string.IsNullOrWhiteSpace(tr.Text))
                {
                    sb.AppendLine(Indent + NormalizeComment(tr.Text));
                    blankCount = 0;
                }
            }
        }

        private static string NormalizeComment(string raw)
        {
            string t = raw.Trim();
            if (t.StartsWith("#") && !t.StartsWith("#!") && !t.StartsWith("#include") && !t.StartsWith("#define") && !t.StartsWith("#if") && !t.StartsWith("#pragma"))
            {
                return "//" + t[1..];
            }
            return t;
        }

        public string PrintStmt(PilotIrStmt stmt)
        {
            return stmt switch
            {
                PilotIrCommentStmt c => Indent + NormalizeComment(c.CommentText),
                PilotIrImportStmt imp => PrintImport(imp),
                PilotIrVarDeclStmt vd => PrintVarDecl(vd),
                PilotIrAssignStmt ass => $"{Indent}{PrintExpr(ass.Target)} {ass.Operator} {PrintExpr(ass.Value)}{Semi}",
                PilotIrFuncDefStmt fn => PrintFuncDef(fn),
                PilotIrClassDefStmt cls => PrintClassDef(cls),
                PilotIrAdtDefStmt adt => PrintAdtDef(adt),
                PilotIrTypeAliasStmt ta => $"{Indent}type {ta.Name}{(ta.GenericParams.Count > 0 ? $"[{string.Join(", ", ta.GenericParams)}]" : "")} = {ta.TargetType}{Semi}",
                PilotIrEnumDefStmt en => PrintEnumDef(en),
                PilotIrExtensionDefStmt ext => PrintExtensionDef(ext),
                PilotIrIfStmt ifs => PrintIfStmt(ifs),
                PilotIrGuardStmt gs => PrintGuardStmt(gs),
                PilotIrDeferStmt ds => PrintDeferStmt(ds),
                PilotIrWhileStmt ws => PrintWhileStmt(ws),
                PilotIrForStmt fs => PrintForStmt(fs),
                PilotIrMatchStmt ms => PrintMatchStmt(ms),
                PilotIrTryStmt ts => PrintTryStmt(ts),
                PilotIrReturnStmt ret => ret.Value != null ? $"{Indent}return {PrintExpr(ret.Value)}{Semi}" : $"{Indent}return{Semi}",
                PilotIrYieldStmt yld => yld.Value != null ? $"{Indent}yield {PrintExpr(yld.Value)}{Semi}" : $"{Indent}yield{Semi}",
                PilotIrRaiseStmt rs => rs.Value != null ? $"{Indent}raise {PrintExpr(rs.Value)}{Semi}" : $"{Indent}raise{Semi}",
                PilotIrBreakStmt brk => brk.Value != null ? $"{Indent}break {PrintExpr(brk.Value)}{Semi}" : $"{Indent}break{Semi}",
                PilotIrContinueStmt cnt => cnt.RangePivot != null
                    ? $"{Indent}continue {PrintExpr(cnt.RangePivot)}{(cnt.IsSubLoopPrepend ? ", ..." : "")}{Semi}"
                    : $"{Indent}continue{Semi}",
                PilotIrPassStmt => $"{Indent}pass{Semi}",
                PilotIrAssertStmt ast => ast.Message != null
                    ? $"{Indent}{(ast.IsComptime ? "comptime " : "")}assert {PrintExpr(ast.Condition)}, {PrintExpr(ast.Message)}{Semi}"
                    : $"{Indent}{(ast.IsComptime ? "comptime " : "")}assert {PrintExpr(ast.Condition)}{Semi}",
                PilotIrComptimeStmt ct => $"{Indent}comptime{PrintSuite(ct.Body)}",
                PilotIrExprStmt es => $"{Indent}{PrintExpr(es.Expression)}{Semi}",
                _ => ""
            };
        }

        private string PrintImport(PilotIrImportStmt imp)
        {
            if (imp.IsMappedNativeEquivalent)
                return $"{Indent}// [Native Pilot Library Equivalent] import {imp.ModulePath}";

            if (imp.IsForeignImport)
            {
                string aliasPart = !string.IsNullOrEmpty(imp.Alias) ? $" as {imp.Alias}" : "";
                return $"{Indent}import {imp.ForeignLanguage}:{imp.ModulePath}{aliasPart}{Semi}";
            }

            if (imp.ImportedItems.Count > 0)
            {
                var items = imp.ImportedItems.Select(i => !string.IsNullOrEmpty(i.Alias) ? $"{i.Name} as {i.Alias}" : i.Name);
                return $"{Indent}from {imp.ModulePath} import {string.Join(", ", items)}{Semi}";
            }

            string alias = !string.IsNullOrEmpty(imp.Alias) ? $" as {imp.Alias}" : "";
            return $"{Indent}import {imp.ModulePath}{alias}{Semi}";
        }

        private string PrintVarDecl(PilotIrVarDeclStmt vd)
        {
            string kw = vd.Kind switch
            {
                PilotVarDeclKind.Const => "const",
                PilotVarDeclKind.Var => "var",
                _ => "let"
            };
            string namePart = vd.TupleDestructureNames.Count > 1
                ? $"({string.Join(", ", vd.TupleDestructureNames)})"
                : vd.Name;
            string typeAnnot = !string.IsNullOrWhiteSpace(vd.DeclaredType) ? $": {vd.DeclaredType}" : "";
            string initPart = vd.Initializer != null ? $" = {PrintExpr(vd.Initializer)}" : "";
            return $"{Indent}{kw} {namePart}{typeAnnot}{initPart}{Semi}";
        }

        private string PrintFuncDef(PilotIrFuncDefStmt fn)
        {
            var sb = new StringBuilder();
            foreach (var dec in fn.Decorators)
            {
                sb.AppendLine($"{Indent}{(dec.StartsWith("@") ? dec : "@" + dec)}");
            }

            string prefix = fn.IsComptime ? "comptime " : (fn.IsPure ? "pure " : (fn.IsAsync ? "async " : ""));
            string gen = fn.GenericParams.Count > 0 ? $"[{string.Join(", ", fn.GenericParams)}]" : "";
            var paramStrs = fn.Parameters.Select(p =>
            {
                string pPrefix = p.IsContext ? "context " : (p.IsComptime ? "comptime " : (p.IsMutable ? "ref " : ""));
                string pType = !string.IsNullOrWhiteSpace(p.TypeAnnotation) ? $": {p.TypeAnnotation}" : "";
                string pDef = p.DefaultValue != null ? $" = {PrintExpr(p.DefaultValue)}" : "";
                return $"{pPrefix}{p.Name}{pType}{pDef}";
            });

            string? ret = !string.IsNullOrWhiteSpace(fn.DeclaredReturnType) ? fn.DeclaredReturnType : fn.InferredReturnType;
            string retPart = !string.IsNullOrWhiteSpace(ret) && ret != "void" && ret != "auto" ? $" -> {ret}" : "";

            var contracts = new List<string>();
            foreach (var req in fn.RequiresClauses)
                contracts.Add($"requires {PrintExpr(req)}");
            foreach (var ens in fn.EnsuresClauses)
                contracts.Add($"ensures {PrintExpr(ens)}");
            string contractPart = contracts.Count > 0 ? " " + string.Join(" ", contracts) : "";

            sb.Append($"{Indent}{prefix}def {fn.Name}{gen}({string.Join(", ", paramStrs)}){retPart}{contractPart}{PrintSuite(fn.Body)}");
            return sb.ToString();
        }

        private string PrintClassDef(PilotIrClassDefStmt cls)
        {
            var sb = new StringBuilder();
            foreach (var dec in cls.Decorators)
                sb.AppendLine($"{Indent}{(dec.StartsWith("@") ? dec : "@" + dec)}");

            string kw = cls.Kind switch
            {
                PilotTypeDeclKind.Struct => "struct",
                PilotTypeDeclKind.Record => "record",
                PilotTypeDeclKind.Interface => "interface",
                _ => "class"
            };
            string gen = cls.GenericParams.Count > 0 ? $"[{string.Join(", ", cls.GenericParams)}]" : "";

            if (cls.Kind == PilotTypeDeclKind.Record && cls.Methods.Count == 0)
            {
                var fieldParams = cls.Fields.Select(f =>
                    f.DefaultValue != null ? $"{f.Name}: {f.TypeAnnotation} = {PrintExpr(f.DefaultValue)}" : $"{f.Name}: {f.TypeAnnotation}");
                sb.Append($"{Indent}record {cls.Name}{gen}({string.Join(", ", fieldParams)}){Semi}");
                return sb.ToString();
            }

            string bases = cls.BaseTypes.Count > 0 ? $"({string.Join(", ", cls.BaseTypes)})" : "";
            var bodyStmts = new List<PilotIrStmt>();
            foreach (var f in cls.Fields)
            {
                bodyStmts.Add(new PilotIrVarDeclStmt
                {
                    Kind = f.IsMutable ? PilotVarDeclKind.Var : PilotVarDeclKind.Let,
                    Name = f.Name,
                    DeclaredType = f.TypeAnnotation,
                    Initializer = f.DefaultValue
                });
            }
            bodyStmts.AddRange(cls.Methods);

            sb.Append($"{Indent}{kw} {cls.Name}{gen}{bases}{PrintSuite(bodyStmts)}");
            return sb.ToString();
        }

        private string PrintAdtDef(PilotIrAdtDefStmt adt)
        {
            string gen = adt.GenericParams.Count > 0 ? $"[{string.Join(", ", adt.GenericParams)}]" : "";
            var variants = adt.Variants.Select(v =>
            {
                if (v.Fields.Count == 0)
                    return v.Name;
                var flds = v.Fields.Select(f => string.IsNullOrEmpty(f.Name) || f.Name.StartsWith("v") && int.TryParse(f.Name[1..], out _)
                    ? f.Type
                    : $"{f.Name}: {f.Type}");
                return $"{v.Name}({string.Join(", ", flds)})";
            });
            return $"{Indent}type {adt.Name}{gen} = {string.Join(" | ", variants)}{Semi}";
        }

        private string PrintEnumDef(PilotIrEnumDefStmt en)
        {
            var members = en.Members.Select(m => m.Value != null ? $"{m.Name} = {PrintExpr(m.Value)}" : m.Name);
            return $"{Indent}enum {en.Name} {{ {string.Join(", ", members)} }}";
        }

        private string PrintExtensionDef(PilotIrExtensionDefStmt ext)
        {
            var bodyStmts = ext.Methods.Cast<PilotIrStmt>().ToList();
            return $"{Indent}extend {ext.TargetType}{PrintSuite(bodyStmts)}";
        }

        private string PrintIfStmt(PilotIrIfStmt ifs)
        {
            var sb = new StringBuilder();
            string condPart = !string.IsNullOrEmpty(ifs.OptionalBindingVar)
                ? $"some {ifs.OptionalBindingVar} = {PrintExpr(ifs.Condition)}"
                : PrintExpr(ifs.Condition);

            sb.Append($"{Indent}if {condPart}{PrintSuite(ifs.ThenBody)}");
            foreach (var (elifCond, elifBody) in ifs.ElifBranches)
            {
                if (_colonSuites)
                    sb.AppendLine().Append($"{Indent}elif {PrintExpr(elifCond)}{PrintSuite(elifBody)}");
                else
                    sb.Append($" elif {PrintExpr(elifCond)}{PrintSuite(elifBody)}");
            }
            if (ifs.ElseBody.Count > 0)
            {
                if (_colonSuites)
                    sb.AppendLine().Append($"{Indent}else{PrintSuite(ifs.ElseBody)}");
                else
                    sb.Append($" else{PrintSuite(ifs.ElseBody)}");
            }
            return sb.ToString();
        }

        private string PrintGuardStmt(PilotIrGuardStmt gs)
        {
            string head;
            if (gs.IsSomeBinding && !string.IsNullOrEmpty(gs.BoundVariable))
            {
                string src = PrintExpr(gs.ConditionOrSource);
                head = src == gs.BoundVariable
                    ? $"guard some {gs.BoundVariable}"
                    : $"guard some {gs.BoundVariable} = {src}";
            }
            else
            {
                head = $"guard {PrintExpr(gs.ConditionOrSource)}";
            }

            if (gs.PatternGuard != null)
                head += $" when {PrintExpr(gs.PatternGuard)}";

            string elseKw = !string.IsNullOrEmpty(gs.CatchErrorVar) ? $"catch {gs.CatchErrorVar}" : "else";
            return $"{Indent}{head} {elseKw}{PrintSuite(gs.ElseBody)}";
        }

        private string PrintDeferStmt(PilotIrDeferStmt ds)
        {
            if (ds.Body.Count == 1 && ds.Body[0] is PilotIrExprStmt es)
                return $"{Indent}defer {PrintExpr(es.Expression)}{Semi}";
            return $"{Indent}defer{PrintSuite(ds.Body)}";
        }

        private string PrintWhileStmt(PilotIrWhileStmt ws) =>
            $"{Indent}while {PrintExpr(ws.Condition)}{PrintSuite(ws.Body)}";

        private string PrintForStmt(PilotIrForStmt fs)
        {
            string targets = fs.TargetVars.Count > 0 ? string.Join(", ", fs.TargetVars) : "item";
            string main = $"{Indent}for {targets} in {PrintExpr(fs.Iterable)}{PrintSuite(fs.Body)}";
            if (fs.CompletionThenBody.Count > 0)
                main += $" then{PrintSuite(fs.CompletionThenBody)}";
            return main;
        }

        private string PrintMatchStmt(PilotIrMatchStmt ms)
        {
            var sb = new StringBuilder();
            sb.AppendLine($"{Indent}match {PrintExpr(ms.Scrutinee)} {{");
            _indent++;
            foreach (var arm in ms.Arms)
            {
                string guardPart = arm.Guard != null ? $" when {PrintExpr(arm.Guard)}" : "";
                if (arm.ExpressionBody != null)
                {
                    sb.AppendLine($"{Indent}case {PrintPattern(arm.Pattern)}{guardPart} => {PrintExpr(arm.ExpressionBody)}{Semi}");
                }
                else
                {
                    sb.AppendLine($"{Indent}case {PrintPattern(arm.Pattern)}{guardPart} =>{PrintSuite(arm.Body)}");
                }
            }
            _indent--;
            sb.Append($"{Indent}}}");
            return sb.ToString();
        }

        private string PrintTryStmt(PilotIrTryStmt ts)
        {
            var sb = new StringBuilder();
            sb.Append($"{Indent}try{PrintSuite(ts.TryBody)}");
            foreach (var c in ts.Catches)
            {
                string exClause = !string.IsNullOrEmpty(c.ExceptionType) && !string.IsNullOrEmpty(c.VariableName)
                    ? $" {c.ExceptionType} as {c.VariableName}"
                    : (!string.IsNullOrEmpty(c.VariableName) ? $" as {c.VariableName}" : "");
                sb.Append($" except{exClause}{PrintSuite(c.Body)}");
            }
            if (ts.FinallyBody.Count > 0)
            {
                sb.Append($" finally{PrintSuite(ts.FinallyBody)}");
            }
            return sb.ToString();
        }

        private string PrintSuite(List<PilotIrStmt> stmts)
        {
            if (_colonSuites)
            {
                var sb = new StringBuilder();
                sb.AppendLine(":");
                _indent++;
                if (stmts.Count == 0)
                {
                    sb.Append($"{Indent}pass");
                }
                else
                {
                    for (int i = 0; i < stmts.Count; i++)
                    {
                        EmitLeadingTrivia(sb, stmts[i].LeadingTrivia, isFirstInBlock: i == 0);
                        string line = PrintStmt(stmts[i]);
                        if (i < stmts.Count - 1) sb.AppendLine(line);
                        else sb.Append(line);
                    }
                }
                _indent--;
                return sb.ToString();
            }
            else
            {
                var sb = new StringBuilder();
                sb.AppendLine(" {");
                _indent++;
                for (int i = 0; i < stmts.Count; i++)
                {
                    EmitLeadingTrivia(sb, stmts[i].LeadingTrivia, isFirstInBlock: i == 0);
                    string line = PrintStmt(stmts[i]);
                    if (!string.IsNullOrEmpty(line))
                        sb.AppendLine(line);
                }
                _indent--;
                sb.Append($"{Indent}}}");
                return sb.ToString();
            }
        }

        // ====================================================================
        // PRECEDENCE-AWARE EXPRESSION PRETTY-PRINTER
        // ====================================================================

        public string PrintExpr(PilotIrExpr expr, PilotIrPrecedence parentPrec = PilotIrPrecedence.Lowest, bool isRightOperand = false)
        {
            string core = expr switch
            {
                PilotIrLiteralExpr lit => PrintLiteral(lit),
                PilotIrInterpolatedStringExpr fstr => PrintInterpolatedString(fstr),
                PilotIrIdentifierExpr id => id.Name,
                PilotIrUnaryExpr un => un.Operator is "not" or "await" or "spawn"
                    ? $"{un.Operator} {PrintExpr(un.Operand, un.Precedence)}"
                    : $"{un.Operator}{PrintExpr(un.Operand, un.Precedence)}",
                PilotIrBinaryExpr bin => $"{PrintExpr(bin.Left, bin.Precedence, false)} {bin.Operator} {PrintExpr(bin.Right, bin.Precedence, true)}",
                PilotIrRangeExpr rng => $"{PrintExpr(rng.Start, rng.Precedence)}..{(rng.IsInclusive ? "" : "<")}{PrintExpr(rng.End, rng.Precedence)}",
                PilotIrCallExpr call => PrintCall(call),
                PilotIrMemberAccessExpr mem => $"{PrintExpr(mem.Target, mem.Precedence)}{(mem.IsSafeNavigation ? "?." : ".")}{mem.MemberName}",
                PilotIrIndexExpr idx => idx.IsSlice
                    ? $"{PrintExpr(idx.Target, idx.Precedence)}[{PrintExpr(idx.Index)}..{(idx.SliceEnd != null ? PrintExpr(idx.SliceEnd) : "")}]"
                    : $"{PrintExpr(idx.Target, idx.Precedence)}{(idx.IsSafeNavigation ? "?[" : "[")}{PrintExpr(idx.Index)}]",
                PilotIrPostfixTryExpr ptry => ptry.IsQuestionPropagate
                    ? $"{PrintExpr(ptry.Operand, ptry.Precedence)}?"
                    : PrintCatchExpr(ptry),
                PilotIrCollectionExpr coll => PrintCollection(coll),
                PilotIrLambdaExpr lam => PrintLambda(lam),
                PilotIrForExpr fe => PrintForExpr(fe),
                PilotIrIfExpr ife => $"{PrintExpr(ife.ThenBranch, ife.Precedence)} if {PrintExpr(ife.Condition, ife.Precedence)} else {PrintExpr(ife.ElseBranch, ife.Precedence)}",
                PilotIrMatchExpr me => PrintMatchExpr(me),
                PilotIrSqlQueryExpr sql => PrintSqlQuery(sql),
                _ => ""
            };

            bool needsParens = expr.Precedence < parentPrec ||
                               (isRightOperand && expr is PilotIrBinaryExpr && expr.Precedence == parentPrec && expr.Precedence != PilotIrPrecedence.Power);
            return needsParens ? $"({core})" : core;
        }

        private static string PrintLiteral(PilotIrLiteralExpr lit)
        {
            return lit.Kind switch
            {
                PilotLiteralKind.String => lit.RawValue.StartsWith("\"") ? lit.RawValue : $"\"{lit.StringContent}\"",
                PilotLiteralKind.Null => "null",
                PilotLiteralKind.None => "none",
                PilotLiteralKind.Boolean => lit.RawValue == "true" || lit.RawValue == "True" ? "true" : "false",
                _ => lit.RawValue
            };
        }

        private string PrintInterpolatedString(PilotIrInterpolatedStringExpr fstr)
        {
            var sb = new StringBuilder("f\"");
            foreach (var p in fstr.Parts)
            {
                if (!p.IsInterpolation)
                {
                    sb.Append(p.LiteralText);
                }
                else
                {
                    string exprText = p.Expression != null ? PrintExpr(p.Expression) : p.LiteralText;
                    string fmt = !string.IsNullOrEmpty(p.FormatSpec) ? $":{p.FormatSpec}" : "";
                    sb.Append($"{{{exprText}{fmt}}}");
                }
            }
            sb.Append('"');
            return sb.ToString();
        }

        private string PrintCall(PilotIrCallExpr call)
        {
            string callee = PrintExpr(call.Callee, call.Precedence);
            string quote = call.QuoteOperator ?? "";
            var args = call.Arguments.Select(a =>
            {
                string mod = !string.IsNullOrEmpty(a.Modifier) ? $"{a.Modifier} " : "";
                string name = !string.IsNullOrEmpty(a.Name) ? $"{a.Name}: " : "";
                return $"{name}{mod}{PrintExpr(a.Value)}";
            });
            return $"{callee}{quote}({string.Join(", ", args)})";
        }

        private string PrintCatchExpr(PilotIrPostfixTryExpr ptry)
        {
            string op = PrintExpr(ptry.Operand, ptry.Precedence);
            if (!string.IsNullOrEmpty(ptry.CatchVarName) && ptry.CatchFallbackExpr != null)
                return $"{op} catch {ptry.CatchVarName} => {PrintExpr(ptry.CatchFallbackExpr)}";
            if (ptry.CatchFallbackExpr != null)
                return $"{op} catch {PrintExpr(ptry.CatchFallbackExpr)}";
            return $"{op} catch{PrintSuite(ptry.CatchFallbackBody)}";
        }

        private string PrintCollection(PilotIrCollectionExpr coll)
        {
            return coll.Kind switch
            {
                PilotCollectionKind.List => $"[{string.Join(", ", coll.Elements.Select(e => PrintExpr(e)))}]",
                PilotCollectionKind.Tuple => coll.Elements.Count == 1
                    ? $"({PrintExpr(coll.Elements[0])},)"
                    : $"({string.Join(", ", coll.Elements.Select(e => PrintExpr(e)))})",
                PilotCollectionKind.Set => $"{{{string.Join(", ", coll.Elements.Select(e => PrintExpr(e)))}}}",
                PilotCollectionKind.Map => $"{{{string.Join(", ", coll.MapEntries.Select(kv => $"{PrintExpr(kv.Key)}: {PrintExpr(kv.Value)}"))}}}",
                _ => "[]"
            };
        }

        private string PrintLambda(PilotIrLambdaExpr lam)
        {
            string pars = string.Join(", ", lam.Parameters.Select(p => !string.IsNullOrEmpty(p.Type) ? $"{p.Name}: {p.Type}" : p.Name));
            if (lam.BodyExpr != null)
                return $"({pars}) => {PrintExpr(lam.BodyExpr)}";
            return $"({pars}) =>{PrintSuite(lam.BodyStmts)}";
        }

        private string PrintForExpr(PilotIrForExpr fe)
        {
            string targets = fe.TargetVars.Count > 0 ? string.Join(", ", fe.TargetVars) : "x";
            string filter = fe.FilterCondition != null ? $" if {PrintExpr(fe.FilterCondition)}" : "";
            return $"[{PrintExpr(fe.YieldExpr)} for {targets} in {PrintExpr(fe.Iterable)}{filter}]";
        }

        private string PrintMatchExpr(PilotIrMatchExpr me)
        {
            var ms = new PilotIrMatchStmt { Scrutinee = me.Scrutinee };
            ms.Arms.AddRange(me.Arms);
            return PrintMatchStmt(ms).TrimStart();
        }

        private string PrintSqlQuery(PilotIrSqlQueryExpr sql)
        {
            var sb = new StringBuilder($"from {sql.RangeVariable} in {PrintExpr(sql.Source)}");
            if (sql.WherePredicate != null)
                sb.Append($" where {PrintExpr(sql.WherePredicate)}");
            if (sql.OrderByKey != null)
                sb.Append($" order by {PrintExpr(sql.OrderByKey)}{(sql.OrderDescending ? " desc" : " asc")}");
            sb.Append($" select {PrintExpr(sql.SelectProjection)}");
            if (sql.LimitCount != null)
                sb.Append($" limit {PrintExpr(sql.LimitCount)}");
            return sb.ToString();
        }

        public string PrintPattern(PilotIrPattern pat)
        {
            return pat switch
            {
                PilotIrWildcardPattern => "_",
                PilotIrLiteralPattern lp => PrintLiteral(lp.Literal),
                PilotIrBindingPattern bp => !string.IsNullOrEmpty(bp.TypeName)
                    ? $"{bp.TypeName} {bp.Name}"
                    : (bp.IsMutable ? $"var {bp.Name}" : bp.Name),
                PilotIrOptionPattern op => op.IsSome
                    ? (op.InnerPattern != null ? $"some({PrintPattern(op.InnerPattern)})" : "some(_)")
                    : "none",
                PilotIrVariantPattern vp => PrintVariantPattern(vp),
                PilotIrRangePattern rp => $"{PrintExpr(rp.Start)}..{(rp.IsInclusive ? "" : "<")}{PrintExpr(rp.End)}",
                PilotIrRelationalPattern rel => $"{rel.Operator} {PrintExpr(rel.Value)}",
                PilotIrOrPattern orp => string.Join(" | ", orp.Alternatives.Select(PrintPattern)),
                PilotIrTuplePattern tp => $"({string.Join(", ", tp.Elements.Select(PrintPattern))})",
                _ => "_"
            };
        }

        private string PrintVariantPattern(PilotIrVariantPattern vp)
        {
            string head = !string.IsNullOrEmpty(vp.TypeName) ? $"{vp.TypeName}.{vp.VariantName}" : vp.VariantName;
            if (vp.Arguments.Count == 0)
                return head;
            var args = vp.Arguments.Select(a => !string.IsNullOrEmpty(a.FieldName)
                ? $"{a.FieldName}: {PrintPattern(a.SubPattern)}"
                : PrintPattern(a.SubPattern));
            return $"{head}({string.Join(", ", args)})";
        }
    }
}
