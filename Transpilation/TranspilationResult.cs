using System;
using System.Collections.Generic;

namespace Pilot.Transpilation;

/// <summary>
/// Encapsulates the output of a compile-time code conversion operation.
/// </summary>
public sealed class TranspilationResult
{
    public bool Success { get; }
    public string Code { get; }
    public string TargetLanguage { get; }
    public IReadOnlyList<string> Diagnostics { get; }

    public TranspilationResult(
        bool success,
        string code,
        string targetLanguage,
        IReadOnlyList<string> diagnostics)
    {
        Success = success;
        Code = code;
        TargetLanguage = targetLanguage;
        Diagnostics = diagnostics;
    }

    public static TranspilationResult CreateSuccess(string code, string targetLanguage) =>
        new(true, code, targetLanguage, Array.Empty<string>());

    public static TranspilationResult CreateFailure(string errorMessage, string targetLanguage) =>
        new(false, string.Empty, targetLanguage, new[] { errorMessage });

    public static TranspilationResult CreateFailure(IEnumerable<string> errors, string targetLanguage) =>
        new(false, string.Empty, targetLanguage, new List<string>(errors));
}
