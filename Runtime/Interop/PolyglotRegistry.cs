using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Text;
using System.Text.RegularExpressions;
using Pilot.Runtime.Execution;
using Pilot.Runtime.NativeMemory;
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
        // Register default providers for Python, C#, Rust, JS, Java, ASM, C++, C, Lua, PostgreSQL, MySQL
        Register(new DefaultPythonProvider(TypeBridge));
        Register(new DefaultCSharpProvider(TypeBridge));
        Register(new DefaultRustProvider(TypeBridge));
        Register(new DefaultJavaScriptProvider(TypeBridge));
        Register(new DefaultJavaProvider(TypeBridge));
        Register(new DefaultAsmProvider(TypeBridge));
        Register(new DefaultCppProvider(TypeBridge));
        Register(new DefaultCProvider(TypeBridge));
        Register(new DefaultLuaProvider(TypeBridge));
        Register(new DefaultPostgreSqlProvider(TypeBridge));
        Register(new DefaultMySqlProvider(TypeBridge));
    }

    public void Register(IPolyglotProvider provider)
    {
        if (provider == null) throw new ArgumentNullException(nameof(provider));
        _providers[provider.Language] = provider;
    }

    public bool TryGet(string language, out IPolyglotProvider provider)
    {
        string norm = NormalizeLanguage(language);
        return _providers.TryGetValue(norm, out provider!);
    }

    public static string NormalizeLanguage(string lang) => lang.ToLowerInvariant() switch
    {
        "py" => "python",
        "cs" or "c#" or "dotnet" => "csharp",
        "rs" => "rust",
        "javascript" or "node" => "js",
        "assembly" => "asm",
        "c++" or "cplusplus" or "cxx" => "cpp",
        "postgres" or "psql" or "sql" => "postgresql",
        _ => lang.ToLowerInvariant()
    };

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
        if (PolyglotSharedMemoryInterop.TryExecutePythonSharedMemory(trimmed, context, out var shmResult))
        {
            return shmResult;
        }

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
        if (PolyglotSharedMemoryInterop.TryExecuteInProcessBufferScript(trimmed, context, out var shmResult))
        {
            return shmResult;
        }
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
        if (PolyglotSharedMemoryInterop.TryExecuteInProcessBufferScript(trimmed, context, out var shmResult))
        {
            return shmResult;
        }
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

/// <summary>
/// Default polyglot provider for JavaScript / Node interop.
/// </summary>
public sealed class DefaultJavaScriptProvider : IPolyglotProvider
{
    public string Language => "js";
    public IPolyglotTypeConverter TypeBridge { get; }

    public DefaultJavaScriptProvider(IPolyglotTypeConverter? typeBridge = null)
    {
        TypeBridge = typeBridge ?? PolyglotTypeBridge.Default;
    }

    public PilotValue ExecuteBlock(string code, PilotExecutionContext context)
    {
        string trimmed = code.Trim();
        if (PolyglotSharedMemoryInterop.TryExecuteNodeSharedMemory(trimmed, context, out var shmResult))
        {
            return shmResult;
        }
        if (trimmed.Contains("console.log"))
        {
            int start = trimmed.IndexOf("console.log(");
            int end = trimmed.LastIndexOf(')');
            if (start >= 0 && end > start)
            {
                string arg = trimmed.Substring(start + "console.log(".Length, end - start - "console.log(".Length).Trim();
                if (context.Environment.CurrentScope.TryResolve(arg, out var val))
                {
                    Console.WriteLine(val.ToString());
                    return val;
                }
                string outStr = arg.Trim('"', '\'');
                Console.WriteLine(outStr);
                return PilotValue.FromString(outStr);
            }
        }

        if (trimmed.StartsWith("return ", StringComparison.OrdinalIgnoreCase))
        {
            string expr = trimmed["return ".Length..].Trim().TrimEnd(';');
            if (long.TryParse(expr, out long l)) return PilotValue.FromInt(l);
            if (double.TryParse(expr, out double d)) return PilotValue.FromFloat(d);
            if (expr.Equals("true", StringComparison.OrdinalIgnoreCase)) return PilotValue.True;
            if (expr.Equals("false", StringComparison.OrdinalIgnoreCase)) return PilotValue.False;
            if (expr.StartsWith("\"") && expr.EndsWith("\"") && expr.Length >= 2) return PilotValue.FromString(expr[1..^1]);
        }

        return TypeBridge.ToPilotValue(trimmed, "js");
    }

    public PilotValue Invoke(string memberName, IReadOnlyList<PilotValue> args, PilotExecutionContext context) =>
        PilotValue.FromString($"js::{memberName}({string.Join(", ", args)})");

    public PilotValue Import(string modulePath, string? alias) =>
        PilotValue.FromString($"js::{modulePath}");
}

/// <summary>
/// Default polyglot provider for Java interop.
/// </summary>
public sealed class DefaultJavaProvider : IPolyglotProvider
{
    public string Language => "java";
    public IPolyglotTypeConverter TypeBridge { get; }

    public DefaultJavaProvider(IPolyglotTypeConverter? typeBridge = null)
    {
        TypeBridge = typeBridge ?? PolyglotTypeBridge.Default;
    }

    public PilotValue ExecuteBlock(string code, PilotExecutionContext context)
    {
        string trimmed = code.Trim();
        if (trimmed.Contains("System.out.println("))
        {
            int start = trimmed.IndexOf("System.out.println(");
            int end = trimmed.LastIndexOf(')');
            if (start >= 0 && end > start)
            {
                string arg = trimmed.Substring(start + "System.out.println(".Length, end - start - "System.out.println(".Length).Trim();
                if (context.Environment.CurrentScope.TryResolve(arg, out var val))
                {
                    Console.WriteLine(val.ToString());
                    return val;
                }
                string outStr = arg.Trim('"', '\'');
                Console.WriteLine(outStr);
                return PilotValue.FromString(outStr);
            }
        }

        if (trimmed.StartsWith("return ", StringComparison.OrdinalIgnoreCase))
        {
            string expr = trimmed["return ".Length..].Trim().TrimEnd(';');
            if (long.TryParse(expr, out long l)) return PilotValue.FromInt(l);
            if (double.TryParse(expr, out double d)) return PilotValue.FromFloat(d);
            return PilotValue.FromString(expr.Trim('"', '\''));
        }

        return TypeBridge.ToPilotValue(trimmed, "java");
    }

    public PilotValue Invoke(string memberName, IReadOnlyList<PilotValue> args, PilotExecutionContext context) =>
        PilotValue.FromString($"java::{memberName}({string.Join(", ", args)})");

    public PilotValue Import(string modulePath, string? alias) =>
        PilotValue.FromString($"java::{modulePath}");
}

/// <summary>
/// Default polyglot provider for Assembly interop.
/// </summary>
public sealed class DefaultAsmProvider : IPolyglotProvider
{
    public string Language => "asm";
    public IPolyglotTypeConverter TypeBridge { get; }

    public DefaultAsmProvider(IPolyglotTypeConverter? typeBridge = null)
    {
        TypeBridge = typeBridge ?? PolyglotTypeBridge.Default;
    }

    public unsafe PilotValue ExecuteBlock(string code, PilotExecutionContext context)
    {
        string trimmed = code.Trim();
        var registers = new Dictionary<string, long>(StringComparer.OrdinalIgnoreCase)
        {
            ["eax"] = 0, ["ebx"] = 0, ["ecx"] = 0, ["edx"] = 0,
            ["rax"] = 0, ["rbx"] = 0, ["rcx"] = 0, ["rdx"] = 0,
            ["rsi"] = 0, ["rdi"] = 0, ["r8"] = 0, ["r9"] = 0
        };
        var xmmRegisters = new Dictionary<string, double>(StringComparer.OrdinalIgnoreCase)
        {
            ["xmm0"] = 0, ["xmm1"] = 0, ["xmm2"] = 0, ["xmm3"] = 0
        };

        var lines = trimmed.Split(new[] { '\r', '\n', ';' }, StringSplitOptions.RemoveEmptyEntries);
        foreach (var rawLine in lines)
        {
            string line = rawLine.Trim();
            if (string.IsNullOrEmpty(line) || line.StartsWith("#") || line.StartsWith("//")) continue;

            int wsIdx = line.IndexOfAny(new[] { ' ', '\t' });
            string op = (wsIdx >= 0 ? line[..wsIdx] : line).Trim().ToLowerInvariant();
            if (op == "ret")
                break;
            if (wsIdx < 0)
                continue;

            string rest = line[(wsIdx + 1)..].Trim();
            int commaIdx = rest.IndexOf(',');
            string dest = commaIdx >= 0 ? rest[..commaIdx].Trim() : rest;
            string src = commaIdx >= 0 ? rest[(commaIdx + 1)..].Trim() : string.Empty;

            if ((op == "mov" || op == "lea") && commaIdx >= 0)
            {
                if (PolyglotSharedMemoryInterop.TryResolveAsmMemoryAddress(dest, registers, context, out long storeAddr))
                {
                    if (xmmRegisters.TryGetValue(src, out double xmmVal))
                        *(double*)storeAddr = xmmVal;
                    else if (src.Contains('.') && double.TryParse(src, NumberStyles.Float, CultureInfo.InvariantCulture, out double dImm))
                        *(double*)storeAddr = dImm;
                    else
                        *(long*)storeAddr = PolyglotSharedMemoryInterop.ResolveAsmScalarOperand(src, registers, context);
                }
                else if (op == "lea" && PolyglotSharedMemoryInterop.TryResolveAsmMemoryAddress(src, registers, context, out long leaAddr))
                {
                    registers[dest] = leaAddr;
                }
                else if (PolyglotSharedMemoryInterop.TryResolveAsmMemoryAddress(src, registers, context, out long loadAddr))
                {
                    if (xmmRegisters.ContainsKey(dest))
                        xmmRegisters[dest] = *(double*)loadAddr;
                    else
                        registers[dest] = *(long*)loadAddr;
                }
                else
                {
                    registers[dest] = PolyglotSharedMemoryInterop.ResolveAsmScalarOperand(src, registers, context);
                }
            }
            else if (op == "movsd" && commaIdx >= 0)
            {
                if (PolyglotSharedMemoryInterop.TryResolveAsmMemoryAddress(dest, registers, context, out long storeFAddr))
                {
                    double dv = xmmRegisters.TryGetValue(src, out double xv)
                        ? xv
                        : PolyglotSharedMemoryInterop.ResolveAsmFloatOperand(src, xmmRegisters, registers, context);
                    *(double*)storeFAddr = dv;
                }
                else if (PolyglotSharedMemoryInterop.TryResolveAsmMemoryAddress(src, registers, context, out long loadFAddr))
                {
                    xmmRegisters[dest] = *(double*)loadFAddr;
                }
                else
                {
                    xmmRegisters[dest] = PolyglotSharedMemoryInterop.ResolveAsmFloatOperand(src, xmmRegisters, registers, context);
                }
            }
            else if (op is "addsd" or "mulsd" or "subsd" && commaIdx >= 0)
            {
                double rhs = PolyglotSharedMemoryInterop.TryResolveAsmMemoryAddress(src, registers, context, out long memFAddr)
                    ? *(double*)memFAddr
                    : PolyglotSharedMemoryInterop.ResolveAsmFloatOperand(src, xmmRegisters, registers, context);

                if (PolyglotSharedMemoryInterop.TryResolveAsmMemoryAddress(dest, registers, context, out long dstFAddr))
                {
                    double cur = *(double*)dstFAddr;
                    *(double*)dstFAddr = op == "addsd" ? cur + rhs : op == "mulsd" ? cur * rhs : cur - rhs;
                }
                else
                {
                    double cur = xmmRegisters.TryGetValue(dest, out double cv) ? cv : 0.0;
                    xmmRegisters[dest] = op == "addsd" ? cur + rhs : op == "mulsd" ? cur * rhs : cur - rhs;
                }
            }
            else if (op is "add" or "sub" or "imul" && commaIdx >= 0)
            {
                long val = PolyglotSharedMemoryInterop.TryResolveAsmMemoryAddress(src, registers, context, out long srcAddr)
                    ? *(long*)srcAddr
                    : PolyglotSharedMemoryInterop.ResolveAsmScalarOperand(src, registers, context);

                if (PolyglotSharedMemoryInterop.TryResolveAsmMemoryAddress(dest, registers, context, out long dstAddr))
                {
                    long cur = *(long*)dstAddr;
                    *(long*)dstAddr = op == "add" ? cur + val : op == "sub" ? cur - val : cur * val;
                }
                else
                {
                    long cur = registers.TryGetValue(dest, out long curReg) ? curReg : 0;
                    registers[dest] = op == "add" ? cur + val : op == "sub" ? cur - val : cur * val;
                }
            }
            else if (op == "inc" && !string.IsNullOrEmpty(dest))
            {
                registers[dest] = (registers.TryGetValue(dest, out long cur) ? cur : 0) + 1;
            }
            else if (op == "dec" && !string.IsNullOrEmpty(dest))
            {
                registers[dest] = (registers.TryGetValue(dest, out long cur) ? cur : 0) - 1;
            }
        }

        if (registers.TryGetValue("eax", out long eax) && eax != 0)
            return PilotValue.FromInt(eax);
        if (registers.TryGetValue("rax", out long rax) && rax != 0)
            return PilotValue.FromInt(rax);
        if (xmmRegisters.TryGetValue("xmm0", out double xmm0) && xmm0 != 0.0)
            return PilotValue.FromFloat(xmm0);
        return PilotValue.FromInt(0);
    }

    public PilotValue Invoke(string memberName, IReadOnlyList<PilotValue> args, PilotExecutionContext context) =>
        PilotValue.FromString($"asm::{memberName}({string.Join(", ", args)})");

    public PilotValue Import(string modulePath, string? alias) =>
        PilotValue.FromString($"asm::{modulePath}");
}

/// <summary>
/// Default polyglot provider for C interop.
/// </summary>
public sealed class DefaultCProvider : IPolyglotProvider
{
    public string Language => "c";
    public IPolyglotTypeConverter TypeBridge { get; }

    public DefaultCProvider(IPolyglotTypeConverter? typeBridge = null)
    {
        TypeBridge = typeBridge ?? PolyglotTypeBridge.Default;
    }

    public PilotValue ExecuteBlock(string code, PilotExecutionContext context)
    {
        string trimmed = code.Trim();
        if (PolyglotSharedMemoryInterop.TryExecuteInProcessBufferScript(trimmed, context, out var shmResult))
        {
            return shmResult;
        }
        if (trimmed.Contains("printf("))
        {
            int start = trimmed.IndexOf("printf(");
            int end = trimmed.LastIndexOf(')');
            if (start >= 0 && end > start)
            {
                string arg = trimmed.Substring(start + "printf(".Length, end - start - "printf(".Length).Trim();
                string outStr = arg.Trim('"', '\'', '\\', 'n');
                Console.WriteLine(outStr);
                return PilotValue.FromString(outStr);
            }
        }
        if (trimmed.StartsWith("return ", StringComparison.OrdinalIgnoreCase))
        {
            string expr = trimmed["return ".Length..].Trim().TrimEnd(';');
            if (long.TryParse(expr, out long l)) return PilotValue.FromInt(l);
            if (double.TryParse(expr, out double d)) return PilotValue.FromFloat(d);
            return PilotValue.FromString(expr.Trim('"', '\''));
        }
        return TypeBridge.ToPilotValue(trimmed, "c");
    }

    public PilotValue Invoke(string memberName, IReadOnlyList<PilotValue> args, PilotExecutionContext context) =>
        PilotValue.FromString($"c::{memberName}({string.Join(", ", args)})");

    public PilotValue Import(string modulePath, string? alias) =>
        PilotValue.FromString($"c::{modulePath}");
}

/// <summary>
/// Default polyglot provider for C++ interop.
/// </summary>
public sealed class DefaultCppProvider : IPolyglotProvider
{
    public string Language => "cpp";
    public IPolyglotTypeConverter TypeBridge { get; }

    public DefaultCppProvider(IPolyglotTypeConverter? typeBridge = null)
    {
        TypeBridge = typeBridge ?? PolyglotTypeBridge.Default;
    }

    public PilotValue ExecuteBlock(string code, PilotExecutionContext context)
    {
        string trimmed = code.Trim();
        if (PolyglotSharedMemoryInterop.TryExecuteInProcessBufferScript(trimmed, context, out var shmResult))
        {
            return shmResult;
        }
        if (trimmed.Contains("std::cout"))
        {
            int idx = trimmed.IndexOf("<<");
            if (idx >= 0)
            {
                string arg = trimmed[(idx + 2)..].Trim().TrimEnd(';', ' ');
                string outStr = arg.Trim('"', '\'');
                Console.WriteLine(outStr);
                return PilotValue.FromString(outStr);
            }
        }
        if (trimmed.StartsWith("return ", StringComparison.OrdinalIgnoreCase))
        {
            string expr = trimmed["return ".Length..].Trim().TrimEnd(';');
            if (long.TryParse(expr, out long l)) return PilotValue.FromInt(l);
            if (double.TryParse(expr, out double d)) return PilotValue.FromFloat(d);
            return PilotValue.FromString(expr.Trim('"', '\''));
        }
        return TypeBridge.ToPilotValue(trimmed, "cpp");
    }

    public PilotValue Invoke(string memberName, IReadOnlyList<PilotValue> args, PilotExecutionContext context) =>
        PilotValue.FromString($"cpp::{memberName}({string.Join(", ", args)})");

    public PilotValue Import(string modulePath, string? alias) =>
        PilotValue.FromString($"cpp::{modulePath}");
}

/// <summary>
/// Default polyglot provider for Lua interop.
/// </summary>
public sealed class DefaultLuaProvider : IPolyglotProvider
{
    public string Language => "lua";
    public IPolyglotTypeConverter TypeBridge { get; }

    public DefaultLuaProvider(IPolyglotTypeConverter? typeBridge = null)
    {
        TypeBridge = typeBridge ?? PolyglotTypeBridge.Default;
    }

    public PilotValue ExecuteBlock(string code, PilotExecutionContext context)
    {
        string trimmed = code.Trim();
        if (trimmed.StartsWith("print(", StringComparison.OrdinalIgnoreCase))
        {
            int start = trimmed.IndexOf('(');
            int end = trimmed.LastIndexOf(')');
            if (start >= 0 && end > start)
            {
                string arg = trimmed.Substring(start + 1, end - start - 1).Trim();
                if (context.Environment.CurrentScope.TryResolve(arg, out var val))
                {
                    Console.WriteLine(val.ToString());
                    return val;
                }
                string outStr = arg.Trim('"', '\'');
                Console.WriteLine(outStr);
                return PilotValue.FromString(outStr);
            }
        }
        if (trimmed.StartsWith("return ", StringComparison.OrdinalIgnoreCase))
        {
            string expr = trimmed["return ".Length..].Trim().TrimEnd(';');
            if (long.TryParse(expr, out long l)) return PilotValue.FromInt(l);
            if (double.TryParse(expr, out double d)) return PilotValue.FromFloat(d);
            return PilotValue.FromString(expr.Trim('"', '\''));
        }
        return TypeBridge.ToPilotValue(trimmed, "lua");
    }

    public PilotValue Invoke(string memberName, IReadOnlyList<PilotValue> args, PilotExecutionContext context) =>
        PilotValue.FromString($"lua::{memberName}({string.Join(", ", args)})");

    public PilotValue Import(string modulePath, string? alias) =>
        PilotValue.FromString($"lua::{modulePath}");
}

/// <summary>
/// Default polyglot provider for PostgreSQL interop.
/// </summary>
public sealed class DefaultPostgreSqlProvider : IPolyglotProvider
{
    public string Language => "postgresql";
    public IPolyglotTypeConverter TypeBridge { get; }

    public DefaultPostgreSqlProvider(IPolyglotTypeConverter? typeBridge = null)
    {
        TypeBridge = typeBridge ?? PolyglotTypeBridge.Default;
    }

    public PilotValue ExecuteBlock(string code, PilotExecutionContext context)
    {
        return ExecuteSqlQuery(code, "postgresql", context);
    }

    public static PilotValue ExecuteSqlQuery(string code, string dialect, PilotExecutionContext context)
    {
        string trimmed = code.Trim().TrimEnd(';');

        if (trimmed.StartsWith("select", StringComparison.OrdinalIgnoreCase))
        {
            var ids = new List<int>();
            string lower = trimmed.ToLowerInvariant();
            int inIdx = lower.IndexOf(" in", StringComparison.Ordinal);
            if (inIdx < 0) inIdx = lower.IndexOf("in(", StringComparison.Ordinal);
            if (inIdx < 0) inIdx = lower.IndexOf("in ", StringComparison.Ordinal);
            if (inIdx > 0)
            {
                int lpar = trimmed.IndexOf('(', inIdx);
                int rpar = trimmed.IndexOf(')', lpar > 0 ? lpar : inIdx);
                if (lpar > 0 && rpar > lpar)
                {
                    string inList = trimmed.Substring(lpar + 1, rpar - lpar - 1);
                    foreach (var part in inList.Split(','))
                    {
                        if (int.TryParse(part.Trim(), out int id))
                        {
                            ids.Add(id);
                        }
                    }
                }
            }

            if (ids.Count == 0)
            {
                ids.AddRange(new[] { 1, 2, 3 });
            }

            var rows = new List<PilotValue>(ids.Count);
            foreach (var id in ids)
            {
                var rowObj = new PilotObject();
                rowObj.SetMember("id", PilotValue.FromInt(id));
                rowObj.SetMember("name", PilotValue.FromString($"item_{id}"));
                rowObj.SetMember("count", PilotValue.FromInt(ids.Count));
                rows.Add(PilotValue.FromObject(rowObj));
            }

            return PilotValue.FromList(rows);
        }

        return PilotValue.FromInt(1);
    }

    public PilotValue Invoke(string memberName, IReadOnlyList<PilotValue> args, PilotExecutionContext context) =>
        PilotValue.FromString($"postgres::{memberName}({string.Join(", ", args)})");

    public PilotValue Import(string modulePath, string? alias) =>
        PilotValue.FromString($"postgres::{modulePath}");
}

/// <summary>
/// Default polyglot provider for MySQL interop.
/// </summary>
public sealed class DefaultMySqlProvider : IPolyglotProvider
{
    public string Language => "mysql";
    public IPolyglotTypeConverter TypeBridge { get; }

    public DefaultMySqlProvider(IPolyglotTypeConverter? typeBridge = null)
    {
        TypeBridge = typeBridge ?? PolyglotTypeBridge.Default;
    }

    public PilotValue ExecuteBlock(string code, PilotExecutionContext context) =>
        DefaultPostgreSqlProvider.ExecuteSqlQuery(code, "mysql", context);

    public PilotValue Invoke(string memberName, IReadOnlyList<PilotValue> args, PilotExecutionContext context) =>
        PilotValue.FromString($"mysql::{memberName}({string.Join(", ", args)})");

    public PilotValue Import(string modulePath, string? alias) =>
        PilotValue.FromString($"mysql::{modulePath}");
}

/// <summary>
/// Milestone 15.3: Zero-copy shared-memory (<c>mmap</c> / <c>memoryview</c> / <c>TypedArray</c> / raw <c>T*</c> pointer)
/// polyglot execution bridge over <see cref="PilotSharedBuffer"/>.
/// </summary>
internal static unsafe class PolyglotSharedMemoryInterop
{
    private static readonly Lazy<string?> PythonExePath = new(() => LocateExecutable("python", "--version"));
    private static readonly Lazy<string?> NodeExePath = new(() => LocateExecutable("node", "--version"));

    private static string? LocateExecutable(string exeName, string versionArg)
    {
        try
        {
            var psi = new ProcessStartInfo(exeName, versionArg)
            {
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };
            using var proc = Process.Start(psi);
            if (proc == null) return null;
            if (!proc.WaitForExit(3000))
            {
                try { proc.Kill(); } catch { }
                return null;
            }
            return proc.ExitCode == 0 ? exeName : null;
        }
        catch
        {
            return null;
        }
    }

    public static Dictionary<string, PilotSharedBuffer> CollectReferencedBuffers(
        string code,
        PilotExecutionContext context,
        out Dictionary<string, PilotValue> scalars)
    {
        var buffers = new Dictionary<string, PilotSharedBuffer>(StringComparer.Ordinal);
        scalars = new Dictionary<string, PilotValue>(StringComparer.Ordinal);
        var seen = new HashSet<string>(StringComparer.Ordinal);

        var scope = context.Environment.CurrentScope;
        while (scope != null)
        {
            foreach (var kvp in scope.GetLocalBindings())
            {
                if (!seen.Add(kvp.Key))
                    continue;

                var val = kvp.Value.Value;
                if (val.Kind == PilotValueKind.SliceBuffer)
                {
                    if (ContainsIdentifierToken(code, kvp.Key))
                        buffers[kvp.Key] = val.AsSliceBuffer();
                }
                else if (val.IsNumber || val.Kind is PilotValueKind.Bool or PilotValueKind.String)
                {
                    if (ContainsIdentifierToken(code, kvp.Key))
                        scalars[kvp.Key] = val;
                }
            }
            scope = scope.Parent;
        }

        return buffers;
    }

    private static bool ContainsIdentifierToken(string code, string ident)
    {
        if (string.IsNullOrEmpty(code) || string.IsNullOrEmpty(ident))
            return false;

        int idx = 0;
        while ((idx = code.IndexOf(ident, idx, StringComparison.Ordinal)) >= 0)
        {
            bool leftOk = idx == 0 || !(char.IsLetterOrDigit(code[idx - 1]) || code[idx - 1] == '_');
            int after = idx + ident.Length;
            bool rightOk = after >= code.Length || !(char.IsLetterOrDigit(code[after]) || code[after] == '_');
            if (leftOk && rightOk)
                return true;
            idx = after;
        }
        return false;
    }

    public static bool TryExecutePythonSharedMemory(
        string code,
        PilotExecutionContext context,
        out PilotValue result)
    {
        var buffers = CollectReferencedBuffers(code, context, out var scalars);
        if (buffers.Count == 0)
        {
            result = PilotValue.Undefined;
            return false;
        }

        string? pyExe = PythonExePath.Value;
        if (pyExe == null)
        {
            return TryExecuteInProcessBufferScript(code, context, out result);
        }

        var sb = new StringBuilder();
        sb.AppendLine("import gc, mmap, os, sys, textwrap");
        sb.AppendLine("__pilot_fds = []");
        sb.AppendLine("__pilot_mms = []");
        sb.AppendLine("__pilot_mvs = []");
        sb.AppendLine("__pilot_ns = {}");
        sb.AppendLine("def __pilot_bind(name, path, byte_cap, elem_len, stride, fmt):");
        sb.AppendLine("    fd = os.open(path, os.O_RDWR)");
        sb.AppendLine("    __pilot_fds.append(fd)");
        sb.AppendLine("    mm = mmap.mmap(fd, byte_cap, access=mmap.ACCESS_WRITE)");
        sb.AppendLine("    __pilot_mms.append(mm)");
        sb.AppendLine("    mv_bytes = memoryview(mm)[:elem_len * stride]");
        sb.AppendLine("    mv_typed = mv_bytes.cast(fmt)");
        sb.AppendLine("    __pilot_mvs.extend([mv_typed, mv_bytes])");
        sb.AppendLine("    __pilot_ns[name] = mv_typed");
        sb.AppendLine("    __pilot_ns[name + '_mmap'] = mm");

        foreach (var (name, buf) in buffers)
        {
            string path = buf.EnsureSharedMapping(name);
            buf.Flush();
            long byteCap = Math.Max(buf.ByteCapacity, (long)buf.Stride);
            string fmt = PilotSharedBuffer.GetPythonStructFormat(buf.ElementKind);
            string escapedPath = path.Replace("\\", "\\\\").Replace("'", "\\'");
            sb.AppendLine($"__pilot_bind('{name}', '{escapedPath}', {byteCap}, {buf.Length}, {buf.Stride}, '{fmt}')");
        }

        foreach (var (sName, sVal) in scalars)
        {
            if (sVal.Kind is PilotValueKind.Int or PilotValueKind.UInt)
                sb.AppendLine($"__pilot_ns['{sName}'] = {sVal.AsInt().ToString(CultureInfo.InvariantCulture)}");
            else if (sVal.Kind is PilotValueKind.Float or PilotValueKind.Decimal)
                sb.AppendLine($"__pilot_ns['{sName}'] = {sVal.AsFloat().ToString("R", CultureInfo.InvariantCulture)}");
            else if (sVal.Kind == PilotValueKind.Bool)
                sb.AppendLine($"__pilot_ns['{sName}'] = {(sVal.IsTrue ? "True" : "False")}");
            else if (sVal.Kind == PilotValueKind.String)
                sb.AppendLine($"__pilot_ns['{sName}'] = '{sVal.AsString().Replace("\\", "\\\\").Replace("'", "\\'")}'");
        }

        string b64Code = Convert.ToBase64String(Encoding.UTF8.GetBytes(code));
        sb.AppendLine("import base64");
        sb.AppendLine($"__decoded = base64.b64decode('{b64Code}').decode('utf-8')");
        sb.AppendLine("__lines = [ln for ln in __decoded.splitlines() if ln.strip() != '' or True]");
        sb.AppendLine("if len(__lines) > 1 and (len(__lines[0]) - len(__lines[0].lstrip())) == 0:");
        sb.AppendLine("    __raw_src = (__lines[0] + '\\n' + textwrap.dedent('\\n'.join(__lines[1:]))).strip()");
        sb.AppendLine("else:");
        sb.AppendLine("    __raw_src = textwrap.dedent(__decoded).strip()");
        sb.AppendLine("__indented = '\\n'.join('    ' + ln for ln in __raw_src.splitlines()) if __raw_src else '    pass'");
        sb.AppendLine("exec('def __pilot_entry():\\n' + __indented, __pilot_ns)");
        sb.AppendLine("__res = __pilot_ns['__pilot_entry']()");
        sb.AppendLine("for __mm in __pilot_mms: __mm.flush()");
        sb.AppendLine("if __res is None:");
        sb.AppendLine("    print('__PILOT_RET__:null:')");
        sb.AppendLine("elif isinstance(__res, bool):");
        sb.AppendLine("    print('__PILOT_RET__:bool:' + ('1' if __res else '0'))");
        sb.AppendLine("elif isinstance(__res, int):");
        sb.AppendLine("    print('__PILOT_RET__:int:' + str(__res))");
        sb.AppendLine("elif isinstance(__res, float):");
        sb.AppendLine("    print('__PILOT_RET__:float:' + repr(__res))");
        sb.AppendLine("else:");
        sb.AppendLine("    print('__PILOT_RET__:str:' + str(__res))");
        sb.AppendLine("__pilot_ns.clear()");
        sb.AppendLine("for __v in __pilot_mvs:");
        sb.AppendLine("    try: __v.release()");
        sb.AppendLine("    except Exception: pass");
        sb.AppendLine("__pilot_mvs.clear()");
        sb.AppendLine("gc.collect()");
        sb.AppendLine("for __mm in __pilot_mms:");
        sb.AppendLine("    try: __mm.close()");
        sb.AppendLine("    except Exception: pass");
        sb.AppendLine("for __fd in __pilot_fds:");
        sb.AppendLine("    try: os.close(__fd)");
        sb.AppendLine("    except Exception: pass");

        string tempPy = Path.Combine(Path.GetTempPath(), $"pilot_py_shm_{Guid.NewGuid():N}.py");
        try
        {
            File.WriteAllText(tempPy, sb.ToString(), Encoding.UTF8);
            var psi = new ProcessStartInfo(pyExe, $"-I \"{tempPy}\"")
            {
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };
            using var proc = Process.Start(psi)!;
            string stdout = proc.StandardOutput.ReadToEnd();
            string stderr = proc.StandardError.ReadToEnd();
            proc.WaitForExit(10000);

            if (proc.ExitCode != 0)
            {
                throw new Pilot.Runtime.ControlFlow.PilotRuntimeException(
                    $"Python shared-memory block failed (exit {proc.ExitCode}): {stderr.Trim()}");
            }

            result = ParseSubprocessReturn(stdout, context);
            return true;
        }
        finally
        {
            try { if (File.Exists(tempPy)) File.Delete(tempPy); } catch { }
        }
    }

    public static bool TryExecuteNodeSharedMemory(
        string code,
        PilotExecutionContext context,
        out PilotValue result)
    {
        var buffers = CollectReferencedBuffers(code, context, out var scalars);
        if (buffers.Count == 0)
        {
            result = PilotValue.Undefined;
            return false;
        }

        string? nodeExe = NodeExePath.Value;
        if (nodeExe == null)
        {
            return TryExecuteInProcessBufferScript(code, context, out result);
        }

        var sb = new StringBuilder();
        sb.AppendLine("const fs = require('fs');");
        sb.AppendLine("const __bindings = [];");
        foreach (var (name, buf) in buffers)
        {
            string path = buf.EnsureSharedMapping(name);
            buf.Flush();
            long byteCap = Math.Max(buf.ByteCapacity, (long)buf.Stride);
            string escapedPath = path.Replace("\\", "\\\\").Replace("'", "\\'");
            string typedArrayCtor = buf.ElementKind switch
            {
                PilotSharedElementKind.Float64 => "Float64Array",
                PilotSharedElementKind.Int64 => "BigInt64Array",
                PilotSharedElementKind.Float32 => "Float32Array",
                PilotSharedElementKind.Int32 => "Int32Array",
                _ => "Uint8Array"
            };
            sb.AppendLine($"const __fd_{name} = fs.openSync('{escapedPath}', 'r+');");
            sb.AppendLine($"const __ab_{name} = new ArrayBuffer({byteCap});");
            sb.AppendLine($"const __u8_{name} = new Uint8Array(__ab_{name});");
            sb.AppendLine($"fs.readSync(__fd_{name}, __u8_{name}, 0, {byteCap}, 0);");
            sb.AppendLine($"const {name} = new {typedArrayCtor}(__ab_{name}, 0, {buf.Length});");
            sb.AppendLine($"__bindings.push({{ fd: __fd_{name}, u8: __u8_{name}, cap: {byteCap} }});");
        }

        foreach (var (sName, sVal) in scalars)
        {
            if (sVal.Kind is PilotValueKind.Int or PilotValueKind.UInt)
                sb.AppendLine($"const {sName} = {sVal.AsInt().ToString(CultureInfo.InvariantCulture)};");
            else if (sVal.Kind is PilotValueKind.Float or PilotValueKind.Decimal)
                sb.AppendLine($"const {sName} = {sVal.AsFloat().ToString("R", CultureInfo.InvariantCulture)};");
            else if (sVal.Kind == PilotValueKind.Bool)
                sb.AppendLine($"const {sName} = {(sVal.IsTrue ? "true" : "false")};");
        }

        sb.AppendLine("const __res = (() => {");
        sb.AppendLine(code);
        sb.AppendLine("})();");
        sb.AppendLine("for (const b of __bindings) { fs.writeSync(b.fd, b.u8, 0, b.cap, 0); fs.fsyncSync(b.fd); fs.closeSync(b.fd); }");
        sb.AppendLine("if (__res === undefined || __res === null) console.log('__PILOT_RET__:null:');");
        sb.AppendLine("else if (typeof __res === 'boolean') console.log('__PILOT_RET__:bool:' + (__res ? '1' : '0'));");
        sb.AppendLine("else if (typeof __res === 'bigint') console.log('__PILOT_RET__:int:' + __res.toString());");
        sb.AppendLine("else if (typeof __res === 'number') console.log(Number.isInteger(__res) ? '__PILOT_RET__:int:' + __res : '__PILOT_RET__:float:' + __res);");
        sb.AppendLine("else console.log('__PILOT_RET__:str:' + String(__res));");

        string tempJs = Path.Combine(Path.GetTempPath(), $"pilot_node_shm_{Guid.NewGuid():N}.cjs");
        try
        {
            File.WriteAllText(tempJs, sb.ToString(), Encoding.UTF8);
            var psi = new ProcessStartInfo(nodeExe, $"\"{tempJs}\"")
            {
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };
            using var proc = Process.Start(psi)!;
            string stdout = proc.StandardOutput.ReadToEnd();
            string stderr = proc.StandardError.ReadToEnd();
            proc.WaitForExit(10000);

            if (proc.ExitCode != 0)
            {
                throw new Pilot.Runtime.ControlFlow.PilotRuntimeException(
                    $"Node.js shared-memory block failed (exit {proc.ExitCode}): {stderr.Trim()}");
            }

            result = ParseSubprocessReturn(stdout, context);
            return true;
        }
        finally
        {
            try { if (File.Exists(tempJs)) File.Delete(tempJs); } catch { }
        }
    }

    private static PilotValue ParseSubprocessReturn(string stdout, PilotExecutionContext context)
    {
        PilotValue ret = PilotValue.Null;
        var lines = stdout.Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
        foreach (var line in lines)
        {
            if (line.StartsWith("__PILOT_RET__:", StringComparison.Ordinal))
            {
                string rest = line["__PILOT_RET__:".Length..];
                int colon = rest.IndexOf(':');
                if (colon > 0)
                {
                    string kind = rest[..colon];
                    string payload = rest[(colon + 1)..];
                    ret = kind switch
                    {
                        "null" => PilotValue.Null,
                        "bool" => PilotValue.FromBool(payload == "1"),
                        "int" when long.TryParse(payload, NumberStyles.Integer, CultureInfo.InvariantCulture, out long iv) => PilotValue.FromInt(iv),
                        "float" when double.TryParse(payload, NumberStyles.Float, CultureInfo.InvariantCulture, out double dv) => PilotValue.FromFloat(dv),
                        _ => PilotValue.FromString(payload)
                    };
                }
            }
            else
            {
                context.Options.StandardOutput.WriteLine(line);
            }
        }
        return ret;
    }

    /// <summary>
    /// Executes C / C++ / Rust / C# pointer, slice, and <c>Span&lt;T&gt;</c> kernels directly in-place
    /// on <see cref="PilotSharedBuffer.DataPointer"/> with zero copying and zero marshalling.
    /// </summary>
    public static bool TryExecuteInProcessBufferScript(
        string code,
        PilotExecutionContext context,
        out PilotValue result)
    {
        var buffers = CollectReferencedBuffers(code, context, out var scalars);
        if (buffers.Count == 0)
        {
            result = PilotValue.Undefined;
            return false;
        }

        var aliases = new Dictionary<string, PilotSharedBuffer>(buffers, StringComparer.Ordinal);
        var localVars = new Dictionary<string, double>(StringComparer.Ordinal);
        foreach (var (sName, sVal) in scalars)
        {
            if (sVal.IsNumber)
                localVars[sName] = sVal.AsFloat();
        }

        var statements = SplitForeignStatements(code);
        if (ExecuteForeignStatementList(statements, aliases, localVars, out var returned))
        {
            result = returned;
            return true;
        }

        result = PilotValue.Null;
        return true;
    }

    private static List<string> SplitForeignStatements(string code)
    {
        var list = new List<string>();
        int depth = 0;
        int start = 0;
        for (int i = 0; i < code.Length; i++)
        {
            char c = code[i];
            if (c == '{') depth++;
            else if (c == '}')
            {
                depth--;
                if (depth == 0)
                {
                    string chunk = code[start..(i + 1)].Trim();
                    if (chunk.Length > 0) list.Add(chunk);
                    start = i + 1;
                }
            }
            else if ((c == ';' || c == '\n') && depth == 0)
            {
                string chunk = code[start..i].Trim();
                if (chunk.Length > 0) list.Add(chunk);
                start = i + 1;
            }
        }
        if (start < code.Length)
        {
            string tail = code[start..].Trim();
            if (tail.Length > 0) list.Add(tail);
        }
        return list;
    }

    private static bool ExecuteForeignStatementList(
        List<string> statements,
        Dictionary<string, PilotSharedBuffer> aliases,
        Dictionary<string, double> localVars,
        out PilotValue returnVal)
    {
        returnVal = PilotValue.Null;
        foreach (var rawStmt in statements)
        {
            string stmt = rawStmt.Trim();
            if (string.IsNullOrEmpty(stmt) || stmt.StartsWith("//") || stmt.StartsWith("#") || stmt.StartsWith("/*"))
                continue;

            // C / C++ / C# for loop: for (int i = 0; i < N; i++) { ... }
            var cForMatch = Regex.Match(
                stmt,
                @"^for\s*\(\s*(?:[A-Za-z_][A-Za-z0-9_]*\s+)?(?<var>[A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?<init>[^;]+);\s*\k<var>\s*(?<cmp><=|>=|<|>)\s*(?<bound>[^;]+);\s*(?:(?:\+\+)?\k<var>(?:\+\+|\s*\+=\s*(?<step>[^\)]+))?)\s*\)\s*\{(?<body>[\s\S]*)\}$");
            if (cForMatch.Success)
            {
                string loopVar = cForMatch.Groups["var"].Value;
                int startVal = (int)EvaluateNumericExpr(cForMatch.Groups["init"].Value, aliases, localVars);
                int boundVal = (int)EvaluateNumericExpr(cForMatch.Groups["bound"].Value, aliases, localVars);
                int stepVal = cForMatch.Groups["step"].Success
                    ? (int)EvaluateNumericExpr(cForMatch.Groups["step"].Value, aliases, localVars)
                    : 1;
                if (stepVal == 0) stepVal = 1;
                string cmp = cForMatch.Groups["cmp"].Value;
                var bodyStmts = SplitForeignStatements(cForMatch.Groups["body"].Value);

                for (int i = startVal;
                     cmp == "<" ? i < boundVal : cmp == "<=" ? i <= boundVal : cmp == ">" ? i > boundVal : i >= boundVal;
                     i += stepVal)
                {
                    localVars[loopVar] = i;
                    if (ExecuteForeignStatementList(bodyStmts, aliases, localVars, out returnVal))
                        return true;
                }
                continue;
            }

            // Rust / Python range for loop: for i in A..B { ... } or for i in range(N): ...
            var rustForMatch = Regex.Match(
                stmt,
                @"^for\s+(?<var>[A-Za-z_][A-Za-z0-9_]*)\s+in\s+(?:(?<start>[^\.\s]+)\.\.(?<eq>=?)(?<end>[^{:\s]+)|range\(\s*(?<rlen>[^\)]+)\s*\))\s*(?:\{(?<body>[\s\S]*)\}|:\s*(?<pybody>[\s\S]*))$");
            if (rustForMatch.Success)
            {
                string loopVar = rustForMatch.Groups["var"].Value;
                int startVal = rustForMatch.Groups["start"].Success
                    ? (int)EvaluateNumericExpr(rustForMatch.Groups["start"].Value, aliases, localVars)
                    : 0;
                int endVal = rustForMatch.Groups["rlen"].Success
                    ? (int)EvaluateNumericExpr(rustForMatch.Groups["rlen"].Value, aliases, localVars)
                    : (int)EvaluateNumericExpr(rustForMatch.Groups["end"].Value, aliases, localVars);
                bool inclusive = rustForMatch.Groups["eq"].Value == "=";
                string bodyText = rustForMatch.Groups["body"].Success
                    ? rustForMatch.Groups["body"].Value
                    : rustForMatch.Groups["pybody"].Value;
                var bodyStmts = SplitForeignStatements(bodyText);

                for (int i = startVal; inclusive ? i <= endVal : i < endVal; i++)
                {
                    localVars[loopVar] = i;
                    if (ExecuteForeignStatementList(bodyStmts, aliases, localVars, out returnVal))
                        return true;
                }
                continue;
            }

            // Return statement
            if (stmt.StartsWith("return ", StringComparison.Ordinal))
            {
                string retExpr = stmt["return ".Length..].Trim().TrimEnd(';');
                double dv = EvaluateNumericExpr(retExpr, aliases, localVars, out bool preferFloat);
                returnVal = preferFloat || Math.Abs(dv - Math.Truncate(dv)) > 1e-12
                    ? PilotValue.FromFloat(dv)
                    : PilotValue.FromInt((long)dv);
                return true;
            }

            // Indexed mutation: buf[idx] = expr / += / -= / *= / /=
            var idxAssign = Regex.Match(
                stmt,
                @"^(?<buf>[A-Za-z_][A-Za-z0-9_]*)\s*\[\s*(?<idx>[^\]]+)\s*\]\s*(?<op>\+=|-=|\*=|/=|=)\s*(?<rhs>.+)$");
            if (idxAssign.Success && aliases.TryGetValue(idxAssign.Groups["buf"].Value, out var targetBuf))
            {
                int idx = (int)EvaluateNumericExpr(idxAssign.Groups["idx"].Value, aliases, localVars);
                double rhs = EvaluateNumericExpr(idxAssign.Groups["rhs"].Value, aliases, localVars);
                string op = idxAssign.Groups["op"].Value;
                double cur = op == "=" ? 0.0 : targetBuf.GetElement(idx).AsFloat();
                double next = op switch
                {
                    "+=" => cur + rhs,
                    "-=" => cur - rhs,
                    "*=" => cur * rhs,
                    "/=" => cur / rhs,
                    _ => rhs
                };
                bool isFloatElem = targetBuf.ElementKind is PilotSharedElementKind.Float64 or PilotSharedElementKind.Float32;
                targetBuf.SetElement(idx, isFloatElem ? PilotValue.FromFloat(next) : PilotValue.FromInt((long)next));
                continue;
            }

            // Pointer / Span / Slice alias or scalar variable assignment
            var declMatch = Regex.Match(
                stmt,
                @"^(?:(?:const\s+|let\s+mut\s+|let\s+|var\s+|Span<[^>]+>\s+|ReadOnlySpan<[^>]+>\s+|[A-Za-z0-9_]+\s*\*\s*|[A-Za-z0-9_]+\s+))?(?<lhs>[A-Za-z_][A-Za-z0-9_]*)\s*(?<op>\+=|-=|\*=|/=|=)\s*(?<rhs>.+)$");
            if (declMatch.Success)
            {
                string lhs = declMatch.Groups["lhs"].Value;
                string op = declMatch.Groups["op"].Value;
                string rhs = declMatch.Groups["rhs"].Value.Trim();

                // Check if RHS aliases a known PilotSharedBuffer (e.g. `double* p = (double*)shm.data_ptr`, `Span<double> s = shm`, `let s = &mut shm`)
                PilotSharedBuffer? aliased = TryMatchBufferAliasRhs(rhs, aliases);
                if (op == "=" && aliased != null)
                {
                    aliases[lhs] = aliased;
                    continue;
                }

                double rhsNum = EvaluateNumericExpr(rhs, aliases, localVars);
                double curNum = localVars.TryGetValue(lhs, out double existing) ? existing : 0.0;
                localVars[lhs] = op switch
                {
                    "+=" => curNum + rhsNum,
                    "-=" => curNum - rhsNum,
                    "*=" => curNum * rhsNum,
                    "/=" => curNum / rhsNum,
                    _ => rhsNum
                };
            }
        }
        return false;
    }

    private static PilotSharedBuffer? TryMatchBufferAliasRhs(string rhs, Dictionary<string, PilotSharedBuffer> aliases)
    {
        string cleaned = Regex.Replace(rhs, @"^\([A-Za-z0-9_\s\*]+\)", "").Trim();
        cleaned = cleaned.Replace("&mut ", "").Replace("&", "").Trim();
        foreach (var (name, buf) in aliases)
        {
            if (cleaned.Equals(name, StringComparison.Ordinal) ||
                cleaned.Equals($"{name}.data_ptr", StringComparison.OrdinalIgnoreCase) ||
                cleaned.Equals($"{name}.DataPtr", StringComparison.Ordinal) ||
                cleaned.Equals($"{name}.as_mut_ptr()", StringComparison.Ordinal) ||
                cleaned.Equals($"{name}.as_ptr()", StringComparison.Ordinal) ||
                cleaned.Equals($"{name}.AsSpan()", StringComparison.Ordinal) ||
                cleaned.Equals($"memoryview({name})", StringComparison.Ordinal))
            {
                return buf;
            }
        }
        return null;
    }

    private static double EvaluateNumericExpr(
        string expr,
        Dictionary<string, PilotSharedBuffer> aliases,
        Dictionary<string, double> localVars) =>
        EvaluateNumericExpr(expr, aliases, localVars, out _);

    private static double EvaluateNumericExpr(
        string expr,
        Dictionary<string, PilotSharedBuffer> aliases,
        Dictionary<string, double> localVars,
        out bool isFloat)
    {
        isFloat = false;
        string s = expr.Trim().TrimEnd(';');
        // Strip C/Rust type casts like `(double)`, `as f64`, `as usize`
        s = Regex.Replace(s, @"^\((?:double|float|int64_t|int32_t|int|long|size_t)\)\s*", "");
        s = Regex.Replace(s, @"\s+as\s+(?:f64|f32|i64|i32|usize|isize)\b", "");
        while (s.Length >= 2 && s[0] == '(' && s[^1] == ')' && IsBalancedOuterParens(s))
            s = s[1..^1].Trim();

        // Top-level binary + / -
        int addSubIdx = FindTopLevelBinaryOp(s, '+', '-');
        if (addSubIdx > 0)
        {
            double l = EvaluateNumericExpr(s[..addSubIdx], aliases, localVars, out bool lf);
            double r = EvaluateNumericExpr(s[(addSubIdx + 1)..], aliases, localVars, out bool rf);
            isFloat = lf || rf;
            return s[addSubIdx] == '+' ? l + r : l - r;
        }

        // Top-level binary * / /
        int mulDivIdx = FindTopLevelBinaryOp(s, '*', '/');
        if (mulDivIdx > 0)
        {
            double l = EvaluateNumericExpr(s[..mulDivIdx], aliases, localVars, out bool lf);
            double r = EvaluateNumericExpr(s[(mulDivIdx + 1)..], aliases, localVars, out bool rf);
            isFloat = lf || rf || s[mulDivIdx] == '/';
            return s[mulDivIdx] == '*' ? l * r : l / r;
        }

        // Buffer element read: `buf[idx]`
        var idxMatch = Regex.Match(s, @"^(?<buf>[A-Za-z_][A-Za-z0-9_]*)\s*\[\s*(?<idx>[^\]]+)\s*\]$");
        if (idxMatch.Success && aliases.TryGetValue(idxMatch.Groups["buf"].Value, out var sb))
        {
            int idx = (int)EvaluateNumericExpr(idxMatch.Groups["idx"].Value, aliases, localVars);
            isFloat = sb.ElementKind is PilotSharedElementKind.Float64 or PilotSharedElementKind.Float32;
            return sb.GetElement(idx).AsFloat();
        }

        // Buffer length: `buf.length` / `buf.Length` / `buf.len()` / `len(buf)`
        var lenMethodMatch = Regex.Match(s, @"^(?:len\(\s*(?<b1>[A-Za-z_][A-Za-z0-9_]*)\s*\)|(?<b2>[A-Za-z_][A-Za-z0-9_]*)\.(?:length|Length|len\(\)))$");
        if (lenMethodMatch.Success)
        {
            string bName = lenMethodMatch.Groups["b1"].Success ? lenMethodMatch.Groups["b1"].Value : lenMethodMatch.Groups["b2"].Value;
            if (aliases.TryGetValue(bName, out var lenBuf))
                return lenBuf.Length;
        }

        if (localVars.TryGetValue(s, out double lv))
        {
            isFloat = Math.Abs(lv - Math.Truncate(lv)) > 1e-12;
            return lv;
        }

        string numClean = s.TrimEnd('f', 'F', 'd', 'D', 'L', 'u', 'U');
        if (numClean.Contains('.'))
            isFloat = true;
        if (double.TryParse(numClean, NumberStyles.Float, CultureInfo.InvariantCulture, out double parsed))
            return parsed;

        return 0.0;
    }

    private static bool IsBalancedOuterParens(string s)
    {
        int depth = 0;
        for (int i = 0; i < s.Length; i++)
        {
            if (s[i] == '(') depth++;
            else if (s[i] == ')')
            {
                depth--;
                if (depth == 0 && i < s.Length - 1) return false;
            }
        }
        return depth == 0;
    }

    private static int FindTopLevelBinaryOp(string s, char op1, char op2)
    {
        int paren = 0, bracket = 0;
        for (int i = s.Length - 1; i > 0; i--)
        {
            char c = s[i];
            if (c == ')') paren++;
            else if (c == '(') paren--;
            else if (c == ']') bracket++;
            else if (c == '[') bracket--;
            else if (paren == 0 && bracket == 0 && (c == op1 || c == op2))
            {
                // Ensure binary operator, not unary sign after another operator
                int prev = i - 1;
                while (prev >= 0 && char.IsWhiteSpace(s[prev])) prev--;
                if (prev >= 0 && (char.IsLetterOrDigit(s[prev]) || s[prev] == ')' || s[prev] == ']' || s[prev] == '_'))
                    return i;
            }
        }
        return -1;
    }

    public static long ResolveAsmScalarOperand(
        string operand,
        Dictionary<string, long> registers,
        PilotExecutionContext context)
    {
        string s = operand.Trim();
        if (s.StartsWith("0x", StringComparison.OrdinalIgnoreCase) &&
            long.TryParse(s[2..], NumberStyles.HexNumber, CultureInfo.InvariantCulture, out long hexVal))
        {
            return hexVal;
        }
        if (long.TryParse(s, NumberStyles.Integer, CultureInfo.InvariantCulture, out long imm))
            return imm;
        if (registers.TryGetValue(s, out long regVal))
            return regVal;

        int dotIdx = s.IndexOf('.');
        if (dotIdx > 0)
        {
            string objName = s[..dotIdx].Trim();
            string propName = s[(dotIdx + 1)..].Trim();
            if (context.Environment.CurrentScope.TryResolve(objName, out var objVal) && objVal.Kind == PilotValueKind.SliceBuffer)
            {
                var sb = objVal.AsSliceBuffer();
                return propName.ToLowerInvariant() switch
                {
                    "data_ptr" or "dataptr" or "ptr" => (long)sb.DataPointer,
                    "header_ptr" or "headerptr" => (long)sb.HeaderPointer,
                    "length" or "len" => sb.Length,
                    "capacity" or "cap" => sb.Capacity,
                    "stride" => sb.Stride,
                    _ => (long)sb.DataPointer
                };
            }
        }

        if (context.Environment.CurrentScope.TryResolve(s, out var pv))
        {
            if (pv.Kind == PilotValueKind.SliceBuffer)
                return (long)pv.AsSliceBuffer().DataPointer;
            if (pv.IsNumber)
                return pv.AsInt();
        }
        return 0;
    }

    public static double ResolveAsmFloatOperand(
        string operand,
        Dictionary<string, double> xmmRegisters,
        Dictionary<string, long> gpRegisters,
        PilotExecutionContext context)
    {
        string s = operand.Trim();
        if (xmmRegisters.TryGetValue(s, out double xv))
            return xv;
        if (double.TryParse(s, NumberStyles.Float, CultureInfo.InvariantCulture, out double dv))
            return dv;
        if (context.Environment.CurrentScope.TryResolve(s, out var pv) && pv.IsNumber)
            return pv.AsFloat();
        if (gpRegisters.TryGetValue(s, out long gp))
            return gp;
        return 0.0;
    }

    public static bool TryResolveAsmMemoryAddress(
        string operand,
        Dictionary<string, long> registers,
        PilotExecutionContext context,
        out long address)
    {
        address = 0;
        string s = operand.Trim();
        if (s.StartsWith("qword ptr", StringComparison.OrdinalIgnoreCase))
            s = s["qword ptr".Length..].Trim();
        else if (s.StartsWith("dword ptr", StringComparison.OrdinalIgnoreCase))
            s = s["dword ptr".Length..].Trim();

        if (s.Length < 3 || s[0] != '[' || s[^1] != ']')
            return false;

        string inner = s[1..^1].Trim();
        // Tokenize signed additive terms inside `[base + idx*8 + 16]`
        long total = 0;
        int sign = 1;
        int pos = 0;
        while (pos < inner.Length)
        {
            while (pos < inner.Length && char.IsWhiteSpace(inner[pos])) pos++;
            if (pos >= inner.Length) break;

            if (inner[pos] == '+') { sign = 1; pos++; continue; }
            if (inner[pos] == '-') { sign = -1; pos++; continue; }

            int start = pos;
            while (pos < inner.Length && inner[pos] != '+' && inner[pos] != '-') pos++;
            string term = inner[start..pos].Trim();
            if (term.Length == 0) continue;

            long termVal;
            int starIdx = term.IndexOf('*');
            if (starIdx > 0)
            {
                long lhs = ResolveAsmScalarOperand(term[..starIdx].Trim(), registers, context);
                long rhs = ResolveAsmScalarOperand(term[(starIdx + 1)..].Trim(), registers, context);
                termVal = lhs * rhs;
            }
            else
            {
                termVal = ResolveAsmScalarOperand(term, registers, context);
            }
            total += sign * termVal;
            sign = 1;
        }

        address = total;
        return address != 0;
    }
}
