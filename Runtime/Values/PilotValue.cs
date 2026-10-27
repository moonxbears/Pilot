using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Execution;

namespace Pilot.Runtime.Values;

/// <summary>
/// Universal runtime value in the Pilot language.
/// Designed for high performance, dynamic flexibility, and plug-and-play decoupling.
/// </summary>
public readonly struct PilotValue : IEquatable<PilotValue>
{
    public PilotValueKind Kind { get; }
    private readonly object? _reference;
    private readonly long _integer;
    private readonly double _floating;

    public static readonly PilotValue Null = new(PilotValueKind.Null, null, 0, 0);
    public static readonly PilotValue Undefined = new(PilotValueKind.Undefined, null, 0, 0);
    public static readonly PilotValue True = new(PilotValueKind.Bool, null, 1, 0);
    public static readonly PilotValue False = new(PilotValueKind.Bool, null, 0, 0);

    private PilotValue(PilotValueKind kind, object? reference, long integer = 0, double floating = 0)
    {
        Kind = kind;
        _reference = reference;
        _integer = integer;
        _floating = floating;
    }

    #region Factory Methods

    public static PilotValue FromBool(bool value) => value ? True : False;
    public static PilotValue FromInt(long value) => new(PilotValueKind.Int, null, value, 0);
    public static PilotValue FromUInt(ulong value) => new(PilotValueKind.UInt, null, unchecked((long)value), 0);
    public static PilotValue FromFloat(double value) => new(PilotValueKind.Float, null, 0, value);
    public static PilotValue FromDecimal(decimal value) => new(PilotValueKind.Decimal, value, 0, 0);
    public static PilotValue FromChar(char value) => new(PilotValueKind.Char, null, value, 0);
    public static PilotValue FromString(string value) => new(PilotValueKind.String, value ?? string.Empty, 0, 0);
    public static PilotValue FromKeyword(PilotKeyword keyword) => new(PilotValueKind.Keyword, keyword, 0, 0);
    public static PilotValue FromKeyword(string text) => new(PilotValueKind.Keyword, PilotKeyword.Intern(text), 0, 0);
    public static PilotValue FromList(List<PilotValue> list) => new(PilotValueKind.List, list ?? new List<PilotValue>(), 0, 0);
    public static PilotValue FromTuple(IReadOnlyList<PilotValue> tuple) => new(PilotValueKind.Tuple, tuple ?? Array.Empty<PilotValue>(), 0, 0);
    public static PilotValue FromMap(Dictionary<PilotValue, PilotValue> map) => new(PilotValueKind.Map, map ?? new Dictionary<PilotValue, PilotValue>(), 0, 0);
    public static PilotValue FromSet(HashSet<PilotValue> set) => new(PilotValueKind.Set, set ?? new HashSet<PilotValue>(), 0, 0);
    public static PilotValue FromRange(PilotRange range) => new(PilotValueKind.Range, range, 0, 0);
    public static PilotValue FromMatrix(PilotMatrix matrix) => new(PilotValueKind.Matrix, matrix, 0, 0);
    public static PilotValue FromFunction(PilotFunction function) => new(PilotValueKind.Function, function, 0, 0);
    public static PilotValue FromObject(PilotObject obj) => new(PilotValueKind.Object, obj, 0, 0);
    public static PilotValue FromGenerator(PilotGenerator gen) => new(PilotValueKind.Generator, gen, 0, 0);
    public static PilotValue FromLazy(PilotLazy l) => new(PilotValueKind.Lazy, l, 0, 0);
    public static PilotValue FromFuture(Pilot.Runtime.Concurrency.PilotFuture f) => new(PilotValueKind.Future, f, 0, 0);
    public static PilotValue FromForeign(object? foreign) => foreign == null ? Null : new(PilotValueKind.Foreign, foreign, 0, 0);
    public static PilotValue FromMethodGroup(PilotMethodGroup mg) => new(PilotValueKind.MethodGroup, mg, 0, 0);
    public static PilotValue FromNativeArray(Pilot.Runtime.NativeMemory.PilotNativeArray na) => new(PilotValueKind.NativeArray, na, 0, 0);
    public static PilotValue FromNativeMap(Pilot.Runtime.NativeMemory.PilotNativeMap nm) => new(PilotValueKind.NativeMap, nm, 0, 0);

    #endregion

    #region Implicit Conversions

    public static implicit operator PilotValue(bool b) => FromBool(b);
    public static implicit operator PilotValue(int i) => FromInt(i);
    public static implicit operator PilotValue(long l) => FromInt(l);
    public static implicit operator PilotValue(ulong u) => FromUInt(u);
    public static implicit operator PilotValue(float f) => FromFloat(f);
    public static implicit operator PilotValue(double d) => FromFloat(d);
    public static implicit operator PilotValue(decimal m) => FromDecimal(m);
    public static implicit operator PilotValue(char c) => FromChar(c);
    public static implicit operator PilotValue(string s) => FromString(s);

    #endregion

    #region Accessors and Coercions

    public bool IsNull => Kind == PilotValueKind.Null;
    public bool IsUndefined => Kind == PilotValueKind.Undefined;
    public bool IsNumber => Kind is PilotValueKind.Int or PilotValueKind.UInt or PilotValueKind.Float or PilotValueKind.Decimal;

    public bool IsTrue => Kind switch
    {
        PilotValueKind.Bool => _integer != 0,
        PilotValueKind.Int => _integer != 0,
        PilotValueKind.UInt => unchecked((ulong)_integer) != 0,
        PilotValueKind.Float => _floating != 0.0 && !double.IsNaN(_floating),
        PilotValueKind.Decimal => _reference is decimal m && m != 0,
        PilotValueKind.Char => _integer != 0,
        PilotValueKind.String => !string.IsNullOrEmpty((string)_reference!),
        PilotValueKind.List => ((List<PilotValue>)_reference!).Count > 0,
        PilotValueKind.Tuple => ((IReadOnlyList<PilotValue>)_reference!).Count > 0,
        PilotValueKind.Map => ((Dictionary<PilotValue, PilotValue>)_reference!).Count > 0,
        PilotValueKind.Set => ((HashSet<PilotValue>)_reference!).Count > 0,
        PilotValueKind.Range => ((PilotRange)_reference!).Count > 0,
        PilotValueKind.Null or PilotValueKind.Undefined => false,
        _ => true
    };

    public long AsInt() => Kind switch
    {
        PilotValueKind.Int => _integer,
        PilotValueKind.UInt => unchecked((long)_integer),
        PilotValueKind.Float => (long)_floating,
        PilotValueKind.Decimal => (long)(decimal)_reference!,
        PilotValueKind.Bool => _integer,
        PilotValueKind.Char => _integer,
        PilotValueKind.String => long.TryParse((string)_reference!, out var l) ? l : 0,
        _ => 0
    };

    public double AsFloat() => Kind switch
    {
        PilotValueKind.Float => _floating,
        PilotValueKind.Int => _integer,
        PilotValueKind.UInt => unchecked((ulong)_integer),
        PilotValueKind.Decimal => (double)(decimal)_reference!,
        PilotValueKind.Bool => _integer,
        PilotValueKind.String => double.TryParse((string)_reference!, NumberStyles.Any, CultureInfo.InvariantCulture, out var d) ? d : 0.0,
        _ => 0.0
    };

    public bool IsKeyword => Kind == PilotValueKind.Keyword;
    public string AsString() => ToString();
    public decimal AsDecimal() => _reference is decimal d ? d : (decimal)AsFloat();
    public char AsChar() => (char)_integer;
    public PilotKeyword AsKeyword() => _reference as PilotKeyword ?? throw new InvalidCastException($"Expected Keyword, got {Kind}");

    public List<PilotValue> AsList() => _reference as List<PilotValue> ?? throw new InvalidCastException($"Expected List, got {Kind}");
    public IReadOnlyList<PilotValue> AsTuple() => _reference as IReadOnlyList<PilotValue> ?? throw new InvalidCastException($"Expected Tuple, got {Kind}");
    public Dictionary<PilotValue, PilotValue> AsMap() => _reference as Dictionary<PilotValue, PilotValue> ?? throw new InvalidCastException($"Expected Map, got {Kind}");
    public HashSet<PilotValue> AsSet() => _reference as HashSet<PilotValue> ?? throw new InvalidCastException($"Expected Set, got {Kind}");
    public PilotRange AsRange() => _reference as PilotRange ?? throw new InvalidCastException($"Expected Range, got {Kind}");
    public PilotMatrix AsMatrix() => _reference as PilotMatrix ?? throw new InvalidCastException($"Expected Matrix, got {Kind}");
    public PilotFunction AsFunction() => _reference as PilotFunction ?? throw new InvalidCastException($"Expected Function, got {Kind}");
    public PilotObject AsObject() => _reference as PilotObject ?? throw new InvalidCastException($"Expected Object, got {Kind}");
    public PilotGenerator AsGenerator() => _reference as PilotGenerator ?? throw new InvalidCastException($"Expected Generator, got {Kind}");
    public PilotLazy AsLazy() => _reference as PilotLazy ?? throw new InvalidCastException($"Expected Lazy, got {Kind}");
    public Pilot.Runtime.Concurrency.PilotFuture AsFuture() => _reference as Pilot.Runtime.Concurrency.PilotFuture ?? throw new InvalidCastException($"Expected Future, got {Kind}");
    public PilotMethodGroup AsMethodGroup() => _reference as PilotMethodGroup ?? throw new InvalidCastException($"Expected MethodGroup, got {Kind}");
    public Pilot.Runtime.NativeMemory.PilotNativeArray AsNativeArray() => _reference as Pilot.Runtime.NativeMemory.PilotNativeArray ?? throw new InvalidCastException($"Expected NativeArray, got {Kind}");
    public Pilot.Runtime.NativeMemory.PilotNativeMap AsNativeMap() => _reference as Pilot.Runtime.NativeMemory.PilotNativeMap ?? throw new InvalidCastException($"Expected NativeMap, got {Kind}");
    public object? AsForeign() => _reference;

    #endregion

    #region Operators

    public static PilotValue CreateClassConstructor(PilotClassDefinition classDef)
    {
        var ctor = PilotFunction.CreateNative(
            classDef.Name,
            (ctx, args) => FromObject(classDef.Instantiate(ctx, args)));
        ctor.ClassDef = classDef;
        return FromFunction(ctor);
    }

    public static PilotValue operator |(PilotValue a, PilotValue b)
    {
        // Class Algebraic Union: ClassA | ClassB
        if (a.Kind == PilotValueKind.Function && a.AsFunction().ClassDef != null &&
            b.Kind == PilotValueKind.Function && b.AsFunction().ClassDef != null)
        {
            var composed = a.AsFunction().ClassDef!.Union(b.AsFunction().ClassDef!);
            return CreateClassConstructor(composed);
        }

        // Method Group / Function union and fallback: GroupA | GroupB
        if (a.Kind is PilotValueKind.MethodGroup or PilotValueKind.Function &&
            b.Kind is PilotValueKind.MethodGroup or PilotValueKind.Function)
        {
            var mgA = a.Kind == PilotValueKind.MethodGroup
                ? a.AsMethodGroup()
                : new PilotMethodGroup(a.AsFunction().Name, new[] { a.AsFunction() });

            var mgB = b.Kind == PilotValueKind.MethodGroup
                ? b.AsMethodGroup()
                : new PilotMethodGroup(b.AsFunction().Name, new[] { b.AsFunction() });

            return FromMethodGroup(mgA.Union(mgB));
        }

        if (a.Kind == PilotValueKind.Bool && b.Kind == PilotValueKind.Bool)
            return FromBool(a.IsTrue || b.IsTrue);

        return FromInt(a.AsInt() | b.AsInt());
    }

    public static PilotValue operator &(PilotValue a, PilotValue b)
    {
        // Class Algebraic Intersection: ClassA & ClassB
        if (a.Kind == PilotValueKind.Function && a.AsFunction().ClassDef != null &&
            b.Kind == PilotValueKind.Function && b.AsFunction().ClassDef != null)
        {
            var intersected = a.AsFunction().ClassDef!.Intersect(b.AsFunction().ClassDef!);
            return CreateClassConstructor(intersected);
        }

        if (a.Kind == PilotValueKind.Bool && b.Kind == PilotValueKind.Bool)
            return FromBool(a.IsTrue && b.IsTrue);

        return FromInt(a.AsInt() & b.AsInt());
    }

    public static PilotValue operator +(PilotValue a, PilotValue b)
    {
        if (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
            return new(PilotValueKind.Int, null, a._integer + b._integer, 0);

        if (a.Kind == PilotValueKind.String || b.Kind == PilotValueKind.String)
            return FromString(a.ToString() + b.ToString());

        if (a.Kind == PilotValueKind.List && b.Kind == PilotValueKind.List)
        {
            var combined = new List<PilotValue>(a.AsList());
            combined.AddRange(b.AsList());
            return FromList(combined);
        }

        if (a.Kind == PilotValueKind.Matrix && b.Kind == PilotValueKind.Matrix)
        {
            return FromMatrix(a.AsMatrix().Add(b.AsMatrix()));
        }

        if (a.Kind == PilotValueKind.Float || b.Kind == PilotValueKind.Float)
            return FromFloat(a.AsFloat() + b.AsFloat());

        return FromInt(a.AsInt() + b.AsInt());
    }

    public static PilotValue operator -(PilotValue a, PilotValue b)
    {
        if (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
            return new(PilotValueKind.Int, null, a._integer - b._integer, 0);

        // Class Algebraic Subtraction: ClassA - ClassB or ClassA - "member"
        if (a.Kind == PilotValueKind.Function && a.AsFunction().ClassDef != null)
        {
            if (b.Kind == PilotValueKind.Function && b.AsFunction().ClassDef != null)
            {
                var subtracted = a.AsFunction().ClassDef!.Subtract(b.AsFunction().ClassDef!);
                return CreateClassConstructor(subtracted);
            }
            if (b.Kind == PilotValueKind.String)
            {
                var subtracted = a.AsFunction().ClassDef!.Subtract(b.AsString());
                return CreateClassConstructor(subtracted);
            }
        }

        if (a.Kind == PilotValueKind.Matrix && b.Kind == PilotValueKind.Matrix)
        {
            return FromMatrix(a.AsMatrix().Subtract(b.AsMatrix()));
        }

        if (a.Kind == PilotValueKind.Float || b.Kind == PilotValueKind.Float)
            return FromFloat(a.AsFloat() - b.AsFloat());

        return FromInt(a.AsInt() - b.AsInt());
    }

    public static PilotValue operator *(PilotValue a, PilotValue b)
    {
        if (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
            return new(PilotValueKind.Int, null, a._integer * b._integer, 0);

        if (a.Kind == PilotValueKind.Matrix && b.Kind == PilotValueKind.Matrix)
        {
            return FromMatrix(a.AsMatrix().Multiply(b.AsMatrix()));
        }
        if (a.Kind == PilotValueKind.Matrix && b.IsNumber)
        {
            return FromMatrix(a.AsMatrix().MultiplyScalar(b.AsFloat()));
        }

        if (a.Kind == PilotValueKind.String && b.Kind == PilotValueKind.Int)
        {
            string s = (string)a._reference!;
            long count = Math.Max(0, b.AsInt());
            var sb = new StringBuilder((int)(s.Length * Math.Min(count, 10000)));
            for (long i = 0; i < count; i++) sb.Append(s);
            return FromString(sb.ToString());
        }

        if (a.Kind == PilotValueKind.Float || b.Kind == PilotValueKind.Float)
            return FromFloat(a.AsFloat() * b.AsFloat());

        return FromInt(a.AsInt() * b.AsInt());
    }

    public static PilotValue operator /(PilotValue a, PilotValue b)
    {
        if (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
        {
            if (b._integer == 0) throw new PilotDivideByZeroException("Division by zero.");
            return new(PilotValueKind.Int, null, a._integer / b._integer, 0);
        }

        if (a.Kind == PilotValueKind.Float || b.Kind == PilotValueKind.Float)
        {
            double divisor = b.AsFloat();
            if (divisor == 0.0) throw new PilotDivideByZeroException("Division by zero.");
            return FromFloat(a.AsFloat() / divisor);
        }

        long div = b.AsInt();
        if (div == 0) throw new PilotDivideByZeroException("Division by zero.");
        return FromInt(a.AsInt() / div);
    }

    public static PilotValue operator %(PilotValue a, PilotValue b)
    {
        if (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
        {
            if (b._integer == 0) throw new PilotDivideByZeroException("Modulo by zero.");
            return new(PilotValueKind.Int, null, a._integer % b._integer, 0);
        }

        if (a.Kind == PilotValueKind.Float || b.Kind == PilotValueKind.Float)
        {
            double divisor = b.AsFloat();
            if (divisor == 0.0) throw new PilotDivideByZeroException("Modulo by zero.");
            return FromFloat(a.AsFloat() % divisor);
        }

        long div = b.AsInt();
        if (div == 0) throw new PilotDivideByZeroException("Modulo by zero.");
        return FromInt(a.AsInt() % div);
    }

    public static PilotValue operator -(PilotValue a)
    {
        if (a.Kind == PilotValueKind.Int) return new(PilotValueKind.Int, null, -a._integer, 0);
        if (a.Kind == PilotValueKind.Float) return FromFloat(-a._floating);
        return FromInt(-a.AsInt());
    }

    public static PilotValue operator !(PilotValue a) => FromBool(!a.IsTrue);

    public static bool operator ==(PilotValue a, PilotValue b) => a.Equals(b);
    public static bool operator !=(PilotValue a, PilotValue b) => !a.Equals(b);

    public static bool operator <(PilotValue a, PilotValue b) =>
        (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
            ? a._integer < b._integer
            : (a.Kind == PilotValueKind.Float || b.Kind == PilotValueKind.Float)
                ? a.AsFloat() < b.AsFloat()
                : a.AsInt() < b.AsInt();

    public static bool operator <=(PilotValue a, PilotValue b) =>
        (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
            ? a._integer <= b._integer
            : (a.Kind == PilotValueKind.Float || b.Kind == PilotValueKind.Float)
                ? a.AsFloat() <= b.AsFloat()
                : a.AsInt() <= b.AsInt();

    public static bool operator >(PilotValue a, PilotValue b) =>
        (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
            ? a._integer > b._integer
            : (a.Kind == PilotValueKind.Float || b.Kind == PilotValueKind.Float)
                ? a.AsFloat() > b.AsFloat()
                : a.AsInt() > b.AsInt();

    public static bool operator >=(PilotValue a, PilotValue b) =>
        (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
            ? a._integer >= b._integer
            : (a.Kind == PilotValueKind.Float || b.Kind == PilotValueKind.Float)
                ? a.AsFloat() >= b.AsFloat()
                : a.AsInt() >= b.AsInt();

    #endregion

    #region Indexing and Member Resolution

    public PilotValue this[PilotValue index]
    {
        get
        {
            if (Kind == PilotValueKind.List)
            {
                var list = (List<PilotValue>)_reference!;
                int idx = (int)index.AsInt();
                if (idx < 0) idx += list.Count;
                return (idx >= 0 && idx < list.Count) ? list[idx] : Undefined;
            }
            if (Kind == PilotValueKind.Tuple)
            {
                var tuple = (IReadOnlyList<PilotValue>)_reference!;
                int idx = (int)index.AsInt();
                if (idx < 0) idx += tuple.Count;
                return (idx >= 0 && idx < tuple.Count) ? tuple[idx] : Undefined;
            }
            if (Kind == PilotValueKind.Map)
            {
                var map = (Dictionary<PilotValue, PilotValue>)_reference!;
                return map.TryGetValue(index, out var v) ? v : Undefined;
            }
            if (Kind == PilotValueKind.NativeArray)
            {
                var na = (Pilot.Runtime.NativeMemory.PilotNativeArray)_reference!;
                int idx = (int)index.AsInt();
                if (idx < 0) idx += na.Length;
                return (idx >= 0 && idx < na.Length) ? na.GetValue(idx) : Undefined;
            }
            if (Kind == PilotValueKind.NativeMap)
            {
                var nm = (Pilot.Runtime.NativeMemory.PilotNativeMap)_reference!;
                return nm.Get(index);
            }
            if (Kind == PilotValueKind.String)
            {
                string s = (string)_reference!;
                int idx = (int)index.AsInt();
                if (idx < 0) idx += s.Length;
                return (idx >= 0 && idx < s.Length) ? FromChar(s[idx]) : Undefined;
            }
            return Undefined;
        }
        set
        {
            if (Kind == PilotValueKind.NativeArray)
            {
                var na = (Pilot.Runtime.NativeMemory.PilotNativeArray)_reference!;
                int idx = (int)index.AsInt();
                if (idx < 0) idx += na.Length;
                if (idx >= 0 && idx < na.Length) na.SetValue(idx, value);
            }
            else if (Kind == PilotValueKind.NativeMap)
            {
                var nm = (Pilot.Runtime.NativeMemory.PilotNativeMap)_reference!;
                nm.Set(index, value);
            }
            else if (Kind == PilotValueKind.List)
            {
                var list = (List<PilotValue>)_reference!;
                int idx = (int)index.AsInt();
                if (idx < 0) idx += list.Count;
                if (idx >= 0 && idx < list.Count) list[idx] = value;
            }
            else if (Kind == PilotValueKind.Map)
            {
                var map = (Dictionary<PilotValue, PilotValue>)_reference!;
                map[index] = value;
            }
        }
    }

    public PilotValue GetMember(string memberName, PilotExecutionContext? context = null)
    {
        if (Kind == PilotValueKind.Object)
        {
            return ((PilotObject)_reference!).GetMember(memberName, context);
        }

        if (Kind == PilotValueKind.Generator)
        {
            return ((PilotGenerator)_reference!).GetMember(memberName);
        }

        if (Kind == PilotValueKind.Lazy)
        {
            return ((PilotLazy)_reference!).GetMember(memberName);
        }

        if (Kind == PilotValueKind.Future)
        {
            return ((Pilot.Runtime.Concurrency.PilotFuture)_reference!).GetMember(memberName);
        }

        if (Kind == PilotValueKind.Function)
        {
            var fn = (PilotFunction)_reference!;
            if (fn.TryGetMember(memberName, out var memberVal))
            {
                return memberVal;
            }
            if (memberName == "patch_version") return FromInt(fn.PatchVersion);
            if (memberName == "signature") return FromString(fn.GetSignatureKey());
            if (fn.ClassDef != null)
            {
                if (fn.ClassDef.MethodGroups.TryGetValue(memberName, out var mg))
                {
                    return FromMethodGroup(mg);
                }
                if (fn.ClassDef.Methods.TryGetValue(memberName, out var method))
                {
                    return FromFunction(method);
                }
                if (fn.ClassDef.StaticFields.TryGetValue(memberName, out var sField))
                {
                    return sField;
                }
            }
            if (fn.HostDelegate != null && fn.Parameters.Count == 0)
            {
                try
                {
                    var res = fn.HostDelegate(null!, Array.Empty<PilotValue>());
                    if (res.Kind == PilotValueKind.Object)
                    {
                        return res.GetMember(memberName);
                    }
                }
                catch { }
            }
            return ResolveRootMember(memberName);
        }

        if (Kind == PilotValueKind.MethodGroup)
        {
            var mg = (PilotMethodGroup)_reference!;
            return memberName switch
            {
                "overloads" => FromList(mg.GetOverloadValues()),
                "name" => FromString(mg.Name),
                "broadcast" => FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                {
                    return mg.Invoke(ctx, args, null, isBroadcast: true);
                })),
                _ => ResolveRootMember(memberName)
            };
        }

        // Collection methods
        if (Kind == PilotValueKind.NativeArray)
        {
            var na = (Pilot.Runtime.NativeMemory.PilotNativeArray)_reference!;
            return memberName switch
            {
                "length" or "count" => FromInt(na.Length),
                "to_list" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromList(na.ToList()))),
                "fill" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count > 0) na.Fill(args[0].AsFloat());
                    return Null;
                }, "val")),
                "sum" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromFloat(na.Sum()))),
                "dot" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count > 0 && args[0].Kind == PilotValueKind.NativeArray)
                    {
                        return FromFloat(na.Dot(args[0].AsNativeArray()));
                    }
                    return FromFloat(0.0);
                }, "other")),
                "scale" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count > 0) na.Scale(args[0].AsFloat());
                    return Null;
                }, "factor")),
                "copy_to" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count > 0 && args[0].Kind == PilotValueKind.NativeArray)
                    {
                        int cnt = args.Count > 1 ? (int)args[1].AsInt() : -1;
                        na.CopyTo(args[0].AsNativeArray(), cnt);
                    }
                    return Null;
                }, "dest", "count")),
                _ => ResolveRootMember(memberName)
            };
        }

        if (Kind == PilotValueKind.NativeMap)
        {
            var nm = (Pilot.Runtime.NativeMemory.PilotNativeMap)_reference!;
            return memberName switch
            {
                "length" or "count" => FromInt(nm.Count),
                "to_map" or "to_dict" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromMap(nm.ToDictionary()))),
                "get" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count == 0) return Undefined;
                    var def = args.Count > 1 ? args[1] : Undefined;
                    return nm.Get(args[0], def);
                }, "key", "default")),
                "set" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count >= 2) nm.Set(args[0], args[1]);
                    return Null;
                }, "key", "val")),
                "has" or "contains" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    return FromBool(args.Count > 0 && nm.Has(args[0]));
                }, "key")),
                "delete" or "remove" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    return FromBool(args.Count > 0 && nm.Delete(args[0]));
                }, "key")),
                "clear" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) =>
                {
                    nm.Clear();
                    return Null;
                })),
                "keys" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromList(nm.Keys()))),
                "values" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromList(nm.Values()))),
                _ => ResolveRootMember(memberName)
            };
        }

        if (Kind == PilotValueKind.List)
        {
            var list = (List<PilotValue>)_reference!;
            return memberName switch
            {
                "length" or "count" => FromInt(list.Count),
                "to_native" or "as_native" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromNativeArray(new Pilot.Runtime.NativeMemory.PilotNativeArray(list)))),
                "append" or "push" => FromFunction(CreateAppendFunction(memberName, list)),
                "map" => FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                {
                    if (args.Count == 0) return FromList(new List<PilotValue>(list));
                    var target = args[0];
                    var mapped = new List<PilotValue>(list.Count);
                    foreach (var item in list)
                    {
                        if (target.Kind == PilotValueKind.MethodGroup)
                        {
                            mapped.Add(target.AsMethodGroup().Invoke(ctx, new[] { item }, null, false));
                        }
                        else if (target.Kind == PilotValueKind.Function)
                        {
                            mapped.Add(target.AsFunction().Invoke(ctx, new[] { item }));
                        }
                        else
                        {
                            mapped.Add(item);
                        }
                    }
                    return FromList(mapped);
                }, "fn")),
                "pop" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) =>
                {
                    if (list.Count == 0) return Undefined;
                    var last = list[^1];
                    list.RemoveAt(list.Count - 1);
                    return last;
                })),
                "clear" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) =>
                {
                    list.Clear();
                    return Null;
                })),
                _ => ResolveRootMember(memberName)
            };
        }

        if (Kind == PilotValueKind.Map)
        {
            var map = (Dictionary<PilotValue, PilotValue>)_reference!;
            var strKey = FromString(memberName);
            if (map.TryGetValue(strKey, out var foundVal))
            {
                return foundVal;
            }

            return memberName switch
            {
                "length" or "count" => FromInt(map.Count),
                "to_native" or "as_native" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromNativeMap(new Pilot.Runtime.NativeMemory.PilotNativeMap(map)))),
                "keys" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromList(new List<PilotValue>(map.Keys)))),
                "values" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromList(new List<PilotValue>(map.Values)))),
                "contains_key" or "ContainsKey" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                    FromBool(args.Count > 0 && map.ContainsKey(args[0])))),
                "clear" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) =>
                {
                    map.Clear();
                    return Null;
                })),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        if (Kind == PilotValueKind.Set)
        {
            var set = (HashSet<PilotValue>)_reference!;
            return memberName switch
            {
                "length" or "count" => FromInt(set.Count),
                "add" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count > 0) set.Add(args[0]);
                    return Null;
                })),
                "contains" or "Contains" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                    FromBool(args.Count > 0 && set.Contains(args[0])))),
                "clear" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) =>
                {
                    set.Clear();
                    return Null;
                })),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        if (Kind == PilotValueKind.String)
        {
            string s = (string)_reference!;
            return memberName switch
            {
                "length" => FromInt(s.Length),
                "to_upper" or "ToUpper" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromString(s.ToUpperInvariant()))),
                "to_lower" or "ToLower" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromString(s.ToLowerInvariant()))),
                "trim" or "Trim" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromString(s.Trim()))),
                "starts_with" or "StartsWith" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                    FromBool(args.Count > 0 && s.StartsWith(args[0].ToString())))),
                "ends_with" or "EndsWith" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                    FromBool(args.Count > 0 && s.EndsWith(args[0].ToString())))),
                "contains" or "Contains" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                    FromBool(args.Count > 0 && s.Contains(args[0].ToString())))),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        return ResolveExtensionOrRoot(memberName, context);
    }

    private static IEnumerable<PilotValue> EnumerateCollection(PilotValue val)
    {
        if (val.Kind == PilotValueKind.List) return val.AsList();
        if (val.Kind == PilotValueKind.Tuple) return val.AsTuple();
        if (val.Kind == PilotValueKind.Set) return val.AsSet();
        return new[] { val };
    }

    private static PilotFunction CreateAppendFunction(string memberName, List<PilotValue> list)
    {
        var fn = PilotFunction.CreateNative(memberName, (ctx, args) =>
        {
            if (args.Count > 0)
            {
                var first = args[0];
                bool isRepeat = (ctx != null && ctx.IsAutoRepeatActive);
                if (args.Count > 1 && args[1].IsTrue) isRepeat = true;
                if (isRepeat && (first.Kind == PilotValueKind.List || first.Kind == PilotValueKind.Tuple || first.Kind == PilotValueKind.Set))
                {
                    foreach (var item in EnumerateCollection(first))
                    {
                        list.Add(item);
                    }
                }
                else
                {
                    list.Add(first);
                }
            }
            return Null;
        }, "item");

        var repeatFn = PilotFunction.CreateNative($"{memberName}.each", (_, args) =>
        {
            if (args.Count > 0 && (args[0].Kind == PilotValueKind.List || args[0].Kind == PilotValueKind.Tuple || args[0].Kind == PilotValueKind.Set))
            {
                foreach (var item in EnumerateCollection(args[0]))
                {
                    list.Add(item);
                }
            }
            else if (args.Count > 0)
            {
                list.Add(args[0]);
            }
            return Null;
        }, "items");

        fn.Members["each"] = FromFunction(repeatFn);
        fn.Members["repeat"] = FromFunction(repeatFn);
        fn.Members["broadcast"] = FromFunction(repeatFn);
        return fn;
    }

    private PilotValue ResolveExtensionOrRoot(string memberName, PilotExecutionContext? context)
    {
        if (context != null && context.ActiveExtensions.Count > 0)
        {
            var self = this;
            foreach (var ext in context.ActiveExtensions)
            {
                if (ext.MatchesType(self))
                {
                    if (ext.MethodGroups.TryGetValue(memberName, out var mg))
                    {
                        return FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                        {
                            var fullArgs = new List<PilotValue>(args.Count + 1) { self };
                            fullArgs.AddRange(args);
                            return mg.Invoke(ctx, fullArgs, null, false);
                        }));
                    }
                    if (ext.Methods.TryGetValue(memberName, out var extMethod))
                    {
                        return FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                        {
                            var fullArgs = new List<PilotValue>(args.Count + 1) { self };
                            fullArgs.AddRange(args);
                            return extMethod.Invoke(ctx, fullArgs);
                        }));
                    }
                }
            }
        }

        return ResolveRootMember(memberName);
    }

    private PilotValue ResolveRootMember(string memberName)
    {
        var self = this;
        return memberName switch
        {
            "to_string" or "ToString" or "__str__" => FromFunction(PilotFunction.CreateNative(
                memberName, (_, _) => FromString(self.ToString()))),
            "type_of" or "GetType" => FromFunction(PilotFunction.CreateNative(
                memberName, (_, _) => FromString(self.Kind.ToString().ToLowerInvariant()))),
            "hash" or "GetHashCode" or "__hash__" => FromFunction(PilotFunction.CreateNative(
                memberName, (_, _) => FromInt(self.GetHashCode()))),
            "clone" or "Clone" or "__clone__" => FromFunction(PilotFunction.CreateNative(
                memberName, (_, _) => self.CloneValue())),
            "equals" or "Equals" or "__eq__" => FromFunction(PilotFunction.CreateNative(
                memberName, (_, args) => FromBool(args.Count > 0 && self.Equals(args[0])))),
            "to_json" or "toJson" => FromFunction(PilotFunction.CreateNative(
                memberName, (_, args) => FromString(Pilot.Runtime.Serialization.PilotJsonSerializer.Serialize(self, args.Count > 0 && args[0].IsTrue)))),
            _ => Undefined
        };
    }

    public PilotValue CloneValue()
    {
        return Kind switch
        {
            PilotValueKind.List => FromList(new List<PilotValue>((List<PilotValue>)_reference!)),
            PilotValueKind.Map => FromMap(new Dictionary<PilotValue, PilotValue>((Dictionary<PilotValue, PilotValue>)_reference!)),
            PilotValueKind.Set => FromSet(new HashSet<PilotValue>((HashSet<PilotValue>)_reference!)),
            PilotValueKind.Object => FromObject(((PilotObject)_reference!).Clone()),
            _ => this
        };
    }

    #endregion

    #region Equality and Representation

    public bool Equals(PilotValue other)
    {
        if (Kind == PilotValueKind.Int && other.Kind == PilotValueKind.Int)
            return _integer == other._integer;

        if (Kind != other.Kind)
        {
            if (IsNumber && other.IsNumber)
                return Math.Abs(AsFloat() - other.AsFloat()) < 1e-9;
            return false;
        }

        return Kind switch
        {
            PilotValueKind.Null or PilotValueKind.Undefined => true,
            PilotValueKind.Bool => _integer == other._integer,
            PilotValueKind.Int or PilotValueKind.UInt or PilotValueKind.Char => _integer == other._integer,
            PilotValueKind.Float => Math.Abs(_floating - other._floating) < 1e-9,
            PilotValueKind.Decimal => Equals(_reference, other._reference),
            PilotValueKind.String => string.Equals((string)_reference!, (string)other._reference!, StringComparison.Ordinal),
            PilotValueKind.Keyword => _reference is PilotKeyword kw && other._reference is PilotKeyword okw && kw.Equals(okw),
            PilotValueKind.List => EqualsList((List<PilotValue>)_reference!, (List<PilotValue>)other._reference!),
            PilotValueKind.Tuple => EqualsTuple((IReadOnlyList<PilotValue>)_reference!, (IReadOnlyList<PilotValue>)other._reference!),
            PilotValueKind.Map => EqualsMap((Dictionary<PilotValue, PilotValue>)_reference!, (Dictionary<PilotValue, PilotValue>)other._reference!),
            PilotValueKind.Set => ((HashSet<PilotValue>)_reference!).SetEquals((HashSet<PilotValue>)other._reference!),
            PilotValueKind.Range => Equals(_reference, other._reference),
            PilotValueKind.Matrix => Equals(_reference, other._reference),
            PilotValueKind.Object => Equals(_reference, other._reference),
            _ => ReferenceEquals(_reference, other._reference)
        };
    }

    private static bool EqualsList(List<PilotValue> a, List<PilotValue> b)
    {
        if (a.Count != b.Count) return false;
        for (int i = 0; i < a.Count; i++) if (!a[i].Equals(b[i])) return false;
        return true;
    }

    private static bool EqualsTuple(IReadOnlyList<PilotValue> a, IReadOnlyList<PilotValue> b)
    {
        if (a.Count != b.Count) return false;
        for (int i = 0; i < a.Count; i++) if (!a[i].Equals(b[i])) return false;
        return true;
    }

    private static bool EqualsMap(Dictionary<PilotValue, PilotValue> a, Dictionary<PilotValue, PilotValue> b)
    {
        if (a.Count != b.Count) return false;
        foreach (var kvp in a)
        {
            if (!b.TryGetValue(kvp.Key, out var bVal) || !kvp.Value.Equals(bVal)) return false;
        }
        return true;
    }

    public override bool Equals(object? obj) => obj is PilotValue pv && Equals(pv);

    public override int GetHashCode() => Kind switch
    {
        PilotValueKind.Null => 0,
        PilotValueKind.Undefined => -1,
        PilotValueKind.Bool => _integer.GetHashCode(),
        PilotValueKind.Int or PilotValueKind.UInt or PilotValueKind.Char => _integer.GetHashCode(),
        PilotValueKind.Float => _floating.GetHashCode(),
        PilotValueKind.String => ((string)_reference!).GetHashCode(StringComparison.Ordinal),
        _ => _reference?.GetHashCode() ?? 0
    };

    public override string ToString() => Kind switch
    {
        PilotValueKind.Null => "null",
        PilotValueKind.Undefined => "undefined",
        PilotValueKind.Bool => _integer != 0 ? "true" : "false",
        PilotValueKind.Int => _integer.ToString(CultureInfo.InvariantCulture),
        PilotValueKind.UInt => unchecked((ulong)_integer).ToString(CultureInfo.InvariantCulture),
        PilotValueKind.Float => _floating.ToString("G", CultureInfo.InvariantCulture),
        PilotValueKind.Decimal => ((decimal)_reference!).ToString(CultureInfo.InvariantCulture),
        PilotValueKind.Char => ((char)_integer).ToString(),
        PilotValueKind.String => (string)_reference!,
        PilotValueKind.Keyword => _reference!.ToString()!,
        PilotValueKind.List => $"[{string.Join(", ", (List<PilotValue>)_reference!)}]",
        PilotValueKind.Tuple => $"({string.Join(", ", (IReadOnlyList<PilotValue>)_reference!)})",
        PilotValueKind.Map => $"{{{string.Join(", ", ((Dictionary<PilotValue, PilotValue>)_reference!).Select(k => $"{k.Key}: {k.Value}"))}}}",
        PilotValueKind.Set => $"#{{{string.Join(", ", (HashSet<PilotValue>)_reference!)}}}",
        PilotValueKind.Range => _reference!.ToString()!,
        PilotValueKind.Matrix => _reference!.ToString()!,
        PilotValueKind.NativeArray => _reference!.ToString()!,
        PilotValueKind.NativeMap => _reference!.ToString()!,
        PilotValueKind.Function => _reference!.ToString()!,
        PilotValueKind.Object => _reference!.ToString()!,
        PilotValueKind.Generator => _reference!.ToString()!,
        PilotValueKind.Lazy => _reference!.ToString()!,
        PilotValueKind.Future => _reference!.ToString()!,
        PilotValueKind.MethodGroup => _reference!.ToString()!,
        PilotValueKind.Foreign => _reference?.ToString() ?? "null",
        _ => "<unknown>"
    };

    #endregion
}
