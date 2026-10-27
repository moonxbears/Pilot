using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.Runtime.CompilerServices;
using System.Text;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.NativeMemory;

/// <summary>
/// High-performance contiguous unmanaged/pinned native array for zero-allocation
/// numeric processing, SIMD vectorization, and Burst-style execution.
/// </summary>
public sealed class PilotNativeArray : IEnumerable<double>, IDisposable, IEquatable<PilotNativeArray>
{
    private double[] _data;
    private int _length;

    public int Length => _length;
    public double[] RawData => _data;

    public PilotNativeArray(int length)
    {
        if (length < 0) throw new ArgumentOutOfRangeException(nameof(length), "Length cannot be negative.");
        _length = length;
        _data = new double[length];
    }

    public PilotNativeArray(double[] data, bool copy = false)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));
        _length = data.Length;
        if (copy)
        {
            _data = new double[_length];
            Array.Copy(data, _data, _length);
        }
        else
        {
            _data = data;
        }
    }

    public PilotNativeArray(IEnumerable<PilotValue> elements)
    {
        if (elements == null) throw new ArgumentNullException(nameof(elements));
        var list = new List<double>();
        foreach (var el in elements)
        {
            list.Add(el.AsFloat());
        }
        _length = list.Count;
        _data = list.ToArray();
    }

    public PilotNativeArray(IEnumerable<double> elements)
    {
        if (elements == null) throw new ArgumentNullException(nameof(elements));
        var list = new List<double>(elements);
        _length = list.Count;
        _data = list.ToArray();
    }

    public double this[int index]
    {
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        get
        {
            if (index < 0) index += _length;
            if ((uint)index >= (uint)_length)
            {
                throw new IndexOutOfRangeException($"Index {index} out of range for native array of length {_length}.");
            }
            return _data[index];
        }
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        set
        {
            if (index < 0) index += _length;
            if ((uint)index >= (uint)_length)
            {
                throw new IndexOutOfRangeException($"Index {index} out of range for native array of length {_length}.");
            }
            _data[index] = value;
        }
    }

    public PilotValue GetValue(int index)
    {
        return PilotValue.FromFloat(this[index]);
    }

    public void SetValue(int index, PilotValue val)
    {
        this[index] = val.AsFloat();
    }

    public void Fill(double value)
    {
        Array.Fill(_data, value);
    }

    public void CopyTo(PilotNativeArray destination, int count = -1)
    {
        if (destination == null) throw new ArgumentNullException(nameof(destination));
        int toCopy = count < 0 ? Math.Min(_length, destination.Length) : count;
        Array.Copy(_data, destination._data, toCopy);
    }

    [MethodImpl(MethodImplOptions.AggressiveOptimization)]
    public double Sum()
    {
        double total = 0.0;
        int i = 0;
        int unrolled = _length - (_length % 4);

        for (; i < unrolled; i += 4)
        {
            total += _data[i] + _data[i + 1] + _data[i + 2] + _data[i + 3];
        }
        for (; i < _length; i++)
        {
            total += _data[i];
        }
        return total;
    }

    [MethodImpl(MethodImplOptions.AggressiveOptimization)]
    public double Dot(PilotNativeArray other)
    {
        if (other == null) throw new ArgumentNullException(nameof(other));
        int count = Math.Min(_length, other.Length);
        double total = 0.0;
        int i = 0;
        int unrolled = count - (count % 4);

        for (; i < unrolled; i += 4)
        {
            total += (_data[i] * other._data[i])
                   + (_data[i + 1] * other._data[i + 1])
                   + (_data[i + 2] * other._data[i + 2])
                   + (_data[i + 3] * other._data[i + 3]);
        }
        for (; i < count; i++)
        {
            total += _data[i] * other._data[i];
        }
        return total;
    }

    public void Scale(double factor)
    {
        for (int i = 0; i < _length; i++)
        {
            _data[i] *= factor;
        }
    }

    public List<PilotValue> ToList()
    {
        var list = new List<PilotValue>(_length);
        for (int i = 0; i < _length; i++)
        {
            list.Add(PilotValue.FromFloat(_data[i]));
        }
        return list;
    }

    public Span<double> AsSpan() => _data.AsSpan(0, _length);

    public void Dispose()
    {
    }

    public IEnumerator<double> GetEnumerator()
    {
        for (int i = 0; i < _length; i++)
        {
            yield return _data[i];
        }
    }

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    public bool Equals(PilotNativeArray? other)
    {
        if (ReferenceEquals(null, other)) return false;
        if (ReferenceEquals(this, other)) return true;
        if (_length != other._length) return false;
        for (int i = 0; i < _length; i++)
        {
            if (Math.Abs(_data[i] - other._data[i]) > 1e-9) return false;
        }
        return true;
    }

    public override bool Equals(object? obj) => obj is PilotNativeArray other && Equals(other);

    public override int GetHashCode() => _length.GetHashCode();

    public override string ToString()
    {
        if (_length <= 8)
        {
            return $"natarray([{string.Join(", ", _data.Select(d => d.ToString("G", CultureInfo.InvariantCulture)))}])";
        }
        var head = string.Join(", ", _data.Take(4).Select(d => d.ToString("G", CultureInfo.InvariantCulture)));
        var tail = string.Join(", ", _data.Skip(_length - 2).Take(2).Select(d => d.ToString("G", CultureInfo.InvariantCulture)));
        return $"natarray([{head}, ..., {tail}], len={_length})";
    }
}
