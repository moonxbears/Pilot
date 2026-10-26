using System;
using System.Collections.Generic;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Environment;

/// <summary>
/// A lexical scope holding bindings with fallback to its parent scope.
/// </summary>
public sealed class PilotScope
{
    private readonly Dictionary<string, PilotBinding> _bindings = new(StringComparer.Ordinal);

    public PilotScope? Parent { get; }
    public string? ScopeName { get; }

    public PilotScope(PilotScope? parent = null, string? scopeName = null)
    {
        Parent = parent;
        ScopeName = scopeName;
    }

    public bool Declare(string name, PilotValue value, bool isConst = false, string? typeName = null)
    {
        if (_bindings.ContainsKey(name))
        {
            return false;
        }

        _bindings[name] = new PilotBinding(name, value, isConst, typeName);
        return true;
    }

    public bool Assign(string name, PilotValue value)
    {
        if (_bindings.TryGetValue(name, out var binding))
        {
            if (binding.IsConst)
            {
                throw new InvalidOperationException($"Cannot assign to constant variable '{name}'.");
            }
            binding.Value = value;
            return true;
        }

        if (Parent != null)
        {
            return Parent.Assign(name, value);
        }

        return false;
    }

    public bool TryResolve(string name, out PilotValue value)
    {
        if (_bindings.TryGetValue(name, out var binding))
        {
            value = binding.Value;
            return true;
        }

        if (Parent != null)
        {
            return Parent.TryResolve(name, out value);
        }

        value = PilotValue.Undefined;
        return false;
    }

    public PilotValue Resolve(string name)
    {
        if (TryResolve(name, out var val))
        {
            return val;
        }

        throw new KeyNotFoundException($"Undefined variable '{name}'.");
    }

    public bool ContainsLocal(string name) => _bindings.ContainsKey(name);

    public IReadOnlyDictionary<string, PilotBinding> GetLocalBindings() => _bindings;
}
