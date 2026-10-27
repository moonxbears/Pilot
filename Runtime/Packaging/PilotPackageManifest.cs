using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Pilot.Runtime.Packaging;

/// <summary>
/// Manifest definition for a Pilot package (pilot.json).
/// Encapsulates native Pilot dependencies and adopted interop package dependencies
/// across ecosystems: NuGet, npm, pip, cargo, maven, and luarocks.
/// </summary>
public sealed class PilotPackageManifest
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "my-pilot-package";

    [JsonPropertyName("version")]
    public string Version { get; set; } = "1.0.0";

    [JsonPropertyName("description")]
    public string Description { get; set; } = "Pilot package";

    [JsonPropertyName("main")]
    public string Main { get; set; } = "main.pilot";

    [JsonPropertyName("author")]
    public string Author { get; set; } = string.Empty;

    [JsonPropertyName("license")]
    public string License { get; set; } = "MIT";

    [JsonPropertyName("dependencies")]
    public Dictionary<string, string> Dependencies { get; set; } = new(StringComparer.OrdinalIgnoreCase);

    [JsonPropertyName("interopDependencies")]
    public Dictionary<string, Dictionary<string, string>> InteropDependencies { get; set; } = new(StringComparer.OrdinalIgnoreCase);

    public PilotPackageManifest()
    {
        EnsureInteropSections();
    }

    public void EnsureInteropSections()
    {
        string[] ecosystems = { "nuget", "npm", "pip", "cargo", "maven", "luarocks" };
        foreach (var eco in ecosystems)
        {
            if (!InteropDependencies.ContainsKey(eco))
            {
                InteropDependencies[eco] = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            }
        }
    }

    public static PilotPackageManifest CreateDefault(string name)
    {
        return new PilotPackageManifest
        {
            Name = name,
            Version = "0.1.0",
            Description = $"Package {name} created with Pilot package manager.",
            Main = "main.pilot"
        };
    }

    public static PilotPackageManifest Load(string manifestPath)
    {
        if (!File.Exists(manifestPath))
        {
            throw new FileNotFoundException($"Package manifest not found: '{manifestPath}'", manifestPath);
        }

        string json = File.ReadAllText(manifestPath);
        var options = new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true,
            ReadCommentHandling = JsonCommentHandling.Skip,
            AllowTrailingCommas = true
        };

        var manifest = JsonSerializer.Deserialize<PilotPackageManifest>(json, options)
                       ?? throw new InvalidOperationException($"Failed to deserialize package manifest at '{manifestPath}'");

        manifest.EnsureInteropSections();
        return manifest;
    }

    public void Save(string manifestPath)
    {
        EnsureInteropSections();
        var options = new JsonSerializerOptions
        {
            WriteIndented = true
        };
        string json = JsonSerializer.Serialize(this, options);
        File.WriteAllText(manifestPath, json);
    }
}
