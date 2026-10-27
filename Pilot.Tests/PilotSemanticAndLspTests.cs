using System.Threading.Tasks;
using System.Linq;
using System.Text.Json.Nodes;
using Pilot.LanguageServer.Lsp;
using Pilot.LanguageServer.Semantics;
using Xunit;

namespace Pilot.Tests;

public class PilotSemanticAndLspTests
{
    [Fact]
    public void Test_Immutable_Let_And_Const_Reassignment_Caught_As_PILOT101()
    {
        string code = @"
def mutate_test() -> void {
    let fixed_val: int = 10
    const MAX_VAL: int = 100
    var counter: int = 0

    counter += 1
    fixed_val = 20
    MAX_VAL = 200
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var errors = result.Diagnostics.Where(d => d.Code == "PILOT101").ToList();
        Assert.Equal(2, errors.Count);
        Assert.Contains(errors, e => e.Message.Contains("fixed_val"));
        Assert.Contains(errors, e => e.Message.Contains("MAX_VAL"));
    }

    [Fact]
    public void Test_Type_Mismatch_And_Null_Safety_Caught_As_PILOT102()
    {
        string code = @"
let bad_int: int = ""not an int""
let bad_null: str = null
let good_opt: str? = null
let good_union: int | str = ""valid""
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var errors = result.Diagnostics.Where(d => d.Code == "PILOT102").ToList();
        Assert.Equal(2, errors.Count);
        Assert.Contains(errors, e => e.Message.Contains("bad_int"));
        Assert.Contains(errors, e => e.Message.Contains("bad_null"));
    }

    [Fact]
    public void Test_Dimensional_Units_Of_Measure_Type_Checking()
    {
        string code = @"
unit meter
unit second
unit speed = meter / second

let d: float<meter> = 100.0<meter>
let t: float<second> = 5.0<second>
let v_ok: float<speed> = d / t
let v_bad: float<meter> = 5.0<second>
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var errors = result.Diagnostics.Where(d => d.Code == "PILOT102").ToList();
        Assert.Single(errors);
        Assert.Contains("v_bad", errors[0].Message);
    }

    [Fact]
    public void Test_Guard_Missing_Early_Exit_Caught_As_PILOT103()
    {
        string code = @"
def check_input(x: int) -> int {
    guard x > 0 else {
        print(""Failed precondition without exiting!"")
    }
    return x
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var errors = result.Diagnostics.Where(d => d.Code == "PILOT103").ToList();
        Assert.Single(errors);
    }

    [Fact]
    public void Test_Hard_RealTime_And_NoAlloc_Unbounded_Loop_Caught_As_PILOT104()
    {
        string code = @"
@hard_realtime
def rt_loop() -> void {
    noalloc {
        for i in 0..10 bounded 10 {
            pass
        }
        while True {
            pass
        }
    }
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var errors = result.Diagnostics.Where(d => d.Code == "PILOT104").ToList();
        Assert.Single(errors);
        Assert.Contains("Unbounded 'while' loop", errors[0].Message);
    }

    [Fact]
    public void Test_Await_Outside_Async_Function_Caught_As_PILOT106()
    {
        string code = @"
def sync_fn(client: Client) -> str {
    let res = await client.fetch()
    return res
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var errors = result.Diagnostics.Where(d => d.Code == "PILOT106").ToList();
        Assert.Single(errors);
    }

    [Fact]
    public void Test_Rollback_To_Undeclared_Savepoint_Caught_As_PILOT107()
    {
        string code = @"
def tx_work() -> void {
    transaction(db) {
        savepoint step_one
        rollback to savepoint step_one
        rollback to savepoint missing_sp
    }
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        var errors = result.Diagnostics.Where(d => d.Code == "PILOT107").ToList();
        Assert.Single(errors);
        Assert.Contains("missing_sp", errors[0].Message);
    }

    [Fact]
    public void Test_Lsp_Server_Initialize_DidOpen_Hover_Definition_Symbols_And_SemanticTokens()
    {
        var server = new PilotLspServer();

        // 1. initialize
        var initResp = server.ProcessMessage(@"{""jsonrpc"":""2.0"",""id"":1,""method"":""initialize"",""params"":{}}");
        Assert.Single(initResp);
        var initJson = JsonNode.Parse(initResp[0])!;
        Assert.NotNull(initJson["result"]?["capabilities"]?["semanticTokensProvider"]);

        // 2. textDocument/didOpen
        string docUri = "file:///test/sample.pilot";
        string sampleCode = "struct Sensor {\n    let id: int\n}\ndef read_sensor(s: Sensor) -> int {\n    return s.id\n}\n";
        var didOpenMsg = new JsonObject
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
        }.ToJsonString();

        var diagNotifications = server.ProcessMessage(didOpenMsg);
        Assert.Single(diagNotifications);
        var diagJson = JsonNode.Parse(diagNotifications[0])!;
        Assert.Equal("textDocument/publishDiagnostics", diagJson["method"]?.GetValue<string>());
        Assert.Empty(diagJson["params"]?["diagnostics"]?.AsArray()!);

        // 3. textDocument/semanticTokens/full
        var semResp = server.ProcessMessage($@"{{""jsonrpc"":""2.0"",""id"":2,""method"":""textDocument/semanticTokens/full"",""params"":{{""textDocument"":{{""uri"":""{docUri}""}}}}}}");
        var semData = JsonNode.Parse(semResp[0])!["result"]?["data"]?.AsArray();
        Assert.NotNull(semData);
        Assert.True(semData!.Count >= 20);

        // 4. textDocument/hover on 'Sensor' parameter type (line 3, col 20)
        var hoverResp = server.ProcessMessage($@"{{""jsonrpc"":""2.0"",""id"":3,""method"":""textDocument/hover"",""params"":{{""textDocument"":{{""uri"":""{docUri}""}},""position"":{{""line"":3,""character"":20}}}}}}");
        var hoverVal = JsonNode.Parse(hoverResp[0])!["result"]?["contents"]?["value"]?.GetValue<string>();
        Assert.NotNull(hoverVal);
        Assert.Contains("struct Sensor", hoverVal);

        // 5. textDocument/definition on 'Sensor' parameter type (line 3, col 20) -> jumps to line 0
        var defResp = server.ProcessMessage($@"{{""jsonrpc"":""2.0"",""id"":4,""method"":""textDocument/definition"",""params"":{{""textDocument"":{{""uri"":""{docUri}""}},""position"":{{""line"":3,""character"":20}}}}}}");
        int defLine = JsonNode.Parse(defResp[0])!["result"]?["range"]?["start"]?["line"]?.GetValue<int>() ?? -1;
        Assert.Equal(0, defLine);

        // 6. textDocument/documentSymbol
        var symResp = server.ProcessMessage($@"{{""jsonrpc"":""2.0"",""id"":5,""method"":""textDocument/documentSymbol"",""params"":{{""textDocument"":{{""uri"":""{docUri}""}}}}}}");
        var docSymbols = JsonNode.Parse(symResp[0])!["result"]?.AsArray();
        Assert.NotNull(docSymbols);
        Assert.Equal(2, docSymbols!.Count);
    }

    [Fact]
    public void Test_Destructuring_Unpacking_Declares_Symbols_In_Scope()
    {
        string code = @"
def test_unpack() -> void {
    let (first_val, second_val) = (10, 20)
    let [head, *tail] = [1, 2, 3]
    let {user_id, user_name} = get_user()

    var sum = first_val + second_val
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.Empty(result.Diagnostics);

        var funcScope = result.RootScope.Children.First();
        Assert.NotNull(funcScope.Resolve("first_val"));
        Assert.NotNull(funcScope.Resolve("second_val"));
        Assert.NotNull(funcScope.Resolve("head"));
        Assert.NotNull(funcScope.Resolve("user_id"));
        Assert.NotNull(funcScope.Resolve("user_name"));
    }

    [Fact]
    public void Test_Modern_Closure_Scope_And_Captures()
    {
        string code = @"
def test_closure() -> void {
    let captured_x = 42
    let add_closure = [captured_x](arg: int) => arg + captured_x;
    let block_closure = [](msg: str) => {
        print(msg);
        return True;
    };
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.Empty(result.Diagnostics);

        var funcScope = result.RootScope.Children.First();
        Assert.NotNull(funcScope.Resolve("captured_x"));
        Assert.NotNull(funcScope.Resolve("add_closure"));
        Assert.NotNull(funcScope.Resolve("block_closure"));
    }

    [Fact]
    public void Test_Keywords_Ref_Dynamic_Symbol_And_LabeledStmt_Semantics()
    {
        string code = @"
public def test_advanced() -> void {
    dynamic ext_service = get_service()
    ref local_ref = ext_service
    let sym: symbol = :active

    loop_label: for i in 0..10 {
        if i > 5 {
            break loop_label;
        }
    }

    println!(""Execution completed."");
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.Empty(result.Diagnostics);

        var funcScope = result.RootScope.Children.First();
        Assert.NotNull(funcScope.Resolve("ext_service"));
        Assert.NotNull(funcScope.Resolve("local_ref"));
        Assert.NotNull(funcScope.Resolve("sym"));
    }

    [Fact]
    public void Test_Cascade_Operator_Declares_Intermediate_Assigned_Variables_In_Scope()
    {
        string code = @"
class Worker {
    def step1(self) -> void { }
    def step2(self, n: int) -> int { return n * 2; }
    def step3(self, v: int) -> str { return ""done""; }
}

def run_cascade() -> void {
    let w = Worker()
    w..step1()
    ..let res = step2(5)
    ..step3(res) -> final_msg

    let check_res = res
    let check_msg = final_msg
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.Empty(result.Diagnostics);

        var runScope = result.RootScope.Children.First(c => c.Name == "def:run_cascade");
        Assert.NotNull(runScope.Resolve("res"));
        Assert.NotNull(runScope.Resolve("final_msg"));
    }

    [Fact]
    public void Test_Nameof_And_Static_Modifiers_Semantics()
    {
        string code = @"
public static class MathHelper {
    public static def add(params vals: List[int]) -> int {
        return 0;
    }
}

def test_nameof() -> void {
    let local_var = 100
    let str_name: str = nameof(local_var)
    let class_name: str = nameof(MathHelper.add)
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.Empty(result.Diagnostics);

        var classSym = result.RootScope.Resolve("MathHelper");
        Assert.NotNull(classSym);
        var addMethod = classSym!.Members["add"];
        Assert.NotNull(addMethod);
        Assert.True(addMethod.IsStatic);
    }

    [Fact]
    public void Test_Pattern_Matching_And_Collection_Expressions_Semantics()
    {
        string code = @"
def test_matching_semantics(data: any, items: List[int]) -> void {
    let row0 = [1, 2, 3]
    let row1 = [4, 5, 6]
    let combined = [..row0, ..row1]

    if data is > 0 and <= 100 {
        pass
    }

    match items {
        case [var head, ..var tail] when head > 0 => {
            let h = head
            let t = tail
        }
        case [] => {
            let empty = True
        }
        case _ => {
            let other = True
        }
    }
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.Empty(result.Diagnostics);

        var funcScope = result.RootScope.Children.First(c => c.Name == "def:test_matching_semantics");
        Assert.NotNull(funcScope.Resolve("row0"));
        Assert.NotNull(funcScope.Resolve("row1"));
        Assert.NotNull(funcScope.Resolve("combined"));

        // Match arm child scope should contain the bound pattern variables 'head' and 'tail'
        var armScope = funcScope.Children.FirstOrDefault(c => c.Name.StartsWith("match_arm_"));
        Assert.NotNull(armScope);
        Assert.NotNull(armScope!.Resolve("head"));
        Assert.NotNull(armScope!.Resolve("tail"));
    }

    [Fact]
    public void Test_BuiltIn_Types_And_Matrix_Semantics()
    {
        string code = @"
def test_builtins() -> void {
    let c: char = 'X'
    let u: uint = 42
    let f16_val: f16 = 1.0
    let d: double = 3.14159
    let dec: decimal = 100.0
    let comp: complex = 2.0
    let un: undefined = undefined
    let mat: matrix3x3<u8> = get_mat()
    let items: list<int> = [1, 2, 3]
    let lookup: map<str, int> = {""pilot"": 1}
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.Empty(result.Diagnostics);

        var funcScope = result.RootScope.Children.First(c => c.Name == "def:test_builtins");
        Assert.NotNull(funcScope.Resolve("c"));
        Assert.NotNull(funcScope.Resolve("u"));
        Assert.NotNull(funcScope.Resolve("f16_val"));
        Assert.NotNull(funcScope.Resolve("d"));
        Assert.NotNull(funcScope.Resolve("dec"));
        Assert.NotNull(funcScope.Resolve("comp"));
        Assert.NotNull(funcScope.Resolve("un"));
        Assert.NotNull(funcScope.Resolve("mat"));
        Assert.NotNull(funcScope.Resolve("items"));
        Assert.NotNull(funcScope.Resolve("lookup"));

        Assert.Equal("matrix3x3<u8>", funcScope.Resolve("mat")!.TypeName);
    }

    [Fact]
    public void Test_Preprocessing_In_Semantic_Analysis()
    {
        string code = @"
#define LINUX
#if LINUX
let os_name = ""Linux""
#else
let os_name = ""Windows""
#endif
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.Empty(result.Diagnostics);

        var sym = result.RootScope.Resolve("os_name");
        Assert.NotNull(sym);
    }

    [Fact]
    public void Test_Error_And_Warning_In_Semantic_Analysis()
    {
        string code = @"
#warning ""Check security context""
#error ""Fatal environment check failed""
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.Contains(result.Diagnostics, d => d.Severity == PilotDiagnosticSeverity.Warning && d.Message.Contains("Check security context"));
        Assert.Contains(result.Diagnostics, d => d.Severity == PilotDiagnosticSeverity.Error && d.Message.Contains("Fatal environment check failed"));
    }

    [Fact]
    public void Test_Polyglot_Foreign_Imports_Semantic_Resolution()
    {
        string code = @"
import python:numpy as np
from python:math import sin, cos, pi as PI
import csharp:System.Text.Json as Json
from csharp:System.IO import File, Path
import rust:""libpilot_core"" as core_rs
from rust:""libcrypto"" import sha256_hash, verify_sig as verify_signature
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.Empty(result.Diagnostics);

        // Python imports
        var npSym = result.RootScope.Resolve("np");
        Assert.NotNull(npSym);
        Assert.Equal(PilotSymbolKind.Module, npSym.Kind);
        Assert.Equal("Python Interop", npSym.Domain);
        Assert.Contains("import python:numpy as np", npSym.Signature);
        Assert.Contains("Python Interop", npSym.FormatHoverMarkdown());

        var sinSym = result.RootScope.Resolve("sin");
        Assert.NotNull(sinSym);
        Assert.Equal(PilotSymbolKind.Function, sinSym.Kind);
        Assert.Equal("Python Interop", sinSym.Domain);

        var piSym = result.RootScope.Resolve("PI");
        Assert.NotNull(piSym);
        Assert.Equal("Python Interop", piSym.Domain);

        // C# imports
        var jsonSym = result.RootScope.Resolve("Json");
        Assert.NotNull(jsonSym);
        Assert.Equal(PilotSymbolKind.Module, jsonSym.Kind);
        Assert.Equal("C# / .NET Interop", jsonSym.Domain);

        var fileSym = result.RootScope.Resolve("File");
        Assert.NotNull(fileSym);
        Assert.Equal(PilotSymbolKind.Class, fileSym.Kind);
        Assert.Equal("C# / .NET Interop", fileSym.Domain);
        Assert.Equal("System.IO.File", fileSym.TypeName);

        // Rust imports
        var coreSym = result.RootScope.Resolve("core_rs");
        Assert.NotNull(coreSym);
        Assert.Equal(PilotSymbolKind.Module, coreSym.Kind);
        Assert.Equal("Rust FFI", coreSym.Domain);

        var hashSym = result.RootScope.Resolve("sha256_hash");
        Assert.NotNull(hashSym);
        Assert.Equal(PilotSymbolKind.Function, hashSym.Kind);
        Assert.Equal("Rust FFI", hashSym.Domain);

        var verifySym = result.RootScope.Resolve("verify_signature");
        Assert.NotNull(verifySym);
        Assert.Equal(PilotSymbolKind.Function, verifySym.Kind);
        Assert.Equal("Rust FFI", verifySym.Domain);
    }

    [Fact]
    public void Test_Extern_FFI_Blocks_Semantic_Resolution()
    {
        string code = @"
extern ""rust"" lib ""libphysics"" {
    fn simulate_step(dt: f32) -> void;
    fn get_particle_count() -> i32;
}

extern ""csharp"" assembly ""System.Data"" {
    class SqlConnection {
        public def open() -> void;
        public def close() -> void;
    }
}

extern ""rust"" lib ""libmath"" fn fast_inv_sqrt(number: f32) -> f32;
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.Empty(result.Diagnostics);

        var simSym = result.RootScope.Resolve("simulate_step");
        Assert.NotNull(simSym);
        Assert.Equal(PilotSymbolKind.Function, simSym.Kind);
        Assert.Equal("Rust FFI", simSym.Domain);
        Assert.Single(simSym.Parameters);
        Assert.Equal("dt", simSym.Parameters[0].Name);
        Assert.Equal("f32", simSym.Parameters[0].Type);
        Assert.Contains("libphysics", simSym.Documentation);

        var countSym = result.RootScope.Resolve("get_particle_count");
        Assert.NotNull(countSym);
        Assert.Equal("i32", countSym.TypeName);

        var sqlSym = result.RootScope.Resolve("SqlConnection");
        Assert.NotNull(sqlSym);
        Assert.Equal(PilotSymbolKind.Class, sqlSym.Kind);
        Assert.Equal("C# / .NET Interop", sqlSym.Domain);
        Assert.True(sqlSym.Members.ContainsKey("open"));
        Assert.True(sqlSym.Members.ContainsKey("close"));

        var fastInvSym = result.RootScope.Resolve("fast_inv_sqrt");
        Assert.NotNull(fastInvSym);
        Assert.Equal("Rust FFI", fastInvSym.Domain);
        Assert.Equal("f32", fastInvSym.TypeName);
    }

    [Fact]
    public void Test_Inline_Foreign_Blocks_Semantic_Analysis()
    {
        string code = @"
def test_polyglot() -> void {
    let py_val = python {
        import math
        return math.sqrt(256.0)
    }

    csharp {
        let x: int = 10
    }
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.Empty(result.Diagnostics);

        var funcScope = result.RootScope.Children.First(c => c.Name == "def:test_polyglot");
        var pyValSym = funcScope.Resolve("py_val");
        Assert.NotNull(pyValSym);
    }

    [Fact]
    public void Test_Root_Object_Methods_Resolution()
    {
        string code = @"
class CustomWidget {
    var id: int
    def to_string(self) -> str { return ""widget"" }
    def hash(self) -> int { return self.id }
}

class BaseItem {
    var name: str
}

def test_root_methods() -> void {
    let w = CustomWidget()
    let s_custom: str = w.to_string()
    let h_custom: int = w.hash()

    let item = BaseItem()
    let s_default: str = item.to_string()
    let s_cs: str = item.ToString()
    let h_default: int = item.hash()
    let h_cs: int = item.GetHashCode()
    let eq: bool = item.equals(item)
    let eq_cs: bool = item.Equals(item)
    let cloned: any = item.clone()
    let cloned_cs: any = item.Clone()
    let t_name: str = item.type_of()
    let t_cs: any = item.GetType()
    let d: tuple = item.deconstruct()
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.Empty(result.Diagnostics);

        // Verify root 'object' symbol in RootScope
        var objSym = result.RootScope.Resolve("object");
        Assert.NotNull(objSym);
        Assert.Equal(PilotSymbolKind.Class, objSym.Kind);
        Assert.True(objSym.Members.ContainsKey("to_string"));
        Assert.True(objSym.Members.ContainsKey("ToString"));
        Assert.True(objSym.Members.ContainsKey("hash"));
        Assert.True(objSym.Members.ContainsKey("GetHashCode"));
        Assert.True(objSym.Members.ContainsKey("equals"));
        Assert.True(objSym.Members.ContainsKey("Equals"));
        Assert.True(objSym.Members.ContainsKey("clone"));
        Assert.True(objSym.Members.ContainsKey("type_of"));
        Assert.True(objSym.Members.ContainsKey("deconstruct"));

        // Verify custom override on CustomWidget
        var widgetSym = result.RootScope.Resolve("CustomWidget");
        Assert.NotNull(widgetSym);
        Assert.True(widgetSym.Members.ContainsKey("to_string"));
        Assert.True(widgetSym.Members.ContainsKey("hash"));
    }

    [Fact]
    public void Test_Lisp_Homoiconic_Metaprogramming_Semantics()
    {
        string code = @"
defmacro make_adder(x: AstNode) -> AstNode {
    let tmp = gensym(""param"")
    return quasiquote {
        def adder($tmp: int) -> int {
            return $tmp + $x
        }
    }
}

def test_metaprog() -> void {
    let quoted: AstNode = quote { 100 * 2 }
    let templated: AstNode = quasiquote { print($quoted) }

    let sym_a: AstNode = symbol(""a"")
    let sym_b: AstNode = symbol(""b"")
    let pair: AstNode = cons(sym_a, sym_b)

    let head: any = car(pair)
    let tail: any = cdr(pair)
    let is_at: bool = atom(sym_a)

    let eval_res: any = eval(quoted)
    let expanded: AstNode = macroexpand(templated)
    let single_expanded: AstNode = macroexpand_1(templated)
}
";
        var result = PilotSemanticAnalyzer.Analyze(code);
        Assert.Empty(result.Diagnostics);

        // Verify defmacro registration
        var macroSym = result.RootScope.Resolve("make_adder");
        Assert.NotNull(macroSym);
        Assert.Equal(PilotSymbolKind.Macro, macroSym.Kind);
        Assert.Equal("Metaprogramming / Lisp", macroSym.Domain);

        // Verify AstNode class and its homoiconic members
        var astType = result.RootScope.Resolve("AstNode");
        Assert.NotNull(astType);
        Assert.True(astType.Members.ContainsKey("car"));
        Assert.True(astType.Members.ContainsKey("cdr"));
        Assert.True(astType.Members.ContainsKey("head"));
        Assert.True(astType.Members.ContainsKey("tail"));
        Assert.True(astType.Members.ContainsKey("cons"));
        Assert.True(astType.Members.ContainsKey("eval"));
        Assert.True(astType.Members.ContainsKey("to_source"));
        Assert.True(astType.Members.ContainsKey("is_atom"));
        Assert.True(astType.Members.ContainsKey("is_list"));

        // Verify Lisp built-in functions
        Assert.NotNull(result.RootScope.Resolve("gensym"));
        Assert.NotNull(result.RootScope.Resolve("macroexpand"));
        Assert.NotNull(result.RootScope.Resolve("macroexpand_1"));
        Assert.NotNull(result.RootScope.Resolve("eval"));
        Assert.NotNull(result.RootScope.Resolve("cons"));
        Assert.NotNull(result.RootScope.Resolve("car"));
        Assert.NotNull(result.RootScope.Resolve("cdr"));
        Assert.NotNull(result.RootScope.Resolve("atom"));
        Assert.NotNull(result.RootScope.Resolve("symbol"));
    }

    [Fact]
    public async Task Test_Lsp_Server_RunAsync_Processes_Messages_And_Exits()
    {
        var server = new PilotLspServer();

        string initMsg = LspMessageHelper.BuildLspMessage("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}");
        string shutdownMsg = LspMessageHelper.BuildLspMessage("{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"shutdown\",\"params\":{}}");
        string exitMsg = LspMessageHelper.BuildLspMessage("{\"jsonrpc\":\"2.0\",\"method\":\"exit\",\"params\":{}}");

        string fullInput = initMsg + shutdownMsg + exitMsg;

        using var inputStream = new System.IO.MemoryStream(System.Text.Encoding.UTF8.GetBytes(fullInput));
        using var outputStream = new System.IO.MemoryStream();

        await server.RunAsync(inputStream, outputStream);

        string outputStr = System.Text.Encoding.UTF8.GetString(outputStream.ToArray());


        Assert.Contains("\"id\":1", outputStr);
        Assert.Contains("\"id\":2", outputStr);
    }

    [Fact]
    public async Task Test_Lsp_Server_RunAsync_Respects_CancellationToken()
    {
        var server = new PilotLspServer();

        using var inputStream = new System.IO.MemoryStream();
        using var outputStream = new System.IO.MemoryStream();

        using var cts = new System.Threading.CancellationTokenSource();
        cts.Cancel();

        await server.RunAsync(inputStream, outputStream, cts.Token);

        Assert.Equal(0, outputStream.Length);
    }
}
