using System;
using System.IO;
using System.Linq;
using Pilot.Analysis;
using Pilot.Analysis.TypeSystem;
using Pilot.Compiler;
using Pilot.LanguageServer.Semantics;
using Pilot.Native.CAbi;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Comprehensive test suite for Milestone 13:
/// Values-First Error System (Transparent, Localized, Must-Use Return Errors `T ! E`).
/// Covers deliverables 13.1 (`T ! E` and inferred `T!` grammar + zero-unwinding register ABI),
/// 13.2 (universal postfix `?` propagation, error `adapter`s, inline `catch`, `guard ... catch`),
/// and 13.3 (`ERR001`–`ERR005` must-use enforcement, partial `catch` narrowing, and error stratification).
/// </summary>
public class PilotErrorSystemTests
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
    // 13.1: Fallible Return Type Grammar (`T ! E`, `T!`) & Zero-Unwinding ABI
    // =========================================================================

    [Fact]
    public void FallibleGrammar_PrecedenceAndInferredErrorSets_ParseAndAnalyzeCleanly()
    {
        // `User? ! IoError | ParseError` parses as `(User?) ! (IoError | ParseError)` (Conflict Resolution #6)
        bool splitOk = PilotAdtMemoryLayout.TrySplitFallibleTypeSpec(
            "User? ! IoError | ParseError",
            out string okPart,
            out string errPart);
        Assert.True(splitOk);
        Assert.Equal("User?", okPart);
        Assert.Equal("IoError | ParseError", errPart);

        string code = @"
type ConfigError =
    | FileMissing(path: str)
    | BadPort(raw: str);

def read_port_inferred(path: str) -> int! {
    if path == ""missing.conf"" {
        return fail(FileMissing(path));
    }
    if path == ""bad.conf"" {
        return fail(BadPort(path));
    }
    return 8080;
}

let p = read_port_inferred(""app.conf"") catch 80;
";
        var analysis = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.False(analysis.HasErrors, string.Join("; ", analysis.Diagnostics));
        Assert.True(analysis.InferredFunctionReturnTypes.TryGetValue("read_port_inferred", out string? inferredSig));
        Assert.Equal("int ! FileMissing | BadPort", inferredSig);
    }

    [Fact]
    public void ZeroUnwindingRegisterAbi_ResultMemoryLayoutAndCAbiHeader_AreDeterministic()
    {
        // `int ! int` -> `{ u8 is_ok; union { i64 ok; i64 err } }` (size 16, align 8, 0 heap allocations)
        var layout = PilotAdtMemoryLayout.ComputeResultLayout("int", "int");
        Assert.Equal(16, layout.SizeBytes);
        Assert.Equal(8, layout.AlignmentBytes);
        Assert.Equal(8, layout.PayloadOffsetBytes);
        Assert.Equal(8, layout.PayloadSizeBytes);

        string code = @"
def compute_tick(input: int) -> int ! int {
    if input < 0 {
        return fail(-1);
    }
    return input * 2;
}
sizeof_type(""int ! int"")
";
        AssertParity(code, PilotValue.FromInt(16));

        string cHeader = PilotCAbiEmitter.EmitHeader(code, "tick_abi");
        Assert.Contains("struct { uint8_t is_ok; union { int64_t ok; int64_t err; } payload; }", cHeader);
    }

    // =========================================================================
    // 13.2: Universal Postfix `?`, Error Adapters, `catch`, & `guard ... catch`
    // =========================================================================

    [Fact]
    public void PostfixQuestionPropagation_AndErrorAdapters_InterpreterAndPilVmParity()
    {
        string code = @"
type ConfigError =
    | FileMissing(path: str)
    | BadPort(raw: str);

record ServiceError(status: int, reason: str);

adapter ConfigError -> ServiceError {
    def convert(err: ConfigError) -> ServiceError {
        return match err {
            case FileMissing(p) => ServiceError(404, p),
            case BadPort(r) => ServiceError(400, r)
        };
    }
}

def fs_read_text(path: str) -> str ! ConfigError {
    if path == ""missing.conf"" {
        return fail(FileMissing(path));
    }
    if path == ""invalid.conf"" {
        return ""not_an_int"";
    }
    return ""  8080  "";
}

def read_port(path: str) -> int ! ConfigError {
    let text = fs_read_text(path)?;
    guard some parsed = text.Trim().ToInt() else {
        return fail(BadPort(text.Trim()));
    }
    return parsed;
}

def start_gateway(path: str) -> int ! ServiceError {
    let p = read_port(path)?;
    return p + 10;
}

let ok_val = start_gateway(""app.conf"") catch -1;
let err_missing = start_gateway(""missing.conf"") catch err => err.status;
let err_bad = start_gateway(""invalid.conf"") catch err => err.status;
f""{ok_val}:{err_missing}:{err_bad}""
";
        AssertParity(code, PilotValue.FromString("8090:404:400"));
    }

    [Fact]
    public void PostfixQuestionOnOptionals_PropagatesNoneCleanly_InterpreterAndPilVmParity()
    {
        string code = @"
def parse_and_double(raw: str?) -> int? {
    let s = raw?;
    let n = s.Trim().ToInt()?;
    return n * 2;
}

let r1 = parse_and_double(""21"") ?? -1;
let r2 = parse_and_double(""abc"") ?? -2;
let r3 = parse_and_double(none) ?? -3;
f""{r1}:{r2}:{r3}""
";
        AssertParity(code, PilotValue.FromString("42:-2:-3"));
    }

    [Fact]
    public void InlineCatch_AndGuardCatch_InterpreterAndPilVmParity()
    {
        string code = @"
type ConfigError =
    | FileMissing(path: str)
    | BadPort(raw: str);

def read_port(path: str) -> int ! ConfigError {
    if path == ""missing.conf"" {
        return fail(FileMissing(path));
    }
    if path == ""bad.conf"" {
        return fail(BadPort(""xyz""));
    }
    return 9000;
}

// 1. Concise default value on any error
let p1 = read_port(""missing.conf"") catch 8080;

// 2. Localized error inspection/exhaustiveness right at the call site
let p2 = read_port(""bad.conf"") catch err => match err {
    case FileMissing(_) => 8080,
    case BadPort(raw) => 7000 + raw.length
};

// 3. Flat happy-path guard-catch
def boot_message(path: str) -> str {
    guard let active_port = read_port(path) catch err {
        return f""ERR:{err}"";
    }
    return f""OK:{active_port}"";
}

let b1 = boot_message(""app.conf"");
let b2 = boot_message(""missing.conf"");
f""{p1}|{p2}|{b1}|{b2}""
";
        AssertParity(
            code,
            PilotValue.FromString("8080|7003|OK:9000|ERR:FileMissing(path: missing.conf)"));
    }

    [Fact]
    public void ResultPatternMatching_OkAndFailConstructors_AndExhaustivenessCheck()
    {
        string code = @"
def divide(a: int, b: int) -> int ! str {
    if b == 0 {
        return fail(""div0"");
    }
    return a / b;
}

let r1 = match divide(10, 2) {
    case ok(v) => v * 10,
    case fail(_) => -1
};
let r2 = match divide(10, 0) {
    case ok(v) => v * 10,
    case fail(msg) => msg.length
};
r1 + r2
";
        AssertParity(code, PilotValue.FromInt(54));

        // Non-exhaustive match missing `case fail(_)` on `int ! str` emits `MATCH001` with `fail(_)` witness
        string nonExhaustive = @"
def divide(a: int, b: int) -> int ! str {
    if b == 0 { return fail(""div0""); }
    return a / b;
}
let res = divide(10, 2);
let x = match res {
    case ok(v) => v
};
";
        var analysis = PilotAnalysisPipeline.AnalyzeSource(nonExhaustive);
        Assert.Contains(analysis.Diagnostics, d => d.Code == "MATCH001" && d.Message.Contains("fail("));
    }

    // =========================================================================
    // 13.3: Must-Use Enforcement & Unified Error Stratification (`ERR001`–`ERR005`)
    // =========================================================================

    [Fact]
    public void Err001_UnhandledStatementLevelFallibleCall_IsCompileTimeError()
    {
        string badCode = @"
type IoError = | DiskFull | PermissionDenied;

def write_log(msg: str) -> int ! IoError {
    return msg.length;
}

write_log(""hello"");
";
        var analysis = PilotAnalysisPipeline.AnalyzeSource(badCode);
        Assert.Contains(analysis.Diagnostics, d => d.Code == "ERR001");

        var lsp = PilotSemanticAnalyzer.Analyze(badCode);
        Assert.Contains(lsp.Diagnostics, d => d.Code == "ERR001");

        // Handled with `ignore_err(...)` or `catch` or `?` passes cleanly!
        string goodCode = @"
type IoError = | DiskFull | PermissionDenied;

def write_log(msg: str) -> int ! IoError {
    return msg.length;
}

ignore_err(write_log(""ignored""));
let written = write_log(""handled"") catch 0;
";
        var goodAnalysis = PilotAnalysisPipeline.AnalyzeSource(goodCode);
        Assert.DoesNotContain(goodAnalysis.Diagnostics, d => d.Code == "ERR001");
    }

    [Fact]
    public void Err002_IncompatibleQuestionPropagation_OrMissingErrorAdapter_IsRejected()
    {
        string badCode = @"
type IoError = | DiskFull;
type DbError = | Deadlock;

def read_disk() -> int ! IoError {
    return 1;
}

// 1. Propagating `IoError` into `DbError` with no `adapter IoError -> DbError`
def query_db() -> int ! DbError {
    let x = read_disk()?;
    return x;
}
";
        var analysis = PilotAnalysisPipeline.AnalyzeSource(badCode);
        Assert.Contains(analysis.Diagnostics, d => d.Code == "ERR002" && d.Message.Contains("DiskFull"));

        // Adding `adapter IoError -> DbError` resolves ERR002!
        string adaptedCode = @"
type IoError = | DiskFull;
type DbError = | Deadlock | StorageFault;

adapter IoError -> DbError {
    def convert(e: IoError) -> DbError {
        return StorageFault;
    }
}

def read_disk() -> int ! IoError {
    return 1;
}

def query_db() -> int ! DbError {
    let x = read_disk()?;
    return x;
}
";
        var adaptedAnalysis = PilotAnalysisPipeline.AnalyzeSource(adaptedCode);
        Assert.DoesNotContain(adaptedAnalysis.Diagnostics, d => d.Code == "ERR002");
    }

    [Fact]
    public void Err003_NonExhaustiveCatch_AndPartialCatchErrorUnionNarrowing()
    {
        // 1. Non-exhaustive `catch err => match err` missing `BadPort` emits `ERR003`
        string nonExhaustiveCatch = @"
type ConfigError =
    | FileMissing(path: str)
    | BadPort(raw: str);

def read_port(path: str) -> int ! ConfigError {
    return 8080;
}

let p = read_port(""app.conf"") catch err => match err {
    case FileMissing(_) => 8080
};
";
        var badAnalysis = PilotAnalysisPipeline.AnalyzeSource(nonExhaustiveCatch);
        Assert.Contains(badAnalysis.Diagnostics, d => d.Code == "ERR003" && d.Message.Contains("BadPort"));

        // 2. Partial `catch` narrowing shrinks `FileMissing | BadPort` -> `BadPort`,
        // allowing `?` to propagate into a function declared as `-> int ! BadPort`!
        string partialCatchNarrowing = @"
type ConfigError =
    | FileMissing(path: str)
    | BadPort(raw: str);

def read_port(path: str) -> int ! ConfigError {
    return 8080;
}

def load_strict_port(path: str) -> int ! BadPort {
    let p = (read_port(path) catch err => match err {
        case FileMissing(_) => 8080,
        case other => fail(other)
    })?;
    return p;
}
";
        var narrowedAnalysis = PilotAnalysisPipeline.AnalyzeSource(partialCatchNarrowing);
        Assert.DoesNotContain(narrowedAnalysis.Diagnostics, d => d.Code is "ERR002" or "ERR003");
    }

    [Fact]
    public void Err004_ReturningFailFromNonFallibleFunction_OrUndeclaredErrorVariant_IsRejected()
    {
        string badCode = @"
type IoError = | NotFound;

def plain_func() -> int {
    return fail(NotFound);
}
";
        var analysis = PilotAnalysisPipeline.AnalyzeSource(badCode);
        Assert.Contains(analysis.Diagnostics, d => d.Code == "ERR004");
    }

    [Fact]
    public void Err005_ErrorStratificationLint_ScopesTryExceptToIsolateAndFfiBoundaries()
    {
        string nativeTryCode = @"
def compute() -> int {
    try {
        return 1;
    } except e {
        return 0;
    }
}
";
        var nativeAnalysis = PilotAnalysisPipeline.AnalyzeSource(nativeTryCode);
        Assert.Contains(nativeAnalysis.Diagnostics, d => d.Code == "ERR005" && !d.IsError);

        // Inside `isolate` supervisor or `@ffi_boundary`, `try/except` is clean (no ERR005)
        string isolateAndFfiCode = @"
@ffi_boundary
def call_foreign() -> int {
    try {
        return 1;
    } except e {
        return 0;
    }
}

isolate {
    try {
        let x = 1;
    } except e {
        let y = 0;
    }
}
";
        var boundaryAnalysis = PilotAnalysisPipeline.AnalyzeSource(isolateAndFfiCode);
        Assert.DoesNotContain(boundaryAnalysis.Diagnostics, d => d.Code == "ERR005");
    }

    [Fact]
    public void EndToEnd_PilotErrorSystemTestScript_AndShowcase_ExecuteCleanly()
    {
        string repoRoot = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", ".."));
        string testScriptPath = Path.Combine(repoRoot, "tests", "30_values_first_errors.pilot");
        string showcasePath = Path.Combine(repoRoot, "examples", "showcase", "18_values_first_error_system.pilot");

        string testCode = File.ReadAllText(testScriptPath);
        var interpRes = RunInterpreter(testCode);
        Assert.True(interpRes.Success, "Interpreter failed on 30_values_first_errors.pilot: " + string.Join("; ", interpRes.Diagnostics));

        var vmRes = RunBytecode(testCode);
        Assert.True(vmRes.Success, "PilVM failed on 30_values_first_errors.pilot: " + string.Join("; ", vmRes.Diagnostics));

        string showcaseCode = File.ReadAllText(showcasePath);
        var showInterp = RunInterpreter(showcaseCode);
        Assert.True(showInterp.Success, "Interpreter failed on showcase 18: " + string.Join("; ", showInterp.Diagnostics));

        var showVm = RunBytecode(showcaseCode);
        Assert.True(showVm.Success, "PilVM failed on showcase 18: " + string.Join("; ", showVm.Diagnostics));
    }
}
