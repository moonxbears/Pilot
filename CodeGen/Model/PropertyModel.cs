using System;
using System.Collections.Generic;
using System.Linq;

namespace Pilot.CodeGen.Model;

/// <summary>
/// Semantic model representing a class field or property.
/// Simple access: name, type, default value, mutability, decorators, and metadata.
/// </summary>
public sealed class PropertyModel
{
    public string Name { get; set; } = string.Empty;
    public string? TypeName { get; set; }
    public string? DefaultValue { get; set; }
    public bool IsMutable { get; set; } = true;
    public bool IsStatic { get; set; }
    public List<DecoratorModel> Decorators { get; } = new();
    public MetadataModel Meta { get; } = new();

    public bool HasDecorator(string name) =>
        Decorators.Any(d => string.Equals(d.Name, name, StringComparison.OrdinalIgnoreCase));

    public DecoratorModel? GetDecorator(string name) =>
        Decorators.FirstOrDefault(d => string.Equals(d.Name, name, StringComparison.OrdinalIgnoreCase));

    public override string ToString()
    {
        string kw = IsMutable ? "var" : "let";
        string decl = $"{kw} {Name}";
        if (!string.IsNullOrEmpty(TypeName)) decl += $": {TypeName}";
        if (!string.IsNullOrEmpty(DefaultValue)) decl += $" = {DefaultValue}";
        return decl;
    }
}
