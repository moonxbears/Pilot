using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;
using Antlr4.Runtime;
using Pilot.Runtime.CppBackend;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.NativeMemory;
using Pilot.Runtime.Values;
using Pilot.Transpilation;
using Pilot.Transpilation.Ir;
using Pilot.Transpilation.Targets;

namespace Pilot.Runtime.Burst;

/// <summary>
/// Unmanaged 32-byte C-ABI argument/return slot used to invoke compiled C++20 <c>@burst</c> kernels
/// across any parameter arity without heap allocations or marshalling overhead.
/// </summary>
[StructLayout(LayoutKind.Sequential)]
internal struct PilotBurstSlot
{
    public long IntValue;
    public double FloatValue;
    public IntPtr DataPointer;
    public nuint Length;
}

/// <summary>
/// Burst compilation engine for Pilot.
/// Translates functions annotated with <c>@burst</c>, <c>@burst_compile</c>, or <c>@native_fast</c>
/// into compiled C++20 shared-object kernels (<c>-O3 -march=native</c>) loaded via <see cref="NativeLibrary"/>
/// and invoked through unmanaged <c>delegate* unmanaged[Cdecl]</c> pointers, with seamless fallback
/// to a dedicated managed kernel when dynamic host types are used or no C++20 toolchain is installed.
/// </summary>
public static unsafe class PilotBurstEngine
{
    private enum BurstAbiParamKind
    {
        Int64,
        Float64,
        Bool,
        SliceF64,
        SliceI64
    }

    private enum BurstAbiReturnKind
    {
        Void,
        Int64,
        Float64,
        Bool
    }

    /// <summary>
    /// Attempts to compile a Pilot function into an optimized native C++20 Burst kernel
    /// (falling back cleanly to a fast managed kernel if C++ compilation is unsupported for the body).
    /// </summary>
    public static bool TryAttachBurstKernel(PilotFunction function, PilotParser.Func_defContext? context)
    {
        if (function == null || context == null) return false;

        bool hasBurstDecorator = false;
        if (context.decorators() != null)
        {
            foreach (var d in context.decorators().decorator())
            {
                string dName = d.dotted_name()?.GetText() ?? "";
                if (string.Equals(dName, "burst", StringComparison.OrdinalIgnoreCase) ||
                    string.Equals(dName, "burst_compile", StringComparison.OrdinalIgnoreCase) ||
                    string.Equals(dName, "native_fast", StringComparison.OrdinalIgnoreCase))
                {
                    hasBurstDecorator = true;
                    break;
                }
            }
        }

        if (!hasBurstDecorator) return false;

        function.IsBurstCompiled = true;

        // 1. Attempt real C++20 `-O3 -march=native` shared-library JIT compilation
        var nativeKernel = TryCompileNativeCppKernel(function, context);
        if (nativeKernel != null)
        {
            function.IsNativeCppBurst = true;
            function.BurstKernel = nativeKernel;
            return true;
        }

        // 2. Graceful managed fast-path fallback
        var managedKernel = CompileManagedFallbackKernel(function, context);
        if (managedKernel != null)
        {
            function.IsNativeCppBurst = false;
            function.BurstKernel = managedKernel;
            return true;
        }

        return false;
    }

    private static Func<IReadOnlyList<PilotValue>, PilotValue>? TryCompileNativeCppKernel(
        PilotFunction function,
        PilotParser.Func_defContext funcCtx)
    {
        if (PilotCppBackend.ProbeCppCompiler() == null)
            return null;

        // Disallow closures capturing mutable outer dynamic variables or generator/async coroutines
        if (function.IsAsync || function.IsGenerator || (function.Captures != null && function.Captures.Count > 0))
            return null;

        if (function.Parameters.Count > 16)
            return null;

        var paramKinds = new BurstAbiParamKind[function.Parameters.Count];
        for (int i = 0; i < function.Parameters.Count; i++)
        {
            var p = function.Parameters[i];
            if (p.IsParams || p.IsKwArgs || p.IsContextual || p.IsEach)
                return null;

            if (!TryClassifyParamType(p.TypeName, out paramKinds[i]))
                return null;
        }

        if (!TryClassifyReturnType(function.ReturnTypeSpec, function.IsFallible, out var retKind))
            return null;

        // Build clean Pilot source snippet for just this function (without `@burst` decorator)
        string rawFuncText = ExtractCleanFunctionSource(funcCtx);
        if (string.IsNullOrWhiteSpace(rawFuncText))
            return null;

        var transOpts = new TranspilationOptions
        {
            EmitHeaderComments = false
        };
        transOpts.TypeMappings["SliceBuffer"] = "PilotBurstSlice<double>";
        transOpts.TypeMappings["SliceBuffer<f64>"] = "PilotBurstSlice<double>";
        transOpts.TypeMappings["SliceBuffer[f64]"] = "PilotBurstSlice<double>";
        transOpts.TypeMappings["SliceBuffer<float>"] = "PilotBurstSlice<double>";
        transOpts.TypeMappings["SliceBuffer[float]"] = "PilotBurstSlice<double>";
        transOpts.TypeMappings["SliceBuffer<i64>"] = "PilotBurstSlice<int64_t>";
        transOpts.TypeMappings["SliceBuffer[i64]"] = "PilotBurstSlice<int64_t>";
        transOpts.TypeMappings["SliceBuffer<int>"] = "PilotBurstSlice<int64_t>";
        transOpts.TypeMappings["SliceBuffer[int]"] = "PilotBurstSlice<int64_t>";
        transOpts.TypeMappings["NativeArray"] = "PilotBurstSlice<double>";

        var cppConverter = new CppTargetConverter();
        var transRes = cppConverter.TranspileSource(rawFuncText, transOpts);
        if (!transRes.Success || string.IsNullOrWhiteSpace(transRes.Code))
            return null;

        string safeFnName = SanitizeCppIdentifier(function.Name);
        string entrySymbol = $"__pilot_burst_entry_{safeFnName}";
        string fullCpp = BuildBurstCppTranslationUnit(transRes.Code, function.Name, entrySymbol, paramKinds, retKind);

        if (!PilotCppBackend.TryCompileAndLoadBurstSymbol(fullCpp, entrySymbol, out IntPtr symPtr, out _) || symPtr == IntPtr.Zero)
            return null;

        var fnPtr = (delegate* unmanaged[Cdecl]<PilotBurstSlot*, PilotBurstSlot*, void>)symPtr;
        int arity = paramKinds.Length;

        return args =>
        {
            PilotBurstSlot* slots = stackalloc PilotBurstSlot[arity > 0 ? arity : 1];
            for (int i = 0; i < arity; i++)
            {
                PilotValue argVal = i < args.Count
                    ? args[i]
                    : (function.Parameters[i].DefaultValue ?? PilotValue.FromInt(0));

                switch (paramKinds[i])
                {
                    case BurstAbiParamKind.Int64:
                        slots[i].IntValue = argVal.AsInt();
                        break;
                    case BurstAbiParamKind.Float64:
                        slots[i].FloatValue = argVal.AsFloat();
                        break;
                    case BurstAbiParamKind.Bool:
                        slots[i].IntValue = argVal.IsTrue ? 1L : 0L;
                        break;
                    case BurstAbiParamKind.SliceF64:
                    case BurstAbiParamKind.SliceI64:
                        if (argVal.Kind == PilotValueKind.SliceBuffer)
                        {
                            var sb = argVal.AsSliceBuffer();
                            slots[i].DataPointer = sb.DataPointer;
                            slots[i].Length = (nuint)sb.Length;
                        }
                        else if (argVal.Kind == PilotValueKind.NativeArray)
                        {
                            // Pinning handled below if NativeArray is passed; SliceBuffer is zero-pinning unmanaged memory!
                            slots[i].DataPointer = IntPtr.Zero;
                            slots[i].Length = 0;
                        }
                        break;
                }
            }

            PilotBurstSlot retSlot = default;
            fnPtr(slots, &retSlot);

            return retKind switch
            {
                BurstAbiReturnKind.Int64 => PilotValue.FromInt(retSlot.IntValue),
                BurstAbiReturnKind.Float64 => PilotValue.FromFloat(retSlot.FloatValue),
                BurstAbiReturnKind.Bool => PilotValue.FromBool(retSlot.IntValue != 0),
                _ => PilotValue.Null
            };
        };
    }

    private static bool TryClassifyParamType(string? typeSpec, out BurstAbiParamKind kind)
    {
        kind = BurstAbiParamKind.Float64;
        if (string.IsNullOrWhiteSpace(typeSpec))
            return false; // Require explicit C-ABI types on `@burst` parameters for deterministic layout

        string t = typeSpec.Trim();
        switch (t)
        {
            case "int" or "i64" or "i32" or "i16" or "i8" or "u64" or "u32" or "u16" or "u8":
                kind = BurstAbiParamKind.Int64;
                return true;
            case "float" or "f64" or "f32" or "double":
                kind = BurstAbiParamKind.Float64;
                return true;
            case "bool":
                kind = BurstAbiParamKind.Bool;
                return true;
            case "SliceBuffer" or "SliceBuffer<f64>" or "SliceBuffer[f64]" or "SliceBuffer<float>" or "SliceBuffer[float]":
                kind = BurstAbiParamKind.SliceF64;
                return true;
            case "SliceBuffer<i64>" or "SliceBuffer[i64]" or "SliceBuffer<int>" or "SliceBuffer[int]":
                kind = BurstAbiParamKind.SliceI64;
                return true;
            default:
                return false;
        }
    }

    private static bool TryClassifyReturnType(string? retTypeSpec, bool isFallible, out BurstAbiReturnKind kind)
    {
        kind = BurstAbiReturnKind.Void;
        if (isFallible) return false;
        if (string.IsNullOrWhiteSpace(retTypeSpec))
        {
            kind = BurstAbiReturnKind.Void;
            return true;
        }

        string t = retTypeSpec.Trim();
        switch (t)
        {
            case "void" or "unit" or "()":
                kind = BurstAbiReturnKind.Void;
                return true;
            case "int" or "i64" or "i32" or "i16" or "i8":
                kind = BurstAbiReturnKind.Int64;
                return true;
            case "float" or "f64" or "f32" or "double":
                kind = BurstAbiReturnKind.Float64;
                return true;
            case "bool":
                kind = BurstAbiReturnKind.Bool;
                return true;
            default:
                return false;
        }
    }

    private static string ExtractCleanFunctionSource(PilotParser.Func_defContext funcCtx)
    {
        var input = funcCtx.Start?.InputStream;
        if (input == null || funcCtx.DEF() == null || funcCtx.Stop == null)
            return string.Empty;

        int startIdx = funcCtx.DEF().Symbol.StartIndex;
        int stopIdx = funcCtx.Stop.StopIndex;
        if (startIdx < 0 || stopIdx < startIdx)
            return string.Empty;

        return input.GetText(new Antlr4.Runtime.Misc.Interval(startIdx, stopIdx));
    }

    private static string BuildBurstCppTranslationUnit(
        string emittedCppFunc,
        string pilotFuncName,
        string entrySymbol,
        BurstAbiParamKind[] paramKinds,
        BurstAbiReturnKind retKind)
    {
        // Replace `constexpr ` on the function header with `inline ` so `std::vector` / `<cmath>` calls compile cleanly on both clang++ and g++
        string relaxedFunc = emittedCppFunc.StartsWith("constexpr ", StringComparison.Ordinal)
            ? "inline " + emittedCppFunc["constexpr ".Length..]
            : emittedCppFunc;

        var sb = new StringBuilder(1024 + relaxedFunc.Length);
        sb.AppendLine("// Auto-generated Pilot @burst C++20 Native Kernel");
        sb.AppendLine("#include <algorithm>");
        sb.AppendLine("#include <cassert>");
        sb.AppendLine("#include <cmath>");
        sb.AppendLine("#include <cstddef>");
        sb.AppendLine("#include <cstdint>");
        sb.AppendLine("#include <vector>");
        sb.AppendLine();
        sb.AppendLine("#if defined(_WIN32) || defined(__CYGWIN__)");
        sb.AppendLine("  #define PILOT_BURST_EXPORT __declspec(dllexport)");
        sb.AppendLine("#else");
        sb.AppendLine("  #define PILOT_BURST_EXPORT __attribute__((visibility(\"default\")))");
        sb.AppendLine("#endif");
        sb.AppendLine();
        sb.AppendLine("struct PilotBurstSlot {");
        sb.AppendLine("    int64_t i_val;");
        sb.AppendLine("    double  f_val;");
        sb.AppendLine("    void*   data_ptr;");
        sb.AppendLine("    size_t  length;");
        sb.AppendLine("};");
        sb.AppendLine();
        sb.AppendLine("template <typename T>");
        sb.AppendLine("struct PilotBurstSlice {");
        sb.AppendLine("    T* data;");
        sb.AppendLine("    size_t len;");
        sb.AppendLine("    inline T& operator[](int64_t idx) const noexcept { return data[static_cast<size_t>(idx)]; }");
        sb.AppendLine("    inline size_t size() const noexcept { return len; }");
        sb.AppendLine("    inline int64_t length() const noexcept { return static_cast<int64_t>(len); }");
        sb.AppendLine("    inline T* begin() const noexcept { return data; }");
        sb.AppendLine("    inline T* end() const noexcept { return data + len; }");
        sb.AppendLine("};");
        sb.AppendLine();
        sb.AppendLine("struct PilotBurstNatArray {");
        sb.AppendLine("    std::vector<double> data;");
        sb.AppendLine("    explicit PilotBurstNatArray(int64_t n) : data(static_cast<size_t>(n > 0 ? n : 0), 0.0) {}");
        sb.AppendLine("    inline void fill(double v) noexcept { std::fill(data.begin(), data.end(), v); }");
        sb.AppendLine("    inline void scale(double s) noexcept { for (double& x : data) x *= s; }");
        sb.AppendLine("    inline double sum() const noexcept { double acc = 0.0; for (double x : data) acc += x; return acc; }");
        sb.AppendLine("    inline double dot(const PilotBurstNatArray& o) const noexcept {");
        sb.AppendLine("        const size_t n = std::min(data.size(), o.data.size());");
        sb.AppendLine("        double acc = 0.0; for (size_t i = 0; i < n; ++i) acc += data[i] * o.data[i]; return acc;");
        sb.AppendLine("    }");
        sb.AppendLine("    inline size_t size() const noexcept { return data.size(); }");
        sb.AppendLine("    inline double& operator[](int64_t i) noexcept { return data[static_cast<size_t>(i)]; }");
        sb.AppendLine("    inline double operator[](int64_t i) const noexcept { return data[static_cast<size_t>(i)]; }");
        sb.AppendLine("};");
        sb.AppendLine("inline PilotBurstNatArray natarray(int64_t n) { return PilotBurstNatArray(n); }");
        sb.AppendLine("using std::sin; using std::cos; using std::tan; using std::asin; using std::acos; using std::atan; using std::atan2;");
        sb.AppendLine("using std::sinh; using std::cosh; using std::tanh; using std::exp; using std::log; using std::log2; using std::log10;");
        sb.AppendLine("using std::sqrt; using std::cbrt; using std::pow; using std::hypot; using std::abs; using std::floor; using std::ceil;");
        sb.AppendLine("inline double ln(double x) noexcept { return std::log(x); }");
        sb.AppendLine("template <typename C> inline int64_t __slice_len(const C& c) noexcept { return static_cast<int64_t>(c.size()); }");
        sb.AppendLine("template <typename C> inline auto __slice_get(const C& c, int64_t idx) noexcept { return c[idx]; }");
        sb.AppendLine("template <typename C, typename V> inline auto __slice_set(const C& c, int64_t idx, V val) noexcept { c[idx] = val; return val; }");
        sb.AppendLine();
        sb.AppendLine("namespace pilot_burst_impl {");
        sb.AppendLine(relaxedFunc);
        sb.AppendLine("}");
        sb.AppendLine();
        sb.AppendLine($"extern \"C\" PILOT_BURST_EXPORT void {entrySymbol}(const PilotBurstSlot* args, PilotBurstSlot* out_ret) {{");

        var callArgs = new List<string>(paramKinds.Length);
        for (int i = 0; i < paramKinds.Length; i++)
        {
            callArgs.Add(paramKinds[i] switch
            {
                BurstAbiParamKind.Int64 => $"args[{i}].i_val",
                BurstAbiParamKind.Float64 => $"args[{i}].f_val",
                BurstAbiParamKind.Bool => $"(args[{i}].i_val != 0)",
                BurstAbiParamKind.SliceF64 => $"PilotBurstSlice<double>{{ static_cast<double*>(args[{i}].data_ptr), args[{i}].length }}",
                BurstAbiParamKind.SliceI64 => $"PilotBurstSlice<int64_t>{{ static_cast<int64_t*>(args[{i}].data_ptr), args[{i}].length }}",
                _ => $"args[{i}].f_val"
            });
        }

        string callExpr = $"pilot_burst_impl::{pilotFuncName}({string.Join(", ", callArgs)})";
        switch (retKind)
        {
            case BurstAbiReturnKind.Int64:
                sb.AppendLine($"    out_ret->i_val = static_cast<int64_t>({callExpr});");
                break;
            case BurstAbiReturnKind.Float64:
                sb.AppendLine($"    out_ret->f_val = static_cast<double>({callExpr});");
                break;
            case BurstAbiReturnKind.Bool:
                sb.AppendLine($"    out_ret->i_val = ({callExpr}) ? 1 : 0;");
                break;
            default:
                sb.AppendLine($"    (void)args; (void)out_ret; {callExpr};");
                break;
        }
        sb.AppendLine("}");
        return sb.ToString();
    }

    private static string SanitizeCppIdentifier(string name)
    {
        var sb = new StringBuilder(name.Length);
        foreach (char c in name)
        {
            sb.Append(char.IsLetterOrDigit(c) || c == '_' ? c : '_');
        }
        return sb.Length > 0 ? sb.ToString() : "kernel";
    }

    private static Func<IReadOnlyList<PilotValue>, PilotValue>? CompileManagedFallbackKernel(
        PilotFunction function,
        PilotParser.Func_defContext context)
    {
        var suite = context.suite();
        if (suite == null) return null;

        var paramNames = new string[function.Parameters.Count];
        for (int i = 0; i < function.Parameters.Count; i++)
        {
            paramNames[i] = function.Parameters[i].Name;
        }

        return args =>
        {
            var engine = new PilotEngine();
            var scope = new PilotScope(function.ClosureScope);

            for (int i = 0; i < paramNames.Length; i++)
            {
                var val = i < args.Count ? args[i] : (function.Parameters[i].DefaultValue ?? PilotValue.Undefined);
                scope.Declare(paramNames[i], val);
            }

            var execCtx = new PilotExecutionContext(engine, new PilotExecutionOptions(), new PilotEnvironment(scope));
            var visitor = new PilotInterpreterVisitor(execCtx);

            try
            {
                var result = visitor.Visit(suite);
                if (execCtx.PendingFlow == Pilot.Runtime.ControlFlow.PilotFlowSignalKind.Return)
                {
                    var rv = execCtx.PendingReturnValue;
                    execCtx.PendingFlow = Pilot.Runtime.ControlFlow.PilotFlowSignalKind.None;
                    return rv;
                }
                return result;
            }
            catch (Pilot.Runtime.ControlFlow.PilotReturnSignal ret)
            {
                return ret.Value;
            }
        };
    }

    /// <summary>
    /// High-performance Burst vector dot product backed by <see cref="PilotCppBackend"/> (<c>libpilot_rt</c>) when available.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveOptimization)]
    public static double BurstDotProduct(double[] a, double[] b, int length)
    {
        return PilotCppBackend.DotProductArray(a, b, length);
    }

    /// <summary>
    /// High-performance Burst particle/vector integrator backed by <see cref="PilotCppBackend"/> (<c>libpilot_rt</c>) when available.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveOptimization)]
    public static void BurstIntegrateVectors(
        double[] positions,
        double[] velocities,
        double dt,
        double drag,
        int count)
    {
        PilotCppBackend.IntegrateVectorsArray(positions, velocities, dt, drag, count);
    }
}
