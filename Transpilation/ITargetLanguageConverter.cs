namespace Pilot.Transpilation;

/// <summary>
/// Combined bidirectional contract for compile-time conversion of code to and from a target language.
/// Follows a decoupled, plug-and-play philosophy.
/// </summary>
public interface ITargetLanguageConverter : ITargetLanguageTranspiler, ITargetLanguageIngestor
{
    /// <summary>
    /// The target language identifier (e.g. "python", "csharp", "rust").
    /// </summary>
    string Language { get; }
}
