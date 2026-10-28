using System;

namespace Pilot.Diagnostics;

/// <summary>
/// Centralized commit-bound version authority for the Pilot compiler, runtime, CLI, virtual environments, and LSP server.
/// Every new Git commit automatically advances <see cref="CommitCount"/>, <see cref="SemVer"/>, <see cref="ShortCommitSha"/>,
/// and <see cref="FullVersion"/> at build time so no two commits ever share the same version identifier.
/// </summary>
public static class PilotVersion
{
    /// <summary>Major.Minor base version series (e.g., "1.1").</summary>
    public static string BaseVersion => PilotGitBuildStamp.BaseVersion;

    /// <summary>Monotonically increasing Git commit count on the current branch (e.g., 45).</summary>
    public static int CommitCount => PilotGitBuildStamp.CommitCount;

    /// <summary>Commit-bound semantic version string: Major.Minor.CommitCount (e.g., "1.1.45").</summary>
    public static string SemVer => PilotGitBuildStamp.SemVer;

    /// <summary>7-character short Git commit hash (e.g., "b905d08").</summary>
    public static string ShortCommitSha => PilotGitBuildStamp.ShortCommitSha;

    /// <summary>Full 40-character Git commit SHA-1 hash.</summary>
    public static string FullCommitSha => PilotGitBuildStamp.FullCommitSha;

    /// <summary>Git branch name recorded at build time.</summary>
    public static string Branch => PilotGitBuildStamp.Branch;

    /// <summary>ISO-8601 timestamp of the HEAD commit recorded at build time.</summary>
    public static string CommitDate => PilotGitBuildStamp.CommitDate;

    /// <summary>Full version string combining SemVer and commit SHA: "1.1.45+b905d08".</summary>
    public static string FullVersion => PilotGitBuildStamp.FullVersion;

    /// <summary>Human-readable one-line banner suitable for CLI headers and logs.</summary>
    public static string DisplayBanner =>
        $"Pilot v{FullVersion} (commit #{CommitCount}: {ShortCommitSha} on {Branch}, {CommitDate})";

    /// <summary>
    /// Checks whether <paramref name="otherVersion"/> matches the currently running Pilot commit version.
    /// Accepts either full informational version ("1.1.45+b905d08") or exact SemVer ("1.1.45") if no "+sha" suffix is present.
    /// </summary>
    public static bool IsExactMatch(string? otherVersion)
    {
        if (string.IsNullOrWhiteSpace(otherVersion))
            return false;

        string trimmed = otherVersion.Trim();
        if (trimmed.StartsWith('v') || trimmed.StartsWith('V'))
            trimmed = trimmed[1..];

        if (string.Equals(trimmed, FullVersion, StringComparison.OrdinalIgnoreCase))
            return true;

        // If the compared version does not include "+sha", require exact SemVer match (which includes CommitCount)
        if (!trimmed.Contains('+') && string.Equals(trimmed, SemVer, StringComparison.OrdinalIgnoreCase))
            return true;

        return false;
    }

    /// <summary>
    /// Verifies compatibility between an artifact/environment version and the running Pilot runtime,
    /// returning false and a human-readable diagnostic message when a commit/version mismatch is detected.
    /// </summary>
    public static bool CheckCompatibility(string? artifactVersion, string artifactKind, out string? mismatchMessage)
    {
        if (string.IsNullOrWhiteSpace(artifactVersion))
        {
            mismatchMessage = null;
            return true;
        }

        if (IsExactMatch(artifactVersion))
        {
            mismatchMessage = null;
            return true;
        }

        mismatchMessage =
            $"Version mismatch detected for {artifactKind}: created with Pilot '{artifactVersion}', " +
            $"but currently running Pilot '{FullVersion}' (commit #{CommitCount}, {ShortCommitSha}).";
        return false;
    }
}
