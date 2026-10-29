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
    public const uint MagicHeader = 0x434C4950; // 'PILC' in little-endian
    private const byte VersionLegacyV1 = 1;
    private const byte CurrentFormatVersion = 2;
    private const int CompilerVersionFooterMarker = 0x52455650; // 'PVER' in little-endian
    private static readonly PilotEngine SharedEngine = new();

    public string Source { get; }
    public string? FilePath { get; }
    public DateTime CompiledAt { get; }
    public string CompilerVersion { get; }
    public IReadOnlyList<string> DefinedSymbols { get; }
    public Pilot.Runtime.Bytecode.PilotBytecodeChunk? BytecodeChunk { get; set; }
    public IReadOnlyDictionary<string, string>? GeneratedSources { get; set; }

    /// <summary>
    /// True if this compiled binary was built by the exact same commit version of Pilot currently running.
    /// </summary>
    public bool IsVersionMatched => string.IsNullOrEmpty(CompilerVersion) || PilotVersion.IsExactMatch(CompilerVersion);

    /// <summary>
    /// Checks whether this compiled binary's embedded compiler version matches the currently running Pilot commit version.
    /// </summary>
    public bool CheckVersionCompatibility(out string? mismatchMessage) =>
        PilotVersion.CheckCompatibility(CompilerVersion, $"compiled binary '{FilePath ?? "<memory>"}'", out mismatchMessage);

    private PilotParser.ProgramContext? _programTree;
    public PilotParser.ProgramContext ProgramTree
    {
        get
        {
            if (_programTree == null && !string.IsNullOrEmpty(Source))
            {
                var inputStream = new AntlrInputStream(Source);
                var lexer = new PilotLexer(inputStream);
                var tokenStream = new CommonTokenStream(lexer);
                var parser = new PilotParser(tokenStream);
                var errorTracker = new ErrorTracker();
                parser.RemoveErrorListeners();
                parser.AddErrorListener(errorTracker);
                _programTree = parser.ParseProgramFast();
            }
            return _programTree!;
        }
    }

    public PilotCompiledProgram(
        string source,
        PilotParser.ProgramContext? programTree,
        string? filePath = null,
        DateTime? compiledAt = null,
        IEnumerable<string>? definedSymbols = null,
        Pilot.Runtime.Bytecode.PilotBytecodeChunk? bytecodeChunk = null,
        string? compilerVersion = null)
    {
        Source = source ?? string.Empty;
        _programTree = programTree;
        FilePath = filePath;
        CompiledAt = compiledAt ?? DateTime.UtcNow;
        CompilerVersion = string.IsNullOrWhiteSpace(compilerVersion) ? PilotVersion.FullVersion : compilerVersion;
        DefinedSymbols = definedSymbols != null ? new List<string>(definedSymbols) : Array.Empty<string>();
        BytecodeChunk = bytecodeChunk;
    }

    public PilotExecutionResult Run(PilotExecutionOptions? options = null)
    {
        return ExecuteCore(options, null);
    }

    public PilotExecutionResult Run(PilotExecutionContext context)
    {
        var sw = Stopwatch.StartNew();
        try
        {
            PilotValue value;
            if (BytecodeChunk != null)
            {
                value = Pilot.Runtime.Bytecode.PilVirtualMachine.Execute(BytecodeChunk, context);
            }
            else
            {
                var visitor = new PilotInterpreterVisitor(context);
                value = visitor.VisitProgram(ProgramTree);
            }

            if (context.PendingFlow == PilotFlowSignalKind.Return)
            {
                value = context.PendingReturnValue;
                context.PendingFlow = PilotFlowSignalKind.None;
            }
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
            var context = new PilotExecutionContext(SharedEngine, options);

            // Register standard builtins into global scope
            foreach (var kvp in options.Builtins.GetAll())
            {
                context.Environment.GlobalScope.Declare(kvp.Key, PilotValue.FromFunction(kvp.Value));
            }

            // Ingest scripting globals: args, env, cwd, exit, read_text, write_text
            PilotScriptContext.Inject(context, scriptArgs, FilePath);

            PilotValue resultValue;
            if (BytecodeChunk != null)
            {
                resultValue = Pilot.Runtime.Bytecode.PilVirtualMachine.Execute(BytecodeChunk, context);
            }
            else
            {
                var visitor = new PilotInterpreterVisitor(context);
                resultValue = visitor.VisitProgram(ProgramTree);
            }

            if (context.PendingFlow == PilotFlowSignalKind.Return)
            {
                resultValue = context.PendingReturnValue;
                context.PendingFlow = PilotFlowSignalKind.None;
            }

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

        // Serialize bytecode chunk
        bool hasBytecode = BytecodeChunk != null;
        writer.Write(hasBytecode);
        if (hasBytecode)
        {
            BytecodeChunk!.Write(writer);
        }

        // Write compiler commit version footer for strict version mismatch detection
        writer.Write(CompilerVersionFooterMarker);
        writer.Write(CompilerVersion);
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
        if (version != VersionLegacyV1 && version != CurrentFormatVersion)
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

        Pilot.Runtime.Bytecode.PilotBytecodeChunk? bytecodeChunk = null;
        if (version >= CurrentFormatVersion)
        {
            bool hasBytecode = reader.ReadBoolean();
            if (hasBytecode)
            {
                bytecodeChunk = Pilot.Runtime.Bytecode.PilotBytecodeChunk.Read(reader);
            }
        }

        string? compilerVersion = null;
        if (reader.BaseStream.CanSeek && reader.BaseStream.Position + 4 <= reader.BaseStream.Length)
        {
            long pos = reader.BaseStream.Position;
            int footerMarker = reader.ReadInt32();
            if (footerMarker == CompilerVersionFooterMarker)
            {
                compilerVersion = reader.ReadString();
            }
            else
            {
                reader.BaseStream.Position = pos;
            }
        }
        else if (!reader.BaseStream.CanSeek)
        {
            try
            {
                int footerMarker = reader.ReadInt32();
                if (footerMarker == CompilerVersionFooterMarker)
                {
                    compilerVersion = reader.ReadString();
                }
            }
            catch (EndOfStreamException)
            {
            }
        }

        PilotParser.ProgramContext? tree = null;
        bool hasBurstDecorator = !string.IsNullOrEmpty(source) &&
                                 (source.Contains("@burst", StringComparison.OrdinalIgnoreCase) ||
                                  source.Contains("@native_fast", StringComparison.OrdinalIgnoreCase));
        bool needsAstRehydration = bytecodeChunk != null && (hasBurstDecorator || NeedsAstFallbackRehydration(bytecodeChunk));
        if ((bytecodeChunk == null || needsAstRehydration) && !string.IsNullOrEmpty(source))
        {
            var inputStream = new AntlrInputStream(source);
            var lexer = new PilotLexer(inputStream);
            var tokenStream = new CommonTokenStream(lexer);
            var parser = new PilotParser(tokenStream);

            var errorTracker = new ErrorTracker();
            parser.RemoveErrorListeners();
            parser.AddErrorListener(errorTracker);

            tree = parser.ParseProgramFast();
            if (errorTracker.HasErrors)
            {
                throw new InvalidOperationException($"Corrupt compiled program AST: {string.Join("; ", errorTracker.Errors)}");
            }

            if (needsAstRehydration)
            {
                bytecodeChunk = PilotBytecodeCompiler.Compile(tree, source);
            }
        }

        return new PilotCompiledProgram(source, tree, string.IsNullOrEmpty(filePath) ? null : filePath, compiledAt, symbols, bytecodeChunk, compilerVersion);
    }

    private static bool NeedsAstFallbackRehydration(Pilot.Runtime.Bytecode.PilotBytecodeChunk chunk)
    {
        bool checkEvalAst = chunk.AstFallbackNodes.Count == 0;
        for (int i = 0; i < chunk.Instructions.Count; i++)
        {
            var op = chunk.Instructions[i].Opcode;
            if ((checkEvalAst && op == Pilot.Runtime.Bytecode.PilotOpcode.EvalAst) ||
                (chunk.FuncDefContext == null && op == Pilot.Runtime.Bytecode.PilotOpcode.Yield))
            {
                return true;
            }
        }
        for (int i = 0; i < chunk.NestedChunks.Count; i++)
        {
            if (NeedsAstFallbackRehydration(chunk.NestedChunks[i]))
                return true;
        }
        return false;
    }

    public static PilotCompiledProgram Load(string filePath)
    {
        using var fs = File.OpenRead(filePath);
        return Load(fs);
    }

    public static PilotCompiledProgram FromBytes(byte[] bytes, string? filePath = null)
    {
        using var ms = new MemoryStream(bytes);
        var loaded = Load(ms);
        if (!string.IsNullOrEmpty(filePath) && string.IsNullOrEmpty(loaded.FilePath))
        {
            return new PilotCompiledProgram(loaded.Source, loaded.ProgramTree, filePath, loaded.CompiledAt, loaded.DefinedSymbols, loaded.BytecodeChunk, loaded.CompilerVersion);
        }
        return loaded;
    }

    #region Binary & Script Patching / Hot-Reloading

    /// <summary>
    /// Applies this compiled program as a live hot-patch into an already-running <see cref="PilotExecutionContext"/>.
    /// Existing functions, method groups, classes, and typestates are updated in-place while preserving live object state.
    /// </summary>
    public Pilot.Runtime.HotReload.PilotPatchResult ApplyHotPatch(PilotExecutionContext context)
    {
        if (context == null) throw new ArgumentNullException(nameof(context));
        return context.HotReload.ApplyCompiledPatch(this, isCompiledBinary: true, sourceLabel: FilePath ?? "<compiled-patch>");
    }

    /// <summary>
    /// Produces a new patched <see cref="PilotCompiledProgram"/> (suitable for saving as a <c>.pilc</c> binary
    /// or executing directly) by merging a <c>.pilot</c> source patch into this compiled program.
    /// </summary>
    public PilotCompiledProgram ApplyPatchSource(string patchSource, string? patchFilePath = null)
    {
        var opts = new PilotCompilerOptions().WithSourceFile(patchFilePath ?? "<patch>");
        var comp = PilotCompiler.Default.Compile(patchSource, opts);
        if (!comp.Success || comp.Program is not PilotCompiledProgram compiledPatch)
        {
            string errors = string.Join("; ", comp.Diagnostics);
            throw new InvalidOperationException($"Failed to compile patch: {errors}");
        }
        return ApplyPatch(compiledPatch);
    }

    /// <summary>
    /// Loads a <c>.pilot</c> source file or <c>.pilc</c> compiled binary patch from disk and merges it
    /// into this compiled program to produce a new patched <see cref="PilotCompiledProgram"/>.
    /// </summary>
    public PilotCompiledProgram ApplyPatchFile(string patchFilePath)
    {
        if (!File.Exists(patchFilePath))
        {
            throw new FileNotFoundException($"Patch file not found: '{patchFilePath}'", patchFilePath);
        }

        byte[] raw = File.ReadAllBytes(patchFilePath);
        if (Pilot.Runtime.HotReload.PilotHotReloadSession.IsCompiledBinary(raw, patchFilePath))
        {
            var patchProg = FromBytes(raw, patchFilePath);
            return ApplyPatch(patchProg);
        }
        else
        {
            string patchSource = Encoding.UTF8.GetString(raw);
            if (patchSource.Length > 0 && patchSource[0] == '\uFEFF')
            {
                patchSource = patchSource[1..];
            }
            return ApplyPatchSource(patchSource, patchFilePath);
        }
    }

    /// <summary>
    /// Merges <paramref name="patch"/> into this <see cref="PilotCompiledProgram"/> and returns a newly
    /// compiled <see cref="PilotCompiledProgram"/> with all patched functions, method overloads, classes,
    /// and typestates woven in-place before top-level execution statements.
    /// </summary>
    public PilotCompiledProgram ApplyPatch(PilotCompiledProgram patch)
    {
        if (patch == null) throw new ArgumentNullException(nameof(patch));

        string mergedSource = MergeProgramSources(this, patch);
        string mergedPath = FilePath ?? patch.FilePath ?? "<patched.pilc>";
        var opts = new PilotCompilerOptions().WithSourceFile(mergedPath);
        var comp = PilotCompiler.Default.Compile(mergedSource, opts);
        if (!comp.Success || comp.Program is not PilotCompiledProgram mergedProg)
        {
            string errors = string.Join("; ", comp.Diagnostics);
            throw new InvalidOperationException($"Failed to compile merged binary patch: {errors}");
        }

        return mergedProg;
    }

    private static string MergeProgramSources(PilotCompiledProgram baseProg, PilotCompiledProgram patchProg)
    {
        var baseStmts = baseProg.ProgramTree.stmt() ?? Array.Empty<PilotParser.StmtContext>();
        var patchStmts = patchProg.ProgramTree.stmt() ?? Array.Empty<PilotParser.StmtContext>();

        var baseDeclNames = new HashSet<string>(StringComparer.Ordinal);
        foreach (var s in baseStmts)
        {
            string? dName = GetDeclarationName(s);
            if (!string.IsNullOrEmpty(dName))
            {
                baseDeclNames.Add(dName);
            }
        }

        // Group patch statements into:
        // 1. New helper declarations (functions/classes/extensions not in base) -> injected before first patched symbol
        // 2. Patched declarations (matching a function/class/extension in base) -> injected immediately after base declaration
        // 3. Trailing imperative patch statements -> appended at end
        var newHelperDecls = new List<string>();
        var patchesBySymbol = new Dictionary<string, List<string>>(StringComparer.Ordinal);
        var trailingStmts = new List<string>();

        foreach (var pStmt in patchStmts)
        {
            string text = ExtractStatementSource(patchProg.Source, pStmt);
            if (string.IsNullOrWhiteSpace(text)) continue;

            string? declName = GetDeclarationName(pStmt);
            if (!string.IsNullOrEmpty(declName))
            {
                if (baseDeclNames.Contains(declName))
                {
                    if (!patchesBySymbol.TryGetValue(declName, out var list))
                    {
                        list = new List<string>();
                        patchesBySymbol[declName] = list;
                    }
                    list.Add(text);
                }
                else
                {
                    newHelperDecls.Add(text);
                }
            }
            else
            {
                trailingStmts.Add(text);
            }
        }

        // Track last occurrence index of each declared symbol in baseStmts
        var lastBaseIndexForSymbol = new Dictionary<string, int>(StringComparer.Ordinal);
        for (int i = 0; i < baseStmts.Length; i++)
        {
            string? dName = GetDeclarationName(baseStmts[i]);
            if (!string.IsNullOrEmpty(dName))
            {
                lastBaseIndexForSymbol[dName] = i;
            }
        }

        var sb = new StringBuilder();
        bool helpersInjected = false;

        for (int i = 0; i < baseStmts.Length; i++)
        {
            string? dName = GetDeclarationName(baseStmts[i]);
            if (!helpersInjected && dName != null && patchesBySymbol.ContainsKey(dName) && newHelperDecls.Count > 0)
            {
                foreach (var helper in newHelperDecls)
                {
                    sb.AppendLine(helper.TrimEnd());
                    sb.AppendLine();
                }
                helpersInjected = true;
            }

            string baseStmtText = ExtractStatementSource(baseProg.Source, baseStmts[i]);
            sb.AppendLine(baseStmtText.TrimEnd());

            if (dName != null &&
                lastBaseIndexForSymbol.TryGetValue(dName, out int lastIdx) &&
                lastIdx == i &&
                patchesBySymbol.TryGetValue(dName, out var symbolPatches))
            {
                sb.AppendLine();
                foreach (var pText in symbolPatches)
                {
                    sb.AppendLine(pText.TrimEnd());
                }
                sb.AppendLine();
            }
        }

        if (!helpersInjected && newHelperDecls.Count > 0)
        {
            sb.AppendLine();
            foreach (var helper in newHelperDecls)
            {
                sb.AppendLine(helper.TrimEnd());
                sb.AppendLine();
            }
        }

        if (trailingStmts.Count > 0)
        {
            sb.AppendLine();
            foreach (var t in trailingStmts)
            {
                sb.AppendLine(t.TrimEnd());
            }
        }

        return sb.ToString();
    }

    private static string? GetDeclarationName(PilotParser.StmtContext stmt)
    {
        var comp = stmt.compound_stmt();
        if (comp != null)
        {
            if (comp.func_def() != null) return comp.func_def().identifier()?.GetText();
            if (comp.class_def() != null) return comp.class_def().identifier()?.GetText();
            if (comp.extension_def() != null) return comp.extension_def().identifier()?.GetText();
            if (comp.macro_def() != null) return comp.macro_def().identifier()?.GetText();
        }
        return null;
    }

    private static string ExtractStatementSource(string fullSource, PilotParser.StmtContext stmt)
    {
        if (stmt.Start == null || stmt.Stop == null) return stmt.GetText();
        int startIdx = stmt.Start.StartIndex;
        int stopIdx = stmt.Stop.StopIndex;
        if (startIdx < 0 || stopIdx < startIdx || stopIdx >= fullSource.Length)
        {
            return stmt.GetText();
        }

        // Include leading indentation on the first line of the statement
        while (startIdx > 0 && (fullSource[startIdx - 1] == ' ' || fullSource[startIdx - 1] == '\t'))
        {
            startIdx--;
        }

        return fullSource.Substring(startIdx, stopIdx - startIdx + 1);
    }

    #endregion
}
