using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.Numerics;
using System.Reflection;
using System.Runtime.CompilerServices;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Interop;

/// <summary>
/// Universal bidirectional type bridge and marshaler between foreign/host languages and Pilot values.
/// Converts foreign primitives, collections, dictionaries, and POCOs into equivalent <see cref="PilotValue"/> instances.
/// </summary>
public sealed class PolyglotTypeBridge : IPolyglotTypeConverter
{
    private static readonly Lazy<PolyglotTypeBridge> _default = new(() => new PolyglotTypeBridge());
    public static PolyglotTypeBridge Default => _default.Value;

    private readonly Dictionary<Type, Func<object, PilotValue>> _customToPilotMappers = new();
    private readonly Dictionary<Type, Func<PilotValue, object?>> _customToForeignMappers = new();

    public void RegisterCustomMapper<T>(Func<T, PilotValue> toPilot, Func<PilotValue, T> toForeign)
    {
        _customToPilotMappers[typeof(T)] = obj => toPilot((T)obj);
        _customToForeignMappers[typeof(T)] = val => toForeign(val);
    }

    public PilotValue ToPilotValue(object? foreignObject, string? sourceLanguage = null)
    {
        if (foreignObject is null || foreignObject is DBNull ||
            (foreignObject is IntPtr ptr && ptr == IntPtr.Zero) ||
            (foreignObject is UIntPtr uptr && uptr == UIntPtr.Zero))
        {
            return PilotValue.Null;
        }

        if (foreignObject is PilotValue alreadyPilot)
        {
            return alreadyPilot;
        }

        var type = foreignObject.GetType();

        // Check user-registered custom mappers
        if (_customToPilotMappers.TryGetValue(type, out var customMapper))
        {
            return customMapper(foreignObject);
        }

        if (TryMapPrimitiveToPilotValue(foreignObject, out var primitiveValue))
        {
            return primitiveValue;
        }

        if (TryMapCollectionToPilotValue(foreignObject, type, sourceLanguage, out var collectionValue))
        {
            return collectionValue;
        }

        // Complex Objects, POCOs, Anonymous Types
        return ConvertPocoToPilotObject(foreignObject, type);
    }

    private bool TryMapPrimitiveToPilotValue(object foreignObject, out PilotValue result)
    {
        switch (foreignObject)
        {
            case bool b:
                result = PilotValue.FromBool(b);
                return true;
            case sbyte sb:
                result = PilotValue.FromInt(sb);
                return true;
            case byte b:
                result = PilotValue.FromUInt(b);
                return true;
            case short s:
                result = PilotValue.FromInt(s);
                return true;
            case ushort us:
                result = PilotValue.FromUInt(us);
                return true;
            case int i:
                result = PilotValue.FromInt(i);
                return true;
            case uint u:
                result = PilotValue.FromUInt(u);
                return true;
            case long l:
                result = PilotValue.FromInt(l);
                return true;
            case ulong ul:
                result = PilotValue.FromUInt(ul);
                return true;
            case BigInteger bi:
                result = PilotValue.FromInt((long)bi);
                return true;
            case float f:
                result = PilotValue.FromFloat(f);
                return true;
            case double d:
                result = PilotValue.FromFloat(d);
                return true;
            case decimal dec:
                result = PilotValue.FromDecimal(dec);
                return true;
            case char c:
                result = PilotValue.FromChar(c);
                return true;
            case string str:
                result = PilotValue.FromString(str);
                return true;
            case Guid guid:
                result = PilotValue.FromString(guid.ToString());
                return true;
            case DateTime dt:
                result = PilotValue.FromString(dt.ToString("o", CultureInfo.InvariantCulture));
                return true;
            case DateTimeOffset dto:
                result = PilotValue.FromString(dto.ToString("o", CultureInfo.InvariantCulture));
                return true;
            case TimeSpan ts:
                result = PilotValue.FromFloat(ts.TotalMilliseconds);
                return true;
            case PilotRange range:
                result = PilotValue.FromRange(range);
                return true;
            case PilotMatrix matrix:
                result = PilotValue.FromMatrix(matrix);
                return true;
            case PilotFunction func:
                result = PilotValue.FromFunction(func);
                return true;
            case PilotObject obj:
                result = PilotValue.FromObject(obj);
                return true;
            default:
                result = default;
                return false;
        }
    }

    private bool TryMapCollectionToPilotValue(object foreignObject, Type type, string? sourceLanguage, out PilotValue result)
    {
        // Dictionaries and Map representations
        if (foreignObject is IDictionary dict)
        {
            var pilotMap = new Dictionary<PilotValue, PilotValue>(dict.Count);
            foreach (DictionaryEntry entry in dict)
            {
                var k = ToPilotValue(entry.Key, sourceLanguage);
                var v = ToPilotValue(entry.Value, sourceLanguage);
                pilotMap[k] = v;
            }
            result = PilotValue.FromMap(pilotMap);
            return true;
        }

        // Tuples
        if (foreignObject is ITuple tuple)
        {
            var tupleItems = new List<PilotValue>(tuple.Length);
            for (int i = 0; i < tuple.Length; i++)
            {
                tupleItems.Add(ToPilotValue(tuple[i], sourceLanguage));
            }
            result = PilotValue.FromTuple(tupleItems);
            return true;
        }

        // Sets
        if (type.IsGenericType && type.GetGenericTypeDefinition() == typeof(HashSet<>))
        {
            var pilotSet = new HashSet<PilotValue>();
            foreach (var item in (IEnumerable)foreignObject)
            {
                pilotSet.Add(ToPilotValue(item, sourceLanguage));
            }
            result = PilotValue.FromSet(pilotSet);
            return true;
        }

        // Lists, Arrays, and Sequences
        if (foreignObject is IEnumerable enumerable && !(foreignObject is string))
        {
            var pilotList = new List<PilotValue>();
            foreach (var item in enumerable)
            {
                pilotList.Add(ToPilotValue(item, sourceLanguage));
            }
            result = PilotValue.FromList(pilotList);
            return true;
        }

        result = default;
        return false;
    }

    public object? ToForeignValue(PilotValue value, Type? targetType = null)
    {
        if (targetType != null && _customToForeignMappers.TryGetValue(targetType, out var customMapper))
        {
            return customMapper(value);
        }

        if (targetType == typeof(PilotValue))
        {
            return value;
        }

        if (targetType == null || targetType == typeof(object))
        {
            return MapKindToForeignValue(value);
        }

        if (TryCoerceToForeignType(value, targetType, out var coercedValue))
        {
            return coercedValue;
        }

        return value.ToString();
    }

    private static object? MapKindToForeignValue(PilotValue value)
    {
        return value.Kind switch
        {
            PilotValueKind.Null => null,
            PilotValueKind.Undefined => null,
            PilotValueKind.Bool => value.IsTrue,
            PilotValueKind.Int => value.AsInt(),
            PilotValueKind.UInt => (ulong)value.AsInt(),
            PilotValueKind.Float => value.AsFloat(),
            PilotValueKind.Decimal => (decimal)value.AsFloat(),
            PilotValueKind.Char => (char)value.AsInt(),
            PilotValueKind.String => value.AsString(),
            PilotValueKind.List => ConvertListToHost(value.AsList()),
            PilotValueKind.Tuple => ConvertListToHost(value.AsTuple()),
            PilotValueKind.Map => ConvertMapToHost(value.AsMap()),
            PilotValueKind.Matrix => value.AsMatrix(),
            PilotValueKind.Range => value.AsRange(),
            PilotValueKind.Object => value.AsObject().Fields,
            PilotValueKind.Foreign => value.AsForeign(),
            _ => value.ToString()
        };
    }

    private static bool TryCoerceToForeignType(PilotValue value, Type targetType, out object? result)
    {
        // Explicit target type coercions
        if (targetType == typeof(int)) { result = (int)value.AsInt(); return true; }
        if (targetType == typeof(long)) { result = value.AsInt(); return true; }
        if (targetType == typeof(uint)) { result = (uint)value.AsInt(); return true; }
        if (targetType == typeof(ulong)) { result = (ulong)value.AsInt(); return true; }
        if (targetType == typeof(double)) { result = value.AsFloat(); return true; }
        if (targetType == typeof(float)) { result = (float)value.AsFloat(); return true; }
        if (targetType == typeof(decimal)) { result = (decimal)value.AsFloat(); return true; }
        if (targetType == typeof(bool)) { result = value.IsTrue; return true; }
        if (targetType == typeof(string)) { result = value.AsString(); return true; }
        if (targetType == typeof(char)) { result = (char)value.AsInt(); return true; }

        if (targetType == typeof(PilotMatrix) && value.Kind == PilotValueKind.Matrix)
        {
            result = value.AsMatrix();
            return true;
        }

        if (targetType == typeof(PilotRange) && value.Kind == PilotValueKind.Range)
        {
            result = value.AsRange();
            return true;
        }

        result = null;
        return false;
    }

    private PilotValue ConvertPocoToPilotObject(object foreignObject, Type type)
    {
        string typeName = type.Name;
        // Clean up anonymous types (e.g. <>f__AnonymousType0)
        if (typeName.StartsWith("<>f__AnonymousType", StringComparison.Ordinal))
        {
            typeName = "Anonymous";
        }

        var classDef = new PilotClassDefinition(typeName);
        var pilotObj = new PilotObject(classDef);

        // Read public properties
        foreach (var prop in type.GetProperties(BindingFlags.Public | BindingFlags.Instance))
        {
            if (prop.CanRead)
            {
                try
                {
                    var val = prop.GetValue(foreignObject);
                    pilotObj.SetMember(prop.Name, ToPilotValue(val));
                }
                catch
                {
                    // Ignore inaccessible properties
                }
            }
        }

        // Read public fields
        foreach (var field in type.GetFields(BindingFlags.Public | BindingFlags.Instance))
        {
            try
            {
                var val = field.GetValue(foreignObject);
                pilotObj.SetMember(field.Name, ToPilotValue(val));
            }
            catch
            {
                // Ignore inaccessible fields
            }
        }

        return PilotValue.FromObject(pilotObj);
    }

    private static List<object?> ConvertListToHost(IReadOnlyList<PilotValue> list)
    {
        var result = new List<object?>(list.Count);
        foreach (var item in list)
        {
            result.Add(Default.ToForeignValue(item));
        }
        return result;
    }

    private static Dictionary<object, object?> ConvertMapToHost(Dictionary<PilotValue, PilotValue> map)
    {
        var result = new Dictionary<object, object?>(map.Count);
        foreach (var kvp in map)
        {
            var k = Default.ToForeignValue(kvp.Key) ?? "null";
            var v = Default.ToForeignValue(kvp.Value);
            result[k] = v;
        }
        return result;
    }

    /// <summary>
    /// Milestone 12.4: Foreign Interop Null-Boundary Hardening.
    /// Surfaces nullable C pointers, `Nullable&lt;T&gt;`, and nullable/unannotated .NET reference returns as `T?`.
    /// </summary>
    public static string InferForeignReturnPilotType(MethodInfo method)
    {
        var retType = method.ReturnType;
        if (retType == typeof(void)) return "void";

        var underlyingNullable = Nullable.GetUnderlyingType(retType);
        if (underlyingNullable != null)
        {
            return $"{MapClrBaseType(underlyingNullable)}?";
        }

        if (retType.IsPointer || retType == typeof(IntPtr) || retType == typeof(UIntPtr))
        {
            if (retType.IsPointer && retType.GetElementType() is { } elemT && elemT != typeof(void))
                return $"{MapClrBaseType(elemT)}?";
            return "ptr?";
        }

        if (retType.IsValueType)
        {
            return MapClrBaseType(retType);
        }

        // Reference return: check .NET NullabilityInfoContext; only NotNull is non-optional `T`,
        // while Nullable or Unknown (unannotated foreign reference return) hardens to `T?`!
        var nullCtx = new NullabilityInfoContext();
        var nullInfo = nullCtx.Create(method.ReturnParameter);
        string basePilot = MapClrBaseType(retType);
        return nullInfo.ReadState == NullabilityState.NotNull ? basePilot : $"{basePilot}?";
    }

    public static string InferForeignClrPilotType(Type clrType, bool isNullableAnnotation = true)
    {
        if (clrType == typeof(void)) return "void";
        var underlying = Nullable.GetUnderlyingType(clrType);
        if (underlying != null)
        {
            return $"{MapClrBaseType(underlying)}?";
        }
        if (clrType.IsPointer || clrType == typeof(IntPtr) || clrType == typeof(UIntPtr))
        {
            return "ptr?";
        }
        string baseName = MapClrBaseType(clrType);
        if (clrType.IsValueType)
        {
            return isNullableAnnotation ? $"{baseName}?" : baseName;
        }
        return isNullableAnnotation ? $"{baseName}?" : baseName;
    }

    /// <summary>
    /// Milestone 12.4: SQL Null-Boundary Hardening.
    /// Maps SQL column definitions to Pilot types, automatically surfacing nullable columns as `T?`.
    /// </summary>
    public static string InferSqlColumnPilotType(string sqlColumnSpec, bool isNotNullConstraint = false)
    {
        if (string.IsNullOrWhiteSpace(sqlColumnSpec)) return "any?";
        string upper = sqlColumnSpec.Trim().ToUpperInvariant();
        bool hasInlineNotNull = upper.Contains("NOT NULL", StringComparison.Ordinal) ||
                                upper.Contains("PRIMARY KEY", StringComparison.Ordinal) ||
                                upper.Contains("SERIAL", StringComparison.Ordinal);
        bool nonNull = isNotNullConstraint || hasInlineNotNull;

        string firstToken = upper.Split(new[] { ' ', '(', '\t' }, StringSplitOptions.RemoveEmptyEntries)[0];
        string baseType = firstToken switch
        {
            "INT" or "INTEGER" or "BIGINT" or "SMALLINT" or "TINYINT" or "SERIAL" or "BIGSERIAL" => "int",
            "REAL" or "FLOAT" or "DOUBLE" or "NUMERIC" or "DECIMAL" => "float",
            "BOOL" or "BOOLEAN" or "BIT" => "bool",
            "TEXT" or "VARCHAR" or "CHAR" or "NVARCHAR" or "UUID" or "TIMESTAMP" or "TIMESTAMPTZ" or "DATE" or "JSON" or "JSONB" => "str",
            "BYTEA" or "BLOB" or "BINARY" or "VARBINARY" => "bytes",
            _ => "any"
        };

        return nonNull ? baseType : $"{baseType}?";
    }

    private static string MapClrBaseType(Type t)
    {
        if (t == typeof(bool)) return "bool";
        if (t == typeof(int) || t == typeof(long) || t == typeof(short) || t == typeof(sbyte)) return "int";
        if (t == typeof(uint) || t == typeof(ulong) || t == typeof(ushort) || t == typeof(byte)) return "uint";
        if (t == typeof(double) || t == typeof(float)) return "float";
        if (t == typeof(decimal)) return "decimal";
        if (t == typeof(char)) return "char";
        if (t == typeof(string) || t == typeof(Guid) || t == typeof(DateTime) || t == typeof(DateTimeOffset)) return "str";
        if (t.IsArray && t.GetElementType() is { } elem) return $"{MapClrBaseType(elem)}[]";
        return t.Name;
    }
}
