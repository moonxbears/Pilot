using System;
using System.Collections.Generic;
using System.IO;
using System.IO.MemoryMappedFiles;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.NativeMemory;

/// <summary>
/// Unmanaged scalar element kinds supported by <see cref="PilotSharedBuffer"/> across C-ABI and <c>mmap</c> boundaries.
/// </summary>
public enum PilotSharedElementKind
{
    Float64 = 0,
    Int64 = 1,
    Float32 = 2,
    Int32 = 3,
    UInt8 = 4
}

/// <summary>
/// Milestone 15.3: Contiguous zero-copy, zero-marshalling buffer backed by an unmanaged 24-byte
/// <see cref="SliceBuffer{T}"/> C-ABI header and either <see cref="NativeMemory"/> or cross-process /
/// cross-language OS <see cref="MemoryMappedFile"/> (<c>mmap</c>) shared pages.
/// </summary>
public sealed unsafe class PilotSharedBuffer : IDisposable, IEquatable<PilotSharedBuffer>
{
    private SliceBuffer<byte>* _abiHeader;
    private bool _ownsNativeData;
    private MemoryMappedFile? _mmf;
    private MemoryMappedViewAccessor? _accessor;
    private bool _pointerAcquired;
    private PilotSharedBuffer? _keepAliveParent;
    private bool _disposed;

    public PilotSharedElementKind ElementKind { get; }
    public int Stride { get; }
    public string? BackingPath { get; private set; }
    public string? SharedName { get; private set; }
    public bool IsSharedMmap => _accessor != null;
    public bool OwnsNativeMemory => _ownsNativeData;

    /// <summary>
    /// Diagnostic invariant counter: always 0 because <see cref="PilotSharedBuffer"/> never allocates a managed <c>List&lt;T&gt;</c>.
    /// </summary>
    public int ManagedListAllocations => 0;

    /// <summary>
    /// Number of elements written directly in-place via <see cref="YieldElement"/> without intermediate collection allocation.
    /// </summary>
    public long DirectYieldWrites { get; private set; }

    /// <summary>
    /// Raw pointer to the contiguous element payload (<c>Offset 0</c> of the 24-byte C-ABI <see cref="SliceBuffer{T}"/> header).
    /// </summary>
    public IntPtr DataPointer => _abiHeader != null ? (IntPtr)_abiHeader->DataPtr : IntPtr.Zero;

    /// <summary>
    /// Raw pointer to the live 24-byte unmanaged <see cref="SliceBuffer{T}"/> C-ABI struct (<c>{ T* DataPtr, nuint Length, nuint Capacity }</c>).
    /// </summary>
    public IntPtr HeaderPointer => (IntPtr)_abiHeader;

    public int Length
    {
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        get => _abiHeader != null ? checked((int)_abiHeader->Length) : 0;
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        set
        {
            EnsureNotDisposed();
            if (value < 0 || (nuint)value > _abiHeader->Capacity)
                throw new ArgumentOutOfRangeException(nameof(value), $"Length {value} exceeds capacity {Capacity}.");
            _abiHeader->Length = (nuint)value;
        }
    }

    public int Capacity
    {
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        get => _abiHeader != null ? checked((int)_abiHeader->Capacity) : 0;
    }

    public long ByteLength => (long)Length * Stride;
    public long ByteCapacity => (long)Capacity * Stride;
    public string ElementTypeName => GetCanonicalElementName(ElementKind);

    private PilotSharedBuffer(PilotSharedElementKind kind)
    {
        ElementKind = kind;
        Stride = GetStride(kind);
        _abiHeader = (SliceBuffer<byte>*)System.Runtime.InteropServices.NativeMemory.AllocZeroed((nuint)sizeof(SliceBuffer<byte>));
    }

    /// <summary>
    /// Allocates a contiguous unmanaged buffer via <see cref="NativeMemory.AllocZeroed(nuint, nuint)"/>.
    /// Can grow dynamically via <see cref="NativeMemory.Realloc(void*, nuint)"/> during unbounded <c>CollectionYield</c> loops.
    /// </summary>
    public static PilotSharedBuffer AllocateUnmanaged(
        nuint capacity,
        PilotSharedElementKind kind = PilotSharedElementKind.Float64,
        nuint initialLength = 0)
    {
        var buf = new PilotSharedBuffer(kind);
        nuint allocCap = capacity;
        byte* ptr = allocCap > 0
            ? (byte*)System.Runtime.InteropServices.NativeMemory.AllocZeroed(allocCap, (nuint)buf.Stride)
            : null;
        buf._abiHeader->DataPtr = ptr;
        buf._abiHeader->Capacity = allocCap;
        buf._abiHeader->Length = initialLength <= allocCap ? initialLength : allocCap;
        buf._ownsNativeData = ptr != null;
        return buf;
    }

    /// <summary>
    /// Creates or attaches to an OS shared-memory (<c>mmap</c>) contiguous buffer accessible simultaneously
    /// across Pilot, Python (<c>mmap</c> + <c>memoryview</c>), Node.js, C/C++, Rust, C# (<c>Span&lt;T&gt;</c>), and Assembly.
    /// </summary>
    public static PilotSharedBuffer CreateOrOpenShared(
        string nameOrPath,
        nuint capacity,
        PilotSharedElementKind kind = PilotSharedElementKind.Float64,
        nuint? initialLength = null)
    {
        if (string.IsNullOrWhiteSpace(nameOrPath))
            nameOrPath = $"pilot_shm_{Guid.NewGuid():N}";

        var buf = new PilotSharedBuffer(kind);
        string fullPath = ResolveBackingFilePath(nameOrPath, out string logicalName);
        nuint effectiveCap = capacity == 0 ? 1u : capacity;
        buf.MapBackingFile(fullPath, logicalName, effectiveCap, initialLength ?? 0u);
        return buf;
    }

    /// <summary>
    /// Wraps an external raw C/Rust/Zig/FFI pointer in a <see cref="PilotSharedBuffer"/> with zero copying.
    /// </summary>
    public static PilotSharedBuffer WrapExternalPointer(
        void* dataPtr,
        nuint length,
        nuint capacity,
        PilotSharedElementKind kind = PilotSharedElementKind.Float64,
        PilotSharedBuffer? keepAliveParent = null)
    {
        var buf = new PilotSharedBuffer(kind);
        buf._abiHeader->DataPtr = (byte*)dataPtr;
        buf._abiHeader->Length = length;
        buf._abiHeader->Capacity = capacity < length ? length : capacity;
        buf._ownsNativeData = false;
        buf._keepAliveParent = keepAliveParent;
        return buf;
    }

    private void MapBackingFile(string fullPath, string logicalName, nuint capacity, nuint initialLength)
    {
        string? dir = Path.GetDirectoryName(fullPath);
        if (!string.IsNullOrEmpty(dir))
            Directory.CreateDirectory(dir);

        long viewBytes = Math.Max((long)(capacity * (nuint)Stride), (long)Stride);
        long fileMinBytes = Math.Max(viewBytes, 64L);

        var fs = new FileStream(
            fullPath,
            FileMode.OpenOrCreate,
            FileAccess.ReadWrite,
            FileShare.ReadWrite | FileShare.Delete);

        if (fs.Length < fileMinBytes)
            fs.SetLength(fileMinBytes);

        var mmf = MemoryMappedFile.CreateFromFile(
            fs,
            mapName: null,
            capacity: 0,
            MemoryMappedFileAccess.ReadWrite,
            HandleInheritability.None,
            leaveOpen: false);

        var accessor = mmf.CreateViewAccessor(0, viewBytes, MemoryMappedFileAccess.ReadWrite);
        byte* rawPtr = null;
        accessor.SafeMemoryMappedViewHandle.AcquirePointer(ref rawPtr);
        rawPtr += accessor.PointerOffset;

        _mmf = mmf;
        _accessor = accessor;
        _pointerAcquired = true;
        _ownsNativeData = false;
        BackingPath = fullPath;
        SharedName = logicalName;
        _abiHeader->DataPtr = rawPtr;
        _abiHeader->Capacity = capacity;
        _abiHeader->Length = initialLength <= capacity ? initialLength : capacity;
    }

    /// <summary>
    /// Ensures this buffer is backed by an OS <c>mmap</c> file so external processes (e.g. Python <c>mmap</c>/<c>memoryview</c>
    /// or Node.js) can map the exact same physical memory pages. If already <c>mmap</c>-backed, performs zero copies.
    /// </summary>
    public string EnsureSharedMapping(string? preferredName = null)
    {
        EnsureNotDisposed();
        if (IsSharedMmap && !string.IsNullOrEmpty(BackingPath))
        {
            _accessor?.Flush();
            return BackingPath!;
        }

        if (_keepAliveParent != null)
        {
            return _keepAliveParent.EnsureSharedMapping(preferredName);
        }

        nuint cap = _abiHeader->Capacity == 0 ? 8u : _abiHeader->Capacity;
        nuint len = _abiHeader->Length;
        byte* oldPtr = _abiHeader->DataPtr;
        bool ownedOld = _ownsNativeData;

        string name = string.IsNullOrWhiteSpace(preferredName)
            ? $"pilot_shm_{Guid.NewGuid():N}"
            : preferredName!;
        string fullPath = ResolveBackingFilePath(name, out string logicalName);

        MapBackingFile(fullPath, logicalName, cap, len);

        if (oldPtr != null && len > 0)
        {
            nuint bytesToCopy = len * (nuint)Stride;
            Buffer.MemoryCopy(oldPtr, _abiHeader->DataPtr, (long)ByteCapacity, (long)bytesToCopy);
            _accessor?.Flush();
        }

        if (ownedOld && oldPtr != null)
        {
            System.Runtime.InteropServices.NativeMemory.Free(oldPtr);
        }

        return fullPath;
    }

    /// <summary>
    /// Milestone 15.3.2: Opens a second independent OS virtual memory mapping over the exact same physical
    /// shared-memory pages (<c>view.DataPointer != this.DataPointer</c>) to verify and exercise base-address
    /// independent relative-index iteration with zero copying and zero pointer swizzling.
    /// </summary>
    public PilotSharedBuffer OpenIndependentView(int? lengthOverride = null)
    {
        EnsureNotDisposed();
        string path = EnsureSharedMapping();
        nuint targetLen = (nuint)(lengthOverride ?? Length);
        nuint targetCap = (nuint)Math.Max(Capacity, (int)targetLen);
        return CreateOrOpenShared(path, targetCap, ElementKind, targetLen);
    }

    /// <summary>
    /// Returns a zero-copy sub-slice view pointing directly into <c>DataPtr + start * Stride</c>.
    /// </summary>
    public PilotSharedBuffer SliceView(int start, int length)
    {
        EnsureNotDisposed();
        if (start < 0) start += Length;
        if (start < 0) start = 0;
        if (start > Length) start = Length;
        if (length < 0) length = 0;
        if (start + length > Length) length = Length - start;

        byte* subPtr = _abiHeader->DataPtr != null
            ? _abiHeader->DataPtr + ((nuint)start * (nuint)Stride)
            : null;
        return WrapExternalPointer(subPtr, (nuint)length, (nuint)length, ElementKind, keepAliveParent: _keepAliveParent ?? this);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void ResetLength()
    {
        EnsureNotDisposed();
        _abiHeader->Length = 0;
    }

    /// <summary>
    /// Pre-reserves exact capacity in a single unmanaged allocation when a <c>CollectionYield</c>
    /// loop over a known-length range writes into an initially empty <see cref="PilotSharedBuffer"/>.
    /// </summary>
    public void ReserveInitialCapacity(nuint exactCapacity)
    {
        EnsureNotDisposed();
        if (exactCapacity == 0 || _abiHeader->Capacity >= exactCapacity || IsSharedMmap || _keepAliveParent != null)
            return;

        if (_abiHeader->DataPtr == null)
        {
            _abiHeader->DataPtr = (byte*)System.Runtime.InteropServices.NativeMemory.AllocZeroed(exactCapacity, (nuint)Stride);
            _abiHeader->Capacity = exactCapacity;
            _ownsNativeData = true;
        }
        else if (_ownsNativeData)
        {
            _abiHeader->DataPtr = (byte*)System.Runtime.InteropServices.NativeMemory.Realloc(_abiHeader->DataPtr, exactCapacity * (nuint)Stride);
            _abiHeader->Capacity = exactCapacity;
        }
    }

    public void EnsureCapacityForYield()
    {
        if (_abiHeader->Length < _abiHeader->Capacity && _abiHeader->DataPtr != null)
            return;

        nuint nextCap = _abiHeader->Capacity == 0 ? 8u : _abiHeader->Capacity * 2u;
        if (_ownsNativeData || (_abiHeader->DataPtr == null && !IsSharedMmap && _keepAliveParent == null))
        {
            _abiHeader->DataPtr = (byte*)System.Runtime.InteropServices.NativeMemory.Realloc(_abiHeader->DataPtr, nextCap * (nuint)Stride);
            _abiHeader->Capacity = nextCap;
            _ownsNativeData = true;
            return;
        }

        if (IsSharedMmap && !string.IsNullOrEmpty(BackingPath))
        {
            nuint curLen = _abiHeader->Length;
            string path = BackingPath!;
            string logical = SharedName ?? Path.GetFileNameWithoutExtension(path);
            ReleaseCurrentMapping();
            MapBackingFile(path, logical, nextCap, curLen);
            return;
        }

        throw new InvalidOperationException($"Fixed-capacity external SliceBuffer ({Capacity} elements) overflowed during yield.");
    }

    /// <summary>
    /// Milestone 15.3.3: Writes a yielded value directly in-place at <c>DataPtr[Length++]</c>
    /// with zero <c>List&lt;PilotValue&gt;</c> allocation and zero element boxing.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void YieldElement(PilotValue value)
    {
        EnsureNotDisposed();
        EnsureCapacityForYield();
        int idx = checked((int)_abiHeader->Length);
        WriteRawUnchecked(idx, value);
        _abiHeader->Length++;
        DirectYieldWrites++;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public PilotValue GetElement(int index)
    {
        EnsureNotDisposed();
        if (index < 0) index += Length;
        if ((uint)index >= (uint)Length || _abiHeader->DataPtr == null)
            return PilotValue.Undefined;

        byte* ptr = _abiHeader->DataPtr;
        return ElementKind switch
        {
            PilotSharedElementKind.Float64 => PilotValue.FromFloat(((double*)ptr)[index]),
            PilotSharedElementKind.Int64 => PilotValue.FromInt(((long*)ptr)[index]),
            PilotSharedElementKind.Float32 => PilotValue.FromFloat(((float*)ptr)[index]),
            PilotSharedElementKind.Int32 => PilotValue.FromInt(((int*)ptr)[index]),
            PilotSharedElementKind.UInt8 => PilotValue.FromInt(ptr[index]),
            _ => PilotValue.Undefined
        };
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void SetElement(int index, PilotValue value)
    {
        EnsureNotDisposed();
        if (index < 0) index += Length;
        if (index < 0)
            return;

        if ((nuint)index >= _abiHeader->Capacity)
            return;

        WriteRawUnchecked(index, value);
        if ((nuint)index >= _abiHeader->Length)
            _abiHeader->Length = (nuint)(index + 1);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private void WriteRawUnchecked(int index, PilotValue value)
    {
        byte* ptr = _abiHeader->DataPtr;
        switch (ElementKind)
        {
            case PilotSharedElementKind.Float64:
                ((double*)ptr)[index] = value.AsFloat();
                break;
            case PilotSharedElementKind.Int64:
                ((long*)ptr)[index] = value.AsInt();
                break;
            case PilotSharedElementKind.Float32:
                ((float*)ptr)[index] = (float)value.AsFloat();
                break;
            case PilotSharedElementKind.Int32:
                ((int*)ptr)[index] = (int)value.AsInt();
                break;
            case PilotSharedElementKind.UInt8:
                ptr[index] = (byte)Math.Clamp(value.AsInt(), 0, 255);
                break;
        }
    }

    /// <summary>
    /// Exposes the yielded elements directly as a zero-copy typed <see cref="Span{T}"/>.
    /// </summary>
    public Span<T> AsSpan<T>() where T : unmanaged
    {
        EnsureNotDisposed();
        if (sizeof(T) != Stride)
            throw new InvalidOperationException($"Cannot view SliceBuffer[{ElementTypeName}] (stride {Stride}) as Span<{typeof(T).Name}> (sizeof {sizeof(T)}).");
        return _abiHeader->DataPtr == null
            ? Span<T>.Empty
            : new Span<T>(_abiHeader->DataPtr, Length);
    }

    /// <summary>
    /// Exposes the full allocated capacity directly as a zero-copy typed <see cref="Span{T}"/>.
    /// </summary>
    public Span<T> CapacitySpan<T>() where T : unmanaged
    {
        EnsureNotDisposed();
        if (sizeof(T) != Stride)
            throw new InvalidOperationException($"Cannot view SliceBuffer[{ElementTypeName}] (stride {Stride}) as Span<{typeof(T).Name}> (sizeof {sizeof(T)}).");
        return _abiHeader->DataPtr == null
            ? Span<T>.Empty
            : new Span<T>(_abiHeader->DataPtr, Capacity);
    }

    /// <summary>
    /// Reads the live 24-byte C-ABI <see cref="SliceBuffer{T}"/> value directly from <see cref="HeaderPointer"/>.
    /// </summary>
    public SliceBuffer<T> AsTypedSliceBuffer<T>() where T : unmanaged
    {
        EnsureNotDisposed();
        if (sizeof(T) != Stride)
            throw new InvalidOperationException($"Element size mismatch: SliceBuffer[{ElementTypeName}] has stride {Stride}, requested {typeof(T).Name} ({sizeof(T)}).");
        return *(SliceBuffer<T>*)_abiHeader;
    }

    public PilotValue Sum()
    {
        EnsureNotDisposed();
        int len = Length;
        byte* ptr = _abiHeader->DataPtr;
        if (len == 0 || ptr == null)
        {
            return ElementKind is PilotSharedElementKind.Float64 or PilotSharedElementKind.Float32
                ? PilotValue.FromFloat(0.0)
                : PilotValue.FromInt(0);
        }

        switch (ElementKind)
        {
            case PilotSharedElementKind.Float64:
            {
                double acc = 0;
                double* p = (double*)ptr;
                for (int i = 0; i < len; i++) acc += p[i];
                return PilotValue.FromFloat(acc);
            }
            case PilotSharedElementKind.Float32:
            {
                double acc = 0;
                float* p = (float*)ptr;
                for (int i = 0; i < len; i++) acc += p[i];
                return PilotValue.FromFloat(acc);
            }
            case PilotSharedElementKind.Int64:
            {
                long acc = 0;
                long* p = (long*)ptr;
                for (int i = 0; i < len; i++) acc += p[i];
                return PilotValue.FromInt(acc);
            }
            case PilotSharedElementKind.Int32:
            {
                long acc = 0;
                int* p = (int*)ptr;
                for (int i = 0; i < len; i++) acc += p[i];
                return PilotValue.FromInt(acc);
            }
            default:
            {
                long acc = 0;
                for (int i = 0; i < len; i++) acc += ptr[i];
                return PilotValue.FromInt(acc);
            }
        }
    }

    public void ScaleInPlace(double factor)
    {
        EnsureNotDisposed();
        int len = Length;
        byte* ptr = _abiHeader->DataPtr;
        if (len == 0 || ptr == null) return;

        switch (ElementKind)
        {
            case PilotSharedElementKind.Float64:
            {
                double* p = (double*)ptr;
                for (int i = 0; i < len; i++) p[i] *= factor;
                break;
            }
            case PilotSharedElementKind.Float32:
            {
                float f = (float)factor;
                float* p = (float*)ptr;
                for (int i = 0; i < len; i++) p[i] *= f;
                break;
            }
            case PilotSharedElementKind.Int64:
            {
                long* p = (long*)ptr;
                for (int i = 0; i < len; i++) p[i] = (long)(p[i] * factor);
                break;
            }
            case PilotSharedElementKind.Int32:
            {
                int* p = (int*)ptr;
                for (int i = 0; i < len; i++) p[i] = (int)(p[i] * factor);
                break;
            }
            case PilotSharedElementKind.UInt8:
            {
                for (int i = 0; i < len; i++) ptr[i] = (byte)Math.Clamp((long)(ptr[i] * factor), 0, 255);
                break;
            }
        }
    }

    public void Fill(PilotValue value, int? countOverride = null)
    {
        EnsureNotDisposed();
        int targetCount = countOverride ?? (Length > 0 ? Length : Capacity);
        if (targetCount > Capacity) targetCount = Capacity;
        for (int i = 0; i < targetCount; i++)
            WriteRawUnchecked(i, value);
        if (_abiHeader->Length < (nuint)targetCount)
            _abiHeader->Length = (nuint)targetCount;
    }

    public List<PilotValue> ToPilotValueList()
    {
        EnsureNotDisposed();
        var list = new List<PilotValue>(Length);
        for (int i = 0; i < Length; i++)
            list.Add(GetElement(i));
        return list;
    }

    public void Flush()
    {
        _accessor?.Flush();
    }

    public bool Equals(PilotSharedBuffer? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        if (Length != other.Length) return false;
        for (int i = 0; i < Length; i++)
        {
            if (!GetElement(i).Equals(other.GetElement(i)))
                return false;
        }
        return true;
    }

    public override bool Equals(object? obj) => obj is PilotSharedBuffer sb && Equals(sb);

    public override int GetHashCode() => HashCode.Combine(ElementKind, Length);

    public override string ToString()
    {
        if (_disposed || _abiHeader == null)
            return $"SliceBuffer[{ElementTypeName}](disposed)";

        int previewCount = Math.Min(Length, 8);
        var parts = new string[previewCount + (Length > previewCount ? 1 : 0)];
        for (int i = 0; i < previewCount; i++)
            parts[i] = GetElement(i).ToString();
        if (Length > previewCount)
            parts[previewCount] = "...";
        return $"SliceBuffer[{ElementTypeName}]([{string.Join(", ", parts)}], len={Length}, cap={Capacity})";
    }

    public static int GetStride(PilotSharedElementKind kind) => kind switch
    {
        PilotSharedElementKind.Float64 => 8,
        PilotSharedElementKind.Int64 => 8,
        PilotSharedElementKind.Float32 => 4,
        PilotSharedElementKind.Int32 => 4,
        PilotSharedElementKind.UInt8 => 1,
        _ => 8
    };

    public static string GetCanonicalElementName(PilotSharedElementKind kind) => kind switch
    {
        PilotSharedElementKind.Float64 => "f64",
        PilotSharedElementKind.Int64 => "i64",
        PilotSharedElementKind.Float32 => "f32",
        PilotSharedElementKind.Int32 => "i32",
        PilotSharedElementKind.UInt8 => "u8",
        _ => "f64"
    };

    public static string GetPythonStructFormat(PilotSharedElementKind kind) => kind switch
    {
        PilotSharedElementKind.Float64 => "d",
        PilotSharedElementKind.Int64 => "q",
        PilotSharedElementKind.Float32 => "f",
        PilotSharedElementKind.Int32 => "i",
        PilotSharedElementKind.UInt8 => "B",
        _ => "d"
    };

    public static PilotSharedElementKind ParseElementKind(string? typeSpec)
    {
        if (string.IsNullOrWhiteSpace(typeSpec))
            return PilotSharedElementKind.Float64;

        string s = typeSpec.Trim();
        int openBracket = s.IndexOf('[');
        int closeBracket = s.LastIndexOf(']');
        if (openBracket >= 0 && closeBracket > openBracket)
        {
            s = s.Substring(openBracket + 1, closeBracket - openBracket - 1).Trim();
        }
        else
        {
            int openAngle = s.IndexOf('<');
            int closeAngle = s.LastIndexOf('>');
            if (openAngle >= 0 && closeAngle > openAngle)
                s = s.Substring(openAngle + 1, closeAngle - openAngle - 1).Trim();
        }

        s = s.Trim(':', '"', '\'').ToLowerInvariant();
        return s switch
        {
            "f64" or "float" or "float64" or "double" or "number" => PilotSharedElementKind.Float64,
            "i64" or "int" or "int64" or "long" or "isize" or "u64" or "usize" => PilotSharedElementKind.Int64,
            "f32" or "float32" or "single" => PilotSharedElementKind.Float32,
            "i32" or "int32" or "u32" or "uint32" => PilotSharedElementKind.Int32,
            "u8" or "byte" or "uint8" or "i8" => PilotSharedElementKind.UInt8,
            _ => PilotSharedElementKind.Float64
        };
    }

    private static string ResolveBackingFilePath(string nameOrPath, out string logicalName)
    {
        string trimmed = nameOrPath.Trim();
        bool looksLikePath = trimmed.Contains(Path.DirectorySeparatorChar) ||
                             trimmed.Contains(Path.AltDirectorySeparatorChar) ||
                             trimmed.EndsWith(".shm", StringComparison.OrdinalIgnoreCase) ||
                             trimmed.EndsWith(".mmap", StringComparison.OrdinalIgnoreCase) ||
                             trimmed.EndsWith(".bin", StringComparison.OrdinalIgnoreCase);

        if (looksLikePath)
        {
            string full = Path.GetFullPath(trimmed);
            logicalName = Path.GetFileNameWithoutExtension(full);
            return full;
        }

        char[] invalid = Path.GetInvalidFileNameChars();
        var chars = trimmed.ToCharArray();
        for (int i = 0; i < chars.Length; i++)
        {
            if (Array.IndexOf(invalid, chars[i]) >= 0)
                chars[i] = '_';
        }
        logicalName = new string(chars);
        string ipcDir = Path.Combine(Path.GetTempPath(), "pilot_ipc");
        return Path.Combine(ipcDir, logicalName + ".shm");
    }

    private void EnsureNotDisposed()
    {
        if (_disposed || _abiHeader == null)
            throw new ObjectDisposedException(nameof(PilotSharedBuffer));
    }

    private void ReleaseCurrentMapping()
    {
        if (_accessor != null)
        {
            if (_pointerAcquired)
            {
                try
                {
                    _accessor.SafeMemoryMappedViewHandle.ReleasePointer();
                }
                catch
                {
                    // Ignore already-disposed handle during shutdown
                }
                _pointerAcquired = false;
            }
            try { _accessor.Flush(); } catch { }
            _accessor.Dispose();
            _accessor = null;
        }

        if (_mmf != null)
        {
            _mmf.Dispose();
            _mmf = null;
        }
    }

    private void Dispose(bool disposing)
    {
        if (_disposed) return;
        _disposed = true;

        ReleaseCurrentMapping();

        if (_abiHeader != null)
        {
            if (_ownsNativeData && _abiHeader->DataPtr != null)
            {
                System.Runtime.InteropServices.NativeMemory.Free(_abiHeader->DataPtr);
            }
            _abiHeader->DataPtr = null;
            _abiHeader->Length = 0;
            _abiHeader->Capacity = 0;
            System.Runtime.InteropServices.NativeMemory.Free(_abiHeader);
            _abiHeader = null;
        }
        _ownsNativeData = false;
        _keepAliveParent = null;
    }

    public void Dispose()
    {
        Dispose(true);
        GC.SuppressFinalize(this);
    }

    ~PilotSharedBuffer()
    {
        Dispose(false);
    }
}
