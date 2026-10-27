using System;
using System.Collections.Generic;
using System.Linq;

namespace Pilot.CodeGen.Model;

/// <summary>
/// Semantic model representing a complete Pilot script compilation unit.
/// Clean, Pythonic, and straightforward.
/// Provides capabilities to:
/// - Inspect classes, top-level functions, variables, metadata
/// - Self-modify the script (replace_code, add_code)
/// - Add new companion scripts to the compilation (add_script)
/// </summary>
public sealed class ScriptModel
{
    public string Name { get; set; } = string.Empty;
    public string Source { get; set; } = string.Empty;
    public List<ClassModel> Classes { get; } = new();
    public List<MethodModel> Functions { get; } = new();
    public List<PropertyModel> Variables { get; } = new();
    public MetadataModel Meta { get; } = new();

    /// <summary>
    /// Additional code blocks appended to this script.
    /// </summary>
    public List<string> AddedCode { get; } = new();

    /// <summary>
    /// Companion scripts created by generators (e.g. "generated_api.pilot").
    /// </summary>
    public Dictionary<string, string> AddedScripts { get; } = new(StringComparer.OrdinalIgnoreCase);

    /// <summary>
    /// Pattern replacements performed on the script source code.
    /// </summary>
    public List<(string Target, string Replacement)> Replacements { get; } = new();

    public ClassModel? FindClass(string name) =>
        Classes.FirstOrDefault(c => string.Equals(c.Name, name, StringComparison.Ordinal));

    public MethodModel? FindFunction(string name) =>
        Functions.FirstOrDefault(f => string.Equals(f.Name, name, StringComparison.Ordinal));

    /// <summary>
    /// Appends source code to this script.
    /// </summary>
    public void AddCode(string code)
    {
        if (string.IsNullOrWhiteSpace(code)) return;
        AddedCode.Add(code.Trim());
    }

    /// <summary>
    /// Adds a new virtual script to the compilation unit.
    /// </summary>
    public void AddScript(string scriptName, string code)
    {
        if (string.IsNullOrWhiteSpace(scriptName)) return;
        AddedScripts[scriptName] = code ?? string.Empty;
    }

    /// <summary>
    /// Replaces occurrences of target with replacement in the script source.
    /// </summary>
    public void ReplaceCode(string target, string replacement)
    {
        if (!string.IsNullOrEmpty(target))
        {
            Replacements.Add((target, replacement));
        }
    }
}
