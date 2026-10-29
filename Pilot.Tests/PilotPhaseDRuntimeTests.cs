using System;
using System.IO;
using Pilot.Compiler;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Milestone 34 verification suite:
/// - 34.1: 16-Byte Unboxed C-ABI Value Representation (NativePilotVal) with SSO up to 14 UTF-8 bytes.
/// - 34.2: 3-Tier Deterministic Memory Engine (Bump Region + Perceus ARC Header).
/// - 34.3: Register-Based PilVM v3 & .pilc v3 Binary Format Execution.
/// </summary>
public class PilotPhaseDRuntimeTests
{
    [Fact]
    public void UnboxedPilotVal_Arithmetic_And_SmallStringOptimization_ZeroAlloc()
    {
        // 1. Integer unboxed register value
        var a = NativeAstBridge.CreatePilotValInt(100);
        var b = NativeAstBridge.CreatePilotValInt(250);
        var sum = NativeAstBridge.AddPilotVal(a, b);

        Assert.True(sum.IsInt);
        Assert.Equal(350, sum.AsInt);
        Assert.Equal("350", sum.ToString());

        // 2. Float unboxed register value
        var fa = NativeAstBridge.CreatePilotValFloat(1.25);
        var fb = NativeAstBridge.CreatePilotValFloat(2.50);
        var fsum = NativeAstBridge.AddPilotVal(fa, fb);

        Assert.True(fsum.IsFloat);
        Assert.Equal(3.75, fsum.AsFloat);

        // 3. Small String Optimization (SSO <= 14 bytes)
        var ssoVal = NativeAstBridge.CreatePilotValString("hello_sso");
        Assert.True(ssoVal.IsSsoStr);
        Assert.True(NativeAstBridge.PilotValIsSso(ssoVal));
        Assert.Equal("hello_sso", ssoVal.ToString());

        // 4. Exact 14-byte string
        var sso14 = NativeAstBridge.CreatePilotValString("12345678901234");
        Assert.True(sso14.IsSsoStr);
        Assert.Equal("12345678901234", sso14.ToString());
    }

    [Fact]
    public void RegisterBasedPilVM_Executes_PilcV3BinaryFormat()
    {
        using var vm = NativeAstBridge.CreateVm();

        // Build .pilc v3 binary:
        // Magic "PIL3" (4 bytes)
        // Version 3 (4 bytes)
        // Constants count = 2 (4 bytes)
        // Constant 0: PilotVal Int 15 (16 bytes)
        // Constant 1: PilotVal Int 27 (16 bytes)
        // Instructions count = 4 (4 bytes)
        // Ins 0: LoadConst r1, 0, 0   (Opcode 9 = LoadConst)
        // Ins 1: LoadConst r2, 0, 1
        // Ins 2: AddI64 r3, r1, r2    (Opcode 1 = AddI64)
        // Ins 3: Ret 0, r3, 0         (Opcode 17 = Ret)

        using var ms = new MemoryStream();
        using var bw = new BinaryWriter(ms);

        // Header
        bw.Write(new byte[] { 0x50, 0x49, 0x4C, 0x33 }); // "PIL3"
        bw.Write((uint)3); // Version 3

        // Constants
        bw.Write((uint)2); // 2 constants
        // Constant 0: TAG_INT = 3, payload = 15
        bw.Write((ulong)3); // tag_meta
        bw.Write((ulong)15); // payload
        // Constant 1: TAG_INT = 3, payload = 27
        bw.Write((ulong)3); // tag_meta
        bw.Write((ulong)27); // payload

        // Instructions
        bw.Write((uint)4); // 4 instructions
        // [opcode:8 | dst:8 | src1:8 | src2:8]
        bw.Write((uint)((9 << 24) | (1 << 16) | (0 << 8) | 0)); // LoadConst r1, 0
        bw.Write((uint)((9 << 24) | (2 << 16) | (0 << 8) | 1)); // LoadConst r2, 1
        bw.Write((uint)((1 << 24) | (3 << 16) | (1 << 8) | 2)); // AddI64 r3, r1, r2
        bw.Write((uint)((17 << 24) | (0 << 16) | (3 << 8) | 0)); // Ret r3

        byte[] pilcBytes = ms.ToArray();

        var res = vm.ExecuteBytes(pilcBytes);
        Assert.True(res.IsInt);
        Assert.Equal(42, res.AsInt);
    }
}
