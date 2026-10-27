using System;
using System.Collections.Generic;

namespace Pilot.Transpilation;

/// <summary>
/// Configuration options for compile-time code generation and conversion.
/// </summary>
public sealed class TranspilationOptions
{
    public string IndentString { get; set; } = "    ";
    public string? TargetNamespaceOrModule { get; set; }
    public bool EmitHeaderComments { get; set; } = true;
    public bool GenerateTypeAnnotations { get; set; } = true;
    public bool Optimize { get; set; } = false;
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
}
