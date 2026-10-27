using System;
using System.Collections;
using System.Collections.Generic;
using Pilot.Runtime.Values;

namespace Pilot.CodeGen.Model;

/// <summary>
/// Lightweight, dynamic metadata container attached to scripts, classes,
/// properties, methods, and parameters.
/// Simple and Pythonic: meta["key"], meta.get("key", default), meta.has("key").
/// </summary>
public sealed class MetadataModel : IEnumerable<KeyValuePair<string, object?>>
{
    private readonly Dictionary<string, object?> _entries = new(StringComparer.OrdinalIgnoreCase);

    public int Count => _entries.Count;

    public object? this[string key]
    {
        get => _entries.TryGetValue(key, out var val) ? val : null;
        set => _entries[key] = value;
    }

    public void Set(string key, object? value)
    {
        if (string.IsNullOrEmpty(key)) return;
        _entries[key] = value;
    }

    public object? Get(string key, object? defaultValue = null)
    {
        return _entries.TryGetValue(key, out var val) ? val : defaultValue;
    }

    public T? Get<T>(string key, T? defaultValue = default)
    {
        if (_entries.TryGetValue(key, out var val) && val != null)
        {
            if (val is T typed) return typed;
            try
            {
                return (T)Convert.ChangeType(val, typeof(T));
            }
            catch
            {
                return defaultValue;
            }
        }
        return defaultValue;
    }

    public bool Has(string key) => _entries.ContainsKey(key);

    public bool Remove(string key) => _entries.Remove(key);

    public void Clear() => _entries.Clear();

    public IReadOnlyDictionary<string, object?> Entries => _entries;

    public PilotValue ToPilotMap()
    {
        var map = new Dictionary<PilotValue, PilotValue>(_entries.Count);
        foreach (var (k, v) in _entries)
        {
            map[PilotValue.FromString(k)] = v switch
            {
                null => PilotValue.Null,
                bool b => b ? PilotValue.True : PilotValue.False,
                int i => PilotValue.FromInt(i),
                long l => PilotValue.FromInt(l),
                double d => PilotValue.FromFloat(d),
                float f => PilotValue.FromFloat(f),
                string s => PilotValue.FromString(s),
                PilotValue pv => pv,
                _ => PilotValue.FromString(v.ToString() ?? "")
            };
        }
        return PilotValue.FromMap(map);
    }

    public IEnumerator<KeyValuePair<string, object?>> GetEnumerator() => _entries.GetEnumerator();
    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    public override string ToString() => $"Meta({string.Join(", ", _entries.Select(e => $"{e.Key}={e.Value}"))})";
}
