using System;
using System.IO;
using System.Linq;
using System.Runtime.CompilerServices;
using Pilot.Analysis;
using Pilot.Analysis.Autodiff;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Effects;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Reactive;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Milestone 18 Acceptance Tests:
/// - Mode 5 (<c>fn!^(a, b) -&gt; Signal[TReturn]</c>): Glitch-free topological Reactive Signals &amp; <c>QUOTE005</c>.
/// - Mode 6 (<c>fn!@(a, b) -&gt; Dual[TReturn, Grad_&lt;FnId&gt;]</c>): Symbolic Adjoint Differentiation, CSE, 12-byte <see cref="Dual_ComputeImpulse"/> struct, <c>QUOTE001</c> &amp; <c>QUOTE004</c>.
/// - Mode 7 (<c>fn!?(a, b) -&gt; EffectPlan[TReturn]</c>): Copy-on-Write Transactional Effect Dry-Run (<c>StoreField</c>, <c>ArrayWrite</c>, <c>DatabaseExec</c>, <c>FileWrite</c>).
/// </summary>
public class PilotSemanticCallProjectionsTests
{
    private static string FormatFailure(PilotExecutionResult result) =>
        string.Join(" | ", result.Diagnostics.Concat(new[] { result.StandardError }).Where(s => !string.IsNullOrWhiteSpace(s)));

    private static (PilotExecutionResult Result, string Output) RunScript(string code)
    {
        var sw = new StringWriter();
        var options = new PilotExecutionOptions { StandardOutput = sw };
        var engine = new PilotEngine();
        var result = engine.Execute(code, options);
        return (result, sw.ToString());
    }

    [Fact]
    public void Case6_SymbolicAdjoint_ComputeImpulse_And_UnboxedStruct_And_Cse()
    {
        Assert.Equal(12, Unsafe.SizeOf<Dual_ComputeImpulse>());

        string code = @"
def ComputeImpulse(mass: float, velocity: float) -> float:
    return 0.5 * mass * (velocity * velocity)

let dual = ComputeImpulse!@(10.0, 5.0)
assert(dual == (125.0, 12.5, 50.0))
let (val, dm, dv) = ComputeImpulse!@(10.0, 5.0)
assert(val == 125.0)
assert(dm == 12.5)
assert(dv == 50.0)
dual
";
        var (res, _) = RunScript(code);
        Assert.True(res.Success, FormatFailure(res));
        Assert.Equal(PilotValueKind.Object, res.Value.Kind);

        var obj = res.Value.AsObject();
        var dualPayload = Assert.IsType<PilotDualValue>(obj.NativePayload);

        var unboxed = dualPayload.ToDualComputeImpulseStruct();
        Assert.Equal(125.0f, unboxed.Value);
        Assert.Equal(12.5f, unboxed.dP_0);
        Assert.Equal(50.0f, unboxed.dP_1);
        Assert.Equal("(125, 12.5, 50)", unboxed.ToString());

        // Verify Common Subexpression Elimination (CSE) extracted shared `(velocity * velocity)` intermediate
        Assert.NotEmpty(dualPayload.CseSchedule);
        Assert.Contains(dualPayload.CseSchedule, b => b.Expression.ToSExpr() == "(* velocity velocity)" && b.ReferenceCount >= 2);
    }

    [Fact]
    public void Case6_SymbolicAdjoint_Transcendental_ChainRule_And_Conditionals()
    {
        string code = @"
def WaveEnergy(x: float, scale: float) -> float:
    return scale * sin(x) + (x ** 2.0)

let d = WaveEnergy!@(0.0, 3.0)
// At x = 0.0, scale = 3.0:
// Value = 3*sin(0) + 0 = 0.0
// d/dx = 3*cos(0) + 2*0 = 3.0
// d/dscale = sin(0) = 0.0
assert(d == (0.0, 3.0, 0.0))
d
";
        var (res, _) = RunScript(code);
        Assert.True(res.Success, FormatFailure(res));
    }

    [Fact]
    public void Case7_EffectDryRun_MutateRecord_ZeroLiveMutation_Until_Commit()
    {
        string code = @"
class Record:
    def __init__(self, v: int):
        self.Val = v

def MutateRecord(rec, newVal: int) -> int:
    let before = rec.Val
    rec.Val = newVal
    return before + rec.Val

let rec = Record(10)
let plan = MutateRecord!?(rec, 42)

// 1. Live object is completely unmutated during dry-run
assert(rec.Val == 10)
// 2. Dry-run read-after-write sees shadow value 42 (10 + 42 == 52)
assert(plan.Value == 52)
assert(plan.diff_count == 1)
assert(plan.Diffs[0].Kind == ""StoreField"")
assert(plan.Diffs[0].OldValue == 10)
assert(plan.Diffs[0].NewValue == 42)

let report = plan.Inspect()
// 3. Commit applies deferred mutation to live memory
plan.Commit()
assert(rec.Val == 42)
report
";
        var (res, _) = RunScript(code);
        Assert.True(res.Success, FormatFailure(res));
        Assert.Contains("StoreField", res.Value.AsString());
        Assert.Contains("10 -> 42", res.Value.AsString());
    }

    [Fact]
    public void Case7_EffectDryRun_ArrayWrite_DatabaseExec_And_FileWrite_CoW()
    {
        string tempFile = Path.Combine(Path.GetTempPath(), $"pilot_m18_dryrun_{Guid.NewGuid():N}.txt");
        try
        {
            string escapedPath = tempFile.Replace("\\", "\\\\");
            string code = $@"
def PerformEffects(items, path: str) -> str:
    items.append(99)
    items[0] = 777
    insert into db.audit_log (action, code) values (""mutated"", 99)
    write_file(path, ""shadow-payload"")
    return read_file(path)

let myItems = [1, 2, 3]
let plan = PerformEffects!?(myItems, ""{escapedPath}"")

// Verify zero live mutations before Commit():
assert(len(myItems) == 3)
assert(myItems[0] == 1)
assert(plan.Value == ""shadow-payload"")
assert(plan.diff_count == 4)
assert(plan.Diffs[0].Kind == ""ArrayWrite"")
assert(plan.Diffs[1].Kind == ""ArrayWrite"")
assert(plan.Diffs[2].Kind == ""DatabaseExec"")
assert(plan.Diffs[3].Kind == ""FileWrite"")
plan
";
            var (res, _) = RunScript(code);
            Assert.True(res.Success, FormatFailure(res));
            Assert.False(File.Exists(tempFile));

            // Now commit the plan from C# and verify the file write and list mutations take effect
            var planPayload = Assert.IsType<PilotEffectPlan>(res.Value.AsObject().NativePayload);
            planPayload.Commit();
            Assert.True(File.Exists(tempFile));
            Assert.Equal("shadow-payload", File.ReadAllText(tempFile));
        }
        finally
        {
            if (File.Exists(tempFile))
                File.Delete(tempFile);
        }
    }

    [Fact]
    public void Mode5_ReactiveSignal_PropertyPath_And_DiamondGlitchFreedom()
    {
        string code = @"
def ComputeImpulse(mass: float, velocity: float) -> float:
    return 0.5 * mass * (velocity * velocity)

class Ship:
    def __init__(self, m: float, v: float):
        self.Mass = m
        self.Vel = v

let ship = Ship(10.0, 5.0)
let sig = ComputeImpulse!^(ship.Mass, ship.Vel)
assert(sig.Value == 125.0)

ship.Mass = 20.0
assert(sig.Value == 250.0)

ship.Vel = 10.0
assert(sig.Value == 1000.0)

// Diamond topology: root -> (left, right) -> sink
def LeftBranch(x: float) -> float:
    return x + 1.0

def RightBranch(x: float) -> float:
    return x * 10.0

def CombineBranches(l: float, r: float) -> float:
    return l + r

let root = signal(2.0, ""root"")
let left = LeftBranch!^(root)
let right = RightBranch!^(root)
let sink = CombineBranches!^(left, right)

assert(left.Rank == 1 and right.Rank == 1)
assert(sink.Rank == 2)
assert(sink.Value == 23.0)
let beforeCount = sink.EvalCount

root.set(5.0)
assert(sink.Value == 56.0)
// Sink must have recomputed strictly ONCE (no intermediate glitch with new left + old right)
assert(sink.EvalCount == beforeCount + 1)
sink.Value
";
        var (res, _) = RunScript(code);
        Assert.True(res.Success, FormatFailure(res));
        Assert.Equal(56.0, res.Value.AsFloat(), 5);
    }

    [Fact]
    public void Diagnostics_QUOTE001_QUOTE004_QUOTE005_Enforced_Statically_And_At_Runtime()
    {
        // 1. QUOTE005: Reactive signal call quote with zero observable inputs
        string codeQuote005 = @"
def ComputeImpulse(mass: float, velocity: float) -> float:
    return 0.5 * mass * (velocity * velocity)

let badSig = ComputeImpulse!^(10.0, 5.0)
";
        var analysis005 = PilotAnalysisPipeline.AnalyzeSource(codeQuote005);
        Assert.Contains(analysis005.TypeInference.Diagnostics, d => d.Code == "QUOTE005");
        var (res005, _) = RunScript(codeQuote005);
        Assert.False(res005.Success);
        Assert.Contains("QUOTE005", FormatFailure(res005));

        // 2. QUOTE004: Symbolic adjoint on function with unbounded loop or impure I/O
        string codeQuote004 = @"
def ImpureLoop(x: float) -> float:
    var acc = x
    while acc < 100.0:
        acc = acc + 1.0
    print(acc)
    return acc

let badDual = ImpureLoop!@(1.0)
";
        var analysis004 = PilotAnalysisPipeline.AnalyzeSource(codeQuote004);
        Assert.Contains(analysis004.TypeInference.Diagnostics, d => d.Code == "QUOTE004");
        var (res004, _) = RunScript(codeQuote004);
        Assert.False(res004.Success);
        Assert.Contains("QUOTE004", FormatFailure(res004));

        // 3. QUOTE001: Symbolic adjoint on external FFI symbol without white-box AST
        string codeQuote001Clean = @"
extern ""C"" {
    def opaque_energy(m: float, v: float) -> float
}
let d = opaque_energy!@(10.0, 5.0)
";
        var analysis001 = PilotAnalysisPipeline.AnalyzeSource(codeQuote001Clean);
        Assert.Contains(analysis001.TypeInference.Diagnostics, d => d.Code == "QUOTE001");
    }

    [Fact]
    public void EndToEnd_TestSuite35_And_Showcase23_ExecuteCleanly()
    {
        string repoRoot = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", ".."));
        string test35 = File.ReadAllText(Path.Combine(repoRoot, "tests", "35_semantic_call_projections.pilot"));
        var (res35, out35) = RunScript(test35);
        Assert.True(res35.Success, FormatFailure(res35));
        Assert.Contains("35_semantic_call_projections: ALL PASSED", out35);

        string showcase23 = File.ReadAllText(Path.Combine(repoRoot, "examples", "showcase", "23_semantic_call_projections.pilot"));
        var (res23, out23) = RunScript(showcase23);
        Assert.True(res23.Success, FormatFailure(res23));
        Assert.Contains("Value=125", out23);
    }
}
