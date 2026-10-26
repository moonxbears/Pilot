using System;
using System.Collections.Generic;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Debugging;

/// <summary>
/// Represents a single execution frame on the call stack.
/// Contains function identity, source location, parameters, scope, and local variables.
/// </summary>
public sealed class PilotStackFrame
{
    private static int _nextFrameId;

    public int Id { get; }
    public string FunctionName { get; }
    public string SourceFile { get; }
    public int Line { get; internal set; }
    public int Column { get; internal set; }
    public IReadOnlyList<PilotValue> Arguments { get; }
    public PilotScope Scope { get; }
    public PilotExecutionContext Context { get; }

    public PilotStackFrame(
        string functionName,
        string sourceFile,
        int line,
        int column,
        PilotScope scope,
        PilotExecutionContext context,
        IReadOnlyList<PilotValue>? arguments = null)
    {
        Id = System.Threading.Interlocked.Increment(ref _nextFrameId);
        FunctionName = string.IsNullOrEmpty(functionName) ? "<anonymous>" : functionName;
        SourceFile = sourceFile ?? string.Empty;
        Line = line;
        Column = column;
        Scope = scope ?? throw new ArgumentNullException(nameof(scope));
        Context = context ?? throw new ArgumentNullException(nameof(context));
        Arguments = arguments ?? Array.Empty<PilotValue>();
    }

    /// <summary>
    /// Gets all local variables defined strictly within this stack frame's scope.
    /// </summary>
    public IReadOnlyDictionary<string, PilotValue> GetLocals()
    {
        var locals = new Dictionary<string, PilotValue>(StringComparer.Ordinal);
        foreach (var kvp in Scope.GetLocalBindings())
        {
            locals[kvp.Key] = kvp.Value.Value;
        }
        return locals;
    }

    /// <summary>
    /// Gets all variables visible from this stack frame (locals + parent closure scopes + globals).
    /// </summary>
    public IReadOnlyDictionary<string, PilotValue> GetAllVariables()
    {
        var all = new Dictionary<string, PilotValue>(StringComparer.Ordinal);

        // Collect starting from global up to current scope so inner scopes override outer scopes
        var scopes = new List<PilotScope>();
        var current = Scope;
        while (current != null)
        {
            scopes.Add(current);
            current = current.Parent;
        }

        scopes.Reverse();
        foreach (var sc in scopes)
        {
            foreach (var kvp in sc.GetLocalBindings())
            {
                all[kvp.Key] = kvp.Value.Value;
            }
        }

        return all;
    }

    /// <summary>
    /// Modifies a variable visible in this stack frame.
    /// </summary>
    public bool SetVariable(string name, PilotValue value)
    {
        return Scope.Assign(name, value);
    }

    /// <summary>
    /// Evaluates an expression dynamically in the context of this frame's scope.
    /// </summary>
    public PilotValue Evaluate(string expression)
    {
        if (string.IsNullOrWhiteSpace(expression)) return PilotValue.Undefined;
        return Context.Engine.EvaluateExpression(expression, Scope);
    }

    public override string ToString() =>
        $"{FunctionName}() at {SourceFile}:{Line}:{Column}";
}
