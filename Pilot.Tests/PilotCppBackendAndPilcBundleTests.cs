using System;
using System.IO;
using System.Linq;
using Pilot.Compiler;
using Pilot.Runtime.CppBackend;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Stdlib;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

public class PilotCppBackendAndPilcBundleTests
{
    private static PilotExecutionResult ExecuteCode(string code, bool useBytecode)
    {
        if (useBytecode)
        {
            var comp = PilotCompiler.Default.Compile(code, new PilotCompilerOptions { EnableCaching = false });
            Assert.True(comp.Success, $"Bytecode compilation failed: {string.Join("; ", comp.Diagnostics)}");
            return comp.Program!.Run();
        }

        var engine = new PilotEngine();
        return engine.Execute(code);
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void CppBackend_SharedRuntime_LoadsAndExecutesSimdSliceBufferKernels(bool useBytecode)
    {
        Assert.True(PilotCppBackend.IsAvailable, "Expected C++20 libpilot_rt backend to be compiled and loaded.");
        Assert.Equal(2101, PilotCppBackend.AbiVersion);
        Assert.True(PilotCppBackend.SimdWidthBits >= 64);

        long callsBefore = PilotCppBackend.NativeDispatchCalls;

        string code = @"
            import std.mem;

            var x = mem.alloc_slice(6, ""f64"", 0);
            var y = mem.alloc_slice(6, ""f64"", 0);
            for i in 1..6 {
                mem.slice_push(x, i * 1.0);
                mem.slice_push(y, 10.0);
            }

            let dot_xy = mem.slice_dot(x, y); // (1+2+3+4+5+6)*10 = 210.0
            mem.slice_saxpy(2.0, x, y);       // y[i] = 10 + 2*(i+1) -> [12, 14, 16, 18, 20, 22], sum = 102.0
            let sum_y = mem.slice_sum(y);

            var unsorted = mem.alloc_slice(5, ""f64"", 0);
            mem.slice_push(unsorted, 42.0);
            mem.slice_push(unsorted, -7.5);
            mem.slice_push(unsorted, 19.25);
            mem.slice_push(unsorted, 0.0);
            mem.slice_push(unsorted, 8.5);
            mem.slice_sort(unsorted);
            let sorted_list = mem.slice_to_list(unsorted);

            let welford = mem.slice_welford(x); // mean of 1..6 = 3.5, m2 = 17.5, min = 1.0, max = 6.0

            return (
                mem.cpp_backend_available(),
                mem.cpp_simd_bits(),
                dot_xy,
                sum_y,
                sorted_list,
                welford
            );
        ";

        var res = ExecuteCode(code, useBytecode);
        Assert.True(res.Success, res.ErrorMessage);
        Assert.True(PilotCppBackend.NativeDispatchCalls > callsBefore);

        var t = res.Value.AsTuple();
        Assert.True(t[0].IsTrue);
        Assert.True(t[1].AsInt() >= 64);
        Assert.Equal(210.0, t[2].AsFloat(), 6);
        Assert.Equal(102.0, t[3].AsFloat(), 6);

        var sorted = t[4].AsList().Select(v => v.AsFloat()).ToArray();
        Assert.Equal(new[] { -7.5, 0.0, 8.5, 19.25, 42.0 }, sorted);

        var wf = t[5].AsTuple();
        Assert.Equal(3.5, wf[0].AsFloat(), 6);
        Assert.Equal(17.5, wf[1].AsFloat(), 6);
        Assert.Equal(1.0, wf[2].AsFloat(), 6);
        Assert.Equal(6.0, wf[3].AsFloat(), 6);
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void CppBackend_DfaRegex_ScansUtf16Strings_And_ZeroCopyUtf8Slices_Natively(bool useBytecode)
    {
        long callsBefore = PilotCppBackend.NativeDispatchCalls;

        string code = @"
            import std.text;
            import std.option;

            let token_re = comptime(compile_regex(""[a-z]+_[0-9]+""));
            let raw_slice = text.str_slice(""header :: metric_4096 :: footer"");

            // Zero-copy UTF-8 SliceBuffer<u8> scan directly via C++20 `pilot_rt_dfa_find_utf8`
            let matched_utf8 = __dfa_is_match(token_re, raw_slice);
            let span_utf8 = option.unwrap(__dfa_find(token_re, raw_slice, 0));

            // Standard UTF-16 string scan via C++20 `pilot_rt_dfa_find_utf16`
            let span_utf16 = option.unwrap(text.regex_find(token_re, ""prefix alpha_99 suffix""));

            return (matched_utf8, span_utf8[0], span_utf8[1], span_utf8[2], span_utf16[2]);
        ";

        var res = ExecuteCode(code, useBytecode);
        Assert.True(res.Success, res.ErrorMessage);
        Assert.True(PilotCppBackend.NativeDispatchCalls > callsBefore);

        var t = res.Value.AsTuple();
        Assert.True(t[0].IsTrue);
        Assert.Equal(10L, t[1].AsInt());
        Assert.Equal(21L, t[2].AsInt());
        Assert.Equal("metric_4096", t[3].AsString());
        Assert.Equal("alpha_99", t[4].AsString());
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void BurstEngine_CompilesTypedPilotFunctions_ToNativeCpp20SharedObjects_Parity(bool useBytecode)
    {
        string code = @"
            import std.mem;

            @burst
            def fma_saxpy_kernel(alpha: f64, x: SliceBuffer<f64>, y: SliceBuffer<f64>, n: i64) -> f64 {
                var total: f64 = 0.0;
                for i in 0..<n {
                    y[i] = y[i] + alpha * x[i];
                    total = total + y[i];
                }
                return total;
            }

            @burst
            def trig_energy(steps: i64, scale: f64) -> f64 {
                var acc: f64 = 0.0;
                for k in 1..steps {
                    let fk: f64 = k * 0.1;
                    acc = acc + (sin(fk) * sin(fk) + cos(fk) * cos(fk)) * scale;
                }
                return acc;
            }

            var x = mem.alloc_slice(4, ""f64"", 0);
            var y = mem.alloc_slice(4, ""f64"", 0);
            for i in 1..4 {
                mem.slice_push(x, i * 1.0);   // [1, 2, 3, 4]
                mem.slice_push(y, 100.0);     // [100, 100, 100, 100]
            }

            let sum_after = fma_saxpy_kernel(3.0, x, y, 4); // [103, 106, 109, 112] -> sum = 430.0
            let energy = trig_energy(25, 2.0);              // 25 * 1.0 * 2.0 = 50.0

            return (
                fma_saxpy_kernel.is_burst,
                fma_saxpy_kernel.is_native_burst,
                trig_energy.is_native_burst,
                sum_after,
                mem.slice_to_list(y),
                energy
            );
        ";

        var res = ExecuteCode(code, useBytecode);
        Assert.True(res.Success, res.ErrorMessage);

        var t = res.Value.AsTuple();
        Assert.True(t[0].IsTrue, "Expected fma_saxpy_kernel.is_burst == true");
        Assert.True(t[1].IsTrue, "Expected fma_saxpy_kernel.is_native_burst == true (compiled via clang++/g++)");
        Assert.True(t[2].IsTrue, "Expected trig_energy.is_native_burst == true");
        Assert.Equal(430.0, t[3].AsFloat(), 6);
        Assert.Equal(new[] { 103.0, 106.0, 109.0, 112.0 }, t[4].AsList().Select(v => v.AsFloat()).ToArray());
        Assert.Equal(50.0, t[5].AsFloat(), 5);
    }

    [Fact]
    public void BurstEngine_SurvivesPilcRoundTripSerialization_WithNativeCppExecution()
    {
        string code = @"
            @burst
            def collatz_steps(start_val: i64) -> i64 {
                var n: i64 = start_val;
                var steps: i64 = 0;
                while n > 1 {
                    if n % 2 == 0 {
                        n = n / 2;
                    } else {
                        n = 3 * n + 1;
                    }
                    steps = steps + 1;
                }
                return steps;
            }

            return (collatz_steps.is_native_burst, collatz_steps(27));
        ";

        var comp = PilotCompiler.Default.Compile(code, new PilotCompilerOptions { EnableCaching = false });
        Assert.True(comp.Success);

        byte[] pilcBytes = comp.Program!.ToBytes();
        var reloaded = PilotCompiledProgram.FromBytes(pilcBytes, "burst_collatz.pilot");
        var res = reloaded.Run();
        Assert.True(res.Success, res.ErrorMessage);

        var t = res.Value.AsTuple();
        Assert.True(t[0].IsTrue);
        Assert.Equal(111L, t[1].AsInt());
    }

    [Fact]
    public void StdlibPilcBundle_ZeroColdStart_LoadsFromBuildTimeBundleWithoutRuntimeCompile()
    {
        string bundleDir = Path.Combine(PilotStdlibResolver.GetStdlibRootDirectory(), ".pilc-precompiled");
        string bundlePath = PilotStdlibResolver.BuildPrecompiledStdlibBundle(bundleDir);
        Assert.True(File.Exists(bundlePath));
        Assert.True(new FileInfo(bundlePath).Length > 1024);

        PilotStdlibResolver.ClearMemoryCacheForTesting();
        PilotStdlibResolver.ResetCacheCountersForTesting();

        var res = ExecuteCode(@"
            import std.math;
            import std.mem;
            import std.option;
            import std.result;

            let v = option.unwrap_or(some(21), 0) * 2;
            return (v, math.gcd(84, 126));
        ", useBytecode: true);

        Assert.True(res.Success, res.ErrorMessage);
        var tup = res.Value.AsTuple();
        Assert.Equal(42L, tup[0].AsInt());
        Assert.Equal(42L, tup[1].AsInt());

        Assert.True(PilotStdlibResolver.PrecompiledBundleHits >= 4,
            $"Expected >= 4 precompiled bundle hits, got {PilotStdlibResolver.PrecompiledBundleHits}");
        Assert.Equal(0L, PilotStdlibResolver.DiskCacheBuilds);
    }
}
