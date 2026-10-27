using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Pilot.Native;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.NativeMemory;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

public class PilotBurstAndNativeTypesTests
{
    private static (PilotExecutionResult Result, PilotScope Scope) RunScript(string code)
    {
        var engine = new PilotEngine();
        var res = engine.Execute(code);
        Assert.True(res.Success, string.Join("; ", res.Diagnostics));
        return (res, res.Context!.Environment.CurrentScope);
    }

    [Fact]
    public void Test_NativeArray_Basics_Indexing_And_Methods()
    {
        string code = @"
let arr = natarray(5)
arr[0] = 10.0
arr[1] = 20.0
arr[2] = 30.0
let arr_len = arr.length
let sumVal = arr.sum()
let list = arr.to_list()
";
        var (_, scope) = RunScript(code);

        Assert.Equal(5, scope.Resolve("arr_len").AsInt());
        Assert.Equal(60.0, scope.Resolve("sumVal").AsFloat(), 4);
        var list = scope.Resolve("list").AsList();
        Assert.Equal(5, list.Count);
        Assert.Equal(10.0, list[0].AsFloat(), 4);
        Assert.Equal(20.0, list[1].AsFloat(), 4);
    }

    [Fact]
    public void Test_NativeArray_Dot_Product_And_Vector_Scale()
    {
        string code = @"
let a = natarray([1.0, 2.0, 3.0, 4.0])
let b = natarray([2.0, 3.0, 4.0, 5.0])
let dotVal = a.dot(b)
a.scale(2.0)
let scaledSum = a.sum()
";
        var (_, scope) = RunScript(code);

        // dot = 1*2 + 2*3 + 3*4 + 4*5 = 2 + 6 + 12 + 20 = 40.0
        Assert.Equal(40.0, scope.Resolve("dotVal").AsFloat(), 4);
        // sum = (1+2+3+4)*2 = 20.0
        Assert.Equal(20.0, scope.Resolve("scaledSum").AsFloat(), 4);
    }

    [Fact]
    public void Test_NativeArray_Conversion_From_List()
    {
        string code = @"
let standardList = [10.5, 20.5, 30.5]
let nat1 = standardList.to_native()
let nat2 = to_native(standardList)
let s1 = nat1.sum()
let s2 = nat2.sum()
";
        var (_, scope) = RunScript(code);

        Assert.Equal(61.5, scope.Resolve("s1").AsFloat(), 4);
        Assert.Equal(61.5, scope.Resolve("s2").AsFloat(), 4);
    }

    [Fact]
    public void Test_NativeMap_Basics_And_Conversions()
    {
        string code = @"
let m = natmap()
m[""alpha""] = 100
m[""beta""] = 200
let hasAlpha = m.has(""alpha"")
let hasGamma = m.has(""gamma"")
let alphaVal = m[""alpha""]
let map_len = m.length
";
        var (_, scope) = RunScript(code);

        Assert.True(scope.Resolve("hasAlpha").IsTrue);
        Assert.False(scope.Resolve("hasGamma").IsTrue);
        Assert.Equal(100, scope.Resolve("alphaVal").AsInt());
        Assert.Equal(2, scope.Resolve("map_len").AsInt());
    }

    [Fact]
    public void Test_Burst_Decorator_Compiles_And_Executes_Kernel()
    {
        string code = @"
@burst
def burst_vector_compute(count: int, factor: float) -> float {
    let arr = natarray(count)
    arr.fill(factor)
    return arr.sum()
}

let result = burst_vector_compute(1000, 2.5)
";
        var (_, scope) = RunScript(code);

        var fnVal = scope.Resolve("burst_vector_compute");
        Assert.Equal(PilotValueKind.Function, fnVal.Kind);
        Assert.True(fnVal.AsFunction().IsBurstCompiled);
        Assert.NotNull(fnVal.AsFunction().BurstKernel);

        // 1000 * 2.5 = 2500.0
        Assert.Equal(2500.0, scope.Resolve("result").AsFloat(), 4);
    }

    [Fact]
    public void Test_Assembly_Emission_Produces_Vector_Instructions()
    {
        string code = @"
def vector_kernel(a: float, b: float) -> float {
    return a * b + 42.0
}
";
        var options = new PilotNativeOptions
        {
            Target = NativeCompilationTarget.Assembly,
            ModuleName = "burst_test",
            OptimizationLevel = 3,
            BurstOptimized = true
        };

        var result = PilotNativeCompiler.Compile(code, options);
        Assert.True(result.Success);
        Assert.NotNull(result.OutputCode);

        string asm = result.OutputCode;
        Assert.Contains(".globl", asm);
        Assert.Contains("vector_kernel", asm);
        Assert.Contains("Pilot Burst Native Compilation Engine", asm);
        Assert.True(asm.Contains("vaddpd") || asm.Contains("addsd") || asm.Contains("vmulpd") || asm.Contains("mulsd"));
    }
}
