using System.Collections.Generic;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Execution;

/// <summary>
/// Primary plug-and-play contract for executing Pilot scripts, expressions, and functions.
/// </summary>
public interface IPilotEngine
{
    /// <summary>
    /// Executes source code with the given execution options.
    /// </summary>
    PilotExecutionResult Execute(string source, PilotExecutionOptions? options = null);

    /// <summary>
    /// Executes a file from disk with the given execution options.
    /// </summary>
    PilotExecutionResult ExecuteFile(string filePath, PilotExecutionOptions? options = null);

    /// <summary>
    /// Evaluates a standalone expression string and returns the resulting value.
    /// </summary>
    PilotValue EvaluateExpression(string expression, PilotScope? scope = null);

    /// <summary>
    /// Invokes a user-defined or native Pilot function with arguments.
    /// </summary>
    PilotValue InvokeFunction(PilotFunction function, IReadOnlyList<PilotValue> arguments, PilotExecutionContext context);

    /// <summary>
    /// Instantiates a stateful generator from a generator function.
    /// </summary>
    PilotValue CreateGenerator(PilotFunction function, IReadOnlyList<PilotValue> arguments, PilotExecutionContext context);

    /// <summary>
    /// Transpiles Pilot source code to a target language (e.g. "python", "csharp", "rust").
    /// </summary>
    Pilot.Transpilation.TranspilationResult TranspileTo(string pilotSource, string targetLanguage, Pilot.Transpilation.TranspilationOptions? options = null);

    /// <summary>
    /// Ingests code from a target language into equivalent Pilot source code.
    /// </summary>
    Pilot.Transpilation.TranspilationResult TranspileFrom(string foreignSource, string sourceLanguage, Pilot.Transpilation.TranspilationOptions? options = null);

    /// <summary>
    /// Compiles Pilot source code into a reusable, executable program.
    /// </summary>
    Pilot.Compiler.IPilotProgram Compile(string source, Pilot.Compiler.PilotCompilerOptions? options = null);

    /// <summary>
    /// Compiles a Pilot source file on disk into a reusable, executable program.
    /// </summary>
    Pilot.Compiler.IPilotProgram CompileFile(string filePath, Pilot.Compiler.PilotCompilerOptions? options = null);
}
