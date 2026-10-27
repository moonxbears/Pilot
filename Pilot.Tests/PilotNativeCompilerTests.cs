#nullable disable
using System;
using System.IO;
using Pilot.Cli;
using Pilot.Native;
using Pilot.Native.CAbi;
using Pilot.Native.Llvm;
using Pilot.Native.Wasm;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Unit tests for Milestone 2: Native Ahead-Of-Time (AOT) Compilation &amp; LLVM/Wasm Backend.
/// </summary>
public class PilotNativeCompilerTests
{
    [Fact]
    public void Test_Native_Llvm_Ir_Generation_Arithmetic_And_Print()
    {
        string code = @"
            let x = 10;
            let y = 20;
            let sum = x + y * 2;
            print(sum);
        ";

        string llvmIr = PilotLlvmEmitter.Emit(code, "test_arithmetic");

        Assert.NotEmpty(llvmIr);
        Assert.Contains("; ModuleID = 'test_arithmetic'", llvmIr);
        Assert.Contains("target triple", llvmIr);
        Assert.Contains("declare i32 @printf", llvmIr);
        Assert.Contains("define i32 @main(i32 %argc, ptr %argv)", llvmIr);
        Assert.Contains("mul i64", llvmIr);
        Assert.Contains("add i64", llvmIr);
        Assert.Contains("call i32 (ptr, ...) @printf", llvmIr);
        Assert.Contains("ret i32 0", llvmIr);
    }

    [Fact]
    public void Test_Native_Llvm_Ir_Function_Definitions_And_Calls()
    {
        string code = @"
            def square(n: i64) -> i64 {
                return n * n;
            }

            def fib(n: i64) -> i64 {
                if (n <= 1) {
                    return n;
                }
                return fib(n - 1) + fib(n - 2);
            }

            let result = square(fib(6));
            print(result);
        ";

        string llvmIr = PilotLlvmEmitter.Emit(code, "test_funcs");

        Assert.NotEmpty(llvmIr);
        Assert.Contains("define i64 @square(i64 %arg.n)", llvmIr);
        Assert.Contains("define i64 @fib(i64 %arg.n)", llvmIr);
        Assert.Contains("icmp sle i64", llvmIr);
        Assert.Contains("br i1", llvmIr);
        Assert.Contains("call i64 @fib", llvmIr);
        Assert.Contains("call i64 @square", llvmIr);
        Assert.Contains("ret i64", llvmIr);
    }

    [Fact]
    public void Test_Native_Llvm_Ir_While_And_For_Loops()
    {
        string code = @"
            let total = 0;
            let i = 0;
            while (i < 100) {
                total = total + i;
                i = i + 1;
            }

            for j in 0..50 {
                total = total + j;
            }

            print(total);
        ";

        string llvmIr = PilotLlvmEmitter.Emit(code, "test_loops");

        Assert.NotEmpty(llvmIr);
        Assert.Contains("while.cond", llvmIr);
        Assert.Contains("while.body", llvmIr);
        Assert.Contains("while.end", llvmIr);
        Assert.Contains("for.cond", llvmIr);
        Assert.Contains("for.body", llvmIr);
        Assert.Contains("for.step", llvmIr);
        Assert.Contains("for.end", llvmIr);
    }

    [Fact]
    public void Test_Native_WebAssembly_Wat_Emission()
    {
        string code = @"
            def add(a: i64, b: i64) -> i64 {
                return a + b;
            }

            let x = 15;
            let y = 25;
            let res = add(x, y);
            print(res);
        ";

        string wat = PilotWasmEmitter.EmitWat(code, targetWasi: true);

        Assert.NotEmpty(wat);
        Assert.Contains("(module", wat);
        Assert.Contains("(import \"env\" \"print_i64\"", wat);
        Assert.Contains("(memory (export \"memory\") 1)", wat);
        Assert.Contains("(func $add (export \"add\") (param $a i64) (param $b i64) (result i64)", wat);
        Assert.Contains("i64.add", wat);
        Assert.Contains("(func $main (export \"main\") (result i32)", wat);
        Assert.Contains("(func $_start (export \"_start\")", wat);
        Assert.Contains("call $add", wat);
        Assert.Contains("call $main", wat);
    }

    [Fact]
    public void Test_Native_WebAssembly_Binary_Wasm_Emission()
    {
        string code = @"
            let a = 100;
            let b = 200;
            let c = a + b;
        ";

        byte[] wasmBytes = PilotWasmEmitter.EmitWasm(code, targetWasi: true);

        Assert.NotNull(wasmBytes);
        Assert.True(wasmBytes.Length >= 8);

        // Verify WASM magic header: \0asm
        Assert.Equal(0x00, wasmBytes[0]);
        Assert.Equal(0x61, wasmBytes[1]); // 'a'
        Assert.Equal(0x73, wasmBytes[2]); // 's'
        Assert.Equal(0x6D, wasmBytes[3]); // 'm'

        // Verify WASM version 1
        Assert.Equal(0x01, wasmBytes[4]);
        Assert.Equal(0x00, wasmBytes[5]);
        Assert.Equal(0x00, wasmBytes[6]);
        Assert.Equal(0x00, wasmBytes[7]);
    }

    [Fact]
    public void Test_Native_CAbi_Header_Export()
    {
        string code = @"
            class Vector3 {
                var x: float;
                var y: float;
                var z: float;
            }

            def compute_dot(a_x: float, a_y: float, b_x: float, b_y: float) -> float {
                return a_x * b_x + a_y * b_y;
            }

            def compute_norm(val: int) -> int {
                return val * val;
            }
        ";

        string header = PilotCAbiEmitter.EmitHeader(code, "vector_math");

        Assert.NotEmpty(header);
        Assert.Contains("#ifndef VECTOR_MATH_H", header);
        Assert.Contains("#define VECTOR_MATH_H", header);
        Assert.Contains("extern \"C\" {", header);
        Assert.Contains("typedef struct Vector3 {", header);
        Assert.Contains("double x;", header);
        Assert.Contains("double y;", header);
        Assert.Contains("double z;", header);
        Assert.Contains("double compute_dot(double a_x, double a_y, double b_x, double b_y);", header);
        Assert.Contains("int64_t compute_norm(int64_t val);", header);
        Assert.Contains("#endif /* VECTOR_MATH_H */", header);
    }

    [Fact]
    public void Test_Native_Compiler_CompileFile_Pipeline()
    {
        string tempDir = Path.Combine(Path.GetTempPath(), $"pilot_test_aot_{Guid.NewGuid():N}");
        Directory.CreateDirectory(tempDir);

        try
        {
            string pilotFile = Path.Combine(tempDir, "sample.pilot");
            File.WriteAllText(pilotFile, @"
                def factorial(n: i64) -> i64 {
                    if (n <= 1) { return 1; }
                    return n * factorial(n - 1);
                }
                let res = factorial(5);
                print(res);
            ");

            // 1. LLVM IR output
            string llOut = Path.Combine(tempDir, "sample.ll");
            var llvmRes = PilotNativeCompiler.CompileFile(pilotFile, new PilotNativeOptions
            {
                Target = NativeCompilationTarget.LlvmIr,
                OutputPath = llOut
            });
            Assert.True(llvmRes.Success);
            Assert.True(File.Exists(llOut));
            Assert.Contains("@factorial", File.ReadAllText(llOut));

            // 2. WebAssembly Text (WAT) output
            string watOut = Path.Combine(tempDir, "sample.wat");
            var watRes = PilotNativeCompiler.CompileFile(pilotFile, new PilotNativeOptions
            {
                Target = NativeCompilationTarget.WebAssemblyText,
                OutputPath = watOut
            });
            Assert.True(watRes.Success);
            Assert.True(File.Exists(watOut));
            Assert.Contains("(func $factorial", File.ReadAllText(watOut));

            // 3. WebAssembly Binary (WASM) output
            string wasmOut = Path.Combine(tempDir, "sample.wasm");
            var wasmRes = PilotNativeCompiler.CompileFile(pilotFile, new PilotNativeOptions
            {
                Target = NativeCompilationTarget.WebAssemblyBinary,
                OutputPath = wasmOut
            });
            Assert.True(wasmRes.Success);
            Assert.True(File.Exists(wasmOut));
            byte[] bytes = File.ReadAllBytes(wasmOut);
            Assert.Equal(0x00, bytes[0]);
            Assert.Equal(0x61, bytes[1]);

            // 4. C-ABI Header output
            string hOut = Path.Combine(tempDir, "sample.h");
            var cabiRes = PilotNativeCompiler.CompileFile(pilotFile, new PilotNativeOptions
            {
                Target = NativeCompilationTarget.CAbiHeader,
                OutputPath = hOut
            });
            Assert.True(cabiRes.Success);
            Assert.True(File.Exists(hOut));
            Assert.Contains("int64_t factorial(int64_t n);", File.ReadAllText(hOut));
        }
        finally
        {
            if (Directory.Exists(tempDir))
            {
                try { Directory.Delete(tempDir, true); } catch { }
            }
        }
    }

    [Fact]
    public void Test_Native_Cli_Build_Subcommand()
    {
        string tempDir = Path.Combine(Path.GetTempPath(), $"pilot_cli_build_{Guid.NewGuid():N}");
        Directory.CreateDirectory(tempDir);

        try
        {
            string pilotFile = Path.Combine(tempDir, "hello.pilot");
            File.WriteAllText(pilotFile, "print('Hello Native AOT');");

            string llOut = Path.Combine(tempDir, "hello.ll");
            int exitCode = Program.Main(new[] { "build", pilotFile, "--target", "llvm", "-o", llOut });

            Assert.Equal(0, exitCode);
            Assert.True(File.Exists(llOut));
            Assert.Contains("@printf", File.ReadAllText(llOut));
        }
        finally
        {
            if (Directory.Exists(tempDir))
            {
                try { Directory.Delete(tempDir, true); } catch { }
            }
        }
    }
}
