using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using Antlr4.Runtime;
using Pilot.Diagnostics;
using Pilot.Preprocessing;

namespace Pilot.Compiler;

/// <summary>
/// Primary compiler for Pilot programs.
/// Thread-safe, decoupled, high performance, and supports in-memory compilation caching.
/// </summary>
public sealed class PilotCompiler : IPilotCompiler
{
    private static readonly Lazy<PilotCompiler> DefaultInstance = new(() => new PilotCompiler());
    public static PilotCompiler Default => DefaultInstance.Value;

    private readonly ConcurrentDictionary<string, IPilotProgram> _cache = new();

    /// <summary>
    /// Gets the number of compiled programs currently cached.
    /// </summary>
    public int CachedProgramCount => _cache.Count;

    /// <summary>
    /// Clears the in-memory compiler cache.
    /// </summary>
    public void ClearCache() => _cache.Clear();

    public PilotCompilationResult Compile(string source, PilotCompilerOptions? options = null)
    {
        var sw = Stopwatch.StartNew();
        options ??= new PilotCompilerOptions();

        // 1. Check in-memory compilation cache if enabled
        string cacheKey = string.Empty;
        if (options.EnableCaching)
        {
            cacheKey = ComputeCacheKey(source, options);
            if (_cache.TryGetValue(cacheKey, out var cachedProgram))
            {
                return PilotCompilationResult.CreateSuccess(cachedProgram, sw.Elapsed);
            }
        }

        try
        {
            // 2. Preprocessing step
            string preprocessedCode = source;
            if (options.EnablePreprocessing)
            {
                var preprocOpts = new PilotPreprocessorOptions(options.DefinedSymbols);
                var preproc = PilotPreprocessor.Preprocess(source, options.SourceFilePath ?? string.Empty, preprocOpts);

                if (preproc.HasErrors)
                {
                    var errors = preproc.Diagnostics
                        .Where(d => d.Severity == PreprocessorDiagnosticSeverity.Error)
                        .Select(e => $"[Preproc {e.Line}:{e.Column}] {e.Message}")
                        .ToList();

                    return PilotCompilationResult.CreateFailure(errors, sw.Elapsed);
                }

                preprocessedCode = preproc.PreprocessedSource;
            }

            // 3. Parsing and Syntax Tree Generation
            var inputStream = new AntlrInputStream(preprocessedCode);
            var lexer = new PilotLexer(inputStream);
            var tokenStream = new CommonTokenStream(lexer);
            var parser = new PilotParser(tokenStream);

            var errorTracker = new ErrorTracker();
            parser.RemoveErrorListeners();
            parser.AddErrorListener(errorTracker);

            var tree = parser.ParseProgramFast();

            if (errorTracker.HasErrors)
            {
                return PilotCompilationResult.CreateFailure(errorTracker.Errors, sw.Elapsed);
            }

            // 4. Construct compiled executable program
            var program = new PilotCompiledProgram(
                preprocessedCode,
                tree,
                options.SourceFilePath,
                DateTime.UtcNow,
                options.DefinedSymbols);

            try
            {
                program.BytecodeChunk = PilotBytecodeCompiler.Compile(tree, preprocessedCode);
            }
            catch
            {
                // Graceful fallback to AST interpretation
            }

            // 5. Store in cache if enabled
            if (options.EnableCaching && !string.IsNullOrEmpty(cacheKey))
            {
                _cache[cacheKey] = program;
            }

            return PilotCompilationResult.CreateSuccess(program, sw.Elapsed);
        }
        catch (Exception ex)
        {
            return PilotCompilationResult.CreateFailure($"Compilation fatal error: {ex.Message}", sw.Elapsed);
        }
    }

    public PilotCompilationResult CompileFile(string filePath, PilotCompilerOptions? options = null)
    {
        var sw = Stopwatch.StartNew();
        if (!File.Exists(filePath))
        {
            return PilotCompilationResult.CreateFailure($"File not found: '{filePath}'", sw.Elapsed);
        }

        string source = File.ReadAllText(filePath);
        options ??= new PilotCompilerOptions();
        options.SourceFilePath = filePath;

        return Compile(source, options);
    }

    public IPilotProgram LoadCompiled(Stream stream)
    {
        return PilotCompiledProgram.Load(stream);
    }

    public IPilotProgram LoadCompiled(string filePath)
    {
        return PilotCompiledProgram.Load(filePath);
    }

    public IPilotProgram LoadCompiled(byte[] bytes)
    {
        return PilotCompiledProgram.FromBytes(bytes);
    }

    private static string ComputeCacheKey(string source, PilotCompilerOptions options)
    {
        if (options.DefinedSymbols.Count == 0)
        {
            return string.Concat(
                source.Length.ToString(),
                ":",
                source.GetHashCode(StringComparison.Ordinal).ToString(),
                options.StrictMode ? "|1|" : "|0|",
                source);
        }

        var sb = new System.Text.StringBuilder(source.Length + 64);
        sb.Append(source.Length).Append(':').Append(source.GetHashCode(StringComparison.Ordinal));
        sb.Append('|').Append(options.StrictMode);
        sb.Append('|').Append(string.Join(",", options.DefinedSymbols.OrderBy(s => s, StringComparer.Ordinal)));
        sb.Append('|').Append(source);
        return sb.ToString();
    }
}
