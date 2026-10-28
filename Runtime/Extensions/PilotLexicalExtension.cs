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
    public string? GenericParam { get; set; }
    public string? GenericConstraint { get; set; }
    public Dictionary<string, PilotFunction> Methods { get; } = new(StringComparer.Ordinal);
    public Dictionary<string, PilotMethodGroup> MethodGroups { get; } = new(StringComparer.Ordinal);
    public Dictionary<string, PilotFunction> Properties { get; } = new(StringComparer.Ordinal);
    public Dictionary<string, PilotFunction> Setters { get; } = new(StringComparer.Ordinal);

    public PilotLexicalExtension(string name, string targetType, string? genericParam = null, string? genericConstraint = null)
    {
        Name = name;
        TargetType = targetType;
        GenericParam = genericParam;
        GenericConstraint = genericConstraint;
    }

    public bool MatchesType(PilotValue value)
    {
        return MatchesTypeSpec(value, TargetType.Trim());
    }

    private bool MatchesTypeSpec(PilotValue value, string typeSpec)
    {
        string normTarget = typeSpec.Trim().ToLowerInvariant();

        if (!string.IsNullOrEmpty(GenericParam) &&
            string.Equals(typeSpec.Trim(), GenericParam, StringComparison.OrdinalIgnoreCase))
        {
            return MatchesConstraint(value, GenericConstraint);
        }

        if (normTarget.EndsWith("[]", StringComparison.Ordinal))
        {
            if (value.Kind == PilotValueKind.NativeArray)
            {
                return true;
            }
            if (value.Kind != PilotValueKind.List && value.Kind != PilotValueKind.Tuple)
            {
                return false;
            }
            string elemSpec = typeSpec.Trim()[..^2].Trim();
            var items = value.Kind == PilotValueKind.List ? (IReadOnlyList<PilotValue>)value.AsList() : value.AsTuple();
            for (int i = 0; i < items.Count; i++)
            {
                if (!MatchesTypeSpec(items[i], elemSpec))
                {
                    return false;
                }
            }
            return true;
        }

        if (normTarget.StartsWith("list[", StringComparison.Ordinal) && normTarget.EndsWith("]", StringComparison.Ordinal))
        {
            if (value.Kind != PilotValueKind.List) return false;
            string elemSpec = typeSpec.Trim()[5..^1].Trim();
            var list = value.AsList();
            for (int i = 0; i < list.Count; i++)
            {
                if (!MatchesTypeSpec(list[i], elemSpec))
                {
                    return false;
                }
            }
            return true;
        }

        switch (normTarget)
        {
            case "any" or "dynamic":
                return true;
            case "str" or "string":
                return value.Kind == PilotValueKind.String;
            case "int" or "integer" or "i8" or "i16" or "i32" or "i64" or "u8" or "u16" or "u32" or "u64" or "isize" or "usize":
                return value.Kind == PilotValueKind.Int || value.Kind == PilotValueKind.UInt;
            case "float" or "double" or "f32" or "f64" or "decimal":
                return value.Kind == PilotValueKind.Float || value.Kind == PilotValueKind.Decimal;
            case "numeric" or "number":
                return value.IsNumber;
            case "bool" or "boolean":
                return value.Kind == PilotValueKind.Bool;
            case "char":
                return value.Kind == PilotValueKind.Char || (value.Kind == PilotValueKind.String && value.AsString().Length == 1);
            case "list" or "array":
                return value.Kind == PilotValueKind.List || value.Kind == PilotValueKind.NativeArray;
            case "map" or "dict":
                return value.Kind == PilotValueKind.Map || value.Kind == PilotValueKind.NativeMap;
            case "set":
                return value.Kind == PilotValueKind.Set;
            default:
                if (value.Kind == PilotValueKind.Object)
                {
                    return string.Equals(value.AsObject().TypeName, typeSpec, StringComparison.OrdinalIgnoreCase);
                }
                if (value.Kind == PilotValueKind.Record)
                {
                    return string.Equals(value.GetRecordDefinition().Name, typeSpec, StringComparison.OrdinalIgnoreCase);
                }
                if (value.Kind == PilotValueKind.Adt)
                {
                    var vd = value.GetAdtVariantDefinition();
                    return string.Equals(vd.ParentAdt.Name, typeSpec, StringComparison.OrdinalIgnoreCase) ||
                           string.Equals(vd.Name, typeSpec, StringComparison.OrdinalIgnoreCase);
                }
                return false;
        }
    }

    private static bool MatchesConstraint(PilotValue value, string? constraint)
    {
        if (string.IsNullOrWhiteSpace(constraint)) return true;
        string norm = constraint.Trim().ToLowerInvariant();
        return norm switch
        {
            "numeric" or "number" => value.IsNumber,
            "int" or "integer" => value.Kind is PilotValueKind.Int or PilotValueKind.UInt,
            "float" or "f64" or "f32" => value.Kind is PilotValueKind.Float or PilotValueKind.Decimal,
            "str" or "string" => value.Kind == PilotValueKind.String,
            _ => true
        };
    }

    public override string ToString() => $"<extension {Name} on {TargetType}>";
}
