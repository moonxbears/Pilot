using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using Pilot.Analysis.Autodiff;
using Pilot.Analysis.CallQuoting;
using Pilot.Runtime.Builtins;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Effects;
using Pilot.Runtime.Execution;
using Pilot.Runtime.NativeMemory;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Stdlib;

/// <summary>
/// Milestone 21: Zero-dependency native runtime intrinsics backing the pure Pilot <c>stdlib/</c> modules:
/// <list type="bullet">
/// <item><description><c>std.option</c> &amp; <c>std.result</c> niche/register inspection</description></item>
/// <item><description><c>std.mem</c> <see cref="PilotSharedBuffer"/> (<c>SliceBuffer&lt;T&gt;</c>) &amp; <see cref="PilotFrozenMap"/> operations</description></item>
/// <item><description><c>std.meta</c> keyword DAG reflection, <c>AstExpr</c> folding/differentiation, &amp; <c>EffectPlan</c> control</description></item>
/// <item><description><c>std.math</c> differentiable elementary functions &amp; IEEE-754 predicates</description></item>
/// <item><description><c>std.text</c> zero-copy UTF-8 <c>SliceBuffer&lt;u8&gt;</c> views &amp; <c>comptime</c> DFA regex primitives</description></item>
/// <item><description><c>std.io</c> / <c>std.fs</c> / <c>std.path</c> fallible (<c>! IoError</c>) OS operations with <c>!?</c> CoW shadow support</description></item>
/// </list>
/// </summary>
public static class PilotStdlibIntrinsics
{
    private static readonly Lazy<PilotAdtDefinition> CanonicalIoErrorAdt = new(BuildCanonicalIoErrorAdt);
    private static readonly UTF8Encoding Utf8NoBom = new(encoderShouldEmitUTF8Identifier: false);

    private static PilotAdtDefinition BuildCanonicalIoErrorAdt()
    {
        var adt = new PilotAdtDefinition("IoError");
        adt.AddVariant("NotFound", new[] { new PilotAdtFieldDefinition("path", "str", 0) });
        adt.AddVariant("PermissionDenied", new[] { new PilotAdtFieldDefinition("path", "str", 0) });
        adt.AddVariant("InvalidPath", new[] { new PilotAdtFieldDefinition("msg", "str", 0) });
        adt.AddVariant("AlreadyExists", new[] { new PilotAdtFieldDefinition("path", "str", 0) });
        adt.AddVariant("UnexpectedEof", new[] { new PilotAdtFieldDefinition("msg", "str", 0) });
        adt.AddVariant("Other", new[] { new PilotAdtFieldDefinition("msg", "str", 0) });
        return adt;
    }

    public static PilotValue CreateIoErrorFailure(PilotExecutionContext ctx, string variantName, string detail)
    {
        PilotAdtDefinition adt = ctx.AdtDefinitions.TryGetValue("IoError", out var existing)
            ? existing
            : CanonicalIoErrorAdt.Value;

        if (!adt.TryGetVariant(variantName, out var variantDef))
        {
            adt = CanonicalIoErrorAdt.Value;
            if (!adt.TryGetVariant(variantName, out variantDef))
            {
                adt.TryGetVariant("Other", out variantDef);
            }
        }

        var adtVal = variantDef!.Instantiate(ctx, new[] { PilotValue.FromString(detail ?? string.Empty) });
        return PilotValue.FromFailure(adtVal);
    }

    private static PilotValue MapIoException(PilotExecutionContext ctx, Exception ex, string targetPath)
    {
        return ex switch
        {
            FileNotFoundException or DirectoryNotFoundException =>
                CreateIoErrorFailure(ctx, "NotFound", targetPath),
            UnauthorizedAccessException =>
                CreateIoErrorFailure(ctx, "PermissionDenied", targetPath),
            ArgumentException or NotSupportedException or PathTooLongException =>
                CreateIoErrorFailure(ctx, "InvalidPath", ex.Message),
            EndOfStreamException =>
                CreateIoErrorFailure(ctx, "UnexpectedEof", ex.Message),
            IOException ioEx when ioEx.Message.Contains("already exists", StringComparison.OrdinalIgnoreCase) =>
                CreateIoErrorFailure(ctx, "AlreadyExists", targetPath),
            _ =>
                CreateIoErrorFailure(ctx, "Other", ex.Message)
        };
    }

    public static void RegisterAll(PilotBuiltinRegistry registry)
    {
        RegisterOptionAndResultIntrinsics(registry);
        RegisterMemoryAndFrozenMapIntrinsics(registry);
        RegisterMetaAndReflectionIntrinsics(registry);
        RegisterElementaryMathIntrinsics(registry);
        RegisterTextUtf8AndRegexIntrinsics(registry);
        RegisterFilesystemAndOsIntrinsics(registry);
        RegisterCodecAndFormatIntrinsics(registry);
    }

    #region 21.2 Option, Result, Memory & Meta Intrinsics

    private static void RegisterOptionAndResultIntrinsics(PilotBuiltinRegistry registry)
    {
        registry.Register("__opt_is_some", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.False;
            var v = args[0];
            if (v.IsNestedSome) return PilotValue.True;
            return PilotValue.FromBool(!v.IsNull && !v.IsUndefined && !v.IsFailure);
        }, "opt");

        registry.Register("__opt_is_none", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.True;
            var v = args[0];
            if (v.IsNestedSome) return PilotValue.False;
            return PilotValue.FromBool(v.IsNull || v.IsUndefined);
        }, "opt");

        registry.Register("__opt_unwrap", (_, args) =>
        {
            if (args.Count == 0)
                throw new PilotRuntimeException("[ERR005] unwrap called on none");
            var v = args[0];
            if (!v.IsNestedSome && (v.IsNull || v.IsUndefined))
                throw new PilotRuntimeException("[ERR005] unwrap called on none");
            return v.UnwrapOption();
        }, "opt");

        registry.Register("__result_is_ok", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.False;
            return PilotValue.FromBool(!args[0].IsFailure);
        }, "res");

        registry.Register("__result_is_err", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.True;
            return PilotValue.FromBool(args[0].IsFailure);
        }, "res");

        registry.Register("__result_unwrap_ok", (_, args) =>
        {
            if (args.Count == 0)
                throw new PilotRuntimeException("[ERR005] unwrap_ok called with no value");
            if (args[0].IsFailure)
                throw new PilotRuntimeException($"[ERR005] unwrap_ok called on failure: {args[0].UnwrapFailure()}");
            return args[0];
        }, "res");

        registry.Register("__result_unwrap_err", (_, args) =>
        {
            if (args.Count == 0 || !args[0].IsFailure)
                throw new PilotRuntimeException("[ERR005] unwrap_err called on ok value");
            return args[0].UnwrapFailure();
        }, "res");
    }

    private static void RegisterMemoryAndFrozenMapIntrinsics(PilotBuiltinRegistry registry)
    {
        registry.Register("__slice_len", (_, args) =>
            args.Count > 0 && args[0].Kind == PilotValueKind.SliceBuffer
                ? PilotValue.FromInt(args[0].AsSliceBuffer().Length)
                : PilotValue.FromInt(0), "buf");

        registry.Register("__slice_cap", (_, args) =>
            args.Count > 0 && args[0].Kind == PilotValueKind.SliceBuffer
                ? PilotValue.FromInt(args[0].AsSliceBuffer().Capacity)
                : PilotValue.FromInt(0), "buf");

        registry.Register("__slice_stride", (_, args) =>
            args.Count > 0 && args[0].Kind == PilotValueKind.SliceBuffer
                ? PilotValue.FromInt(args[0].AsSliceBuffer().Stride)
                : PilotValue.FromInt(0), "buf");

        registry.Register("__slice_ptr", (_, args) =>
            args.Count > 0 && args[0].Kind == PilotValueKind.SliceBuffer
                ? PilotValue.FromInt(args[0].AsSliceBuffer().DataPointer.ToInt64())
                : PilotValue.FromInt(0), "buf");

        registry.Register("__slice_is_readonly", (_, args) =>
            PilotValue.FromBool(args.Count > 0 && args[0].Kind == PilotValueKind.SliceBuffer && args[0].AsSliceBuffer().IsReadOnly), "buf");

        registry.Register("__slice_get", (_, args) =>
        {
            if (args.Count < 2 || args[0].Kind != PilotValueKind.SliceBuffer)
                return PilotValue.Null;
            var buf = args[0].AsSliceBuffer();
            int idx = (int)args[1].AsInt();
            if (idx < 0) idx += buf.Length;
            if (idx < 0 || idx >= buf.Length)
                return PilotValue.Null;
            if (PilotEffectDryRunEngine.TryReadSliceBufferElement(buf, idx, out var shadowVal))
                return PilotValue.FromSome(shadowVal);
            var elem = buf.GetElement(idx);
            return elem.IsUndefined ? PilotValue.Null : PilotValue.FromSome(elem);
        }, "buf", "index");

        registry.Register("__slice_set", (ctx, args) =>
        {
            if (args.Count < 3 || args[0].Kind != PilotValueKind.SliceBuffer)
                return PilotValue.False;
            var buf = args[0].AsSliceBuffer();
            if (buf.IsReadOnly)
                return CreateIoErrorFailure(ctx, "PermissionDenied", buf.BackingPath ?? "<readonly-mmap>");
            int idx = (int)args[1].AsInt();
            if (idx < 0) idx += buf.Length;
            if (idx < 0 || idx >= buf.Capacity)
                return PilotValue.False;
            if (!PilotEffectDryRunEngine.TryRecordSliceBufferWrite(buf, idx, args[2]))
            {
                buf.SetElement(idx, args[2]);
            }
            return PilotValue.True;
        }, "buf", "index", "val");

        registry.Register("__slice_push", (ctx, args) =>
        {
            if (args.Count < 2 || args[0].Kind != PilotValueKind.SliceBuffer)
                return PilotValue.FromInt(0);
            var buf = args[0].AsSliceBuffer();
            if (buf.IsReadOnly)
                return CreateIoErrorFailure(ctx, "PermissionDenied", buf.BackingPath ?? "<readonly-mmap>");
            buf.YieldElement(args[1]);
            return PilotValue.FromInt(buf.Length);
        }, "buf", "val");

        registry.Register("__slice_clear", (_, args) =>
        {
            if (args.Count > 0 && args[0].Kind == PilotValueKind.SliceBuffer)
            {
                var buf = args[0].AsSliceBuffer();
                if (!buf.IsReadOnly) buf.ResetLength();
            }
            return PilotValue.Null;
        }, "buf");

        registry.Register("__slice_subview", (_, args) =>
        {
            if (args.Count < 3 || args[0].Kind != PilotValueKind.SliceBuffer)
                return PilotValue.Null;
            var buf = args[0].AsSliceBuffer();
            int start = (int)args[1].AsInt();
            int length = (int)args[2].AsInt();
            return PilotValue.FromSliceBuffer(buf.SliceView(start, length));
        }, "buf", "start", "length");

        registry.Register("__slice_to_list", (_, args) =>
        {
            if (args.Count == 0 || args[0].Kind != PilotValueKind.SliceBuffer)
                return PilotValue.FromList(new List<PilotValue>());
            var buf = args[0].AsSliceBuffer();
            var list = new List<PilotValue>(buf.Length);
            for (int i = 0; i < buf.Length; i++)
            {
                if (PilotEffectDryRunEngine.TryReadSliceBufferElement(buf, i, out var sv))
                    list.Add(sv);
                else
                    list.Add(buf.GetElement(i));
            }
            return PilotValue.FromList(list);
        }, "buf");

        registry.Register("__freeze_value", (_, args) =>
            args.Count > 0 ? args[0].Freeze() : PilotValue.Null, "val");

        registry.Register("__is_frozen", (_, args) =>
            PilotValue.FromBool(args.Count > 0 && args[0].IsFrozen), "val");

        registry.Register("__is_perfect_hash_map", (_, args) =>
            PilotValue.FromBool(args.Count > 0 && args[0].TryGetFrozenMap(out var fm) && fm.IsPerfectHash), "map");

        registry.Register("__frozen_map_table_size", (_, args) =>
            args.Count > 0 && args[0].TryGetFrozenMap(out var fm)
                ? PilotValue.FromInt(fm.TableSize)
                : PilotValue.FromInt(0), "map");

        // C++20 `libpilot_rt` Shared Backend & SIMD SliceBuffer Accelerators
        registry.Register("__cpp_backend_available", (_, _) =>
            PilotValue.FromBool(Pilot.Runtime.CppBackend.PilotCppBackend.IsAvailable));

        registry.Register("__cpp_simd_bits", (_, _) =>
            PilotValue.FromInt(Pilot.Runtime.CppBackend.PilotCppBackend.SimdWidthBits));

        registry.Register("__cpp_dispatch_count", (_, _) =>
            PilotValue.FromInt(Pilot.Runtime.CppBackend.PilotCppBackend.NativeDispatchCalls));

        registry.Register("__slice_sum", (_, args) =>
        {
            if (args.Count == 0 || args[0].Kind != PilotValueKind.SliceBuffer)
                return PilotValue.FromFloat(0.0);
            return Pilot.Runtime.CppBackend.PilotCppBackend.SliceSum(args[0].AsSliceBuffer());
        }, "buf");

        registry.Register("__slice_dot", (_, args) =>
        {
            if (args.Count < 2 || args[0].Kind != PilotValueKind.SliceBuffer || args[1].Kind != PilotValueKind.SliceBuffer)
                return PilotValue.FromFloat(0.0);
            return PilotValue.FromFloat(Pilot.Runtime.CppBackend.PilotCppBackend.SliceDot(
                args[0].AsSliceBuffer(),
                args[1].AsSliceBuffer()));
        }, "a", "b");

        registry.Register("__slice_saxpy", (ctx, args) =>
        {
            if (args.Count < 3 || args[1].Kind != PilotValueKind.SliceBuffer || args[2].Kind != PilotValueKind.SliceBuffer)
                return PilotValue.False;
            var y = args[2].AsSliceBuffer();
            if (y.IsReadOnly)
                return CreateIoErrorFailure(ctx, "PermissionDenied", y.BackingPath ?? "<readonly-mmap>");
            Pilot.Runtime.CppBackend.PilotCppBackend.SliceSaxpy(args[0].AsFloat(), args[1].AsSliceBuffer(), y);
            return PilotValue.True;
        }, "alpha", "x", "y");

        registry.Register("__slice_scale", (ctx, args) =>
        {
            if (args.Count < 2 || args[1].Kind != PilotValueKind.SliceBuffer)
                return PilotValue.False;
            var buf = args[1].AsSliceBuffer();
            if (buf.IsReadOnly)
                return CreateIoErrorFailure(ctx, "PermissionDenied", buf.BackingPath ?? "<readonly-mmap>");
            Pilot.Runtime.CppBackend.PilotCppBackend.SliceScale(args[0].AsFloat(), buf);
            return PilotValue.True;
        }, "alpha", "buf");

        registry.Register("__slice_sort", (ctx, args) =>
        {
            if (args.Count == 0 || args[0].Kind != PilotValueKind.SliceBuffer)
                return PilotValue.False;
            var buf = args[0].AsSliceBuffer();
            if (buf.IsReadOnly)
                return CreateIoErrorFailure(ctx, "PermissionDenied", buf.BackingPath ?? "<readonly-mmap>");
            Pilot.Runtime.CppBackend.PilotCppBackend.SliceSort(buf);
            return PilotValue.True;
        }, "buf");

        registry.Register("__slice_welford", (_, args) =>
        {
            if (args.Count == 0 || args[0].Kind != PilotValueKind.SliceBuffer)
            {
                return PilotValue.FromTuple(new[]
                {
                    PilotValue.FromFloat(0.0),
                    PilotValue.FromFloat(0.0),
                    PilotValue.FromFloat(0.0),
                    PilotValue.FromFloat(0.0)
                });
            }
            Pilot.Runtime.CppBackend.PilotCppBackend.SliceWelford(
                args[0].AsSliceBuffer(),
                out double mean,
                out double m2,
                out double min,
                out double max);
            return PilotValue.FromTuple(new[]
            {
                PilotValue.FromFloat(mean),
                PilotValue.FromFloat(m2),
                PilotValue.FromFloat(min),
                PilotValue.FromFloat(max)
            });
        }, "buf");
    }

    private static void RegisterMetaAndReflectionIntrinsics(PilotBuiltinRegistry registry)
    {
        registry.Register("__kw_parents", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromList(new List<PilotValue>());
            var kw = args[0].Kind == PilotValueKind.Keyword ? args[0].AsKeyword() : PilotKeyword.Intern(args[0].ToString());
            return PilotValue.FromList(kw.GetParents().Select(PilotValue.FromKeyword).ToList());
        }, "kw");

        registry.Register("__kw_ancestors", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromList(new List<PilotValue>());
            var kw = args[0].Kind == PilotValueKind.Keyword ? args[0].AsKeyword() : PilotKeyword.Intern(args[0].ToString());
            return PilotValue.FromList(kw.GetAncestors().Select(PilotValue.FromKeyword).ToList());
        }, "kw");

        registry.Register("__kw_descendants", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromList(new List<PilotValue>());
            var kw = args[0].Kind == PilotValueKind.Keyword ? args[0].AsKeyword() : PilotKeyword.Intern(args[0].ToString());
            return PilotValue.FromList(kw.GetDescendants().Select(PilotValue.FromKeyword).ToList());
        }, "kw");

        registry.Register("__kw_distance", (_, args) =>
        {
            if (args.Count < 2) return PilotValue.FromInt(-1);
            var child = args[0].Kind == PilotValueKind.Keyword ? args[0].AsKeyword() : PilotKeyword.Intern(args[0].ToString());
            var parent = args[1].Kind == PilotValueKind.Keyword ? args[1].AsKeyword() : PilotKeyword.Intern(args[1].ToString());
            return PilotValue.FromInt(child.GetDerivationDistance(parent));
        }, "child", "ancestor");

        registry.Register("__ast_fold", (_, args) =>
        {
            if (args.Count == 0 || !TryUnwrapAstExpr(args[0], out var ast))
                return PilotValue.Null;
            return ast.FoldConstants().ToPilotObject();
        }, "ast");

        registry.Register("__ast_subst", (_, args) =>
        {
            if (args.Count < 3 || !TryUnwrapAstExpr(args[0], out var ast))
                return PilotValue.Null;
            string sym = args[1].Kind == PilotValueKind.Keyword ? args[1].AsKeyword().Name : args[1].ToString().TrimStart(':');
            PilotAstExpr repl = TryUnwrapAstExpr(args[2], out var replAst)
                ? replAst
                : PilotAstExpr.Const(args[2]);
            return PilotAstExpr.SubstituteSymbol(ast, sym, repl).FoldConstants().ToPilotObject();
        }, "ast", "symbol", "replacement");

        registry.Register("__ast_diff", (_, args) =>
        {
            if (args.Count < 2 || !TryUnwrapAstExpr(args[0], out var ast))
                return PilotValue.Null;
            string wrt = args[1].Kind == PilotValueKind.Keyword ? args[1].AsKeyword().Name : args[1].ToString().TrimStart(':');
            return PilotSymbolicDifferentiator.Differentiate(ast, wrt).FoldConstants().ToPilotObject();
        }, "ast", "wrt_var");

        registry.Register("__ast_eval", (ctx, args) =>
        {
            if (args.Count == 0 || !TryUnwrapAstExpr(args[0], out var ast))
                return PilotValue.Null;
            var env = new Dictionary<string, PilotValue>(StringComparer.Ordinal);
            if (args.Count > 1 && args[1].Kind == PilotValueKind.Map)
            {
                foreach (var kvp in args[1].AsMap())
                {
                    string k = kvp.Key.Kind == PilotValueKind.Keyword
                        ? kvp.Key.AsKeyword().Name
                        : kvp.Key.ToString().TrimStart(':');
                    env[k] = kvp.Value;
                }
            }
            return ast.Evaluate(env, ctx);
        }, "ast", "bindings");
    }

    private static bool TryUnwrapAstExpr(PilotValue val, out PilotAstExpr ast)
    {
        if (val.Kind == PilotValueKind.Object)
        {
            var obj = val.AsObject();
            if (obj.NativePayload is PilotAstExpr nativeAst)
            {
                ast = nativeAst;
                return true;
            }
            if (obj.Fields.TryGetValue("__ast_expr__", out var foreignVal) && foreignVal.AsForeign() is PilotAstExpr fAst)
            {
                ast = fAst;
                return true;
            }
        }
        ast = null!;
        return false;
    }

    #endregion

    #region 21.4 Elementary Math Intrinsics

    private static void RegisterElementaryMathIntrinsics(PilotBuiltinRegistry registry)
    {
        registry.Register("sin", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Sin(a[0].AsFloat()) : 0.0), "x");
        registry.Register("cos", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Cos(a[0].AsFloat()) : 1.0), "x");
        registry.Register("tan", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Tan(a[0].AsFloat()) : 0.0), "x");
        registry.Register("asin", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Asin(a[0].AsFloat()) : 0.0), "x");
        registry.Register("acos", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Acos(a[0].AsFloat()) : 0.0), "x");
        registry.Register("atan", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Atan(a[0].AsFloat()) : 0.0), "x");
        registry.Register("atan2", (_, a) => PilotValue.FromFloat(a.Count >= 2 ? Math.Atan2(a[0].AsFloat(), a[1].AsFloat()) : 0.0), "y", "x");
        registry.Register("sinh", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Sinh(a[0].AsFloat()) : 0.0), "x");
        registry.Register("cosh", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Cosh(a[0].AsFloat()) : 1.0), "x");
        registry.Register("tanh", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Tanh(a[0].AsFloat()) : 0.0), "x");
        registry.Register("exp", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Exp(a[0].AsFloat()) : 1.0), "x");
        registry.Register("ln", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Log(a[0].AsFloat()) : double.NegativeInfinity), "x");
        registry.Register("log", (_, a) =>
        {
            if (a.Count == 0) return PilotValue.FromFloat(double.NegativeInfinity);
            if (a.Count >= 2) return PilotValue.FromFloat(Math.Log(a[0].AsFloat(), a[1].AsFloat()));
            return PilotValue.FromFloat(Math.Log(a[0].AsFloat()));
        }, "x", "base_val");
        registry.Register("log2", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Log2(a[0].AsFloat()) : double.NegativeInfinity), "x");
        registry.Register("log10", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Log10(a[0].AsFloat()) : double.NegativeInfinity), "x");
        registry.Register("sqrt", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Sqrt(a[0].AsFloat()) : 0.0), "x");
        registry.Register("cbrt", (_, a) => PilotValue.FromFloat(a.Count > 0 ? Math.Cbrt(a[0].AsFloat()) : 0.0), "x");
        registry.Register("pow", (_, a) => PilotValue.FromFloat(a.Count >= 2 ? Math.Pow(a[0].AsFloat(), a[1].AsFloat()) : 0.0), "x", "y");
        registry.Register("hypot", (_, a) =>
        {
            if (a.Count < 2) return PilotValue.FromFloat(0.0);
            double dx = a[0].AsFloat();
            double dy = a[1].AsFloat();
            return PilotValue.FromFloat(Math.Sqrt(dx * dx + dy * dy));
        }, "x", "y");

        registry.Register("abs", (_, a) =>
        {
            if (a.Count == 0) return PilotValue.FromInt(0);
            return a[0].Kind == PilotValueKind.Int
                ? PilotValue.FromInt(Math.Abs(a[0].AsInt()))
                : PilotValue.FromFloat(Math.Abs(a[0].AsFloat()));
        }, "x");

        registry.Register("min", (_, a) =>
        {
            if (a.Count == 0) return PilotValue.Null;
            if (a.Count == 1 && a[0].Kind == PilotValueKind.List)
            {
                var list = a[0].AsList();
                if (list.Count == 0) return PilotValue.Null;
                var best = list[0];
                for (int i = 1; i < list.Count; i++)
                    if (list[i] < best) best = list[i];
                return best;
            }
            var m = a[0];
            for (int i = 1; i < a.Count; i++)
                if (a[i] < m) m = a[i];
            return m;
        }, "a", "b");

        registry.Register("max", (_, a) =>
        {
            if (a.Count == 0) return PilotValue.Null;
            if (a.Count == 1 && a[0].Kind == PilotValueKind.List)
            {
                var list = a[0].AsList();
                if (list.Count == 0) return PilotValue.Null;
                var best = list[0];
                for (int i = 1; i < list.Count; i++)
                    if (list[i] > best) best = list[i];
                return best;
            }
            var m = a[0];
            for (int i = 1; i < a.Count; i++)
                if (a[i] > m) m = a[i];
            return m;
        }, "a", "b");

        registry.Register("floor", (_, a) => PilotValue.FromInt(a.Count > 0 ? (long)Math.Floor(a[0].AsFloat()) : 0L), "x");
        registry.Register("ceil", (_, a) => PilotValue.FromInt(a.Count > 0 ? (long)Math.Ceiling(a[0].AsFloat()) : 0L), "x");
        registry.Register("trunc", (_, a) => PilotValue.FromInt(a.Count > 0 ? (long)Math.Truncate(a[0].AsFloat()) : 0L), "x");
        registry.Register("round", (_, a) =>
        {
            if (a.Count == 0) return PilotValue.FromInt(0);
            if (a.Count >= 2)
                return PilotValue.FromFloat(Math.Round(a[0].AsFloat(), (int)a[1].AsInt(), MidpointRounding.AwayFromZero));
            return PilotValue.FromInt((long)Math.Round(a[0].AsFloat(), MidpointRounding.AwayFromZero));
        }, "x", "digits");

        registry.Register("isnan", (_, a) => PilotValue.FromBool(a.Count > 0 && double.IsNaN(a[0].AsFloat())), "x");
        registry.Register("isinf", (_, a) => PilotValue.FromBool(a.Count > 0 && double.IsInfinity(a[0].AsFloat())), "x");
        registry.Register("isfinite", (_, a) => PilotValue.FromBool(a.Count > 0 && double.IsFinite(a[0].AsFloat())), "x");
    }

    #endregion

    #region 21.4 UTF-8 Zero-Copy Slice & DFA Regex Intrinsics

    private static unsafe void RegisterTextUtf8AndRegexIntrinsics(PilotBuiltinRegistry registry)
    {
        registry.Register("compile_regex", (_, args) =>
        {
            string pattern = args.Count > 0 ? args[0].AsString() : string.Empty;
            var dfa = PilotDfaRegex.Compile(pattern);
            return dfa.ToFrozenPilotMap();
        }, "pattern");

        registry.Register("__dfa_is_match", (_, args) =>
        {
            if (args.Count < 2) return PilotValue.False;
            var dfa = PilotDfaRegex.ResolveFromPilotValue(args[0]);
            if (TryExtractUtf8SliceBuffer(args[1], out var utf8Buf))
                return PilotValue.FromBool(dfa.IsMatchUtf8(utf8Buf));
            return PilotValue.FromBool(dfa.IsMatch(args[1].AsString()));
        }, "dfa_or_pattern", "text");

        registry.Register("__dfa_find", (_, args) =>
        {
            if (args.Count < 2) return PilotValue.Null;
            var dfa = PilotDfaRegex.ResolveFromPilotValue(args[0]);
            int startAt = args.Count >= 3 ? (int)args[2].AsInt() : 0;
            if (TryExtractUtf8SliceBuffer(args[1], out var utf8Buf))
            {
                if (!dfa.TryFindFirstUtf8(utf8Buf, startAt, out int bs, out int be))
                    return PilotValue.Null;
                string sub = Encoding.UTF8.GetString(new ReadOnlySpan<byte>((byte*)utf8Buf.DataPointer + bs, be - bs));
                return PilotValue.FromSome(PilotValue.FromTuple(new[]
                {
                    PilotValue.FromInt(bs),
                    PilotValue.FromInt(be),
                    PilotValue.FromString(sub)
                }));
            }

            string text = args[1].AsString();
            if (!dfa.TryFindFirst(text, startAt, out int s, out int e))
                return PilotValue.Null;

            var tup = new PilotValue[]
            {
                PilotValue.FromInt(s),
                PilotValue.FromInt(e),
                PilotValue.FromString(text.Substring(s, e - s))
            };
            return PilotValue.FromSome(PilotValue.FromTuple(tup));
        }, "dfa_or_pattern", "text", "start");

        registry.Register("__dfa_find_all", (_, args) =>
        {
            if (args.Count < 2) return PilotValue.FromList(new List<PilotValue>());
            var dfa = PilotDfaRegex.ResolveFromPilotValue(args[0]);
            var hits = dfa.FindAll(args[1].AsString());
            return PilotValue.FromList(hits.Select(h => PilotValue.FromString(h.Value)).ToList());
        }, "dfa_or_pattern", "text");

        registry.Register("__dfa_find_spans", (_, args) =>
        {
            if (args.Count < 2) return PilotValue.FromList(new List<PilotValue>());
            var dfa = PilotDfaRegex.ResolveFromPilotValue(args[0]);
            var hits = dfa.FindAll(args[1].AsString());
            return PilotValue.FromList(hits.Select(h =>
                PilotValue.FromTuple(new[]
                {
                    PilotValue.FromInt(h.Start),
                    PilotValue.FromInt(h.End),
                    PilotValue.FromString(h.Value)
                })).ToList());
        }, "dfa_or_pattern", "text");

        registry.Register("__dfa_replace_all", (_, args) =>
        {
            if (args.Count < 3) return args.Count >= 2 ? args[1] : PilotValue.FromString("");
            var dfa = PilotDfaRegex.ResolveFromPilotValue(args[0]);
            return PilotValue.FromString(dfa.ReplaceAll(args[1].AsString(), args[2].AsString()));
        }, "dfa_or_pattern", "text", "replacement");

        registry.Register("__dfa_split", (_, args) =>
        {
            if (args.Count < 2) return PilotValue.FromList(new List<PilotValue>());
            var dfa = PilotDfaRegex.ResolveFromPilotValue(args[0]);
            return PilotValue.FromList(dfa.Split(args[1].AsString()).Select(PilotValue.FromString).ToList());
        }, "dfa_or_pattern", "text");

        registry.Register("__utf8_encode_slice", (_, args) =>
        {
            string s = args.Count > 0 ? args[0].AsString() : string.Empty;
            byte[] utf8 = Encoding.UTF8.GetBytes(s);
            var buf = PilotSharedBuffer.AllocateUnmanaged((nuint)utf8.Length, PilotSharedElementKind.UInt8, (nuint)utf8.Length);
            if (utf8.Length > 0)
            {
                utf8.AsSpan().CopyTo(buf.AsSpan<byte>());
            }
            return PilotValue.FromSliceBuffer(buf);
        }, "text");

        registry.Register("__utf8_decode_slice", (ctx, args) =>
        {
            if (args.Count == 0 || args[0].Kind != PilotValueKind.SliceBuffer)
                return PilotValue.FromString(string.Empty);
            var buf = args[0].AsSliceBuffer();
            int offset = args.Count >= 2 ? (int)args[1].AsInt() : 0;
            int length = args.Count >= 3 ? (int)args[2].AsInt() : (buf.ByteLength > int.MaxValue ? int.MaxValue : (int)buf.ByteLength - offset);
            if (offset < 0) offset = 0;
            int totalBytes = (int)buf.ByteLength;
            if (offset >= totalBytes || length <= 0)
                return PilotValue.FromString(string.Empty);
            if (offset + length > totalBytes)
                length = totalBytes - offset;

            var span = new ReadOnlySpan<byte>((byte*)buf.DataPointer + offset, length);
            return PilotValue.FromString(Encoding.UTF8.GetString(span));
        }, "buf", "offset", "length");

        registry.Register("__utf8_is_valid_boundary", (_, args) =>
        {
            if (args.Count < 2 || args[0].Kind != PilotValueKind.SliceBuffer)
                return PilotValue.False;
            var buf = args[0].AsSliceBuffer();
            int idx = (int)args[1].AsInt();
            int totalBytes = (int)buf.ByteLength;
            if (idx < 0 || idx > totalBytes) return PilotValue.False;
            if (idx == 0 || idx == totalBytes) return PilotValue.True;
            byte b = *((byte*)buf.DataPointer + idx);
            // UTF-8 continuation bytes have top 2 bits == 10xxxxxx (0x80..0xBF)
            return PilotValue.FromBool((b & 0xC0) != 0x80);
        }, "buf", "byte_offset");

        registry.Register("__utf8_rune_count", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromInt(0);
            if (args[0].Kind == PilotValueKind.SliceBuffer)
            {
                var buf = args[0].AsSliceBuffer();
                int totalBytes = (int)buf.ByteLength;
                byte* ptr = (byte*)buf.DataPointer;
                int count = 0;
                for (int i = 0; i < totalBytes; i++)
                {
                    if ((ptr[i] & 0xC0) != 0x80)
                        count++;
                }
                return PilotValue.FromInt(count);
            }
            string s = args[0].AsString();
            return PilotValue.FromInt(s.EnumerateRunes().Count());
        }, "text_or_buf");

        registry.Register("__str_index_of", (_, args) =>
        {
            if (args.Count < 2) return PilotValue.FromInt(-1);
            string s = args[0].AsString();
            string sub = args[1].AsString();
            int start = args.Count >= 3 ? Math.Clamp((int)args[2].AsInt(), 0, s.Length) : 0;
            return PilotValue.FromInt(s.IndexOf(sub, start, StringComparison.Ordinal));
        }, "s", "sub", "start");

        registry.Register("__str_last_index_of", (_, args) =>
        {
            if (args.Count < 2) return PilotValue.FromInt(-1);
            string s = args[0].AsString();
            string sub = args[1].AsString();
            return PilotValue.FromInt(s.LastIndexOf(sub, StringComparison.Ordinal));
        }, "s", "sub");

        registry.Register("__str_slice", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromString("");
            string s = args[0].AsString();
            int start = args.Count >= 2 ? (int)args[1].AsInt() : 0;
            int end = args.Count >= 3 ? (int)args[2].AsInt() : s.Length;
            if (start < 0) start += s.Length;
            if (end < 0) end += s.Length;
            start = Math.Clamp(start, 0, s.Length);
            end = Math.Clamp(end, start, s.Length);
            return PilotValue.FromString(s[start..end]);
        }, "s", "start", "end");

        registry.Register("__str_split", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromList(new List<PilotValue>());
            string s = args[0].AsString();
            string sep = args.Count >= 2 ? args[1].AsString() : " ";
            if (string.IsNullOrEmpty(sep))
            {
                return PilotValue.FromList(s.Select(ch => PilotValue.FromString(ch.ToString())).ToList());
            }
            return PilotValue.FromList(s.Split(sep, StringSplitOptions.None).Select(PilotValue.FromString).ToList());
        }, "s", "sep");

        registry.Register("__str_trim", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromString("");
            string s = args[0].AsString();
            int mode = args.Count >= 2 ? (int)args[1].AsInt() : 0;
            return mode switch
            {
                -1 => PilotValue.FromString(s.TrimStart()),
                1 => PilotValue.FromString(s.TrimEnd()),
                _ => PilotValue.FromString(s.Trim())
            };
        }, "s", "mode");

        registry.Register("__str_case", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromString("");
            string s = args[0].AsString();
            bool upper = args.Count < 2 || args[1].IsTrue;
            return PilotValue.FromString(upper ? s.ToUpperInvariant() : s.ToLowerInvariant());
        }, "s", "upper");

        registry.Register("__str_replace", (_, args) =>
        {
            if (args.Count < 3) return args.Count > 0 ? args[0] : PilotValue.FromString("");
            return PilotValue.FromString(args[0].AsString().Replace(args[1].AsString(), args[2].AsString(), StringComparison.Ordinal));
        }, "s", "old_val", "new_val");

        registry.Register("__str_char_code", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromInt(-1);
            string s = args[0].AsString();
            int idx = args.Count >= 2 ? (int)args[1].AsInt() : 0;
            if (idx < 0 || idx >= s.Length) return PilotValue.FromInt(-1);
            return PilotValue.FromInt(s[idx]);
        }, "s", "idx");

        registry.Register("__char_from_code", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromString("");
            int code = (int)args[0].AsInt();
            if (code < 0 || code > 0x10FFFF) return PilotValue.FromString("");
            return PilotValue.FromString(char.ConvertFromUtf32(code));
        }, "code");
    }

    #endregion

    private static bool TryExtractUtf8SliceBuffer(PilotValue val, out PilotSharedBuffer buf)
    {
        if (val.Kind == PilotValueKind.SliceBuffer && val.AsSliceBuffer().ElementKind == PilotSharedElementKind.UInt8)
        {
            buf = val.AsSliceBuffer();
            return true;
        }
        if (val.Kind == PilotValueKind.Object &&
            val.AsObject().Fields.TryGetValue("_buf", out var inner) &&
            inner.Kind == PilotValueKind.SliceBuffer &&
            inner.AsSliceBuffer().ElementKind == PilotSharedElementKind.UInt8)
        {
            buf = inner.AsSliceBuffer();
            return true;
        }
        buf = null!;
        return false;
    }

    #region 21.5 Fallible Filesystem, MMap & OS Intrinsics

    private static void RegisterFilesystemAndOsIntrinsics(PilotBuiltinRegistry registry)
    {
        registry.Register("__os_is_windows", (_, _) =>
            PilotValue.FromBool(RuntimeInformation.IsOSPlatform(OSPlatform.Windows)));

        registry.Register("__os_cwd", (_, _) =>
            PilotValue.FromString(Directory.GetCurrentDirectory()));

        registry.Register("__os_temp_dir", (_, _) =>
            PilotValue.FromString(Path.GetTempPath()));

        registry.Register("__fs_exists", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.False;
            string p = args[0].AsString();
            if (string.IsNullOrWhiteSpace(p)) return PilotValue.False;
            if (PilotEffectDryRunEngine.TryReadFileContent(p, out var shadow))
                return PilotValue.FromBool(shadow != null);
            return PilotValue.FromBool(File.Exists(p) || Directory.Exists(p));
        }, "path");

        registry.Register("__fs_is_file", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.False;
            string p = args[0].AsString();
            if (string.IsNullOrWhiteSpace(p)) return PilotValue.False;
            if (PilotEffectDryRunEngine.TryReadFileContent(p, out var shadow))
                return PilotValue.FromBool(shadow != null);
            return PilotValue.FromBool(File.Exists(p));
        }, "path");

        registry.Register("__fs_is_dir", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.False;
            string p = args[0].AsString();
            return PilotValue.FromBool(!string.IsNullOrWhiteSpace(p) && Directory.Exists(p));
        }, "path");

        registry.Register("__fs_file_size", (ctx, args) =>
        {
            if (args.Count == 0 || string.IsNullOrWhiteSpace(args[0].AsString()))
                return CreateIoErrorFailure(ctx, "InvalidPath", "empty path");
            string p = args[0].AsString();
            try
            {
                if (PilotEffectDryRunEngine.TryReadFileContent(p, out var shadow))
                {
                    if (shadow == null) return CreateIoErrorFailure(ctx, "NotFound", p);
                    return PilotValue.FromInt(Encoding.UTF8.GetByteCount(shadow));
                }
                var fi = new FileInfo(p);
                if (!fi.Exists) return CreateIoErrorFailure(ctx, "NotFound", p);
                return PilotValue.FromInt(fi.Length);
            }
            catch (Exception ex)
            {
                return MapIoException(ctx, ex, p);
            }
        }, "path");

        registry.Register("__fs_read_text", (ctx, args) =>
        {
            if (args.Count == 0 || string.IsNullOrWhiteSpace(args[0].AsString()))
                return CreateIoErrorFailure(ctx, "InvalidPath", "empty path");
            string p = args[0].AsString();
            try
            {
                if (PilotEffectDryRunEngine.TryReadFileContent(p, out var shadow))
                {
                    if (shadow == null) return CreateIoErrorFailure(ctx, "NotFound", p);
                    return PilotValue.FromString(shadow);
                }
                if (!File.Exists(p))
                    return CreateIoErrorFailure(ctx, "NotFound", p);
                return PilotValue.FromString(File.ReadAllText(p, Encoding.UTF8));
            }
            catch (Exception ex)
            {
                return MapIoException(ctx, ex, p);
            }
        }, "path");

        registry.Register("__fs_read_bytes", (ctx, args) =>
        {
            if (args.Count == 0 || string.IsNullOrWhiteSpace(args[0].AsString()))
                return CreateIoErrorFailure(ctx, "InvalidPath", "empty path");
            string p = args[0].AsString();
            try
            {
                byte[] bytes;
                if (PilotEffectDryRunEngine.TryReadFileContent(p, out var shadow))
                {
                    if (shadow == null) return CreateIoErrorFailure(ctx, "NotFound", p);
                    bytes = Encoding.UTF8.GetBytes(shadow);
                }
                else
                {
                    if (!File.Exists(p))
                        return CreateIoErrorFailure(ctx, "NotFound", p);
                    bytes = File.ReadAllBytes(p);
                }
                var buf = PilotSharedBuffer.AllocateUnmanaged((nuint)bytes.Length, PilotSharedElementKind.UInt8, (nuint)bytes.Length);
                if (bytes.Length > 0)
                    bytes.AsSpan().CopyTo(buf.AsSpan<byte>());
                return PilotValue.FromSliceBuffer(buf);
            }
            catch (Exception ex)
            {
                return MapIoException(ctx, ex, p);
            }
        }, "path");

        registry.Register("__fs_write_text", (ctx, args) =>
        {
            if (args.Count < 2 || string.IsNullOrWhiteSpace(args[0].AsString()))
                return CreateIoErrorFailure(ctx, "InvalidPath", "empty path");
            string p = args[0].AsString();
            string content = args[1].AsString();
            bool append = args.Count >= 3 && args[2].IsTrue;
            try
            {
                if (PilotEffectDryRunEngine.TryRecordFileWrite(p, content, append))
                    return PilotValue.FromInt(Encoding.UTF8.GetByteCount(content));

                string full = Path.GetFullPath(p);
                string? dir = Path.GetDirectoryName(full);
                if (!string.IsNullOrEmpty(dir) && !Directory.Exists(dir))
                    return CreateIoErrorFailure(ctx, "NotFound", dir);

                if (append)
                    File.AppendAllText(full, content, Utf8NoBom);
                else
                    File.WriteAllText(full, content, Utf8NoBom);

                return PilotValue.FromInt(Encoding.UTF8.GetByteCount(content));
            }
            catch (Exception ex)
            {
                return MapIoException(ctx, ex, p);
            }
        }, "path", "content", "append");

        registry.Register("__fs_remove_file", (ctx, args) =>
        {
            if (args.Count == 0 || string.IsNullOrWhiteSpace(args[0].AsString()))
                return CreateIoErrorFailure(ctx, "InvalidPath", "empty path");
            string p = args[0].AsString();
            try
            {
                if (PilotEffectDryRunEngine.TryRecordFileDelete(p))
                    return PilotValue.True;

                if (!File.Exists(p))
                    return CreateIoErrorFailure(ctx, "NotFound", p);
                File.Delete(p);
                return PilotValue.True;
            }
            catch (Exception ex)
            {
                return MapIoException(ctx, ex, p);
            }
        }, "path");

        registry.Register("__fs_copy_file", (ctx, args) =>
        {
            if (args.Count < 2 || string.IsNullOrWhiteSpace(args[0].AsString()) || string.IsNullOrWhiteSpace(args[1].AsString()))
                return CreateIoErrorFailure(ctx, "InvalidPath", "empty source or destination path");
            string src = args[0].AsString();
            string dst = args[1].AsString();
            bool overwrite = args.Count >= 3 && args[2].IsTrue;
            try
            {
                if (!File.Exists(src))
                    return CreateIoErrorFailure(ctx, "NotFound", src);
                if (!overwrite && File.Exists(dst))
                    return CreateIoErrorFailure(ctx, "AlreadyExists", dst);
                if (PilotEffectDryRunEngine.IsDryRunActive)
                {
                    string data = File.ReadAllText(src, Encoding.UTF8);
                    PilotEffectDryRunEngine.TryRecordFileWrite(dst, data, append: false);
                    return PilotValue.True;
                }
                File.Copy(src, dst, overwrite);
                return PilotValue.True;
            }
            catch (Exception ex)
            {
                return MapIoException(ctx, ex, dst);
            }
        }, "src", "dst", "overwrite");

        registry.Register("__fs_move_file", (ctx, args) =>
        {
            if (args.Count < 2 || string.IsNullOrWhiteSpace(args[0].AsString()) || string.IsNullOrWhiteSpace(args[1].AsString()))
                return CreateIoErrorFailure(ctx, "InvalidPath", "empty source or destination path");
            string src = args[0].AsString();
            string dst = args[1].AsString();
            bool overwrite = args.Count >= 3 && args[2].IsTrue;
            try
            {
                if (!File.Exists(src))
                    return CreateIoErrorFailure(ctx, "NotFound", src);
                if (!overwrite && File.Exists(dst))
                    return CreateIoErrorFailure(ctx, "AlreadyExists", dst);
                if (PilotEffectDryRunEngine.IsDryRunActive)
                {
                    string data = File.ReadAllText(src, Encoding.UTF8);
                    PilotEffectDryRunEngine.TryRecordFileWrite(dst, data, append: false);
                    PilotEffectDryRunEngine.TryRecordFileDelete(src);
                    return PilotValue.True;
                }
                File.Move(src, dst, overwrite);
                return PilotValue.True;
            }
            catch (Exception ex)
            {
                return MapIoException(ctx, ex, dst);
            }
        }, "src", "dst", "overwrite");

        registry.Register("__fs_mkdir", (ctx, args) =>
        {
            if (args.Count == 0 || string.IsNullOrWhiteSpace(args[0].AsString()))
                return CreateIoErrorFailure(ctx, "InvalidPath", "empty path");
            string p = args[0].AsString();
            bool recursive = args.Count < 2 || args[1].IsTrue;
            try
            {
                if (!recursive)
                {
                    string? parent = Path.GetDirectoryName(Path.GetFullPath(p));
                    if (!string.IsNullOrEmpty(parent) && !Directory.Exists(parent))
                        return CreateIoErrorFailure(ctx, "NotFound", parent);
                }
                if (!PilotEffectDryRunEngine.IsDryRunActive)
                {
                    Directory.CreateDirectory(p);
                }
                return PilotValue.True;
            }
            catch (Exception ex)
            {
                return MapIoException(ctx, ex, p);
            }
        }, "path", "recursive");

        registry.Register("__fs_rmdir", (ctx, args) =>
        {
            if (args.Count == 0 || string.IsNullOrWhiteSpace(args[0].AsString()))
                return CreateIoErrorFailure(ctx, "InvalidPath", "empty path");
            string p = args[0].AsString();
            bool recursive = args.Count >= 2 && args[1].IsTrue;
            try
            {
                if (!Directory.Exists(p))
                    return CreateIoErrorFailure(ctx, "NotFound", p);
                if (!PilotEffectDryRunEngine.IsDryRunActive)
                {
                    Directory.Delete(p, recursive);
                }
                return PilotValue.True;
            }
            catch (Exception ex)
            {
                return MapIoException(ctx, ex, p);
            }
        }, "path", "recursive");

        registry.Register("__fs_list_dir", (ctx, args) =>
        {
            if (args.Count == 0 || string.IsNullOrWhiteSpace(args[0].AsString()))
                return CreateIoErrorFailure(ctx, "InvalidPath", "empty path");
            string p = args[0].AsString();
            try
            {
                if (!Directory.Exists(p))
                    return CreateIoErrorFailure(ctx, "NotFound", p);
                var entries = Directory.GetFileSystemEntries(p)
                    .OrderBy(e => e, StringComparer.Ordinal)
                    .Select(e => PilotValue.FromString(e.Replace('\\', '/')))
                    .ToList();
                return PilotValue.FromList(entries);
            }
            catch (Exception ex)
            {
                return MapIoException(ctx, ex, p);
            }
        }, "path");

        registry.Register("__fs_walk_dir", (ctx, args) =>
        {
            if (args.Count == 0 || string.IsNullOrWhiteSpace(args[0].AsString()))
                return CreateIoErrorFailure(ctx, "InvalidPath", "empty path");
            string p = args[0].AsString();
            try
            {
                if (!Directory.Exists(p))
                    return CreateIoErrorFailure(ctx, "NotFound", p);
                var entries = Directory.EnumerateFileSystemEntries(p, "*", SearchOption.AllDirectories)
                    .OrderBy(e => e, StringComparer.Ordinal)
                    .Select(e => PilotValue.FromString(e.Replace('\\', '/')))
                    .ToList();
                return PilotValue.FromList(entries);
            }
            catch (Exception ex)
            {
                return MapIoException(ctx, ex, p);
            }
        }, "path");

        registry.Register("__fs_mmap_read", (ctx, args) =>
        {
            if (args.Count == 0 || string.IsNullOrWhiteSpace(args[0].AsString()))
                return CreateIoErrorFailure(ctx, "InvalidPath", "empty path");
            string p = args[0].AsString();
            string elemType = args.Count >= 2 ? args[1].AsString() : "u8";
            try
            {
                if (!File.Exists(p))
                    return CreateIoErrorFailure(ctx, "NotFound", p);
                var kind = PilotSharedBuffer.ParseElementKind(elemType);
                var buf = PilotSharedBuffer.MapExistingFileReadOnly(p, kind);
                return PilotValue.FromSliceBuffer(buf);
            }
            catch (Exception ex)
            {
                return MapIoException(ctx, ex, p);
            }
        }, "path", "elem_type");

        registry.Register("__fs_mmap_rw", (ctx, args) =>
        {
            if (args.Count < 2 || string.IsNullOrWhiteSpace(args[0].AsString()))
                return CreateIoErrorFailure(ctx, "InvalidPath", "empty path");
            string p = args[0].AsString();
            nuint cap = (nuint)Math.Max(1L, args[1].AsInt());
            string elemType = args.Count >= 3 ? args[2].AsString() : "u8";
            try
            {
                var kind = PilotSharedBuffer.ParseElementKind(elemType);
                nuint initLen = args.Count >= 4 ? (nuint)Math.Max(0L, args[3].AsInt()) : cap;
                var buf = PilotSharedBuffer.CreateOrOpenShared(Path.GetFullPath(p), cap, kind, initLen);
                return PilotValue.FromSliceBuffer(buf);
            }
            catch (Exception ex)
            {
                return MapIoException(ctx, ex, p);
            }
        }, "path", "capacity", "elem_type", "initial_length");
    }

    #endregion

    #region 24.2 Zero-Copy Byte Slice, Strict UTF-8 & Format Parser Intrinsics

    private static unsafe void RegisterCodecAndFormatIntrinsics(PilotBuiltinRegistry registry)
    {
        registry.Register("__slice_byte_at", (_, args) =>
        {
            if (args.Count < 2 || args[0].Kind != PilotValueKind.SliceBuffer)
                return PilotValue.FromInt(-1);
            var buf = args[0].AsSliceBuffer();
            int totalBytes = buf.ByteLength > int.MaxValue ? int.MaxValue : (int)buf.ByteLength;
            int idx = (int)args[1].AsInt();
            if (idx < 0) idx += totalBytes;
            if (idx < 0 || idx >= totalBytes)
                return PilotValue.FromInt(-1);
            if (PilotEffectDryRunEngine.TryReadSliceBufferElement(buf, idx, out var sv))
                return PilotValue.FromInt(sv.AsInt() & 0xFF);
            byte b = *((byte*)buf.DataPointer + idx);
            return PilotValue.FromInt(b);
        }, "buf", "idx");

        registry.Register("__utf8_validate_slice", (_, args) =>
        {
            if (args.Count == 0 || args[0].Kind != PilotValueKind.SliceBuffer)
                return PilotValue.FromInt(0);
            var buf = args[0].AsSliceBuffer();
            int totalBytes = buf.ByteLength > int.MaxValue ? int.MaxValue : (int)buf.ByteLength;
            int offset = args.Count >= 2 ? (int)args[1].AsInt() : 0;
            int length = args.Count >= 3 && args[2].AsInt() >= 0
                ? (int)args[2].AsInt()
                : totalBytes - offset;
            if (offset < 0 || offset > totalBytes) return PilotValue.FromInt(0);
            if (length <= 0) return PilotValue.FromInt(-1);
            if (offset + length > totalBytes) length = totalBytes - offset;

            byte* ptr = (byte*)buf.DataPointer + offset;
            int i = 0;
            while (i < length)
            {
                byte b0 = ptr[i];
                if (b0 < 0x80)
                {
                    i++;
                }
                else if (b0 is >= 0xC2 and <= 0xDF)
                {
                    if (i + 1 >= length || (ptr[i + 1] & 0xC0) != 0x80)
                        return PilotValue.FromInt(offset + i);
                    i += 2;
                }
                else if (b0 is >= 0xE0 and <= 0xEF)
                {
                    if (i + 2 >= length)
                        return PilotValue.FromInt(offset + i);
                    byte b1 = ptr[i + 1];
                    byte b2 = ptr[i + 2];
                    if ((b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80)
                        return PilotValue.FromInt(offset + i);
                    if (b0 == 0xE0 && b1 < 0xA0) // overlong 3-byte
                        return PilotValue.FromInt(offset + i);
                    if (b0 == 0xED && b1 >= 0xA0) // UTF-16 surrogate half
                        return PilotValue.FromInt(offset + i);
                    i += 3;
                }
                else if (b0 is >= 0xF0 and <= 0xF4)
                {
                    if (i + 3 >= length)
                        return PilotValue.FromInt(offset + i);
                    byte b1 = ptr[i + 1];
                    byte b2 = ptr[i + 2];
                    byte b3 = ptr[i + 3];
                    if ((b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80 || (b3 & 0xC0) != 0x80)
                        return PilotValue.FromInt(offset + i);
                    if (b0 == 0xF0 && b1 < 0x90) // overlong 4-byte
                        return PilotValue.FromInt(offset + i);
                    if (b0 == 0xF4 && b1 >= 0x90) // > U+10FFFF
                        return PilotValue.FromInt(offset + i);
                    i += 4;
                }
                else
                {
                    return PilotValue.FromInt(offset + i);
                }
            }
            return PilotValue.FromInt(-1);
        }, "buf", "offset", "length");

        registry.Register("__parse_int_strict", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.Null;
            string raw = args[0].AsString().Trim();
            int radix = args.Count >= 2 ? (int)args[1].AsInt() : 10;
            if (string.IsNullOrEmpty(raw)) return PilotValue.Null;

            if (radix == 10)
            {
                if (long.TryParse(raw, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out long val10))
                    return PilotValue.FromSome(PilotValue.FromInt(val10));
                return PilotValue.Null;
            }

            if (radix is not (2 or 8 or 16))
                return PilotValue.Null;

            bool neg = false;
            int idx = 0;
            if (raw[0] == '+' || raw[0] == '-')
            {
                neg = raw[0] == '-';
                idx = 1;
            }
            if (idx >= raw.Length) return PilotValue.Null;

            ulong acc = 0;
            ulong uRadix = (ulong)radix;
            ulong maxMag = neg ? (ulong)long.MaxValue + 1UL : (ulong)long.MaxValue;
            for (; idx < raw.Length; idx++)
            {
                char ch = raw[idx];
                int digit = ch switch
                {
                    >= '0' and <= '9' => ch - '0',
                    >= 'a' and <= 'f' => 10 + (ch - 'a'),
                    >= 'A' and <= 'F' => 10 + (ch - 'A'),
                    _ => -1
                };
                if (digit < 0 || digit >= radix)
                    return PilotValue.Null;
                if (acc > (maxMag - (ulong)digit) / uRadix)
                    return PilotValue.Null;
                acc = acc * uRadix + (ulong)digit;
            }

            long signedVal = neg
                ? (acc == (ulong)long.MaxValue + 1UL ? long.MinValue : -(long)acc)
                : (long)acc;
            return PilotValue.FromSome(PilotValue.FromInt(signedVal));
        }, "raw", "radix");

        registry.Register("__parse_float_strict", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.Null;
            string raw = args[0].AsString().Trim();
            if (string.IsNullOrEmpty(raw)) return PilotValue.Null;

            return raw switch
            {
                "inf" or "+inf" => PilotValue.FromSome(PilotValue.FromFloat(double.PositiveInfinity)),
                "-inf" => PilotValue.FromSome(PilotValue.FromFloat(double.NegativeInfinity)),
                "nan" or "+nan" or "-nan" => PilotValue.FromSome(PilotValue.FromFloat(double.NaN)),
                _ => double.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out double d)
                    ? PilotValue.FromSome(PilotValue.FromFloat(d))
                    : PilotValue.Null
            };
        }, "raw");

        registry.Register("__map_entries_sorted", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromList(new List<PilotValue>());
            if (args[0].Kind == PilotValueKind.Map)
            {
                var pairs = args[0].AsMap()
                    .Select(kvp => (
                        KeyStr: kvp.Key.Kind == PilotValueKind.Keyword ? kvp.Key.AsKeyword().Name : kvp.Key.AsString(),
                        Val: kvp.Value))
                    .OrderBy(p => p.KeyStr, StringComparer.Ordinal)
                    .Select(p => PilotValue.FromTuple(new[] { PilotValue.FromString(p.KeyStr), p.Val }))
                    .ToList();
                return PilotValue.FromList(pairs);
            }
            if (args[0].Kind == PilotValueKind.Object)
            {
                var pairs = args[0].AsObject().Fields
                    .Where(kvp => !kvp.Key.StartsWith("__", StringComparison.Ordinal))
                    .OrderBy(kvp => kvp.Key, StringComparer.Ordinal)
                    .Select(kvp => PilotValue.FromTuple(new[] { PilotValue.FromString(kvp.Key), kvp.Value }))
                    .ToList();
                return PilotValue.FromList(pairs);
            }
            return PilotValue.FromList(new List<PilotValue>());
        }, "map_or_obj");
    }

    #endregion
}
