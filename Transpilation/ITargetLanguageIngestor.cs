namespace Pilot.Transpilation;

/// <summary>
/// Converts source code from a target language into equivalent Pilot source code.
/// </summary>
public interface ITargetLanguageIngestor
{
    /// <summary>
    /// Identifier of the source language (e.g. "python", "csharp", "rust").
    /// </summary>
    string SourceLanguage { get; }

    /// <summary>
    /// Ingests source code written in the target language and produces equivalent Pilot source code.
    /// </summary>
    TranspilationResult IngestToPilot(string foreignSource, TranspilationOptions options);
}
