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

    public PilotParameter(string name, string? typeName = null, PilotValue? defaultValue = null, bool isParams = false, bool isKwArgs = false)
    {
        Name = name;
        TypeName = typeName;
        DefaultValue = defaultValue;
        IsParams = isParams;
        IsKwArgs = isKwArgs;
    }
}

/// <summary>
/// Represents a callable function, lambda, or native delegate in Pilot.
/// </summary>
public sealed class PilotFunction
{
    public string Name { get; }
    public IReadOnlyList<PilotParameter> Parameters { get; }
    public PilotScope? ClosureScope { get; }
    public ParserRuleContext? AstBody { get; }
    public Func<PilotExecutionContext, IReadOnlyList<PilotValue>, PilotValue>? HostDelegate { get; }
    public bool IsAsync { get; }
    public bool IsGenerator { get; }
    public IReadOnlyDictionary<string, PilotValue>? Captures { get; }
    public PilotClassDefinition? ClassDef { get; set; }
    public Dictionary<string, PilotValue> Members { get; } = new(StringComparer.Ordinal);

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
        var parameters = new List<PilotParameter>();
        foreach (var p in paramNames)
        {
            parameters.Add(new PilotParameter(p));
        }
        return new PilotFunction(name, parameters, hostDelegate);
    }

    public PilotValue Invoke(PilotExecutionContext context, IReadOnlyList<PilotValue> arguments)
    {
        if (HostDelegate != null)
        {
            return HostDelegate(context, arguments);
        }

        if (IsGenerator && context.Engine != null && AstBody != null)
        {
            return context.Engine.CreateGenerator(this, arguments, context);
        }

        if (context.Engine != null && AstBody != null)
        {
            return context.Engine.InvokeFunction(this, arguments, context);
        }

        throw new InvalidOperationException($"Cannot invoke function '{Name}': no executable body or host delegate available.");
    }

    public override string ToString() => $"<function {Name}>";
}
