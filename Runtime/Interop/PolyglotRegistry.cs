using System;
using System.Collections.Generic;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Interop;

/// <summary>
/// Registry of active polyglot language providers for Pilot interop.
/// Follows a decoupled, plug-and-play architecture.
/// </summary>
public sealed class PolyglotRegistry
{
    private readonly Dictionary<string, IPolyglotProvider> _providers = new(StringComparer.OrdinalIgnoreCase);
    public IPolyglotTypeConverter TypeBridge { get; set; } = PolyglotTypeBridge.Default;

    public PolyglotRegistry(IPolyglotTypeConverter? typeBridge = null)
    {
        TypeBridge = typeBridge ?? PolyglotTypeBridge.Default;
        // Register default providers for Python, C#, and Rust
        Register(new DefaultPythonProvider(TypeBridge));
        Register(new DefaultCSharpProvider(TypeBridge));
        Register(new DefaultRustProvider(TypeBridge));
    }

    public void Register(IPolyglotProvider provider)
    {
        if (provider == null) throw new ArgumentNullException(nameof(provider));
        _providers[provider.Language] = provider;
    }

    public bool TryGet(string language, out IPolyglotProvider provider) =>
        _providers.TryGetValue(language, out provider!);

    public IPolyglotProvider Get(string language)
    {
        if (TryGet(language, out var provider))
        {
            return provider;
        }

        throw new NotSupportedException($"Polyglot provider for language '{language}' is not registered.");
    }
}

/// <summary>
/// Default polyglot provider for Python interop with typed value marshaling.
/// </summary>
public sealed class DefaultPythonProvider : IPolyglotProvider
{
    public string Language => "python";
    public IPolyglotTypeConverter TypeBridge { get; }

    public DefaultPythonProvider(IPolyglotTypeConverter? typeBridge = null)
    {
        TypeBridge = typeBridge ?? PolyglotTypeBridge.Default;
    }

    public PilotValue ExecuteBlock(string code, PilotExecutionContext context)
    {
        string trimmed = code.Trim();
        // If code ends with an expression or return, extract and marshal equivalent value
        if (trimmed.StartsWith("return ", StringComparison.OrdinalIgnoreCase))
        {
            string expr = trimmed["return ".Length..].Trim();
            return EvaluatePythonLiteralOrMarshal(expr);
        }

        // Check for trailing assignment or expression
        var lines = trimmed.Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
        if (lines.Length > 0)
        {
            string lastLine = lines[^1].Trim();
            int eqIdx = lastLine.IndexOf('=');
            if (eqIdx > 0 && !lastLine.Contains("=="))
            {
                string valExpr = lastLine[(eqIdx + 1)..].Trim();
                return EvaluatePythonLiteralOrMarshal(valExpr);
            }
        }

        return TypeBridge.ToPilotValue(new { Code = trimmed, Language = "Python" });
    }

    public PilotValue Invoke(string memberName, IReadOnlyList<PilotValue> args, PilotExecutionContext context)
    {
        object? result = memberName switch
        {
            "sin" when args.Count > 0 => Math.Sin(args[0].AsFloat()),
            "cos" when args.Count > 0 => Math.Cos(args[0].AsFloat()),
            "tan" when args.Count > 0 => Math.Tan(args[0].AsFloat()),
            "sqrt" when args.Count > 0 => Math.Sqrt(args[0].AsFloat()),
            "pi" => Math.PI,
            "e" => Math.E,
            "dict" => new Dictionary<string, object?>(),
            "list" => new List<object?>(),
            _ => $"python::{memberName}({string.Join(", ", args)})"
        };

        return TypeBridge.ToPilotValue(result, "python");
    }

    public PilotValue Import(string modulePath, string? alias)
    {
        return TypeBridge.ToPilotValue(new { Module = modulePath, Alias = alias, Language = "python" });
    }

    private PilotValue EvaluatePythonLiteralOrMarshal(string expr)
    {
        if (long.TryParse(expr, out long l)) return PilotValue.FromInt(l);
        if (double.TryParse(expr, out double d)) return PilotValue.FromFloat(d);
        if (expr.Equals("True", StringComparison.Ordinal)) return PilotValue.True;
        if (expr.Equals("False", StringComparison.Ordinal)) return PilotValue.False;
        if (expr.Equals("None", StringComparison.Ordinal)) return PilotValue.Null;

        if (expr.StartsWith("\"") && expr.EndsWith("\"") && expr.Length >= 2)
            return PilotValue.FromString(expr[1..^1]);

        return TypeBridge.ToPilotValue(expr, "python");
    }
}

/// <summary>
/// Default polyglot provider for C# (.NET) interop with typed value marshaling.
/// </summary>
public sealed class DefaultCSharpProvider : IPolyglotProvider
{
    public string Language => "csharp";
    public IPolyglotTypeConverter TypeBridge { get; }

    public DefaultCSharpProvider(IPolyglotTypeConverter? typeBridge = null)
    {
        TypeBridge = typeBridge ?? PolyglotTypeBridge.Default;
    }

    public PilotValue ExecuteBlock(string code, PilotExecutionContext context)
    {
        string trimmed = code.Trim();
        if (trimmed.StartsWith("return ", StringComparison.OrdinalIgnoreCase))
        {
            string expr = trimmed["return ".Length..].Trim().TrimEnd(';');
            return EvaluateCSharpLiteralOrMarshal(expr);
        }

        var lines = trimmed.Split(new[] { '\r', '\n', ';' }, StringSplitOptions.RemoveEmptyEntries);
        if (lines.Length > 0)
        {
            string lastLine = lines[^1].Trim();
            int eqIdx = lastLine.IndexOf('=');
            if (eqIdx > 0 && !lastLine.Contains("=="))
            {
                string valExpr = lastLine[(eqIdx + 1)..].Trim();
                return EvaluateCSharpLiteralOrMarshal(valExpr);
            }
        }

        return TypeBridge.ToPilotValue(new { Code = trimmed, Language = "CSharp" });
    }

    public PilotValue Invoke(string memberName, IReadOnlyList<PilotValue> args, PilotExecutionContext context)
    {
        object? result = memberName switch
        {
            "NewGuid" => Guid.NewGuid(),
            "Now" => DateTime.UtcNow,
            "Sin" when args.Count > 0 => Math.Sin(args[0].AsFloat()),
            "Cos" when args.Count > 0 => Math.Cos(args[0].AsFloat()),
            "Abs" when args.Count > 0 => Math.Abs(args[0].AsFloat()),
            _ => $"csharp::{memberName}({string.Join(", ", args)})"
        };

        return TypeBridge.ToPilotValue(result, "csharp");
    }

    public PilotValue Import(string modulePath, string? alias)
    {
        return TypeBridge.ToPilotValue(new { Assembly = modulePath, Alias = alias, Language = "csharp" });
    }

    private PilotValue EvaluateCSharpLiteralOrMarshal(string expr)
    {
        if (long.TryParse(expr, out long l)) return PilotValue.FromInt(l);
        if (double.TryParse(expr, out double d)) return PilotValue.FromFloat(d);
        if (expr.Equals("true", StringComparison.OrdinalIgnoreCase)) return PilotValue.True;
        if (expr.Equals("false", StringComparison.OrdinalIgnoreCase)) return PilotValue.False;
        if (expr.Equals("null", StringComparison.OrdinalIgnoreCase)) return PilotValue.Null;

        if (expr.StartsWith("\"") && expr.EndsWith("\"") && expr.Length >= 2)
            return PilotValue.FromString(expr[1..^1]);

        return TypeBridge.ToPilotValue(expr, "csharp");
    }
}

/// <summary>
/// Default polyglot provider for Rust interop with typed value marshaling.
/// </summary>
public sealed class DefaultRustProvider : IPolyglotProvider
{
    public string Language => "rust";
    public IPolyglotTypeConverter TypeBridge { get; }

    public DefaultRustProvider(IPolyglotTypeConverter? typeBridge = null)
    {
        TypeBridge = typeBridge ?? PolyglotTypeBridge.Default;
    }

    public PilotValue ExecuteBlock(string code, PilotExecutionContext context)
    {
        string trimmed = code.Trim();
        var lines = trimmed.Split(new[] { '\r', '\n', ';' }, StringSplitOptions.RemoveEmptyEntries);
        if (lines.Length > 0)
        {
            string lastLine = lines[^1].Trim();
            int eqIdx = lastLine.IndexOf('=');
            if (eqIdx > 0 && !lastLine.Contains("=="))
            {
                string valExpr = lastLine[(eqIdx + 1)..].Trim();
                if (long.TryParse(valExpr, out long l)) return PilotValue.FromInt(l);
                if (double.TryParse(valExpr, out double d)) return PilotValue.FromFloat(d);
            }
        }

        return TypeBridge.ToPilotValue(new { Code = trimmed, Language = "Rust" });
    }

    public PilotValue Invoke(string memberName, IReadOnlyList<PilotValue> args, PilotExecutionContext context)
    {
        return TypeBridge.ToPilotValue($"rust::{memberName}({string.Join(", ", args)})", "rust");
    }

    public PilotValue Import(string modulePath, string? alias)
    {
        return TypeBridge.ToPilotValue(new { Crate = modulePath, Alias = alias, Language = "rust" });
    }
}
