using System;
using System.Collections.Generic;
using System.Globalization;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Adapters;

/// <summary>
/// Registry for compile-time and runtime argument type adapters.
/// Allows functions and method group signatures expecting a specific type
/// to seamlessly accept convertible types (e.g. str -> int, dict -> Model, scalar unrolling).
/// </summary>
public sealed class PilotTypeAdapterRegistry
{
    private static readonly Lazy<PilotTypeAdapterRegistry> _default = new(() => new PilotTypeAdapterRegistry());
    public static PilotTypeAdapterRegistry Default => _default.Value;

    private readonly Dictionary<(string From, string To), Func<PilotValue, PilotExecutionContext?, PilotValue>> _adapters
        = new(new AdapterKeyComparer());

    public PilotTypeAdapterRegistry()
    {
        RegisterBuiltins();
    }

    private void RegisterBuiltins()
    {
        // String -> Numbers
        Register("str", "int", (val, _) =>
        {
            if (long.TryParse(val.AsString().Trim(), NumberStyles.Integer, CultureInfo.InvariantCulture, out var n))
            {
                return PilotValue.FromInt(n);
            }
            if (double.TryParse(val.AsString().Trim(), NumberStyles.Float, CultureInfo.InvariantCulture, out var d))
            {
                return PilotValue.FromInt((long)d);
            }
            return PilotValue.Undefined;
        });

        Register("string", "int", (val, ctx) => _adapters[("str", "int")](val, ctx));

        Register("str", "float", (val, _) =>
        {
            if (double.TryParse(val.AsString().Trim(), NumberStyles.Float, CultureInfo.InvariantCulture, out var d))
            {
                return PilotValue.FromFloat(d);
            }
            return PilotValue.Undefined;
        });

        Register("string", "float", (val, ctx) => _adapters[("str", "float")](val, ctx));

        Register("str", "bool", (val, _) =>
        {
            var s = val.AsString().Trim().ToLowerInvariant();
            if (s is "true" or "1" or "yes") return PilotValue.True;
            if (s is "false" or "0" or "no") return PilotValue.False;
            return PilotValue.Undefined;
        });

        // Numbers -> String
        Register("int", "str", (val, _) => PilotValue.FromString(val.AsInt().ToString(CultureInfo.InvariantCulture)));
        Register("int", "string", (val, ctx) => _adapters[("int", "str")](val, ctx));
        Register("float", "str", (val, _) => PilotValue.FromString(val.AsFloat().ToString(CultureInfo.InvariantCulture)));
        Register("float", "string", (val, ctx) => _adapters[("float", "str")](val, ctx));
        Register("bool", "str", (val, _) => PilotValue.FromString(val.IsTrue ? "true" : "false"));

        // Numeric conversions
        Register("int", "float", (val, _) => PilotValue.FromFloat(val.AsInt()));
        Register("float", "int", (val, _) => PilotValue.FromInt((long)val.AsFloat()));

        // Collections: List <-> Tuple <-> Set
        Register("list", "tuple", (val, _) => PilotValue.FromTuple(val.AsList()));
        Register("tuple", "list", (val, _) => PilotValue.FromList(new List<PilotValue>(val.AsTuple())));
        Register("list", "set", (val, _) => PilotValue.FromSet(new HashSet<PilotValue>(val.AsList())));
        Register("set", "list", (val, _) => PilotValue.FromList(new List<PilotValue>(val.AsSet())));

        // Map / Dict -> Object
        Register("dict", "object", ConvertMapToObject);
        Register("map", "object", ConvertMapToObject);
    }

    private static PilotValue ConvertMapToObject(PilotValue val, PilotExecutionContext? ctx)
    {
        if (val.Kind != PilotValueKind.Map) return val;
        var map = val.AsMap();
        var obj = new PilotObject(new PilotClassDefinition("AdaptedObject"));
        foreach (var kvp in map)
        {
            string key = kvp.Key.AsString();
            obj.SetMember(key, kvp.Value);
        }
        return PilotValue.FromObject(obj);
    }

    public void Register(string fromType, string toType, Func<PilotValue, PilotExecutionContext?, PilotValue> converter)
    {
        if (string.IsNullOrWhiteSpace(fromType) || string.IsNullOrWhiteSpace(toType) || converter == null) return;
        _adapters[(fromType.Trim(), toType.Trim())] = converter;
    }

    public void Register(string fromType, string toType, PilotFunction converterFunc)
    {
        if (converterFunc == null) return;
        Register(fromType, toType, (val, ctx) =>
        {
            if (ctx == null) return val;
            return converterFunc.Invoke(ctx, new[] { val });
        });
    }

    /// <summary>
    /// Attempts to convert source value to target expected type.
    /// </summary>
    public bool TryConvert(PilotValue source, string? targetType, PilotExecutionContext? context, out PilotValue converted)
    {
        converted = source;
        if (string.IsNullOrWhiteSpace(targetType)) return true;
        targetType = targetType.Trim();

        string sourceTypeName = GetTypeName(source);

        // 1. Direct identity or 'any' / 'dynamic' / 'object'
        if (string.Equals(sourceTypeName, targetType, StringComparison.OrdinalIgnoreCase) ||
            string.Equals(targetType, "any", StringComparison.OrdinalIgnoreCase) ||
            string.Equals(targetType, "dynamic", StringComparison.OrdinalIgnoreCase) ||
            (string.Equals(targetType, "object", StringComparison.OrdinalIgnoreCase) && source.Kind == PilotValueKind.Object))
        {
            converted = source;
            return true;
        }

        // 2. Check registered adapters
        if (_adapters.TryGetValue((sourceTypeName, targetType), out var adapter))
        {
            try
            {
                var res = adapter(source, context);
                if (!res.IsUndefined)
                {
                    converted = res;
                    return true;
                }
                return false;
            }
            catch
            {
                return false;
            }
        }

        // 3. Structural Map -> Target Class conversion (e.g. { name: "x" } -> User)
        if (source.Kind == PilotValueKind.Map && context != null && context.Environment.TryResolve(targetType, out var classVal) && classVal.Kind == PilotValueKind.Function)
        {
            var classDef = classVal.AsFunction().ClassDef;
            if (classDef != null)
            {
                var newObj = new PilotObject(classDef);
                var map = source.AsMap();
                foreach (var kvp in map)
                {
                    string k = kvp.Key.AsString();
                    newObj.SetMember(k, kvp.Value);
                }
                converted = PilotValue.FromObject(newObj);
                return true;
            }
        }

        return false;
    }

    public static string GetTypeName(PilotValue val)
    {
        return val.Kind switch
        {
            PilotValueKind.Int or PilotValueKind.UInt => "int",
            PilotValueKind.Float => "float",
            PilotValueKind.String => "str",
            PilotValueKind.Bool => "bool",
            PilotValueKind.List => "list",
            PilotValueKind.Map => "map",
            PilotValueKind.Set => "set",
            PilotValueKind.Tuple => "tuple",
            PilotValueKind.Object => val.AsObject().TypeName,
            PilotValueKind.Function => "function",
            PilotValueKind.MethodGroup => "methodgroup",
            _ => val.Kind.ToString().ToLowerInvariant()
        };
    }

    private sealed class AdapterKeyComparer : IEqualityComparer<(string From, string To)>
    {
        public bool Equals((string From, string To) x, (string From, string To) y)
        {
            return string.Equals(x.From, y.From, StringComparison.OrdinalIgnoreCase) &&
                   string.Equals(x.To, y.To, StringComparison.OrdinalIgnoreCase);
        }

        public int GetHashCode((string From, string To) obj)
        {
            return HashCode.Combine(
                StringComparer.OrdinalIgnoreCase.GetHashCode(obj.From),
                StringComparer.OrdinalIgnoreCase.GetHashCode(obj.To));
        }
    }
}
