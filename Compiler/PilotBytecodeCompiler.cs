using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Runtime.Bytecode;
using Pilot.Runtime.Values;

namespace Pilot.Compiler;

/// <summary>
/// Compiles Pilot ANTLR parse trees into optimized <see cref="PilotBytecodeChunk"/> binaries.
/// Translates AST statements, expressions, control flow, functions, ADTs, records, O(1) match
/// jump tables, and collections into flat linear instruction streams with local slot registers.
/// </summary>
public sealed class PilotBytecodeCompiler : PilotBaseVisitor<object?>
{
    private readonly PilotBytecodeChunk _chunk;
    private readonly PilotBytecodeCompiler? _parent;
    private readonly Stack<LoopContext> _loops = new();
    private readonly bool _isFunction;
    private bool _discardExpressionResult = true;
    private int _tempCounter;

    private readonly HashSet<string> _immutableBindings = new(StringComparer.Ordinal);
    private readonly HashSet<string> _mutableBindings = new(StringComparer.Ordinal);
    private readonly Dictionary<string, PilotBytecodeTypeDescriptor> _adtDescriptors = new(StringComparer.Ordinal);
    private readonly Dictionary<string, (PilotBytecodeTypeDescriptor Adt, int Tag, PilotSerializedVariantDescriptor Variant)> _variantLookup = new(StringComparer.Ordinal);
    private readonly Dictionary<string, PilotBytecodeTypeDescriptor> _recordDescriptors = new(StringComparer.Ordinal);
    private readonly HashSet<string> _classAndStructNames = new(StringComparer.Ordinal);

    internal PilotBytecodeChunk Chunk => _chunk;
    internal bool IsFunction => _isFunction;
    internal bool DiscardExpressionResult
    {
        get => _discardExpressionResult;
        set => _discardExpressionResult = value;
    }

    private bool IsImmutable(string name)
    {
        if (_mutableBindings.Contains(name)) return false;
        if (_immutableBindings.Contains(name)) return true;
        return _parent?.IsImmutable(name) ?? false;
    }

    internal int AllocateTempSlot(string prefix) =>
        _chunk.GetOrAllocateLocalSlot($"<{prefix}_{_tempCounter++}>");

    internal void RegisterBindingMetadata(string name, bool isMutable)
    {
        if (string.IsNullOrEmpty(name) || name == "_") return;
        if (isMutable)
        {
            _mutableBindings.Add(name);
            _immutableBindings.Remove(name);
        }
        else
        {
            _immutableBindings.Add(name);
            _mutableBindings.Remove(name);
        }

        if (_isFunction)
        {
            _chunk.GetOrAllocateLocalSlot(name);
        }
    }

    internal bool HasDeclaredBinding(string name)
    {
        if (_chunk.LocalSlotMap.ContainsKey(name) ||
            _immutableBindings.Contains(name) ||
            _mutableBindings.Contains(name))
        {
            return true;
        }
        return _parent?.HasDeclaredBinding(name) ?? false;
    }

    internal void EmitStoreBinding(string name, bool isMutable)
    {
        RegisterBindingMetadata(name, isMutable);
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

    internal void EmitRestoreBinding(string name)
    {
        if (_isFunction && _chunk.TryGetLocalSlot(name, out int slot))
        {
            _chunk.Emit(PilotOpcode.StoreLocal, slot);
        }
        else
        {
            int idIndex = _chunk.AddIdentifier(name);
            _chunk.Emit(PilotOpcode.StoreNamed, idIndex);
        }
    }

    internal void EmitLoadBindingOrUndefined(string name)
    {
        if (_isFunction && _chunk.TryGetLocalSlot(name, out int slot))
        {
            _chunk.Emit(PilotOpcode.LoadLocal, slot);
        }
        else
        {
            int idIndex = _chunk.AddIdentifier(name);
            _chunk.Emit(PilotOpcode.LoadNamed, idIndex);
        }
    }

    internal void EmitLiteralConstant(PilotValue val)
    {
        switch (val.Kind)
        {
            case PilotValueKind.Null:
                _chunk.Emit(PilotOpcode.ConstNull);
                break;
            case PilotValueKind.Undefined:
                _chunk.Emit(PilotOpcode.ConstUndefined);
                break;
            case PilotValueKind.Bool:
                _chunk.Emit(val.IsTrue ? PilotOpcode.ConstTrue : PilotOpcode.ConstFalse);
                break;
            case PilotValueKind.Int when val.AsInt() >= int.MinValue && val.AsInt() <= int.MaxValue:
                _chunk.Emit(PilotOpcode.ConstInt, (int)val.AsInt());
                break;
            default:
                _chunk.Emit(PilotOpcode.Const, _chunk.AddConstant(val));
                break;
        }
    }

    internal bool TryResolveAdtVariant(
        string? explicitAdtName,
        string variantName,
        out PilotBytecodeTypeDescriptor adt,
        out int tag,
        out PilotSerializedVariantDescriptor variant)
    {
        if (!string.IsNullOrEmpty(explicitAdtName))
        {
            if (TryResolveAdtType(explicitAdtName, out var foundAdt))
            {
                for (int i = 0; i < foundAdt.Variants.Count; i++)
                {
                    if (string.Equals(foundAdt.Variants[i].Name, variantName, StringComparison.Ordinal))
                    {
                        adt = foundAdt;
                        tag = i;
                        variant = foundAdt.Variants[i];
                        return true;
                    }
                }
            }
            adt = null!;
            tag = -1;
            variant = null!;
            return false;
        }

        if (_variantLookup.TryGetValue(variantName, out var hit))
        {
            adt = hit.Adt;
            tag = hit.Tag;
            variant = hit.Variant;
            return true;
        }

        if (_parent != null)
        {
            return _parent.TryResolveAdtVariant(null, variantName, out adt, out tag, out variant);
        }

        adt = null!;
        tag = -1;
        variant = null!;
        return false;
    }

    internal bool TryResolveAdtType(string adtName, out PilotBytecodeTypeDescriptor adt)
    {
        if (_adtDescriptors.TryGetValue(adtName, out var found))
        {
            adt = found;
            return true;
        }
        if (_parent != null)
        {
            return _parent.TryResolveAdtType(adtName, out adt);
        }
        adt = null!;
        return false;
    }

    internal bool TryResolveRecordType(string recName, out PilotBytecodeTypeDescriptor rec)
    {
        if (_recordDescriptors.TryGetValue(recName, out var found))
        {
            rec = found;
            return true;
        }
        if (_parent != null)
        {
            return _parent.TryResolveRecordType(recName, out rec);
        }
        rec = null!;
        return false;
    }

    internal bool IsKnownClassOrStructName(string name) =>
        _classAndStructNames.Contains(name) || (_parent?.IsKnownClassOrStructName(name) ?? false);

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
        compiler.PrescanProgramTypeDeclarations(program);
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
                compiler._mutableBindings.Add(p);
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

    #region Pre-Scan & Type Descriptor Registration

    private void PrescanProgramTypeDeclarations(PilotParser.ProgramContext program)
    {
        var stmts = program.stmt();
        if (stmts == null) return;

        foreach (var stmt in stmts)
        {
            if (stmt.compound_stmt() is { } comp)
            {
                if (comp.enum_def() is { } enumCtx)
                {
                    RegisterAdtCompileTime(BuildEnumDescriptor(enumCtx, out _));
                }
                else if (comp.record_def() is { } recCtx)
                {
                    RegisterRecordCompileTime(BuildRecordDescriptor(recCtx, out _));
                }
                else if (comp.class_def() is { } clsCtx)
                {
                    _classAndStructNames.Add(clsCtx.identifier().GetText());
                }
                else if (comp.struct_def() is { } strCtx)
                {
                    _classAndStructNames.Add(strCtx.identifier().GetText());
                }
            }
            else if (stmt.simple_stmt()?.small_stmt() is { } smalls)
            {
                foreach (var sm in smalls)
                {
                    if (sm.type_alias_stmt() is { } taCtx &&
                        TryBuildTypeAliasAdtDescriptor(taCtx, out var adtTd, out _))
                    {
                        RegisterAdtCompileTime(adtTd);
                    }
                }
            }
        }
    }

    private void RegisterAdtCompileTime(PilotBytecodeTypeDescriptor td)
    {
        _adtDescriptors[td.Name] = td;
        for (int i = 0; i < td.Variants.Count; i++)
        {
            _variantLookup[td.Variants[i].Name] = (td, i, td.Variants[i]);
        }
    }

    private void RegisterRecordCompileTime(PilotBytecodeTypeDescriptor td)
    {
        _recordDescriptors[td.Name] = td;
    }

    private bool TryBuildTypeAliasAdtDescriptor(
        PilotParser.Type_alias_stmtContext context,
        out PilotBytecodeTypeDescriptor descriptor,
        out bool hasDefaultFieldExprs)
    {
        hasDefaultFieldExprs = false;
        string typeName = context.identifier().GetText();
        var genericParams = ExtractGenericParamList(context.generic_params());

        if (context.adt_sum_body() != null)
        {
            descriptor = new PilotBytecodeTypeDescriptor
            {
                IsRecord = false,
                Name = typeName,
                GenericParameters = genericParams
            };

            var sumBody = context.adt_sum_body();
            for (int i = 0; i < sumBody.ChildCount; i++)
            {
                var child = sumBody.GetChild(i);
                if (child is PilotParser.IdentifierContext unitId)
                {
                    descriptor.Variants.Add(new PilotSerializedVariantDescriptor { Name = unitId.GetText() });
                }
                else if (child is PilotParser.Adt_variantContext vCtx)
                {
                    descriptor.Variants.Add(new PilotSerializedVariantDescriptor
                    {
                        Name = vCtx.identifier().GetText(),
                        Fields = ExtractSerializedFields(vCtx.adt_field_list(), vCtx.struct_member(), ref hasDefaultFieldExprs)
                    });
                }
                else if (child is PilotParser.Adt_variant_payloadContext vpCtx)
                {
                    descriptor.Variants.Add(new PilotSerializedVariantDescriptor
                    {
                        Name = vpCtx.identifier().GetText(),
                        Fields = ExtractSerializedFields(vpCtx.adt_field_list(), vpCtx.struct_member(), ref hasDefaultFieldExprs)
                    });
                }
            }
            return true;
        }

        var branches = context.type_spec()?.union_type()?.intersection_type();
        if (branches != null && branches.Length >= 2)
        {
            var unitNames = new List<string>(branches.Length);
            bool allPascalUnits = true;
            foreach (var b in branches)
            {
                string txt = b.GetText().Trim();
                if (string.IsNullOrEmpty(txt) ||
                    !char.IsUpper(txt[0]) ||
                    !txt.All(c => char.IsLetterOrDigit(c) || c == '_') ||
                    txt is "Duration" or "Frequency" or "MemorySize" ||
                    IsKnownClassOrStructName(txt) ||
                    TryResolveAdtType(txt, out _) ||
                    TryResolveRecordType(txt, out _))
                {
                    allPascalUnits = false;
                    break;
                }
                unitNames.Add(txt);
            }

            if (allPascalUnits)
            {
                descriptor = new PilotBytecodeTypeDescriptor
                {
                    IsRecord = false,
                    Name = typeName,
                    GenericParameters = genericParams
                };
                foreach (var un in unitNames)
                {
                    descriptor.Variants.Add(new PilotSerializedVariantDescriptor { Name = un });
                }
                return true;
            }
        }

        descriptor = null!;
        return false;
    }

    private static PilotBytecodeTypeDescriptor BuildEnumDescriptor(
        PilotParser.Enum_defContext context,
        out bool hasDefaultFieldExprs)
    {
        hasDefaultFieldExprs = false;
        var td = new PilotBytecodeTypeDescriptor
        {
            IsRecord = false,
            Name = context.identifier().GetText(),
            GenericParameters = ExtractGenericParamList(context.generic_params())
        };

        var members = context.enum_suite()?.enum_member();
        if (members != null)
        {
            foreach (var m in members)
            {
                td.Variants.Add(new PilotSerializedVariantDescriptor
                {
                    Name = m.identifier().GetText(),
                    Fields = ExtractSerializedFields(m.adt_field_list(), m.struct_member(), ref hasDefaultFieldExprs)
                });
            }
        }

        return td;
    }

    private static PilotBytecodeTypeDescriptor BuildRecordDescriptor(
        PilotParser.Record_defContext context,
        out bool requiresAstFallback)
    {
        bool hasDefaults = false;
        var fields = ExtractSerializedFields(context.adt_field_list(), context.struct_member(), ref hasDefaults);
        bool hasBodyMethods = context.stmt() != null && context.stmt().Length > 0;
        requiresAstFallback = hasDefaults || hasBodyMethods;

        return new PilotBytecodeTypeDescriptor
        {
            IsRecord = true,
            Name = context.identifier().GetText(),
            GenericParameters = ExtractGenericParamList(context.generic_params()),
            RecordFields = fields
        };
    }

    private static List<string> ExtractGenericParamList(PilotParser.Generic_paramsContext? gp)
    {
        if (gp == null) return new List<string>();
        return gp.generic_param().Select(p => p.identifier().GetText()).ToList();
    }

    private static List<PilotSerializedFieldDescriptor> ExtractSerializedFields(
        PilotParser.Adt_field_listContext? fieldList,
        PilotParser.Struct_memberContext[]? structMembers,
        ref bool hasDefaultExpr)
    {
        var list = new List<PilotSerializedFieldDescriptor>();
        if (fieldList?.adt_field() != null)
        {
            foreach (var f in fieldList.adt_field())
            {
                if (f.test() != null) hasDefaultExpr = true;
                list.Add(new PilotSerializedFieldDescriptor
                {
                    Name = f.identifier()?.GetText(),
                    TypeName = f.type_spec()?.GetText() ?? "any"
                });
            }
        }
        if (structMembers != null)
        {
            foreach (var sm in structMembers)
            {
                if (sm.test() != null) hasDefaultExpr = true;
                list.Add(new PilotSerializedFieldDescriptor
                {
                    Name = sm.identifier()?.GetText(),
                    TypeName = sm.type_annotation()?.type_spec()?.GetText() ?? "any"
                });
            }
        }
        return list;
    }

    #endregion

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
        if (context.type_alias_stmt() != null) return Visit(context.type_alias_stmt());

        int fallback = _chunk.AddAstFallback(context);
        _chunk.Emit(PilotOpcode.EvalAst, fallback);
        _chunk.Emit(PilotOpcode.Pop);
        return null;
    }

    public override object? VisitCompound_stmt(PilotParser.Compound_stmtContext context)
    {
        if (context.if_stmt() != null) return Visit(context.if_stmt());
        if (context.while_stmt() != null) return Visit(context.while_stmt());
        if (context.for_stmt() != null) return Visit(context.for_stmt());
        if (context.func_def() != null) return Visit(context.func_def());
        if (context.match_stmt() != null) return Visit(context.match_stmt());
        if (context.guard_stmt() != null) return Visit(context.guard_stmt());
        if (context.enum_def() != null) return Visit(context.enum_def());
        if (context.record_def() != null) return Visit(context.record_def());

        if (context.class_def() != null)
        {
            _classAndStructNames.Add(context.class_def().identifier().GetText());
        }
        else if (context.struct_def() != null)
        {
            _classAndStructNames.Add(context.struct_def().identifier().GetText());
        }

        int fallback = _chunk.AddAstFallback(context);
        _chunk.Emit(PilotOpcode.EvalAst, fallback);
        _chunk.Emit(PilotOpcode.Pop);
        return null;
    }

    public override object? VisitType_alias_stmt(PilotParser.Type_alias_stmtContext context)
    {
        if (TryBuildTypeAliasAdtDescriptor(context, out var td, out bool hasDefaults))
        {
            RegisterAdtCompileTime(td);
            if (hasDefaults)
            {
                int fb = _chunk.AddAstFallback(context);
                _chunk.Emit(PilotOpcode.EvalAst, fb);
                _chunk.Emit(PilotOpcode.Pop);
            }
            else
            {
                int tdIdx = _chunk.AddTypeDescriptor(td);
                _chunk.Emit(PilotOpcode.DeclareTypeDescriptor, tdIdx);
            }
        }
        // Pure static type aliases (`type ID = int;`) have no runtime effect.
        return null;
    }

    public override object? VisitEnum_def(PilotParser.Enum_defContext context)
    {
        var td = BuildEnumDescriptor(context, out bool hasDefaults);
        RegisterAdtCompileTime(td);
        if (hasDefaults)
        {
            int fb = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fb);
            _chunk.Emit(PilotOpcode.Pop);
        }
        else
        {
            int tdIdx = _chunk.AddTypeDescriptor(td);
            _chunk.Emit(PilotOpcode.DeclareTypeDescriptor, tdIdx);
        }
        return null;
    }

    public override object? VisitRecord_def(PilotParser.Record_defContext context)
    {
        var td = BuildRecordDescriptor(context, out bool requiresFallback);
        RegisterRecordCompileTime(td);
        if (requiresFallback)
        {
            int fb = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fb);
            _chunk.Emit(PilotOpcode.Pop);
        }
        else
        {
            int tdIdx = _chunk.AddTypeDescriptor(td);
            _chunk.Emit(PilotOpcode.DeclareTypeDescriptor, tdIdx);
        }
        return null;
    }

    public override object? VisitMatch_stmt(PilotParser.Match_stmtContext context)
    {
        new PilotMatchBytecodeLowerer(this).LowerMatch(context, isExpression: false);
        return null;
    }

    public override object? VisitGuard_stmt(PilotParser.Guard_stmtContext context)
    {
        new PilotMatchBytecodeLowerer(this).LowerGuardStmt(context);
        return null;
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
        foreach (var id in ExtractBindingIdentifiers(context.binding_pattern()))
        {
            _immutableBindings.Add(id);
            _mutableBindings.Remove(id);
        }

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
        foreach (var id in ExtractBindingIdentifiers(context.binding_pattern()))
        {
            _mutableBindings.Add(id);
            _immutableBindings.Remove(id);
        }

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
        foreach (var id in ExtractBindingIdentifiers(context.binding_pattern()))
        {
            _immutableBindings.Add(id);
            _mutableBindings.Remove(id);
        }

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

    private static IEnumerable<string> ExtractBindingIdentifiers(PilotParser.Binding_patternContext? patternCtx)
    {
        if (patternCtx == null) yield break;
        if (patternCtx.identifier() is { } idCtx)
        {
            yield return idCtx.GetText();
            yield break;
        }
        if (patternCtx.splice_expr() is { } spliceCtx)
        {
            yield return spliceCtx.GetText();
            yield break;
        }
        if (patternCtx.tuple_pattern() is { } tuplePat)
        {
            foreach (var item in tuplePat.binding_pattern_item())
            {
                foreach (var name in ExtractBindingIdentifiers(item.binding_pattern()))
                    yield return name;
            }
        }
        else if (patternCtx.list_pattern() is { } listPat)
        {
            foreach (var item in listPat.binding_pattern_item())
            {
                foreach (var name in ExtractBindingIdentifiers(item.binding_pattern()))
                    yield return name;
            }
        }
        else if (patternCtx.record_pattern() is { } recPat)
        {
            foreach (var field in recPat.record_pattern_field())
            {
                if (field.binding_pattern() is { } subPat)
                {
                    foreach (var name in ExtractBindingIdentifiers(subPat))
                        yield return name;
                }
                else if (field.identifier() is { } fieldId)
                {
                    yield return fieldId.GetText();
                }
            }
        }
    }

    private void CompileVariableDeclaration(string name, PilotParser.TestContext? initExpr)
    {
        bool prevDiscard = _discardExpressionResult;
        _discardExpressionResult = false;
        if (initExpr != null)
        {
            Visit(initExpr);
        }
        else
        {
            _chunk.Emit(PilotOpcode.ConstUndefined);
        }
        _discardExpressionResult = prevDiscard;

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
            bool prevDiscard = _discardExpressionResult;
            _discardExpressionResult = false;
            Visit(context.test(0)); // Evaluate RHS
            _discardExpressionResult = prevDiscard;
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
            bool isLast = i == tests.Length - 1;
            bool shouldDiscard = _discardExpressionResult || !isLast;
            bool prevDiscard = _discardExpressionResult;
            _discardExpressionResult = false;
            Visit(tests[i]);
            _discardExpressionResult = prevDiscard;

            if (shouldDiscard)
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
            if (IsImmutable(idName))
            {
                throw new PilotCompilationException(
                    $"Cannot reassign immutable binding '{idName}' (declared with 'let' or 'const'). Use 'var' if mutation is intended.",
                    targetNode.Start.Line,
                    targetNode.Start.Column);
            }

            if (_isFunction && _chunk.TryGetLocalSlot(idName, out int targetSlot))
            {
                _chunk.Emit(PilotOpcode.StoreLocal, targetSlot);
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
                    int tempVal = _chunk.GetOrAllocateLocalSlot("<temp_val>");
                    _chunk.Emit(PilotOpcode.StoreLocal, tempVal);
                    CompileAtomExprPrefix(atomExpr, trailers.Length - 1);
                    Visit(subCtx.test(0));
                    _chunk.Emit(PilotOpcode.LoadLocal, tempVal);
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
        string op = aug.GetText();

        if (IsSimpleIdentifier(targetNode, out string idName))
        {
            if (IsImmutable(idName))
            {
                throw new PilotCompilationException(
                    $"Cannot reassign immutable binding '{idName}' (declared with 'let' or 'const'). Use 'var' if mutation is intended.",
                    targetNode.Start.Line,
                    targetNode.Start.Column);
            }

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
            EmitAugassignOp(op);

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

        // Member or index access assignment: a[i] += rhs or a.b += rhs
        var atomExpr = GetAtomExpr(targetNode);
        if (atomExpr != null && atomExpr.trailer()?.Length > 0)
        {
            var trailers = atomExpr.trailer();
            var lastTrailer = trailers[^1];

            if (lastTrailer.LSQB() != null)
            {
                var subCtx = lastTrailer.subscriptlist()?.subscript(0);
                if (subCtx != null)
                {
                    int tempReceiver = _chunk.GetOrAllocateLocalSlot("<temp_receiver>");
                    int tempIndex = _chunk.GetOrAllocateLocalSlot("<temp_index>");
                    int tempVal = _chunk.GetOrAllocateLocalSlot("<temp_val>");

                    CompileAtomExprPrefix(atomExpr, trailers.Length - 1);
                    _chunk.Emit(PilotOpcode.StoreLocal, tempReceiver);

                    Visit(subCtx.test(0));
                    _chunk.Emit(PilotOpcode.StoreLocal, tempIndex);

                    // Load current value: receiver[index]
                    _chunk.Emit(PilotOpcode.LoadLocal, tempReceiver);
                    _chunk.Emit(PilotOpcode.LoadLocal, tempIndex);
                    _chunk.Emit(PilotOpcode.GetIndex);

                    // Evaluate rhs
                    Visit(rhsNode);

                    // Compute op
                    EmitAugassignOp(op);

                    // Store back: receiver[index] = result
                    _chunk.Emit(PilotOpcode.StoreLocal, tempVal);
                    _chunk.Emit(PilotOpcode.LoadLocal, tempReceiver);
                    _chunk.Emit(PilotOpcode.LoadLocal, tempIndex);
                    _chunk.Emit(PilotOpcode.LoadLocal, tempVal);
                    _chunk.Emit(PilotOpcode.SetIndex);
                    return;
                }
            }

            if (lastTrailer.DOT() != null)
            {
                string member = lastTrailer.identifier().GetText();
                int memberId = _chunk.AddIdentifier(member);
                int tempReceiver = _chunk.GetOrAllocateLocalSlot("<temp_receiver>");
                int tempVal = _chunk.GetOrAllocateLocalSlot("<temp_val>");

                CompileAtomExprPrefix(atomExpr, trailers.Length - 1);
                _chunk.Emit(PilotOpcode.StoreLocal, tempReceiver);

                // Load current value: receiver.member
                _chunk.Emit(PilotOpcode.LoadLocal, tempReceiver);
                _chunk.Emit(PilotOpcode.GetMember, memberId);

                // Evaluate rhs
                Visit(rhsNode);

                // Compute op
                EmitAugassignOp(op);

                // Store back: receiver.member = result
                _chunk.Emit(PilotOpcode.StoreLocal, tempVal);
                _chunk.Emit(PilotOpcode.LoadLocal, tempReceiver);
                _chunk.Emit(PilotOpcode.LoadLocal, tempVal);
                _chunk.Emit(PilotOpcode.SetMember, memberId);
                return;
            }
        }

        // Fallback
        int fallback = _chunk.AddAstFallback(targetNode);
        _chunk.Emit(PilotOpcode.EvalAst, fallback);
        _chunk.Emit(PilotOpcode.Pop);
    }

    private void EmitAugassignOp(string op)
    {
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
    }

    public override object? VisitFlow_stmt(PilotParser.Flow_stmtContext context)
    {
        if (context.RETURN() != null)
        {
            var testlist = context.testlist();
            if (testlist?.test()?.Length > 0)
            {
                bool prevDiscard = _discardExpressionResult;
                _discardExpressionResult = false;
                Visit(testlist.test(0));
                _discardExpressionResult = prevDiscard;
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
        if (context.pattern() != null && context.pattern().Length > 0)
        {
            new PilotMatchBytecodeLowerer(this).LowerIfStmt(context);
            return null;
        }

        // Fast path for standard boolean `if / elif / else`
        bool prevCondDiscard = _discardExpressionResult;
        _discardExpressionResult = false;
        Visit(context.test(0));
        _discardExpressionResult = prevCondDiscard;
        int falseJump = _chunk.EmitJump(PilotOpcode.JumpIfFalse);

        Visit(context.suite(0));

        var endJumps = new List<int>();
        int endJump = _chunk.EmitJump(PilotOpcode.Jump);
        endJumps.Add(endJump);

        _chunk.PatchJump(falseJump);

        // elif / else if clauses
        int testCount = context.test().Length;
        int suiteIndex = 1;
        for (int i = 1; i < testCount; i++)
        {
            bool prevElifDiscard = _discardExpressionResult;
            _discardExpressionResult = false;
            Visit(context.test(i));
            _discardExpressionResult = prevElifDiscard;
            int elifFalseJump = _chunk.EmitJump(PilotOpcode.JumpIfFalse);
            Visit(context.suite(suiteIndex++));
            endJumps.Add(_chunk.EmitJump(PilotOpcode.Jump));
            _chunk.PatchJump(elifFalseJump);
        }

        // else clause
        if (suiteIndex < context.suite().Length)
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

        var exitJumps = new List<int>();
        if (context.pattern() != null)
        {
            new PilotMatchBytecodeLowerer(this).LowerWhileLetStmt(context, loopStart, exitJumps);
        }
        else
        {
            bool prevCondDiscard = _discardExpressionResult;
            _discardExpressionResult = false;
            Visit(context.test(0));
            _discardExpressionResult = prevCondDiscard;
            exitJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfFalse));
        }

        // Body
        bool prevDiscard = _discardExpressionResult;
        _discardExpressionResult = true;
        Visit(context.suite(0));
        _discardExpressionResult = prevDiscard;

        // Jump back to condition
        _chunk.Emit(PilotOpcode.Jump, loopStart);

        // Patch exit jumps
        foreach (int ej in exitJumps)
        {
            _chunk.PatchJump(ej);
        }

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
        bool prevIterDiscard = _discardExpressionResult;
        _discardExpressionResult = false;
        Visit(context.test(0));
        _discardExpressionResult = prevIterDiscard;
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
        _mutableBindings.Add(targetName);
        _immutableBindings.Remove(targetName);
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
        nestedChunk.FuncDefContext = context;
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
        if (context.IS()?.Length > 0 && context.pattern()?.Length > 0)
        {
            new PilotMatchBytecodeLowerer(this).LowerIsPatternExpr(context);
            return null;
        }

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
            bool isExclusive = context.DOTDOT_EQUAL() == null && context.ELLIPSIS() == null;
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
        if (context.MINUS() != null)
        {
            Visit(context.factor());
            _chunk.Emit(PilotOpcode.Neg);
            return null;
        }
        if (context.PLUS() != null)
        {
            Visit(context.factor());
            _chunk.Emit(PilotOpcode.Pos);
            return null;
        }
        if (context.TILDE() != null)
        {
            Visit(context.factor());
            _chunk.Emit(PilotOpcode.BitNot);
            return null;
        }
        if (context.unary_op() != null)
        {
            int fallback = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fallback);
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
        var trailers = context.trailer();
        if (trailers != null && RequiresAtomExprAstFallback(context, trailers))
        {
            int fallback = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fallback);
            return null;
        }

        Visit(context.atom());

        if (trailers != null)
        {
            for (int i = 0; i < trailers.Length; i++)
            {
                var trailer = trailers[i];

                // Method / Function / Variant / Record Call: (args)
                if (trailer.LPAR() != null)
                {
                    var args = trailer.arglist()?.argument();
                    var orderedTests = OrderCallArgumentExpressions(context, trailers, i, args);
                    for (int a = 0; a < orderedTests.Count; a++)
                    {
                        Visit(orderedTests[a]);
                    }
                    _chunk.Emit(PilotOpcode.Call, orderedTests.Count);
                }
                // Member Access: .member
                else if (trailer.DOT() != null)
                {
                    string member = trailer.identifier().GetText();
                    // Peek ahead: is next trailer a call?
                    if (i + 1 < trailers.Length && trailers[i + 1].LPAR() != null &&
                        !HasNamedArguments(trailers[i + 1].arglist()?.argument()))
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
                    if (subCtx != null && subCtx.test()?.Length == 1 && subCtx.DOTDOT() == null && subCtx.DOTDOT_EQUAL() == null)
                    {
                        Visit(subCtx.test(0));
                        _chunk.Emit(PilotOpcode.GetIndex);
                    }
                }
            }
        }

        return null;
    }

    private bool RequiresAtomExprAstFallback(PilotParser.Atom_exprContext context, PilotParser.TrailerContext[] trailers)
    {
        for (int i = 0; i < trailers.Length; i++)
        {
            var t = trailers[i];
            if (t.STAR() != null || t.SAFE_NAV() != null || t.cascade_trailer() != null ||
                t.unit_expr() != null || t.over_clause() != null)
            {
                return true;
            }
            if (t.LSQB() != null)
            {
                var sub = t.subscriptlist()?.subscript(0);
                if (sub == null || t.subscriptlist().subscript().Length != 1 ||
                    sub.DOTDOT() != null || sub.DOTDOT_EQUAL() != null || sub.test()?.Length != 1)
                {
                    return true;
                }
            }
            if (t.LPAR() != null)
            {
                var args = t.arglist()?.argument();
                if (args != null)
                {
                    foreach (var a in args)
                    {
                        if (a.EACH() != null || a.REPEAT() != null || a.DOTDOT() != null ||
                            a.STAR() != null || a.DOUBLESTAR() != null || a.test() == null)
                        {
                            return true;
                        }
                    }

                    if (HasNamedArguments(args) && !TryResolveKnownFieldOrderForCall(context, trailers, i, out _))
                    {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    private static bool HasNamedArguments(PilotParser.ArgumentContext[]? args)
    {
        if (args == null) return false;
        for (int i = 0; i < args.Length; i++)
        {
            if (args[i].identifier() != null && (args[i].COLON() != null || args[i].EQUAL() != null))
                return true;
        }
        return false;
    }

    private bool TryResolveKnownFieldOrderForCall(
        PilotParser.Atom_exprContext context,
        PilotParser.TrailerContext[] trailers,
        int callTrailerIndex,
        out IReadOnlyList<PilotSerializedFieldDescriptor> fields)
    {
        fields = null!;
        string? rootId = context.atom()?.identifier()?.GetText();
        if (string.IsNullOrEmpty(rootId)) return false;

        // Direct call: `Circle(radius: 5.0)` or `Point(y: 2.0, x: 1.0)`
        if (callTrailerIndex == 0)
        {
            if (TryResolveAdtVariant(null, rootId, out _, out _, out var vDesc))
            {
                fields = vDesc.Fields;
                return true;
            }
            if (TryResolveRecordType(rootId, out var rDesc))
            {
                fields = rDesc.RecordFields;
                return true;
            }
            return false;
        }

        // Qualified call: `Shape.Circle(radius: 5.0)` or `Expr[int].Const(value: 42)`
        if (callTrailerIndex >= 1 && trailers[callTrailerIndex - 1].DOT() != null)
        {
            string memberName = trailers[callTrailerIndex - 1].identifier().GetText();
            if (TryResolveAdtVariant(rootId, memberName, out _, out _, out var qualVar))
            {
                fields = qualVar.Fields;
                return true;
            }
        }

        return false;
    }

    private IReadOnlyList<PilotParser.TestContext> OrderCallArgumentExpressions(
        PilotParser.Atom_exprContext context,
        PilotParser.TrailerContext[] trailers,
        int callTrailerIndex,
        PilotParser.ArgumentContext[]? args)
    {
        if (args == null || args.Length == 0) return Array.Empty<PilotParser.TestContext>();
        if (!HasNamedArguments(args))
        {
            return args.Select(a => a.test()).ToArray();
        }

        if (TryResolveKnownFieldOrderForCall(context, trailers, callTrailerIndex, out var fields) &&
            args.Length == fields.Count)
        {
            var ordered = new PilotParser.TestContext?[fields.Count];
            int nextPos = 0;
            foreach (var a in args)
            {
                if (a.identifier() != null && (a.COLON() != null || a.EQUAL() != null))
                {
                    string fName = a.identifier().GetText();
                    int idx = -1;
                    for (int f = 0; f < fields.Count; f++)
                    {
                        if (string.Equals(fields[f].Name, fName, StringComparison.Ordinal))
                        {
                            idx = f;
                            break;
                        }
                    }
                    if (idx >= 0) ordered[idx] = a.test();
                }
                else
                {
                    while (nextPos < ordered.Length && ordered[nextPos] != null) nextPos++;
                    if (nextPos < ordered.Length) ordered[nextPos++] = a.test();
                }
            }
            if (ordered.All(o => o != null))
            {
                return ordered.Select(o => o!).ToArray();
            }
        }

        return args.Select(a => a.test()).ToArray();
    }

    public override object? VisitAtom(PilotParser.AtomContext context)
    {
        if (context.match_stmt() != null)
        {
            new PilotMatchBytecodeLowerer(this).LowerMatch(context.match_stmt(), isExpression: true);
            return null;
        }

        if (context.symbol_literal() != null)
        {
            var sym = context.symbol_literal();
            if (sym.identifier() != null)
            {
                int idx = _chunk.AddConstant(PilotValue.FromKeyword(sym.identifier().GetText()));
                _chunk.Emit(PilotOpcode.Const, idx);
                return null;
            }
            if (sym.SYMBOL() != null && sym.LPAR() == null)
            {
                int idx = _chunk.AddConstant(PilotValue.FromKeyword(sym.SYMBOL().GetText().TrimStart(':')));
                _chunk.Emit(PilotOpcode.Const, idx);
                return null;
            }
        }

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

        if (context.LPAR() != null && context.RPAR() != null)
        {
            if (context.testlist_comp() == null)
            {
                _chunk.Emit(PilotOpcode.BuildTuple, 0);
                return null;
            }

            if (context.testlist_comp().comp_for() == null)
            {
                var tests = context.testlist_comp().test();
                int count = tests?.Length ?? 0;
                bool hasTrailingComma = context.testlist_comp().COMMA()?.Length > 0;
                if (tests != null)
                {
                    for (int i = 0; i < tests.Length; i++)
                    {
                        Visit(tests[i]);
                    }
                }
                if (count > 1 || (count == 1 && hasTrailingComma))
                {
                    _chunk.Emit(PilotOpcode.BuildTuple, count);
                }
                return null;
            }
        }

        if (context.LBRACE() != null && context.RBRACE() != null)
        {
            var maker = context.dictorsetmaker();
            if (maker == null)
            {
                _chunk.Emit(PilotOpcode.BuildMap, 0);
                return null;
            }

            if (maker.comp_for() == null)
            {
                var dictEntries = maker.dict_entry();
                if (dictEntries != null && dictEntries.Length > 0 && dictEntries.All(de => de.COLON() != null))
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
                if (colElements != null && colElements.Length > 0 && colElements.All(ce => ce.DOTDOT() == null && ce.STAR() == null))
                {
                    for (int i = 0; i < colElements.Length; i++)
                    {
                        Visit(colElements[i].test());
                    }
                    _chunk.Emit(PilotOpcode.BuildSet, colElements.Length);
                    return null;
                }
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

        if (context.STRING()?.Length > 0 && context.STRING().Length == 1)
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
        if (context.comp_for() != null)
        {
            int fb = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fb);
            return null;
        }

        var elements = context.collection_element();
        if (elements != null && elements.Any(e => e.DOTDOT() != null || e.STAR() != null))
        {
            int fb = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fb);
            return null;
        }

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
