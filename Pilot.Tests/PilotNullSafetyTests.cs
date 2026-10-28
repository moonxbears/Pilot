using System;
using System.IO;
using System.Linq;
using System.Reflection;
using Pilot.Analysis;
using Pilot.Analysis.TypeSystem;
using Pilot.Compiler;
using Pilot.LanguageServer.Semantics;
using Pilot.Native.CAbi;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Interop;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Comprehensive test suite for Milestone 12:
/// Concise Modern Null-Safety System (`T?` Optionals, Zero `NullReferenceException`).
/// Covers deliverables 12.1 (`NULL001`–`NULL005` + removal of `x!`), 12.2 (`T?` implicit lifting,
/// `none`, `some x` in-place narrowing, `!= none`, `??`, `??=`, `?.`), 12.3 (zero-allocation niche
/// memory optimization), and 12.4 (Foreign Interop &amp; SQL null-boundary hardening).
/// </summary>
public class PilotNullSafetyTests
{
    private static PilotExecutionResult RunInterpreter(string code)
    {
        var engine = new PilotEngine();
        return engine.Execute(code);
    }

    private static PilotExecutionResult RunBytecode(string code)
    {
        var compileRes = PilotCompiler.Default.Compile(code);
        Assert.True(compileRes.Success, "PilVM compile failed: " + string.Join("; ", compileRes.Diagnostics));
        return compileRes.Program!.Run();
    }

    private static void AssertParity(string code, PilotValue expected)
    {
        var interpRes = RunInterpreter(code);
        Assert.True(interpRes.Success, "Interpreter failed: " + string.Join("; ", interpRes.Diagnostics));

        var vmRes = RunBytecode(code);
        Assert.True(vmRes.Success, "PilVM failed: " + string.Join("; ", vmRes.Diagnostics));

        Assert.Equal(expected, interpRes.Value);
        Assert.Equal(expected, vmRes.Value);
    }

    // =========================================================================
    // 12.1: Non-Nullable by Default & Removal of Unsafe `x!` (`NULL001`–`NULL005`)
    // =========================================================================

    [Fact]
    public void Null001_AssigningNoneOrOptionalToNonNullableType_IsRejected()
    {
        string code = @"
let ok_opt: str? = none
let bad_direct: str = none
let bad_from_opt: str = ok_opt
";
        var analysis = PilotAnalysisPipeline.AnalyzeSource(code);
        var null001 = analysis.Diagnostics.Where(d => d.Code == "NULL001").ToList();
        Assert.Equal(2, null001.Count);
        Assert.Contains(null001, d => d.Message.Contains("bad_direct"));
        Assert.Contains(null001, d => d.Message.Contains("bad_from_opt"));

        var lsp = PilotSemanticAnalyzer.Analyze(code);
        Assert.Contains(lsp.Diagnostics, d => d.Code == PilotDiagnosticCatalog.NullAssignedToNonNullable);
    }

    [Fact]
    public void Null002_UncheckedMemberOrIndexAccessOnOptional_IsRejected()
    {
        string code = @"
record User(name: str)

def check(u: User?, items: int[]?) -> void {
    let bad_member = u.name
    let bad_index = items[0]
    let ok_safe = u?.name
}
";
        var analysis = PilotAnalysisPipeline.AnalyzeSource(code);
        var null002 = analysis.Diagnostics.Where(d => d.Code == "NULL002").ToList();
        Assert.Equal(2, null002.Count);
        Assert.Contains(null002, d => d.Message.Contains(".name"));
        Assert.Contains(null002, d => d.Message.Contains("items"));
    }

    [Fact]
    public void Null003_ReturningNoneOrUnnarrowedOptionalFromNonNullableFunction_IsRejected()
    {
        string code = @"
def bad_return_none() -> int {
    return none
}

def bad_return_opt(x: int?) -> int {
    return x
}

def good_return_opt(x: int) -> int? {
    return x
}
";
        var analysis = PilotAnalysisPipeline.AnalyzeSource(code);
        var null003 = analysis.Diagnostics.Where(d => d.Code == "NULL003").ToList();
        Assert.Equal(2, null003.Count);
        Assert.Contains(null003, d => d.Message.Contains("bad_return_none"));
        Assert.Contains(null003, d => d.Message.Contains("bad_return_opt"));
    }

    [Fact]
    public void Null004_PassingNoneOrUnnarrowedOptionalToNonNullableParameter_IsRejected()
    {
        string code = @"
def require_text(msg: str) -> int {
    return msg.length
}

let maybe_msg: str? = ""hello""
let a = require_text(none)
let b = require_text(maybe_msg)
let c = require_text(maybe_msg ?? ""default"")
";
        var analysis = PilotAnalysisPipeline.AnalyzeSource(code);
        var null004 = analysis.Diagnostics.Where(d => d.Code == "NULL004").ToList();
        Assert.Equal(2, null004.Count);
    }

    [Fact]
    public void Null005_PostfixExclamationAssertionRemoved_AndOrPanicWorks()
    {
        string badCode = @"
let x: int? = 42
let y = x!
";
        var analysis = PilotAnalysisPipeline.AnalyzeSource(badCode);
        Assert.Contains(analysis.Diagnostics, d => d.Code == "NULL005");

        var lsp = PilotSemanticAnalyzer.Analyze(badCode);
        Assert.Contains(lsp.Diagnostics, d => d.Code == PilotDiagnosticCatalog.UnsafeNonNullAssertionRemoved);

        // Explicit `.OrPanic("reason")` succeeds when value is present and throws [NULL005] when `none`
        AssertParity(@"
let ok: int? = 123
return ok.OrPanic(""must exist"")
", PilotValue.FromInt(123));

        var panicRes = RunInterpreter(@"
let empty: str? = none
return empty.OrPanic(""missing configuration key"")
");
        Assert.False(panicRes.Success);
        Assert.Contains(panicRes.Diagnostics, d => d.Contains("[NULL005] missing configuration key", StringComparison.Ordinal));
    }

    // =========================================================================
    // 12.2: Effortless `T?` Ergonomics (Implicit Lifting + `some` Narrowing)
    // =========================================================================

    [Fact]
    public void ImplicitLifting_And_GuardSomeInPlaceNarrowing_Parity()
    {
        string code = @"
def greet(nickname: str?) -> str {
    guard some nickname else {
        return ""Hello, Guest!""
    }
    return f""Hello, {nickname.ToUpper()}!""
}

let a = greet(""Maverick"")
let b = greet(none)
return f""{a} | {b}""
";
        // Static analyzer must report 0 errors because `guard some nickname` narrows `nickname` to `str`!
        var analysis = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.DoesNotContain(analysis.Diagnostics, d => d.Code.StartsWith("NULL", StringComparison.Ordinal));

        var lsp = PilotSemanticAnalyzer.Analyze(code);
        Assert.DoesNotContain(lsp.Diagnostics, d => d.Code == PilotDiagnosticCatalog.PossibleNullDereference);

        AssertParity(code, PilotValue.FromString("Hello, MAVERICK! | Hello, Guest!"));
    }

    [Fact]
    public void ConciseSomeBinding_InGuardIfAndWhile_Parity()
    {
        string code = @"
def parse_and_scale(raw: str) -> int {
    guard some n = raw.Trim().ToInt() when n > 0 else {
        return -1
    }
    if some doubled = (f""{n * 2}"").ToInt() {
        return doubled
    }
    return n
}

return (parse_and_scale("" 21 ""), parse_and_scale(""bad""), parse_and_scale(""-4""))
";
        var analysis = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.DoesNotContain(analysis.Diagnostics, d => d.Code.StartsWith("NULL", StringComparison.Ordinal));

        AssertParity(code, PilotValue.FromTuple(new[]
        {
            PilotValue.FromInt(42),
            PilotValue.FromInt(-1),
            PilotValue.FromInt(-1)
        }));
    }

    [Fact]
    public void FlowSensitiveNarrowing_NotEqualNone_EarlyExit_And_Coalesce_Parity()
    {
        string code = @"
record Profile(handle: str)

def extract_handle(p: Profile?) -> str {
    if p != none {
        return p.handle.ToUpper()
    }
    return ""NONE""
}

def extract_early_exit(p: Profile?) -> str {
    if p == none {
        return ""EARLY_NONE""
    }
    return p.handle
}

def extract_coalesce(p: Profile?) -> str {
    let safe_p: Profile = p ?? Profile(""fallback_user"")
    return safe_p.handle
}

let prof: Profile? = Profile(""pilot_dev"")
return f""{extract_handle(prof)}:{extract_early_exit(prof)}:{extract_coalesce(none)}""
";
        var analysis = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.DoesNotContain(analysis.Diagnostics, d => d.Code.StartsWith("NULL", StringComparison.Ordinal));

        AssertParity(code, PilotValue.FromString("PILOT_DEV:pilot_dev:fallback_user"));
    }

    [Fact]
    public void ExhaustiveMatchOnOptional_WithSomeAndNone_VerifiedByMarangetChecker()
    {
        string exhaustiveCode = @"
def describe(x: int?) -> str {
    return match x {
        case some(v) => f""val={v}"",
        case none => ""none""
    }
}
return f""{describe(10)},{describe(none)}""
";
        var okAnalysis = PilotAnalysisPipeline.AnalyzeSource(exhaustiveCode);
        Assert.DoesNotContain(okAnalysis.Diagnostics, d => d.Code == "MATCH001");
        AssertParity(exhaustiveCode, PilotValue.FromString("val=10,none"));

        string missingNoneCode = @"
def incomplete(x: int?) -> int {
    return match x {
        case some(v) => v
    }
}
";
        var badAnalysis = PilotAnalysisPipeline.AnalyzeSource(missingNoneCode);
        var match001 = badAnalysis.Diagnostics.FirstOrDefault(d => d.Code == "MATCH001");
        Assert.NotNull(match001);
        Assert.Contains("none", match001!.Message);
    }

    // =========================================================================
    // 12.3: Zero-Allocation Niche Memory Optimization & Nested `T??`
    // =========================================================================

    [Fact]
    public void ZeroAllocationNicheOptimization_And_NestedOptionDisambiguation()
    {
        string code = @"
record Point(x: f64, y: f64)
class Session {
    var token: str = ""abc""
}

let user_niche = sizeof_type(""Session?"") == sizeof_type(""Session"")
let str_niche = sizeof_type(""str?"") == sizeof_type(""str"")
let bool_niche = sizeof_type(""bool?"") == sizeof_type(""bool"")

let n1: int?? = none
let n2: int?? = some(none)
let n3: int?? = some(77)

def classify(v: int??) -> str {
    return match v {
        case some(some(x)) => f""val:{x}"",
        case some(none) => ""inner_none"",
        case none => ""outer_none""
    }
}

return (user_niche, str_niche, bool_niche, n2.is_unboxed, classify(n1), classify(n2), classify(n3))
";
        AssertParity(code, PilotValue.FromTuple(new[]
        {
            PilotValue.True,
            PilotValue.True,
            PilotValue.True,
            PilotValue.True,
            PilotValue.FromString("outer_none"),
            PilotValue.FromString("inner_none"),
            PilotValue.FromString("val:77")
        }));

        // Verify C-ABI emission uses pointer niche for `str?` / `Session?` and inline struct for `i64?`
        string cHeader = PilotCAbiEmitter.EmitHeader(@"
export def find_name(id: i64) -> str? { return none }
export def find_score(id: i64) -> i64? { return 10 }
", "niche_abi");
        Assert.Contains("const char* find_name(int64_t id);", cHeader);
        Assert.Contains("struct { int64_t value; uint8_t has_value; } find_score(int64_t id);", cHeader);
    }

    // =========================================================================
    // 12.4: Foreign Interop & SQL Null-Boundary Hardening
    // =========================================================================

    private static class SampleForeignApi
    {
        public static int NonNullPrimitive() => 42;
        public static int? NullablePrimitive() => null;
        public static IntPtr NullablePointer() => IntPtr.Zero;
    }

    [Fact]
    public void ForeignInteropAndSqlNullBoundary_AutomaticallySurfacesOptionals()
    {
        // 1. C / FFI extern functions with pointer or reference returns harden to `T?` and enforce NULL001/NULL002
        string ffiCode = @"
extern ""C"" {
    def getenv(name: char*) -> char*
}

let raw_env = getenv(""PATH"")
let bad_non_null: str = raw_env
let bad_deref = raw_env.ToUpper()
let safe_env: str = raw_env ?? ""/usr/bin""
";
        var ffiAnalysis = PilotAnalysisPipeline.AnalyzeSource(ffiCode);
        Assert.Contains(ffiAnalysis.Diagnostics, d => d.Code == "NULL001" && d.Message.Contains("bad_non_null"));
        Assert.Contains(ffiAnalysis.Diagnostics, d => d.Code == "NULL002" && d.Message.Contains("raw_env"));

        // 2. SQL model nullable column boundary check
        string sqlCode = @"
model UserRow {
    let id: int = 1
    let nickname: str? = none
}

def read_row(row: UserRow) -> str {
    let bad: str = row.nickname
    return row.nickname ?? ""anon""
}
";
        var sqlAnalysis = PilotAnalysisPipeline.AnalyzeSource(sqlCode);
        Assert.Contains(sqlAnalysis.Diagnostics, d => d.Code == "NULL001" && d.Message.Contains("row.nickname"));

        // 3. PolyglotTypeBridge reflection & SQL column nullability helpers
        var nonNullMethod = typeof(SampleForeignApi).GetMethod(nameof(SampleForeignApi.NonNullPrimitive), BindingFlags.Public | BindingFlags.Static)!;
        var nullableMethod = typeof(SampleForeignApi).GetMethod(nameof(SampleForeignApi.NullablePrimitive), BindingFlags.Public | BindingFlags.Static)!;
        var ptrMethod = typeof(SampleForeignApi).GetMethod(nameof(SampleForeignApi.NullablePointer), BindingFlags.Public | BindingFlags.Static)!;

        Assert.Equal("int", PolyglotTypeBridge.InferForeignReturnPilotType(nonNullMethod));
        Assert.Equal("int?", PolyglotTypeBridge.InferForeignReturnPilotType(nullableMethod));
        Assert.Equal("ptr?", PolyglotTypeBridge.InferForeignReturnPilotType(ptrMethod));

        Assert.Equal(PilotValue.Null, PolyglotTypeBridge.Default.ToPilotValue(DBNull.Value));
        Assert.Equal(PilotValue.Null, PolyglotTypeBridge.Default.ToPilotValue(IntPtr.Zero));

        Assert.Equal("str?", PolyglotTypeBridge.InferSqlColumnPilotType("VARCHAR(255)"));
        Assert.Equal("str", PolyglotTypeBridge.InferSqlColumnPilotType("VARCHAR(255) NOT NULL"));
        Assert.Equal("int", PolyglotTypeBridge.InferSqlColumnPilotType("SERIAL PRIMARY KEY"));
        Assert.Equal("int?", PolyglotTypeBridge.InferSqlColumnPilotType("INTEGER"));
    }

    [Fact]
    public void IntegrationScript_29_And_Showcase_17_RunInInterpreterAndBytecode()
    {
        string path29 = Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "tests", "29_null_safety_and_optionals.pilot");
        string source29 = File.ReadAllText(path29);
        var interp29 = RunInterpreter(source29);
        Assert.True(interp29.Success, "Interpreter failed on 29: " + string.Join("; ", interp29.Diagnostics));
        var vm29 = RunBytecode(source29);
        Assert.True(vm29.Success, "PilVM failed on 29: " + string.Join("; ", vm29.Diagnostics));

        string path17 = Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "examples", "showcase", "17_modern_null_safety.pilot");
        string source17 = File.ReadAllText(path17);
        var interp17 = RunInterpreter(source17);
        Assert.True(interp17.Success, "Interpreter failed on showcase 17: " + string.Join("; ", interp17.Diagnostics));
        var vm17 = RunBytecode(source17);
        Assert.True(vm17.Success, "PilVM failed on showcase 17: " + string.Join("; ", vm17.Diagnostics));
    }
}
