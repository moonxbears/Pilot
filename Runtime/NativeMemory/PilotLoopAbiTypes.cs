using System;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Pilot.Runtime.NativeMemory;

/// <summary>
/// Milestone 15.3.1: ABI-Stable Worklist Struct (<see cref="RangeWindow{TIter}"/>).
/// Explicitly laid out sequentially (<c>#[repr(C)]</c> equivalent) with zero padding bytes:
/// <c>sizeof(RangeWindow&lt;TIter&gt;) == 2 * sizeof(TIter)</c>.
/// Stores range-relative integer indices (<c>Current</c> and <c>Max</c>), never raw pointers,
/// guaranteeing base-address independence across shared-memory / <c>mmap</c> IPC boundaries.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public struct RangeWindow<TIter> : IEquatable<RangeWindow<TIter>> where TIter : unmanaged
{
    /// <summary>Offset: 0. Next relative index to visit.</summary>
    public TIter Current;

    /// <summary>Offset: sizeof(TIter). Inclusive upper bound of this range window.</summary>
    public TIter Max;

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public RangeWindow(TIter current, TIter max)
    {
        Current = current;
        Max = max;
    }

    public bool Equals(RangeWindow<TIter> other) =>
         EqualityComparer<TIter>.Default.Equals(Current, other.Current) &&
         EqualityComparer<TIter>.Default.Equals(Max, other.Max);

    public override bool Equals(object? obj) => obj is RangeWindow<TIter> other && Equals(other);

    public override int GetHashCode() => HashCode.Combine(Current, Max);

    public override string ToString() => $"RangeWindow({Current}..={Max})";
}

/// <summary>
/// Milestone 15.3.3: Universal 24-byte C-ABI Slice Header on 64-bit targets (<see cref="SliceBuffer{T}"/>).
/// Allows <c>CollectionYield</c> <c>for</c> expressions to write yielded elements directly into
/// contiguous unmanaged/pinned memory without allocating a managed <see cref="System.Collections.Generic.List{T}"/>
/// or GC header across FFI and shared-memory boundaries.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
public unsafe struct SliceBuffer<T> where T : unmanaged
{
    /// <summary>Offset 0: Raw pointer to contiguous T elements.</summary>
    public T* DataPtr;

    /// <summary>Offset 8: Number of elements yielded so far.</summary>
    public nuint Length;

    /// <summary>Offset 16: Allocated element capacity.</summary>
    public nuint Capacity;

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public SliceBuffer(T* dataPtr, nuint capacity, nuint length = 0)
    {
        DataPtr = dataPtr;
        Length = length;
        Capacity = capacity;
    }

    /// <summary>
    /// Appends a yielded element in-place at <c>DataPtr[Length]</c> with zero marshalling.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public bool TryYield(T value)
    {
        if (DataPtr == null || Length >= Capacity)
            return false;
        DataPtr[Length++] = value;
        return true;
    }

    /// <summary>
    /// Appends a yielded element in-place, growing the unmanaged buffer if owned/resizable.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void YieldUnchecked(T value)
    {
        DataPtr[Length++] = value;
    }

    /// <summary>
    /// Exposes the yielded elements as a zero-copy <see cref="Span{T}"/>.
    /// </summary>
    public Span<T> AsSpan() => DataPtr == null ? Span<T>.Empty : new Span<T>(DataPtr, checked((int)Length));

    /// <summary>
    /// Exposes the full backing capacity as a zero-copy <see cref="Span{T}"/>.
    /// </summary>
    public Span<T> CapacitySpan() => DataPtr == null ? Span<T>.Empty : new Span<T>(DataPtr, checked((int)Capacity));

    /// <summary>
    /// Allocates a new unmanaged <see cref="SliceBuffer{T}"/> via <see cref="NativeMemory.Alloc(nuint, nuint)"/>.
    /// </summary>
    public static SliceBuffer<T> AllocateNative(nuint capacity)
    {
        if (capacity == 0)
            return new SliceBuffer<T>(null, 0, 0);
        T* ptr = (T*)System.Runtime.InteropServices.NativeMemory.Alloc(capacity, (nuint)sizeof(T));
        return new SliceBuffer<T>(ptr, capacity, 0);
    }

    /// <summary>
    /// Frees an unmanaged <see cref="SliceBuffer{T}"/> previously allocated via <see cref="AllocateNative"/>.
    /// </summary>
    public void FreeNative()
    {
        if (DataPtr != null)
        {
            System.Runtime.InteropServices.NativeMemory.Free(DataPtr);
            DataPtr = null;
            Length = 0;
            Capacity = 0;
        }
    }
}
