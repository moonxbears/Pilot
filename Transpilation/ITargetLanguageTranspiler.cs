namespace Pilot.Transpilation;

/// <summary>
/// Converts Pilot source code or parse tree into a target language source representation.
/// </summary>
public interface ITargetLanguageTranspiler
{
    /// <summary>
    /// Identifier of the target language (e.g. "python", "csharp", "rust").
    /// </summary>
    string TargetLanguage { get; }

    /// <summary>
    /// Transpiles a parsed Pilot AST program into the target language.
    /// </summary>
    TranspilationResult Transpile(PilotParser.ProgramContext programAst, TranspilationOptions options);

    /// <summary>
    /// Transpiles raw Pilot source code into the target language.
    /// </summary>
    TranspilationResult TranspileSource(string pilotSource, TranspilationOptions options);
}
