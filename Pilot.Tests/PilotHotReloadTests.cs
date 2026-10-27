using System;
using System.IO;
using Pilot.Compiler;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Pilot.Scripting;
using Xunit;

namespace Pilot.Tests;

public class PilotHotReloadTests
{
    private readonly PilotEngine _engine = new();

    [Fact]
    public void Test_LiveFunctionPatching_UpdatesInPlace_IncrementsVersion()
    {
        string baseScript = "def compute(x: int) -> int:\n" +
                            "    return x + 10\n\n" +
                            "let fn_ptr = compute\n" +
                            "let v1 = compute(5)\n" +
                            "let ptr_v1 = fn_ptr(5)\n" +
                            "let initial_ver = compute.patch_version\n\n" +
                            "let patch_res = HotReload.apply(\"def compute(x: int) -> int:\\n    return x * 100\\n\")\n\n" +
                            "let v2 = compute(5)\n" +
                            "let ptr_v2 = fn_ptr(5)\n" +
                            "let patched_ver = compute.patch_version\n";

        var res = _engine.Execute(baseScript);
        Assert.True(res.Success, string.Join("; ", res.Diagnostics));

        var scope = res.Context!.Environment.GlobalScope;
        Assert.Equal(15, scope.Resolve("v1").AsInt());
        Assert.Equal(15, scope.Resolve("ptr_v1").AsInt());
        Assert.Equal(0, scope.Resolve("initial_ver").AsInt());

        Assert.Equal(500, scope.Resolve("v2").AsInt());
        Assert.Equal(500, scope.Resolve("ptr_v2").AsInt());
        Assert.Equal(1, scope.Resolve("patched_ver").AsInt());
    }

    [Fact]
    public void Test_LiveClassEvolution_PreservesObjectFieldsAndTypestate()
    {
        string baseScript = @"
class Worker:
    count: int = 0
    def __init__(self, initial: int):
        self.count = initial

    def process(self, delta: int) -> int:
        self.count = self.count + delta
        return self.count

    state Active:
        def get_info(self) -> str:
            return f""Active-v1({self.count})""

let w = Worker(100)
let initial_processed = w.process(5)
let initial_info = w.get_info()
";

        var res = _engine.Execute(baseScript);
        Assert.True(res.Success, string.Join("; ", res.Diagnostics));

        var ctx = res.Context!;
        var patchResult = ctx.HotReload.ApplySourcePatch(@"
class Worker:
    def process(self, delta: int) -> int:
        self.count = self.count + (delta * 10)
        return self.count

    def summary(self) -> str:
        return f""Worker(count={self.count})""

    state Active:
        def get_info(self) -> str:
            return f""Active-v2-HOT({self.count})""
");
        Assert.True(patchResult.Success, patchResult.ErrorMessage);

        var evalVisitor = ctx.GetOrCreateVisitor();
        var patchedProcessed = evalVisitor.Visit(PilotParser.GetOrParseExpression("w.process(5)"));
        var patchedInfo = evalVisitor.Visit(PilotParser.GetOrParseExpression("w.get_info()"));
        var patchedSummary = evalVisitor.Visit(PilotParser.GetOrParseExpression("w.summary()"));

        // Process delta of 5 * 10 = 50, added to 105 = 155
        Assert.Equal(155, patchedProcessed.AsInt());
        Assert.Equal("Active-v2-HOT(155)", patchedInfo.AsString());
        Assert.Equal("Worker(count=155)", patchedSummary.AsString());
    }

    [Fact]
    public void Test_HotReloadRollback_RestoresOriginalImplementations()
    {
        string script = "def calculate(x: int) -> int:\n" +
                        "    return x + 1\n\n" +
                        "let before = calculate(10)\n\n" +
                        "HotReload.apply(\"def calculate(x: int) -> int:\\n    return x * 100\\n\")\n" +
                        "let during = calculate(10)\n" +
                        "let ver_during = HotReload.version()\n\n" +
                        "let rb = HotReload.rollback()\n" +
                        "let after = calculate(10)\n" +
                        "let ver_after = HotReload.version()\n";

        var res = _engine.Execute(script);
        Assert.True(res.Success, string.Join("; ", res.Diagnostics));

        var scope = res.Context!.Environment.GlobalScope;
        Assert.Equal(11, scope.Resolve("before").AsInt());
        Assert.Equal(1000, scope.Resolve("during").AsInt());
        Assert.Equal(1, scope.Resolve("ver_during").AsInt());

        Assert.Equal(11, scope.Resolve("after").AsInt());
        Assert.Equal(0, scope.Resolve("ver_after").AsInt());
    }

    [Fact]
    public void Test_CompiledBinaryPatching_Offline_MergesSuccessfully()
    {
        string baseSource = @"
def get_greeting(name: str) -> str:
    return f""Hello v1, {name}!""

def get_answer() -> int:
    return 42
";

        string patchSource = @"
def get_greeting(name: str) -> str:
    return f""Hello v2-PATCHED, {name}!""

def get_bonus() -> int:
    return 100
";

        var baseComp = PilotCompiler.Default.Compile(baseSource);
        Assert.True(baseComp.Success && baseComp.Program is PilotCompiledProgram);
        var baseProg = (PilotCompiledProgram)baseComp.Program;

        var patchedProg = baseProg.ApplyPatchSource(patchSource);
        Assert.NotNull(patchedProg);

        var runRes = patchedProg.Run();
        Assert.True(runRes.Success, string.Join("; ", runRes.Diagnostics));

        var scope = runRes.Context!.Environment.GlobalScope;
        var greetFn = scope.Resolve("get_greeting").AsFunction();
        var greetVal = greetFn.Invoke(runRes.Context, new[] { PilotValue.FromString("Pilot") });
        Assert.Equal("Hello v2-PATCHED, Pilot!", greetVal.AsString());

        var answerVal = scope.Resolve("get_answer").AsFunction().Invoke(runRes.Context, Array.Empty<PilotValue>());
        Assert.Equal(42, answerVal.AsInt());

        var bonusVal = scope.Resolve("get_bonus").AsFunction().Invoke(runRes.Context, Array.Empty<PilotValue>());
        Assert.Equal(100, bonusVal.AsInt());
    }
}
