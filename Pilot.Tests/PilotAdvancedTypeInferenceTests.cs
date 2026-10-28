using System.Linq;
using System.Text.Json.Nodes;
using Pilot.Analysis;
using Pilot.Analysis.TypeSystem;
using Pilot.LanguageServer.Lsp;
using Pilot.LanguageServer.Semantics;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Canonical acceptance suite for Milestone 16:
/// - 16.1: Bidirectional Type Synthesis, Top-Down Lambda/Literal Checking &amp; Tarjan SCC Fixed-Point Unification
/// - 16.2: Dominator-Based Control-Flow Type Narrowing &amp; Typestate Transitions
/// - 16.3: Structural Row Polymorphism &amp; Compile-Time Call-Site Monomorphization
/// - 16.4: Constraint-Provenance Diagnostics (TYPE001–TYPE025) &amp; LSP Inlay Hints
/// </summary>
public class PilotAdvancedTypeInferenceTests
{
    [Fact]
    public void Test_16_1_Bidirectional_Lambda_Chain_And_Empty_Literals()
    {
        string code = @"
record User(Name: str, Age: int)

let empty_ids: List<int> = []
let empty_lookup: Map<str, float> = {}

let users = [User(""Alice"", 25), User(""Bob"", 17), User(""Carol"", 30)]
let adult_names = users.Filter(u => u.Age >= 21).Map(u => u.Name)
let total_age = users.Reduce(0, (acc, u) => {
    let next = acc + u.Age
    return next
})
";
        var result = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.False(result.HasErrors, string.Join(" | ", result.Diagnostics));

        Assert.Equal(PilotType.ListOf(PilotType.Int), result.InferredTypes["empty_ids"]);
        Assert.Equal(PilotType.MapOf(PilotType.String, PilotType.Float), result.InferredTypes["empty_lookup"]);
        Assert.Equal(PilotType.ListOf(new PilotType(PilotTypeKind.Primitive, "User")), result.InferredTypes["users"]);
        Assert.Equal(PilotType.ListOf(PilotType.String), result.InferredTypes["adult_names"]);
        Assert.Equal(PilotType.Int, result.InferredTypes["total_age"]);
    }

    [Fact]
    public void Test_16_1_MethodGroup_Overload_Resolution_And_Tarjan_SCC_Mutual_Recursion()
    {
        string code = @"
type ParseErr = BadDigit | EmptyInput

// Overloaded method group
def transform(items: List<int>, f: (int) -> int) -> List<int> {
    return items.Map(f)
}

def transform(text: str, f: (str) -> str) -> str {
    return f(text)
}

let doubled = transform([1, 2, 3], x => x * 2)
let shouted = transform(""pilot"", s => s + ""!"")

// Mutually recursive functions without explicit return types (solved via Tarjan SCC fixed-point)
def is_even(n: int) => true if n == 0 else is_odd(n - 1)
def is_odd(n: int) => false if n == 0 else is_even(n - 1)

let check_even = is_even(4)

// Recursive function with inferred error set (`-> int !`)
def parse_Countdown(n: int) -> int ! {
    if n < 0 {
        return fail(BadDigit)
    }
    if n == 0 {
        return 0
    }
    let sub = parse_Countdown(n - 1)?
    return sub + 1
}
";
        var result = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.False(result.HasErrors, string.Join(" | ", result.Diagnostics));

        Assert.Equal(PilotType.ListOf(PilotType.Int), result.InferredTypes["doubled"]);
        Assert.Equal(PilotType.String, result.InferredTypes["shouted"]);
        Assert.Equal(PilotType.Bool, result.InferredTypes["check_even"]);

        var isEvenFn = result.InferredTypes["is_even"];
        Assert.Equal(PilotType.Bool, isEvenFn.ReturnType);
        var isOddFn = result.InferredTypes["is_odd"];
        Assert.Equal(PilotType.Bool, isOddFn.ReturnType);

        var countdownFn = result.InferredTypes["parse_Countdown"];
        Assert.NotNull(countdownFn.ReturnType);
        Assert.True(countdownFn.ReturnType!.IsResult);
        Assert.Equal(PilotType.Int, countdownFn.ReturnType.TypeArguments[0]);
        Assert.Equal("BadDigit", countdownFn.ReturnType.TypeArguments[1].Name);
    }

    [Fact]
    public void Test_16_2_Dominator_Control_Flow_Narrowing_Guard_If_And_IsPattern()
    {
        string code = @"
type Shape = Circle(radius: float) | Rect(width: float, height: float)

def measure(opt_val: int?, s: Shape) -> float {
    guard some opt_val else {
        return 0.0
    }
    // Dominated continuation after `guard some`: opt_val is narrowed from `int?` to `int`
    let narrowed_int = opt_val + 10

    if s is Circle(r) {
        let area = 3.14 * r * r
        return area
    } else {
        // Negative branch narrowing: `Shape` minus `Circle` narrows `s` to `Rect`
        let fallback_state = s
        return 1.0
    }
}
";
        var result = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.False(result.HasErrors, string.Join(" | ", result.Diagnostics));
        Assert.NotNull(result.TypeEngine);

        // Verify `opt_val` was narrowed from `int?` to `int` after `guard some`
        Assert.Equal(PilotType.Int, result.InferredTypes["narrowed_int"]);

        // Verify `s` was narrowed to `Circle` in the `then` branch (line 11) and `Rect` in the `else` branch (line 16)
        var sInThen = result.TypeEngine!.GetFlowTypeAtLine(11, "s");
        Assert.NotNull(sInThen);
        Assert.Equal("Circle", sInThen!.Name);

        var rInThen = result.TypeEngine.GetFlowTypeAtLine(11, "r");
        Assert.NotNull(rInThen);
        Assert.Equal(PilotType.Float, rInThen);

        var sInElse = result.TypeEngine.GetFlowTypeAtLine(16, "s");
        Assert.NotNull(sInElse);
        Assert.Equal("Rect", sInElse!.Name);
    }

    [Fact]
    public void Test_16_2_Typestate_Transitions_Across_Control_Flow()
    {
        string validTypestateCode = @"
class Conn {
    state Closed {
        def open(self) {
            self.transition_to(""Open"")
            return self
        }
    }
    state Open {
        def query(self, sql: str) -> str {
            return sql
        }
        def close(self) {
            self.transition_to(""Closed"")
            return self
        }
    }
}

let c = Conn()
c.open()
let rows = c.query(""SELECT 1"")
c.close()
";
        var validRes = PilotAnalysisPipeline.AnalyzeSource(validTypestateCode);
        Assert.DoesNotContain(validRes.Diagnostics, d => d.Code == "TYPE012");
        Assert.Equal(PilotType.String, validRes.InferredTypes["rows"]);

        string invalidTypestateCode = @"
class Conn {
    state Closed {
        def open(self) {
            self.transition_to(""Open"")
            return self
        }
    }
    state Open {
        def query(self, sql: str) -> str {
            return sql
        }
    }
}

let c = Conn()
let illegal = c.query(""SELECT * FROM users"")
";
        var invalidRes = PilotAnalysisPipeline.AnalyzeSource(invalidTypestateCode);
        Assert.True(invalidRes.HasErrors);
        Assert.Contains(invalidRes.Diagnostics, d => d.Code == "TYPE012" && d.Message.Contains("Conn[Open]") && d.Message.Contains("Conn[Closed]"));
    }

    [Fact]
    public void Test_16_3_Structural_Row_Polymorphism_And_CompileTime_Monomorphization()
    {
        string code = @"
record Rect(width: float, height: float, label: str)
struct SwappedBox {
    let tag: str = ""box""
    let height: int = 4
    let width: int = 3
}

def area(s) => s.width * s.height

let r = Rect(10.0, 5.0, ""panel"")
let b = SwappedBox(""crate"", 4, 3)

let r_area = area(r)
let b_area = area(b)
";
        var result = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.False(result.HasErrors, string.Join(" | ", result.Diagnostics));

        // 1. Check principal row-polymorphic signature of `area`
        var areaFn = result.InferredTypes["area"];
        Assert.Equal(PilotTypeKind.Function, areaFn.Kind);
        var paramRow = areaFn.TypeArguments[0];
        Assert.Equal(PilotTypeKind.RowRecord, paramRow.Kind);
        Assert.True(paramRow.IsOpenRow);
        Assert.True(paramRow.RowFields.ContainsKey("width"));
        Assert.True(paramRow.RowFields.ContainsKey("height"));
        Assert.Contains("where", paramRow.ToString());
        Assert.Contains("Numeric", paramRow.ToString());

        // 2. Check specialized return types at each call site
        Assert.Equal(PilotType.Float, result.InferredTypes["r_area"]);
        Assert.Equal(PilotType.Int, result.InferredTypes["b_area"]);

        // 3. Check compile-time monomorphized call sites & field offsets!
        Assert.Equal(2, result.MonomorphizedCallSites.Count);

        var rectSite = result.MonomorphizedCallSites.First(m => m.MonomorphizedSymbol == "area$Rect");
        Assert.Equal(PilotType.Float, rectSite.SpecializedReturnType);
        Assert.Equal(0, rectSite.FieldOffsets["width"]);
        Assert.Equal(1, rectSite.FieldOffsets["height"]);

        var boxSite = result.MonomorphizedCallSites.First(m => m.MonomorphizedSymbol == "area$SwappedBox");
        Assert.Equal(PilotType.Int, boxSite.SpecializedReturnType);
        Assert.Equal(2, boxSite.FieldOffsets["width"]);
        Assert.Equal(1, boxSite.FieldOffsets["height"]);
    }

    [Fact]
    public void Test_16_3_Missing_Row_Field_Reports_TYPE008_With_Provenance()
    {
        string code = @"
record Circle(radius: float)
def area(s) => s.width * s.height

let c = Circle(5.0)
let bad = area(c)
";
        var result = PilotAnalysisPipeline.AnalyzeSource(code);
        Assert.True(result.HasErrors);
        Assert.Contains(result.Diagnostics, d =>
            d.Code == "TYPE008" &&
            d.Message.Contains("Circle") &&
            d.Message.Contains("width"));
    }

    [Fact]
    public void Test_16_4_Constraint_Provenance_And_Lsp_Inlay_Hints()
    {
        string mismatchCode = @"
let count: int = [""not"", ""an"", ""int""]
";
        var mismatchRes = PilotAnalysisPipeline.AnalyzeSource(mismatchCode);
        Assert.True(mismatchRes.HasErrors);
        var failedConstraint = mismatchRes.TypeConstraints.FirstOrDefault(c => !c.IsSatisfied);
        Assert.NotNull(failedConstraint);
        Assert.Equal("TYPE001", failedConstraint!.Code);
        Assert.Equal(PilotType.Int, failedConstraint.ExpectedType);
        Assert.Equal(PilotType.ListOf(PilotType.String), failedConstraint.ActualType);
        Assert.Contains("type annotation on 'count'", failedConstraint.ExpectedProvenance.OriginDescription);

        // Verify LSP textDocument/inlayHint endpoint
        var server = new PilotLspServer();
        string uri = "file:///test/inlay_hints.pilot";
        string hintSource = @"def area(s) => s.width * s.height
let total = 42 + 8
";
        server.ProcessMessage(new JsonObject
        {
            ["jsonrpc"] = "2.0",
            ["method"] = "textDocument/didOpen",
            ["params"] = new JsonObject
            {
                ["textDocument"] = new JsonObject
                {
                    ["uri"] = uri,
                    ["languageId"] = "pilot",
                    ["version"] = 1,
                    ["text"] = hintSource
                }
            }
        }.ToJsonString());

        var hintResponses = server.ProcessMessage(new JsonObject
        {
            ["jsonrpc"] = "2.0",
            ["id"] = 77,
            ["method"] = "textDocument/inlayHint",
            ["params"] = new JsonObject
            {
                ["textDocument"] = new JsonObject { ["uri"] = uri },
                ["range"] = new JsonObject
                {
                    ["start"] = new JsonObject { ["line"] = 0, ["character"] = 0 },
                    ["end"] = new JsonObject { ["line"] = 10, ["character"] = 0 }
                }
            }
        }.ToJsonString());

        Assert.Single(hintResponses);
        var hintsArray = JsonNode.Parse(hintResponses[0])!["result"]?.AsArray();
        Assert.NotNull(hintsArray);
        var labels = hintsArray!.Select(h => h?["label"]?.GetValue<string>() ?? "").ToList();

        // Should include `: int` for `let total` and row-polymorphic `: { width: ..., height: ..., ..R }` for `def area(s)`
        Assert.Contains(": int", labels);
        Assert.Contains(labels, l => l.Contains("width") && l.Contains("height") && l.Contains("..R"));
    }

    [Fact]
    public void Test_Showcase_20_And_21_Semantic_Hover_And_Inlay_Inference()
    {
        string showcase21 = @"
record Product(title: str, price: float, in_stock: bool);
record Frame2D(width: int, height: int, name: str);
record Canvas2D(width: float, height: float);

def compute_area(surface) => surface.width * surface.height;

let frame_area = compute_area(Frame2D(1920, 1080, ""HD""));
let canvas_area = compute_area(Canvas2D(12.5, 8.0));

def is_even_step(n: int) => true if n == 0 else is_odd_step(n - 1);
def is_odd_step(n: int) => false if n == 0 else is_even_step(n - 1);
";
        var sem21 = Pilot.LanguageServer.Semantics.PilotSemanticAnalyzer.Analyze(showcase21);
        Assert.Equal("int", sem21.RootScope.Symbols["frame_area"].TypeName);
        Assert.Equal("let frame_area: int", sem21.RootScope.Symbols["frame_area"].Signature);
        Assert.Equal("float", sem21.RootScope.Symbols["canvas_area"].TypeName);
        Assert.Equal("let canvas_area: float", sem21.RootScope.Symbols["canvas_area"].Signature);

        var computeAreaSym = sem21.RootScope.Symbols["compute_area"];
        Assert.NotEqual("void", computeAreaSym.TypeName);
        Assert.Contains("width:", computeAreaSym.Signature);
        Assert.Contains("height:", computeAreaSym.Signature);
        Assert.Contains("Numeric", computeAreaSym.Signature);
        Assert.Contains("compute_area(Frame2D) -> int", computeAreaSym.Documentation);
        Assert.Contains("compute_area(Canvas2D) -> float", computeAreaSym.Documentation);

        Assert.Equal("bool", sem21.RootScope.Symbols["is_even_step"].TypeName);
        Assert.Equal("def is_even_step(n: int) -> bool", sem21.RootScope.Symbols["is_even_step"].Signature);
        Assert.Equal("bool", sem21.RootScope.Symbols["is_odd_step"].TypeName);
        Assert.Equal("def is_odd_step(n: int) -> bool", sem21.RootScope.Symbols["is_odd_step"].Signature);

        string showcase20 = @"
let cubes = for (i in 1..5) {
    yield i * i * i;
};
let skipped_middle = for (i in 0..10) {
    yield i;
    if i == 2 {
        continue 8..10;
    }
};
let hierarchical_walk = for (i in 1..4) {
    yield i;
    if i == 2 {
        continue 100..101, ...;
    }
};
let ports = [80, 443, 8080, 9000];
let port_idx = for (idx in 0..<4) {
    if ports[idx] == 8080 {
        break idx;
    }
} then -1;
let fallback_idx = for (idx in 0..<4) {
    if ports[idx] == 3000 {
        break idx;
    }
} then -1;
var shm = shared_buffer(""showcase_20_shm"", 6, :f64);
let base_ptr = shm.data_ptr;
var second_view = shm.open_view();
";
        var sem20 = Pilot.LanguageServer.Semantics.PilotSemanticAnalyzer.Analyze(showcase20);
        Assert.Equal("List[int]", sem20.RootScope.Symbols["cubes"].TypeName);
        Assert.Equal("let cubes: List[int]", sem20.RootScope.Symbols["cubes"].Signature);
        Assert.Equal("List[int]", sem20.RootScope.Symbols["skipped_middle"].TypeName);
        Assert.Equal("let skipped_middle: List[int]", sem20.RootScope.Symbols["skipped_middle"].Signature);
        Assert.Equal("List[int]", sem20.RootScope.Symbols["hierarchical_walk"].TypeName);
        Assert.Equal("let hierarchical_walk: List[int]", sem20.RootScope.Symbols["hierarchical_walk"].Signature);
        Assert.Equal("int", sem20.RootScope.Symbols["port_idx"].TypeName);
        Assert.Equal("let port_idx: int", sem20.RootScope.Symbols["port_idx"].Signature);
        Assert.Equal("int", sem20.RootScope.Symbols["fallback_idx"].TypeName);
        Assert.Equal("let fallback_idx: int", sem20.RootScope.Symbols["fallback_idx"].Signature);
        Assert.Equal("SliceBuffer[f64]", sem20.RootScope.Symbols["shm"].TypeName);
        Assert.Equal("int", sem20.RootScope.Symbols["base_ptr"].TypeName);
        Assert.Equal("SliceBuffer[f64]", sem20.RootScope.Symbols["second_view"].TypeName);
    }
}
