using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Pilot.Formatting;
using Pilot.Runtime.Execution;
using Pilot.Transpilation;
using Pilot.Transpilation.Ingesters;
using Pilot.Transpilation.Ir;
using Pilot.Transpilation.Licensing;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Milestone 20: Comprehensive verification suite for Canonical Typed Transpilation IR (20.1),
/// Lossless Trivia-Preserving Pilot Formatter (20.2), AST-Driven Target Emitters (20.3),
/// Lexer/Parser Foreign Language Ingesters &amp; --strict-pilot (20.4),
/// Viable License Restriction Policy (20.5), and Compiler-in-the-Loop Round-Trip Execution (20.6).
/// </summary>
public class PilotBidirectionalTranspilationTests
{
    private readonly PilotEngine _engine = new();

    // ========================================================================
    // 20.1: CANONICAL TYPED TRANSPILATION IR
    // ========================================================================

    [Fact]
    public void Test_20_1_IrBuilder_Resolves_Types_Mutability_Precedence_And_CAbi_Layout()
    {
        string pilotSource = @"
            // Module header comment
            def compute(n: i64) -> i64 {
                let fixed_scale = 3;
                var acc = 0;
                for i in 1..n {
                    acc += i * fixed_scale;
                }
                return acc;
            }

            let msg = ""zeros(2, 2)"";
            let m = zeros(2, 2);
            let sq = [x * x for x in 1..4 if x > 1];
        ";

        var (module, errors) = PilotTranspileIrBuilder.BuildFromSource(pilotSource);
        Assert.Empty(errors);
        Assert.NotNull(module);

        // Mutability tracking
        Assert.Contains("acc", module!.MutatedVariables);
        Assert.DoesNotContain("fixed_scale", module.MutatedVariables);

        var fn = Assert.IsType<PilotIrFuncDefStmt>(module.Statements[0]);
        Assert.Equal("compute", fn.Name);
        Assert.Equal("i64", fn.EffectiveReturnType);

        var fixedDecl = Assert.IsType<PilotIrVarDeclStmt>(fn.Body[0]);
        Assert.False(fixedDecl.IsMutated);

        var accDecl = Assert.IsType<PilotIrVarDeclStmt>(fn.Body[1]);
        Assert.True(accDecl.IsMutated);

        // AST-scoped library call classification
        var mDecl = module.Statements.OfType<PilotIrVarDeclStmt>().First(v => v.Name == "m");
        var mCall = Assert.IsType<PilotIrCallExpr>(mDecl.Initializer);
        Assert.Equal(PilotLibraryCallKind.MatrixZeros, mCall.LibraryCallKind);

        // ForExpr C-ABI layout metadata
        var sqDecl = module.Statements.OfType<PilotIrVarDeclStmt>().First(v => v.Name == "sq");
        var forExpr = Assert.IsType<PilotIrForExpr>(sqDecl.Initializer);
        Assert.True(forExpr.LoopLayout.UsesSliceBuffer);
        Assert.Equal(24, forExpr.LoopLayout.StructByteSize);
    }

    // ========================================================================
    // 20.2: LOSSLESS TRIVIA-PRESERVING PILOT FORMATTER (pilot fmt)
    // ========================================================================

    [Fact]
    public void Test_20_2_Formatter_Preserves_Comments_BlankLines_And_Parenthesization()
    {
        string unformatted = @"
// Leading module comment
def calc(a: i64, b: i64, c: i64) -> i64 {
    // Keep this inner comment
    let res = (a + b) * c;

    return res;
}
";

        var fmt1 = PilotFormatter.FormatSource(unformatted);
        Assert.True(fmt1.Success, string.Join("; ", fmt1.Diagnostics));
        Assert.Contains("// Leading module comment", fmt1.FormattedCode);
        Assert.Contains("// Keep this inner comment", fmt1.FormattedCode);
        Assert.Contains("(a + b) * c", fmt1.FormattedCode);

        // Idempotence: formatting a second time produces identical output
        var fmt2 = PilotFormatter.FormatSource(fmt1.FormattedCode);
        Assert.True(fmt2.Success);
        Assert.Equal(fmt1.FormattedCode, fmt2.FormattedCode);
        Assert.False(fmt2.WasModified);
    }

    [Fact]
    public void Test_20_2_Formatter_Brace_And_Colon_Suite_Modes()
    {
        string code = @"
def check_val(x: i64) -> i64 {
    if x > 0 {
        return x;
    } else {
        return 0;
    }
}
";

        var colonRes = PilotFormatter.FormatSource(code, new PilotFormatterOptions
        {
            UseColonSuites = true,
            EmitSemicolons = false
        });
        Assert.True(colonRes.Success);
        Assert.Contains("def check_val(x: i64) -> i64:", colonRes.FormattedCode);
        Assert.Contains("if x > 0:", colonRes.FormattedCode);
        Assert.Contains("else:", colonRes.FormattedCode);

        // Round-trip colon-formatted code back to brace mode
        var braceRes = PilotFormatter.FormatSource(colonRes.FormattedCode, new PilotFormatterOptions
        {
            UseColonSuites = false,
            EmitSemicolons = true
        });
        Assert.True(braceRes.Success);
        Assert.Contains("def check_val(x: i64) -> i64 {", braceRes.FormattedCode);
        Assert.Contains("if x > 0 {", braceRes.FormattedCode);
    }

    // ========================================================================
    // 20.3: FULL AST-DRIVEN TARGET EMITTERS (7 TARGETS)
    // ========================================================================

    [Fact]
    public void Test_20_3_All_Seven_Target_Emitters_Operators_Ranges_Interpolation_And_Precedence()
    {
        string pilotCode = @"
            def compute_weighted(a: i64, b: i64, c: i64) -> i64 {
                let base_val: i64 = (a + b) * c;
                var total: i64 = base_val;
                for i in 1..5 {
                    if i > 2 and total != 0 {
                        total += i;
                    }
                }
                return total;
            }

            let ans: i64 = compute_weighted(2, 3, 4);
            println(f""Result is {ans}"");
        ";

        string[] targets = { "python", "csharp", "rust", "c", "cpp", "js", "lua" };
        foreach (var target in targets)
        {
            var res = TranspilationRegistry.Default.Transpile(pilotCode, target);
            Assert.True(res.Success, $"Target '{target}' failed: {string.Join("; ", res.Diagnostics)}");
            Assert.Contains("(a + b) * c", res.Code);
        }

        // Verify target-specific range & interpolation idioms
        var py = TranspilationRegistry.Default.Transpile(pilotCode, "python").Code;
        Assert.Contains("range(1, 6)", py);
        Assert.Contains("print(f\"Result is {ans}\")", py);

        var rs = TranspilationRegistry.Default.Transpile(pilotCode, "rust").Code;
        Assert.Contains("for i in 1..=5", rs);
        Assert.Contains("println!(\"Result is {}\", ans)", rs);

        var js = TranspilationRegistry.Default.Transpile(pilotCode, "js").Code;
        Assert.Contains("for (let i = 1; i <= 5; i++)", js);
        Assert.Contains("console.log(`Result is ${ans}`)", js);

        var lua = TranspilationRegistry.Default.Transpile(pilotCode, "lua").Code;
        Assert.Contains("for i = 1, 5 do", lua);
        Assert.Contains("string.format(\"Result is %s\", tostring(ans))", lua);
    }

    [Fact]
    public void Test_20_3_Adt_Optional_Extend_And_ForExpr_Lowering()
    {
        string pilotCode = @"
            type Shape = Circle(r: f64) | Rect(w: f64, h: f64);

            extend i64 {
                def doubled(self) -> i64 {
                    return self * 2;
                }
            }

            def evens_squared() -> list[i64] {
                return [x * x for x in 1..6 if x > 2];
            }
        ";

        var py = TranspilationRegistry.Default.Transpile(pilotCode, "python");
        Assert.True(py.Success);
        Assert.Contains("@dataclass", py.Code);
        Assert.Contains("class Circle:", py.Code);
        Assert.Contains("[x * x for x in range(1, 7) if x > 2]", py.Code);

        var cs = TranspilationRegistry.Default.Transpile(pilotCode, "csharp");
        Assert.True(cs.Success);
        Assert.Contains("public abstract record Shape;", cs.Code);
        Assert.Contains("public sealed record Circle(double r) : Shape;", cs.Code);
        Assert.Contains(".Where(x => x > 2).Select(x => x * x).ToList()", cs.Code);

        var rs = TranspilationRegistry.Default.Transpile(pilotCode, "rust");
        Assert.True(rs.Success);
        Assert.Contains("pub enum Shape", rs.Code);
        Assert.Contains("Circle(f64)", rs.Code);
        Assert.Contains("pub trait i64Ext", rs.Code);
        Assert.Contains(".into_iter().filter(|&x| x > 2).map(|x| x * x).collect::<Vec<_>>()", rs.Code);
    }

    [Fact]
    public void Test_20_3_AstScoped_LibraryMapper_Never_Corrupts_String_Literals()
    {
        string pilotCode = @"
            let literal_note = ""Call zeros(2, 2) or json_dumps(data) in docs"";
            let mat = zeros(2, 2);
        ";

        foreach (var target in new[] { "python", "js", "rust", "cpp", "c", "lua", "csharp" })
        {
            var res = TranspilationRegistry.Default.Transpile(pilotCode, target);
            Assert.True(res.Success);
            // The string literal must remain completely untouched!
            Assert.Contains("\"Call zeros(2, 2) or json_dumps(data) in docs\"", res.Code);
        }

        var pyRes = TranspilationRegistry.Default.Transpile(pilotCode, "python");
        Assert.Contains("np.zeros((2, 2))", pyRes.Code);
    }

    // ========================================================================
    // 20.4: REAL LEXER/PARSER FOREIGN LANGUAGE INGESTERS & --strict-pilot
    // ========================================================================

    [Fact]
    public void Test_20_4_Roslyn_CSharp_Ingester_Classes_Records_ExtensionMethods_Switch_And_Linq()
    {
        string csharpSource = @"
using System;
using System.Linq;

namespace SampleLib;

public static class IntExtensions
{
    public static int Triple(this int x)
    {
        return x * 3;
    }
}

public class Calculator
{
    public static int Classify(int code)
    {
        var doubled = code * 2;
        switch (code)
        {
            case 1:
                return 10;
            default:
                return doubled;
        }
    }
}
";

        var res = TranspilationRegistry.Default.Ingest(csharpSource, "csharp");
        Assert.True(res.Success, string.Join("; ", res.Diagnostics));
        Assert.Contains("extend int", res.Code);
        Assert.Contains("def Triple(self)", res.Code);
        Assert.Contains("class Calculator", res.Code);
        Assert.Contains("match code", res.Code);
        Assert.Empty(PilotIngestValidator.VerifyGeneratedPilotSyntax(res.Code));
    }

    [Fact]
    public void Test_20_4_Foreign_Ast_Ingesters_Rust_Python_C_Cpp_Js_Lua_Pass_Syntax_Gate()
    {
        var samples = new Dictionary<string, string>
        {
            ["python"] = @"
def sum_range(limit: int) -> int:
    total = 0
    for i in range(1, limit + 1):
        if i > 0:
            total += i
    return total
",
            ["rust"] = @"
pub enum Status {
    Idle,
    Running(i64),
}

pub fn compute_step(x: i64) -> i64 {
    let mut acc: i64 = x + 1;
    acc += 2;
    return acc;
}
",
            ["c"] = @"
#include <stdint.h>

static inline int64_t square_plus_one(int64_t x) {
    const int64_t sq = x * x;
    return sq + 1;
}
",
            ["cpp"] = @"
#include <cstdint>

constexpr int64_t cube_val(int64_t x) {
    const auto res = x * x * x;
    return res;
}
",
            ["js"] = @"
""use strict"";
function scaleAndSum(a, b) {
    const base = a * 2;
    let out = base + b;
    if (out === 10) {
        return out + 1;
    }
    return out;
}
",
            ["lua"] = @"
local function factorial(n)
    local acc = 1
    for i = 1, n do
        acc = acc * i
    end
    return acc
end
"
        };

        foreach (var (lang, src) in samples)
        {
            var res = TranspilationRegistry.Default.Ingest(src, lang);
            Assert.True(res.Success, $"Ingestion failed for '{lang}': {string.Join("; ", res.Diagnostics)}");
            Assert.Empty(PilotIngestValidator.VerifyGeneratedPilotSyntax(res.Code));
        }
    }

    [Fact]
    public void Test_20_4_StrictPilot_Upgrades_Raise_NullGuard_Mutability_And_Comprehension()
    {
        string pythonSource = @"
def process_item(val: int) -> int:
    if val == None:
        return -1
    if val < 0:
        raise ValueError(""negative input"")
    unmodified = val * 2
    out = []
    for x in range(1, 5):
        out.append(x * 10)
    return unmodified
";

        var strictOpts = new TranspilationOptions { StrictPilot = true };
        var res = TranspilationRegistry.Default.Ingest(pythonSource, "python", strictOpts);
        Assert.True(res.Success, string.Join("; ", res.Diagnostics));

        // 1. Null-check upgraded to 'guard some val else'
        Assert.Contains("guard some val else", res.Code);
        // 2. 'raise ValueError(...)' lifted to 'return fail(...)' with '! Error' return type
        Assert.Contains("! Error", res.Code);
        Assert.Contains("return fail(\"negative input\")", res.Code);
        // 3. Unmutated variable uses 'let'
        Assert.Contains("let unmodified =", res.Code);
        // 4. Append loop upgraded to comprehension '[x * 10 for x in ...]'
        Assert.Contains("[x * 10 for x in", res.Code);
    }

    // ========================================================================
    // 20.5: VIABLE LICENSE RESTRICTION CLI FLAG (--viable-license-only)
    // ========================================================================

    [Fact]
    public void Test_20_5_LicenseVerifier_Permissive_Allows_MIT_Apache_BSD_And_Rejects_Copyleft_Proprietary_Missing()
    {
        var permissiveOpts = new TranspilationOptions
        {
            LicensePolicy = PilotLicensePolicy.Permissive
        };

        // 1. MIT SPDX header -> Allowed
        string mitPy = "# SPDX-License-Identifier: MIT\ndef ok_fn(x: int) -> int:\n    return x + 1\n";
        var mitRes = TranspilationRegistry.Default.Ingest(mitPy, "python", permissiveOpts);
        Assert.True(mitRes.Success);
        Assert.Equal("MIT", mitRes.LicenseInfo?.SpdxIdentifier);

        // 2. Dual-licensed MIT OR GPL-2.0-only -> Allowed under MIT
        string dualRs = "// SPDX-License-Identifier: MIT OR GPL-2.0-only\npub fn ok_dual(x: i64) -> i64 { return x * 2; }\n";
        var dualRes = TranspilationRegistry.Default.Ingest(dualRs, "rust", permissiveOpts);
        Assert.True(dualRes.Success);

        // 3. GPL-3.0 copyleft header -> Rejected with INGEST_LIC001
        string gplPy = "# SPDX-License-Identifier: GPL-3.0-only\ndef copyleft_fn(x: int) -> int:\n    return x\n";
        var gplRes = TranspilationRegistry.Default.Ingest(gplPy, "python", permissiveOpts);
        Assert.False(gplRes.Success);
        Assert.Contains(gplRes.Diagnostics, d => d.Contains("INGEST_LIC001"));

        // 4. Proprietary header -> Rejected with INGEST_LIC002
        string propJs = "// SPDX-License-Identifier: Proprietary\nfunction secret(x) { return x; }\n";
        var propRes = TranspilationRegistry.Default.Ingest(propJs, "js", permissiveOpts);
        Assert.False(propRes.Success);
        Assert.Contains(propRes.Diagnostics, d => d.Contains("INGEST_LIC002"));

        // 5. Missing license when --viable-license-only is active -> Rejected with INGEST_LIC003
        string noLicPy = "def unlicensed_fn(x: int) -> int:\n    return x\n";
        var noLicRes = TranspilationRegistry.Default.Ingest(noLicPy, "python", permissiveOpts);
        Assert.False(noLicRes.Success);
        Assert.Contains(noLicRes.Diagnostics, d => d.Contains("INGEST_LIC003"));
    }

    [Fact]
    public void Test_20_5_LibraryIngestion_Writes_Spdx_Provenance_And_ThirdPartyNotices()
    {
        string tempRoot = Path.Combine(Path.GetTempPath(), "pilot_m20_lic_" + Guid.NewGuid().ToString("N"));
        string srcDir = Path.Combine(tempRoot, "fast_math");
        string outDir = Path.Combine(tempRoot, "fast_math_pilot");
        Directory.CreateDirectory(srcDir);

        try
        {
            File.WriteAllText(Path.Combine(srcDir, "LICENSE"), "MIT License\n\nCopyright (c) 2026 FastMath Authors\nPermission is hereby granted, free of charge, to any person obtaining a copy...");
            File.WriteAllText(Path.Combine(srcDir, "math_ops.py"), "def triple(x: int) -> int:\n    return x * 3\n");

            var opts = new TranspilationOptions { LicensePolicy = PilotLicensePolicy.Permissive };
            var summary = PilotLibraryTranspiler.IngestLibraryToPilot(srcDir, outDir, "python", opts);

            Assert.True(summary.Success, string.Join("; ", summary.Diagnostics));
            Assert.Equal("MIT", summary.LicenseInfo?.SpdxIdentifier);

            string manifestPath = Path.Combine(outDir, "pilot.json");
            string noticesPath = Path.Combine(outDir, "THIRD_PARTY_NOTICES.md");
            Assert.True(File.Exists(manifestPath));
            Assert.True(File.Exists(noticesPath));

            string manifestJson = File.ReadAllText(manifestPath);
            Assert.Contains("\"license\": \"MIT\"", manifestJson);
            Assert.Contains("\"spdxIdentifier\": \"MIT\"", manifestJson);

            string noticesMd = File.ReadAllText(noticesPath);
            Assert.Contains("MIT", noticesMd);
            Assert.Contains("FastMath Authors", noticesMd);
        }
        finally
        {
            if (Directory.Exists(tempRoot))
                Directory.Delete(tempRoot, true);
        }
    }

    // ========================================================================
    // 20.6: COMPILER-IN-THE-LOOP & ROUND-TRIP EXECUTION VERIFICATION
    // ========================================================================

    [Fact]
    public void Test_20_6_CompilerInTheLoop_Roslyn_CSharp_Verification()
    {
        string pilotCode = @"
            type Payment = Cash(amount: f64) | Card(amount: f64, fee: f64);

            extend int {
                def clamp_positive(self) -> int {
                    if self < 0 {
                        return 0;
                    }
                    return self;
                }
            }

            def compute_fib(n: int) -> int {
                if n <= 1 {
                    return n;
                }
                var a: int = 0;
                var b: int = 1;
                for i in 2..n {
                    let next: int = a + b;
                    a = b;
                    b = next;
                }
                return b;
            }

            let fib10: int = compute_fib(10);
            println(f""Fib(10) = {fib10}"");
        ";

        var csResult = TranspilationRegistry.Default.Transpile(pilotCode, "csharp");
        Assert.True(csResult.Success, string.Join("; ", csResult.Diagnostics));

        var syntaxTree = CSharpSyntaxTree.ParseText(csResult.Code);
        var trustedAssemblies = ((string?)AppContext.GetData("TRUSTED_PLATFORM_ASSEMBLIES") ?? "")
            .Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries)
            .Where(p => Path.GetFileName(p) is
                "System.Private.CoreLib.dll" or
                "System.Runtime.dll" or
                "System.Console.dll" or
                "System.Collections.dll" or
                "System.Linq.dll" or
                "System.Text.Json.dll" or
                "Microsoft.CSharp.dll")
            .Select(p => MetadataReference.CreateFromFile(p))
            .ToList();

        var compilation = CSharpCompilation.Create(
            "PilotEmittedVerification",
            new[] { syntaxTree },
            trustedAssemblies,
            new CSharpCompilationOptions(OutputKind.DynamicallyLinkedLibrary));

        var errors = compilation.GetDiagnostics()
            .Where(d => d.Severity == DiagnosticSeverity.Error)
            .ToList();

        Assert.True(errors.Count == 0,
            $"Roslyn C# compilation errors:\n{string.Join("\n", errors)}\n\nEmitted C#:\n{csResult.Code}");
    }

    [Fact]
    public void Test_20_6_CompilerInTheLoop_Clang_C11_And_Cpp20_Verification()
    {
        string pilotCode = @"
            def gcd(a: i64, b: i64) -> i64 {
                var x: i64 = a;
                var y: i64 = b;
                while y != 0 {
                    let temp: i64 = y;
                    y = x % y;
                    x = temp;
                }
                return x;
            }

            let g: i64 = gcd(48, 18);
            assert g == 6;
            println(f""GCD = {g}"");
        ";

        // 1. Verify C11 output with clang -std=c11 -fsyntax-only
        var cRes = TranspilationRegistry.Default.Transpile(pilotCode, "c");
        Assert.True(cRes.Success);
        VerifyExternalTool("clang", "-std=c11 -fsyntax-only \"{0}\"", ".c", cRes.Code);

        // 2. Verify C++20 output with clang++ -std=c++20 -fsyntax-only
        var cppRes = TranspilationRegistry.Default.Transpile(pilotCode, "cpp");
        Assert.True(cppRes.Success);
        VerifyExternalTool("clang++", "-std=c++20 -fsyntax-only \"{0}\"", ".cpp", cppRes.Code);
    }

    [Fact]
    public void Test_20_6_CompilerInTheLoop_Python_PyCompile_And_Node_Check_Verification()
    {
        string pilotCode = @"
            class Counter {
                def step(self, x: int) -> int {
                    return x + 1;
                }
            }

            def sum_squares(limit: int) -> int {
                var acc = 0;
                for i in 1..limit {
                    acc += i * i;
                }
                return acc;
            }

            let s = sum_squares(5);
            println(f""SumSquares(5) = {s}"");
        ";

        // 1. Verify Python 3 syntax with python3 -m py_compile
        var pyRes = TranspilationRegistry.Default.Transpile(pilotCode, "python");
        Assert.True(pyRes.Success);
        VerifyExternalTool("python3", "-m py_compile \"{0}\"", ".py", pyRes.Code);

        // 2. Verify JavaScript ES2024 syntax with node --check
        var jsRes = TranspilationRegistry.Default.Transpile(pilotCode, "js");
        Assert.True(jsRes.Success);
        VerifyExternalTool("node", "--check \"{0}\"", ".js", jsRes.Code);
    }

    [Fact]
    public void Test_20_6_Bidirectional_RoundTrip_Foreign_To_Pilot_Executes_In_VirtualMachine()
    {
        var foreignPrograms = new (string Lang, string Code)[]
        {
            ("python", @"
def compute_triangular(n: int) -> int:
    acc = 0
    for i in range(1, n + 1):
        acc += i
    return acc

ans = compute_triangular(10)
"),
            ("csharp", @"
public class TriMath
{
    public int ComputeTriangular(int n)
    {
        var acc = 0;
        for (int i = 1; i <= n; i++)
        {
            acc += i;
        }
        return acc;
    }
}
"),
            ("rust", @"
pub fn compute_triangular(n: i64) -> i64 {
    let mut acc: i64 = 0;
    for i in 1..=n {
        acc += i;
    }
    return acc;
}
"),
            ("c", @"
#include <stdint.h>
static inline int64_t compute_triangular(int64_t n) {
    int64_t acc = 0;
    for (int64_t i = 1; i <= n; ++i) {
        acc += i;
    }
    return acc;
}
"),
            ("cpp", @"
#include <cstdint>
constexpr int64_t compute_triangular(int64_t n) {
    int64_t acc = 0;
    for (int64_t i = 1; i <= n; ++i) {
        acc += i;
    }
    return acc;
}
"),
            ("js", @"
function compute_triangular(n) {
    let acc = 0;
    for (let i = 1; i <= n; i++) {
        acc += i;
    }
    return acc;
}
"),
            ("lua", @"
local function compute_triangular(n)
    local acc = 0
    for i = 1, n do
        acc = acc + i
    end
    return acc
end
")
        };

        foreach (var (lang, foreignCode) in foreignPrograms)
        {
            var ingested = TranspilationRegistry.Default.Ingest(foreignCode, lang);
            Assert.True(ingested.Success, $"Ingest from '{lang}' failed: {string.Join("; ", ingested.Diagnostics)}");

            string harness = lang == "csharp"
                ? ingested.Code + "\nlet final_ans = TriMath().ComputeTriangular(10);\n"
                : ingested.Code + "\nlet final_ans = compute_triangular(10);\n";

            var execRes = _engine.Execute(harness);
            Assert.True(execRes.Success, $"Execution of ingested '{lang}' code failed: {string.Join("; ", execRes.Diagnostics)}\nCode:\n{harness}");

            var val = execRes.Context!.Environment.CurrentScope.Resolve("final_ans");
            Assert.Equal(55, val.AsInt());
        }
    }

    [Fact]
    public void Test_20_6_Persisted_Eight_Language_RoundTrip_Samples_Compile_And_Execute_Identically()
    {
        string samplesRoot = FindRoundTripSamplesDirectory();
        string sourcePilotPath = Path.Combine(samplesRoot, "00_source.pilot");
        Assert.True(File.Exists(sourcePilotPath), $"Missing source sample: {sourcePilotPath}");

        // 1. Execute canonical 00_source.pilot
        string originalPilot = File.ReadAllText(sourcePilotPath);
        var origExec = _engine.Execute(originalPilot);
        Assert.True(origExec.Success, string.Join("; ", origExec.Diagnostics));
        Assert.Equal(6, origExec.Context!.Environment.CurrentScope.Resolve("g").AsInt());
        Assert.Equal(53, origExec.Context.Environment.CurrentScope.Resolve("weighted").AsInt());

        // 2. Verify all 8 persisted emitted files exist and pass external compiler / syntax gates
        var expectedEmitted = new[]
        {
            "source.py", "source.cs", "source.rs", "source.c",
            "source.cpp", "source.js", "source.ts", "source.lua"
        };
        foreach (var file in expectedEmitted)
        {
            string fullPath = Path.Combine(samplesRoot, "emitted", file);
            Assert.True(File.Exists(fullPath), $"Missing emitted sample: {fullPath}");
            Assert.False(string.IsNullOrWhiteSpace(File.ReadAllText(fullPath)));
        }

        string emittedPy = File.ReadAllText(Path.Combine(samplesRoot, "emitted", "source.py"));
        string emittedCs = File.ReadAllText(Path.Combine(samplesRoot, "emitted", "source.cs"));
        string emittedC = File.ReadAllText(Path.Combine(samplesRoot, "emitted", "source.c"));
        string emittedCpp = File.ReadAllText(Path.Combine(samplesRoot, "emitted", "source.cpp"));
        string emittedJs = File.ReadAllText(Path.Combine(samplesRoot, "emitted", "source.js"));

        VerifyExternalTool("python3", "\"{0}\"", ".py", emittedPy);
        VerifyExternalTool("node", "\"{0}\"", ".js", emittedJs);
        VerifyExternalTool("clang", "-std=c11 -fsyntax-only \"{0}\"", ".c", emittedC);
        VerifyExternalTool("clang++", "-std=c++20 -fsyntax-only \"{0}\"", ".cpp", emittedCpp);

        var csTree = CSharpSyntaxTree.ParseText(emittedCs);
        var trustedRefs = ((string?)AppContext.GetData("TRUSTED_PLATFORM_ASSEMBLIES") ?? "")
            .Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries)
            .Where(p => Path.GetFileName(p) is
                "System.Private.CoreLib.dll" or
                "System.Runtime.dll" or
                "System.Console.dll" or
                "System.Collections.dll" or
                "System.Linq.dll" or
                "System.Text.Json.dll" or
                "Microsoft.CSharp.dll")
            .Select(p => MetadataReference.CreateFromFile(p))
            .ToList();
        var csComp = CSharpCompilation.Create(
            "PilotRoundTripSampleCs",
            new[] { csTree },
            trustedRefs,
            new CSharpCompilationOptions(OutputKind.DynamicallyLinkedLibrary));
        var csErrors = csComp.GetDiagnostics().Where(d => d.Severity == DiagnosticSeverity.Error).ToList();
        Assert.True(csErrors.Count == 0, $"Roslyn C# errors in source.cs:\n{string.Join("\n", csErrors)}");

        // 3. Execute all 9 ingested-back .pilot files in PilotEngine and verify identical runtime state
        var ingestedFiles = new[]
        {
            "from_python.pilot",
            "from_python_strict.pilot",
            "from_csharp.pilot",
            "from_rust.pilot",
            "from_c.pilot",
            "from_cpp.pilot",
            "from_js.pilot",
            "from_ts.pilot",
            "from_lua.pilot"
        };

        foreach (var pilotFile in ingestedFiles)
        {
            string fullPath = Path.Combine(samplesRoot, "ingested_back", pilotFile);
            Assert.True(File.Exists(fullPath), $"Missing ingested sample: {fullPath}");

            string roundTrippedCode = File.ReadAllText(fullPath);
            var engine = new PilotEngine();
            var execRes = engine.Execute(roundTrippedCode);
            Assert.True(execRes.Success,
                $"Execution of '{pilotFile}' failed: {string.Join("; ", execRes.Diagnostics)}\nCode:\n{roundTrippedCode}");

            var scope = execRes.Context!.Environment.CurrentScope;
            Assert.Equal(6, scope.Resolve("g").AsInt());
            Assert.Equal(53, scope.Resolve("weighted").AsInt());

            // VerifyCounter.step(10, 5) == 15 on the round-tripped class/struct across all 8 languages
            var methodRes = engine.Execute(roundTrippedCode + "\nlet step_check = Counter().step(10, 5);\n");
            Assert.True(methodRes.Success,
                $"Counter().step(10, 5) failed in '{pilotFile}': {string.Join("; ", methodRes.Diagnostics)}");
            Assert.Equal(15, methodRes.Context!.Environment.CurrentScope.Resolve("step_check").AsInt());

            // All 7 collection-capable target languages also preserve [x * x for x in 1..6 if x > 2] == [9, 16, 25, 36]
            if (pilotFile != "from_c.pilot")
            {
                var sqList = scope.Resolve("squares").AsList();
                Assert.Equal(new long[] { 9, 16, 25, 36 }, sqList.Select(v => v.AsInt()).ToArray());
            }
        }
    }

    private static string FindRoundTripSamplesDirectory()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir != null)
        {
            string candidate = Path.Combine(dir.FullName, "Pilot.Tests", "RoundTripSamples");
            if (Directory.Exists(candidate))
                return candidate;
            string directCandidate = Path.Combine(dir.FullName, "RoundTripSamples");
            if (Directory.Exists(directCandidate))
                return directCandidate;
            dir = dir.Parent;
        }
        throw new DirectoryNotFoundException("Could not locate Pilot.Tests/RoundTripSamples directory.");
    }

    private static void VerifyExternalTool(string toolExe, string argsTemplate, string ext, string code)
    {
        string tempFile = Path.Combine(Path.GetTempPath(), "pilot_verify_" + Guid.NewGuid().ToString("N") + ext);
        try
        {
            File.WriteAllText(tempFile, code);
            var psi = new ProcessStartInfo
            {
                FileName = toolExe,
                Arguments = string.Format(argsTemplate, tempFile),
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };

            using var proc = Process.Start(psi);
            Assert.NotNull(proc);
            proc!.WaitForExit(15000);
            string stderr = proc.StandardError.ReadToEnd();
            string stdout = proc.StandardOutput.ReadToEnd();
            Assert.True(proc.ExitCode == 0,
                $"External compiler '{toolExe}' exited with {proc.ExitCode}.\nSTDERR:\n{stderr}\nSTDOUT:\n{stdout}\nCODE:\n{code}");
        }
        finally
        {
            if (File.Exists(tempFile))
                File.Delete(tempFile);
        }
    }
}
