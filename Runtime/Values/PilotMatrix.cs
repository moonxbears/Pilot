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
        int stride = size + 1;
        var data = m._data;
        for (int idx = 0; idx < data.Length; idx += stride)
        {
            data[idx] = 1.0;
        }
        return m;
    }

    public static PilotMatrix Zeros(int rows, int cols, string elementType = "float") =>
        new PilotMatrix(rows, cols, elementType);

    public static PilotMatrix Ones(int rows, int cols, string elementType = "float")
    {
        var m = new PilotMatrix(rows, cols, elementType);
        Array.Fill(m._data, 1.0);
        return m;
    }

    public PilotMatrix Add(PilotMatrix other)
    {
        if (Rows != other.Rows || Cols != other.Cols)
            throw new InvalidOperationException($"Cannot add matrices of mismatched dimensions: ({Rows}x{Cols}) vs ({other.Rows}x{other.Cols})");

        var result = new PilotMatrix(Rows, Cols, ElementType);
        var a = _data;
        var b = other._data;
        var dst = result._data;
        for (int i = 0; i < a.Length; i++)
        {
            dst[i] = a[i] + b[i];
        }
        return result;
    }

    public PilotMatrix Subtract(PilotMatrix other)
    {
        if (Rows != other.Rows || Cols != other.Cols)
            throw new InvalidOperationException($"Cannot subtract matrices of mismatched dimensions: ({Rows}x{Cols}) vs ({other.Rows}x{other.Cols})");

        var result = new PilotMatrix(Rows, Cols, ElementType);
        var a = _data;
        var b = other._data;
        var dst = result._data;
        for (int i = 0; i < a.Length; i++)
        {
            dst[i] = a[i] - b[i];
        }
        return result;
    }

    public PilotMatrix Multiply(PilotMatrix other)
    {
        if (Cols != other.Rows)
            throw new InvalidOperationException($"Cannot multiply matrices with inner dimensions mismatch: ({Rows}x{Cols}) * ({other.Rows}x{other.Cols})");

        int rows = Rows;
        int kDim = Cols;
        int otherCols = other.Cols;
        var result = new PilotMatrix(rows, otherCols, ElementType);
        var a = _data;
        var b = other._data;
        var c = result._data;

        // Cache-friendly row-major (i, k, j) loop order without per-element bounds checks
        for (int i = 0; i < rows; i++)
        {
            int aRowOffset = i * kDim;
            int cRowOffset = i * otherCols;
            for (int k = 0; k < kDim; k++)
            {
                double aik = a[aRowOffset + k];
                if (aik == 0.0) continue;
                int bRowOffset = k * otherCols;
                for (int j = 0; j < otherCols; j++)
                {
                    c[cRowOffset + j] += aik * b[bRowOffset + j];
                }
            }
        }
        return result;
    }

    public PilotMatrix MultiplyScalar(double scalar)
    {
        var result = new PilotMatrix(Rows, Cols, ElementType);
        var src = _data;
        var dst = result._data;
        for (int i = 0; i < src.Length; i++)
        {
            dst[i] = src[i] * scalar;
        }
        return result;
    }

    public PilotMatrix Transpose()
    {
        int rows = Rows;
        int cols = Cols;
        var result = new PilotMatrix(cols, rows, ElementType);
        var src = _data;
        var dst = result._data;
        for (int r = 0; r < rows; r++)
        {
            int rOffset = r * cols;
            for (int c = 0; c < cols; c++)
            {
                dst[c * rows + r] = src[rOffset + c];
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
            int rOffset = r * Cols;
            for (int c = 0; c < Cols; c++)
            {
                if (c > 0) sb.Append(", ");
                sb.Append(_data[rOffset + c]);
            }
        }
        sb.Append(']');
        return sb.ToString();
    }

    private void ValidateBounds(int r, int c)
    {
        if ((uint)r >= (uint)Rows || (uint)c >= (uint)Cols)
            throw new IndexOutOfRangeException($"Matrix index [{r}, {c}] out of bounds for {Rows}x{Cols} matrix.");
    }
}
