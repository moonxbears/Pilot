using System.Linq;
using Pilot.LanguageServer.Semantics;
using Xunit;

namespace Pilot.Tests;

public class PilotTypeInferenceAndHomogenizationTests
{
    [Fact]
    public void Test_Function_Without_Explicit_ReturnType_Infers_Return_Type()
    {
        string code = @"
def compute_answer() {
    return 42
}

let answer = compute_answer()
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.DoesNotContain(result.Diagnostics, d => d.Severity == PilotDiagnosticSeverity.Error);

        var fnSym = result.RootScope.Resolve("compute_answer");
        Assert.NotNull(fnSym);
        Assert.Equal("int", fnSym.TypeName);
        Assert.Contains("-> int", fnSym.Signature);

        var ansSym = result.RootScope.Resolve("answer");
        Assert.NotNull(ansSym);
        Assert.Equal("int", ansSym.TypeName);
        Assert.NotEqual("void", ansSym.TypeName);
    }

    [Fact]
    public void Test_Collection_Element_Type_Derivation_Integer_List()
    {
        string code = @"
let nums = [1, 2, 3, 4, 5]
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.DoesNotContain(result.Diagnostics, d => d.Severity == PilotDiagnosticSeverity.Error);

        var sym = result.RootScope.Resolve("nums");
        Assert.NotNull(sym);
        Assert.Equal("List[int]", sym.TypeName);
    }

    [Fact]
    public void Test_Collection_Element_Type_Derivation_Dict_And_Set()
    {
        string code = @"
let scores = { ""alice"": 95, ""bob"": 100 }
let unique_ids = { 1, 2, 3 }
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.DoesNotContain(result.Diagnostics, d => d.Severity == PilotDiagnosticSeverity.Error);

        var scoresSym = result.RootScope.Resolve("scores");
        Assert.NotNull(scoresSym);
        Assert.Equal("Dict[str, int]", scoresSym.TypeName);

        var setSym = result.RootScope.Resolve("unique_ids");
        Assert.NotNull(setSym);
        Assert.Equal("Set[int]", setSym.TypeName);
    }

    [Fact]
    public void Test_Homogenized_Collection_Type_Furthest_Union()
    {
        string code = @"
let mixed = [1, ""two"", 3.14]
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.DoesNotContain(result.Diagnostics, d => d.Severity == PilotDiagnosticSeverity.Error);

        var mixedSym = result.RootScope.Resolve("mixed");
        Assert.NotNull(mixedSym);
        Assert.Contains("int", mixedSym.TypeName);
        Assert.Contains("str", mixedSym.TypeName);
        Assert.Contains("float", mixedSym.TypeName);
        Assert.StartsWith("List[", mixedSym.TypeName);
    }

    [Fact]
    public void Test_Multi_Return_Homogenization_Concise_Union()
    {
        string code = @"
class Candies {}
class Sweets : Candies {}
class Sours : Candies {}

def get_candies(choice: int) {
    if choice == 1:
        return [Sweets()]
    else if choice == 2:
        return [Sours()]
    else:
        return Candies()
}

let candies = get_candies(1)
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.DoesNotContain(result.Diagnostics, d => d.Severity == PilotDiagnosticSeverity.Error);

        var fnSym = result.RootScope.Resolve("get_candies");
        Assert.NotNull(fnSym);
        Assert.Equal("List[Sweets | Sours] | Candies", fnSym.TypeName);

        var candiesSym = result.RootScope.Resolve("candies");
        Assert.NotNull(candiesSym);
        Assert.Equal("List[Sweets | Sours] | Candies", candiesSym.TypeName);
    }

    [Fact]
    public void Test_Indexing_Unwraps_Element_Type()
    {
        string code = @"
let nums = [10, 20, 30]
let first = nums[0]
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.DoesNotContain(result.Diagnostics, d => d.Severity == PilotDiagnosticSeverity.Error);

        var firstSym = result.RootScope.Resolve("first");
        Assert.NotNull(firstSym);
        Assert.Equal("int", firstSym.TypeName);
    }

    [Fact]
    public void Test_Spawn_And_Await_Deduce_Future_And_Unwrapped_Types()
    {
        string code = @"
def fetch_heavy_data() {
    return ""heavy_payload""
}

let bg_future = spawn fetch_heavy_data()
let data_result = await bg_future
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.DoesNotContain(result.Diagnostics, d => d.Severity == PilotDiagnosticSeverity.Error);

        var fSym = result.RootScope.Resolve("bg_future");
        Assert.NotNull(fSym);
        Assert.Equal("Future[str]", fSym.TypeName);

        var resSym = result.RootScope.Resolve("data_result");
        Assert.NotNull(resSym);
        Assert.Equal("str", resSym.TypeName);
    }

    [Fact]
    public void Test_No_Return_Function_Defaults_To_Void()
    {
        string code = @"
def do_side_effect() {
    println(""done"")
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.DoesNotContain(result.Diagnostics, d => d.Severity == PilotDiagnosticSeverity.Error);

        var fnSym = result.RootScope.Resolve("do_side_effect");
        Assert.NotNull(fnSym);
        Assert.Equal("void", fnSym.TypeName);
    }
}
