using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using Pilot.Diagnostics;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Execution;

namespace Pilot.Runtime.Values;

/// <summary>
/// Describes a single field of an ADT variant or immutable record product type.
/// </summary>
public sealed class PilotAdtFieldDefinition
{
    public string? Name { get; }
    public string EffectiveName { get; }
    public string TypeName { get; }
    public int Index { get; }
    public PilotParser.TestContext? DefaultValueExpr { get; }

    public PilotAdtFieldDefinition(string? name, string typeName, int index, PilotParser.TestContext? defaultValueExpr = null)
    {
        Name = string.IsNullOrWhiteSpace(name) ? null : name;
        EffectiveName = Name ?? $"_{index}";
        TypeName = string.IsNullOrWhiteSpace(typeName) ? "any" : typeName.Trim();
        Index = index;
        DefaultValueExpr = defaultValueExpr;
    }
}

/// <summary>
/// Describes the byte-level memory layout of a single field within a C-ABI / unboxed ADT variant or record.
/// </summary>
public sealed record PilotFieldMemoryLayout(
    string FieldName,
    string TypeName,
    int OffsetBytes,
    int SizeBytes,
    int AlignmentBytes,
    bool IsBoxedIndirection);

/// <summary>
/// Describes the byte-level layout of a single variant inside an ADT payload union.
/// </summary>
public sealed record PilotVariantMemoryLayout(
    string VariantName,
    ushort Tag,
    int SizeBytes,
    int AlignmentBytes,
    IReadOnlyList<PilotFieldMemoryLayout> Fields);

/// <summary>
/// Compact unboxed memory layout (`#[repr(C, u8/u16)]` equivalent) for sum types and product records.
/// Non-recursive ADTs lower inline to `{ u8/u16 tag; union payload }`; recursive edges are automatically
/// detected via cycle analysis on the ADT type graph and lowered to boxed/arena pointer indirections.
/// </summary>
public sealed class PilotAdtMemoryLayout
{
    /// <summary>Tag width in bytes: 0 for product records, 1 (u8) for &lt;= 256 variants, 2 (u16) for &gt; 256 variants.</summary>
    public int TagBytes { get; }
    public int PayloadOffsetBytes { get; }
    public int PayloadSizeBytes { get; }
    public int AlignmentBytes { get; }
    public int TotalSizeBytes { get; }
    public bool IsRecursive { get; }
    public IReadOnlyList<string> RecursiveEdges { get; }
    public IReadOnlyList<PilotVariantMemoryLayout> Variants { get; }

    public PilotAdtMemoryLayout(
        int tagBytes,
        int payloadOffsetBytes,
        int payloadSizeBytes,
        int alignmentBytes,
        int totalSizeBytes,
        bool isRecursive,
        IReadOnlyList<string> recursiveEdges,
        IReadOnlyList<PilotVariantMemoryLayout> variants)
    {
        TagBytes = tagBytes;
        PayloadOffsetBytes = payloadOffsetBytes;
        PayloadSizeBytes = payloadSizeBytes;
        AlignmentBytes = alignmentBytes;
        TotalSizeBytes = totalSizeBytes;
        IsRecursive = isRecursive;
        RecursiveEdges = recursiveEdges;
        Variants = variants;
    }

    public static PilotAdtMemoryLayout ComputeForAdt(
        PilotAdtDefinition adt,
        IReadOnlyDictionary<string, PilotAdtDefinition>? knownAdts = null,
        IReadOnlyDictionary<string, PilotRecordDefinition>? knownRecords = null)
    {
        int variantCount = adt.Variants.Count;
        int tagBytes = variantCount <= 256 ? 1 : 2;
        int tagAlign = tagBytes;

        var recursiveEdges = new List<string>();
        var variantLayouts = new List<PilotVariantMemoryLayout>(variantCount);

        int maxPayloadSize = 0;
        int maxPayloadAlign = 1;

        foreach (var variant in adt.Variants)
        {
            int offset = 0;
            int varAlign = 1;
            var fieldLayouts = new List<PilotFieldMemoryLayout>(variant.Fields.Count);

            foreach (var field in variant.Fields)
            {
                bool isCycle = FormsDirectValueCycle(adt.Name, field.TypeName, knownAdts, knownRecords, new HashSet<string>(StringComparer.Ordinal) { adt.Name });
                int fSize;
                int fAlign;
                if (isCycle)
                {
                    recursiveEdges.Add($"{variant.Name}.{field.EffectiveName} -> {StripGenericsAndModifiers(field.TypeName)}");
                    fSize = 8;   // 64-bit boxed/arena pointer indirection
                    fAlign = 8;
                }
                else
                {
                    (fSize, fAlign) = EstimateInlineTypeSizeAndAlign(field.TypeName, adt.GenericParameters, knownAdts, knownRecords, new HashSet<string>(StringComparer.Ordinal) { adt.Name });
                }

                offset = AlignUp(offset, fAlign);
                fieldLayouts.Add(new PilotFieldMemoryLayout(field.EffectiveName, field.TypeName, offset, fSize, fAlign, isCycle));
                offset += fSize;
                if (fAlign > varAlign) varAlign = fAlign;
            }

            int varSize = variant.Fields.Count == 0 ? 0 : AlignUp(offset, varAlign);
            variantLayouts.Add(new PilotVariantMemoryLayout(variant.Name, variant.Tag, varSize, varAlign, fieldLayouts));

            if (varSize > maxPayloadSize) maxPayloadSize = varSize;
            if (varAlign > maxPayloadAlign) maxPayloadAlign = varAlign;
        }

        int totalAlign = Math.Max(tagAlign, maxPayloadAlign);
        int payloadOffset = maxPayloadSize == 0 ? tagBytes : AlignUp(tagBytes, maxPayloadAlign);
        int totalSize = Math.Max(tagBytes, AlignUp(payloadOffset + maxPayloadSize, totalAlign));

        return new PilotAdtMemoryLayout(
            tagBytes,
            payloadOffset,
            maxPayloadSize,
            totalAlign,
            totalSize,
            isRecursive: recursiveEdges.Count > 0,
            recursiveEdges,
            variantLayouts);
    }

    public static PilotAdtMemoryLayout ComputeForRecord(
        PilotRecordDefinition record,
        IReadOnlyDictionary<string, PilotAdtDefinition>? knownAdts = null,
        IReadOnlyDictionary<string, PilotRecordDefinition>? knownRecords = null)
    {
        var recursiveEdges = new List<string>();
        var fieldLayouts = new List<PilotFieldMemoryLayout>(record.Fields.Count);
        int offset = 0;
        int maxAlign = 1;

        foreach (var field in record.Fields)
        {
            bool isCycle = FormsDirectValueCycle(record.Name, field.TypeName, knownAdts, knownRecords, new HashSet<string>(StringComparer.Ordinal) { record.Name });
            int fSize;
            int fAlign;
            if (isCycle)
            {
                recursiveEdges.Add($"{record.Name}.{field.EffectiveName} -> {StripGenericsAndModifiers(field.TypeName)}");
                fSize = 8;
                fAlign = 8;
            }
            else
            {
                (fSize, fAlign) = EstimateInlineTypeSizeAndAlign(field.TypeName, record.GenericParameters, knownAdts, knownRecords, new HashSet<string>(StringComparer.Ordinal) { record.Name });
            }

            offset = AlignUp(offset, fAlign);
            fieldLayouts.Add(new PilotFieldMemoryLayout(field.EffectiveName, field.TypeName, offset, fSize, fAlign, isCycle));
            offset += fSize;
            if (fAlign > maxAlign) maxAlign = fAlign;
        }

        int totalSize = record.Fields.Count == 0 ? 1 : AlignUp(offset, maxAlign);
        var singleVariant = new PilotVariantMemoryLayout(record.Name, 0, totalSize, maxAlign, fieldLayouts);

        return new PilotAdtMemoryLayout(
            tagBytes: 0,
            payloadOffsetBytes: 0,
            payloadSizeBytes: totalSize,
            alignmentBytes: maxAlign,
            totalSizeBytes: totalSize,
            isRecursive: recursiveEdges.Count > 0,
            recursiveEdges,
            new[] { singleVariant });
    }

    private static bool FormsDirectValueCycle(
        string rootTypeName,
        string fieldTypeSpec,
        IReadOnlyDictionary<string, PilotAdtDefinition>? knownAdts,
        IReadOnlyDictionary<string, PilotRecordDefinition>? knownRecords,
        HashSet<string> visited)
    {
        if (string.IsNullOrWhiteSpace(fieldTypeSpec)) return false;
        string trimmed = fieldTypeSpec.Trim();

        // Slice / dynamic array / pointer / reference types (e.g., Point[], List[Expr[T]], ref Node)
        // hold heap/arena fat-pointers { ptr, len } and do NOT cause infinite inline value size!
        if (trimmed.EndsWith("[]", StringComparison.Ordinal) ||
            trimmed.StartsWith("ref ", StringComparison.Ordinal) ||
            trimmed.StartsWith("List[", StringComparison.OrdinalIgnoreCase) ||
            trimmed.StartsWith("Array[", StringComparison.OrdinalIgnoreCase) ||
            trimmed.StartsWith("Set[", StringComparison.OrdinalIgnoreCase) ||
            trimmed.StartsWith("Dict[", StringComparison.OrdinalIgnoreCase) ||
            trimmed.StartsWith("Map[", StringComparison.OrdinalIgnoreCase) ||
            trimmed.StartsWith("Box[", StringComparison.OrdinalIgnoreCase) ||
            (trimmed.StartsWith("[", StringComparison.Ordinal) && !trimmed.Contains(';')))
        {
            return false;
        }

        // Tuple inline value containment: (A, B)
        if (trimmed.StartsWith("(", StringComparison.Ordinal) && trimmed.EndsWith(")", StringComparison.Ordinal))
        {
            foreach (var part in SplitTopLevel(trimmed[1..^1], ','))
            {
                if (FormsDirectValueCycle(rootTypeName, part, knownAdts, knownRecords, visited))
                    return true;
            }
            return false;
        }

        // Nullable value containment: Expr[T]? still embeds Expr[T] unless boxed
        if (trimmed.EndsWith("?", StringComparison.Ordinal))
        {
            trimmed = trimmed[..^1].Trim();
        }

        string baseName = StripGenericsAndModifiers(trimmed);
        if (string.Equals(baseName, rootTypeName, StringComparison.Ordinal))
            return true;

        if (!visited.Add(baseName))
            return false;

        if (knownAdts != null && knownAdts.TryGetValue(baseName, out var targetAdt))
        {
            foreach (var v in targetAdt.Variants)
            {
                foreach (var f in v.Fields)
                {
                    if (FormsDirectValueCycle(rootTypeName, f.TypeName, knownAdts, knownRecords, new HashSet<string>(visited, StringComparer.Ordinal)))
                        return true;
                }
            }
        }

        if (knownRecords != null && knownRecords.TryGetValue(baseName, out var targetRec))
        {
            foreach (var f in targetRec.Fields)
            {
                if (FormsDirectValueCycle(rootTypeName, f.TypeName, knownAdts, knownRecords, new HashSet<string>(visited, StringComparer.Ordinal)))
                    return true;
            }
        }

        return false;
    }

    private static (int Size, int Align) EstimateInlineTypeSizeAndAlign(
        string typeSpec,
        IReadOnlyList<string> genericParams,
        IReadOnlyDictionary<string, PilotAdtDefinition>? knownAdts,
        IReadOnlyDictionary<string, PilotRecordDefinition>? knownRecords,
        HashSet<string> computing)
    {
        if (string.IsNullOrWhiteSpace(typeSpec)) return (8, 8);
        string t = typeSpec.Trim();

        if (t.EndsWith("[]", StringComparison.Ordinal) ||
            t.StartsWith("List[", StringComparison.OrdinalIgnoreCase) ||
            t.StartsWith("Array[", StringComparison.OrdinalIgnoreCase) ||
            t == "str" || t == "string" || t == "bytes")
        {
            // C-ABI slice / string fat-pointer { ptr, usize len }
            return (16, 8);
        }

        if (t.StartsWith("(", StringComparison.Ordinal) && t.EndsWith(")", StringComparison.Ordinal))
        {
            int off = 0;
            int al = 1;
            foreach (var elem in SplitTopLevel(t[1..^1], ','))
            {
                var (es, ea) = EstimateInlineTypeSizeAndAlign(elem, genericParams, knownAdts, knownRecords, computing);
                off = AlignUp(off, ea);
                off += es;
                if (ea > al) al = ea;
            }
            return (Math.Max(1, AlignUp(off, al)), al);
        }

        // Generic parameter defaults to 8-byte word slot in unspecialized template layout
        if (genericParams.Contains(t))
            return (8, 8);

        string baseName = StripGenericsAndModifiers(t);
        switch (baseName.ToLowerInvariant())
        {
            case "bool" or "i8" or "u8" or "byte":
                return (1, 1);
            case "i16" or "u16" or "f16":
                return (2, 2);
            case "i32" or "u32" or "f32" or "char":
                return (4, 4);
            case "int" or "uint" or "i64" or "u64" or "float" or "f64" or "double" or "duration" or "frequency" or "memorysize":
                return (8, 8);
            case "decimal" or "complex":
                return (16, 8);
        }

        if (computing.Add(baseName))
        {
            if (knownRecords != null && knownRecords.TryGetValue(baseName, out var rec))
            {
                var recLayout = ComputeForRecord(rec, knownAdts, knownRecords);
                return (recLayout.TotalSizeBytes, recLayout.AlignmentBytes);
            }
            if (knownAdts != null && knownAdts.TryGetValue(baseName, out var nestedAdt))
            {
                var adtLayout = ComputeForAdt(nestedAdt, knownAdts, knownRecords);
                return (adtLayout.TotalSizeBytes, adtLayout.AlignmentBytes);
            }
        }

        return (8, 8);
    }

    public static string StripGenericsAndModifiers(string typeSpec)
    {
        string s = typeSpec.Trim();
        if (s.StartsWith("ref ", StringComparison.Ordinal)) s = s[4..].Trim();
        while (s.EndsWith("?", StringComparison.Ordinal) || s.EndsWith("[]", StringComparison.Ordinal))
        {
            s = s.EndsWith("?", StringComparison.Ordinal) ? s[..^1].Trim() : s[..^2].Trim();
        }
        int bracket = s.IndexOf('[');
        if (bracket > 0) s = s[..bracket].Trim();
        int angle = s.IndexOf('<');
        if (angle > 0) s = s[..angle].Trim();
        return s;
    }

    private static List<string> SplitTopLevel(string input, char separator)
    {
        var parts = new List<string>();
        int depth = 0;
        int start = 0;
        for (int i = 0; i < input.Length; i++)
        {
            char c = input[i];
            if (c is '[' or '(' or '<' or '{') depth++;
            else if (c is ']' or ')' or '>' or '}') depth--;
            else if (c == separator && depth == 0)
            {
                parts.Add(input[start..i].Trim());
                start = i + 1;
            }
        }
        if (start <= input.Length)
        {
            string tail = input[start..].Trim();
            if (tail.Length > 0) parts.Add(tail);
        }
        return parts;
    }

    public static int AlignUp(int value, int align)
    {
        if (align <= 1) return value;
        int rem = value % align;
        return rem == 0 ? value : value + (align - rem);
    }
}

/// <summary>
/// Singleton marker per <see cref="PilotAdtVariantDefinition"/> enabling collision-free 64-bit IEEE-754
/// unboxed float storage inside <see cref="PilotValue"/> with zero per-instance heap allocations.
/// </summary>
public sealed class PilotAdtFloatSlot
{
    public PilotAdtVariantDefinition Variant { get; }
    public PilotAdtFloatSlot(PilotAdtVariantDefinition variant) => Variant = variant;
}

/// <summary>
/// Runtime definition of a single discriminated sum-type variant (e.g., `Circle(radius: f64)` or `Origin`).
/// </summary>
public sealed class PilotAdtVariantDefinition
{
    public string Name { get; }
    public ushort Tag { get; }
    public PilotAdtDefinition ParentAdt { get; }
    public IReadOnlyList<PilotAdtFieldDefinition> Fields { get; }
    private readonly Dictionary<string, int> _fieldIndexByName;
    public PilotValue UnitSingleton { get; }
    public PilotAdtFloatSlot FloatSlot { get; }
    public PilotFunction? ConstructorFunction { get; private set; }

    public int Arity => Fields.Count;
    public bool IsUnit => Fields.Count == 0;

    public PilotAdtVariantDefinition(
        string name,
        ushort tag,
        PilotAdtDefinition parentAdt,
        IReadOnlyList<PilotAdtFieldDefinition> fields)
    {
        Name = name;
        Tag = tag;
        ParentAdt = parentAdt;
        Fields = fields;
        _fieldIndexByName = new Dictionary<string, int>(StringComparer.Ordinal);
        for (int i = 0; i < fields.Count; i++)
        {
            if (fields[i].Name != null)
            {
                _fieldIndexByName[fields[i].Name!] = i;
            }
            _fieldIndexByName[$"_{i}"] = i;
            _fieldIndexByName[$"item{i}"] = i;
        }

        FloatSlot = new PilotAdtFloatSlot(this);

        if (fields.Count == 0)
        {
            UnitSingleton = PilotValue.FromAdtUnit(this);
        }
        else
        {
            var parameters = fields
                .Select(f => new PilotParameter(f.EffectiveName, f.TypeName))
                .ToArray();
            ConstructorFunction = new PilotFunction(
                name,
                parameters,
                (ctx, args) => Instantiate(ctx, args, null));
        }
    }

    public int GetFieldIndex(string fieldName)
    {
        if (_fieldIndexByName.TryGetValue(fieldName, out int idx))
            return idx;
        if (Fields.Count == 1 && fieldName == "value")
            return 0;
        return -1;
    }

    public PilotValue Instantiate(
        PilotExecutionContext? context,
        IReadOnlyList<PilotValue> positionalArgs,
        IReadOnlyDictionary<string, PilotValue>? namedArgs = null)
    {
        if (Fields.Count == 0)
        {
            if (positionalArgs.Count > 0 || (namedArgs != null && namedArgs.Count > 0))
            {
                throw new PilotRuntimeException($"Unit variant '{ParentAdt.Name}.{Name}' takes 0 arguments.");
            }
            return UnitSingleton;
        }

        // Fast zero-allocation path for 1-field unboxed primitive payload (e.g. Circle(5.0), Const(42))
        if (Fields.Count == 1 && (namedArgs == null || namedArgs.Count == 0) && positionalArgs.Count == 1)
        {
            var singleVal = CoerceFieldValue(Fields[0].TypeName, positionalArgs[0]);
            if (PilotValue.TryCreateUnboxedAdtSingle(this, singleVal, out var unboxed))
            {
                return unboxed;
            }
            return PilotValue.FromAdtBoxed(new PilotAdtInstance(this, new[] { singleVal }));
        }

        var resolved = new PilotValue[Fields.Count];
        var filled = new bool[Fields.Count];

        int posCount = Math.Min(positionalArgs.Count, Fields.Count);
        for (int i = 0; i < posCount; i++)
        {
            resolved[i] = CoerceFieldValue(Fields[i].TypeName, positionalArgs[i]);
            filled[i] = true;
        }

        if (namedArgs != null && namedArgs.Count > 0)
        {
            foreach (var kvp in namedArgs)
            {
                int fIdx = GetFieldIndex(kvp.Key);
                if (fIdx < 0)
                {
                    throw new PilotRuntimeException($"Variant '{ParentAdt.Name}.{Name}' has no field named '{kvp.Key}'.");
                }
                resolved[fIdx] = CoerceFieldValue(Fields[fIdx].TypeName, kvp.Value);
                filled[fIdx] = true;
            }
        }

        for (int i = 0; i < Fields.Count; i++)
        {
            if (!filled[i])
            {
                if (Fields[i].DefaultValueExpr != null && context != null)
                {
                    resolved[i] = CoerceFieldValue(Fields[i].TypeName, context.GetOrCreateVisitor().Visit(Fields[i].DefaultValueExpr));
                }
                else
                {
                    throw new PilotRuntimeException($"Missing required field '{Fields[i].EffectiveName}' for variant '{ParentAdt.Name}.{Name}'.");
                }
            }
        }

        if (Fields.Count == 1 && PilotValue.TryCreateUnboxedAdtSingle(this, resolved[0], out var unboxedNamed))
        {
            return unboxedNamed;
        }

        return PilotValue.FromAdtBoxed(new PilotAdtInstance(this, resolved));
    }

    internal static PilotValue CoerceFieldValue(string declaredType, PilotValue value)
    {
        if (value.Kind == PilotValueKind.Int && declaredType is "f64" or "float" or "double" or "f32")
        {
            return PilotValue.FromFloat(value.AsFloat());
        }
        return value;
    }

    public override string ToString() => $"{ParentAdt.Name}.{Name}";
}

/// <summary>
/// Runtime definition of a discriminated sum type (`type Name = | ...` or `enum Name { ... }`).
/// </summary>
public sealed class PilotAdtDefinition
{
    public string Name { get; }
    public IReadOnlyList<string> GenericParameters { get; }
    private readonly List<PilotAdtVariantDefinition> _variants = new();
    private readonly Dictionary<string, PilotAdtVariantDefinition> _variantByName = new(StringComparer.Ordinal);
    public PilotAdtMemoryLayout Layout { get; set; }

    public IReadOnlyList<PilotAdtVariantDefinition> Variants => _variants;

    public PilotAdtDefinition(string name, IReadOnlyList<string>? genericParameters = null)
    {
        Name = name;
        GenericParameters = genericParameters ?? Array.Empty<string>();
        Layout = new PilotAdtMemoryLayout(1, 1, 0, 1, 1, false, Array.Empty<string>(), Array.Empty<PilotVariantMemoryLayout>());
    }

    public PilotAdtVariantDefinition AddVariant(string variantName, IReadOnlyList<PilotAdtFieldDefinition> fields)
    {
        ushort tag = checked((ushort)_variants.Count);
        var variant = new PilotAdtVariantDefinition(variantName, tag, this, fields);
        _variants.Add(variant);
        _variantByName[variantName] = variant;
        return variant;
    }

    public bool TryGetVariant(string name, out PilotAdtVariantDefinition? variant) =>
        _variantByName.TryGetValue(name, out variant);

    public bool TryGetVariantByTag(int tag, out PilotAdtVariantDefinition? variant)
    {
        if (tag >= 0 && tag < _variants.Count)
        {
            variant = _variants[tag];
            return true;
        }
        variant = null;
        return false;
    }

    public override string ToString() => $"<type {Name}>";
}

/// <summary>
/// Compact multi-field ADT payload holder (flat array, zero dictionary overhead).
/// Single-field primitive and unit variants bypass this completely and live inline in <see cref="PilotValue"/>.
/// </summary>
public sealed class PilotAdtInstance : IEquatable<PilotAdtInstance>
{
    public PilotAdtVariantDefinition Variant { get; }
    public PilotValue[] Fields { get; }

    public PilotAdtInstance(PilotAdtVariantDefinition variant, PilotValue[] fields)
    {
        Variant = variant;
        Fields = fields;
    }

    public bool Equals(PilotAdtInstance? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        if (!ReferenceEquals(Variant, other.Variant) &&
            !(Variant.Tag == other.Variant.Tag && Variant.Name == other.Variant.Name && Variant.ParentAdt.Name == other.Variant.ParentAdt.Name))
        {
            return false;
        }
        if (Fields.Length != other.Fields.Length) return false;
        for (int i = 0; i < Fields.Length; i++)
        {
            if (!Fields[i].Equals(other.Fields[i])) return false;
        }
        return true;
    }

    public override bool Equals(object? obj) => obj is PilotAdtInstance other && Equals(other);

    public override int GetHashCode()
    {
        var hc = new HashCode();
        hc.Add(Variant.ParentAdt.Name, StringComparer.Ordinal);
        hc.Add(Variant.Tag);
        for (int i = 0; i < Fields.Length; i++)
        {
            hc.Add(Fields[i]);
        }
        return hc.ToHashCode();
    }

    public override string ToString()
    {
        if (Fields.Length == 0) return Variant.Name;
        var parts = new string[Fields.Length];
        for (int i = 0; i < Fields.Length; i++)
        {
            string? fName = i < Variant.Fields.Count ? Variant.Fields[i].Name : null;
            parts[i] = fName != null ? $"{fName}: {Fields[i]}" : Fields[i].ToString();
        }
        return $"{Variant.Name}({string.Join(", ", parts)})";
    }
}

/// <summary>
/// Runtime definition of an immutable value-semantic product type (`record Point(x: f64, y: f64)`).
/// Supports zero-heap-allocation unboxed storage inside <see cref="PilotValue"/> for 0-field,
/// 1-field primitive, and 2-field `(f64, f64)` records!
/// </summary>
public sealed class PilotRecordDefinition
{
    public string Name { get; }
    public IReadOnlyList<string> GenericParameters { get; }
    public IReadOnlyList<PilotAdtFieldDefinition> Fields { get; }
    public Dictionary<string, PilotFunction> Methods { get; } = new(StringComparer.Ordinal);
    public PilotAdtMemoryLayout Layout { get; set; }
    public bool CanUnboxPairF64 { get; }
    public PilotFunction ConstructorFunction { get; }

    private readonly Dictionary<string, int> _fieldIndexByName = new(StringComparer.Ordinal);

    public PilotRecordDefinition(
        string name,
        IReadOnlyList<PilotAdtFieldDefinition> fields,
        IReadOnlyList<string>? genericParameters = null)
    {
        Name = name;
        Fields = fields;
        GenericParameters = genericParameters ?? Array.Empty<string>();
        for (int i = 0; i < fields.Count; i++)
        {
            if (fields[i].Name != null)
            {
                _fieldIndexByName[fields[i].Name!] = i;
            }
            _fieldIndexByName[$"_{i}"] = i;
            _fieldIndexByName[$"item{i}"] = i;
        }

        CanUnboxPairF64 = fields.Count == 2 &&
                          IsFloatTypeName(fields[0].TypeName) &&
                          IsFloatTypeName(fields[1].TypeName);

        Layout = PilotAdtMemoryLayout.ComputeForRecord(this);

        var parameters = fields
            .Select(f => new PilotParameter(f.EffectiveName, f.TypeName))
            .ToArray();
        ConstructorFunction = new PilotFunction(
            name,
            parameters,
            (ctx, args) => Instantiate(ctx, args, null));
    }

    private static bool IsFloatTypeName(string typeName) =>
        typeName is "f64" or "float" or "double" or "f32";

    public int GetFieldIndex(string name) =>
        _fieldIndexByName.TryGetValue(name, out int idx) ? idx : -1;

    public PilotValue Instantiate(
        PilotExecutionContext? context,
        IReadOnlyList<PilotValue> positionalArgs,
        IReadOnlyDictionary<string, PilotValue>? namedArgs = null)
    {
        if (Fields.Count == 0)
        {
            return PilotValue.FromRecordUnboxedEmpty(this);
        }

        // Fast zero-allocation path for 2-field f64 records such as `record Point(x: f64, y: f64)`!
        if (CanUnboxPairF64 && (namedArgs == null || namedArgs.Count == 0) && positionalArgs.Count == 2 &&
            positionalArgs[0].IsNumber && positionalArgs[1].IsNumber)
        {
            return PilotValue.FromRecordUnboxedPairF64(this, positionalArgs[0].AsFloat(), positionalArgs[1].AsFloat());
        }

        var resolved = new PilotValue[Fields.Count];
        var filled = new bool[Fields.Count];

        int posCount = Math.Min(positionalArgs.Count, Fields.Count);
        for (int i = 0; i < posCount; i++)
        {
            resolved[i] = PilotAdtVariantDefinition.CoerceFieldValue(Fields[i].TypeName, positionalArgs[i]);
            filled[i] = true;
        }

        if (namedArgs != null && namedArgs.Count > 0)
        {
            foreach (var kvp in namedArgs)
            {
                int idx = GetFieldIndex(kvp.Key);
                if (idx < 0)
                {
                    throw new PilotRuntimeException($"Record '{Name}' has no field named '{kvp.Key}'.");
                }
                resolved[idx] = PilotAdtVariantDefinition.CoerceFieldValue(Fields[idx].TypeName, kvp.Value);
                filled[idx] = true;
            }
        }

        for (int i = 0; i < Fields.Count; i++)
        {
            if (!filled[i])
            {
                if (Fields[i].DefaultValueExpr != null && context != null)
                {
                    resolved[i] = PilotAdtVariantDefinition.CoerceFieldValue(
                        Fields[i].TypeName,
                        context.GetOrCreateVisitor().Visit(Fields[i].DefaultValueExpr));
                }
                else
                {
                    throw new PilotRuntimeException($"Missing required field '{Fields[i].EffectiveName}' for record '{Name}'.");
                }
            }
        }

        if (CanUnboxPairF64 && resolved[0].IsNumber && resolved[1].IsNumber)
        {
            return PilotValue.FromRecordUnboxedPairF64(this, resolved[0].AsFloat(), resolved[1].AsFloat());
        }

        return PilotValue.FromRecordBoxed(new PilotRecordInstance(this, resolved));
    }

    public override string ToString() => $"<record {Name}>";
}

/// <summary>
/// Compact value-semantic record payload for general N-field records.
/// </summary>
public sealed class PilotRecordInstance : IEquatable<PilotRecordInstance>
{
    public PilotRecordDefinition Definition { get; }
    public PilotValue[] Fields { get; }

    public PilotRecordInstance(PilotRecordDefinition definition, PilotValue[] fields)
    {
        Definition = definition;
        Fields = fields;
    }

    public bool Equals(PilotRecordInstance? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        if (Definition.Name != other.Definition.Name || Fields.Length != other.Fields.Length) return false;
        for (int i = 0; i < Fields.Length; i++)
        {
            if (!Fields[i].Equals(other.Fields[i])) return false;
        }
        return true;
    }

    public override bool Equals(object? obj) => obj is PilotRecordInstance other && Equals(other);

    public override int GetHashCode()
    {
        var hc = new HashCode();
        hc.Add(Definition.Name, StringComparer.Ordinal);
        for (int i = 0; i < Fields.Length; i++)
        {
            hc.Add(Fields[i]);
        }
        return hc.ToHashCode();
    }

    public override string ToString()
    {
        var parts = new string[Fields.Length];
        for (int i = 0; i < Fields.Length; i++)
        {
            string? fName = i < Definition.Fields.Count ? Definition.Fields[i].Name : null;
            parts[i] = fName != null ? $"{fName}: {Fields[i]}" : Fields[i].ToString();
        }
        return $"{Definition.Name}({string.Join(", ", parts)})";
    }
}

/// <summary>
/// Ad-hoc Clojure/Elixir-inspired keyword-tagged variant value, e.g., `:ok([1, 2, 3])` or `:err("timeout")`.
/// </summary>
public sealed class PilotKeywordVariant : IEquatable<PilotKeywordVariant>
{
    public PilotKeyword Keyword { get; }
    public PilotValue[] Fields { get; }

    public PilotKeywordVariant(PilotKeyword keyword, PilotValue[] fields)
    {
        Keyword = keyword;
        Fields = fields;
    }

    public bool Equals(PilotKeywordVariant? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        if (!Keyword.Equals(other.Keyword) || Fields.Length != other.Fields.Length) return false;
        for (int i = 0; i < Fields.Length; i++)
        {
            if (!Fields[i].Equals(other.Fields[i])) return false;
        }
        return true;
    }

    public override bool Equals(object? obj) => obj is PilotKeywordVariant other && Equals(other);

    public override int GetHashCode()
    {
        var hc = new HashCode();
        hc.Add(Keyword);
        for (int i = 0; i < Fields.Length; i++) hc.Add(Fields[i]);
        return hc.ToHashCode();
    }

    public override string ToString() =>
        Fields.Length == 0 ? Keyword.ToString() : $"{Keyword}({string.Join(", ", (IEnumerable<PilotValue>)Fields)})";
}
