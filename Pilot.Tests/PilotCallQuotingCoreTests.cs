using System;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using Pilot.Analysis;
using Pilot.Analysis.CallQuoting;
using Pilot.LanguageServer.Semantics;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Milestone 17 Acceptance & Verification Suite:
/// - 17.1: Pervasive First-Class Keywords (:kw, :ns/kw), DAG Hierarchies (.Derive, .IsA),
///         Callable Map Lookup (:user/role(u, default=:guest)), Keyword-Dispatched Method Groups,
///         and Reified Special Forms, Types, Operators & Lexical Scopes.
/// - 17.2: Zero-Whitespace Call-Quoting Lexer & Parser Grammar (!, !!, ~, !&, !^, !@, !?)
///         with Lexer Inequality / Logical NOT / Preprocessor Hash Safety.
/// - 17.3: Unified CallQuoteKind AST & Core Lowering Modes 0–4 (ShallowQuote, DeepSplice,
///         Quasiquote + Constant Folding + Dead-Branch Elimination, and 12-byte C-ABI CallFrame).
/// </summary>
public class PilotCallQuotingCoreTests
{
    private const string ComputeImpulseFixture = @"
def ComputeImpulse(mass: f32, velocity: f32) -> f32 {
    return 0.5 * mass * (velocity * velocity)
}
";

    private static string FormatFailure(PilotExecutionResult result) =>
        string.Join(" | ", result.Diagnostics.Concat(new[] { result.StandardError }).Where(s => !string.IsNullOrWhiteSpace(s)));

    [Fact]
    public void Case1_Lexer_Inequality_LogicalNot_And_Hash_Directive_Safety()
    {
        string code = ComputeImpulseFixture + @"
#define IMPULSE_SCALE
let a = 10
let b = 20
let flag = false
let c1 = a != b
let c2 = !(a == b)
let c3 = !!flag
let c4 = ComputeImpulse!(1.0, 2.0)
assert(c1 == true)
assert(c2 == true)
assert(c3 == false)
assert(c4.eval() == 2.0)
";
        var engine = new PilotEngine();
        var result = engine.Execute(code);
        Assert.True(result.Success, FormatFailure(result));
    }

    [Fact]
    public void Case2_Shallow_Call_Quote_Mode1_Defers_Execution()
    {
        string code = ComputeImpulseFixture + @"
var side_effect_count = 0
def TrackedImpulse(mass: f32, velocity: f32) -> f32 {
    side_effect_count += 1
    return ComputeImpulse(mass, velocity)
}

let q = TrackedImpulse!(10.0, 4.0)
assert(side_effect_count == 0)
assert(q.fn_name == ""TrackedImpulse"")
let evaluated = q.eval()
assert(side_effect_count == 1)
assert(evaluated == 80.0)
";
        var engine = new PilotEngine();
        var result = engine.Execute(code);
        Assert.True(result.Success, FormatFailure(result));
    }

    [Fact]
    public void Case3_Deep_WhiteBox_Splice_And_Quasiquote_Constant_Folding_Mode2_And_Mode3()
    {
        string code = ComputeImpulseFixture + @"
let full_ast = ComputeImpulse!!(m, v)
let folded = ComputeImpulse!!(~10.0, v)
let folded_str = folded.to_string()
let folded_val = folded.eval({ ""v"": 4.0 })
assert(folded_str == ""Mul(5.0, Mul(Sym(\""v\""), Sym(\""v\"")))"")
assert(folded_val == 80.0)
";
        var engine = new PilotEngine();
        var result = engine.Execute(code);
        Assert.True(result.Success, FormatFailure(result));
    }

    [Fact]
    public unsafe void Case4_ZeroMarshalling_CAbi_Sequential_Struct_Projection_Mode4()
    {
        Assert.Equal(12, sizeof(CallFrame_ComputeImpulse));
        var nativeFrame = new CallFrame_ComputeImpulse(PilotCallQuotingEngine.ComputeFnMetadataId("ComputeImpulse"), 10.0f, 4.0f);
        Assert.Equal(10.0f, nativeFrame.P_0);
        Assert.Equal(4.0f, nativeFrame.P_1);

        string code = ComputeImpulseFixture + @"
let pkt = ComputeImpulse!&(10.0, 4.0)
assert(pkt.size_bytes == 12)
assert(pkt.P_0 == 10.0)
assert(pkt.P_1 == 4.0)
let res = pkt.dispatch()
assert(res == 80.0)
";
        var engine = new PilotEngine();
        var result = engine.Execute(code);
        Assert.True(result.Success, FormatFailure(result));
    }

    [Fact]
    public void Case5_Keywords_Hierarchies_MethodDispatch_And_Reified_SpecialForms()
    {
        string code = @"
let u = { :id: 101, :user/role: :admin }
let r1 = :user/role(u, default=:guest)
let r2 = :user/missing(u, default=:guest)
assert(r1 == :admin)
assert(r2 == :guest)

:admin.Derive(:user)
assert(:admin.IsA(:user) == true)

def HandleRole(role: :user) -> str { return ""standard_user"" }
def HandleRole(role: :admin) -> str { return ""elevated_admin"" }
assert(HandleRole(:admin) == ""elevated_admin"")
assert(HandleRole(:user) == ""standard_user"")

let g = keyword(guard)
assert(g.is_special_form == true)

let T = int | str
assert(T.Check(42) == true)
assert(T.Check(""hello"") == true)
assert(T.Check(3.14) == false)

let add = (+)
assert(add(20, 22) == 42)

var local_x = 99
let sc = current_scope()
assert(sc.get(:local_x) == 99)
";
        var engine = new PilotEngine();
        var result = engine.Execute(code);
        Assert.True(result.Success, FormatFailure(result));
    }

    [Fact]
    public void Diagnostics_QUOTE001_QUOTE002_QUOTE003_Enforced_Statically_And_At_Runtime()
    {
        // QUOTE001: Deep splice `!!` on external FFI function without white-box AST body
        string quote001Code = @"
extern ""libc.so"" {
    def cos(x: f64) -> f64
}
let bad = cos!!(1.0)
";
        var analysis1 = PilotAnalysisPipeline.AnalyzeSource(quote001Code);
        Assert.Contains(analysis1.Diagnostics, d => d.Code == PilotDiagnosticCatalog.MissingWhiteBoxAstForDeepSplice);

        // QUOTE002: Unquote `~arg` outside `!!` call
        string quote002Code = ComputeImpulseFixture + @"
let bad = ComputeImpulse!(~10.0, 4.0)
";
        var analysis2 = PilotAnalysisPipeline.AnalyzeSource(quote002Code);
        Assert.Contains(analysis2.Diagnostics, d => d.Code == PilotDiagnosticCatalog.InvalidUnquotePrefixOutsideDeepSplice);

        // QUOTE003: `!&` ABI frame on managed parameter type (str)
        string quote003Code = @"
def Greet(name: str) -> str {
    return ""Hello "" + name
}
let bad = Greet!&(""Pilot"")
";
        var analysis3 = PilotAnalysisPipeline.AnalyzeSource(quote003Code);
        Assert.Contains(analysis3.Diagnostics, d => d.Code == PilotDiagnosticCatalog.NonUnmanagedParameterInAbiCallFrame);
    }

    [Fact]
    public void IntegrationSuite_34_Keywords_And_Call_Quoting_Runs_Cleanly()
    {
        string repoRoot = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", ".."));
        string scriptPath = Path.Combine(repoRoot, "tests", "34_keywords_and_call_quoting.pilot");
        Assert.True(File.Exists(scriptPath), $"Expected test script at {scriptPath}");

        var engine = new PilotEngine();
        var result = engine.ExecuteFile(scriptPath);
        Assert.True(result.Success, FormatFailure(result));
    }
}
