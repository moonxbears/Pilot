#nullable disable
using System;
using System.IO;
using Pilot.Compiler;
using Pilot.Runtime.Bytecode;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Pilot.Scripting;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Unit tests for Milestone 1: PilVM (Bytecode Virtual Machine &amp; Execution Engine).
/// </summary>
public class PilotVirtualMachineTests
{
    [Fact]
    public void Test_VM_Direct_Bytecode_Stack_Arithmetic()
    {
        var chunk = new PilotBytecodeChunk("direct_arithmetic");
        // Compute: (10 + 20) * 3 - 15 / 3 = 30 * 3 - 5 = 90 - 5 = 85
        chunk.Emit(PilotOpcode.ConstInt, 10);
        chunk.Emit(PilotOpcode.ConstInt, 20);
        chunk.Emit(PilotOpcode.Add);
        chunk.Emit(PilotOpcode.ConstInt, 3);
        chunk.Emit(PilotOpcode.Mul);
        chunk.Emit(PilotOpcode.ConstInt, 15);
        chunk.Emit(PilotOpcode.ConstInt, 3);
        chunk.Emit(PilotOpcode.Div);
        chunk.Emit(PilotOpcode.Sub);
        chunk.Emit(PilotOpcode.Halt);

        var engine = new PilotEngine();
        var context = new PilotExecutionContext(engine, new PilotExecutionOptions());
        var result = PilVirtualMachine.Execute(chunk, context);

        Assert.Equal(85, result.AsInt());
    }

    [Fact]
    public void Test_VM_Local_Slot_Registers_And_Loop_Branching()
    {
        var chunk = new PilotBytecodeChunk("direct_loop");
        int sumSlot = chunk.GetOrAllocateLocalSlot("sum");
        int iSlot = chunk.GetOrAllocateLocalSlot("i");

        // sum = 0
        chunk.Emit(PilotOpcode.ConstInt, 0);
        chunk.Emit(PilotOpcode.StoreLocal, sumSlot);

        // i = 1
        chunk.Emit(PilotOpcode.ConstInt, 1);
        chunk.Emit(PilotOpcode.StoreLocal, iSlot);

        // Loop header: while (i <= 10)
        int loopHead = chunk.Instructions.Count;
        chunk.Emit(PilotOpcode.LoadLocal, iSlot);
        chunk.Emit(PilotOpcode.ConstInt, 10);
        chunk.Emit(PilotOpcode.Le);
        int exitJump = chunk.EmitJump(PilotOpcode.JumpIfFalse);

        // Body: sum += i
        chunk.Emit(PilotOpcode.LoadLocal, sumSlot);
        chunk.Emit(PilotOpcode.LoadLocal, iSlot);
        chunk.Emit(PilotOpcode.Add);
        chunk.Emit(PilotOpcode.StoreLocal, sumSlot);

        // i += 1
        chunk.Emit(PilotOpcode.LoadLocal, iSlot);
        chunk.Emit(PilotOpcode.ConstInt, 1);
        chunk.Emit(PilotOpcode.Add);
        chunk.Emit(PilotOpcode.StoreLocal, iSlot);

        // Jump back
        chunk.Emit(PilotOpcode.Jump, loopHead);

        // Loop exit
        chunk.PatchJump(exitJump);
        chunk.Emit(PilotOpcode.LoadLocal, sumSlot);
        chunk.Emit(PilotOpcode.Halt);

        var engine = new PilotEngine();
        var context = new PilotExecutionContext(engine, new PilotExecutionOptions());
        var result = PilVirtualMachine.Execute(chunk, context);

        // Sum 1..10 = 55
        Assert.Equal(55, result.AsInt());
    }

    [Fact]
    public void Test_VM_Recursive_Function_Execution()
    {
        string code = @"
            def fib(n) {
                if (n <= 1) {
                    return n;
                }
                return fib(n - 1) + fib(n - 2);
            }
            fib(10);
        ";

        var compileResult = PilotCompiler.Default.Compile(code);
        Assert.True(compileResult.Success, string.Join("; ", compileResult.Diagnostics));
        Assert.NotNull(compileResult.Program);
        Assert.NotNull(compileResult.Program.BytecodeChunk);

        var result = compileResult.Program.Run();
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Equal(55, result.Value.AsInt());
    }

    [Fact]
    public void Test_VM_Factorial_And_While_Loop()
    {
        string code = @"
            def factorial(n) {
                let result = 1;
                let cur = n;
                while (cur > 1) {
                    result = result * cur;
                    cur = cur - 1;
                }
                return result;
            }
            factorial(6);
        ";

        var compileResult = PilotCompiler.Default.Compile(code);
        Assert.True(compileResult.Success);
        var result = compileResult.Program.Run();
        Assert.True(result.Success);
        Assert.Equal(720, result.Value.AsInt());
    }

    [Fact]
    public void Test_VM_Nested_Closure_Captures()
    {
        string code = @"
            def make_multiplier(factor) {
                def multiply(x) {
                    return x * factor;
                }
                return multiply;
            }
            let triple = make_multiplier(3);
            triple(7);
        ";

        var compileResult = PilotCompiler.Default.Compile(code);
        Assert.True(compileResult.Success);
        var result = compileResult.Program.Run();
        Assert.True(result.Success);
        Assert.Equal(21, result.Value.AsInt());
    }

    [Fact]
    public void Test_VM_Binary_Serialization_Roundtrip()
    {
        string code = @"
            def compute(base, exponent) {
                let total = 1;
                for i in 0..exponent {
                    total = total * base;
                }
                return total;
            }
            compute(3, 4);
        ";

        var compileResult = PilotCompiler.Default.Compile(code);
        Assert.True(compileResult.Success);
        var originalProgram = compileResult.Program;

        // Serialize to binary .pilc stream
        using var ms = new MemoryStream();
        originalProgram.Save(ms);

        byte[] binaryData = ms.ToArray();
        Assert.NotEmpty(binaryData);

        // Deserialize
        using var msRead = new MemoryStream(binaryData);
        var loadedProgram = PilotCompiledProgram.Load(msRead);

        Assert.NotNull(loadedProgram);
        Assert.NotNull(loadedProgram.BytecodeChunk);

        var runResult = loadedProgram.Run();
        Assert.True(runResult.Success);
        // 3^4 = 81
        Assert.Equal(81, runResult.Value.AsInt());
    }

    [Fact]
    public void Test_VM_Collections_List_Map_Tuple_Set()
    {
        string code = @"
            let list = [1, 2, 3];
            list.append(4);
            let map = { 'a': 10, 'b': 20 };
            let tup = (100, 200);
            let s = { 5, 10, 15 };

            list[3] + map['b'] + tup[0] + s.length;
        ";

        var compileResult = PilotCompiler.Default.Compile(code);
        Assert.True(compileResult.Success);
        var result = compileResult.Program.Run();
        Assert.True(result.Success);
        // list[3]=4, map['b']=20, tup[0]=100, s.length=3 => 4 + 20 + 100 + 3 = 127
        Assert.Equal(127, result.Value.AsInt());
    }

    [Fact]
    public void Test_VM_Short_Circuit_Operators()
    {
        string code = @"
            let side_effect = 0;
            def inc() {
                side_effect = side_effect + 1;
                return true;
            }

            // false && inc() should short-circuit and not execute inc()
            let r1 = false && inc();

            // true || inc() should short-circuit and not execute inc()
            let r2 = true || inc();

            side_effect;
        ";

        var compileResult = PilotCompiler.Default.Compile(code);
        Assert.True(compileResult.Success);
        var result = compileResult.Program.Run();
        Assert.True(result.Success);
        Assert.Equal(0, result.Value.AsInt());
    }

    [Fact]
    public void Test_VM_Bitwise_And_Shift_Operations()
    {
        string code = @"
            let a = 0b1010; // 10
            let b = 0b1100; // 12
            let andVal = a & b; // 8
            let orVal = a | b;  // 14
            let xorVal = a ^ b; // 6
            let shlVal = a << 2; // 40
            let shrVal = b >> 1; // 6

            andVal + orVal + xorVal + shlVal + shrVal;
        ";

        var compileResult = PilotCompiler.Default.Compile(code);
        Assert.True(compileResult.Success);
        var result = compileResult.Program.Run();
        Assert.True(result.Success);
        // 8 + 14 + 6 + 40 + 6 = 74
        Assert.Equal(74, result.Value.AsInt());
    }

    [Fact]
    public void Test_VM_Conditionals_If_Elif_Else()
    {
        string code = @"
            def classify(n) {
                if (n < 0) {
                    return 'negative';
                } elif (n == 0) {
                    return 'zero';
                } elif (n < 10) {
                    return 'small';
                } else {
                    return 'large';
                }
            }

            classify(-5) + '_' + classify(0) + '_' + classify(7) + '_' + classify(100);
        ";

        var compileResult = PilotCompiler.Default.Compile(code);
        Assert.True(compileResult.Success);
        var result = compileResult.Program.Run();
        Assert.True(result.Success);
        Assert.Equal("negative_zero_small_large", result.Value.AsString());
    }

    [Fact]
    public void Test_VM_Execution_Speed_Benchmark()
    {
        string code = @"
            let total = 0;
            let i = 0;
            while (i < 30000) {
                total = total + i;
                i = i + 1;
            }
            total;
        ";

        var compilerResult = PilotCompiler.Default.Compile(code);
        Assert.True(compilerResult.Success);
        var compiledProg = (PilotCompiledProgram)compilerResult.Program;
        var tree = compiledProg.ProgramTree;
        var engine = new PilotEngine();

        // Warmup
        compiledProg.Run();

        // PilVM Bytecode Run (5 runs)
        var swVm = System.Diagnostics.Stopwatch.StartNew();
        for (int r = 0; r < 5; r++)
        {
            var res = compilerResult.Program.Run();
            Assert.True(res.Success);
            Assert.Equal(449985000L, res.Value.AsInt());
        }
        swVm.Stop();

        // Baseline AST Visitor Run (5 runs)
        var swAst = System.Diagnostics.Stopwatch.StartNew();
        for (int r = 0; r < 5; r++)
        {
            var context = new PilotExecutionContext(engine, new PilotExecutionOptions());
            var visitor = context.GetOrCreateVisitor();
            var astRes = visitor.VisitProgram(tree);
            Assert.Equal(449985000L, astRes.AsInt());
        }
        swAst.Stop();

        // VM should outperform AST visitor significantly
        Assert.True(swVm.ElapsedTicks < swAst.ElapsedTicks,
            $"PilVM ({swVm.ElapsedMilliseconds}ms) should be faster than AST visitor ({swAst.ElapsedMilliseconds}ms)");
    }
}
