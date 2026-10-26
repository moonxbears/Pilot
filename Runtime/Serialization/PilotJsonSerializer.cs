using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Serialization;

/// <summary>
/// Provides bidirectional JSON serialization and deserialization for Pilot native values and classes.
/// </summary>
public static class PilotJsonSerializer
{
    public static string Serialize(PilotValue value, bool formatted = false)
    {
        var options = new JsonWriterOptions { Indented = formatted };
        using var stream = new MemoryStream();
        using var writer = new Utf8JsonWriter(stream, options);

        WriteValue(writer, value);
        writer.Flush();

        return Encoding.UTF8.GetString(stream.ToArray());
    }

    public static PilotValue Deserialize(string json, PilotClassDefinition? targetClass = null, PilotExecutionContext? context = null)
    {
        if (string.IsNullOrWhiteSpace(json))
        {
            return PilotValue.Null;
        }

        using var doc = JsonDocument.Parse(json);
        return ConvertJsonElement(doc.RootElement, targetClass, context);
    }

    private static void WriteValue(Utf8JsonWriter writer, PilotValue val)
    {
        switch (val.Kind)
        {
            case PilotValueKind.Null:
            case PilotValueKind.Undefined:
                writer.WriteNullValue();
                break;

            case PilotValueKind.Bool:
                writer.WriteBooleanValue(val.IsTrue);
                break;

            case PilotValueKind.Int:
                writer.WriteNumberValue(val.AsInt());
                break;

            case PilotValueKind.UInt:
                writer.WriteNumberValue((ulong)val.AsInt());
                break;

            case PilotValueKind.Float:
                writer.WriteNumberValue(val.AsFloat());
                break;

            case PilotValueKind.Decimal:
                writer.WriteNumberValue((decimal)val.AsFloat());
                break;

            case PilotValueKind.Char:
            case PilotValueKind.String:
                writer.WriteStringValue(val.AsString());
                break;

            case PilotValueKind.List:
                writer.WriteStartArray();
                foreach (var item in val.AsList())
                {
                    WriteValue(writer, item);
                }
                writer.WriteEndArray();
                break;

            case PilotValueKind.Tuple:
                writer.WriteStartArray();
                foreach (var item in val.AsTuple())
                {
                    WriteValue(writer, item);
                }
                writer.WriteEndArray();
                break;

            case PilotValueKind.Set:
                writer.WriteStartArray();
                foreach (var item in val.AsSet())
                {
                    WriteValue(writer, item);
                }
                writer.WriteEndArray();
                break;

            case PilotValueKind.Map:
                writer.WriteStartObject();
                foreach (var kvp in val.AsMap())
                {
                    string propName = kvp.Key.AsString();
                    writer.WritePropertyName(propName);
                    WriteValue(writer, kvp.Value);
                }
                writer.WriteEndObject();
                break;

            case PilotValueKind.Object:
                var obj = val.AsObject();
                writer.WriteStartObject();
                foreach (var kvp in obj.Fields)
                {
                    writer.WritePropertyName(kvp.Key);
                    WriteValue(writer, kvp.Value);
                }
                writer.WriteEndObject();
                break;

            case PilotValueKind.Matrix:
                var matrix = val.AsMatrix();
                writer.WriteStartArray();
                for (int r = 0; r < matrix.Rows; r++)
                {
                    writer.WriteStartArray();
                    for (int c = 0; c < matrix.Cols; c++)
                    {
                        writer.WriteNumberValue(matrix[r, c]);
                    }
                    writer.WriteEndArray();
                }
                writer.WriteEndArray();
                break;

            default:
                writer.WriteStringValue(val.ToString());
                break;
        }
    }

    private static PilotValue ConvertJsonElement(JsonElement el, PilotClassDefinition? targetClass, PilotExecutionContext? context)
    {
        switch (el.ValueKind)
        {
            case JsonValueKind.Null:
            case JsonValueKind.Undefined:
                return PilotValue.Null;

            case JsonValueKind.True:
                return PilotValue.True;

            case JsonValueKind.False:
                return PilotValue.False;

            case JsonValueKind.Number:
                if (el.TryGetInt64(out long l)) return PilotValue.FromInt(l);
                if (el.TryGetDouble(out double d)) return PilotValue.FromFloat(d);
                return PilotValue.FromFloat(0);

            case JsonValueKind.String:
                return PilotValue.FromString(el.GetString() ?? string.Empty);

            case JsonValueKind.Array:
                var list = new List<PilotValue>(el.GetArrayLength());
                foreach (var item in el.EnumerateArray())
                {
                    list.Add(ConvertJsonElement(item, null, context));
                }
                return PilotValue.FromList(list);

            case JsonValueKind.Object:
                if (targetClass != null)
                {
                    var instance = targetClass.Instantiate(context ?? new PilotExecutionContext(new PilotEngine(), new PilotExecutionOptions()), Array.Empty<PilotValue>());
                    foreach (var prop in el.EnumerateObject())
                    {
                        instance.SetMember(prop.Name, ConvertJsonElement(prop.Value, null, context));
                    }
                    return PilotValue.FromObject(instance);
                }
                else
                {
                    var map = new Dictionary<PilotValue, PilotValue>();
                    foreach (var prop in el.EnumerateObject())
                    {
                        map[PilotValue.FromString(prop.Name)] = ConvertJsonElement(prop.Value, null, context);
                    }
                    return PilotValue.FromMap(map);
                }

            default:
                return PilotValue.Undefined;
        }
    }
}
