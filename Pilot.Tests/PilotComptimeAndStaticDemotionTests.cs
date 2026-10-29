using System;
using System.IO;
using System.Linq;
using Antlr4.Runtime;
using Pilot.Analysis;
using Pilot.Analysis.Comptime;
using Pilot.Cli;
using Pilot.Compiler;
using Pilot.LanguageServer.Semantics;
using Pilot.Preprocessing;
using Pilot.Runtime.Bytecode;
using Pilot.Runtime.Values;
using Pilot.Scripting;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Comprehensive test suite for Milestone 19:
/// 19.1: Compiler-Wide Runtime-to-Compile-Time Demotion Matrix (upvalues, field slots, DTO projections,
///       unit erasure, contract elision, O(1) context display slots).
/// 19.2: First-Class `comptime` Execution &amp; Frozen `.rodata` Materialization (`comptime { ... }`,
///       `comptime(...)`, `comptime` parameters, `comptime assert`, `PilotFrozenMap` perfect hashing,
///       and `.pilc` constant pool serialization).
/// 19.3: Runtime-Leak Verification CLI (`pilot check --comptime-audit`) &amp; `@zero_runtime` / `@hard_realtime`
///       diagnostics (`COMPTIME001`–`COMPTIME010`).
/// </summary>
public class PilotComptimeAndStaticDemotionTests
{
    private static PilotValue EvalCode(string code)
    {
        var res = PilotScript.Run(code);
        if (!res.Success)
        {
            throw new InvalidOperationException(string.Join("; ", res.Diagnostics));
        }
        return res.Value;
    }

    private static PilotParser.ProgramContext ParseProgram(string source)
    {
        var preproc = PilotPreprocessor.Preprocess(source, "test.pilot");
        var input = new AntlrInputStream(preproc.PreprocessedSource);
        var lexer = new PilotLexer(input);
        lexer.RemoveErrorListeners();
        var tokens = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokens);
        parser.RemoveErrorListeners();
        return parser.ParseProgramFast();
    }

    // =========================================================================
    // 19.1: Runtime-to-Compile-Time Demotion Matrix
    // =========================================================================

    [Fact]
    public void UpvalueDisplay_NestedClosure_EmitsLoadAndStoreUpvalueOpcodes()
    {
        string code = @"
def make_accumulator(start: int) -> int {
    var acc = start;
    def add_to_acc(delta: int) -> int {
        acc += delta;
        return acc;
    }
    let a = add_to_acc(10);
    let b = add_to_acc(25);
    return a + b;
}
make_accumulator(5);
";
        var tree = ParseProgram(code);
        var chunk = PilotBytecodeCompiler.Compile(tree, code);

        var outerChunk = chunk.NestedChunks.First(c => c.Name == "make_accumulator");
        var innerChunk = outerChunk.NestedChunks.First(c => c.Name == "add_to_acc");

        Assert.Contains(innerChunk.Instructions, i => i.Opcode == PilotOpcode.LoadUpvalue && i.Extra == 1);
        Assert.Contains(innerChunk.Instructions, i => i.Opcode == PilotOpcode.StoreUpvalue && i.Extra == 1);
        Assert.DoesNotContain(innerChunk.Instructions, i => i.Opcode == PilotOpcode.LoadNamed);

        var res = EvalCode(code);
        // a = 15, b = 40 -> 55
        Assert.Equal(55L, res.AsInt());
    }

    [Fact]
    public void StaticFieldLayout_StructAndRecordAccess_EmitsGetFieldSlotAndSetFieldSlot()
    {
        string code = @"
struct SensorReading {
    var id: int;
    var voltage: float;
    var active: bool;
}

def update_and_read(s: SensorReading) -> float {
    s.voltage = s.voltage + 1.25;
    return s.voltage;
}

var reading = SensorReading(42, 3.75, true);
let updated = update_and_read(reading);
updated;
";
        var tree = ParseProgram(code);
        var chunk = PilotBytecodeCompiler.Compile(tree, code);
        var fnChunk = chunk.NestedChunks.First(c => c.Name == "update_and_read");

        Assert.Contains(fnChunk.Instructions, i => i.Opcode == PilotOpcode.GetFieldSlot && i.Operand == 1);
        Assert.Contains(fnChunk.Instructions, i => i.Opcode == PilotOpcode.SetFieldSlot && i.Operand == 1);

        var res = EvalCode(code);
        Assert.Equal(5.0, res.AsFloat(), 4);
    }

    [Fact]
    public void DtoArrowProjection_LowersToGetFieldSlotAndConstructStruct()
    {
        string code = @"
struct SourceTelemetry {
    var id: int;
    var extra: str;
    var score: int;
}

struct TargetSummary {
    var score: int;
    var id: int;
}

def project_telemetry(src: SourceTelemetry) -> TargetSummary {
    return src -> TargetSummary;
}

let src = SourceTelemetry(101, ""ignored"", 900);
let dst = project_telemetry(src);
dst.id + dst.score;
";
        var tree = ParseProgram(code);
        var chunk = PilotBytecodeCompiler.Compile(tree, code);
        var fnChunk = chunk.NestedChunks.First(c => c.Name == "project_telemetry");

        Assert.Contains(fnChunk.Instructions, i => i.Opcode == PilotOpcode.ConstructStruct);
        Assert.DoesNotContain(fnChunk.Instructions, i => i.Opcode == PilotOpcode.EvalAst);

        var res = EvalCode(code);
        Assert.Equal(1001L, res.AsInt());
    }

    [Fact]
    public void DimensionalUnitErasure_ErasesUnitTrailerInBytecodeWithZeroEvalAst()
    {
        string code = @"
def kinetic_step(dist: float, dt: float) -> float {
    let d = 120.0<m>;
    let t = 4.0<s>;
    return (d / t) + (dist / dt);
}
kinetic_step(30.0, 2.0);
";
        var tree = ParseProgram(code);
        var analysis = PilotAnalysisPipeline.Analyze(tree);
        Assert.NotNull(analysis.DemotionReport);
        Assert.True(analysis.DemotionReport!.ErasedUnitCount >= 2);

        var chunk = PilotBytecodeCompiler.Compile(tree, code);
        var fnChunk = chunk.NestedChunks.First(c => c.Name == "kinetic_step");
        Assert.DoesNotContain(fnChunk.Instructions, i => i.Opcode == PilotOpcode.EvalAst);

        var res = EvalCode(code);
        Assert.Equal(45.0, res.AsFloat(), 4);
    }

    [Fact]
    public void ContractElision_ProvenAssertionsAndRequires_AreElidedFromBytecode()
    {
        string code = @"
const LIMIT = 100;

def clamp_checked(x: int) -> int requires x > 0 && x <= 100 {
    assert x > 0;
    return x * 2;
}

let out = clamp_checked(25);
assert LIMIT == 100;
out;
";
        var tree = ParseProgram(code);
        var analysis = PilotAnalysisPipeline.Analyze(tree);
        Assert.NotNull(analysis.DemotionReport);
        Assert.Contains("clamp_checked", analysis.DemotionReport!.FullyDischargedFunctions);
        Assert.NotEmpty(analysis.DemotionReport.ElidedContractNodes);

        var chunk = PilotBytecodeCompiler.Compile(tree, code);
        Assert.DoesNotContain(chunk.Instructions, i => i.Opcode == PilotOpcode.AssertCheck);

        var res = EvalCode(code);
        Assert.Equal(50L, res.AsInt());
    }

    [Fact]
    public void ContextualDisplaySlot_WithContextAndContextParam_EmitsO1DisplayOpcodes()
    {
        string code = @"
def compute_with_scale(x: int, context scale: int = 2) -> int {
    return x * scale;
}

var total = compute_with_scale(10);
with context(scale = 7) {
    total += compute_with_scale(10);
}
total += compute_with_scale(10);
total;
";
        var tree = ParseProgram(code);
        var chunk = PilotBytecodeCompiler.Compile(tree, code);
        var fnChunk = chunk.NestedChunks.First(c => c.Name == "compute_with_scale");

        Assert.Contains(fnChunk.Instructions, i => i.Opcode == PilotOpcode.LoadContextSlot);
        Assert.Contains(chunk.Instructions, i => i.Opcode == PilotOpcode.PushContextSlot);
        Assert.Contains(chunk.Instructions, i => i.Opcode == PilotOpcode.PopContextSlot);

        var res = EvalCode(code);
        // 20 + 70 + 20 = 110
        Assert.Equal(110L, res.AsInt());
    }

    // =========================================================================
    // 19.2: First-Class `comptime` Execution & Frozen `.rodata` Materialization
    // =========================================================================

    [Fact]
    public void ComptimeExprAndStmt_FoldsLoopsAndFunctionsIntoFrozenConstants()
    {
        string code = @"
comptime def fib(n: int) -> int {
    if n <= 1 { return n; }
    var a = 0;
    var b = 1;
    for i in 2..=n {
        let next = a + b;
        a = b;
        b = next;
    }
    return b;
}

const FIB_10 = comptime(fib(10));
let SQUARES = comptime {
    var acc = [];
    for i in 1..=5 {
        acc.append(i * i);
    }
    return acc;
};

comptime assert FIB_10 == 55, ""FIB_10 must be 55"";
FIB_10 + SQUARES[4] + (100 if SQUARES.is_frozen else 0);
";
        var tree = ParseProgram(code);
        var analysis = PilotAnalysisPipeline.Analyze(tree);
        Assert.DoesNotContain(analysis.Diagnostics, d => d.IsError);
        Assert.NotNull(analysis.DemotionReport);
        Assert.NotEmpty(analysis.DemotionReport!.MaterializedComptimeValues);

        var res = EvalCode(code);
        // 55 + 25 + 100 = 180
        Assert.Equal(180L, res.AsInt());
    }

    [Fact]
    public void FrozenPerfectHashMap_ComptimeDictionary_BuildsZeroCollisionDisplacementTableAndRoundTripsPilc()
    {
        string code = @"
const HTTP_STATUS = comptime({
    ""ok"": 200,
    ""created"": 201,
    ""bad_request"": 400,
    ""unauthorized"": 401,
    ""not_found"": 404,
    ""server_error"": 500
});

let code_sum = HTTP_STATUS[""ok""] + HTTP_STATUS[""not_found""] + HTTP_STATUS[""server_error""];
let flags = (1000 if HTTP_STATUS.is_frozen else 0) + (2000 if HTTP_STATUS.is_perfect_hash else 0);
code_sum + flags;
";
        var tree = ParseProgram(code);
        var chunk = PilotBytecodeCompiler.Compile(tree, code);

        // Verify constant pool contains a PilotFrozenMap with IsPerfectHash == true
        var frozenMapConst = chunk.Constants.FirstOrDefault(c => c.Kind == PilotValueKind.Map && c.IsFrozen);
        Assert.True(frozenMapConst.TryGetFrozenMap(out var frozenMap));
        Assert.NotNull(frozenMap);
        Assert.True(frozenMap!.IsPerfectHash);
        Assert.True(frozenMap.TryLookupFast(PilotValue.FromString("not_found"), out var nfVal));
        Assert.Equal(404L, nfVal.AsInt());

        // Round-trip through .pilc binary serialization
        using var ms = new MemoryStream();
        using (var bw = new BinaryWriter(ms, System.Text.Encoding.UTF8, leaveOpen: true))
        {
            chunk.Write(bw);
        }
        ms.Position = 0;
        using var br = new BinaryReader(ms, System.Text.Encoding.UTF8, leaveOpen: true);
        var deserializedChunk = PilotBytecodeChunk.Read(br);

        var roundTripMapConst = deserializedChunk.Constants.FirstOrDefault(c => c.Kind == PilotValueKind.Map && c.IsFrozen);
        Assert.True(roundTripMapConst.TryGetFrozenMap(out var roundTripMap));
        Assert.True(roundTripMap!.IsPerfectHash);
        Assert.True(roundTripMap.TryLookupFast(PilotValue.FromString("server_error"), out var seVal));
        Assert.Equal(500L, seVal.AsInt());

        // Execute deserialized chunk on PilVirtualMachine
        var engine = new Pilot.Runtime.Execution.PilotEngine();
        var ctx = new Pilot.Runtime.Execution.PilotExecutionContext(engine, new Pilot.Runtime.Execution.PilotExecutionOptions());
        var vmRes = PilVirtualMachine.Execute(deserializedChunk, ctx);
        // (200 + 404 + 500) + 1000 + 2000 = 4104
        Assert.Equal(4104L, vmRes.AsInt());
    }

    [Fact]
    public void FrozenCollection_RuntimeMutation_RaisesError()
    {
        string code = @"
let lut = comptime([10, 20, 30]);
lut[0] = 99;
";
        var ex = Assert.ThrowsAny<Exception>(() => EvalCode(code));
        Assert.Contains("frozen", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void ComptimeParameter_FoldsWithConstantArgAndRejectsDynamicArgWithCOMPTIME010()
    {
        string validCode = @"
def unroll_power(base_val: int, comptime exp: int) -> int {
    var acc = 1;
    for i in 1..=exp {
        acc = acc * base_val;
    }
    return acc;
}
unroll_power(3, 4);
";
        var validRes = EvalCode(validCode);
        Assert.Equal(81L, validRes.AsInt());

        string invalidCode = @"
def unroll_power(base_val: int, comptime exp: int) -> int {
    return base_val ** exp;
}
var runtime_exp = 4;
let x = unroll_power(3, runtime_exp);
";
        var sem = PilotSemanticAnalyzer.Analyze(invalidCode);
        Assert.Contains(sem.Diagnostics, d => d.Code == "COMPTIME010");
    }

    [Fact]
    public void ComptimeAssertAndPurity_ReportsCOMPTIME008AndCOMPTIME009()
    {
        string assertFailCode = @"
const PROTOCOL_VER = 3;
comptime assert PROTOCOL_VER == 4, ""Protocol version must be 4"";
";
        var semAssert = PilotSemanticAnalyzer.Analyze(assertFailCode);
        Assert.Contains(semAssert.Diagnostics, d => d.Code == "COMPTIME008");

        string impureCode = @"
let bad = comptime {
    println(""side effect in comptime"");
    return 1;
};
";
        var semImpure = PilotSemanticAnalyzer.Analyze(impureCode);
        Assert.Contains(semImpure.Diagnostics, d => d.Code == "COMPTIME009");
    }

    // =========================================================================
    // 19.3: `@zero_runtime` Verification & `pilot check --comptime-audit` CLI
    // =========================================================================

    [Fact]
    public void ZeroRuntimeDecorator_FlagsDynamicLeaksAndAcceptsFullyDemotedFunction()
    {
        string cleanCode = @"
struct Vec2 {
    var x: float;
    var y: float;
}

@zero_runtime
def dot_product(a: Vec2, b: Vec2) -> float {
    return (a.x * b.x) + (a.y * b.y);
}
";
        var cleanSem = PilotSemanticAnalyzer.Analyze(cleanCode);
        Assert.DoesNotContain(cleanSem.Diagnostics, d => d.Code.StartsWith("COMPTIME", StringComparison.Ordinal));

        string leakyCode = @"
@zero_runtime
def leaky_fn(obj: any) -> int {
    dynamic d = 10;
    return obj.unknown_field + d;
}
";
        var leakySem = PilotSemanticAnalyzer.Analyze(leakyCode);
        Assert.Contains(leakySem.Diagnostics, d => d.Code == "COMPTIME001");
        Assert.Contains(leakySem.Diagnostics, d => d.Code == "COMPTIME002");
    }

    [Fact]
    public void CliCheckComptimeAudit_PrintsStructuredDemotionReportAndReturnsZeroForCleanFile()
    {
        string tempFile = Path.Combine(Path.GetTempPath(), $"pilot_m19_audit_{Guid.NewGuid():N}.pilot");
        try
        {
            File.WriteAllText(tempFile, @"
struct PacketHeader {
    var magic: int;
    var length: int;
}

const MAGIC_LUT = comptime({
    ""v1"": 101,
    ""v2"": 202
});

@zero_runtime
def verify_header(hdr: PacketHeader) -> int {
    let dist = 50.0<m>;
    return hdr.magic + hdr.length;
}
");
            var sw = new StringWriter();
            var prevOut = Console.Out;
            int exitCode;
            try
            {
                Console.SetOut(sw);
                exitCode = Program.Main(new[] { "check", "--comptime-audit", tempFile });
            }
            finally
            {
                Console.SetOut(prevOut);
            }

            string output = sw.ToString();
            Assert.Equal(0, exitCode);
            Assert.Contains("Pilot Compile-Time Demotion & .rodata Audit", output);
            Assert.Contains("Frozen Perfect-Hash Maps: 1", output);
            Assert.Contains("verify_header [@zero_runtime]", output);
            Assert.Contains("[CHECK PASS]", output);
        }
        finally
        {
            if (File.Exists(tempFile)) File.Delete(tempFile);
        }
    }
}
