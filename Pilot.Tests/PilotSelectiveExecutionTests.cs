using System;
using System.IO;
using System.Linq;
using Pilot.Runtime.Values;
using Pilot.Scripting;
using Xunit;

namespace Pilot.Tests;

public class PilotSelectiveExecutionTests
{
    [Fact]
    public void ExecuteFunction_SelectivelyExecutesTarget_WithArguments()
    {
        string code = @"
def add(a: int, b: int) -> int:
    return a + b

def multiply(a: int, b: int) -> int:
    return a * b
";
        var resAdd = PilotScript.RunFunction(code, "add", "15", "25");
        Assert.True(resAdd.Success);
        Assert.Equal(40, resAdd.Value.AsInt());

        var resMul = PilotScript.RunFunction(code, "multiply", "6", "7");
        Assert.True(resMul.Success);
        Assert.Equal(42, resMul.Value.AsInt());
    }

    [Fact]
    public void ExecuteFunction_SkipsTopLevelMainGuards()
    {
        string code = @"
var side_effect_counter = 0

if __name__ == ""__main__"":
    side_effect_counter = 9999

def get_value(bonus: int) -> int:
    return side_effect_counter + bonus
";
        var res = PilotScript.RunFunction(code, "get_value", "50");
        Assert.True(res.Success);
        // __name__ is 'get_value' (not '__main__'), so side_effect_counter was not set to 9999!
        Assert.Equal(50, res.Value.AsInt());
    }

    [Fact]
    public void RunTests_DiscoversDecoratedAndPrefixedTests()
    {
        string code = @"
@test
def verify_addition():
    assert 2 + 3 == 5

def test_multiplication():
    assert 4 * 5 == 20

def helper_utility():
    return 123
";
        var report = PilotScript.RunTests(code);
        Assert.Equal(2, report.TotalCount);
        Assert.Equal(2, report.PassedCount);
        Assert.Equal(0, report.FailedCount);

        var names = report.Results.Select(r => r.TestName).ToList();
        Assert.Contains("verify_addition", names);
        Assert.Contains("test_multiplication", names);
        Assert.DoesNotContain("helper_utility", names);
    }

    [Fact]
    public void RunTests_ReportsFailureAccurately()
    {
        string code = @"
@test
def test_passing():
    assert 1 == 1

@test
def test_failing():
    assert 1 == 2, ""One is not equal to two!""
";
        var report = PilotScript.RunTests(code);
        Assert.Equal(2, report.TotalCount);
        Assert.Equal(1, report.PassedCount);
        Assert.Equal(1, report.FailedCount);

        var failCase = report.Results.First(r => !r.Passed);
        Assert.Equal("test_failing", failCase.TestName);
        Assert.False(string.IsNullOrEmpty(failCase.ErrorMessage));
    }

    [Fact]
    public void RunBenchmarks_MeasuresPerformanceAndIterations()
    {
        string code = @"
@benchmark
def bench_simple_computation():
    var x = 0
    for i in 0..50:
        x = x + i
    return x

def bench_fast_op():
    return 42 * 2
";
        var report = PilotScript.RunBenchmarks(code, iterations: 20);
        Assert.True(report.Results.Count >= 2);
        Assert.All(report.Results, r =>
        {
            Assert.Equal(20, r.Iterations);
            Assert.True(r.TotalTime > TimeSpan.Zero);
            Assert.True(r.OpsPerSecond > 0);
            Assert.False(string.IsNullOrWhiteSpace(r.FormatMeanTime()));
        });
    }

    [Fact]
    public void RunBenchmarks_FilterAppliesRegex()
    {
        string code = @"
@bench
def bench_target_alpha():
    return 1

@bench
def bench_target_beta():
    return 2

@bench
def bench_other():
    return 3
";
        var report = PilotScript.RunBenchmarks(code, filter: "alpha", iterations: 10);
        Assert.Single(report.Results);
        Assert.Equal("bench_target_alpha", report.Results[0].BenchmarkName);
    }

    [Fact]
    public void IsSelective_BuiltinFunction_ReportsSelectiveExecutionState()
    {
        string code = @"
var was_bench = false
var was_selective = false

if is_selective():
    was_selective = true

if is_selective(""bench""):
    was_bench = true

@bench
def bench_probe():
    return was_selective && was_bench
";
        var report = PilotScript.RunBenchmarks(code, iterations: 5);
        Assert.Single(report.Results);
        Assert.True(report.Results[0].Iterations == 5);
    }
}
