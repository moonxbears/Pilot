using System;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using Pilot.Runtime.NativeMemory;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.CppBackend;

/// <summary>
/// Manages Pilot's C++20 native shared runtime (<c>libpilot_rt</c>) and content-cached C++20 JIT
/// shared-object compilation for <c>@burst</c> functions.
/// Uses unmanaged <c>delegate* unmanaged[Cdecl]</c> function pointers for zero-marshalling dispatch.
/// </summary>
public static unsafe class PilotCppBackend
{
    private static readonly object InitLock = new();
    private static readonly object CompileLock = new();
    private static readonly ConcurrentDictionary<string, IntPtr> KernelSymbolCache = new(StringComparer.Ordinal);

    private static bool _initialized;
    private static bool _available;
    private static IntPtr _libHandle;
    private static string? _libraryPath;
    private static string? _compilerExecutable;
    private static int _abiVersion;
    private static int _simdWidthBits;
    private static long _nativeDispatchCalls;
    private static long _burstCompiledKernels;
    private static long _burstCacheHits;

    // Unmanaged C-ABI function pointers into `libpilot_rt`
    private static delegate* unmanaged[Cdecl]<int> _fnVersion;
    private static delegate* unmanaged[Cdecl]<int> _fnSimdWidth;
    private static delegate* unmanaged[Cdecl]<double*, nuint, double> _fnSliceSumF64;
    private static delegate* unmanaged[Cdecl]<long*, nuint, long> _fnSliceSumI64;
    private static delegate* unmanaged[Cdecl]<double*, double*, nuint, double> _fnSliceDotF64;
    private static delegate* unmanaged[Cdecl]<double, double*, double*, nuint, void> _fnSliceSaxpyF64;
    private static delegate* unmanaged[Cdecl]<double, double*, nuint, void> _fnSliceScaleF64;
    private static delegate* unmanaged[Cdecl]<double*, nuint, double, void> _fnSliceFillF64;
    private static delegate* unmanaged[Cdecl]<long*, nuint, long, void> _fnSliceFillI64;
    private static delegate* unmanaged[Cdecl]<double*, nuint, void> _fnSliceSortF64;
    private static delegate* unmanaged[Cdecl]<long*, nuint, void> _fnSliceSortI64;
    private static delegate* unmanaged[Cdecl]<double*, nuint, double*, double*, double*, double*, void> _fnWelfordF64;
    private static delegate* unmanaged[Cdecl]<double*, double*, double, double, nuint, void> _fnIntegrateVectorsF64;
    private static delegate* unmanaged[Cdecl]<int*, byte*, int, int, int, int, ushort*, int, int, int*, int*, int> _fnDfaFindUtf16;
    private static delegate* unmanaged[Cdecl]<int*, byte*, int, int, int, int, byte*, int, int, int*, int*, int> _fnDfaFindUtf8;

    public static bool IsAvailable
    {
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        get
        {
            if (!_initialized) EnsureInitialized();
            return _available;
        }
    }

    public static string? LibraryPath
    {
        get { EnsureInitialized(); return _libraryPath; }
    }

    public static string? CompilerExecutable => ProbeCppCompiler();

    public static int AbiVersion
    {
        get { EnsureInitialized(); return _abiVersion; }
    }

    public static int SimdWidthBits
    {
        get { EnsureInitialized(); return _simdWidthBits; }
    }

    public static long NativeDispatchCalls => Interlocked.Read(ref _nativeDispatchCalls);
    public static long BurstCompiledKernels => Interlocked.Read(ref _burstCompiledKernels);
    public static long BurstCacheHits => Interlocked.Read(ref _burstCacheHits);

    public static string SharedLibraryExtension =>
        RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? ".dll" :
        RuntimeInformation.IsOSPlatform(OSPlatform.OSX) ? ".dylib" : ".so";

    public static string SharedLibraryFileName =>
        RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? "pilot_rt.dll" :
        RuntimeInformation.IsOSPlatform(OSPlatform.OSX) ? "libpilot_rt.dylib" : "libpilot_rt.so";

    public static void EnsureInitialized()
    {
        if (_initialized) return;

        lock (InitLock)
        {
            if (_initialized) return;

            try
            {
                string? candidatePath = ResolveOrBuildRuntimeLibrary();
                if (!string.IsNullOrEmpty(candidatePath) && File.Exists(candidatePath))
                {
                    IntPtr handle = NativeLibrary.Load(candidatePath);
                    BindExports(handle);
                    _libHandle = handle;
                    _libraryPath = candidatePath;
                    _abiVersion = _fnVersion();
                    _simdWidthBits = _fnSimdWidth();
                    _available = _abiVersion >= 2101;
                }
            }
            catch
            {
                _available = false;
            }
            finally
            {
                _initialized = true;
            }
        }
    }

    private static void BindExports(IntPtr handle)
    {
        _fnVersion = (delegate* unmanaged[Cdecl]<int>)NativeLibrary.GetExport(handle, "pilot_rt_version");
        _fnSimdWidth = (delegate* unmanaged[Cdecl]<int>)NativeLibrary.GetExport(handle, "pilot_rt_simd_width_bits");
        _fnSliceSumF64 = (delegate* unmanaged[Cdecl]<double*, nuint, double>)NativeLibrary.GetExport(handle, "pilot_rt_slice_sum_f64");
        _fnSliceSumI64 = (delegate* unmanaged[Cdecl]<long*, nuint, long>)NativeLibrary.GetExport(handle, "pilot_rt_slice_sum_i64");
        _fnSliceDotF64 = (delegate* unmanaged[Cdecl]<double*, double*, nuint, double>)NativeLibrary.GetExport(handle, "pilot_rt_slice_dot_f64");
        _fnSliceSaxpyF64 = (delegate* unmanaged[Cdecl]<double, double*, double*, nuint, void>)NativeLibrary.GetExport(handle, "pilot_rt_slice_saxpy_f64");
        _fnSliceScaleF64 = (delegate* unmanaged[Cdecl]<double, double*, nuint, void>)NativeLibrary.GetExport(handle, "pilot_rt_slice_scale_f64");
        _fnSliceFillF64 = (delegate* unmanaged[Cdecl]<double*, nuint, double, void>)NativeLibrary.GetExport(handle, "pilot_rt_slice_fill_f64");
        _fnSliceFillI64 = (delegate* unmanaged[Cdecl]<long*, nuint, long, void>)NativeLibrary.GetExport(handle, "pilot_rt_slice_fill_i64");
        _fnSliceSortF64 = (delegate* unmanaged[Cdecl]<double*, nuint, void>)NativeLibrary.GetExport(handle, "pilot_rt_slice_sort_f64");
        _fnSliceSortI64 = (delegate* unmanaged[Cdecl]<long*, nuint, void>)NativeLibrary.GetExport(handle, "pilot_rt_slice_sort_i64");
        _fnWelfordF64 = (delegate* unmanaged[Cdecl]<double*, nuint, double*, double*, double*, double*, void>)NativeLibrary.GetExport(handle, "pilot_rt_welford_f64");
        _fnIntegrateVectorsF64 = (delegate* unmanaged[Cdecl]<double*, double*, double, double, nuint, void>)NativeLibrary.GetExport(handle, "pilot_rt_integrate_vectors_f64");
        _fnDfaFindUtf16 = (delegate* unmanaged[Cdecl]<int*, byte*, int, int, int, int, ushort*, int, int, int*, int*, int>)NativeLibrary.GetExport(handle, "pilot_rt_dfa_find_utf16");
        _fnDfaFindUtf8 = (delegate* unmanaged[Cdecl]<int*, byte*, int, int, int, int, byte*, int, int, int*, int*, int>)NativeLibrary.GetExport(handle, "pilot_rt_dfa_find_utf8");
    }

    public static string GetNativeCacheDirectory()
    {
        string baseDir = Path.Combine(Path.GetTempPath(), "pilot_cpp_backend_cache");
        Directory.CreateDirectory(baseDir);
        return baseDir;
    }

    public static string? LocateRuntimeSourceDirectory()
    {
        string[] probeRoots =
        {
            Directory.GetCurrentDirectory(),
            AppContext.BaseDirectory
        };

        foreach (string start in probeRoots)
        {
            if (string.IsNullOrWhiteSpace(start)) continue;
            var dir = new DirectoryInfo(start);
            while (dir != null)
            {
                string rtDir = Path.Combine(dir.FullName, "Native", "Runtime");
                if (File.Exists(Path.Combine(rtDir, "pilot_runtime.cpp")) &&
                    File.Exists(Path.Combine(rtDir, "pilot_runtime.h")))
                {
                    return rtDir;
                }
                dir = dir.Parent;
            }
        }
        return null;
    }

    private static string? ResolveOrBuildRuntimeLibrary()
    {
        // 1. Check build-time precompiled native binary alongside Pilot.dll (`<OutDir>/native/libpilot_rt.*`)
        string prebuilt = Path.Combine(AppContext.BaseDirectory, "native", SharedLibraryFileName);
        if (File.Exists(prebuilt))
        {
            return prebuilt;
        }

        // 2. Compile on-demand from `Native/Runtime/pilot_runtime.cpp` into content-addressed cache
        string? rtSourceDir = LocateRuntimeSourceDirectory();
        if (rtSourceDir == null) return null;

        string cppPath = Path.Combine(rtSourceDir, "pilot_runtime.cpp");
        string hPath = Path.Combine(rtSourceDir, "pilot_runtime.h");
        string combined = File.ReadAllText(cppPath, Encoding.UTF8) + "\n//---\n" + File.ReadAllText(hPath, Encoding.UTF8);
        string hash = ComputeSha256Short(combined + "|" + RuntimeInformation.ProcessArchitecture);

        string cachedSo = Path.Combine(GetNativeCacheDirectory(), $"pilot_rt_{hash}{SharedLibraryExtension}");
        if (File.Exists(cachedSo))
        {
            return cachedSo;
        }

        if (CompileCppFileToSharedLibrary(cppPath, rtSourceDir, cachedSo, out _))
        {
            return cachedSo;
        }

        return null;
    }

    public static string? ProbeCppCompiler()
    {
        if (_compilerExecutable != null)
            return _compilerExecutable.Length == 0 ? null : _compilerExecutable;

        string[] candidates = RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
            ? new[] { "clang++", "g++", "clang++.exe", "g++.exe" }
            : new[] { "clang++", "g++", "c++" };

        foreach (string c in candidates)
        {
            try
            {
                var psi = new ProcessStartInfo
                {
                    FileName = c,
                    Arguments = "--version",
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    UseShellExecute = false,
                    CreateNoWindow = true
                };
                using var proc = Process.Start(psi);
                if (proc != null && proc.WaitForExit(2000) && proc.ExitCode == 0)
                {
                    _compilerExecutable = c;
                    return c;
                }
            }
            catch
            {
            }
        }

        _compilerExecutable = string.Empty;
        return null;
    }

    public static bool CompileCppFileToSharedLibrary(
        string cppFilePath,
        string? includeDir,
        string outputSharedLibPath,
        out string? errorOutput)
    {
        errorOutput = null;
        string? cxx = ProbeCppCompiler();
        if (cxx == null)
        {
            errorOutput = "No C++20 compiler (clang++ / g++) found on PATH.";
            return false;
        }

        lock (CompileLock)
        {
            if (File.Exists(outputSharedLibPath))
                return true;

            string? outDir = Path.GetDirectoryName(outputSharedLibPath);
            if (!string.IsNullOrEmpty(outDir))
                Directory.CreateDirectory(outDir);

            string tmpOut = outputSharedLibPath + $".{Guid.NewGuid():N}{SharedLibraryExtension}";
            string picFlag = RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? "" : "-fPIC";
            string archFlag = RuntimeInformation.ProcessArchitecture is Architecture.X64 or Architecture.Arm64
                ? "-march=native"
                : "";
            string incArg = !string.IsNullOrEmpty(includeDir) ? $"-I\"{includeDir}\"" : "";

            var psi = new ProcessStartInfo
            {
                FileName = cxx,
                Arguments = $"-std=c++20 -O3 {archFlag} {picFlag} -shared {incArg} \"{cppFilePath}\" -o \"{tmpOut}\"",
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };

            try
            {
                using var proc = Process.Start(psi);
                if (proc == null)
                {
                    errorOutput = $"Failed to start '{cxx}'.";
                    return false;
                }

                string stderr = proc.StandardError.ReadToEnd();
                proc.WaitForExit(30000);
                if (proc.ExitCode != 0 || !File.Exists(tmpOut))
                {
                    try { if (File.Exists(tmpOut)) File.Delete(tmpOut); } catch { }
                    errorOutput = stderr;
                    return false;
                }

                File.Move(tmpOut, outputSharedLibPath, overwrite: true);
                return true;
            }
            catch (Exception ex)
            {
                try { if (File.Exists(tmpOut)) File.Delete(tmpOut); } catch { }
                errorOutput = ex.Message;
                return false;
            }
        }
    }

    /// <summary>
    /// Compiles generated C++20 <c>@burst</c> source code into a cached native shared library and exports
    /// the unmanaged entry point pointer for <paramref name="symbolName"/>.
    /// </summary>
    public static bool TryCompileAndLoadBurstSymbol(
        string cppSource,
        string symbolName,
        out IntPtr symbolPtr,
        out string? error)
    {
        symbolPtr = IntPtr.Zero;
        error = null;

        string hash = ComputeSha256Short(cppSource + "|" + symbolName + "|" + RuntimeInformation.ProcessArchitecture);
        string cacheKey = $"{hash}:{symbolName}";
        if (KernelSymbolCache.TryGetValue(cacheKey, out var cachedPtr) && cachedPtr != IntPtr.Zero)
        {
            Interlocked.Increment(ref _burstCacheHits);
            symbolPtr = cachedPtr;
            return true;
        }

        string? cxx = ProbeCppCompiler();
        if (cxx == null)
        {
            error = "No C++20 compiler available.";
            return false;
        }

        string cacheDir = Path.Combine(GetNativeCacheDirectory(), "burst");
        Directory.CreateDirectory(cacheDir);
        string cppPath = Path.Combine(cacheDir, $"burst_{hash}.cpp");
        string soPath = Path.Combine(cacheDir, $"burst_{hash}{SharedLibraryExtension}");

        bool existedOnDisk = File.Exists(soPath);
        if (!existedOnDisk)
        {
            File.WriteAllText(cppPath, cppSource, new UTF8Encoding(false));
            string? incDir = LocateRuntimeSourceDirectory();
            if (!CompileCppFileToSharedLibrary(cppPath, incDir, soPath, out error))
            {
                return false;
            }
            Interlocked.Increment(ref _burstCompiledKernels);
        }
        else
        {
            Interlocked.Increment(ref _burstCacheHits);
        }

        try
        {
            IntPtr lib = NativeLibrary.Load(soPath);
            IntPtr exp = NativeLibrary.GetExport(lib, symbolName);
            KernelSymbolCache[cacheKey] = exp;
            symbolPtr = exp;
            return true;
        }
        catch (Exception ex)
        {
            error = ex.Message;
            return false;
        }
    }

    #region Zero-Marshalling SliceBuffer & Regex Accelerators

    public static PilotValue SliceSum(PilotSharedBuffer buf)
    {
        int len = buf.Length;
        if (len == 0)
        {
            return buf.ElementKind is PilotSharedElementKind.Int64 or PilotSharedElementKind.Int32 or PilotSharedElementKind.UInt8
                ? PilotValue.FromInt(0)
                : PilotValue.FromFloat(0.0);
        }

        if (IsAvailable)
        {
            if (buf.ElementKind == PilotSharedElementKind.Float64)
            {
                Interlocked.Increment(ref _nativeDispatchCalls);
                return PilotValue.FromFloat(_fnSliceSumF64((double*)buf.DataPointer, (nuint)len));
            }
            if (buf.ElementKind == PilotSharedElementKind.Int64)
            {
                Interlocked.Increment(ref _nativeDispatchCalls);
                return PilotValue.FromInt(_fnSliceSumI64((long*)buf.DataPointer, (nuint)len));
            }
        }

        // Managed fallback for other element kinds or if C++ toolchain is absent
        if (buf.ElementKind is PilotSharedElementKind.Int64 or PilotSharedElementKind.Int32 or PilotSharedElementKind.UInt8)
        {
            long acc = 0;
            for (int i = 0; i < len; i++) acc += buf.GetElement(i).AsInt();
            return PilotValue.FromInt(acc);
        }
        else
        {
            double acc = 0.0;
            for (int i = 0; i < len; i++) acc += buf.GetElement(i).AsFloat();
            return PilotValue.FromFloat(acc);
        }
    }

    public static double SliceDot(PilotSharedBuffer a, PilotSharedBuffer b)
    {
        int len = Math.Min(a.Length, b.Length);
        if (len == 0) return 0.0;

        if (IsAvailable &&
            a.ElementKind == PilotSharedElementKind.Float64 &&
            b.ElementKind == PilotSharedElementKind.Float64)
        {
            Interlocked.Increment(ref _nativeDispatchCalls);
            return _fnSliceDotF64((double*)a.DataPointer, (double*)b.DataPointer, (nuint)len);
        }

        double sum = 0.0;
        for (int i = 0; i < len; i++)
        {
            sum += a.GetElement(i).AsFloat() * b.GetElement(i).AsFloat();
        }
        return sum;
    }

    public static void SliceSaxpy(double alpha, PilotSharedBuffer x, PilotSharedBuffer y)
    {
        if (y.IsReadOnly)
            throw new InvalidOperationException("Cannot mutate a read-only SliceBuffer view.");

        int len = Math.Min(x.Length, y.Length);
        if (len == 0) return;

        if (IsAvailable &&
            x.ElementKind == PilotSharedElementKind.Float64 &&
            y.ElementKind == PilotSharedElementKind.Float64)
        {
            Interlocked.Increment(ref _nativeDispatchCalls);
            _fnSliceSaxpyF64(alpha, (double*)x.DataPointer, (double*)y.DataPointer, (nuint)len);
            return;
        }

        for (int i = 0; i < len; i++)
        {
            double updated = y.GetElement(i).AsFloat() + alpha * x.GetElement(i).AsFloat();
            y.SetElement(i, PilotValue.FromFloat(updated));
        }
    }

    public static void SliceScale(double alpha, PilotSharedBuffer buf)
    {
        if (buf.IsReadOnly)
            throw new InvalidOperationException("Cannot mutate a read-only SliceBuffer view.");

        int len = buf.Length;
        if (len == 0) return;

        if (IsAvailable && buf.ElementKind == PilotSharedElementKind.Float64)
        {
            Interlocked.Increment(ref _nativeDispatchCalls);
            _fnSliceScaleF64(alpha, (double*)buf.DataPointer, (nuint)len);
            return;
        }

        for (int i = 0; i < len; i++)
        {
            buf.SetElement(i, PilotValue.FromFloat(buf.GetElement(i).AsFloat() * alpha));
        }
    }

    public static void SliceSort(PilotSharedBuffer buf)
    {
        if (buf.IsReadOnly)
            throw new InvalidOperationException("Cannot sort a read-only SliceBuffer view.");

        int len = buf.Length;
        if (len <= 1) return;

        if (IsAvailable)
        {
            if (buf.ElementKind == PilotSharedElementKind.Float64)
            {
                Interlocked.Increment(ref _nativeDispatchCalls);
                _fnSliceSortF64((double*)buf.DataPointer, (nuint)len);
                return;
            }
            if (buf.ElementKind == PilotSharedElementKind.Int64)
            {
                Interlocked.Increment(ref _nativeDispatchCalls);
                _fnSliceSortI64((long*)buf.DataPointer, (nuint)len);
                return;
            }
        }

        switch (buf.ElementKind)
        {
            case PilotSharedElementKind.Float64:
                buf.AsSpan<double>().Sort();
                break;
            case PilotSharedElementKind.Int64:
                buf.AsSpan<long>().Sort();
                break;
            case PilotSharedElementKind.Float32:
                buf.AsSpan<float>().Sort();
                break;
            case PilotSharedElementKind.Int32:
                buf.AsSpan<int>().Sort();
                break;
            case PilotSharedElementKind.UInt8:
                buf.AsSpan<byte>().Sort();
                break;
        }
    }

    public static void SliceWelford(
        PilotSharedBuffer buf,
        out double mean,
        out double m2,
        out double min,
        out double max)
    {
        int len = buf.Length;
        if (len == 0)
        {
            mean = 0.0;
            m2 = 0.0;
            min = 0.0;
            max = 0.0;
            return;
        }

        if (IsAvailable && buf.ElementKind == PilotSharedElementKind.Float64)
        {
            Interlocked.Increment(ref _nativeDispatchCalls);
            double cMean, cM2, cMin, cMax;
            _fnWelfordF64((double*)buf.DataPointer, (nuint)len, &cMean, &cM2, &cMin, &cMax);
            mean = cMean;
            m2 = cM2;
            min = cMin;
            max = cMax;
            return;
        }

        double m = buf.GetElement(0).AsFloat();
        double s = 0.0;
        double lo = m;
        double hi = m;
        for (int i = 1; i < len; i++)
        {
            double x = buf.GetElement(i).AsFloat();
            double delta = x - m;
            m += delta / (i + 1);
            s += delta * (x - m);
            if (x < lo) lo = x;
            if (x > hi) hi = x;
        }
        mean = m;
        m2 = s;
        min = lo;
        max = hi;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static double DotProductArray(double[] a, double[] b, int length)
    {
        if (length <= 0) return 0.0;
        if (IsAvailable)
        {
            Interlocked.Increment(ref _nativeDispatchCalls);
            fixed (double* pa = a)
            fixed (double* pb = b)
            {
                return _fnSliceDotF64(pa, pb, (nuint)length);
            }
        }

        double sum = 0.0;
        for (int i = 0; i < length; i++) sum += a[i] * b[i];
        return sum;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static void IntegrateVectorsArray(double[] positions, double[] velocities, double dt, double drag, int count)
    {
        if (count <= 0) return;
        if (IsAvailable)
        {
            Interlocked.Increment(ref _nativeDispatchCalls);
            fixed (double* pp = positions)
            fixed (double* pv = velocities)
            {
                _fnIntegrateVectorsF64(pp, pv, dt, drag, (nuint)count);
                return;
            }
        }

        for (int i = 0; i < count; i++)
        {
            velocities[i] *= drag;
            positions[i] += velocities[i] * dt;
        }
    }

    /// <summary>
    /// Invokes the C++20 native DFA UTF-16 scanner.
    /// Returns <c>1</c> on match, <c>0</c> on no match, or <c>-1</c> if managed fallback should run.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static int TryDfaFindUtf16(
        int[] flatAscii128,
        byte[] acceptFlags,
        int stateCount,
        bool hasNonAscii,
        bool anchoredStart,
        bool anchoredEnd,
        string text,
        int startAt,
        out int matchStart,
        out int matchEnd)
    {
        matchStart = -1;
        matchEnd = -1;
        if (!IsAvailable) return -1;

        Interlocked.Increment(ref _nativeDispatchCalls);
        int outS = -1, outE = -1;
        fixed (int* pTable = flatAscii128)
        fixed (byte* pAcc = acceptFlags)
        fixed (char* pText = text)
        {
            int rc = _fnDfaFindUtf16(
                pTable,
                pAcc,
                stateCount,
                hasNonAscii ? 1 : 0,
                anchoredStart ? 1 : 0,
                anchoredEnd ? 1 : 0,
                (ushort*)pText,
                text.Length,
                startAt,
                &outS,
                &outE);
            matchStart = outS;
            matchEnd = outE;
            return rc;
        }
    }

    /// <summary>
    /// Invokes the C++20 native DFA UTF-8 scanner directly over an unmanaged <see cref="PilotSharedBuffer"/> (<c>u8</c>).
    /// Zero <see cref="string"/> allocations!
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static int TryDfaFindUtf8(
        int[] flatAscii128,
        byte[] acceptFlags,
        int stateCount,
        bool hasNonAscii,
        bool anchoredStart,
        bool anchoredEnd,
        byte* utf8Bytes,
        int byteLen,
        int startAt,
        out int matchStart,
        out int matchEnd)
    {
        matchStart = -1;
        matchEnd = -1;
        if (!IsAvailable) return -1;

        Interlocked.Increment(ref _nativeDispatchCalls);
        int outS = -1, outE = -1;
        fixed (int* pTable = flatAscii128)
        fixed (byte* pAcc = acceptFlags)
        {
            int rc = _fnDfaFindUtf8(
                pTable,
                pAcc,
                stateCount,
                hasNonAscii ? 1 : 0,
                anchoredStart ? 1 : 0,
                anchoredEnd ? 1 : 0,
                utf8Bytes,
                byteLen,
                startAt,
                &outS,
                &outE);
            matchStart = outS;
            matchEnd = outE;
            return rc;
        }
    }

    #endregion

    private static string ComputeSha256Short(string text)
    {
        byte[] hash = SHA256.HashData(Encoding.UTF8.GetBytes(text));
        return Convert.ToHexString(hash)[..16].ToLowerInvariant();
    }
}
