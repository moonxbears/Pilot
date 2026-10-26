using System;
using System.Collections.Generic;
using Pilot.Transpilation.Targets;

namespace Pilot.Transpilation;

/// <summary>
/// Registry of active compile-time target language converters for Pilot.
/// Follows a decoupled, plug-and-play architecture.
/// </summary>
public sealed class TranspilationRegistry
{
    private static readonly Lazy<TranspilationRegistry> _default = new(() => new TranspilationRegistry());
    public static TranspilationRegistry Default => _default.Value;

    private readonly Dictionary<string, ITargetLanguageConverter> _converters = new(StringComparer.OrdinalIgnoreCase);

    public TranspilationRegistry()
    {
        Register(new PythonTargetConverter());
        Register(new CSharpTargetConverter());
        Register(new RustTargetConverter());
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
        return converter.TranspileSource(pilotSource, options ?? new TranspilationOptions());
    }

    public TranspilationResult Ingest(string foreignSource, string sourceLanguage, TranspilationOptions? options = null)
    {
        var converter = Get(sourceLanguage);
        return converter.IngestToPilot(foreignSource, options ?? new TranspilationOptions());
    }
}
