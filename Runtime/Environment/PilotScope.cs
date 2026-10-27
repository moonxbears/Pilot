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

    /// <summary>
    /// Declares a binding in this local scope, or shadows an existing local/outer binding with a new definition.
    /// </summary>
    public PilotBinding DeclareOrShadow(string name, PilotValue value, bool isConst = false, string? typeName = null)
    {
        var binding = new PilotBinding(name, value, isConst, typeName);
        _bindings[name] = binding;
        return binding;
    }

    /// <summary>
    /// Resolves an existing mutable binding in the scope chain or declares a new local binding,
    /// returning the direct <see cref="PilotBinding"/> slot for zero-lookup loop updates.
    /// </summary>
    public PilotBinding GetOrDeclareBinding(string name, PilotValue initialValue)
    {
        var current = this;
        while (current != null)
        {
            if (current._bindings.TryGetValue(name, out var existing))
            {
                if (existing.IsConst)
                {
                    throw new InvalidOperationException($"Cannot assign to immutable binding '{name}' (declared with 'let' or 'const'). Use 'var' if mutation is intended.");
                }
                existing.Value = initialValue;
                return existing;
            }
            current = current.Parent;
        }

        var binding = new PilotBinding(name, initialValue, false, null);
        _bindings[name] = binding;
        return binding;
    }

    public bool Assign(string name, PilotValue value)
    {
        var current = this;
        while (current != null)
        {
            if (current._bindings.TryGetValue(name, out var binding))
            {
                if (binding.IsConst)
                {
                    throw new InvalidOperationException($"Cannot assign to immutable binding '{name}' (declared with 'let' or 'const'). Use 'var' if mutation is intended.");
                }
                binding.Value = value;
                return true;
            }
            current = current.Parent;
        }

        return false;
    }

    public bool TryResolve(string name, out PilotValue value)
    {
        var current = this;
        while (current != null)
        {
            if (current._bindings.TryGetValue(name, out var binding))
            {
                value = binding.Value;
                return true;
            }
            current = current.Parent;
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

    /// <summary>
    /// Unconditionally sets or declares a binding in this scope (used by hot-reloading and rollback).
    /// </summary>
    public void ForceSet(string name, PilotValue value, bool isConst = false, string? typeName = null)
    {
        if (_bindings.TryGetValue(name, out var existing))
        {
            existing.Value = value;
        }
        else
        {
            _bindings[name] = new PilotBinding(name, value, isConst, typeName);
        }
    }

    public bool RemoveLocal(string name) => _bindings.Remove(name);
}
