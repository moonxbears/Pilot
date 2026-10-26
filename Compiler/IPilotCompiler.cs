using System.IO;

namespace Pilot.Compiler;

/// <summary>
/// Contract for compiling Pilot source code into reusable, executable programs.
/// </summary>
public interface IPilotCompiler
{
    /// <summary>
    /// Compiles Pilot source code into an executable program.
    /// </summary>
    PilotCompilationResult Compile(string source, PilotCompilerOptions? options = null);

    /// <summary>
    /// Compiles a Pilot source file on disk into an executable program.
    /// </summary>
    PilotCompilationResult CompileFile(string filePath, PilotCompilerOptions? options = null);

    /// <summary>
    /// Loads a pre-compiled program from a binary stream (.pilc).
    /// </summary>
    IPilotProgram LoadCompiled(Stream stream);

    /// <summary>
    /// Loads a pre-compiled program from a disk file (.pilc).
    /// </summary>
    IPilotProgram LoadCompiled(string filePath);

    /// <summary>
    /// Loads a pre-compiled program from raw byte payload (.pilc).
    /// </summary>
    IPilotProgram LoadCompiled(byte[] bytes);
}
