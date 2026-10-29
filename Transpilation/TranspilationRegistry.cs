using System;
using System.Collections.Generic;
using System.IO;
using Pilot.Transpilation.Targets;

namespace Pilot.Transpilation;

/// <summary>
/// Registry of active compile-time target language converters for Pilot.
/// Supports Python, Lua, JavaScript/TypeScript, C++, C, Rust, and C#.
/// </summary>
public sealed class TranspilationRegistry
{
    private static readonly Lazy<TranspilationRegistry> _default = new(() => new TranspilationRegistry());
    public static TranspilationRegistry Default => _default.Value;

    private readonly Dictionary<string, ITargetLanguageConverter> _converters = new(StringComparer.OrdinalIgnoreCase);

    public TranspilationRegistry()
    {
        Register(new PythonTargetConverter("python"));
        Register(new PythonTargetConverter("py"));
        Register(new LuaTargetConverter());
        Register(new JavaScriptTargetConverter("js"));
        Register(new JavaScriptTargetConverter("javascript"));
        Register(new JavaScriptTargetConverter("ts"));
        Register(new JavaScriptTargetConverter("typescript"));
        Register(new CTargetConverter());
        Register(new CppTargetConverter("cpp"));
        Register(new CppTargetConverter("c++"));
        Register(new RustTargetConverter("rust"));
        Register(new RustTargetConverter("rs"));
        Register(new JavaTargetConverter("java"));
        Register(new RubyTargetConverter("ruby"));
        Register(new RubyTargetConverter("rb"));
        Register(new CSharpTargetConverter());
        _converters["cs"] = new CSharpTargetConverter();
    }

    public void Register(ITargetLanguageConverter converter)
    {
        if (converter == null) throw new ArgumentNullException(nameof(converter));
        _converters[converter.Language] = converter;
    }

    public bool TryGet(string language, out ITargetLanguageConverter converter) =>
        _converters.TryGetValue(language, out converter!);

    public ITargetLanguageConverter Get(string language)
    {
        if (TryGet(language, out var converter))
        {
            return converter;
        }

        throw new NotSupportedException($"No target language converter registered for '{language}'. Available targets: {string.Join(", ", _converters.Keys)}");
    }

    public IReadOnlyCollection<string> SupportedLanguages => _converters.Keys;

    public TranspilationResult Transpile(string pilotSource, string targetLanguage, TranspilationOptions? options = null)
    {
        var converter = Get(targetLanguage);
        var res = converter.TranspileSource(pilotSource, options ?? new TranspilationOptions());
        if (!res.Success)
            return res;

        string emittedCode = res.Code;
        if (PilotLibraryMapper.UsesMatrixPrimitives(pilotSource))
        {
            string preamble = PilotLibraryMapper.GetMatrixRuntimePreamble(targetLanguage);
            if (!string.IsNullOrEmpty(preamble) && !emittedCode.Contains(preamble.Trim(), StringComparison.Ordinal))
            {
                emittedCode = InjectPreambleAfterHeaders(emittedCode, preamble, targetLanguage);
            }
        }
        return TranspilationResult.CreateSuccess(emittedCode, res.TargetLanguage);
    }

    public TranspilationResult Ingest(string foreignSource, string sourceLanguage, TranspilationOptions? options = null)
    {
        var converter = Get(sourceLanguage);
        return converter.IngestToPilot(foreignSource, options ?? new TranspilationOptions());
    }

    private static string InjectPreambleAfterHeaders(string code, string preamble, string targetLang)
    {
        string lang = targetLang.ToLowerInvariant();
        if (lang is "c" or "cpp" or "c++")
        {
            int lastInclude = code.LastIndexOf("#include", StringComparison.Ordinal);
            if (lastInclude >= 0)
            {
                int lineEnd = code.IndexOf('\n', lastInclude);
                if (lineEnd >= 0)
                {
                    return code[..(lineEnd + 1)] + Environment.NewLine + preamble.Trim() + Environment.NewLine + code[(lineEnd + 1)..];
                }
            }
        }
        else if (lang is "python" or "py")
        {
            int lastImport = code.LastIndexOf("from typing import", StringComparison.Ordinal);
            if (lastImport >= 0)
            {
                int lineEnd = code.IndexOf('\n', lastImport);
                if (lineEnd >= 0)
                {
                    return code[..(lineEnd + 1)] + preamble.Trim() + Environment.NewLine + code[(lineEnd + 1)..];
                }
            }
        }
        else if (lang is "js" or "javascript" or "ts" or "typescript")
        {
            int useStrict = code.IndexOf("\"use strict\";", StringComparison.Ordinal);
            if (useStrict >= 0)
            {
                int lineEnd = code.IndexOf('\n', useStrict);
                if (lineEnd >= 0)
                {
                    return code[..(lineEnd + 1)] + Environment.NewLine + preamble.Trim() + Environment.NewLine + code[(lineEnd + 1)..];
                }
            }
        }
        else if (lang == "java")
        {
            // Keep ProgramModule as the first top-level class so JEP 330 `java file.java` executes ProgramModule.main
            return code.TrimEnd() + Environment.NewLine + Environment.NewLine + preamble.Trim() + Environment.NewLine;
        }
        else if (lang is "ruby" or "rb")
        {
            int lastReq = code.LastIndexOf("require ", StringComparison.Ordinal);
            int anchor = lastReq >= 0 ? lastReq : code.IndexOf("# frozen_string_literal:", StringComparison.Ordinal);
            if (anchor >= 0)
            {
                int lineEnd = code.IndexOf('\n', anchor);
                if (lineEnd >= 0)
                {
                    return code[..(lineEnd + 1)] + Environment.NewLine + preamble.Trim() + Environment.NewLine + code[(lineEnd + 1)..];
                }
            }
        }
        return preamble.Trim() + Environment.NewLine + Environment.NewLine + code;
    }

    /// <summary>
    /// Infers the source/target language identifier from a file path extension.
    /// </summary>
    public static string InferLanguageFromPath(string filePath)
    {
        string ext = Path.GetExtension(filePath).ToLowerInvariant();
        return ext switch
        {
            ".py" => "python",
            ".lua" => "lua",
            ".js" or ".mjs" or ".cjs" => "js",
            ".ts" => "ts",
            ".c" or ".h" => "c",
            ".cpp" or ".cc" or ".cxx" or ".hpp" => "cpp",
            ".rs" => "rust",
            ".cs" => "csharp",
            ".java" => "java",
            ".rb" => "ruby",
            _ => "python"
        };
    }

    /// <summary>
    /// Returns the default file extension for a given target language.
    /// </summary>
    public static string GetDefaultExtension(string targetLanguage) => targetLanguage.ToLowerInvariant() switch
    {
        "python" or "py" => ".py",
        "lua" => ".lua",
        "js" or "javascript" => ".js",
        "ts" or "typescript" => ".ts",
        "c" => ".c",
        "cpp" or "c++" => ".cpp",
        "rust" or "rs" => ".rs",
        "csharp" or "cs" => ".cs",
        "java" => ".java",
        "ruby" or "rb" => ".rb",
        _ => ".txt"
    };
}
