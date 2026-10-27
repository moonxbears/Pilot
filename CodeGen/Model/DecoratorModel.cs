using System;
using System.Collections.Generic;

namespace Pilot.CodeGen.Model;

/// <summary>
/// Represents a decorator/attribute applied to a class, property, method, or parameter.
/// Simple access: name, positional args, and keyword args.
/// </summary>
public sealed class DecoratorModel
{
    public string Name { get; set; } = string.Empty;
    public List<string> PositionalArgs { get; } = new();
    public Dictionary<string, string> NamedArgs { get; } = new(StringComparer.OrdinalIgnoreCase);

    public DecoratorModel() { }

    public DecoratorModel(string name)
    {
        Name = name;
    }

    public string? GetArg(int index) => (index >= 0 && index < PositionalArgs.Count) ? PositionalArgs[index] : null;

    public string? GetNamed(string key, string? defaultValue = null) =>
        NamedArgs.TryGetValue(key, out var val) ? val : defaultValue;

    public override string ToString()
    {
        var parts = new List<string>(PositionalArgs);
        foreach (var (k, v) in NamedArgs)
        {
            parts.Add($"{k}={v}");
        }
        return parts.Count > 0 ? $"@{Name}({string.Join(", ", parts)})" : $"@{Name}";
    }
}
