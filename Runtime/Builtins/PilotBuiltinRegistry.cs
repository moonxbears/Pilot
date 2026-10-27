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
                _ => PilotValue.FromInt(0)
            };
        }, "collection");

        Register("type_of", (_, args) =>
        {
            if (args.Count == 0) return PilotValue.FromString("undefined");
            return PilotValue.FromString(args[0].Kind.ToString().ToLowerInvariant());
        }, "value");

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
            return PilotValue.FromString(s);
        }, "name");

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

            System.Threading.Tasks.Parallel.For(0, items.Count, i =>
            {
                results[i] = func.Invoke(ctx, new[] { items[i] });
            });

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
    }
}
