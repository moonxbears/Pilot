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
        if (foreignObject is null || foreignObject is DBNull)
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

        // Direct primitive mappings
        switch (foreignObject)
        {
            case bool b:
                return PilotValue.FromBool(b);

            case sbyte sb:
                return PilotValue.FromInt(sb);
            case byte b:
                return PilotValue.FromUInt(b);
            case short s:
                return PilotValue.FromInt(s);
            case ushort us:
                return PilotValue.FromUInt(us);
            case int i:
                return PilotValue.FromInt(i);
            case uint u:
                return PilotValue.FromUInt(u);
            case long l:
                return PilotValue.FromInt(l);
            case ulong ul:
                return PilotValue.FromUInt(ul);
            case BigInteger bi:
                return PilotValue.FromInt((long)bi);

            case float f:
                return PilotValue.FromFloat(f);
            case double d:
                return PilotValue.FromFloat(d);
            case decimal dec:
                return PilotValue.FromDecimal(dec);

            case char c:
                return PilotValue.FromChar(c);
            case string str:
                return PilotValue.FromString(str);

            case Guid guid:
                return PilotValue.FromString(guid.ToString());
            case DateTime dt:
                return PilotValue.FromString(dt.ToString("o", CultureInfo.InvariantCulture));
            case DateTimeOffset dto:
                return PilotValue.FromString(dto.ToString("o", CultureInfo.InvariantCulture));
            case TimeSpan ts:
                return PilotValue.FromFloat(ts.TotalMilliseconds);

            case PilotRange range:
                return PilotValue.FromRange(range);
            case PilotMatrix matrix:
                return PilotValue.FromMatrix(matrix);
            case PilotFunction func:
                return PilotValue.FromFunction(func);
            case PilotObject obj:
                return PilotValue.FromObject(obj);
        }

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
            return PilotValue.FromMap(pilotMap);
        }

        // Tuples
        if (foreignObject is ITuple tuple)
        {
            var tupleItems = new List<PilotValue>(tuple.Length);
            for (int i = 0; i < tuple.Length; i++)
            {
                tupleItems.Add(ToPilotValue(tuple[i], sourceLanguage));
            }
            return PilotValue.FromTuple(tupleItems);
        }

        // Sets
        if (type.IsGenericType && type.GetGenericTypeDefinition() == typeof(HashSet<>))
        {
            var pilotSet = new HashSet<PilotValue>();
            foreach (var item in (IEnumerable)foreignObject)
            {
                pilotSet.Add(ToPilotValue(item, sourceLanguage));
            }
            return PilotValue.FromSet(pilotSet);
        }

        // Lists, Arrays, and Sequences
        if (foreignObject is IEnumerable enumerable && !(foreignObject is string))
        {
            var pilotList = new List<PilotValue>();
            foreach (var item in enumerable)
            {
                pilotList.Add(ToPilotValue(item, sourceLanguage));
            }
            return PilotValue.FromList(pilotList);
        }

        // Complex Objects, POCOs, Anonymous Types
        return ConvertPocoToPilotObject(foreignObject, type);
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

        // Explicit target type coercions
        if (targetType == typeof(int)) return (int)value.AsInt();
        if (targetType == typeof(long)) return value.AsInt();
        if (targetType == typeof(uint)) return (uint)value.AsInt();
        if (targetType == typeof(ulong)) return (ulong)value.AsInt();
        if (targetType == typeof(double)) return value.AsFloat();
        if (targetType == typeof(float)) return (float)value.AsFloat();
        if (targetType == typeof(decimal)) return (decimal)value.AsFloat();
        if (targetType == typeof(bool)) return value.IsTrue;
        if (targetType == typeof(string)) return value.AsString();
        if (targetType == typeof(char)) return (char)value.AsInt();

        if (targetType == typeof(PilotMatrix) && value.Kind == PilotValueKind.Matrix)
        {
            return value.AsMatrix();
        }

        if (targetType == typeof(PilotRange) && value.Kind == PilotValueKind.Range)
        {
            return value.AsRange();
        }

        return value.ToString();
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
}
