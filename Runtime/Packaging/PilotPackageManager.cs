using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using Pilot.Runtime.Packaging.Adapters;

namespace Pilot.Runtime.Packaging;

/// <summary>
/// Universal package manager for Pilot.
/// Manages native Pilot packages and adopts packages from interop ecosystems:
/// NuGet (.NET), npm (JavaScript), pip (Python), Cargo (Rust), Maven (Java), and LuaRocks (Lua).
/// </summary>
public sealed class PilotPackageManager
{
    private static readonly Lazy<PilotPackageManager> _default = new(() => new PilotPackageManager());
    public static PilotPackageManager Default => _default.Value;

    private readonly Dictionary<string, IInteropPackageAdapter> _adapters = new(StringComparer.OrdinalIgnoreCase);

    public PilotPackageManager()
    {
        RegisterAdapter(new NuGetPackageAdapter());
        RegisterAdapter(new NpmPackageAdapter());
        RegisterAdapter(new PipPackageAdapter());
        RegisterAdapter(new CargoPackageAdapter());
        RegisterAdapter(new MavenPackageAdapter());
        RegisterAdapter(new LuaRocksPackageAdapter());
    }

    public void RegisterAdapter(IInteropPackageAdapter adapter)
    {
        if (adapter != null)
        {
            _adapters[adapter.Ecosystem] = adapter;
        }
    }

    public bool TryGetAdapter(string ecosystem, out IInteropPackageAdapter adapter)
    {
        return _adapters.TryGetValue(ecosystem, out adapter!);
    }

    /// <summary>
    /// Initializes a new pilot.json manifest in the target directory.
    /// </summary>
    public PilotPackageManifest Init(string projectDirectory, string? name = null)
    {
        Directory.CreateDirectory(projectDirectory);
        string manifestPath = Path.Combine(projectDirectory, "pilot.json");
        if (File.Exists(manifestPath))
        {
            return PilotPackageManifest.Load(manifestPath);
        }

        string pkgName = !string.IsNullOrWhiteSpace(name)
            ? name
            : new DirectoryInfo(projectDirectory).Name.ToLowerInvariant();

        var manifest = PilotPackageManifest.CreateDefault(pkgName);
        manifest.Save(manifestPath);

        // Create initial entry point if it doesn't exist
        string mainFile = Path.Combine(projectDirectory, manifest.Main);
        if (!File.Exists(mainFile))
        {
            File.WriteAllText(mainFile, $"# {pkgName} entry point\nprintln(\"Hello from {pkgName}!\");\n");
        }

        return manifest;
    }

    /// <summary>
    /// Adds a native or interop dependency to pilot.json and restores it.
    /// Format: [ecosystem:]package[@version]
    /// Examples:
    ///   - math-lib@1.0.0
    ///   - nuget:Newtonsoft.Json@13.0.3
    ///   - npm:lodash@4.17.21
    ///   - pip:requests@2.31.0
    /// </summary>
    public bool Add(string projectDirectory, string packageSpec, string? version = null, Action<string>? log = null)
    {
        string manifestPath = Path.Combine(projectDirectory, "pilot.json");
        var manifest = File.Exists(manifestPath)
            ? PilotPackageManifest.Load(manifestPath)
            : Init(projectDirectory);

        var (ecosystem, pkgName, resolvedVer) = ParsePackageSpec(packageSpec, version);

        if (ecosystem == "pilot")
        {
            manifest.Dependencies[pkgName] = resolvedVer;
            log?.Invoke($"[PKG] Added native Pilot dependency: '{pkgName}': '{resolvedVer}'");
        }
        else
        {
            if (!_adapters.ContainsKey(ecosystem))
            {
                log?.Invoke($"[PKG ERROR] Unsupported package ecosystem '{ecosystem}'. Supported: {string.Join(", ", _adapters.Keys)}");
                return false;
            }

            if (!manifest.InteropDependencies.TryGetValue(ecosystem, out var ecoDict))
            {
                ecoDict = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
                manifest.InteropDependencies[ecosystem] = ecoDict;
            }
            ecoDict[pkgName] = resolvedVer;
            log?.Invoke($"[PKG] Added adopted {ecosystem} dependency: '{pkgName}': '{resolvedVer}'");
        }

        manifest.Save(manifestPath);
        return Restore(projectDirectory, log);
    }

    /// <summary>
    /// Removes a dependency from pilot.json and updates lockfile.
    /// </summary>
    public bool Remove(string projectDirectory, string packageSpec, Action<string>? log = null)
    {
        string manifestPath = Path.Combine(projectDirectory, "pilot.json");
        if (!File.Exists(manifestPath))
        {
            log?.Invoke($"[PKG ERROR] No pilot.json found in '{projectDirectory}'");
            return false;
        }

        var manifest = PilotPackageManifest.Load(manifestPath);
        var (ecosystem, pkgName, _) = ParsePackageSpec(packageSpec, null);

        bool removed = false;
        if (ecosystem == "pilot")
        {
            removed = manifest.Dependencies.Remove(pkgName);
        }
        else if (manifest.InteropDependencies.TryGetValue(ecosystem, out var ecoDict))
        {
            removed = ecoDict.Remove(pkgName);
        }

        if (removed)
        {
            manifest.Save(manifestPath);
            log?.Invoke($"[PKG] Removed dependency '{packageSpec}' from pilot.json.");
            return Restore(projectDirectory, log);
        }

        log?.Invoke($"[PKG WARN] Dependency '{packageSpec}' was not found in pilot.json.");
        return false;
    }

    /// <summary>
    /// Restores all declared dependencies (native and adopted) and writes pilot.lock.json.
    /// </summary>
    public bool Restore(string projectDirectory, Action<string>? log = null)
    {
        string manifestPath = Path.Combine(projectDirectory, "pilot.json");
        if (!File.Exists(manifestPath))
        {
            log?.Invoke($"[PKG ERROR] No pilot.json found in '{projectDirectory}'");
            return false;
        }

        var manifest = PilotPackageManifest.Load(manifestPath);
        string lockPath = Path.Combine(projectDirectory, "pilot.lock.json");
        var lockFile = new PilotPackageLock
        {
            Name = manifest.Name,
            Version = manifest.Version,
            LockVersion = 1
        };

        log?.Invoke($"--> Restoring dependencies for '{manifest.Name}@{manifest.Version}'...");

        string modulesDir = Path.Combine(projectDirectory, ".pilot_modules");
        Directory.CreateDirectory(modulesDir);

        // 1. Restore Native Pilot dependencies
        foreach (var dep in manifest.Dependencies)
        {
            string pkgDir = Path.Combine(modulesDir, "pilot", dep.Key);
            Directory.CreateDirectory(pkgDir);

            // Create module stub / entry point if not already present
            string entryFile = Path.Combine(pkgDir, "index.pilot");
            if (!File.Exists(entryFile))
            {
                File.WriteAllText(entryFile, $"# Native Pilot module '{dep.Key}'\nexport let __version = \"{dep.Value}\";\n");
            }

            string integrity = ComputeHash(dep.Key + ":" + dep.Value);
            lockFile.Packages[$"pilot:{dep.Key}"] = new LockedPackageEntry
            {
                Version = dep.Value,
                Ecosystem = "pilot",
                Resolved = $"local://{dep.Key}",
                Integrity = integrity,
                InstalledPath = pkgDir
            };
            log?.Invoke($"  [PILOT] Restored '{dep.Key}@{dep.Value}'");
        }

        // 2. Restore Adopted Interop dependencies across all ecosystems
        foreach (var kvp in manifest.InteropDependencies)
        {
            string ecosystem = kvp.Key;
            var deps = kvp.Value;
            if (deps.Count == 0) continue;

            if (_adapters.TryGetValue(ecosystem, out var adapter))
            {
                log?.Invoke($"  [{ecosystem.ToUpperInvariant()}] Restoring {deps.Count} adopted package(s)...");
                adapter.Restore(deps, projectDirectory, log);

                foreach (var d in deps)
                {
                    string modPath = Path.Combine(modulesDir, ecosystem, d.Key);
                    lockFile.Packages[$"{ecosystem}:{d.Key}"] = new LockedPackageEntry
                    {
                        Version = d.Value,
                        Ecosystem = ecosystem,
                        Resolved = $"{ecosystem}://{d.Key}",
                        Integrity = ComputeHash($"{ecosystem}:{d.Key}:{d.Value}"),
                        InstalledPath = modPath
                    };
                }
            }
            else
            {
                log?.Invoke($"  [WARN] No interop adapter registered for ecosystem '{ecosystem}'.");
            }
        }

        lockFile.Save(lockPath);
        log?.Invoke($"[PKG SUCCESS] Restored all dependencies. Lockfile updated at '{lockPath}'.");
        return true;
    }

    /// <summary>
    /// Lists all installed native and adopted interop dependencies.
    /// </summary>
    public void List(string projectDirectory, Action<string>? log = null)
    {
        string manifestPath = Path.Combine(projectDirectory, "pilot.json");
        if (!File.Exists(manifestPath))
        {
            log?.Invoke($"[PKG ERROR] No pilot.json found in '{projectDirectory}'");
            return;
        }

        var manifest = PilotPackageManifest.Load(manifestPath);
        log?.Invoke($"Project: {manifest.Name}@{manifest.Version} ({manifest.Description})");
        log?.Invoke($"License: {manifest.License} | Main: {manifest.Main}");

        log?.Invoke($"\nNative Pilot Dependencies ({manifest.Dependencies.Count}):");
        if (manifest.Dependencies.Count == 0)
        {
            log?.Invoke("  (none)");
        }
        else
        {
            foreach (var dep in manifest.Dependencies)
            {
                log?.Invoke($"  - {dep.Key} @ {dep.Value}");
            }
        }

        log?.Invoke("\nAdopted Interop Dependencies:");
        int totalAdopted = 0;
        foreach (var kvp in manifest.InteropDependencies)
        {
            if (kvp.Value.Count > 0)
            {
                log?.Invoke($"  [{kvp.Key.ToUpperInvariant()}] ({kvp.Value.Count}):");
                foreach (var dep in kvp.Value)
                {
                    log?.Invoke($"    - {dep.Key} @ {dep.Value}");
                    totalAdopted++;
                }
            }
        }

        if (totalAdopted == 0)
        {
            log?.Invoke("  (none)");
        }
    }

    /// <summary>
    /// Bundles the project directory into a distributable .pilpkg archive.
    /// </summary>
    public string Pack(string projectDirectory, string? outputZipPath = null, Action<string>? log = null)
    {
        string manifestPath = Path.Combine(projectDirectory, "pilot.json");
        var manifest = File.Exists(manifestPath)
            ? PilotPackageManifest.Load(manifestPath)
            : Init(projectDirectory);

        string safeName = manifest.Name.Replace('/', '-').Replace('\\', '-');
        string defaultFileName = $"{safeName}-{manifest.Version}.pilpkg";
        outputZipPath ??= Path.Combine(projectDirectory, defaultFileName);

        if (File.Exists(outputZipPath))
        {
            File.Delete(outputZipPath);
        }

        log?.Invoke($"--> Packaging '{manifest.Name}' into '{outputZipPath}'...");

        using (var zip = ZipFile.Open(outputZipPath, ZipArchiveMode.Create))
        {
            var files = Directory.GetFiles(projectDirectory, "*.*", SearchOption.AllDirectories);
            foreach (var file in files)
            {
                // Exclude .pilot_modules, obj, bin, and the output archive itself
                string rel = Path.GetRelativePath(projectDirectory, file);
                if (rel.StartsWith(".pilot_modules", StringComparison.OrdinalIgnoreCase) ||
                    rel.StartsWith("bin", StringComparison.OrdinalIgnoreCase) ||
                    rel.StartsWith("obj", StringComparison.OrdinalIgnoreCase) ||
                    rel.EndsWith(".pilpkg", StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                zip.CreateEntryFromFile(file, rel);
            }
        }

        long size = new FileInfo(outputZipPath).Length;
        log?.Invoke($"[PKG SUCCESS] Built package archive: '{outputZipPath}' ({size} bytes).");
        return outputZipPath;
    }

    private static (string Ecosystem, string Name, string Version) ParsePackageSpec(string spec, string? version)
    {
        spec = spec.Trim();
        string ecosystem = "pilot";

        int colonIdx = spec.IndexOf(':');
        if (colonIdx > 0)
        {
            ecosystem = spec[..colonIdx].ToLowerInvariant();
            spec = spec[(colonIdx + 1)..];
        }

        int atIdx = spec.IndexOf('@');
        if (atIdx > 0)
        {
            version = spec[(atIdx + 1)..];
            spec = spec[..atIdx];
        }

        version ??= "*";
        return (ecosystem, spec, version);
    }

    private static string ComputeHash(string text)
    {
        byte[] bytes = SHA256.HashData(Encoding.UTF8.GetBytes(text));
        return Convert.ToHexString(bytes).ToLowerInvariant();
    }
}
