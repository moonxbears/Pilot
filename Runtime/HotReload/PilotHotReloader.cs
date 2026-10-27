using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using Pilot.Compiler;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.HotReload;

/// <summary>
/// Captures the pre-patch state of a symbol in a <see cref="PilotScope"/> so it can be
/// rolled back transactionally or on demand via <c>HotReload.rollback()</c>.
/// </summary>
public sealed class PilotSymbolSnapshot
{
    public string Name { get; }
    public bool Existed { get; }
    public bool IsConst { get; }
    public string? TypeName { get; }
    public PilotValue RawValue { get; }
    public PilotFunction? FunctionSnapshot { get; }
    public PilotClassDefinition? ClassSnapshot { get; }
    public PilotMethodGroup? MethodGroupSnapshot { get; }

    private PilotSymbolSnapshot(
        string name,
        bool existed,
        bool isConst,
        string? typeName,
        PilotValue rawValue,
        PilotFunction? functionSnapshot,
        PilotClassDefinition? classSnapshot,
        PilotMethodGroup? methodGroupSnapshot)
    {
        Name = name;
        Existed = existed;
        IsConst = isConst;
        TypeName = typeName;
        RawValue = rawValue;
        FunctionSnapshot = functionSnapshot;
        ClassSnapshot = classSnapshot;
        MethodGroupSnapshot = methodGroupSnapshot;
    }

    public static PilotSymbolSnapshot CreateNonExistent(string name) =>
        new(name, existed: false, isConst: false, typeName: null, rawValue: PilotValue.Undefined,
            functionSnapshot: null, classSnapshot: null, methodGroupSnapshot: null);

    public static PilotSymbolSnapshot Capture(PilotScope scope, string name)
    {
        var bindings = scope.GetLocalBindings();
        if (!bindings.TryGetValue(name, out var binding))
        {
            return new PilotSymbolSnapshot(
                name,
                existed: false,
                isConst: false,
                typeName: null,
                rawValue: PilotValue.Undefined,
                functionSnapshot: null,
                classSnapshot: null,
                methodGroupSnapshot: null);
        }

        var val = binding.Value;
        PilotFunction? fnSnap = null;
        PilotClassDefinition? classSnap = null;
        PilotMethodGroup? mgSnap = null;

        if (val.Kind == PilotValueKind.Function)
        {
            var fn = val.AsFunction();
            if (fn.ClassDef != null)
            {
                classSnap = fn.ClassDef.CloneSnapshot();
            }
            else
            {
                fnSnap = fn.CloneSnapshot();
            }
        }
        else if (val.Kind == PilotValueKind.MethodGroup)
        {
            mgSnap = val.AsMethodGroup().CloneSnapshot();
        }

        return new PilotSymbolSnapshot(
            name,
            existed: true,
            isConst: binding.IsConst,
            typeName: binding.TypeName,
            rawValue: val.CloneValue(),
            functionSnapshot: fnSnap,
            classSnapshot: classSnap,
            methodGroupSnapshot: mgSnap);
    }

    /// <summary>
    /// Restores this symbol snapshot in-place inside <paramref name="scope"/>.
    /// Live objects, function references, and method groups are mutated in-place.
    /// </summary>
    public void Restore(PilotScope scope)
    {
        if (!Existed)
        {
            scope.RemoveLocal(Name);
            return;
        }

        var bindings = scope.GetLocalBindings();
        if (bindings.TryGetValue(Name, out var currentBinding))
        {
            var currentVal = currentBinding.Value;
            if (ClassSnapshot != null &&
                currentVal.Kind == PilotValueKind.Function &&
                currentVal.AsFunction().ClassDef != null)
            {
                currentVal.AsFunction().ClassDef!.RestoreFromSnapshot(ClassSnapshot);
                return;
            }

            if (FunctionSnapshot != null &&
                currentVal.Kind == PilotValueKind.Function &&
                currentVal.AsFunction().ClassDef == null)
            {
                currentVal.AsFunction().ApplyPatchFrom(FunctionSnapshot, isRollback: true);
                return;
            }

            if (MethodGroupSnapshot != null &&
                currentVal.Kind == PilotValueKind.MethodGroup)
            {
                currentVal.AsMethodGroup().RestoreFromSnapshot(MethodGroupSnapshot);
                return;
            }
        }

        // If the binding kind changed (e.g. single function was promoted to MethodGroup during patch),
        // restore the original underlying object state and rebind it.
        if (FunctionSnapshot != null && RawValue.Kind == PilotValueKind.Function)
        {
            RawValue.AsFunction().ApplyPatchFrom(FunctionSnapshot, isRollback: true);
        }
        else if (ClassSnapshot != null && RawValue.Kind == PilotValueKind.Function && RawValue.AsFunction().ClassDef != null)
        {
            RawValue.AsFunction().ClassDef!.RestoreFromSnapshot(ClassSnapshot);
        }
        else if (MethodGroupSnapshot != null && RawValue.Kind == PilotValueKind.MethodGroup)
        {
            RawValue.AsMethodGroup().RestoreFromSnapshot(MethodGroupSnapshot);
        }

        scope.ForceSet(Name, RawValue, IsConst, TypeName);
    }
}

/// <summary>
/// Metadata and rollback snapshot for a single applied hot-reload patch.
/// </summary>
public sealed class PilotPatchRecord
{
    public int Version { get; }
    public DateTimeOffset AppliedAt { get; }
    public string SourceLabel { get; }
    public bool IsCompiledBinary { get; }
    public IReadOnlyList<string> PatchedSymbols { get; }
    internal IReadOnlyDictionary<string, PilotSymbolSnapshot> Snapshots { get; }

    internal PilotPatchRecord(
        int version,
        DateTimeOffset appliedAt,
        string sourceLabel,
        bool isCompiledBinary,
        IReadOnlyList<string> patchedSymbols,
        IReadOnlyDictionary<string, PilotSymbolSnapshot> snapshots)
    {
        Version = version;
        AppliedAt = appliedAt;
        SourceLabel = sourceLabel;
        IsCompiledBinary = isCompiledBinary;
        PatchedSymbols = patchedSymbols;
        Snapshots = snapshots;
    }

    public PilotValue ToPilotMap()
    {
        var map = new Dictionary<PilotValue, PilotValue>
        {
            [PilotValue.FromString("version")] = PilotValue.FromInt(Version),
            [PilotValue.FromString("applied_at")] = PilotValue.FromString(AppliedAt.ToString("O")),
            [PilotValue.FromString("source")] = PilotValue.FromString(SourceLabel),
            [PilotValue.FromString("is_compiled")] = PilotValue.FromBool(IsCompiledBinary),
            [PilotValue.FromString("patched_symbols")] = PilotValue.FromList(
                PatchedSymbols.Select(PilotValue.FromString).ToList())
        };
        return PilotValue.FromMap(map);
    }
}

/// <summary>
/// Result of applying or rolling back a hot-reload patch.
/// </summary>
public sealed class PilotPatchResult
{
    public bool Success { get; init; }
    public bool Changed { get; init; } = true;
    public int Version { get; init; }
    public string SourceLabel { get; init; } = "";
    public IReadOnlyList<string> PatchedSymbols { get; init; } = Array.Empty<string>();
    public PilotValue ReturnValue { get; init; } = PilotValue.Undefined;
    public string? ErrorMessage { get; init; }

    public PilotValue ToPilotMap()
    {
        var map = new Dictionary<PilotValue, PilotValue>
        {
            [PilotValue.FromString("success")] = PilotValue.FromBool(Success),
            [PilotValue.FromString("changed")] = PilotValue.FromBool(Changed),
            [PilotValue.FromString("version")] = PilotValue.FromInt(Version),
            [PilotValue.FromString("source")] = PilotValue.FromString(SourceLabel),
            [PilotValue.FromString("patched_symbols")] = PilotValue.FromList(
                PatchedSymbols.Select(PilotValue.FromString).ToList()),
            [PilotValue.FromString("result")] = ReturnValue.IsUndefined ? PilotValue.Null : ReturnValue
        };
        if (!string.IsNullOrEmpty(ErrorMessage))
        {
            map[PilotValue.FromString("error")] = PilotValue.FromString(ErrorMessage);
        }
        return PilotValue.FromMap(map);
    }
}

/// <summary>
/// Manages live hot-reloading, binary/script patching, transactional rollback, and file watching
/// for a <see cref="PilotExecutionContext"/>.
/// </summary>
public sealed class PilotHotReloadSession : IDisposable
{
    private readonly PilotExecutionContext _context;
    private readonly Stack<PilotPatchRecord> _undoStack = new();
    private readonly List<PilotPatchRecord> _history = new();
    private readonly ConcurrentDictionary<string, string> _fileHashes = new(StringComparer.OrdinalIgnoreCase);
    private readonly ConcurrentDictionary<string, FileSystemWatcher> _watchers = new(StringComparer.OrdinalIgnoreCase);
    private readonly object _syncLock = new();

    public int Version { get; private set; } = 0;
    public IReadOnlyList<PilotPatchRecord> History => _history;

    public PilotHotReloadSession(PilotExecutionContext context)
    {
        _context = context ?? throw new ArgumentNullException(nameof(context));
    }

    /// <summary>
    /// Applies a patch from either a file path (<c>.pilot</c> or <c>.pilc</c>) or inline Pilot source code.
    /// </summary>
    public PilotPatchResult ApplyCodeOrFile(string codeOrPath)
    {
        if (string.IsNullOrWhiteSpace(codeOrPath))
        {
            return new PilotPatchResult
            {
                Success = false,
                Changed = false,
                Version = Version,
                ErrorMessage = "Patch input cannot be empty."
            };
        }

        // Check if the argument refers to an existing file on disk (.pilot or .pilc)
        if (!codeOrPath.Contains('\n') && !codeOrPath.Contains('\r') && File.Exists(codeOrPath))
        {
            return ApplyPatchFile(codeOrPath, onlyIfChanged: false);
        }

        return ApplySourcePatch(codeOrPath, "<inline-patch>");
    }

    /// <summary>
    /// Compiles and applies a Pilot source code patch in-place against the live execution context.
    /// </summary>
    public PilotPatchResult ApplySourcePatch(string patchSource, string sourceLabel = "<inline-patch>")
    {
        var opts = new PilotCompilerOptions().WithSourceFile(sourceLabel);
        var compResult = PilotCompiler.Default.Compile(patchSource, opts);
        if (!compResult.Success || compResult.Program is not PilotCompiledProgram compiledProg)
        {
            string err = string.Join("; ", compResult.Diagnostics.Select(e => e.ToString()));
            return new PilotPatchResult
            {
                Success = false,
                Changed = false,
                Version = Version,
                SourceLabel = sourceLabel,
                ErrorMessage = $"Patch compilation failed: {err}"
            };
        }

        return ApplyCompiledPatch(compiledProg, isCompiledBinary: false, sourceLabel: sourceLabel);
    }

    /// <summary>
    /// Applies a pre-compiled <see cref="PilotCompiledProgram"/> (<c>.pilc</c>) directly to the live
    /// execution context at zero parse cost, with transactional rollback on runtime failure.
    /// </summary>
    public PilotPatchResult ApplyCompiledPatch(
        PilotCompiledProgram patchProgram,
        bool isCompiledBinary = true,
        string? sourceLabel = null)
    {
        if (patchProgram == null) throw new ArgumentNullException(nameof(patchProgram));

        lock (_syncLock)
        {
            string label = sourceLabel ?? patchProgram.FilePath ?? "<compiled-patch>";
            var targetScope = _context.Environment.GlobalScope;

            // 1. Capture pre-patch snapshots for all existing global symbols + any symbols declared in the patch
            var candidateSymbols = new HashSet<string>(
                targetScope.GetLocalBindings().Keys,
                StringComparer.Ordinal);
            foreach (var sym in patchProgram.DefinedSymbols)
            {
                candidateSymbols.Add(sym);
            }

            var preSnapshots = new Dictionary<string, PilotSymbolSnapshot>(StringComparer.Ordinal);
            foreach (var sym in candidateSymbols)
            {
                preSnapshots[sym] = PilotSymbolSnapshot.Capture(targetScope, sym);
            }

            // 2. Execute the patch AST in the global scope
            var savedScope = _context.Environment.CurrentScope;
            while (_context.Environment.CurrentScope.Parent != null)
            {
                _context.Environment.PopScope();
            }

            PilotValue resultVal;
            try
            {
                var visitor = new PilotInterpreterVisitor(_context);
                resultVal = visitor.Visit(patchProgram.ProgramTree);
            }
            catch (Exception ex)
            {
                // Transactional rollback: restore all pre-patch snapshots so live state is uncorrupted
                foreach (var snap in preSnapshots.Values)
                {
                    snap.Restore(targetScope);
                }
                // Remove any brand-new symbols added before the exception
                var postKeys = targetScope.GetLocalBindings().Keys.ToList();
                foreach (var k in postKeys)
                {
                    if (!preSnapshots.ContainsKey(k))
                    {
                        targetScope.RemoveLocal(k);
                    }
                }

                return new PilotPatchResult
                {
                    Success = false,
                    Changed = false,
                    Version = Version,
                    SourceLabel = label,
                    ErrorMessage = $"Patch execution failed (rolled back): {ex.Message}"
                };
            }
            finally
            {
                // Restore scope stack if hot_reload was called from inside a function
                RestoreScopeChain(savedScope);
            }

            // 3. Identify all symbols touched or added by the patch
            var astDeclaredSymbols = ExtractTopLevelDeclaredSymbols(patchProgram.ProgramTree);
            var touchedSymbols = new List<string>();
            var rollbackSnapshots = new Dictionary<string, PilotSymbolSnapshot>(StringComparer.Ordinal);

            foreach (var sym in astDeclaredSymbols)
            {
                if (!touchedSymbols.Contains(sym))
                {
                    touchedSymbols.Add(sym);
                }
                if (preSnapshots.TryGetValue(sym, out var s))
                {
                    rollbackSnapshots[sym] = s;
                }
            }

            foreach (var kvp in targetScope.GetLocalBindings())
            {
                if (!preSnapshots.TryGetValue(kvp.Key, out var preSnap) || !preSnap.Existed)
                {
                    if (!touchedSymbols.Contains(kvp.Key))
                    {
                        touchedSymbols.Add(kvp.Key);
                    }
                    rollbackSnapshots[kvp.Key] = preSnap ?? PilotSymbolSnapshot.CreateNonExistent(kvp.Key);
                }
                else if (kvp.Value.Value != preSnap.RawValue ||
                         (preSnap.FunctionSnapshot != null && kvp.Value.Value.Kind == PilotValueKind.Function &&
                          kvp.Value.Value.AsFunction().PatchVersion != preSnap.FunctionSnapshot.PatchVersion))
                {
                    if (!touchedSymbols.Contains(kvp.Key))
                    {
                        touchedSymbols.Add(kvp.Key);
                    }
                    rollbackSnapshots[kvp.Key] = preSnap;
                }
            }

            // Ensure every touched symbol has its pre-patch snapshot available for rollback
            foreach (var sym in touchedSymbols)
            {
                if (!rollbackSnapshots.ContainsKey(sym) && preSnapshots.TryGetValue(sym, out var snap))
                {
                    rollbackSnapshots[sym] = snap;
                }
            }

            Version++;
            var record = new PilotPatchRecord(
                Version,
                DateTimeOffset.UtcNow,
                label,
                isCompiledBinary,
                touchedSymbols,
                rollbackSnapshots);

            _undoStack.Push(record);
            _history.Add(record);

            return new PilotPatchResult
            {
                Success = true,
                Changed = true,
                Version = Version,
                SourceLabel = label,
                PatchedSymbols = touchedSymbols,
                ReturnValue = resultVal
            };
        }
    }

    /// <summary>
    /// Applies a <c>.pilot</c> script patch or <c>.pilc</c> compiled binary patch from disk.
    /// When <paramref name="onlyIfChanged"/> is true, skips reloading if the file's SHA-256 hash has not changed.
    /// </summary>
    public PilotPatchResult ApplyPatchFile(string filePath, bool onlyIfChanged = false)
    {
        if (string.IsNullOrWhiteSpace(filePath))
        {
            throw new ArgumentException("File path cannot be empty.", nameof(filePath));
        }

        string fullPath = Path.GetFullPath(filePath);
        if (!File.Exists(fullPath))
        {
            return new PilotPatchResult
            {
                Success = false,
                Changed = false,
                Version = Version,
                SourceLabel = fullPath,
                ErrorMessage = $"Patch file not found: '{fullPath}'"
            };
        }

        byte[] rawBytes = File.ReadAllBytes(fullPath);
        string hash = ComputeSha256Hex(rawBytes);

        if (onlyIfChanged && _fileHashes.TryGetValue(fullPath, out var prevHash) && prevHash == hash)
        {
            return new PilotPatchResult
            {
                Success = true,
                Changed = false,
                Version = Version,
                SourceLabel = fullPath
            };
        }

        PilotPatchResult result;
        if (IsCompiledBinary(rawBytes, fullPath))
        {
            var compiledPatch = PilotCompiledProgram.FromBytes(rawBytes, fullPath);
            result = ApplyCompiledPatch(compiledPatch, isCompiledBinary: true, sourceLabel: fullPath);
        }
        else
        {
            string source = Encoding.UTF8.GetString(rawBytes);
            // Strip UTF-8 BOM if present
            if (source.Length > 0 && source[0] == '\uFEFF')
            {
                source = source[1..];
            }
            result = ApplySourcePatch(source, fullPath);
        }

        if (result.Success)
        {
            _fileHashes[fullPath] = hash;
        }

        return result;
    }

    /// <summary>
    /// Rolls back the last <paramref name="steps"/> applied patches in reverse chronological order,
    /// restoring previous function bodies, method group overloads, class definitions, typestates, and globals.
    /// </summary>
    public PilotPatchResult Rollback(int steps = 1)
    {
        if (steps <= 0) steps = 1;

        lock (_syncLock)
        {
            if (_undoStack.Count == 0)
            {
                return new PilotPatchResult
                {
                    Success = false,
                    Changed = false,
                    Version = Version,
                    SourceLabel = "<rollback>",
                    ErrorMessage = "No applied patches to roll back."
                };
            }

            var targetScope = _context.Environment.GlobalScope;
            var restoredSymbols = new List<string>();
            int actualSteps = Math.Min(steps, _undoStack.Count);

            for (int i = 0; i < actualSteps; i++)
            {
                var record = _undoStack.Pop();
                foreach (var kvp in record.Snapshots)
                {
                    kvp.Value.Restore(targetScope);
                    if (!restoredSymbols.Contains(kvp.Key))
                    {
                        restoredSymbols.Add(kvp.Key);
                    }
                }
                Version = Math.Max(0, Version - 1);
            }

            return new PilotPatchResult
            {
                Success = true,
                Changed = true,
                Version = Version,
                SourceLabel = $"<rollback:{actualSteps}>",
                PatchedSymbols = restoredSymbols
            };
        }
    }

    /// <summary>
    /// Registers a live file watcher on a <c>.pilot</c> or <c>.pilc</c> file so that saving the file
    /// automatically hot-patches the running <see cref="PilotExecutionContext"/>.
    /// </summary>
    public void WatchFile(string filePath, PilotFunction? onReloadCallback = null)
    {
        string fullPath = Path.GetFullPath(filePath);
        string? dir = Path.GetDirectoryName(fullPath);
        string fileName = Path.GetFileName(fullPath);
        if (string.IsNullOrEmpty(dir) || !Directory.Exists(dir))
        {
            throw new DirectoryNotFoundException($"Cannot watch '{fullPath}': directory does not exist.");
        }

        if (File.Exists(fullPath))
        {
            _fileHashes[fullPath] = ComputeSha256Hex(File.ReadAllBytes(fullPath));
        }

        if (_watchers.TryRemove(fullPath, out var oldWatcher))
        {
            oldWatcher.Dispose();
        }

        var watcher = new FileSystemWatcher(dir, fileName)
        {
            NotifyFilter = NotifyFilters.LastWrite | NotifyFilters.Size | NotifyFilters.FileName,
            EnableRaisingEvents = true
        };

        void OnFileEvent(object sender, FileSystemEventArgs e)
        {
            try
            {
                // Brief retry loop in case the editor is still flushing the file
                for (int attempt = 0; attempt < 3; attempt++)
                {
                    try
                    {
                        var res = ApplyPatchFile(fullPath, onlyIfChanged: true);
                        if (res.Changed && onReloadCallback != null)
                        {
                            onReloadCallback.Invoke(_context, new[] { res.ToPilotMap() });
                        }
                        break;
                    }
                    catch (IOException)
                    {
                        System.Threading.Thread.Sleep(40);
                    }
                }
            }
            catch
            {
                // Swallow background watcher exceptions so host process never crashes
            }
        }

        watcher.Changed += OnFileEvent;
        watcher.Created += OnFileEvent;
        watcher.Renamed += (s, e) => OnFileEvent(s, e);

        _watchers[fullPath] = watcher;
    }

    public void UnwatchAll()
    {
        foreach (var kvp in _watchers)
        {
            if (_watchers.TryRemove(kvp.Key, out var w))
            {
                w.Dispose();
            }
        }
    }

    public void Dispose()
    {
        UnwatchAll();
    }

    public static bool IsCompiledBinary(byte[] bytes, string? filePath = null)
    {
        if (bytes != null && bytes.Length >= 4)
        {
            uint magic = BitConverter.ToUInt32(bytes, 0);
            if (magic == PilotCompiledProgram.MagicHeader)
            {
                return true;
            }
        }
        return filePath != null && filePath.EndsWith(".pilc", StringComparison.OrdinalIgnoreCase);
    }

    private void RestoreScopeChain(PilotScope savedScope)
    {
        if (ReferenceEquals(_context.Environment.CurrentScope, savedScope))
        {
            return;
        }

        var chain = new Stack<PilotScope>();
        var cursor = savedScope;
        while (cursor != null && cursor.Parent != null)
        {
            chain.Push(cursor);
            cursor = cursor.Parent;
        }

        while (chain.Count > 0)
        {
            var next = chain.Pop();
            var pushed = _context.Environment.PushScope(next.ScopeName);
            foreach (var kvp in next.GetLocalBindings())
            {
                pushed.ForceSet(kvp.Key, kvp.Value.Value, kvp.Value.IsConst, kvp.Value.TypeName);
            }
        }
    }

    private static List<string> ExtractTopLevelDeclaredSymbols(PilotParser.ProgramContext tree)
    {
        var list = new List<string>();
        var stmts = tree?.stmt();
        if (stmts == null) return list;

        foreach (var stmt in stmts)
        {
            var comp = stmt.compound_stmt();
            if (comp != null)
            {
                string? name = comp.func_def()?.identifier()?.GetText()
                            ?? comp.class_def()?.identifier()?.GetText()
                            ?? comp.extension_def()?.identifier()?.GetText()
                            ?? comp.macro_def()?.identifier()?.GetText();
                if (!string.IsNullOrEmpty(name) && !list.Contains(name))
                {
                    list.Add(name);
                }
            }
            var simple = stmt.simple_stmt()?.small_stmt();
            if (simple != null)
            {
                foreach (var sm in simple)
                {
                    string? vName = sm.let_stmt()?.binding_pattern()?.identifier()?.GetText()
                                 ?? sm.var_stmt()?.binding_pattern()?.identifier()?.GetText()
                                 ?? sm.const_stmt()?.binding_pattern()?.identifier()?.GetText();
                    if (!string.IsNullOrEmpty(vName) && !list.Contains(vName))
                    {
                        list.Add(vName);
                    }
                }
            }
        }
        return list;
    }

    private static string ComputeSha256Hex(byte[] data)
    {
        byte[] hash = SHA256.HashData(data);
        return Convert.ToHexString(hash);
    }
}
