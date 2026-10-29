using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;

namespace Pilot.Transpilation.Licensing;

/// <summary>
/// License enforcement policy for foreign code and library ingestion (<c>pilot --from</c>).
/// </summary>
public enum PilotLicensePolicy
{
    /// <summary>
    /// Default policy: allow any license (including unlicensed/internal/private codebases).
    /// </summary>
    Any = 0,

    /// <summary>
    /// Restricts ingestion to OSI-approved permissive, redistributable licenses
    /// (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, PSF-2.0, Zlib, 0BSD, CC0-1.0, Unlicense).
    /// Rejects copyleft (GPL/LGPL/AGPL/SSPL/EUPL/MPL), proprietary, or missing licenses.
    /// Activated via <c>--viable-license-only</c> or <c>--license-policy permissive</c>.
    /// </summary>
    Permissive = 1,

    /// <summary>
    /// Restricts ingestion to a caller-supplied SPDX identifier allowlist.
    /// Activated via <c>--license-policy spdx-allowlist</c>.
    /// </summary>
    SpdxAllowlist = 2
}

/// <summary>
/// Result of verifying the license of a foreign source file or library directory.
/// </summary>
public sealed class PilotLicenseCheckResult
{
    public bool IsAllowed { get; init; }
    public string SpdxIdentifier { get; init; } = "NOASSERTION";
    public string DetectedSource { get; init; } = "";
    public string DetectedFromLocation => DetectedSource;
    public string? CopyrightHolder { get; init; }
    public string? DiagnosticCode { get; init; }
    public string? DiagnosticMessage { get; init; }
    public IReadOnlyList<string> AllDiagnostics { get; init; } = Array.Empty<string>();

    public static PilotLicenseCheckResult Allowed(string spdxId, string source = "", string? copyright = null) =>
        new()
        {
            IsAllowed = true,
            SpdxIdentifier = string.IsNullOrWhiteSpace(spdxId) ? "NOASSERTION" : spdxId,
            DetectedSource = source,
            CopyrightHolder = copyright
        };

    public static PilotLicenseCheckResult Rejected(string code, string message, string spdxId = "NOASSERTION", string source = "") =>
        new()
        {
            IsAllowed = false,
            SpdxIdentifier = spdxId,
            DetectedSource = source,
            DiagnosticCode = code,
            DiagnosticMessage = $"{code}: {message}",
            AllDiagnostics = new[] { $"{code}: {message}" }
        };
}

/// <summary>
/// Milestone 20.5: SPDX license detector, policy verifier, and third-party attribution generator.
/// Emits diagnostics <c>INGEST_LIC001</c> (copyleft), <c>INGEST_LIC002</c> (proprietary/non-permissive),
/// and <c>INGEST_LIC003</c> (missing/unidentified license whenviable-only policy is active).
/// </summary>
public static partial class PilotLicenseVerifier
{
    public static readonly IReadOnlySet<string> PermissiveSpdxIds = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
    {
        "MIT",
        "MIT-0",
        "Apache-2.0",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "ISC",
        "PSF-2.0",
        "Python-2.0",
        "Zlib",
        "0BSD",
        "CC0-1.0",
        "Unlicense",
        "BSL-1.0",
        "BlueOak-1.0.0"
    };

    private static readonly string[] CopyleftPrefixes =
    {
        "GPL", "LGPL", "AGPL", "SSPL", "EUPL", "MPL", "CDDL", "EPL", "CPAL", "OSL", "Cecill"
    };

    private static readonly string[] LicenseFileNames =
    {
        "LICENSE", "LICENSE.txt", "LICENSE.md", "LICENSE-MIT", "LICENSE-APACHE",
        "COPYING", "COPYING.txt", "LICENCE", "LICENCE.txt", "UNLICENSE"
    };

    [GeneratedRegex(@"SPDX-License-Identifier:\s*([A-Za-z0-9.\-+]+(?:\s+(?:OR|AND)\s+[A-Za-z0-9.\-+]+)*)", RegexOptions.IgnoreCase)]
    private static partial Regex SpdxHeaderRegex();

    [GeneratedRegex(@"Copyright\s+(?:\(c\)\s*)?(\d{4}(?:[-–]\d{4})?\s+[^\r\n*/]+)", RegexOptions.IgnoreCase)]
    private static partial Regex CopyrightRegex();

    [GeneratedRegex(@"""license""\s*:\s*""([^""]+)""", RegexOptions.IgnoreCase)]
    private static partial Regex PackageJsonLicenseRegex();

    [GeneratedRegex(@"^\s*license\s*=\s*(?:\{\s*text\s*=\s*)?""([^""]+)""", RegexOptions.IgnoreCase | RegexOptions.Multiline)]
    private static partial Regex TomlLicenseRegex();

    [GeneratedRegex(@"<PackageLicenseExpression>([^<]+)</PackageLicenseExpression>", RegexOptions.IgnoreCase)]
    private static partial Regex CsprojLicenseRegex();

    /// <summary>
    /// Verifies the license of a single source code string (and optional file path).
    /// </summary>
    public static PilotLicenseCheckResult VerifySource(
        string sourceCode,
        string? filePath,
        PilotLicensePolicy policy,
        ISet<string>? customAllowlist = null)
    {
        string spdx = DetectFromSourceHeader(sourceCode, out string? copyright);
        string detectedFrom = !string.IsNullOrEmpty(spdx) ? "SPDX-Header" : "";

        if (string.IsNullOrEmpty(spdx) && !string.IsNullOrEmpty(filePath))
        {
            string? dir = Path.GetDirectoryName(Path.GetFullPath(filePath));
            if (!string.IsNullOrEmpty(dir) && Directory.Exists(dir))
            {
                var dirDetection = DetectFromDirectory(dir);
                if (!string.IsNullOrEmpty(dirDetection.spdx))
                {
                    spdx = dirDetection.spdx;
                    detectedFrom = dirDetection.source;
                    copyright ??= dirDetection.copyright;
                }
            }
        }

        return EvaluatePolicy(spdx, detectedFrom, copyright, policy, customAllowlist);
    }

    /// <summary>
    /// Verifies the license of an entire foreign library directory.
    /// </summary>
    public static PilotLicenseCheckResult VerifyDirectory(
        string directoryPath,
        PilotLicensePolicy policy,
        ISet<string>? customAllowlist = null)
    {
        if (!Directory.Exists(directoryPath))
        {
            return EvaluatePolicy("", "", null, policy, customAllowlist);
        }

        var (spdx, source, copyright) = DetectFromDirectory(directoryPath);
        if (string.IsNullOrEmpty(spdx))
        {
            // Fallback: check top-level source files for SPDX-License-Identifier
            foreach (var file in Directory.EnumerateFiles(directoryPath, "*.*", SearchOption.TopDirectoryOnly).Take(20))
            {
                try
                {
                    string head = ReadFirstLines(file, 40);
                    string fileSpdx = DetectFromSourceHeader(head, out string? fileCopy);
                    if (!string.IsNullOrEmpty(fileSpdx))
                    {
                        spdx = fileSpdx;
                        source = Path.GetFileName(file);
                        copyright ??= fileCopy;
                        break;
                    }
                }
                catch
                {
                    // Ignore unreadable binary files
                }
            }
        }

        return EvaluatePolicy(spdx, source, copyright, policy, customAllowlist);
    }

    private static PilotLicenseCheckResult EvaluatePolicy(
        string spdx,
        string detectedFrom,
        string? copyright,
        PilotLicensePolicy policy,
        ISet<string>? customAllowlist)
    {
        if (policy == PilotLicensePolicy.Any)
        {
            return PilotLicenseCheckResult.Allowed(
                string.IsNullOrWhiteSpace(spdx) ? "NOASSERTION" : spdx,
                detectedFrom,
                copyright);
        }

        if (string.IsNullOrWhiteSpace(spdx) || string.Equals(spdx, "NOASSERTION", StringComparison.OrdinalIgnoreCase) || string.Equals(spdx, "UNKNOWN", StringComparison.OrdinalIgnoreCase))
        {
            return PilotLicenseCheckResult.Rejected(
                "INGEST_LIC003",
                "Missing or unidentified software license. When '--viable-license-only' or '--license-policy permissive' is enabled, source files or packages must declare a valid permissive SPDX license via LICENSE file, package manifest, or 'SPDX-License-Identifier' header.",
                "NOASSERTION",
                detectedFrom);
        }

        // Handle compound SPDX expressions such as "MIT OR Apache-2.0"
        var orParts = spdx.Split(new[] { " OR ", " or " }, StringSplitOptions.RemoveEmptyEntries)
            .Select(p => p.Trim('(', ')', ' '))
            .ToList();

        if (policy == PilotLicensePolicy.SpdxAllowlist)
        {
            Func<string, bool> isInAllowlist = p => customAllowlist != null ? customAllowlist.Contains(p) : PermissiveSpdxIds.Contains(p);
            if (orParts.Any(isInAllowlist))
            {
                string matched = orParts.First(isInAllowlist);
                return PilotLicenseCheckResult.Allowed(matched, detectedFrom, copyright);
            }

            if (IsCopyleftLicense(spdx))
            {
                return PilotLicenseCheckResult.Rejected(
                    "INGEST_LIC001",
                    $"Copyleft or reciprocal license '{spdx}' (from {detectedFrom}) is rejected by '--license-policy spdx-allowlist'.",
                    spdx,
                    detectedFrom);
            }

            return PilotLicenseCheckResult.Rejected(
                "INGEST_LIC002",
                $"License '{spdx}' (from {detectedFrom}) is not in the configured SPDX allowlist.",
                spdx,
                detectedFrom);
        }

        // PilotLicensePolicy.Permissive (--viable-license-only)
        // If dual-licensed (e.g., "MIT OR Apache-2.0" or "MIT OR GPL-2.0-only"), check if any OR branch is permissive
        if (orParts.Count > 1 && orParts.Any(p => IsPermissiveExpression(p)))
        {
            string chosen = orParts.First(p => IsPermissiveExpression(p));
            return PilotLicenseCheckResult.Allowed(chosen, detectedFrom, copyright);
        }

        // Handle AND expressions: all parts must be permissive
        var andParts = spdx.Split(new[] { " AND ", " and " }, StringSplitOptions.RemoveEmptyEntries)
            .Select(p => p.Trim('(', ')', ' '))
            .ToList();

        if (andParts.All(p => PermissiveSpdxIds.Contains(p)))
        {
            return PilotLicenseCheckResult.Allowed(spdx, detectedFrom, copyright);
        }

        if (IsCopyleftLicense(spdx))
        {
            return PilotLicenseCheckResult.Rejected(
                "INGEST_LIC001",
                $"Copyleft/viral license '{spdx}' detected ({detectedFrom}). Only permissive redistributable licenses ({string.Join(", ", PermissiveSpdxIds.Take(8))}) are permitted under '--viable-license-only'.",
                spdx,
                detectedFrom);
        }

        return PilotLicenseCheckResult.Rejected(
            "INGEST_LIC002",
            $"Proprietary or non-permissive license '{spdx}' detected ({detectedFrom}). Rejected under '--viable-license-only'.",
            spdx,
            detectedFrom);
    }

    private static bool IsPermissiveExpression(string expr)
    {
        var andParts = expr.Split(new[] { " AND ", " and " }, StringSplitOptions.RemoveEmptyEntries)
            .Select(p => p.Trim('(', ')', ' '))
            .ToList();
        return andParts.Count > 0 && andParts.All(p => PermissiveSpdxIds.Contains(p));
    }

    private static bool IsCopyleftLicense(string spdx)
    {
        foreach (var prefix in CopyleftPrefixes)
        {
            if (spdx.Contains(prefix, StringComparison.OrdinalIgnoreCase))
                return true;
        }
        return false;
    }

    public static string DetectFromSourceHeader(string sourceCode, out string? copyright)
    {
        copyright = null;
        if (string.IsNullOrWhiteSpace(sourceCode))
            return "";

        string headerRegion = sourceCode.Length > 4096 ? sourceCode[..4096] : sourceCode;

        var copyMatch = CopyrightRegex().Match(headerRegion);
        if (copyMatch.Success)
        {
            copyright = copyMatch.Groups[1].Value.Trim();
        }

        var spdxMatch = SpdxHeaderRegex().Match(headerRegion);
        if (spdxMatch.Success)
        {
            return spdxMatch.Groups[1].Value.Trim().TrimEnd('*', '/', ' ');
        }

        return ClassifyLicenseText(headerRegion);
    }

    private static (string spdx, string source, string? copyright) DetectFromDirectory(string dir)
    {
        string? currentDir = Path.GetFullPath(dir);
        for (int depth = 0; depth < 3 && !string.IsNullOrEmpty(currentDir) && Directory.Exists(currentDir); depth++)
        {
            // 1. Check LICENSE / COPYING files
            foreach (var licName in LicenseFileNames)
            {
                string candidate = Path.Combine(currentDir, licName);
                if (File.Exists(candidate))
                {
                    string text = File.ReadAllText(candidate);
                    string spdx = DetectFromSourceHeader(text, out string? copy);
                    if (string.IsNullOrEmpty(spdx))
                    {
                        spdx = ClassifyLicenseText(text);
                    }
                    if (!string.IsNullOrEmpty(spdx))
                    {
                        return (spdx, licName, copy);
                    }
                }
            }

            // 2. Check package.json
            string pkgJson = Path.Combine(currentDir, "package.json");
            if (File.Exists(pkgJson))
            {
                string content = File.ReadAllText(pkgJson);
                var m = PackageJsonLicenseRegex().Match(content);
                if (m.Success)
                {
                    return (m.Groups[1].Value.Trim(), "package.json", null);
                }
            }

            // 3. Check Cargo.toml / pyproject.toml
            foreach (var tomlName in new[] { "Cargo.toml", "pyproject.toml" })
            {
                string tomlPath = Path.Combine(currentDir, tomlName);
                if (File.Exists(tomlPath))
                {
                    string content = File.ReadAllText(tomlPath);
                    var m = TomlLicenseRegex().Match(content);
                    if (m.Success)
                    {
                        return (m.Groups[1].Value.Trim(), tomlName, null);
                    }
                }
            }

            // 4. Check *.csproj
            foreach (var csproj in Directory.EnumerateFiles(currentDir, "*.csproj", SearchOption.TopDirectoryOnly))
            {
                string content = File.ReadAllText(csproj);
                var m = CsprojLicenseRegex().Match(content);
                if (m.Success)
                {
                    return (m.Groups[1].Value.Trim(), Path.GetFileName(csproj), null);
                }
            }

            currentDir = Path.GetDirectoryName(currentDir);
        }

        return ("", "", null);
    }

    public static string ClassifyLicenseText(string text)
    {
        if (string.IsNullOrWhiteSpace(text))
            return "";

        if (text.Contains("GNU AFFERO GENERAL PUBLIC LICENSE", StringComparison.OrdinalIgnoreCase) ||
            text.Contains("AGPL-", StringComparison.OrdinalIgnoreCase))
            return "AGPL-3.0-only";

        if (text.Contains("GNU LESSER GENERAL PUBLIC LICENSE", StringComparison.OrdinalIgnoreCase) ||
            text.Contains("LGPL-", StringComparison.OrdinalIgnoreCase))
            return "LGPL-3.0-only";

        if (text.Contains("GNU GENERAL PUBLIC LICENSE", StringComparison.OrdinalIgnoreCase) ||
            text.Contains("either version 3 of the License", StringComparison.OrdinalIgnoreCase) ||
            text.Contains("GPL-3.0", StringComparison.OrdinalIgnoreCase) ||
            text.Contains("GPL-2.0", StringComparison.OrdinalIgnoreCase))
            return "GPL-3.0-only";

        if (text.Contains("Server Side Public License", StringComparison.OrdinalIgnoreCase) ||
            text.Contains("SSPL", StringComparison.Ordinal))
            return "SSPL-1.0";

        if (text.Contains("European Union Public Licence", StringComparison.OrdinalIgnoreCase) ||
            text.Contains("EUPL", StringComparison.Ordinal))
            return "EUPL-1.2";

        if (text.Contains("Mozilla Public License", StringComparison.OrdinalIgnoreCase))
            return "MPL-2.0";

        if (text.Contains("Proprietary", StringComparison.OrdinalIgnoreCase) ||
            text.Contains("All Rights Reserved", StringComparison.OrdinalIgnoreCase) &&
            !text.Contains("Permission is hereby granted", StringComparison.OrdinalIgnoreCase) &&
            !text.Contains("Redistribution and use in source and binary forms", StringComparison.OrdinalIgnoreCase))
            return "Proprietary";

        if (text.Contains("Permission is hereby granted, free of charge, to any person obtaining a copy", StringComparison.OrdinalIgnoreCase) ||
            text.Contains("MIT License", StringComparison.OrdinalIgnoreCase))
            return "MIT";

        if (text.Contains("Apache License", StringComparison.OrdinalIgnoreCase) &&
            text.Contains("Version 2.0", StringComparison.OrdinalIgnoreCase))
            return "Apache-2.0";

        if (text.Contains("Redistribution and use in source and binary forms", StringComparison.OrdinalIgnoreCase))
        {
            if (text.Contains("Neither the name of", StringComparison.OrdinalIgnoreCase) ||
                text.Contains("nor the names of its contributors", StringComparison.OrdinalIgnoreCase))
                return "BSD-3-Clause";
            return "BSD-2-Clause";
        }

        if (text.Contains("Permission to use, copy, modify, and/or distribute this software for any purpose", StringComparison.OrdinalIgnoreCase) ||
            text.Contains("ISC License", StringComparison.OrdinalIgnoreCase))
            return "ISC";

        if (text.Contains("PYTHON SOFTWARE FOUNDATION LICENSE", StringComparison.OrdinalIgnoreCase) ||
            text.Contains("PSF LICENSE", StringComparison.OrdinalIgnoreCase))
            return "PSF-2.0";

        if (text.Contains("This is free and unencumbered software released into the public domain", StringComparison.OrdinalIgnoreCase))
            return "Unlicense";

        if (text.Contains("Creative Commons Zero", StringComparison.OrdinalIgnoreCase) ||
            text.Contains("CC0 1.0 Universal", StringComparison.OrdinalIgnoreCase))
            return "CC0-1.0";

        if (text.Contains("Boost Software License", StringComparison.OrdinalIgnoreCase))
            return "BSL-1.0";

        return "";
    }

    public static PilotLicensePolicy ParsePolicy(string? policy) =>
        (policy ?? "").Trim().ToLowerInvariant() switch
        {
            "permissive" or "viable" or "viable-only" => PilotLicensePolicy.Permissive,
            "spdx-allowlist" or "allowlist" => PilotLicensePolicy.SpdxAllowlist,
            _ => PilotLicensePolicy.Any
        };

    public static string GenerateThirdPartyNotices(
        string packageName,
        string sourceDirectory,
        PilotLicenseCheckResult licenseResult) =>
        GenerateThirdPartyNotices(packageName, sourceDirectory, "foreign", licenseResult, Array.Empty<string>());

    /// <summary>
    /// Generates the contents of <c>THIRD_PARTY_NOTICES.md</c> for an ingested foreign package.
    /// </summary>
    public static string GenerateThirdPartyNotices(
        string packageName,
        string sourceDirectory,
        string sourceLanguage,
        PilotLicenseCheckResult licenseResult,
        IReadOnlyList<string> generatedFiles)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# Third-Party Software Notices & License Provenance");
        sb.AppendLine();
        sb.AppendLine($"This Pilot package (`{packageName}`) was generated via `pilot --from {sourceLanguage}` from upstream source.");
        sb.AppendLine();
        sb.AppendLine("## Upstream Provenance");
        sb.AppendLine($"- **Package Name**: `{packageName}`");
        sb.AppendLine($"- **Source Language**: `{sourceLanguage}`");
        sb.AppendLine($"- **SPDX License Identifier**: `{licenseResult.SpdxIdentifier}`");
        if (!string.IsNullOrEmpty(licenseResult.DetectedSource))
        {
            sb.AppendLine($"- **License Source**: `{licenseResult.DetectedSource}`");
        }
        if (!string.IsNullOrEmpty(licenseResult.CopyrightHolder))
        {
            sb.AppendLine($"- **Copyright**: {licenseResult.CopyrightHolder}");
        }
        sb.AppendLine($"- **Transpiled Files**: {generatedFiles.Count}");
        sb.AppendLine();
        sb.AppendLine("## Attribution Notice");
        sb.AppendLine("Original copyright and permission notices from the upstream project are preserved in accordance with its SPDX license terms.");
        return sb.ToString();
    }

    private static string ReadFirstLines(string filePath, int maxLines)
    {
        var sb = new StringBuilder();
        using var reader = new StreamReader(filePath);
        for (int i = 0; i < maxLines && !reader.EndOfStream; i++)
        {
            sb.AppendLine(reader.ReadLine());
        }
        return sb.ToString();
    }
}
