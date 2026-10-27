using System;
using System.Collections.Generic;
using System.Linq;

namespace Pilot.Analysis.TypeSystem;

/// <summary>
/// Kind tag for static Pilot types.
/// </summary>
public enum PilotTypeKind
{
    Primitive,
    Collection,
    Tuple,
    Option,
    Function,
    Matrix,
    Unit,
    Union,
    TypeVariable,
    Resource,
    Any,
    Void,
    Null
}

/// <summary>
/// Represents a static type in the Pilot type inference and checking system.
/// Supports Hindley-Milner bidirectional type unification, generic collections,
/// dimensional units, nullability, and union types.
/// </summary>
public sealed class PilotType : IEquatable<PilotType>
{
    public PilotTypeKind Kind { get; }
    public string Name { get; }
    public IReadOnlyList<PilotType> TypeArguments { get; }
    public PilotType? ReturnType { get; }
    public string? UnitDimension { get; }
    public bool IsResource { get; }
    public int TypeVarId { get; }

    // Common primitive singletons
    public static readonly PilotType Int = new(PilotTypeKind.Primitive, "int");
    public static readonly PilotType Float = new(PilotTypeKind.Primitive, "float");
    public static readonly PilotType String = new(PilotTypeKind.Primitive, "str");
    public static readonly PilotType Bool = new(PilotTypeKind.Primitive, "bool");
    public static readonly PilotType Void = new(PilotTypeKind.Void, "void");
    public static readonly PilotType Any = new(PilotTypeKind.Any, "any");
    public static readonly PilotType Null = new(PilotTypeKind.Null, "null");

    private static int _nextVarId = 0;

    public PilotType(
        PilotTypeKind kind,
        string name,
        IReadOnlyList<PilotType>? typeArgs = null,
        PilotType? returnType = null,
        string? unitDimension = null,
        bool isResource = false,
        int typeVarId = 0)
    {
        Kind = kind;
        Name = name;
        TypeArguments = typeArgs ?? Array.Empty<PilotType>();
        ReturnType = returnType;
        UnitDimension = unitDimension;
        IsResource = isResource;
        TypeVarId = typeVarId;
    }

    public static PilotType ListOf(PilotType elementType) =>
        new(PilotTypeKind.Collection, "List", new[] { elementType });

    public static PilotType MapOf(PilotType keyType, PilotType valueType) =>
        new(PilotTypeKind.Collection, "Map", new[] { keyType, valueType });

    public static PilotType TupleOf(params PilotType[] elementTypes) =>
        new(PilotTypeKind.Tuple, "Tuple", elementTypes);

    public static PilotType OptionOf(PilotType innerType) =>
        new(PilotTypeKind.Option, $"{innerType.Name}?", new[] { innerType });

    public static PilotType FunctionOf(IReadOnlyList<PilotType> paramTypes, PilotType returnType) =>
        new(PilotTypeKind.Function, "Function", paramTypes, returnType);

    public static PilotType MatrixOf(int rows, int cols) =>
        new(PilotTypeKind.Matrix, $"matrix{rows}x{cols}");

    public static PilotType WithUnit(PilotType baseType, string unit) =>
        new(PilotTypeKind.Unit, $"{baseType.Name}<{unit}>", new[] { baseType }, unitDimension: unit);

    public static PilotType UnionOf(params PilotType[] types) =>
        new(PilotTypeKind.Union, string.Join(" | ", types.Select(t => t.ToString())), types);

    public static PilotType ResourceOf(string name) =>
        new(PilotTypeKind.Resource, name, isResource: true);

    public static PilotType NewTypeVariable(string prefix = "T") =>
        new(PilotTypeKind.TypeVariable, $"'{prefix}{++_nextVarId}", typeVarId: _nextVarId);

    private static readonly HashSet<string> IntegerTypes = new(StringComparer.OrdinalIgnoreCase)
    {
        "int", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "uint", "long", "byte"
    };

    private static readonly HashSet<string> FloatTypes = new(StringComparer.OrdinalIgnoreCase)
    {
        "float", "f8", "f16", "f32", "f64", "double", "decimal", "complex"
    };

    /// <summary>
    /// Checks whether this type is assignable to <paramref name="target"/>.
    /// Supports 'any' compatibility, subtyping, and optional unwrapping.
    /// </summary>
    public bool IsAssignableTo(PilotType target)
    {
        if (ReferenceEquals(this, target)) return true;
        if (target.Kind == PilotTypeKind.Any || Kind == PilotTypeKind.Any) return true;

        // Null assignable to Option<T>
        if (Kind == PilotTypeKind.Null && target.Kind == PilotTypeKind.Option) return true;

        // T assignable to Option<T>
        if (target.Kind == PilotTypeKind.Option && target.TypeArguments.Count == 1)
        {
            return IsAssignableTo(target.TypeArguments[0]);
        }

        // Numeric subtyping
        if (IntegerTypes.Contains(Name) && IntegerTypes.Contains(target.Name)) return true;
        if (FloatTypes.Contains(Name) && FloatTypes.Contains(target.Name)) return true;
        if (IntegerTypes.Contains(Name) && FloatTypes.Contains(target.Name)) return true;

        // Unit assignable from base numeric type
        if (target.Kind == PilotTypeKind.Unit && (IntegerTypes.Contains(Name) || FloatTypes.Contains(Name))) return true;

        // Union assignment: this matches if target contains this type
        if (target.Kind == PilotTypeKind.Union)
        {
            return target.TypeArguments.Any(t => IsAssignableTo(t));
        }

        // Collection assignment: List<T1> assignable to List<T2> if T1 assignable to T2
        if (Kind == target.Kind && Name == target.Name && TypeArguments.Count == target.TypeArguments.Count)
        {
            for (int i = 0; i < TypeArguments.Count; i++)
            {
                if (!TypeArguments[i].IsAssignableTo(target.TypeArguments[i]))
                    return false;
            }
            return true;
        }

        // Dimensional check: same base and unit
        if (Kind == PilotTypeKind.Unit && target.Kind == PilotTypeKind.Unit)
        {
            return string.Equals(UnitDimension, target.UnitDimension, StringComparison.OrdinalIgnoreCase) &&
                   TypeArguments[0].IsAssignableTo(target.TypeArguments[0]);
        }

        return string.Equals(Name, target.Name, StringComparison.OrdinalIgnoreCase);
    }

    /// <summary>
    /// Unifies this type with <paramref name="other"/> under a substitution map.
    /// Implements Robinson's unification algorithm for Hindley-Milner type inference.
    /// </summary>
    public bool Unify(PilotType other, Dictionary<int, PilotType> substitutions)
    {
        var t1 = Resolve(this, substitutions);
        var t2 = Resolve(other, substitutions);

        if (t1.Equals(t2)) return true;
        if (t1.Kind == PilotTypeKind.Any || t2.Kind == PilotTypeKind.Any) return true;

        if (t1.Kind == PilotTypeKind.TypeVariable)
        {
            substitutions[t1.TypeVarId] = t2;
            return true;
        }

        if (t2.Kind == PilotTypeKind.TypeVariable)
        {
            substitutions[t2.TypeVarId] = t1;
            return true;
        }

        if (t1.Kind == t2.Kind && string.Equals(t1.Name, t2.Name, StringComparison.OrdinalIgnoreCase))
        {
            if (t1.TypeArguments.Count != t2.TypeArguments.Count) return false;
            for (int i = 0; i < t1.TypeArguments.Count; i++)
            {
                if (!t1.TypeArguments[i].Unify(t2.TypeArguments[i], substitutions))
                    return false;
            }
            if (t1.ReturnType != null && t2.ReturnType != null)
            {
                return t1.ReturnType.Unify(t2.ReturnType, substitutions);
            }
            return true;
        }

        return false;
    }

    public static PilotType Resolve(PilotType type, Dictionary<int, PilotType> substitutions)
    {
        while (type.Kind == PilotTypeKind.TypeVariable && substitutions.TryGetValue(type.TypeVarId, out var resolved))
        {
            type = resolved;
        }
        return type;
    }

    public override string ToString()
    {
        if (Kind == PilotTypeKind.Option)
            return $"{TypeArguments[0]}?";
        if (Kind == PilotTypeKind.Collection && TypeArguments.Count > 0)
            return $"{Name}<{string.Join(", ", TypeArguments)}>";
        if (Kind == PilotTypeKind.Function && ReturnType != null)
            return $"({string.Join(", ", TypeArguments)}) -> {ReturnType}";
        if (Kind == PilotTypeKind.Unit)
            return $"{TypeArguments[0]}<{UnitDimension}>";
        if (Kind == PilotTypeKind.Union)
            return string.Join(" | ", TypeArguments);
        return Name;
    }

    public bool Equals(PilotType? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        if (Kind != other.Kind || !string.Equals(Name, other.Name, StringComparison.OrdinalIgnoreCase)) return false;
        if (TypeVarId != other.TypeVarId) return false;
        if (!string.Equals(UnitDimension, other.UnitDimension, StringComparison.OrdinalIgnoreCase)) return false;
        if (TypeArguments.Count != other.TypeArguments.Count) return false;

        for (int i = 0; i < TypeArguments.Count; i++)
        {
            if (!TypeArguments[i].Equals(other.TypeArguments[i])) return false;
        }
        return Equals(ReturnType, other.ReturnType);
    }

    public override bool Equals(object? obj) => obj is PilotType pt && Equals(pt);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add((int)Kind);
        hash.Add(Name, StringComparer.OrdinalIgnoreCase);
        hash.Add(TypeVarId);
        hash.Add(UnitDimension, StringComparer.OrdinalIgnoreCase);
        foreach (var arg in TypeArguments) hash.Add(arg);
        if (ReturnType != null) hash.Add(ReturnType);
        return hash.ToHashCode();
    }
}
