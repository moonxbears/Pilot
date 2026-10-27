using System;
using System.Collections.Generic;
using System.Linq;

namespace Pilot.CodeGen.Model;

/// <summary>
/// Semantic model representing a method or top-level function.
/// </summary>
public sealed class MethodModel
{
    public string Name { get; set; } = string.Empty;
    public List<ParameterModel> Parameters { get; } = new();
    public string? ReturnType { get; set; }
    public bool IsAsync { get; set; }
    public bool IsStatic { get; set; }
    public List<DecoratorModel> Decorators { get; } = new();
    public MetadataModel Meta { get; } = new();
    public string? BodyText { get; set; }

    public bool HasDecorator(string name) =>
        Decorators.Any(d => string.Equals(d.Name, name, StringComparison.OrdinalIgnoreCase));

    public DecoratorModel? GetDecorator(string name) =>
        Decorators.FirstOrDefault(d => string.Equals(d.Name, name, StringComparison.OrdinalIgnoreCase));

    public override string ToString()
    {
        string asyncKw = IsAsync ? "async " : "";
        string ret = !string.IsNullOrEmpty(ReturnType) ? $" -> {ReturnType}" : "";
        return $"{asyncKw}def {Name}({string.Join(", ", Parameters)}){ret}";
    }
}
