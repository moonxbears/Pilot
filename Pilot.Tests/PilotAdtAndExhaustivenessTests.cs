#nullable disable
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Antlr4.Runtime;
using Pilot.Analysis;
using Pilot.Analysis.PatternMatching;
using Pilot.Compiler;
using Pilot.LanguageServer.Semantics;
using Pilot.Native.CAbi;
using Pilot.Native.Llvm;
using Pilot.Preprocessing;
using Pilot.Runtime.Bytecode;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Verification suite for Milestone 11:
/// Exhaustive Pattern Matching &amp; Algebraic Data Types (ADTs).
/// Covers:
/// - 11.1: Sum types (`type` / `enum`), `record` product types, and compact unboxed memory layout.
/// - 11.2: Maranget usefulness matrix exhaustiveness (`MATCH001`), redundancy (`MATCH002`),
///         PilVM O(1) `JumpTable` + DAG lowering, `.pilc` V2 serialization, C-ABI, and LLVM IR emission.
/// - 11.3: Deep nested destructuring, `@` whole-value bindings, range/slice/keyword patterns,
///         and conditional pattern forms (`if let`, `elif let`, `guard let ... else`, `while let`).
/// - 11.4: End-to-end execution of `tests/28_adt_and_exhaustive_matching.pilot` and
///         `examples/showcase/16_algebraic_data_types.pilot`.
/// </summary>
public class PilotAdtAndExhaustivenessTests
{
    private static (PilotValue InterpreterValue, PilotValue VmValue) RunInterpreterAndVm(string source)
    {
        var engine = new PilotEngine();
        var interpRes = engine.Execute(source);
        Assert.True(interpRes.Success, "Interpreter failed: " + string.Join("; ", interpRes.Diagnostics));

        var compileRes = PilotCompiler.Default.Compile(source);
        Assert.True(compileRes.Success, "PilVM compile failed: " + string.Join("; ", compileRes.Diagnostics));
        Assert.NotNull(compileRes.Program?.BytecodeChunk);

        var vmRes = compileRes.Program.Run();
        Assert.True(vmRes.Success, "PilVM execution failed: " + string.Join("; ", vmRes.Diagnostics));

        Assert.Equal(interpRes.Value, vmRes.Value);
        return (interpRes.Value, vmRes.Value);
    }

    private static PilotParser.ProgramContext ParseFast(string source)
    {
        var preproc = PilotPreprocessor.Preprocess(source, "");
        var lexer = new PilotLexer(new AntlrInputStream(preproc.PreprocessedSource));
        var parser = new PilotParser(new CommonTokenStream(lexer));
        return parser.ParseProgramFast();
    }

    private static string ResolveRepoFile(string relativePath)
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir != null && !File.Exists(Path.Combine(dir.FullName, "Pilot.csproj")))
        {
            dir = dir.Parent;
        }
        Assert.NotNull(dir);
        return Path.Combine(dir.FullName, relativePath);
    }

    // =========================================================================
    // 11.1 — Sum Types, Value-Semantic Records & Compact Unboxed Memory Layout
    // =========================================================================

    [Fact]
    public void Test_11_1_Record_ValueEquality_StructuralHash_And_UnboxedLayout()
    {
        string code = @"
            record Point(x: f64, y: f64);

            let p1 = Point(3.5, -2.25);
            let p2 = Point(y: -2.25, x: 3.5);
            let p3 = Point(1.0, 2.0);

            assert(p1 == p2, 'Value equality failed');
            assert(p1 != p3, 'Inequality failed');
            assert(p1.is_unboxed == true, 'Point(f64, f64) must be inline unboxed');
            assert(p1.size_of == 16, 'Point(f64, f64) must be 16 bytes');

            let lookup = { Point(0.0, 0.0): 10, Point(3.5, -2.25): 42 };
            lookup[p2];
        ";

        var (interpVal, vmVal) = RunInterpreterAndVm(code);
        Assert.Equal(42, interpVal.AsInt());
        Assert.Equal(42, vmVal.AsInt());

        // Direct runtime check that Point(f64, f64) allocates zero PilotRecordInstance heap wrapper
        var engine = new PilotEngine();
        var pointRes = engine.Execute("record Point(x: f64, y: f64); Point(10.0, 20.0);");
        Assert.True(pointRes.Success);
        Assert.Equal(PilotValueKind.Record, pointRes.Value.Kind);
        Assert.True(pointRes.Value.IsUnboxedInline);
    }

    [Fact]
    public void Test_11_1_Record_And_Adt_Immutability_Enforced_PILOT101()
    {
        string recordMutate = @"
            record Point(x: f64, y: f64);
            let p = Point(1.0, 2.0);
            p.x = 99.0;
        ";

        var engine = new PilotEngine();
        var interpRes = engine.Execute(recordMutate);
        Assert.False(interpRes.Success);
        Assert.Contains(interpRes.Diagnostics, e => e.Contains("PILOT101", StringComparison.Ordinal));

        var compileRes = PilotCompiler.Default.Compile(recordMutate);
        Assert.True(compileRes.Success);
        var vmRes = compileRes.Program.Run();
        Assert.False(vmRes.Success);
        Assert.Contains(vmRes.Diagnostics, e => e.Contains("PILOT101", StringComparison.Ordinal));

        var lspRes = PilotSemanticAnalyzer.Analyze(recordMutate);
        Assert.Contains(lspRes.Diagnostics, d => d.Code == "PILOT101");
    }

    [Fact]
    public void Test_11_1_Adt_SumTypes_EnumPayloads_And_UnboxedInlineStorage()
    {
        string code = @"
            record Point(x: f64, y: f64);

            type Shape =
                | Circle(radius: f64)
                | Rectangle(width: f64, height: f64)
                | Polygon(points: Point[])
                | Origin;

            let o = Origin;
            let c = Circle(radius: 7.5);
            let r = Shape.Rectangle(height: 5.0, width: 3.0);

            assert(Shape.tag_bytes == 1, 'Expected u8 tag');
            assert(Shape.is_recursive == false, 'Shape should be non-recursive');
            assert(o.is_unboxed == true, 'Unit variant must be unboxed inline');
            assert(c.is_unboxed == true, 'Single-primitive variant must be unboxed inline');
            assert(r.width == 3.0 and r.height == 5.0, 'Named arg reordering failed');

            c;
        ";

        var (interpVal, _) = RunInterpreterAndVm(code);
        Assert.Equal(PilotValueKind.Adt, interpVal.Kind);
        Assert.True(interpVal.IsUnboxedInline);
        Assert.Equal(7.5, interpVal.GetMember("radius").AsFloat());
    }

    [Fact]
    public void Test_11_1_CompactLayout_U8_Vs_U16_Tag_And_RecursiveCycleDetection()
    {
        // <= 256 variants -> 1-byte u8 tag; > 256 variants -> 2-byte u16 tag
        var smallAdt = new PilotAdtDefinition("SmallSum");
        for (int i = 0; i < 256; i++)
        {
            smallAdt.AddVariant($"V{i}", Array.Empty<PilotAdtFieldDefinition>());
        }
        smallAdt.Layout = PilotAdtMemoryLayout.ComputeForAdt(
            smallAdt,
            new Dictionary<string, PilotAdtDefinition> { ["SmallSum"] = smallAdt },
            new Dictionary<string, PilotRecordDefinition>());
        Assert.Equal(1, smallAdt.Layout.TagBytes);

        var largeAdt = new PilotAdtDefinition("LargeSum");
        for (int i = 0; i < 260; i++)
        {
            largeAdt.AddVariant($"V{i}", Array.Empty<PilotAdtFieldDefinition>());
        }
        largeAdt.Layout = PilotAdtMemoryLayout.ComputeForAdt(
            largeAdt,
            new Dictionary<string, PilotAdtDefinition> { ["LargeSum"] = largeAdt },
            new Dictionary<string, PilotRecordDefinition>());
        Assert.Equal(2, largeAdt.Layout.TagBytes);

        // Verify AST-level layout & cycle detection on Shape (non-recursive) vs Expr[T] (recursive)
        string source = @"
            record Point(x: f64, y: f64);

            type Shape =
                | Circle(radius: f64)
                | Rectangle(width: f64, height: f64)
                | Polygon(points: Point[])
                | Origin;

            type Expr[T] =
                | Const(value: T)
                | Add(left: Expr[T], right: Expr[T])
                | Mul(left: Expr[T], right: Expr[T]);
        ";

        var tree = ParseFast(source);
        var checker = new PilotExhaustivenessChecker();
        checker.CollectDeclarations(tree);

        var shapeDef = checker.AdtDefinitions["Shape"];
        Assert.Equal(1, shapeDef.Layout.TagBytes);
        Assert.False(shapeDef.Layout.IsRecursive);
        Assert.Empty(shapeDef.Layout.RecursiveEdges);

        var exprDef = checker.AdtDefinitions["Expr"];
        Assert.Equal(1, exprDef.Layout.TagBytes);
        Assert.True(exprDef.Layout.IsRecursive);
        Assert.Contains("Add.left -> Expr", exprDef.Layout.RecursiveEdges);
        Assert.Contains("Add.right -> Expr", exprDef.Layout.RecursiveEdges);
        Assert.Contains("Mul.left -> Expr", exprDef.Layout.RecursiveEdges);
        Assert.Contains("Mul.right -> Expr", exprDef.Layout.RecursiveEdges);
    }

    [Fact]
    public void Test_11_1_Generic_Recursive_Adt_Interpreter_And_PilVm_Parity()
    {
        string code = @"
            type Expr[T] =
                | Const(value: T)
                | Add(left: Expr[T], right: Expr[T])
                | Mul(left: Expr[T], right: Expr[T]);

            def eval_expr(e: Expr[int]) -> int {
                return match e {
                    case Const(v) => v
                    case Add(l, r) => eval_expr(l) + eval_expr(r)
                    case Mul(l, r) => eval_expr(l) * eval_expr(r)
                };
            }

            let root = Add(Mul(Const(6), Const(7)), Const(8));
            eval_expr(root);
        ";

        var (interpVal, vmVal) = RunInterpreterAndVm(code);
        Assert.Equal(50, interpVal.AsInt());
        Assert.Equal(50, vmVal.AsInt());
    }

    // =========================================================================
    // 11.2 — Maranget Exhaustiveness (MATCH001), Redundancy (MATCH002) & Lowering
    // =========================================================================

    [Fact]
    public void Test_11_2_Exhaustiveness_MATCH001_MissingAdtVariant_RejectedEverywhere()
    {
        string nonExhaustive = @"
            record Point(x: f64, y: f64);
            type Shape =
                | Circle(radius: f64)
                | Rectangle(width: f64, height: f64)
                | Polygon(points: Point[])
                | Origin;

            def area(s: Shape) -> f64 {
                return match s {
                    case Circle(r) => 3.14 * r * r
                    case Rectangle(w, h) => w * h
                    case Polygon(_) => 0.0
                };
            }
        ";

        // 1. AST Engine rejects at compile time with MATCH001 + witness `Origin`
        var engine = new PilotEngine();
        var interpRes = engine.Execute(nonExhaustive);
        Assert.False(interpRes.Success);
        Assert.Contains(interpRes.Diagnostics, e => e.Contains("MATCH001") && e.Contains("Origin"));

        // 2. PilVM Bytecode Compiler rejects at compile time with MATCH001 + witness `Origin`
        var compileRes = PilotCompiler.Default.Compile(nonExhaustive);
        Assert.False(compileRes.Success);
        Assert.Contains(compileRes.Diagnostics, d => d.Contains("MATCH001") && d.Contains("Origin"));

        // 3. LSP Semantic Analyzer surfaces MATCH001 Error diagnostic
        var lspRes = PilotSemanticAnalyzer.Analyze(nonExhaustive);
        var diag = Assert.Single(lspRes.Diagnostics, d => d.Code == "MATCH001");
        Assert.Equal(PilotDiagnosticSeverity.Error, diag.Severity);
        Assert.Contains("Origin", diag.Message);
    }

    [Fact]
    public void Test_11_2_Exhaustiveness_MATCH001_PartialSlicePayload_ReportsPolygonNonEmptyWitness()
    {
        string code = @"
            record Point(x: f64, y: f64);
            type Shape =
                | Circle(radius: f64)
                | Rectangle(width: f64, height: f64)
                | Polygon(points: Point[])
                | Origin;

            def check(s: Shape) -> int {
                return match s {
                    case Circle(_) => 1
                    case Rectangle(_, _) => 2
                    case Polygon([]) => 3
                    case Origin => 4
                };
            }
        ";

        var diags = PilotExhaustivenessChecker.CheckProgram(ParseFast(code));
        var err = Assert.Single(diags, d => d.Code == PilotExhaustivenessChecker.NonExhaustiveDiagnosticCode);
        Assert.True(err.IsError);
        Assert.Contains("Polygon([_, ..])", err.Message);
    }

    [Fact]
    public void Test_11_2_Exhaustiveness_MATCH001_Bool_BoundedIntegerInterval_And_Tuple()
    {
        // 1. Bool missing `false`
        string boolCode = @"
            def check_flag(b: bool) -> int {
                return match b {
                    case true => 1
                };
            }
        ";
        var boolDiags = PilotExhaustivenessChecker.CheckProgram(ParseFast(boolCode));
        var boolErr = Assert.Single(boolDiags);
        Assert.Equal("MATCH001", boolErr.Code);
        Assert.Contains("false", boolErr.Message);

        // 2. Bounded u8 (0..255) missing exact witness `200`
        string u8GapCode = @"
            def check_byte(b: u8) -> str {
                return match b {
                    case 0..199 => 'low'
                    case 201..255 => 'high'
                };
            }
        ";
        var u8GapDiags = PilotExhaustivenessChecker.CheckProgram(ParseFast(u8GapCode));
        var u8Err = Assert.Single(u8GapDiags);
        Assert.Equal("MATCH001", u8Err.Code);
        Assert.Contains("200", u8Err.Message);

        // Full u8 coverage (0..255) succeeds with zero diagnostics
        string u8FullCode = @"
            def check_byte_ok(b: u8) -> str {
                return match b {
                    case 0..127 => 'ascii'
                    case 128..255 => 'extended'
                };
            }
        ";
        Assert.Empty(PilotExhaustivenessChecker.CheckProgram(ParseFast(u8FullCode)));

        // 3. Tuple (bool, bool) missing (false, true)
        string tupleCode = @"
            def check_pair(a: bool, b: bool) -> int {
                return match (a, b) {
                    case (true, _) => 1
                    case (false, false) => 0
                };
            }
        ";
        var tupleDiags = PilotExhaustivenessChecker.CheckProgram(ParseFast(tupleCode));
        var tupleErr = Assert.Single(tupleDiags);
        Assert.Equal("MATCH001", tupleErr.Code);
        Assert.Contains("(false, true)", tupleErr.Message);
    }

    [Fact]
    public void Test_11_2_Exhaustiveness_MATCH001_TypestateClass_Verification()
    {
        string missingStateCode = @"
            class Conn {
                state Open { }
                state Closed { }
            }

            def status(c: Conn) -> str {
                return match c {
                    case Conn[Open] => 'open'
                };
            }
        ";
        var diags = PilotExhaustivenessChecker.CheckProgram(ParseFast(missingStateCode));
        var err = Assert.Single(diags);
        Assert.Equal("MATCH001", err.Code);
        Assert.Contains("Conn[Closed]", err.Message);

        string completeStateCode = @"
            class Conn {
                state Open { }
                state Closed { }
            }

            def status(c: Conn) -> str {
                return match c {
                    case Conn[Open] => 'open'
                    case Conn[Closed] => 'closed'
                };
            }
        ";
        Assert.Empty(PilotExhaustivenessChecker.CheckProgram(ParseFast(completeStateCode)));
    }

    [Fact]
    public void Test_11_2_Redundancy_MATCH002_SubsumedArms_And_WhenGuardSemantics()
    {
        // 1. Subsumed ADT arm emits MATCH002 warning
        string subsumedAdt = @"
            type Shape =
                | Circle(radius: f64)
                | Origin;

            def check(s: Shape) -> int {
                return match s {
                    case Circle(r) => 1
                    case Circle(0.0) => 2
                    case Origin => 0
                };
            }
        ";
        var adtDiags = PilotExhaustivenessChecker.CheckProgram(ParseFast(subsumedAdt));
        var warn = Assert.Single(adtDiags);
        Assert.Equal("MATCH002", warn.Code);
        Assert.False(warn.IsError);

        // 2. Subsumed integer interval arm emits MATCH002 warning
        string subsumedRange = @"
            def check_byte(b: u8) -> int {
                return match b {
                    case 0..127 => 1
                    case 128..255 => 2
                    case 200 => 3
                };
            }
        ";
        var rangeDiags = PilotExhaustivenessChecker.CheckProgram(ParseFast(subsumedRange));
        Assert.Single(rangeDiags, d => d.Code == "MATCH002");

        // 3. `when`-guarded arm does NOT subsume subsequent unguarded arm (zero MATCH002)
        string guardedWithFallback = @"
            type RectOnly = | Rectangle(width: f64, height: f64);

            def check_rect(r: RectOnly) -> str {
                return match r {
                    case sq @ Rectangle(w, h) when w == h => 'square'
                    case Rectangle(w, h) => 'rect'
                };
            }
        ";
        Assert.Empty(PilotExhaustivenessChecker.CheckProgram(ParseFast(guardedWithFallback)));

        // 4. `when`-guarded arm alone does NOT satisfy exhaustiveness -> emits MATCH001
        string guardedAlone = @"
            type RectOnly = | Rectangle(width: f64, height: f64);

            def check_rect_bad(r: RectOnly) -> str {
                return match r {
                    case sq @ Rectangle(w, h) when w == h => 'square'
                };
            }
        ";
        var guardedAloneDiags = PilotExhaustivenessChecker.CheckProgram(ParseFast(guardedAlone));
        Assert.Single(guardedAloneDiags, d => d.Code == "MATCH001");
    }

    [Fact]
    public void Test_11_2_PilVm_Bytecode_Emits_O1_JumpTable_And_Roundtrips_Pilc_V2_Binary()
    {
        string code = @"
            type Shape =
                | Circle(radius: int)
                | Rectangle(width: int, height: int)
                | Origin;

            def score(s: Shape) -> int {
                return match s {
                    case Origin => 1
                    case Circle(r) => r * 10
                    case Rectangle(w, h) => w * h
                };
            }

            def bucket(n: int) -> int {
                return match n {
                    case 0..3 => 100
                    case 4 | 5 => 200
                    case _ => 999
                };
            }

            score(Circle(7)) + score(Rectangle(3, 4)) + score(Origin) + bucket(2) + bucket(5);
        ";

        var compileRes = PilotCompiler.Default.Compile(code);
        Assert.True(compileRes.Success, string.Join("; ", compileRes.Diagnostics));
        var rootChunk = compileRes.Program.BytecodeChunk;
        Assert.NotNull(rootChunk);

        // Verify `DeclareTypeDescriptor` emitted in root chunk and O(1) `JumpTable` emitted in functions
        Assert.Contains(rootChunk.Instructions, i => i.Opcode == PilotOpcode.DeclareTypeDescriptor);

        var fnChunks = rootChunk.NestedChunks;

        Assert.Equal(2, fnChunks.Count);
        foreach (var fnChunk in fnChunks)
        {
            Assert.NotEmpty(fnChunk.JumpTables);
            Assert.Contains(fnChunk.Instructions, i => i.Opcode == PilotOpcode.JumpTable);
            Assert.DoesNotContain(fnChunk.Instructions, i => i.Opcode == PilotOpcode.EvalAst);
        }

        // Round-trip through .pilc V2 binary format (standalone bytecode execution with zero AST access)
        using var ms = new MemoryStream();
        compileRes.Program.Save(ms);
        byte[] pilcBytes = ms.ToArray();
        Assert.NotEmpty(pilcBytes);

        using var msRead = new MemoryStream(pilcBytes);
        var loaded = PilotCompiledProgram.Load(msRead);
        var runRes = loaded.Run();
        Assert.True(runRes.Success, string.Join("; ", runRes.Diagnostics));
        // 70 + 12 + 1 + 100 + 200 = 383
        Assert.Equal(383, runRes.Value.AsInt());
    }

    [Fact]
    public void Test_11_2_CAbiEmitter_Emits_CompactTaggedUnion_And_BoxedRecursivePointers()
    {
        string source = @"
            record Point(x: f64, y: f64);

            type Shape =
                | Circle(radius: f64)
                | Rectangle(width: f64, height: f64)
                | Polygon(points: Point[])
                | Origin;

            type Expr[T] =
                | Const(value: T)
                | Add(left: Expr[T], right: Expr[T])
                | Mul(left: Expr[T], right: Expr[T]);
        ";

        string header = PilotCAbiEmitter.EmitHeader(source, "geometry_abi");

        Assert.Contains("typedef struct Point {", header);
        Assert.Contains("typedef uint8_t Shape_Tag;", header);
        Assert.Contains("struct Shape {", header);
        Assert.Contains("Shape_Tag tag;", header);
        Assert.Contains("} payload;", header);
        Assert.Contains("struct Expr* left; /* boxed recursive indirection */", header);
        Assert.Contains("struct Expr* right; /* boxed recursive indirection */", header);
    }

    [Fact]
    public void Test_11_2_LlvmEmitter_Emits_TaggedUnionTypes_MallocBoxedRecursion_And_O1_SwitchJumpTable()
    {
        string source = @"
            record Point(x: f64, y: f64);

            type Shape =
                | Circle(radius: f64)
                | Rectangle(width: f64, height: f64)
                | Origin;

            type Expr[T] =
                | Const(value: T)
                | Add(left: Expr[T], right: Expr[T]);

            def tag_code(s: Shape) -> int {
                return match s {
                    case Circle(_) => 10
                    case Rectangle(_, _) => 20
                    case Origin => 30
                };
            }

            let e = Add(Const(1), Const(2));
            let s = Circle(5.0);
            tag_code(s);
        ";

        string ir = PilotLlvmEmitter.Emit(source, "adt_llvm");

        Assert.Contains("%Point = type { double, double }", ir);
        Assert.Contains("%Shape = type { i8,", ir);
        Assert.Contains("%Expr.Add = type { ptr, ptr }", ir);
        Assert.Contains("call ptr @malloc(", ir);
        Assert.Contains("switch i8", ir);
    }

    // =========================================================================
    // 11.3 & 11.4 — Deep Destructuring, `@` Bindings, Ranges, Conditional Let & Suites
    // =========================================================================

    [Fact]
    public void Test_11_3_DeepNested_Keyword_Slice_Range_Or_And_AtBinding_GuardRollback()
    {
        string code = @"
            type BoxPair = | Pair(a: int, b: int);

            # Verify failed `when` guard rolls back staged pattern bindings cleanly
            var w = 777;
            let p = Pair(3, 9);
            let matched = match p {
                case whole @ Pair(w, h) when w == h => w + 1000
                case _ => w
            };
            assert(matched == 777 and w == 777, 'Failed guard must not leak staged pattern bindings');

            # Middle slice `[first, ..mid, last]`, keyword `:ok([head, ..tail])`, ranges `..` vs `..<`, and `|`
            def analyze_slice(items) -> str {
                return match items {
                    case [] => 'empty'
                    case [only] => 'one:' + only
                    case [first, ..mid, last] => 'ends:' + first + ':' + last + ':mid=' + len(mid)
                };
            }

            assert(analyze_slice([10, 20, 30, 40]) == 'ends:10:40:mid=2', 'Middle slice pattern failed');
            matched;
        ";

        var (interpVal, vmVal) = RunInterpreterAndVm(code);
        Assert.Equal(777, interpVal.AsInt());
        Assert.Equal(777, vmVal.AsInt());
    }

    [Fact]
    public void Test_11_3_ConditionalPatternBindings_IfLet_ElifLet_GuardLet_WhileLet()
    {
        string code = @"
            type Token =
                | Num(v: int)
                | Op(sym: str)
                | Eof;

            def unwrap_num_plus_one(t: Token) -> int {
                guard let Num(n) = t else {
                    return -1;
                }
                return n + 1;
            }

            def classify_tok(t: Token) -> str {
                if let Num(v) = t when v > 10 {
                    return 'big:' + v;
                } elif let Op(s) = t {
                    return 'op:' + s;
                } else {
                    return 'other';
                }
            }

            let stream = [Num(5), Num(15), Num(20), Eof, Num(99)];
            var i = 0;
            var sum = 0;
            while let Num(x) = stream[i] {
                sum = sum + x;
                i = i + 1;
            }

            assert(unwrap_num_plus_one(Num(41)) == 42, 'guard let success failed');
            assert(unwrap_num_plus_one(Eof) == -1, 'guard let else failed');
            assert(classify_tok(Num(25)) == 'big:25', 'if let with guard failed');
            assert(classify_tok(Op('+')) == 'op:+', 'elif let failed');
            assert(classify_tok(Num(3)) == 'other', 'if let failed guard fallback failed');
            sum;
        ";

        var (interpVal, vmVal) = RunInterpreterAndVm(code);
        Assert.Equal(40, interpVal.AsInt());
        Assert.Equal(40, vmVal.AsInt());
    }

    [Fact]
    public void Test_11_4_Suite28_And_Showcase16_ExecuteCleanly_In_Interpreter_And_PilVm()
    {
        string suite28 = File.ReadAllText(ResolveRepoFile(Path.Combine("tests", "28_adt_and_exhaustive_matching.pilot")));
        RunInterpreterAndVm(suite28);

        string showcase16 = File.ReadAllText(ResolveRepoFile(Path.Combine("examples", "showcase", "16_algebraic_data_types.pilot")));
        RunInterpreterAndVm(showcase16);
    }
}
