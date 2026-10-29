using System;
using Pilot.Compiler;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Milestone 35 verification suite:
/// - 35.1: Unified Native AOT Driver (pilot build) emitting LLVM IR, C11/C23, and Binary WASM/WASI (crates/pilot-codegen).
/// - 35.2: Native Rust Bidirectional Transpilers for 7 languages (C, C++, C#, Rust, Python, JS, Lua) & SPDX license verification.
/// - 35.3: Zero-.NET Polyglot Interop & Out-of-Process External Polyglot Bridge (crates/pilot-polyglot).
/// </summary>
public class PilotPhaseDCodegenTests
{
    [Fact]
    public void NativeCodegen_BuildsAllTargets_Llvm_C_And_Wasm()
    {
        // Target 0: LLVM IR
        string llvmIr = NativeAstBridge.BuildModule("test_app", 0);
        Assert.False(string.IsNullOrWhiteSpace(llvmIr));
        Assert.Contains("; ModuleID = 'test_app'", llvmIr);
        Assert.Contains("target triple =", llvmIr);
        Assert.Contains("define i64 @main()", llvmIr);
        Assert.Contains("ret i64 0", llvmIr);

        // Target 1: C11 / C23 source
        string cCode = NativeAstBridge.BuildModule("test_app", 1);
        Assert.False(string.IsNullOrWhiteSpace(cCode));
        Assert.Contains("#include <stdint.h>", cCode);
        Assert.Contains("int main(void)", cCode);
        Assert.Contains("return 0;", cCode);

        // Target 2: WebAssembly WAT / WASI binary representation
        string wasmWat = NativeAstBridge.BuildModule("test_app", 2);
        Assert.False(string.IsNullOrWhiteSpace(wasmWat));
        Assert.Contains("(module", wasmWat);
        Assert.Contains("(func $main (export \"_start\")", wasmWat);
        Assert.Contains("(result i64)", wasmWat);
        Assert.Contains("i64.const 0", wasmWat);
    }

    [Fact]
    public void NativeTranspiler_EmitsAllSevenTargets()
    {
        // Target 0: C
        string c = NativeAstBridge.TranspileSample(0);
        Assert.Contains("#include <stdint.h>", c);
        Assert.Contains("int64_t compute(int64_t x)", c);
        Assert.Contains("return (x * 2);", c);

        // Target 1: C++
        string cpp = NativeAstBridge.TranspileSample(1);
        Assert.Contains("#include <cstdint>", cpp);
        Assert.Contains("int64_t compute(int64_t x)", cpp);
        Assert.Contains("return (x * 2);", cpp);

        // Target 2: C#
        string csharp = NativeAstBridge.TranspileSample(2);
        Assert.Contains("public static class TranspiledMod", csharp);
        Assert.Contains("public static long compute(long x)", csharp);
        Assert.Contains("return (x * 2);", csharp);

        // Target 3: Rust
        string rust = NativeAstBridge.TranspileSample(3);
        Assert.Contains("pub fn compute(x: i64) -> i64", rust);
        Assert.Contains("(x * 2)", rust);

        // Target 4: Python
        string python = NativeAstBridge.TranspileSample(4);
        Assert.Contains("def compute(x):", python);
        Assert.Contains("return (x * 2)", python);

        // Target 5: JavaScript
        string js = NativeAstBridge.TranspileSample(5);
        Assert.Contains("export function compute(x)", js);
        Assert.Contains("return (x * 2);", js);

        // Target 6: Lua
        string lua = NativeAstBridge.TranspileSample(6);
        Assert.Contains("local function compute(x)", lua);
        Assert.Contains("return (x * 2)", lua);
        Assert.Contains("end", lua);

        // Unknown target index
        string unknown = NativeAstBridge.TranspileSample(99);
        Assert.Empty(unknown);
    }

    [Fact]
    public void NativePolyglot_VerifiesSpdx_And_ExecutesExternalSnippet()
    {
        // Permissive Open-Source SPDX verification (--viable-license-only)
        Assert.True(NativeAstBridge.VerifySpdxLicense("MIT"));
        Assert.True(NativeAstBridge.VerifySpdxLicense("Apache-2.0"));
        Assert.True(NativeAstBridge.VerifySpdxLicense("BSD-2-Clause"));
        Assert.True(NativeAstBridge.VerifySpdxLicense("BSD-3-Clause"));
        Assert.True(NativeAstBridge.VerifySpdxLicense("ISC"));
        Assert.True(NativeAstBridge.VerifySpdxLicense("0BSD"));
        Assert.True(NativeAstBridge.VerifySpdxLicense("Unlicense"));

        // Copyleft or non-permissive SPDX verification fails viable-license-only filter
        Assert.False(NativeAstBridge.VerifySpdxLicense("GPL-3.0"));
        Assert.False(NativeAstBridge.VerifySpdxLicense("AGPL-3.0"));
        Assert.False(NativeAstBridge.VerifySpdxLicense("LGPL-3.0"));
        Assert.False(NativeAstBridge.VerifySpdxLicense("Proprietary"));
        Assert.False(NativeAstBridge.VerifySpdxLicense("UnknownLicense123"));

        // External Polyglot Out-of-Process execution adapter
        string pyRes = NativeAstBridge.PolyglotExecute("python", "print('hello from python')");
        Assert.Contains("chars of python out-of-process", pyRes);

        string csRes = NativeAstBridge.PolyglotExecute("csharp", "System.Console.WriteLine(\"hi\");");
        Assert.Contains("chars of csharp out-of-process", csRes);

        string jsRes = NativeAstBridge.PolyglotExecute("js", "console.log('hi');");
        Assert.Contains("chars of js out-of-process", jsRes);
    }
}
