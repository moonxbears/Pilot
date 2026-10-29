using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;

namespace Pilot.Compiler;

/// <summary>
/// Canonical 16-bit syntax/AST node discriminator (<c>0..65535</c>) matching <c>pilot_ast::NodeKind</c>.
/// </summary>
public enum NativeNodeKind : ushort
{
    // --- Root & Module Declarations (0..99) ---
    Program = 0,
    ImportDecl = 1,
    FromImportDecl = 2,
    FnDecl = 3,
    ParamDecl = 4,
    TypeAdtDecl = 5,
    AdtVariant = 6,
    RecordDecl = 7,
    StructDecl = 8,
    ClassDecl = 9,
    InterfaceDecl = 10,
    ExtendDecl = 11,
    ConstDecl = 12,

    // --- Statements & Control Flow (100..199) ---
    LetStmt = 100,
    VarStmt = 101,
    AssignStmt = 102,
    ReturnStmt = 103,
    GuardStmt = 104,
    DeferStmt = 105,
    WhileStmt = 106,
    BreakStmt = 107,
    ContinueStmt = 108,
    YieldStmt = 109,
    ExprStmt = 110,

    // --- Expressions (200..299) ---
    BlockExpr = 200,
    IfExpr = 201,
    MatchExpr = 202,
    MatchArm = 203,
    ForExpr = 204,
    BinaryExpr = 205,
    UnaryExpr = 206,
    CallExpr = 207,
    CallArg = 208,
    MemberExpr = 209,
    IndexExpr = 210,
    CascadeExpr = 211,
    PipeExpr = 212,
    TryPropagateExpr = 213,
    CatchRecoveryExpr = 214,
    CoalesceExpr = 215,
    LambdaExpr = 216,
    TupleExpr = 217,
    ListExpr = 218,
    MapExpr = 219,
    RangeExpr = 220,
    ComptimeExpr = 221,

    // --- Atomic Literals & Identifiers (300..349) ---
    IdentExpr = 300,
    IntLit = 301,
    FloatLit = 302,
    BoolLit = 303,
    StrLit = 304,
    NoneLit = 305,
    KeywordLit = 306,
    UnitLit = 307,

    // --- Type Specifications (400..449, Invariant #6) ---
    TypeNamed = 400,
    TypeOptional = 401,     // T? (bp = 30)
    TypeIntersection = 402, // A & B (bp = 20)
    TypeUnion = 403,        // A | B (bp = 15)
    TypeFn = 404,           // (A) -> B (bp = 10)
    TypeFallible = 405,     // T ! E (bp = 5)
    TypeSlice = 406,        // SliceBuffer[T]
    TypeList = 407,         // [T]
    TypeMap = 408,          // {K: V}
    TypeTuple = 409,        // (T1, T2)

    // --- Exhaustive Patterns (500..549, M11) ---
    PatWildcard = 500,
    PatBind = 501,
    PatLiteral = 502,
    PatVariant = 503,
    PatRecord = 504,
    PatTuple = 505,
    PatSlice = 506,
    PatOr = 507,
    PatRange = 508,

    // --- Resilient Error Recovery Node (M31.2) ---
    ErrorNode = 65535,
}

/// <summary>
/// 16-bit representation of all lexical tokens in Pilot matching <c>pilot_lexer::TokenKind</c>.
/// </summary>
public enum NativeTokenKind : ushort
{
    Eof = 0,
    Newline = 1,
    VirtualLBrace = 2,
    VirtualRBrace = 3,
    Comment = 4,
    BlockComment = 5,
    Shebang = 6,
    ErrorToken = 7,

    Ident = 20,
    KeywordLit = 21,
    IntLit = 22,
    HexIntLit = 23,
    BinIntLit = 24,
    OctIntLit = 25,
    FloatLit = 26,
    StrLit = 27,
    TripleStrLit = 28,
    FStrStart = 29,
    FStrMiddle = 30,
    FStrEnd = 31,
    DurationLit = 32,
    FrequencyLit = 33,
    MemoryLit = 34,
    RawDslBlock = 35,

    KwLet = 60,
    KwVar = 61,
    KwConst = 62,
    KwDef = 63,
    KwFn = 64,
    KwClass = 65,
    KwStruct = 66,
    KwRecord = 67,
    KwInterface = 68,
    KwProtocol = 69,
    KwEnum = 70,
    KwActor = 71,
    KwType = 72,
    KwRef = 73,
    KwDynamic = 74,
    KwSymbol = 75,
    KwPure = 76,
    KwComptime = 77,
    KwMut = 78,
    KwImmut = 79,
    KwProperty = 80,
    KwGet = 81,
    KwOperator = 82,
    KwExtension = 83,
    KwExtend = 84,
    KwState = 85,
    KwIf = 86,
    KwElif = 87,
    KwElse = 88,
    KwThen = 89,
    KwWhile = 90,
    KwFor = 91,
    KwIn = 92,
    KwMatch = 93,
    KwCase = 94,
    KwWhen = 95,
    KwReturn = 96,
    KwYield = 97,
    KwBreak = 98,
    KwContinue = 99,
    KwPass = 100,
    KwRaise = 101,
    KwTry = 102,
    KwCatch = 103,
    KwFinally = 104,
    KwWith = 105,
    KwAs = 106,
    KwAssert = 107,
    KwImport = 108,
    KwFrom = 109,
    KwGlobal = 110,
    KwLambda = 111,
    KwAnd = 112,
    KwOr = 113,
    KwNot = 114,
    KwIs = 115,
    KwGuard = 116,
    KwDefer = 117,
    KwContext = 118,
    KwRequires = 119,

    KwEnsures = 120,
    KwAsync = 121,
    KwAwait = 122,
    KwSpawn = 123,
    KwParallel = 124,
    KwChan = 125,
    KwChannel = 126,
    KwAtomic = 127,
    KwSync = 128,
    KwLock = 129,
    KwIsolate = 130,
    KwRealtime = 131,
    KwDeadline = 132,
    KwOnMissed = 133,
    KwWithin = 134,
    KwFallback = 135,
    KwPeriod = 136,
    KwPeriodic = 137,
    KwPriority = 138,
    KwJitter = 139,
    KwBounded = 140,
    KwNoalloc = 141,
    KwStackalloc = 142,
    KwRegion = 143,
    KwWcet = 144,
    KwHardRt = 145,
    KwSoftRt = 146,
    KwPublic = 147,
    KwPrivate = 148,
    KwProtected = 149,
    KwInternal = 150,
    KwStatic = 151,
    KwPartial = 152,
    KwSteal = 153,
    KwContracts = 154,
    KwDelegate = 155,
    KwParams = 156,
    KwArgs = 157,
    KwKwargs = 158,
    KwNameof = 159,
    KwEach = 160,
    KwRepeat = 161,
    KwAdapter = 162,
    KwConverter = 163,
    KwDefault = 164,
    KwTimeout = 165,
    KwSelect = 166,
    KwWhere = 167,
    KwJoin = 168,
    KwInner = 169,
    KwLeft = 170,
    KwRight = 171,
    KwFull = 172,
    KwCross = 173,
    KwOuter = 174,
    KwOn = 175,
    KwGroup = 176,
    KwBy = 177,
    KwHaving = 178,
    KwOrder = 179,
    KwAsc = 180,
    KwDesc = 181,
    KwAscending = 182,
    KwDescending = 183,
    KwLimit = 184,
    KwOffset = 185,
    KwInto = 186,
    KwDistinct = 187,
    KwUnion = 188,
    KwAll = 189,
    KwIntersect = 190,
    KwExcept = 191,
    KwDifference = 192,
    KwIndex = 193,
    KwIndexed = 194,
    KwLookup = 195,
    KwTable = 196,
    KwSchema = 197,
    KwInsert = 198,
    KwUpdate = 199,

    KwDelete = 200,
    KwValues = 201,
    KwSet = 202,
    KwReturning = 203,
    KwConflict = 204,
    KwDo = 205,
    KwNothing = 206,
    KwOver = 207,
    KwPartition = 208,
    KwWindow = 209,
    KwTransaction = 210,
    KwCommit = 211,
    KwRollback = 212,
    KwSavepoint = 213,
    KwTo = 214,
    KwConnect = 215,
    KwDatabase = 216,
    KwModel = 217,
    KwDsl = 218,
    KwDomain = 219,
    KwUnit = 220,
    KwMacro = 221,
    KwDefmacro = 222,
    KwDefkeyword = 223,
    KwQuote = 224,
    KwQuasiquote = 225,
    KwSplice = 226,
    KwUnquote = 227,
    KwUnquoteSplicing = 228,
    KwExtern = 229,
    KwModule = 230,
    KwLib = 231,
    KwTrue = 232,
    KwFalse = 233,
    KwSome = 234,
    KwNone = 235,
    KwNull = 236,
    KwUndefined = 237,

    LParen = 240,
    RParen = 241,
    LBracket = 242,
    RBracket = 243,
    LBrace = 244,
    RBrace = 245,
    Colon = 246,
    Semicolon = 247,
    Comma = 248,
    Dot = 249,
    At = 250,
    Question = 251,
    Exclamation = 252,
    Tilde = 253,
    Dollar = 254,
    Backquote = 255,
    Underscore = 256,
    Walrus = 257,
    ArrowLeft = 258,

    Plus = 270,
    Minus = 271,
    Star = 272,
    Slash = 273,
    Percent = 274,
    DoubleStar = 275,
    DoubleSlash = 276,
    Equal = 277,
    EqEqual = 278,
    NotEqual = 279,
    Less = 280,
    LessEqual = 281,
    Greater = 282,
    GreaterEqual = 283,
    Amper = 284,
    Vbar = 285,
    Caret = 286,
    ShiftLeft = 287,
    ShiftRight = 288,
    PlusEqual = 289,
    MinusEqual = 290,
    StarEqual = 291,
    SlashEqual = 292,
    PercentEqual = 293,
    DoubleStarEqual = 294,
    DoubleSlashEqual = 295,
    AmperEqual = 296,
    VbarEqual = 297,
    CaretEqual = 298,
    ShiftLeftEqual = 299,
    ShiftRightEqual = 300,
    CoalesceEqual = 301,
    Coalesce = 302,
    SafeNav = 303,
    SafeCascade = 304,
    PipeForward = 305,
    PipeBackward = 306,
    Arrow = 307,
    FatArrow = 308,
    JsonGetText = 309,
    JsonContains = 310,
    JsonContained = 311,
    DotDot = 312,
    DotDotEqual = 313,
    DotDotLess = 314,
    Ellipsis = 315,

    CallQuoteShallow = 320,
    CallQuoteDeep = 321,
    CallQuoteAbi = 322,
    CallQuoteReact = 323,
    CallQuoteAdjoint = 324,
    CallQuoteEffect = 325,
}

/// <summary>
/// Bitflags associated with each emitted token matching <c>pilot_lexer::TokenFlags</c>.
/// </summary>
[Flags]
public enum NativeTokenFlags : ushort
{
    None = 0,
    IsKeyword = 1 << 0,
    IsContextualIdent = 1 << 1,
    IsVirtual = 1 << 2,
    IsDelimiterOpen = 1 << 3,
    IsDelimiterClose = 1 << 4,
    PrecedingWhitespace = 1 << 5,
    TrailingWhitespace = 1 << 6,
    IsDimensional = 1 << 7,
}

/// <summary>
/// Milestone 17 &amp; 18 Unified Call-Quote Projection Kind (Invariant #1) matching <c>pilot_ast::CallQuoteKind</c>.
/// </summary>
public enum NativeCallQuoteKind : byte
{
    None = 0,            // fn(a, b)      -> TReturn
    ShallowQuote = 1,    // fn!(a, b)     -> CallExpr<TReturn>
    DeepSplice = 2,      // fn!!(a, b)    -> AstExpr<TReturn> (supports `~arg` quasiquoting)
    AbiFrame = 3,        // fn!&(a, b)    -> CallFrame_<FnSig>
    ReactiveSignal = 4,  // fn!^(a, b)    -> Signal<TReturn>
    SymbolicAdjoint = 5, // fn!@(a, b)    -> Dual<TReturn, TGradTuple>
    EffectDryRun = 6,    // fn!?(a, b)    -> EffectPlan<TReturn>
}

/// <summary>
/// Milestone 28.3 &amp; 30.1 Pratt Expression Binding-Power Hierarchy (Invariant #8) matching <c>pilot_ast::ExprPrecedence</c>.
/// </summary>
public enum NativeExprPrecedence : byte
{
    Lowest = 0,
    Cascade = 1,        // .. and ?.. on outer receiver (binds looser than =, eliminating post-parse hoisting!)
    Assign = 2,         // =, +=, -=, *=, /=, ??=, := (Right-assoc)
    LambdaTernary = 4,  // =>, expr if cond else alt (Right-assoc)
    CatchRecovery = 6,  // expr catch default / expr catch err => ... (M13)
    Pipe = 8,           // |> (Left), <| (Right), fluent |> .Method()
    Coalesce = 10,      // ?? (Right-assoc)
    LogicalOr = 12,     // or, ||
    LogicalAnd = 14,    // and, &&
    LogicalNot = 16,    // not (Prefix)
    Comparison = 18,    // ==, !=, <, <=, >, >=, is, in, not in (Chained)
    ArrowDto = 20,      // -> DTO projection, ->> JSON extract
    Range = 22,         // .., ..=, ..< (M15.1 Invariant #3)
    BitOr = 24,         // |
    BitXor = 26,        // ^
    BitAnd = 28,        // &
    Shift = 30,         // <<, >>
    AddSub = 32,        // +, -
    MulDivRem = 34,     // *, /, //, %
    UnaryPrefix = 36,   // -, +, ~, !, await, some, ref, *
    Power = 38,         // ** (Right-assoc)
    PostfixLed = 40,    // .prop, ?.prop, [idx], (args), !/!!/!&/!^/!@/!?, postfix ?
}

/// <summary>
/// Milestone 28.3 &amp; Invariant #6 Pratt Type-Spec Binding-Power Hierarchy (<c>T? &gt; &amp; &gt; | &gt; -&gt; &gt; !</c>).
/// </summary>
public enum NativeTypePrecedence : byte
{
    Lowest = 0,
    Fallible = 5,         // T ! E (binds loosest so `User? ! IoError | ParseError` == `(User?) ! (IoError | ParseError)`)
    FnArrow = 10,         // (A, B) -> R
    Union = 15,           // A | B
    Intersection = 20,    // A & B
    PostfixOptional = 30, // T? (binds tightest)
}

/// <summary>
/// Binary operator codes packed into <see cref="NativeAstNode.Flags"/> matching <c>pilot_ast::BinaryOp</c>.
/// </summary>
public enum NativeBinaryOp : ushort
{
    Add = 1,
    Sub = 2,
    Mul = 3,
    Div = 4,
    FloorDiv = 5,
    Rem = 6,
    Pow = 7,
    Eq = 8,
    Ne = 9,
    Lt = 10,
    Le = 11,
    Gt = 12,
    Ge = 13,
    And = 14,
    Or = 15,
    BitAnd = 16,
    BitOr = 17,
    BitXor = 18,
    Shl = 19,
    Shr = 20,
    Pipe = 21,
    Coalesce = 22,
    RangeInclusive = 23,
    RangeHalfOpen = 24,
}

/// <summary>
/// Hardware-packed <b>16-byte</b> C-ABI AST node (<c>4</c> nodes per 64-byte L1 cache line)
/// matching <c>#[repr(C, align(8))] pub struct AstNode</c> in <c>pilot-ast</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential, Pack = 8, Size = 16)]
public readonly struct NativeAstNode : IEquatable<NativeAstNode>
{
    public const uint NoneId = uint.MaxValue;

    public readonly ushort Kind;
    public readonly ushort Flags;
    public readonly uint SpanStart;
    public readonly uint Lhs;
    public readonly uint Rhs;

    public NativeAstNode(NativeNodeKind kind, ushort flags, uint spanStart, uint lhs, uint rhs)
    {
        Kind = (ushort)kind;
        Flags = flags;
        SpanStart = spanStart;
        Lhs = lhs;
        Rhs = rhs;
    }

    public NativeNodeKind NodeKind => (NativeNodeKind)Kind;

    public bool Equals(NativeAstNode other) =>
        Kind == other.Kind &&
        Flags == other.Flags &&
        SpanStart == other.SpanStart &&
        Lhs == other.Lhs &&
        Rhs == other.Rhs;

    public override bool Equals(object? obj) => obj is NativeAstNode other && Equals(other);

    public override int GetHashCode() => HashCode.Combine(Kind, Flags, SpanStart, Lhs, Rhs);

    public override string ToString() => $"{NodeKind}(flags={Flags}, span={SpanStart}, lhs={Lhs}, rhs={Rhs})";
}

/// <summary>
/// Compact <b>8-byte</b> half-open UTF-8 byte span <c>[Start, End)</c> matching <c>pilot_span::ByteSpan</c>.
/// </summary>
[StructLayout(LayoutKind.Sequential, Pack = 4, Size = 8)]
public readonly struct NativeByteSpan : IEquatable<NativeByteSpan>
{
    public readonly uint Start;
    public readonly uint End;

    public NativeByteSpan(uint start, uint end)
    {
        Start = start;
        End = end;
    }

    public uint Length => End >= Start ? End - Start : 0u;

    public bool Equals(NativeByteSpan other) => Start == other.Start && End == other.End;

    public override bool Equals(object? obj) => obj is NativeByteSpan other && Equals(other);

    public override int GetHashCode() => HashCode.Combine(Start, End);

    public override string ToString() => $"[{Start}..{End})";
}

/// <summary>
/// Milestone 34.1: 16-byte unboxed universal Pilot value representation.
/// </summary>
[StructLayout(LayoutKind.Sequential, Pack = 8, Size = 16)]
public readonly struct NativePilotVal : IEquatable<NativePilotVal>
{
    public readonly ulong TagMeta;
    public readonly ulong Payload;

    public NativePilotVal(ulong tagMeta, ulong payload)
    {
        TagMeta = tagMeta;
        Payload = payload;
    }

    public byte Tag => (byte)(TagMeta & 0xFF);
    public bool IsNull => Tag == 0;
    public bool IsInt => Tag == 3;
    public bool IsFloat => Tag == 5;
    public bool IsSsoStr => Tag == 8;

    public long? AsInt => NativeAstBridge.PilotValAsInt(this);
    public double? AsFloat => NativeAstBridge.PilotValAsFloat(this);

    public override string ToString() => NativeAstBridge.PilotValToString(this);

    public bool Equals(NativePilotVal other) => TagMeta == other.TagMeta && Payload == other.Payload;
    public override bool Equals(object? obj) => obj is NativePilotVal other && Equals(other);
    public override int GetHashCode() => HashCode.Combine(TagMeta, Payload);
}

/// <summary>
/// Zero-copy disposable view over a native Rust <c>PilotFfiArenaHandle*</c> returned by <c>pilot-ffi</c>.
/// Exposes <see cref="Nodes"/>, <see cref="Spans"/>, and <see cref="ExtraChildren"/> directly as
/// <see cref="ReadOnlySpan{T}"/> slices over native memory with zero per-node marshalling.
/// </summary>
public sealed unsafe class NativeAstArenaView : IDisposable
{
    private IntPtr _handle;
    private readonly NativeAstNode* _nodesPtr;
    private readonly int _nodeCount;
    private readonly NativeByteSpan* _spansPtr;
    private readonly int _spanCount;
    private readonly uint* _extraChildrenPtr;
    private readonly int _extraChildrenCount;

    public uint RootId { get; }
    public uint LineCount { get; }
    public IntPtr Handle => _handle;

    internal NativeAstArenaView(
        IntPtr handle,
        NativeAstNode* nodesPtr,
        int nodeCount,
        NativeByteSpan* spansPtr,
        int spanCount,
        uint* extraChildrenPtr,
        int extraChildrenCount,
        uint rootId,
        uint lineCount)
    {
        _handle = handle;
        _nodesPtr = nodesPtr;
        _nodeCount = nodeCount;
        _spansPtr = spansPtr;
        _spanCount = spanCount;
        _extraChildrenPtr = extraChildrenPtr;
        _extraChildrenCount = extraChildrenCount;
        RootId = rootId;
        LineCount = lineCount;
    }

    public int NodeCount => _nodeCount;
    public int SpanCount => _spanCount;
    public int ExtraChildrenCount => _extraChildrenCount;

    public ReadOnlySpan<NativeAstNode> Nodes
    {
        get
        {
            ObjectDisposedException.ThrowIf(_handle == IntPtr.Zero, this);
            return _nodesPtr == null || _nodeCount <= 0
                ? ReadOnlySpan<NativeAstNode>.Empty
                : new ReadOnlySpan<NativeAstNode>(_nodesPtr, _nodeCount);
        }
    }

    public ReadOnlySpan<NativeByteSpan> Spans
    {
        get
        {
            ObjectDisposedException.ThrowIf(_handle == IntPtr.Zero, this);
            return _spansPtr == null || _spanCount <= 0
                ? ReadOnlySpan<NativeByteSpan>.Empty
                : new ReadOnlySpan<NativeByteSpan>(_spansPtr, _spanCount);
        }
    }

    public ReadOnlySpan<uint> ExtraChildren
    {
        get
        {
            ObjectDisposedException.ThrowIf(_handle == IntPtr.Zero, this);
            return _extraChildrenPtr == null || _extraChildrenCount <= 0
                ? ReadOnlySpan<uint>.Empty
                : new ReadOnlySpan<uint>(_extraChildrenPtr, _extraChildrenCount);
        }
    }

    public ReadOnlySpan<uint> GetChildrenSlice(uint start, uint len)
    {
        var all = ExtraChildren;
        if (len == 0 || start >= (uint)all.Length)
        {
            return ReadOnlySpan<uint>.Empty;
        }
        int s = (int)start;
        int l = (int)Math.Min(len, (uint)(all.Length - s));
        return all.Slice(s, l);
    }

    public (uint Line, uint Col) LookupLineCol(uint byteOffset)
    {
        ObjectDisposedException.ThrowIf(_handle == IntPtr.Zero, this);
        return NativeAstBridge.LookupLineCol(_handle, byteOffset);
    }

    public void Dispose()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreeArena(h);
        }
        GC.SuppressFinalize(this);
    }

    ~NativeAstArenaView()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreeArena(h);
        }
    }
}

/// <summary>
/// Zero-copy disposable view over a native Rust <c>PilotFfiTokenTapeHandle*</c> returned by <c>pilot-ffi</c>.
/// Exposes <see cref="Kinds"/>, <see cref="Flags"/>, <see cref="Spans"/>, and <see cref="BracketLinks"/> directly as
/// <see cref="ReadOnlySpan{T}"/> slices over native memory with zero per-node marshalling.
/// </summary>
public sealed unsafe class NativeTokenTapeView : IDisposable
{
    private IntPtr _handle;
    private readonly NativeTokenKind* _kindsPtr;
    private readonly ushort* _flagsPtr;
    private readonly NativeByteSpan* _spansPtr;
    private readonly uint* _bracketLinksPtr;
    private readonly int _len;

    internal NativeTokenTapeView(
        IntPtr handle,
        NativeTokenKind* kindsPtr,
        ushort* flagsPtr,
        NativeByteSpan* spansPtr,
        uint* bracketLinksPtr,
        int len)
    {
        _handle = handle;
        _kindsPtr = kindsPtr;
        _flagsPtr = flagsPtr;
        _spansPtr = spansPtr;
        _bracketLinksPtr = bracketLinksPtr;
        _len = len;
    }

    public int Length => _len;

    public ReadOnlySpan<NativeTokenKind> Kinds
    {
        get
        {
            ObjectDisposedException.ThrowIf(_handle == IntPtr.Zero, this);
            return _kindsPtr == null || _len <= 0 ? ReadOnlySpan<NativeTokenKind>.Empty : new ReadOnlySpan<NativeTokenKind>(_kindsPtr, _len);
        }
    }

    public ReadOnlySpan<ushort> Flags
    {
        get
        {
            ObjectDisposedException.ThrowIf(_handle == IntPtr.Zero, this);
            return _flagsPtr == null || _len <= 0 ? ReadOnlySpan<ushort>.Empty : new ReadOnlySpan<ushort>(_flagsPtr, _len);
        }
    }

    public ReadOnlySpan<NativeByteSpan> Spans
    {
        get
        {
            ObjectDisposedException.ThrowIf(_handle == IntPtr.Zero, this);
            return _spansPtr == null || _len <= 0 ? ReadOnlySpan<NativeByteSpan>.Empty : new ReadOnlySpan<NativeByteSpan>(_spansPtr, _len);
        }
    }

    public ReadOnlySpan<uint> BracketLinks
    {
        get
        {
            ObjectDisposedException.ThrowIf(_handle == IntPtr.Zero, this);
            return _bracketLinksPtr == null || _len <= 0 ? ReadOnlySpan<uint>.Empty : new ReadOnlySpan<uint>(_bracketLinksPtr, _len);
        }
    }

    public uint? GetMatchingBracket(int index)
    {
        var links = BracketLinks;
        if (index >= 0 && index < links.Length && links[index] != uint.MaxValue)
        {
            return links[index];
        }
        return null;
    }

    public void Dispose()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreeTokenTape(h);
        }
        GC.SuppressFinalize(this);
    }

    ~NativeTokenTapeView()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreeTokenTape(h);
        }
    }
}

/// <summary>
/// Milestone 31.1: Managed wrapper around a native lossless Red-Green Concrete Syntax Tree (CST).
/// </summary>
public sealed class NativeCstView : IDisposable
{
    private IntPtr _handle;

    internal NativeCstView(IntPtr handle)
    {
        _handle = handle;
    }

    public IntPtr Handle => _handle;
    public nuint DiagnosticCount => NativeAstBridge.GetCstDiagnosticCount(_handle);
    public bool HasMissingIdent => NativeAstBridge.GetCstHasMissingIdent(_handle);
    public bool HasMissingRParen => NativeAstBridge.GetCstHasMissingRParen(_handle);

    public string ToText()
    {
        return NativeAstBridge.GetCstText(_handle);
    }

    public void Dispose()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreeCst(h);
        }
        GC.SuppressFinalize(this);
    }

    ~NativeCstView()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreeCst(h);
        }
    }
}

/// <summary>
/// Milestone 32.2: Managed wrapper around the demand-driven incremental database with early cutoff.
/// </summary>
public sealed class NativeDatabaseHandle : IDisposable
{
    private IntPtr _handle;

    internal NativeDatabaseHandle(IntPtr handle)
    {
        _handle = handle;
    }

    public IntPtr Handle => _handle;

    public void SetSource(uint fileId, string source)
    {
        ObjectDisposedException.ThrowIf(_handle == IntPtr.Zero, this);
        NativeAstBridge.DbSetSource(_handle, fileId, source);
    }

    public void ApplyEdit(uint fileId, uint start, uint end, string replacement)
    {
        ObjectDisposedException.ThrowIf(_handle == IntPtr.Zero, this);
        NativeAstBridge.DbApplyEdit(_handle, fileId, start, end, replacement);
    }

    public nuint EarlyCutoffCount
    {
        get
        {
            ObjectDisposedException.ThrowIf(_handle == IntPtr.Zero, this);
            return NativeAstBridge.DbEarlyCutoffCount(_handle);
        }
    }

    public ulong GetSignatureHash(uint fileId)
    {
        ObjectDisposedException.ThrowIf(_handle == IntPtr.Zero, this);
        return NativeAstBridge.DbSignatureHash(_handle, fileId);
    }

    public void Dispose()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreeDb(h);
        }
        GC.SuppressFinalize(this);
    }

    ~NativeDatabaseHandle()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreeDb(h);
        }
    }
}

/// <summary>
/// Milestone 32.3: Managed wrapper around the native zero-GC language server.
/// </summary>
public sealed class NativeLspServerHandle : IDisposable
{
    private IntPtr _handle;

    internal NativeLspServerHandle(IntPtr handle)
    {
        _handle = handle;
    }

    public IntPtr Handle => _handle;

    public string? SendMessage(string jsonMsg)
    {
        ObjectDisposedException.ThrowIf(_handle == IntPtr.Zero, this);
        return NativeAstBridge.LspHandleMessage(_handle, jsonMsg);
    }

    public void Dispose()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreeLsp(h);
        }
        GC.SuppressFinalize(this);
    }

    ~NativeLspServerHandle()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreeLsp(h);
        }
    }
}

/// <summary>
/// Milestone 33.1–33.3: Managed wrapper around static analysis results.
/// </summary>
public sealed class NativeAnalysisHandle : IDisposable
{
    private IntPtr _handle;

    internal NativeAnalysisHandle(IntPtr handle)
    {
        _handle = handle;
    }

    public IntPtr Handle => _handle;
    public nuint DiagnosticCount => NativeAstBridge.AnalysisDiagnosticCount(_handle);
    public bool HasErrors => NativeAstBridge.AnalysisHasErrors(_handle);

    public void Dispose()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreeAnalysis(h);
        }
        GC.SuppressFinalize(this);
    }

    ~NativeAnalysisHandle()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreeAnalysis(h);
        }
    }
}

/// <summary>
/// Milestone 33.4: Managed wrapper around Perceus optimizer results.
/// </summary>
public sealed class NativePerceusHandle : IDisposable
{
    private IntPtr _handle;

    internal NativePerceusHandle(IntPtr handle)
    {
        _handle = handle;
    }

    public IntPtr Handle => _handle;
    public nuint RegionAllocCount => NativeAstBridge.PerceusRegionAllocCount(_handle);
    public nuint StackAllocCount => NativeAstBridge.PerceusStackAllocCount(_handle);
    public nuint GlobalAllocCount => NativeAstBridge.PerceusGlobalAllocCount(_handle);
    public nuint DropReuseCount => NativeAstBridge.PerceusDropReuseCount(_handle);
    public nuint ElidedRetainCount => NativeAstBridge.PerceusElidedRetainCount(_handle);

    public void Dispose()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreePerceus(h);
        }
        GC.SuppressFinalize(this);
    }

    ~NativePerceusHandle()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreePerceus(h);
        }
    }
}

/// <summary>
/// Milestone 34.3: Managed wrapper around the register-based PilVirtualMachine.
/// </summary>
public sealed class NativeVmHandle : IDisposable
{
    private IntPtr _handle;

    internal NativeVmHandle(IntPtr handle)
    {
        _handle = handle;
    }

    public IntPtr Handle => _handle;

    public NativePilotVal ExecuteBytes(ReadOnlySpan<byte> bytes)
    {
        ObjectDisposedException.ThrowIf(_handle == IntPtr.Zero, this);
        return NativeAstBridge.VmExecuteBytes(_handle, bytes);
    }

    public void Dispose()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreeVm(h);
        }
        GC.SuppressFinalize(this);
    }

    ~NativeVmHandle()
    {
        var h = _handle;
        if (h != IntPtr.Zero)
        {
            _handle = IntPtr.Zero;
            NativeAstBridge.FreeVm(h);
        }
    }
}

/// <summary>
/// Milestone 28.4 &amp; 29.5: Zero-copy C-ABI strangler-fig bridge between C# and the Rust <c>pilot-ffi</c> crate.
/// </summary>
public static unsafe class NativeAstBridge
{
    public const uint ExpectedAbiVersion = 3401;

    private static readonly object SyncLock = new();
    private static IntPtr _libraryHandle;
    private static delegate* unmanaged[Cdecl]<uint> _fnAbiVersion;
    private static delegate* unmanaged[Cdecl]<uint> _fnSizeOfAstNode;
    private static delegate* unmanaged[Cdecl]<byte*, nuint, IntPtr> _fnParseUtf8ToArena;
    private static delegate* unmanaged[Cdecl]<IntPtr, nuint*, NativeAstNode*> _fnArenaNodesPtr;
    private static delegate* unmanaged[Cdecl]<IntPtr, nuint*, NativeByteSpan*> _fnArenaSpansPtr;
    private static delegate* unmanaged[Cdecl]<IntPtr, nuint*, uint*> _fnArenaExtraChildrenPtr;
    private static delegate* unmanaged[Cdecl]<IntPtr, uint> _fnArenaRootId;
    private static delegate* unmanaged[Cdecl]<IntPtr, uint> _fnArenaLineCount;
    private static delegate* unmanaged[Cdecl]<IntPtr, uint, uint*, uint*, void> _fnArenaLookupLineCol;
    private static delegate* unmanaged[Cdecl]<IntPtr, void> _fnArenaFree;

    private static delegate* unmanaged[Cdecl]<byte*, nuint, IntPtr> _fnTokenizeUtf8;
    private static delegate* unmanaged[Cdecl]<IntPtr, uint> _fnTokenTapeLen;
    private static delegate* unmanaged[Cdecl]<IntPtr, NativeTokenKind*> _fnTokenTapeKindsPtr;
    private static delegate* unmanaged[Cdecl]<IntPtr, ushort*> _fnTokenTapeFlagsPtr;
    private static delegate* unmanaged[Cdecl]<IntPtr, NativeByteSpan*> _fnTokenTapeSpansPtr;
    private static delegate* unmanaged[Cdecl]<IntPtr, uint*> _fnTokenTapeBracketLinksPtr;
    private static delegate* unmanaged[Cdecl]<IntPtr, void> _fnTokenTapeFree;

    private static delegate* unmanaged[Cdecl]<byte*, nuint, IntPtr> _fnCstParse;
    private static delegate* unmanaged[Cdecl]<IntPtr, void> _fnCstFree;
    private static delegate* unmanaged[Cdecl]<IntPtr, nuint> _fnCstDiagnosticCount;
    private static delegate* unmanaged[Cdecl]<IntPtr, byte> _fnCstHasMissingIdent;
    private static delegate* unmanaged[Cdecl]<IntPtr, byte> _fnCstHasMissingRParen;
    private static delegate* unmanaged[Cdecl]<IntPtr, nuint*, byte*> _fnCstToText;
    private static delegate* unmanaged[Cdecl]<byte*, nuint, int, byte, nuint*, byte*, byte*> _fnFormatSource;
    private static delegate* unmanaged[Cdecl]<byte*, nuint, void> _fnFreeString;

    private static delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, uint, uint, byte*, nuint, ulong*, nuint*, IntPtr> _fnIncrementalReparse;
    private static delegate* unmanaged[Cdecl]<IntPtr> _fnDbNew;
    private static delegate* unmanaged[Cdecl]<IntPtr, void> _fnDbFree;
    private static delegate* unmanaged[Cdecl]<IntPtr, uint, byte*, nuint, void> _fnDbSetSource;
    private static delegate* unmanaged[Cdecl]<IntPtr, uint, uint, uint, byte*, nuint, void> _fnDbApplyEdit;
    private static delegate* unmanaged[Cdecl]<IntPtr, nuint> _fnDbEarlyCutoffCount;
    private static delegate* unmanaged[Cdecl]<IntPtr, uint, ulong> _fnDbSignatureHash;

    private static delegate* unmanaged[Cdecl]<IntPtr> _fnLspNew;
    private static delegate* unmanaged[Cdecl]<IntPtr, void> _fnLspFree;
    private static delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, nuint*, byte*> _fnLspHandleMessage;

    private static delegate* unmanaged[Cdecl]<IntPtr, IntPtr> _fnAnalysisRun;
    private static delegate* unmanaged[Cdecl]<IntPtr, void> _fnAnalysisFree;
    private static delegate* unmanaged[Cdecl]<IntPtr, nuint> _fnAnalysisDiagnosticCount;
    private static delegate* unmanaged[Cdecl]<IntPtr, byte> _fnAnalysisHasErrors;

    private static delegate* unmanaged[Cdecl]<IntPtr, IntPtr> _fnPerceusRun;
    private static delegate* unmanaged[Cdecl]<IntPtr, void> _fnPerceusFree;
    private static delegate* unmanaged[Cdecl]<IntPtr, nuint> _fnPerceusRegionAllocCount;
    private static delegate* unmanaged[Cdecl]<IntPtr, nuint> _fnPerceusStackAllocCount;
    private static delegate* unmanaged[Cdecl]<IntPtr, nuint> _fnPerceusGlobalAllocCount;
    private static delegate* unmanaged[Cdecl]<IntPtr, nuint> _fnPerceusDropReuseCount;
    private static delegate* unmanaged[Cdecl]<IntPtr, nuint> _fnPerceusElidedRetainCount;

    private static delegate* unmanaged[Cdecl]<long, NativePilotVal> _fnValFromInt;
    private static delegate* unmanaged[Cdecl]<double, NativePilotVal> _fnValFromFloat;
    private static delegate* unmanaged[Cdecl]<byte, NativePilotVal> _fnValFromBool;
    private static delegate* unmanaged[Cdecl]<byte*, nuint, NativePilotVal> _fnValFromStr;
    private static delegate* unmanaged[Cdecl]<NativePilotVal, nuint*, byte*> _fnValToStr;
    private static delegate* unmanaged[Cdecl]<NativePilotVal, byte> _fnValIsSso;
    private static delegate* unmanaged[Cdecl]<NativePilotVal, long*, byte> _fnValAsInt;
    private static delegate* unmanaged[Cdecl]<NativePilotVal, double*, byte> _fnValAsFloat;
    private static delegate* unmanaged[Cdecl]<NativePilotVal, NativePilotVal, NativePilotVal> _fnValAdd;

    private static delegate* unmanaged[Cdecl]<IntPtr> _fnVmNew;
    private static delegate* unmanaged[Cdecl]<IntPtr, void> _fnVmFree;
    private static delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, NativePilotVal> _fnVmExecuteBytes;

    static NativeAstBridge()
    {
        // Compile-time / class-load hardware size check (Invariant #9)
        if (Unsafe.SizeOf<NativeAstNode>() != 16)
        {
            throw new InvalidOperationException($"NativeAstNode must be strictly 16 bytes, got {Unsafe.SizeOf<NativeAstNode>()}.");
        }
        if (Unsafe.SizeOf<NativeByteSpan>() != 8)
        {
            throw new InvalidOperationException($"NativeByteSpan must be strictly 8 bytes, got {Unsafe.SizeOf<NativeByteSpan>()}.");
        }
        if (Unsafe.SizeOf<NativePilotVal>() != 16)
        {
            throw new InvalidOperationException($"NativePilotVal must be strictly 16 bytes, got {Unsafe.SizeOf<NativePilotVal>()}.");
        }
    }

    public static uint GetAbiVersion()
    {
        EnsureLoaded();
        return _fnAbiVersion();
    }

    public static uint GetNativeSizeOfAstNode()
    {
        EnsureLoaded();
        return _fnSizeOfAstNode();
    }

    /// <summary>
    /// Parses a UTF-8 string into a native Rust <c>AstArena</c> and returns a zero-copy
    /// <see cref="NativeAstArenaView"/> over its contiguous 16-byte <see cref="NativeAstNode"/> buffer.
    /// </summary>
    public static NativeAstArenaView ParseUtf8ToArena(string source)
    {
        ArgumentNullException.ThrowIfNull(source);
        byte[] utf8 = Encoding.UTF8.GetBytes(source);
        return ParseUtf8ToArena(utf8);
    }

    /// <summary>
    /// Parses a raw UTF-8 byte span into a native Rust <c>AstArena</c> and returns a zero-copy
    /// <see cref="NativeAstArenaView"/> over its contiguous 16-byte <see cref="NativeAstNode"/> buffer.
    /// </summary>
    public static NativeAstArenaView ParseUtf8ToArena(ReadOnlySpan<byte> utf8Bytes)
    {
        EnsureLoaded();
        IntPtr handle;
        fixed (byte* ptr = utf8Bytes)
        {
            handle = _fnParseUtf8ToArena(ptr, (nuint)utf8Bytes.Length);
        }

        if (handle == IntPtr.Zero)
        {
            throw new InvalidOperationException("pilot_parse_utf8_to_arena returned a null arena handle.");
        }

        nuint nodeLen = 0;
        var nodesPtr = _fnArenaNodesPtr(handle, &nodeLen);

        nuint spanLen = 0;
        var spansPtr = _fnArenaSpansPtr(handle, &spanLen);

        nuint extraLen = 0;
        var extraPtr = _fnArenaExtraChildrenPtr(handle, &extraLen);

        uint rootId = _fnArenaRootId(handle);
        uint lineCount = _fnArenaLineCount(handle);

        return new NativeAstArenaView(
            handle,
            nodesPtr,
            (int)nodeLen,
            spansPtr,
            (int)spanLen,
            extraPtr,
            (int)extraLen,
            rootId,
            lineCount);
    }

    /// <summary>
    /// Tokenizes a UTF-8 string into a native Rust <c>TokenTape</c> and returns a zero-copy
    /// <see cref="NativeTokenTapeView"/> over its contiguous token kind, flag, span, and bracket links buffers.
    /// </summary>
    public static NativeTokenTapeView TokenizeUtf8(string source)
    {
        ArgumentNullException.ThrowIfNull(source);
        byte[] utf8 = Encoding.UTF8.GetBytes(source);
        return TokenizeUtf8(utf8);
    }

    /// <summary>
    /// Tokenizes a raw UTF-8 byte span into a native Rust <c>TokenTape</c> and returns a zero-copy
    /// <see cref="NativeTokenTapeView"/> over its contiguous token kind, flag, span, and bracket links buffers.
    /// </summary>
    public static NativeTokenTapeView TokenizeUtf8(ReadOnlySpan<byte> utf8Bytes)
    {
        EnsureLoaded();
        IntPtr handle;
        fixed (byte* ptr = utf8Bytes)
        {
            handle = _fnTokenizeUtf8(ptr, (nuint)utf8Bytes.Length);
        }

        if (handle == IntPtr.Zero)
        {
            throw new InvalidOperationException("pilot_tokenize_utf8 returned a null token tape handle.");
        }

        uint len = _fnTokenTapeLen(handle);
        var kindsPtr = _fnTokenTapeKindsPtr(handle);
        var flagsPtr = _fnTokenTapeFlagsPtr(handle);
        var spansPtr = _fnTokenTapeSpansPtr(handle);
        var linksPtr = _fnTokenTapeBracketLinksPtr(handle);

        return new NativeTokenTapeView(
            handle,
            kindsPtr,
            flagsPtr,
            spansPtr,
            linksPtr,
            (int)len);
    }

    internal static (uint Line, uint Col) LookupLineCol(IntPtr handle, uint byteOffset)
    {
        EnsureLoaded();
        uint line = 0;
        uint col = 0;
        _fnArenaLookupLineCol(handle, byteOffset, &line, &col);
        return (line, col);
    }

    internal static void FreeArena(IntPtr handle)
    {
        if (handle != IntPtr.Zero && _fnArenaFree != null)
        {
            _fnArenaFree(handle);
        }
    }

    internal static void FreeTokenTape(IntPtr handle)
    {
        if (handle != IntPtr.Zero && _fnTokenTapeFree != null)
        {
            _fnTokenTapeFree(handle);
        }
    }

    /// <summary>
    /// Milestone 31.1: Parses UTF-8 Pilot source code into a lossless Red-Green Concrete Syntax Tree (CST).
    /// </summary>
    public static NativeCstView ParseUtf8ToCst(string source)
    {
        ArgumentNullException.ThrowIfNull(source);
        byte[] utf8 = Encoding.UTF8.GetBytes(source);
        EnsureLoaded();

        IntPtr handle;
        fixed (byte* ptr = utf8)
        {
            handle = _fnCstParse(ptr, (nuint)utf8.Length);
        }

        if (handle == IntPtr.Zero)
        {
            throw new InvalidOperationException("pilot_cst_parse returned a null handle.");
        }

        return new NativeCstView(handle);
    }

    /// <summary>
    /// Milestone 31.3: Formats Pilot source code using native Wadler-Lindig CST pretty-printer.
    /// </summary>
    public static (string FormattedCode, bool WasModified) FormatSourceNative(
        string source,
        bool? useColonSuites = null,
        bool emitSemicolons = true)
    {
        ArgumentNullException.ThrowIfNull(source);
        byte[] utf8 = Encoding.UTF8.GetBytes(source);
        EnsureLoaded();

        int colonMode = useColonSuites switch
        {
            true => 1,
            false => 0,
            null => -1,
        };

        nuint outLen = 0;
        byte wasModifiedByte = 0;
        byte* resultPtr;

        fixed (byte* ptr = utf8)
        {
            resultPtr = _fnFormatSource(ptr, (nuint)utf8.Length, colonMode, emitSemicolons ? (byte)1 : (byte)0, &outLen, &wasModifiedByte);
        }

        if (resultPtr == null || outLen == 0)
        {
            return (string.Empty, false);
        }

        try
        {
            string formatted = Encoding.UTF8.GetString(resultPtr, (int)outLen);
            return (formatted, wasModifiedByte != 0);
        }
        finally
        {
            _fnFreeString(resultPtr, outLen);
        }
    }

    internal static nuint GetCstDiagnosticCount(IntPtr handle)
    {
        EnsureLoaded();
        return _fnCstDiagnosticCount(handle);
    }

    internal static bool GetCstHasMissingIdent(IntPtr handle)
    {
        EnsureLoaded();
        return _fnCstHasMissingIdent(handle) != 0;
    }

    internal static bool GetCstHasMissingRParen(IntPtr handle)
    {
        EnsureLoaded();
        return _fnCstHasMissingRParen(handle) != 0;
    }

    internal static string GetCstText(IntPtr handle)
    {
        EnsureLoaded();
        nuint len = 0;
        byte* ptr = _fnCstToText(handle, &len);
        if (ptr == null || len == 0)
        {
            return string.Empty;
        }

        try
        {
            return Encoding.UTF8.GetString(ptr, (int)len);
        }
        finally
        {
            _fnFreeString(ptr, len);
        }
    }

    internal static void FreeCst(IntPtr handle)
    {
        if (handle != IntPtr.Zero && _fnCstFree != null)
        {
            _fnCstFree(handle);
        }
    }

    /// <summary>
    /// Milestone 32.1: Incrementally updates a native CST with sub-millisecond subtree splicing.
    /// </summary>
    public static NativeCstView ReparseIncremental(
        NativeCstView oldCst,
        string oldSource,
        uint start,
        uint end,
        string replacement,
        out ulong reparseMicros,
        out nuint reusedNodes)
    {
        ArgumentNullException.ThrowIfNull(oldCst);
        ArgumentNullException.ThrowIfNull(oldSource);
        ArgumentNullException.ThrowIfNull(replacement);
        EnsureLoaded();

        byte[] oldBytes = Encoding.UTF8.GetBytes(oldSource);
        byte[] repBytes = Encoding.UTF8.GetBytes(replacement);

        ulong micros = 0;
        nuint reused = 0;
        IntPtr newHandle;

        fixed (byte* oldPtr = oldBytes)
        fixed (byte* repPtr = repBytes)
        {
            newHandle = _fnIncrementalReparse(
                oldCst.Handle,
                oldPtr,
                (nuint)oldBytes.Length,
                start,
                end,
                repPtr,
                (nuint)repBytes.Length,
                &micros,
                &reused);
        }

        reparseMicros = micros;
        reusedNodes = reused;

        if (newHandle == IntPtr.Zero)
        {
            throw new InvalidOperationException("pilot_incremental_reparse returned null.");
        }

        return new NativeCstView(newHandle);
    }

    public static NativeDatabaseHandle CreateDatabase()
    {
        EnsureLoaded();
        IntPtr h = _fnDbNew();
        return new NativeDatabaseHandle(h);
    }

    public static NativeLspServerHandle CreateLspServer()
    {
        EnsureLoaded();
        IntPtr h = _fnLspNew();
        return new NativeLspServerHandle(h);
    }

    internal static void DbSetSource(IntPtr db, uint fileId, string source)
    {
        EnsureLoaded();
        byte[] bytes = Encoding.UTF8.GetBytes(source);
        fixed (byte* ptr = bytes)
        {
            _fnDbSetSource(db, fileId, ptr, (nuint)bytes.Length);
        }
    }

    internal static void DbApplyEdit(IntPtr db, uint fileId, uint start, uint end, string replacement)
    {
        EnsureLoaded();
        byte[] repBytes = Encoding.UTF8.GetBytes(replacement);
        fixed (byte* ptr = repBytes)
        {
            _fnDbApplyEdit(db, fileId, start, end, ptr, (nuint)repBytes.Length);
        }
    }

    internal static nuint DbEarlyCutoffCount(IntPtr db)
    {
        EnsureLoaded();
        return _fnDbEarlyCutoffCount(db);
    }

    internal static ulong DbSignatureHash(IntPtr db, uint fileId)
    {
        EnsureLoaded();
        return _fnDbSignatureHash(db, fileId);
    }

    internal static void FreeDb(IntPtr db)
    {
        if (db != IntPtr.Zero && _fnDbFree != null)
        {
            _fnDbFree(db);
        }
    }

    internal static string? LspHandleMessage(IntPtr server, string jsonMsg)
    {
        EnsureLoaded();
        byte[] msgBytes = Encoding.UTF8.GetBytes(jsonMsg);
        nuint outLen = 0;
        byte* resultPtr;

        fixed (byte* ptr = msgBytes)
        {
            resultPtr = _fnLspHandleMessage(server, ptr, (nuint)msgBytes.Length, &outLen);
        }

        if (resultPtr == null || outLen == 0)
        {
            return null;
        }

        try
        {
            return Encoding.UTF8.GetString(resultPtr, (int)outLen);
        }
        finally
        {
            _fnFreeString(resultPtr, outLen);
        }
    }

    internal static void FreeLsp(IntPtr server)
    {
        if (server != IntPtr.Zero && _fnLspFree != null)
        {
            _fnLspFree(server);
        }
    }

    public static NativeAnalysisHandle RunAnalysis(NativeAstArenaView arena)
    {
        ArgumentNullException.ThrowIfNull(arena);
        EnsureLoaded();
        IntPtr h = _fnAnalysisRun(arena.Handle);
        return new NativeAnalysisHandle(h);
    }

    public static NativePerceusHandle RunPerceus(NativeAstArenaView arena)
    {
        ArgumentNullException.ThrowIfNull(arena);
        EnsureLoaded();
        IntPtr h = _fnPerceusRun(arena.Handle);
        return new NativePerceusHandle(h);
    }

    internal static nuint AnalysisDiagnosticCount(IntPtr handle)
    {
        EnsureLoaded();
        return _fnAnalysisDiagnosticCount(handle);
    }

    internal static bool AnalysisHasErrors(IntPtr handle)
    {
        EnsureLoaded();
        return _fnAnalysisHasErrors(handle) != 0;
    }

    internal static void FreeAnalysis(IntPtr handle)
    {
        if (handle != IntPtr.Zero && _fnAnalysisFree != null)
        {
            _fnAnalysisFree(handle);
        }
    }

    internal static nuint PerceusRegionAllocCount(IntPtr handle)
    {
        EnsureLoaded();
        return _fnPerceusRegionAllocCount(handle);
    }

    internal static nuint PerceusStackAllocCount(IntPtr handle)
    {
        EnsureLoaded();
        return _fnPerceusStackAllocCount(handle);
    }

    internal static nuint PerceusGlobalAllocCount(IntPtr handle)
    {
        EnsureLoaded();
        return _fnPerceusGlobalAllocCount(handle);
    }

    internal static nuint PerceusDropReuseCount(IntPtr handle)
    {
        EnsureLoaded();
        return _fnPerceusDropReuseCount(handle);
    }

    internal static nuint PerceusElidedRetainCount(IntPtr handle)
    {
        EnsureLoaded();
        return _fnPerceusElidedRetainCount(handle);
    }

    internal static void FreePerceus(IntPtr handle)
    {
        if (handle != IntPtr.Zero && _fnPerceusFree != null)
        {
            _fnPerceusFree(handle);
        }
    }

    public static NativePilotVal CreatePilotValInt(long val)
    {
        EnsureLoaded();
        return _fnValFromInt(val);
    }

    public static NativePilotVal CreatePilotValFloat(double val)
    {
        EnsureLoaded();
        return _fnValFromFloat(val);
    }

    public static NativePilotVal CreatePilotValBool(bool val)
    {
        EnsureLoaded();
        return _fnValFromBool((byte)(val ? 1 : 0));
    }

    public static NativePilotVal CreatePilotValString(string s)
    {
        EnsureLoaded();
        byte[] bytes = Encoding.UTF8.GetBytes(s);
        fixed (byte* ptr = bytes)
        {
            return _fnValFromStr(ptr, (nuint)bytes.Length);
        }
    }

    public static NativePilotVal AddPilotVal(NativePilotVal a, NativePilotVal b)
    {
        EnsureLoaded();
        return _fnValAdd(a, b);
    }

    public static bool PilotValIsSso(NativePilotVal val)
    {
        EnsureLoaded();
        return _fnValIsSso(val) != 0;
    }

    public static NativeVmHandle CreateVm()
    {
        EnsureLoaded();
        IntPtr h = _fnVmNew();
        return new NativeVmHandle(h);
    }

    internal static long? PilotValAsInt(NativePilotVal val)
    {
        EnsureLoaded();
        long res = 0;
        if (_fnValAsInt(val, &res) != 0)
        {
            return res;
        }
        return null;
    }

    internal static double? PilotValAsFloat(NativePilotVal val)
    {
        EnsureLoaded();
        double res = 0;
        if (_fnValAsFloat(val, &res) != 0)
        {
            return res;
        }
        return null;
    }

    internal static string PilotValToString(NativePilotVal val)
    {
        EnsureLoaded();
        nuint len = 0;
        byte* ptr = _fnValToStr(val, &len);
        if (ptr == null || len == 0)
        {
            return string.Empty;
        }
        try
        {
            return Encoding.UTF8.GetString(ptr, (int)len);
        }
        finally
        {
            _fnFreeString(ptr, len);
        }
    }

    internal static NativePilotVal VmExecuteBytes(IntPtr vm, ReadOnlySpan<byte> bytes)
    {
        EnsureLoaded();
        fixed (byte* ptr = bytes)
        {
            return _fnVmExecuteBytes(vm, ptr, (nuint)bytes.Length);
        }
    }

    internal static void FreeVm(IntPtr vm)
    {
        if (vm != IntPtr.Zero && _fnVmFree != null)
        {
            _fnVmFree(vm);
        }
    }

    private static void EnsureLoaded()
    {
        if (_libraryHandle != IntPtr.Zero)
        {
            return;
        }

        lock (SyncLock)
        {
            if (_libraryHandle != IntPtr.Zero)
            {
                return;
            }

            string libPath = ResolveOrBuildNativeLibrary();
            IntPtr lib = NativeLibrary.Load(libPath);

            _fnAbiVersion = (delegate* unmanaged[Cdecl]<uint>)NativeLibrary.GetExport(lib, "pilot_ffi_abi_version");
            _fnSizeOfAstNode = (delegate* unmanaged[Cdecl]<uint>)NativeLibrary.GetExport(lib, "pilot_ffi_sizeof_ast_node");
            _fnParseUtf8ToArena = (delegate* unmanaged[Cdecl]<byte*, nuint, IntPtr>)NativeLibrary.GetExport(lib, "pilot_parse_utf8_to_arena");
            _fnArenaNodesPtr = (delegate* unmanaged[Cdecl]<IntPtr, nuint*, NativeAstNode*>)NativeLibrary.GetExport(lib, "pilot_arena_nodes_ptr");
            _fnArenaSpansPtr = (delegate* unmanaged[Cdecl]<IntPtr, nuint*, NativeByteSpan*>)NativeLibrary.GetExport(lib, "pilot_arena_spans_ptr");
            _fnArenaExtraChildrenPtr = (delegate* unmanaged[Cdecl]<IntPtr, nuint*, uint*>)NativeLibrary.GetExport(lib, "pilot_arena_extra_children_ptr");
            _fnArenaRootId = (delegate* unmanaged[Cdecl]<IntPtr, uint>)NativeLibrary.GetExport(lib, "pilot_arena_root_id");
            _fnArenaLineCount = (delegate* unmanaged[Cdecl]<IntPtr, uint>)NativeLibrary.GetExport(lib, "pilot_arena_line_count");
            _fnArenaLookupLineCol = (delegate* unmanaged[Cdecl]<IntPtr, uint, uint*, uint*, void>)NativeLibrary.GetExport(lib, "pilot_arena_lookup_line_col");
            _fnArenaFree = (delegate* unmanaged[Cdecl]<IntPtr, void>)NativeLibrary.GetExport(lib, "pilot_arena_free");

            _fnTokenizeUtf8 = (delegate* unmanaged[Cdecl]<byte*, nuint, IntPtr>)NativeLibrary.GetExport(lib, "pilot_tokenize_utf8");
            _fnTokenTapeLen = (delegate* unmanaged[Cdecl]<IntPtr, uint>)NativeLibrary.GetExport(lib, "pilot_token_tape_len");
            _fnTokenTapeKindsPtr = (delegate* unmanaged[Cdecl]<IntPtr, NativeTokenKind*>)NativeLibrary.GetExport(lib, "pilot_token_tape_kinds_ptr");
            _fnTokenTapeFlagsPtr = (delegate* unmanaged[Cdecl]<IntPtr, ushort*>)NativeLibrary.GetExport(lib, "pilot_token_tape_flags_ptr");
            _fnTokenTapeSpansPtr = (delegate* unmanaged[Cdecl]<IntPtr, NativeByteSpan*>)NativeLibrary.GetExport(lib, "pilot_token_tape_spans_ptr");
            _fnTokenTapeBracketLinksPtr = (delegate* unmanaged[Cdecl]<IntPtr, uint*>)NativeLibrary.GetExport(lib, "pilot_token_tape_bracket_links_ptr");
            _fnTokenTapeFree = (delegate* unmanaged[Cdecl]<IntPtr, void>)NativeLibrary.GetExport(lib, "pilot_token_tape_free");

            _fnCstParse = (delegate* unmanaged[Cdecl]<byte*, nuint, IntPtr>)NativeLibrary.GetExport(lib, "pilot_cst_parse");
            _fnCstFree = (delegate* unmanaged[Cdecl]<IntPtr, void>)NativeLibrary.GetExport(lib, "pilot_cst_free");
            _fnCstDiagnosticCount = (delegate* unmanaged[Cdecl]<IntPtr, nuint>)NativeLibrary.GetExport(lib, "pilot_cst_diagnostic_count");
            _fnCstHasMissingIdent = (delegate* unmanaged[Cdecl]<IntPtr, byte>)NativeLibrary.GetExport(lib, "pilot_cst_has_missing_ident");
            _fnCstHasMissingRParen = (delegate* unmanaged[Cdecl]<IntPtr, byte>)NativeLibrary.GetExport(lib, "pilot_cst_has_missing_rparen");
            _fnCstToText = (delegate* unmanaged[Cdecl]<IntPtr, nuint*, byte*>)NativeLibrary.GetExport(lib, "pilot_cst_to_text");
            _fnFormatSource = (delegate* unmanaged[Cdecl]<byte*, nuint, int, byte, nuint*, byte*, byte*>)NativeLibrary.GetExport(lib, "pilot_format_source");
            _fnFreeString = (delegate* unmanaged[Cdecl]<byte*, nuint, void>)NativeLibrary.GetExport(lib, "pilot_free_string");

            _fnIncrementalReparse = (delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, uint, uint, byte*, nuint, ulong*, nuint*, IntPtr>)NativeLibrary.GetExport(lib, "pilot_incremental_reparse");
            _fnDbNew = (delegate* unmanaged[Cdecl]<IntPtr>)NativeLibrary.GetExport(lib, "pilot_db_new");
            _fnDbFree = (delegate* unmanaged[Cdecl]<IntPtr, void>)NativeLibrary.GetExport(lib, "pilot_db_free");
            _fnDbSetSource = (delegate* unmanaged[Cdecl]<IntPtr, uint, byte*, nuint, void>)NativeLibrary.GetExport(lib, "pilot_db_set_source");
            _fnDbApplyEdit = (delegate* unmanaged[Cdecl]<IntPtr, uint, uint, uint, byte*, nuint, void>)NativeLibrary.GetExport(lib, "pilot_db_apply_edit");
            _fnDbEarlyCutoffCount = (delegate* unmanaged[Cdecl]<IntPtr, nuint>)NativeLibrary.GetExport(lib, "pilot_db_early_cutoff_count");
            _fnDbSignatureHash = (delegate* unmanaged[Cdecl]<IntPtr, uint, ulong>)NativeLibrary.GetExport(lib, "pilot_db_signature_hash");

            _fnLspNew = (delegate* unmanaged[Cdecl]<IntPtr>)NativeLibrary.GetExport(lib, "pilot_lsp_new");
            _fnLspFree = (delegate* unmanaged[Cdecl]<IntPtr, void>)NativeLibrary.GetExport(lib, "pilot_lsp_free");
            _fnLspHandleMessage = (delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, nuint*, byte*>)NativeLibrary.GetExport(lib, "pilot_lsp_handle_message");

            _fnAnalysisRun = (delegate* unmanaged[Cdecl]<IntPtr, IntPtr>)NativeLibrary.GetExport(lib, "pilot_analysis_run");
            _fnAnalysisFree = (delegate* unmanaged[Cdecl]<IntPtr, void>)NativeLibrary.GetExport(lib, "pilot_analysis_free");
            _fnAnalysisDiagnosticCount = (delegate* unmanaged[Cdecl]<IntPtr, nuint>)NativeLibrary.GetExport(lib, "pilot_analysis_diagnostic_count");
            _fnAnalysisHasErrors = (delegate* unmanaged[Cdecl]<IntPtr, byte>)NativeLibrary.GetExport(lib, "pilot_analysis_has_errors");

            _fnPerceusRun = (delegate* unmanaged[Cdecl]<IntPtr, IntPtr>)NativeLibrary.GetExport(lib, "pilot_perceus_run");
            _fnPerceusFree = (delegate* unmanaged[Cdecl]<IntPtr, void>)NativeLibrary.GetExport(lib, "pilot_perceus_free");
            _fnPerceusRegionAllocCount = (delegate* unmanaged[Cdecl]<IntPtr, nuint>)NativeLibrary.GetExport(lib, "pilot_perceus_region_alloc_count");
            _fnPerceusStackAllocCount = (delegate* unmanaged[Cdecl]<IntPtr, nuint>)NativeLibrary.GetExport(lib, "pilot_perceus_stack_alloc_count");
            _fnPerceusGlobalAllocCount = (delegate* unmanaged[Cdecl]<IntPtr, nuint>)NativeLibrary.GetExport(lib, "pilot_perceus_global_alloc_count");
            _fnPerceusDropReuseCount = (delegate* unmanaged[Cdecl]<IntPtr, nuint>)NativeLibrary.GetExport(lib, "pilot_perceus_drop_reuse_count");
            _fnPerceusElidedRetainCount = (delegate* unmanaged[Cdecl]<IntPtr, nuint>)NativeLibrary.GetExport(lib, "pilot_perceus_elided_retain_count");

            _fnValFromInt = (delegate* unmanaged[Cdecl]<long, NativePilotVal>)NativeLibrary.GetExport(lib, "pilot_val_from_int");
            _fnValFromFloat = (delegate* unmanaged[Cdecl]<double, NativePilotVal>)NativeLibrary.GetExport(lib, "pilot_val_from_float");
            _fnValFromBool = (delegate* unmanaged[Cdecl]<byte, NativePilotVal>)NativeLibrary.GetExport(lib, "pilot_val_from_bool");
            _fnValFromStr = (delegate* unmanaged[Cdecl]<byte*, nuint, NativePilotVal>)NativeLibrary.GetExport(lib, "pilot_val_from_str");
            _fnValToStr = (delegate* unmanaged[Cdecl]<NativePilotVal, nuint*, byte*>)NativeLibrary.GetExport(lib, "pilot_val_to_str");
            _fnValIsSso = (delegate* unmanaged[Cdecl]<NativePilotVal, byte>)NativeLibrary.GetExport(lib, "pilot_val_is_sso");
            _fnValAsInt = (delegate* unmanaged[Cdecl]<NativePilotVal, long*, byte>)NativeLibrary.GetExport(lib, "pilot_val_as_int");
            _fnValAsFloat = (delegate* unmanaged[Cdecl]<NativePilotVal, double*, byte>)NativeLibrary.GetExport(lib, "pilot_val_as_float");
            _fnValAdd = (delegate* unmanaged[Cdecl]<NativePilotVal, NativePilotVal, NativePilotVal>)NativeLibrary.GetExport(lib, "pilot_val_add");

            _fnVmNew = (delegate* unmanaged[Cdecl]<IntPtr>)NativeLibrary.GetExport(lib, "pilot_vm_new");
            _fnVmFree = (delegate* unmanaged[Cdecl]<IntPtr, void>)NativeLibrary.GetExport(lib, "pilot_vm_free");
            _fnVmExecuteBytes = (delegate* unmanaged[Cdecl]<IntPtr, byte*, nuint, NativePilotVal>)NativeLibrary.GetExport(lib, "pilot_vm_execute_bytes");

            uint abi = _fnAbiVersion();
            if (abi != ExpectedAbiVersion)
            {
                throw new InvalidOperationException($"pilot-ffi ABI mismatch: expected {ExpectedAbiVersion}, got {abi}.");
            }

            uint nativeNodeSize = _fnSizeOfAstNode();
            if (nativeNodeSize != 16)
            {
                throw new InvalidOperationException($"pilot-ffi AstNode size mismatch: expected 16 bytes, got {nativeNodeSize}.");
            }

            _libraryHandle = lib;
        }
    }

    private static string ResolveOrBuildNativeLibrary()
    {
        string fileName = RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
            ? "pilot_ffi.dll"
            : RuntimeInformation.IsOSPlatform(OSPlatform.OSX)
                ? "libpilot_ffi.dylib"
                : "libpilot_ffi.so";

        string baseDir = AppContext.BaseDirectory;
        string bundledCandidate = Path.Combine(baseDir, "native", fileName);
        if (File.Exists(bundledCandidate))
        {
            return bundledCandidate;
        }

        string? repoRoot = FindRepoRoot(baseDir) ?? FindRepoRoot(Directory.GetCurrentDirectory());
        if (repoRoot != null)
        {
            string releasePath = Path.Combine(repoRoot, "target", "release", fileName);
            if (File.Exists(releasePath))
            {
                return releasePath;
            }

            string debugPath = Path.Combine(repoRoot, "target", "debug", fileName);
            if (File.Exists(debugPath))
            {
                return debugPath;
            }

            // Build on-demand via cargo if Cargo.toml exists
            BuildRustFfiLibrary(repoRoot);
            if (File.Exists(releasePath))
            {
                return releasePath;
            }
        }

        throw new FileNotFoundException($"Could not locate or build native library '{fileName}'.");
    }

    private static string? FindRepoRoot(string startDir)
    {
        var dir = new DirectoryInfo(startDir);
        while (dir != null)
        {
            if (File.Exists(Path.Combine(dir.FullName, "Cargo.toml")) &&
                Directory.Exists(Path.Combine(dir.FullName, "crates", "pilot-ffi")))
            {
                return dir.FullName;
            }
            dir = dir.Parent;
        }
        return null;
    }

    private static void BuildRustFfiLibrary(string repoRoot)
    {
        string cargoExe = "cargo";
        string homeCargo = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            ".cargo",
            "bin",
            RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? "cargo.exe" : "cargo");
        if (File.Exists(homeCargo))
        {
            cargoExe = homeCargo;
        }

        var psi = new ProcessStartInfo
        {
            FileName = cargoExe,
            Arguments = "build --release -p pilot-ffi",
            WorkingDirectory = repoRoot,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true,
        };

        string cargoBinDir = Path.GetDirectoryName(homeCargo) ?? string.Empty;
        if (!string.IsNullOrEmpty(cargoBinDir) && Directory.Exists(cargoBinDir))
        {
            string existingPath = Environment.GetEnvironmentVariable("PATH") ?? string.Empty;
            psi.Environment["PATH"] = $"{cargoBinDir}{Path.PathSeparator}{existingPath}";
        }

        using var proc = Process.Start(psi)
            ?? throw new InvalidOperationException("Failed to start cargo build for pilot-ffi.");
        string stderr = proc.StandardError.ReadToEnd();
        proc.WaitForExit();
        if (proc.ExitCode != 0)
        {
            throw new InvalidOperationException($"cargo build --release -p pilot-ffi failed (exit {proc.ExitCode}): {stderr}");
        }
    }
}
