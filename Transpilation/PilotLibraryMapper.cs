using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using Pilot.Transpilation.Ir;

namespace Pilot.Transpilation;

/// <summary>
/// Bidirectional semantic library and ecosystem translator between Pilot's native primitives
/// (PilotMatrix, LINQ/SQL queries, parallel_map, spawn, json, file I/O, math) and foreign libraries:
/// - Python: numpy (np), pandas (pd), math, json, pathlib/os, concurrent.futures, functools
/// - Rust: ndarray, nalgebra, serde_json, rayon, std::fs, std::thread
/// - C++: Eigen, std::filesystem, std::async, nlohmann::json, &lt;cmath&gt;
/// - C: &lt;math.h&gt;, &lt;stdio.h&gt;, &lt;stdlib.h&gt;, PilotMatrix C11 runtime
/// - JavaScript: mathjs, lodash (_), JSON, fs, Promise
/// - Lua: math, cjson, table, io
/// </summary>
public static partial class PilotLibraryMapper
{
    static PilotLibraryMapper()
    {
        if (Regex.CacheSize < 256)
            Regex.CacheSize = 256;
    }

    private static readonly Regex MatrixPrimitivesRegex = MyRegex();

    private static readonly Regex JsonPrimitivesRegex = MyRegex1();

    /// <summary>
    /// Milestone 20.3: AST-scoped library call formatter.
    /// Translates <see cref="PilotIrCallExpr"/> nodes carrying <see cref="PilotLibraryCallKind"/>
    /// into target ecosystem API calls without touching string literals or comments.
    /// </summary>
    public static bool TryFormatAstLibraryCall(
        PilotIrCallExpr call,
        string targetLang,
        Func<PilotIrExpr, string> renderExpr,
        out string rendered)
    {
        var args = call.Arguments.Select(a => renderExpr(a.Value)).ToList();
        return TryFormatAstLibraryCall(call.LibraryCallKind, args, targetLang, out rendered, call);
    }

    public static bool TryFormatAstLibraryCall(
        PilotLibraryCallKind kind,
        IReadOnlyList<string> args,
        string targetLang,
        out string rendered,
        PilotIrCallExpr? call = null)
    {
        rendered = "";
        if (kind == PilotLibraryCallKind.None)
            return false;

        string lang = targetLang.ToLowerInvariant();
        string a0 = args.Count > 0 ? args[0] : "";
        string a1 = args.Count > 1 ? args[1] : "";

        switch (kind)
        {
            case PilotLibraryCallKind.Println:
                rendered = lang switch
                {
                    "python" or "py" => $"print({string.Join(", ", args)})",
                    "csharp" or "cs" => $"Console.WriteLine({string.Join(", ", args)})",
                    "java" => $"System.out.println({string.Join(", ", args)})",
                    "ruby" or "rb" => $"puts({string.Join(", ", args)})",
                    "js" or "javascript" or "ts" or "typescript" => $"console.log({string.Join(", ", args)})",
                    "lua" => $"print({string.Join(", ", args)})",
                    "rust" or "rs" => args.Count > 0 ? $"println!(\"{{:?}}\", {a0})" : "println!()",
                    "cpp" or "c++" => args.Count > 0 ? $"std::cout << {a0} << '\\n'" : "std::cout << '\\n'",
                    "c" => args.Count > 0 ? $"printf(\"%lld\\n\", (long long)({a0}))" : "printf(\"\\n\")",
                    _ => $"println({string.Join(", ", args)})"
                };
                return true;

            case PilotLibraryCallKind.Print:
                rendered = lang switch
                {
                    "python" or "py" => args.Count > 0 ? $"print({a0}, end=\"\")" : "print(\"\", end=\"\")",
                    "csharp" or "cs" => $"Console.Write({string.Join(", ", args)})",
                    "java" => $"System.out.print({string.Join(", ", args)})",
                    "ruby" or "rb" => $"print({string.Join(", ", args)})",
                    "js" or "javascript" or "ts" or "typescript" => $"process.stdout.write(String({a0}))",
                    "lua" => $"io.write(tostring({a0}))",
                    "rust" or "rs" => $"print!(\"{{:?}}\", {a0})",
                    "cpp" or "c++" => $"std::cout << {a0}",
                    "c" => $"printf(\"%lld\", (long long)({a0}))",
                    _ => $"print({string.Join(", ", args)})"
                };
                return true;

            case PilotLibraryCallKind.MatrixZeros:
                rendered = lang switch
                {
                    "python" or "py" => $"np.zeros(({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)}))",
                    "js" or "javascript" or "ts" or "typescript" => $"PilotMatrix.zeros({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)})",
                    "lua" => $"PilotMatrix.zeros({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)})",
                    "java" => $"PilotMatrix.zeros({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)})",
                    "ruby" or "rb" => $"PilotMatrix.zeros({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)})",
                    "rust" or "rs" => $"PilotMatrix::zeros({a0} as usize, {(string.IsNullOrEmpty(a1) ? a0 : a1)} as usize)",
                    "cpp" or "c++" => $"PilotMatrix::zeros({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)})",
                    "c" => $"pilot_matrix_zeros({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)})",
                    "csharp" or "cs" => $"PilotMatrix.Zeros({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)})",
                    _ => $"zeros({string.Join(", ", args)})"
                };
                return true;

            case PilotLibraryCallKind.MatrixOnes:
                rendered = lang switch
                {
                    "python" or "py" => $"np.ones(({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)}))",
                    "js" or "javascript" or "ts" or "typescript" => $"PilotMatrix.ones({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)})",
                    "lua" => $"PilotMatrix.ones({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)})",
                    "java" => $"PilotMatrix.ones({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)})",
                    "ruby" or "rb" => $"PilotMatrix.ones({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)})",
                    "rust" or "rs" => $"PilotMatrix::ones({a0} as usize, {(string.IsNullOrEmpty(a1) ? a0 : a1)} as usize)",
                    "cpp" or "c++" => $"PilotMatrix::ones({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)})",
                    "c" => $"pilot_matrix_ones({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)})",
                    "csharp" or "cs" => $"PilotMatrix.Ones({a0}, {(string.IsNullOrEmpty(a1) ? a0 : a1)})",
                    _ => $"ones({string.Join(", ", args)})"
                };
                return true;

            case PilotLibraryCallKind.MatrixEye:
                rendered = lang switch
                {
                    "python" or "py" => $"np.eye({a0})",
                    "js" or "javascript" or "ts" or "typescript" => $"PilotMatrix.eye({a0})",
                    "lua" => $"PilotMatrix.eye({a0})",
                    "java" => $"PilotMatrix.eye({a0})",
                    "ruby" or "rb" => $"PilotMatrix.eye({a0})",
                    "rust" or "rs" => $"PilotMatrix::eye({a0} as usize)",
                    "cpp" or "c++" => $"PilotMatrix::eye({a0})",
                    "c" => $"pilot_matrix_eye({a0})",
                    "csharp" or "cs" => $"PilotMatrix.Eye({a0})",
                    _ => $"eye({a0})"
                };
                return true;

            case PilotLibraryCallKind.MatrixCreate:
                if (lang is "python" or "py")
                {
                    if (call != null && call.Arguments.Count > 1 && call.Arguments[1].Value is PilotIrLiteralExpr dtypeLit)
                    {
                        rendered = $"np.array({a0}, dtype=np.{MapPilotTypeToNumpy(dtypeLit.StringContent)})";
                        return true;
                    }
                    if (!string.IsNullOrEmpty(a1))
                    {
                        string rawType = a1.Trim('"', '\'');
                        rendered = $"np.array({a0}, dtype=np.{MapPilotTypeToNumpy(rawType)})";
                        return true;
                    }
                    rendered = $"np.array({a0})";
                    return true;
                }
                if (lang is "js" or "javascript" or "ts" or "typescript")
                {
                    rendered = $"PilotMatrix.fromArray({a0})";
                    return true;
                }
                if (lang == "java")
                {
                    rendered = $"PilotMatrix.fromList({a0})";
                    return true;
                }
                if (lang is "ruby" or "rb")
                {
                    rendered = $"PilotMatrix.from_array({a0})";
                    return true;
                }
                if (lang == "lua")
                {
                    rendered = $"PilotMatrix.from_table({a0.Replace('[', '{').Replace(']', '}')})";
                    return true;
                }
                if (lang is "rust" or "rs")
                {
                    rendered = $"PilotMatrix::from_rows(&{a0})";
                    return true;
                }
                if (lang is "cpp" or "c++")
                {
                    rendered = $"PilotMatrix({a0.Replace('[', '{').Replace(']', '}')})";
                    return true;
                }
                return false;

            case PilotLibraryCallKind.MatrixVectorize:
                if (lang is "python" or "py")
                {
                    rendered = $"np.vectorize({a0})";
                    return true;
                }
                return false;

            case PilotLibraryCallKind.JsonDumps:
                rendered = lang switch
                {
                    "python" or "py" => $"json.dumps({a0})",
                    "js" or "javascript" or "ts" or "typescript" => $"JSON.stringify({a0})",
                    "csharp" or "cs" => $"System.Text.Json.JsonSerializer.Serialize({a0})",
                    "java" => $"String.valueOf({a0})",
                    "ruby" or "rb" => $"JSON.generate({a0})",
                    "rust" or "rs" => $"format!(\"{{:?}}\", {a0})",
                    _ => $"json_dumps({a0})"
                };
                return true;

            case PilotLibraryCallKind.JsonLoads:
                rendered = lang switch
                {
                    "python" or "py" => $"json.loads({a0})",
                    "js" or "javascript" or "ts" or "typescript" => $"JSON.parse({a0})",
                    "csharp" or "cs" => $"System.Text.Json.JsonSerializer.Deserialize<object>({a0})",
                    "java" => a0,
                    "ruby" or "rb" => $"JSON.parse({a0})",
                    _ => $"json_loads({a0})"
                };
                return true;

            case PilotLibraryCallKind.FileReadText:
                rendered = lang switch
                {
                    "python" or "py" => $"open({a0}, encoding=\"utf-8\").read()",
                    "js" or "javascript" or "ts" or "typescript" => $"fs.readFileSync({a0}, \"utf8\")",
                    "csharp" or "cs" => $"System.IO.File.ReadAllText({a0})",
                    "java" => $"Files.readString(Path.of({a0}))",
                    "ruby" or "rb" => $"File.read({a0}, encoding: \"UTF-8\")",
                    "rust" or "rs" => $"std::fs::read_to_string({a0}).unwrap_or_default()",
                    _ => $"read_text({a0})"
                };
                return true;

            case PilotLibraryCallKind.FileWriteText:
                rendered = lang switch
                {
                    "python" or "py" => $"open({a0}, \"w\", encoding=\"utf-8\").write({a1})",
                    "js" or "javascript" or "ts" or "typescript" => $"fs.writeFileSync({a0}, {a1})",
                    "csharp" or "cs" => $"System.IO.File.WriteAllText({a0}, {a1})",
                    "java" => $"Files.writeString(Path.of({a0}), {a1})",
                    "ruby" or "rb" => $"File.write({a0}, {a1})",
                    "rust" or "rs" => $"std::fs::write({a0}, {a1}).unwrap()",
                    _ => $"write_text({a0}, {a1})"
                };
                return true;

            case PilotLibraryCallKind.FileExists:
                rendered = lang switch
                {
                    "python" or "py" => $"os.path.exists({a0})",
                    "js" or "javascript" or "ts" or "typescript" => $"fs.existsSync({a0})",
                    "csharp" or "cs" => $"System.IO.File.Exists({a0})",
                    "java" => $"Files.exists(Path.of({a0}))",
                    "ruby" or "rb" => $"File.exist?({a0})",
                    "rust" or "rs" => $"std::path::Path::new({a0}).exists()",
                    _ => $"file_exists({a0})"
                };
                return true;

            case PilotLibraryCallKind.MathSqrt:
            case PilotLibraryCallKind.MathSin:
            case PilotLibraryCallKind.MathCos:
            case PilotLibraryCallKind.MathAbs:
            case PilotLibraryCallKind.MathFloor:
            case PilotLibraryCallKind.MathCeil:
                string fn = kind switch
                {
                    PilotLibraryCallKind.MathSqrt => "sqrt",
                    PilotLibraryCallKind.MathSin => "sin",
                    PilotLibraryCallKind.MathCos => "cos",
                    PilotLibraryCallKind.MathAbs => "abs",
                    PilotLibraryCallKind.MathFloor => "floor",
                    _ => "ceil"
                };
                string csFn = char.ToUpperInvariant(fn[0]) + fn[1..];
                rendered = lang switch
                {
                    "python" or "py" => fn == "abs" ? $"abs({a0})" : $"math.{fn}({a0})",
                    "js" or "javascript" or "ts" or "typescript" => $"Math.{fn}({a0})",
                    "java" => $"Math.{fn}({a0})",
                    "ruby" or "rb" => fn is "abs" or "floor" or "ceil" ? $"({a0}).{fn}" : $"Math.{fn}({a0})",
                    "lua" => $"math.{fn}({a0})",
                    "csharp" or "cs" => $"Math.{csFn}({a0})",
                    "rust" or "rs" => $"({a0} as f64).{fn}()",
                    "cpp" or "c++" => $"std::{fn}({a0})",
                    "c" => fn == "abs" ? $"fabs((double)({a0}))" : $"{fn}((double)({a0}))",
                    _ => $"{fn}({a0})"
                };
                return true;

            case PilotLibraryCallKind.Len:
                rendered = lang switch
                {
                    "python" or "py" => $"len({a0})",
                    "js" or "javascript" or "ts" or "typescript" => $"{a0}.length",
                    "lua" => $"#{a0}",
                    "csharp" or "cs" => $"{a0}.Count",
                    "java" => $"{a0}.size()",
                    "ruby" or "rb" => $"{a0}.length",
                    "rust" or "rs" => $"({a0}.len() as i64)",
                    "cpp" or "c++" => $"static_cast<int64_t>({a0}.size())",
                    _ => $"len({a0})"
                };
                return true;

            case PilotLibraryCallKind.SomeConstructor:
                rendered = lang switch
                {
                    "rust" or "rs" => $"Some({a0})",
                    "cpp" or "c++" => $"std::make_optional({a0})",
                    _ => a0
                };
                return true;

            case PilotLibraryCallKind.FailConstructor:
                rendered = lang switch
                {
                    "rust" or "rs" => $"Err({a0})",
                    "csharp" or "cs" => $"PilotResult.Fail({a0})",
                    "java" => $"Map.of(\"ok\", false, \"error\", {a0})",
                    "ruby" or "rb" => $"{{ ok: false, error: {a0} }}",
                    "js" or "javascript" or "ts" or "typescript" => $"{{ ok: false, error: {a0} }}",
                    "lua" => $"{{ ok = false, error = {a0} }}",
                    _ => $"fail({a0})"
                };
                return true;
        }

        return false;
    }

    public static bool TryFormatAstMemberAccess(
        PilotIrMemberAccessExpr mem,
        string targetLang,
        Func<PilotIrExpr, string> renderExpr,
        out string rendered) =>
        TryFormatAstMemberAccess(renderExpr(mem.Target), mem.MemberName, targetLang, out rendered);

    public static bool TryFormatAstMemberAccess(
        string targetRendered,
        string memberName,
        string targetLang,
        out string rendered)
    {
        rendered = "";
        string lang = targetLang.ToLowerInvariant();
        if (memberName == "element_type" && (lang is "python" or "py"))
        {
            rendered = $"{targetRendered}.dtype.name";
            return true;
        }
        return false;
    }

    /// <summary>
    /// Detects whether a Pilot program AST/source uses native matrix/linear-algebra constructs
    /// (matrix, zeros, ones, eye) so target emitters can inject ecosystem imports or runtime shims.
    /// Ignores occurrences inside string literals or comments.
    /// </summary>
    public static bool UsesMatrixPrimitives(string pilotSource)
    {
        string stripped = StripStringsAndComments(pilotSource);
        return MatrixPrimitivesRegex.IsMatch(stripped);
    }

    /// <summary>
    /// Detects whether a Pilot source uses JSON serialization primitives.
    /// </summary>
    public static bool UsesJsonPrimitives(string pilotSource)
    {
        string stripped = StripStringsAndComments(pilotSource);
        return JsonPrimitivesRegex.IsMatch(stripped);
    }

    private static string StripStringsAndComments(string src)
    {
        if (string.IsNullOrEmpty(src)) return src;
        var sb = new StringBuilder(src.Length);
        bool inStr = false;
        bool inLineComment = false;
        for (int i = 0; i < src.Length; i++)
        {
            char c = src[i];
            if (inLineComment)
            {
                if (c == '\n') { inLineComment = false; sb.Append('\n'); }
                continue;
            }
            if (inStr)
            {
                if (c == '\\' && i + 1 < src.Length) { i++; continue; }
                if (c == '"') inStr = false;
                continue;
            }
            if (c == '/' && i + 1 < src.Length && src[i + 1] == '/')
            {
                inLineComment = true;
                i++;
                continue;
            }
            if (c == '"')
            {
                inStr = true;
                sb.Append("\"\"");
                continue;
            }
            sb.Append(c);
        }
        return sb.ToString();
    }

    // =========================================================================
    // 1. FOREIGN LIBRARY CALLS -> NATIVE PILOT EQUIVALENTS (INGESTION)
    // =========================================================================

    /// <summary>
    /// Translates foreign library imports and API calls (such as Python's NumPy, Pandas, math, json;
    /// Rust's ndarray, serde_json, rayon; C++ Eigen, std::filesystem; JS lodash/mathjs/fs) into
    /// native Pilot constructs and standard built-ins.
    /// </summary>
    public static string TranslateForeignLibrariesToPilot(string line, string sourceLang)
    {
        if (string.IsNullOrWhiteSpace(line))
            return line;

        string s = line;

        // ---------------------------------------------------------------------
        // A. Import & Header Stripping / Native Mapping
        // ---------------------------------------------------------------------
        var trimmed = s.Trim();
        if (TryMapForeignStdlibImportToPilot(trimmed, sourceLang, out string? mappedPilotStdImport))
        {
            return mappedPilotStdImport!;
        }
        if (trimmed.StartsWith("import ", StringComparison.Ordinal) || trimmed.StartsWith("from ", StringComparison.Ordinal))
        {
            if (MyRegex2().IsMatch(trimmed))
            {
                return $"// [Native Pilot Library Equivalent] {trimmed}";
            }
        }
        if (trimmed.StartsWith("use ", StringComparison.Ordinal))
        {
            if (MyRegex3().IsMatch(trimmed))
            {
                return $"// [Native Pilot Library Equivalent] {trimmed}";
            }
        }
        if (trimmed.Contains("require(", StringComparison.Ordinal) || trimmed.StartsWith("require ", StringComparison.Ordinal))
        {
            if (MyRegex4().IsMatch(trimmed) || RubyRequireStdRegex().IsMatch(trimmed))
            {
                return $"// [Native Pilot Library Equivalent] {trimmed}";
            }
        }

        // Fast-path check: if line has no calls, dots, brackets, or @ operator, skip regex replacements
        if (s.IndexOfAny(new[] { '.', '(', '[', '@', ':' }) < 0)
            return s;

        // ---------------------------------------------------------------------
        // B. Linear Algebra: NumPy (np.*) / Eigen / ndarray / mathjs -> PilotMatrix
        // ---------------------------------------------------------------------
        // np.zeros((r, c)) or np.zeros([r, c]) or Array2::zeros((r, c)) or Eigen::MatrixXd::Zero(r, c) -> zeros(r, c)
        s = NPRegexZeroes1().Replace(s, "zeros($1, $2)");
        s = NPRegexZeroes2().Replace(s, "zeros(1, $1)");
        s = NPRegexZeroes3().Replace(s, "zeros($1, $2)");
        s = NPRegexZeroes4().Replace(s, "zeros($1, $2)");

        // np.ones((r, c)) / Array2::ones((r, c)) / Eigen::MatrixXd::Ones(r, c) -> ones(r, c)
        s = NPRegexOnes1().Replace(s, "ones($1, $2)");
        s = NPRegexOnes2().Replace(s, "ones(1, $1)");
        s = NPRegexOnes3().Replace(s, "ones($1, $2)");
        s = NPRegexOnes4().Replace(s, "ones($1, $2)");

        // np.eye(n) / np.identity(n) / Array2::eye(n) / Eigen::MatrixXd::Identity(n, n) / math.identity(n) -> eye(n)
        s = NPRegexEye1().Replace(s, "eye($1)");
        s = NPRegexEye2().Replace(s, "eye($1)");
        s = NPRegexEye3().Replace(s, "eye($1)");

        // np.array([[...]], dtype=np.float32) -> matrix([[...]], "f32")
        s = NPRegexArray1().Replace(s, m =>
        {
            string arr = m.Groups[1].Value;
            string dtype = MapNumpyDtypeToPilot(m.Groups[2].Value.Trim('\'', '"'));
            return $"matrix({arr}, \"{dtype}\")";
        });
        s = NPRegexMatrix1().Replace(s, "matrix($1)");
        s = NPRegexMatrix2().Replace(s, "matrix($1)");

        // Matrix multiplication: np.dot(A, B), np.matmul(A, B), math.multiply(A, B), A.dot(&B), A @ B -> A * B
        s = NPRegexMatrix3().Replace(s, "($1 * $2)");
        s = NPRegexMatrix4().Replace(s, "($1 * $2)");
        s = NPRegexMatrix5().Replace(s, "$1 * $2");

        // np.arange(start, stop) -> range(start, stop)
        s = NPRegexRange1().Replace(s, "range(");

        // Matrix properties: .dtype -> .element_type
        s = NPRegexMatrix6().Replace(s, ".element_type");

        // Vectorized NumPy ufuncs: np.sqrt(arr), np.sin(arr), np.cos(arr), np.abs(arr)
        s = NPRegexVector1().Replace(s, "vectorize($1)");

        // ---------------------------------------------------------------------
        // C. DataFrames & Functional Pipelines (Pandas / Lodash / Python / Rust Iterators)
        // ---------------------------------------------------------------------
        // Pandas DataFrame filter: df[df["col"] > val] -> from row in df where row["col"] > val select row
        s = PandasRegexDF1().Replace(s, "from row in $1 where row[$2] $3 $4 select row");

        // Python list(map(fn, items)) / Lodash _.map(items, fn) -> items.map(fn)
        s = PyRegexMap1().Replace(s, "$2.map($1)");
        s = PyRegexMap2().Replace(s, "$2.filter($1)");
        s = PyRegexMap3().Replace(s, "$1.map($2)");
        s = PyRegexMap4().Replace(s, "$1.filter($2)");
        s = PyRegexMap5().Replace(s, m =>
            m.Groups[3].Success ? $"{m.Groups[2].Value}.reduce({m.Groups[1].Value}, {m.Groups[3].Value})" : $"{m.Groups[2].Value}.reduce({m.Groups[1].Value})");

        // Rust Rayon par_iter: items.par_iter().map(fn).collect() -> parallel_map(fn, items)
        s = RustRegexMap1().Replace(s, "parallel_map($2, $1)");
        s = RustRegexMap2().Replace(s, "$1.map($2)");
        s = RustRegexMap3().Replace(s, "$1.filter($2)");

        // ---------------------------------------------------------------------
        // D. JSON & File System Equivalents
        // ---------------------------------------------------------------------
        // JSON.stringify / serde_json::to_string / cjson.encode -> json.dumps
        s = JsonRegexString1().Replace(s, "json.dumps($1)");
        s = JsonRegexString2().Replace(s, "json.loads($1)");

        // File I/O: fs.readFileSync(p, 'utf8') / std::fs::read_to_string(p).unwrap() / Path(p).read_text() -> read_text(p)
        s = FileIORegex1().Replace(s, "read_text($1)");
        s = FileIORegex2().Replace(s, "write_text($1, $2)");
        s = FileIORegex3().Replace(s, "file_exists($1)");
        s = FileIORegex4().Replace(s, "read_text($1)");
        s = FileIORegex5().Replace(s, "write_text($1, $2)");
        s = FileIORegex6().Replace(s, "file_exists($1)");

        // Concurrency: tokio::spawn / std::async / asyncio.create_task -> spawn
        s = ConcurrencyRegex1().Replace(s, "spawn($1)");

        // Java & Ruby PilotMatrix / Files / JSON / Math ecosystem calls -> Pilot built-ins
        if (sourceLang is "java" or "ruby" or "rb")
        {
            s = PilotMatrixZerosCallRegex().Replace(s, "zeros($1, $2)");
            s = PilotMatrixOnesCallRegex().Replace(s, "ones($1, $2)");
            s = PilotMatrixEyeCallRegex().Replace(s, "eye($1)");
            s = PilotMatrixFromCallRegex().Replace(s, "matrix($1)");
            s = RubyJsonGenRegex().Replace(s, "json.dumps($1)");
            s = JavaFilesReadRegex().Replace(s, "read_text($1)");
            s = JavaFilesWriteRegex().Replace(s, "write_text($1, $2)");
            s = JavaFilesExistsRegex().Replace(s, "file_exists($1)");
            s = RubyFileReadRegex().Replace(s, "read_text($1)");
            s = RubyFileWriteRegex().Replace(s, "write_text($1, $2)");
            s = RubyFileExistRegex().Replace(s, "file_exists($1)");
            s = JavaRubyMathFnRegex().Replace(s, "$1($2)");
        }

        return s;
    }

    private static string MapNumpyDtypeToPilot(string dtype) => dtype.ToLowerInvariant() switch
    {
        "float32" or "f4" => "f32",
        "float64" or "f8" or "float" => "f64",
        "float16" or "f2" => "f16",
        "int8" or "i1" => "i8",
        "int16" or "i2" => "i16",
        "int32" or "i4" or "int" => "i32",
        "int64" or "i8" => "i64",
        "uint8" or "u1" => "u8",
        "uint16" or "u2" => "u16",
        "uint32" or "u4" => "u32",
        "uint64" or "u8" => "u64",
        _ => "f64"
    };

    // =========================================================================
    // 2. NATIVE PILOT CONSTRUCTS -> TARGET ECOSYSTEM LIBRARIES (EMISSION)
    // =========================================================================

    /// <summary>
    /// Translates Pilot's native matrix, JSON, functional, and file I/O expressions
    /// into the idiomatic library calls of the target language (e.g., NumPy in Python,
    /// PilotMatrix/ndarray in Rust, PilotMatrix/Eigen in C++, etc.).
    /// </summary>
    public static string TranslatePilotLibrariesToTarget(string expr, string targetLang)
    {
        if (string.IsNullOrWhiteSpace(expr) || (expr.IndexOf('(') < 0 && expr.IndexOf('.') < 0))
            return expr;

        string s = expr;
        string lang = targetLang.ToLowerInvariant();

        switch (lang)
        {
            case "python" or "py":
                // Pilot matrix([[...]], "f32") -> np.array([[...]], dtype=np.float32)
                s = Regex.Replace(s, @"\bmatrix\(\s*(\[.+\])\s*,\s*""([^""]+)""\s*\)", m =>
                    $"np.array({m.Groups[1].Value}, dtype=np.{MapPilotTypeToNumpy(m.Groups[2].Value)})");
                s = Regex.Replace(s, @"\bmatrix\(\s*(\[.+\])\s*\)", "np.array($1)");
                s = Regex.Replace(s, @"\bzeros\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "np.zeros(($1, $2))");
                s = Regex.Replace(s, @"\bones\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "np.ones(($1, $2))");
                s = Regex.Replace(s, @"\beye\(\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "np.eye($1)");
                s = Regex.Replace(s, @"\bvectorize\(\s*([^)]+)\s*\)", "np.vectorize($1)");
                s = Regex.Replace(s, @"\.element_type\b", ".dtype.name");
                s = Regex.Replace(s, @"\bjson_dumps\(", "json.dumps(");
                s = Regex.Replace(s, @"\bjson_loads\(", "json.loads(");
                break;

            case "js" or "javascript":
                s = Regex.Replace(s, @"\bmatrix\(\s*(\[.+\])(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.fromArray($1)");
                s = Regex.Replace(s, @"\bzeros\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.zeros($1, $2)");
                s = Regex.Replace(s, @"\bones\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.ones($1, $2)");
                s = Regex.Replace(s, @"\beye\(\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.eye($1)");
                s = Regex.Replace(s, @"\b(?:json\.dumps|json_dumps)\(", "JSON.stringify(");
                s = Regex.Replace(s, @"\b(?:json\.loads|json_loads)\(", "JSON.parse(");
                break;

            case "lua":
                s = Regex.Replace(s, @"\bmatrix\(\s*(\[.+\])(?:\s*,\s*""[^""]+"")?\s*\)", m =>
                    $"PilotMatrix.from_table({m.Groups[1].Value.Replace('[', '{').Replace(']', '}')})");
                s = Regex.Replace(s, @"\bzeros\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.zeros($1, $2)");
                s = Regex.Replace(s, @"\bones\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.ones($1, $2)");
                s = Regex.Replace(s, @"\beye\(\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.eye($1)");
                break;

            case "rust" or "rs":
                s = Regex.Replace(s, @"\bmatrix\(\s*(\[.+\])(?:\s*,\s*""[^""]+"")?\s*\)", m =>
                    $"PilotMatrix::from_rows(&{m.Groups[1].Value.Replace("[", "vec![" )})");
                s = Regex.Replace(s, @"\bzeros\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix::zeros($1, $2)");
                s = Regex.Replace(s, @"\bones\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix::ones($1, $2)");
                s = Regex.Replace(s, @"\beye\(\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix::eye($1)");
                break;

            case "cpp" or "c++":
                s = Regex.Replace(s, @"\bmatrix\(\s*(\[.+\])(?:\s*,\s*""[^""]+"")?\s*\)", m =>
                    $"PilotMatrix({m.Groups[1].Value.Replace('[', '{').Replace(']', '}')})");
                s = Regex.Replace(s, @"\bzeros\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix::zeros($1, $2)");
                s = Regex.Replace(s, @"\bones\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix::ones($1, $2)");
                s = Regex.Replace(s, @"\beye\(\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix::eye($1)");
                break;

            case "c":
                s = Regex.Replace(s, @"\bzeros\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "pilot_matrix_zeros($1, $2)");
                s = Regex.Replace(s, @"\bones\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "pilot_matrix_ones($1, $2)");
                s = Regex.Replace(s, @"\beye\(\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "pilot_matrix_eye($1)");
                break;

            case "java":
                s = Regex.Replace(s, @"\bmatrix\(\s*(\[.+\])(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.fromList($1)");
                s = Regex.Replace(s, @"\bzeros\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.zeros($1, $2)");
                s = Regex.Replace(s, @"\bones\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.ones($1, $2)");
                s = Regex.Replace(s, @"\beye\(\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.eye($1)");
                break;

            case "ruby" or "rb":
                s = Regex.Replace(s, @"\bmatrix\(\s*(\[.+\])(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.from_array($1)");
                s = Regex.Replace(s, @"\bzeros\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.zeros($1, $2)");
                s = Regex.Replace(s, @"\bones\(\s*([^,)]+)\s*,\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.ones($1, $2)");
                s = Regex.Replace(s, @"\beye\(\s*([^,)]+)(?:\s*,\s*""[^""]+"")?\s*\)", "PilotMatrix.eye($1)");
                s = Regex.Replace(s, @"\b(?:json\.dumps|json_dumps)\(", "JSON.generate(");
                s = Regex.Replace(s, @"\b(?:json\.loads|json_loads)\(", "JSON.parse(");
                break;
        }

        return s;
    }

    /// <summary>
    /// Milestone 21.1: Maps a native Pilot <c>std.*</c> module import (e.g. <c>std.math</c>, <c>std.collections</c>)
    /// to the idiomatic target ecosystem import/header statement during <c>--emit &lt;target&gt;</c>.
    /// </summary>
    public static string MapPilotStdModuleToTarget(string stdModule, string targetLang)
    {
        string mod = stdModule.Trim();
        string lang = targetLang.ToLowerInvariant();
        return (mod, lang) switch
        {
            ("std.math", "python" or "py") => "import math, statistics",
            ("std.math", "rust" or "rs") => "use std::f64::consts;",
            ("std.math", "c") => "#include <math.h>",
            ("std.math", "cpp" or "c++") => "#include <cmath>",
            ("std.math", "csharp" or "cs") => "using static System.Math;",
            ("std.math", "js" or "javascript" or "ts" or "typescript") => "// Math (built-in global)",
            ("std.math", "lua") => "local math = require(\"math\")",

            ("std.collections", "python" or "py") => "from collections import deque, defaultdict\nimport heapq, bisect",
            ("std.collections", "rust" or "rs") => "use std::collections::{VecDeque, BinaryHeap, BTreeMap, HashMap};",
            ("std.collections", "cpp" or "c++") => "#include <deque>\n#include <queue>\n#include <map>",
            ("std.collections", "csharp" or "cs") => "using System.Collections.Generic;",

            ("std.itertools", "python" or "py") => "import itertools",
            ("std.itertools", "rust" or "rs") => "// std::iter::Iterator combinators",
            ("std.itertools", "csharp" or "cs") => "using System.Linq;",

            ("std.text", "python" or "py") => "import re",
            ("std.text", "rust" or "rs") => "use std::str;",
            ("std.text", "cpp" or "c++") => "#include <string_view>\n#include <regex>",
            ("std.text", "csharp" or "cs") => "using System.Text;\nusing System.Text.RegularExpressions;",

            ("std.random", "python" or "py") => "import random",
            ("std.random", "rust" or "rs") => "// seeded PRNG (SplitMix64 / PcgRng)",
            ("std.random", "cpp" or "c++") => "#include <random>",
            ("std.random", "csharp" or "cs") => "using System;",

            ("std.io", "python" or "py") => "import io",
            ("std.io", "rust" or "rs") => "use std::io::{self, BufRead, BufReader, BufWriter, Write};",
            ("std.io", "c") => "#include <stdio.h>",
            ("std.io", "cpp" or "c++") => "#include <iostream>\n#include <fstream>",
            ("std.io", "csharp" or "cs") => "using System.IO;",

            ("std.fs", "python" or "py") => "import os, glob, mmap",
            ("std.fs", "rust" or "rs") => "use std::fs;",
            ("std.fs", "cpp" or "c++") => "#include <filesystem>",
            ("std.fs", "csharp" or "cs") => "using System.IO;\nusing System.IO.MemoryMappedFiles;",
            ("std.fs", "js" or "javascript" or "ts" or "typescript") => "const fs = require(\"fs\");",

            ("std.path", "python" or "py") => "from pathlib import Path",
            ("std.path", "rust" or "rs") => "use std::path::{Path, PathBuf};",
            ("std.path", "cpp" or "c++") => "#include <filesystem>",
            ("std.path", "csharp" or "cs") => "using System.IO;",
            ("std.path", "js" or "javascript" or "ts" or "typescript") => "const path = require(\"path\");",

            ("std.encoding", "python" or "py") => "import base64, binascii, urllib.parse",
            ("std.encoding", "rust" or "rs") => "// std.encoding: base64 / hex / RFC-3986 url codecs",
            ("std.encoding", "csharp" or "cs") => "using System;\nusing System.Text;",
            ("std.encoding", "js" or "javascript" or "ts" or "typescript") => "// std.encoding: Buffer / URL / URLSearchParams",
            ("std.encoding", "java") => "import java.util.Base64;\nimport java.net.URI;\nimport java.net.URLEncoder;\nimport java.net.URLDecoder;",
            ("std.encoding", "ruby" or "rb") => "require \"base64\"\nrequire \"uri\"\nrequire \"cgi\"",

            ("std.formats", "python" or "py") => "import json, csv, tomllib",
            ("std.formats", "rust" or "rs") => "// std.formats: serde_json / toml / csv / dotenv",
            ("std.formats", "csharp" or "cs") => "using System.Text.Json;",
            ("std.formats", "js" or "javascript" or "ts" or "typescript") => "// std.formats: JSON / TOML / CSV / dotenv",
            ("std.formats", "java") => "// std.formats: JSON / TOML / CSV / dotenv",
            ("std.formats", "ruby" or "rb") => "require \"json\"\nrequire \"csv\"",

            _ => $"// {mod}"
        };
    }

    /// <summary>
    /// Milestone 21.1 &amp; 24.2: Maps foreign standard-library imports (Python <c>collections</c>/<c>itertools</c>/<c>pathlib</c>/<c>re</c>/<c>random</c>/<c>base64</c>/<c>csv</c>/<c>tomllib</c>,
    /// Rust <c>std::collections</c>/<c>std::path</c>/<c>std::io</c>, C <c>&lt;math.h&gt;</c>) directly to native Pilot <c>import std.*</c>.
    /// </summary>
    public static bool TryMapForeignStdlibImportToPilot(string trimmedLine, string sourceLang, out string? pilotImport)
    {
        pilotImport = null;
        string clean = trimmedLine.TrimEnd(';').Trim();
        switch (clean)
        {
            case "import collections":
            case "from collections import deque":
            case "import heapq":
            case "import bisect":
            case "use std::collections::VecDeque":
            case "use std::collections::BinaryHeap":
            case "use std::collections::BTreeMap":
                pilotImport = "import std.collections;";
                return true;
            case "import itertools":
                pilotImport = "import std.itertools;";
                return true;
            case "import re":
                pilotImport = "import std.text;";
                return true;
            case "import statistics":
            case "#include <math.h>":
            case "#include <cmath>":
                pilotImport = "import std.math;";
                return true;
            case "import random":
                pilotImport = "import std.random;";
                return true;
            case "import pathlib":
            case "from pathlib import Path":
            case "use std::path::Path":
            case "use std::path::PathBuf":
                pilotImport = "import std.path;";
                return true;
            case "import io":
            case "use std::io":
                pilotImport = "import std.io;";
                return true;
            case "import glob":
            case "import mmap":
                pilotImport = "import std.fs;";
                return true;
            case "import base64":
            case "import binascii":
            case "import urllib.parse":
            case "from urllib.parse import urlparse":
            case "from urllib.parse import quote, unquote":
            case "import java.util.Base64":
            case "require \"base64\"":
            case "require 'base64'":
            case "require \"uri\"":
            case "require 'uri'":
                pilotImport = "import std.encoding;";
                return true;
            case "import csv":
            case "import tomllib":
            case "import tomli":
            case "import dotenv":
            case "from dotenv import dotenv_values":
            case "require \"csv\"":
            case "require 'csv'":
                pilotImport = "import std.formats;";
                return true;
            default:
                return false;
        }
    }

    private static string MapPilotTypeToNumpy(string pType) => pType.ToLowerInvariant() switch
    {
        "f16" => "float16",
        "f32" => "float32",
        "f64" or "float" or "double" => "float64",
        "i8" => "int8",
        "i16" => "int16",
        "i32" or "int" => "int32",
        "i64" => "int64",
        "u8" => "uint8",
        "u16" => "uint16",
        "u32" or "uint" => "uint32",
        "u64" => "uint64",
        _ => "float64"
    };

    /// <summary>
    /// Emits a zero-dependency native matrix runtime preamble for target languages (Lua, JS, C, C++, Rust)
    /// when a Pilot script uses PilotMatrix operations (matrix, zeros, ones, eye).
    /// For Python, emits 'import numpy as np'.
    /// </summary>
    public static string GetMatrixRuntimePreamble(string targetLang) => targetLang.ToLowerInvariant() switch
    {
        "python" or "py" => "import numpy as np\nimport json\n",

        "js" or "javascript" => """
class PilotMatrix {
    constructor(rows, cols, data = null) {
        this.rows = rows;
        this.cols = cols;
        this.shape = [rows, cols];
        this.data = data ? Float64Array.from(data) : new Float64Array(rows * cols);
    }
    static zeros(r, c = r) { return new PilotMatrix(r, c); }
    static ones(r, c = r) { const m = new PilotMatrix(r, c); m.data.fill(1.0); return m; }
    static eye(n) { const m = new PilotMatrix(n, n); for (let i = 0; i < n; i++) m.data[i * n + i] = 1.0; return m; }
    static fromArray(arr) {
        const r = arr.length, c = arr[0].length;
        const m = new PilotMatrix(r, c);
        for (let i = 0; i < r; i++) for (let j = 0; j < c; j++) m.data[i * c + j] = arr[i][j];
        return m;
    }
    get(r, c) { return this.data[r * this.cols + c]; }
    set(r, c, v) { this.data[r * this.cols + c] = v; }
    mul(other) {
        if (typeof other === "number") {
            return new PilotMatrix(this.rows, this.cols, this.data.map(v => v * other));
        }
        const out = new PilotMatrix(this.rows, other.cols);
        for (let i = 0; i < this.rows; i++)
            for (let k = 0; k < this.cols; k++)
                for (let j = 0; j < other.cols; j++)
                    out.data[i * other.cols + j] += this.get(i, k) * other.get(k, j);
        return out;
    }
}
""",

        "lua" => """
local PilotMatrix = {}
PilotMatrix.__index = PilotMatrix
function PilotMatrix.new(r, c, fill)
    local self = setmetatable({ rows = r, cols = c, shape = {r, c}, data = {} }, PilotMatrix)
    for i = 1, r * c do self.data[i] = fill or 0.0 end
    return self
end
function PilotMatrix.zeros(r, c) return PilotMatrix.new(r, c or r, 0.0) end
function PilotMatrix.ones(r, c) return PilotMatrix.new(r, c or r, 1.0) end
function PilotMatrix.eye(n)
    local m = PilotMatrix.zeros(n, n)
    for i = 0, n - 1 do m.data[i * n + i + 1] = 1.0 end
    return m
end
function PilotMatrix.from_table(tbl)
    local r, c = #tbl, #tbl[1]
    local m = PilotMatrix.zeros(r, c)
    for i = 1, r do for j = 1, c do m.data[(i - 1) * c + j] = tbl[i][j] end end
    return m
end
""",

        "cpp" or "c++" => """
struct PilotMatrix {
    int64_t rows, cols;
    std::vector<double> data;
    PilotMatrix(int64_t r, int64_t c, double fill = 0.0) : rows(r), cols(c), data(r * c, fill) {}
    PilotMatrix(std::initializer_list<std::initializer_list<double>> init) {
        rows = init.size();
        cols = rows > 0 ? init.begin()->size() : 0;
        data.reserve(rows * cols);
        for (auto& r : init) for (double v : r) data.push_back(v);
    }
    static PilotMatrix zeros(int64_t r, int64_t c) { return PilotMatrix(r, c, 0.0); }
    static PilotMatrix ones(int64_t r, int64_t c) { return PilotMatrix(r, c, 1.0); }
    static PilotMatrix eye(int64_t n) {
        PilotMatrix m(n, n, 0.0);
        for (int64_t i = 0; i < n; ++i) m.data[i * n + i] = 1.0;
        return m;
    }
    double& operator()(int64_t r, int64_t c) { return data[r * cols + c]; }
    double operator()(int64_t r, int64_t c) const { return data[r * cols + c]; }
    PilotMatrix operator*(double s) const {
        PilotMatrix out(rows, cols);
        for (size_t i = 0; i < data.size(); ++i) out.data[i] = data[i] * s;
        return out;
    }
    PilotMatrix operator*(const PilotMatrix& o) const {
        PilotMatrix out(rows, o.cols, 0.0);
        for (int64_t i = 0; i < rows; ++i)
            for (int64_t k = 0; k < cols; ++k)
                for (int64_t j = 0; j < o.cols; ++j)
                    out(i, j) += (*this)(i, k) * o(k, j);
        return out;
    }
};
""",

        "c" => """
typedef struct {
    int64_t rows;
    int64_t cols;
    double data[64];
} PilotMatrix;

static inline PilotMatrix pilot_matrix_zeros(int64_t r, int64_t c) {
    PilotMatrix m = { r, c, {0} };
    return m;
}
static inline PilotMatrix pilot_matrix_ones(int64_t r, int64_t c) {
    PilotMatrix m = { r, c, {0} };
    for (int64_t i = 0; i < r * c; ++i) m.data[i] = 1.0;
    return m;
}
static inline PilotMatrix pilot_matrix_eye(int64_t n) {
    PilotMatrix m = pilot_matrix_zeros(n, n);
    for (int64_t i = 0; i < n; ++i) m.data[i * n + i] = 1.0;
    return m;
}
""",

        "rust" or "rs" => """
#[derive(Clone, Debug, PartialEq)]
pub struct PilotMatrix {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f64>,
}
impl PilotMatrix {
    pub fn zeros(r: usize, c: usize) -> Self { Self { rows: r, cols: c, data: vec![0.0; r * c] } }
    pub fn ones(r: usize, c: usize) -> Self { Self { rows: r, cols: c, data: vec![1.0; r * c] } }
    pub fn eye(n: usize) -> Self {
        let mut m = Self::zeros(n, n);
        for i in 0..n { m.data[i * n + i] = 1.0; }
        m
    }
    pub fn from_rows(rows_vec: &[Vec<f64>]) -> Self {
        let r = rows_vec.len();
        let c = if r > 0 { rows_vec[0].len() } else { 0 };
        let mut data = Vec::with_capacity(r * c);
        for row in rows_vec { data.extend_from_slice(row); }
        Self { rows: r, cols: c, data }
    }
}
""",

        "java" => """
final class PilotMatrix {
    public final long rows;
    public final long cols;
    public final double[] data;
    public PilotMatrix(long rows, long cols) {
        this.rows = rows;
        this.cols = cols;
        this.data = new double[(int)(rows * cols)];
    }
    public static PilotMatrix zeros(long r, long c) { return new PilotMatrix(r, c); }
    public static PilotMatrix ones(long r, long c) {
        PilotMatrix m = new PilotMatrix(r, c);
        Arrays.fill(m.data, 1.0);
        return m;
    }
    public static PilotMatrix eye(long n) {
        PilotMatrix m = new PilotMatrix(n, n);
        for (int i = 0; i < (int)n; i++) m.data[i * (int)n + i] = 1.0;
        return m;
    }
    public static PilotMatrix fromList(List<? extends List<? extends Number>> arr) {
        int r = arr.size();
        int c = r > 0 ? arr.get(0).size() : 0;
        PilotMatrix m = new PilotMatrix(r, c);
        for (int i = 0; i < r; i++)
            for (int j = 0; j < c; j++)
                m.data[i * c + j] = arr.get(i).get(j).doubleValue();
        return m;
    }
    public double get(long r, long c) { return data[(int)(r * cols + c)]; }
    public void set(long r, long c, double v) { data[(int)(r * cols + c)] = v; }
}
""",

        "ruby" or "rb" => """
class PilotMatrix
  attr_reader :rows, :cols, :shape, :data
  def initialize(rows, cols, fill = 0.0)
    @rows = rows
    @cols = cols
    @shape = [rows, cols]
    @data = Array.new(rows * cols, fill)
  end
  def self.zeros(r, c = r); PilotMatrix.new(r, c, 0.0); end
  def self.ones(r, c = r); PilotMatrix.new(r, c, 1.0); end
  def self.eye(n)
    m = PilotMatrix.zeros(n, n)
    (0...n).each { |i| m.data[i * n + i] = 1.0 }
    m
  end
  def self.from_array(arr)
    r = arr.length
    c = r > 0 ? arr[0].length : 0
    m = PilotMatrix.zeros(r, c)
    (0...r).each { |i| (0...c).each { |j| m.data[i * c + j] = arr[i][j].to_f } }
    m
  end
end
""",

        _ => string.Empty
    };
    [GeneratedRegex(@"\b(matrix|zeros|ones|eye|matrix2x2|matrix3x3|matrix4x4)\b", RegexOptions.Compiled)]
    private static partial Regex MyRegex();
    [GeneratedRegex(@"\b(json\.dumps|json\.loads|json_dumps|json_loads|\.to_json\()\b", RegexOptions.Compiled)]
    private static partial Regex MyRegex1();
    [GeneratedRegex(@"^(import\s+numpy(\s+as\s+\w+)?|from\s+numpy\s+import\s+.+|import\s+pandas(\s+as\s+\w+)?|import\s+json|import\s+math|import\s+functools)$")]
    private static partial Regex MyRegex2();
    [GeneratedRegex(@"^use\s+(ndarray|nalgebra|serde_json|rayon|std::fs)(::.+)?;?$")]
    private static partial Regex MyRegex3();
    [GeneratedRegex(@"^(const|let|var)\s+\w+\s*=\s*require\(['""](mathjs|lodash|fs|path)['""]\);?$")]
    private static partial Regex MyRegex4();
    [GeneratedRegex(@"\b(?:np|numpy|math)\.zeros\(\s*[\(\[]\s*([^,\)\]]+)\s*,\s*([^\)\]]+)\s*[\)\]]\s*(?:,\s*dtype\s*=\s*[^)]+)?\)")]
    private static partial Regex NPRegexZeroes1();
    [GeneratedRegex(@"\b(?:np|numpy|math)\.zeros\(\s*([^,)]+)\s*\)")]
    private static partial Regex NPRegexZeroes2();
    [GeneratedRegex(@"\b(?:Array2|DMatrix)::(?:<[^>]+>::)?zeros\(\s*\(\s*([^,)]+)\s*,\s*([^)]+)\s*\)\s*\)")]
    private static partial Regex NPRegexZeroes3();
    [GeneratedRegex(@"\bEigen::Matrix[A-Za-z0-9_]*::Zero\(\s*([^,)]+)\s*,\s*([^)]+)\s*\)")]
    private static partial Regex NPRegexZeroes4();
    [GeneratedRegex(@"\b(?:np|numpy|math)\.ones\(\s*[\(\[]\s*([^,\)\]]+)\s*,\s*([^\)\]]+)\s*[\)\]]\s*(?:,\s*dtype\s*=\s*[^)]+)?\)")]
    private static partial Regex NPRegexOnes1();
    [GeneratedRegex(@"\b(?:np|numpy|math)\.ones\(\s*([^,)]+)\s*\)")]
    private static partial Regex NPRegexOnes2();
    [GeneratedRegex(@"\b(?:Array2|DMatrix)::(?:<[^>]+>::)?ones\(\s*\(\s*([^,)]+)\s*,\s*([^)]+)\s*\)\s*\)")]
    private static partial Regex NPRegexOnes3();
    [GeneratedRegex(@"\bEigen::Matrix[A-Za-z0-9_]*::Ones\(\s*([^,)]+)\s*,\s*([^)]+)\s*\)")]
    private static partial Regex NPRegexOnes4();
    [GeneratedRegex(@"\b(?:np|numpy|math)\.(?:eye|identity)\(\s*([^,)]+)(?:\s*,\s*[^)]+)?\)")]
    private static partial Regex NPRegexEye1();
    [GeneratedRegex(@"\b(?:Array2|DMatrix)::(?:<[^>]+>::)?eye\(\s*([^)]+)\s*\)")]
    private static partial Regex NPRegexEye2();
    [GeneratedRegex(@"\bEigen::Matrix[A-Za-z0-9_]*::Identity\(\s*([^,)]+)\s*,\s*[^)]+\)")]
    private static partial Regex NPRegexEye3();
    [GeneratedRegex(@"\b(?:np|numpy)\.(?:array|matrix|asarray)\(\s*(\[.+\])\s*,\s*dtype\s*=\s*(?:np\.)?([A-Za-z0-9_'\""]+)\s*\)")]
    private static partial Regex NPRegexArray1();
    [GeneratedRegex(@"\b(?:np|numpy|math)\.(?:array|matrix|asarray)\(\s*(\[.+\])\s*\)")]
    private static partial Regex NPRegexMatrix1();
    [GeneratedRegex(@"\barr2\(\s*&(\[.+\])\s*\)")]
    private static partial Regex NPRegexMatrix2();
    [GeneratedRegex(@"\b(?:np|numpy|math)\.(?:dot|matmul|multiply)\(\s*([^,]+?)\s*,\s*([^)]+?)\s*\)")]
    private static partial Regex NPRegexMatrix3();
    [GeneratedRegex(@"\b([A-Za-z_][A-Za-z0-9_]*)\.dot\(\s*&?([A-Za-z_][A-Za-z0-9_]*)\s*\)")]
    private static partial Regex NPRegexMatrix4();
    [GeneratedRegex(@"\b([A-Za-z_][A-Za-z0-9_]*)\s+@\s+([A-Za-z_][A-Za-z0-9_]*)\b")]
    private static partial Regex NPRegexMatrix5();
    [GeneratedRegex(@"\b(?:np|numpy)\.arange\(")]
    private static partial Regex NPRegexRange1();
    [GeneratedRegex(@"\.dtype\b")]
    private static partial Regex NPRegexMatrix6();
    [GeneratedRegex(@"\b(?:np|numpy)\.vectorize\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)")]
    private static partial Regex NPRegexVector1();
    [GeneratedRegex(@"\b([A-Za-z_][A-Za-z0-9_]*)\[\s*\1\[(['""][^'""]+['""])\]\s*([><=!]+)\s*([^\]]+)\]")]
    private static partial Regex PandasRegexDF1();
    [GeneratedRegex(@"\blist\(\s*map\(\s*([^,]+?)\s*,\s*([^)]+?)\s*\)\s*\)")]
    private static partial Regex PyRegexMap1();
    [GeneratedRegex(@"\blist\(\s*filter\(\s*([^,]+?)\s*,\s*([^)]+?)\s*\)\s*\)")]
    private static partial Regex PyRegexMap2();
    [GeneratedRegex(@"\b_\.map\(\s*([^,]+?)\s*,\s*([^)]+?)\s*\)")]
    private static partial Regex PyRegexMap3();
    [GeneratedRegex(@"\b_\.filter\(\s*([^,]+?)\s*,\s*([^)]+?)\s*\)")]
    private static partial Regex PyRegexMap4();
    [GeneratedRegex(@"\bfunctools\.reduce\(\s*([^,]+?)\s*,\s*([^,)]+?)(?:\s*,\s*([^)]+?))?\s*\)")]
    private static partial Regex PyRegexMap5();
    [GeneratedRegex(@"\b([A-Za-z_][A-Za-z0-9_]*)\.(?:par_iter|into_par_iter)\(\)\.map\(\s*([^)]+)\s*\)\.collect(?::<[^>]+>)?\(\)")]
    private static partial Regex RustRegexMap1();
    [GeneratedRegex(@"\b([A-Za-z_][A-Za-z0-9_]*)\.(?:iter|into_iter)\(\)\.map\(\s*([^)]+)\s*\)\.collect(?::<[^>]+>)?\(\)")]
    private static partial Regex RustRegexMap2();
    [GeneratedRegex(@"\b([A-Za-z_][A-Za-z0-9_]*)\.(?:iter|into_iter)\(\)\.filter\(\s*([^)]+)\s*\)\.collect(?::<[^>]+>)?\(\)")]
    private static partial Regex RustRegexMap3();
    [GeneratedRegex(@"\b(?:JSON\.stringify|serde_json::to_string|cjson\.encode)\(\s*&?([^)]+)\s*\)(?:\.unwrap\(\))?")]
    private static partial Regex JsonRegexString1();
    [GeneratedRegex(@"\b(?:JSON\.parse|serde_json::from_str(?::<[^>]+>)?|cjson\.decode)\(\s*&?([^)]+)\s*\)(?:\.unwrap\(\))?")]
    private static partial Regex JsonRegexString2();
    [GeneratedRegex(@"\bfs\.readFileSync\(\s*([^,)]+)(?:\s*,\s*['""]utf-?8['""])?\s*\)")]
    private static partial Regex FileIORegex1();
    [GeneratedRegex(@"\bfs\.writeFileSync\(\s*([^,]+)\s*,\s*([^)]+)\s*\)")]
    private static partial Regex FileIORegex2();
    [GeneratedRegex(@"\bfs\.existsSync\(\s*([^)]+)\s*\)")]
    private static partial Regex FileIORegex3();
    [GeneratedRegex(@"\bstd::fs::read_to_string\(\s*&?([^)]+)\s*\)(?:\.unwrap\(\)|\?)?")]
    private static partial Regex FileIORegex4();
    [GeneratedRegex(@"\bstd::fs::write\(\s*&?([^,]+)\s*,\s*&?([^)]+)\s*\)(?:\.unwrap\(\)|\?)?")]
    private static partial Regex FileIORegex5();
    [GeneratedRegex(@"\bos\.path\.exists\(\s*([^)]+)\s*\)")]
    private static partial Regex FileIORegex6();
    [GeneratedRegex(@"\b(?:tokio::spawn|asyncio\.create_task|std::async)\(\s*([^)]+)\s*\)")]
    private static partial Regex ConcurrencyRegex1();
    [GeneratedRegex(@"^require(?:\(|\s+)['""](json|matrix|set|fileutils|pathname)['""]\)?$")]
    private static partial Regex RubyRequireStdRegex();
    [GeneratedRegex(@"\bPilotMatrix\.zeros\(\s*([^,)]+)\s*,\s*([^)]+)\s*\)")]
    private static partial Regex PilotMatrixZerosCallRegex();
    [GeneratedRegex(@"\bPilotMatrix\.ones\(\s*([^,)]+)\s*,\s*([^)]+)\s*\)")]
    private static partial Regex PilotMatrixOnesCallRegex();
    [GeneratedRegex(@"\bPilotMatrix\.eye\(\s*([^)]+)\s*\)")]
    private static partial Regex PilotMatrixEyeCallRegex();
    [GeneratedRegex(@"\bPilotMatrix\.(?:fromList|from_array)\(\s*(.+)\s*\)")]
    private static partial Regex PilotMatrixFromCallRegex();
    [GeneratedRegex(@"\bJSON\.generate\(\s*([^)]+)\s*\)")]
    private static partial Regex RubyJsonGenRegex();
    [GeneratedRegex(@"\bFiles\.readString\(\s*Path\.of\(\s*([^)]+)\s*\)\s*\)")]
    private static partial Regex JavaFilesReadRegex();
    [GeneratedRegex(@"\bFiles\.writeString\(\s*Path\.of\(\s*([^)]+)\s*\)\s*,\s*([^)]+)\s*\)")]
    private static partial Regex JavaFilesWriteRegex();
    [GeneratedRegex(@"\bFiles\.exists\(\s*Path\.of\(\s*([^)]+)\s*\)\s*\)")]
    private static partial Regex JavaFilesExistsRegex();
    [GeneratedRegex(@"\bFile\.read\(\s*([^,)]+)(?:\s*,\s*encoding:\s*['""][^'""]+['""])?\s*\)")]
    private static partial Regex RubyFileReadRegex();
    [GeneratedRegex(@"\bFile\.write\(\s*([^,]+)\s*,\s*([^)]+)\s*\)")]
    private static partial Regex RubyFileWriteRegex();
    [GeneratedRegex(@"\bFile\.exist\?\(\s*([^)]+)\s*\)")]
    private static partial Regex RubyFileExistRegex();
    [GeneratedRegex(@"\bMath\.(sqrt|sin|cos|abs|floor|ceil)\(\s*([^)]+)\s*\)")]
    private static partial Regex JavaRubyMathFnRegex();

}
