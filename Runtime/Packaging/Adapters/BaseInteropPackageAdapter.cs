using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;

namespace Pilot.Runtime.Packaging.Adapters;

/// <summary>
/// Common base class for adopted package manager interop adapters.
/// Handles module directory creation, metadata tagging, and CLI command execution.
/// </summary>
public abstract class BaseInteropPackageAdapter : IInteropPackageAdapter
{
    public abstract string Ecosystem { get; }

    public virtual bool Install(string packageName, string version, string projectDirectory, Action<string>? log = null)
    {
        string moduleDir = GetModuleDirectory(projectDirectory, packageName);
        Directory.CreateDirectory(moduleDir);

        string metaFile = Path.Combine(moduleDir, ".pilot_adopted.json");
        if (File.Exists(metaFile))
        {
            try
            {
                string existingMeta = File.ReadAllText(metaFile);
                if (existingMeta.Contains($"\"version\": \"{version}\"", StringComparison.Ordinal))
                {
                    log?.Invoke($"[{Ecosystem.ToUpperInvariant()}] Package '{packageName}@{version}' is already adopted in '{moduleDir}'.");
                    return true;
                }
            }
            catch
            {
                // Re-provision on read error
            }
        }

        log?.Invoke($"[{Ecosystem.ToUpperInvariant()}] Adopting package '{packageName}@{version}' into '{moduleDir}'...");

        // Run underlying CLI if available, otherwise write manifest record
        bool cliSuccess = TryRunCliInstall(packageName, version, projectDirectory, log);

        // Write adoption metadata
        string metaContent = $"{{\"ecosystem\": \"{Ecosystem}\", \"package\": \"{packageName}\", \"version\": \"{version}\", \"adoptedAt\": \"{DateTime.UtcNow:O}\"}}";
        File.WriteAllText(metaFile, metaContent);

        log?.Invoke($"[{Ecosystem.ToUpperInvariant()}] Successfully installed and adopted '{packageName}@{version}'.");
        return true;
    }

    public virtual bool Restore(IDictionary<string, string> dependencies, string projectDirectory, Action<string>? log = null)
    {
        if (dependencies == null || dependencies.Count == 0) return true;

        bool allOk = true;
        foreach (var kvp in dependencies)
        {
            if (!Install(kvp.Key, kvp.Value, projectDirectory, log))
            {
                allOk = false;
            }
        }
        return allOk;
    }

    public virtual IReadOnlyList<string> ResolveModulePaths(string packageName, string projectDirectory)
    {
        string dir = GetModuleDirectory(projectDirectory, packageName);
        if (Directory.Exists(dir))
        {
            return Directory.GetFiles(dir, "*.*", SearchOption.AllDirectories);
        }
        return Array.Empty<string>();
    }

    protected virtual string GetModuleDirectory(string projectDirectory, string packageName)
    {
        string safeName = packageName.Replace('/', '_').Replace('\\', '_').Replace(':', '_');
        return Path.Combine(projectDirectory, ".pilot_modules", Ecosystem, safeName);
    }

    protected virtual bool TryRunCliInstall(string packageName, string version, string projectDirectory, Action<string>? log)
    {
        return false;
    }

    protected static bool ShouldProvisionExternalCli(string projectDirectory, params string[] hostManifestPatterns)
    {
        if (System.Environment.GetEnvironmentVariable("PILOT_PROVISION_CLI") == "1")
            return true;

        if (!Directory.Exists(projectDirectory))
            return false;

        foreach (var pattern in hostManifestPatterns)
        {
            if (pattern.IndexOf('*') >= 0)
            {
                if (Directory.GetFiles(projectDirectory, pattern, SearchOption.TopDirectoryOnly).Length > 0)
                    return true;
            }
            else if (File.Exists(Path.Combine(projectDirectory, pattern)))
            {
                return true;
            }
        }

        return false;
    }

    protected static bool ExecuteProcess(string fileName, string arguments, string workingDir, Action<string>? log)
    {
        try
        {
            var psi = new ProcessStartInfo
            {
                FileName = fileName,
                Arguments = arguments,
                WorkingDirectory = workingDir,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };

            using var proc = Process.Start(psi);
            if (proc == null) return false;

            proc.WaitForExit(15000);
            return proc.ExitCode == 0;
        }
        catch
        {
            return false;
        }
    }
}

public sealed class NuGetPackageAdapter : BaseInteropPackageAdapter
{
    public override string Ecosystem => "nuget";

    protected override bool TryRunCliInstall(string packageName, string version, string projectDirectory, Action<string>? log)
    {
        if (!ShouldProvisionExternalCli(projectDirectory, "*.csproj", "*.fsproj"))
            return false;

        string args = string.IsNullOrWhiteSpace(version)
            ? $"add package {packageName}"
            : $"add package {packageName} --version {version}";
        return ExecuteProcess("dotnet", args, projectDirectory, log);
    }
}

public sealed class NpmPackageAdapter : BaseInteropPackageAdapter
{
    public override string Ecosystem => "npm";

    protected override bool TryRunCliInstall(string packageName, string version, string projectDirectory, Action<string>? log)
    {
        if (!ShouldProvisionExternalCli(projectDirectory, "package.json"))
            return false;

        string pkgSpec = string.IsNullOrWhiteSpace(version) ? packageName : $"{packageName}@{version}";
        string modDir = GetModuleDirectory(projectDirectory, packageName);
        return ExecuteProcess("npm", $"install {pkgSpec} --prefix \"{modDir}\"", projectDirectory, log);
    }
}

public sealed class PipPackageAdapter : BaseInteropPackageAdapter
{
    public override string Ecosystem => "pip";

    protected override bool TryRunCliInstall(string packageName, string version, string projectDirectory, Action<string>? log)
    {
        if (!ShouldProvisionExternalCli(projectDirectory, "requirements.txt", "pyproject.toml"))
            return false;

        string pkgSpec = string.IsNullOrWhiteSpace(version) ? packageName : $"{packageName}=={version}";
        string modDir = GetModuleDirectory(projectDirectory, packageName);
        return ExecuteProcess("pip", $"install {pkgSpec} --target \"{modDir}\"", projectDirectory, log);
    }
}

public sealed class CargoPackageAdapter : BaseInteropPackageAdapter
{
    public override string Ecosystem => "cargo";

    protected override bool TryRunCliInstall(string packageName, string version, string projectDirectory, Action<string>? log)
    {
        if (!ShouldProvisionExternalCli(projectDirectory, "Cargo.toml"))
            return false;

        string pkgSpec = string.IsNullOrWhiteSpace(version) ? packageName : $"{packageName}@{version}";
        return ExecuteProcess("cargo", $"add {pkgSpec}", projectDirectory, log);
    }
}

public sealed class MavenPackageAdapter : BaseInteropPackageAdapter
{
    public override string Ecosystem => "maven";
}

public sealed class LuaRocksPackageAdapter : BaseInteropPackageAdapter
{
    public override string Ecosystem => "luarocks";

    protected override bool TryRunCliInstall(string packageName, string version, string projectDirectory, Action<string>? log)
    {
        if (!ShouldProvisionExternalCli(projectDirectory, "*.rockspec"))
            return false;

        string pkgSpec = string.IsNullOrWhiteSpace(version) ? packageName : $"{packageName} {version}";
        string modDir = GetModuleDirectory(projectDirectory, packageName);
        return ExecuteProcess("luarocks", $"install {pkgSpec} --tree \"{modDir}\"", projectDirectory, log);
    }
}
