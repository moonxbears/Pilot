using System;
using System.Collections.Generic;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Extensions;

/// <summary>
/// Represents a lexically scoped extension (monkey-patch) on a target type.
/// Only active within designated 'with extension' blocks.
/// </summary>
public sealed class PilotLexicalExtension
{
    public string Name { get; }
    public string TargetType { get; }
    public Dictionary<string, PilotFunction> Methods { get; } = new(StringComparer.Ordinal);
    public Dictionary<string, PilotMethodGroup> MethodGroups { get; } = new(StringComparer.Ordinal);

    public PilotLexicalExtension(string name, string targetType)
    {
        Name = name;
        TargetType = targetType;
    }

    public bool MatchesType(PilotValue value)
    {
        string normTarget = TargetType.Trim().ToLowerInvariant();

        switch (normTarget)
        {
            case "str" or "string":
                return value.Kind == PilotValueKind.String;
            case "int" or "integer":
                return value.Kind == PilotValueKind.Int || value.Kind == PilotValueKind.UInt;
            case "float" or "double":
                return value.Kind == PilotValueKind.Float;
            case "bool" or "boolean":
                return value.Kind == PilotValueKind.Bool;
            case "list" or "array":
                return value.Kind == PilotValueKind.List;
            case "map" or "dict":
                return value.Kind == PilotValueKind.Map;
            case "set":
                return value.Kind == PilotValueKind.Set;
            default:
                if (value.Kind == PilotValueKind.Object)
                {
                    return string.Equals(value.AsObject().TypeName, TargetType, StringComparison.OrdinalIgnoreCase);
                }
                return false;
        }
    }

    public override string ToString() => $"<extension {Name} on {TargetType}>";
}
