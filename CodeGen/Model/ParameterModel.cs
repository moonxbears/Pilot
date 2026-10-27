using System;

namespace Pilot.CodeGen.Model;

/// <summary>
/// Semantic model representing a method or function parameter.
/// </summary>
public sealed class ParameterModel
{
    public string Name { get; set; } = string.Empty;
    public string? TypeName { get; set; }
    public string? DefaultValue { get; set; }
    public bool IsParams { get; set; }
    public bool IsKwargs { get; set; }
    public MetadataModel Meta { get; } = new();

    public ParameterModel() { }

    public ParameterModel(string name, string? typeName = null, string? defaultValue = null)
    {
        Name = name;
        TypeName = typeName;
        DefaultValue = defaultValue;
    }

    public override string ToString()
    {
        string decl = Name;
        if (!string.IsNullOrEmpty(TypeName)) decl += $": {TypeName}";
        if (!string.IsNullOrEmpty(DefaultValue)) decl += $" = {DefaultValue}";
        return decl;
    }
}
