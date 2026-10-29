using System;
using System.Collections.Generic;
using Pilot.Transpilation.Licensing;

namespace Pilot.Transpilation;

/// <summary>
/// Configuration options for compile-time code generation, formatting, and foreign ingestion.
/// </summary>
public sealed class TranspilationOptions
{
    public string IndentString { get; set; } = "    ";
    public string? TargetNamespaceOrModule { get; set; }
    public bool EmitHeaderComments { get; set; } = true;
    public bool GenerateTypeAnnotations { get; set; } = true;
    public bool Optimize { get; set; } = false;
    public bool StrictPilot { get; set; } = false;
    public bool FormatOutput { get; set; } = true;
    public bool PreferColonSuites { get; set; } = false;
    public string? SourceFilePath { get; set; }
    public PilotLicensePolicy LicensePolicy { get; set; } = PilotLicensePolicy.Any;
    public HashSet<string> CustomSpdxAllowlist { get; } = new(StringComparer.OrdinalIgnoreCase);
    public Dictionary<string, string> TypeMappings { get; } = new(StringComparer.OrdinalIgnoreCase);

    public TranspilationOptions()
    {
    }

    public TranspilationOptions WithIndent(string indent)
    {
        IndentString = indent ?? throw new ArgumentNullException(nameof(indent));
        return this;
    }

    public TranspilationOptions WithNamespace(string ns)
    {
        TargetNamespaceOrModule = ns;
        return this;
    }

    public TranspilationOptions WithHeaderComments(bool emit = true)
    {
        EmitHeaderComments = emit;
        return this;
    }

    public TranspilationOptions WithTypeMapping(string pilotType, string targetType)
    {
        TypeMappings[pilotType] = targetType;
        return this;
    }

    public TranspilationOptions WithOptimization(bool optimize = true)
    {
        Optimize = optimize;
        return this;
    }

    public TranspilationOptions WithStrictPilot(bool strict = true)
    {
        StrictPilot = strict;
        return this;
    }

    public TranspilationOptions WithLicensePolicy(PilotLicensePolicy policy, IEnumerable<string>? customAllowlist = null)
    {
        LicensePolicy = policy;
        if (customAllowlist != null)
        {
            foreach (var id in customAllowlist)
                CustomSpdxAllowlist.Add(id);
        }
        return this;
    }

    public TranspilationOptions WithViableLicenseOnly(bool viableOnly = true)
    {
        LicensePolicy = viableOnly ? PilotLicensePolicy.Permissive : PilotLicensePolicy.Any;
        return this;
    }

    public TranspilationOptions WithColonSuites(bool colonSuites = true)
    {
        PreferColonSuites = colonSuites;
        return this;
    }

    public TranspilationOptions WithSourceFilePath(string? filePath)
    {
        SourceFilePath = filePath;
        return this;
    }
}
