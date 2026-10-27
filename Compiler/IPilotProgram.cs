using System;
using System.Collections.Generic;
using System.IO;
using Pilot.Runtime.Bytecode;
using Pilot.Runtime.Execution;

namespace Pilot.Compiler;

/// <summary>
/// Represents a compiled, executable Pilot program.
/// Can be executed repeatedly with zero re-parsing overhead and serialized to/from binary format.
/// </summary>
public interface IPilotProgram
{
    /// <summary>
    /// Gets the preprocessed source code of the program.
    /// </summary>
    string Source { get; }

    /// <summary>
    /// Gets the source file path if compiled from a file.
    /// </summary>
    string? FilePath { get; }

    /// <summary>
    /// Gets the timestamp when the program was compiled.
    /// </summary>
    DateTime CompiledAt { get; }

    /// <summary>
    /// Gets the preprocessor symbols defined when this program was compiled.
    /// </summary>
    IReadOnlyList<string> DefinedSymbols { get; }

    /// <summary>
    /// Gets the compiled bytecode chunk for PilVM execution, if available.
    /// </summary>
    PilotBytecodeChunk? BytecodeChunk { get; }

    /// <summary>
    /// Runs the compiled program with the specified execution options.
    /// </summary>
    PilotExecutionResult Run(PilotExecutionOptions? options = null);

    /// <summary>
    /// Runs the compiled program inside an existing execution context.
    /// </summary>
    PilotExecutionResult Run(PilotExecutionContext context);

    /// <summary>
    /// Runs the compiled program in scripting mode with command-line arguments.
    /// </summary>
    PilotExecutionResult RunWithArgs(IReadOnlyList<string> scriptArgs, PilotExecutionOptions? options = null);

    /// <summary>
    /// Serializes the compiled program to a binary stream (.pilc).
    /// </summary>
    void Save(Stream stream);

    /// <summary>
    /// Serializes the compiled program to a disk file (.pilc).
    /// </summary>
    void Save(string filePath);

    /// <summary>
    /// Returns the compiled program as serialized binary bytes.
    /// </summary>
    byte[] ToBytes();
}
