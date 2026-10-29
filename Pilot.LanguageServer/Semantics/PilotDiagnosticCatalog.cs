using System;
using System.Collections.Generic;

namespace Pilot.LanguageServer.Semantics;

/// <summary>
/// Diagnostic descriptor containing metadata, severity, message format, and QuickFix guidelines.
/// </summary>
public sealed class PilotDiagnosticDescriptor
{
    public string Code { get; }
    public string Title { get; }
    public string Category { get; }
    public PilotDiagnosticSeverity DefaultSeverity { get; }
    public string MessageFormat { get; }
    public string Description { get; }
    public string? QuickFixTitle { get; }

    public PilotDiagnosticDescriptor(
        string code,
        string title,
        string category,
        PilotDiagnosticSeverity defaultSeverity,
        string messageFormat,
        string description,
        string? quickFixTitle = null)
    {
        Code = code;
        Title = title;
        Category = category;
        DefaultSeverity = defaultSeverity;
        MessageFormat = messageFormat;
        Description = description;
        QuickFixTitle = quickFixTitle;
    }

    public string Format(params object[] args)
    {
        if (args == null || args.Length == 0) return MessageFormat;
        try
        {
            return string.Format(MessageFormat, args);
        }
        catch
        {
            return MessageFormat;
        }
    }
}

/// <summary>
/// Comprehensive, structured catalogue of all Pilot compiler and Language Server diagnostic codes (PILOT001–PILOT999).
/// </summary>
public static class PilotDiagnosticCatalog
{
    // =========================================================================
    // Series 000–099: Lexical, Preprocessor & Syntax
    // =========================================================================
    public const string SyntaxError = "PILOT001";
    public const string UnclosedPreprocBlock = "PILOT002";
    public const string UnknownPreprocDirective = "PILOT003";
    public const string UnterminatedString = "PILOT004";
    public const string InvalidLiteralFormat = "PILOT005";
    public const string RedundantSemicolon = "PILOT006";

    // =========================================================================
    // Series 100–199: Scope, Binding & Control Flow
    // =========================================================================
    public const string ImmutableReassignment = "PILOT101";
    public const string TypeMismatch = "PILOT102";
    public const string GuardMissingExit = "PILOT103";
    public const string UnboundedLoopInRealtime = "PILOT104";
    public const string DuplicateSymbol = "PILOT105";
    public const string AwaitOutsideAsync = "PILOT106";
    public const string UndeclaredSavepoint = "PILOT107";
    public const string LoopJumpOutsideLoop = "PILOT108";
    public const string ReturnOutsideFunction = "PILOT109";
    public const string YieldOutsideGenerator = "PILOT110";
    public const string UnreachableCode = "PILOT111";
    public const string UnusedVariable = "PILOT112";
    public const string ParameterReassignment = "PILOT113";

    // =========================================================================
    // Series 200–299: Type System, Signatures & Null-Safety
    // =========================================================================
    public const string TypeMismatchExtended = "PILOT201";
    public const string PossibleNullDereference = "PILOT202";
    public const string MemberNotFound = "PILOT203";
    public const string NoMatchingOverload = "PILOT204";
    public const string ReturnTypeMismatch = "PILOT205";
    public const string NonExhaustiveMatch = "PILOT206";
    public const string ImplicitNarrowing = "PILOT207";
    public const string MissingInterfaceMethod = "PILOT208";

    // =========================================================================
    // Series 300–399: Ownership, Borrowing & Lifetimes
    // =========================================================================
    public const string TypeInferenceMismatch = "PILOT300";
    public const string UseOfMovedValue = "PILOT301";
    public const string BorrowConflict = "PILOT302";
    public const string BorrowOutlivesOwner = "PILOT303";
    public const string MultipleMutableBorrows = "PILOT304";
    public const string OwnerMutatedWhileBorrowed = "PILOT305";

    // =========================================================================
    // Series 400–499: Async, Concurrency & Channels
    // =========================================================================
    public const string UnawaitedFutureDiscarded = "PILOT401";
    public const string ActorDirectStateAccess = "PILOT402";
    public const string UnsharedChannelSend = "PILOT403";
    public const string NonChannelReceive = "PILOT404";
    public const string UnboundedSelectDeadlockRisk = "PILOT405";

    // =========================================================================
    // Series 500–599: Real-Time Systems & Determinism
    // =========================================================================
    public const string HeapAllocInRealtime = "PILOT501";
    public const string WcetExceedsDeadline = "PILOT502";
    public const string UnboundedLoopInRealtimeExtended = "PILOT503";
    public const string BlockingIoInRealtime = "PILOT504";

    // =========================================================================
    // Series 600–699: UDSA, Units of Measure & Matrices
    // =========================================================================
    public const string DimensionalUnitMismatch = "PILOT601";
    public const string MatrixDimensionMismatch = "PILOT602";
    public const string DuplicateUnitDefinition = "PILOT603";
    public const string LossyUnitConversion = "PILOT604";

    // =========================================================================
    // Series 700–799: Contracts & Typestates
    // =========================================================================
    public const string PreconditionUnsatisfiable = "PILOT701";
    public const string PostconditionViolation = "PILOT702";
    public const string IllegalTypestateTransition = "PILOT703";
    public const string MethodUnavailableInState = "PILOT704";
    public const string MethodStealCollision = "PILOT705";

    // =========================================================================
    // Series 800–899: Database, Relational & Interop
    // =========================================================================
    public const string IndexNotFoundOnTable = "PILOT801";
    public const string UnindexedJoinCondition = "PILOT802";
    public const string UndeclaredSavepointExtended = "PILOT803";
    public const string FfiTypeCannotBeMarshaled = "PILOT804";

    // =========================================================================
    // Series 900–999: Code Quality, Modernization & Lints
    // =========================================================================
    public const string SuggestCascadeOperator = "PILOT901";
    public const string SuggestFString = "PILOT902";
    public const string HotReloadRedefinedConst = "PILOT903";
    public const string EmptyCatchBlock = "PILOT904";
    public const string SuggestVectorized = "PILOT905";

    // =========================================================================
    // MATCH Series: Maranget Pattern Matching & ADT Verification (Milestone 11)
    // =========================================================================
    public const string MatchNonExhaustive = "MATCH001";
    public const string MatchRedundantArm = "MATCH002";

    // =========================================================================
    // NULL Series: Concise Modern Null-Safety System (Milestone 12)
    // =========================================================================
    public const string NullAssignedToNonNullable = "NULL001";
    public const string UncheckedOptionalAccess = "NULL002";
    public const string NullReturnedFromNonNullable = "NULL003";
    public const string NullArgumentToNonNullable = "NULL004";
    public const string UnsafeNonNullAssertionRemoved = "NULL005";

    // =========================================================================
    // ERR Series: Values-First Error System T ! E (Milestone 13)
    // =========================================================================
    public const string UnhandledErrorReturn = "ERR001";
    public const string IncompatibleErrorPropagation = "ERR002";
    public const string NonExhaustiveCatchRecovery = "ERR003";
    public const string InvalidFailReturn = "ERR004";
    public const string ErrorStratificationLint = "ERR005";

    // =========================================================================
    // LOOP Series: Dynamic-Schedule & Expression-Oriented For Loops (Milestone 15)
    // =========================================================================
    public const string MixedBreakAndYieldInForLoop = "LOOP001";
    public const string MixedVoidAndValuedBreakInForLoop = "LOOP002";
    public const string InvalidForLoopThenBranch = "LOOP003";
    public const string YieldInStatementForLoopOutsideGenerator = "LOOP004";

    // =========================================================================
    // TYPE Series: Bidirectional, Flow-Sensitive & Row-Polymorphic Inference (Milestone 16)
    // =========================================================================
    public const string BidirectionalConstraintMismatch = "TYPE001";
    public const string CollectionElementMismatch = "TYPE002";
    public const string InfiniteTypeOccursCheck = "TYPE003";
    public const string LambdaReturnMismatch = "TYPE004";
    public const string LambdaParameterArityMismatch = "TYPE005";
    public const string MethodGroupOverloadMismatch = "TYPE006";
    public const string RecursiveReturnUnificationMismatch = "TYPE007";
    public const string MissingStructuralRowField = "TYPE008";
    public const string IncompatibleRowFieldType = "TYPE009";
    public const string ClosedRowExtraField = "TYPE010";
    public const string TypeClassConstraintViolation = "TYPE011";
    public const string TypestateTransitionMismatch = "TYPE012";

    // =========================================================================
    // QUOTE Series: Homoiconic Call-Quoting & Multi-Modal Projection (Milestone 17)
    // =========================================================================
    public const string MissingWhiteBoxAstForDeepSplice = "QUOTE001";
    public const string InvalidUnquotePrefixOutsideDeepSplice = "QUOTE002";
    public const string NonUnmanagedParameterInAbiCallFrame = "QUOTE003";
    public const string NondifferentiableAdjointCallQuote = "QUOTE004";
    public const string UnhandledEffectInDryRunQuote = "QUOTE005";

    // =========================================================================
    // COMPTIME Series: Runtime-to-Compile-Time Demotion & Comptime Audit (Milestone 19)
    // =========================================================================
    public const string DynamicScopeLookupInZeroRuntime = "COMPTIME001";
    public const string DynamicMemberLookupInZeroRuntime = "COMPTIME002";
    public const string DynamicMethodDispatchInZeroRuntime = "COMPTIME003";
    public const string UnloweredDtoProjectionInZeroRuntime = "COMPTIME004";
    public const string UnprovenRuntimeContractInZeroRuntime = "COMPTIME005";
    public const string DynamicContextMapInZeroRuntime = "COMPTIME006";
    public const string AstFallbackInZeroRuntime = "COMPTIME007";
    public const string ComptimeAssertFailed = "COMPTIME008";
    public const string ImpureOrExceededComptimeExecution = "COMPTIME009";
    public const string NonConstantComptimeArgument = "COMPTIME010";

    private static readonly Dictionary<string, PilotDiagnosticDescriptor> Descriptors = new(StringComparer.Ordinal);

    static PilotDiagnosticCatalog()
    {
        void Register(PilotDiagnosticDescriptor d) => Descriptors[d.Code] = d;

        // Series 000: Syntax & Preprocessing
        Register(new(SyntaxError, "Syntax error", "Syntax", PilotDiagnosticSeverity.Error, "Syntax error: {0}", "The code contains an unexpected token or grammar violation."));
        Register(new(UnclosedPreprocBlock, "Unclosed preprocessor block", "Preprocessing", PilotDiagnosticSeverity.Error, "Unclosed preprocessor conditional block '#if'", "An #if, #ifdef, or #region directive was opened without a corresponding closing directive.", "Insert matching closing directive"));
        Register(new(UnknownPreprocDirective, "Unknown preprocessor directive", "Preprocessing", PilotDiagnosticSeverity.Warning, "Unknown preprocessor directive '#{0}'", "Encountered an unrecognized preprocessor directive."));
        Register(new(UnterminatedString, "Unterminated string literal", "Syntax", PilotDiagnosticSeverity.Error, "Unterminated string literal or f-string interpolation", "String opened without closing quotation marks or unclosed f-string interpolation brace.", "Close string or format braces"));
        Register(new(InvalidLiteralFormat, "Invalid literal format", "Syntax", PilotDiagnosticSeverity.Error, "Invalid numeric or unit literal: '{0}'", "Literal format is malformed or invalid suffix."));
        Register(new(RedundantSemicolon, "Redundant semicolon", "Syntax", PilotDiagnosticSeverity.Hint, "Redundant semicolon at end of statement", "Semicolons are optional in Pilot; trailing semicolons can be omitted.", "Remove redundant semicolon"));

        // Series 100: Scope & Flow
        Register(new(ImmutableReassignment, "Cannot assign to immutable binding", "Scope", PilotDiagnosticSeverity.Error, "Cannot reassign immutable binding '{0}' (declared with '{1}'). Use 'var' if mutation is intended.", "Attempted to reassign or mutate a let binding or const symbol.", "Change declaration to 'var'"));
        Register(new(TypeMismatch, "Type mismatch", "TypeSystem", PilotDiagnosticSeverity.Error, "Type mismatch: cannot assign value of type '{0}' to declared type '{1}'", "The evaluated type does not match the target annotation or binding constraint.", "Insert explicit cast"));
        Register(new(GuardMissingExit, "Guard missing early exit", "ControlFlow", PilotDiagnosticSeverity.Error, "A 'guard' block must exit the enclosing function, loop, or transaction via 'return', 'raise', 'break', 'continue', or 'rollback'.", "Guard statements must terminate or branch out of the current control flow block."));
        Register(new(UnboundedLoopInRealtime, "Unbounded loop in real-time context", "RealTime", PilotDiagnosticSeverity.Error, "Unbounded loop is forbidden inside '@hard_realtime' or 'noalloc' contexts. Add 'bounded <limit>' to guarantee WCET.", "Real-time loops must declare static iteration upper bounds.", "Add 'bounded <N>' limit to loop"));
        Register(new(DuplicateSymbol, "Duplicate symbol declaration", "Scope", PilotDiagnosticSeverity.Error, "Symbol '{0}' is already declared in this scope (line {1})", "A variable, function, class, or parameter with this name is already declared in the current lexical scope.", "Rename symbol"));
        Register(new(AwaitOutsideAsync, "Await in synchronous context", "Async", PilotDiagnosticSeverity.Error, "The 'await' operator can only be used inside an 'async def' function, 'actor' handler, or top-level script.", "The 'await' operator requires an asynchronous execution context.", "Add 'async' modifier to enclosing function"));
        Register(new(UndeclaredSavepoint, "Undeclared transaction savepoint", "Database", PilotDiagnosticSeverity.Error, "Cannot rollback to undeclared savepoint '{0}'. Declare 'savepoint {0}' before rolling back to it.", "Transaction savepoint was not marked prior to rollback."));
        Register(new(LoopJumpOutsideLoop, "Loop jump outside of loop", "ControlFlow", PilotDiagnosticSeverity.Error, "'{0}' statement cannot be used outside of a loop", "The 'break' or 'continue' keyword can only appear inside 'for' or 'while' loops."));
        Register(new(ReturnOutsideFunction, "Return outside of function", "ControlFlow", PilotDiagnosticSeverity.Error, "'return' statement cannot appear outside of a function, method, or property accessor", "Return statements can only be used inside callable routines."));
        Register(new(YieldOutsideGenerator, "Yield outside of generator", "ControlFlow", PilotDiagnosticSeverity.Error, "'yield' statement cannot appear outside of a generator function", "The 'yield' keyword can only be used in routines that generate sequences."));
        Register(new(UnreachableCode, "Unreachable code detected", "ControlFlow", PilotDiagnosticSeverity.Warning, "Unreachable code detected after '{0}'", "Code placed immediately after an unconditional jump, return, or raise will never execute.", "Remove unreachable code"));
        Register(new(UnusedVariable, "Unused variable", "Scope", PilotDiagnosticSeverity.Warning, "Variable '{0}' is declared but its value is never used", "A local variable was declared and assigned but never read.", "Prefix with '_' or remove variable"));
        Register(new(ParameterReassignment, "Cannot reassign parameter", "Scope", PilotDiagnosticSeverity.Error, "Cannot reassign immutable parameter '{0}'. Declare with 'mut' if parameter mutation is intended.", "Function parameters are immutable by default in Pilot.", "Add 'mut' to parameter definition"));

        // Series 200: Type System & Null-Safety
        Register(new(TypeMismatchExtended, "Type mismatch (extended)", "TypeSystem", PilotDiagnosticSeverity.Error, "Type mismatch: cannot assign value of type '{0}' to declared type '{1}'", "The evaluated type does not match the target annotation or binding constraint.", "Insert explicit cast"));
        Register(new(PossibleNullDereference, "Possible null dereference", "NullSafety", PilotDiagnosticSeverity.Error, "Possible null dereference on optional expression '{0}': type is '{1}'", "Invoking a member on a nullable type 'T?' without a null guard or safe navigation operator.", "Replace '.' with '?.' safe navigation"));
        Register(new(MemberNotFound, "Member not found on type", "TypeSystem", PilotDiagnosticSeverity.Error, "Type '{0}' does not define a member named '{1}'", "Accessing a nonexistent field, property, or method on the specified static type."));
        Register(new(NoMatchingOverload, "No matching overload", "TypeSystem", PilotDiagnosticSeverity.Error, "No overload of '{0}' matches argument list '({1})'", "Argument count or types do not match any available method group overload."));
        Register(new(ReturnTypeMismatch, "Return type mismatch", "TypeSystem", PilotDiagnosticSeverity.Error, "Function declared return type '{0}', but returned value of type '{1}'", "The expression returned does not conform to the function's return type contract."));
        Register(new(NonExhaustiveMatch, "Non-exhaustive pattern match", "PatternMatching", PilotDiagnosticSeverity.Error, "Non-exhaustive pattern match: missing cases for '{0}'", "Pattern match does not handle all enum variants or typestates.", "Add default arm 'default: pass'"));
        Register(new(ImplicitNarrowing, "Implicit narrowing conversion", "TypeSystem", PilotDiagnosticSeverity.Warning, "Implicit narrowing conversion from '{0}' to '{1}' may lose precision", "Converting a higher precision numeric type to a lower precision numeric type.", "Use explicit conversion T(...)"));
        Register(new(MissingInterfaceMethod, "Missing interface method", "TypeSystem", PilotDiagnosticSeverity.Error, "Class '{0}' does not implement required interface method '{1}'", "A class declaring an interface contract must provide all declared methods.", "Implement missing interface method"));

        // Series 300: Ownership & Borrowing
        Register(new(TypeInferenceMismatch, "Type inference conflict", "TypeSystem", PilotDiagnosticSeverity.Error, "Type inference conflict: expression cannot be unified between '{0}' and '{1}'", "Bidirectional type inference detected conflicting constraints."));
        Register(new(UseOfMovedValue, "Use of moved value", "Ownership", PilotDiagnosticSeverity.Error, "Use of moved value '{0}'", "Reading or transferring a linear/owned value after ownership was already moved.", "Pass reference '&' or call '.clone()'"));
        Register(new(BorrowConflict, "Mutable and immutable borrow conflict", "Ownership", PilotDiagnosticSeverity.Error, "Cannot borrow '{0}' as mutable while it is already borrowed as immutable", "Rust-style borrow rules prevent active mutable borrows while read-only references exist."));
        Register(new(BorrowOutlivesOwner, "Borrow outlives owner", "Ownership", PilotDiagnosticSeverity.Error, "Borrowed reference '{0}' does not live long enough", "A borrowed reference cannot escape the lexical lifetime of its owner."));
        Register(new(MultipleMutableBorrows, "Multiple mutable borrows", "Ownership", PilotDiagnosticSeverity.Error, "Cannot borrow '{0}' as mutable more than once simultaneously", "Only one active mutable reference '&mut' is permitted at a time."));
        Register(new(OwnerMutatedWhileBorrowed, "Owner mutated while borrowed", "Ownership", PilotDiagnosticSeverity.Error, "Owner '{0}' cannot be mutated while an active borrow exists", "Direct mutation of owner invalidates active borrows."));

        // Series 400: Async, Concurrency & Channels
        Register(new(UnawaitedFutureDiscarded, "Unawaited future expression", "Async", PilotDiagnosticSeverity.Warning, "Expression of type 'Future[{0}]' is neither awaited nor spawned and will be discarded", "Calling an async function without await drops the returned task.", "Add 'await' prefix"));
        Register(new(ActorDirectStateAccess, "Direct actor state access", "Concurrency", PilotDiagnosticSeverity.Error, "Cannot access private actor state '{0}' from outside actor context", "Actor state is strictly isolated; communication must happen via actor messages.", "Invoke actor message method"));
        Register(new(UnsharedChannelSend, "Unshared reference across channel", "Concurrency", PilotDiagnosticSeverity.Error, "Cannot send un-isolated mutable reference '{0}' across channel", "Sending shared mutable memory across channels creates data races.", "Send '.clone()' or immutable snapshot"));
        Register(new(NonChannelReceive, "Receive on non-channel", "Concurrency", PilotDiagnosticSeverity.Error, "Channel receive '<-' target must be of type 'Channel[T]', but got '{0}'", "The '<-' receive operator is only valid on Channel instances."));
        Register(new(UnboundedSelectDeadlockRisk, "Select block missing timeout", "Concurrency", PilotDiagnosticSeverity.Warning, "'select' block has neither 'timeout' nor 'default' case and may block indefinitely", "CSP channel multiplexing without fallback can deadlock.", "Add 'timeout 50ms { ... }' or 'default { ... }'"));

        // Series 500: Real-Time & Determinism
        Register(new(HeapAllocInRealtime, "Heap allocation in real-time context", "RealTime", PilotDiagnosticSeverity.Error, "Heap allocation detected inside 'noalloc' / '@hard_realtime' context: '{0}'", "Dynamic allocations trigger non-deterministic GC pauses in hard real-time tasks.", "Use preallocated buffer or 'stackalloc'"));
        Register(new(WcetExceedsDeadline, "Worst-case execution time exceeds deadline", "RealTime", PilotDiagnosticSeverity.Error, "Worst-Case Execution Time (WCET) exceeds deadline: {0}ms > {1}ms", "Static timing analysis proves the code path exceeds its configured deadline budget."));
        Register(new(UnboundedLoopInRealtimeExtended, "Unbounded loop in real-time context (extended)", "RealTime", PilotDiagnosticSeverity.Error, "Unbounded loop is forbidden inside '@hard_realtime' or 'noalloc' contexts. Add 'bounded <limit>' to guarantee WCET.", "Real-time loops must declare static iteration upper bounds.", "Add 'bounded <N>' limit to loop"));
        Register(new(BlockingIoInRealtime, "Blocking I/O in real-time context", "RealTime", PilotDiagnosticSeverity.Error, "Blocking I/O operation '{0}' is forbidden inside deterministic real-time context", "File, network, or console I/O blocks real-time scheduler threads."));

        // Series 600: UDSA, Units & Matrices
        Register(new(DimensionalUnitMismatch, "Dimensional analysis unit mismatch", "UDSA", PilotDiagnosticSeverity.Error, "Dimensional analysis mismatch: cannot combine or compare incompatible units '{0}' and '{1}'", "Units of measure must be dimensionally equivalent (e.g. cannot add meters to seconds).", "Apply unit conversion"));
        Register(new(MatrixDimensionMismatch, "Matrix dimension mismatch", "UDSA", PilotDiagnosticSeverity.Error, "Matrix dimension mismatch: cannot multiply {0}x{1} by {2}x{3}", "In matrix multiplication A * B, columns of A must match rows of B."));
        Register(new(DuplicateUnitDefinition, "Duplicate unit definition", "UDSA", PilotDiagnosticSeverity.Error, "Unit '{0}' has already been defined with a different derivation in this scope", "A unit with this identifier has already been declared."));
        Register(new(LossyUnitConversion, "Lossy unit conversion", "UDSA", PilotDiagnosticSeverity.Warning, "Unit conversion from '{0}' to '{1}' loses precision without explicit rounding", "High-precision unit conversion should be explicitly acknowledged."));

        // Series 700: Contracts & Typestates
        Register(new(PreconditionUnsatisfiable, "Precondition unsatisfiable", "Contracts", PilotDiagnosticSeverity.Error, "Static contract solver: precondition 'requires {0}' is provably unsatisfiable", "Static verification proved that the caller arguments will always fail the precondition."));
        Register(new(PostconditionViolation, "Postcondition violation", "Contracts", PilotDiagnosticSeverity.Error, "Static contract solver: postcondition 'ensures {0}' may not hold on all exit paths", "The function body return expression does not guarantee the promised postcondition."));
        Register(new(IllegalTypestateTransition, "Illegal typestate transition", "Typestates", PilotDiagnosticSeverity.Error, "Illegal typestate transition: '{0}' is not a valid state for class '{1}'", "Object cannot transition to an unregistered typestate.", "Check available class states"));
        Register(new(MethodUnavailableInState, "Method unavailable in current state", "Typestates", PilotDiagnosticSeverity.Error, "Method '{0}' is only available when object is in state '{1}', but current state is '{2}'", "Typestate methods can only be invoked when the object is in that state."));
        Register(new(MethodStealCollision, "Method stealing collision", "Typestates", PilotDiagnosticSeverity.Error, "Cannot steal method '{0}': signature collides with existing member on '{1}'", "Method stealing cannot overwrite explicitly declared methods with the same signature."));

        // Series 800: Database & Interop
        Register(new(IndexNotFoundOnTable, "Index not found on table", "Database", PilotDiagnosticSeverity.Error, "Index '{0}' does not exist on table '{1}'", "Relational query specifies an index that was not defined in the Table[T] model."));
        Register(new(UnindexedJoinCondition, "Unindexed join condition", "Database", PilotDiagnosticSeverity.Warning, "Unindexed join condition on '{0}': query may perform a full table scan", "Joining large tables without index coverage degrades query performance.", "Add index to join column"));
        Register(new(UndeclaredSavepointExtended, "Undeclared transaction savepoint (extended)", "Database", PilotDiagnosticSeverity.Error, "Cannot rollback to undeclared savepoint '{0}'. Declare 'savepoint {0}' before rolling back to it.", "Transaction savepoint was not marked prior to rollback."));
        Register(new(FfiTypeCannotBeMarshaled, "FFI type cannot be marshaled", "Interop", PilotDiagnosticSeverity.Error, "Type '{0}' cannot be marshaled to native foreign signature '{1}'", "Passing un-marshalable types across native FFI boundaries."));

        // Series 900: Code Quality & Lints
        Register(new(SuggestCascadeOperator, "Suggest cascade operator", "Quality", PilotDiagnosticSeverity.Hint, "Sequential method calls on '{0}' can be simplified using the cascade operator '..'", "Fluent mutation can be expressed more cleanly with cascade chaining.", "Refactor to use '..'"));
        Register(new(SuggestFString, "Suggest f-string interpolation", "Quality", PilotDiagnosticSeverity.Hint, "String concatenation can be simplified using an interpolated f-string", "Interpolated f-strings are more readable and performant than string concatenation.", "Convert to f-string"));
        Register(new(HotReloadRedefinedConst, "Hot-reload redefined constant", "Quality", PilotDiagnosticSeverity.Warning, "Hot-reload patch redefined constant '{0}' which may cause inconsistency with cached references", "Live patching constants does not update inlined primitive constants."));
        Register(new(EmptyCatchBlock, "Empty catch block", "Quality", PilotDiagnosticSeverity.Warning, "Empty 'catch' / 'except' block suppresses all exceptions without logging or handling", "Swallowing errors silently can mask bugs and corrupt application state.", "Add logging or re-raise"));
        Register(new(SuggestVectorized, "Suggest vectorized kernel", "Quality", PilotDiagnosticSeverity.Hint, "Loop over numeric collection '{0}' can be accelerated using '@vectorized'", "Element-wise arithmetic loops can run on SIMD hardware kernels.", "Add '@vectorized' attribute"));
        Register(new(MatchNonExhaustive, "Non-exhaustive pattern match", "PatternMatching", PilotDiagnosticSeverity.Error, "Non-exhaustive match. Missing pattern: {0}", "Maranget usefulness matrix verification proved that one or more values of the scrutinee type are not covered by any unguarded match arm.", "Add missing match arm"));
        Register(new(MatchRedundantArm, "Unreachable / subsumed match arm", "PatternMatching", PilotDiagnosticSeverity.Warning, "Unreachable match arm: pattern '{0}' is completely subsumed by prior unguarded arms", "Prior unguarded match arms already cover every possible value matched by this arm.", "Remove unreachable match arm"));

        // NULL Series: Milestone 12 Concise Modern Null-Safety
        Register(new(NullAssignedToNonNullable, "Null or optional assigned to non-nullable type", "NullSafety", PilotDiagnosticSeverity.Error, "Cannot assign '{0}' to non-nullable type '{1}'. Change type to '{1}?' or provide a fallback with '??'.", "All standard Pilot types are strictly non-nullable by default; use 'T?' ('Option[T]') for optional values.", "Change type annotation to 'T?'"));
        Register(new(UncheckedOptionalAccess, "Unchecked member or index access on optional T?", "NullSafety", PilotDiagnosticSeverity.Error, "Cannot access '{0}' on optional expression '{1}' of type '{2}' without narrowing ('guard some', 'if some', '!= none') or safe navigation ('?.').", "Direct member access, indexing, or invocation on an unnarrowed optional 'T?' is rejected at compile time to eliminate NullReferenceException.", "Use safe navigation '?.' or narrow with 'guard some'"));
        Register(new(NullReturnedFromNonNullable, "Null or optional returned from non-nullable function", "NullSafety", PilotDiagnosticSeverity.Error, "Cannot return '{0}' from function '{1}' with non-nullable return type '{2}'.", "Functions declaring a non-nullable return type 'T' cannot return 'none'/'null' or an unnarrowed optional 'T?'.", "Change return type to 'T?' or narrow before returning"));
        Register(new(NullArgumentToNonNullable, "Null or optional argument passed to non-nullable parameter", "NullSafety", PilotDiagnosticSeverity.Error, "Cannot pass '{0}' to non-nullable parameter '{1}: {2}' of '{3}'.", "Passing 'none'/'null' or an unnarrowed optional 'T?' to a non-nullable parameter is rejected at compile time.", "Provide fallback with '??' or narrow argument first"));
        Register(new(UnsafeNonNullAssertionRemoved, "Unchecked postfix x! assertion is forbidden", "NullSafety", PilotDiagnosticSeverity.Error, "Unchecked postfix non-null assertion '{0}!' is forbidden in Pilot. Use 'guard some {0} else', '??', or explicit '{0}.OrPanic(\"reason\")'.", "Pilot removes unsafe postfix 'x!' assertions in favor of flow-sensitive narrowing ('guard some x else') or explicit '.OrPanic(\"reason\")'.", "Replace '!' with '.OrPanic(\"reason\")'"));

        // ERR Series: Milestone 13 Values-First Error System (T ! E)
        Register(new(UnhandledErrorReturn, "Unhandled fallible return value (must-use T ! E)", "ErrorHandling", PilotDiagnosticSeverity.Error, "Must-use error return: expression '{0}' returns fallible type '{1}' and cannot be ignored.", "Statement-level calls to fallible 'T ! E' functions must be handled via '?', 'catch', 'guard ... catch', 'match', assignment, or 'ignore_err(...)'.", "Add '?' propagation or 'catch' recovery"));
        Register(new(IncompatibleErrorPropagation, "Incompatible '?' error propagation or missing error adapter", "ErrorHandling", PilotDiagnosticSeverity.Error, "Cannot propagate error type '{0}' from '{1}?' into function '{2}' returning '{3}'.", "Postfix '?' requires the enclosing function to return a compatible fallible 'T ! E' type or declare an 'adapter E1 -> E2'.", "Declare 'adapter E1 -> E2' or widen return error union"));
        Register(new(NonExhaustiveCatchRecovery, "Non-exhaustive 'catch' error recovery", "ErrorHandling", PilotDiagnosticSeverity.Error, "Non-exhaustive 'catch' error recovery on '{0}' (error set '{1}'): unhandled error variant(s): {2}.", "A 'catch err => match err' recovery block must either handle all error variants in 'E' or re-propagate remaining variants via 'fail(err)'.", "Add missing error variant arm"));
        Register(new(InvalidFailReturn, "Invalid 'fail(...)' return or fallible return type mismatch", "ErrorHandling", PilotDiagnosticSeverity.Error, "Cannot return '{0}' from function '{1}' with return type '{2}'.", "Returning 'fail(err)' is only permitted inside fallible 'T ! E' or 'T!' functions whose error set covers 'err'.", "Change function return type to 'T ! E' or 'T!'"));
        Register(new(ErrorStratificationLint, "Legacy try/except/raise in native Pilot code", "ErrorHandling", PilotDiagnosticSeverity.Warning, "Error stratification lint: legacy '{0}' is discouraged in native Pilot code; use values-first 'T ! E' return errors with '?' / 'catch' for recoverable errors, 'panic(...)' for invariant bugs, or scope 'try/except' to FFI/isolate boundaries.", "Pilot stratifies recoverable errors into register-returned 'T ! E' values, panic(...) for invariant bugs, and isolates/FFI boundaries for foreign exceptions.", "Refactor to 'T ! E' return value"));

        // LOOP Series: Milestone 15 Dynamic-Schedule & Expression-Oriented For Loops
        Register(new(MixedBreakAndYieldInForLoop, "Cannot mix 'break <expr>' and 'yield <expr>' in same for loop", "ControlFlow", PilotDiagnosticSeverity.Error, "Cannot mix valued 'break <expr>' and 'yield <expr>' in the same 'for' loop.", "A 'for' expression must either exit early with a scalar value ('ScalarBreak') or accumulate elements ('CollectionYield'), not both.", "Use either 'break <expr>' or 'yield <expr>'"));
        Register(new(MixedVoidAndValuedBreakInForLoop, "Cannot mix 'break;' and 'break <expr>;' in same for loop", "ControlFlow", PilotDiagnosticSeverity.Error, "Cannot mix valueless 'break;' and valued 'break <expr>;' in the same 'for' loop.", "All 'break' statements targeting a 'ScalarBreak' loop must provide a value expression.", "Provide a value for every 'break' in this loop"));
        Register(new(InvalidForLoopThenBranch, "Loop completion 'then' branch is only valid with ScalarBreak", "ControlFlow", PilotDiagnosticSeverity.Error, "Loop completion 'then' branch is only valid on a ScalarBreak 'for' loop (containing 'break <expr>').", "A 'then <expr>' completion value branch provides the fallback scalar value when a loop finishes without 'break <expr>', and cannot be combined with 'CollectionYield' or 'Void' loops.", "Remove 'then' branch or use 'break <expr>'"));
        Register(new(YieldInStatementForLoopOutsideGenerator, "'yield <expr>' in statement-position for loop outside generator", "ControlFlow", PilotDiagnosticSeverity.Error, "'yield <expr>' inside a statement-position 'for' loop is invalid outside of a generator function. Assign or return the 'for' expression to use CollectionYield.", "In statement position, 'yield' delegates to an enclosing coroutine generator 'def'. To build a collection from a 'for' loop, use the 'for' loop in an expression position.", "Assign the 'for' loop result to a variable"));

        // TYPE Series: Milestone 16 Bidirectional, Flow-Sensitive & Row-Polymorphic Type Inference (TYPE001–TYPE025)
        for (int i = 1; i <= 25; i++)
        {
            string code = $"TYPE{i:D3}";
            Register(new(code, $"Type constraint violation ({code})", "TypeSystem", PilotDiagnosticSeverity.Error, "{0}", "Bidirectional, row-polymorphic, or flow-sensitive type unification constraint failed with provenance tracking."));
        }

        // QUOTE Series: Milestone 17 Homoiconic Call-Quoting & Multi-Modal Projection (QUOTE001–QUOTE005)
        Register(new(MissingWhiteBoxAstForDeepSplice, "Deep-splice call quote requires white-box AST body", "CallQuoting", PilotDiagnosticSeverity.Error, "Cannot deep-splice '{0}!!(...)': target function '{0}' is an external FFI or native intrinsic without a white-box Pilot AST body.", "Deep-splicing ('fn!!(...)') requires a white-box Pilot function AST body to clone, alpha-rename, and constant-fold.", "Use shallow call quote 'fn!(...)' instead"));
        Register(new(InvalidUnquotePrefixOutsideDeepSplice, "Unquote prefix '~' is only valid inside 'fn!!(...)'", "CallQuoting", PilotDiagnosticSeverity.Error, "Unquote prefix '~' on argument '{0}' is only valid inside a deep-splice call quote 'fn!!(...)'.", "Quasiquoted unquote arguments ('~arg') evaluate eagerly in caller scope and splice into a deep-quoted 'fn!!(...)' AST.", "Remove '~' or change call to 'fn!!(...)'"));
        Register(new(NonUnmanagedParameterInAbiCallFrame, "ABI Call-Frame requires unmanaged / repr(C) parameter layout", "CallQuoting", PilotDiagnosticSeverity.Error, "ABI Call-Frame '{0}!&(...)' requires unmanaged / repr(C) parameter layouts, but parameter '{1}' has type '{2}'.", "Zero-allocation C-ABI call frames ('fn!&(...)') pack arguments into a sequential struct and forbid managed heap references.", "Use unmanaged scalar/struct types or 'fn!(...)'"));
        Register(new(NondifferentiableAdjointCallQuote, "Symbolic adjoint call quote requires differentiable body", "CallQuoting", PilotDiagnosticSeverity.Error, "Cannot differentiate '{0}!@(...)': function body contains non-differentiable construct ({1}).", "Symbolic adjoint differentiation ('fn!@(...)') requires pure, bounded white-box expressions without unbounded loops or impure I/O."));
        Register(new(UnhandledEffectInDryRunQuote, "Reactive signal call quote requires observable input", "CallQuoting", PilotDiagnosticSeverity.Error, "Reactive signal call quote '{0}!^(...)' requires at least one observable input (Signal[T], object property path 'obj.Prop', or mutable 'var' binding).", "Reactive signal projection ('fn!^(...)') requires at least one observable dependency."));

        // COMPTIME Series: Milestone 19 Runtime-to-Compile-Time Demotion & Comptime Audit (COMPTIME001–COMPTIME010)
        Register(new(DynamicScopeLookupInZeroRuntime, "Dynamic variable lookup in '@zero_runtime' scope", "Comptime", PilotDiagnosticSeverity.Error, "{0}", "All variable accesses inside '@zero_runtime' / '@hard_realtime' scopes must lower to static local slots or lexical display upvalue indices."));
        Register(new(DynamicMemberLookupInZeroRuntime, "Dynamic dictionary field lookup in '@zero_runtime' scope", "Comptime", PilotDiagnosticSeverity.Error, "{0}", "Field access inside '@zero_runtime' / '@hard_realtime' scopes requires a statically known struct/class/record layout."));
        Register(new(DynamicMethodDispatchInZeroRuntime, "Unmonomorphized dynamic dispatch in '@zero_runtime' scope", "Comptime", PilotDiagnosticSeverity.Error, "{0}", "Method and contract calls inside '@zero_runtime' / '@hard_realtime' scopes must resolve statically."));
        Register(new(UnloweredDtoProjectionInZeroRuntime, "Dynamic DTO '->' projection in '@zero_runtime' scope", "Comptime", PilotDiagnosticSeverity.Error, "{0}", "DTO '->' projections inside '@zero_runtime' / '@hard_realtime' scopes require statically known source and target field slots."));
        Register(new(UnprovenRuntimeContractInZeroRuntime, "Unproven runtime contract in '@zero_runtime' scope", "Comptime", PilotDiagnosticSeverity.Error, "{0}", "Contracts inside '@zero_runtime' / '@hard_realtime' scopes must be statically discharged at compile time."));
        Register(new(DynamicContextMapInZeroRuntime, "Dynamic 'with' resource in '@zero_runtime' scope", "Comptime", PilotDiagnosticSeverity.Error, "{0}", "Only static 'with context(key = val)' display slots are permitted inside '@zero_runtime' / '@hard_realtime' scopes."));
        Register(new(AstFallbackInZeroRuntime, "AST interpreter fallback in '@zero_runtime' scope", "Comptime", PilotDiagnosticSeverity.Error, "{0}", "Constructs requiring 'EvalAst' interpreter fallback are forbidden inside '@zero_runtime' / '@hard_realtime' scopes."));
        Register(new(ComptimeAssertFailed, "Compile-time assertion failed", "Comptime", PilotDiagnosticSeverity.Error, "{0}", "A 'comptime assert' statement evaluated to false during compilation."));
        Register(new(ImpureOrExceededComptimeExecution, "Impure operation or step budget exceeded in 'comptime'", "Comptime", PilotDiagnosticSeverity.Error, "{0}", "Compile-time execution forbids non-deterministic I/O, FFI, and unbounded loops exceeding the step budget."));
        Register(new(NonConstantComptimeArgument, "Non-constant argument passed to 'comptime' parameter", "Comptime", PilotDiagnosticSeverity.Error, "{0}", "Parameters marked 'comptime' require a compile-time constant or 'comptime(...)' expression at every call site."));
    }

    public static PilotDiagnosticDescriptor? GetDescriptor(string code) =>
        Descriptors.TryGetValue(code, out var d) ? d : null;

    public static IReadOnlyDictionary<string, PilotDiagnosticDescriptor> GetAllDescriptors() => Descriptors;
}
