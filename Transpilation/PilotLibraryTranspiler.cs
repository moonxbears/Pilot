using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;

namespace Pilot.Transpilation;

/// <summary>
/// Result of transpiling or ingesting an entire multi-file library directory.
/// </summary>
public sealed class LibraryTranspilationSummary
{
    public bool Success { get; init; }
    public string SourceDirectory { get; init; } = "";
    public string OutputDirectory { get; init; } = "";
    public string Language { get; init; } = "";
    public List<string> GeneratedFiles { get; } = new();
    public List<string> Diagnostics { get; } = new();
}

/// <summary>
/// Full multi-file library and package directory transpiler.
/// Converts entire foreign libraries (e.g. a Python/NumPy package, Rust crate, C/C++ library, JS/Lua module tree)
/// into a native Pilot package (with pilot.json) and vice versa.
/// </summary>
public static class PilotLibraryTranspiler
{
    private static readonly string[] ForeignExtensions =
    {
        ".py", ".lua", ".js", ".mjs", ".ts", ".c", ".h", ".cpp", ".hpp", ".cc", ".rs", ".cs"
    };

    /// <summary>
    /// Ingests an entire foreign library directory (or single file) into an equivalent Pilot library directory.
    /// </summary>
    public static LibraryTranspilationSummary IngestLibraryToPilot(
        string sourcePath,
        string outputDir,
        string sourceLang = "auto",
        TranspilationOptions? options = null)
    {
        var summary = new LibraryTranspilationSummary
        {
            SourceDirectory = Path.GetFullPath(sourcePath),
            OutputDirectory = Path.GetFullPath(outputDir),
            Language = "pilot"
        };

        if (File.Exists(sourcePath))
        {
            Directory.CreateDirectory(outputDir);
            string lang = string.Equals(sourceLang, "auto", StringComparison.OrdinalIgnoreCase)
                ? TranspilationRegistry.InferLanguageFromPath(sourcePath)
                : sourceLang;
            string code = File.ReadAllText(sourcePath);
            var res = TranspilationRegistry.Default.Ingest(code, lang, options);
            if (!res.Success)
            {
                summary.Diagnostics.AddRange(res.Diagnostics);
                return new LibraryTranspilationSummary { Success = false, SourceDirectory = summary.SourceDirectory, OutputDirectory = summary.OutputDirectory };
            }

            string outFile = Path.Combine(outputDir, Path.ChangeExtension(Path.GetFileName(sourcePath), ".pilot"));
            File.WriteAllText(outFile, res.Code);
            summary.GeneratedFiles.Add(outFile);
            return new LibraryTranspilationSummary
            {
                Success = true,
                SourceDirectory = summary.SourceDirectory,
                OutputDirectory = summary.OutputDirectory,
                Language = "pilot"
            };
        }

        if (!Directory.Exists(sourcePath))
        {
            summary.Diagnostics.Add($"Source directory not found: '{sourcePath}'");
            return new LibraryTranspilationSummary { Success = false, SourceDirectory = summary.SourceDirectory, OutputDirectory = summary.OutputDirectory };
        }

        Directory.CreateDirectory(outputDir);
        string fullSourceRoot = Path.GetFullPath(sourcePath);

        var sourceFiles = Directory.GetFiles(fullSourceRoot, "*.*", SearchOption.AllDirectories)
            .Where(f => ForeignExtensions.Contains(Path.GetExtension(f).ToLowerInvariant()))
            .Where(f => !f.Contains(Path.DirectorySeparatorChar + "node_modules" + Path.DirectorySeparatorChar) &&
                        !f.Contains(Path.DirectorySeparatorChar + ".git" + Path.DirectorySeparatorChar) &&
                        !f.Contains(Path.DirectorySeparatorChar + "__pycache__" + Path.DirectorySeparatorChar) &&
                        !f.Contains(Path.DirectorySeparatorChar + "target" + Path.DirectorySeparatorChar))
            .ToList();

        foreach (var file in sourceFiles)
        {
            string fileLang = string.Equals(sourceLang, "auto", StringComparison.OrdinalIgnoreCase)
                ? TranspilationRegistry.InferLanguageFromPath(file)
                : sourceLang;

            if (!TranspilationRegistry.Default.TryGet(fileLang, out _))
                continue;

            string relPath = Path.GetRelativePath(fullSourceRoot, file);
            string baseName = Path.GetFileNameWithoutExtension(file);
            if (baseName == "__init__")
                baseName = "mod";

            string relDir = Path.GetDirectoryName(relPath) ?? "";
            string targetSubDir = Path.Combine(outputDir, relDir);
            Directory.CreateDirectory(targetSubDir);

            string outPilotFile = Path.Combine(targetSubDir, baseName + ".pilot");
            string foreignCode = File.ReadAllText(file);
            var res = TranspilationRegistry.Default.Ingest(foreignCode, fileLang, options);

            if (res.Success)
            {
                File.WriteAllText(outPilotFile, res.Code);
                summary.GeneratedFiles.Add(outPilotFile);
            }
            else
            {
                summary.Diagnostics.AddRange(res.Diagnostics);
            }
        }

        // Generate a pilot.json manifest for the newly ingested Pilot library
        string pkgName = new DirectoryInfo(fullSourceRoot).Name.ToLowerInvariant();
        string manifestPath = Path.Combine(outputDir, "pilot.json");
        if (!File.Exists(manifestPath))
        {
            string entryPoint = summary.GeneratedFiles.Count > 0
                ? Path.GetRelativePath(outputDir, summary.GeneratedFiles[0]).Replace('\\', '/')
                : "main.pilot";
            string manifestJson = $$"""
{
  "name": "{{pkgName}}",
  "version": "1.0.0",
  "description": "Library transpiled & optimized into native Pilot from '{{pkgName}}'",
  "entryPoint": "{{entryPoint}}",
  "dependencies": {}
}
""";
            File.WriteAllText(manifestPath, manifestJson);
            summary.GeneratedFiles.Add(manifestPath);
        }

        return new LibraryTranspilationSummary
        {
            Success = summary.Diagnostics.Count == 0,
            SourceDirectory = summary.SourceDirectory,
            OutputDirectory = summary.OutputDirectory,
            Language = "pilot"
        };
    }

    /// <summary>
    /// Transpiles an entire Pilot library directory into a target language package directory
    /// (Python package, Rust crate, C/C++ project, JS ESM package, or Lua module tree).
    /// </summary>
    public static LibraryTranspilationSummary TranspilePilotLibraryToTarget(
        string pilotDir,
        string outputDir,
        string targetLang,
        TranspilationOptions? options = null)
    {
        string fullPilotRoot = Path.GetFullPath(pilotDir);
        string fullOutRoot = Path.GetFullPath(outputDir);
        Directory.CreateDirectory(fullOutRoot);

        var generated = new List<string>();
        var diagnostics = new List<string>();
        string targetExt = TranspilationRegistry.GetDefaultExtension(targetLang);

        var pilotFiles = Directory.GetFiles(fullPilotRoot, "*.pilot", SearchOption.AllDirectories)
            .Where(f => !f.Contains(Path.DirectorySeparatorChar + ".pilot_modules" + Path.DirectorySeparatorChar) &&
                        !f.Contains(Path.DirectorySeparatorChar + ".pilot_venv" + Path.DirectorySeparatorChar))
            .ToList();

        foreach (var file in pilotFiles)
        {
            string relPath = Path.GetRelativePath(fullPilotRoot, file);
            string outFilePath = Path.Combine(fullOutRoot, Path.ChangeExtension(relPath, targetExt));
            string? outSubDir = Path.GetDirectoryName(outFilePath);
            if (!string.IsNullOrEmpty(outSubDir))
                Directory.CreateDirectory(outSubDir);

            string pilotCode = File.ReadAllText(file);
            var res = TranspilationRegistry.Default.Transpile(pilotCode, targetLang, options);
            if (res.Success)
            {
                File.WriteAllText(outFilePath, res.Code);
                generated.Add(outFilePath);
            }
            else
            {
                diagnostics.AddRange(res.Diagnostics);
            }
        }

        // Emit language-specific library scaffolding
        string libName = new DirectoryInfo(fullPilotRoot).Name.ToLowerInvariant().Replace('-', '_');
        EmitTargetLibraryScaffolding(fullOutRoot, libName, targetLang, generated);

        var result = new LibraryTranspilationSummary
        {
            Success = diagnostics.Count == 0,
            SourceDirectory = fullPilotRoot,
            OutputDirectory = fullOutRoot,
            Language = targetLang
        };
        result.GeneratedFiles.AddRange(generated);
        result.Diagnostics.AddRange(diagnostics);
        return result;
    }

    private static void EmitTargetLibraryScaffolding(string outDir, string libName, string targetLang, List<string> generated)
    {
        switch (targetLang.ToLowerInvariant())
        {
            case "python" or "py":
                string initPy = Path.Combine(outDir, "__init__.py");
                if (!File.Exists(initPy))
                {
                    var exports = generated
                        .Where(f => f.EndsWith(".py") && Path.GetDirectoryName(f) == outDir)
                        .Select(f => $"from .{Path.GetFileNameWithoutExtension(f)} import *");
                    File.WriteAllText(initPy, $"# Python package transpiled from Pilot library '{libName}'\n" + string.Join("\n", exports) + "\n");
                    generated.Add(initPy);
                }
                break;

            case "rust" or "rs":
                string cargoToml = Path.Combine(outDir, "Cargo.toml");
                if (!File.Exists(cargoToml))
                {
                    File.WriteAllText(cargoToml, $"""
[package]
name = "{libName}"
version = "0.1.0"
edition = "2024"
""");
                    generated.Add(cargoToml);
                }
                break;

            case "js" or "javascript":
                string pkgJson = Path.Combine(outDir, "package.json");
                if (!File.Exists(pkgJson))
                {
                    File.WriteAllText(pkgJson, $$"""
{
  "name": "{{libName}}",
  "version": "1.0.0",
  "type": "module"
}
""");
                    generated.Add(pkgJson);
                }
                break;
        }
    }
}
