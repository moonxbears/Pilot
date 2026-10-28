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
    RowRecord,
    Resource,
    Any,
    Void,
    Null
}

/// <summary>
/// Built-in typeclass / trait bounds attached to type variables during Hindley-Milner and row inference.
/// </summary>
[Flags]
public enum PilotTypeClassConstraint
{
    None = 0,
    Numeric = 1 << 0,
    Comparable = 1 << 1,
    Equatable = 1 << 2,
    Addable = 1 << 3,
    Iterable = 1 << 4
}

/// <summary>
/// Represents a static type in the Pilot type inference and checking system.
/// Supports Hindley-Milner bidirectional type unification, generic collections,
/// structural row polymorphism, typestates, dimensional units, nullability, and union types.
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
    public PilotTypeClassConstraint TypeVarConstraint { get; }
    public IReadOnlyDictionary<string, PilotType> RowFields { get; }
    public bool IsOpenRow { get; }
    public string? RowTailName { get; }
    public string? ActiveTypestate { get; }

    private static readonly IReadOnlyDictionary<string, PilotType> EmptyRowFields =
        new Dictionary<string, PilotType>(StringComparer.Ordinal);

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
        int typeVarId = 0,
        PilotTypeClassConstraint typeVarConstraint = PilotTypeClassConstraint.None,
        IReadOnlyDictionary<string, PilotType>? rowFields = null,
        bool isOpenRow = false,
        string? rowTailName = null,
        string? activeTypestate = null)
    {
        Kind = kind;
        Name = name;
        TypeArguments = typeArgs ?? Array.Empty<PilotType>();
        ReturnType = returnType;
        UnitDimension = unitDimension;
        IsResource = isResource;
        TypeVarId = typeVarId;
        TypeVarConstraint = typeVarConstraint;
        RowFields = rowFields ?? EmptyRowFields;
        IsOpenRow = isOpenRow;
        RowTailName = rowTailName;
        ActiveTypestate = activeTypestate;
    }

    public static PilotType ListOf(PilotType elementType) =>
        new(PilotTypeKind.Collection, "List", new[] { elementType });

    public static PilotType SetOf(PilotType elementType) =>
        new(PilotTypeKind.Collection, "Set", new[] { elementType });

    public static PilotType MapOf(PilotType keyType, PilotType valueType) =>
        new(PilotTypeKind.Collection, "Map", new[] { keyType, valueType });

    public static PilotType FutureOf(PilotType innerType) =>
        new(PilotTypeKind.Collection, "Future", new[] { innerType });

    public static PilotType TupleOf(params PilotType[] elementTypes) =>
        new(PilotTypeKind.Tuple, "Tuple", elementTypes);

    public static PilotType OptionOf(PilotType innerType)
    {
        if (innerType.Kind == PilotTypeKind.Option) return innerType;
        return new(PilotTypeKind.Option, $"{innerType.Name}?", new[] { innerType });
    }

    public static PilotType ResultOf(PilotType okType, PilotType errType) =>
        new(PilotTypeKind.Primitive, "Result", new[] { okType, errType });

    public bool IsResult =>
        Kind == PilotTypeKind.Primitive &&
        string.Equals(Name, "Result", StringComparison.Ordinal) &&
        TypeArguments.Count == 2;

    public static PilotType FunctionOf(IReadOnlyList<PilotType> paramTypes, PilotType returnType) =>
        new(PilotTypeKind.Function, "Function", paramTypes, returnType);

    public static PilotType MatrixOf(int rows, int cols) =>
        new(PilotTypeKind.Matrix, $"matrix{rows}x{cols}");

    public static PilotType WithUnit(PilotType baseType, string unit) =>
        new(PilotTypeKind.Unit, $"{baseType.Name}<{unit}>", new[] { baseType }, unitDimension: unit);

    public static PilotType WithTypestate(PilotType baseType, string stateName) =>
        new(
            baseType.Kind,
            baseType.Name,
            baseType.TypeArguments,
            baseType.ReturnType,
            baseType.UnitDimension,
            baseType.IsResource,
            baseType.TypeVarId,
            baseType.TypeVarConstraint,
            baseType.RowFields,
            baseType.IsOpenRow,
            baseType.RowTailName,
            activeTypestate: stateName);

    public static PilotType RowRecordOf(
        IReadOnlyDictionary<string, PilotType> fields,
        bool isOpenRow = true,
        string? rowTailName = "R",
        string? nominalName = null)
    {
        var copy = new Dictionary<string, PilotType>(fields, StringComparer.Ordinal);
        return new PilotType(
            PilotTypeKind.RowRecord,
            nominalName ?? "RowRecord",
            rowFields: copy,
            isOpenRow: isOpenRow,
            rowTailName: isOpenRow ? (rowTailName ?? "R") : null);
    }

    public static PilotType UnionOf(params PilotType[] types)
    {
        var flat = new List<PilotType>();
        bool hasNull = false;
        foreach (var t in types)
        {
            if (t == null) continue;
            if (t.Kind == PilotTypeKind.Null)
            {
                hasNull = true;
                continue;
            }
            if (t.Kind == PilotTypeKind.Option && t.TypeArguments.Count == 1)
            {
                hasNull = true;
                AddUnique(flat, t.TypeArguments[0]);
                continue;
            }
            if (t.Kind == PilotTypeKind.Union)
            {
                foreach (var sub in t.TypeArguments)
                {
                    if (sub.Kind == PilotTypeKind.Null) hasNull = true;
                    else AddUnique(flat, sub);
                }
            }
            else
            {
                AddUnique(flat, t);
            }
        }

        if (flat.Count == 0)
            return hasNull ? Null : Void;

        PilotType core = flat.Count == 1
            ? flat[0]
            : new PilotType(PilotTypeKind.Union, string.Join(" | ", flat.Select(t => t.ToString())), flat);

        return hasNull ? OptionOf(core) : core;
    }

    private static void AddUnique(List<PilotType> list, PilotType item)
    {
        if (!list.Any(existing => existing.Equals(item)))
            list.Add(item);
    }

    public static PilotType ResourceOf(string name) =>
        new(PilotTypeKind.Resource, name, isResource: true);

    public static PilotType NewTypeVariable(string prefix = "T", PilotTypeClassConstraint constraint = PilotTypeClassConstraint.None) =>
        new(PilotTypeKind.TypeVariable, $"'{prefix}{++_nextVarId}", typeVarId: _nextVarId, typeVarConstraint: constraint);

    public PilotType WithConstraint(PilotTypeClassConstraint additionalConstraint) =>
        Kind == PilotTypeKind.TypeVariable
            ? new PilotType(
                PilotTypeKind.TypeVariable,
                Name,
                TypeArguments,
                ReturnType,
                UnitDimension,
                IsResource,
                TypeVarId,
                TypeVarConstraint | additionalConstraint)
            : this;

    private static readonly HashSet<string> IntegerTypes = new(StringComparer.OrdinalIgnoreCase)
    {
        "int", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "uint", "long", "byte"
    };

    private static readonly HashSet<string> FloatTypes = new(StringComparer.OrdinalIgnoreCase)
    {
        "float", "f8", "f16", "f32", "f64", "double", "decimal", "complex"
    };

    public bool IsNumericType =>
        IntegerTypes.Contains(Name) || FloatTypes.Contains(Name) || Kind == PilotTypeKind.Unit ||
        (Kind == PilotTypeKind.TypeVariable && (TypeVarConstraint & PilotTypeClassConstraint.Numeric) != 0);

    public bool IsIntegerPrimitive => IntegerTypes.Contains(Name);
    public bool IsFloatPrimitive => FloatTypes.Contains(Name);

    public bool SatisfiesConstraint(PilotTypeClassConstraint constraint)
    {
        if (constraint == PilotTypeClassConstraint.None || Kind == PilotTypeKind.Any)
            return true;
        if (Kind == PilotTypeKind.TypeVariable)
            return true;

        if ((constraint & PilotTypeClassConstraint.Numeric) != 0)
        {
            if (!IsNumericType && Kind != PilotTypeKind.Matrix)
                return false;
        }
        if ((constraint & PilotTypeClassConstraint.Addable) != 0)
        {
            if (!IsNumericType && !string.Equals(Name, "str", StringComparison.OrdinalIgnoreCase) && Kind != PilotTypeKind.Collection)
                return false;
        }
        if ((constraint & PilotTypeClassConstraint.Comparable) != 0)
        {
            if (!IsNumericType && !string.Equals(Name, "str", StringComparison.OrdinalIgnoreCase) && !string.Equals(Name, "char", StringComparison.OrdinalIgnoreCase))
                return false;
        }
        return true;
    }

    /// <summary>
    /// Subtracts <paramref name="removed"/> from this type during negative branch flow narrowing
    /// (e.g., <c>T?</c> minus <c>null</c> yields <c>T</c>; <c>A | B</c> minus <c>A</c> yields <c>B</c>).
    /// </summary>
    public PilotType SubtractType(PilotType removed)
    {
        if (removed.Kind == PilotTypeKind.Null)
        {
            if (Kind == PilotTypeKind.Option && TypeArguments.Count == 1)
                return TypeArguments[0];
        }

        if (Kind == PilotTypeKind.Option && TypeArguments.Count == 1 && TypeArguments[0].Equals(removed))
        {
            return Null;
        }

        if (Kind == PilotTypeKind.Union)
        {
            var remaining = TypeArguments
                .Where(t => !t.Equals(removed) && !string.Equals(t.Name, removed.Name, StringComparison.OrdinalIgnoreCase))
                .ToArray();
            if (remaining.Length == 0) return Void;
            if (remaining.Length == 1) return remaining[0];
            return UnionOf(remaining);
        }

        return this;
    }

    /// <summary>
    /// Checks whether this type is assignable to <paramref name="target"/>.
    /// Supports 'any' compatibility, subtyping, structural rows, typestates, and optional unwrapping.
    /// </summary>
    public bool IsAssignableTo(PilotType target)
    {
        if (ReferenceEquals(this, target)) return true;
        if (target.Kind == PilotTypeKind.Any || Kind == PilotTypeKind.Any) return true;
        if (target.Kind == PilotTypeKind.TypeVariable)
            return SatisfiesConstraint(target.TypeVarConstraint);
        if (Kind == PilotTypeKind.TypeVariable)
            return target.SatisfiesConstraint(TypeVarConstraint);

        // Null assignable to Option<T>
        if (Kind == PilotTypeKind.Null && target.Kind == PilotTypeKind.Option) return true;

        // T assignable to Option<T>
        if (target.Kind == PilotTypeKind.Option && target.TypeArguments.Count == 1)
        {
            if (Kind == PilotTypeKind.Option && TypeArguments.Count == 1)
                return TypeArguments[0].IsAssignableTo(target.TypeArguments[0]);
            return IsAssignableTo(target.TypeArguments[0]);
        }

        // T assignable to Result<T, E>
        if (target.IsResult && !IsResult)
        {
            return IsAssignableTo(target.TypeArguments[0]);
        }

        // Result<T1, E1> assignable to Result<T2, E2>
        if (IsResult && target.IsResult)
        {
            return TypeArguments[0].IsAssignableTo(target.TypeArguments[0]) &&
                   TypeArguments[1].IsAssignableTo(target.TypeArguments[1]);
        }

        // Numeric subtyping
        if (IntegerTypes.Contains(Name) && IntegerTypes.Contains(target.Name)) return true;
        if (FloatTypes.Contains(Name) && FloatTypes.Contains(target.Name)) return true;
        if (IntegerTypes.Contains(Name) && FloatTypes.Contains(target.Name)) return true;

        // Unit assignable from base numeric type
        if (target.Kind == PilotTypeKind.Unit && (IntegerTypes.Contains(Name) || FloatTypes.Contains(Name))) return true;

        // Union assignment:
        // 1) If this is a Union, every member must be assignable to target
        if (Kind == PilotTypeKind.Union)
        {
            return TypeArguments.All(t => t.IsAssignableTo(target));
        }

        // 2) If target is a Union, this matches if target contains a member compatible with this type
        if (target.Kind == PilotTypeKind.Union)
        {
            return target.TypeArguments.Any(t => IsAssignableTo(t));
        }

        // Structural Row Polymorphism: target is a RowRecord
        if (target.Kind == PilotTypeKind.RowRecord)
        {
            if (Kind == PilotTypeKind.RowRecord)
            {
                foreach (var req in target.RowFields)
                {
                    if (!RowFields.TryGetValue(req.Key, out var actualFieldType) ||
                        !actualFieldType.IsAssignableTo(req.Value))
                    {
                        return false;
                    }
                }
                if (!target.IsOpenRow && RowFields.Count != target.RowFields.Count)
                    return false;
                return true;
            }
        }

        // Typestate check: if target specifies an active typestate, this must match it
        if (!string.IsNullOrEmpty(target.ActiveTypestate) &&
            !string.IsNullOrEmpty(ActiveTypestate) &&
            !string.Equals(ActiveTypestate, target.ActiveTypestate, StringComparison.Ordinal))
        {
            return false;
        }

        // Function assignment
        if (Kind == PilotTypeKind.Function && target.Kind == PilotTypeKind.Function)
        {
            if (TypeArguments.Count != target.TypeArguments.Count) return false;
            for (int i = 0; i < TypeArguments.Count; i++)
            {
                // Contravariant parameter check
                if (!target.TypeArguments[i].IsAssignableTo(TypeArguments[i]))
                    return false;
            }
            if (ReturnType != null && target.ReturnType != null)
                return ReturnType.IsAssignableTo(target.ReturnType);
            return true;
        }

        // Collection / Tuple assignment: List<T1> assignable to List<T2> if T1 assignable to T2
        if (Kind == target.Kind && string.Equals(Name, target.Name, StringComparison.OrdinalIgnoreCase) && TypeArguments.Count == target.TypeArguments.Count)
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
    /// Implements Robinson's unification algorithm + structural row unification for Hindley-Milner type inference.
    /// </summary>
    public bool Unify(PilotType other, Dictionary<int, PilotType> substitutions)
    {
        var t1 = Resolve(this, substitutions);
        var t2 = Resolve(other, substitutions);

        if (t1.Equals(t2)) return true;
        if (t1.Kind == PilotTypeKind.Any || t2.Kind == PilotTypeKind.Any) return true;

        if (t1.Kind == PilotTypeKind.TypeVariable)
        {
            if (OccursIn(t1.TypeVarId, t2, substitutions))
                return false;
            if (!t2.SatisfiesConstraint(t1.TypeVarConstraint))
                return false;
            if (t2.Kind == PilotTypeKind.TypeVariable && t1.TypeVarConstraint != PilotTypeClassConstraint.None)
                t2 = t2.WithConstraint(t1.TypeVarConstraint);
            substitutions[t1.TypeVarId] = t2;
            return true;
        }

        if (t2.Kind == PilotTypeKind.TypeVariable)
        {
            if (OccursIn(t2.TypeVarId, t1, substitutions))
                return false;
            if (!t1.SatisfiesConstraint(t2.TypeVarConstraint))
                return false;
            if (t1.Kind == PilotTypeKind.TypeVariable && t2.TypeVarConstraint != PilotTypeClassConstraint.None)
                t1 = t1.WithConstraint(t2.TypeVarConstraint);
            substitutions[t2.TypeVarId] = t1;
            return true;
        }

        // Null with Option<T>
        if (t1.Kind == PilotTypeKind.Null && t2.Kind == PilotTypeKind.Option) return true;
        if (t2.Kind == PilotTypeKind.Null && t1.Kind == PilotTypeKind.Option) return true;

        // Numeric widening unification (int with float)
        if ((t1.IsIntegerPrimitive && t2.IsFloatPrimitive) || (t1.IsFloatPrimitive && t2.IsIntegerPrimitive))
            return true;

        // Structural Row Polymorphism Unification
        if (t1.Kind == PilotTypeKind.RowRecord && t2.Kind == PilotTypeKind.RowRecord)
        {
            foreach (var kvp in t1.RowFields)
            {
                if (t2.RowFields.TryGetValue(kvp.Key, out var otherField))
                {
                    if (!kvp.Value.Unify(otherField, substitutions))
                        return false;
                }
                else if (!t2.IsOpenRow)
                {
                    return false;
                }
            }
            if (!t1.IsOpenRow)
            {
                foreach (var kvp in t2.RowFields)
                {
                    if (!t1.RowFields.ContainsKey(kvp.Key))
                        return false;
                }
            }
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

    public static bool OccursIn(int typeVarId, PilotType target, Dictionary<int, PilotType> substitutions)
    {
        var resolved = Resolve(target, substitutions);
        if (resolved.Kind == PilotTypeKind.TypeVariable)
            return resolved.TypeVarId == typeVarId;
        foreach (var arg in resolved.TypeArguments)
        {
            if (OccursIn(typeVarId, arg, substitutions))
                return true;
        }
        if (resolved.ReturnType != null && OccursIn(typeVarId, resolved.ReturnType, substitutions))
            return true;
        foreach (var field in resolved.RowFields.Values)
        {
            if (OccursIn(typeVarId, field, substitutions))
                return true;
        }
        return false;
    }

    public static PilotType Resolve(PilotType type, Dictionary<int, PilotType> substitutions) =>
        ResolveCore(type, substitutions, new HashSet<int>());

    private static PilotType ResolveCore(PilotType type, Dictionary<int, PilotType> substitutions, HashSet<int> activeVars)
    {
        while (type.Kind == PilotTypeKind.TypeVariable &&
               substitutions.TryGetValue(type.TypeVarId, out var resolved) &&
               !ReferenceEquals(resolved, type) &&
               activeVars.Add(type.TypeVarId))
        {
            if (type.TypeVarConstraint != PilotTypeClassConstraint.None && resolved.Kind == PilotTypeKind.TypeVariable)
            {
                resolved = resolved.WithConstraint(type.TypeVarConstraint);
            }
            type = resolved;
        }

        if (type.Kind == PilotTypeKind.Option && type.TypeArguments.Count == 1)
        {
            var inner = ResolveCore(type.TypeArguments[0], substitutions, activeVars);
            return ReferenceEquals(inner, type.TypeArguments[0]) ? type : OptionOf(inner);
        }

        if (type.IsResult)
        {
            var ok = ResolveCore(type.TypeArguments[0], substitutions, activeVars);
            var err = ResolveCore(type.TypeArguments[1], substitutions, activeVars);
            return ReferenceEquals(ok, type.TypeArguments[0]) && ReferenceEquals(err, type.TypeArguments[1])
                ? type
                : ResultOf(ok, err);
        }

        if (type.Kind == PilotTypeKind.Function && type.ReturnType != null)
        {
            bool changed = false;
            var resolvedParams = new PilotType[type.TypeArguments.Count];
            for (int i = 0; i < type.TypeArguments.Count; i++)
            {
                resolvedParams[i] = ResolveCore(type.TypeArguments[i], substitutions, activeVars);
                if (!ReferenceEquals(resolvedParams[i], type.TypeArguments[i])) changed = true;
            }
            var resolvedRet = ResolveCore(type.ReturnType, substitutions, activeVars);
            if (!ReferenceEquals(resolvedRet, type.ReturnType)) changed = true;
            return changed ? FunctionOf(resolvedParams, resolvedRet) : type;
        }

        if (type.Kind == PilotTypeKind.RowRecord && type.RowFields.Count > 0)
        {
            bool changed = false;
            var newFields = new Dictionary<string, PilotType>(StringComparer.Ordinal);
            foreach (var kvp in type.RowFields)
            {
                var rf = ResolveCore(kvp.Value, substitutions, activeVars);
                if (!ReferenceEquals(rf, kvp.Value)) changed = true;
                newFields[kvp.Key] = rf;
            }
            return changed
                ? RowRecordOf(newFields, type.IsOpenRow, type.RowTailName, type.Name == "RowRecord" ? null : type.Name)
                : type;
        }

        if (type.TypeArguments.Count > 0)
        {
            bool changed = false;
            var newArgs = new PilotType[type.TypeArguments.Count];
            for (int i = 0; i < type.TypeArguments.Count; i++)
            {
                newArgs[i] = ResolveCore(type.TypeArguments[i], substitutions, activeVars);
                if (!ReferenceEquals(newArgs[i], type.TypeArguments[i])) changed = true;
            }
            if (changed)
            {
                if (type.Kind == PilotTypeKind.Union)
                    return UnionOf(newArgs);
                return new PilotType(
                    type.Kind,
                    type.Name,
                    newArgs,
                    type.ReturnType,
                    type.UnitDimension,
                    type.IsResource,
                    type.TypeVarId,
                    type.TypeVarConstraint,
                    type.RowFields,
                    type.IsOpenRow,
                    type.RowTailName,
                    type.ActiveTypestate);
            }
        }

        return type;
    }

    public override string ToString()
    {
        string baseRepr;
        if (Kind == PilotTypeKind.Option)
        {
            baseRepr = $"{TypeArguments[0]}?";
        }
        else if (IsResult)
        {
            baseRepr = TypeArguments[1].Kind == PilotTypeKind.Any
                ? $"{TypeArguments[0]}!"
                : $"{TypeArguments[0]} ! {TypeArguments[1]}";
        }
        else if (Kind == PilotTypeKind.Collection && TypeArguments.Count > 0)
        {
            baseRepr = $"{Name}<{string.Join(", ", TypeArguments)}>";
        }
        else if (Kind == PilotTypeKind.Tuple)
        {
            baseRepr = $"({string.Join(", ", TypeArguments)})";
        }
        else if (Kind == PilotTypeKind.Function && ReturnType != null)
        {
            baseRepr = $"({string.Join(", ", TypeArguments)}) -> {ReturnType}";
        }
        else if (Kind == PilotTypeKind.Unit)
        {
            baseRepr = $"{TypeArguments[0]}<{UnitDimension}>";
        }
        else if (Kind == PilotTypeKind.Union)
        {
            baseRepr = string.Join(" | ", TypeArguments);
        }
        else if (Kind == PilotTypeKind.RowRecord)
        {
            var fieldParts = RowFields.Select(kvp => $"{kvp.Key}: {kvp.Value}").ToList();
            if (IsOpenRow)
                fieldParts.Add($"..{RowTailName ?? "R"}");
            var constraints = new List<string>();
            foreach (var kvp in RowFields)
            {
                if (kvp.Value.Kind == PilotTypeKind.TypeVariable && kvp.Value.TypeVarConstraint != PilotTypeClassConstraint.None)
                {
                    string cStr = $"{kvp.Value.Name}: {FormatConstraint(kvp.Value.TypeVarConstraint)}";
                    if (!constraints.Contains(cStr, StringComparer.Ordinal))
                        constraints.Add(cStr);
                }
            }
            string rowCore = $"{{ {string.Join(", ", fieldParts)} }}";
            baseRepr = constraints.Count > 0
                ? $"{rowCore} where {string.Join(", ", constraints)}"
                : rowCore;
        }
        else
        {
            baseRepr = Name;
        }

        if (!string.IsNullOrEmpty(ActiveTypestate))
            return $"{baseRepr}[{ActiveTypestate}]";

        return baseRepr;
    }

    private static string FormatConstraint(PilotTypeClassConstraint constraint)
    {
        var parts = new List<string>();
        if ((constraint & PilotTypeClassConstraint.Numeric) != 0) parts.Add("Numeric");
        if ((constraint & PilotTypeClassConstraint.Comparable) != 0) parts.Add("Comparable");
        if ((constraint & PilotTypeClassConstraint.Equatable) != 0) parts.Add("Equatable");
        if ((constraint & PilotTypeClassConstraint.Addable) != 0) parts.Add("Addable");
        if ((constraint & PilotTypeClassConstraint.Iterable) != 0) parts.Add("Iterable");
        return string.Join(" + ", parts);
    }

    public bool Equals(PilotType? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        if (Kind != other.Kind || !string.Equals(Name, other.Name, StringComparison.OrdinalIgnoreCase)) return false;
        if (TypeVarId != other.TypeVarId) return false;
        if (!string.Equals(UnitDimension, other.UnitDimension, StringComparison.OrdinalIgnoreCase)) return false;
        if (!string.Equals(ActiveTypestate, other.ActiveTypestate, StringComparison.Ordinal)) return false;
        if (TypeArguments.Count != other.TypeArguments.Count) return false;

        for (int i = 0; i < TypeArguments.Count; i++)
        {
            if (!TypeArguments[i].Equals(other.TypeArguments[i])) return false;
        }
        if (Kind == PilotTypeKind.RowRecord)
        {
            if (IsOpenRow != other.IsOpenRow || RowFields.Count != other.RowFields.Count) return false;
            foreach (var kvp in RowFields)
            {
                if (!other.RowFields.TryGetValue(kvp.Key, out var of) || !kvp.Value.Equals(of))
                    return false;
            }
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
        hash.Add(ActiveTypestate, StringComparer.Ordinal);
        foreach (var arg in TypeArguments) hash.Add(arg);
        foreach (var kvp in RowFields)
        {
            hash.Add(kvp.Key, StringComparer.Ordinal);
            hash.Add(kvp.Value);
        }
        if (ReturnType != null) hash.Add(ReturnType);
        return hash.ToHashCode();
    }
}
