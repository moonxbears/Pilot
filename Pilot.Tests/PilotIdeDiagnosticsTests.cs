using System;
using System.Linq;
using System.Text.Json.Nodes;
using Pilot.LanguageServer.Lsp;
using Pilot.LanguageServer.Semantics;
using Xunit;

namespace Pilot.Tests;

public class PilotIdeDiagnosticsTests
{
    [Fact]
    public void Test_Catalog_Completeness_And_Metadata()
    {
        var descriptors = PilotDiagnosticCatalog.GetAllDescriptors();
        Assert.NotEmpty(descriptors);

        // Verify key codes across all language domains are registered
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.SyntaxError));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.RedundantSemicolon));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.ImmutableReassignment));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.TypeMismatch));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.GuardMissingExit));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.UnboundedLoopInRealtime));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.DuplicateSymbol));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.AwaitOutsideAsync));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.UndeclaredSavepoint));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.LoopJumpOutsideLoop));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.ReturnOutsideFunction));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.YieldOutsideGenerator));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.UnreachableCode));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.UnusedVariable));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.ParameterReassignment));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.PossibleNullDereference));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.ReturnTypeMismatch));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.UnawaitedFutureDiscarded));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.UnboundedSelectDeadlockRisk));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.HeapAllocInRealtime));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.SuggestFString));
        Assert.True(descriptors.ContainsKey(PilotDiagnosticCatalog.EmptyCatchBlock));

        // Verify descriptor metadata
        var awaitDesc = descriptors[PilotDiagnosticCatalog.AwaitOutsideAsync];
        Assert.Equal("Async", awaitDesc.Category);
        Assert.Equal(PilotDiagnosticSeverity.Error, awaitDesc.DefaultSeverity);
        Assert.False(string.IsNullOrWhiteSpace(awaitDesc.MessageFormat));
        Assert.NotNull(awaitDesc.QuickFixTitle);
    }

    [Fact]
    public void Test_Checker_PILOT006_RedundantSemicolon()
    {
        string code = "let x = 10;;\n";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var diag = result.Diagnostics.FirstOrDefault(d => d.Code == PilotDiagnosticCatalog.RedundantSemicolon);

        Assert.NotNull(diag);
        Assert.Equal(PilotDiagnosticSeverity.Hint, diag.Severity);
        Assert.Contains(diag.CodeActions, a => a.Title == "Remove redundant semicolon");
    }

    [Fact]
    public void Test_Checker_PILOT108_LoopJumpOutsideLoop()
    {
        string code = @"
def standalone_fn() -> void {
    break
    continue
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var diags = result.Diagnostics.Where(d => d.Code == PilotDiagnosticCatalog.LoopJumpOutsideLoop).ToList();

        Assert.Equal(2, diags.Count);
        Assert.All(diags, d => Assert.Equal(PilotDiagnosticSeverity.Error, d.Severity));
    }

    [Fact]
    public void Test_Checker_PILOT109_ReturnOutsideFunction()
    {
        string code = @"
let a = 1
return a
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var diag = result.Diagnostics.FirstOrDefault(d => d.Code == PilotDiagnosticCatalog.ReturnOutsideFunction);

        Assert.NotNull(diag);
        Assert.Equal(PilotDiagnosticSeverity.Error, diag.Severity);
        Assert.Contains("'return' statement can only appear inside a function", diag.Message);
    }

    [Fact]
    public void Test_Checker_PILOT110_YieldOutsideGenerator()
    {
        string code = @"
let a = 100
yield a
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var diag = result.Diagnostics.FirstOrDefault(d => d.Code == PilotDiagnosticCatalog.YieldOutsideGenerator);

        Assert.NotNull(diag);
        Assert.Equal(PilotDiagnosticSeverity.Error, diag.Severity);
        Assert.Contains("'yield' statement can only appear inside a generator", diag.Message);
    }

    [Fact]
    public void Test_Checker_PILOT111_UnreachableCode()
    {
        string code = @"
def compute(x: int) -> int {
    if x > 10 {
        return x * 2
        let dead = 999
        print(dead)
    }
    return x
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var diag = result.Diagnostics.FirstOrDefault(d => d.Code == PilotDiagnosticCatalog.UnreachableCode);

        Assert.NotNull(diag);
        Assert.Equal(PilotDiagnosticSeverity.Warning, diag.Severity);
        Assert.Contains("Unreachable code detected", diag.Message);
        Assert.Contains(diag.CodeActions, a => a.Title == "Remove unreachable code");
    }

    [Fact]
    public void Test_Checker_PILOT112_UnusedVariable()
    {
        string code = @"
def worker() -> void {
    let unread_secret = 42
    let _intentionally_unused = 100
    print(""done"")
}
";
        var result = PilotSemanticAnalyzer.Analyze(code, checkUnused: true);
        var diag = result.Diagnostics.FirstOrDefault(d => d.Code == PilotDiagnosticCatalog.UnusedVariable);

        Assert.NotNull(diag);
        Assert.Contains("unread_secret", diag.Message);
        Assert.DoesNotContain(result.Diagnostics, d => d.Message.Contains("_intentionally_unused"));
        Assert.Contains(diag.CodeActions, a => a.Title.Contains("Rename 'unread_secret' to '_unread_secret'"));
    }

    [Fact]
    public void Test_Checker_PILOT113_ParameterReassignment()
    {
        string code = @"
def process(param: int) -> void {
    param = 42
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var diag = result.Diagnostics.FirstOrDefault(d => d.Code == PilotDiagnosticCatalog.ParameterReassignment);

        Assert.NotNull(diag);
        Assert.Equal(PilotDiagnosticSeverity.Error, diag.Severity);
        Assert.Contains("Cannot reassign parameter 'param'", diag.Message);
    }

    [Fact]
    public void Test_Checker_PILOT202_PossibleNullDereference()
    {
        string code = @"
struct User {
    let name: str
}

def greet(u: User?) -> void {
    let n = u.name
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var diag = result.Diagnostics.FirstOrDefault(d => d.Code == PilotDiagnosticCatalog.PossibleNullDereference);

        Assert.NotNull(diag);
        Assert.Equal(PilotDiagnosticSeverity.Error, diag.Severity);
        Assert.Contains("Possible null dereference", diag.Message);
        Assert.Contains(diag.CodeActions, a => a.Title == "Replace '.' with '?.' safe navigation");
    }

    [Fact]
    public void Test_Checker_PILOT205_ReturnTypeMismatch()
    {
        string code = @"
def calculate() -> int {
    return ""not an int""
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var diag = result.Diagnostics.FirstOrDefault(d => d.Code == PilotDiagnosticCatalog.ReturnTypeMismatch);

        Assert.NotNull(diag);
        Assert.Equal(PilotDiagnosticSeverity.Error, diag.Severity);
        Assert.Contains("Function declared return type 'int', but returned value of type 'str'", diag.Message);
    }

    [Fact]
    public void Test_Checker_PILOT401_UnawaitedFutureDiscarded()
    {
        string code = @"
async def fetch_data() -> str {
    return ""data""
}

def caller() -> void {
    fetch_data()
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var diag = result.Diagnostics.FirstOrDefault(d => d.Code == PilotDiagnosticCatalog.UnawaitedFutureDiscarded);

        Assert.NotNull(diag);
        Assert.Equal(PilotDiagnosticSeverity.Warning, diag.Severity);
        Assert.Contains("discarded without awaiting", diag.Message);
        Assert.Contains(diag.CodeActions, a => a.Title == "Add 'await' to future expression");
    }

    [Fact]
    public void Test_Checker_PILOT405_UnboundedSelectDeadlockRisk()
    {
        string code = @"
def listen_channels(ch1: Channel[int], ch2: Channel[str]) -> void {
    select {
        case msg1 = <-ch1 {
            print(msg1)
        }
        case msg2 = <-ch2 {
            print(msg2)
        }
    }
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var diag = result.Diagnostics.FirstOrDefault(d => d.Code == PilotDiagnosticCatalog.UnboundedSelectDeadlockRisk);

        Assert.NotNull(diag);
        Assert.Equal(PilotDiagnosticSeverity.Warning, diag.Severity);
        Assert.Contains("has neither 'timeout' nor 'default' case", diag.Message);
    }

    [Fact]
    public void Test_Checker_PILOT501_HeapAllocInRealtime()
    {
        string code = @"
@hard_realtime
def control_loop() -> void {
    noalloc {
        let dynamic_items = [1, 2, 3]
    }
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var diag = result.Diagnostics.FirstOrDefault(d => d.Code == PilotDiagnosticCatalog.HeapAllocInRealtime);

        Assert.NotNull(diag);
        Assert.Equal(PilotDiagnosticSeverity.Error, diag.Severity);
        Assert.Contains("Heap allocation detected inside '@hard_realtime' / 'noalloc'", diag.Message);
    }

    [Fact]
    public void Test_Checker_PILOT902_SuggestFString()
    {
        string code = @"
def format_greeting(name: str) -> str {
    return ""Hello, "" + name
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var diag = result.Diagnostics.FirstOrDefault(d => d.Code == PilotDiagnosticCatalog.SuggestFString);

        Assert.NotNull(diag);
        Assert.Equal(PilotDiagnosticSeverity.Hint, diag.Severity);
        Assert.Contains("f-string", diag.Message);
        Assert.Contains(diag.CodeActions, a => a.Title == "Convert to f-string");
    }

    [Fact]
    public void Test_Checker_PILOT904_EmptyCatchBlock()
    {
        string code = @"
def risky_operation() -> void {
    try {
        print(""running"")
    } except {
        pass
    }
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var diag = result.Diagnostics.FirstOrDefault(d => d.Code == PilotDiagnosticCatalog.EmptyCatchBlock);

        Assert.NotNull(diag);
        Assert.Equal(PilotDiagnosticSeverity.Warning, diag.Severity);
        Assert.Contains("Empty 'except' block suppresses all exceptions", diag.Message);
    }

    [Fact]
    public void Test_Lsp_CodeAction_QuickFix_Execution()
    {
        var server = new PilotLspServer();

        // 1. Initialize LSP server
        server.ProcessMessage(@"{""jsonrpc"":""2.0"",""id"":1,""method"":""initialize"",""params"":{}}");

        // 2. Open document with redundant semicolon (PILOT006)
        string docUri = "file:///workspace/test_fix.pilot";
        string sampleCode = "let a = 100;;\n";
        server.ProcessMessage(new JsonObject
        {
            ["jsonrpc"] = "2.0",
            ["method"] = "textDocument/didOpen",
            ["params"] = new JsonObject
            {
                ["textDocument"] = new JsonObject
                {
                    ["uri"] = docUri,
                    ["languageId"] = "pilot",
                    ["version"] = 1,
                    ["text"] = sampleCode
                }
            }
        }.ToJsonString());

        // 3. Request codeAction on line 0
        var caResp = server.ProcessMessage(new JsonObject
        {
            ["jsonrpc"] = "2.0",
            ["id"] = 2,
            ["method"] = "textDocument/codeAction",
            ["params"] = new JsonObject
            {
                ["textDocument"] = new JsonObject { ["uri"] = docUri },
                ["range"] = new JsonObject
                {
                    ["start"] = new JsonObject { ["line"] = 0, ["character"] = 0 },
                    ["end"] = new JsonObject { ["line"] = 0, ["character"] = 14 }
                }
            }
        }.ToJsonString());

        Assert.Single(caResp);
        var caJson = JsonNode.Parse(caResp[0])!;
        var actions = caJson["result"]?.AsArray();
        Assert.NotNull(actions);
        Assert.NotEmpty(actions);

        var firstAction = actions![0]!.AsObject();
        Assert.Equal("Remove redundant semicolon", firstAction["title"]?.GetValue<string>());
        Assert.Equal("quickfix", firstAction["kind"]?.GetValue<string>());
        Assert.True(firstAction["isPreferred"]?.GetValue<bool>());

        // Check workspace edit
        var edit = firstAction["edit"]?["changes"]?[docUri]?.AsArray();
        Assert.NotNull(edit);
        Assert.Single(edit);
        Assert.Equal("", edit![0]!["newText"]?.GetValue<string>());
    }
}
