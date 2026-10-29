using System;
using System.Collections.Generic;
using System.Linq;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Pilot.Transpilation.Ir;
using Pilot.Transpilation.Licensing;

namespace Pilot.Transpilation.Ingesters;

/// <summary>
/// Milestone 20.4: Structured Roslyn (<c>Microsoft.CodeAnalysis.CSharp</c>) syntax tree walker
/// that lowers C# compilation units into <see cref="PilotIrModule"/> and emits canonical Pilot via <c>PilotFormatter</c>.
/// </summary>
public static class CSharpAstIngester
{
    public static TranspilationResult Ingest(string csharpSource, TranspilationOptions options)
    {
        var lic = PilotLicenseVerifier.VerifySource(
            csharpSource,
            options.SourceFilePath,
            options.LicensePolicy,
            options.CustomSpdxAllowlist);
        if (!lic.IsAllowed)
        {
            return TranspilationResult.CreateFailure(lic.DiagnosticMessage ?? "License rejected", "pilot", lic);
        }

        var tree = CSharpSyntaxTree.ParseText(csharpSource);
        var root = tree.GetCompilationUnitRoot();

        var module = new PilotIrModule
        {
            PreferColonSuites = options.PreferColonSuites
        };
        if (options.EmitHeaderComments)
        {
            module.HeaderComments.Add("// Converted from C# to Pilot");
        }

        // Convert top-level members
        ConvertMemberList(root.Members, module.Statements);

        // Also support top-level C# statements
        foreach (var globalStmt in root.Members.OfType<GlobalStatementSyntax>())
        {
            var s = ConvertStatement(globalStmt.Statement);
            if (s != null)
                module.Statements.Add(s);
        }

        return PilotIngestValidator.ValidateAndFinalize(module, options, "csharp", lic);
    }

    private static void ConvertMemberList(SyntaxList<MemberDeclarationSyntax> members, List<PilotIrStmt> target)
    {
        for (int i = 0; i < members.Count; i++)
        {
            var member = members[i];

            // Reconstruct sibling ADT records:
            // public abstract record Shape;
            // public sealed record Circle(double r) : Shape;
            if (member is RecordDeclarationSyntax abstractRec &&
                abstractRec.Modifiers.Any(SyntaxKind.AbstractKeyword) &&
                abstractRec.Members.Count == 0)
            {
                string adtName = abstractRec.Identifier.Text;
                var adt = new PilotIrAdtDefStmt { Name = adtName };
                int j = i + 1;
                while (j < members.Count &&
                       members[j] is RecordDeclarationSyntax variantRec &&
                       variantRec.BaseList != null &&
                       variantRec.BaseList.Types.Any(bt => bt.Type.ToString().Split('<')[0].Trim() == adtName))
                {
                    var fields = new List<(string Name, string Type)>();
                    if (variantRec.ParameterList != null)
                    {
                        foreach (var p in variantRec.ParameterList.Parameters)
                            fields.Add((p.Identifier.Text, MapCSharpType(p.Type)));
                    }
                    adt.Variants.Add(new PilotIrAdtVariant(variantRec.Identifier.Text, fields));
                    j++;
                }

                if (adt.Variants.Count > 0)
                {
                    target.Add(adt);
                    i = j - 1;
                    continue;
                }
            }

            ConvertMemberDeclaration(member, target);
        }
    }

    private static void ConvertMemberDeclaration(MemberDeclarationSyntax member, List<PilotIrStmt> target)
    {
        switch (member)
        {
            case BaseNamespaceDeclarationSyntax ns:
                ConvertMemberList(ns.Members, target);
                break;

            case ClassDeclarationSyntax cls:
                // Check if this is a static class containing C# extension methods (this T self)
                var extMethods = cls.Members.OfType<MethodDeclarationSyntax>()
                    .Where(m => m.ParameterList.Parameters.Count > 0 &&
                                m.ParameterList.Parameters[0].Modifiers.Any(SyntaxKind.ThisKeyword))
                    .ToList();

                if (extMethods.Count > 0 && extMethods.Count == cls.Members.OfType<MethodDeclarationSyntax>().Count())
                {
                    foreach (var group in extMethods.GroupBy(m => MapCSharpType(m.ParameterList.Parameters[0].Type)))
                    {
                        var ext = new PilotIrExtensionDefStmt
                        {
                            TargetType = group.Key,
                            ExtensionName = cls.Identifier.Text.EndsWith("PilotExtensions", StringComparison.Ordinal)
                                ? null
                                : cls.Identifier.Text
                        };
                        foreach (var m in group)
                        {
                            ext.Methods.Add(ConvertMethod(m, isExtensionMethod: true));
                        }
                        target.Add(ext);
                    }
                    break;
                }

                // If static wrapper class named ProgramModule, unwrap top-level declarations and Main/RunMain
                if (cls.Identifier.Text == "ProgramModule" && cls.Modifiers.Any(SyntaxKind.StaticKeyword))
                {
                    var nonMainMembers = new SyntaxList<MemberDeclarationSyntax>(
                        cls.Members.Where(m => m is not MethodDeclarationSyntax md || md.Identifier.Text is not ("Main" or "RunMain")));
                    ConvertMemberList(nonMainMembers, target);

                    foreach (var m in cls.Members.OfType<MethodDeclarationSyntax>())
                    {
                        if (m.Identifier.Text is "Main" or "RunMain" && m.Body != null)
                        {
                            foreach (var s in m.Body.Statements)
                            {
                                var converted = ConvertStatement(s);
                                if (converted != null) target.Add(converted);
                            }
                        }
                    }
                    break;
                }

                target.Add(ConvertClassOrStruct(cls, PilotTypeDeclKind.Class));
                break;

            case StructDeclarationSyntax st:
                target.Add(ConvertClassOrStruct(st, PilotTypeDeclKind.Struct));
                break;

            case RecordDeclarationSyntax rec:
                // Check if abstract record with nested sealed record variants -> ADT!
                var nestedRecords = rec.Members.OfType<RecordDeclarationSyntax>().ToList();
                if (rec.Modifiers.Any(SyntaxKind.AbstractKeyword) && nestedRecords.Count > 0)
                {
                    var adt = new PilotIrAdtDefStmt { Name = rec.Identifier.Text };
                    foreach (var nr in nestedRecords)
                    {
                        var fields = new List<(string Name, string Type)>();
                        if (nr.ParameterList != null)
                        {
                            foreach (var p in nr.ParameterList.Parameters)
                            {
                                fields.Add((p.Identifier.Text, MapCSharpType(p.Type)));
                            }
                        }
                        adt.Variants.Add(new PilotIrAdtVariant(nr.Identifier.Text, fields));
                    }
                    target.Add(adt);
                    break;
                }

                var recStmt = new PilotIrClassDefStmt
                {
                    Kind = PilotTypeDeclKind.Record,
                    Name = rec.Identifier.Text
                };
                if (rec.ParameterList != null)
                {
                    foreach (var p in rec.ParameterList.Parameters)
                    {
                        recStmt.Fields.Add(new PilotIrFieldDecl(
                            p.Identifier.Text,
                            MapCSharpType(p.Type),
                            p.Default != null ? ConvertExpression(p.Default.Value) : null,
                            IsMutable: false));
                    }
                }
                foreach (var prop in rec.Members.OfType<PropertyDeclarationSyntax>())
                {
                    recStmt.Fields.Add(new PilotIrFieldDecl(
                        prop.Identifier.Text,
                        MapCSharpType(prop.Type),
                        prop.Initializer != null ? ConvertExpression(prop.Initializer.Value) : null,
                        IsMutable: false));
                }
                foreach (var m in rec.Members.OfType<MethodDeclarationSyntax>())
                {
                    recStmt.Methods.Add(ConvertMethod(m));
                }
                target.Add(recStmt);
                break;

            case InterfaceDeclarationSyntax iface:
                target.Add(ConvertClassOrStruct(iface, PilotTypeDeclKind.Interface));
                break;

            case EnumDeclarationSyntax en:
                var enStmt = new PilotIrEnumDefStmt { Name = en.Identifier.Text };
                foreach (var em in en.Members)
                {
                    enStmt.Members.Add((em.Identifier.Text, em.EqualsValue != null ? ConvertExpression(em.EqualsValue.Value) : null));
                }
                target.Add(enStmt);
                break;

            case MethodDeclarationSyntax method:
                target.Add(ConvertMethod(method));
                break;
        }
    }

    private static PilotIrClassDefStmt ConvertClassOrStruct(TypeDeclarationSyntax typeDecl, PilotTypeDeclKind kind)
    {
        var cls = new PilotIrClassDefStmt
        {
            Kind = kind,
            Name = typeDecl.Identifier.Text
        };

        if (typeDecl.TypeParameterList != null)
        {
            foreach (var tp in typeDecl.TypeParameterList.Parameters)
                cls.GenericParams.Add(tp.Identifier.Text);
        }

        if (typeDecl.BaseList != null)
        {
            foreach (var b in typeDecl.BaseList.Types)
                cls.BaseTypes.Add(MapCSharpType(b.Type));
        }

        foreach (var member in typeDecl.Members)
        {
            if (member is FieldDeclarationSyntax fd)
            {
                string fType = MapCSharpType(fd.Declaration.Type);
                bool isMut = !fd.Modifiers.Any(SyntaxKind.ReadOnlyKeyword) && !fd.Modifiers.Any(SyntaxKind.ConstKeyword);
                foreach (var v in fd.Declaration.Variables)
                {
                    cls.Fields.Add(new PilotIrFieldDecl(
                        v.Identifier.Text,
                        fType,
                        v.Initializer != null ? ConvertExpression(v.Initializer.Value) : null,
                        IsMutable: isMut));
                }
            }
            else if (member is PropertyDeclarationSyntax pd && pd.ExpressionBody == null)
            {
                string pType = MapCSharpType(pd.Type);
                cls.Fields.Add(new PilotIrFieldDecl(
                    pd.Identifier.Text,
                    pType,
                    pd.Initializer != null ? ConvertExpression(pd.Initializer.Value) : null,
                    IsMutable: true));
            }
            else if (member is MethodDeclarationSyntax md)
            {
                cls.Methods.Add(ConvertMethod(md, isExtensionMethod: false, isClassMember: true));
            }
        }

        return cls;
    }

    private static PilotIrFuncDefStmt ConvertMethod(MethodDeclarationSyntax md, bool isExtensionMethod = false, bool isClassMember = false)
    {
        string retType = MapCSharpType(md.ReturnType);
        var fn = new PilotIrFuncDefStmt
        {
            Name = md.Identifier.Text,
            DeclaredReturnType = retType == "void" ? null : retType,
            IsStatic = md.Modifiers.Any(SyntaxKind.StaticKeyword),
            IsAsync = md.Modifiers.Any(SyntaxKind.AsyncKeyword)
        };

        if (md.TypeParameterList != null)
        {
            foreach (var tp in md.TypeParameterList.Parameters)
                fn.GenericParams.Add(tp.Identifier.Text);
        }

        if (isClassMember && !fn.IsStatic && !isExtensionMethod)
        {
            fn.Parameters.Add(new PilotIrParameter("self"));
        }

        for (int i = 0; i < md.ParameterList.Parameters.Count; i++)
        {
            var p = md.ParameterList.Parameters[i];
            if (i == 0 && isExtensionMethod)
            {
                fn.Parameters.Add(new PilotIrParameter("self"));
                continue;
            }
            string pName = p.Identifier.Text;
            string pType = MapCSharpType(p.Type);
            PilotIrExpr? defVal = p.Default != null ? ConvertExpression(p.Default.Value) : null;
            fn.Parameters.Add(new PilotIrParameter(
                pName,
                pType,
                defVal,
                IsMutable: p.Modifiers.Any(SyntaxKind.RefKeyword) || p.Modifiers.Any(SyntaxKind.OutKeyword),
                IsVariadic: p.Modifiers.Any(SyntaxKind.ParamsKeyword)));
        }

        if (md.Body != null)
        {
            foreach (var s in md.Body.Statements)
            {
                var st = ConvertStatement(s);
                if (st != null)
                    fn.Body.Add(st);
            }
        }
        else if (md.ExpressionBody != null)
        {
            fn.Body.Add(new PilotIrReturnStmt { Value = ConvertExpression(md.ExpressionBody.Expression) });
        }

        return fn;
    }

    private static PilotIrStmt? ConvertStatement(StatementSyntax stmt)
    {
        switch (stmt)
        {
            case BlockSyntax block:
                var list = block.Statements.Select(ConvertStatement).Where(x => x != null).Cast<PilotIrStmt>().ToList();
                return list.Count == 1 ? list[0] : null;

            case LocalDeclarationStatementSyntax localDecl:
                var v = localDecl.Declaration.Variables.FirstOrDefault();
                if (v == null) return null;
                bool isConst = localDecl.IsConst;
                string rawType = localDecl.Declaration.Type.ToString();
                string? declType = rawType == "var" ? null : MapCSharpType(localDecl.Declaration.Type);
                PilotIrExpr? init = v.Initializer != null ? ConvertExpression(v.Initializer.Value) : null;
                return new PilotIrVarDeclStmt
                {
                    Kind = isConst ? PilotVarDeclKind.Const : PilotVarDeclKind.Let,
                    Name = v.Identifier.Text,
                    DeclaredType = declType,
                    Initializer = init
                };

            case ExpressionStatementSyntax exprStmt:
                if (exprStmt.Expression is AssignmentExpressionSyntax assign)
                {
                    string op = assign.OperatorToken.Text;
                    return new PilotIrAssignStmt
                    {
                        Target = ConvertExpression(assign.Left),
                        Operator = op,
                        Value = ConvertExpression(assign.Right)
                    };
                }
                if (exprStmt.Expression is PostfixUnaryExpressionSyntax postUnary &&
                    postUnary.IsKind(SyntaxKind.PostIncrementExpression))
                {
                    return new PilotIrAssignStmt
                    {
                        Target = ConvertExpression(postUnary.Operand),
                        Operator = "+=",
                        Value = new PilotIrLiteralExpr { Kind = PilotLiteralKind.Integer, RawValue = "1" }
                    };
                }
                if (exprStmt.Expression is PrefixUnaryExpressionSyntax preUnary &&
                    preUnary.IsKind(SyntaxKind.PreIncrementExpression))
                {
                    return new PilotIrAssignStmt
                    {
                        Target = ConvertExpression(preUnary.Operand),
                        Operator = "+=",
                        Value = new PilotIrLiteralExpr { Kind = PilotLiteralKind.Integer, RawValue = "1" }
                    };
                }
                return new PilotIrExprStmt { Expression = ConvertExpression(exprStmt.Expression) };

            case ReturnStatementSyntax ret:
                return new PilotIrReturnStmt
                {
                    Value = ret.Expression != null ? ConvertExpression(ret.Expression) : null
                };

            case ThrowStatementSyntax thr:
                return new PilotIrRaiseStmt
                {
                    Value = thr.Expression != null ? ConvertExpression(thr.Expression) : null
                };

            case BreakStatementSyntax:
                return new PilotIrBreakStmt();

            case ContinueStatementSyntax:
                return new PilotIrContinueStmt();

            case YieldStatementSyntax yld when yld.IsKind(SyntaxKind.YieldReturnStatement):
                return new PilotIrYieldStmt
                {
                    Value = yld.Expression != null ? ConvertExpression(yld.Expression) : null
                };

            case IfStatementSyntax ifs:
                // Reconstruct emitted assertion: if (!(cond)) throw new InvalidOperationException(...);
                if (ifs.Else == null &&
                    ifs.Condition is PrefixUnaryExpressionSyntax { OperatorToken.Text: "!" } negCond &&
                    ifs.Statement is ThrowStatementSyntax { Expression: ObjectCreationExpressionSyntax thrObj } &&
                    thrObj.Type.ToString().Contains("InvalidOperationException"))
                {
                    PilotIrExpr? msgExpr = null;
                    if (thrObj.ArgumentList?.Arguments.Count == 1)
                    {
                        var rawMsg = ConvertExpression(thrObj.ArgumentList.Arguments[0].Expression);
                        if (rawMsg is not PilotIrLiteralExpr { StringContent: "Assertion failed" })
                            msgExpr = rawMsg;
                    }
                    return new PilotIrAssertStmt
                    {
                        Condition = ConvertExpression(negCond.Operand),
                        Message = msgExpr
                    };
                }

                var irIf = new PilotIrIfStmt
                {
                    Condition = ConvertExpression(ifs.Condition)
                };
                irIf.ThenBody.AddRange(UnwrapBlock(ifs.Statement));

                var currentElse = ifs.Else;
                while (currentElse != null)
                {
                    if (currentElse.Statement is IfStatementSyntax elifSyntax)
                    {
                        irIf.ElifBranches.Add((ConvertExpression(elifSyntax.Condition), UnwrapBlock(elifSyntax.Statement)));
                        currentElse = elifSyntax.Else;
                    }
                    else
                    {
                        irIf.ElseBody.AddRange(UnwrapBlock(currentElse.Statement));
                        break;
                    }
                }
                return irIf;

            case WhileStatementSyntax ws:
                var irWhile = new PilotIrWhileStmt
                {
                    Condition = ConvertExpression(ws.Condition)
                };
                irWhile.Body.AddRange(UnwrapBlock(ws.Statement));
                return irWhile;

            case ForEachStatementSyntax fe:
                var irFe = new PilotIrForStmt
                {
                    Iterable = ConvertExpression(fe.Expression)
                };
                irFe.TargetVars.Add(fe.Identifier.Text);
                irFe.Body.AddRange(UnwrapBlock(fe.Statement));
                return irFe;

            case ForStatementSyntax fs:
                if (TryConvertNumericForLoop(fs, out var rangeFor))
                    return rangeFor;
                break;

            case SwitchStatementSyntax sw:
                var matchStmt = new PilotIrMatchStmt
                {
                    Scrutinee = ConvertExpression(sw.Expression)
                };
                foreach (var sec in sw.Sections)
                {
                    var pat = ConvertSwitchLabels(sec.Labels);
                    PilotIrExpr? guard = ExtractWhenGuard(sec.Labels);
                    var body = new List<PilotIrStmt>();
                    foreach (var s in sec.Statements)
                    {
                        if (s is BreakStatementSyntax) continue;
                        var cs = ConvertStatement(s);
                        if (cs != null) body.Add(cs);
                    }
                    matchStmt.Arms.Add(new PilotIrMatchArm(pat, guard, body));
                }
                return matchStmt;

            case TryStatementSyntax tr:
                var irTry = new PilotIrTryStmt();
                irTry.TryBody.AddRange(UnwrapBlock(tr.Block));
                foreach (var c in tr.Catches)
                {
                    string? exType = c.Declaration?.Type != null ? MapCSharpType(c.Declaration.Type) : null;
                    string? exVar = c.Declaration?.Identifier.Text;
                    if (string.IsNullOrWhiteSpace(exVar)) exVar = "err";
                    irTry.Catches.Add(new PilotIrCatchClause(exType, exVar, UnwrapBlock(c.Block)));
                }
                if (tr.Finally != null)
                    irTry.FinallyBody.AddRange(UnwrapBlock(tr.Finally.Block));
                return irTry;
        }

        return null;
    }

    private static bool TryConvertNumericForLoop(ForStatementSyntax fs, out PilotIrForStmt? result)
    {
        result = null;
        if (fs.Declaration == null || fs.Declaration.Variables.Count != 1 || fs.Condition == null)
            return false;

        var v = fs.Declaration.Variables[0];
        if (v.Initializer == null)
            return false;

        string loopVar = v.Identifier.Text;
        var startExpr = ConvertExpression(v.Initializer.Value);

        if (fs.Condition is BinaryExpressionSyntax binCond &&
            binCond.Left is IdentifierNameSyntax idLeft &&
            idLeft.Identifier.Text == loopVar)
        {
            bool inclusive = binCond.IsKind(SyntaxKind.LessThanOrEqualExpression);
            bool exclusive = binCond.IsKind(SyntaxKind.LessThanExpression);
            if (inclusive || exclusive)
            {
                var endExpr = ConvertExpression(binCond.Right);
                result = new PilotIrForStmt
                {
                    Iterable = new PilotIrRangeExpr
                    {
                        Start = startExpr,
                        End = endExpr,
                        IsInclusive = inclusive
                    }
                };
                result.TargetVars.Add(loopVar);
                result.Body.AddRange(UnwrapBlock(fs.Statement));
                return true;
            }
        }
        return false;
    }

    private static List<PilotIrStmt> UnwrapBlock(StatementSyntax stmt)
    {
        if (stmt is BlockSyntax block)
        {
            return block.Statements.Select(ConvertStatement).Where(x => x != null).Cast<PilotIrStmt>().ToList();
        }
        var single = ConvertStatement(stmt);
        return single != null ? new List<PilotIrStmt> { single } : new List<PilotIrStmt>();
    }

    private static PilotIrPattern ConvertSwitchLabels(SyntaxList<SwitchLabelSyntax> labels)
    {
        var patterns = new List<PilotIrPattern>();
        foreach (var lbl in labels)
        {
            if (lbl is DefaultSwitchLabelSyntax)
                return new PilotIrWildcardPattern();
            if (lbl is CaseSwitchLabelSyntax caseLbl)
            {
                var expr = ConvertExpression(caseLbl.Value);
                if (expr is PilotIrLiteralExpr lit)
                    patterns.Add(new PilotIrLiteralPattern { Literal = lit });
                else
                    patterns.Add(new PilotIrBindingPattern { Name = caseLbl.Value.ToString() });
            }
            else if (lbl is CasePatternSwitchLabelSyntax patLbl)
            {
                patterns.Add(ConvertPattern(patLbl.Pattern));
            }
        }
        if (patterns.Count == 0) return new PilotIrWildcardPattern();
        if (patterns.Count == 1) return patterns[0];
        var orPat = new PilotIrOrPattern();
        orPat.Alternatives.AddRange(patterns);
        return orPat;
    }

    private static PilotIrExpr? ExtractWhenGuard(SyntaxList<SwitchLabelSyntax> labels)
    {
        foreach (var lbl in labels.OfType<CasePatternSwitchLabelSyntax>())
        {
            if (lbl.WhenClause != null)
                return ConvertExpression(lbl.WhenClause.Condition);
        }
        return null;
    }

    private static PilotIrPattern ConvertPattern(PatternSyntax pat)
    {
        switch (pat)
        {
            case DiscardPatternSyntax:
                return new PilotIrWildcardPattern();
            case ConstantPatternSyntax cp:
                var ce = ConvertExpression(cp.Expression);
                if (ce is PilotIrLiteralExpr lit)
                    return new PilotIrLiteralPattern { Literal = lit };
                return new PilotIrBindingPattern { Name = cp.Expression.ToString() };
            case DeclarationPatternSyntax dp:
                string typeName = MapCSharpType(dp.Type);
                string varName = dp.Designation is SingleVariableDesignationSyntax sv ? sv.Identifier.Text : "_";
                return new PilotIrBindingPattern { Name = varName, TypeName = typeName };
            case RecursivePatternSyntax rp:
                string vName = rp.Type != null ? rp.Type.ToString().Split('.').Last() : "Variant";
                var vp = new PilotIrVariantPattern { VariantName = vName };
                if (rp.PositionalPatternClause != null)
                {
                    foreach (var sub in rp.PositionalPatternClause.Subpatterns)
                        vp.Arguments.Add((sub.NameColon?.Name.Identifier.Text, ConvertPattern(sub.Pattern)));
                }
                return vp;
            case RelationalPatternSyntax rel:
                return new PilotIrRelationalPattern
                {
                    Operator = rel.OperatorToken.Text,
                    Value = ConvertExpression(rel.Expression)
                };
            case BinaryPatternSyntax bp when bp.IsKind(SyntaxKind.OrPattern):
                var orPat = new PilotIrOrPattern();
                orPat.Alternatives.Add(ConvertPattern(bp.Left));
                orPat.Alternatives.Add(ConvertPattern(bp.Right));
                return orPat;
            default:
                return new PilotIrWildcardPattern();
        }
    }

    private static PilotIrExpr ConvertExpression(ExpressionSyntax expr)
    {
        switch (expr)
        {
            case ParenthesizedExpressionSyntax paren:
                return ConvertExpression(paren.Expression);

            case LiteralExpressionSyntax lit:
                if (lit.IsKind(SyntaxKind.TrueLiteralExpression))
                    return new PilotIrLiteralExpr { Kind = PilotLiteralKind.Boolean, RawValue = "true" };
                if (lit.IsKind(SyntaxKind.FalseLiteralExpression))
                    return new PilotIrLiteralExpr { Kind = PilotLiteralKind.Boolean, RawValue = "false" };
                if (lit.IsKind(SyntaxKind.NullLiteralExpression))
                    return new PilotIrLiteralExpr { Kind = PilotLiteralKind.Null, RawValue = "null" };
                if (lit.IsKind(SyntaxKind.StringLiteralExpression))
                {
                    string val = lit.Token.ValueText;
                    return new PilotIrLiteralExpr
                    {
                        Kind = PilotLiteralKind.String,
                        RawValue = $"\"{val.Replace("\\", "\\\\").Replace("\"", "\\\"")}\"",
                        StringContent = val
                    };
                }
                if (lit.Token.Value is double or float or decimal)
                {
                    string raw = lit.Token.Text.TrimEnd('f', 'F', 'd', 'D', 'm', 'M');
                    return new PilotIrLiteralExpr { Kind = PilotLiteralKind.Float, RawValue = raw };
                }
                return new PilotIrLiteralExpr
                {
                    Kind = PilotLiteralKind.Integer,
                    RawValue = lit.Token.Text.TrimEnd('L', 'l', 'U', 'u')
                };

            case InterpolatedStringExpressionSyntax interp:
                var fstr = new PilotIrInterpolatedStringExpr();
                foreach (var content in interp.Contents)
                {
                    if (content is InterpolatedStringTextSyntax txt)
                    {
                        fstr.Parts.Add(new PilotIrFStringPart(false, txt.TextToken.ValueText));
                    }
                    else if (content is InterpolationSyntax hole)
                    {
                        fstr.Parts.Add(new PilotIrFStringPart(
                            true,
                            hole.Expression.ToString(),
                            ConvertExpression(hole.Expression),
                            hole.FormatClause?.FormatStringToken.ValueText));
                    }
                }
                return fstr;

            case IdentifierNameSyntax id:
                return new PilotIrIdentifierExpr
                {
                    Name = id.Identifier.Text is "this" or "self_val" ? "self" : id.Identifier.Text
                };

            case ThisExpressionSyntax:
                return new PilotIrIdentifierExpr { Name = "self" };

            case CastExpressionSyntax cast:
                return ConvertExpression(cast.Expression);

            case PrefixUnaryExpressionSyntax pre:
                string uOp = pre.OperatorToken.Text == "!" ? "not" : pre.OperatorToken.Text;
                return new PilotIrUnaryExpr
                {
                    Operator = uOp,
                    Operand = ConvertExpression(pre.Operand)
                };

            case AwaitExpressionSyntax aw:
                return new PilotIrUnaryExpr
                {
                    Operator = "await",
                    Operand = ConvertExpression(aw.Expression)
                };

            case BinaryExpressionSyntax bin:
                string bOp = bin.OperatorToken.Text switch
                {
                    "&&" => "and",
                    "||" => "or",
                    _ => bin.OperatorToken.Text
                };
                return new PilotIrBinaryExpr
                {
                    Operator = bOp,
                    Left = ConvertExpression(bin.Left),
                    Right = ConvertExpression(bin.Right)
                };

            case ConditionalExpressionSyntax cond:
                return new PilotIrIfExpr
                {
                    Condition = ConvertExpression(cond.Condition),
                    ThenBranch = ConvertExpression(cond.WhenTrue),
                    ElseBranch = ConvertExpression(cond.WhenFalse)
                };

            case ConditionalAccessExpressionSyntax condAccess:
                var target = ConvertExpression(condAccess.Expression);
                if (condAccess.WhenNotNull is MemberBindingExpressionSyntax mb)
                {
                    return new PilotIrMemberAccessExpr
                    {
                        Target = target,
                        MemberName = mb.Name.Identifier.Text,
                        IsSafeNavigation = true
                    };
                }
                if (condAccess.WhenNotNull is InvocationExpressionSyntax inv &&
                    inv.Expression is MemberBindingExpressionSyntax imb)
                {
                    var call = new PilotIrCallExpr
                    {
                        Callee = new PilotIrMemberAccessExpr
                        {
                            Target = target,
                            MemberName = imb.Name.Identifier.Text,
                            IsSafeNavigation = true
                        }
                    };
                    foreach (var a in inv.ArgumentList.Arguments)
                        call.Arguments.Add(new PilotIrCallArg(a.NameColon?.Name.Identifier.Text, ConvertExpression(a.Expression)));
                    return call;
                }
                return target;

            case MemberAccessExpressionSyntax mem:
                string targetStr = mem.Expression.ToString();
                string memName = mem.Name.Identifier.Text;
                if (targetStr is "Console" or "System.Console")
                {
                    if (memName == "WriteLine") return new PilotIrIdentifierExpr { Name = "println" };
                    if (memName == "Write") return new PilotIrIdentifierExpr { Name = "print" };
                }
                if (memName is "Length" or "Count")
                {
                    var lenCall = new PilotIrCallExpr
                    {
                        Callee = new PilotIrIdentifierExpr { Name = "len" },
                        LibraryCallKind = PilotLibraryCallKind.Len
                    };
                    lenCall.Arguments.Add(new PilotIrCallArg(null, ConvertExpression(mem.Expression)));
                    return lenCall;
                }
                return new PilotIrMemberAccessExpr
                {
                    Target = ConvertExpression(mem.Expression),
                    MemberName = memName
                };

            case InvocationExpressionSyntax invExpr:
                return ConvertInvocation(invExpr);

            case ObjectCreationExpressionSyntax objCreate:
                string tName = MapCSharpType(objCreate.Type);
                if (tName.StartsWith("List") || tName.StartsWith("HashSet"))
                {
                    var coll = new PilotIrCollectionExpr { Kind = PilotCollectionKind.List };
                    if (objCreate.Initializer != null)
                    {
                        foreach (var e in objCreate.Initializer.Expressions)
                            coll.Elements.Add(ConvertExpression(e));
                    }
                    return coll;
                }
                var ctorCall = new PilotIrCallExpr
                {
                    Callee = new PilotIrIdentifierExpr { Name = tName }
                };
                if (objCreate.ArgumentList != null)
                {
                    foreach (var a in objCreate.ArgumentList.Arguments)
                        ctorCall.Arguments.Add(new PilotIrCallArg(a.NameColon?.Name.Identifier.Text, ConvertExpression(a.Expression)));
                }
                return ctorCall;

            case CollectionExpressionSyntax collExpr:
                var listColl = new PilotIrCollectionExpr { Kind = PilotCollectionKind.List };
                foreach (var el in collExpr.Elements.OfType<ExpressionElementSyntax>())
                    listColl.Elements.Add(ConvertExpression(el.Expression));
                return listColl;

            case ArrayCreationExpressionSyntax arrCreate when arrCreate.Initializer != null:
                var arrColl = new PilotIrCollectionExpr { Kind = PilotCollectionKind.List };
                foreach (var el in arrCreate.Initializer.Expressions)
                    arrColl.Elements.Add(ConvertExpression(el));
                return arrColl;

            case ImplicitArrayCreationExpressionSyntax impArr:
                var impColl = new PilotIrCollectionExpr { Kind = PilotCollectionKind.List };
                foreach (var el in impArr.Initializer.Expressions)
                    impColl.Elements.Add(ConvertExpression(el));
                return impColl;

            case ElementAccessExpressionSyntax elemAccess:
                var firstArg = elemAccess.ArgumentList.Arguments.FirstOrDefault();
                return new PilotIrIndexExpr
                {
                    Target = ConvertExpression(elemAccess.Expression),
                    Index = firstArg != null ? ConvertExpression(firstArg.Expression) : new PilotIrLiteralExpr { Kind = PilotLiteralKind.Integer, RawValue = "0" }
                };

            case SimpleLambdaExpressionSyntax sLam:
                var irSLam = new PilotIrLambdaExpr();
                irSLam.Parameters.Add((sLam.Parameter.Identifier.Text, null));
                if (sLam.ExpressionBody != null)
                    irSLam.BodyExpr = ConvertExpression(sLam.ExpressionBody);
                else if (sLam.Block != null)
                    irSLam.BodyStmts.AddRange(UnwrapBlock(sLam.Block));
                return irSLam;

            case ParenthesizedLambdaExpressionSyntax pLam:
                var irPLam = new PilotIrLambdaExpr();
                foreach (var p in pLam.ParameterList.Parameters)
                    irPLam.Parameters.Add((p.Identifier.Text, p.Type != null ? MapCSharpType(p.Type) : null));
                if (pLam.ExpressionBody != null)
                    irPLam.BodyExpr = ConvertExpression(pLam.ExpressionBody);
                else if (pLam.Block != null)
                    irPLam.BodyStmts.AddRange(UnwrapBlock(pLam.Block));
                return irPLam;

            case SwitchExpressionSyntax swExpr:
                var me = new PilotIrMatchExpr { Scrutinee = ConvertExpression(swExpr.GoverningExpression) };
                foreach (var arm in swExpr.Arms)
                {
                    var pat = ConvertPattern(arm.Pattern);
                    PilotIrExpr? guard = arm.WhenClause != null ? ConvertExpression(arm.WhenClause.Condition) : null;
                    var val = ConvertExpression(arm.Expression);
                    me.Arms.Add(new PilotIrMatchArm(pat, guard, new List<PilotIrStmt> { new PilotIrExprStmt { Expression = val } }, val));
                }
                return me;
        }

        return new PilotIrIdentifierExpr { Name = expr.ToString() };
    }

    private static PilotIrExpr ConvertInvocation(InvocationExpressionSyntax inv)
    {
        // Check LINQ .Select(x => proj) and .Where(x => pred).Select(x => proj) -> PilotIrForExpr
        if (inv.Expression is MemberAccessExpressionSyntax ma)
        {
            string method = ma.Name.Identifier.Text;
            if (method == "ToList" || method == "ToArray")
            {
                return ConvertExpression(ma.Expression);
            }

            if (method == "Select" && inv.ArgumentList.Arguments.Count == 1)
            {
                var lam = ConvertExpression(inv.ArgumentList.Arguments[0].Expression) as PilotIrLambdaExpr;
                if (lam != null && lam.Parameters.Count > 0 && lam.BodyExpr != null)
                {
                    if (ma.Expression is InvocationExpressionSyntax innerInv &&
                        innerInv.Expression is MemberAccessExpressionSyntax innerMa &&
                        innerMa.Name.Identifier.Text == "Where" &&
                        innerInv.ArgumentList.Arguments.Count == 1 &&
                        ConvertExpression(innerInv.ArgumentList.Arguments[0].Expression) is PilotIrLambdaExpr whereLam &&
                        whereLam.BodyExpr != null)
                    {
                        var fe = new PilotIrForExpr
                        {
                            Iterable = ConvertExpression(innerMa.Expression),
                            FilterCondition = whereLam.BodyExpr,
                            YieldExpr = lam.BodyExpr
                        };
                        fe.TargetVars.Add(lam.Parameters[0].Name);
                        return fe;
                    }

                    var simpleFe = new PilotIrForExpr
                    {
                        Iterable = ConvertExpression(ma.Expression),
                        YieldExpr = lam.BodyExpr
                    };
                    simpleFe.TargetVars.Add(lam.Parameters[0].Name);
                    return simpleFe;
                }
            }

            string targetName = ma.Expression.ToString();
            if (targetName is "Enumerable" or "System.Linq.Enumerable" &&
                method == "Range" &&
                inv.ArgumentList.Arguments.Count == 2)
            {
                var startExpr = ConvertExpression(inv.ArgumentList.Arguments[0].Expression);
                var countExpr = ConvertExpression(inv.ArgumentList.Arguments[1].Expression);

                // (end - start + 1) -> inclusive start..end
                if (countExpr is PilotIrBinaryExpr { Operator: "+", Right: PilotIrLiteralExpr { RawValue: "1" }, Left: PilotIrBinaryExpr { Operator: "-", Left: var endInc } })
                {
                    return new PilotIrRangeExpr { Start = startExpr, End = endInc, IsInclusive = true };
                }
                // (end - start) -> exclusive start..<end
                if (countExpr is PilotIrBinaryExpr { Operator: "-", Left: var endExc })
                {
                    return new PilotIrRangeExpr { Start = startExpr, End = endExc, IsInclusive = false };
                }
                if (startExpr is PilotIrLiteralExpr { Kind: PilotLiteralKind.Integer } sLit &&
                    countExpr is PilotIrLiteralExpr { Kind: PilotLiteralKind.Integer } cLit &&
                    long.TryParse(sLit.RawValue, out long sVal) &&
                    long.TryParse(cLit.RawValue, out long cVal))
                {
                    return new PilotIrRangeExpr
                    {
                        Start = startExpr,
                        End = new PilotIrLiteralExpr { Kind = PilotLiteralKind.Integer, RawValue = (sVal + cVal - 1).ToString() },
                        IsInclusive = true
                    };
                }
            }

            if (targetName is "Math" or "System.Math")
            {
                if (method == "Pow" && inv.ArgumentList.Arguments.Count == 2)
                {
                    return new PilotIrBinaryExpr
                    {
                        Operator = "**",
                        Left = ConvertExpression(inv.ArgumentList.Arguments[0].Expression),
                        Right = ConvertExpression(inv.ArgumentList.Arguments[1].Expression)
                    };
                }
                string mathFn = method.ToLowerInvariant();
                var mCall = new PilotIrCallExpr { Callee = new PilotIrIdentifierExpr { Name = mathFn } };
                foreach (var a in inv.ArgumentList.Arguments)
                    mCall.Arguments.Add(new PilotIrCallArg(null, ConvertExpression(a.Expression)));
                return mCall;
            }
        }

        var callee = ConvertExpression(inv.Expression);
        var call = new PilotIrCallExpr { Callee = callee };
        if (callee is PilotIrIdentifierExpr id)
        {
            if (id.Name == "println") call.LibraryCallKind = PilotLibraryCallKind.Println;
            else if (id.Name == "print") call.LibraryCallKind = PilotLibraryCallKind.Print;
        }
        foreach (var a in inv.ArgumentList.Arguments)
        {
            call.Arguments.Add(new PilotIrCallArg(a.NameColon?.Name.Identifier.Text, ConvertExpression(a.Expression)));
        }
        return call;
    }

    public static string MapCSharpType(TypeSyntax? typeSyntax)
    {
        if (typeSyntax == null)
            return "any";

        if (typeSyntax is NullableTypeSyntax nul)
        {
            return MapCSharpType(nul.ElementType) + "?";
        }

        if (typeSyntax is ArrayTypeSyntax arr)
        {
            return $"[{MapCSharpType(arr.ElementType)}]";
        }

        if (typeSyntax is GenericNameSyntax gen)
        {
            string baseId = gen.Identifier.Text;
            var args = gen.TypeArgumentList.Arguments.Select(MapCSharpType).ToList();
            if (baseId is "List" or "IList" or "IEnumerable" or "IReadOnlyList" && args.Count == 1)
                return $"[{args[0]}]";
            if (baseId is "Dictionary" or "IDictionary" && args.Count == 2)
                return $"{{{args[0]}: {args[1]}}}";
            if (baseId == "PilotResult" && args.Count == 2)
                return $"{args[0]} ! {args[1]}";
            return $"{baseId}[{string.Join(", ", args)}]";
        }

        string raw = typeSyntax.ToString().Trim();
        return raw switch
        {
            "int" or "Int32" => "int",
            "long" or "Int64" => "i64",
            "short" or "Int16" => "i16",
            "sbyte" => "i8",
            "byte" => "u8",
            "ushort" => "u16",
            "uint" => "u32",
            "ulong" => "u64",
            "double" or "Double" => "float",
            "float" or "Single" => "f32",
            "bool" or "Boolean" => "bool",
            "string" or "String" => "str",
            "void" or "Void" => "void",
            "object" or " dynamic" => "any",
            _ => raw
        };
    }
}
