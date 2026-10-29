using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using Antlr4.Runtime;
using Pilot.Diagnostics;
using Pilot.Preprocessing;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Execution;

/// <summary>
/// Default implementation of <see cref="IPilotEngine"/>.
/// Decoupled, configurable, flexible, and designed for high execution speed.
/// </summary>
public sealed class PilotEngine : IPilotEngine
{
    public PilotExecutionResult Execute(string source, Pilot.Runtime.Environment.PilotScope initialScope)
    {
        var options = new PilotExecutionOptions { InitialScope = initialScope };
        return Execute(source, options);
    }

    public PilotExecutionResult Execute(string source, PilotExecutionOptions? options = null)
    {
        var sw = Stopwatch.StartNew();
        options ??= new PilotExecutionOptions();

        // Capture standard output and error in memory buffers if default consoles were not explicitly retained
        var stdoutCapture = new StringWriter();
        var stderrCapture = new StringWriter();

        var originalStdOut = options.StandardOutput;
        var originalStdErr = options.StandardError;

        var mergedStdOut = new MergedTextWriter(originalStdOut, stdoutCapture);
        var mergedStdErr = new MergedTextWriter(originalStdErr, stderrCapture);

        options.StandardOutput = mergedStdOut;
        options.StandardError = mergedStdErr;

        try
        {
            string preprocessedCode = source;
            if (options.EnablePreprocessing)
            {
                var preprocOpts = new PilotPreprocessorOptions(options.DefinedSymbols);
                var preprocResult = PilotPreprocessor.Preprocess(source, "", preprocOpts);

                if (preprocResult.HasErrors)
                {
                    var errors = preprocResult.Diagnostics
                        .Where(d => d.Severity == PreprocessorDiagnosticSeverity.Error)
                        .Select(e => $"[Preproc {e.Line}:{e.Column}] {e.Message}")
                        .ToList();

                    return PilotExecutionResult.CreateFailure(
                        errors,
                        stdoutCapture.ToString(),
                        stderrCapture.ToString(),
                        sw.Elapsed);
                }

                preprocessedCode = preprocResult.PreprocessedSource;
            }

            var inputStream = new AntlrInputStream(preprocessedCode);
            var lexer = new PilotLexer(inputStream);
            var tokenStream = new CommonTokenStream(lexer);
            var parser = new PilotParser(tokenStream);

            var errorTracker = new ErrorTracker();
            parser.RemoveErrorListeners();
            parser.AddErrorListener(errorTracker);

            var tree = parser.ParseProgramFast();

            if (errorTracker.HasErrors)
            {
                return PilotExecutionResult.CreateFailure(
                    errorTracker.Errors,
                    stdoutCapture.ToString(),
                    stderrCapture.ToString(),
                    sw.Elapsed);
            }

            var matchDiagnostics = Pilot.Analysis.PatternMatching.PilotExhaustivenessChecker.CheckProgram(tree);
            var matchErrors = matchDiagnostics
                .Where(d => d.IsError)
                .Select(d => d.ToString())
                .ToList();
            if (matchErrors.Count > 0)
            {
                return PilotExecutionResult.CreateFailure(
                    matchErrors,
                    stdoutCapture.ToString(),
                    stderrCapture.ToString(),
                    sw.Elapsed);
            }

            var context = options.InitialScope != null
                ? new PilotExecutionContext(this, options, new PilotEnvironment(options.InitialScope))
                : new PilotExecutionContext(this, options);

            // Register default builtins into global scope
            foreach (var kvp in options.Builtins.GetAll())
            {
                context.Environment.GlobalScope.Declare(kvp.Key, PilotValue.FromFunction(kvp.Value));
            }

            if (!string.IsNullOrWhiteSpace(options.SourceFilePath) &&
                !context.Environment.GlobalScope.ContainsLocal("__file__"))
            {
                context.Environment.GlobalScope.Declare("__file__", PilotValue.FromString(Path.GetFullPath(options.SourceFilePath)));
            }

            var visitor = context.GetOrCreateVisitor();
            var resultValue = visitor.VisitProgram(tree);
            if (context.PendingFlow == PilotFlowSignalKind.Return)
            {
                resultValue = context.PendingReturnValue;
                context.PendingFlow = PilotFlowSignalKind.None;
            }

            var execResult = PilotExecutionResult.CreateSuccess(
                resultValue,
                stdoutCapture.ToString(),
                stderrCapture.ToString(),
                sw.Elapsed);
            execResult.Context = context;
            return execResult;
        }
        catch (PilotRuntimeException rex)
        {
            return PilotExecutionResult.CreateFailure(
                rex.Message,
                stdoutCapture.ToString(),
                stderrCapture.ToString(),
                sw.Elapsed);
        }
        catch (Exception ex)
        {
            return PilotExecutionResult.CreateFailure(
                ex.Message,
                stdoutCapture.ToString(),
                stderrCapture.ToString(),
                sw.Elapsed);
        }
        finally
        {
            options.StandardOutput = originalStdOut;
            options.StandardError = originalStdErr;
        }
    }

    public PilotExecutionResult ExecuteFile(string filePath, PilotExecutionOptions? options = null)
    {
        if (!File.Exists(filePath))
        {
            return PilotExecutionResult.CreateFailure(
                $"File not found: '{filePath}'",
                string.Empty,
                string.Empty,
                TimeSpan.Zero);
        }

        string source = File.ReadAllText(filePath);
        options ??= new PilotExecutionOptions();
        options.SourceFilePath ??= Path.GetFullPath(filePath);
        return Execute(source, options);
    }

    public PilotValue EvaluateExpression(string expression, PilotScope? scope = null)
    {
        var options = new PilotExecutionOptions();
        var context = new PilotExecutionContext(this, options, new PilotEnvironment(scope));

        if (!context.Environment.GlobalScope.ContainsLocal("print"))
        {
            foreach (var kvp in options.Builtins.GetAll())
            {
                context.Environment.GlobalScope.Declare(kvp.Key, PilotValue.FromFunction(kvp.Value));
            }
        }

        var tree = PilotParser.GetOrParseExpression(expression);
        var visitor = context.GetOrCreateVisitor();
        return visitor.Visit(tree);
    }

    public PilotValue InvokeFunction(
        PilotFunction function,
        IReadOnlyList<PilotValue> arguments,
        PilotExecutionContext context)
    {
        if (context.CallDepth >= context.Options.MaxCallDepth)
        {
            throw new PilotRecursionException($"Stack overflow: exceeded maximum call depth ({context.Options.MaxCallDepth}).");
        }

        context.CallDepth++;

        var callScope = CreateCallScope(function, arguments, context, "Call");
        var previousScope = context.Environment.SwapScope(callScope);
        var previousFunction = context.CurrentFunction;
        context.CurrentFunction = function;
        int deferWatermark = context.DeferStack.Count;
        int activeExtWatermark = context.ActiveExtensions.Count;
        int ctxBindWatermark = context.ContextualBindings.Count;

        var methodVisitor = context.GetOrCreateVisitor();

        // Design-by-Contract: 1. Evaluate Preconditions (requires)
        if (function.HasRequiresContracts && !function.ContractsStaticallyDischarged)
        {
            var reqs = function.RequiresContracts;
            for (int i = 0; i < reqs.Count; i++)
            {
                var req = reqs[i];
                var reqVal = methodVisitor.Visit(req);
                if (!reqVal.IsTrue)
                {
                    context.Environment.SwapScope(previousScope);
                    context.CallDepth--;
                    throw new Pilot.Runtime.ControlFlow.PilotContractException(
                        $"Precondition failed for '{function.Name}': requires {req.GetText()}");
                }
            }
        }

        // Design-by-Contract: 2. Pre-evaluate and snapshot old(...) expressions only if EnsuresContracts exist
        Dictionary<string, PilotValue>? previousSnapshots = null;
        bool hasEnsures = function.HasEnsuresContracts && !function.ContractsStaticallyDischarged;
        if (hasEnsures)
        {
            var snapshots = new Dictionary<string, PilotValue>(StringComparer.Ordinal);
            var ensures = function.EnsuresContracts;
            for (int i = 0; i < ensures.Count; i++)
            {
                SnapshotOldCalls(ensures[i], methodVisitor, snapshots);
            }
            previousSnapshots = context.SwapOldSnapshots(snapshots);
        }

        void ValidatePostconditions(PilotValue ret)
        {
            if (hasEnsures)
            {
                callScope.Declare("result", ret);
                var ensures = function.EnsuresContracts;
                for (int i = 0; i < ensures.Count; i++)
                {
                    var ens = ensures[i];
                    var ensVal = methodVisitor.Visit(ens);
                    if (!ensVal.IsTrue)
                    {
                        throw new Pilot.Runtime.ControlFlow.PilotContractException(
                            $"Postcondition failed for '{function.Name}': ensures {ens.GetText()}");
                    }
                }
            }
        }

        Pilot.Debugging.PilotStackFrame? debugFrame = null;
        if (context.Debugger != null)
        {
            debugFrame = context.Debugger.OnFunctionEnter(function.Name, arguments, callScope, context);
        }

        PilotValue resultVal = PilotValue.Undefined;
        try
        {
            if (function.BytecodeChunk != null && !function.IsGenerator)
            {
                resultVal = Pilot.Runtime.Bytecode.PilVirtualMachine.Execute(function.BytecodeChunk, context, arguments, function);
            }
            else if (function.AstBody is PilotParser.SuiteContext suiteCtx)
            {
                resultVal = methodVisitor.VisitSuite(suiteCtx);
            }
            else if (function.AstBody is PilotParser.TestContext testCtx)
            {
                resultVal = methodVisitor.Visit(testCtx);
            }
            else
            {
                if (function.AstBody is PilotParser.Lambda_exprContext lamCtx)
                {
                    var stmts = lamCtx.stmt()?.Length > 0 ? lamCtx.stmt() : lamCtx.lambda_body()?.stmt();
                    if (stmts != null)
                    {
                        foreach (var s in stmts)
                        {
                            resultVal = methodVisitor.Visit(s);
                            if (context.PendingFlow != PilotFlowSignalKind.None)
                                break;
                        }
                    }
                }
                else
                {
                    resultVal = methodVisitor.Visit(function.AstBody!);
                }
            }

            if (context.PendingFlow == PilotFlowSignalKind.Return)
            {
                resultVal = context.PendingReturnValue;
                context.PendingFlow = PilotFlowSignalKind.None;
            }

            ValidatePostconditions(resultVal);
            return resultVal;
        }
        catch (PilotReturnSignal ret)
        {
            resultVal = ret.Value;
            ValidatePostconditions(resultVal);
            return resultVal;
        }
        finally
        {
            context.ExecuteDefersDownTo(deferWatermark);
            while (context.ActiveExtensions.Count > activeExtWatermark)
            {
                context.ActiveExtensions.Pop();
            }
            while (context.ContextualBindings.Count > ctxBindWatermark)
            {
                context.ContextualBindings.Pop();
            }
            if (hasEnsures)
            {
                context.SwapOldSnapshots(previousSnapshots);
            }
            context.CurrentFunction = previousFunction;
            context.Environment.SwapScope(previousScope);
            context.CallDepth--;
            if (context.Debugger != null && debugFrame != null)
            {
                context.Debugger.OnFunctionExit(debugFrame, resultVal, context);
            }
        }
    }

    public PilotValue CreateGenerator(
        PilotFunction function,
        IReadOnlyList<PilotValue> arguments,
        PilotExecutionContext context)
    {
        var callScope = CreateCallScope(function, arguments, context, "Gen");
        var runner = new GeneratorRunner(this, function, context, callScope);
        return PilotValue.FromGenerator(runner.CreateGenerator());
    }

    private class GeneratorRunner
    {
        private readonly PilotEngine _engine;
        private readonly PilotFunction _function;
        private readonly PilotExecutionContext _callerContext;
        private readonly PilotScope _callScope;

        private readonly CancellationTokenSource _cts = new CancellationTokenSource();
        private readonly AutoResetEvent _runnerResumeEvent = new AutoResetEvent(false);
        private readonly AutoResetEvent _callerResumeEvent = new AutoResetEvent(false);

        private PilotValue _currentYielded = PilotValue.Undefined;
        private PilotValue _currentInput = PilotValue.Undefined;
        private Exception? _executionException = null;
        private bool _isFinished = false;
        private bool _isStarted = false;
        private readonly object _syncLock = new object();

        public GeneratorRunner(
            PilotEngine engine,
            PilotFunction function,
            PilotExecutionContext callerContext,
            PilotScope callScope)
        {
            _engine = engine;
            _function = function;
            _callerContext = callerContext;
            _callScope = callScope;
        }

        public PilotGenerator CreateGenerator()
        {
            return new PilotGenerator(_function.Name, Next, Close);
        }

        private void RunnerBody()
        {
            try
            {
                var genContext = new PilotExecutionContext(
                    _engine,
                    _callerContext.Options,
                    new PilotEnvironment(_callScope),
                    _cts.Token);

                genContext.YieldHandler = yieldedVal =>
                {
                    _currentYielded = yieldedVal;
                    _callerResumeEvent.Set();
                    _runnerResumeEvent.WaitOne();
                    if (_cts.Token.IsCancellationRequested)
                    {
                        throw new OperationCanceledException();
                    }
                    return _currentInput;
                };

                var genVisitor = genContext.GetOrCreateVisitor();

                if (_function.AstBody is PilotParser.SuiteContext suiteCtx)
                {
                    genVisitor.VisitSuite(suiteCtx);
                }
                else if (_function.AstBody != null)
                {
                    genVisitor.Visit(_function.AstBody);
                }
            }
            catch (OperationCanceledException)
            {
                // generator cancelled / closed
            }
            catch (PilotReturnSignal)
            {
                // return inside generator finishes it
            }
            catch (Exception ex)
            {
                _executionException = ex;
            }
            finally
            {
                lock (_syncLock)
                {
                    _isFinished = true;
                }
                _callerResumeEvent.Set();
            }
        }

        private PilotValue Next(PilotValue input)
        {
            lock (_syncLock)
            {
                if (_isFinished) throw new PilotReturnSignal(PilotValue.Undefined);

                _currentInput = input;
                if (!_isStarted)
                {
                    _isStarted = true;
                    Task.Factory.StartNew(RunnerBody, TaskCreationOptions.LongRunning);
                }
                else
                {
                    _runnerResumeEvent.Set();
                }
            }

            _callerResumeEvent.WaitOne();

            if (_executionException != null) throw _executionException;

            lock (_syncLock)
            {
                if (_isFinished) throw new PilotReturnSignal(PilotValue.Undefined);
            }

            return _currentYielded;
        }

        private void Close()
        {
            lock (_syncLock)
            {
                if (!_isFinished)
                {
                    _cts.Cancel();
                    _runnerResumeEvent.Set();
                }
            }
        }
    }

    public Pilot.Transpilation.TranspilationRegistry Transpilers { get; set; } = Pilot.Transpilation.TranspilationRegistry.Default;

    public Pilot.Transpilation.TranspilationResult TranspileTo(
        string pilotSource,
        string targetLanguage,
        Pilot.Transpilation.TranspilationOptions? options = null) =>
        Transpilers.Transpile(pilotSource, targetLanguage, options);

    public Pilot.Transpilation.TranspilationResult TranspileFrom(
        string foreignSource,
        string sourceLanguage,
        Pilot.Transpilation.TranspilationOptions? options = null) =>
        Transpilers.Ingest(foreignSource, sourceLanguage, options);

    public Pilot.Compiler.IPilotProgram Compile(string source, Pilot.Compiler.PilotCompilerOptions? options = null)
    {
        var result = Pilot.Compiler.PilotCompiler.Default.Compile(source, options);
        if (!result.Success || result.Program == null)
        {
            throw new InvalidOperationException($"Compilation failed: {string.Join("; ", result.Diagnostics)}");
        }
        return result.Program;
    }

    public Pilot.Compiler.IPilotProgram CompileFile(string filePath, Pilot.Compiler.PilotCompilerOptions? options = null)
    {
        var result = Pilot.Compiler.PilotCompiler.Default.CompileFile(filePath, options);
        if (!result.Success || result.Program == null)
        {
            throw new InvalidOperationException($"Compilation failed for '{filePath}': {string.Join("; ", result.Diagnostics)}");
        }
        return result.Program;
    }

    private static PilotScope CreateCallScope(
        PilotFunction function,
        IReadOnlyList<PilotValue> arguments,
        PilotExecutionContext context,
        string scopePrefix)
    {
        var callScope = new PilotScope(function.ClosureScope ?? context.Environment.GlobalScope, $"{scopePrefix}:{function.Name}");
        var parameters = function.Parameters;
        int argIdx = 0;

        for (int i = 0; i < parameters.Count; i++)
        {
            var p = parameters[i];
            if (p.IsParams)
            {
                var restArgs = new List<PilotValue>(Math.Max(0, arguments.Count - argIdx));
                while (argIdx < arguments.Count)
                {
                    restArgs.Add(arguments[argIdx++]);
                }
                callScope.Declare(p.Name, PilotValue.FromList(restArgs));
            }
            else if (argIdx < arguments.Count)
            {
                callScope.Declare(p.Name, arguments[argIdx++]);
            }
            else if (p.IsContextual)
            {
                if (context.TryResolveContextual(p.Name, out var contextualVal))
                {
                    callScope.Declare(p.Name, contextualVal);
                }
                else if (p.DefaultValue.HasValue)
                {
                    callScope.Declare(p.Name, p.DefaultValue.Value);
                }
                else
                {
                    throw new Pilot.Runtime.ControlFlow.PilotContextException(
                        $"Contextual parameter '{p.Name}' required by function '{function.Name}' could not be resolved from caller context.");
                }
            }
            else if (p.DefaultValue.HasValue)
            {
                callScope.Declare(p.Name, p.DefaultValue.Value);
            }
            else
            {
                callScope.Declare(p.Name, PilotValue.Undefined);
            }
        }

        if (function.Captures != null)
        {
            foreach (var kvp in function.Captures)
            {
                callScope.Declare(kvp.Key, kvp.Value);
            }
        }

        return callScope;
    }

    private static void SnapshotOldCalls(
        Antlr4.Runtime.Tree.IParseTree node,
        PilotInterpreterVisitor visitor,
        Dictionary<string, PilotValue> snapshots)
    {
        if (node is PilotParser.Atom_exprContext atomExpr)
        {
            if (atomExpr.atom()?.GetText() == "old" && atomExpr.trailer()?.Length > 0 && atomExpr.trailer(0).LPAR() != null)
            {
                var arglist = atomExpr.trailer(0).arglist();
                if (arglist != null)
                {
                    string key = arglist.GetText();
                    if (!snapshots.ContainsKey(key))
                    {
                        var argNode = arglist.argument()?.FirstOrDefault()?.test();
                        if (argNode != null)
                        {
                            var evaluated = visitor.Visit(argNode);
                            snapshots[key] = evaluated.CloneValue();
                        }
                    }
                }
            }
        }

        for (int i = 0; i < node.ChildCount; i++)
        {
            SnapshotOldCalls(node.GetChild(i), visitor, snapshots);
        }
    }
}

internal sealed class MergedTextWriter : TextWriter
{
    private readonly TextWriter _primary;
    private readonly TextWriter _secondary;

    public MergedTextWriter(TextWriter primary, TextWriter secondary)
    {
        _primary = primary;
        _secondary = secondary;
    }

    public override System.Text.Encoding Encoding => _primary.Encoding;

    public override void Write(char value)
    {
        _primary.Write(value);
        _secondary.Write(value);
    }

    public override void Write(string? value)
    {
        _primary.Write(value);
        _secondary.Write(value);
    }

    public override void WriteLine(string? value)
    {
        _primary.WriteLine(value);
        _secondary.WriteLine(value);
    }

    public override void Flush()
    {
        _primary.Flush();
        _secondary.Flush();
    }
}
