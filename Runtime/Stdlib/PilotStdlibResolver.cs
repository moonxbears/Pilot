using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.CompilerServices;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using Antlr4.Runtime;
using Pilot.Compiler;
using Pilot.Diagnostics;
using Pilot.Runtime.Bytecode;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Environment;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Stdlib;

/// <summary>
/// Milestone 21.1: Native Pilot <c>std.*</c> &amp; relative module resolver with precompiled <c>.pilc</c> v2
/// disk caching, in-memory AST/bytecode caching, cycle detection, and ADT/Record/Extension propagation.
/// </summary>
public static class PilotStdlibResolver
{
    private static readonly IReadOnlyDictionary<string, string> BuiltinStdlibPaths =
        new Dictionary<string, string>(StringComparer.Ordinal)
        {
            ["std.core"] = Path.Combine("core", "core.pilot"),
            ["std.option"] = Path.Combine("core", "option.pilot"),
            ["std.result"] = Path.Combine("core", "result.pilot"),
            ["std.mem"] = Path.Combine("core", "mem.pilot"),
            ["std.meta"] = Path.Combine("core", "meta.pilot"),
            ["std.collections"] = Path.Combine("collections", "collections.pilot"),
            ["std.itertools"] = Path.Combine("collections", "itertools.pilot"),
            ["std.text"] = Path.Combine("text", "text.pilot"),
            ["std.math"] = Path.Combine("math", "math.pilot"),
            ["std.random"] = Path.Combine("math", "random.pilot"),
            ["std.io"] = Path.Combine("io", "io.pilot"),
            ["std.fs"] = Path.Combine("io", "fs.pilot"),
            ["std.path"] = Path.Combine("io", "path.pilot"),
            ["std.encoding"] = Path.Combine("encoding", "encoding.pilot"),
            ["std.formats"] = Path.Combine("formats", "formats.pilot")
        };

    public static IEnumerable<string> KnownStdlibModuleNames => BuiltinStdlibPaths.Keys;

    public sealed class CachedModuleArtifact
    {
        private PilotParser.ProgramContext? _ast;

        public string ModuleId { get; }
        public string FilePath { get; }
        public string Source { get; }
        public string Sha256Hex { get; }
        public string PilcPath { get; }
        public PilotParser.ProgramContext Ast => _ast ??= CompiledProgram.ProgramTree ?? ParseSourceFast(Source, FilePath);
        public PilotCompiledProgram CompiledProgram { get; }

        public CachedModuleArtifact(
            string moduleId,
            string filePath,
            string source,
            string sha256Hex,
            string pilcPath,
            PilotParser.ProgramContext? ast,
            PilotCompiledProgram compiledProgram)
        {
            ModuleId = moduleId;
            FilePath = filePath;
            Source = source;
            Sha256Hex = sha256Hex;
            PilcPath = pilcPath;
            _ast = ast;
            CompiledProgram = compiledProgram;
        }
    }

    public sealed class LoadedModuleInstance
    {
        public string ModuleId { get; }
        public PilotObject ModuleObject { get; }
        public IReadOnlyDictionary<string, PilotValue> PublicExports { get; }
        public IReadOnlyDictionary<string, PilotAdtDefinition> ExportedAdts { get; }
        public IReadOnlyDictionary<string, PilotRecordDefinition> ExportedRecords { get; }
        public IReadOnlyDictionary<string, PilotAdtVariantDefinition> ExportedVariants { get; }
        public IReadOnlyDictionary<string, Pilot.Runtime.Extensions.PilotLexicalExtension> ExportedExtensions { get; }

        public LoadedModuleInstance(
            string moduleId,
            PilotObject moduleObject,
            IReadOnlyDictionary<string, PilotValue> publicExports,
            IReadOnlyDictionary<string, PilotAdtDefinition> exportedAdts,
            IReadOnlyDictionary<string, PilotRecordDefinition> exportedRecords,
            IReadOnlyDictionary<string, PilotAdtVariantDefinition> exportedVariants,
            IReadOnlyDictionary<string, Pilot.Runtime.Extensions.PilotLexicalExtension> exportedExtensions)
        {
            ModuleId = moduleId;
            ModuleObject = moduleObject;
            PublicExports = publicExports;
            ExportedAdts = exportedAdts;
            ExportedRecords = exportedRecords;
            ExportedVariants = exportedVariants;
            ExportedExtensions = exportedExtensions;
        }
    }

    private sealed class ContextImportSession
    {
        public readonly Dictionary<string, LoadedModuleInstance> Loaded = new(StringComparer.Ordinal);
        public readonly List<string> LoadingChain = new();
    }

    private sealed record PrecompiledBundleEntry(string ModuleId, string Sha256Hex, byte[] PilcBytes, string BackingPath);

    private const uint BundleMagic = 0x424C4950; // "PILB"
    private const byte BundleFormatVersion = 1;

    private static readonly ConcurrentDictionary<string, CachedModuleArtifact> MemoryCache = new(StringComparer.Ordinal);
    private static readonly ConditionalWeakTable<PilotExecutionContext, ContextImportSession> SessionTable = new();
    private static readonly object DiskCacheLock = new();
    private static readonly object BundleLock = new();

    private static Dictionary<string, PrecompiledBundleEntry>? _loadedBundle;
    private static bool _bundleProbed;
    private static string? _activeBundlePath;

    private static long _memoryCacheHits;
    private static long _precompiledBundleHits;
    private static long _diskCacheHits;
    private static long _diskCacheBuilds;
    private static string? _cachedStdlibRoot;

    public static long MemoryCacheHits => Interlocked.Read(ref _memoryCacheHits);
    public static long PrecompiledBundleHits => Interlocked.Read(ref _precompiledBundleHits);
    public static long DiskCacheHits => Interlocked.Read(ref _diskCacheHits);
    public static long DiskCacheBuilds => Interlocked.Read(ref _diskCacheBuilds);
    public static string? ActivePrecompiledBundlePath
    {
        get { EnsureBundleProbed(); return _activeBundlePath; }
    }

    public static void ClearMemoryCacheForTesting()
    {
        MemoryCache.Clear();
        lock (BundleLock)
        {
            _bundleProbed = false;
            _loadedBundle = null;
            _activeBundlePath = null;
        }
    }

    public static void ResetCacheCountersForTesting()
    {
        Interlocked.Exchange(ref _memoryCacheHits, 0);
        Interlocked.Exchange(ref _precompiledBundleHits, 0);
        Interlocked.Exchange(ref _diskCacheHits, 0);
        Interlocked.Exchange(ref _diskCacheBuilds, 0);
    }

    /// <summary>
    /// Locates the root <c>stdlib/</c> directory on disk.
    /// </summary>
    public static string GetStdlibRootDirectory()
    {
        string? envOverride = System.Environment.GetEnvironmentVariable("PILOT_STDLIB_PATH");
        if (!string.IsNullOrWhiteSpace(envOverride) && Directory.Exists(envOverride))
        {
            return Path.GetFullPath(envOverride);
        }

        if (_cachedStdlibRoot != null && Directory.Exists(_cachedStdlibRoot))
        {
            return _cachedStdlibRoot;
        }

        string[] probeRoots =
        {
            Directory.GetCurrentDirectory(),
            AppContext.BaseDirectory
        };

        foreach (string start in probeRoots)
        {
            if (string.IsNullOrWhiteSpace(start)) continue;
            var dir = new DirectoryInfo(start);
            while (dir != null)
            {
                string candidate = Path.Combine(dir.FullName, "stdlib");
                if (Directory.Exists(candidate) &&
                    (File.Exists(Path.Combine(candidate, "pilot.json")) ||
                     File.Exists(Path.Combine(candidate, "core", "core.pilot"))))
                {
                    _cachedStdlibRoot = candidate;
                    return candidate;
                }
                dir = dir.Parent;
            }
        }

        string fallback = Path.Combine(AppContext.BaseDirectory, "stdlib");
        return fallback;
    }

    /// <summary>
    /// Returns the directory storing precompiled <c>.pilc</c> binaries for <c>stdlib/</c>.
    /// </summary>
    public static string GetPilcCacheDirectory()
    {
        string stdlibRoot = GetStdlibRootDirectory();
        string primary = Path.Combine(stdlibRoot, ".pilc-cache");
        try
        {
            Directory.CreateDirectory(primary);
            return primary;
        }
        catch
        {
            string tempFallback = Path.Combine(Path.GetTempPath(), "pilot_stdlib_pilc_cache");
            Directory.CreateDirectory(tempFallback);
            return tempFallback;
        }
    }

    /// <summary>
    /// Returns the canonical <c>.pilc</c> path for a known <c>std.*</c> module.
    /// </summary>
    public static string GetCachedPilcPath(string moduleName)
    {
        string safe = SanitizeModuleName(moduleName);
        return Path.Combine(GetPilcCacheDirectory(), $"{safe}.pilc");
    }

    /// <summary>
    /// Precompiles all native <c>std.*</c> modules to <c>.pilc</c> v2 cache artifacts.
    /// </summary>
    public static IReadOnlyList<CachedModuleArtifact> PrecompileAllStdlibModules()
    {
        var results = new List<CachedModuleArtifact>(BuiltinStdlibPaths.Count);
        foreach (string modName in BuiltinStdlibPaths.Keys.OrderBy(k => k, StringComparer.Ordinal))
        {
            if (TryResolveModulePath(modName, 0, null, out string? fullPath, out string resolvedId) && fullPath != null)
            {
                results.Add(GetOrCompileModuleArtifact(resolvedId, fullPath));
            }
        }
        return results;
    }

    /// <summary>
    /// Compiles every registered <c>std.*</c> module into standalone <c>.pilc</c> v2 binaries AND a single
    /// indexed <c>stdlib.pilcbundle</c> archive for zero-cold-start runtime loading during <c>dotnet build</c>.
    /// </summary>
    public static string BuildPrecompiledStdlibBundle(string outputDirectory)
    {
        Directory.CreateDirectory(outputDirectory);
        string bundlePath = Path.Combine(outputDirectory, "stdlib.pilcbundle");
        var entries = new List<(string ModuleId, string ShaHex, byte[] Bytes)>(BuiltinStdlibPaths.Count);

        foreach (string modName in BuiltinStdlibPaths.Keys.OrderBy(k => k, StringComparer.Ordinal))
        {
            if (!TryResolveModulePath(modName, 0, null, out string? fullPath, out string resolvedId) || fullPath == null)
                continue;

            string normalizedPath = Path.GetFullPath(fullPath);
            string source = File.ReadAllText(normalizedPath, Encoding.UTF8);
            string shaHex = ComputeSha256Short(source + "|" + PilotVersion.FullVersion);

            var compResult = PilotCompiler.Default.Compile(source, new PilotCompilerOptions
            {
                SourceFilePath = normalizedPath,
                EnableCaching = false
            });
            if (!compResult.Success || compResult.Program is not PilotCompiledProgram cp)
            {
                string errText = compResult.Diagnostics != null ? string.Join("; ", compResult.Diagnostics) : "unknown error";
                throw new PilotRuntimeException($"[IMP004] Failed to precompile stdlib module '{resolvedId}': {errText}");
            }

            byte[] pilcBytes = cp.ToBytes();
            string modPilcPath = Path.Combine(outputDirectory, $"{SanitizeModuleName(resolvedId)}.pilc");
            string tmpMod = modPilcPath + "." + Guid.NewGuid().ToString("N") + ".tmp";
            File.WriteAllBytes(tmpMod, pilcBytes);
            File.Move(tmpMod, modPilcPath, overwrite: true);

            entries.Add((resolvedId, shaHex, pilcBytes));
        }

        string tmpBundle = bundlePath + "." + Guid.NewGuid().ToString("N") + ".tmp";
        using (var fs = File.Create(tmpBundle))
        using (var writer = new BinaryWriter(fs, Encoding.UTF8, leaveOpen: false))
        {
            writer.Write(BundleMagic);
            writer.Write(BundleFormatVersion);
            writer.Write(PilotVersion.FullVersion);
            writer.Write(entries.Count);
            foreach (var (modId, shaHex, bytes) in entries)
            {
                writer.Write(modId);
                writer.Write(shaHex);
                writer.Write(bytes.Length);
                writer.Write(bytes);
            }
        }
        File.Move(tmpBundle, bundlePath, overwrite: true);

        lock (BundleLock)
        {
            _bundleProbed = false;
            _loadedBundle = null;
            _activeBundlePath = null;
        }

        return bundlePath;
    }

    private static void EnsureBundleProbed()
    {
        if (_bundleProbed) return;

        lock (BundleLock)
        {
            if (_bundleProbed) return;

            try
            {
                string[] candidates =
                {
                    Path.Combine(AppContext.BaseDirectory, "stdlib", ".pilc-precompiled", "stdlib.pilcbundle"),
                    Path.Combine(GetStdlibRootDirectory(), ".pilc-precompiled", "stdlib.pilcbundle")
                };

                foreach (string candidate in candidates)
                {
                    if (!File.Exists(candidate)) continue;

                    using var fs = File.OpenRead(candidate);
                    using var reader = new BinaryReader(fs, Encoding.UTF8, leaveOpen: false);
                    uint magic = reader.ReadUInt32();
                    byte ver = reader.ReadByte();
                    string compilerVer = reader.ReadString();
                    if (magic != BundleMagic || ver != BundleFormatVersion ||
                        !string.Equals(compilerVer, PilotVersion.FullVersion, StringComparison.Ordinal))
                    {
                        continue;
                    }

                    int count = reader.ReadInt32();
                    var dict = new Dictionary<string, PrecompiledBundleEntry>(count, StringComparer.Ordinal);
                    string bundleDir = Path.GetDirectoryName(candidate) ?? string.Empty;
                    for (int i = 0; i < count; i++)
                    {
                        string modId = reader.ReadString();
                        string shaHex = reader.ReadString();
                        int byteLen = reader.ReadInt32();
                        byte[] bytes = reader.ReadBytes(byteLen);
                        string modPilc = Path.Combine(bundleDir, $"{SanitizeModuleName(modId)}.pilc");
                        string backing = File.Exists(modPilc) ? modPilc : candidate;
                        dict[modId] = new PrecompiledBundleEntry(modId, shaHex, bytes, backing);
                    }

                    _loadedBundle = dict;
                    _activeBundlePath = candidate;
                    break;
                }
            }
            catch
            {
                _loadedBundle = null;
                _activeBundlePath = null;
            }
            finally
            {
                _bundleProbed = true;
            }
        }
    }

    /// <summary>
    /// Resolves a dotted module name (and optional leading relative dots) to a physical <c>.pilot</c> file path.
    /// </summary>
    public static bool TryResolveModulePath(
        string? dottedModuleName,
        int leadingRelativeDots,
        string? importerFilePath,
        out string? resolvedFilePath,
        out string canonicalModuleId)
    {
        resolvedFilePath = null;
        canonicalModuleId = string.Empty;
        string cleanName = (dottedModuleName ?? string.Empty).Trim();

        // 1. Built-in `std.*` standard library modules
        if (leadingRelativeDots == 0)
        {
            if (cleanName == "std")
            {
                canonicalModuleId = "std";
                return true;
            }

            if (BuiltinStdlibPaths.TryGetValue(cleanName, out string? relStdPath))
            {
                string stdRoot = GetStdlibRootDirectory();
                string fullStdPath = Path.Combine(stdRoot, relStdPath);
                if (File.Exists(fullStdPath))
                {
                    resolvedFilePath = Path.GetFullPath(fullStdPath);
                    canonicalModuleId = cleanName;
                    return true;
                }
            }
        }

        // 2. Relative or workspace-local `.pilot` module resolution
        string baseDir = !string.IsNullOrWhiteSpace(importerFilePath)
            ? (Path.GetDirectoryName(Path.GetFullPath(importerFilePath)) ?? Directory.GetCurrentDirectory())
            : Directory.GetCurrentDirectory();

        if (leadingRelativeDots > 1)
        {
            for (int i = 1; i < leadingRelativeDots; i++)
            {
                baseDir = Directory.GetParent(baseDir)?.FullName ?? baseDir;
            }
        }

        string[] segments = string.IsNullOrEmpty(cleanName)
            ? Array.Empty<string>()
            : cleanName.Split('.', StringSplitOptions.RemoveEmptyEntries);

        if (segments.Length > 0)
        {
            string relPath = Path.Combine(segments);
            string directFile = Path.Combine(baseDir, relPath + ".pilot");
            if (File.Exists(directFile))
            {
                resolvedFilePath = Path.GetFullPath(directFile);
                canonicalModuleId = $"file:{resolvedFilePath}";
                return true;
            }

            string pkgInitFile = Path.Combine(baseDir, relPath, segments[^1] + ".pilot");
            if (File.Exists(pkgInitFile))
            {
                resolvedFilePath = Path.GetFullPath(pkgInitFile);
                canonicalModuleId = $"file:{resolvedFilePath}";
                return true;
            }
        }

        return false;
    }

    /// <summary>
    /// Fast AST lookup for static analyzers (<see cref="Pilot.Analysis.TypeSystem.PilotErrorFlowAnalyzer"/>,
    /// <see cref="Pilot.Analysis.PatternMatching.PilotExhaustivenessChecker"/>,
    /// <see cref="Pilot.Analysis.TypeSystem.PilotBidirectionalInferencer"/>, and LSP).
    /// </summary>
    public static bool TryGetParsedModuleAst(
        string? dottedModuleName,
        int leadingRelativeDots,
        string? importerFilePath,
        out PilotParser.ProgramContext? ast,
        out string canonicalModuleId)
    {
        ast = null;
        if (!TryResolveModulePath(dottedModuleName, leadingRelativeDots, importerFilePath, out string? filePath, out canonicalModuleId))
            return false;

        if (canonicalModuleId == "std" || filePath == null)
            return false;

        var artifact = GetOrCompileModuleArtifact(canonicalModuleId, filePath);
        ast = artifact.Ast;
        return true;
    }

    /// <summary>
    /// Compiles or loads a module artifact from the in-memory or <c>.pilc</c> v2 disk cache.
    /// </summary>
    public static CachedModuleArtifact GetOrCompileModuleArtifact(string moduleId, string fullFilePath)
    {
        string normalizedPath = Path.GetFullPath(fullFilePath);
        string source = File.ReadAllText(normalizedPath, Encoding.UTF8);
        string shaHex = ComputeSha256Short(source + "|" + PilotVersion.FullVersion);
        string cacheKey = $"{moduleId}@{shaHex}";

        if (MemoryCache.TryGetValue(cacheKey, out var memHit))
        {
            Interlocked.Increment(ref _memoryCacheHits);
            return memHit;
        }

        // Fast-path: Zero-cold-start load from build-time precompiled `stdlib.pilcbundle`
        if (moduleId.StartsWith("std.", StringComparison.Ordinal))
        {
            EnsureBundleProbed();
            if (_loadedBundle != null &&
                _loadedBundle.TryGetValue(moduleId, out var bundleEntry) &&
                string.Equals(bundleEntry.Sha256Hex, shaHex, StringComparison.Ordinal))
            {
                try
                {
                    var fromBundle = PilotCompiledProgram.FromBytes(bundleEntry.PilcBytes, normalizedPath);
                    Interlocked.Increment(ref _precompiledBundleHits);
                    Interlocked.Increment(ref _diskCacheHits);
                    var bundleArtifact = new CachedModuleArtifact(
                        moduleId,
                        normalizedPath,
                        source,
                        shaHex,
                        bundleEntry.BackingPath,
                        fromBundle.ProgramTree,
                        fromBundle);
                    MemoryCache[cacheKey] = bundleArtifact;
                    return bundleArtifact;
                }
                catch
                {
                    // Fall through to standard disk cache / compiler if bundle deserialization fails
                }
            }
        }

        string cacheDir = GetPilcCacheDirectory();
        string safeName = SanitizeModuleName(moduleId.StartsWith("std.", StringComparison.Ordinal)
            ? moduleId
            : Path.GetFileNameWithoutExtension(normalizedPath));
        string hashedPilcPath = Path.Combine(cacheDir, $"{safeName}_{shaHex}.pilc");
        string canonicalPilcPath = moduleId.StartsWith("std.", StringComparison.Ordinal)
            ? Path.Combine(cacheDir, $"{safeName}.pilc")
            : hashedPilcPath;

        PilotCompiledProgram? compiled = null;

        lock (DiskCacheLock)
        {
            if (File.Exists(hashedPilcPath))
            {
                try
                {
                    var loaded = PilotCompiledProgram.Load(hashedPilcPath);
                    if (string.Equals(loaded.CompilerVersion, PilotVersion.FullVersion, StringComparison.Ordinal))
                    {
                        compiled = loaded;
                        Interlocked.Increment(ref _diskCacheHits);
                    }
                }
                catch
                {
                    compiled = null;
                }
            }

            if (compiled == null)
            {
                var compResult = PilotCompiler.Default.Compile(source, new PilotCompilerOptions
                {
                    SourceFilePath = normalizedPath,
                    EnableCaching = false
                });
                if (!compResult.Success || compResult.Program is not PilotCompiledProgram cp)
                {
                    string errText = compResult.Diagnostics != null ? string.Join("; ", compResult.Diagnostics) : "unknown compile error";
                    throw new PilotRuntimeException($"[IMP004] Failed to compile module '{moduleId}' ({normalizedPath}): {errText}");
                }
                compiled = cp;
                try
                {
                    string tmpPath = hashedPilcPath + "." + Guid.NewGuid().ToString("N") + ".tmp";
                    compiled.Save(tmpPath);
                    File.Move(tmpPath, hashedPilcPath, overwrite: true);
                    if (!string.Equals(canonicalPilcPath, hashedPilcPath, StringComparison.Ordinal))
                    {
                        string tmpCanon = canonicalPilcPath + "." + Guid.NewGuid().ToString("N") + ".tmp";
                        compiled.Save(tmpCanon);
                        File.Move(tmpCanon, canonicalPilcPath, overwrite: true);
                    }
                }
                catch
                {
                    // Disk cache write is best-effort on read-only filesystems
                }
                Interlocked.Increment(ref _diskCacheBuilds);
            }
            else if (!string.Equals(canonicalPilcPath, hashedPilcPath, StringComparison.Ordinal) && !File.Exists(canonicalPilcPath))
            {
                try
                {
                    File.Copy(hashedPilcPath, canonicalPilcPath, overwrite: true);
                }
                catch
                {
                }
            }
        }

        string reportedPilc = File.Exists(canonicalPilcPath) ? canonicalPilcPath : hashedPilcPath;
        var artifact = new CachedModuleArtifact(moduleId, normalizedPath, source, shaHex, reportedPilc, compiled.ProgramTree, compiled);
        MemoryCache[cacheKey] = artifact;
        return artifact;
    }

    #region Runtime Import Execution (`import ...` & `from ... import ...`)

    public static PilotValue ExecuteImportStmt(
        PilotParser.Import_stmtContext ctx,
        PilotExecutionContext execCtx,
        string? importerFilePath = null)
    {
        if (ctx.import_name() != null)
        {
            return ExecuteImportName(ctx.import_name(), execCtx, importerFilePath);
        }
        if (ctx.import_from() != null)
        {
            return ExecuteImportFrom(ctx.import_from(), execCtx, importerFilePath);
        }
        return PilotValue.Null;
    }

    private static string? ResolveActiveImporterFilePath(PilotExecutionContext execCtx, string? explicitImporterFilePath)
    {
        if (!string.IsNullOrWhiteSpace(explicitImporterFilePath))
            return explicitImporterFilePath;

        if (execCtx.CurrentScope.TryResolve("__file__", out var fileVal) &&
            fileVal.Kind == PilotValueKind.String &&
            !string.IsNullOrWhiteSpace(fileVal.AsString()))
        {
            return fileVal.AsString();
        }

        if (!string.IsNullOrWhiteSpace(execCtx.Options.SourceFilePath))
            return execCtx.Options.SourceFilePath;

        return null;
    }

    public static PilotValue ExecuteImportName(
        PilotParser.Import_nameContext ctx,
        PilotExecutionContext execCtx,
        string? importerFilePath = null)
    {
        importerFilePath = ResolveActiveImporterFilePath(execCtx, importerFilePath);
        var specs = ctx.dotted_as_names()?.dotted_as_name();
        if (specs == null) return PilotValue.Null;

        PilotValue lastModuleVal = PilotValue.Null;
        foreach (var spec in specs)
        {
            string dotted = spec.dotted_name().GetText();
            string? alias = spec.identifier()?.GetText();

            if (!TryResolveModulePath(dotted, 0, importerFilePath, out string? filePath, out string moduleId))
            {
                throw new PilotRuntimeException(
                    $"[IMP001] Cannot resolve module '{dotted}'.",
                    spec.Start.Line,
                    spec.Start.Column);
            }

            if (moduleId == "std")
            {
                var stdObj = BuildOrGetFullStdUmbrella(execCtx);
                string targetName = !string.IsNullOrEmpty(alias) ? alias! : "std";
                BindInCurrentScope(execCtx, targetName, PilotValue.FromObject(stdObj));
                lastModuleVal = PilotValue.FromObject(stdObj);
                continue;
            }

            var loaded = LoadModuleIntoSession(moduleId, filePath!, execCtx, spec.Start.Line, spec.Start.Column);
            MergeModuleTypesAndExtensions(loaded, execCtx);
            var modVal = PilotValue.FromObject(loaded.ModuleObject);
            lastModuleVal = modVal;

            if (!string.IsNullOrEmpty(alias))
            {
                BindInCurrentScope(execCtx, alias!, modVal);
            }
            else
            {
                string[] parts = dotted.Split('.', StringSplitOptions.RemoveEmptyEntries);
                if (parts.Length == 1)
                {
                    BindInCurrentScope(execCtx, parts[0], modVal);
                }
                else
                {
                    // Attach hierarchical namespace chain (e.g. `std.math`) AND convenience leaf name (`math`)
                    PilotObject rootNs = GetOrCreateNamespaceObject(execCtx, parts[0]);
                    PilotObject curNs = rootNs;
                    for (int i = 1; i < parts.Length - 1; i++)
                    {
                        if (curNs.Fields.TryGetValue(parts[i], out var subVal) && subVal.Kind == PilotValueKind.Object)
                        {
                            curNs = subVal.AsObject();
                        }
                        else
                        {
                            var nextNs = new PilotObject(new PilotClassDefinition($"Namespace:{string.Join(".", parts[..(i + 1)])}"));
                            curNs.Fields[parts[i]] = PilotValue.FromObject(nextNs);
                            curNs = nextNs;
                        }
                    }
                    string leaf = parts[^1];
                    curNs.Fields[leaf] = modVal;

                    // Also bind leaf module name (`math` for `import std.math;`, `meta` for `import std.meta;`)
                    BindInCurrentScope(execCtx, leaf, modVal);
                }
            }
        }

        return lastModuleVal;
    }

    public static PilotValue ExecuteImportFrom(
        PilotParser.Import_fromContext ctx,
        PilotExecutionContext execCtx,
        string? importerFilePath = null)
    {
        importerFilePath = ResolveActiveImporterFilePath(execCtx, importerFilePath);
        int dotCount = ctx.DOT()?.Length ?? 0;
        string dotted = ctx.dotted_name()?.GetText() ?? string.Empty;

        if (!TryResolveModulePath(dotted, dotCount, importerFilePath, out string? filePath, out string moduleId))
        {
            string displayName = (new string('.', dotCount)) + dotted;
            throw new PilotRuntimeException(
                $"[IMP001] Cannot resolve module '{displayName}'.",
                ctx.Start.Line,
                ctx.Start.Column);
        }

        // Handle `from std import math, collections as col;`
        if (moduleId == "std")
        {
            if (ctx.STAR() != null)
            {
                foreach (string fullStdMod in BuiltinStdlibPaths.Keys)
                {
                    string subName = fullStdMod["std.".Length..];
                    if (TryResolveModulePath(fullStdMod, 0, null, out string? subPath, out string subId) && subPath != null)
                    {
                        var subLoaded = LoadModuleIntoSession(subId, subPath, execCtx, ctx.Start.Line, ctx.Start.Column);
                        MergeModuleTypesAndExtensions(subLoaded, execCtx);
                        BindInCurrentScope(execCtx, subName, PilotValue.FromObject(subLoaded.ModuleObject));
                    }
                }
                return PilotValue.Null;
            }

            var stdItems = ctx.import_as_names()?.import_as_name();
            if (stdItems != null)
            {
                foreach (var item in stdItems)
                {
                    var ids = item.identifier();
                    string subName = ids[0].GetText();
                    string bindName = ids.Length > 1 ? ids[1].GetText() : subName;
                    string fullStdMod = $"std.{subName}";
                    if (!TryResolveModulePath(fullStdMod, 0, null, out string? subPath, out string subId) || subPath == null)
                    {
                        throw new PilotRuntimeException(
                            $"[IMP003] Standard library namespace 'std' has no submodule '{subName}'.",
                            item.Start.Line,
                            item.Start.Column);
                    }
                    var subLoaded = LoadModuleIntoSession(subId, subPath, execCtx, item.Start.Line, item.Start.Column);
                    MergeModuleTypesAndExtensions(subLoaded, execCtx);
                    BindInCurrentScope(execCtx, bindName, PilotValue.FromObject(subLoaded.ModuleObject));
                }
            }
            return PilotValue.Null;
        }

        var loaded = LoadModuleIntoSession(moduleId, filePath!, execCtx, ctx.Start.Line, ctx.Start.Column);
        MergeModuleTypesAndExtensions(loaded, execCtx);

        if (ctx.STAR() != null)
        {
            foreach (var kvp in loaded.PublicExports)
            {
                BindInCurrentScope(execCtx, kvp.Key, kvp.Value);
            }
            return PilotValue.FromObject(loaded.ModuleObject);
        }

        var importItems = ctx.import_as_names()?.import_as_name();
        if (importItems != null)
        {
            foreach (var item in importItems)
            {
                var ids = item.identifier();
                string memberName = ids[0].GetText();
                string bindName = ids.Length > 1 ? ids[1].GetText() : memberName;

                if (loaded.ModuleObject.Fields.TryGetValue(memberName, out var exportedVal))
                {
                    BindInCurrentScope(execCtx, bindName, exportedVal);
                }
                else
                {
                    throw new PilotRuntimeException(
                        $"[IMP003] Module '{moduleId}' does not export symbol '{memberName}'.",
                        item.Start.Line,
                        item.Start.Column);
                }
            }
        }

        return PilotValue.FromObject(loaded.ModuleObject);
    }

    private static LoadedModuleInstance LoadModuleIntoSession(
        string moduleId,
        string fullFilePath,
        PilotExecutionContext execCtx,
        int line,
        int col)
    {
        var session = SessionTable.GetOrCreateValue(execCtx);

        if (session.Loaded.TryGetValue(moduleId, out var alreadyLoaded))
        {
            return alreadyLoaded;
        }

        if (session.LoadingChain.Contains(moduleId, StringComparer.Ordinal))
        {
            string cycleTrace = string.Join(" -> ", session.LoadingChain.Append(moduleId));
            throw new PilotRuntimeException(
                $"[IMP002] Circular module import detected: {cycleTrace}",
                line,
                col);
        }

        var artifact = GetOrCompileModuleArtifact(moduleId, fullFilePath);
        session.LoadingChain.Add(moduleId);

        // Snapshot pre-existing ADTs/records/extensions so we know what the module defined/imported
        var prevAdts = new HashSet<string>(execCtx.AdtDefinitions.Keys, StringComparer.Ordinal);
        var prevRecords = new HashSet<string>(execCtx.RecordDefinitions.Keys, StringComparer.Ordinal);
        var prevVariants = new HashSet<string>(execCtx.VariantDefinitions.Keys, StringComparer.Ordinal);
        var prevExts = new HashSet<string>(execCtx.RegisteredExtensions.Keys, StringComparer.Ordinal);

        var moduleScope = new PilotScope(execCtx.Environment.GlobalScope, $"module:{moduleId}");
        moduleScope.Declare("__file__", PilotValue.FromString(artifact.FilePath));
        var savedScope = execCtx.Environment.SwapScope(moduleScope);
        var savedFlow = execCtx.PendingFlow;
        execCtx.PendingFlow = PilotFlowSignalKind.None;

        try
        {
            if (artifact.CompiledProgram.BytecodeChunk != null)
            {
                PilVirtualMachine.Execute(artifact.CompiledProgram.BytecodeChunk, execCtx);
            }
            else
            {
                var modVisitor = new PilotInterpreterVisitor(execCtx);
                modVisitor.VisitProgram(artifact.Ast);
            }
        }
        finally
        {
            execCtx.PendingFlow = savedFlow;
            execCtx.Environment.SwapScope(savedScope);
            session.LoadingChain.RemoveAt(session.LoadingChain.Count - 1);
        }

        var modClass = new PilotClassDefinition($"Module:{moduleId}");
        var modObj = new PilotObject(modClass);
        var publicExports = new Dictionary<string, PilotValue>(StringComparer.Ordinal);

        var localBindings = moduleScope.GetLocalBindings();
        foreach (var kvp in localBindings)
        {
            modObj.Fields[kvp.Key] = kvp.Value.Value;
            if (!kvp.Key.StartsWith("__", StringComparison.Ordinal))
            {
                publicExports[kvp.Key] = kvp.Value.Value;
            }
        }

        // Re-export zero-overhead native elementary math & regex intrinsics on their canonical std.* namespaces
        string[]? reexportedBuiltins = moduleId switch
        {
            "std.math" => new[]
            {
                "sin", "cos", "tan", "asin", "acos", "atan", "atan2",
                "sinh", "cosh", "tanh", "exp", "ln", "log", "log2", "log10",
                "sqrt", "cbrt", "pow", "hypot", "abs", "min", "max",
                "floor", "ceil", "trunc", "round", "isnan", "isinf", "isfinite"
            },
            "std.text" => new[] { "compile_regex" },
            _ => null
        };

        if (reexportedBuiltins != null)
        {
            var globals = execCtx.Environment.GlobalScope.GetLocalBindings();
            foreach (string builtinName in reexportedBuiltins)
            {
                if (!modObj.Fields.ContainsKey(builtinName) && globals.TryGetValue(builtinName, out var globalBind))
                {
                    modObj.Fields[builtinName] = globalBind.Value;
                    publicExports[builtinName] = globalBind.Value;
                }
            }
        }

        modObj.Fields["__name__"] = PilotValue.FromString(moduleId);
        modObj.Fields["__file__"] = PilotValue.FromString(artifact.FilePath);
        modObj.Fields["__pilc__"] = PilotValue.FromString(artifact.PilcPath);

        var expAdts = new Dictionary<string, PilotAdtDefinition>(StringComparer.Ordinal);
        foreach (var kvp in execCtx.AdtDefinitions)
        {
            if (!prevAdts.Contains(kvp.Key) || localBindings.ContainsKey(kvp.Key))
                expAdts[kvp.Key] = kvp.Value;
        }

        var expRecords = new Dictionary<string, PilotRecordDefinition>(StringComparer.Ordinal);
        foreach (var kvp in execCtx.RecordDefinitions)
        {
            if (!prevRecords.Contains(kvp.Key) || localBindings.ContainsKey(kvp.Key))
                expRecords[kvp.Key] = kvp.Value;
        }

        var expVariants = new Dictionary<string, PilotAdtVariantDefinition>(StringComparer.Ordinal);
        foreach (var kvp in execCtx.VariantDefinitions)
        {
            if (!prevVariants.Contains(kvp.Key) || localBindings.ContainsKey(kvp.Key))
                expVariants[kvp.Key] = kvp.Value;
        }

        var expExtensions = new Dictionary<string, Pilot.Runtime.Extensions.PilotLexicalExtension>(StringComparer.Ordinal);
        foreach (var kvp in execCtx.RegisteredExtensions)
        {
            if (!prevExts.Contains(kvp.Key))
                expExtensions[kvp.Key] = kvp.Value;
        }

        var instance = new LoadedModuleInstance(
            moduleId,
            modObj,
            publicExports,
            expAdts,
            expRecords,
            expVariants,
            expExtensions);

        session.Loaded[moduleId] = instance;
        return instance;
    }

    private static void MergeModuleTypesAndExtensions(LoadedModuleInstance loaded, PilotExecutionContext execCtx)
    {
        foreach (var kvp in loaded.ExportedAdts)
            execCtx.AdtDefinitions[kvp.Key] = kvp.Value;

        foreach (var kvp in loaded.ExportedRecords)
            execCtx.RecordDefinitions[kvp.Key] = kvp.Value;

        foreach (var kvp in loaded.ExportedVariants)
            execCtx.VariantDefinitions[kvp.Key] = kvp.Value;

        foreach (var kvp in loaded.ExportedExtensions)
        {
            execCtx.RegisteredExtensions[kvp.Key] = kvp.Value;
            if (!execCtx.ActiveExtensions.Contains(kvp.Value))
            {
                execCtx.ActiveExtensions.Push(kvp.Value);
            }
        }
    }

    private static PilotObject BuildOrGetFullStdUmbrella(PilotExecutionContext execCtx)
    {
        var stdObj = GetOrCreateNamespaceObject(execCtx, "std");
        foreach (string fullMod in BuiltinStdlibPaths.Keys)
        {
            string sub = fullMod["std.".Length..];
            if (!stdObj.Fields.ContainsKey(sub) &&
                TryResolveModulePath(fullMod, 0, null, out string? subPath, out string subId) &&
                subPath != null)
            {
                var loaded = LoadModuleIntoSession(subId, subPath, execCtx, 0, 0);
                MergeModuleTypesAndExtensions(loaded, execCtx);
                stdObj.Fields[sub] = PilotValue.FromObject(loaded.ModuleObject);
            }
        }
        return stdObj;
    }

    private static PilotObject GetOrCreateNamespaceObject(PilotExecutionContext execCtx, string rootName)
    {
        if (execCtx.CurrentScope.ContainsLocal(rootName))
        {
            var existing = execCtx.CurrentScope.Resolve(rootName);
            if (existing.Kind == PilotValueKind.Object)
            {
                return existing.AsObject();
            }
        }

        var nsObj = new PilotObject(new PilotClassDefinition($"Namespace:{rootName}"));
        BindInCurrentScope(execCtx, rootName, PilotValue.FromObject(nsObj));
        return nsObj;
    }

    private static void BindInCurrentScope(PilotExecutionContext execCtx, string name, PilotValue value)
    {
        if (execCtx.CurrentScope.ContainsLocal(name))
        {
            execCtx.CurrentScope.ForceSet(name, value);
        }
        else
        {
            execCtx.CurrentScope.Declare(name, value);
        }
    }

    #endregion

    #region Helpers

    private static PilotParser.ProgramContext ParseSourceFast(string source, string filePath)
    {
        var inputStream = new AntlrInputStream(source);
        var lexer = new PilotLexer(inputStream);
        var tokenStream = new CommonTokenStream(lexer);
        var parser = new PilotParser(tokenStream);
        parser.RemoveErrorListeners();
        return parser.ParseProgramFast();
    }

    private static string ComputeSha256Short(string text)
    {
        byte[] bytes = SHA256.HashData(Encoding.UTF8.GetBytes(text));
        return Convert.ToHexString(bytes)[..16].ToLowerInvariant();
    }

    private static string SanitizeModuleName(string name)
    {
        var sb = new StringBuilder(name.Length);
        foreach (char c in name)
        {
            sb.Append(char.IsLetterOrDigit(c) || c is '.' or '_' or '-' ? c : '_');
        }
        return sb.ToString();
    }

    #endregion
}
