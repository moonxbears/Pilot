using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Text;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.NativeMemory;

/// <summary>
/// High-performance cache-friendly open-addressing native map with linear probing
/// for fast, zero-allocation dictionary access.
/// </summary>
public sealed class PilotNativeMap : IEnumerable<KeyValuePair<PilotValue, PilotValue>>, IDisposable, IEquatable<PilotNativeMap>
{
    private const byte StateEmpty = 0;
    private const byte StateOccupied = 1;
    private const byte StateTombstone = 2;

    private PilotValue[] _keys;
    private PilotValue[] _values;
    private byte[] _states;
    private int _count;
    private int _capacity;

    public int Count => _count;
    public int Length => _count;

    public PilotNativeMap(int initialCapacity = 16)
    {
        _capacity = Math.Max(8, RoundUpToPowerOf2(initialCapacity));
        _keys = new PilotValue[_capacity];
        _values = new PilotValue[_capacity];
        _states = new byte[_capacity];
        _count = 0;
    }

    public PilotNativeMap(IDictionary<PilotValue, PilotValue> dictionary)
        : this(dictionary?.Count * 2 ?? 16)
    {
        if (dictionary != null)
        {
            foreach (var kvp in dictionary)
            {
                Set(kvp.Key, kvp.Value);
            }
        }
    }

    public PilotValue this[PilotValue key]
    {
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        get => Get(key);
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        set => Set(key, value);
    }

    public bool Has(PilotValue key)
    {
        return FindSlot(key, out _);
    }

    public PilotValue Get(PilotValue key, PilotValue defaultValue = default)
    {
        if (FindSlot(key, out int slot))
        {
            return _values[slot];
        }
        return defaultValue.Kind == PilotValueKind.Undefined ? PilotValue.Undefined : defaultValue;
    }

    public void Set(PilotValue key, PilotValue value)
    {
        if ((_count + 1) * 10 >= _capacity * 7) // 70% load factor
        {
            Resize(_capacity * 2);
        }

        uint hash = (uint)key.GetHashCode();
        int mask = _capacity - 1;
        int slot = (int)(hash & mask);
        int firstTombstone = -1;

        for (int i = 0; i < _capacity; i++)
        {
            byte state = _states[slot];
            if (state == StateEmpty)
            {
                int targetSlot = firstTombstone >= 0 ? firstTombstone : slot;
                _keys[targetSlot] = key;
                _values[targetSlot] = value;
                _states[targetSlot] = StateOccupied;
                _count++;
                return;
            }
            if (state == StateOccupied && _keys[slot].Equals(key))
            {
                _values[slot] = value;
                return;
            }
            if (state == StateTombstone && firstTombstone < 0)
            {
                firstTombstone = slot;
            }

            slot = (slot + 1) & mask;
        }

        if (firstTombstone >= 0)
        {
            _keys[firstTombstone] = key;
            _values[firstTombstone] = value;
            _states[firstTombstone] = StateOccupied;
            _count++;
        }
    }

    public bool Delete(PilotValue key)
    {
        if (FindSlot(key, out int slot))
        {
            _states[slot] = StateTombstone;
            _keys[slot] = PilotValue.Undefined;
            _values[slot] = PilotValue.Undefined;
            _count--;
            return true;
        }
        return false;
    }

    public void Clear()
    {
        Array.Clear(_keys, 0, _capacity);
        Array.Clear(_values, 0, _capacity);
        Array.Clear(_states, 0, _capacity);
        _count = 0;
    }

    public Dictionary<PilotValue, PilotValue> ToDictionary()
    {
        var dict = new Dictionary<PilotValue, PilotValue>(_count);
        for (int i = 0; i < _capacity; i++)
        {
            if (_states[i] == StateOccupied)
            {
                dict[_keys[i]] = _values[i];
            }
        }
        return dict;
    }

    public List<PilotValue> Keys()
    {
        var keys = new List<PilotValue>(_count);
        for (int i = 0; i < _capacity; i++)
        {
            if (_states[i] == StateOccupied)
            {
                keys.Add(_keys[i]);
            }
        }
        return keys;
    }

    public List<PilotValue> Values()
    {
        var values = new List<PilotValue>(_count);
        for (int i = 0; i < _capacity; i++)
        {
            if (_states[i] == StateOccupied)
            {
                values.Add(_values[i]);
            }
        }
        return values;
    }

    private bool FindSlot(PilotValue key, out int resultSlot)
    {
        uint hash = (uint)key.GetHashCode();
        int mask = _capacity - 1;
        int slot = (int)(hash & mask);

        for (int i = 0; i < _capacity; i++)
        {
            byte state = _states[slot];
            if (state == StateEmpty)
            {
                resultSlot = -1;
                return false;
            }
            if (state == StateOccupied && _keys[slot].Equals(key))
            {
                resultSlot = slot;
                return true;
            }
            slot = (slot + 1) & mask;
        }

        resultSlot = -1;
        return false;
    }

    private void Resize(int newCapacity)
    {
        var oldKeys = _keys;
        var oldValues = _values;
        var oldStates = _states;
        int oldCap = _capacity;

        _capacity = newCapacity;
        _keys = new PilotValue[_capacity];
        _values = new PilotValue[_capacity];
        _states = new byte[_capacity];
        _count = 0;

        for (int i = 0; i < oldCap; i++)
        {
            if (oldStates[i] == StateOccupied)
            {
                Set(oldKeys[i], oldValues[i]);
            }
        }
    }

    private static int RoundUpToPowerOf2(int v)
    {
        v--;
        v |= v >> 1;
        v |= v >> 2;
        v |= v >> 4;
        v |= v >> 8;
        v |= v >> 16;
        return v + 1;
    }

    public void Dispose()
    {
    }

    public IEnumerator<KeyValuePair<PilotValue, PilotValue>> GetEnumerator()
    {
        for (int i = 0; i < _capacity; i++)
        {
            if (_states[i] == StateOccupied)
            {
                yield return new KeyValuePair<PilotValue, PilotValue>(_keys[i], _values[i]);
            }
        }
    }

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    public bool Equals(PilotNativeMap? other)
    {
        if (ReferenceEquals(null, other)) return false;
        if (ReferenceEquals(this, other)) return true;
        if (_count != other._count) return false;
        foreach (var kvp in this)
        {
            if (!other.FindSlot(kvp.Key, out int s) || !other._values[s].Equals(kvp.Value))
            {
                return false;
            }
        }
        return true;
    }

    public override bool Equals(object? obj) => obj is PilotNativeMap other && Equals(other);

    public override int GetHashCode() => _count.GetHashCode();

    public override string ToString()
    {
        var entries = this.Take(6).Select(kvp => $"{kvp.Key}: {kvp.Value}");
        string inner = string.Join(", ", entries);
        if (_count > 6) inner += ", ...";
        return $"natmap({{{inner}}}, count={_count})";
    }
}
