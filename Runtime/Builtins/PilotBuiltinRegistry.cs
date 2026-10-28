using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Builtins;

/// <summary>
/// Registry of all default and user-extended built-in functions in Pilot.
/// Fully configurable and decoupled.
/// </summary>
public sealed class PilotBuiltinRegistry
{
    private readonly Dictionary<string, PilotFunction> _functions = new(StringComparer.Ordinal);
    private static long _gensymCounter = 1000;

    public PilotBuiltinRegistry()
    {
        RegisterDefaults();
    }

    public void Register(string name, Func<PilotExecutionContext, IReadOnlyList<PilotValue>, PilotValue> func, params string[] paramNames)
    {
        _functions[name] = PilotFunction.CreateNative(name, func, paramNames);
    }

    public void Register(IBuiltinFunction builtin)
    {
        _functions[builtin.Name] = PilotFunction.CreateNative(
            builtin.Name,
            builtin.Call,
            builtin.ParameterNames.ToArray());
    }

    public bool TryGet(string name, out PilotFunction func) =>
        _functions.TryGetValue(name, out func!);

    public IReadOnlyDictionary<string, PilotFunction> GetAll() => _functions;

    private void RegisterDefaults()
    {
        // I/O
        Register("print", (ctx, args) =>
        {
            string text = string.Join(" ", args.Select(a => a.ToString()));
            ctx.Options.StandardOutput.Write(text);
            return PilotValue.Null;
        }, "text");

        Register("println", (ctx, args) =>
        {
            string text = string.Join(" ", args.Select(a => a.ToString()));
            ctx.Options.StandardOutput.WriteLine(text);
            return PilotValue.Null;
        }, "text");

        // Collections & Introspection
        Register("len", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromInt(0);
            var val = args[0];
            return val.Kind switch
            {
                PilotValueKind.String => PilotValue.FromInt(((string)val.AsString()).Length),
                PilotValueKind.List => PilotValue.FromInt(val.AsList().Count),
                PilotValueKind.Tuple => PilotValue.FromInt(val.AsTuple().Count),
                PilotValueKind.Map => PilotValue.FromInt(val.AsMap().Count),
                PilotValueKind.Set => PilotValue.FromInt(val.AsSet().Count),
                PilotValueKind.Range => PilotValue.FromInt(val.AsRange().Count),
                PilotValueKind.Matrix => PilotValue.FromInt(val.AsMatrix().Rows * val.AsMatrix().Cols),
                PilotValueKind.NativeArray => PilotValue.FromInt(val.AsNativeArray().Length),
                PilotValueKind.NativeMap => PilotValue.FromInt(val.AsNativeMap().Count),
                _ => PilotValue.FromInt(0)
            };
        }, "collection");

        Register("type_of", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromString("undefined");
            return PilotValue.FromString(args[0].Kind.ToString().ToLowerInvariant());
        }, "value");

        Register("some", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromSome(PilotValue.Null);
            return PilotValue.FromSome(args[0]);
        }, "value");

        Register("fail", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromFailure(PilotValue.FromString("error"));
            return PilotValue.FromFailure(args[0]);
        }, "error");

        Register("ok", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.Null;
            return args[0];
        }, "value");

        Register("ignore_err", (_, _) => PilotValue.Null, "result");

        Register("panic", (_, args) =>
        {
            string msg = args.Count > 0 ? args[0].ToString() : "Explicit runtime panic";
            throw new PilotRuntimeException($"[ERR005] {msg}");
        }, "message");

        Register("sizeof_type", (ctx, args) =>
        {
            if (args.Count == 0) return PilotValue.FromInt(8);
            string typeSpec = args[0].ToString().Trim();
            if (PilotAdtMemoryLayout.TrySplitFallibleTypeSpec(typeSpec, out string okType, out string errType))
            {
                var (resSz, _, _, _) = PilotAdtMemoryLayout.ComputeResultLayout(
                    okType,
                    errType,
                    ctx.AdtDefinitions,
                    ctx.RecordDefinitions);
                return PilotValue.FromInt(resSz);
            }
            if (typeSpec.EndsWith("?", StringComparison.Ordinal) && typeSpec.Length > 1)
            {
                var (sz, _, _) = PilotAdtMemoryLayout.ComputeOptionLayout(
                    typeSpec[..^1],
                    ctx.AdtDefinitions,
                    ctx.RecordDefinitions);
                return PilotValue.FromInt(sz);
            }
            if ((typeSpec.StartsWith("Option[", StringComparison.OrdinalIgnoreCase) && typeSpec.EndsWith("]", StringComparison.Ordinal)) ||
                (typeSpec.StartsWith("Option<", StringComparison.OrdinalIgnoreCase) && typeSpec.EndsWith(">", StringComparison.Ordinal)))
            {
                var (sz, _, _) = PilotAdtMemoryLayout.ComputeOptionLayout(
                    typeSpec[7..^1],
                    ctx.AdtDefinitions,
                    ctx.RecordDefinitions);
                return PilotValue.FromInt(sz);
            }
            var (baseSz, _, _) = PilotAdtMemoryLayout.ComputeOptionLayout(
                typeSpec,
                ctx.AdtDefinitions,
                ctx.RecordDefinitions);
            // If `typeSpec` has a zero niche, `ComputeOptionLayout(typeSpec)` already equals `sizeof(typeSpec)`.
            // Otherwise recompute via single-field dummy record layout.
            if (PilotAdtMemoryLayout.HasZeroAllocationNiche(typeSpec, ctx.AdtDefinitions, ctx.RecordDefinitions))
            {
                return PilotValue.FromInt(baseSz);
            }
            var dummyRec = new PilotRecordDefinition("__sizeof", new[] { new PilotAdtFieldDefinition("v", typeSpec, 0) }, Array.Empty<string>());
            var layout = PilotAdtMemoryLayout.ComputeForRecord(dummyRec, ctx.AdtDefinitions, ctx.RecordDefinitions);
            return PilotValue.FromInt(layout.TotalSizeBytes);
        }, "type_name");

        Register("option_has_niche", (ctx, args) =>
        {
            if (args.Count == 0) return PilotValue.False;
            string typeSpec = args[0].ToString().Trim();
            if (typeSpec.EndsWith("?", StringComparison.Ordinal) && typeSpec.Length > 1)
            {
                typeSpec = typeSpec[..^1].Trim();
            }
            else if ((typeSpec.StartsWith("Option[", StringComparison.OrdinalIgnoreCase) && typeSpec.EndsWith("]", StringComparison.Ordinal)) ||
                     (typeSpec.StartsWith("Option<", StringComparison.OrdinalIgnoreCase) && typeSpec.EndsWith(">", StringComparison.Ordinal)))
            {
                typeSpec = typeSpec[7..^1].Trim();
            }
            return PilotValue.FromBool(PilotAdtMemoryLayout.HasZeroAllocationNiche(typeSpec, ctx.AdtDefinitions, ctx.RecordDefinitions));
        }, "type_name");

        Register("assert", (_, args) =>
        {
            if (args.Count == 0 || !args[0].IsTrue)
            {
                string msg = args.Count > 1 ? args[1].ToString() : "Assertion failed.";
                throw new PilotRuntimeException(msg);
            }
            return PilotValue.True;
        }, "condition", "message");

        Register("clock", (_, _) =>
            PilotValue.FromFloat(Stopwatch.GetTimestamp() / (double)Stopwatch.Frequency * 1000.0));

        Register("range", (_, args) =>
        {
            if (args.Count == 1) return PilotValue.FromRange(new PilotRange(0, args[0].AsInt()));
            if (args.Count == 2) return PilotValue.FromRange(new PilotRange(args[0].AsInt(), args[1].AsInt()));
            if (args.Count >= 3) return PilotValue.FromRange(new PilotRange(args[0].AsInt(), args[1].AsInt(), args[2].AsInt()));
            return PilotValue.FromRange(new PilotRange(0, 0));
        }, "start", "end", "step");

        // High-Performance Native Types (Burst / HPC# / SIMD)
        Register("natarray", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromNativeArray(new Pilot.Runtime.NativeMemory.PilotNativeArray(0));
            var first = args[0];
            if (first.Kind == PilotValueKind.List) return PilotValue.FromNativeArray(new Pilot.Runtime.NativeMemory.PilotNativeArray(first.AsList()));
            if (first.Kind == PilotValueKind.Tuple) return PilotValue.FromNativeArray(new Pilot.Runtime.NativeMemory.PilotNativeArray(first.AsTuple()));
            if (first.Kind == PilotValueKind.NativeArray) return first;
            int len = (int)first.AsInt();
            return PilotValue.FromNativeArray(new Pilot.Runtime.NativeMemory.PilotNativeArray(len));
        }, "size_or_elements");

        Register("natmap", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromNativeMap(new Pilot.Runtime.NativeMemory.PilotNativeMap());
            var first = args[0];
            if (first.Kind == PilotValueKind.Map) return PilotValue.FromNativeMap(new Pilot.Runtime.NativeMemory.PilotNativeMap(first.AsMap()));
            if (first.Kind == PilotValueKind.NativeMap) return first;
            int cap = (int)first.AsInt();
            return PilotValue.FromNativeMap(new Pilot.Runtime.NativeMemory.PilotNativeMap(cap));
        }, "capacity_or_dict");

        Register("to_native", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.Null;
            var val = args[0];
            if (val.Kind == PilotValueKind.List) return PilotValue.FromNativeArray(new Pilot.Runtime.NativeMemory.PilotNativeArray(val.AsList()));
            if (val.Kind == PilotValueKind.Tuple) return PilotValue.FromNativeArray(new Pilot.Runtime.NativeMemory.PilotNativeArray(val.AsTuple()));
            if (val.Kind == PilotValueKind.Map) return PilotValue.FromNativeMap(new Pilot.Runtime.NativeMemory.PilotNativeMap(val.AsMap()));
            return val;
        }, "val");

        Register("as_native", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.Null;
            var val = args[0];
            if (val.Kind == PilotValueKind.List) return PilotValue.FromNativeArray(new Pilot.Runtime.NativeMemory.PilotNativeArray(val.AsList()));
            if (val.Kind == PilotValueKind.Tuple) return PilotValue.FromNativeArray(new Pilot.Runtime.NativeMemory.PilotNativeArray(val.AsTuple()));
            if (val.Kind == PilotValueKind.Map) return PilotValue.FromNativeMap(new Pilot.Runtime.NativeMemory.PilotNativeMap(val.AsMap()));
            return val;
        }, "val");

        // Matrices
        Register("matrix", (_, args) =>
        {
            int r = args.Count > 0 ? (int)args[0].AsInt() : 1;
            int c = args.Count > 1 ? (int)args[1].AsInt() : r;
            return PilotValue.FromMatrix(new PilotMatrix(r, c));
        }, "rows", "cols");

        Register("eye", (_, args) =>
        {
            int n = args.Count > 0 ? (int)args[0].AsInt() : 3;
            return PilotValue.FromMatrix(PilotMatrix.Identity(n));
        }, "size");

        Register("zeros", (_, args) =>
        {
            int r = args.Count > 0 ? (int)args[0].AsInt() : 3;
            int c = args.Count > 1 ? (int)args[1].AsInt() : r;
            return PilotValue.FromMatrix(PilotMatrix.Zeros(r, c));
        }, "rows", "cols");

        Register("ones", (_, args) =>
        {
            int r = args.Count > 0 ? (int)args[0].AsInt() : 3;
            int c = args.Count > 1 ? (int)args[1].AsInt() : r;
            return PilotValue.FromMatrix(PilotMatrix.Ones(r, c));
        }, "rows", "cols");

        // Type Casts
        Register("str", (_, args) => args.Count > 0 ? PilotValue.FromString(args[0].ToString()) : PilotValue.FromString(""), "val");
        Register("int", (_, args) => args.Count > 0 ? PilotValue.FromInt(args[0].AsInt()) : PilotValue.FromInt(0), "val");
        Register("float", (_, args) => args.Count > 0 ? PilotValue.FromFloat(args[0].AsFloat()) : PilotValue.FromFloat(0.0), "val");
        Register("bool", (_, args) => args.Count > 0 ? PilotValue.FromBool(args[0].IsTrue) : PilotValue.False, "val");

        // Lisp Metaprogramming primitives
        Register("gensym", (_, args) =>
        {
            string prefix = args.Count > 0 ? args[0].ToString() : "sym";
            long id = System.Threading.Interlocked.Increment(ref _gensymCounter);
            return PilotValue.FromString($"{prefix}__{id}");
        }, "prefix");

        Register("cons", (_, args) =>
        {
            if (args.Count < 2) return PilotValue.Null;
            var list = new List<PilotValue> { args[0] };
            if (args[1].Kind == PilotValueKind.List)
            {
                list.AddRange(args[1].AsList());
            }
            else
            {
                list.Add(args[1]);
            }
            return PilotValue.FromList(list);
        }, "head", "tail");

        Register("car", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.Null;
            if (args[0].Kind == PilotValueKind.List)
            {
                var l = args[0].AsList();
                return l.Count > 0 ? l[0] : PilotValue.Null;
            }
            return args[0];
        }, "list");

        Register("cdr", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromList(new List<PilotValue>());
            if (args[0].Kind == PilotValueKind.List)
            {
                var l = args[0].AsList();
                return PilotValue.FromList(l.Skip(1).ToList());
            }
            return PilotValue.FromList(new List<PilotValue>());
        }, "list");

        Register("atom", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.True;
            return PilotValue.FromBool(args[0].Kind != PilotValueKind.List && args[0].Kind != PilotValueKind.Tuple);
        }, "val");

        Register("symbol", (_, args) =>
        {
            string s = args.Count > 0 ? args[0].ToString() : "";
            return PilotValue.FromKeyword(s);
        }, "name");

        Register("keyword", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromKeyword("");
            if (args.Count == 1)
            {
                var v = args[0];
                if (v.Kind == PilotValueKind.Keyword) return v;
                return PilotValue.FromKeyword(v.ToString());
            }
            string ns = args[0].ToString().TrimStart(':');
            string name = args[1].ToString().TrimStart(':');
            return PilotValue.FromKeyword(PilotKeyword.Intern(name, ns));
        }, "name_or_ns", "name");

        Register("is_keyword", (_, args) =>
        {
            return PilotValue.FromBool(args.Count > 0 && args[0].Kind == PilotValueKind.Keyword);
        }, "val");

        Register("name", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromString("");
            var first = args[0];
            if (first.Kind == PilotValueKind.Keyword)
            {
                return PilotValue.FromString(first.AsKeyword().Name);
            }
            if (first.Kind == PilotValueKind.String)
            {
                string s = first.AsString().TrimStart(':');
                int slash = s.IndexOf('/');
                return PilotValue.FromString(slash >= 0 ? s[(slash + 1)..] : s);
            }
            return PilotValue.FromString(first.ToString());
        }, "kw");

        Register("namespace", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.Null;
            var first = args[0];
            if (first.Kind == PilotValueKind.Keyword)
            {
                var ns = first.AsKeyword().Namespace;
                return ns != null ? PilotValue.FromString(ns) : PilotValue.Null;
            }
            if (first.Kind == PilotValueKind.String)
            {
                string s = first.AsString().TrimStart(':');
                int slash = s.IndexOf('/');
                return slash >= 0 ? PilotValue.FromString(s[..slash]) : PilotValue.Null;
            }
            return PilotValue.Null;
        }, "kw");

        // First-class operators
        Register("operator", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.Null;
            string op = args[0].ToString().Trim();
            return PilotValue.FromFunction(GetOperatorFunction(op));
        }, "op_name");

        Register("add", (_, a) => a.Count >= 2 ? a[0] + a[1] : (a.Count == 1 ? a[0] : PilotValue.FromInt(0)), "a", "b");
        Register("sub", (_, a) => a.Count >= 2 ? a[0] - a[1] : (a.Count == 1 ? -a[0] : PilotValue.FromInt(0)), "a", "b");
        Register("mul", (_, a) => a.Count >= 2 ? a[0] * a[1] : (a.Count == 1 ? a[0] : PilotValue.FromInt(1)), "a", "b");
        Register("div", (_, a) => a.Count >= 2 ? a[0] / a[1] : (a.Count == 1 ? a[0] : PilotValue.FromInt(1)), "a", "b");
        Register("mod", (_, a) => a.Count >= 2 ? a[0] % a[1] : PilotValue.FromInt(0), "a", "b");
        Register("eq", (_, a) => PilotValue.FromBool(a.Count >= 2 && a[0].Equals(a[1])), "a", "b");
        Register("neq", (_, a) => PilotValue.FromBool(a.Count >= 2 && !a[0].Equals(a[1])), "a", "b");
        Register("lt", (_, a) => PilotValue.FromBool(a.Count >= 2 && a[0] < a[1]), "a", "b");
        Register("lte", (_, a) => PilotValue.FromBool(a.Count >= 2 && a[0] <= a[1]), "a", "b");
        Register("gt", (_, a) => PilotValue.FromBool(a.Count >= 2 && a[0] > a[1]), "a", "b");
        Register("gte", (_, a) => PilotValue.FromBool(a.Count >= 2 && a[0] >= a[1]), "a", "b");

        Register("eval", (ctx, args) =>
        {
            if (args.Count == 0 || ctx.Engine == null) return PilotValue.Null;
            string code = args[0].ToString();
            var result = ctx.Engine.Execute(code, ctx.Options);
            return result.Value;
        }, "code");

        // Generators & Coroutines
        Register("next", (_, args) =>
        {
            if (args.Count > 0 && args[0].Kind == PilotValueKind.Generator)
            {
                return args[0].AsGenerator().Next();
            }
            throw new PilotRuntimeException("Expected generator as first argument to next().");
        }, "generator");

        // Multiprocessing & Concurrency
        Register("spawn", (ctx, args) =>
        {
            if (args.Count == 0) return PilotValue.Null;
            var target = args[0];
            var callArgs = args.Skip(1).ToList();

            var future = Pilot.Runtime.Concurrency.PilotFuture.Run(() =>
            {
                if (target.Kind == PilotValueKind.Function)
                {
                    return target.AsFunction().Invoke(ctx, callArgs);
                }
                return target;
            });

            return PilotValue.FromFuture(future);
        }, "callable");

        Register("parallel_map", (ctx, args) =>
        {
            if (args.Count < 2 || args[0].Kind != PilotValueKind.Function || args[1].Kind != PilotValueKind.List)
            {
                throw new PilotRuntimeException("parallel_map expects (callable, list).");
            }

            var func = args[0].AsFunction();
            var items = args[1].AsList();
            var results = new PilotValue[items.Count];

            System.Threading.Tasks.Parallel.For<PilotExecutionContext>(0, items.Count, () => new PilotExecutionContext(ctx.Engine, ctx.Options, new Pilot.Runtime.Environment.PilotEnvironment(ctx.Environment.CurrentScope)), (i, state, threadCtx) =>
            {
                results[i] = func.Invoke(threadCtx, new[] { items[i] });
                return threadCtx;
            }, _ => {});

            return PilotValue.FromList(results.ToList());
        }, "callable", "list");

        // Lazy Evaluation
        Register("lazy", (ctx, args) =>
        {
            if (args.Count == 0) return PilotValue.Null;
            var target = args[0];
            if (target.Kind == PilotValueKind.Function)
            {
                return PilotValue.FromLazy(new PilotLazy(c => target.AsFunction().Invoke(c, Array.Empty<PilotValue>()), ctx));
            }
            return PilotValue.FromLazy(new PilotLazy(_ => target, ctx));
        }, "callable");

        // JSON Serialization & Deserialization
        Register("json_dumps", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromString("null");
            bool formatted = args.Count > 1 && args[1].IsTrue;
            string json = Pilot.Runtime.Serialization.PilotJsonSerializer.Serialize(args[0], formatted);
            return PilotValue.FromString(json);
        }, "value", "formatted");

        Register("json_loads", (ctx, args) =>
        {
            if (args.Count == 0) return PilotValue.Null;
            string json = args[0].AsString();
            PilotClassDefinition? targetClass = null;

            if (args.Count > 1)
            {
                if (args[1].Kind == PilotValueKind.Function && args[1].AsFunction().ClassDef != null)
                {
                    targetClass = args[1].AsFunction().ClassDef;
                }
                else if (args[1].Kind == PilotValueKind.String)
                {
                    string className = args[1].AsString();
                    if (ctx.Environment.TryResolve(className, out var cVal) && cVal.Kind == PilotValueKind.Function)
                    {
                        targetClass = cVal.AsFunction().ClassDef;
                    }
                }
            }

            return Pilot.Runtime.Serialization.PilotJsonSerializer.Deserialize(json, targetClass, ctx);
        }, "json_string", "target_class");

        var jsonModule = new PilotObject(new PilotClassDefinition("json"));
        jsonModule.SetMember("dumps", PilotValue.FromFunction(PilotFunction.CreateNative("dumps", (_, a) =>
        {
            if (a.Count == 0) return PilotValue.FromString("null");
            bool formatted = a.Count > 1 && a[1].IsTrue;
            return PilotValue.FromString(Pilot.Runtime.Serialization.PilotJsonSerializer.Serialize(a[0], formatted));
        }, "value", "formatted")));
        jsonModule.SetMember("loads", PilotValue.FromFunction(PilotFunction.CreateNative("loads", (ctx, a) =>
        {
            if (a.Count == 0) return PilotValue.Null;
            string json = a[0].AsString();
            PilotClassDefinition? targetClass = null;
            if (a.Count > 1 && a[1].Kind == PilotValueKind.Function)
            {
                targetClass = a[1].AsFunction().ClassDef;
            }
            return Pilot.Runtime.Serialization.PilotJsonSerializer.Deserialize(json, targetClass, ctx);
        }, "json_string", "target_class")));

        Register("json", (_, _) => PilotValue.FromObject(jsonModule));

        // Type Adapters & Repeat / Vectorization
        Register("register_adapter", (ctx, args) =>
        {
            if (args.Count >= 3)
            {
                string fromType = args[0].AsString();
                string toType = args[1].AsString();
                if (args[2].Kind == PilotValueKind.Function)
                {
                    Pilot.Runtime.Adapters.PilotTypeAdapterRegistry.Default.Register(fromType, toType, args[2].AsFunction());
                    return PilotValue.True;
                }
            }
            return PilotValue.False;
        }, "from_type", "to_type", "converter");

        var adapterObj = new PilotObject(new PilotClassDefinition("TypeAdapter"));
        adapterObj.SetMember("register", PilotValue.FromFunction(PilotFunction.CreateNative("register", (ctx, args) =>
        {
            if (args.Count >= 3 && args[2].Kind == PilotValueKind.Function)
            {
                Pilot.Runtime.Adapters.PilotTypeAdapterRegistry.Default.Register(args[0].AsString(), args[1].AsString(), args[2].AsFunction());
                return PilotValue.True;
            }
            return PilotValue.False;
        }, "from_type", "to_type", "converter")));
        Register("TypeAdapter", (_, _) => PilotValue.FromObject(adapterObj));

        Register("vectorize", (_, args) =>
        {
            if (args.Count > 0 && args[0].Kind == PilotValueKind.Function)
            {
                var fn = args[0].AsFunction();
                fn.IsVectorized = true;
                return PilotValue.FromFunction(fn);
            }
            return PilotValue.Null;
        }, "target");

        Register("auto_repeat", (_, _) => PilotValue.FromString("auto_repeat"));
        Register("repeat", (_, _) => PilotValue.FromString("repeat"));

        // Virtual Environment Module (venv / virtual_environment)
        var venvModule = new PilotObject(new PilotClassDefinition("venv"));
        venvModule.SetMember("create", PilotValue.FromFunction(PilotFunction.CreateNative("create", (_, a) =>
        {
            if (a.Count == 0) return PilotValue.Null;
            string path = a[0].AsString();
            string? prompt = a.Count > 1 ? a[1].AsString() : null;
            var cfg = Pilot.Runtime.Venv.PilotVirtualEnvironment.Create(path, new Pilot.Runtime.Venv.PilotVenvOptions { Prompt = prompt });
            return PilotValue.FromString(path);
        }, "path", "prompt")));

        venvModule.SetMember("is_active", PilotValue.FromFunction(PilotFunction.CreateNative("is_active", (_, _) =>
            PilotValue.FromBool(Pilot.Runtime.Venv.PilotVirtualEnvironment.IsActive))));

        venvModule.SetMember("active_path", PilotValue.FromFunction(PilotFunction.CreateNative("active_path", (_, _) =>
        {
            string? p = Pilot.Runtime.Venv.PilotVirtualEnvironment.ActiveVenvPath;
            return p != null ? PilotValue.FromString(p) : PilotValue.Null;
        })));

        venvModule.SetMember("activate", PilotValue.FromFunction(PilotFunction.CreateNative("activate", (_, a) =>
        {
            if (a.Count == 0) return PilotValue.Null;
            Pilot.Runtime.Venv.PilotVirtualEnvironment.Activate(a[0].AsString());
            return PilotValue.True;
        }, "path")));

        venvModule.SetMember("deactivate", PilotValue.FromFunction(PilotFunction.CreateNative("deactivate", (_, _) =>
        {
            Pilot.Runtime.Venv.PilotVirtualEnvironment.Deactivate();
            return PilotValue.True;
        })));

        Register("venv", (_, _) => PilotValue.FromObject(venvModule));
        Register("virtual_environment", (_, _) => PilotValue.FromObject(venvModule));

        // Code Generation & Metaprogramming (codegen)
        var codegenModule = new PilotObject(new PilotClassDefinition("codegen"));
        codegenModule.SetMember("generator", PilotValue.FromFunction(PilotFunction.CreateNative("generator", (_, a) =>
            a.Count > 0 ? a[0] : PilotValue.Null, "fn")));
        codegenModule.SetMember("generate", PilotValue.FromFunction(PilotFunction.CreateNative("generate", (_, a) =>
            PilotValue.FromFunction(PilotFunction.CreateNative("decorator", (_, innerArgs) => innerArgs.Count > 0 ? innerArgs[0] : PilotValue.Null)), "name")));

        Register("codegen", (_, _) => PilotValue.FromObject(codegenModule));
        Register("generator", (_, a) => a.Count > 0 ? a[0] : PilotValue.Null, "target");
        Register("generate", (_, a) =>
            PilotValue.FromFunction(PilotFunction.CreateNative("decorator", (_, innerArgs) => innerArgs.Count > 0 ? innerArgs[0] : PilotValue.Null)), "name");
        Register("builder", (_, a) => a.Count > 0 ? a[0] : PilotValue.Null, "target");
        Register("observable", (_, a) => a.Count > 0 ? a[0] : PilotValue.Null, "target");
        Register("equality", (_, a) => a.Count > 0 ? a[0] : PilotValue.Null, "target");
        Register("dataclass", (_, a) => a.Count > 0 ? a[0] : PilotValue.Null, "target");
        Register("meta", (_, a) =>
            PilotValue.FromFunction(PilotFunction.CreateNative("meta_decorator", (_, innerArgs) => innerArgs.Count > 0 ? innerArgs[0] : PilotValue.Null)));
        Register("metadata", (_, a) =>
            PilotValue.FromFunction(PilotFunction.CreateNative("meta_decorator", (_, innerArgs) => innerArgs.Count > 0 ? innerArgs[0] : PilotValue.Null)));

        // Selective Execution, Benchmarking, and Testing Built-ins
        Register("test", (_, a) => a.Count > 0 ? a[0] : PilotValue.Null, "fn");
        Register("bench", (_, a) => a.Count > 0 ? a[0] : PilotValue.Null, "fn");
        Register("benchmark", (_, a) =>
        {
            if (a.Count > 0 && a[0].Kind == PilotValueKind.Function) return a[0];
            return PilotValue.FromFunction(PilotFunction.CreateNative("benchmark_dec", (_, innerArgs) =>
                innerArgs.Count > 0 ? innerArgs[0] : PilotValue.Null));
        });
        Register("tag", (_, _) =>
            PilotValue.FromFunction(PilotFunction.CreateNative("tag_dec", (_, innerArgs) =>
                innerArgs.Count > 0 ? innerArgs[0] : PilotValue.Null)));
        Register("entry", (_, a) => a.Count > 0 ? a[0] : PilotValue.Null, "fn");
        Register("is_selective", (ctx, a) =>
        {
            if (ctx.Environment != null && ctx.Environment.TryResolve("__selective__", out var selVal))
            {
                if (a.Count > 0)
                {
                    return PilotValue.FromBool(string.Equals(selVal.AsString(), a[0].AsString(), StringComparison.OrdinalIgnoreCase));
                }
                return PilotValue.True;
            }
            return PilotValue.False;
        }, "target_mode");

        // Bidirectional Optimizing Transpilation & Ingestion Built-ins
        Register("transpile", (_, args) =>
        {
            if (args.Count < 2) return PilotValue.Null;
            string pilotCode = args[0].AsString();
            string targetLang = args[1].AsString();
            var res = Pilot.Transpilation.TranspilationRegistry.Default.Transpile(pilotCode, targetLang);
            return res.Success ? PilotValue.FromString(res.Code) : PilotValue.Null;
        }, "pilot_code", "target_lang");

        Register("transpile_file", (_, args) =>
        {
            if (args.Count < 2) return PilotValue.Null;
            string inputPath = args[0].AsString();
            string targetLang = args[1].AsString();
            if (!System.IO.File.Exists(inputPath)) return PilotValue.Null;

            string pilotCode = System.IO.File.ReadAllText(inputPath);
            var res = Pilot.Transpilation.TranspilationRegistry.Default.Transpile(pilotCode, targetLang);
            if (!res.Success) return PilotValue.Null;

            if (args.Count >= 3 && args[2].Kind == PilotValueKind.String)
            {
                string outPath = args[2].AsString();
                if (!string.IsNullOrWhiteSpace(outPath))
                {
                    System.IO.File.WriteAllText(outPath, res.Code);
                }
            }
            return PilotValue.FromString(res.Code);
        }, "input_path", "target_lang", "output_path");

        Register("ingest", (_, args) =>
        {
            if (args.Count < 2) return PilotValue.Null;
            string foreignCode = args[0].AsString();
            string sourceLang = args[1].AsString();
            var res = Pilot.Transpilation.TranspilationRegistry.Default.Ingest(foreignCode, sourceLang);
            return res.Success ? PilotValue.FromString(res.Code) : PilotValue.Null;
        }, "foreign_code", "source_lang");

        Register("ingest_file", (_, args) =>
        {
            if (args.Count < 1) return PilotValue.Null;
            string inputPath = args[0].AsString();
            if (!System.IO.File.Exists(inputPath)) return PilotValue.Null;

            string sourceLang = args.Count >= 2 && args[1].Kind == PilotValueKind.String && !string.IsNullOrWhiteSpace(args[1].AsString())
                ? args[1].AsString()
                : Pilot.Transpilation.TranspilationRegistry.InferLanguageFromPath(inputPath);

            string foreignCode = System.IO.File.ReadAllText(inputPath);
            var res = Pilot.Transpilation.TranspilationRegistry.Default.Ingest(foreignCode, sourceLang);
            if (!res.Success) return PilotValue.Null;

            if (args.Count >= 3 && args[2].Kind == PilotValueKind.String)
            {
                string outPath = args[2].AsString();
                if (!string.IsNullOrWhiteSpace(outPath))
                {
                    System.IO.File.WriteAllText(outPath, res.Code);
                }
            }
            return PilotValue.FromString(res.Code);
        }, "input_path", "source_lang", "output_path");

        Register("transpile_library", (_, args) =>
        {
            if (args.Count < 3) return PilotValue.Null;
            string pilotDir = args[0].AsString();
            string targetLang = args[1].AsString();
            string outputDir = args[2].AsString();
            var summary = Pilot.Transpilation.PilotLibraryTranspiler.TranspilePilotLibraryToTarget(pilotDir, outputDir, targetLang);
            var list = new List<PilotValue>(summary.GeneratedFiles.Count);
            foreach (var f in summary.GeneratedFiles) list.Add(PilotValue.FromString(f));
            return PilotValue.FromList(list);
        }, "pilot_dir", "target_lang", "output_dir");

        Register("ingest_library", (_, args) =>
        {
            if (args.Count < 2) return PilotValue.Null;
            string sourceDir = args[0].AsString();
            string sourceLang = args.Count >= 3 ? args[1].AsString() : "auto";
            string outputDir = args.Count >= 3 ? args[2].AsString() : args[1].AsString();
            var summary = Pilot.Transpilation.PilotLibraryTranspiler.IngestLibraryToPilot(sourceDir, outputDir, sourceLang);
            var list = new List<PilotValue>(summary.GeneratedFiles.Count);
            foreach (var f in summary.GeneratedFiles) list.Add(PilotValue.FromString(f));
            return PilotValue.FromList(list);
        }, "source_dir", "source_lang", "output_dir");

        // Hot-Reloading & Binary/Script Patching Built-ins
        Register("hot_reload", (ctx, args) =>
        {
            if (args.Count < 1) return PilotValue.Null;
            string codeOrPath = args[0].AsString();
            return ctx.HotReload.ApplyCodeOrFile(codeOrPath).ToPilotMap();
        }, "code_or_path");

        Register("apply_patch", (ctx, args) =>
        {
            if (args.Count < 1) return PilotValue.Null;
            string patchCode = args[0].AsString();
            string label = args.Count >= 2 ? args[1].AsString() : "<inline-patch>";
            return ctx.HotReload.ApplySourcePatch(patchCode, label).ToPilotMap();
        }, "patch_code", "label");

        Register("patch_file", (ctx, args) =>
        {
            if (args.Count < 1) return PilotValue.Null;
            string path = args[0].AsString();
            bool onlyIfChanged = args.Count >= 2 && args[1].IsTrue;
            return ctx.HotReload.ApplyPatchFile(path, onlyIfChanged).ToPilotMap();
        }, "file_path", "only_if_changed");

        Register("rollback_patch", (ctx, args) =>
        {
            int steps = args.Count >= 1 ? (int)args[0].AsInt() : 1;
            return ctx.HotReload.Rollback(steps).ToPilotMap();
        }, "steps");

        Register("patch_binary", (_, args) =>
        {
            if (args.Count < 2) return PilotValue.False;
            string basePath = args[0].AsString();
            string patchPath = args[1].AsString();
            string outPath = args.Count >= 3 && !string.IsNullOrWhiteSpace(args[2].AsString())
                ? args[2].AsString()
                : basePath;

            if (!System.IO.File.Exists(basePath) || !System.IO.File.Exists(patchPath))
            {
                return PilotValue.False;
            }

            byte[] baseBytes = System.IO.File.ReadAllBytes(basePath);
            Pilot.Compiler.PilotCompiledProgram baseProg;
            if (Pilot.Runtime.HotReload.PilotHotReloadSession.IsCompiledBinary(baseBytes, basePath))
            {
                baseProg = Pilot.Compiler.PilotCompiledProgram.FromBytes(baseBytes, basePath);
            }
            else
            {
                var comp = Pilot.Compiler.PilotCompiler.Default.CompileFile(basePath);
                if (!comp.Success || comp.Program is not Pilot.Compiler.PilotCompiledProgram compiledBase) return PilotValue.False;
                baseProg = compiledBase;
            }

            var patchedProg = baseProg.ApplyPatchFile(patchPath);
            patchedProg.Save(outPath);
            return PilotValue.FromString(System.IO.Path.GetFullPath(outPath));
        }, "base_path", "patch_path", "output_path");

        var hotReloadModule = PilotFunction.CreateNative("HotReload", (ctx, args) =>
        {
            if (args.Count == 0)
            {
                return PilotValue.FromInt(ctx.HotReload.Version);
            }
            return ctx.HotReload.ApplyCodeOrFile(args[0].AsString()).ToPilotMap();
        }, "code_or_path");

        hotReloadModule.Members["apply"] = PilotValue.FromFunction(
            PilotFunction.CreateNative("apply", (ctx, args) =>
            {
                if (args.Count < 1) return PilotValue.Null;
                string code = args[0].AsString();
                string label = args.Count >= 2 ? args[1].AsString() : "<inline-patch>";
                return ctx.HotReload.ApplySourcePatch(code, label).ToPilotMap();
            }, "patch_code", "label"));

        hotReloadModule.Members["apply_file"] = PilotValue.FromFunction(
            PilotFunction.CreateNative("apply_file", (ctx, args) =>
            {
                if (args.Count < 1) return PilotValue.Null;
                return ctx.HotReload.ApplyPatchFile(args[0].AsString(), onlyIfChanged: false).ToPilotMap();
            }, "file_path"));

        hotReloadModule.Members["reload_if_changed"] = PilotValue.FromFunction(
            PilotFunction.CreateNative("reload_if_changed", (ctx, args) =>
            {
                if (args.Count < 1) return PilotValue.Null;
                return ctx.HotReload.ApplyPatchFile(args[0].AsString(), onlyIfChanged: true).ToPilotMap();
            }, "file_path"));

        hotReloadModule.Members["rollback"] = PilotValue.FromFunction(
            PilotFunction.CreateNative("rollback", (ctx, args) =>
            {
                int steps = args.Count >= 1 ? (int)args[0].AsInt() : 1;
                return ctx.HotReload.Rollback(steps).ToPilotMap();
            }, "steps"));

        hotReloadModule.Members["version"] = PilotValue.FromFunction(
            PilotFunction.CreateNative("version", (ctx, _) => PilotValue.FromInt(ctx.HotReload.Version)));

        hotReloadModule.Members["history"] = PilotValue.FromFunction(
            PilotFunction.CreateNative("history", (ctx, _) =>
            {
                var list = ctx.HotReload.History.Select(h => h.ToPilotMap()).ToList();
                return PilotValue.FromList(list);
            }));

        hotReloadModule.Members["watch"] = PilotValue.FromFunction(
            PilotFunction.CreateNative("watch", (ctx, args) =>
            {
                if (args.Count < 1) return PilotValue.False;
                string path = args[0].AsString();
                PilotFunction? cb = args.Count >= 2 && args[1].Kind == PilotValueKind.Function
                    ? args[1].AsFunction()
                    : null;
                ctx.HotReload.WatchFile(path, cb);
                return PilotValue.True;
            }, "file_path", "callback"));

        hotReloadModule.Members["unwatch_all"] = PilotValue.FromFunction(
            PilotFunction.CreateNative("unwatch_all", (ctx, _) =>
            {
                ctx.HotReload.UnwatchAll();
                return PilotValue.True;
            }));

        hotReloadModule.Members["patch_binary"] = PilotValue.FromFunction(_functions["patch_binary"]);

        _functions["HotReload"] = hotReloadModule;

        // ─── Commit-Bound Runtime Version Introspection ───
        Register("pilot_version", (_, args) =>
        {
            if (args.Count == 0)
            {
                return PilotValue.FromString(Pilot.Diagnostics.PilotVersion.FullVersion);
            }

            string part = args[0].AsString().Trim().ToLowerInvariant();
            return part switch
            {
                "semver" => PilotValue.FromString(Pilot.Diagnostics.PilotVersion.SemVer),
                "base" => PilotValue.FromString(Pilot.Diagnostics.PilotVersion.BaseVersion),
                "sha" or "commit" or "short_sha" => PilotValue.FromString(Pilot.Diagnostics.PilotVersion.ShortCommitSha),
                "full_sha" => PilotValue.FromString(Pilot.Diagnostics.PilotVersion.FullCommitSha),
                "count" or "commit_count" => PilotValue.FromInt(Pilot.Diagnostics.PilotVersion.CommitCount),
                "branch" => PilotValue.FromString(Pilot.Diagnostics.PilotVersion.Branch),
                "date" or "commit_date" => PilotValue.FromString(Pilot.Diagnostics.PilotVersion.CommitDate),
                "banner" => PilotValue.FromString(Pilot.Diagnostics.PilotVersion.DisplayBanner),
                _ => PilotValue.FromString(Pilot.Diagnostics.PilotVersion.FullVersion)
            };
        }, "part");

        Register("pilot_version_info", (_, _) =>
        {
            var dict = new Dictionary<PilotValue, PilotValue>
            {
                [PilotValue.FromString("version")] = PilotValue.FromString(Pilot.Diagnostics.PilotVersion.FullVersion),
                [PilotValue.FromString("semver")] = PilotValue.FromString(Pilot.Diagnostics.PilotVersion.SemVer),
                [PilotValue.FromString("base_version")] = PilotValue.FromString(Pilot.Diagnostics.PilotVersion.BaseVersion),
                [PilotValue.FromString("commit_count")] = PilotValue.FromInt(Pilot.Diagnostics.PilotVersion.CommitCount),
                [PilotValue.FromString("commit_sha")] = PilotValue.FromString(Pilot.Diagnostics.PilotVersion.ShortCommitSha),
                [PilotValue.FromString("full_commit_sha")] = PilotValue.FromString(Pilot.Diagnostics.PilotVersion.FullCommitSha),
                [PilotValue.FromString("branch")] = PilotValue.FromString(Pilot.Diagnostics.PilotVersion.Branch),
                [PilotValue.FromString("commit_date")] = PilotValue.FromString(Pilot.Diagnostics.PilotVersion.CommitDate)
            };
            return PilotValue.FromMap(dict);
        });
    }

    private static PilotFunction GetOperatorFunction(string op)
    {
        return op switch
        {
            "+" => PilotFunction.CreateNative("+", (_, a) => a.Count >= 2 ? a[0] + a[1] : (a.Count == 1 ? a[0] : PilotValue.FromInt(0)), "a", "b"),
            "-" => PilotFunction.CreateNative("-", (_, a) => a.Count >= 2 ? a[0] - a[1] : (a.Count == 1 ? -a[0] : PilotValue.FromInt(0)), "a", "b"),
            "*" => PilotFunction.CreateNative("*", (_, a) => a.Count >= 2 ? a[0] * a[1] : (a.Count == 1 ? a[0] : PilotValue.FromInt(1)), "a", "b"),
            "/" => PilotFunction.CreateNative("/", (_, a) => a.Count >= 2 ? a[0] / a[1] : (a.Count == 1 ? a[0] : PilotValue.FromInt(1)), "a", "b"),
            "%" => PilotFunction.CreateNative("%", (_, a) => a.Count >= 2 ? a[0] % a[1] : PilotValue.FromInt(0), "a", "b"),
            "==" => PilotFunction.CreateNative("==", (_, a) => PilotValue.FromBool(a.Count >= 2 && a[0].Equals(a[1])), "a", "b"),
            "!=" => PilotFunction.CreateNative("!=", (_, a) => PilotValue.FromBool(a.Count >= 2 && !a[0].Equals(a[1])), "a", "b"),
            "<" => PilotFunction.CreateNative("<", (_, a) => PilotValue.FromBool(a.Count >= 2 && a[0] < a[1]), "a", "b"),
            "<=" => PilotFunction.CreateNative("<=", (_, a) => PilotValue.FromBool(a.Count >= 2 && a[0] <= a[1]), "a", "b"),
            ">" => PilotFunction.CreateNative(">", (_, a) => PilotValue.FromBool(a.Count >= 2 && a[0] > a[1]), "a", "b"),
            ">=" => PilotFunction.CreateNative(">=", (_, a) => PilotValue.FromBool(a.Count >= 2 && a[0] >= a[1]), "a", "b"),
            "&" => PilotFunction.CreateNative("&", (_, a) => a.Count >= 2 ? a[0] & a[1] : PilotValue.FromInt(0), "a", "b"),
            "|" => PilotFunction.CreateNative("|", (_, a) => a.Count >= 2 ? a[0] | a[1] : PilotValue.FromInt(0), "a", "b"),
            "^" => PilotFunction.CreateNative("^", (_, a) => a.Count >= 2 ? a[0] ^ a[1] : PilotValue.FromInt(0), "a", "b"),
            "!" or "not" => PilotFunction.CreateNative("not", (_, a) => PilotValue.FromBool(a.Count > 0 && !a[0].IsTrue), "a"),
            _ => throw new PilotRuntimeException($"Unknown operator '{op}' passed to operator().")
        };
    }
}
