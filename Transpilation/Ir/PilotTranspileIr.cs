using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using Antlr4.Runtime;
using Antlr4.Runtime.Tree;
using Pilot.Analysis;
using Pilot.Analysis.Loops;
using Pilot.Analysis.TypeSystem;
using Pilot.Diagnostics;
using Pilot.Preprocessing;

namespace Pilot.Transpilation.Ir;

/// <summary>
/// Operator precedence tiers for precedence-aware parenthesization in target emitters and <c>PilotFormatter</c>.
/// </summary>
public enum PilotIrPrecedence
{
    Lowest = 0,
    Lambda = 1,
    Conditional = 2,
    Pipe = 3,
    Coalesce = 4,
    LogicalOr = 5,
    LogicalAnd = 6,
    LogicalNot = 7,
    Comparison = 8,
    Range = 9,
    BitwiseOr = 10,
    BitwiseXor = 11,
    BitwiseAnd = 12,
    Shift = 13,
    Additive = 14,
    Multiplicative = 15,
    Unary = 16,
    Power = 17,
    Postfix = 18,
    Primary = 19
}

public enum PilotTriviaKind
{
    LineComment,
    BlockComment,
    DocComment,
    BlankLine
}

/// <summary>
/// Trivia item (comment or blank line) preserved during CST/AST round-tripping.
/// </summary>
public sealed record PilotTrivia(PilotTriviaKind Kind, string Text, int Line);

/// <summary>
/// C-ABI memory layout metadata attached to lowered <c>for</c> expressions and range loops.
/// </summary>
public sealed record PilotCAbiLoopLayout(
    LoopScheduleMode ScheduleMode,
    LoopReturnMode ReturnMode,
    string ElementType,
    bool UsesSliceBuffer,
    bool UsesRangeWindow,
    int StructByteSize);

/// <summary>
/// AST-scoped library call classification resolved on <see cref="PilotIrCallExpr"/> nodes
/// so target emitters never run regexes over string literals or comments.
/// </summary>
public enum PilotLibraryCallKind
{
    None = 0,
    Println,
    Print,
    MatrixCreate,
    MatrixZeros,
    MatrixOnes,
    MatrixEye,
    MatrixVectorize,
    JsonDumps,
    JsonLoads,
    FileReadText,
    FileWriteText,
    FileExists,
    ParallelMap,
    MathSqrt,
    MathSin,
    MathCos,
    MathAbs,
    MathFloor,
    MathCeil,
    MathPow,
    Len,
    Range,
    SomeConstructor,
    FailConstructor
}

/// <summary>
/// Base class for all Canonical Typed Transpilation IR nodes (Milestone 20.1).
/// </summary>
public abstract class PilotIrNode
{
    public int Line { get; init; }
    public int Column { get; init; }
    public PilotType ResolvedType { get; set; } = PilotType.Any;
    public List<PilotTrivia> LeadingTrivia { get; } = new();
    public List<PilotTrivia> TrailingTrivia { get; } = new();
}

/// <summary>
/// Canonical typed module representing a complete Pilot translation unit after preprocessing,
/// parsing, type inference (M16), and static demotion / comptime folding (M19).
/// </summary>
public sealed class PilotIrModule : PilotIrNode
{
    public List<string> HeaderComments { get; } = new();
    public List<PilotIrImportStmt> Imports { get; } = new();
    public List<PilotIrStmt> Statements { get; } = new();
    public List<PilotTrivia> EndOfFileTrivia { get; } = new();
    public HashSet<string> MutatedVariables { get; } = new(StringComparer.Ordinal);
    public Dictionary<string, PilotType> SymbolTypes { get; } = new(StringComparer.Ordinal);
    public Dictionary<string, string> FunctionReturnTypes { get; } = new(StringComparer.Ordinal);
    public PilotStaticAnalysisResult? AnalysisResult { get; set; }

    public bool UsesMatrixPrimitives { get; set; }
    public bool UsesJsonPrimitives { get; set; }
    public bool UsesOptionTypes { get; set; }
    public bool UsesResultTypes { get; set; }
    public bool UsesSliceBuffer { get; set; }
    public bool UsesRangeWindow { get; set; }
    public bool PreferColonSuites { get; set; }
}

// ============================================================================
// IR STATEMENTS
// ============================================================================

public abstract class PilotIrStmt : PilotIrNode
{
}

public sealed class PilotIrCommentStmt : PilotIrStmt
{
    public string CommentText { get; init; } = "";
    public bool IsDocComment { get; init; }
}

public sealed class PilotIrImportStmt : PilotIrStmt
{
    public string ModulePath { get; init; } = "";
    public string? Alias { get; init; }
    public List<(string Name, string? Alias)> ImportedItems { get; } = new();
    public bool IsForeignImport { get; init; }
    public string? ForeignLanguage { get; init; }
    public bool IsMappedNativeEquivalent { get; set; }
}

public enum PilotVarDeclKind
{
    Let,
    Var,
    Const
}

public sealed class PilotIrVarDeclStmt : PilotIrStmt
{
    public PilotVarDeclKind Kind { get; set; } = PilotVarDeclKind.Let;
    public string Name { get; set; } = "";
    public List<string> TupleDestructureNames { get; } = new();
    public string? DeclaredType { get; set; }
    public string EffectiveType => !string.IsNullOrWhiteSpace(DeclaredType)
        ? DeclaredType!
        : (ResolvedType != PilotType.Any && ResolvedType != PilotType.Void ? ResolvedType.ToString() : "auto");
    public PilotIrExpr? Initializer { get; set; }
    public bool IsMutated { get; set; }
    public string? ComptimeFoldedLiteral { get; set; }
}

public sealed class PilotIrAssignStmt : PilotIrStmt
{
    public PilotIrExpr Target { get; set; } = null!;
    public string Operator { get; set; } = "=";
    public PilotIrExpr Value { get; set; } = null!;
}

public sealed record PilotIrParameter(
    string Name,
    string? TypeAnnotation = null,
    PilotIrExpr? DefaultValue = null,
    bool IsContext = false,
    bool IsComptime = false,
    bool IsMutable = false,
    bool IsVariadic = false);

public sealed class PilotIrFuncDefStmt : PilotIrStmt
{
    public string Name { get; set; } = "";
    public List<string> GenericParams { get; } = new();
    public List<PilotIrParameter> Parameters { get; } = new();
    public string? DeclaredReturnType { get; set; }
    public string? InferredReturnType { get; set; }
    public string EffectiveReturnType => !string.IsNullOrWhiteSpace(DeclaredReturnType)
        ? DeclaredReturnType!
        : (!string.IsNullOrWhiteSpace(InferredReturnType) ? InferredReturnType! : "void");
    public List<PilotIrExpr> RequiresClauses { get; } = new();
    public List<PilotIrExpr> EnsuresClauses { get; } = new();
    public List<PilotIrStmt> Body { get; } = new();
    public List<string> Decorators { get; } = new();
    public string Visibility { get; set; } = "public";
    public bool IsStatic { get; set; }
    public bool IsAsync { get; set; }
    public bool IsPure { get; set; }
    public bool IsComptime { get; set; }
    public bool IsExpressionBody { get; set; }
    public HashSet<string> MutatedLocals { get; } = new(StringComparer.Ordinal);
}

public enum PilotTypeDeclKind
{
    Class,
    Struct,
    Record,
    Interface
}

public sealed record PilotIrFieldDecl(
    string Name,
    string TypeAnnotation,
    PilotIrExpr? DefaultValue = null,
    bool IsMutable = true,
    string Visibility = "public");

public sealed class PilotIrClassDefStmt : PilotIrStmt
{
    public PilotTypeDeclKind Kind { get; set; } = PilotTypeDeclKind.Class;
    public string Name { get; set; } = "";
    public List<string> GenericParams { get; } = new();
    public List<string> BaseTypes { get; } = new();
    public List<PilotIrFieldDecl> Fields { get; } = new();
    public List<PilotIrFuncDefStmt> Methods { get; } = new();
    public List<string> Decorators { get; } = new();
    public string Visibility { get; set; } = "public";
}

public sealed record PilotIrAdtVariant(
    string Name,
    IReadOnlyList<(string Name, string Type)> Fields);

/// <summary>
/// Algebraic Data Type declaration (<c>type Shape = Circle(r: f64) | Rect(w: f64, h: f64) | Point</c>).
/// </summary>
public sealed class PilotIrAdtDefStmt : PilotIrStmt
{
    public string Name { get; set; } = "";
    public List<string> GenericParams { get; } = new();
    public List<PilotIrAdtVariant> Variants { get; } = new();
}

public sealed class PilotIrTypeAliasStmt : PilotIrStmt
{
    public string Name { get; set; } = "";
    public List<string> GenericParams { get; } = new();
    public string TargetType { get; set; } = "";
}

public sealed class PilotIrEnumDefStmt : PilotIrStmt
{
    public string Name { get; set; } = "";
    public List<(string Name, PilotIrExpr? Value)> Members { get; } = new();
}

/// <summary>
/// First-class type extension block (<c>extend str { def shout(self) -&gt; str { ... } }</c>).
/// </summary>
public sealed class PilotIrExtensionDefStmt : PilotIrStmt
{
    public string TargetType { get; set; } = "";
    public string? ExtensionName { get; set; }
    public List<PilotIrFuncDefStmt> Methods { get; } = new();
}

public sealed class PilotIrIfStmt : PilotIrStmt
{
    public PilotIrExpr Condition { get; set; } = null!;
    public string? OptionalBindingVar { get; set; }
    public List<PilotIrStmt> ThenBody { get; } = new();
    public List<(PilotIrExpr Condition, List<PilotIrStmt> Body)> ElifBranches { get; } = new();
    public List<PilotIrStmt> ElseBody { get; } = new();
}

public sealed class PilotIrGuardStmt : PilotIrStmt
{
    public bool IsSomeBinding { get; set; }
    public string? BoundVariable { get; set; }
    public PilotIrExpr ConditionOrSource { get; set; } = null!;
    public PilotIrExpr? PatternGuard { get; set; }
    public string? CatchErrorVar { get; set; }
    public List<PilotIrStmt> ElseBody { get; } = new();
}

public sealed class PilotIrDeferStmt : PilotIrStmt
{
    public List<PilotIrStmt> Body { get; } = new();
}

public sealed class PilotIrWhileStmt : PilotIrStmt
{
    public PilotIrExpr Condition { get; set; } = null!;
    public List<PilotIrStmt> Body { get; } = new();
}

public sealed class PilotIrForStmt : PilotIrStmt
{
    public List<string> TargetVars { get; } = new();
    public PilotIrExpr Iterable { get; set; } = null!;
    public PilotIrExpr? WhereFilter { get; set; }
    public List<PilotIrStmt> Body { get; } = new();
    public List<PilotIrStmt> CompletionThenBody { get; } = new();
    public PilotCAbiLoopLayout LoopLayout { get; set; } = new(
        LoopScheduleMode.FixedScalar,
        LoopReturnMode.Void,
        "i64",
        false,
        false,
        24);
}

public sealed record PilotIrMatchArm(
    PilotIrPattern Pattern,
    PilotIrExpr? Guard,
    List<PilotIrStmt> Body,
    PilotIrExpr? ExpressionBody = null);

public sealed class PilotIrMatchStmt : PilotIrStmt
{
    public PilotIrExpr Scrutinee { get; set; } = null!;
    public List<PilotIrMatchArm> Arms { get; } = new();
}

public sealed record PilotIrCatchClause(
    string? ExceptionType,
    string? VariableName,
    List<PilotIrStmt> Body);

public sealed class PilotIrTryStmt : PilotIrStmt
{
    public List<PilotIrStmt> TryBody { get; } = new();
    public List<PilotIrCatchClause> Catches { get; } = new();
    public List<PilotIrStmt> FinallyBody { get; } = new();
}

public sealed class PilotIrReturnStmt : PilotIrStmt
{
    public PilotIrExpr? Value { get; set; }
}

public sealed class PilotIrYieldStmt : PilotIrStmt
{
    public PilotIrExpr? Value { get; set; }
}

public sealed class PilotIrRaiseStmt : PilotIrStmt
{
    public PilotIrExpr? Value { get; set; }
}

public sealed class PilotIrBreakStmt : PilotIrStmt
{
    public PilotIrExpr? Value { get; set; }
}

public sealed class PilotIrContinueStmt : PilotIrStmt
{
    public PilotIrExpr? RangePivot { get; set; }
    public bool IsSubLoopPrepend { get; set; }
}

public sealed class PilotIrPassStmt : PilotIrStmt
{
}

public sealed class PilotIrAssertStmt : PilotIrStmt
{
    public PilotIrExpr Condition { get; set; } = null!;
    public PilotIrExpr? Message { get; set; }
    public bool IsComptime { get; set; }
}

public sealed class PilotIrComptimeStmt : PilotIrStmt
{
    public List<PilotIrStmt> Body { get; } = new();
}

public sealed class PilotIrExprStmt : PilotIrStmt
{
    public PilotIrExpr Expression { get; set; } = null!;
}

// ============================================================================
// IR EXPRESSIONS
// ============================================================================

public abstract class PilotIrExpr : PilotIrNode
{
    public abstract PilotIrPrecedence Precedence { get; }
}

public enum PilotLiteralKind
{
    Integer,
    Float,
    Boolean,
    String,
    Null,
    None,
    Keyword,
    DimensionalUnit
}

public sealed class PilotIrLiteralExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => PilotIrPrecedence.Primary;
    public PilotLiteralKind Kind { get; set; }
    public string RawValue { get; set; } = "";
    public string StringContent { get; set; } = "";
    public string? UnitSuffix { get; set; }
    public double? NormalizedNumericValue { get; set; }
}

public sealed record PilotIrFStringPart(
    bool IsInterpolation,
    string LiteralText,
    PilotIrExpr? Expression = null,
    string? FormatSpec = null);

public sealed class PilotIrInterpolatedStringExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => PilotIrPrecedence.Primary;
    public List<PilotIrFStringPart> Parts { get; } = new();
}

public sealed class PilotIrIdentifierExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => PilotIrPrecedence.Primary;
    public string Name { get; set; } = "";
}

public sealed class PilotIrUnaryExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => Operator is "not" or "!"
        ? PilotIrPrecedence.LogicalNot
        : PilotIrPrecedence.Unary;
    public string Operator { get; set; } = "";
    public PilotIrExpr Operand { get; set; } = null!;
}

public sealed class PilotIrBinaryExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => GetOperatorPrecedence(Operator);
    public string Operator { get; set; } = "";
    public PilotIrExpr Left { get; set; } = null!;
    public PilotIrExpr Right { get; set; } = null!;

    public static PilotIrPrecedence GetOperatorPrecedence(string op) => op switch
    {
        "|>" or "<|" => PilotIrPrecedence.Pipe,
        "??" => PilotIrPrecedence.Coalesce,
        "or" or "||" => PilotIrPrecedence.LogicalOr,
        "and" or "&&" => PilotIrPrecedence.LogicalAnd,
        "==" or "!=" or "<" or "<=" or ">" or ">=" or "in" or "not in" or "is" or "is not" => PilotIrPrecedence.Comparison,
        ".." or "..=" or "..<" or "..." => PilotIrPrecedence.Range,
        "|" => PilotIrPrecedence.BitwiseOr,
        "^" => PilotIrPrecedence.BitwiseXor,
        "&" => PilotIrPrecedence.BitwiseAnd,
        "<<" or ">>" => PilotIrPrecedence.Shift,
        "+" or "-" => PilotIrPrecedence.Additive,
        "*" or "/" or "//" or "%" or "->" => PilotIrPrecedence.Multiplicative,
        "**" => PilotIrPrecedence.Power,
        _ => PilotIrPrecedence.Additive
    };
}

public sealed class PilotIrRangeExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => PilotIrPrecedence.Range;
    public PilotIrExpr Start { get; set; } = null!;
    public PilotIrExpr End { get; set; } = null!;
    public PilotIrExpr? Step { get; set; }
    public bool IsInclusive { get; set; } = true;
}

public sealed record PilotIrCallArg(string? Name, PilotIrExpr Value, string? Modifier = null);

public sealed class PilotIrCallExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => PilotIrPrecedence.Postfix;
    public PilotIrExpr Callee { get; set; } = null!;
    public List<PilotIrCallArg> Arguments { get; } = new();
    public string? QuoteOperator { get; set; }
    public PilotLibraryCallKind LibraryCallKind { get; set; } = PilotLibraryCallKind.None;
}

public sealed class PilotIrMemberAccessExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => PilotIrPrecedence.Postfix;
    public PilotIrExpr Target { get; set; } = null!;
    public string MemberName { get; set; } = "";
    public bool IsSafeNavigation { get; set; }
}

public sealed class PilotIrIndexExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => PilotIrPrecedence.Postfix;
    public PilotIrExpr Target { get; set; } = null!;
    public PilotIrExpr Index { get; set; } = null!;
    public PilotIrExpr? SliceEnd { get; set; }
    public bool IsSlice { get; set; }
    public bool IsSafeNavigation { get; set; }
}

/// <summary>
/// Postfix error propagation (<c>expr?</c>) or inline catch recovery (<c>expr catch err =&gt; fallback</c>).
/// </summary>
public sealed class PilotIrPostfixTryExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => PilotIrPrecedence.Postfix;
    public PilotIrExpr Operand { get; set; } = null!;
    public bool IsQuestionPropagate { get; set; }
    public string? CatchVarName { get; set; }
    public PilotIrExpr? CatchFallbackExpr { get; set; }
    public List<PilotIrStmt> CatchFallbackBody { get; } = new();
}

public enum PilotCollectionKind
{
    List,
    Tuple,
    Set,
    Map
}

public sealed class PilotIrCollectionExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => PilotIrPrecedence.Primary;
    public PilotCollectionKind Kind { get; set; } = PilotCollectionKind.List;
    public List<PilotIrExpr> Elements { get; } = new();
    public List<(PilotIrExpr Key, PilotIrExpr Value)> MapEntries { get; } = new();
}

public sealed class PilotIrLambdaExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => PilotIrPrecedence.Lambda;
    public List<(string Name, string? Type)> Parameters { get; } = new();
    public PilotIrExpr? BodyExpr { get; set; }
    public List<PilotIrStmt> BodyStmts { get; } = new();
}

/// <summary>
/// Expression-oriented <c>for</c> loop / comprehension (<c>[x * 2 for x in 1..5 if x &gt; 1]</c> or <c>for x in 1..5 { yield x * 2; }</c>).
/// </summary>
public sealed class PilotIrForExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => PilotIrPrecedence.Primary;
    public List<string> TargetVars { get; } = new();
    public PilotIrExpr Iterable { get; set; } = null!;
    public PilotIrExpr? FilterCondition { get; set; }
    public PilotIrExpr YieldExpr { get; set; } = null!;
    public PilotCAbiLoopLayout LoopLayout { get; set; } = new(
        LoopScheduleMode.FixedScalar,
        LoopReturnMode.CollectionYield,
        "i64",
        true,
        false,
        24);
}

public sealed class PilotIrIfExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => PilotIrPrecedence.Conditional;
    public PilotIrExpr Condition { get; set; } = null!;
    public PilotIrExpr ThenBranch { get; set; } = null!;
    public PilotIrExpr ElseBranch { get; set; } = null!;
}

public sealed class PilotIrMatchExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => PilotIrPrecedence.Primary;
    public PilotIrExpr Scrutinee { get; set; } = null!;
    public List<PilotIrMatchArm> Arms { get; } = new();
}

public sealed class PilotIrSqlQueryExpr : PilotIrExpr
{
    public override PilotIrPrecedence Precedence => PilotIrPrecedence.Primary;
    public string RangeVariable { get; set; } = "row";
    public PilotIrExpr Source { get; set; } = null!;
    public PilotIrExpr? WherePredicate { get; set; }
    public PilotIrExpr? OrderByKey { get; set; }
    public bool OrderDescending { get; set; }
    public PilotIrExpr SelectProjection { get; set; } = null!;
    public PilotIrExpr? LimitCount { get; set; }
}

// ============================================================================
// IR PATTERNS
// ============================================================================

public abstract class PilotIrPattern : PilotIrNode
{
}

public sealed class PilotIrWildcardPattern : PilotIrPattern
{
}

public sealed class PilotIrLiteralPattern : PilotIrPattern
{
    public PilotIrLiteralExpr Literal { get; set; } = null!;
}

public sealed class PilotIrBindingPattern : PilotIrPattern
{
    public string Name { get; set; } = "";
    public string? TypeName { get; set; }
    public bool IsMutable { get; set; }
}

public sealed class PilotIrOptionPattern : PilotIrPattern
{
    public bool IsSome { get; set; }
    public PilotIrPattern? InnerPattern { get; set; }
}

public sealed class PilotIrVariantPattern : PilotIrPattern
{
    public string? TypeName { get; set; }
    public string VariantName { get; set; } = "";
    public List<(string? FieldName, PilotIrPattern SubPattern)> Arguments { get; } = new();
}

public sealed class PilotIrRangePattern : PilotIrPattern
{
    public PilotIrExpr Start { get; set; } = null!;
    public PilotIrExpr End { get; set; } = null!;
    public bool IsInclusive { get; set; } = true;
}

public sealed class PilotIrRelationalPattern : PilotIrPattern
{
    public string Operator { get; set; } = "";
    public PilotIrExpr Value { get; set; } = null!;
}

public sealed class PilotIrOrPattern : PilotIrPattern
{
    public List<PilotIrPattern> Alternatives { get; } = new();
}

public sealed class PilotIrTuplePattern : PilotIrPattern
{
    public List<PilotIrPattern> Elements { get; } = new();
}

// ============================================================================
// CANONICAL IR BUILDER (ANTLR4 CST + M16/M19 STATIC ANALYSIS -> PilotIrModule)
// ============================================================================

public sealed class PilotTranspileIrBuilder
{
    private readonly CommonTokenStream? _tokenStream;
    private readonly PilotStaticAnalysisResult? _analysis;
    private readonly HashSet<string> _mutatedVars;
    private readonly TranspilationOptions _options;
    private readonly PilotIrModule _module;
    private int _consumedTokenIndex = -1;

    private PilotTranspileIrBuilder(
        CommonTokenStream? tokenStream,
        PilotStaticAnalysisResult? analysis,
        HashSet<string> mutatedVars,
        TranspilationOptions options)
    {
        _tokenStream = tokenStream;
        _analysis = analysis;
        _mutatedVars = mutatedVars;
        _options = options;
        _module = new PilotIrModule
        {
            AnalysisResult = analysis,
            PreferColonSuites = options.PreferColonSuites
        };
        foreach (var v in mutatedVars)
            _module.MutatedVariables.Add(v);

        if (analysis != null)
        {
            foreach (var kv in analysis.InferredTypes)
                _module.SymbolTypes[kv.Key] = kv.Value;
            foreach (var kv in analysis.InferredFunctionReturnTypes)
                _module.FunctionReturnTypes[kv.Key] = kv.Value;
        }
    }

    /// <summary>
    /// Runs the full <c>PilotPreprocessor</c> -&gt; <c>PilotLexer</c> -&gt; <c>PilotParser</c> -&gt; <c>PilotAnalysisPipeline</c>
    /// and lowers the parse tree into a <see cref="PilotIrModule"/>.
    /// </summary>
    public static (PilotIrModule? Module, IReadOnlyList<string> Errors) BuildFromSource(
        string pilotSource,
        TranspilationOptions? options = null)
    {
        options ??= new TranspilationOptions();
        var preproc = PilotPreprocessor.Preprocess(pilotSource, options.SourceFilePath ?? "");
        if (preproc.HasErrors)
        {
            return (null, preproc.Diagnostics.Select(d => d.Message).ToList());
        }

        var inputStream = new AntlrInputStream(preproc.PreprocessedSource);
        var lexer = new PilotLexer(inputStream);
        var errorTracker = new ErrorTracker();
        lexer.RemoveErrorListeners();

        var tokenStream = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokenStream);
        parser.RemoveErrorListeners();
        parser.AddErrorListener(errorTracker);

        var tree = parser.ParseProgramFast();
        if (errorTracker.HasErrors)
        {
            return (null, errorTracker.Errors);
        }

        PilotStaticAnalysisResult? analysis = null;
        try
        {
            analysis = PilotAnalysisPipeline.Analyze(tree);
        }
        catch
        {
            // Keep building IR even if an optional analysis sub-pass encounters dynamic constructs
        }

        var module = Build(tree, tokenStream, analysis, options, pilotSource);
        return (module, Array.Empty<string>());
    }

    public static PilotIrModule Build(
        PilotParser.ProgramContext program,
        CommonTokenStream? tokenStream = null,
        PilotStaticAnalysisResult? analysis = null,
        TranspilationOptions? options = null,
        string? rawOriginalSource = null)
    {
        options ??= new TranspilationOptions();
        if (analysis == null)
        {
            try
            {
                analysis = PilotAnalysisPipeline.Analyze(program);
            }
            catch
            {
                // Ignore
            }
        }

        var mutatedVars = PilotOptimizer.AnalyzeMutatedVariables(program);
        var builder = new PilotTranspileIrBuilder(tokenStream, analysis, mutatedVars, options);
        if (!string.IsNullOrEmpty(rawOriginalSource))
        {
            builder.ExtractRawSourceLeadingComments(rawOriginalSource!);
        }
        builder.VisitProgram(program);
        return builder._module;
    }

    private void ExtractRawSourceLeadingComments(string rawSource)
    {
        // Detect if the source predominantly uses colon suites
        var lines = rawSource.Split(new[] { "\r\n", "\r", "\n" }, StringSplitOptions.None);
        int colonSuiteCount = 0;
        int braceSuiteCount = 0;
        foreach (var line in lines)
        {
            string t = line.Trim();
            if ((t.StartsWith("def ") || t.StartsWith("fn ") || t.StartsWith("if ") || t.StartsWith("for ") || t.StartsWith("while ")) && t.EndsWith(":"))
                colonSuiteCount++;
            else if ((t.StartsWith("def ") || t.StartsWith("fn ") || t.StartsWith("if ") || t.StartsWith("for ") || t.StartsWith("while ")) && t.EndsWith("{"))
                braceSuiteCount++;
        }
        if (colonSuiteCount > braceSuiteCount && colonSuiteCount > 0)
        {
            _module.PreferColonSuites = true;
        }
    }

    private List<PilotTrivia> ConsumeTriviaBefore(ParserRuleContext ctx)
    {
        var trivia = new List<PilotTrivia>();
        if (_tokenStream == null || ctx.Start == null)
            return trivia;

        int targetIdx = ctx.Start.TokenIndex;
        if (targetIdx <= _consumedTokenIndex)
            return trivia;

        _tokenStream.Fill();
        var tokens = _tokenStream.GetTokens();
        int lastLine = _consumedTokenIndex >= 0 && _consumedTokenIndex < tokens.Count
            ? tokens[_consumedTokenIndex].Line
            : 0;

        for (int i = _consumedTokenIndex + 1; i < targetIdx && i < tokens.Count; i++)
        {
            var tok = tokens[i];
            if (tok.Channel != Lexer.DefaultTokenChannel)
            {
                string txt = tok.Text.Trim();
                if (string.IsNullOrEmpty(txt))
                    continue;

                if (lastLine > 0 && tok.Line > lastLine + 1)
                {
                    trivia.Add(new PilotTrivia(PilotTriviaKind.BlankLine, "", tok.Line - 1));
                }

                if (txt.StartsWith("///") || txt.StartsWith("##"))
                    trivia.Add(new PilotTrivia(PilotTriviaKind.DocComment, txt, tok.Line));
                else if (txt.StartsWith("/*"))
                    trivia.Add(new PilotTrivia(PilotTriviaKind.BlockComment, txt, tok.Line));
                else if (txt.StartsWith("//") || txt.StartsWith("#"))
                    trivia.Add(new PilotTrivia(PilotTriviaKind.LineComment, txt, tok.Line));

                lastLine = tok.Line;
            }
        }

        if (lastLine > 0 && ctx.Start.Line > lastLine + 1)
        {
            trivia.Add(new PilotTrivia(PilotTriviaKind.BlankLine, "", ctx.Start.Line - 1));
        }

        _consumedTokenIndex = targetIdx;

        return trivia;
    }

    private void CollectRemainingTrivia()
    {
        if (_tokenStream == null)
            return;

        _tokenStream.Fill();
        var tokens = _tokenStream.GetTokens();
        for (int i = _consumedTokenIndex + 1; i < tokens.Count; i++)
        {
            var tok = tokens[i];
            if (tok.Channel != Lexer.DefaultTokenChannel)
            {
                string txt = tok.Text.Trim();
                if (txt.StartsWith("//") || txt.StartsWith("/*") || txt.StartsWith("#"))
                {
                    _module.EndOfFileTrivia.Add(new PilotTrivia(
                        txt.StartsWith("/*") ? PilotTriviaKind.BlockComment : PilotTriviaKind.LineComment,
                        txt,
                        tok.Line));
                }
            }
        }
    }

    private void VisitProgram(PilotParser.ProgramContext ctx)
    {
        foreach (var stmtCtx in ctx.stmt())
        {
            var leading = ConsumeTriviaBefore(stmtCtx);
            var stmts = VisitStmt(stmtCtx);
            if (stmts.Count > 0)
            {
                stmts[0].LeadingTrivia.AddRange(leading);
                foreach (var s in stmts)
                {
                    if (s is PilotIrImportStmt imp)
                        _module.Imports.Add(imp);
                    _module.Statements.Add(s);
                }
            }
            else if (leading.Count > 0)
            {
                foreach (var tr in leading.Where(t => t.Kind != PilotTriviaKind.BlankLine))
                {
                    _module.Statements.Add(new PilotIrCommentStmt
                    {
                        CommentText = tr.Text,
                        IsDocComment = tr.Kind == PilotTriviaKind.DocComment,
                        Line = tr.Line
                    });
                }
            }
        }
        CollectRemainingTrivia();
    }

    private List<PilotIrStmt> VisitSuite(PilotParser.SuiteContext? suite)
    {
        var list = new List<PilotIrStmt>();
        if (suite == null)
            return list;

        if (suite.simple_stmt() != null)
        {
            list.AddRange(VisitSimpleStmt(suite.simple_stmt()));
            return list;
        }

        foreach (var stmt in suite.stmt())
        {
            var leading = ConsumeTriviaBefore(stmt);
            var sub = VisitStmt(stmt);
            if (sub.Count > 0)
            {
                sub[0].LeadingTrivia.AddRange(leading);
                list.AddRange(sub);
            }
            else
            {
                foreach (var tr in leading.Where(t => t.Kind != PilotTriviaKind.BlankLine))
                {
                    list.Add(new PilotIrCommentStmt
                    {
                        CommentText = tr.Text,
                        IsDocComment = tr.Kind == PilotTriviaKind.DocComment,
                        Line = tr.Line
                    });
                }
            }
        }

        return list;
    }

    private List<PilotIrStmt> VisitStmt(PilotParser.StmtContext ctx)
    {
        List<PilotIrStmt> result;
        if (ctx.compound_stmt() != null)
        {
            var c = VisitCompoundStmt(ctx.compound_stmt());
            result = c != null ? new List<PilotIrStmt> { c } : new List<PilotIrStmt>();
        }
        else if (ctx.simple_stmt() != null)
        {
            result = VisitSimpleStmt(ctx.simple_stmt());
        }
        else
        {
            result = new List<PilotIrStmt>();
        }

        if (ctx.Stop != null && ctx.Stop.TokenIndex > _consumedTokenIndex)
        {
            _consumedTokenIndex = ctx.Stop.TokenIndex;
        }
        return result;
    }

    private List<PilotIrStmt> VisitSimpleStmt(PilotParser.Simple_stmtContext ctx)
    {
        var list = new List<PilotIrStmt>();
        foreach (var small in ctx.small_stmt())
        {
            var s = VisitSmallStmt(small);
            if (s != null)
                list.Add(s);
        }
        return list;
    }

    private PilotIrStmt? VisitSmallStmt(PilotParser.Small_stmtContext ctx)
    {
        int line = ctx.Start?.Line ?? 0;

        if (ctx.let_stmt() != null)
        {
            var letCtx = ctx.let_stmt();
            var (name, tupleNames) = ExtractBindingPattern(letCtx.binding_pattern());
            string? declType = FormatTypeSpec(letCtx.type_annotation()?.type_spec());
            PilotIrExpr? init = letCtx.test() != null ? VisitTest(letCtx.test()) : null;
            var resolved = ResolveVarType(name, declType, init);
            var stmt = new PilotIrVarDeclStmt
            {
                Line = line,
                Kind = PilotVarDeclKind.Let,
                Name = name,
                DeclaredType = declType,
                Initializer = init,
                IsMutated = _mutatedVars.Contains(name),
                ResolvedType = resolved
            };
            stmt.TupleDestructureNames.AddRange(tupleNames);
            CheckTypeFlags(declType, resolved);
            return stmt;
        }

        if (ctx.var_stmt() != null)
        {
            var varCtx = ctx.var_stmt();
            var (name, tupleNames) = ExtractBindingPattern(varCtx.binding_pattern());
            string? declType = FormatTypeSpec(varCtx.type_annotation()?.type_spec());
            PilotIrExpr? init = varCtx.test() != null ? VisitTest(varCtx.test()) : null;
            var resolved = ResolveVarType(name, declType, init);
            var stmt = new PilotIrVarDeclStmt
            {
                Line = line,
                Kind = PilotVarDeclKind.Var,
                Name = name,
                DeclaredType = declType,
                Initializer = init,
                IsMutated = true,
                ResolvedType = resolved
            };
            stmt.TupleDestructureNames.AddRange(tupleNames);
            CheckTypeFlags(declType, resolved);
            return stmt;
        }

        if (ctx.const_stmt() != null)
        {
            var constCtx = ctx.const_stmt();
            var (name, _) = ExtractBindingPattern(constCtx.binding_pattern());
            string? declType = FormatTypeSpec(constCtx.type_annotation()?.type_spec());
            PilotIrExpr init = VisitTest(constCtx.test());
            var resolved = ResolveVarType(name, declType, init);
            return new PilotIrVarDeclStmt
            {
                Line = line,
                Kind = PilotVarDeclKind.Const,
                Name = name,
                DeclaredType = declType,
                Initializer = init,
                IsMutated = false,
                ResolvedType = resolved
            };
        }

        if (ctx.type_alias_stmt() != null)
        {
            return VisitTypeAliasOrAdt(ctx.type_alias_stmt());
        }

        if (ctx.guard_stmt() != null)
        {
            return VisitGuardStmt(ctx.guard_stmt());
        }

        if (ctx.defer_stmt() != null)
        {
            var deferCtx = ctx.defer_stmt();
            var deferStmt = new PilotIrDeferStmt { Line = line };
            if (deferCtx.suite() != null)
                deferStmt.Body.AddRange(VisitSuite(deferCtx.suite()));
            else if (deferCtx.small_stmt() != null)
            {
                var inner = VisitSmallStmt(deferCtx.small_stmt());
                if (inner != null) deferStmt.Body.Add(inner);
            }
            else if (deferCtx.test() != null)
            {
                deferStmt.Body.Add(new PilotIrExprStmt { Line = line, Expression = VisitTest(deferCtx.test()) });
            }
            return deferStmt;
        }

        if (ctx.flow_stmt() != null)
        {
            var flow = ctx.flow_stmt();
            if (flow.RETURN() != null)
            {
                PilotIrExpr? val = flow.testlist() != null ? VisitTestlist(flow.testlist()) : null;
                return new PilotIrReturnStmt { Line = line, Value = val };
            }
            if (flow.YIELD() != null)
            {
                PilotIrExpr? val = flow.testlist() != null ? VisitTestlist(flow.testlist()) : null;
                return new PilotIrYieldStmt { Line = line, Value = val };
            }
            if (flow.RAISE() != null)
            {
                PilotIrExpr? val = flow.test() != null && flow.test().Length > 0 ? VisitTest(flow.test(0)) : null;
                return new PilotIrRaiseStmt { Line = line, Value = val };
            }
            if (flow.BREAK() != null)
            {
                PilotIrExpr? val = flow.test() != null && flow.test().Length > 0 ? VisitTest(flow.test(0)) : null;
                return new PilotIrBreakStmt { Line = line, Value = val };
            }
            if (flow.CONTINUE() != null)
            {
                PilotIrExpr? pivot = flow.range_expr() != null ? VisitRangeExpr(flow.range_expr()) : null;
                bool isSubLoop = flow.ELLIPSIS() != null;
                if (pivot != null)
                {
                    if (isSubLoop) _module.UsesRangeWindow = true;
                }
                return new PilotIrContinueStmt
                {
                    Line = line,
                    RangePivot = pivot,
                    IsSubLoopPrepend = isSubLoop
                };
            }
        }

        if (ctx.pass_stmt() != null)
            return new PilotIrPassStmt { Line = line };

        if (ctx.assert_stmt() != null)
        {
            var tests = ctx.assert_stmt().test();
            return new PilotIrAssertStmt
            {
                Line = line,
                IsComptime = ctx.assert_stmt().COMPTIME() != null,
                Condition = VisitTest(tests[0]),
                Message = tests.Length > 1 ? VisitTest(tests[1]) : null
            };
        }

        if (ctx.import_stmt() != null)
        {
            return VisitImportStmt(ctx.import_stmt());
        }

        if (ctx.annassign_stmt() != null)
        {
            var ann = ctx.annassign_stmt();
            var targetExpr = VisitTest(ann.test(0));
            string declType = FormatTypeSpec(ann.type_spec()) ?? "auto";
            PilotIrExpr? init = ann.test().Length > 1 ? VisitTest(ann.test(1)) : null;
            if (targetExpr is PilotIrIdentifierExpr id)
            {
                return new PilotIrVarDeclStmt
                {
                    Line = line,
                    Kind = _mutatedVars.Contains(id.Name) ? PilotVarDeclKind.Var : PilotVarDeclKind.Let,
                    Name = id.Name,
                    DeclaredType = declType,
                    Initializer = init,
                    IsMutated = _mutatedVars.Contains(id.Name),
                    ResolvedType = ResolveVarType(id.Name, declType, init)
                };
            }
            if (init != null)
            {
                return new PilotIrAssignStmt { Line = line, Target = targetExpr, Operator = "=", Value = init };
            }
        }

        if (ctx.expr_stmt() != null)
        {
            var exprStmt = ctx.expr_stmt();
            var lhs = VisitTestlist(exprStmt.testlist());
            if (exprStmt.augassign() != null && exprStmt.augassign().Length > 0 && exprStmt.test().Length > 0)
            {
                string op = exprStmt.augassign(0).GetText();
                var rhs = VisitTest(exprStmt.test(0));
                return new PilotIrAssignStmt { Line = line, Target = lhs, Operator = op, Value = rhs };
            }
            if ((exprStmt.EQUAL().Length > 0 || exprStmt.WALRUS().Length > 0) && exprStmt.test().Length > 0)
            {
                var rhs = VisitTest(exprStmt.test(0));
                return new PilotIrAssignStmt { Line = line, Target = lhs, Operator = "=", Value = rhs };
            }
            return new PilotIrExprStmt { Line = line, Expression = lhs };
        }

        return null;
    }

    private PilotIrStmt? VisitCompoundStmt(PilotParser.Compound_stmtContext ctx)
    {
        if (ctx.func_def() != null) return VisitFuncDef(ctx.func_def());
        if (ctx.class_def() != null) return VisitClassDef(ctx.class_def());
        if (ctx.struct_def() != null) return VisitStructDef(ctx.struct_def());
        if (ctx.record_def() != null) return VisitRecordDef(ctx.record_def());
        if (ctx.interface_def() != null) return VisitInterfaceDef(ctx.interface_def());
        if (ctx.enum_def() != null) return VisitEnumDef(ctx.enum_def());
        if (ctx.extension_def() != null) return VisitExtensionDef(ctx.extension_def());
        if (ctx.if_stmt() != null) return VisitIfStmt(ctx.if_stmt());
        if (ctx.while_stmt() != null) return VisitWhileStmt(ctx.while_stmt());
        if (ctx.for_stmt() != null) return VisitForStmt(ctx.for_stmt());
        if (ctx.match_stmt() != null) return VisitMatchStmt(ctx.match_stmt());
        if (ctx.try_stmt() != null) return VisitTryStmt(ctx.try_stmt());
        if (ctx.guard_stmt() != null) return VisitGuardStmt(ctx.guard_stmt());
        if (ctx.defer_stmt() != null)
        {
            var d = new PilotIrDeferStmt { Line = ctx.Start?.Line ?? 0 };
            if (ctx.defer_stmt().suite() != null)
                d.Body.AddRange(VisitSuite(ctx.defer_stmt().suite()));
            return d;
        }
        if (ctx.comptime_stmt() != null)
        {
            var ct = new PilotIrComptimeStmt { Line = ctx.Start?.Line ?? 0 };
            foreach (var s in ctx.comptime_stmt().stmt())
                ct.Body.AddRange(VisitStmt(s));
            if (ctx.comptime_stmt().if_stmt() != null)
                ct.Body.Add(VisitIfStmt(ctx.comptime_stmt().if_stmt()));
            if (ctx.comptime_stmt().for_stmt() != null)
                ct.Body.Add(VisitForStmt(ctx.comptime_stmt().for_stmt()));
            return ct;
        }
        return null;
    }

    private PilotIrFuncDefStmt VisitFuncDef(PilotParser.Func_defContext ctx)
    {
        string fnName = ctx.identifier().GetText();
        string? declRet = FormatTypeSpec(ctx.type_spec());
        string? inferredRet = _module.FunctionReturnTypes.TryGetValue(fnName, out var ir) ? ir : null;
        CheckTypeFlags(declRet, null);
        CheckTypeFlags(inferredRet, null);

        var fn = new PilotIrFuncDefStmt
        {
            Line = ctx.Start?.Line ?? 0,
            Name = fnName,
            DeclaredReturnType = declRet,
            InferredReturnType = inferredRet,
            IsAsync = ctx.ASYNC() != null,
            IsPure = ctx.PURE() != null,
            IsComptime = ctx.COMPTIME() != null,
            IsExpressionBody = ctx.FAT_ARROW() != null
        };

        if (ctx.decorators() != null)
        {
            foreach (var dec in ctx.decorators().decorator())
                fn.Decorators.Add(FormatDecorator(dec));
        }

        foreach (var mod in ctx.declaration_modifier())
        {
            if (mod.STATIC() != null) fn.IsStatic = true;
            if (mod.access_modifier() != null) fn.Visibility = mod.access_modifier().GetText();
        }

        if (ctx.generic_params() != null)
        {
            foreach (var gp in ctx.generic_params().generic_param())
                fn.GenericParams.Add(gp.identifier().GetText());
        }

        if (ctx.parameters()?.parameter() != null)
        {
            foreach (var p in ctx.parameters().parameter())
            {
                string pName = p.identifier()?.GetText() ?? "arg";
                string? pType = FormatTypeSpec(p.type_annotation()?.type_spec());
                CheckTypeFlags(pType, null);
                PilotIrExpr? defVal = p.test() != null ? VisitTest(p.test()) : null;
                fn.Parameters.Add(new PilotIrParameter(
                    pName,
                    pType,
                    defVal,
                    IsContext: p.CONTEXT() != null,
                    IsComptime: p.COMPTIME() != null,
                    IsMutable: p.REF() != null,
                    IsVariadic: p.PARAMS() != null || p.STAR() != null));
            }
        }

        foreach (var contract in ctx.contract_clause())
        {
            if (contract.REQUIRES() != null)
                fn.RequiresClauses.Add(VisitTest(contract.test()));
            else if (contract.ENSURES() != null)
                fn.EnsuresClauses.Add(VisitTest(contract.test()));
        }

        if (ctx.suite() != null)
        {
            var localMutations = PilotOptimizer.AnalyzeMutatedVariables(ctx.suite());
            foreach (var m in localMutations)
                fn.MutatedLocals.Add(m);
            fn.Body.AddRange(VisitSuite(ctx.suite()));
        }

        return fn;
    }

    private PilotIrClassDefStmt VisitClassDef(PilotParser.Class_defContext ctx)
    {
        var cls = new PilotIrClassDefStmt
        {
            Line = ctx.Start?.Line ?? 0,
            Kind = PilotTypeDeclKind.Class,
            Name = ctx.identifier().GetText()
        };

        if (ctx.decorators() != null)
        {
            foreach (var dec in ctx.decorators().decorator())
                cls.Decorators.Add(FormatDecorator(dec));
        }

        if (ctx.generic_params() != null)
        {
            foreach (var gp in ctx.generic_params().generic_param())
                cls.GenericParams.Add(gp.identifier().GetText());
        }

        if (ctx.class_bases() != null)
        {
            foreach (var b in ctx.class_bases().type_spec())
                cls.BaseTypes.Add(FormatTypeSpec(b) ?? "object");
        }

        if (ctx.suite() != null)
        {
            PopulateTypeMembersFromSuite(cls, ctx.suite());
        }

        return cls;
    }

    private PilotIrClassDefStmt VisitStructDef(PilotParser.Struct_defContext ctx)
    {
        var st = new PilotIrClassDefStmt
        {
            Line = ctx.Start?.Line ?? 0,
            Kind = PilotTypeDeclKind.Struct,
            Name = ctx.identifier().GetText()
        };

        if (ctx.struct_member() != null)
        {
            foreach (var m in ctx.struct_member())
            {
                string fName = m.identifier().GetText();
                string fType = FormatTypeSpec(m.type_annotation()?.type_spec()) ?? "i64";
                PilotIrExpr? defVal = m.test() != null ? VisitTest(m.test()) : null;
                st.Fields.Add(new PilotIrFieldDecl(fName, fType, defVal, IsMutable: m.CONST() == null && m.LET() == null));
            }
        }

        if (ctx.suite() != null)
        {
            PopulateTypeMembersFromSuite(st, ctx.suite());
        }

        return st;
    }

    private PilotIrClassDefStmt VisitRecordDef(PilotParser.Record_defContext ctx)
    {
        var rec = new PilotIrClassDefStmt
        {
            Line = ctx.Start?.Line ?? 0,
            Kind = PilotTypeDeclKind.Record,
            Name = ctx.identifier().GetText()
        };

        if (ctx.adt_field_list() != null)
        {
            int idx = 0;
            foreach (var af in ctx.adt_field_list().adt_field())
            {
                string fName = af.identifier()?.GetText() ?? $"item{idx++}";
                string fType = FormatTypeSpec(af.type_spec()) ?? "any";
                PilotIrExpr? defVal = af.test() != null ? VisitTest(af.test()) : null;
                rec.Fields.Add(new PilotIrFieldDecl(fName, fType, defVal, IsMutable: false));
            }
        }

        if (ctx.struct_member() != null)
        {
            foreach (var m in ctx.struct_member())
            {
                string fName = m.identifier().GetText();
                string fType = FormatTypeSpec(m.type_annotation()?.type_spec()) ?? "any";
                PilotIrExpr? defVal = m.test() != null ? VisitTest(m.test()) : null;
                rec.Fields.Add(new PilotIrFieldDecl(fName, fType, defVal, IsMutable: false));
            }
        }

        return rec;
    }

    private PilotIrClassDefStmt VisitInterfaceDef(PilotParser.Interface_defContext ctx)
    {
        var iface = new PilotIrClassDefStmt
        {
            Line = ctx.Start?.Line ?? 0,
            Kind = PilotTypeDeclKind.Interface,
            Name = ctx.identifier().GetText()
        };
        if (ctx.suite() != null)
            PopulateTypeMembersFromSuite(iface, ctx.suite());
        return iface;
    }

    private void PopulateTypeMembersFromSuite(PilotIrClassDefStmt target, PilotParser.SuiteContext suite)
    {
        var stmts = VisitSuite(suite);
        foreach (var s in stmts)
        {
            if (s is PilotIrFuncDefStmt fn)
            {
                target.Methods.Add(fn);
            }
            else if (s is PilotIrVarDeclStmt vd)
            {
                target.Fields.Add(new PilotIrFieldDecl(
                    vd.Name,
                    vd.DeclaredType ?? "any",
                    vd.Initializer,
                    IsMutable: vd.Kind == PilotVarDeclKind.Var));
            }
        }
    }

    private PilotIrStmt VisitTypeAliasOrAdt(PilotParser.Type_alias_stmtContext ctx)
    {
        string name = ctx.identifier().GetText();
        int line = ctx.Start?.Line ?? 0;

        if (ctx.adt_sum_body() != null)
        {
            var adt = new PilotIrAdtDefStmt { Line = line, Name = name };
            if (ctx.generic_params() != null)
            {
                foreach (var gp in ctx.generic_params().generic_param())
                    adt.GenericParams.Add(gp.identifier().GetText());
            }

            var sumBody = ctx.adt_sum_body();
            foreach (var ident in sumBody.identifier())
            {
                adt.Variants.Add(new PilotIrAdtVariant(ident.GetText(), Array.Empty<(string, string)>()));
            }
            if (sumBody.adt_variant_payload() != null)
            {
                adt.Variants.Add(ParseAdtVariantPayload(sumBody.adt_variant_payload()));
            }
            foreach (var v in sumBody.adt_variant())
            {
                adt.Variants.Add(ParseAdtVariant(v));
            }
            return adt;
        }

        var alias = new PilotIrTypeAliasStmt
        {
            Line = line,
            Name = name,
            TargetType = FormatTypeSpec(ctx.type_spec()) ?? "any"
        };
        if (ctx.generic_params() != null)
        {
            foreach (var gp in ctx.generic_params().generic_param())
                alias.GenericParams.Add(gp.identifier().GetText());
        }
        return alias;
    }

    private PilotIrAdtVariant ParseAdtVariant(PilotParser.Adt_variantContext v)
    {
        string vName = v.identifier().GetText();
        var fields = new List<(string Name, string Type)>();
        if (v.adt_field_list() != null)
        {
            int idx = 0;
            foreach (var f in v.adt_field_list().adt_field())
            {
                string fName = f.identifier()?.GetText() ?? $"v{idx++}";
                string fType = FormatTypeSpec(f.type_spec()) ?? "any";
                fields.Add((fName, fType));
            }
        }
        else if (v.struct_member() != null)
        {
            foreach (var sm in v.struct_member())
            {
                string fName = sm.identifier().GetText();
                string fType = FormatTypeSpec(sm.type_annotation()?.type_spec()) ?? "any";
                fields.Add((fName, fType));
            }
        }
        return new PilotIrAdtVariant(vName, fields);
    }

    private PilotIrAdtVariant ParseAdtVariantPayload(PilotParser.Adt_variant_payloadContext v)
    {
        string vName = v.identifier().GetText();
        var fields = new List<(string Name, string Type)>();
        if (v.adt_field_list() != null)
        {
            int idx = 0;
            foreach (var f in v.adt_field_list().adt_field())
            {
                string fName = f.identifier()?.GetText() ?? $"v{idx++}";
                string fType = FormatTypeSpec(f.type_spec()) ?? "any";
                fields.Add((fName, fType));
            }
        }
        else if (v.struct_member() != null)
        {
            foreach (var sm in v.struct_member())
            {
                string fName = sm.identifier().GetText();
                string fType = FormatTypeSpec(sm.type_annotation()?.type_spec()) ?? "any";
                fields.Add((fName, fType));
            }
        }
        return new PilotIrAdtVariant(vName, fields);
    }

    private PilotIrStmt VisitEnumDef(PilotParser.Enum_defContext ctx)
    {
        string name = ctx.identifier().GetText();
        int line = ctx.Start?.Line ?? 0;
        var members = ctx.enum_suite()?.enum_member() ?? Array.Empty<PilotParser.Enum_memberContext>();

        // If any enum member has payload fields, treat it as an ADT!
        if (members.Any(m => m.adt_field_list() != null || (m.struct_member() != null && m.struct_member().Length > 0)))
        {
            var adt = new PilotIrAdtDefStmt { Line = line, Name = name };
            foreach (var m in members)
            {
                var fields = new List<(string Name, string Type)>();
                if (m.adt_field_list() != null)
                {
                    int idx = 0;
                    foreach (var f in m.adt_field_list().adt_field())
                    {
                        fields.Add((f.identifier()?.GetText() ?? $"v{idx++}", FormatTypeSpec(f.type_spec()) ?? "any"));
                    }
                }
                adt.Variants.Add(new PilotIrAdtVariant(m.identifier().GetText(), fields));
            }
            return adt;
        }

        var en = new PilotIrEnumDefStmt { Line = line, Name = name };
        foreach (var m in members)
        {
            en.Members.Add((m.identifier().GetText(), m.test() != null ? VisitTest(m.test()) : null));
        }
        return en;
    }

    private PilotIrExtensionDefStmt VisitExtensionDef(PilotParser.Extension_defContext ctx)
    {
        var ext = new PilotIrExtensionDefStmt
        {
            Line = ctx.Start?.Line ?? 0,
            ExtensionName = ctx.identifier()?.GetText(),
            TargetType = FormatTypeSpec(ctx.type_spec()) ?? "any"
        };

        var stmts = VisitSuite(ctx.suite());
        foreach (var s in stmts.OfType<PilotIrFuncDefStmt>())
        {
            ext.Methods.Add(s);
        }
        return ext;
    }

    private PilotIrIfStmt VisitIfStmt(PilotParser.If_stmtContext ctx)
    {
        var tests = ctx.test();
        var suites = ctx.suite();
        var patterns = ctx.pattern();

        var ifStmt = new PilotIrIfStmt
        {
            Line = ctx.Start?.Line ?? 0,
            Condition = tests.Length > 0 ? VisitTest(tests[0]) : new PilotIrLiteralExpr { Kind = PilotLiteralKind.Boolean, RawValue = "true" }
        };

        if (patterns != null && patterns.Length > 0)
        {
            ifStmt.OptionalBindingVar = patterns[0].GetText();
            _module.UsesOptionTypes = true;
        }

        if (suites.Length > 0)
            ifStmt.ThenBody.AddRange(VisitSuite(suites[0]));

        int elifCount = tests.Length - 1;
        for (int i = 0; i < elifCount; i++)
        {
            var cond = VisitTest(tests[i + 1]);
            var body = i + 1 < suites.Length ? VisitSuite(suites[i + 1]) : new List<PilotIrStmt>();
            ifStmt.ElifBranches.Add((cond, body));
        }

        if (ctx.ELSE() != null && suites.Length > tests.Length)
        {
            ifStmt.ElseBody.AddRange(VisitSuite(suites[^1]));
        }

        return ifStmt;
    }

    private PilotIrGuardStmt VisitGuardStmt(PilotParser.Guard_stmtContext ctx)
    {
        var testNode = ctx.test();
        var suiteNode = ctx.suite();
        bool isSome = ctx.SOME() != null || ctx.LET() != null || ctx.VAR() != null;
        string? boundName = ctx.pattern()?.GetText();

        PilotIrExpr condOrSource = testNode != null
            ? VisitTest(testNode)
            : new PilotIrLiteralExpr { Kind = PilotLiteralKind.Boolean, RawValue = "true" };

        if (isSome && string.IsNullOrEmpty(boundName) && condOrSource is PilotIrIdentifierExpr idExpr)
        {
            boundName = idExpr.Name;
        }
        if (isSome)
            _module.UsesOptionTypes = true;

        var guardStmt = new PilotIrGuardStmt
        {
            Line = ctx.Start?.Line ?? 0,
            IsSomeBinding = isSome,
            BoundVariable = boundName,
            ConditionOrSource = condOrSource,
            PatternGuard = ctx.pattern_guard()?.test() != null ? VisitTest(ctx.pattern_guard().test()) : null,
            CatchErrorVar = ctx.identifier()?.GetText()
        };

        if (suiteNode != null)
            guardStmt.ElseBody.AddRange(VisitSuite(suiteNode));

        return guardStmt;
    }

    private PilotIrWhileStmt VisitWhileStmt(PilotParser.While_stmtContext ctx)
    {
        var w = new PilotIrWhileStmt
        {
            Line = ctx.Start?.Line ?? 0,
            Condition = VisitTest(ctx.test(0))
        };
        if (ctx.suite().Length > 0)
            w.Body.AddRange(VisitSuite(ctx.suite(0)));
        return w;
    }

    private PilotIrForStmt VisitForStmt(PilotParser.For_stmtContext ctx)
    {
        var f = new PilotIrForStmt
        {
            Line = ctx.Start?.Line ?? 0,
            Iterable = VisitTest(ctx.test(0))
        };

        var targetCtx = ctx.for_target();
        if (targetCtx.binding_pattern() != null && targetCtx.binding_pattern().Length > 0)
        {
            foreach (var bp in targetCtx.binding_pattern())
            {
                var (name, tupleNames) = ExtractBindingPattern(bp);
                if (tupleNames.Count > 0)
                    f.TargetVars.AddRange(tupleNames);
                else
                    f.TargetVars.Add(name);
            }
        }
        else if (targetCtx.exprlist() != null)
        {
            foreach (var t in targetCtx.exprlist().test())
                f.TargetVars.Add(t.GetText());
        }

        if (_analysis != null && _analysis.LoopInfos.TryGetValue(ctx, out var loopInfo))
        {
            bool usesSlice = loopInfo.ReturnMode == LoopReturnMode.CollectionYield;
            bool usesWindow = loopInfo.ScheduleMode != LoopScheduleMode.FixedScalar;
            if (usesSlice) _module.UsesSliceBuffer = true;
            if (usesWindow) _module.UsesRangeWindow = true;
            f.LoopLayout = new PilotCAbiLoopLayout(
                loopInfo.ScheduleMode,
                loopInfo.ReturnMode,
                "i64",
                usesSlice,
                usesWindow,
                24);
        }

        var suites = ctx.suite();
        if (suites.Length > 0)
            f.Body.AddRange(VisitSuite(suites[0]));
        if (suites.Length > 1)
            f.CompletionThenBody.AddRange(VisitSuite(suites[1]));

        return f;
    }

    private PilotIrMatchStmt VisitMatchStmt(PilotParser.Match_stmtContext ctx)
    {
        var m = new PilotIrMatchStmt
        {
            Line = ctx.Start?.Line ?? 0,
            Scrutinee = ctx.test() != null ? VisitTest(ctx.test()) : new PilotIrLiteralExpr { Kind = PilotLiteralKind.Boolean, RawValue = "true" }
        };

        if (ctx.match_suite()?.match_arm() != null)
        {
            foreach (var armCtx in ctx.match_suite().match_arm())
            {
                var pat = VisitPattern(armCtx.pattern());
                PilotIrExpr? guard = armCtx.test() != null ? VisitTest(armCtx.test()) : null;
                var body = VisitSuite(armCtx.suite());
                PilotIrExpr? exprBody = body.Count == 1 && body[0] is PilotIrExprStmt es ? es.Expression : null;
                m.Arms.Add(new PilotIrMatchArm(pat, guard, body, exprBody));
            }
        }

        return m;
    }

    private PilotIrTryStmt VisitTryStmt(PilotParser.Try_stmtContext ctx)
    {
        var tr = new PilotIrTryStmt { Line = ctx.Start?.Line ?? 0 };
        var suites = ctx.suite();
        if (suites.Length > 0)
            tr.TryBody.AddRange(VisitSuite(suites[0]));

        foreach (var ex in ctx.except_clause())
        {
            string? exType = FormatTypeSpec(ex.type_spec());
            string? exVar = ex.identifier()?.GetText();
            var body = VisitSuite(ex.suite());
            tr.Catches.Add(new PilotIrCatchClause(exType, exVar, body));
        }

        if (ctx.FINALLY() != null && suites.Length > 1)
        {
            tr.FinallyBody.AddRange(VisitSuite(suites[^1]));
        }

        return tr;
    }

    private PilotIrImportStmt VisitImportStmt(PilotParser.Import_stmtContext ctx)
    {
        int line = ctx.Start?.Line ?? 0;
        if (ctx.foreign_import_stmt() != null)
        {
            var fi = ctx.foreign_import_stmt();
            string lang = fi.foreign_target().foreign_language()?.GetText()
                          ?? fi.foreign_target().identifier()?.GetText()
                          ?? "extern";
            string modPath = fi.foreign_target().dotted_name()?.GetText()
                             ?? fi.foreign_target().STRING()?.GetText().Trim('"', '\'')
                             ?? "";
            return new PilotIrImportStmt
            {
                Line = line,
                IsForeignImport = true,
                ForeignLanguage = lang,
                ModulePath = modPath,
                Alias = fi.identifier()?.GetText()
            };
        }

        if (ctx.import_name() != null)
        {
            var first = ctx.import_name().dotted_as_names().dotted_as_name(0);
            return new PilotIrImportStmt
            {
                Line = line,
                ModulePath = first.dotted_name().GetText(),
                Alias = first.identifier()?.GetText()
            };
        }

        if (ctx.import_from() != null)
        {
            var inf = ctx.import_from();
            var imp = new PilotIrImportStmt
            {
                Line = line,
                ModulePath = inf.dotted_name()?.GetText() ?? "."
            };
            if (inf.import_as_names() != null)
            {
                foreach (var item in inf.import_as_names().import_as_name())
                {
                    imp.ImportedItems.Add((item.identifier(0).GetText(), item.identifier().Length > 1 ? item.identifier(1).GetText() : null));
                }
            }
            return imp;
        }

        return new PilotIrImportStmt { Line = line };
    }

    // ========================================================================
    // EXPRESSION LOWERING (ZERO RAW ctx.GetText() ON COMPOUND NODES)
    // ========================================================================

    private PilotIrExpr VisitTestlist(PilotParser.TestlistContext ctx)
    {
        var tests = ctx.test();
        if (tests.Length == 1 && ctx.COMMA().Length == 0)
            return VisitTest(tests[0]);

        var tuple = new PilotIrCollectionExpr { Line = ctx.Start?.Line ?? 0, Kind = PilotCollectionKind.Tuple };
        foreach (var t in tests)
            tuple.Elements.Add(VisitTest(t));
        return tuple;
    }

    private PilotIrExpr VisitTest(PilotParser.TestContext ctx)
    {
        if (ctx.lambda_expr() != null)
            return VisitLambdaExpr(ctx.lambda_expr());

        var pipeExprs = ctx.pipe_expr();
        if (ctx.IF() != null && pipeExprs.Length >= 2 && ctx.test() != null)
        {
            return new PilotIrIfExpr
            {
                Line = ctx.Start?.Line ?? 0,
                ThenBranch = VisitPipeExpr(pipeExprs[0]),
                Condition = VisitPipeExpr(pipeExprs[1]),
                ElseBranch = VisitTest(ctx.test())
            };
        }

        return VisitPipeExpr(pipeExprs[0]);
    }

    private PilotIrExpr VisitLambdaExpr(PilotParser.Lambda_exprContext ctx)
    {
        var lam = new PilotIrLambdaExpr { Line = ctx.Start?.Line ?? 0 };
        if (ctx.identifier() != null)
        {
            lam.Parameters.Add((ctx.identifier().GetText(), null));
        }
        else if (ctx.parameters()?.parameter() != null)
        {
            foreach (var p in ctx.parameters().parameter())
                lam.Parameters.Add((p.identifier()?.GetText() ?? "x", FormatTypeSpec(p.type_annotation()?.type_spec())));
        }
        else if (ctx.lambda_params()?.lambda_param() != null)
        {
            foreach (var p in ctx.lambda_params().lambda_param())
                lam.Parameters.Add((p.identifier().GetText(), FormatTypeSpec(p.type_annotation()?.type_spec())));
        }

        if (ctx.test() != null)
            lam.BodyExpr = VisitTest(ctx.test());
        else if (ctx.lambda_body()?.test() != null)
            lam.BodyExpr = VisitTest(ctx.lambda_body().test());
        else if (ctx.stmt() != null)
        {
            foreach (var s in ctx.stmt())
                lam.BodyStmts.AddRange(VisitStmt(s));
        }
        return lam;
    }

    private PilotIrExpr VisitPipeExpr(PilotParser.Pipe_exprContext ctx)
    {
        var items = ctx.set_query_expr();
        var expr = VisitSetQueryExpr(items[0]);
        for (int i = 1; i < items.Length; i++)
        {
            string op = ctx.GetChild(2 * i - 1).GetText();
            var right = VisitSetQueryExpr(items[i]);
            expr = new PilotIrBinaryExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Operator = op,
                Left = expr,
                Right = right
            };
        }
        return expr;
    }

    private PilotIrExpr VisitSetQueryExpr(PilotParser.Set_query_exprContext ctx)
    {
        if (ctx.query_expr() != null && ctx.query_expr().Length > 0)
            return VisitQueryExpr(ctx.query_expr(0));

        var items = ctx.coalesce_expr();
        if (items == null || items.Length == 0)
            return new PilotIrLiteralExpr { Kind = PilotLiteralKind.Null, RawValue = "null" };

        var expr = VisitCoalesceExpr(items[0]);
        for (int i = 1; i < items.Length; i++)
        {
            string op = ctx.set_op(i - 1).GetText();
            expr = new PilotIrBinaryExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Operator = op,
                Left = expr,
                Right = VisitCoalesceExpr(items[i])
            };
        }
        return expr;
    }

    private PilotIrExpr VisitQueryExpr(PilotParser.Query_exprContext ctx)
    {
        string rangeVar = ctx.from_clause().identifier() != null && ctx.from_clause().identifier().Length > 0
            ? ctx.from_clause().identifier(0).GetText()
            : "row";
        var q = new PilotIrSqlQueryExpr
        {
            Line = ctx.Start?.Line ?? 0,
            RangeVariable = rangeVar,
            Source = VisitTest(ctx.from_clause().test()),
            SelectProjection = ctx.select_clause()?.select_projection()?.select_item(0)?.test() != null
                ? VisitTest(ctx.select_clause().select_projection().select_item(0).test())
                : new PilotIrIdentifierExpr { Name = rangeVar }
        };
        if (ctx.where_clause()?.test() != null)
            q.WherePredicate = VisitTest(ctx.where_clause().test());
        if (ctx.order_clause()?.ordering_term(0)?.test() != null)
        {
            q.OrderByKey = VisitTest(ctx.order_clause().ordering_term(0).test());
            q.OrderDescending = ctx.order_clause().ordering_term(0).DESC() != null || ctx.order_clause().ordering_term(0).DESCENDING() != null;
        }
        if (ctx.limit_clause()?.test(0) != null)
            q.LimitCount = VisitTest(ctx.limit_clause().test(0));
        return q;
    }

    private PilotIrExpr VisitCoalesceExpr(PilotParser.Coalesce_exprContext ctx)
    {
        var orTests = ctx.or_test();
        var expr = VisitOrTest(orTests[0]);
        int orIdx = 1;
        int catchIdx = 0;

        for (int i = 1; i < ctx.ChildCount; i++)
        {
            var child = ctx.GetChild(i);
            if (child.GetText() == "??" && orIdx < orTests.Length)
            {
                _module.UsesOptionTypes = true;
                expr = new PilotIrBinaryExpr
                {
                    Line = ctx.Start?.Line ?? 0,
                    Operator = "??",
                    Left = expr,
                    Right = VisitOrTest(orTests[orIdx++])
                };
            }
            else if (child is PilotParser.Catch_clauseContext catchCtx)
            {
                _module.UsesResultTypes = true;
                var tryExpr = new PilotIrPostfixTryExpr
                {
                    Line = ctx.Start?.Line ?? 0,
                    Operand = expr,
                    IsQuestionPropagate = false,
                    CatchVarName = catchCtx.identifier()?.GetText()
                };
                if (catchCtx.test() != null)
                    tryExpr.CatchFallbackExpr = VisitTest(catchCtx.test());
                else if (catchCtx.or_test() != null)
                    tryExpr.CatchFallbackExpr = VisitOrTest(catchCtx.or_test());
                else if (catchCtx.block() != null)
                {
                    foreach (var s in catchCtx.block().stmt())
                        tryExpr.CatchFallbackBody.AddRange(VisitStmt(s));
                }
                expr = tryExpr;
                catchIdx++;
            }
        }

        return expr;
    }

    private PilotIrExpr VisitOrTest(PilotParser.Or_testContext ctx)
    {
        var items = ctx.and_test();
        var expr = VisitAndTest(items[0]);
        for (int i = 1; i < items.Length; i++)
        {
            expr = new PilotIrBinaryExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Operator = "or",
                Left = expr,
                Right = VisitAndTest(items[i])
            };
        }
        return expr;
    }

    private PilotIrExpr VisitAndTest(PilotParser.And_testContext ctx)
    {
        var items = ctx.not_test();
        var expr = VisitNotTest(items[0]);
        for (int i = 1; i < items.Length; i++)
        {
            expr = new PilotIrBinaryExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Operator = "and",
                Left = expr,
                Right = VisitNotTest(items[i])
            };
        }
        return expr;
    }

    private PilotIrExpr VisitNotTest(PilotParser.Not_testContext ctx)
    {
        if (ctx.not_test() != null)
        {
            return new PilotIrUnaryExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Operator = "not",
                Operand = VisitNotTest(ctx.not_test())
            };
        }
        return VisitComparison(ctx.comparison());
    }

    private PilotIrExpr VisitComparison(PilotParser.ComparisonContext ctx)
    {
        var ranges = ctx.range_expr();
        var expr = VisitRangeExpr(ranges[0]);
        var compOps = ctx.comp_op();

        for (int i = 0; i < compOps.Length && i + 1 < ranges.Length; i++)
        {
            string opText = compOps[i].NOT() != null && compOps[i].IN() != null
                ? "not in"
                : compOps[i].GetText();
            expr = new PilotIrBinaryExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Operator = opText,
                Left = expr,
                Right = VisitRangeExpr(ranges[i + 1])
            };
        }
        return expr;
    }

    private PilotIrExpr VisitRangeExpr(PilotParser.Range_exprContext ctx)
    {
        var bitors = ctx.bitor_expr();
        var left = VisitBitorExpr(bitors[0]);
        if (bitors.Length > 1)
        {
            bool inclusive = ctx.DOTDOT_LESS() == null;
            return new PilotIrRangeExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Start = left,
                End = VisitBitorExpr(bitors[1]),
                IsInclusive = inclusive
            };
        }
        return left;
    }

    private PilotIrExpr VisitBitorExpr(PilotParser.Bitor_exprContext ctx)
    {
        var items = ctx.bitxor_expr();
        var expr = VisitBitxorExpr(items[0]);
        for (int i = 1; i < items.Length; i++)
        {
            expr = new PilotIrBinaryExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Operator = "|",
                Left = expr,
                Right = VisitBitxorExpr(items[i])
            };
        }
        return expr;
    }

    private PilotIrExpr VisitBitxorExpr(PilotParser.Bitxor_exprContext ctx)
    {
        var items = ctx.bitand_expr();
        var expr = VisitBitandExpr(items[0]);
        for (int i = 1; i < items.Length; i++)
        {
            expr = new PilotIrBinaryExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Operator = "^",
                Left = expr,
                Right = VisitBitandExpr(items[i])
            };
        }
        return expr;
    }

    private PilotIrExpr VisitBitandExpr(PilotParser.Bitand_exprContext ctx)
    {
        var items = ctx.shift_expr();
        var expr = VisitShiftExpr(items[0]);
        for (int i = 1; i < items.Length; i++)
        {
            expr = new PilotIrBinaryExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Operator = "&",
                Left = expr,
                Right = VisitShiftExpr(items[i])
            };
        }
        return expr;
    }

    private PilotIrExpr VisitShiftExpr(PilotParser.Shift_exprContext ctx)
    {
        var items = ctx.arith_expr();
        var expr = VisitArithExpr(items[0]);
        for (int i = 1; i < items.Length; i++)
        {
            string op = ctx.GetChild(2 * i - 1).GetText();
            expr = new PilotIrBinaryExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Operator = op,
                Left = expr,
                Right = VisitArithExpr(items[i])
            };
        }
        return expr;
    }

    private PilotIrExpr VisitArithExpr(PilotParser.Arith_exprContext ctx)
    {
        var items = ctx.term();
        var expr = VisitTerm(items[0]);
        for (int i = 1; i < items.Length; i++)
        {
            string op = ctx.GetChild(2 * i - 1).GetText();
            expr = new PilotIrBinaryExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Operator = op,
                Left = expr,
                Right = VisitTerm(items[i])
            };
        }
        return expr;
    }

    private PilotIrExpr VisitTerm(PilotParser.TermContext ctx)
    {
        var items = ctx.factor();
        var expr = VisitFactor(items[0]);
        for (int i = 1; i < items.Length; i++)
        {
            string op = ctx.GetChild(2 * i - 1).GetText();
            expr = new PilotIrBinaryExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Operator = op,
                Left = expr,
                Right = VisitFactor(items[i])
            };
        }
        return expr;
    }

    private PilotIrExpr VisitFactor(PilotParser.FactorContext ctx)
    {
        if (ctx.factor() != null)
        {
            string op = ctx.GetChild(0).GetText();
            return new PilotIrUnaryExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Operator = op,
                Operand = VisitFactor(ctx.factor())
            };
        }
        if (ctx.unary_op() != null)
        {
            var u = ctx.unary_op();
            if (u.AWAIT() != null)
                return new PilotIrUnaryExpr { Line = ctx.Start?.Line ?? 0, Operator = "await", Operand = VisitFactor(u.factor()) };
            if (u.SPAWN() != null)
                return new PilotIrUnaryExpr { Line = ctx.Start?.Line ?? 0, Operator = "spawn", Operand = VisitFactor(u.factor()) };
        }
        return VisitPower(ctx.power());
    }

    private PilotIrExpr VisitPower(PilotParser.PowerContext ctx)
    {
        var left = VisitAtomExpr(ctx.atom_expr());
        if (ctx.factor() != null)
        {
            return new PilotIrBinaryExpr
            {
                Line = ctx.Start?.Line ?? 0,
                Operator = "**",
                Left = left,
                Right = VisitFactor(ctx.factor())
            };
        }
        return left;
    }

    private PilotIrExpr VisitAtomExpr(PilotParser.Atom_exprContext ctx)
    {
        var expr = VisitAtom(ctx.atom());
        foreach (var tr in ctx.trailer())
        {
            int line = tr.Start?.Line ?? 0;
            if (tr.LPAR() != null)
            {
                var call = new PilotIrCallExpr
                {
                    Line = line,
                    Callee = expr,
                    QuoteOperator = tr.call_quote_op()?.GetText()
                };
                if (tr.arglist()?.argument() != null)
                {
                    foreach (var arg in tr.arglist().argument())
                    {
                        string? argName = arg.identifier()?.GetText();
                        var argVal = arg.test() != null ? VisitTest(arg.test()) : new PilotIrLiteralExpr { Kind = PilotLiteralKind.Null, RawValue = "null" };
                        string? mod = arg.REF() != null ? "ref" : (arg.EACH() != null ? "each" : null);
                        call.Arguments.Add(new PilotIrCallArg(argName, argVal, mod));
                    }
                }
                ClassifyLibraryCall(call);
                expr = call;
            }
            else if (tr.DOT() != null && tr.identifier() != null)
            {
                expr = new PilotIrMemberAccessExpr
                {
                    Line = line,
                    Target = expr,
                    MemberName = tr.identifier().GetText(),
                    IsSafeNavigation = false
                };
            }
            else if (tr.SAFE_NAV() != null && tr.identifier() != null)
            {
                _module.UsesOptionTypes = true;
                expr = new PilotIrMemberAccessExpr
                {
                    Line = line,
                    Target = expr,
                    MemberName = tr.identifier().GetText(),
                    IsSafeNavigation = true
                };
            }
            else if (tr.LSQB() != null && tr.subscriptlist() != null)
            {
                var sub = tr.subscriptlist().subscript(0);
                var tests = sub.test();
                bool isSlice = sub.DOTDOT() != null || sub.DOTDOT_EQUAL() != null;
                expr = new PilotIrIndexExpr
                {
                    Line = line,
                    Target = expr,
                    Index = tests.Length > 0 ? VisitTest(tests[0]) : new PilotIrLiteralExpr { Kind = PilotLiteralKind.Integer, RawValue = "0" },
                    SliceEnd = tests.Length > 1 ? VisitTest(tests[1]) : null,
                    IsSlice = isSlice,
                    IsSafeNavigation = tr.SAFE_NAV() != null || tr.QUESTION() != null
                };
            }
            else if (tr.QUESTION() != null && tr.LSQB() == null && tr.LPAR() == null)
            {
                _module.UsesResultTypes = true;
                expr = new PilotIrPostfixTryExpr
                {
                    Line = line,
                    Operand = expr,
                    IsQuestionPropagate = true
                };
            }
        }
        return expr;
    }

    private PilotIrExpr VisitAtom(PilotParser.AtomContext ctx)
    {
        int line = ctx.Start?.Line ?? 0;
        if (ctx.literal() != null)
            return VisitLiteral(ctx.literal());

        if (ctx.identifier() != null && ctx.DOT() == null)
        {
            string id = ctx.identifier().GetText();
            var idExpr = new PilotIrIdentifierExpr { Line = line, Name = id };
            if (_module.SymbolTypes.TryGetValue(id, out var t))
                idExpr.ResolvedType = t;
            return idExpr;
        }

        if (ctx.symbol_literal() != null)
        {
            return new PilotIrLiteralExpr
            {
                Line = line,
                Kind = PilotLiteralKind.Keyword,
                RawValue = ctx.symbol_literal().GetText(),
                StringContent = ctx.symbol_literal().GetText().TrimStart(':')
            };
        }

        if (ctx.collection_expr() != null)
        {
            var coll = ctx.collection_expr();
            var elems = coll.collection_element();
            if (coll.comp_for() != null && elems.Length > 0)
            {
                _module.UsesSliceBuffer = true;
                var cf = coll.comp_for();
                var forExpr = new PilotIrForExpr
                {
                    Line = line,
                    YieldExpr = VisitTest(elems[0].test()),
                    Iterable = VisitTest(cf.test())
                };
                foreach (var t in cf.exprlist().test())
                    forExpr.TargetVars.Add(t.GetText());
                if (cf.comp_iter()?.comp_if()?.test() != null)
                    forExpr.FilterCondition = VisitTest(cf.comp_iter().comp_if().test());
                return forExpr;
            }

            var listExpr = new PilotIrCollectionExpr { Line = line, Kind = PilotCollectionKind.List };
            foreach (var el in elems)
                listExpr.Elements.Add(VisitTest(el.test()));
            return listExpr;
        }

        if (ctx.LPAR() != null && ctx.testlist_comp() != null)
        {
            var tc = ctx.testlist_comp();
            var tests = tc.test();
            if (tests.Length == 1 && tc.COMMA().Length == 0 && tc.comp_for() == null)
            {
                // Parenthesized sub-expression: precedence is handled structurally by PilotIrBinaryExpr!
                return VisitTest(tests[0]);
            }
            var tup = new PilotIrCollectionExpr { Line = line, Kind = PilotCollectionKind.Tuple };
            foreach (var t in tests)
                tup.Elements.Add(VisitTest(t));
            return tup;
        }

        if (ctx.LBRACE() != null)
        {
            var ds = ctx.dictorsetmaker();
            if (ds == null)
                return new PilotIrCollectionExpr { Line = line, Kind = PilotCollectionKind.Map };

            if (ds.dict_entry() != null && ds.dict_entry().Length > 0)
            {
                var map = new PilotIrCollectionExpr { Line = line, Kind = PilotCollectionKind.Map };
                foreach (var de in ds.dict_entry())
                {
                    var kv = de.test();
                    if (kv.Length >= 2)
                        map.MapEntries.Add((VisitTest(kv[0]), VisitTest(kv[1])));
                }
                return map;
            }

            var set = new PilotIrCollectionExpr { Line = line, Kind = PilotCollectionKind.Set };
            if (ds.collection_element() != null)
            {
                foreach (var el in ds.collection_element())
                    set.Elements.Add(VisitTest(el.test()));
            }
            return set;
        }

        if (ctx.for_stmt() != null)
        {
            var fs = VisitForStmt(ctx.for_stmt());
            _module.UsesSliceBuffer = true;
            PilotIrExpr yieldExpr = new PilotIrIdentifierExpr { Name = fs.TargetVars.FirstOrDefault() ?? "x" };
            PilotIrExpr? filter = fs.WhereFilter;
            foreach (var s in fs.Body)
            {
                if (s is PilotIrYieldStmt ys && ys.Value != null)
                    yieldExpr = ys.Value;
                else if (s is PilotIrIfStmt ifs && ifs.ThenBody.Count == 1 && ifs.ThenBody[0] is PilotIrYieldStmt iys && iys.Value != null)
                {
                    filter = ifs.Condition;
                    yieldExpr = iys.Value;
                }
            }
            var fe = new PilotIrForExpr
            {
                Line = line,
                Iterable = fs.Iterable,
                FilterCondition = filter,
                YieldExpr = yieldExpr,
                LoopLayout = fs.LoopLayout with { ReturnMode = LoopReturnMode.CollectionYield, UsesSliceBuffer = true }
            };
            fe.TargetVars.AddRange(fs.TargetVars);
            return fe;
        }

        if (ctx.match_stmt() != null)
        {
            var ms = VisitMatchStmt(ctx.match_stmt());
            var me = new PilotIrMatchExpr { Line = line, Scrutinee = ms.Scrutinee };
            me.Arms.AddRange(ms.Arms);
            return me;
        }

        if (ctx.query_expr() != null)
            return VisitQueryExpr(ctx.query_expr());

        return new PilotIrIdentifierExpr { Line = line, Name = ctx.GetText() };
    }

    private PilotIrExpr VisitLiteral(PilotParser.LiteralContext ctx)
    {
        int line = ctx.Start?.Line ?? 0;
        if (ctx.FSTRING() != null && ctx.FSTRING().Length > 0)
        {
            string rawF = string.Concat(ctx.FSTRING().Select(t => UnquoteFString(t.GetText())));
            return ParseInterpolatedString(rawF, line);
        }

        if (ctx.STRING() != null && ctx.STRING().Length > 0)
        {
            string raw = ctx.STRING(0).GetText();
            string content = raw.Length >= 2 ? raw[1..^1] : "";
            return new PilotIrLiteralExpr
            {
                Line = line,
                Kind = PilotLiteralKind.String,
                RawValue = raw,
                StringContent = content,
                ResolvedType = PilotType.String
            };
        }

        if (ctx.TRIPLE_STRING() != null && ctx.TRIPLE_STRING().Length > 0)
        {
            string raw = ctx.TRIPLE_STRING(0).GetText();
            string content = raw.Length >= 6 ? raw[3..^3] : "";
            return new PilotIrLiteralExpr
            {
                Line = line,
                Kind = PilotLiteralKind.String,
                RawValue = $"\"{EscapeString(content)}\"",
                StringContent = content,
                ResolvedType = PilotType.String
            };
        }

        if (ctx.TRUE() != null || ctx.FALSE() != null)
        {
            return new PilotIrLiteralExpr
            {
                Line = line,
                Kind = PilotLiteralKind.Boolean,
                RawValue = ctx.TRUE() != null ? "true" : "false",
                ResolvedType = PilotType.Bool
            };
        }

        if (ctx.NONE() != null)
        {
            _module.UsesOptionTypes = true;
            return new PilotIrLiteralExpr
            {
                Line = line,
                Kind = PilotLiteralKind.None,
                RawValue = "none",
                ResolvedType = PilotType.Null
            };
        }

        if (ctx.NULL() != null || ctx.UNDEFINED() != null)
        {
            return new PilotIrLiteralExpr
            {
                Line = line,
                Kind = PilotLiteralKind.Null,
                RawValue = "null",
                ResolvedType = PilotType.Null
            };
        }

        if (ctx.DURATION_LITERAL() != null || ctx.MEMORY_LITERAL() != null || ctx.FREQUENCY_LITERAL() != null)
        {
            string rawUnit = ctx.GetText();
            string normalized = PilotOptimizer.NormalizeDimensionalLiterals(rawUnit);
            double.TryParse(normalized, NumberStyles.Float, CultureInfo.InvariantCulture, out double numVal);
            return new PilotIrLiteralExpr
            {
                Line = line,
                Kind = PilotLiteralKind.DimensionalUnit,
                RawValue = rawUnit,
                NormalizedNumericValue = numVal,
                ResolvedType = PilotType.Float
            };
        }

        if (ctx.FLOAT_NUMBER() != null)
        {
            return new PilotIrLiteralExpr
            {
                Line = line,
                Kind = PilotLiteralKind.Float,
                RawValue = ctx.FLOAT_NUMBER().GetText().Replace("_", ""),
                ResolvedType = PilotType.Float
            };
        }

        return new PilotIrLiteralExpr
        {
            Line = line,
            Kind = PilotLiteralKind.Integer,
            RawValue = ctx.GetText().Replace("_", ""),
            ResolvedType = PilotType.Int
        };
    }

    public static PilotIrInterpolatedStringExpr ParseInterpolatedString(string content, int line = 0)
    {
        var fExpr = new PilotIrInterpolatedStringExpr { Line = line, ResolvedType = PilotType.String };
        var litBuf = new StringBuilder();
        int i = 0;

        while (i < content.Length)
        {
            if (content[i] == '{')
            {
                if (i + 1 < content.Length && content[i + 1] == '{')
                {
                    litBuf.Append('{');
                    i += 2;
                    continue;
                }

                if (litBuf.Length > 0)
                {
                    fExpr.Parts.Add(new PilotIrFStringPart(false, litBuf.ToString()));
                    litBuf.Clear();
                }

                int depth = 1;
                int start = i + 1;
                i++;
                while (i < content.Length && depth > 0)
                {
                    if (content[i] == '{') depth++;
                    else if (content[i] == '}') depth--;
                    i++;
                }

                string innerCode = content[start..(i - 1)].Trim();
                string? fmtSpec = null;
                int colonIdx = FindTopLevelFormatColon(innerCode);
                if (colonIdx > 0)
                {
                    fmtSpec = innerCode[(colonIdx + 1)..].Trim();
                    innerCode = innerCode[..colonIdx].Trim();
                }

                var innerExpr = ParseInlineExpression(innerCode, line);
                fExpr.Parts.Add(new PilotIrFStringPart(true, innerCode, innerExpr, fmtSpec));
            }
            else if (content[i] == '}' && i + 1 < content.Length && content[i + 1] == '}')
            {
                litBuf.Append('}');
                i += 2;
            }
            else
            {
                litBuf.Append(content[i]);
                i++;
            }
        }

        if (litBuf.Length > 0)
        {
            fExpr.Parts.Add(new PilotIrFStringPart(false, litBuf.ToString()));
        }

        return fExpr;
    }

    private static int FindTopLevelFormatColon(string s)
    {
        int depth = 0;
        for (int i = 0; i < s.Length; i++)
        {
            char c = s[i];
            if (c is '(' or '[' or '{') depth++;
            else if (c is ')' or ']' or '}') depth--;
            else if (c == ':' && depth == 0)
            {
                if (i + 1 < s.Length && s[i + 1] == ':') { i++; continue; }
                return i;
            }
        }
        return -1;
    }

    private static PilotIrExpr ParseInlineExpression(string exprCode, int line)
    {
        try
        {
            var input = new AntlrInputStream(exprCode + ";");
            var lexer = new PilotLexer(input);
            lexer.RemoveErrorListeners();
            var tokens = new CommonTokenStream(lexer);
            var parser = new PilotParser(tokens);
            parser.RemoveErrorListeners();
            var tree = parser.ParseProgramFast();
            var subBuilder = new PilotTranspileIrBuilder(tokens, null, new HashSet<string>(), new TranspilationOptions());
            subBuilder.VisitProgram(tree);
            if (subBuilder._module.Statements.Count > 0 && subBuilder._module.Statements[0] is PilotIrExprStmt es)
                return es.Expression;
        }
        catch
        {
            // Fallback to identifier
        }
        return new PilotIrIdentifierExpr { Line = line, Name = exprCode };
    }

    private PilotIrPattern VisitPattern(PilotParser.PatternContext ctx)
    {
        var andPats = ctx.or_pattern().and_pattern();
        if (andPats.Length > 1)
        {
            var orPat = new PilotIrOrPattern { Line = ctx.Start?.Line ?? 0 };
            foreach (var ap in andPats)
                orPat.Alternatives.Add(VisitPrimaryPattern(ap.not_pattern(0).primary_pattern()));
            return orPat;
        }
        return VisitPrimaryPattern(andPats[0].not_pattern(0).primary_pattern());
    }

    private PilotIrPattern VisitPrimaryPattern(PilotParser.Primary_patternContext? ctx)
    {
        int line = ctx?.Start?.Line ?? 0;
        if (ctx == null || ctx.UNDERSCORE() != null)
            return new PilotIrWildcardPattern { Line = line };

        if (ctx.literal() != null)
        {
            var lit = VisitLiteral(ctx.literal());
            if (lit is PilotIrLiteralExpr le)
            {
                if (le.Kind == PilotLiteralKind.None)
                {
                    _module.UsesOptionTypes = true;
                    return new PilotIrOptionPattern { Line = line, IsSome = false };
                }
                return new PilotIrLiteralPattern { Line = line, Literal = le };
            }
        }

        if (ctx.range_pattern() != null)
        {
            var rp = ctx.range_pattern();
            return new PilotIrRangePattern
            {
                Line = line,
                Start = new PilotIrLiteralExpr { Kind = PilotLiteralKind.Integer, RawValue = rp.range_bound(0).GetText() },
                End = new PilotIrLiteralExpr { Kind = PilotLiteralKind.Integer, RawValue = rp.range_bound(1).GetText() },
                IsInclusive = rp.DOTDOT_LESS() == null
            };
        }

        if (ctx.relational_pattern() != null)
        {
            var rel = ctx.relational_pattern();
            return new PilotIrRelationalPattern
            {
                Line = line,
                Operator = rel.GetChild(0).GetText(),
                Value = VisitShiftExpr(rel.shift_expr())
            };
        }

        if (ctx.tuple_pattern_match() != null)
        {
            var tp = new PilotIrTuplePattern { Line = line };
            foreach (var p in ctx.tuple_pattern_match().pattern())
                tp.Elements.Add(VisitPattern(p));
            return tp;
        }

        if (ctx.type_or_var_pattern() != null)
        {
            var tv = ctx.type_or_var_pattern();
            if (tv.LET() != null || tv.VAR() != null)
            {
                return new PilotIrBindingPattern
                {
                    Line = line,
                    Name = tv.identifier().GetText(),
                    IsMutable = tv.VAR() != null
                };
            }

            string dotted = tv.dotted_name().GetText();
            if (dotted is "some" or "Some")
            {
                _module.UsesOptionTypes = true;
                PilotIrPattern? inner = tv.pattern_arg().Length > 0
                    ? VisitPattern(tv.pattern_arg(0).pattern())
                    : (tv.identifier() != null ? new PilotIrBindingPattern { Name = tv.identifier().GetText() } : null);
                return new PilotIrOptionPattern { Line = line, IsSome = true, InnerPattern = inner };
            }

            if (tv.LPAR() != null || dotted.Contains('.'))
            {
                string? typePrefix = null;
                string variantName = dotted;
                int dotIdx = dotted.LastIndexOf('.');
                if (dotIdx > 0)
                {
                    typePrefix = dotted[..dotIdx];
                    variantName = dotted[(dotIdx + 1)..];
                }
                var vp = new PilotIrVariantPattern { Line = line, TypeName = typePrefix, VariantName = variantName };
                foreach (var pa in tv.pattern_arg())
                {
                    vp.Arguments.Add((pa.identifier()?.GetText(), VisitPattern(pa.pattern())));
                }
                return vp;
            }

            if (tv.identifier() != null)
            {
                return new PilotIrBindingPattern
                {
                    Line = line,
                    Name = tv.identifier().GetText(),
                    TypeName = dotted
                };
            }

            return new PilotIrBindingPattern { Line = line, Name = dotted };
        }

        return new PilotIrWildcardPattern { Line = line };
    }

    private void ClassifyLibraryCall(PilotIrCallExpr call)
    {
        if (call.Callee is PilotIrIdentifierExpr id)
        {
            call.LibraryCallKind = id.Name switch
            {
                "println" => PilotLibraryCallKind.Println,
                "print" => PilotLibraryCallKind.Print,
                "matrix" => PilotLibraryCallKind.MatrixCreate,
                "zeros" => PilotLibraryCallKind.MatrixZeros,
                "ones" => PilotLibraryCallKind.MatrixOnes,
                "eye" => PilotLibraryCallKind.MatrixEye,
                "vectorize" => PilotLibraryCallKind.MatrixVectorize,
                "json_dumps" => PilotLibraryCallKind.JsonDumps,
                "json_loads" => PilotLibraryCallKind.JsonLoads,
                "read_text" => PilotLibraryCallKind.FileReadText,
                "write_text" => PilotLibraryCallKind.FileWriteText,
                "file_exists" => PilotLibraryCallKind.FileExists,
                "parallel_map" => PilotLibraryCallKind.ParallelMap,
                "sqrt" => PilotLibraryCallKind.MathSqrt,
                "sin" => PilotLibraryCallKind.MathSin,
                "cos" => PilotLibraryCallKind.MathCos,
                "abs" => PilotLibraryCallKind.MathAbs,
                "floor" => PilotLibraryCallKind.MathFloor,
                "ceil" => PilotLibraryCallKind.MathCeil,
                "pow" => PilotLibraryCallKind.MathPow,
                "len" => PilotLibraryCallKind.Len,
                "range" => PilotLibraryCallKind.Range,
                "some" => PilotLibraryCallKind.SomeConstructor,
                "fail" => PilotLibraryCallKind.FailConstructor,
                _ => PilotLibraryCallKind.None
            };

            if (call.LibraryCallKind is PilotLibraryCallKind.MatrixCreate or PilotLibraryCallKind.MatrixZeros or PilotLibraryCallKind.MatrixOnes or PilotLibraryCallKind.MatrixEye or PilotLibraryCallKind.MatrixVectorize)
                _module.UsesMatrixPrimitives = true;
            if (call.LibraryCallKind is PilotLibraryCallKind.JsonDumps or PilotLibraryCallKind.JsonLoads)
                _module.UsesJsonPrimitives = true;
            if (call.LibraryCallKind == PilotLibraryCallKind.SomeConstructor)
                _module.UsesOptionTypes = true;
            if (call.LibraryCallKind == PilotLibraryCallKind.FailConstructor)
                _module.UsesResultTypes = true;
        }
        else if (call.Callee is PilotIrMemberAccessExpr mem && mem.Target is PilotIrIdentifierExpr targetId)
        {
            if (targetId.Name == "json" && mem.MemberName == "dumps")
            {
                call.LibraryCallKind = PilotLibraryCallKind.JsonDumps;
                _module.UsesJsonPrimitives = true;
            }
            else if (targetId.Name == "json" && mem.MemberName == "loads")
            {
                call.LibraryCallKind = PilotLibraryCallKind.JsonLoads;
                _module.UsesJsonPrimitives = true;
            }
            else if (targetId.Name == "math")
            {
                call.LibraryCallKind = mem.MemberName switch
                {
                    "sqrt" => PilotLibraryCallKind.MathSqrt,
                    "sin" => PilotLibraryCallKind.MathSin,
                    "cos" => PilotLibraryCallKind.MathCos,
                    "abs" => PilotLibraryCallKind.MathAbs,
                    "floor" => PilotLibraryCallKind.MathFloor,
                    "ceil" => PilotLibraryCallKind.MathCeil,
                    "pow" => PilotLibraryCallKind.MathPow,
                    _ => PilotLibraryCallKind.None
                };
            }
        }
    }

    private (string PrimaryName, List<string> TupleNames) ExtractBindingPattern(PilotParser.Binding_patternContext? ctx)
    {
        var tupleNames = new List<string>();
        if (ctx == null)
            return ("_", tupleNames);
        if (ctx.identifier() != null)
            return (ctx.identifier().GetText(), tupleNames);
        if (ctx.tuple_pattern() != null)
        {
            foreach (var item in ctx.tuple_pattern().binding_pattern_item())
            {
                string n = item.binding_pattern().identifier()?.GetText() ?? "_";
                tupleNames.Add(n);
            }
            return (string.Join(", ", tupleNames), tupleNames);
        }
        return (ctx.GetText(), tupleNames);
    }

    public static string? FormatTypeSpec(PilotParser.Type_specContext? ctx)
    {
        if (ctx == null)
            return null;

        string unionStr = FormatUnionType(ctx.union_type());
        if (ctx.EXCLAMATION() != null)
        {
            string errStr = ctx.error_union_type() != null
                ? string.Join(" | ", ctx.error_union_type().error_primary_type().Select(e => e.GetText()))
                : "Error";
            return $"{unionStr} ! {errStr}";
        }
        return unionStr;
    }

    private static string FormatUnionType(PilotParser.Union_typeContext ctx)
    {
        var parts = ctx.intersection_type().Select(it =>
        {
            return string.Join(" & ", it.primary_type().Select(pt => pt.GetText()));
        });
        return string.Join(" | ", parts);
    }

    private static string FormatDecorator(PilotParser.DecoratorContext dec)
    {
        string name = dec.dotted_name().GetText();
        if (dec.LPAR() != null)
        {
            string args = dec.arglist() != null ? dec.arglist().GetText() : "";
            return $"@{name}({args})";
        }
        return $"@{name}";
    }

    private PilotType ResolveVarType(string name, string? declType, PilotIrExpr? init)
    {
        if (_module.SymbolTypes.TryGetValue(name, out var inferred) && inferred != PilotType.Any)
            return inferred;
        if (init != null && init.ResolvedType != PilotType.Any)
            return init.ResolvedType;
        if (!string.IsNullOrEmpty(declType))
        {
            return declType switch
            {
                "int" or "i32" or "i64" or "i16" or "i8" or "u8" or "u16" or "u32" or "u64" => PilotType.Int,
                "float" or "f64" or "f32" => PilotType.Float,
                "str" or "string" => PilotType.String,
                "bool" => PilotType.Bool,
                _ => PilotType.Any
            };
        }
        return PilotType.Any;
    }

    private void CheckTypeFlags(string? typeStr, PilotType? resolved)
    {
        if (!string.IsNullOrEmpty(typeStr))
        {
            if (typeStr!.EndsWith("?")) _module.UsesOptionTypes = true;
            if (typeStr.Contains(" ! ")) _module.UsesResultTypes = true;
        }
        if (resolved?.Kind == PilotTypeKind.Option)
            _module.UsesOptionTypes = true;
    }

    private static string UnquoteFString(string raw)
    {
        if (raw.StartsWith("f\"", StringComparison.OrdinalIgnoreCase) && raw.EndsWith("\""))
            return raw[2..^1];
        if (raw.StartsWith("f'", StringComparison.OrdinalIgnoreCase) && raw.EndsWith("'"))
            return raw[2..^1];
        return raw;
    }

    private static string EscapeString(string s) =>
        s.Replace("\\", "\\\\").Replace("\"", "\\\"").Replace("\r", "\\r").Replace("\n", "\\n");
}
