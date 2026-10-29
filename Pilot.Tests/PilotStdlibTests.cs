using System;
using System.IO;
using System.Linq;
using Antlr4.Runtime;
using Pilot.Analysis.PatternMatching;
using Pilot.Analysis.TypeSystem;
using Pilot.Compiler;
using Pilot.LanguageServer.Semantics;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Stdlib;
using Pilot.Runtime.Values;
using Pilot.Transpilation;
using Pilot.Transpilation.Licensing;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Milestone 21: Comprehensive verification suite for the Foundational Native Pilot Standard Library (<c>stdlib/</c>),
/// exercising both AST Interpreter (<see cref="PilotEngine"/>) and PilVM Bytecode / <c>.pilc</c> modes across
/// sub-tasks 21.1 through 21.5.
/// </summary>
public class PilotStdlibTests
{
    private static PilotExecutionResult ExecuteCode(string source, bool useBytecode, string? sourceFilePath = null)
    {
        if (useBytecode)
        {
            var compileOpts = new PilotCompilerOptions
            {
                EnableCaching = false,
                SourceFilePath = sourceFilePath
            };
            var comp = PilotCompiler.Default.Compile(source, compileOpts);
            Assert.True(comp.Success, $"Compilation failed: {string.Join("; ", comp.Diagnostics)}");
            Assert.NotNull(comp.Program);
            var prog = Assert.IsType<PilotCompiledProgram>(comp.Program);
            Assert.NotNull(prog.BytecodeChunk);
            return prog.Run();
        }
        else
        {
            var engine = new PilotEngine();
            var execOpts = new PilotExecutionOptions
            {
                SourceFilePath = sourceFilePath
            };
            return engine.Execute(source, execOpts);
        }
    }

    // =========================================================================
    // 21.1: `std.*` Resolver, `.pilc` Cache, Relative Imports, LSP & M20 License Gate
    // =========================================================================

    [Fact]
    public void Stdlib_All13Modules_PassStaticCheckAndCompileWithZeroErrors()
    {
        var known = PilotStdlibResolver.KnownStdlibModuleNames.ToList();
        Assert.Equal(15, known.Count);

        foreach (string modName in known)
        {
            Assert.True(
                PilotStdlibResolver.TryResolveModulePath(modName, 0, null, out string? fullPath, out string canonicalId),
                $"Failed to resolve '{modName}'");
            Assert.Equal(modName, canonicalId);
            Assert.NotNull(fullPath);
            Assert.True(File.Exists(fullPath!), $"Missing stdlib file on disk: {fullPath}");

            string source = File.ReadAllText(fullPath!);
            var lexer = new PilotLexer(new AntlrInputStream(source));
            var parser = new PilotParser(new CommonTokenStream(lexer));
            var tree = parser.ParseProgramFast();

            var typeEngine = new PilotTypeInferenceEngine();
            typeEngine.VisitProgram(tree);
            Assert.True(
                !typeEngine.Diagnostics.Any(d => d.IsError),
                $"Module '{modName}' type errors: {string.Join("; ", typeEngine.Diagnostics.Where(d => d.IsError))}");

            var matchDiags = PilotExhaustivenessChecker.CheckProgram(tree, typeEngine);
            Assert.True(
                !matchDiags.Any(d => d.IsError),
                $"Module '{modName}' match errors: {string.Join("; ", matchDiags.Where(d => d.IsError))}");

            var errAnalyzer = new PilotErrorFlowAnalyzer();
            errAnalyzer.CheckProgram(tree);
            Assert.True(
                !errAnalyzer.Diagnostics.Any(d => d.IsError),
                $"Module '{modName}' error-flow errors: {string.Join("; ", errAnalyzer.Diagnostics.Where(d => d.IsError))}");

            var comp = PilotCompiler.Default.CompileFile(fullPath!, new PilotCompilerOptions { EnableCaching = false });
            Assert.True(comp.Success, $"Module '{modName}' failed compilation: {string.Join("; ", comp.Diagnostics)}");
        }
    }

    [Fact]
    public void Stdlib_PrecompileCache_PersistsAndReloadsFromDiskPilc()
    {
        var artifacts = PilotStdlibResolver.PrecompileAllStdlibModules();
        Assert.Equal(15, artifacts.Count);

        foreach (var art in artifacts)
        {
            Assert.True(File.Exists(art.PilcPath), $"Expected cached .pilc file at '{art.PilcPath}'");
            Assert.NotNull(art.CompiledProgram.BytecodeChunk);
        }

        // Evict in-memory cache and verify subsequent std.* imports reload directly from .pilc disk cache
        PilotStdlibResolver.ClearMemoryCacheForTesting();
        PilotStdlibResolver.ResetCacheCountersForTesting();

        var res = ExecuteCode(@"
            import std.math;
            import std.collections;
            var dq = collections.Deque();
            dq.push_back(42);
            return dq.pop_front();
        ", useBytecode: true);

        Assert.True(res.Success, res.ErrorMessage);
        Assert.Equal(42L, res.Value.UnwrapOption().AsInt());
        Assert.True(PilotStdlibResolver.DiskCacheHits >= 2, $"Expected >= 2 .pilc disk cache hits, got {PilotStdlibResolver.DiskCacheHits}");
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void Stdlib_ImportForms_UmbrellaAliasFromAndStar_Parity(bool useBytecode)
    {
        string code = @"
            import std;
            import std.math as m;
            from std.collections import Deque, PriorityQueue as PQ;
            from std import option, result as res;
            from std.core import *;

            var dq = Deque();
            dq.push_back(10);
            dq.push_front(5);

            var pq = PQ();
            pq.push(20, ""low"");
            pq.push(1, ""high"");

            let ord = compare(3, 9);
            let clamped = clamp(150, 0, 100);
            let opt_v = option.unwrap_or(some(7), 0);
            let res_v = res.unwrap_or(ok(9), 0);
            let trig_ok = m.approx_eq(m.sin(0.0) + std.math.cos(0.0), 1.0);

            return (
                option.unwrap(dq.pop_front()),
                option.unwrap(pq.pop()),
                is_less(ord),
                clamped,
                opt_v + res_v,
                trig_ok
            );
        ";

        var exec = ExecuteCode(code, useBytecode);
        Assert.True(exec.Success, exec.ErrorMessage);
        var tup = exec.Value.AsTuple();
        Assert.Equal(5L, tup[0].AsInt());
        Assert.Equal("high", tup[1].AsString());
        Assert.True(tup[2].IsTrue);
        Assert.Equal(100L, tup[3].AsInt());
        Assert.Equal(16L, tup[4].AsInt());
        Assert.True(tup[5].IsTrue);
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void Stdlib_RelativeImports_And_CycleAndMissingDiagnostics(bool useBytecode)
    {
        string tempDir = Path.Combine(Path.GetTempPath(), $"pilot_m21_rel_{Guid.NewGuid():N}");
        Directory.CreateDirectory(tempDir);
        try
        {
            string subPath = Path.Combine(tempDir, "sub.pilot");
            string helperPath = Path.Combine(tempDir, "helper.pilot");
            string mainPath = Path.Combine(tempDir, "main.pilot");

            File.WriteAllText(subPath, "const FACTOR: i64 = 6;\n");
            File.WriteAllText(helperPath, @"
                from .sub import FACTOR;
                def scale(x: i64) -> i64 {
                    return x * FACTOR;
                }
            ");
            File.WriteAllText(mainPath, @"
                from .helper import scale;
                return scale(7);
            ");

            var okRes = ExecuteCode(File.ReadAllText(mainPath), useBytecode, mainPath);
            Assert.True(okRes.Success, okRes.ErrorMessage);
            Assert.Equal(42L, okRes.Value.AsInt());

            // [IMP001] Missing module
            var missingMod = ExecuteCode("import std.nonexistent_mod;", useBytecode);
            Assert.False(missingMod.Success);
            Assert.Contains("IMP001", missingMod.ErrorMessage);

            // [IMP003] Missing symbol
            var missingSym = ExecuteCode("from std.math import definitely_not_a_symbol;", useBytecode);
            Assert.False(missingSym.Success);
            Assert.Contains("IMP003", missingSym.ErrorMessage);

            // [IMP002] Circular module import
            string cycAPath = Path.Combine(tempDir, "cyc_a.pilot");
            string cycBPath = Path.Combine(tempDir, "cyc_b.pilot");
            File.WriteAllText(cycAPath, "from .cyc_b import b_fn;\ndef a_fn() -> i64 { return 1; }\n");
            File.WriteAllText(cycBPath, "from .cyc_a import a_fn;\ndef b_fn() -> i64 { return 2; }\n");

            var cycRes = ExecuteCode(File.ReadAllText(cycAPath), useBytecode, cycAPath);
            Assert.False(cycRes.Success);
            Assert.Contains("IMP002", cycRes.ErrorMessage);
        }
        finally
        {
            if (Directory.Exists(tempDir))
                Directory.Delete(tempDir, recursive: true);
        }
    }

    [Fact]
    public void Stdlib_LibraryMapper_M20ViableLicenseProvenance_And_LspSymbols()
    {
        // 1. Verify stdlib/pilot.json & stdlib/THIRD_PARTY_NOTICES.md provenance manifests exist
        string stdRoot = PilotStdlibResolver.GetStdlibRootDirectory();
        string manifestPath = Path.Combine(stdRoot, "pilot.json");
        string noticesPath = Path.Combine(stdRoot, "THIRD_PARTY_NOTICES.md");
        Assert.True(File.Exists(manifestPath), $"Missing {manifestPath}");
        Assert.True(File.Exists(noticesPath), $"Missing {noticesPath}");
        Assert.Contains("PSF-2.0", File.ReadAllText(noticesPath));
        Assert.Contains("MIT", File.ReadAllText(manifestPath));

        // 2. Verify bidirectional PilotLibraryMapper std.* mappings
        Assert.Contains("deque", PilotLibraryMapper.MapPilotStdModuleToTarget("std.collections", "python"));
        Assert.Contains("VecDeque", PilotLibraryMapper.MapPilotStdModuleToTarget("std.collections", "rust"));
        Assert.Contains("math.h", PilotLibraryMapper.MapPilotStdModuleToTarget("std.math", "c"));

        Assert.True(PilotLibraryMapper.TryMapForeignStdlibImportToPilot("from pathlib import Path", "python", out string? mappedPath));
        Assert.Equal("import std.path;", mappedPath);
        Assert.True(PilotLibraryMapper.TryMapForeignStdlibImportToPilot("import itertools", "python", out string? mappedIter));
        Assert.Equal("import std.itertools;", mappedIter);
        Assert.Contains("base64", PilotLibraryMapper.MapPilotStdModuleToTarget("std.encoding", "python"));
        Assert.Contains("Base64", PilotLibraryMapper.MapPilotStdModuleToTarget("std.encoding", "java"));
        Assert.Contains("tomllib", PilotLibraryMapper.MapPilotStdModuleToTarget("std.formats", "python"));
        Assert.Contains("csv", PilotLibraryMapper.MapPilotStdModuleToTarget("std.formats", "ruby"));
        Assert.True(PilotLibraryMapper.TryMapForeignStdlibImportToPilot("import base64", "python", out string? mappedEnc));
        Assert.Equal("import std.encoding;", mappedEnc);
        Assert.True(PilotLibraryMapper.TryMapForeignStdlibImportToPilot("import tomllib", "python", out string? mappedFmt));
        Assert.Equal("import std.formats;", mappedFmt);

        // 3. Verify M20 --strict-pilot --viable-license-only accepts PSF-2.0/MIT and rejects GPL-3.0
        var viableOpts = new TranspilationOptions().WithStrictPilot().WithViableLicenseOnly();
        var okIngest = TranspilationRegistry.Default.Ingest(
            "# SPDX-License-Identifier: PSF-2.0\ndef clamp_int(x: int, lo: int, hi: int) -> int:\n    return lo if x < lo else (hi if x > hi else x)\n",
            "python",
            viableOpts);
        Assert.True(okIngest.Success, string.Join("; ", okIngest.Diagnostics));

        var gplIngest = TranspilationRegistry.Default.Ingest(
            "# SPDX-License-Identifier: GPL-3.0-only\ndef copyleft_fn(x: int) -> int:\n    return x + 1\n",
            "python",
            viableOpts);
        Assert.False(gplIngest.Success);
        Assert.Contains("LIC", string.Join(" ", gplIngest.Diagnostics));

        // 4. Verify LSP PilotSemanticAnalyzer registers symbols for std.* imports
        var semDoc = PilotSemanticAnalyzer.Analyze(
            "import std.collections as col;\nfrom std.io import BufferedReader, open_reader;\n");
        var colSym = semDoc.RootScope.Resolve("col");
        var readerSym = semDoc.RootScope.Resolve("BufferedReader");
        var openSym = semDoc.RootScope.Resolve("open_reader");
        Assert.NotNull(colSym);
        Assert.Equal(PilotSymbolKind.Module, colSym!.Kind);
        Assert.NotNull(readerSym);
        Assert.Equal(PilotSymbolKind.Class, readerSym!.Kind);
        Assert.NotNull(openSym);
        Assert.Equal(PilotSymbolKind.Function, openSym!.Kind);
    }

    // =========================================================================
    // 21.2: `std.core`, `std.option`, `std.result`, `std.mem`, `std.meta`
    // =========================================================================

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void StdCore_Option_And_Result_Combinators_Parity(bool useBytecode)
    {
        string code = @"
            import std.core;
            import std.option;
            import std.result;

            let ord_label = match core.compare(42, 10) {
                case Ordering.Less => ""lt"",
                case Ordering.Equal => ""eq"",
                case Ordering.Greater => ""gt""
            };

            let mapped_some = option.map(some(21), (x) => x * 2);
            let mapped_none = option.map(none, (x) => x * 2);
            let filtered = option.filter(some(15), (x) => x > 10);
            let zipped = option.zip(some(""a""), some(1));
            let nested_none = some(none);
            let flat_inner = option.flatten(nested_none);

            def try_chain(opt_in) -> i64 ! str {
                let v = option.ok_or(opt_in, ""missing"")?;
                return ok(v + 8);
            }

            let ok_chain = try_chain(some(34));
            let err_chain = try_chain(none);
            let remapped_err = result.map_err(err_chain, (e) => f""err:{e}"");
            let collected_ok = result.collect_ok([ok(1), ok(2), ok(3)]);
            let collected_fail = result.collect_ok([ok(1), fail(""boom""), ok(3)]);

            return (
                ord_label,
                option.unwrap(mapped_some),
                option.is_none(mapped_none),
                option.unwrap(filtered),
                option.unwrap(zipped),
                option.is_none(flat_inner),
                result.unwrap(ok_chain),
                result.unwrap_err(remapped_err),
                result.unwrap(collected_ok),
                result.unwrap_err(collected_fail)
            );
        ";

        var exec = ExecuteCode(code, useBytecode);
        Assert.True(exec.Success, exec.ErrorMessage);
        var t = exec.Value.AsTuple();
        Assert.Equal("gt", t[0].AsString());
        Assert.Equal(42L, t[1].AsInt());
        Assert.True(t[2].IsTrue);
        Assert.Equal(15L, t[3].AsInt());
        Assert.Equal("a", t[4].AsTuple()[0].AsString());
        Assert.True(t[5].IsTrue);
        Assert.Equal(42L, t[6].AsInt());
        Assert.Equal("err:missing", t[7].AsString());
        Assert.Equal(3, t[8].AsList().Count);
        Assert.Equal("boom", t[9].AsString());
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void StdMem_ZeroCopySliceBuffer_RangeWindow_And_MetaReflection_Parity(bool useBytecode)
    {
        string code = @"
            import std.mem;
            import std.meta;
            import std.option;

            // 1. Zero-copy SliceBuffer<f64> subview write-through
            var buf = mem.alloc_slice(8, ""f64"", 0);
            mem.slice_push(buf, 10.0);
            mem.slice_push(buf, 20.0);
            mem.slice_push(buf, 30.0);
            mem.slice_push(buf, 40.0);

            var sub = mem.slice_subview(buf, 1, 2);
            mem.slice_set(sub, 0, 99.5); // mutates buf[1] in-place with zero copy!
            let ptr_delta = mem.slice_ptr(sub) - mem.slice_ptr(buf);

            // 2. RangeWindow partitioning
            let win = mem.RangeWindow(0, 10);
            let halves = win.split_mid();
            let chunks = win.split_chunks(4);

            // 3. Frozen perfect-hash map
            let fmap = mem.freeze({ ""alpha"": 1, ""beta"": 2, ""gamma"": 3, ""delta"": 4 });

            // 4. Homoiconic CallExpr (`!`), AstExpr (`!!`), & Keyword DAG reflection
            def poly(x: f64) -> f64 {
                return x * x * x + 2.0 * x;
            }

            let qcall = poly!(3.0);
            let qast = poly!!(x);
            let d_ast = meta.diff_ast(qast, ""x"");
            let d_at_3 = meta.eval_ast(d_ast, { ""x"": 3.0 }); // 3*(3^2) + 2 = 29.0

            let k_entity = meta.kw_intern(""entity"", ""ecs"");
            let k_agent = meta.kw_intern(""agent"", ""ecs"");
            let k_boss = meta.kw_intern(""boss"", ""ecs"");
            meta.kw_derive(k_agent, k_entity);
            meta.kw_derive(k_boss, k_agent);

            return (
                mem.slice_get(buf, 1),
                ptr_delta,
                halves[0].len(),
                halves[1].len(),
                len(chunks),
                mem.is_perfect_hash(fmap),
                meta.call_name(qcall),
                meta.invoke_call(qcall),
                d_at_3,
                meta.kw_isa(k_boss, k_entity),
                meta.kw_distance(k_boss, k_entity)
            );
        ";

        var exec = ExecuteCode(code, useBytecode);
        Assert.True(exec.Success, exec.ErrorMessage);
        var t = exec.Value.AsTuple();
        Assert.Equal(99.5, t[0].AsFloat(), 6);
        Assert.Equal(8L, t[1].AsInt()); // 1 * sizeof(f64) == 8 bytes!
        Assert.Equal(5L, t[2].AsInt());
        Assert.Equal(5L, t[3].AsInt());
        Assert.Equal(3L, t[4].AsInt());
        Assert.True(t[5].IsTrue);
        Assert.Equal("poly", t[6].AsString());
        Assert.Equal(33.0, t[7].AsFloat(), 6);
        Assert.Equal(29.0, t[8].AsFloat(), 6);
        Assert.True(t[9].IsTrue);
        Assert.Equal(2L, t[10].AsInt());
    }

    // =========================================================================
    // 21.3: `std.collections` & `std.itertools`
    // =========================================================================

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void StdCollections_AllDataStructures_And_Bisect_Parity(bool useBytecode)
    {
        string code = @"
            import std.collections as col;
            import std.option;

            // 1. Deque wrap-around & growth
            var dq = col.Deque(4);
            dq.push_back(2);
            dq.push_back(3);
            dq.push_front(1);
            dq.push_front(0);
            dq.push_back(4); // triggers _grow across wrapped head!
            let dq_list = dq.to_list();

            // 2. PriorityQueue min-heap + FIFO tie-breaking
            var pq = col.PriorityQueue();
            pq.push(10, ""second_ten"");
            pq.push(5, ""five"");
            pq.push(10, ""third_ten"");
            let p0 = option.unwrap(pq.pop());
            let p1 = option.unwrap(pq.pop());

            // 3. OrderedMap sorted keys & range queries
            var omap = col.OrderedMap();
            omap.put(30, ""thirty"");
            omap.put(10, ""ten"");
            omap.put(20, ""twenty"");
            omap.put(40, ""forty"");
            let okeys = omap.keys();
            let orange = omap.range_entries(15, 35);

            // 4. MultiMap
            var mm = col.MultiMap();
            mm.put(""tag"", ""fast"");
            mm.put(""tag"", ""safe"");

            // 5. RingBuffer bounded push & eviction
            var rb = col.RingBuffer(2);
            let pushed1 = rb.try_push(100);
            let pushed2 = rb.try_push(200);
            let pushed3 = rb.try_push(300); // false (full)
            let evicted = option.unwrap(rb.push_overwrite(300)); // evicts 100

            // 6. Multi-word BitSet (> 62 bits)
            var b1 = col.BitSet(160);
            b1.set(0);
            b1.set(62);
            b1.set(130);
            var b2 = col.BitSet(160);
            b2.set(62);
            b2.set(140);
            let inter = b1.intersect_with(b2);
            let uni = b1.union_with(b2);

            // 7. FrozenMap & bisect
            let fm = col.FrozenMap({ ""k1"": 111, ""k2"": 222 });
            let bl = col.bisect_left([10, 20, 20, 30], 20);
            let br = col.bisect_right([10, 20, 20, 30], 20);

            return (
                dq_list,
                p0,
                p1,
                okeys,
                len(orange),
                len(mm.get_all(""tag"")),
                pushed3,
                evicted,
                inter.count_ones(),
                uni.count_ones(),
                fm.is_perfect_hash(),
                fm.get_or(""k2"", 0),
                bl,
                br
            );
        ";

        var exec = ExecuteCode(code, useBytecode);
        Assert.True(exec.Success, exec.ErrorMessage);
        var t = exec.Value.AsTuple();
        Assert.Equal(new long[] { 0, 1, 2, 3, 4 }, t[0].AsList().Select(v => v.AsInt()).ToArray());
        Assert.Equal("five", t[1].AsString());
        Assert.Equal("second_ten", t[2].AsString());
        Assert.Equal(new long[] { 10, 20, 30, 40 }, t[3].AsList().Select(v => v.AsInt()).ToArray());
        Assert.Equal(2L, t[4].AsInt());
        Assert.Equal(2L, t[5].AsInt());
        Assert.False(t[6].IsTrue);
        Assert.Equal(100L, t[7].AsInt());
        Assert.Equal(1L, t[8].AsInt());
        Assert.Equal(4L, t[9].AsInt());
        Assert.True(t[10].IsTrue);
        Assert.Equal(222L, t[11].AsInt());
        Assert.Equal(1L, t[12].AsInt());
        Assert.Equal(3L, t[13].AsInt());
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void StdItertools_EagerAndLazyGeneratorCombinators_Parity(bool useBytecode)
    {
        string code = @"
            import std.itertools as it;

            let chunks = it.chunk([1, 2, 3, 4, 5], 2);
            let wins = it.window([10, 20, 30, 40], 3, 1);
            let uniq = it.unique([3, 1, 3, 2, 1, 4]);
            let parts = it.partition([1, 2, 3, 4, 5, 6], (x) => (x % 2) == 0);
            let freqs = it.frequencies([""a"", ""b"", ""a"", ""c"", ""a""]);
            let scanned = it.scan([1, 2, 3, 4], 0, (acc, x) => acc + x);

            // Lazy yield pipeline: lazy_map -> lazy_filter -> lazy_take
            let mapped_gen = it.lazy_map(1..100, (x) => x * x);
            let filtered_gen = it.lazy_filter(mapped_gen, (sq) => (sq % 2) == 1);
            let top3 = [v for v in it.lazy_take(filtered_gen, 3)];

            return (
                len(chunks),
                len(wins),
                uniq,
                parts[0],
                freqs[""a""],
                scanned,
                top3
            );
        ";

        var exec = ExecuteCode(code, useBytecode);
        Assert.True(exec.Success, exec.ErrorMessage);
        var t = exec.Value.AsTuple();
        Assert.Equal(3L, t[0].AsInt());
        Assert.Equal(2L, t[1].AsInt());
        Assert.Equal(new long[] { 3, 1, 2, 4 }, t[2].AsList().Select(v => v.AsInt()).ToArray());
        Assert.Equal(new long[] { 2, 4, 6 }, t[3].AsList().Select(v => v.AsInt()).ToArray());
        Assert.Equal(3L, t[4].AsInt());
        Assert.Equal(new long[] { 1, 3, 6, 10 }, t[5].AsList().Select(v => v.AsInt()).ToArray());
        Assert.Equal(new long[] { 1, 9, 25 }, t[6].AsList().Select(v => v.AsInt()).ToArray());
    }

    // =========================================================================
    // 21.4: `std.text`, `std.math`, `std.random`
    // =========================================================================

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void StdText_ZeroCopyUtf8StrSlice_And_ComptimeDfaRegex_Parity(bool useBytecode)
    {
        string code = @"
            import std.text;
            import std.option;

            // ""αβγ🚀"" = 2 + 2 + 2 + 4 = 10 UTF-8 bytes, 4 Unicode scalar values
            let ss = text.str_slice(""αβγ🚀"");
            let sub_ok = option.unwrap(ss.subslice(2, 4)); // ""βγ"" (zero-copy pointer + 2!)
            let sub_bad = ss.subslice(1, 2); // byte 1 is a UTF-8 continuation byte -> none!

            // Comptime Thompson NFA -> Powerset DFA frozen into .rodata!
            const EMAIL_DFA = comptime(compile_regex(""^[a-z0-9._%+-]+@[a-z0-9.-]+\\.[a-z]{2,}$""));
            let re = text.Regex(EMAIL_DFA);
            let m_ok = re.is_match(""pilot.dev+21@deepmind.org"");
            let m_bad = re.is_match(""invalid@@domain"");

            let num_re = text.Regex(""[0-9]+"");
            let all_nums = num_re.find_all(""m20 -> m21 -> 100%"");
            let scrubbed = num_re.replace_all(""id=42,port=8080"", ""#"");

            return (
                ss.byte_len(),
                ss.rune_count(),
                sub_ok.to_str(),
                sub_ok.data_ptr() - ss.data_ptr(),
                option.is_none(sub_bad),
                EMAIL_DFA.is_dfa,
                m_ok,
                m_bad,
                all_nums,
                scrubbed
            );
        ";

        var exec = ExecuteCode(code, useBytecode);
        Assert.True(exec.Success, exec.ErrorMessage);
        var t = exec.Value.AsTuple();
        Assert.Equal(10L, t[0].AsInt());
        Assert.Equal(4L, t[1].AsInt());
        Assert.Equal("βγ", t[2].AsString());
        Assert.Equal(2L, t[3].AsInt());
        Assert.True(t[4].IsTrue);
        Assert.True(t[5].IsTrue);
        Assert.True(t[6].IsTrue);
        Assert.False(t[7].IsTrue);
        Assert.Equal(new[] { "20", "21", "100" }, t[8].AsList().Select(v => v.AsString()).ToArray());
        Assert.Equal("id=#,port=#", t[9].AsString());
    }

    [Fact]
    public void StdText_ComptimeCompiledRegex_SurvivesPilcBinarySerialization()
    {
        string code = @"
            import std.text;
            const HEX_DFA = comptime(compile_regex(""^0x[0-9a-fA-F]+$""));
            let r = text.Regex(HEX_DFA);
            return (r.is_match(""0xCAFE21""), r.is_match(""0xGHIJ""), r.state_count() > 0);
        ";

        var comp = PilotCompiler.Default.Compile(code, new PilotCompilerOptions { EnableCaching = false });
        Assert.True(comp.Success, string.Join("; ", comp.Diagnostics));

        var origProg = Assert.IsType<PilotCompiledProgram>(comp.Program);
        byte[] pilcBytes = origProg.ToBytes();
        var reloaded = PilotCompiledProgram.FromBytes(pilcBytes);
        var runRes = reloaded.Run();

        Assert.True(runRes.Success, runRes.ErrorMessage);
        var tup = runRes.Value.AsTuple();
        Assert.True(tup[0].IsTrue);
        Assert.False(tup[1].IsTrue);
        Assert.True(tup[2].IsTrue);
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void StdMath_WelfordStats_NumberTheory_And_SymbolicAutodiff_Parity(bool useBytecode)
    {
        string code = @"
            import std.math;
            import std.option;

            def wave_energy(x: f64) -> f64 {
                return math.sin(x) * math.exp(x);
            }

            // Symbolic adjoint `fn!@`: d/dx [sin(x)*exp(x)] = (cos(x) + sin(x)) * exp(x)
            // At x = 0.0: value = 0.0, derivative = (1 + 0) * 1 = 1.0!
            let adj = wave_energy!@(0.0);

            def act(x: f64) -> f64 {
                return math.sigmoid(x);
            }
            // At x = 0.0: sigmoid(0) = 0.5, d/dx sigmoid(0) = 0.5 * (1 - 0.5) = 0.25!
            let sig_adj = act!@(0.0);

            let data = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
            let mu = option.unwrap(math.mean(data));       // 5.0
            let sd = option.unwrap(math.stdev(data));      // 2.0
            let med = option.unwrap(math.median(data));    // 4.5
            let corr = option.unwrap(math.correlation([1, 2, 3, 4], [2, 4, 6, 8])); // 1.0

            return (
                adj.value,
                adj.d_x,
                sig_adj.value,
                sig_adj.d_x,
                mu,
                sd,
                med,
                corr,
                math.gcd(48, 18),
                math.binomial(10, 3),
                math.is_prime(97)
            );
        ";

        var exec = ExecuteCode(code, useBytecode);
        Assert.True(exec.Success, exec.ErrorMessage);
        var t = exec.Value.AsTuple();
        Assert.Equal(0.0, t[0].AsFloat(), 9);
        Assert.Equal(1.0, t[1].AsFloat(), 9);
        Assert.Equal(0.5, t[2].AsFloat(), 9);
        Assert.Equal(0.25, t[3].AsFloat(), 9);
        Assert.Equal(5.0, t[4].AsFloat(), 9);
        Assert.Equal(2.0, t[5].AsFloat(), 9);
        Assert.Equal(4.5, t[6].AsFloat(), 9);
        Assert.Equal(1.0, t[7].AsFloat(), 9);
        Assert.Equal(6L, t[8].AsInt());
        Assert.Equal(120L, t[9].AsInt());
        Assert.True(t[10].IsTrue);
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void StdRandom_SplitMix64_And_PcgRng_DeterministicReplay_Parity(bool useBytecode)
    {
        string code = @"
            import std.random;

            var sm_a = random.SplitMix64(20261029);
            var sm_b = random.SplitMix64(20261029);
            let sm_same = (sm_a.next_bits64() == sm_b.next_bits64()) and (sm_a.next_bits64() == sm_b.next_bits64());

            var pcg1 = random.PcgRng(1337, 7);
            var pcg2 = random.PcgRng(1337, 7);
            let seq1 = [pcg1.next_int(10, 50) for i in 0..<8];
            let seq2 = [pcg2.next_int(10, 50) for i in 0..<8];

            var in_bounds: bool = true;
            for v in seq1 {
                if v < 10 or v >= 50 {
                    in_bounds = false;
                }
            }

            let shuf = pcg1.shuffle([1, 2, 3, 4, 5, 6]);
            let g = pcg1.next_gaussian(0.0, 1.0);

            return (sm_same, seq1 == seq2, in_bounds, len(shuf), isfinite(g));
        ";

        var exec = ExecuteCode(code, useBytecode);
        Assert.True(exec.Success, exec.ErrorMessage);
        var t = exec.Value.AsTuple();
        Assert.True(t[0].IsTrue);
        Assert.True(t[1].IsTrue);
        Assert.True(t[2].IsTrue);
        Assert.Equal(6L, t[3].AsInt());
        Assert.True(t[4].IsTrue);
    }

    // =========================================================================
    // 21.5: `std.path`, `std.io`, `std.fs` & `!?` Transactional CoW Shadowing
    // =========================================================================

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void StdPath_CrossPlatformManipulation_Parity(bool useBytecode)
    {
        string code = @"
            import std.path;
            import std.option;

            let p_posix = path.normalize(""/a/b/../c/./module.pilot"");
            let p_win = path.normalize(""C:\\Users\\pilot\\..\\repo\\src\\app.pilot"");
            let p_obj = path.Path(""/workspace/stdlib/io/fs.pilot"");
            let swapped = p_obj.with_extension(""pilc"").to_str();
            let rel = option.unwrap(path.relative_to(""/workspace/stdlib/io/fs.pilot"", ""/workspace/stdlib""));

            return (
                p_posix,
                p_win,
                option.unwrap(p_obj.stem()),
                option.unwrap(p_obj.extension()),
                swapped,
                rel
            );
        ";

        var exec = ExecuteCode(code, useBytecode);
        Assert.True(exec.Success, exec.ErrorMessage);
        var t = exec.Value.AsTuple();
        Assert.Equal("/a/c/module.pilot", t[0].AsString());
        Assert.Equal("C:/Users/repo/src/app.pilot", t[1].AsString());
        Assert.Equal("fs", t[2].AsString());
        Assert.Equal("pilot", t[3].AsString());
        Assert.Equal("/workspace/stdlib/io/fs.pilc", t[4].AsString());
        Assert.Equal("io/fs.pilot", t[5].AsString());
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void StdIo_And_StdFs_BufferedStreams_ZeroCopyMmap_Glob_And_DryRunEffectPlan(bool useBytecode)
    {
        string tempDir = Path.Combine(Path.GetTempPath(), $"pilot_m21_fs_{Guid.NewGuid():N}").Replace('\\', '/');
        Directory.CreateDirectory(tempDir);
        try
        {
            string code = $@"
                import std.io;
                import std.fs;
                import std.path;
                import std.mem;
                import std.meta;
                import std.option;
                import std.result;

                let root = ""{tempDir}"";
                let sub = path.join(root, ""nested"");
                let log_file = path.join(sub, ""notes.txt"");
                let shadow_file = path.join(root, ""staged.pilot"");

                def write_and_read() -> (str, str?, bool, i64, i64) ! io.IoError {{
                    let _mk = fs.create_dir(sub)?;
                    var writer = io.open_writer(log_file)?;
                    writer.write_line(""alpha"");
                    writer.write_line(""beta"");
                    let _flushed = writer.close()?;

                    var reader = io.open_reader(log_file)?;
                    let l1 = option.unwrap(reader.read_line());
                    let l2 = option.unwrap(reader.read_line());
                    let eof_after = reader.read_line(); // none on EOF!

                    // Zero-copy read-only OS mmap over notes.txt
                    let mapped = fs.mmap_read(log_file, ""u8"")?;
                    let ro = mem.slice_is_readonly(mapped);
                    let first_byte = mem.slice_get(mapped, 0); // 'a' == 97
                    let map_len = mem.slice_len(mapped);

                    return ok((f""{{l1}},{{l2}}"", eof_after, ro, first_byte, map_len));
                }}

                let io_res = result.unwrap(write_and_read());

                // Missing file returns `IoError.NotFound` value (zero exceptions!)
                let missing_res = io.open_reader(path.join(root, ""does_not_exist.txt""));
                let err_msg = io.format_io_error(result.unwrap_err(missing_res));

                // Transactional `!?` Copy-on-Write dry-run over `fs.write_text`
                def stage_artifact(target: str) -> i64 ! io.IoError {{
                    let written = fs.write_text(target, ""// staged via EffectPlan\n"")?;
                    return ok(written);
                }}

                let plan = stage_artifact!?(shadow_file);
                let existed_before_commit = fs.exists(shadow_file);
                let diff_cnt = meta.plan_diff_count(plan);
                let _committed = meta.commit_plan(plan);
                let existed_after_commit = fs.exists(shadow_file);

                let glob_hits = result.unwrap(fs.glob(root, ""**/*.txt""));

                return (
                    io_res[0],
                    option.is_none(io_res[1]),
                    io_res[2],
                    io_res[3],
                    err_msg,
                    existed_before_commit,
                    diff_cnt,
                    existed_after_commit,
                    len(glob_hits)
                );
            ";

            var exec = ExecuteCode(code, useBytecode);
            Assert.True(exec.Success, exec.ErrorMessage);
            var t = exec.Value.AsTuple();
            Assert.Equal("alpha,beta", t[0].AsString());
            Assert.True(t[1].IsTrue);
            Assert.True(t[2].IsTrue);
            Assert.Equal(97L, t[3].AsInt());
            Assert.StartsWith("NotFound:", t[4].AsString());
            Assert.False(t[5].IsTrue); // CoW dry-run did NOT touch disk before commit!
            Assert.Equal(1L, t[6].AsInt());
            Assert.True(t[7].IsTrue);  // Committed cleanly to disk!
            Assert.Equal(1L, t[8].AsInt());
        }
        finally
        {
            if (Directory.Exists(tempDir))
                Directory.Delete(tempDir, recursive: true);
        }
    }

    // =========================================================================
    // 24.2: `std.encoding` & `std.formats` (RFC-4648, RFC-3986, JSON, TOML, CSV, .env)
    // =========================================================================

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void StdEncoding_Hex_Base64_Base64Url_Comptime_And_ZeroCopySlices_Parity(bool useBytecode)
    {
        string code = @"
            import std.encoding;
            import std.mem;
            import std.text;
            import std.result;

            // Compile-time evaluation of pure RFC-4648 encoders
            const CT_HEX: str = comptime(encoding.hex_encode(""Pilot""));
            const CT_B64: str = comptime(encoding.base64_encode(""foobar""));

            // RFC 4648 Section 10 test vectors
            let v0 = encoding.base64_encode("""");
            let v1 = encoding.base64_encode(""f"");
            let v2 = encoding.base64_encode(""fo"");
            let v3 = encoding.base64_encode(""foo"");
            let v4 = encoding.base64_encode(""foob"");
            let v5 = encoding.base64_encode(""fooba"");
            let v6 = encoding.base64_encode(""foobar"");

            let d6 = result.unwrap(encoding.base64_decode(v6));
            let hex_rt = result.unwrap(encoding.hex_decode(CT_HEX));

            // Binary bytes [0, 1, 127, 128, 251, 254, 255] -> '+' & '/' in standard base64 vs '-' & '_' in base64url
            let raw_bytes = [0, 1, 127, 128, 251, 254, 255];
            let std_b64 = encoding.base64_encode(raw_bytes);
            let url_b64 = encoding.base64url_encode(raw_bytes, false);
            let round_std = result.unwrap(encoding.base64_decode_bytes(std_b64));
            let round_url = result.unwrap(encoding.base64url_decode_bytes(url_b64));

            // Zero-copy SliceBuffer<u8> subslice input
            let framed = text.as_utf8_slice(""xx50696c6f74yy"");
            let sub = mem.slice_subview(framed, 2, 10);
            let sub_decoded = result.unwrap(encoding.hex_decode(sub));

            // Values-first DecodeError verification (zero exceptions)
            let err_odd_hex = encoding.format_decode_error(result.unwrap_err(encoding.hex_decode(""abc"")));
            let err_bad_hex = encoding.format_decode_error(result.unwrap_err(encoding.hex_decode(""deadbeeg"")));
            let err_bad_pad = encoding.format_decode_error(result.unwrap_err(encoding.base64_decode(""Zg=a"")));
            let err_non_canonical_pad = encoding.format_decode_error(result.unwrap_err(encoding.base64_decode(""Zh=="")));
            // Invalid UTF-8 bytes 0xFF 0xFE encoded in hex
            let err_bad_utf8 = encoding.format_decode_error(result.unwrap_err(encoding.hex_decode(""fffe"")));

            return (
                CT_HEX,
                CT_B64,
                f""{v0}|{v1}|{v2}|{v3}|{v4}|{v5}|{v6}"",
                d6,
                hex_rt,
                url_b64,
                mem.slice_to_list(round_std) == raw_bytes and mem.slice_to_list(round_url) == raw_bytes,
                sub_decoded,
                err_odd_hex,
                err_bad_hex,
                err_bad_pad,
                err_non_canonical_pad,
                err_bad_utf8
            );
        ";

        var exec = ExecuteCode(code, useBytecode);
        Assert.True(exec.Success, exec.ErrorMessage);
        var t = exec.Value.AsTuple();
        Assert.Equal("50696c6f74", t[0].AsString());
        Assert.Equal("Zm9vYmFy", t[1].AsString());
        Assert.Equal("|Zg==|Zm8=|Zm9v|Zm9vYg==|Zm9vYmE=|Zm9vYmFy", t[2].AsString());
        Assert.Equal("foobar", t[3].AsString());
        Assert.Equal("Pilot", t[4].AsString());
        Assert.DoesNotContain("+", t[5].AsString());
        Assert.DoesNotContain("/", t[5].AsString());
        Assert.DoesNotContain("=", t[5].AsString());
        Assert.True(t[6].IsTrue);
        Assert.Equal("Pilot", t[7].AsString());
        Assert.StartsWith("InvalidLength", t[8].AsString());
        Assert.StartsWith("InvalidChar", t[9].AsString());
        Assert.StartsWith("InvalidPadding", t[10].AsString());
        Assert.StartsWith("InvalidPadding", t[11].AsString());
        Assert.StartsWith("InvalidUtf8", t[12].AsString());
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void StdEncoding_Rfc3986Url_QueryCodec_And_ParseUrl_Parity(bool useBytecode)
    {
        string code = @"
            import std.encoding;
            import std.option;
            import std.result;

            let raw = ""hello world/λ?a=1&b=2"";
            let pct = encoding.url_encode(raw);
            let dec = result.unwrap(encoding.url_decode(pct));

            var qmap = {};
            qmap[""q""] = ""pilot stdlib"";
            qmap[""lang""] = ""en/us"";
            let qs = encoding.query_encode(qmap);
            let qparsed = result.unwrap(encoding.query_parse(qs));

            let multi = result.unwrap(encoding.query_parse_multi(""tag=core&tag=codecs&empty=&flag""));
            let tags = multi[""tag""];

            let u = result.unwrap(encoding.parse_url(""https://dev:s3cr3t@registry.pilot-lang.org:8443/v1/packages?tag=core&sort=asc#readme""));
            let u_ipv6 = result.unwrap(encoding.parse_url(""http://[::1]:9000/status""));
            let u_q = result.unwrap(u.query_params());

            let bad_pct = encoding.format_decode_error(result.unwrap_err(encoding.url_decode(""100%2G"")));
            let trunc_pct = encoding.format_decode_error(result.unwrap_err(encoding.url_decode(""abc%2"")));

            return (
                pct,
                dec == raw,
                qs,
                qparsed[""q""],
                qparsed[""lang""],
                len(tags),
                tags[1],
                u.scheme,
                u.userinfo,
                u.host,
                option.unwrap(u.port),
                u.path,
                u.fragment,
                u.origin(),
                u_q[""tag""],
                u_ipv6.host,
                option.unwrap(u_ipv6.port),
                bad_pct,
                trunc_pct
            );
        ";

        var exec = ExecuteCode(code, useBytecode);
        Assert.True(exec.Success, exec.ErrorMessage);
        var t = exec.Value.AsTuple();
        Assert.Equal("hello%20world%2F%CE%BB%3Fa%3D1%26b%3D2", t[0].AsString());
        Assert.True(t[1].IsTrue);
        Assert.Equal("lang=en%2Fus&q=pilot+stdlib", t[2].AsString());
        Assert.Equal("pilot stdlib", t[3].AsString());
        Assert.Equal("en/us", t[4].AsString());
        Assert.Equal(2L, t[5].AsInt());
        Assert.Equal("codecs", t[6].AsString());
        Assert.Equal("https", t[7].AsString());
        Assert.Equal("dev:s3cr3t", t[8].AsString());
        Assert.Equal("registry.pilot-lang.org", t[9].AsString());
        Assert.Equal(8443L, t[10].AsInt());
        Assert.Equal("/v1/packages", t[11].AsString());
        Assert.Equal("readme", t[12].AsString());
        Assert.Equal("https://registry.pilot-lang.org:8443", t[13].AsString());
        Assert.Equal("core", t[14].AsString());
        Assert.Equal("[::1]", t[15].AsString());
        Assert.Equal(9000L, t[16].AsInt());
        Assert.StartsWith("BadPercentEscape", t[17].AsString());
        Assert.StartsWith("BadPercentEscape", t[18].AsString());
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void StdFormats_Json_ParseStringify_Surrogates_Path_And_Errors_Parity(bool useBytecode)
    {
        string code = @"
            import std.formats;
            import std.text;
            import std.option;
            import std.result;

            let doc_src = ""{\n  \""name\"": \""pilot\"",\n  \""version\"": 24,\n  \""pi\"": 3.1415,\n  \""active\"": true,\n  \""nothing\"": null,\n  \""greek\"": \""\\u03BB\"",\n  \""rocket\"": \""\\uD83D\\uDE80\"",\n  \""targets\"": [\n    {\""arch\"": \""x86_64\""},\n    {\""arch\"": \""wasm32\""}\n  ]\n}"";

            // Parse from zero-copy UTF-8 SliceBuffer<u8>
            let doc = result.unwrap(formats.json_parse(text.as_utf8_slice(doc_src), true));
            let arch1 = option.unwrap(formats.json_get_path(doc, ""targets.1.arch""));
            let missing_path = option.is_none(formats.json_get_path(doc, ""targets.9.arch""));

            // Deterministic compact & pretty serialization round-trip
            let compact = formats.json_stringify(doc, false);
            let pretty = formats.json_stringify(doc, true, 2);
            let reparsed = result.unwrap(formats.json_parse(pretty, true));

            // Strict RFC 8259 error cases with line/col diagnostics
            let err_dup = formats.format_parse_error(result.unwrap_err(formats.json_parse(""{\""a\"": 1, \""a\"": 2}"", true)));
            let err_num = formats.format_parse_error(result.unwrap_err(formats.json_parse(""{\""n\"": 01}"", false)));
            let err_sur = formats.format_parse_error(result.unwrap_err(formats.json_parse(""\""\\uD83D\"""", false)));
            let err_eof = formats.format_parse_error(result.unwrap_err(formats.json_parse(""{\""open\"": [1, 2"", false)));
            let err_trail = formats.format_parse_error(result.unwrap_err(formats.json_parse(""[1, 2] extra"", false)));

            return (
                doc[""name""],
                doc[""version""],
                doc[""active""],
                option.is_none(doc[""nothing""]),
                doc[""greek""],
                doc[""rocket""],
                arch1,
                missing_path,
                reparsed[""rocket""] == ""🚀"" and reparsed[""version""] == 24,
                len(compact) < len(pretty),
                err_dup,
                err_num,
                err_sur,
                err_eof,
                err_trail
            );
        ";

        var exec = ExecuteCode(code, useBytecode);
        Assert.True(exec.Success, exec.ErrorMessage);
        var t = exec.Value.AsTuple();
        Assert.Equal("pilot", t[0].AsString());
        Assert.Equal(24L, t[1].AsInt());
        Assert.True(t[2].IsTrue);
        Assert.True(t[3].IsTrue);
        Assert.Equal("λ", t[4].AsString());
        Assert.Equal("🚀", t[5].AsString());
        Assert.Equal("wasm32", t[6].AsString());
        Assert.True(t[7].IsTrue);
        Assert.True(t[8].IsTrue);
        Assert.True(t[9].IsTrue);
        Assert.Contains("DuplicateKey: 'a'", t[10].AsString());
        Assert.Contains("BadNumber: '01'", t[11].AsString());
        Assert.Contains("BadEscape:", t[12].AsString());
        Assert.Contains("PrematureEof", t[13].AsString());
        Assert.Contains("SyntaxError:", t[14].AsString());
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void StdFormats_Toml_Tables_ArrayOfTables_RadixNumbers_And_RoundTrip_Parity(bool useBytecode)
    {
        string code = @"
            import std.formats;
            import std.result;

            let toml_src = ""# Pilot package config\ntitle = \""Pilot TOML\""\nraw_win = 'C:\\tools\\pilot'\nhex_mask = 0xdead_beef\noct_perm = 0o755\nbin_flags = 0b1010_0101\nbudget = 1_250.5\nlimit = inf\ndotted.sub.enabled = true\nports = [\n  8001,\n  8002, # trailing comma allowed\n]\ninline_srv = { host = \""127.0.0.1\"", port = 9090 }\n\n[package]\nname = \""pilot-core\""\nedition = 2026\n\n[[bin]]\nname = \""pilotc\""\npath = \""src/main.pilot\""\n\n[[bin]]\nname = \""pilot-lsp\""\npath = \""src/lsp.pilot\""\n"";

            let parsed = result.unwrap(formats.toml_parse(toml_src));
            let pkg = parsed[""package""];
            let bins = parsed[""bin""];
            let dotted_enabled = parsed[""dotted""][""sub""][""enabled""];
            let inline_host = parsed[""inline_srv""][""host""];

            let serialized = formats.toml_stringify(parsed);
            let reparsed = result.unwrap(formats.toml_parse(serialized));

            let dup_err = formats.format_parse_error(result.unwrap_err(formats.toml_parse(""a = 1\na = 2\n"")));

            return (
                parsed[""title""],
                parsed[""raw_win""],
                parsed[""hex_mask""],
                parsed[""oct_perm""],
                parsed[""bin_flags""],
                parsed[""budget""],
                not isfinite(parsed[""limit""]),
                dotted_enabled,
                inline_host,
                pkg[""name""],
                len(bins),
                bins[1][""name""],
                reparsed[""package""][""name""] == ""pilot-core"" and len(reparsed[""bin""]) == 2,
                dup_err
            );
        ";

        var exec = ExecuteCode(code, useBytecode);
        Assert.True(exec.Success, exec.ErrorMessage);
        var t = exec.Value.AsTuple();
        Assert.Equal("Pilot TOML", t[0].AsString());
        Assert.Equal(@"C:\tools\pilot", t[1].AsString());
        Assert.Equal(0xDEADBEEFL, t[2].AsInt());
        Assert.Equal(493L, t[3].AsInt());
        Assert.Equal(165L, t[4].AsInt());
        Assert.Equal(1250.5, t[5].AsFloat(), 6);
        Assert.True(t[6].IsTrue);
        Assert.True(t[7].IsTrue);
        Assert.Equal("127.0.0.1", t[8].AsString());
        Assert.Equal("pilot-core", t[9].AsString());
        Assert.Equal(2L, t[10].AsInt());
        Assert.Equal("pilot-lsp", t[11].AsString());
        Assert.True(t[12].IsTrue);
        Assert.Contains("DuplicateKey: 'a'", t[13].AsString());
    }

    [Theory]
    [InlineData(false)]
    [InlineData(true)]
    public void StdFormats_Csv_Rfc4180Quotes_Records_And_DotenvExpansion_Parity(bool useBytecode)
    {
        string code = @"
            import std.formats;
            import std.result;

            // RFC 4180 CSV with quoted commas, doubled quotes, and multiline cell
            let csv_src = ""id,title,notes\r\n1,\""Core, M21\"",\""Says \""\""hi\""\""\""\r\n2,\""Codecs\nM24.2\"",plain\r\n"";
            let recs = result.unwrap(formats.csv_parse_records(csv_src, "",""));
            let out_csv = formats.csv_stringify_records(recs, [""id"", ""title"", ""notes""], "","");
            let recs_rt = result.unwrap(formats.csv_parse_records(out_csv, "",""));
            let csv_err = formats.format_parse_error(result.unwrap_err(formats.csv_parse(""a,\""unterminated"", "","")));

            // .env with export, comments, single-quoted literal, double-quoted interpolation & ${VAR:-default}
            let env_src = ""# App environment\nexport HOST=localhost\nPORT=5432 # postgres port\nRAW='$HOST/literal'\nDSN=\""postgres://${HOST}:${PORT}/${DB_NAME:-pilot_dev}\""\n"";
            let env_map = result.unwrap(formats.env_parse(env_src, true));
            let env_dumped = formats.env_stringify(env_map);
            let env_rt = result.unwrap(formats.env_parse(env_dumped, false));

            return (
                len(recs),
                recs[0][""title""],
                recs[0][""notes""],
                recs[1][""title""],
                recs_rt[0][""notes""] == recs[0][""notes""] and recs_rt[1][""title""] == recs[1][""title""],
                csv_err,
                env_map[""HOST""],
                env_map[""PORT""],
                env_map[""RAW""],
                env_map[""DSN""],
                env_rt[""DSN""] == env_map[""DSN""] and env_rt[""RAW""] == ""$HOST/literal""
            );
        ";

        var exec = ExecuteCode(code, useBytecode);
        Assert.True(exec.Success, exec.ErrorMessage);
        var t = exec.Value.AsTuple();
        Assert.Equal(2L, t[0].AsInt());
        Assert.Equal("Core, M21", t[1].AsString());
        Assert.Equal("Says \"hi\"", t[2].AsString());
        Assert.Equal("Codecs\nM24.2", t[3].AsString());
        Assert.True(t[4].IsTrue);
        Assert.Contains("PrematureEof", t[5].AsString());
        Assert.Equal("localhost", t[6].AsString());
        Assert.Equal("5432", t[7].AsString());
        Assert.Equal("$HOST/literal", t[8].AsString());
        Assert.Equal("postgres://localhost:5432/pilot_dev", t[9].AsString());
        Assert.True(t[10].IsTrue);
    }
}
