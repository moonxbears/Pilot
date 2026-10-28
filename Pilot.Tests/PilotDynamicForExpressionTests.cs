#nullable disable
using System;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using Pilot.Analysis;
using Pilot.Analysis.Loops;
using Pilot.Compiler;
using Pilot.Native.CAbi;
using Pilot.Runtime.Execution;
using Pilot.Runtime.NativeMemory;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Milestone 15 Canonical Acceptance Suite:
/// - 15.1: Expression-oriented <c>for</c> loops, <c>then</c> completion clause, inclusive <c>..</c>/<c>..=</c> &amp; half-open <c>..&lt;</c>,
///         valued <c>break &lt;expr&gt;</c>, <c>continue start..end;</c>, and <c>continue start..end, ...;</c>.
/// - 15.2: Single-pass scoped inference (<see cref="LoopScheduleMode"/>, <see cref="LoopReturnMode"/>,
///         <see cref="PilotForLoopInfo.CanPreallocateYieldArray"/>) and diagnostics <c>LOOP001</c>–<c>LOOP004</c>.
/// - 15.3: Zero-marshalling C-ABI layout invariants (<c>sizeof(RangeWindow&lt;int&gt;) == 8</c>,
///         <c>sizeof(SliceBuffer&lt;float&gt;) == 24</c>) and base-address independence.
/// - 15.4: All 5 canonical loop acceptance cases across both AST Interpreter and PilVM Bytecode.
/// </summary>
public class PilotDynamicForExpressionTests
{
    private readonly PilotEngine _engine = new();

    private void AssertAstAndBytecode(string code, Action<PilotExecutionContext> verify)
    {
        // 1. AST Interpreter
        var astRes = _engine.Execute(code);
        Assert.True(astRes.Success, "AST failed: " + string.Join("; ", astRes.Diagnostics));
        verify(astRes.Context);

        // 2. PilVM Bytecode Compiler + VM
        var compileRes = PilotCompiler.Default.Compile(code);
        Assert.True(compileRes.Success, "Bytecode compile failed: " + string.Join("; ", compileRes.Diagnostics));
        var vmRes = compileRes.Program.Run();
        Assert.True(vmRes.Success, "PilVM failed: " + string.Join("; ", vmRes.Diagnostics));
        verify(vmRes.Context);
    }

    // =========================================================================
    // 1. Acceptance Case 1: Standard Fixed Yield + Pre-Allocation Inference
    // =========================================================================

    [Fact]
    public void Test_15_4_Case1_Standard_Fixed_Yield_And_Preallocation()
    {
        string code = @"
            let squares = for (i in 1..5) {
                yield i * i;
            };
            let half_open = for (i in 0..<4) {
                yield i + 10;
            };
        ";

        // Verify static inference classifies both as FixedScalar + CollectionYield with CanPreallocateYieldArray == true
        var analysis = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.False(analysis.HasErrors, string.Join("; ", analysis.Diagnostics.Select(d => $"{d.Code}: {d.Message}")));
        Assert.Equal(2, analysis.LoopInfos.Count);
        foreach (var info in analysis.LoopInfos.Values)
        {
            Assert.Equal(LoopScheduleMode.FixedScalar, info.ScheduleMode);
            Assert.Equal(LoopReturnMode.CollectionYield, info.ReturnMode);
            Assert.True(info.CanPreallocateYieldArray);
        }

        AssertAstAndBytecode(code, ctx =>
        {
            var squares = ctx.Environment.CurrentScope.Resolve("squares").AsList().Select(v => v.AsInt()).ToArray();
            Assert.Equal(new long[] { 1, 4, 9, 16, 25 }, squares);

            var halfOpen = ctx.Environment.CurrentScope.Resolve("half_open").AsList().Select(v => v.AsInt()).ToArray();
            Assert.Equal(new long[] { 10, 11, 12, 13 }, halfOpen);
        });
    }

    // =========================================================================
    // 2. Acceptance Case 2: Destructive Pivot (`continue start..end;` -> MutableBounds)
    // =========================================================================

    [Fact]
    public void Test_15_4_Case2_Destructive_Pivot_MutableBounds()
    {
        string code = @"
            let visited = for (i in 0..10) {
                yield i;
                if i == 2 {
                    continue 8..10;
                }
            };
        ";

        var analysis = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.False(analysis.HasErrors, string.Join("; ", analysis.Diagnostics.Select(d => $"{d.Code}: {d.Message}")));
        var loopInfo = Assert.Single(analysis.LoopInfos.Values);
        Assert.Equal(LoopScheduleMode.MutableBounds, loopInfo.ScheduleMode);
        Assert.Equal(LoopReturnMode.CollectionYield, loopInfo.ReturnMode);
        Assert.False(loopInfo.CanPreallocateYieldArray);

        AssertAstAndBytecode(code, ctx =>
        {
            var visited = ctx.Environment.CurrentScope.Resolve("visited").AsList().Select(v => v.AsInt()).ToArray();
            Assert.Equal(new long[] { 0, 1, 2, 8, 9, 10 }, visited);
        });
    }

    // =========================================================================
    // 3. Acceptance Case 3: Resumable Worklist (`continue start..end, ...;` -> WorklistStack)
    // =========================================================================

    [Fact]
    public void Test_15_4_Case3_Resumable_Worklist_LIFO_Schedule()
    {
        string code = @"
            let order = for (i in 1..4) {
                yield i;
                if i == 2 {
                    continue 10..11, ...;
                }
            };
        ";

        var analysis = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.False(analysis.HasErrors, string.Join("; ", analysis.Diagnostics.Select(d => $"{d.Code}: {d.Message}")));
        var loopInfo = Assert.Single(analysis.LoopInfos.Values);
        Assert.Equal(LoopScheduleMode.WorklistStack, loopInfo.ScheduleMode);
        Assert.Equal(LoopReturnMode.CollectionYield, loopInfo.ReturnMode);
        Assert.False(loopInfo.CanPreallocateYieldArray);

        AssertAstAndBytecode(code, ctx =>
        {
            var order = ctx.Environment.CurrentScope.Resolve("order").AsList().Select(v => v.AsInt()).ToArray();
            // 1, 2 -> pushes remaining (3..4) onto LIFO worklist, runs 10..11, then resumes 3..4
            Assert.Equal(new long[] { 1, 2, 10, 11, 3, 4 }, order);
        });
    }

    // =========================================================================
    // 4. Acceptance Case 4: Scalar Break + `then` Completion Branch (`ScalarBreak`)
    // =========================================================================

    [Fact]
    public void Test_15_4_Case4_Scalar_Break_And_Then_Branch()
    {
        string code = @"
            let data = [10, 25, 42, 80];

            let found_idx = for (i in 0..<4) {
                if data[i] == 42 {
                    break i;
                }
            } then -1;

            let not_found_idx = for (i in 0..<4) {
                if data[i] == 999 {
                    break i;
                }
            } then -1;

            let optional_hit = for (i in 0..<4) {
                if data[i] == 25 {
                    break data[i];
                }
            };

            let optional_miss = for (i in 0..<4) {
                if data[i] == 777 {
                    break data[i];
                }
            };
        ";

        var analysis = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.False(analysis.HasErrors, string.Join("; ", analysis.Diagnostics.Select(d => $"{d.Code}: {d.Message}")));
        Assert.All(analysis.LoopInfos.Values, info => Assert.Equal(LoopReturnMode.ScalarBreak, info.ReturnMode));

        AssertAstAndBytecode(code, ctx =>
        {
            var scope = ctx.Environment.CurrentScope;
            Assert.Equal(2, scope.Resolve("found_idx").AsInt());
            Assert.Equal(-1, scope.Resolve("not_found_idx").AsInt());
            Assert.Equal(25, scope.Resolve("optional_hit").AsInt());
            Assert.True(scope.Resolve("optional_miss").IsNull);
        });
    }

    // =========================================================================
    // 5. Acceptance Case 5: Nested Scope Isolation & Coroutine Generator Coexistence
    // =========================================================================

    [Fact]
    public void Test_15_4_Case5_Nested_Scope_Isolation_And_Generator_Compatibility()
    {
        string code = @"
            // Outer loop is FixedScalar + CollectionYield; inner loop is WorklistStack + ScalarBreak + then
            let results = for (i in 1..3) {
                let resolved = for (j in 1..3) {
                    if i == 2 && j == 1 {
                        continue 10..10, ...;
                    }
                    if j == 10 {
                        break 200 + i;
                    }
                    if i * j == 9 {
                        break 900;
                    }
                } then (i * 10);

                yield resolved;
            };

            // Statement-level `for` inside a coroutine generator still yields from the generator
            def count_up(limit: int) {
                for (k in 1..limit) {
                    yield k * 10;
                }
            }

            var gen_sum = 0;
            for x in count_up(3) {
                gen_sum += x;
            }
        ";

        var analysis = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.False(analysis.HasErrors, string.Join("; ", analysis.Diagnostics.Select(d => $"{d.Code}: {d.Message}")));

        AssertAstAndBytecode(code, ctx =>
        {
            var results = ctx.Environment.CurrentScope.Resolve("results").AsList().Select(v => v.AsInt()).ToArray();
            // i = 1: inner finishes without breaking -> then (1 * 10) = 10
            // i = 2: j = 1 triggers `continue 10..10, ...;` -> j = 10 triggers `break 202`
            // i = 3: j = 3 triggers `i * j == 9` -> `break 900`
            Assert.Equal(new long[] { 10, 202, 900 }, results);

            // 10 + 20 + 30 = 60
            Assert.Equal(60, ctx.Environment.CurrentScope.Resolve("gen_sum").AsInt());
        });
    }

    // =========================================================================
    // 6. Static Diagnostics Verification (`LOOP001` – `LOOP004`)
    // =========================================================================

    [Fact]
    public void Test_15_2_Loop_Diagnostics_LOOP001_Through_LOOP004()
    {
        // LOOP001: Cannot mix `break <expr>` and `yield <expr>` in the same `for` loop
        var diag1 = PilotAnalysisPipeline.AnalyzeSource(@"
            let bad1 = for (i in 0..5) {
                if i == 3 { break i; }
                yield i * 2;
            };
        ");
        Assert.Contains(diag1.Diagnostics, d => d.Code == "LOOP001");

        // LOOP002: Cannot mix valueless `break;` and valued `break <expr>;` in the same `for` loop
        var diag2 = PilotAnalysisPipeline.AnalyzeSource(@"
            let bad2 = for (i in 0..5) {
                if i == 1 { break; }
                if i == 3 { break 42; }
            } then -1;
        ");
        Assert.Contains(diag2.Diagnostics, d => d.Code == "LOOP002");

        // LOOP003: `then` branch is invalid on `CollectionYield` or non-`ScalarBreak` loop
        var diag3 = PilotAnalysisPipeline.AnalyzeSource(@"
            let bad3 = for (i in 0..5) {
                yield i;
            } then [];
        ");
        Assert.Contains(diag3.Diagnostics, d => d.Code == "LOOP003");

        // LOOP004: `yield` in statement-position `for` loop outside a generator function
        var diag4 = PilotAnalysisPipeline.AnalyzeSource(@"
            for (i in 0..5) {
                yield i * 2;
            }
        ");
        Assert.Contains(diag4.Diagnostics, d => d.Code == "LOOP004");
    }

    // =========================================================================
    // 7. C-ABI Zero-Marshalling Memory Layout & Base-Address Independence (15.3)
    // =========================================================================

    [Fact]
    public unsafe void Test_15_3_CAbi_Memory_Layout_And_Zero_Marshalling_Invariants()
    {
        // 15.3.1: sizeof(RangeWindow<int>) == 8 and sizeof(RangeWindow<long>) == 16 with 0 padding bytes
        Assert.Equal(8, Unsafe.SizeOf<RangeWindow<int>>());
        Assert.Equal(16, Unsafe.SizeOf<RangeWindow<long>>());

        var win = new RangeWindow<int>(3, 9);
        byte* winPtr = (byte*)&win;
        Assert.Equal(0, (int)((byte*)&win.Current - winPtr));
        Assert.Equal(4, (int)((byte*)&win.Max - winPtr));

        // 15.3.3: sizeof(SliceBuffer<float>) == 24 on 64-bit targets (Offset 0: DataPtr, Offset 8: Length, Offset 16: Capacity)
        if (IntPtr.Size == 8)
        {
            Assert.Equal(24, Unsafe.SizeOf<SliceBuffer<float>>());
            Assert.Equal(24, Unsafe.SizeOf<SliceBuffer<double>>());

            SliceBuffer<float> slice = default;
            byte* basePtr = (byte*)&slice;
            Assert.Equal(0, (int)((byte*)&slice.DataPtr - basePtr));
            Assert.Equal(8, (int)((byte*)&slice.Length - basePtr));
            Assert.Equal(16, (int)((byte*)&slice.Capacity - basePtr));
        }

        // Verify zero-marshalling unmanaged SliceBuffer<float> direct writes & Span<float> projection
        var nativeSlice = SliceBuffer<float>.AllocateNative(5);
        try
        {
            for (int i = 0; i < 5; i++)
            {
                Assert.True(nativeSlice.TryYield((i + 1) * 1.5f));
            }
            Assert.False(nativeSlice.TryYield(99.0f)); // Capacity reached
            Assert.Equal((nuint)5, nativeSlice.Length);

            Span<float> span = nativeSlice.AsSpan();
            Assert.Equal(5, span.Length);
            Assert.Equal(1.5f, span[0]);
            Assert.Equal(7.5f, span[4]);
        }
        finally
        {
            nativeSlice.FreeNative();
        }

        // Verify C-ABI header emitter includes RangeWindow and SliceBuffer definitions
        string header = PilotCAbiEmitter.EmitHeader(
            "def process_window(buf: SliceBuffer[f64], win: RangeWindow[i32]) -> SliceBuffer[f64] { return buf; }",
            "loop_abi");
        Assert.Contains("RangeWindow_i32", header);
        Assert.Contains("RangeWindow_i64", header);
        Assert.Contains("SliceBuffer_f64", header);
        Assert.Contains("SliceBuffer_f64 process_window(SliceBuffer_f64 buf, RangeWindow_i32 win);", header);
    }

    [Fact]
    public unsafe void Test_15_3_1_UnmanagedWorklist_SharedMmap_BaseAddress_Handoff()
    {
        string shmName = $"pilot_worklist_ipc_{Guid.NewGuid():N}";
        using var mappingA = PilotSharedBuffer.CreateOrOpenShared(shmName, 32, PilotSharedElementKind.Int64);
        using var mappingB = mappingA.OpenIndependentView();

        // Two simultaneous OS virtual memory mappings over the same physical shared pages must have distinct virtual base pointers
        Assert.NotEqual(IntPtr.Zero, mappingA.DataPointer);
        Assert.NotEqual(IntPtr.Zero, mappingB.DataPointer);
        Assert.NotEqual(mappingA.DataPointer, mappingB.DataPointer);

        // Producer attaches at virtual BaseA and pushes suspended RangeWindow<long> frames (strictly relative indices, 0 pointers)
        var worklistA = UnmanagedWorklist<long>.FromSharedSlice((RangeWindow<long>*)mappingA.DataPointer, capacity: 16, initialLength: 0);
        worklistA.Push(new RangeWindow<long>(3, 10));
        worklistA.Push(new RangeWindow<long>(102, 105));
        Assert.Equal(2, worklistA.Count);
        mappingA.Flush();

        // Consumer attaches at virtual BaseB (BaseB != BaseA) and resumes LIFO frames with zero copying and zero pointer swizzling
        var worklistB = UnmanagedWorklist<long>.FromSharedSlice((RangeWindow<long>*)mappingB.DataPointer, capacity: 16, initialLength: 2);
        Assert.True(worklistB.TryPop(out var topFrame));
        Assert.Equal(new RangeWindow<long>(102, 105), topFrame);

        Assert.True(worklistB.TryPop(out var resumedOuterFrame));
        Assert.Equal(new RangeWindow<long>(3, 10), resumedOuterFrame);
        Assert.False(worklistB.TryPop(out _));
    }

    [Fact]
    public void Test_15_3_2_DualVirtualAddress_Mmap_RelativeOffset_ZeroCopy_In_Pilot()
    {
        string code = @"
            var view1 = shared_buffer(""pilot_dual_view_m15"", 6, :f64);
            let orig_ptr = view1.data_ptr;
            view1 = for (i in 0..<6) {
                yield (i + 1) * 10.0;
            };
            let ptr_unchanged = view1.data_ptr == orig_ptr;

            // Open a second simultaneous OS mmap virtual view over the same physical pages
            var view2 = view1.open_view();
            let distinct_virtual_bases = view1.data_ptr != view2.data_ptr;

            // Mutate via relative TIter indices on view1; observe immediately through view2 at a different base address
            for (idx in 0..<len(view1)) {
                view1[idx] = view1[idx] + 0.5;
            }

            let view2_sum = for (val in view2) {
                yield val;
            };

            // Zero-copy sub-slice view: data_ptr == view2.data_ptr + 2 * sizeof(f64)
            let sub = view2[2..<5];
            let sub_offset_bytes = sub.data_ptr - view2.data_ptr;
        ";

        AssertAstAndBytecode(code, ctx =>
        {
            Assert.True(ctx.Environment.CurrentScope.Resolve("ptr_unchanged").IsTrue);
            Assert.True(ctx.Environment.CurrentScope.Resolve("distinct_virtual_bases").IsTrue);

            var v1 = ctx.Environment.CurrentScope.Resolve("view1").AsSliceBuffer();
            var v2 = ctx.Environment.CurrentScope.Resolve("view2").AsSliceBuffer();
            Assert.Equal(0, v1.ManagedListAllocations);
            Assert.Equal(6, v1.DirectYieldWrites);
            Assert.NotEqual(v1.DataPointer, v2.DataPointer);
            Assert.Equal(new[] { 10.5, 20.5, 30.5, 40.5, 50.5, 60.5 }, v2.AsSpan<double>().ToArray());

            // Stride of f64 is 8 bytes -> offset of index 2 is 16 bytes
            Assert.Equal(16L, ctx.Environment.CurrentScope.Resolve("sub_offset_bytes").AsInt());
            var sub = ctx.Environment.CurrentScope.Resolve("sub").AsSliceBuffer();
            Assert.Equal(new[] { 30.5, 40.5, 50.5 }, sub.AsSpan<double>().ToArray());
        });
    }

    [Fact]
    public unsafe void Test_15_3_3_ZeroList_SliceBuffer_CollectionYield_And_Live_24Byte_AbiHeader()
    {
        string code = @"
            let fixed_buf: SliceBuffer[f64] = for (i in 0..<8) {
                yield i * 2.5;
            };

            let dyn_buf: SliceBuffer[i64] = for (i in 1..5) {
                yield i * 10;
                if i == 2 {
                    continue 100..101, ...;
                }
            };

            let sizeof_sb = sizeof_type(""SliceBuffer[f64]"");
            let sizeof_rw32 = sizeof_type(""RangeWindow[i32]"");
            let sizeof_rw64 = sizeof_type(""RangeWindow[i64]"");
        ";

        var analysis = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.DoesNotContain(analysis.Diagnostics, d => d.IsError);

        AssertAstAndBytecode(code, ctx =>
        {
            Assert.Equal(24L, ctx.Environment.CurrentScope.Resolve("sizeof_sb").AsInt());
            Assert.Equal(8L, ctx.Environment.CurrentScope.Resolve("sizeof_rw32").AsInt());
            Assert.Equal(16L, ctx.Environment.CurrentScope.Resolve("sizeof_rw64").AsInt());

            var fixedBuf = ctx.Environment.CurrentScope.Resolve("fixed_buf").AsSliceBuffer();
            Assert.Equal(0, fixedBuf.ManagedListAllocations);
            Assert.Equal(8, fixedBuf.DirectYieldWrites);
            Assert.Equal(8, fixedBuf.Length);
            Assert.Equal(8, fixedBuf.Capacity);

            // Inspect the live unmanaged 24-byte C-ABI SliceBuffer<double> header in memory
            byte* hdr = (byte*)fixedBuf.HeaderPointer;
            Assert.Equal(fixedBuf.DataPointer, *(IntPtr*)(hdr + 0));
            Assert.Equal((nuint)8, *(nuint*)(hdr + 8));
            Assert.Equal((nuint)8, *(nuint*)(hdr + 16));

            // Zero-copy C# Span<double> mutation visible immediately in Pilot SliceBuffer
            Span<double> span = fixedBuf.AsSpan<double>();
            Assert.Equal(new[] { 0.0, 2.5, 5.0, 7.5, 10.0, 12.5, 15.0, 17.5 }, span.ToArray());
            span[3] = 777.25;
            Assert.Equal(777.25, fixedBuf.GetElement(3).AsFloat());

            // Verify WorklistStack + dynamic unmanaged Realloc SliceBuffer[i64]
            var dynBuf = ctx.Environment.CurrentScope.Resolve("dyn_buf").AsSliceBuffer();
            Assert.Equal(0, dynBuf.ManagedListAllocations);
            Assert.Equal(new long[] { 10, 20, 1000, 1010, 30, 40, 50 }, dynBuf.AsSpan<long>().ToArray());
        });
    }

    [Fact]
    public void Test_15_3_CrossLanguage_Contiguous_ZeroCopy_Sharing_Python_Node_C_Rust_CSharp_Asm()
    {
        string code = @"
            var shm = shared_buffer(""pilot_polyglot_contig_m15"", 6, :f64);
            let initial_ptr = shm.data_ptr;

            // 1. Pilot zero-List CollectionYield writes directly into mmap pages [10, 20, 30, 40, 50, 60]
            shm = for (i in 0..<6) {
                yield (i + 1) * 10.0;
            };

            // 2. Python attaches via OS mmap + memoryview (zero element marshalling)
            let py_sum = python """"""
                mv = memoryview(shm)
                mv[0] = mv[0] + 5.5
                mv[1] = mv[1] * 2.0
                return mv[0] + mv[1]
            """""";

            // 3. Node.js attaches via Float64Array view over the shared mapping
            let js_val = js """"""
                shm[2] = shm[2] + 3.25;
                return shm[2];
            """""";

            // 4. C mutates directly through raw double* DataPtr
            let c_val = c """"""
                double* p = (double*)shm.data_ptr;
                p[3] += 7.75;
                return p[3];
            """""";

            // 5. Rust mutates directly through mutable slice borrow
            let rs_val = rust """"""
                let s = &mut shm;
                s[4] = s[4] * 3.0;
                return s[4];
            """""";

            // 6. C# mutates directly through Span<double>
            let cs_val = csharp """"""
                Span<double> sp = shm;
                sp[5] += 40.0;
                return sp[5];
            """""";

            // 7. x86_64 Assembly mutates directly at [DataPtr + 0*8] using SSE2 addsd
            asm """"""
                mov rax, shm
                mov rcx, 0
                movsd xmm0, 100.0
                addsd [rax + rcx*8], xmm0
                ret
            """""";

            let ptr_never_moved = shm.data_ptr == initial_ptr;
        ";

        AssertAstAndBytecode(code, ctx =>
        {
            Assert.True(ctx.Environment.CurrentScope.Resolve("ptr_never_moved").IsTrue);
            Assert.Equal(55.5, ctx.Environment.CurrentScope.Resolve("py_sum").AsFloat(), 5);
            Assert.Equal(33.25, ctx.Environment.CurrentScope.Resolve("js_val").AsFloat(), 5);
            Assert.Equal(47.75, ctx.Environment.CurrentScope.Resolve("c_val").AsFloat(), 5);
            Assert.Equal(150.0, ctx.Environment.CurrentScope.Resolve("rs_val").AsFloat(), 5);
            Assert.Equal(100.0, ctx.Environment.CurrentScope.Resolve("cs_val").AsFloat(), 5);

            var shmVal = ctx.Environment.CurrentScope.Resolve("shm");
            var shm = shmVal.AsSliceBuffer();
            Assert.Equal(0, shm.ManagedListAllocations);
            Assert.Equal(new[] { 115.5, 40.0, 33.25, 47.75, 150.0, 100.0 }, shm.AsSpan<double>().ToArray());

            // Verify PolyglotTypeBridge zero-copy pass-through (same underlying reference and DataPtr)
            var foreignObj = Pilot.Runtime.Interop.PolyglotTypeBridge.Default.ToForeignValue(shmVal);
            Assert.Same(shm, foreignObj);

            var typedSlice = (SliceBuffer<double>)Pilot.Runtime.Interop.PolyglotTypeBridge.Default.ToForeignValue(
                shmVal, typeof(SliceBuffer<double>))!;
            unsafe
            {
                Assert.Equal(shm.DataPointer, (IntPtr)typedSlice.DataPtr);
            }
            Assert.Equal((nuint)6, typedSlice.Length);
        });
    }
}
