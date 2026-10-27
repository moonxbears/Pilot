using System.Linq;
using Pilot.Analysis;
using Pilot.Analysis.TypeSystem;
using Pilot.LanguageServer.Semantics;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Unit test suite for Milestone 3:
/// - Flow-Sensitive Bidirectional Type Inference
/// - Affine & Linear Resource Borrow Checking (preserving Python/C#-like simplicity)
/// - Static Physical Units-of-Measure Dimensional Algebra
/// - Design-by-Contract Compile-Time Predicate Verification
/// </summary>
public class PilotTypeInferenceAndBorrowCheckingTests
{
    [Fact]
    public void Test_Scripting_Code_Preserves_Python_CSharp_Simplicity()
    {
        // Standard casual scripting code: dynamic collections, variables, reassignments,
        // and functions run with full reference semantics and zero borrow checking friction.
        string script = @"
var count = 0
for i in [1, 2, 3, 4, 5] {
    count += i
}

let user = { ""name"": ""Pilot"", ""tags"": [""aot"", ""realtime""] }
let greeting = ""Hello, "" + user[""name""]
let numbers = [10, 20, 30]
numbers.append(40)
";
        var result = PilotAnalysisPipeline.AnalyzeSource(script);
        Assert.False(result.HasErrors);
        Assert.DoesNotContain(result.Diagnostics, d => d.Code.StartsWith("PILOT3"));
    }

    [Fact]
    public void Test_FlowSensitive_Bidirectional_Type_Inference()
    {
        string code = @"
let intVal = 100
let floatVal = 3.14
let strVal = ""pilot""
let listVal = [1, 2, 3]

def compute(x: int) -> int {
    return x * 2
}
";
        var result = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.False(result.HasErrors);

        Assert.Equal(PilotType.Int, result.InferredTypes["intVal"]);
        Assert.Equal(PilotType.Float, result.InferredTypes["floatVal"]);
        Assert.Equal(PilotType.String, result.InferredTypes["strVal"]);
        Assert.Equal(PilotType.ListOf(PilotType.Int), result.InferredTypes["listVal"]);
    }

    [Fact]
    public void Test_Inferred_Type_Mismatch_Caught_As_PILOT300()
    {
        string code = @"
let badNum: int = ""should_be_int""
";
        var result = PilotAnalysisPipeline.AnalyzeSource(code);
        if (!result.HasErrors)
        {
            throw new Exception($"Diagnostics count: {result.Diagnostics.Count}, details: {string.Join(" | ", result.Diagnostics.Select(d => $"{d.Code}:{d.Message}"))}");
        }
        Assert.True(result.HasErrors);
        Assert.Contains(result.Diagnostics, d => d.Code == "PILOT300" && d.Message.Contains("badNum"));
    }

    [Fact]
    public void Test_Affine_Resource_Use_After_Move_Caught_As_PILOT301()
    {
        string code = @"
@resource struct Buffer { id: int }
let buffer = Buffer()

let target = take(buffer)
let invalidAccess = buffer + 5
";
        var result = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.True(result.HasErrors);
        var moveErrors = result.Diagnostics.Where(d => d.Code == "PILOT301").ToList();
        Assert.NotEmpty(moveErrors);
        Assert.Contains(moveErrors, e => e.Message.Contains("buffer") && e.Message.Contains("moved"));
    }

    [Fact]
    public void Test_Linear_Resource_Must_Be_Consumed_Before_Scope_Exit_PILOT303()
    {
        // txn is declared @linear but never closed/consumed
        string codeUnconsumed = @"
@linear struct Transaction { id: int }
def handle_db() {
    let txn = Transaction()
    print(""done without closing txn"")
}
";
        var resultUnconsumed = PilotAnalysisPipeline.AnalyzeSource(codeUnconsumed);
        Assert.True(resultUnconsumed.HasErrors);
        Assert.Contains(resultUnconsumed.Diagnostics, d => d.Code == "PILOT303" && d.Message.Contains("txn"));

        // txn is properly closed/consumed
        string codeConsumed = @"
@linear struct Transaction { id: int }
def handle_db_valid() {
    let txn = Transaction()
    txn.close()
}
";
        var resultConsumed = PilotAnalysisPipeline.AnalyzeSource(codeConsumed);
        Assert.DoesNotContain(resultConsumed.Diagnostics, d => d.Code == "PILOT303");
    }

    [Fact]
    public void Test_Zero_Allocation_Enforced_In_HardRealtime_Scope_PILOT304()
    {
        string code = @"
@hard_realtime def flight_control_tick() {
    let list = [1, 2, 3]
    list.append(4)
}
";
        var result = PilotAnalysisPipeline.AnalyzeSource(code);
        if (!result.HasErrors)
        {
            throw new Exception($"Diagnostics count: {result.Diagnostics.Count}, details: {string.Join(" | ", result.Diagnostics.Select(d => $"{d.Code}:{d.Message}"))}");
        }
        Assert.True(result.HasErrors);
        Assert.Contains(result.Diagnostics, d => d.Code == "PILOT304" && d.Message.Contains("allocation"));
    }

    [Fact]
    public void Test_Static_Unit_Of_Measure_Dimensional_Algebra_PILOT305()
    {
        string validCode = @"
unit meter
unit second
unit speed = meter / second

let d: float<meter> = 100.0<meter>
let t: float<second> = 5.0<second>
let v: float<speed> = d / t
";
        var validResult = PilotAnalysisPipeline.AnalyzeSource(validCode);
        Assert.DoesNotContain(validResult.Diagnostics, d => d.Code == "PILOT305");

        string mismatchCode = @"
unit meter
unit second

let d: float<meter> = 100.0<meter>
let t: float<second> = 5.0<second>
let badSum = d + t
";
        var mismatchResult = PilotAnalysisPipeline.AnalyzeSource(mismatchCode);
        Assert.True(mismatchResult.HasErrors);
        Assert.Contains(mismatchResult.Diagnostics, d => d.Code == "PILOT305");
    }

    [Fact]
    public void Test_Static_Contract_Precondition_Violation_PILOT306()
    {
        string code = @"
def sqrt_positive(x: int) requires x >= 0 {
    return x
}

sqrt_positive(-10)
";
        var result = PilotAnalysisPipeline.AnalyzeSource(code);
        if (!result.HasErrors)
        {
            throw new Exception($"Diagnostics count: {result.Diagnostics.Count}, details: {string.Join(" | ", result.Diagnostics.Select(d => $"{d.Code}:{d.Message}"))}");
        }
        Assert.True(result.HasErrors);
        Assert.Contains(result.Diagnostics, d => d.Code == "PILOT306" && d.Message.Contains("sqrt_positive"));
    }

    [Fact]
    public void Test_Milestone3_Diagnostics_Surface_In_Semantic_Analyzer()
    {
        string code = @"
@resource struct Buffer { id: int }
let res = Buffer()
let res2 = take(res)
let readRes = res
";
        var analysis = PilotSemanticAnalyzer.Analyze(code);
        var moveErrors = analysis.Diagnostics.Where(d => d.Code == "PILOT301").ToList();
        Assert.NotEmpty(moveErrors);
        Assert.Contains(moveErrors, e => e.Message.Contains("res"));
    }
}
