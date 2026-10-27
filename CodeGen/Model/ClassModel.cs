using System;
using System.Collections.Generic;
using System.Linq;

namespace Pilot.CodeGen.Model;

/// <summary>
/// Semantic model representing a Pilot class declaration.
/// Simple and Pythonic: inspect fields, properties, methods, decorators, and metadata.
/// Provides helpers to add methods, fields, properties, or partial class extensions.
/// </summary>
public sealed class ClassModel
{
    public string Name { get; set; } = string.Empty;
    public string? BaseClassName { get; set; }
    public bool IsPartial { get; set; }
    public List<PropertyModel> Fields { get; } = new();
    public List<PropertyModel> Properties { get; } = new();
    public List<MethodModel> Methods { get; } = new();
    public List<DecoratorModel> Decorators { get; } = new();
    public MetadataModel Meta { get; } = new();
    public int EndIndex { get; set; } = -1;

    /// <summary>
    /// Holds generated code blocks for this class (appended via partial class).
    /// </summary>
    public List<string> GeneratedPartials { get; } = new();

    public bool HasDecorator(string name) =>
        Decorators.Any(d => string.Equals(d.Name, name, StringComparison.OrdinalIgnoreCase));

    public DecoratorModel? GetDecorator(string name) =>
        Decorators.FirstOrDefault(d => string.Equals(d.Name, name, StringComparison.OrdinalIgnoreCase));

    public PropertyModel? FindField(string name) =>
        Fields.FirstOrDefault(f => string.Equals(f.Name, name, StringComparison.Ordinal));

    public MethodModel? FindMethod(string name) =>
        Methods.FirstOrDefault(m => string.Equals(m.Name, name, StringComparison.Ordinal));

    /// <summary>
    /// Adds a method to this class via an auto-generated partial class extension.
    /// </summary>
    public void AddMethod(string code)
    {
        if (string.IsNullOrWhiteSpace(code)) return;
        GeneratedPartials.Add(code.Trim());
    }

    /// <summary>
    /// Adds a field to this class via an auto-generated partial class extension.
    /// </summary>
    public void AddField(string code)
    {
        if (string.IsNullOrWhiteSpace(code)) return;
        GeneratedPartials.Add(code.Trim());
    }

    /// <summary>
    /// Adds a property or custom code block directly into the class companion.
    /// </summary>
    public void AddPartial(string code)
    {
        if (string.IsNullOrWhiteSpace(code)) return;
        GeneratedPartials.Add(code.Trim());
    }

    public override string ToString()
    {
        string pfx = IsPartial ? "partial " : "";
        string inherit = !string.IsNullOrEmpty(BaseClassName) ? $"({BaseClassName})" : "";
        return $"{pfx}class {Name}{inherit}";
    }
}
