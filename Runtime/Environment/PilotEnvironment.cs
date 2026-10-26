using System;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Environment;

/// <summary>
/// Manages the runtime lexical scope hierarchy and global bindings.
/// </summary>
public sealed class PilotEnvironment
{
    public PilotScope GlobalScope { get; }
    public PilotScope CurrentScope { get; private set; }

    public PilotEnvironment(PilotScope? globalScope = null)
    {
        GlobalScope = globalScope ?? new PilotScope(null, "Global");
        CurrentScope = GlobalScope;
    }

    public PilotScope PushScope(string? scopeName = null)
    {
        CurrentScope = new PilotScope(CurrentScope, scopeName);
        return CurrentScope;
    }

    public PilotScope PopScope()
    {
        if (CurrentScope.Parent == null)
        {
            throw new InvalidOperationException("Cannot pop the global scope.");
        }

        var popped = CurrentScope;
        CurrentScope = CurrentScope.Parent;
        return popped;
    }

    public void Declare(string name, PilotValue value, bool isConst = false, string? typeName = null) =>
        CurrentScope.Declare(name, value, isConst, typeName);

    public void Assign(string name, PilotValue value) =>
        CurrentScope.Assign(name, value);

    public PilotValue Resolve(string name) =>
        CurrentScope.Resolve(name);

    public bool TryResolve(string name, out PilotValue value) =>
        CurrentScope.TryResolve(name, out value);
}
