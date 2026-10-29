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
/// Milestone 28.4: Zero-copy C-ABI strangler-fig bridge between C# and the Rust <c>pilot-ffi</c> crate.
/// </summary>
public static unsafe class NativeAstBridge
{
    public const uint ExpectedAbiVersion = 2801;

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
