using System;
using System.Collections.Generic;
using Pilot.Transpilation.Licensing;

namespace Pilot.Transpilation;

/// <summary>
/// Encapsulates the output of a compile-time code conversion or ingestion operation.
/// </summary>
public sealed class TranspilationResult
{
    public bool Success { get; }
    public string Code { get; }
    public string TargetLanguage { get; }
    public IReadOnlyList<string> Diagnostics { get; }
    public PilotLicenseCheckResult? LicenseInfo { get; init; }

    public TranspilationResult(
        bool success,
        string code,
        string targetLanguage,
        IReadOnlyList<string> diagnostics,
        PilotLicenseCheckResult? licenseInfo = null)
    {
        Success = success;
        Code = code;
        TargetLanguage = targetLanguage;
        Diagnostics = diagnostics;
        LicenseInfo = licenseInfo;
    }

    public static TranspilationResult CreateSuccess(
        string code,
        string targetLanguage,
        PilotLicenseCheckResult? licenseInfo = null) =>
        new(true, code, targetLanguage, Array.Empty<string>(), licenseInfo);

    public static TranspilationResult CreateFailure(
        string errorMessage,
        string targetLanguage,
        PilotLicenseCheckResult? licenseInfo = null) =>
        new(false, string.Empty, targetLanguage, new[] { errorMessage }, licenseInfo);

    public static TranspilationResult CreateFailure(
        IEnumerable<string> errors,
        string targetLanguage,
        PilotLicenseCheckResult? licenseInfo = null) =>
        new(false, string.Empty, targetLanguage, new List<string>(errors), licenseInfo);
}
