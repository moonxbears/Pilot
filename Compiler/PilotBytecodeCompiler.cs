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
    private readonly HashSet<string> _contextualParameters = new(StringComparer.Ordinal);
    private Pilot.Analysis.Comptime.PilotStaticDemotionReport? _demotionReport;

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

    internal bool TryResolveUpvalueSlot(string name, out int slot, out int depthDelta)
    {
        int depth = 1;
        var curr = _parent;
        while (curr != null)
        {
            if (curr._isFunction && curr._chunk.TryGetLocalSlot(name, out slot))
            {
                depthDelta = depth;
                return true;
            }
            if (curr._isFunction)
            {
                depth++;
            }
            curr = curr._parent;
        }
        slot = -1;
        depthDelta = 0;
        return false;
    }

    internal void EmitRestoreBinding(string name)
    {
        if (_isFunction && _chunk.TryGetLocalSlot(name, out int slot))
        {
            _chunk.Emit(PilotOpcode.StoreLocal, slot);
        }
        else if (_isFunction && TryResolveUpvalueSlot(name, out int upSlot, out int depth))
        {
            _chunk.Emit(PilotOpcode.StoreUpvalue, upSlot, extra: depth);
        }
        else
        {
            int idIndex = _chunk.AddIdentifier(name);
            _chunk.Emit(PilotOpcode.StoreNamed, idIndex);
        }
    }

    internal void EmitLoadBindingOrUndefined(string name)
    {
        if (_contextualParameters.Contains(name))
        {
            int ctxSlot = Pilot.Runtime.Execution.PilotExecutionContext.InternContextSlotId(name);
            int idIdx = _chunk.AddIdentifier(name);
            _chunk.Emit(PilotOpcode.LoadContextSlot, ctxSlot, extra: idIdx);
        }
        else if (_isFunction && _chunk.TryGetLocalSlot(name, out int slot))
        {
            _chunk.Emit(PilotOpcode.LoadLocal, slot);
        }
        else if (_isFunction && TryResolveUpvalueSlot(name, out int upSlot, out int depth))
        {
            _chunk.Emit(PilotOpcode.LoadUpvalue, upSlot, extra: depth);
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
        _demotionReport = parent?._demotionReport;
    }

    /// <summary>
    /// Compiles a complete Pilot program AST into a bytecode chunk.
    /// </summary>
    public static PilotBytecodeChunk Compile(PilotParser.ProgramContext program, string? source = null)
    {
        var compiler = new PilotBytecodeCompiler("<program>");
        try
        {
            var typeEngine = new Pilot.Analysis.TypeSystem.PilotTypeInferenceEngine();
            typeEngine.VisitProgram(program);
            var contractVerifier = new Pilot.Analysis.ContractVerification.PilotContractVerifier();
            contractVerifier.VisitProgram(program);
            var comptimeEvaluator = new Pilot.Analysis.Comptime.PilotComptimeEvaluator();
            comptimeEvaluator.EvaluateProgram(program);
            compiler._demotionReport = Pilot.Analysis.Comptime.PilotStaticDemotionPass.Analyze(
                program, typeEngine, contractVerifier, comptimeEvaluator);
        }
        catch
        {
            // Keep compilation resilient if static analysis encounters partial syntax
        }
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
        PilotBytecodeCompiler? parent = null,
        IReadOnlyList<string>? contextualParameters = null)
    {
        var compiler = new PilotBytecodeCompiler(name, parent, isFunction: true);
        if (contextualParameters != null)
        {
            foreach (var cp in contextualParameters)
            {
                compiler._contextualParameters.Add(cp);
            }
        }
        if (parameters != null)
        {
            foreach (var p in parameters)
            {
                compiler._chunk.ParameterNames.Add(p);
                compiler._chunk.GetOrAllocateLocalSlot(p);
                compiler._mutableBindings.Add(p);
            }
        }

        if (suite.simple_stmt() != null)
        {
            compiler._discardExpressionResult = false;
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

    private void PrescanProgramTypeDeclarations(PilotParser.ProgramContext program, HashSet<string>? visitedModules = null)
    {
        var stmts = program.stmt();
        if (stmts == null) return;
        visitedModules ??= new HashSet<string>(StringComparer.Ordinal);

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
                    else if (sm.import_stmt() is { } impCtx)
                    {
                        PrescanImportedModuleTypes(impCtx, visitedModules);
                    }
                }
            }
        }
    }

    private void PrescanImportedModuleTypes(PilotParser.Import_stmtContext impCtx, HashSet<string> visitedModules)
    {
        if (impCtx.import_name()?.dotted_as_names()?.dotted_as_name() is { } specs)
        {
            foreach (var spec in specs)
            {
                string dotted = spec.dotted_name().GetText();
                if (Pilot.Runtime.Stdlib.PilotStdlibResolver.TryGetParsedModuleAst(dotted, 0, null, out var modAst, out string modId) &&
                    modAst != null && visitedModules.Add(modId))
                {
                    PrescanProgramTypeDeclarations(modAst, visitedModules);
                }
            }
        }
        else if (impCtx.import_from() is { } fromCtx)
        {
            int dots = fromCtx.DOT()?.Length ?? 0;
            string dotted = fromCtx.dotted_name()?.GetText() ?? string.Empty;
            if (Pilot.Runtime.Stdlib.PilotStdlibResolver.TryGetParsedModuleAst(dotted, dots, null, out var modAst, out string modId) &&
                modAst != null && visitedModules.Add(modId))
            {
                PrescanProgramTypeDeclarations(modAst, visitedModules);
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
        if (context.assert_stmt() != null) return Visit(context.assert_stmt());
        if (context.pass_stmt() != null)
        {
            if (!_discardExpressionResult) _chunk.Emit(PilotOpcode.ConstNull);
            return null;
        }

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
        if (context.comptime_stmt() != null) return Visit(context.comptime_stmt());
        if (context.with_stmt() != null && IsPureContextWithStmt(context.with_stmt()))
            return VisitWith_stmt(context.with_stmt());

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

    private static bool IsSliceBufferTypeAnnotation(PilotParser.Type_annotationContext? typeAnn)
    {
        string? rawType = typeAnn?.type_spec()?.GetText()?.Trim();
        if (string.IsNullOrEmpty(rawType)) return false;
        return rawType.Equals("SliceBuffer", StringComparison.OrdinalIgnoreCase) ||
               rawType.StartsWith("SliceBuffer[", StringComparison.OrdinalIgnoreCase) ||
               rawType.StartsWith("SliceBuffer<", StringComparison.OrdinalIgnoreCase) ||
               rawType.Equals("SharedBuffer", StringComparison.OrdinalIgnoreCase) ||
               rawType.StartsWith("SharedBuffer[", StringComparison.OrdinalIgnoreCase) ||
               rawType.StartsWith("SharedBuffer<", StringComparison.OrdinalIgnoreCase);
    }

    private static bool IsTopLevelForExpression(PilotParser.TestContext? test)
    {
        if (test == null) return false;
        string trimmed = test.GetText().TrimStart();
        return trimmed.StartsWith("for(", StringComparison.Ordinal) ||
               trimmed.StartsWith("for ", StringComparison.Ordinal) ||
               trimmed.StartsWith("for\t", StringComparison.Ordinal);
    }

    public override object? VisitLet_stmt(PilotParser.Let_stmtContext context)
    {
        foreach (var id in ExtractBindingIdentifiers(context.binding_pattern()))
        {
            _immutableBindings.Add(id);
            _mutableBindings.Remove(id);
        }

        string name = context.binding_pattern()?.identifier()?.GetText() ?? "";
        if (!string.IsNullOrEmpty(name) && !IsSliceBufferTypeAnnotation(context.type_annotation()))
        {
            CompileVariableDeclaration(name, context.test());
        }
        else if (TryCompileIrrefutableDestructuring(context.binding_pattern(), context.test(), isMutable: false))
        {
            return null;
        }
        else
        {
            if (_isFunction && !string.IsNullOrEmpty(name))
                _chunk.GetOrAllocateLocalSlot(name);
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
        if (!string.IsNullOrEmpty(name) && !IsSliceBufferTypeAnnotation(context.type_annotation()))
        {
            CompileVariableDeclaration(name, context.test());
        }
        else if (TryCompileIrrefutableDestructuring(context.binding_pattern(), context.test(), isMutable: true))
        {
            return null;
        }
        else
        {
            if (_isFunction && !string.IsNullOrEmpty(name))
                _chunk.GetOrAllocateLocalSlot(name);
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
        if (!string.IsNullOrEmpty(name) && !IsSliceBufferTypeAnnotation(context.type_annotation()))
        {
            CompileVariableDeclaration(name, context.test());
        }
        else if (TryCompileIrrefutableDestructuring(context.binding_pattern(), context.test(), isMutable: false))
        {
            return null;
        }
        else
        {
            if (_isFunction && !string.IsNullOrEmpty(name))
                _chunk.GetOrAllocateLocalSlot(name);
            int fallback = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fallback);
            _chunk.Emit(PilotOpcode.Pop);
        }
        return null;
    }

    private bool TryCompileIrrefutableDestructuring(
        PilotParser.Binding_patternContext? pat,
        PilotParser.TestContext? rhs,
        bool isMutable)
    {
        if (pat == null || rhs == null) return false;
        var items = pat.tuple_pattern()?.binding_pattern_item() ?? pat.list_pattern()?.binding_pattern_item();
        if (items == null || items.Length == 0) return false;

        foreach (var item in items)
        {
            if (item.DOTDOT() != null || item.STAR() != null || item.binding_pattern()?.identifier() == null)
            {
                return false;
            }
        }

        bool prevDiscard = _discardExpressionResult;
        _discardExpressionResult = false;
        Visit(rhs);
        _discardExpressionResult = prevDiscard;

        int tempSlot = AllocateTempSlot("destruct");
        _chunk.Emit(PilotOpcode.StoreLocal, tempSlot);

        for (int i = 0; i < items.Length; i++)
        {
            string id = items[i].binding_pattern().identifier().GetText();
            if (id == "_") continue;
            _chunk.Emit(PilotOpcode.LoadLocal, tempSlot);
            _chunk.Emit(PilotOpcode.GetAdtField, i);
            EmitStoreBinding(id, isMutable);
        }

        return true;
    }

    public override object? VisitAssert_stmt(PilotParser.Assert_stmtContext context)
    {
        if (context.COMPTIME() != null ||
            (_demotionReport != null && _demotionReport.ElidedContractNodes.Contains(context)))
        {
            if (!_discardExpressionResult)
            {
                _chunk.Emit(PilotOpcode.ConstTrue);
            }
            return null;
        }

        var tests = context.test();
        if (tests == null || tests.Length == 0) return null;

        bool prevDiscard = _discardExpressionResult;
        _discardExpressionResult = false;
        Visit(tests[0]);
        int hasMsg = 0;
        if (tests.Length > 1)
        {
            Visit(tests[1]);
            hasMsg = 1;
        }
        _discardExpressionResult = prevDiscard;

        _chunk.Emit(PilotOpcode.AssertCheck, context.Start.Line, extra: hasMsg);
        if (!_discardExpressionResult)
        {
            _chunk.Emit(PilotOpcode.ConstTrue);
        }
        return null;
    }

    public override object? VisitComptime_stmt(PilotParser.Comptime_stmtContext context)
    {
        if (_demotionReport != null && _demotionReport.MaterializedComptimeValues.TryGetValue(context, out var preVal))
        {
            if (!_discardExpressionResult)
            {
                EmitLiteralConstant(preVal);
            }
            return null;
        }

        bool prevDiscard = _discardExpressionResult;
        _discardExpressionResult = false;
        if (context.if_stmt() != null)
        {
            Visit(context.if_stmt());
        }
        else if (context.for_stmt() != null)
        {
            Visit(context.for_stmt());
        }
        else if (context.stmt() is { Length: > 0 } stmts)
        {
            for (int i = 0; i < stmts.Length; i++)
            {
                _discardExpressionResult = (i < stmts.Length - 1);
                Visit(stmts[i]);
            }
        }
        else
        {
            _chunk.Emit(PilotOpcode.ConstNull);
        }
        _chunk.Emit(PilotOpcode.FreezeTop);
        _discardExpressionResult = prevDiscard;
        if (_discardExpressionResult)
        {
            _chunk.Emit(PilotOpcode.Pop);
        }
        return null;
    }

    public override object? VisitComptime_expr(PilotParser.Comptime_exprContext context)
    {
        if (_demotionReport != null && _demotionReport.MaterializedComptimeValues.TryGetValue(context, out var preVal))
        {
            EmitLiteralConstant(preVal);
            return null;
        }

        bool prevDiscard = _discardExpressionResult;
        _discardExpressionResult = false;
        if (context.test() != null)
        {
            Visit(context.test());
        }
        else if (context.stmt() is { Length: > 0 } stmts)
        {
            for (int i = 0; i < stmts.Length; i++)
            {
                _discardExpressionResult = (i < stmts.Length - 1);
                Visit(stmts[i]);
            }
        }
        else
        {
            _chunk.Emit(PilotOpcode.ConstNull);
        }
        _chunk.Emit(PilotOpcode.FreezeTop);
        _discardExpressionResult = prevDiscard;
        return null;
    }

    private static bool IsPureContextWithStmt(PilotParser.With_stmtContext ctx)
    {
        if (ctx.ASYNC() != null) return false;
        var items = ctx.with_item();
        if (items == null || items.Length == 0) return false;
        foreach (var item in items)
        {
            if (item.CONTEXT() == null || item.with_context_binding() == null || item.with_context_binding().Length == 0)
            {
                return false;
            }
        }
        return true;
    }

    public override object? VisitWith_stmt(PilotParser.With_stmtContext context)
    {
        var pushedSlots = new List<(int SlotId, int IdIdx)>();
        bool prevDiscard = _discardExpressionResult;
        _discardExpressionResult = false;

        foreach (var item in context.with_item())
        {
            foreach (var cb in item.with_context_binding())
            {
                string key = cb.identifier().GetText();
                int slotId = Pilot.Runtime.Execution.PilotExecutionContext.InternContextSlotId(key);
                int idIdx = _chunk.AddIdentifier(key);
                Visit(cb.test());
                _chunk.Emit(PilotOpcode.PushContextSlot, slotId, extra: idIdx);
                pushedSlots.Add((slotId, idIdx));
            }
        }

        _discardExpressionResult = prevDiscard;
        Visit(context.suite());

        for (int i = pushedSlots.Count - 1; i >= 0; i--)
        {
            _chunk.Emit(PilotOpcode.PopContextSlot, pushedSlots[i].SlotId, extra: pushedSlots[i].IdIdx);
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
            if (_demotionReport != null && _demotionReport.MaterializedComptimeValues.TryGetValue(initExpr, out var preVal))
            {
                EmitLiteralConstant(preVal);
            }
            else
            {
                Visit(initExpr);
            }
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
            if (IsTopLevelForExpression(context.test(0)) &&
                IsSimpleIdentifier(tests[0], out string targetName) &&
                !IsImmutable(targetName))
            {
                int fallback = _chunk.AddAstFallback(context);
                _chunk.Emit(PilotOpcode.EvalAst, fallback);
                _chunk.Emit(PilotOpcode.Pop);
                return null;
            }

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
            else if (_isFunction && TryResolveUpvalueSlot(idName, out int upSlot, out int depth))
            {
                _chunk.Emit(PilotOpcode.StoreUpvalue, upSlot, extra: depth);
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
                if (_demotionReport != null && _demotionReport.FieldAccesses.TryGetValue(lastTrailer, out var fa))
                {
                    _chunk.Emit(PilotOpcode.SetFieldSlot, fa.FieldSlotIndex, extra: memberId);
                }
                else
                {
                    _chunk.Emit(PilotOpcode.SetMember, memberId);
                }
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
            else if (_isFunction && TryResolveUpvalueSlot(idName, out int upLoadSlot, out int loadDepth))
            {
                _chunk.Emit(PilotOpcode.LoadUpvalue, upLoadSlot, extra: loadDepth);
            }
            else
            {
                int idIndex = _chunk.AddIdentifier(idName);
                _chunk.Emit(PilotOpcode.LoadNamed, idIndex);
            }

            if (op == "??=")
            {
                _chunk.Emit(PilotOpcode.Dup);
                int skipJump = _chunk.EmitJump(PilotOpcode.JumpIfNotNull);
                _chunk.Emit(PilotOpcode.Pop);
                Visit(rhsNode);
                _chunk.PatchJump(skipJump);
            }
            else
            {
                Visit(rhsNode);
                EmitAugassignOp(op);
            }

            if (_isFunction && _chunk.TryGetLocalSlot(idName, out int storeSlot))
            {
                _chunk.Emit(PilotOpcode.StoreLocal, storeSlot);
            }
            else if (_isFunction && TryResolveUpvalueSlot(idName, out int upStoreSlot, out int storeDepth))
            {
                _chunk.Emit(PilotOpcode.StoreUpvalue, upStoreSlot, extra: storeDepth);
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
        if ((context.pattern() != null && context.pattern().Length > 0) ||
            (context.SOME() != null && context.SOME().Length > 0))
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
        else if (!_discardExpressionResult)
        {
            _chunk.Emit(PilotOpcode.ConstNull);
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
        if (context.pattern() != null || context.SOME() != null)
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
        var loopInfo = Pilot.Analysis.Loops.PilotForLoopAnalyzer.AnalyzeSingleLoop(context);
        if (loopInfo.ScheduleMode != Pilot.Analysis.Loops.LoopScheduleMode.FixedScalar ||
            loopInfo.ReturnMode != Pilot.Analysis.Loops.LoopReturnMode.Void ||
            loopInfo.HasCompletionBranch)
        {
            int fb = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fb);
            if (_discardExpressionResult)
            {
                _chunk.Emit(PilotOpcode.Pop);
            }
            return null;
        }

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
        if (Pilot.Runtime.Execution.PilotInterpreterVisitor.ContainsYield(context.suite()))
        {
            int fb = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fb);
            if (_discardExpressionResult)
            {
                _chunk.Emit(PilotOpcode.Pop);
            }
            return null;
        }

        string funcName = context.identifier().GetText();

        var paramNames = new List<string>();
        var contextualParams = new List<string>();
        var parameters = context.parameters()?.parameter();
        if (parameters != null)
        {
            foreach (var p in parameters)
            {
                string pName = p.identifier()?.GetText() ?? "arg";
                paramNames.Add(pName);
                if (p.CONTEXT() != null)
                {
                    contextualParams.Add(pName);
                }
            }
        }

        var nestedChunk = CompileSuite(context.suite(), paramNames, funcName, this, contextualParams);
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
            if (TryCompileFluentPipeStage(queries[i]))
            {
                continue;
            }
            Visit(queries[i]);
            _chunk.Emit(PilotOpcode.Swap);
            _chunk.Emit(PilotOpcode.Call, 1);
        }

        return null;
    }

    private bool TryCompileFluentPipeStage(PilotParser.Set_query_exprContext stage)
    {
        if (stage.set_op()?.Length > 0) return false;
        var coalesce = stage.coalesce_expr(0);
        if (coalesce == null || coalesce.or_test().Length != 1) return false;
        var orTest = coalesce.or_test(0);
        if (orTest.and_test().Length != 1) return false;
        var andTest = orTest.and_test(0);
        if (andTest.not_test().Length != 1 || andTest.not_test(0).NOT() != null) return false;
        var comp = andTest.not_test(0).comparison();
        if (comp == null || comp.range_expr().Length != 1) return false;
        var range = comp.range_expr(0);
        if (range.bitor_expr().Length != 1) return false;
        var bitor = range.bitor_expr(0);
        if (bitor.bitxor_expr().Length != 1) return false;
        var bitxor = bitor.bitxor_expr(0);
        if (bitxor.bitand_expr().Length != 1) return false;
        var bitand = bitxor.bitand_expr(0);
        if (bitand.shift_expr().Length != 1) return false;
        var shift = bitand.shift_expr(0);
        if (shift.arith_expr().Length != 1) return false;
        var arith = shift.arith_expr(0);
        if (arith.term().Length != 1) return false;
        var term = arith.term(0);
        if (term.factor().Length != 1) return false;
        var factor = term.factor(0);
        var power = factor.power();
        if (power == null || power.factor() != null) return false;
        var atomExpr = power.atom_expr();
        if (atomExpr == null || atomExpr.atom()?.DOT() == null || atomExpr.atom()?.identifier() == null)
        {
            return false;
        }
        string memberName = atomExpr.atom().identifier().GetText();
        var trailers = atomExpr.trailer();
        if (trailers == null || trailers.Length == 0)
        {
            int memberId = _chunk.AddIdentifier(memberName);
            _chunk.Emit(PilotOpcode.CallPrimIntrinsic, memberId, extra: 0);
            return true;
        }
        if (trailers.Length == 1 && trailers[0].LPAR() != null && trailers[0].STAR() == null &&
            trailers[0].SAFE_NAV() == null && trailers[0].QUESTION() == null)
        {
            var callArgs = trailers[0].arglist()?.argument();
            if (!HasNamedArguments(callArgs))
            {
                int argCount = callArgs?.Length ?? 0;
                if (callArgs != null)
                {
                    for (int a = 0; a < callArgs.Length; a++)
                    {
                        if (callArgs[a].EACH() != null || callArgs[a].REPEAT() != null || callArgs[a].DOTDOT() != null || callArgs[a].test() == null)
                        {
                            return false;
                        }
                    }
                    for (int a = 0; a < callArgs.Length; a++)
                    {
                        Visit(callArgs[a].test());
                    }
                }
                int memberId = _chunk.AddIdentifier(memberName);
                _chunk.Emit(PilotOpcode.CallPrimIntrinsic, memberId, extra: argCount);
                return true;
            }
        }
        return false;
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
        CompileCoalesceExprPrefix(context, stopBeforeCatch: null);
        return null;
    }

    internal void CompileCoalesceExprPrefix(PilotParser.Coalesce_exprContext context, PilotParser.Catch_clauseContext? stopBeforeCatch)
    {
        var orTests = context.or_test();
        if (orTests == null || orTests.Length == 0) return;

        Visit(orTests[0]);

        for (int i = 1; i < context.ChildCount; i++)
        {
            var child = context.GetChild(i);
            if (ReferenceEquals(child, stopBeforeCatch))
            {
                break;
            }

            if (child is PilotParser.Or_testContext rhsOrTest)
            {
                _chunk.Emit(PilotOpcode.Dup);
                int skipJump = _chunk.EmitJump(PilotOpcode.JumpIfNotNull);
                _chunk.Emit(PilotOpcode.Pop);
                Visit(rhsOrTest);
                _chunk.PatchJump(skipJump);
            }
            else if (child is PilotParser.Catch_clauseContext catchCtx)
            {
                _chunk.Emit(PilotOpcode.Dup);
                int skipJump = _chunk.EmitJump(PilotOpcode.JumpIfNotFailure);
                if (catchCtx.FAT_ARROW() != null && catchCtx.identifier() != null)
                {
                    string errVar = catchCtx.identifier().GetText();
                    _chunk.Emit(PilotOpcode.GetAdtField, 0, extra: 3);
                    if (errVar != "_")
                    {
                        EmitStoreBinding(errVar, isMutable: false);
                    }
                    else
                    {
                        _chunk.Emit(PilotOpcode.Pop);
                    }
                    if (catchCtx.block() != null)
                    {
                        CompileCatchBlockAsExpression(catchCtx.block());
                    }
                    else
                    {
                        Visit(catchCtx.test());
                    }
                }
                else if (catchCtx.block() != null)
                {
                    _chunk.Emit(PilotOpcode.Pop);
                    CompileCatchBlockAsExpression(catchCtx.block());
                }
                else if (catchCtx.or_test() != null)
                {
                    _chunk.Emit(PilotOpcode.Pop);
                    Visit(catchCtx.or_test());
                }
                _chunk.PatchJump(skipJump);
            }
        }
    }

    internal void CompileCatchBlockAsExpression(PilotParser.BlockContext block)
    {
        var stmts = block.stmt();
        if (stmts == null || stmts.Length == 0)
        {
            _chunk.Emit(PilotOpcode.ConstUndefined);
            return;
        }
        bool prevDiscard = _discardExpressionResult;
        for (int i = 0; i < stmts.Length; i++)
        {
            bool isLast = i == stmts.Length - 1;
            _discardExpressionResult = !isLast;
            Visit(stmts[i]);
        }
        _discardExpressionResult = prevDiscard;
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
        if (context.NOT() != null || context.EXCLAMATION() != null)
        {
            Visit(context.not_test());
            _chunk.Emit(PilotOpcode.Not);
            return null;
        }
        if (context.CALL_QUOTE_DEEP() != null)
        {
            Visit(context.not_test());
            _chunk.Emit(PilotOpcode.Not);
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
            bool isExclusive = context.DOTDOT_LESS() != null;
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

        if (context.ARROW()?.Length > 0)
        {
            if (_demotionReport != null &&
                _demotionReport.ProjectionPlans.TryGetValue(context, out var plan) &&
                _demotionReport.NominalFieldLayouts.TryGetValue(plan.TargetType, out var targetFields) &&
                factors.Length == 2)
            {
                Visit(factors[0]);
                int srcSlot = AllocateTempSlot("proj_src");
                _chunk.Emit(PilotOpcode.StoreLocal, srcSlot);

                for (int t = 0; t < targetFields.Count; t++)
                {
                    int srcFieldSlot = -1;
                    string? fName = null;
                    for (int m = 0; m < plan.FieldMappings.Count; m++)
                    {
                        if (plan.FieldMappings[m].TargetSlot == t)
                        {
                            srcFieldSlot = plan.FieldMappings[m].SourceSlot;
                            fName = plan.FieldMappings[m].FieldName;
                            break;
                        }
                    }

                    if (srcFieldSlot >= 0 && fName != null)
                    {
                        _chunk.Emit(PilotOpcode.LoadLocal, srcSlot);
                        int fId = _chunk.AddIdentifier(fName);
                        _chunk.Emit(PilotOpcode.GetFieldSlot, srcFieldSlot, extra: fId);
                    }
                    else
                    {
                        _chunk.Emit(PilotOpcode.ConstNull);
                    }
                }

                int typeId = _chunk.AddIdentifier(plan.TargetType);
                _chunk.Emit(PilotOpcode.ConstructStruct, typeId, extra: targetFields.Count);
                return null;
            }

            int fb = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fb);
            return null;
        }

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
        if (_demotionReport != null && _demotionReport.MaterializedComptimeValues.TryGetValue(context, out var preAtomVal))
        {
            EmitLiteralConstant(preAtomVal);
            return null;
        }

        if (context.atom()?.DOT() != null && context.atom()?.identifier() != null)
        {
            int fallback = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fallback);
            return null;
        }

        var trailers = context.trailer();
        int effectiveTrailerLen = PilotParser.GetEffectiveTrailerCount(context);
        if (trailers != null && effectiveTrailerLen > 0 && RequiresAtomExprAstFallback(context, trailers, effectiveTrailerLen))
        {
            PreallocateCascadeFallbackBindings(trailers, effectiveTrailerLen);
            int fallback = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fallback);
            return null;
        }

        Visit(context.atom());

        if (trailers != null && effectiveTrailerLen > 0)
        {
            var shortCircuitJumps = new List<int>();
            for (int i = 0; i < effectiveTrailerLen; i++)
            {
                var trailer = trailers[i];
                bool isSafeCascade = trailer.cascade_trailer()?.SAFE_CASCADE() != null;
                bool isOptional = trailer.SAFE_NAV() != null ||
                                  isSafeCascade ||
                                  (trailer.QUESTION() != null && (trailer.LSQB() != null || trailer.LPAR() != null));
                if (isOptional)
                {
                    shortCircuitJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfNullShortCircuit));
                }

                // Unit expression `<m/s>`: erased at compile time after PilotDimensionChecker (Milestone 19.1.4)
                if (trailer.unit_expr() != null)
                {
                    continue;
                }

                // Cascade Chaining: ..method(args), ..let out = method(args), ..method(args) -> out, ..prop = val, ..[idx] = val
                if (trailer.cascade_trailer() is { } cascade)
                {
                    CompileCascadeTrailer(cascade, shortCircuitJumps);
                }
                // Method / Function / Variant / Record Call: (args) or ?(args) or ?.(args)
                else if (trailer.LPAR() != null)
                {
                    var args = trailer.arglist()?.argument();
                    var orderedTests = OrderCallArgumentExpressions(context, trailers, i, args);
                    for (int a = 0; a < orderedTests.Count; a++)
                    {
                        Visit(orderedTests[a]);
                    }
                    _chunk.Emit(PilotOpcode.Call, orderedTests.Count);
                }
                // Member Access: .member or ?.member
                else if (trailer.identifier() != null && (trailer.DOT() != null || trailer.SAFE_NAV() != null))
                {
                    string member = trailer.identifier().GetText();
                    // Peek ahead: is next trailer a standard call?
                    if (i + 1 < effectiveTrailerLen &&
                        trailers[i + 1].LPAR() != null &&
                        trailers[i + 1].SAFE_NAV() == null &&
                        trailers[i + 1].QUESTION() == null &&
                        !HasNamedArguments(trailers[i + 1].arglist()?.argument()))
                    {
                        // Direct method / primitive intrinsic call: receiver.method(args)
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
                        _chunk.Emit(PilotOpcode.CallPrimIntrinsic, memberId, extra: argCount);
                    }
                    else
                    {
                        int memberId = _chunk.AddIdentifier(member);
                        if (_demotionReport != null && _demotionReport.FieldAccesses.TryGetValue(trailer, out var fa))
                        {
                            _chunk.Emit(PilotOpcode.GetFieldSlot, fa.FieldSlotIndex, extra: memberId);
                        }
                        else
                        {
                            _chunk.Emit(PilotOpcode.GetMember, memberId);
                        }
                    }
                }
                // Subscript Access: [index] or [start..end] or ?[index] or ?.[index]
                else if (trailer.LSQB() != null)
                {
                    var subCtx = trailer.subscriptlist()?.subscript(0);
                    if (subCtx != null && subCtx.test()?.Length == 1 && subCtx.DOTDOT() == null && subCtx.DOTDOT_EQUAL() == null)
                    {
                        Visit(subCtx.test(0));
                        _chunk.Emit(PilotOpcode.GetIndex);
                    }
                    else if (subCtx != null && subCtx.test()?.Length == 2 && (subCtx.DOTDOT() != null || subCtx.DOTDOT_EQUAL() != null))
                    {
                        Visit(subCtx.test(0));
                        Visit(subCtx.test(1));
                        _chunk.Emit(PilotOpcode.Slice, subCtx.DOTDOT_EQUAL() != null ? 0 : 1);
                    }
                }
                // Postfix `?` early-return error/none propagation (Milestone 13.2)
                else if (trailer.QUESTION() != null)
                {
                    _chunk.Emit(PilotOpcode.TryPropagate);
                }
            }

            foreach (int sj in shortCircuitJumps)
            {
                _chunk.PatchJump(sj);
            }
        }

        return null;
    }

    private bool RequiresAtomExprAstFallback(PilotParser.Atom_exprContext context, PilotParser.TrailerContext[] trailers, int effectiveLen)
    {
        if (context.atom()?.identifier()?.GetText() is "keyword" or "symbol" or "old")
        {
            return true;
        }

        for (int i = 0; i < effectiveLen; i++)
        {
            var t = trailers[i];
            if (t.call_quote_op() != null || t.STAR() != null ||
                t.over_clause() != null)
            {
                return true;
            }
            if (t.cascade_trailer() is { } cascade)
            {
                if (RequiresCascadeAstFallback(cascade))
                {
                    return true;
                }
                continue;
            }
            if (t.LSQB() != null)
            {
                var sub = t.subscriptlist()?.subscript(0);
                if (sub == null || t.subscriptlist().subscript().Length != 1)
                {
                    return true;
                }
                bool isSingleIdx = sub.DOTDOT() == null && sub.DOTDOT_EQUAL() == null && sub.test()?.Length == 1;
                bool isTwoBoundSlice = (sub.DOTDOT() != null || sub.DOTDOT_EQUAL() != null) && sub.test()?.Length == 2;
                if (!isSingleIdx && !isTwoBoundSlice)
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
                        if (a.TILDE() != null || a.EACH() != null || a.REPEAT() != null || a.DOTDOT() != null ||
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

    private bool RequiresCascadeAstFallback(PilotParser.Cascade_trailerContext cascade)
    {
        var target = cascade.cascade_target();
        if (target == null) return false;

        if (target.binding_pattern() is { } pat && pat.identifier() == null)
        {
            return true;
        }

        if (target.LPAR() != null)
        {
            string mName = target.dotted_name()?.GetText() ?? "";

            // Potential `obj..field = global_fn(args)` disambiguation case
            if (!mName.Contains('.') &&
                target.binding_pattern()?.identifier() != null &&
                target.LET() == null &&
                target.VAR() == null &&
                target.WALRUS() == null &&
                target.EQUAL() != null &&
                _chunk.TryGetLocalSlot(mName, out _))
            {
                return true;
            }

            var args = target.arglist()?.argument();
            if (args != null)
            {
                if (HasNamedArguments(args)) return true;
                foreach (var a in args)
                {
                    if (a.TILDE() != null || a.EACH() != null || a.REPEAT() != null || a.DOTDOT() != null ||
                        a.STAR() != null || a.DOUBLESTAR() != null || a.test() == null)
                    {
                        return true;
                    }
                }
            }
            return false;
        }

        if (target.LSQB() != null)
        {
            var subList = target.subscriptlist();
            var sub = subList?.subscript(0);
            if (sub == null || subList!.subscript().Length != 1 ||
                sub.DOTDOT() != null || sub.DOTDOT_EQUAL() != null || sub.test()?.Length != 1)
            {
                return true;
            }
        }

        if (target.EQUAL() != null && target.test() != null)
        {
            foreach (var hc in PilotParser.ExtractHoistedCascades(target.test()))
            {
                if (RequiresCascadeAstFallback(hc))
                {
                    return true;
                }
            }
        }

        return false;
    }

    private void PreallocateCascadeFallbackBindings(PilotParser.TrailerContext[] trailers, int effectiveLen)
    {
        for (int i = 0; i < effectiveLen; i++)
        {
            if (trailers[i].cascade_trailer() is { } cascade)
            {
                PreallocateSingleCascadeBindings(cascade);
            }
        }
    }

    private void PreallocateSingleCascadeBindings(PilotParser.Cascade_trailerContext cascade)
    {
        var target = cascade.cascade_target();
        if (target == null) return;

        if (target.binding_pattern() is { } pat)
        {
            bool isImmutable = target.LET() != null || (target.WALRUS() != null && target.VAR() == null);
            foreach (var id in ExtractBindingIdentifiers(pat))
            {
                if (isImmutable)
                {
                    _immutableBindings.Add(id);
                    _mutableBindings.Remove(id);
                }
                else
                {
                    _mutableBindings.Add(id);
                    if (target.VAR() != null) _immutableBindings.Remove(id);
                }

                if (_isFunction && !string.IsNullOrEmpty(id))
                {
                    _chunk.GetOrAllocateLocalSlot(id);
                }
            }
        }

        if (target.LPAR() == null && target.EQUAL() != null && target.test() != null)
        {
            foreach (var hc in PilotParser.ExtractHoistedCascades(target.test()))
            {
                PreallocateSingleCascadeBindings(hc);
            }
        }
    }

    private void CompileCascadeTrailer(PilotParser.Cascade_trailerContext cascade, List<int> shortCircuitJumps)
    {
        var target = cascade.cascade_target();
        if (target == null) return;

        // 1. Cascaded method call: ..method(args), ..let out = method(args), ..out = method(args), or ..method(args) -> out
        if (target.dotted_name() != null && target.LPAR() != null)
        {
            var segments = target.dotted_name().identifier();
            _chunk.Emit(PilotOpcode.Dup);
            for (int s = 0; s < segments.Length - 1; s++)
            {
                int segId = _chunk.AddIdentifier(segments[s].GetText());
                _chunk.Emit(PilotOpcode.GetMember, segId);
            }

            var callArgs = target.arglist()?.argument();
            int argCount = callArgs?.Length ?? 0;
            if (callArgs != null)
            {
                for (int a = 0; a < callArgs.Length; a++)
                {
                    Visit(callArgs[a].test());
                }
            }
            int memberId = _chunk.AddIdentifier(segments[^1].GetText());
            _chunk.Emit(PilotOpcode.CallPrimIntrinsic, memberId, extra: argCount);

            if (target.binding_pattern() is { } pat)
            {
                EmitCascadeCaptureStore(target, pat);
            }
            else
            {
                _chunk.Emit(PilotOpcode.Pop);
            }
            return;
        }

        // 2. Cascaded property read capture: ..let out = prop or ..prop -> out (supports multi-segment `headers.Length`)
        if (target.dotted_name() != null && target.LPAR() == null && target.binding_pattern() is { } propPat)
        {
            _chunk.Emit(PilotOpcode.Dup);
            foreach (var seg in target.dotted_name().identifier())
            {
                int memberId = _chunk.AddIdentifier(seg.GetText());
                _chunk.Emit(PilotOpcode.GetMember, memberId);
            }
            EmitCascadeCaptureStore(target, propPat);
            return;
        }

        // 3. Cascaded property write: ..prop = expr
        if (target.dotted_name() != null && target.LPAR() == null && target.EQUAL() != null && target.test() != null)
        {
            var segments = target.dotted_name().identifier();
            _chunk.Emit(PilotOpcode.Dup);
            for (int s = 0; s < segments.Length - 1; s++)
            {
                int segId = _chunk.AddIdentifier(segments[s].GetText());
                _chunk.Emit(PilotOpcode.GetMember, segId);
            }
            Visit(target.test());
            int memberId = _chunk.AddIdentifier(segments[^1].GetText());
            _chunk.Emit(PilotOpcode.SetMember, memberId);

            foreach (var hc in PilotParser.ExtractHoistedCascades(target.test()))
            {
                if (hc.SAFE_CASCADE() != null)
                {
                    shortCircuitJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfNullShortCircuit));
                }
                CompileCascadeTrailer(hc, shortCircuitJumps);
            }
            return;
        }

        // 4. Cascaded index write: ..[idx] = expr
        if (target.LSQB() != null && target.subscriptlist() != null && target.EQUAL() != null && target.test() != null)
        {
            var sub = target.subscriptlist().subscript(0);
            _chunk.Emit(PilotOpcode.Dup);
            Visit(sub.test(0));
            Visit(target.test());
            _chunk.Emit(PilotOpcode.SetIndex);

            foreach (var hc in PilotParser.ExtractHoistedCascades(target.test()))
            {
                if (hc.SAFE_CASCADE() != null)
                {
                    shortCircuitJumps.Add(_chunk.EmitJump(PilotOpcode.JumpIfNullShortCircuit));
                }
                CompileCascadeTrailer(hc, shortCircuitJumps);
            }
        }
    }

    private void EmitCascadeCaptureStore(
        PilotParser.Cascade_targetContext target,
        PilotParser.Binding_patternContext pat)
    {
        string idName = pat.identifier().GetText();
        bool hasLet = target.LET() != null || (target.WALRUS() != null && target.VAR() == null);
        bool hasVar = target.VAR() != null;

        if (hasLet)
        {
            _immutableBindings.Add(idName);
            _mutableBindings.Remove(idName);
            if (_isFunction)
            {
                int slot = _chunk.GetOrAllocateLocalSlot(idName);
                _chunk.Emit(PilotOpcode.StoreLocal, slot);
            }
            else
            {
                int idIdx = _chunk.AddIdentifier(idName);
                _chunk.Emit(PilotOpcode.DeclareNamed, idIdx);
            }
            return;
        }

        if (hasVar)
        {
            _mutableBindings.Add(idName);
            _immutableBindings.Remove(idName);
            if (_isFunction)
            {
                int slot = _chunk.GetOrAllocateLocalSlot(idName);
                _chunk.Emit(PilotOpcode.StoreLocal, slot);
            }
            else
            {
                int idIdx = _chunk.AddIdentifier(idName);
                _chunk.Emit(PilotOpcode.DeclareNamed, idIdx);
            }
            return;
        }

        // Direct variable capture (`..out = m()` or `..m() -> out`): assign if existing mutable, error if immutable, else declare
        if (IsImmutable(idName))
        {
            throw new PilotCompilationException(
                $"Cannot reassign immutable binding '{idName}' (declared with 'let' or 'const'). Use 'var' if mutation is intended.",
                pat.Start.Line,
                pat.Start.Column);
        }

        _mutableBindings.Add(idName);
        if (_isFunction)
        {
            int slot = _chunk.GetOrAllocateLocalSlot(idName);
            _chunk.Emit(PilotOpcode.StoreLocal, slot);
        }
        else
        {
            int idIdx = _chunk.AddIdentifier(idName);
            _chunk.Emit(PilotOpcode.StoreNamed, idIdx);
        }
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
        if (context.comptime_expr() != null)
        {
            return VisitComptime_expr(context.comptime_expr());
        }

        if (context.with_stmt() != null && IsPureContextWithStmt(context.with_stmt()))
        {
            bool prevDiscard = _discardExpressionResult;
            _discardExpressionResult = false;
            VisitWith_stmt(context.with_stmt());
            _discardExpressionResult = prevDiscard;
            return null;
        }

        if (context.if_stmt() != null)
        {
            bool prevDiscard = _discardExpressionResult;
            _discardExpressionResult = false;
            VisitIf_stmt(context.if_stmt());
            _discardExpressionResult = prevDiscard;
            return null;
        }

        if (context.match_stmt() != null)
        {
            if (context.match_stmt().test() == null)
            {
                int fb = _chunk.AddAstFallback(context.match_stmt());
                _chunk.Emit(PilotOpcode.EvalAst, fb);
                return null;
            }
            new PilotMatchBytecodeLowerer(this).LowerMatch(context.match_stmt(), isExpression: true);
            return null;
        }

        if (context.block() != null)
        {
            var stmts = context.block().stmt();
            if (stmts == null || stmts.Length == 0)
            {
                _chunk.Emit(PilotOpcode.ConstNull);
                return null;
            }
            bool prevDiscard = _discardExpressionResult;
            for (int i = 0; i < stmts.Length; i++)
            {
                _discardExpressionResult = (i < stmts.Length - 1);
                Visit(stmts[i]);
            }
            _discardExpressionResult = prevDiscard;
            return null;
        }

        if (context.symbol_literal() != null)
        {
            var sym = context.symbol_literal();
            if (sym.SYMBOL() == null)
            {
                int idx = _chunk.AddConstant(PilotValue.FromKeyword(sym.GetText().TrimStart(':')));
                _chunk.Emit(PilotOpcode.Const, idx);
                return null;
            }
            if (sym.SYMBOL() != null && sym.LPAR() == null)
            {
                int idx = _chunk.AddConstant(PilotValue.FromKeyword(sym.SYMBOL().GetText().TrimStart(':')));
                _chunk.Emit(PilotOpcode.Const, idx);
                return null;
            }
            int fb = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fb);
            return null;
        }

        if (context.overloadable_op() != null || context.macro_call() != null)
        {
            int fb = _chunk.AddAstFallback(context);
            _chunk.Emit(PilotOpcode.EvalAst, fb);
            return null;
        }

        if (context.identifier() != null)
        {
            string id = context.identifier().GetText();
            EmitLoadBindingOrUndefined(id);
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

        if (context.DURATION_LITERAL() != null)
        {
            double val = Pilot.Runtime.Execution.PilotInterpreterVisitor.ParseDurationLiteralSeconds(context.DURATION_LITERAL().GetText());
            int idx = _chunk.AddConstant(PilotValue.FromFloat(val));
            _chunk.Emit(PilotOpcode.Const, idx);
            return null;
        }

        if (context.FREQUENCY_LITERAL() != null)
        {
            double val = Pilot.Runtime.Execution.PilotInterpreterVisitor.ParseFrequencyLiteralHz(context.FREQUENCY_LITERAL().GetText());
            int idx = _chunk.AddConstant(PilotValue.FromFloat(val));
            _chunk.Emit(PilotOpcode.Const, idx);
            return null;
        }

        if (context.MEMORY_LITERAL() != null)
        {
            long val = Pilot.Runtime.Execution.PilotInterpreterVisitor.ParseMemoryLiteralBytes(context.MEMORY_LITERAL().GetText());
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

        if (context.STRING()?.Length > 0 && context.STRING().Length == 1)
        {
            string raw = context.STRING(0).GetText();
            if ((raw.StartsWith("f\"", StringComparison.OrdinalIgnoreCase) || raw.StartsWith("f'", StringComparison.OrdinalIgnoreCase)) &&
                raw.Length >= 3 && (raw.EndsWith("\"") || raw.EndsWith("'")))
            {
                int fb = _chunk.AddAstFallback(context);
                _chunk.Emit(PilotOpcode.EvalAst, fb);
                return null;
            }
            string unescaped = UnescapeStringLiteral(raw);
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

    private static string UnescapeStringLiteral(string raw)
    {
        if (string.IsNullOrEmpty(raw)) return raw;
        if ((raw.StartsWith("r\"", StringComparison.OrdinalIgnoreCase) || raw.StartsWith("r'", StringComparison.OrdinalIgnoreCase)) && raw.Length >= 3)
        {
            return StripQuotes(raw[1..]);
        }

        string str = StripQuotes(raw);
        if (str.IndexOf('\\') < 0)
        {
            return str;
        }
        var sb = new System.Text.StringBuilder(str.Length);
        for (int i = 0; i < str.Length; i++)
        {
            char c = str[i];
            if (c == '\\' && i + 1 < str.Length)
            {
                char next = str[++i];
                switch (next)
                {
                    case 'n': sb.Append('\n'); break;
                    case 'r': sb.Append('\r'); break;
                    case 't': sb.Append('\t'); break;
                    case '0': sb.Append('\0'); break;
                    case '\\': sb.Append('\\'); break;
                    case '"': sb.Append('"'); break;
                    case '\'': sb.Append('\''); break;
                    default:
                        sb.Append('\\');
                        sb.Append(next);
                        break;
                }
            }
            else
            {
                sb.Append(c);
            }
        }
        return sb.ToString();
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
