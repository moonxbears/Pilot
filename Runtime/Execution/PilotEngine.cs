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

            var tree = parser.program();

            if (errorTracker.HasErrors)
            {
                return PilotExecutionResult.CreateFailure(
                    errorTracker.Errors,
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

            var visitor = new PilotInterpreterVisitor(context);
            var resultValue = visitor.VisitProgram(tree);

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
        return Execute(source, options);
    }

    public PilotValue EvaluateExpression(string expression, PilotScope? scope = null)
    {
        var options = new PilotExecutionOptions();
        var context = new PilotExecutionContext(this, options, new PilotEnvironment(scope));

        foreach (var kvp in options.Builtins.GetAll())
        {
            context.Environment.GlobalScope.Declare(kvp.Key, PilotValue.FromFunction(kvp.Value));
        }

        var inputStream = new AntlrInputStream(expression);
        var lexer = new PilotLexer(inputStream);
        var tokenStream = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokenStream);

        var errorTracker = new ErrorTracker();
        parser.RemoveErrorListeners();
        parser.AddErrorListener(errorTracker);

        var tree = parser.test();

        if (errorTracker.HasErrors)
        {
            throw new PilotRuntimeException($"Parse error in expression: {errorTracker.Errors[0]}");
        }

        var visitor = new PilotInterpreterVisitor(context);
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

        // Function local scope with parent set to its lexical closure scope or global scope
        var callScope = new PilotScope(function.ClosureScope ?? context.Environment.GlobalScope, $"Call:{function.Name}");

        // Bind parameters
        var parameters = function.Parameters;
        int argIdx = 0;

        for (int i = 0; i < parameters.Count; i++)
        {
            var p = parameters[i];
            if (p.IsParams)
            {
                var restArgs = new List<PilotValue>();
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
            else if (p.DefaultValue.HasValue)
            {
                callScope.Declare(p.Name, p.DefaultValue.Value);
            }
            else
            {
                callScope.Declare(p.Name, PilotValue.Undefined);
            }
        }

        // Bind captured variables if any
        if (function.Captures != null)
        {
            foreach (var kvp in function.Captures)
            {
                callScope.Declare(kvp.Key, kvp.Value);
            }
        }

        // Swap scope in environment
        var previousScope = context.Environment.CurrentScope;
        var functionContext = new PilotExecutionContext(
            this,
            context.Options,
            new PilotEnvironment(callScope),
            context.CancellationToken);
        var methodVisitor = new PilotInterpreterVisitor(functionContext);

        Pilot.Debugging.PilotStackFrame? debugFrame = null;
        if (context.Debugger != null)
        {
            debugFrame = context.Debugger.OnFunctionEnter(function.Name, arguments, callScope, functionContext);
        }

        PilotValue resultVal = PilotValue.Undefined;
        try
        {
            if (function.AstBody is PilotParser.SuiteContext suiteCtx)
            {
                resultVal = methodVisitor.VisitSuite(suiteCtx);
            }
            else if (function.AstBody is PilotParser.TestContext testCtx)
            {
                resultVal = methodVisitor.Visit(testCtx);
            }
            else
            {
                resultVal = methodVisitor.Visit(function.AstBody!);
            }

            return resultVal;
        }
        catch (PilotReturnSignal ret)
        {
            resultVal = ret.Value;
            return resultVal;
        }
        finally
        {
            functionContext.ExecuteDefers();
            context.CallDepth--;
            if (context.Debugger != null && debugFrame != null)
            {
                context.Debugger.OnFunctionExit(debugFrame, resultVal, functionContext);
            }
        }
    }

    public PilotValue CreateGenerator(
        PilotFunction function,
        IReadOnlyList<PilotValue> arguments,
        PilotExecutionContext context)
    {
        var callScope = new PilotScope(function.ClosureScope ?? context.Environment.GlobalScope, $"Gen:{function.Name}");
        var parameters = function.Parameters;
        int argIdx = 0;

        for (int i = 0; i < parameters.Count; i++)
        {
            var p = parameters[i];
            if (p.IsParams)
            {
                var restArgs = new List<PilotValue>();
                while (argIdx < arguments.Count) restArgs.Add(arguments[argIdx++]);
                callScope.Declare(p.Name, PilotValue.FromList(restArgs));
            }
            else if (argIdx < arguments.Count) callScope.Declare(p.Name, arguments[argIdx++]);
            else if (p.DefaultValue.HasValue) callScope.Declare(p.Name, p.DefaultValue.Value);
            else callScope.Declare(p.Name, PilotValue.Undefined);
        }

        if (function.Captures != null)
        {
            foreach (var kvp in function.Captures) callScope.Declare(kvp.Key, kvp.Value);
        }

        var cts = new CancellationTokenSource();
        var runnerResumeEvent = new AutoResetEvent(false);
        var callerResumeEvent = new AutoResetEvent(false);

        PilotValue currentYielded = PilotValue.Undefined;
        PilotValue currentInput = PilotValue.Undefined;
        Exception? executionException = null;
        bool isFinished = false;
        bool isStarted = false;
        var syncLock = new object();

        void RunnerBody()
        {
            try
            {
                var genContext = new PilotExecutionContext(
                    this,
                    context.Options,
                    new PilotEnvironment(callScope),
                    cts.Token);

                genContext.YieldHandler = yieldedVal =>
                {
                    currentYielded = yieldedVal;
                    callerResumeEvent.Set();
                    runnerResumeEvent.WaitOne();
                    if (cts.Token.IsCancellationRequested)
                    {
                        throw new OperationCanceledException();
                    }
                    return currentInput;
                };

                var genVisitor = new PilotInterpreterVisitor(genContext);

                if (function.AstBody is PilotParser.SuiteContext suiteCtx)
                {
                    genVisitor.VisitSuite(suiteCtx);
                }
                else if (function.AstBody != null)
                {
                    genVisitor.Visit(function.AstBody);
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
                executionException = ex;
            }
            finally
            {
                lock (syncLock)
                {
                    isFinished = true;
                }
                callerResumeEvent.Set();
            }
        }

        PilotGenerator? gen = null;
        gen = new PilotGenerator(function.Name, input =>
        {
            lock (syncLock)
            {
                if (isFinished)
                {
                    throw new PilotReturnSignal(PilotValue.Undefined);
                }

                currentInput = input;
                if (!isStarted)
                {
                    isStarted = true;
                    Task.Factory.StartNew(RunnerBody, TaskCreationOptions.LongRunning);
                }
                else
                {
                    runnerResumeEvent.Set();
                }
            }

            callerResumeEvent.WaitOne();

            if (executionException != null)
            {
                throw executionException;
            }

            lock (syncLock)
            {
                if (isFinished)
                {
                    throw new PilotReturnSignal(PilotValue.Undefined);
                }
            }

            return currentYielded;
        }, () =>
        {
            lock (syncLock)
            {
                if (!isFinished)
                {
                    cts.Cancel();
                    runnerResumeEvent.Set();
                }
            }
        });

        return PilotValue.FromGenerator(gen);
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
