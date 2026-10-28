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

    private sealed class PilotNestedSomeSentinel
    {
        public static readonly PilotNestedSomeSentinel Instance = new();
    }

    private sealed class PilotFailurePrimitiveSentinel
    {
        public PilotValueKind InnerKind { get; }
        public PilotFailurePrimitiveSentinel(PilotValueKind innerKind) => InnerKind = innerKind;

        public static readonly PilotFailurePrimitiveSentinel[] Table = CreateTable();
        private static PilotFailurePrimitiveSentinel[] CreateTable()
        {
            var vals = Enum.GetValues<PilotValueKind>();
            var arr = new PilotFailurePrimitiveSentinel[vals.Length + 1];
            foreach (var v in vals) arr[(int)v] = new PilotFailurePrimitiveSentinel(v);
            return arr;
        }
    }

    private sealed class PilotNestedFailureBox
    {
        public PilotValue Inner { get; }
        public PilotNestedFailureBox(PilotValue inner) => Inner = inner;
    }

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
    public static PilotValue FromSliceBuffer(Pilot.Runtime.NativeMemory.PilotSharedBuffer sb) => new(PilotValueKind.SliceBuffer, sb, 0, 0);
    public static PilotValue FromAdtType(PilotAdtDefinition adtDef) => new(PilotValueKind.AdtType, adtDef, 0, 0);
    public static PilotValue FromAdtUnit(PilotAdtVariantDefinition variantDef) => new(PilotValueKind.Adt, variantDef, 0, 0);
    public static PilotValue FromAdtBoxed(PilotAdtInstance instance) => new(PilotValueKind.Adt, instance, instance.Variant.Tag, 0);
    public static PilotValue FromRecordUnboxedEmpty(PilotRecordDefinition recDef) => new(PilotValueKind.Record, recDef, 0, 0);
    public static PilotValue FromRecordUnboxedPairF64(PilotRecordDefinition recDef, double f0, double f1) =>
        new(PilotValueKind.Record, recDef, BitConverter.DoubleToInt64Bits(f0), f1);
    public static PilotValue FromRecordBoxed(PilotRecordInstance instance) => new(PilotValueKind.Record, instance, 0, 0);
    public static PilotValue FromKeywordVariant(PilotKeywordVariant kwVar) => new(PilotValueKind.KeywordVariant, kwVar, 0, 0);

    /// <summary>
    /// Constructs an Option `some(inner)` value using zero-allocation niche optimization:
    /// - Any present non-null value is its own `some(v)` representation (0 heap allocation).
    /// - Nested `some(none)` (or `some(some(none))`) encodes its nesting depth inline in `_integer`
    ///   with a shared static sentinel (`PilotNestedSomeSentinel.Instance`, 0 heap allocation).
    /// </summary>
    public static PilotValue FromSome(PilotValue inner)
    {
        if (inner.IsNull || inner.IsUndefined)
        {
            return new PilotValue(PilotValueKind.Foreign, PilotNestedSomeSentinel.Instance, 1, 0);
        }
        if (inner._reference is PilotNestedSomeSentinel)
        {
            return new PilotValue(PilotValueKind.Foreign, PilotNestedSomeSentinel.Instance, inner._integer + 1, 0);
        }
        return inner;
    }

    /// <summary>
    /// Unwraps one layer of `Option[T]` when matching `some(x)` or narrowing `if some x` / `guard some x`.
    /// For normal niche-optimized values, returns `this` unchanged; for nested `some(none)`, unwraps one depth level.
    /// </summary>
    public PilotValue UnwrapOption()
    {
        if (_reference is PilotNestedSomeSentinel)
        {
            return _integer <= 1
                ? Null
                : new PilotValue(PilotValueKind.Foreign, PilotNestedSomeSentinel.Instance, _integer - 1, 0);
        }
        return this;
    }

    public bool IsNestedSome => _reference is PilotNestedSomeSentinel;
    public int NestedSomeDepth => _reference is PilotNestedSomeSentinel ? (int)_integer : 0;

    /// <summary>
    /// Constructs a `T ! E` error value (`fail(err)`) using the Milestone 13.1 zero-unwinding
    /// inline register representation (`{ u8 is_ok = 0; union { T ok; E err } }`).
    /// Preserves unboxed primitive, ADT, record, and string payloads directly inside the 16-byte
    /// <see cref="PilotValue"/> slot with zero heap wrapper allocations.
    /// </summary>
    public static PilotValue FromFailure(PilotValue err)
    {
        if (err.Kind == PilotValueKind.Failure)
        {
            return new PilotValue(PilotValueKind.Failure, new PilotNestedFailureBox(err), 0, 0);
        }
        if (err._reference == null)
        {
            return new PilotValue(
                PilotValueKind.Failure,
                PilotFailurePrimitiveSentinel.Table[(int)err.Kind],
                err._integer,
                err._floating);
        }
        return new PilotValue(PilotValueKind.Failure, err._reference, err._integer, err._floating);
    }

    /// <summary>
    /// Unwraps the error payload `E` from a `PilotValueKind.Failure` (`fail(err)`) slot.
    /// </summary>
    public PilotValue UnwrapFailure()
    {
        if (Kind != PilotValueKind.Failure) return this;
        if (_reference is PilotFailurePrimitiveSentinel prim)
        {
            return new PilotValue(prim.InnerKind, null, _integer, _floating);
        }
        if (_reference is PilotNestedFailureBox nested)
        {
            return nested.Inner;
        }
        return _reference switch
        {
            string s => FromString(s),
            PilotAdtVariantDefinition or PilotAdtFloatSlot or PilotAdtInstance => new PilotValue(PilotValueKind.Adt, _reference, _integer, _floating),
            PilotRecordDefinition or PilotRecordInstance => new PilotValue(PilotValueKind.Record, _reference, _integer, _floating),
            PilotKeyword kw => FromKeyword(kw),
            PilotKeywordVariant kv => FromKeywordVariant(kv),
            PilotObject obj => FromObject(obj),
            List<PilotValue> list => FromList(list),
            IReadOnlyList<PilotValue> tup => FromTuple(tup),
            Dictionary<PilotValue, PilotValue> map => FromMap(map),
            HashSet<PilotValue> set => FromSet(set),
            Pilot.Runtime.NativeMemory.PilotSharedBuffer sb => FromSliceBuffer(sb),
            decimal dec => FromDecimal(dec),
            PilotNestedSomeSentinel => new PilotValue(PilotValueKind.Foreign, _reference, _integer, _floating),
            _ => new PilotValue(PilotValueKind.Foreign, _reference, _integer, _floating)
        };
    }

    public bool IsFailure => Kind == PilotValueKind.Failure;
    public bool IsOk => Kind != PilotValueKind.Failure;

    public static bool TryCreateUnboxedAdtSingle(PilotAdtVariantDefinition variantDef, PilotValue field0, out PilotValue result)
    {
        switch (field0.Kind)
        {
            case PilotValueKind.Float:
                result = new PilotValue(PilotValueKind.Adt, variantDef.FloatSlot, 0, field0._floating);
                return true;
            case PilotValueKind.Int:
            case PilotValueKind.UInt:
            case PilotValueKind.Bool:
            case PilotValueKind.Char:
                result = new PilotValue(PilotValueKind.Adt, variantDef, field0._integer, (double)(int)field0.Kind);
                return true;
            default:
                result = Undefined;
                return false;
        }
    }

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
        PilotValueKind.SliceBuffer => ((Pilot.Runtime.NativeMemory.PilotSharedBuffer)_reference!).Length > 0,
        PilotValueKind.Null or PilotValueKind.Undefined or PilotValueKind.Failure => false,
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
    public Pilot.Runtime.NativeMemory.PilotSharedBuffer AsSliceBuffer() => _reference as Pilot.Runtime.NativeMemory.PilotSharedBuffer ?? throw new InvalidCastException($"Expected SliceBuffer, got {Kind}");
    public object? AsForeign() => _reference;

    public bool IsUnboxedInline =>
        Kind is PilotValueKind.Null or PilotValueKind.Bool or PilotValueKind.Int or PilotValueKind.UInt or PilotValueKind.Float or PilotValueKind.Char ||
        _reference is PilotNestedSomeSentinel or PilotFailurePrimitiveSentinel ||
        (Kind is PilotValueKind.Adt or PilotValueKind.Failure && _reference is PilotAdtVariantDefinition or PilotAdtFloatSlot) ||
        (Kind is PilotValueKind.Record or PilotValueKind.Failure && _reference is PilotRecordDefinition);

    public PilotAdtDefinition AsAdtType() => _reference as PilotAdtDefinition ?? throw new InvalidCastException($"Expected AdtType, got {Kind}");
    public PilotKeywordVariant AsKeywordVariant() => _reference as PilotKeywordVariant ?? throw new InvalidCastException($"Expected KeywordVariant, got {Kind}");

    public PilotAdtVariantDefinition GetAdtVariantDefinition()
    {
        if (Kind != PilotValueKind.Adt) throw new InvalidCastException($"Expected Adt, got {Kind}");
        if (_reference is PilotAdtVariantDefinition vd) return vd;
        if (_reference is PilotAdtFloatSlot fs) return fs.Variant;
        return ((PilotAdtInstance)_reference!).Variant;
    }

    public int GetAdtTag() => Kind == PilotValueKind.Adt ? GetAdtVariantDefinition().Tag : -1;

    public int GetAdtFieldCount() => Kind == PilotValueKind.Adt ? GetAdtVariantDefinition().Arity : 0;

    public PilotValue GetAdtField(int index)
    {
        if (Kind != PilotValueKind.Adt) return Undefined;
        if (_reference is PilotAdtFloatSlot)
        {
            return index == 0 ? FromFloat(_floating) : Undefined;
        }
        if (_reference is PilotAdtVariantDefinition vd)
        {
            if (index != 0 || vd.Arity == 0) return Undefined;
            return ((PilotValueKind)(int)_floating) switch
            {
                PilotValueKind.Int => FromInt(_integer),
                PilotValueKind.UInt => FromUInt(unchecked((ulong)_integer)),
                PilotValueKind.Bool => FromBool(_integer != 0),
                PilotValueKind.Char => FromChar((char)_integer),
                _ => FromInt(_integer)
            };
        }
        var inst = (PilotAdtInstance)_reference!;
        return index >= 0 && index < inst.Fields.Length ? inst.Fields[index] : Undefined;
    }

    public PilotRecordDefinition GetRecordDefinition()
    {
        if (Kind != PilotValueKind.Record) throw new InvalidCastException($"Expected Record, got {Kind}");
        if (_reference is PilotRecordDefinition rd) return rd;
        return ((PilotRecordInstance)_reference!).Definition;
    }

    public int GetRecordFieldCount() => Kind == PilotValueKind.Record ? GetRecordDefinition().Fields.Count : 0;

    public PilotValue GetRecordField(int index)
    {
        if (Kind != PilotValueKind.Record) return Undefined;
        if (_reference is PilotRecordDefinition rd)
        {
            if (rd.CanUnboxPairF64)
            {
                if (index == 0) return FromFloat(BitConverter.Int64BitsToDouble(_integer));
                if (index == 1) return FromFloat(_floating);
            }
            return Undefined;
        }
        var inst = (PilotRecordInstance)_reference!;
        return index >= 0 && index < inst.Fields.Length ? inst.Fields[index] : Undefined;
    }

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

    public static PilotValue operator ^(PilotValue a, PilotValue b)
    {
        if (a.Kind == PilotValueKind.Bool && b.Kind == PilotValueKind.Bool)
            return FromBool(a.IsTrue ^ b.IsTrue);

        return FromInt(a.AsInt() ^ b.AsInt());
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
                : (a.Kind is PilotValueKind.String or PilotValueKind.Char && b.Kind is PilotValueKind.String or PilotValueKind.Char)
                    ? string.CompareOrdinal(a.ToString(), b.ToString()) < 0
                    : a.AsInt() < b.AsInt();

    public static bool operator <=(PilotValue a, PilotValue b) =>
        (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
            ? a._integer <= b._integer
            : (a.Kind == PilotValueKind.Float || b.Kind == PilotValueKind.Float)
                ? a.AsFloat() <= b.AsFloat()
                : (a.Kind is PilotValueKind.String or PilotValueKind.Char && b.Kind is PilotValueKind.String or PilotValueKind.Char)
                    ? string.CompareOrdinal(a.ToString(), b.ToString()) <= 0
                    : a.AsInt() <= b.AsInt();

    public static bool operator >(PilotValue a, PilotValue b) =>
        (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
            ? a._integer > b._integer
            : (a.Kind == PilotValueKind.Float || b.Kind == PilotValueKind.Float)
                ? a.AsFloat() > b.AsFloat()
                : (a.Kind is PilotValueKind.String or PilotValueKind.Char && b.Kind is PilotValueKind.String or PilotValueKind.Char)
                    ? string.CompareOrdinal(a.ToString(), b.ToString()) > 0
                    : a.AsInt() > b.AsInt();

    public static bool operator >=(PilotValue a, PilotValue b) =>
        (a.Kind == PilotValueKind.Int && b.Kind == PilotValueKind.Int)
            ? a._integer >= b._integer
            : (a.Kind == PilotValueKind.Float || b.Kind == PilotValueKind.Float)
                ? a.AsFloat() >= b.AsFloat()
                : (a.Kind is PilotValueKind.String or PilotValueKind.Char && b.Kind is PilotValueKind.String or PilotValueKind.Char)
                    ? string.CompareOrdinal(a.ToString(), b.ToString()) >= 0
                    : a.AsInt() >= b.AsInt();

    #endregion

    #region Indexing and Member Resolution

    public PilotValue this[PilotValue index]
    {
        get
        {
            if (Kind == PilotValueKind.List)
            {
                var list = Pilot.Runtime.Effects.PilotEffectDryRunEngine.GetReadableList((List<PilotValue>)_reference!);
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
                return Pilot.Runtime.Effects.PilotEffectDryRunEngine.TryReadMapEntry(map, index, out var v) ? v : Undefined;
            }
            if (Kind == PilotValueKind.NativeArray)
            {
                var na = (Pilot.Runtime.NativeMemory.PilotNativeArray)_reference!;
                int idx = (int)index.AsInt();
                if (idx < 0) idx += na.Length;
                if (idx >= 0 && idx < na.Length && Pilot.Runtime.Effects.PilotEffectDryRunEngine.TryReadNativeArrayElement(na, idx, out var shadowElem))
                    return shadowElem;
                return (idx >= 0 && idx < na.Length) ? na.GetValue(idx) : Undefined;
            }
            if (Kind == PilotValueKind.NativeMap)
            {
                var nm = (Pilot.Runtime.NativeMemory.PilotNativeMap)_reference!;
                return nm.Get(index);
            }
            if (Kind == PilotValueKind.SliceBuffer)
            {
                var sb = (Pilot.Runtime.NativeMemory.PilotSharedBuffer)_reference!;
                if (index.Kind == PilotValueKind.Range)
                {
                    var r = index.AsRange();
                    if (r.Step == 1)
                        return FromSliceBuffer(sb.SliceView((int)r.Start, (int)r.Count));
                }
                int idx = (int)index.AsInt();
                if (Pilot.Runtime.Effects.PilotEffectDryRunEngine.TryReadSliceBufferElement(sb, idx, out var shadowBufElem))
                    return shadowBufElem;
                return sb.GetElement(idx);
            }
            if (Kind == PilotValueKind.Object)
            {
                var obj = (PilotObject)_reference!;
                if (obj.Fields.TryGetValue("__dual_tuple__", out var dualTuple) && dualTuple.Kind == PilotValueKind.Tuple)
                {
                    return dualTuple[index];
                }
            }
            if (Kind == PilotValueKind.String)
            {
                string s = (string)_reference!;
                int idx = (int)index.AsInt();
                if (idx < 0) idx += s.Length;
                return (idx >= 0 && idx < s.Length) ? FromChar(s[idx]) : Undefined;
            }
            if (Kind is PilotValueKind.AdtType or PilotValueKind.Function or PilotValueKind.MethodGroup)
            {
                return this;
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
                if (idx >= 0 && idx < na.Length)
                {
                    if (!Pilot.Runtime.Effects.PilotEffectDryRunEngine.TryRecordNativeArrayWrite(na, idx, value))
                        na.SetValue(idx, value);
                }
            }
            else if (Kind == PilotValueKind.NativeMap)
            {
                var nm = (Pilot.Runtime.NativeMemory.PilotNativeMap)_reference!;
                nm.Set(index, value);
            }
            else if (Kind == PilotValueKind.SliceBuffer)
            {
                var sb = (Pilot.Runtime.NativeMemory.PilotSharedBuffer)_reference!;
                int idx = (int)index.AsInt();
                if (!Pilot.Runtime.Effects.PilotEffectDryRunEngine.TryRecordSliceBufferWrite(sb, idx, value))
                    sb.SetElement(idx, value);
            }
            else if (Kind == PilotValueKind.List)
            {
                var list = (List<PilotValue>)_reference!;
                int idx = (int)index.AsInt();
                if (!Pilot.Runtime.Effects.PilotEffectDryRunEngine.TryRecordListIndexWrite(list, idx, value))
                {
                    if (idx < 0) idx += list.Count;
                    if (idx >= 0 && idx < list.Count) list[idx] = value;
                }
            }
            else if (Kind == PilotValueKind.Map)
            {
                var map = (Dictionary<PilotValue, PilotValue>)_reference!;
                if (!Pilot.Runtime.Effects.PilotEffectDryRunEngine.TryRecordMapWrite(map, index, value))
                    map[index] = value;
            }
        }
    }

    public PilotValue GetMember(string memberName, PilotExecutionContext? context = null)
    {
        if (Kind == PilotValueKind.AdtType)
        {
            var adt = (PilotAdtDefinition)_reference!;
            if (adt.TryGetVariant(memberName, out var variant) && variant != null)
            {
                return variant.IsUnit ? variant.UnitSingleton : FromFunction(variant.ConstructorFunction!);
            }
            return memberName switch
            {
                "name" => FromString(adt.Name),
                "tag_bytes" => FromInt(adt.Layout.TagBytes),
                "size_of" => FromInt(adt.Layout.TotalSizeBytes),
                "align_of" => FromInt(adt.Layout.AlignmentBytes),
                "is_recursive" => FromBool(adt.Layout.IsRecursive),
                _ => ResolveRootMember(memberName)
            };
        }

        if (Kind == PilotValueKind.Adt)
        {
            var vd = GetAdtVariantDefinition();
            int fIdx = vd.GetFieldIndex(memberName);
            if (fIdx >= 0)
            {
                return GetAdtField(fIdx);
            }
            return memberName switch
            {
                "tag" => FromInt(vd.Tag),
                "variant" or "variant_name" => FromString(vd.Name),
                "adt_name" => FromString(vd.ParentAdt.Name),
                "is_unboxed" => FromBool(IsUnboxedInline),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        if (Kind == PilotValueKind.Record)
        {
            var rd = GetRecordDefinition();
            int fIdx = rd.GetFieldIndex(memberName);
            if (fIdx >= 0)
            {
                return GetRecordField(fIdx);
            }
            if (rd.Methods.TryGetValue(memberName, out var recMethod))
            {
                var self = this;
                return FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                {
                    if (recMethod.Parameters.Count > 0 && recMethod.Parameters[0].Name == "self")
                    {
                        var fullArgs = new List<PilotValue>(args.Count + 1) { self };
                        fullArgs.AddRange(args);
                        return recMethod.Invoke(ctx, fullArgs);
                    }
                    return recMethod.Invoke(ctx, args);
                }));
            }
            return memberName switch
            {
                "is_unboxed" => FromBool(IsUnboxedInline),
                "size_of" => FromInt(rd.Layout.TotalSizeBytes),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        if (Kind == PilotValueKind.Keyword)
        {
            var kw = (PilotKeyword)_reference!;
            return memberName switch
            {
                "name" or "Name" => FromString(kw.Name),
                "namespace" or "Namespace" => kw.Namespace != null ? FromString(kw.Namespace) : Null,
                "full_name" or "FullName" => FromString(kw.FullName),
                "is_special_form" or "IsSpecialForm" => FromBool(kw.IsSpecialForm),
                "parents" or "Parents" => FromList(kw.GetParents().Select(FromKeyword).ToList()),
                "ancestors" or "Ancestors" => FromList(kw.GetAncestors().Select(FromKeyword).ToList()),
                "descendants" or "Descendants" => FromList(kw.GetDescendants().Select(FromKeyword).ToList()),
                "derive" or "Derive" => FromFunction(PilotFunction.CreateNative(memberName, (_, a) =>
                {
                    if (a.Count > 0 && a[0].Kind == PilotValueKind.Keyword)
                    {
                        kw.Derive(a[0].AsKeyword());
                    }
                    return FromKeyword(kw);
                }, "parent")),
                "underive" or "Underive" => FromFunction(PilotFunction.CreateNative(memberName, (_, a) =>
                {
                    if (a.Count > 0 && a[0].Kind == PilotValueKind.Keyword)
                    {
                        kw.Underive(a[0].AsKeyword());
                    }
                    return FromKeyword(kw);
                }, "parent")),
                "is_a" or "IsA" or "isa" => FromFunction(PilotFunction.CreateNative(memberName, (_, a) =>
                {
                    if (a.Count > 0 && a[0].Kind == PilotValueKind.Keyword)
                    {
                        return FromBool(kw.IsA(a[0].AsKeyword()));
                    }
                    return False;
                }, "parent")),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        if (Kind == PilotValueKind.KeywordVariant)
        {
            var kv = (PilotKeywordVariant)_reference!;
            return memberName switch
            {
                "keyword" or "tag" => FromKeyword(kv.Keyword),
                "value" => kv.Fields.Length > 0 ? kv.Fields[0] : Null,
                "values" or "fields" => FromList(new List<PilotValue>(kv.Fields)),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

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
            if (memberName is "name" or "Name") return FromString(fn.Name);
            if (memberName is "is_macro" or "IsMacro" or "is_special_form" or "IsSpecialForm") return FromBool(fn.IsMacro);
            if (memberName is "check" or "Check" or "matches" or "Matches")
            {
                return FromFunction(PilotFunction.CreateNative(memberName, (ctx, a) =>
                    FromBool(a.Count > 0 && PilotMethodGroup.MatchesSingleTypeFunction(fn, a[0], ctx)), "value"));
            }
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
                "overloads" or "Overloads" => FromList(mg.GetOverloadValues()),
                "name" or "Name" => FromString(mg.Name),
                "check" or "Check" or "matches" or "Matches" => FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                    FromBool(args.Count > 0 && mg.MatchesReifiedType(args[0], ctx)), "value")),
                "broadcast" => FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                {
                    return mg.Invoke(ctx, args, null, isBroadcast: true);
                })),
                _ => ResolveRootMember(memberName)
            };
        }

        // Collection methods
        if (Kind == PilotValueKind.SliceBuffer)
        {
            var sb = (Pilot.Runtime.NativeMemory.PilotSharedBuffer)_reference!;
            return memberName switch
            {
                "length" or "Length" or "count" or "Count" => FromInt(sb.Length),
                "capacity" or "Capacity" => FromInt(sb.Capacity),
                "is_empty" or "IsEmpty" => FromBool(sb.Length == 0),
                "data_ptr" or "DataPtr" or "ptr" => FromInt((long)sb.DataPointer),
                "header_ptr" or "HeaderPtr" => FromInt((long)sb.HeaderPointer),
                "abi_header_size" or "AbiHeaderSize" => FromInt(24),
                "stride" or "Stride" or "elem_size" => FromInt(sb.Stride),
                "byte_length" or "ByteLength" => FromInt(sb.ByteLength),
                "byte_capacity" or "ByteCapacity" => FromInt(sb.ByteCapacity),
                "elem_type" or "ElementType" => FromString(sb.ElementTypeName),
                "is_zero_copy" or "IsZeroCopy" => True,
                "is_shared_mmap" or "IsSharedMmap" or "is_shared" => FromBool(sb.IsSharedMmap),
                "managed_list_allocations" or "ManagedListAllocations" => FromInt(sb.ManagedListAllocations),
                "direct_yield_writes" or "DirectYieldWrites" => FromInt(sb.DirectYieldWrites),
                "backing_path" or "BackingPath" => sb.BackingPath != null ? FromString(sb.BackingPath) : Null,
                "open_view" or "OpenView" or "remap_view" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    int? lenOverride = args.Count > 0 ? (int)args[0].AsInt() : null;
                    return FromSliceBuffer(sb.OpenIndependentView(lenOverride));
                }, "length")),
                "slice" or "Slice" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    int start = args.Count > 0 ? (int)args[0].AsInt() : 0;
                    int len = args.Count > 1 ? (int)args[1].AsInt() : Math.Max(0, sb.Length - start);
                    return FromSliceBuffer(sb.SliceView(start, len));
                }, "start", "length")),
                "ensure_shared" or "EnsureShared" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    string? name = args.Count > 0 ? args[0].ToString() : null;
                    return FromString(sb.EnsureSharedMapping(name));
                }, "name")),
                "reset" or "Reset" or "clear" or "Clear" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) =>
                {
                    sb.ResetLength();
                    return Null;
                })),
                "yield_push" or "append" or "Append" or "push" or "Push" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    for (int i = 0; i < args.Count; i++) sb.YieldElement(args[i]);
                    return Null;
                }, "val")),
                "fill" or "Fill" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count > 0)
                    {
                        int? cnt = args.Count > 1 ? (int)args[1].AsInt() : null;
                        sb.Fill(args[0], cnt);
                    }
                    return Null;
                }, "val", "count")),
                "sum" or "Sum" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => sb.Sum())),
                "scale" or "Scale" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count > 0) sb.ScaleInPlace(args[0].AsFloat());
                    return Null;
                }, "factor")),
                "flush" or "Flush" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) =>
                {
                    sb.Flush();
                    return Null;
                })),
                "to_list" or "ToList" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromList(sb.ToPilotValueList()))),
                "dispose" or "Dispose" or "close" or "Close" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) =>
                {
                    sb.Dispose();
                    return Null;
                })),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        if (Kind == PilotValueKind.NativeArray)
        {
            var na = (Pilot.Runtime.NativeMemory.PilotNativeArray)_reference!;
            return memberName switch
            {
                "length" or "Length" or "count" or "Count" => FromInt(na.Length),
                "is_empty" or "IsEmpty" => FromBool(na.Length == 0),
                "to_list" or "ToList" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromList(na.ToList()))),
                "fill" or "Fill" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count > 0) na.Fill(args[0].AsFloat());
                    return Null;
                }, "val")),
                "sum" or "Sum" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromFloat(na.Sum()))),
                "dot" or "Dot" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count > 0 && args[0].Kind == PilotValueKind.NativeArray)
                    {
                        return FromFloat(na.Dot(args[0].AsNativeArray()));
                    }
                    return FromFloat(0.0);
                }, "other")),
                "scale" or "Scale" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count > 0) na.Scale(args[0].AsFloat());
                    return Null;
                }, "factor")),
                "copy_to" or "CopyTo" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count > 0 && args[0].Kind == PilotValueKind.NativeArray)
                    {
                        int cnt = args.Count > 1 ? (int)args[1].AsInt() : -1;
                        na.CopyTo(args[0].AsNativeArray(), cnt);
                    }
                    return Null;
                }, "dest", "count")),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        if (Kind == PilotValueKind.NativeMap)
        {
            var nm = (Pilot.Runtime.NativeMemory.PilotNativeMap)_reference!;
            return memberName switch
            {
                "length" or "Length" or "count" or "Count" => FromInt(nm.Count),
                "to_map" or "ToMap" or "to_dict" or "ToDict" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromMap(nm.ToDictionary()))),
                "get" or "Get" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count == 0) return Undefined;
                    var def = args.Count > 1 ? args[1] : Undefined;
                    return nm.Get(args[0], def);
                }, "key", "default")),
                "set" or "Set" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count >= 2) nm.Set(args[0], args[1]);
                    return Null;
                }, "key", "val")),
                "has" or "Has" or "contains" or "Contains" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    return FromBool(args.Count > 0 && nm.Has(args[0]));
                }, "key")),
                "delete" or "Delete" or "remove" or "Remove" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    return FromBool(args.Count > 0 && nm.Delete(args[0]));
                }, "key")),
                "clear" or "Clear" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) =>
                {
                    nm.Clear();
                    return Null;
                })),
                "keys" or "Keys" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromList(nm.Keys()))),
                "values" or "Values" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromList(nm.Values()))),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        if (Kind == PilotValueKind.List)
        {
            var list = (List<PilotValue>)_reference!;
            var self = this;
            return memberName switch
            {
                "length" or "Length" or "count" or "Count" => FromInt(list.Count),
                "is_empty" or "IsEmpty" => FromBool(list.Count == 0),
                "to_native" or "as_native" or "ToNative" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromNativeArray(new Pilot.Runtime.NativeMemory.PilotNativeArray(list)))),
                "append" or "Append" or "push" or "Push" => FromFunction(CreateAppendFunction(memberName, list)),
                "sum" or "Sum" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => ComputeCollectionSum(list))),
                "map" or "Map" => FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                {
                    if (args.Count == 0) return FromList(new List<PilotValue>(list));
                    return MapCollection(list, args[0], ctx);
                }, "fn")),
                "filter" or "Filter" => FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                {
                    if (args.Count == 0) return FromList(new List<PilotValue>(list));
                    return FilterCollection(list, args[0], ctx);
                }, "fn")),
                "any" or "Any" => FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                {
                    if (args.Count == 0) return FromBool(list.Count > 0);
                    foreach (var item in list)
                    {
                        if (InvokeSingleCallback(args[0], item, ctx).IsTrue) return True;
                    }
                    return False;
                }, "fn")),
                "all" or "All" => FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                {
                    if (args.Count == 0) return True;
                    foreach (var item in list)
                    {
                        if (!InvokeSingleCallback(args[0], item, ctx).IsTrue) return False;
                    }
                    return True;
                }, "fn")),
                "first" or "First" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => list.Count > 0 ? list[0] : Null)),
                "last" or "Last" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => list.Count > 0 ? list[^1] : Null)),
                "contains" or "Contains" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) => FromBool(args.Count > 0 && list.Contains(args[0])))),
                "join" or "Join" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    string sep = args.Count > 0 ? args[0].ToString() : "";
                    return FromString(string.Join(sep, list.Select(x => x.ToString())));
                }, "sep")),
                "reverse" or "Reverse" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) =>
                {
                    var rev = new List<PilotValue>(list);
                    rev.Reverse();
                    return FromList(rev);
                })),
                "pop" or "Pop" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) =>
                {
                    if (list.Count == 0) return Undefined;
                    var last = list[^1];
                    list.RemoveAt(list.Count - 1);
                    return last;
                })),
                "clear" or "Clear" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) =>
                {
                    list.Clear();
                    return Null;
                })),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        if (Kind == PilotValueKind.Tuple)
        {
            var tuple = (IReadOnlyList<PilotValue>)_reference!;
            return memberName switch
            {
                "length" or "Length" or "count" or "Count" => FromInt(tuple.Count),
                "is_empty" or "IsEmpty" => FromBool(tuple.Count == 0),
                "sum" or "Sum" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => ComputeCollectionSum(tuple))),
                "map" or "Map" => FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                    args.Count == 0 ? FromList(new List<PilotValue>(tuple)) : MapCollection(tuple, args[0], ctx), "fn")),
                "filter" or "Filter" => FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                    args.Count == 0 ? FromList(new List<PilotValue>(tuple)) : FilterCollection(tuple, args[0], ctx), "fn")),
                _ => ResolveExtensionOrRoot(memberName, context)
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
                "length" or "Length" or "count" or "Count" => FromInt(map.Count),
                "is_empty" or "IsEmpty" => FromBool(map.Count == 0),
                "to_native" or "as_native" or "ToNative" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromNativeMap(new Pilot.Runtime.NativeMemory.PilotNativeMap(map)))),
                "keys" or "Keys" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromList(new List<PilotValue>(map.Keys)))),
                "values" or "Values" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) => FromList(new List<PilotValue>(map.Values)))),
                "contains_key" or "ContainsKey" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                    FromBool(args.Count > 0 && map.ContainsKey(args[0])))),
                "clear" or "Clear" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) =>
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
                "length" or "Length" or "count" or "Count" => FromInt(set.Count),
                "is_empty" or "IsEmpty" => FromBool(set.Count == 0),
                "add" or "Add" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                {
                    if (args.Count > 0) set.Add(args[0]);
                    return Null;
                })),
                "contains" or "Contains" => FromFunction(PilotFunction.CreateNative(memberName, (_, args) =>
                    FromBool(args.Count > 0 && set.Contains(args[0])))),
                "clear" or "Clear" => FromFunction(PilotFunction.CreateNative(memberName, (_, _) =>
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
            var self = this;
            return memberName switch
            {
                "length" or "Length" or "count" or "Count" => FromInt(s.Length),
                "is_empty" or "IsEmpty" => FromBool(s.Length == 0),
                "is_digit" or "IsDigit" => FromBool(s.Length > 0 && s.All(char.IsDigit)),
                "is_alpha" or "IsAlpha" => FromBool(s.Length > 0 && s.All(char.IsLetter)),
                "is_whitespace" or "IsWhitespace" => FromBool(s.Length > 0 && s.All(char.IsWhiteSpace)),
                "code_point" or "CodePoint" => s.Length > 0 ? FromInt((int)s[0]) : FromInt(0),
                "to_upper" or "ToUpper" or " trim" or "Trim" or "trim" or "trim_start" or "TrimStart" or "trim_end" or "TrimEnd"
                or "to_lower" or "ToLower" or "chars" or "Chars" or "lines" or "Lines"
                or "to_int" or "ToInt" or "parse_int" or "ParseInt"
                or "to_float" or "ToFloat" or "parse_float" or "ParseFloat"
                or "to_bool" or "ToBool" or "parse_bool" or "ParseBool"
                or "substring" or "Substring" or "split" or "Split" or "replace" or "Replace"
                or "index_of" or "IndexOf" or "pad_left" or "PadLeft" or "pad_right" or "PadRight"
                or "starts_with" or "StartsWith" or "ends_with" or "EndsWith" or "contains" or "Contains" =>
                    FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                    {
                        var span = args is PilotValue[] arr ? arr.AsSpan() : new List<PilotValue>(args).ToArray().AsSpan();
                        if (TryInvokePrimitiveIntrinsic(self, memberName, span, ctx, out var res))
                        {
                            return res;
                        }
                        return Undefined;
                    })),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        if (Kind == PilotValueKind.Char)
        {
            char c = AsChar();
            var self = this;
            return memberName switch
            {
                "is_digit" or "IsDigit" => FromBool(char.IsDigit(c)),
                "is_alpha" or "IsAlpha" => FromBool(char.IsLetter(c)),
                "is_whitespace" or "IsWhitespace" => FromBool(char.IsWhiteSpace(c)),
                "code_point" or "CodePoint" => FromInt((int)c),
                "to_upper" or "ToUpper" or "to_lower" or "ToLower" =>
                    FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                    {
                        if (TryInvokePrimitiveIntrinsic(self, memberName, ReadOnlySpan<PilotValue>.Empty, ctx, out var res))
                            return res;
                        return Undefined;
                    })),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        if (Kind is PilotValueKind.Int or PilotValueKind.UInt)
        {
            long iv = AsInt();
            var self = this;
            return memberName switch
            {
                "is_even" or "IsEven" => FromBool(iv % 2 == 0),
                "is_odd" or "IsOdd" => FromBool(iv % 2 != 0),
                "abs" or "Abs" or "clamp" or "Clamp" or "to_hex" or "ToHex" or "to_binary" or "ToBinary"
                or "pow" or "Pow" or "pop_count" or "PopCount" or "bit_length" or "BitLength"
                or "times" or "Times" or "to_float" or "ToFloat" or "to_int" or "ToInt" or "sqrt" or "Sqrt"
                or "to_nanoseconds" or "ToNanoseconds" or "to_microseconds" or "ToMicroseconds"
                or "to_milliseconds" or "ToMilliseconds" or "to_seconds" or "ToSeconds"
                or "to_minutes" or "ToMinutes" or "to_hours" or "ToHours"
                or "period" or "Period" or "to_hertz" or "ToHertz" or "to_kilohertz" or "ToKilohertz" or "to_megahertz" or "ToMegahertz"
                or "to_bytes" or "ToBytes" or "to_kilobytes" or "ToKilobytes" or "to_kb" or "ToKB"
                or "to_megabytes" or "ToMegabytes" or "to_mb" or "ToMB"
                or "to_gigabytes" or "ToGigabytes" or "to_gb" or "ToGB" =>
                    FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                    {
                        var span = args is PilotValue[] arr ? arr.AsSpan() : new List<PilotValue>(args).ToArray().AsSpan();
                        if (TryInvokePrimitiveIntrinsic(self, memberName, span, ctx, out var res))
                            return res;
                        return Undefined;
                    })),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        if (Kind is PilotValueKind.Float or PilotValueKind.Decimal)
        {
            double fv = AsFloat();
            var self = this;
            return memberName switch
            {
                "is_nan" or "IsNaN" => FromBool(double.IsNaN(fv)),
                "is_infinite" or "IsInfinite" => FromBool(double.IsInfinity(fv)),
                "round" or "Round" or "floor" or "Floor" or "ceil" or "Ceil"
                or "abs" or "Abs" or "sqrt" or "Sqrt" or "clamp" or "Clamp" or "lerp" or "Lerp"
                or "pow" or "Pow" or "to_int" or "ToInt" or "to_float" or "ToFloat"
                or "to_nanoseconds" or "ToNanoseconds" or "to_microseconds" or "ToMicroseconds"
                or "to_milliseconds" or "ToMilliseconds" or "to_seconds" or "ToSeconds"
                or "to_minutes" or "ToMinutes" or "to_hours" or "ToHours"
                or "period" or "Period" or "to_hertz" or "ToHertz" or "to_kilohertz" or "ToKilohertz" or "to_megahertz" or "ToMegahertz"
                or "to_bytes" or "ToBytes" or "to_kilobytes" or "ToKilobytes" or "to_kb" or "ToKB"
                or "to_megabytes" or "ToMegabytes" or "to_mb" or "ToMB"
                or "to_gigabytes" or "ToGigabytes" or "to_gb" or "ToGB" =>
                    FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                    {
                        var span = args is PilotValue[] arr ? arr.AsSpan() : new List<PilotValue>(args).ToArray().AsSpan();
                        if (TryInvokePrimitiveIntrinsic(self, memberName, span, ctx, out var res))
                            return res;
                        return Undefined;
                    })),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        if (Kind == PilotValueKind.Bool)
        {
            var self = this;
            return memberName switch
            {
                "toggle" or "Toggle" or "select" or "Select" or "to_int" or "ToInt" =>
                    FromFunction(PilotFunction.CreateNative(memberName, (ctx, args) =>
                    {
                        var span = args is PilotValue[] arr ? arr.AsSpan() : new List<PilotValue>(args).ToArray().AsSpan();
                        if (TryInvokePrimitiveIntrinsic(self, memberName, span, ctx, out var res))
                            return res;
                        return Undefined;
                    })),
                _ => ResolveExtensionOrRoot(memberName, context)
            };
        }

        return ResolveExtensionOrRoot(memberName, context);
    }

    private static readonly PilotRecordDefinition BuiltinParseErrorRecordDef =
        new("ParseError", new[] { new PilotAdtFieldDefinition("message", "str", 0) });

    public static PilotValue CreateParseError(string message, PilotExecutionContext? context = null)
    {
        if (context != null && context.RecordDefinitions.TryGetValue("ParseError", out var userRec))
        {
            return userRec.Instantiate(context, new[] { FromString(message) });
        }
        return BuiltinParseErrorRecordDef.Instantiate(context, new[] { FromString(message) });
    }

    public static PilotValue ComputeCollectionSum(IReadOnlyList<PilotValue> items)
    {
        bool anyFloat = false;
        long intSum = 0;
        double floatSum = 0.0;
        for (int i = 0; i < items.Count; i++)
        {
            var item = items[i];
            if (item.Kind is PilotValueKind.Float or PilotValueKind.Decimal)
            {
                anyFloat = true;
                floatSum += item.AsFloat();
            }
            else if (item.Kind is PilotValueKind.Int or PilotValueKind.UInt)
            {
                long v = item.AsInt();
                intSum += v;
                floatSum += v;
            }
        }
        return anyFloat ? FromFloat(floatSum) : FromInt(intSum);
    }

    public static PilotValue InvokeSingleCallback(PilotValue callback, PilotValue item, PilotExecutionContext? ctx)
    {
        if (callback.Kind == PilotValueKind.MethodGroup)
        {
            return callback.AsMethodGroup().Invoke(ctx!, new[] { item }, null, false);
        }
        if (callback.Kind == PilotValueKind.Function)
        {
            return callback.AsFunction().Invoke(ctx!, new[] { item });
        }
        if (callback.Kind == PilotValueKind.Keyword)
        {
            return callback.AsKeyword().Invoke(item, Undefined);
        }
        return item;
    }

    public static PilotValue MapCollection(IReadOnlyList<PilotValue> items, PilotValue callback, PilotExecutionContext? ctx)
    {
        var mapped = new List<PilotValue>(items.Count);
        for (int i = 0; i < items.Count; i++)
        {
            mapped.Add(InvokeSingleCallback(callback, items[i], ctx));
        }
        return FromList(mapped);
    }

    public static PilotValue FilterCollection(IReadOnlyList<PilotValue> items, PilotValue callback, PilotExecutionContext? ctx)
    {
        var filtered = new List<PilotValue>();
        for (int i = 0; i < items.Count; i++)
        {
            var item = items[i];
            if (InvokeSingleCallback(callback, item, ctx).IsTrue)
            {
                filtered.Add(item);
            }
        }
        return FromList(filtered);
    }

    /// <summary>
    /// Milestone 14.1: Executes primitive and collection intrinsics directly on unboxed <see cref="PilotValue"/>
    /// receivers without allocating intermediate <see cref="PilotFunction"/> wrapper closures.
    /// </summary>
    public static bool TryInvokePrimitiveIntrinsic(
        PilotValue receiver,
        string memberName,
        ReadOnlySpan<PilotValue> args,
        PilotExecutionContext? context,
        out PilotValue result)
    {
        result = Undefined;
        int argc = args.Length;

        switch (receiver.Kind)
        {
            case PilotValueKind.String:
            {
                string s = receiver.AsString();
                switch (memberName)
                {
                    case "length" or "Length" or "count" or "Count" when argc == 0:
                        result = FromInt(s.Length);
                        return true;
                    case "is_empty" or "IsEmpty" when argc == 0:
                        result = FromBool(s.Length == 0);
                        return true;
                    case "to_upper" or "ToUpper" when argc == 0:
                        result = FromString(s.ToUpperInvariant());
                        return true;
                    case "to_lower" or "ToLower" when argc == 0:
                        result = FromString(s.ToLowerInvariant());
                        return true;
                    case "trim" or "Trim" when argc == 0:
                        result = FromString(s.Trim());
                        return true;
                    case "trim_start" or "TrimStart" when argc == 0:
                        result = FromString(s.TrimStart());
                        return true;
                    case "trim_end" or "TrimEnd" when argc == 0:
                        result = FromString(s.TrimEnd());
                        return true;
                    case "substring" or "Substring" when argc is 1 or 2:
                    {
                        int start = Math.Clamp((int)args[0].AsInt(), 0, s.Length);
                        if (argc == 1)
                        {
                            result = FromString(s.Substring(start));
                        }
                        else
                        {
                            int len = Math.Clamp((int)args[1].AsInt(), 0, s.Length - start);
                            result = FromString(s.Substring(start, len));
                        }
                        return true;
                    }
                    case "split" or "Split" when argc is 0 or 1:
                    {
                        string sep = argc > 0 ? args[0].ToString() : " ";
                        var parts = s.Split(sep);
                        var res = new List<PilotValue>(parts.Length);
                        for (int i = 0; i < parts.Length; i++) res.Add(FromString(parts[i]));
                        result = FromList(res);
                        return true;
                    }
                    case "replace" or "Replace" when argc == 2:
                        result = FromString(s.Replace(args[0].ToString(), args[1].ToString()));
                        return true;
                    case "index_of" or "IndexOf" when argc == 1:
                    {
                        int idx = s.IndexOf(args[0].ToString(), StringComparison.Ordinal);
                        result = idx >= 0 ? FromInt(idx) : Null;
                        return true;
                    }
                    case "pad_left" or "PadLeft" when argc is 1 or 2:
                    {
                        int width = (int)args[0].AsInt();
                        char pad = argc > 1 && args[1].ToString().Length > 0 ? args[1].ToString()[0] : ' ';
                        result = FromString(s.PadLeft(width, pad));
                        return true;
                    }
                    case "pad_right" or "PadRight" when argc is 1 or 2:
                    {
                        int width = (int)args[0].AsInt();
                        char pad = argc > 1 && args[1].ToString().Length > 0 ? args[1].ToString()[0] : ' ';
                        result = FromString(s.PadRight(width, pad));
                        return true;
                    }
                    case "starts_with" or "StartsWith" when argc == 1:
                        result = FromBool(s.StartsWith(args[0].ToString(), StringComparison.Ordinal));
                        return true;
                    case "ends_with" or "EndsWith" when argc == 1:
                        result = FromBool(s.EndsWith(args[0].ToString(), StringComparison.Ordinal));
                        return true;
                    case "contains" or "Contains" when argc == 1:
                        result = FromBool(s.Contains(args[0].ToString(), StringComparison.Ordinal));
                        return true;
                    case "chars" or "Chars" when argc == 0:
                    {
                        var chars = new List<PilotValue>(s.Length);
                        for (int i = 0; i < s.Length; i++) chars.Add(FromString(s[i].ToString()));
                        result = FromList(chars);
                        return true;
                    }
                    case "lines" or "Lines" when argc == 0:
                    {
                        var lines = s.Replace("\r\n", "\n").Split('\n');
                        var list = new List<PilotValue>(lines.Length);
                        for (int i = 0; i < lines.Length; i++) list.Add(FromString(lines[i]));
                        result = FromList(list);
                        return true;
                    }
                    case "to_int" or "ToInt" when argc == 0:
                        result = long.TryParse(s.Trim(), NumberStyles.Integer, CultureInfo.InvariantCulture, out var iv)
                            ? FromInt(iv)
                            : Null;
                        return true;
                    case "parse_int" or "ParseInt" when argc == 0:
                        result = long.TryParse(s.Trim(), NumberStyles.Integer, CultureInfo.InvariantCulture, out var piv)
                            ? FromInt(piv)
                            : FromFailure(CreateParseError($"Invalid integer literal: '{s}'", context));
                        return true;
                    case "to_float" or "ToFloat" when argc == 0:
                        result = double.TryParse(s.Trim(), NumberStyles.Float | NumberStyles.AllowThousands, CultureInfo.InvariantCulture, out var dv)
                            ? FromFloat(dv)
                            : Null;
                        return true;
                    case "parse_float" or "ParseFloat" when argc == 0:
                        result = double.TryParse(s.Trim(), NumberStyles.Float | NumberStyles.AllowThousands, CultureInfo.InvariantCulture, out var pdv)
                            ? FromFloat(pdv)
                            : FromFailure(CreateParseError($"Invalid float literal: '{s}'", context));
                        return true;
                    case "to_bool" or "ToBool" or "parse_bool" or "ParseBool" when argc == 0:
                    {
                        string trimmed = s.Trim();
                        if (bool.TryParse(trimmed, out var bv)) { result = FromBool(bv); return true; }
                        if (trimmed == "1") { result = True; return true; }
                        if (trimmed == "0") { result = False; return true; }
                        result = Null;
                        return true;
                    }
                    case "is_digit" or "IsDigit" when argc == 0:
                        result = FromBool(s.Length > 0 && s.All(char.IsDigit));
                        return true;
                    case "is_alpha" or "IsAlpha" when argc == 0:
                        result = FromBool(s.Length > 0 && s.All(char.IsLetter));
                        return true;
                    case "is_whitespace" or "IsWhitespace" when argc == 0:
                        result = FromBool(s.Length > 0 && s.All(char.IsWhiteSpace));
                        return true;
                    case "code_point" or "CodePoint" when argc == 0:
                        result = s.Length > 0 ? FromInt((int)s[0]) : FromInt(0);
                        return true;
                }
                break;
            }

            case PilotValueKind.Char:
            {
                char c = receiver.AsChar();
                switch (memberName)
                {
                    case "to_upper" or "ToUpper" when argc == 0:
                        result = FromChar(char.ToUpperInvariant(c));
                        return true;
                    case "to_lower" or "ToLower" when argc == 0:
                        result = FromChar(char.ToLowerInvariant(c));
                        return true;
                    case "is_digit" or "IsDigit" when argc == 0:
                        result = FromBool(char.IsDigit(c));
                        return true;
                    case "is_alpha" or "IsAlpha" when argc == 0:
                        result = FromBool(char.IsLetter(c));
                        return true;
                    case "is_whitespace" or "IsWhitespace" when argc == 0:
                        result = FromBool(char.IsWhiteSpace(c));
                        return true;
                    case "code_point" or "CodePoint" when argc == 0:
                        result = FromInt((int)c);
                        return true;
                }
                break;
            }

            case PilotValueKind.Int:
            case PilotValueKind.UInt:
            {
                long iv = receiver.AsInt();
                switch (memberName)
                {
                    case "abs" or "Abs" when argc == 0:
                        result = FromInt(Math.Abs(iv));
                        return true;
                    case "clamp" or "Clamp" when argc == 2:
                        result = FromInt(Math.Clamp(iv, args[0].AsInt(), args[1].AsInt()));
                        return true;
                    case "to_hex" or "ToHex" when argc == 0:
                        result = FromString(iv.ToString("X", CultureInfo.InvariantCulture));
                        return true;
                    case "to_binary" or "ToBinary" when argc == 0:
                        result = FromString(Convert.ToString(iv, 2));
                        return true;
                    case "is_even" or "IsEven" when argc == 0:
                        result = FromBool(iv % 2 == 0);
                        return true;
                    case "is_odd" or "IsOdd" when argc == 0:
                        result = FromBool(iv % 2 != 0);
                        return true;
                    case "pow" or "Pow" when argc == 1:
                    {
                        if (args[0].Kind is PilotValueKind.Int or PilotValueKind.UInt && args[0].AsInt() >= 0)
                        {
                            result = FromInt((long)Math.Pow(iv, args[0].AsInt()));
                        }
                        else
                        {
                            result = FromFloat(Math.Pow(iv, args[0].AsFloat()));
                        }
                        return true;
                    }
                    case "pop_count" or "PopCount" when argc == 0:
                        result = FromInt(System.Numerics.BitOperations.PopCount((ulong)iv));
                        return true;
                    case "bit_length" or "BitLength" when argc == 0:
                    {
                        ulong mag = (ulong)Math.Abs(iv);
                        result = FromInt(mag == 0 ? 0 : 64 - System.Numerics.BitOperations.LeadingZeroCount(mag));
                        return true;
                    }
                    case "times" or "Times" when argc is 0 or 1:
                    {
                        if (argc == 0)
                        {
                            result = FromRange(new PilotRange(0, iv, 1, false));
                            return true;
                        }
                        var cb = args[0];
                        for (long i = 0; i < iv; i++)
                        {
                            InvokeSingleCallback(cb, FromInt(i), context);
                        }
                        result = Null;
                        return true;
                    }
                    case "to_float" or "ToFloat" when argc == 0:
                        result = FromFloat((double)iv);
                        return true;
                    case "to_int" or "ToInt" when argc == 0:
                        result = receiver;
                        return true;
                    case "sqrt" or "Sqrt" when argc == 0:
                        result = FromFloat(Math.Sqrt(iv));
                        return true;
                    case "to_bytes" or "ToBytes" when argc == 0:
                        result = FromInt(iv);
                        return true;
                    case "to_kilobytes" or "ToKilobytes" or "to_kb" or "ToKB" when argc == 0:
                        result = FromFloat(iv / 1024.0);
                        return true;
                    case "to_megabytes" or "ToMegabytes" or "to_mb" or "ToMB" when argc == 0:
                        result = FromFloat(iv / (1024.0 * 1024.0));
                        return true;
                    case "to_gigabytes" or "ToGigabytes" or "to_gb" or "ToGB" when argc == 0:
                        result = FromFloat(iv / (1024.0 * 1024.0 * 1024.0));
                        return true;
                    case "period" or "Period" when argc == 0:
                        result = FromFloat(1.0 / iv);
                        return true;
                    case "to_nanoseconds" or "ToNanoseconds" when argc == 0:
                        result = FromFloat(iv * 1e9);
                        return true;
                    case "to_microseconds" or "ToMicroseconds" when argc == 0:
                        result = FromFloat(iv * 1e6);
                        return true;
                    case "to_milliseconds" or "ToMilliseconds" when argc == 0:
                        result = FromFloat(iv * 1e3);
                        return true;
                    case "to_seconds" or "ToSeconds" when argc == 0:
                        result = FromFloat((double)iv);
                        return true;
                }
                break;
            }

            case PilotValueKind.Float:
            case PilotValueKind.Decimal:
            {
                double fv = receiver.AsFloat();
                switch (memberName)
                {
                    case "round" or "Round" when argc is 0 or 1:
                    {
                        int digits = argc > 0 ? Math.Clamp((int)args[0].AsInt(), 0, 15) : 0;
                        result = FromFloat(Math.Round(fv, digits, MidpointRounding.AwayFromZero));
                        return true;
                    }
                    case "floor" or "Floor" when argc == 0:
                        result = FromFloat(Math.Floor(fv));
                        return true;
                    case "ceil" or "Ceil" when argc == 0:
                        result = FromFloat(Math.Ceiling(fv));
                        return true;
                    case "abs" or "Abs" when argc == 0:
                        result = FromFloat(Math.Abs(fv));
                        return true;
                    case "sqrt" or "Sqrt" when argc == 0:
                        result = FromFloat(Math.Sqrt(fv));
                        return true;
                    case "clamp" or "Clamp" when argc == 2:
                        result = FromFloat(Math.Clamp(fv, args[0].AsFloat(), args[1].AsFloat()));
                        return true;
                    case "lerp" or "Lerp" when argc == 2:
                    {
                        double a = args[0].AsFloat();
                        double b = args[1].AsFloat();
                        result = FromFloat(a + (b - a) * fv);
                        return true;
                    }
                    case "pow" or "Pow" when argc == 1:
                        result = FromFloat(Math.Pow(fv, args[0].AsFloat()));
                        return true;
                    case "is_nan" or "IsNaN" when argc == 0:
                        result = FromBool(double.IsNaN(fv));
                        return true;
                    case "is_infinite" or "IsInfinite" when argc == 0:
                        result = FromBool(double.IsInfinity(fv));
                        return true;
                    case "to_int" or "ToInt" when argc == 0:
                        result = FromInt((long)fv);
                        return true;
                    case "to_float" or "ToFloat" when argc == 0:
                        result = FromFloat(fv);
                        return true;
                    case "to_nanoseconds" or "ToNanoseconds" when argc == 0:
                        result = FromFloat(Math.Round(fv * 1e9, 6));
                        return true;
                    case "to_microseconds" or "ToMicroseconds" when argc == 0:
                        result = FromFloat(Math.Round(fv * 1e6, 6));
                        return true;
                    case "to_milliseconds" or "ToMilliseconds" when argc == 0:
                        result = FromFloat(Math.Round(fv * 1e3, 6));
                        return true;
                    case "to_seconds" or "ToSeconds" when argc == 0:
                        result = FromFloat(fv);
                        return true;
                    case "to_minutes" or "ToMinutes" when argc == 0:
                        result = FromFloat(fv / 60.0);
                        return true;
                    case "to_hours" or "ToHours" when argc == 0:
                        result = FromFloat(fv / 3600.0);
                        return true;
                    case "period" or "Period" when argc == 0:
                        result = FromFloat(1.0 / fv);
                        return true;
                    case "to_hertz" or "ToHertz" when argc == 0:
                        result = FromFloat(fv);
                        return true;
                    case "to_kilohertz" or "ToKilohertz" when argc == 0:
                        result = FromFloat(fv / 1e3);
                        return true;
                    case "to_megahertz" or "ToMegahertz" when argc == 0:
                        result = FromFloat(fv / 1e6);
                        return true;
                    case "to_bytes" or "ToBytes" when argc == 0:
                        result = FromInt((long)fv);
                        return true;
                    case "to_kilobytes" or "ToKilobytes" or "to_kb" or "ToKB" when argc == 0:
                        result = FromFloat(fv / 1024.0);
                        return true;
                    case "to_megabytes" or "ToMegabytes" or "to_mb" or "ToMB" when argc == 0:
                        result = FromFloat(fv / (1024.0 * 1024.0));
                        return true;
                    case "to_gigabytes" or "ToGigabytes" or "to_gb" or "ToGB" when argc == 0:
                        result = FromFloat(fv / (1024.0 * 1024.0 * 1024.0));
                        return true;
                }
                break;
            }

            case PilotValueKind.Bool:
            {
                bool bv = receiver.IsTrue;
                switch (memberName)
                {
                    case "toggle" or "Toggle" when argc == 0:
                        result = FromBool(!bv);
                        return true;
                    case "select" or "Select" when argc == 2:
                        result = bv ? args[0] : args[1];
                        return true;
                    case "to_int" or "ToInt" when argc == 0:
                        result = FromInt(bv ? 1 : 0);
                        return true;
                }
                break;
            }

            case PilotValueKind.List:
            {
                var list = receiver.AsList();
                switch (memberName)
                {
                    case "append" or "Append" or "push" or "Push" when argc == 1:
                        if (!Pilot.Runtime.Effects.PilotEffectDryRunEngine.TryRecordListAppend(list, args[0]))
                            list.Add(args[0]);
                        result = Null;
                        return true;
                    case "pop" or "Pop" when argc == 0:
                        if (list.Count == 0) result = Undefined;
                        else { result = list[^1]; list.RemoveAt(list.Count - 1); }
                        return true;
                    case "clear" or "Clear" when argc == 0:
                        list.Clear();
                        result = Null;
                        return true;
                    case "sum" or "Sum" when argc == 0:
                        result = ComputeCollectionSum(list);
                        return true;
                    case "map" or "Map" when argc == 1:
                        result = MapCollection(list, args[0], context);
                        return true;
                    case "filter" or "Filter" when argc == 1:
                        result = FilterCollection(list, args[0], context);
                        return true;
                    case "first" or "First" when argc == 0:
                        result = list.Count > 0 ? list[0] : Null;
                        return true;
                    case "last" or "Last" when argc == 0:
                        result = list.Count > 0 ? list[^1] : Null;
                        return true;
                    case "contains" or "Contains" when argc == 1:
                        result = FromBool(list.Contains(args[0]));
                        return true;
                    case "join" or "Join" when argc is 0 or 1:
                        result = FromString(string.Join(argc > 0 ? args[0].ToString() : "", list.Select(x => x.ToString())));
                        return true;
                    case "reverse" or "Reverse" when argc == 0:
                    {
                        var rev = new List<PilotValue>(list);
                        rev.Reverse();
                        result = FromList(rev);
                        return true;
                    }
                }
                break;
            }

            case PilotValueKind.Keyword:
            {
                var kw = receiver.AsKeyword();
                switch (memberName)
                {
                    case "derive" or "Derive" when argc >= 1 && args[0].Kind == PilotValueKind.Keyword:
                        kw.Derive(args[0].AsKeyword());
                        result = receiver;
                        return true;
                    case "underive" or "Underive" when argc >= 1 && args[0].Kind == PilotValueKind.Keyword:
                        kw.Underive(args[0].AsKeyword());
                        result = receiver;
                        return true;
                    case "is_a" or "IsA" or "isa" when argc >= 1 && args[0].Kind == PilotValueKind.Keyword:
                        result = FromBool(kw.IsA(args[0].AsKeyword()));
                        return true;
                    case "parents" or "Parents" when argc == 0:
                        result = FromList(kw.GetParents().Select(FromKeyword).ToList());
                        return true;
                    case "ancestors" or "Ancestors" when argc == 0:
                        result = FromList(kw.GetAncestors().Select(FromKeyword).ToList());
                        return true;
                    case "descendants" or "Descendants" when argc == 0:
                        result = FromList(kw.GetDescendants().Select(FromKeyword).ToList());
                        return true;
                }
                break;
            }

            case PilotValueKind.MethodGroup:
            {
                var mg = receiver.AsMethodGroup();
                switch (memberName)
                {
                    case "check" or "Check" or "matches" or "Matches" when argc >= 1:
                        result = FromBool(mg.MatchesReifiedType(args[0], context));
                        return true;
                }
                break;
            }
        }

        // Check active extensions directly without allocating a bound wrapper closure
        if (context != null && context.ActiveExtensions.Count > 0)
        {
            foreach (var ext in context.ActiveExtensions)
            {
                if (ext.MatchesType(receiver))
                {
                    if (ext.MethodGroups.TryGetValue(memberName, out var mg))
                    {
                        var fullArgs = new PilotValue[argc + 1];
                        fullArgs[0] = receiver;
                        for (int i = 0; i < argc; i++) fullArgs[i + 1] = args[i];
                        result = mg.Invoke(context, fullArgs, null, false);
                        return true;
                    }
                    if (ext.Methods.TryGetValue(memberName, out var extMethod))
                    {
                        var fullArgs = new PilotValue[argc + 1];
                        fullArgs[0] = receiver;
                        for (int i = 0; i < argc; i++) fullArgs[i + 1] = args[i];
                        result = extMethod.Invoke(context, fullArgs);
                        return true;
                    }
                }
            }
        }

        return false;
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
                    if (!Pilot.Runtime.Effects.PilotEffectDryRunEngine.TryRecordListAppend(list, first))
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
                    if (ext.Properties.TryGetValue(memberName, out var propGetter))
                    {
                        return propGetter.Invoke(context, new[] { self });
                    }
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
            "or_panic" or "OrPanic" => FromFunction(PilotFunction.CreateNative(
                memberName, (_, args) =>
                {
                    if (self.IsFailure)
                    {
                        string msg = args.Count > 0 ? args[0].ToString() : "Unwrapped 'fail' error value";
                        throw new PilotRuntimeException($"[ERR005] {msg}: {self.UnwrapFailure()}");
                    }
                    if (self.IsNull || self.IsUndefined)
                    {
                        string msg = args.Count > 0 ? args[0].ToString() : "Unwrapped 'none' optional value";
                        throw new PilotRuntimeException($"[NULL005] {msg}");
                    }
                    return self.UnwrapOption();
                }, "message")),
            "unwrap_or" or "UnwrapOr" => FromFunction(PilotFunction.CreateNative(
                memberName, (_, args) =>
                {
                    if (self.IsFailure || self.IsNull || self.IsUndefined)
                    {
                        return args.Count > 0 ? args[0] : Null;
                    }
                    return self.UnwrapOption();
                }, "fallback")),
            "is_ok" or "IsOk" => FromBool(!self.IsFailure),
            "is_err" or "IsErr" or "is_fail" or "IsFail" => FromBool(self.IsFailure),
            "err_value" or "ErrValue" or "error" => self.IsFailure ? self.UnwrapFailure() : Null,
            "ok_value" or "OkValue" => self.IsFailure ? Null : self,
            "is_some" or "IsSome" => FromBool(!self.IsNull && !self.IsUndefined),
            "is_none" or "IsNone" => FromBool(self.IsNull || self.IsUndefined),
            "is_unboxed" or "IsUnboxed" => FromBool(self.IsUnboxedInline),
            "has_niche" or "HasNiche" => True,
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
            if ((Kind is PilotValueKind.Null or PilotValueKind.Undefined) &&
                (other.Kind is PilotValueKind.Null or PilotValueKind.Undefined))
                return true;
            if (IsNumber && other.IsNumber)
                return Math.Abs(AsFloat() - other.AsFloat()) < 1e-9;
            if (Kind == PilotValueKind.Char && other.Kind == PilotValueKind.String && other.AsString().Length == 1)
                return (char)_integer == other.AsString()[0];
            if (Kind == PilotValueKind.String && AsString().Length == 1 && other.Kind == PilotValueKind.Char)
                return AsString()[0] == (char)other._integer;
            if (Kind == PilotValueKind.SliceBuffer && other.Kind == PilotValueKind.List)
                return EqualsList(((Pilot.Runtime.NativeMemory.PilotSharedBuffer)_reference!).ToPilotValueList(), (List<PilotValue>)other._reference!);
            if (Kind == PilotValueKind.List && other.Kind == PilotValueKind.SliceBuffer)
                return EqualsList((List<PilotValue>)_reference!, ((Pilot.Runtime.NativeMemory.PilotSharedBuffer)other._reference!).ToPilotValueList());
            if (Kind == PilotValueKind.Object && other.Kind == PilotValueKind.Tuple &&
                ((PilotObject)_reference!).Fields.TryGetValue("__dual_tuple__", out var dtA) && dtA.Kind == PilotValueKind.Tuple)
                return dtA.Equals(other);
            if (Kind == PilotValueKind.Tuple && other.Kind == PilotValueKind.Object &&
                ((PilotObject)other._reference!).Fields.TryGetValue("__dual_tuple__", out var dtB) && dtB.Kind == PilotValueKind.Tuple)
                return Equals(dtB);
            if (Kind == PilotValueKind.Object && ((PilotObject)_reference!).NativePayload is Pilot.Runtime.Reactive.PilotSignalNode sigA)
                return sigA.Read().Equals(other);
            if (other.Kind == PilotValueKind.Object && ((PilotObject)other._reference!).NativePayload is Pilot.Runtime.Reactive.PilotSignalNode sigB)
                return Equals(sigB.Read());
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
            PilotValueKind.SliceBuffer => _reference is Pilot.Runtime.NativeMemory.PilotSharedBuffer sb && other._reference is Pilot.Runtime.NativeMemory.PilotSharedBuffer osb && sb.Equals(osb),
            PilotValueKind.Object => Equals(_reference, other._reference),
            PilotValueKind.Adt => EqualsAdt(other),
            PilotValueKind.Record => EqualsRecord(other),
            PilotValueKind.KeywordVariant => _reference is PilotKeywordVariant kv && other._reference is PilotKeywordVariant okv && kv.Equals(okv),
            PilotValueKind.Failure => UnwrapFailure().Equals(other.UnwrapFailure()),
            PilotValueKind.Foreign when _reference is PilotNestedSomeSentinel || other._reference is PilotNestedSomeSentinel =>
                _reference is PilotNestedSomeSentinel && other._reference is PilotNestedSomeSentinel && _integer == other._integer,
            _ => ReferenceEquals(_reference, other._reference)
        };
    }

    private bool EqualsAdt(PilotValue other)
    {
        var vA = GetAdtVariantDefinition();
        var vB = other.GetAdtVariantDefinition();
        if (!ReferenceEquals(vA, vB) && !(vA.Tag == vB.Tag && vA.Name == vB.Name && vA.ParentAdt.Name == vB.ParentAdt.Name))
            return false;
        int arity = vA.Arity;
        for (int i = 0; i < arity; i++)
        {
            if (!GetAdtField(i).Equals(other.GetAdtField(i)))
                return false;
        }
        return true;
    }

    private bool EqualsRecord(PilotValue other)
    {
        var rA = GetRecordDefinition();
        var rB = other.GetRecordDefinition();
        if (rA.Name != rB.Name || rA.Fields.Count != rB.Fields.Count)
            return false;
        int count = rA.Fields.Count;
        for (int i = 0; i < count; i++)
        {
            if (!GetRecordField(i).Equals(other.GetRecordField(i)))
                return false;
        }
        return true;
    }

    private int ComputeAdtHashCode()
    {
        var vd = GetAdtVariantDefinition();
        var hc = new HashCode();
        hc.Add(vd.ParentAdt.Name, StringComparer.Ordinal);
        hc.Add(vd.Tag);
        for (int i = 0; i < vd.Arity; i++) hc.Add(GetAdtField(i));
        return hc.ToHashCode();
    }

    private int ComputeRecordHashCode()
    {
        var rd = GetRecordDefinition();
        var hc = new HashCode();
        hc.Add(rd.Name, StringComparer.Ordinal);
        for (int i = 0; i < rd.Fields.Count; i++) hc.Add(GetRecordField(i));
        return hc.ToHashCode();
    }

    private string FormatAdt()
    {
        if (_reference is PilotAdtInstance inst) return inst.ToString();
        var vd = GetAdtVariantDefinition();
        if (vd.Arity == 0) return vd.Name;
        var f0 = GetAdtField(0);
        string? fName = vd.Fields[0].Name;
        return fName != null ? $"{vd.Name}({fName}: {f0})" : $"{vd.Name}({f0})";
    }

    private string FormatRecord()
    {
        if (_reference is PilotRecordInstance inst) return inst.ToString();
        var rd = GetRecordDefinition();
        if (rd.Fields.Count == 0) return $"{rd.Name}()";
        var parts = new string[rd.Fields.Count];
        for (int i = 0; i < rd.Fields.Count; i++)
        {
            string? fName = rd.Fields[i].Name;
            parts[i] = fName != null ? $"{fName}: {GetRecordField(i)}" : GetRecordField(i).ToString();
        }
        return $"{rd.Name}({string.Join(", ", parts)})";
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
        PilotValueKind.Adt => ComputeAdtHashCode(),
        PilotValueKind.Record => ComputeRecordHashCode(),
        PilotValueKind.Failure => HashCode.Combine(0x4641494C, UnwrapFailure().GetHashCode()),
        PilotValueKind.Foreign when _reference is PilotNestedSomeSentinel => HashCode.Combine(0x50494C53, _integer),
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
        PilotValueKind.SliceBuffer => _reference!.ToString()!,
        PilotValueKind.Function => _reference!.ToString()!,
        PilotValueKind.Object => _reference!.ToString()!,
        PilotValueKind.Generator => _reference!.ToString()!,
        PilotValueKind.Lazy => _reference!.ToString()!,
        PilotValueKind.Future => _reference!.ToString()!,
        PilotValueKind.MethodGroup => _reference!.ToString()!,
        PilotValueKind.Adt => FormatAdt(),
        PilotValueKind.AdtType => _reference!.ToString()!,
        PilotValueKind.Record => FormatRecord(),
        PilotValueKind.KeywordVariant => _reference!.ToString()!,
        PilotValueKind.Failure => $"fail({UnwrapFailure()})",
        PilotValueKind.Foreign when _reference is PilotNestedSomeSentinel => FormatNestedSome(_integer),
        PilotValueKind.Foreign => _reference?.ToString() ?? "null",
        _ => "<unknown>"
    };

    private static string FormatNestedSome(long depth)
    {
        string s = "null";
        for (long i = 0; i < depth; i++) s = $"some({s})";
        return s;
    }

    #endregion
}
