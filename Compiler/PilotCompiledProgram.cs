using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text;
using Antlr4.Runtime;
using Pilot.Diagnostics;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Pilot.Scripting;

namespace Pilot.Compiler;

/// <summary>
/// Executable compiled Pilot program.
/// Encapsulates preprocessed source, parsed and validated AST, and binary serialization (.pilc).
/// Running this program incurs zero re-parsing or preprocessing overhead.
/// </summary>
public sealed class PilotCompiledProgram : IPilotProgram
{
    private const uint MagicHeader = 0x434C4950; // 'PILC' in little-endian
    private const byte CurrentFormatVersion = 1;

    public string Source { get; }
    public string? FilePath { get; }
    public DateTime CompiledAt { get; }
    public IReadOnlyList<string> DefinedSymbols { get; }
    public PilotParser.ProgramContext ProgramTree { get; }

    public PilotCompiledProgram(
        string source,
        PilotParser.ProgramContext programTree,
        string? filePath = null,
        DateTime? compiledAt = null,
        IEnumerable<string>? definedSymbols = null)
    {
        Source = source ?? string.Empty;
        ProgramTree = programTree ?? throw new ArgumentNullException(nameof(programTree));
        FilePath = filePath;
        CompiledAt = compiledAt ?? DateTime.UtcNow;
        DefinedSymbols = definedSymbols != null ? new List<string>(definedSymbols) : Array.Empty<string>();
    }

    public PilotExecutionResult Run(PilotExecutionOptions? options = null)
    {
        return ExecuteCore(options, null);
    }

    public PilotExecutionResult Run(PilotExecutionContext context)
    {
        var sw = Stopwatch.StartNew();
        var visitor = new PilotInterpreterVisitor(context);
        try
        {
            var value = visitor.VisitProgram(ProgramTree);
            var result = PilotExecutionResult.CreateSuccess(value, string.Empty, string.Empty, sw.Elapsed);
            result.Context = context;
            return result;
        }
        catch (PilotScriptExitException sex)
        {
            var result = PilotExecutionResult.CreateSuccess(PilotValue.FromInt(sex.ExitCode), string.Empty, string.Empty, sw.Elapsed);
            result.Context = context;
            return result;
        }
        catch (PilotRuntimeException pre)
        {
            return PilotExecutionResult.CreateFailure(pre.Message, string.Empty, string.Empty, sw.Elapsed);
        }
        catch (Exception ex)
        {
            return PilotExecutionResult.CreateFailure(ex.Message, string.Empty, string.Empty, sw.Elapsed);
        }
    }

    public PilotExecutionResult RunWithArgs(IReadOnlyList<string> scriptArgs, PilotExecutionOptions? options = null)
    {
        return ExecuteCore(options, scriptArgs);
    }

    private PilotExecutionResult ExecuteCore(PilotExecutionOptions? options, IReadOnlyList<string>? scriptArgs)
    {
        var sw = Stopwatch.StartNew();
        options ??= new PilotExecutionOptions();

        var stdoutCapture = new StringWriter();
        var stderrCapture = new StringWriter();

        var originalStdOut = options.StandardOutput;
        var originalStdErr = options.StandardError;

        var mergedStdOut = new MergedTextWriter(originalStdOut, stdoutCapture);
        var mergedStdErr = new MergedTextWriter(originalStdErr, stderrCapture);

        options.StandardOutput = mergedStdOut;
        options.StandardError = mergedStdErr;

        try
        {
            var context = new PilotExecutionContext(new PilotEngine(), options);

            // Register standard builtins into global scope
            foreach (var kvp in options.Builtins.GetAll())
            {
                context.Environment.GlobalScope.Declare(kvp.Key, PilotValue.FromFunction(kvp.Value));
            }

            // Ingest scripting globals: args, env, cwd, exit, read_text, write_text
            PilotScriptContext.Inject(context, scriptArgs, FilePath);

            var visitor = new PilotInterpreterVisitor(context);
            var resultValue = visitor.VisitProgram(ProgramTree);

            var execResult = PilotExecutionResult.CreateSuccess(
                resultValue,
                stdoutCapture.ToString(),
                stderrCapture.ToString(),
                sw.Elapsed);
            execResult.Context = context;
            return execResult;
        }
        catch (PilotScriptExitException sex)
        {
            return PilotExecutionResult.CreateSuccess(
                PilotValue.FromInt(sex.ExitCode),
                stdoutCapture.ToString(),
                stderrCapture.ToString(),
                sw.Elapsed);
        }
        catch (PilotRuntimeException rex)
        {
            return PilotExecutionResult.CreateFailure(
                rex.Message,
                stdoutCapture.ToString(),
                stderrCapture.ToString(),
                sw.Elapsed);
        }
        catch (Exception ex)
        {
            return PilotExecutionResult.CreateFailure(
                ex.Message,
                stdoutCapture.ToString(),
                stderrCapture.ToString(),
                sw.Elapsed);
        }
        finally
        {
            options.StandardOutput = originalStdOut;
            options.StandardError = originalStdErr;
        }
    }

    public void Save(Stream stream)
    {
        using var writer = new BinaryWriter(stream, Encoding.UTF8, leaveOpen: true);
        writer.Write(MagicHeader);
        writer.Write(CurrentFormatVersion);
        writer.Write(CompiledAt.Ticks);
        writer.Write(FilePath ?? string.Empty);

        writer.Write(DefinedSymbols.Count);
        foreach (var sym in DefinedSymbols)
        {
            writer.Write(sym);
        }

        byte[] sourceBytes = Encoding.UTF8.GetBytes(Source);
        writer.Write(sourceBytes.Length);
        writer.Write(sourceBytes);
    }

    public void Save(string filePath)
    {
        string? dir = Path.GetDirectoryName(filePath);
        if (!string.IsNullOrEmpty(dir) && !Directory.Exists(dir))
        {
            Directory.CreateDirectory(dir);
        }
        using var fs = File.Create(filePath);
        Save(fs);
    }

    public byte[] ToBytes()
    {
        using var ms = new MemoryStream();
        Save(ms);
        return ms.ToArray();
    }

    public static PilotCompiledProgram Load(Stream stream)
    {
        using var reader = new BinaryReader(stream, Encoding.UTF8, leaveOpen: true);
        uint magic = reader.ReadUInt32();
        if (magic != MagicHeader)
        {
            throw new InvalidDataException("Invalid Pilot compiled binary (.pilc): magic header mismatch.");
        }

        byte version = reader.ReadByte();
        if (version != CurrentFormatVersion)
        {
            throw new InvalidDataException($"Unsupported Pilot compiled binary version: {version}. Expected: {CurrentFormatVersion}.");
        }

        long ticks = reader.ReadInt64();
        var compiledAt = new DateTime(ticks, DateTimeKind.Utc);
        string filePath = reader.ReadString();

        int symCount = reader.ReadInt32();
        var symbols = new List<string>(symCount);
        for (int i = 0; i < symCount; i++)
        {
            symbols.Add(reader.ReadString());
        }

        int sourceLen = reader.ReadInt32();
        byte[] sourceBytes = reader.ReadBytes(sourceLen);
        string source = Encoding.UTF8.GetString(sourceBytes);

        // Parse tree once during loading
        var inputStream = new AntlrInputStream(source);
        var lexer = new PilotLexer(inputStream);
        var tokenStream = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokenStream);

        var errorTracker = new ErrorTracker();
        parser.RemoveErrorListeners();
        parser.AddErrorListener(errorTracker);

        var tree = parser.program();
        if (errorTracker.HasErrors)
        {
            throw new InvalidOperationException($"Corrupt compiled program AST: {string.Join("; ", errorTracker.Errors)}");
        }

        return new PilotCompiledProgram(source, tree, string.IsNullOrEmpty(filePath) ? null : filePath, compiledAt, symbols);
    }

    public static PilotCompiledProgram Load(string filePath)
    {
        using var fs = File.OpenRead(filePath);
        return Load(fs);
    }

    public static PilotCompiledProgram FromBytes(byte[] bytes)
    {
        using var ms = new MemoryStream(bytes);
        return Load(ms);
    }
}
