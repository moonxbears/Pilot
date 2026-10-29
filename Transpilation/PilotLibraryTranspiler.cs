using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using Pilot.Transpilation.Licensing;

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
    public PilotLicenseCheckResult? LicenseInfo { get; init; }
    public List<string> GeneratedFiles { get; } = new();
    public List<string> Diagnostics { get; } = new();
}

/// <summary>
/// Full multi-file library and package directory transpiler.
/// Converts entire foreign libraries (e.g. a Python/NumPy package, Rust crate, C/C++ library, JS/Lua module tree, C# project)
/// into a native Pilot package (with <c>pilot.json</c> and <c>THIRD_PARTY_NOTICES.md</c>) and vice versa.
/// </summary>
public static class PilotLibraryTranspiler
{
    private static readonly string[] ForeignExtensions =
    {
        ".py", ".lua", ".js", ".mjs", ".ts", ".c", ".h", ".cpp", ".hpp", ".cc", ".rs", ".cs", ".java", ".rb"
    };

    /// <summary>
    /// Ingests an entire foreign library directory (or single file) into an equivalent Pilot library directory.
    /// Enforces <see cref="TranspilationOptions.LicensePolicy"/> (<c>--viable-license-only</c>) when configured.
    /// </summary>
    public static LibraryTranspilationSummary IngestLibraryToPilot(
        string sourcePath,
        string outputDir,
        string sourceLang = "auto",
        TranspilationOptions? options = null)
    {
        options ??= new TranspilationOptions();
        string fullSourcePath = Path.GetFullPath(sourcePath);
        string fullOutDir = Path.GetFullPath(outputDir);

        if (File.Exists(sourcePath))
        {
            string code = File.ReadAllText(sourcePath);
            var fileOpts = new TranspilationOptions
            {
                IndentString = options.IndentString,
                EmitHeaderComments = options.EmitHeaderComments,
                GenerateTypeAnnotations = options.GenerateTypeAnnotations,
                TargetNamespaceOrModule = options.TargetNamespaceOrModule,
                Optimize = options.Optimize,
                StrictPilot = options.StrictPilot,
                LicensePolicy = options.LicensePolicy,
                PreferColonSuites = options.PreferColonSuites,
                FormatOutput = options.FormatOutput,
                SourceFilePath = fullSourcePath
            };
            foreach (var spdx in options.CustomSpdxAllowlist)
                fileOpts.CustomSpdxAllowlist.Add(spdx);

            string lang = string.Equals(sourceLang, "auto", StringComparison.OrdinalIgnoreCase)
                ? TranspilationRegistry.InferLanguageFromPath(sourcePath)
                : sourceLang;

            var res = TranspilationRegistry.Default.Ingest(code, lang, fileOpts);
            if (!res.Success)
            {
                var failSummary = new LibraryTranspilationSummary
                {
                    Success = false,
                    SourceDirectory = fullSourcePath,
                    OutputDirectory = fullOutDir,
                    Language = "pilot",
                    LicenseInfo = res.LicenseInfo
                };
                failSummary.Diagnostics.AddRange(res.Diagnostics);
                return failSummary;
            }

            Directory.CreateDirectory(outputDir);
            string outFile = Path.Combine(outputDir, Path.ChangeExtension(Path.GetFileName(sourcePath), ".pilot"));
            File.WriteAllText(outFile, res.Code);

            var singleSummary = new LibraryTranspilationSummary
            {
                Success = true,
                SourceDirectory = fullSourcePath,
                OutputDirectory = fullOutDir,
                Language = "pilot",
                LicenseInfo = res.LicenseInfo
            };
            singleSummary.GeneratedFiles.Add(outFile);
            return singleSummary;
        }

        if (!Directory.Exists(sourcePath))
        {
            var missing = new LibraryTranspilationSummary
            {
                Success = false,
                SourceDirectory = fullSourcePath,
                OutputDirectory = fullOutDir,
                Language = "pilot"
            };
            missing.Diagnostics.Add($"Source directory not found: '{sourcePath}'");
            return missing;
        }

        // Verify directory license policy before ingesting files
        var dirLicense = PilotLicenseVerifier.VerifyDirectory(
            fullSourcePath,
            options.LicensePolicy,
            options.CustomSpdxAllowlist);
        if (!dirLicense.IsAllowed)
        {
            var rejected = new LibraryTranspilationSummary
            {
                Success = false,
                SourceDirectory = fullSourcePath,
                OutputDirectory = fullOutDir,
                Language = "pilot",
                LicenseInfo = dirLicense
            };
            if (!string.IsNullOrEmpty(dirLicense.DiagnosticMessage))
                rejected.Diagnostics.Add(dirLicense.DiagnosticMessage);
            return rejected;
        }

        Directory.CreateDirectory(outputDir);
        var generatedFiles = new List<string>();
        var diagnostics = new List<string>();

        var sourceFiles = Directory.GetFiles(fullSourcePath, "*.*", SearchOption.AllDirectories)
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

            string relPath = Path.GetRelativePath(fullSourcePath, file);
            string baseName = Path.GetFileNameWithoutExtension(file);
            if (baseName == "__init__")
                baseName = "mod";

            string relDir = Path.GetDirectoryName(relPath) ?? "";
            string targetSubDir = Path.Combine(outputDir, relDir);
            Directory.CreateDirectory(targetSubDir);

            string outPilotFile = Path.Combine(targetSubDir, baseName + ".pilot");
            string foreignCode = File.ReadAllText(file);

            // File-level ingestion uses LicensePolicy.Any if directory license was already verified,
            // unless the file itself has a conflicting SPDX header.
            var fileOpts = new TranspilationOptions
            {
                IndentString = options.IndentString,
                EmitHeaderComments = options.EmitHeaderComments,
                GenerateTypeAnnotations = options.GenerateTypeAnnotations,
                TargetNamespaceOrModule = options.TargetNamespaceOrModule,
                Optimize = options.Optimize,
                StrictPilot = options.StrictPilot,
                LicensePolicy = dirLicense.SpdxIdentifier != "UNKNOWN" && dirLicense.SpdxIdentifier != "NOASSERTION" ? PilotLicensePolicy.Any : options.LicensePolicy,
                PreferColonSuites = options.PreferColonSuites,
                FormatOutput = options.FormatOutput,
                SourceFilePath = file
            };
            foreach (var spdx in options.CustomSpdxAllowlist)
                fileOpts.CustomSpdxAllowlist.Add(spdx);

            var res = TranspilationRegistry.Default.Ingest(foreignCode, fileLang, fileOpts);
            if (res.Success)
            {
                File.WriteAllText(outPilotFile, res.Code);
                generatedFiles.Add(outPilotFile);
            }
            else
            {
                diagnostics.AddRange(res.Diagnostics);
            }
        }

        // Generate a pilot.json manifest with SPDX provenance for the newly ingested Pilot library
        string pkgName = new DirectoryInfo(fullSourcePath).Name.ToLowerInvariant();
        string manifestPath = Path.Combine(outputDir, "pilot.json");
        if (!File.Exists(manifestPath))
        {
            string entryPoint = generatedFiles.Count > 0
                ? Path.GetRelativePath(outputDir, generatedFiles[0]).Replace('\\', '/')
                : "main.pilot";
            string spdxId = dirLicense.SpdxIdentifier;
            string provLoc = dirLicense.DetectedFromLocation.Replace('\\', '/');
            string manifestJson = $$"""
{
  "name": "{{pkgName}}",
  "version": "1.0.0",
  "license": "{{spdxId}}",
  "description": "Library transpiled & optimized into native Pilot from '{{pkgName}}'",
  "entryPoint": "{{entryPoint}}",
  "provenance": {
    "spdxIdentifier": "{{spdxId}}",
    "detectedFrom": "{{provLoc}}",
    "sourceDirectory": "{{fullSourcePath.Replace('\\', '/')}}"
  },
  "dependencies": {}
}
""";
            File.WriteAllText(manifestPath, manifestJson);
            generatedFiles.Add(manifestPath);
        }

        if (dirLicense.SpdxIdentifier != "UNKNOWN" || options.LicensePolicy != PilotLicensePolicy.Any)
        {
            string noticesPath = Path.Combine(outputDir, "THIRD_PARTY_NOTICES.md");
            string noticesContent = PilotLicenseVerifier.GenerateThirdPartyNotices(pkgName, fullSourcePath, dirLicense);
            File.WriteAllText(noticesPath, noticesContent);
            generatedFiles.Add(noticesPath);
        }

        var result = new LibraryTranspilationSummary
        {
            Success = diagnostics.Count == 0,
            SourceDirectory = fullSourcePath,
            OutputDirectory = fullOutDir,
            Language = "pilot",
            LicenseInfo = dirLicense
        };
        result.GeneratedFiles.AddRange(generatedFiles);
        result.Diagnostics.AddRange(diagnostics);
        return result;
    }

    /// <summary>
    /// Transpiles an entire Pilot library directory into a target language package directory
    /// (Python package, Rust crate, C/C++ project, JS ESM package, C# project, or Lua module tree).
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

            case "java":
                string pomXml = Path.Combine(outDir, "pom.xml");
                if (!File.Exists(pomXml))
                {
                    File.WriteAllText(pomXml, $"""
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
  <modelVersion>4.0.0</modelVersion>
  <groupId>org.pilot.generated</groupId>
  <artifactId>{libName}</artifactId>
  <version>1.0.0</version>
  <properties>
    <maven.compiler.release>21</maven.compiler.release>
    <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
  </properties>
</project>
""");
                    generated.Add(pomXml);
                }
                break;

            case "ruby" or "rb":
                string gemspec = Path.Combine(outDir, $"{libName}.gemspec");
                if (!File.Exists(gemspec))
                {
                    File.WriteAllText(gemspec, $"""
Gem::Specification.new do |spec|
  spec.name          = "{libName}"
  spec.version       = "1.0.0"
  spec.summary       = "Ruby gem transpiled from Pilot library '{libName}'"
  spec.authors       = ["Pilot Toolchain"]
  spec.files         = Dir["**/*.rb"]
  spec.required_ruby_version = ">= 3.2.0"
end
""");
                    generated.Add(gemspec);
                }
                break;
        }
    }
}
