using System;
using System.Text;

namespace Pilot.Runtime.Values;

/// <summary>
/// Represents a fixed or dynamic matrix in Pilot (e.g. matrix3x3&lt;u8&gt; or matrix&lt;float&gt;).
/// </summary>
public sealed class PilotMatrix : IEquatable<PilotMatrix>
{
    private readonly double[] _data;

    public int Rows { get; }
    public int Cols { get; }
    public string ElementType { get; }

    public PilotMatrix(int rows, int cols, string elementType = "float", double[]? initialData = null)
    {
        if (rows <= 0) throw new ArgumentOutOfRangeException(nameof(rows), "Rows must be positive.");
        if (cols <= 0) throw new ArgumentOutOfRangeException(nameof(cols), "Cols must be positive.");

        Rows = rows;
        Cols = cols;
        ElementType = elementType;
        _data = new double[rows * cols];

        if (initialData != null)
        {
            int copyLen = Math.Min(initialData.Length, _data.Length);
            Array.Copy(initialData, _data, copyLen);
        }
    }

    public double this[int r, int c]
    {
        get
        {
            ValidateBounds(r, c);
            return _data[r * Cols + c];
        }
        set
        {
            ValidateBounds(r, c);
            _data[r * Cols + c] = value;
        }
    }

    public static PilotMatrix Identity(int size, string elementType = "float")
    {
        var m = new PilotMatrix(size, size, elementType);
        for (int i = 0; i < size; i++)
        {
            m[i, i] = 1.0;
        }
        return m;
    }

    public static PilotMatrix Zeros(int rows, int cols, string elementType = "float") =>
        new PilotMatrix(rows, cols, elementType);

    public static PilotMatrix Ones(int rows, int cols, string elementType = "float")
    {
        var m = new PilotMatrix(rows, cols, elementType);
        for (int i = 0; i < m._data.Length; i++)
        {
            m._data[i] = 1.0;
        }
        return m;
    }

    public PilotMatrix Add(PilotMatrix other)
    {
        if (Rows != other.Rows || Cols != other.Cols)
            throw new InvalidOperationException($"Cannot add matrices of mismatched dimensions: ({Rows}x{Cols}) vs ({other.Rows}x{other.Cols})");

        var result = new PilotMatrix(Rows, Cols, ElementType);
        for (int i = 0; i < _data.Length; i++)
        {
            result._data[i] = _data[i] + other._data[i];
        }
        return result;
    }

    public PilotMatrix Subtract(PilotMatrix other)
    {
        if (Rows != other.Rows || Cols != other.Cols)
            throw new InvalidOperationException($"Cannot subtract matrices of mismatched dimensions: ({Rows}x{Cols}) vs ({other.Rows}x{other.Cols})");

        var result = new PilotMatrix(Rows, Cols, ElementType);
        for (int i = 0; i < _data.Length; i++)
        {
            result._data[i] = _data[i] - other._data[i];
        }
        return result;
    }

    public PilotMatrix Multiply(PilotMatrix other)
    {
        if (Cols != other.Rows)
            throw new InvalidOperationException($"Cannot multiply matrices with inner dimensions mismatch: ({Rows}x{Cols}) * ({other.Rows}x{other.Cols})");

        var result = new PilotMatrix(Rows, other.Cols, ElementType);
        for (int i = 0; i < Rows; i++)
        {
            for (int j = 0; j < other.Cols; j++)
            {
                double sum = 0;
                for (int k = 0; k < Cols; k++)
                {
                    sum += this[i, k] * other[k, j];
                }
                result[i, j] = sum;
            }
        }
        return result;
    }

    public PilotMatrix MultiplyScalar(double scalar)
    {
        var result = new PilotMatrix(Rows, Cols, ElementType);
        for (int i = 0; i < _data.Length; i++)
        {
            result._data[i] = _data[i] * scalar;
        }
        return result;
    }

    public PilotMatrix Transpose()
    {
        var result = new PilotMatrix(Cols, Rows, ElementType);
        for (int r = 0; r < Rows; r++)
        {
            for (int c = 0; c < Cols; c++)
            {
                result[c, r] = this[r, c];
            }
        }
        return result;
    }

    public bool Equals(PilotMatrix? other)
    {
        if (other is null) return false;
        if (Rows != other.Rows || Cols != other.Cols) return false;
        for (int i = 0; i < _data.Length; i++)
        {
            if (Math.Abs(_data[i] - other._data[i]) > 1e-9) return false;
        }
        return true;
    }

    public override bool Equals(object? obj) => Equals(obj as PilotMatrix);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(Rows);
        hash.Add(Cols);
        foreach (var d in _data) hash.Add(d);
        return hash.ToHashCode();
    }

    public override string ToString()
    {
        var sb = new StringBuilder();
        sb.Append($"matrix{Rows}x{Cols}<{ElementType}>[");
        for (int r = 0; r < Rows; r++)
        {
            if (r > 0) sb.Append("; ");
            for (int c = 0; c < Cols; c++)
            {
                if (c > 0) sb.Append(", ");
                sb.Append(this[r, c]);
            }
        }
        sb.Append(']');
        return sb.ToString();
    }

    private void ValidateBounds(int r, int c)
    {
        if (r < 0 || r >= Rows || c < 0 || c >= Cols)
            throw new IndexOutOfRangeException($"Matrix index [{r}, {c}] out of bounds for {Rows}x{Cols} matrix.");
    }
}
