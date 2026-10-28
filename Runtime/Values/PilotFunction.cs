using System;
using System.Collections.Generic;
using Antlr4.Runtime;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;

namespace Pilot.Runtime.Values;

/// <summary>
/// Parameter metadata for Pilot functions.
/// </summary>
public sealed class PilotParameter
{
    public string Name { get; }
    public string? TypeName { get; }
    public PilotValue? DefaultValue { get; }
    public bool IsParams { get; }
    public bool IsKwArgs { get; }
    public bool IsContextual { get; }
    public bool IsEach { get; }

    public PilotParameter(string name, string? typeName = null, PilotValue? defaultValue = null, bool isParams = false, bool isKwArgs = false, bool isContextual = false, bool isEach = false)
    {
        Name = name;
        TypeName = typeName;
        DefaultValue = defaultValue;
        IsParams = isParams;
        IsKwArgs = isKwArgs;
        IsContextual = isContextual;
        IsEach = isEach;
    }
}

/// <summary>
/// Represents a callable function, lambda, or native delegate in Pilot.
/// </summary>
public sealed class PilotFunction
{
    private Dictionary<string, PilotValue>? _members;
    private List<ParserRuleContext>? _requiresContracts;
    private List<ParserRuleContext>? _ensuresContracts;

    public string Name { get; }
    public IReadOnlyList<PilotParameter> Parameters { get; internal set; }
    public PilotScope? ClosureScope { get; private set; }
    public ParserRuleContext? AstBody { get; private set; }
    public Func<PilotExecutionContext, IReadOnlyList<PilotValue>, PilotValue>? HostDelegate { get; internal set; }
    public bool IsAsync { get; private set; }
    public bool IsGenerator { get; private set; }
    public bool IsVectorized { get; set; }
    public bool IsBurstCompiled { get; set; }
    public Func<IReadOnlyList<PilotValue>, PilotValue>? BurstKernel { get; set; }
    public IReadOnlyDictionary<string, PilotValue>? Captures { get; private set; }
    public PilotClassDefinition? ClassDef { get; set; }
    public Pilot.Runtime.Bytecode.PilotBytecodeChunk? BytecodeChunk { get; set; }
    public int PatchVersion { get; private set; } = 0;
    public string? ReturnTypeSpec { get; set; }
    public string? SuccessTypeSpec { get; set; }
    public string? ErrorTypeSpec { get; set; }
    public bool IsFallible { get; set; }
    public bool IsMacro { get; set; }
    public bool IsExternFfi { get; set; }
    public string? SourceBodyText { get; set; }

    public Dictionary<string, PilotValue> Members => _members ??= new(StringComparer.Ordinal);
    public bool HasMembers => _members != null && _members.Count > 0;
    public bool TryGetMember(string name, out PilotValue value)
    {
        if (_members != null && _members.TryGetValue(name, out value))
        {
            return true;
        }
        value = PilotValue.Undefined;
        return false;
    }

    public List<ParserRuleContext> RequiresContracts => _requiresContracts ??= new();
    public bool HasRequiresContracts => _requiresContracts != null && _requiresContracts.Count > 0;

    public List<ParserRuleContext> EnsuresContracts => _ensuresContracts ??= new();
    public bool HasEnsuresContracts => _ensuresContracts != null && _ensuresContracts.Count > 0;

    public PilotFunction(
        string name,
        IReadOnlyList<PilotParameter> parameters,
        ParserRuleContext? astBody,
        PilotScope? closureScope,
        bool isAsync = false,
        bool isGenerator = false,
        IReadOnlyDictionary<string, PilotValue>? captures = null)
    {
        Name = name;
        Parameters = parameters;
        AstBody = astBody;
        ClosureScope = closureScope;
        IsAsync = isAsync;
        IsGenerator = isGenerator;
        Captures = captures;
    }

    public PilotFunction(
        string name,
        IReadOnlyList<PilotParameter> parameters,
        Func<PilotExecutionContext, IReadOnlyList<PilotValue>, PilotValue> hostDelegate)
    {
        Name = name;
        Parameters = parameters;
        HostDelegate = hostDelegate;
    }

    public static PilotFunction CreateNative(
        string name,
        Func<PilotExecutionContext, IReadOnlyList<PilotValue>, PilotValue> hostDelegate,
        params string[] paramNames)
    {
        if (paramNames.Length == 0)
        {
            return new PilotFunction(name, Array.Empty<PilotParameter>(), hostDelegate);
        }

        var parameters = new PilotParameter[paramNames.Length];
        for (int i = 0; i < paramNames.Length; i++)
        {
            parameters[i] = new PilotParameter(paramNames[i]);
        }
        return new PilotFunction(name, parameters, hostDelegate);
    }

    public PilotValue Invoke(PilotExecutionContext context, IReadOnlyList<PilotValue> arguments)
    {
        if (BurstKernel != null)
        {
            return BurstKernel(arguments);
        }

        if (HostDelegate != null)
        {
            return HostDelegate(context, arguments);
        }

        if (IsGenerator && context.Engine != null && AstBody != null)
        {
            return context.Engine.CreateGenerator(this, arguments, context);
        }

        if (context.Engine != null && (AstBody != null || BytecodeChunk != null))
        {
            return context.Engine.InvokeFunction(this, arguments, context);
        }

        if (BytecodeChunk != null)
        {
            return Pilot.Runtime.Bytecode.PilVirtualMachine.Execute(BytecodeChunk, context, arguments, this);
        }

        throw new InvalidOperationException($"Cannot invoke function '{Name}': no executable body or host delegate available.");
    }

    public PilotValue Invoke(
        PilotExecutionContext context,
        IReadOnlyList<PilotValue> positionalArgs,
        IReadOnlyDictionary<string, PilotValue>? namedArgs)
    {
        if (namedArgs == null || namedArgs.Count == 0 || Parameters == null || Parameters.Count == 0)
        {
            return Invoke(context, positionalArgs);
        }

        var bound = new List<PilotValue>(Parameters.Count);
        int posIdx = 0;
        for (int i = 0; i < Parameters.Count; i++)
        {
            var p = Parameters[i];
            if (namedArgs.TryGetValue(p.Name, out var namedVal))
            {
                bound.Add(namedVal);
            }
            else if (posIdx < positionalArgs.Count)
            {
                bound.Add(positionalArgs[posIdx++]);
            }
            else if (p.DefaultValue.HasValue)
            {
                bound.Add(p.DefaultValue.Value);
            }
        }
        while (posIdx < positionalArgs.Count)
        {
            bound.Add(positionalArgs[posIdx++]);
        }
        return Invoke(context, bound);
    }

    /// <summary>
    /// Returns a canonical parameter signature key (arity + parameter types) for overload &amp; hot-patch matching.
    /// </summary>
    public string GetSignatureKey()
    {
        if (Parameters == null || Parameters.Count == 0) return "()";
        var sb = new System.Text.StringBuilder("(");
        bool first = true;
        for (int i = 0; i < Parameters.Count; i++)
        {
            var p = Parameters[i];
            if (i == 0 && p.Name == "self" && string.IsNullOrEmpty(p.TypeName))
            {
                continue;
            }
            if (!first) sb.Append(',');
            first = false;
            if (p.IsParams) sb.Append('*');
            else if (p.IsKwArgs) sb.Append("**");
            sb.Append(string.IsNullOrWhiteSpace(p.TypeName) ? "any" : p.TypeName.Trim());
        }
        sb.Append(')');
        return sb.ToString();
    }

    /// <summary>
    /// Creates an independent snapshot of this function's state for hot-reload rollback.
    /// </summary>
    public PilotFunction CloneSnapshot()
    {
        var copy = HostDelegate != null
            ? new PilotFunction(Name, new List<PilotParameter>(Parameters), HostDelegate)
            : new PilotFunction(
                Name,
                new List<PilotParameter>(Parameters),
                AstBody,
                ClosureScope,
                IsAsync,
                IsGenerator,
                Captures != null ? new Dictionary<string, PilotValue>(Captures, StringComparer.Ordinal) : null);

        copy.IsVectorized = IsVectorized;
        copy.ClassDef = ClassDef;
        copy.BytecodeChunk = BytecodeChunk;
        copy.PatchVersion = PatchVersion;
        copy.ReturnTypeSpec = ReturnTypeSpec;
        copy.SuccessTypeSpec = SuccessTypeSpec;
        copy.ErrorTypeSpec = ErrorTypeSpec;
        copy.IsFallible = IsFallible;
        copy.IsMacro = IsMacro;
        copy.IsExternFfi = IsExternFfi;
        copy.SourceBodyText = SourceBodyText;
        if (_members != null)
        {
            foreach (var kv in _members) copy.Members[kv.Key] = kv.Value;
        }
        if (_requiresContracts != null)
        {
            copy.RequiresContracts.AddRange(_requiresContracts);
        }
        if (_ensuresContracts != null)
        {
            copy.EnsuresContracts.AddRange(_ensuresContracts);
        }
        return copy;
    }

    /// <summary>
    /// Mutates this function instance in-place with the body, parameters, and contracts of <paramref name="patch"/>.
    /// Existing references, callbacks, and method group slots immediately observe the updated implementation.
    /// </summary>
    public void ApplyPatchFrom(PilotFunction patch, bool isRollback = false)
    {
        if (patch == null || ReferenceEquals(this, patch)) return;

        Parameters = patch.Parameters;
        AstBody = patch.AstBody;
        ClosureScope = patch.ClosureScope;
        HostDelegate = patch.HostDelegate;
        IsAsync = patch.IsAsync;
        IsGenerator = patch.IsGenerator;
        IsVectorized = patch.IsVectorized;
        BytecodeChunk = patch.BytecodeChunk;
        Captures = patch.Captures;
        ReturnTypeSpec = patch.ReturnTypeSpec;
        SuccessTypeSpec = patch.SuccessTypeSpec;
        ErrorTypeSpec = patch.ErrorTypeSpec;
        IsFallible = patch.IsFallible;
        IsMacro = patch.IsMacro;
        IsExternFfi = patch.IsExternFfi;
        SourceBodyText = patch.SourceBodyText;
        if (patch.ClassDef != null)
        {
            ClassDef = patch.ClassDef;
        }

        _requiresContracts?.Clear();
        if (patch.HasRequiresContracts)
        {
            RequiresContracts.AddRange(patch.RequiresContracts);
        }

        _ensuresContracts?.Clear();
        if (patch.HasEnsuresContracts)
        {
            EnsuresContracts.AddRange(patch.EnsuresContracts);
        }

        if (patch.HasMembers)
        {
            foreach (var kv in patch.Members)
            {
                Members[kv.Key] = kv.Value;
            }
        }

        PatchVersion = isRollback ? patch.PatchVersion : PatchVersion + 1;
    }

    public override string ToString() => $"<function {Name}>";
}
