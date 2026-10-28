using System.IO;
using System.Linq;
using Pilot.Analysis;
using Pilot.Compiler;
using Pilot.Runtime.Bytecode;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Milestone 14 Test Suite: Unified Expression Ergonomics, Class-Like Primitive Literals
/// &amp; Zero-Cost Extensions (`14.1`–`14.4`).
/// </summary>
public class PilotExpressionErgonomicsTests
{
    private static PilotExecutionContext RunInterpreter(string code)
    {
        var engine = new PilotEngine();
        var res = engine.Execute(code);
        Assert.True(res.Success, "Interpreter failed: " + string.Join("; ", res.Diagnostics));
        return res.Context!;
    }

    private static (PilotValue Result, PilotCompiledProgram Compiled) RunBytecode(string code)
    {
        var compileRes = PilotCompiler.Default.Compile(code);
        Assert.True(compileRes.Success, "PilVM compile failed: " + string.Join("; ", compileRes.Diagnostics));
        var compiled = (PilotCompiledProgram)compileRes.Program!;
        var result = compiled.Run();
        Assert.True(result.Success, "PilVM failed: " + string.Join("; ", result.Diagnostics));
        return (result.Value, compiled);
    }

    [Fact]
    public void StringLiteral_ClassLikeMethodsAndProperties_WorkInInterpreterAndBytecode()
    {
        string code = @"
            let sub = ""hello world"".Substring(0, 5);
            let parts = ""hello world"".Split("" "").Length;
            let rep = ""hello world"".Replace(""world"", ""Pilot"");
            let idx_found = ""hello world"".IndexOf(""world"");
            let idx_none = ""hello world"".IndexOf(""xyz"");
            let pad_l = ""hi"".PadLeft(5);
            let pad_r = ""hi"".PadRight(5);
            let ch_cnt = ""abc"".Chars().Length;
            let ln_cnt = ""a\nb\nc"".Lines().Length;
            let opt_ok = ""42"".ToInt();
            let opt_none = ""nope"".ToInt();
            let res_ok = ""128"".ParseInt() catch err => -1;
            let res_err = ""bad"".ParseInt() catch err => -99;
        ";

        var ctx = RunInterpreter(code);
        Assert.Equal("hello", ctx.Environment.Resolve("sub").AsString());
        Assert.Equal(2L, ctx.Environment.Resolve("parts").AsInt());
        Assert.Equal("hello Pilot", ctx.Environment.Resolve("rep").AsString());
        Assert.Equal(6L, ctx.Environment.Resolve("idx_found").AsInt());
        Assert.True(ctx.Environment.Resolve("idx_none").IsNull);
        Assert.Equal("   hi", ctx.Environment.Resolve("pad_l").AsString());
        Assert.Equal("hi   ", ctx.Environment.Resolve("pad_r").AsString());
        Assert.Equal(3L, ctx.Environment.Resolve("ch_cnt").AsInt());
        Assert.Equal(3L, ctx.Environment.Resolve("ln_cnt").AsInt());
        Assert.Equal(42L, ctx.Environment.Resolve("opt_ok").AsInt());
        Assert.True(ctx.Environment.Resolve("opt_none").IsNull);
        Assert.Equal(128L, ctx.Environment.Resolve("res_ok").AsInt());
        Assert.Equal(-99L, ctx.Environment.Resolve("res_err").AsInt());

        var (vmRes, compiled) = RunBytecode(code + "\nres_ok + opt_ok.UnwrapOr(0);");
        Assert.Equal(170L, vmRes.AsInt());
        Assert.Contains(compiled.BytecodeChunk!.Instructions, i => i.Opcode == PilotOpcode.CallPrimIntrinsic);
    }

    [Fact]
    public void IntAndFloatLiterals_ClassLikeMethodsAndProperties_WorkInInterpreterAndBytecode()
    {
        string code = @"
            let a = (-42).Abs();
            let c = 42.Clamp(0, 25);
            let hx = 255.ToHex();
            let bn = 42.ToBinary();
            let ev = 42.IsEven;
            let od = 42.IsOdd;
            let pw = 2.Pow(10);
            let pc = 255.PopCount();
            let bl = 16.BitLength();
            let rnd = 3.14159.Round(2);
            let flr = 3.7.Floor();
            let cel = 3.2.Ceil();
            let sq = 16.0.Sqrt();
            let lrp = 0.5.Lerp(0.0, 100.0);
            let ti = 3.7.ToInt();
            var acc = 0;
            5.Times(i => { acc += i; });
        ";

        var ctx = RunInterpreter(code);
        Assert.Equal(42L, ctx.Environment.Resolve("a").AsInt());
        Assert.Equal(25L, ctx.Environment.Resolve("c").AsInt());
        Assert.Equal("FF", ctx.Environment.Resolve("hx").AsString());
        Assert.Equal("101010", ctx.Environment.Resolve("bn").AsString());
        Assert.True(ctx.Environment.Resolve("ev").IsTrue);
        Assert.False(ctx.Environment.Resolve("od").IsTrue);
        Assert.Equal(1024L, ctx.Environment.Resolve("pw").AsInt());
        Assert.Equal(8L, ctx.Environment.Resolve("pc").AsInt());
        Assert.Equal(5L, ctx.Environment.Resolve("bl").AsInt());
        Assert.Equal(3.14, ctx.Environment.Resolve("rnd").AsFloat(), 4);
        Assert.Equal(3.0, ctx.Environment.Resolve("flr").AsFloat(), 4);
        Assert.Equal(4.0, ctx.Environment.Resolve("cel").AsFloat(), 4);
        Assert.Equal(4.0, ctx.Environment.Resolve("sq").AsFloat(), 4);
        Assert.Equal(50.0, ctx.Environment.Resolve("lrp").AsFloat(), 4);
        Assert.Equal(3L, ctx.Environment.Resolve("ti").AsInt());
        Assert.Equal(10L, ctx.Environment.Resolve("acc").AsInt());

        var (vmRes, _) = RunBytecode(code + "\npw + pc + bl + ti + acc;");
        Assert.Equal(1050L, vmRes.AsInt());
    }

    [Fact]
    public void BoolCharAndDimensionalLiterals_WorkInInterpreterAndBytecode()
    {
        string code = @"
            let t1 = true.Toggle();
            let sel = t1.Select(""On"", ""Off"");
            let up = 'a'.ToUpper();
            let lw = 'A'.ToLower();
            let dg = '9'.IsDigit;
            let al = 'z'.IsAlpha;
            let ws = ' '.IsWhitespace;
            let cp = 'A'.CodePoint;
            let us = 250ms.ToMicroseconds();
            let per = 50kHz.Period();
            let by = 64KB.ToBytes();
        ";

        var ctx = RunInterpreter(code);
        Assert.False(ctx.Environment.Resolve("t1").IsTrue);
        Assert.Equal("Off", ctx.Environment.Resolve("sel").AsString());
        Assert.Equal("A", ctx.Environment.Resolve("up").AsString());
        Assert.Equal("a", ctx.Environment.Resolve("lw").AsString());
        Assert.True(ctx.Environment.Resolve("dg").IsTrue);
        Assert.True(ctx.Environment.Resolve("al").IsTrue);
        Assert.True(ctx.Environment.Resolve("ws").IsTrue);
        Assert.Equal(65L, ctx.Environment.Resolve("cp").AsInt());
        Assert.Equal(250000.0, ctx.Environment.Resolve("us").AsFloat(), 3);
        Assert.Equal(0.00002, ctx.Environment.Resolve("per").AsFloat(), 8);
        Assert.Equal(65536L, ctx.Environment.Resolve("by").AsInt());

        var (vmRes, _) = RunBytecode(code + "\nby + cp;");
        Assert.Equal(65601L, vmRes.AsInt());
    }

    [Fact]
    public void ExtendBlock_ComputedPropertiesAndGenericExtensions_LowerCleanly()
    {
        string code = @"
            extend str {
                property IsBlank: bool { get => self.Trim().Length == 0 }
                def Slugify() -> str => self.Trim().ToLower().Replace("" "", ""-"");
            }

            extend[T: Numeric] T[] {
                def Mean() -> f64? => if self.Length == 0 { none } else { self.Sum().ToFloat() / self.Length };
            }

            let blank_check = ""   "".IsBlank;
            let non_blank = "" hi "".IsBlank;
            let slug = ""  Hello World  "".Slugify();
            let direct_slug = strExt_Slugify(""  Static Dispatch  "");
            let avg = [10, 20, 30, 40].Mean();
            let empty_avg = [].Mean();
        ";

        var ctx = RunInterpreter(code);
        Assert.True(ctx.Environment.Resolve("blank_check").IsTrue);
        Assert.False(ctx.Environment.Resolve("non_blank").IsTrue);
        Assert.Equal("hello-world", ctx.Environment.Resolve("slug").AsString());
        Assert.Equal("static-dispatch", ctx.Environment.Resolve("direct_slug").AsString());
        Assert.Equal(25.0, ctx.Environment.Resolve("avg").AsFloat(), 4);
        Assert.True(ctx.Environment.Resolve("empty_avg").IsNull);

        var (vmRes, _) = RunBytecode(code + "\navg ?? 0.0;");
        Assert.Equal(25.0, vmRes.AsFloat(), 4);
    }

    [Fact]
    public void DeepOptionalChaining_MemberIndexAndCall_EvaluatesReceiverOnceAndShortCircuitsToNone()
    {
        string code = @"
            record Address(City: str);
            record Customer(Address: Address?);
            record Order(Customer: Customer?);

            var eval_count = 0;
            def get_order(present: bool) -> Order? {
                eval_count += 1;
                if present { return Order(Customer(Address(""Denver""))); }
                return none;
            }

            let c1 = get_order(false)?.Customer?.Address?.City ?? ""Unknown"";
            let count_after_first = eval_count;
            let c2 = get_order(true)?.Customer?.Address?.City ?? ""Unknown"";
            let count_after_second = eval_count;

            let matrix = [[11, 22], [33, 44]];
            let no_matrix = none;
            let m_val = matrix?[1]?[0] ?? -1;
            let m_none = no_matrix?[1]?[0] ?? -1;

            let cb = x => x + 100;
            let no_cb = none;
            let cb_val = cb?(23) ?? -1;
            let cb_none = no_cb?(23) ?? -1;
        ";

        var ctx = RunInterpreter(code);
        Assert.Equal("Unknown", ctx.Environment.Resolve("c1").AsString());
        Assert.Equal(1L, ctx.Environment.Resolve("count_after_first").AsInt());
        Assert.Equal("Denver", ctx.Environment.Resolve("c2").AsString());
        Assert.Equal(2L, ctx.Environment.Resolve("count_after_second").AsInt());
        Assert.Equal(33L, ctx.Environment.Resolve("m_val").AsInt());
        Assert.Equal(-1L, ctx.Environment.Resolve("m_none").AsInt());
        Assert.Equal(123L, ctx.Environment.Resolve("cb_val").AsInt());
        Assert.Equal(-1L, ctx.Environment.Resolve("cb_none").AsInt());

        var (vmRes, compiled) = RunBytecode(code + "\nm_val + cb_val + count_after_second;");
        Assert.Equal(158L, vmRes.AsInt());
        Assert.Contains(compiled.BytecodeChunk!.Instructions, i => i.Opcode == PilotOpcode.JumpIfNullShortCircuit);
    }

    [Fact]
    public void UnifiedExpressionOrientedControlFlow_IfMatchBlockWithAndFluentPipe_WorkInAllModes()
    {
        string code = @"
            let score = 92;
            let tier = if score >= 90 { ""Gold"" } elif score >= 75 { ""Silver"" } else { ""Bronze"" };
            let items = [:a, :b, :a];
            let mapped = items.Map(match { case :a => 1, case :b => 2 });
            let x1 = 0.0;
            let y1 = 0.0;
            let x2 = 6.0;
            let y2 = 8.0;
            let norm = { let dx = x2 - x1; let dy = y2 - y1; (dx * dx + dy * dy).Sqrt() };

            def fetch_user(id: int, context db: str, context logger: str) -> str => $""{db}:{logger}:{id}"";
            let res = with context (db = ""mock_db"", logger = ""silent_log"") { fetch_user(42) };

            let slug = ""  Hello World  "" |> .Trim() |> .ToLower() |> .Replace("" "", ""-"");
        ";

        var ctx = RunInterpreter(code);
        Assert.Equal("Gold", ctx.Environment.Resolve("tier").AsString());
        Assert.Equal(new[] { 1L, 2L, 1L }, ctx.Environment.Resolve("mapped").AsList().Select(v => v.AsInt()).ToArray());
        Assert.Equal(10.0, ctx.Environment.Resolve("norm").AsFloat(), 4);
        Assert.Equal("mock_db:silent_log:42", ctx.Environment.Resolve("res").AsString());
        Assert.Equal("hello-world", ctx.Environment.Resolve("slug").AsString());

        var (vmRes, _) = RunBytecode(code + "\n$\"{tier}|{norm.ToInt()}|{res}|{slug}\";");
        Assert.Equal("Gold|10|mock_db:silent_log:42|hello-world", vmRes.AsString());
    }

    [Fact]
    public void IntegrationScript_Suite31_ExecutesCleanlyInInterpreterAndBytecode()
    {
        string path = Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "tests", "31_expression_ergonomics_and_extensions.pilot");
        string source = File.ReadAllText(path);

        var analysis = PilotAnalysisPipeline.AnalyzeSource(source);
        Assert.DoesNotContain(analysis.Diagnostics, d => d.IsError);

        RunInterpreter(source);
        RunBytecode(source);
    }
}
