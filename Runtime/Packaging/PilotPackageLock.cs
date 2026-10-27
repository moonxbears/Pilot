using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Pilot.Runtime.Packaging;

/// <summary>
/// Resolved lockfile (pilot.lock.json) for deterministic, reproducible package builds.
/// </summary>
public sealed class PilotPackageLock
{
    [JsonPropertyName("lockVersion")]
    public int LockVersion { get; set; } = 1;

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("version")]
    public string Version { get; set; } = string.Empty;

    [JsonPropertyName("packages")]
    public Dictionary<string, LockedPackageEntry> Packages { get; set; } = new(StringComparer.OrdinalIgnoreCase);

    public static PilotPackageLock Load(string lockPath)
    {
        if (!File.Exists(lockPath)) return new PilotPackageLock();

        try
        {
            string json = File.ReadAllText(lockPath);
            return JsonSerializer.Deserialize<PilotPackageLock>(json) ?? new PilotPackageLock();
        }
        catch
        {
            return new PilotPackageLock();
        }
    }

    public void Save(string lockPath)
    {
        var options = new JsonSerializerOptions { WriteIndented = true };
        string json = JsonSerializer.Serialize(this, options);
        File.WriteAllText(lockPath, json);
    }
}

public sealed class LockedPackageEntry
{
    [JsonPropertyName("version")]
    public string Version { get; set; } = string.Empty;

    [JsonPropertyName("ecosystem")]
    public string Ecosystem { get; set; } = "pilot"; // pilot, nuget, npm, pip, cargo, maven, luarocks

    [JsonPropertyName("resolved")]
    public string Resolved { get; set; } = string.Empty;

    [JsonPropertyName("integrity")]
    public string Integrity { get; set; } = string.Empty;

    [JsonPropertyName("installedPath")]
    public string InstalledPath { get; set; } = string.Empty;
}
