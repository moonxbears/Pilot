using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace Pilot.Runtime.Venv;

/// <summary>
/// Configuration options for creating a Pilot virtual environment.
/// </summary>
public sealed class PilotVenvOptions
{
    public bool IncludeSystemPackages { get; set; } = false;
    public string? Prompt { get; set; }
    public bool CopyBinaries { get; set; } = false;
    public Dictionary<string, string> ExtraConfig { get; } = new(StringComparer.OrdinalIgnoreCase);
}

/// <summary>
/// Metadata loaded from pilotvenv.cfg.
/// </summary>
public sealed class PilotVenvConfig
{
    public string Home { get; set; } = string.Empty;
    public string Version { get; set; } = "1.0.0";
    public string PilotVersion { get; set; } = "1.0.0";
    public bool IncludeSystemPackages { get; set; } = false;
    public string Prompt { get; set; } = string.Empty;
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    public Dictionary<string, string> Properties { get; } = new(StringComparer.OrdinalIgnoreCase);

    public static PilotVenvConfig Load(string cfgPath)
    {
        var config = new PilotVenvConfig();
        if (!File.Exists(cfgPath)) return config;

        foreach (var line in File.ReadAllLines(cfgPath))
        {
            string trimmed = line.Trim();
            if (string.IsNullOrEmpty(trimmed) || trimmed.StartsWith('#') || trimmed.StartsWith(';'))
                continue;

            int eqIdx = trimmed.IndexOf('=');
            if (eqIdx > 0)
            {
                string key = trimmed[..eqIdx].Trim();
                string val = trimmed[(eqIdx + 1)..].Trim();

                switch (key.ToLowerInvariant())
                {
                    case "home":
                        config.Home = val;
                        break;
                    case "version":
                        config.Version = val;
                        break;
                    case "pilot_version":
                        config.PilotVersion = val;
                        break;
                    case "include_system_packages":
                        config.IncludeSystemPackages = bool.TryParse(val, out bool b) && b;
                        break;
                    case "prompt":
                        config.Prompt = val;
                        break;
                    case "created_at":
                        if (DateTime.TryParse(val, out var dt)) config.CreatedAt = dt;
                        break;
                    default:
                        config.Properties[key] = val;
                        break;
                }
            }
        }

        return config;
    }

    public void Save(string cfgPath)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# Pilot Virtual Environment Configuration");
        sb.AppendLine($"home = {Home}");
        sb.AppendLine($"version = {Version}");
        sb.AppendLine($"pilot_version = {PilotVersion}");
        sb.AppendLine($"include_system_packages = {IncludeSystemPackages.ToString().ToLowerInvariant()}");
        sb.AppendLine($"prompt = {Prompt}");
        sb.AppendLine($"created_at = {CreatedAt:O}");

        foreach (var kvp in Properties)
        {
            sb.AppendLine($"{kvp.Key} = {kvp.Value}");
        }

        File.WriteAllText(cfgPath, sb.ToString());
    }
}

/// <summary>
/// Universal virtual environment manager for Pilot.
/// Provides isolated package management, environment variables,
/// launcher activation scripts, and interop sandboxing.
/// </summary>
public static class PilotVirtualEnvironment
{
    public const string ConfigFileName = "pilotvenv.cfg";
    public const string EnvVarName = "PILOT_VENV";

    /// <summary>
    /// Gets the path to the currently active virtual environment, if any.
    /// Checks environment variables, then active process context.
    /// </summary>
    public static string? ActiveVenvPath
    {
        get
        {
            string? env = System.Environment.GetEnvironmentVariable(EnvVarName);
            if (!string.IsNullOrWhiteSpace(env) && Directory.Exists(env))
            {
                return Path.GetFullPath(env);
            }

            string? pyEnv = System.Environment.GetEnvironmentVariable("VIRTUAL_ENV");
            if (!string.IsNullOrWhiteSpace(pyEnv) && Directory.Exists(pyEnv) &&
                File.Exists(Path.Combine(pyEnv, ConfigFileName)))
            {
                return Path.GetFullPath(pyEnv);
            }

            return null;
        }
    }

    public static bool IsActive => !string.IsNullOrEmpty(ActiveVenvPath);

    /// <summary>
    /// Creates a complete, isolated virtual environment at the target path.
    /// </summary>
    public static PilotVenvConfig Create(string targetPath, PilotVenvOptions? options = null)
    {
        options ??= new PilotVenvOptions();
        string fullPath = Path.GetFullPath(targetPath);
        Directory.CreateDirectory(fullPath);

        string envName = options.Prompt ?? new DirectoryInfo(fullPath).Name;

        // 1. Directory Structure:
        //    <venv>/
        //    ├── pilotvenv.cfg
        //    ├── Scripts/ (or bin/ on Unix)
        //    ├── lib/
        //    │   └── .pilot_modules/
        //    └── include/
        string scriptsDir = Path.Combine(fullPath, OperatingSystem.IsWindows() ? "Scripts" : "bin");
        string libDir = Path.Combine(fullPath, "lib");
        string modulesDir = Path.Combine(libDir, ".pilot_modules");
        string includeDir = Path.Combine(fullPath, "include");

        Directory.CreateDirectory(scriptsDir);
        Directory.CreateDirectory(libDir);
        Directory.CreateDirectory(modulesDir);
        Directory.CreateDirectory(includeDir);

        // Also create bin/ on Windows for cross-platform tool compatibility
        if (OperatingSystem.IsWindows())
        {
            Directory.CreateDirectory(Path.Combine(fullPath, "bin"));
        }

        // 2. Write pilotvenv.cfg
        string homeDir = AppContext.BaseDirectory;
        var config = new PilotVenvConfig
        {
            Home = homeDir,
            Version = "1.0.0",
            PilotVersion = "1.0.0",
            IncludeSystemPackages = options.IncludeSystemPackages,
            Prompt = envName,
            CreatedAt = DateTime.UtcNow
        };

        foreach (var kvp in options.ExtraConfig)
        {
            config.Properties[kvp.Key] = kvp.Value;
        }

        string cfgPath = Path.Combine(fullPath, ConfigFileName);
        config.Save(cfgPath);

        // 3. Generate Activation Scripts
        GenerateActivationScripts(fullPath, scriptsDir, envName);

        // 4. Generate local pilot launcher wrapper in Scripts / bin
        GenerateLauncher(fullPath, scriptsDir);

        return config;
    }

    /// <summary>
    /// Detects if the given directory or any parent directory represents a Pilot virtual environment.
    /// </summary>
    public static string? Detect(string startDirectory)
    {
        string? current = Path.GetFullPath(startDirectory);
        while (!string.IsNullOrEmpty(current))
        {
            if (File.Exists(Path.Combine(current, ConfigFileName)))
            {
                return current;
            }

            string dotVenv = Path.Combine(current, ".pilotvenv");
            if (Directory.Exists(dotVenv) && File.Exists(Path.Combine(dotVenv, ConfigFileName)))
            {
                return dotVenv;
            }

            string dotEnv = Path.Combine(current, ".venv");
            if (Directory.Exists(dotEnv) && File.Exists(Path.Combine(dotEnv, ConfigFileName)))
            {
                return dotEnv;
            }

            var parent = Directory.GetParent(current);
            current = parent?.FullName;
        }

        return null;
    }

    /// <summary>
    /// Activates the virtual environment in the current process by setting environment variables.
    /// </summary>
    public static void Activate(string venvPath)
    {
        string fullPath = Path.GetFullPath(venvPath);
        if (!File.Exists(Path.Combine(fullPath, ConfigFileName)))
        {
            throw new InvalidOperationException($"'{venvPath}' is not a valid Pilot virtual environment (missing {ConfigFileName}).");
        }

        System.Environment.SetEnvironmentVariable(EnvVarName, fullPath);
        System.Environment.SetEnvironmentVariable("VIRTUAL_ENV", fullPath);

        string scriptsDir = Path.Combine(fullPath, OperatingSystem.IsWindows() ? "Scripts" : "bin");
        string pathVar = System.Environment.GetEnvironmentVariable("PATH") ?? string.Empty;
        if (!pathVar.Contains(scriptsDir, StringComparison.OrdinalIgnoreCase))
        {
            System.Environment.SetEnvironmentVariable("PATH", scriptsDir + Path.PathSeparator + pathVar);
        }

        // Configure interop isolation paths
        string modulesDir = Path.Combine(fullPath, "lib", ".pilot_modules");
        System.Environment.SetEnvironmentVariable("PILOT_MODULES_PATH", modulesDir);
        System.Environment.SetEnvironmentVariable("NODE_PATH", Path.Combine(modulesDir, "npm"));
        System.Environment.SetEnvironmentVariable("PYTHONPATH", Path.Combine(modulesDir, "pip"));
    }

    /// <summary>
    /// Deactivates any active virtual environment in the current process.
    /// </summary>
    public static void Deactivate()
    {
        System.Environment.SetEnvironmentVariable(EnvVarName, null);
        System.Environment.SetEnvironmentVariable("VIRTUAL_ENV", null);
        System.Environment.SetEnvironmentVariable("PILOT_MODULES_PATH", null);
    }

    private static void GenerateActivationScripts(string venvPath, string scriptsDir, string envName)
    {
        // 1. PowerShell: activate.ps1
        string ps1Path = Path.Combine(scriptsDir, "activate.ps1");
        string ps1Content = $@"# Pilot Virtual Environment Activation Script (PowerShell)
$script:VENV_PATH = Resolve-Path ""$PSScriptRoot\..""
$env:{EnvVarName} = $script:VENV_PATH
$env:VIRTUAL_ENV = $script:VENV_PATH

# Store original values for deactivation
if (-not (Test-Path variable:global:_OLD_PILOT_PATH)) {{
    $global:_OLD_PILOT_PATH = $env:PATH
}}
if (-not (Test-Path variable:global:_OLD_PILOT_PROMPT)) {{
    $global:_OLD_PILOT_PROMPT = ""$function:prompt""
}}

$env:PATH = ""$PSScriptRoot;"" + $env:PATH

function global:deactivate {{
    if (Test-Path variable:global:_OLD_PILOT_PATH) {{
        $env:PATH = $global:_OLD_PILOT_PATH
        Remove-Variable -Scope global -Name _OLD_PILOT_PATH
    }}
    if (Test-Path variable:global:_OLD_PILOT_PROMPT) {{
        Set-Item -Path function:prompt -Value $global:_OLD_PILOT_PROMPT
        Remove-Variable -Scope global -Name _OLD_PILOT_PROMPT
    }}
    $env:{EnvVarName} = $null
    $env:VIRTUAL_ENV = $null
    Write-Host ""Deactivated Pilot virtual environment: {envName}"" -ForegroundColor Cyan
}}

function global:prompt {{
    Write-Host ""({envName}) "" -NoNewline -ForegroundColor Green
    return ""$($executionContext.SessionState.Path.CurrentLocation)$('>' * ($nestedPromptLevel + 1)) ""
}}

Write-Host ""Activated Pilot virtual environment: ({envName}) at $script:VENV_PATH"" -ForegroundColor Green
";
        File.WriteAllText(ps1Path, ps1Content);

        // 2. Windows Command Prompt: activate.bat
        string batPath = Path.Combine(scriptsDir, "activate.bat");
        string batContent = $@"@echo off
set ""{EnvVarName}=%~dp0..""
set ""VIRTUAL_ENV=%~dp0..""
set ""_OLD_PILOT_PATH=%PATH%""
set ""PATH=%~dp0;%PATH%""
if not defined _OLD_PILOT_PROMPT set ""_OLD_PILOT_PROMPT=%PROMPT%""
set ""PROMPT=({envName}) %PROMPT%""
echo Activated Pilot virtual environment: ({envName})
";
        File.WriteAllText(batPath, batContent);

        // deactivate.bat
        string deactBatPath = Path.Combine(scriptsDir, "deactivate.bat");
        string deactBatContent = $@"@echo off
if defined _OLD_PILOT_PATH (
    set ""PATH=%_OLD_PILOT_PATH%""
    set ""_OLD_PILOT_PATH=""
)
if defined _OLD_PILOT_PROMPT (
    set ""PROMPT=%_OLD_PILOT_PROMPT%""
    set ""_OLD_PILOT_PROMPT=""
)
set ""{EnvVarName}=""
set ""VIRTUAL_ENV=""
echo Deactivated Pilot virtual environment: {envName}
";
        File.WriteAllText(deactBatPath, deactBatContent);

        // 3. POSIX Bash/Zsh: activate
        string shPath = Path.Combine(scriptsDir, "activate");
        string shContent = $@"#!/usr/bin/env bash
# Pilot Virtual Environment Activation Script (POSIX)
deactivate () {{
    if [ -n ""${{_OLD_PILOT_PATH:-}}"" ] ; then
        PATH=""${{_OLD_PILOT_PATH}}""
        export PATH
        unset _OLD_PILOT_PATH
    fi
    if [ -n ""${{_OLD_PILOT_PS1:-}}"" ] ; then
        PS1=""${{_OLD_PILOT_PS1}}""
        export PS1
        unset _OLD_PILOT_PS1
    fi
    unset {EnvVarName}
    unset VIRTUAL_ENV
    if [ -n ""${{BASH_VERSION:-}}"" -o -n ""${{ZSH_VERSION:-}}"" ] ; then
        hash -r 2>/dev/null
    fi
    echo ""Deactivated Pilot virtual environment: {envName}""
}}

export {EnvVarName}=""$(cd ""$(dirname ""${{BASH_SOURCE[0]}})/.."" && pwd)""
export VIRTUAL_ENV=""${{{EnvVarName}}}""

_OLD_PILOT_PATH=""$PATH""
PATH=""$(dirname ""${{BASH_SOURCE[0]}}""):$PATH""
export PATH

if [ -z ""${{PILOT_VENV_DISABLE_PROMPT:-}}"" ] ; then
    _OLD_PILOT_PS1=""${{PS1:-}}""
    export PS1=""({envName}) ${{PS1:-}}""
fi

echo ""Activated Pilot virtual environment: ({envName}) at ${{{EnvVarName}}}""
";
        File.WriteAllText(shPath, shContent);
    }

    private static void GenerateLauncher(string venvPath, string scriptsDir)
    {
        // Generate a pilot.cmd launcher script in the venv Scripts directory
        string baseDir = AppContext.BaseDirectory;
        string pilotExe = Path.Combine(baseDir, "Pilot.exe");
        string pilotDll = Path.Combine(baseDir, "Pilot.dll");

        if (OperatingSystem.IsWindows())
        {
            string launcherCmd = Path.Combine(scriptsDir, "pilot.cmd");
            string launcherContent = File.Exists(pilotExe)
                ? $"@echo off\r\n\"{pilotExe}\" %*\r\n"
                : $"@echo off\r\ndotnet \"{pilotDll}\" %*\r\n";
            File.WriteAllText(launcherCmd, launcherContent);
        }
        else
        {
            string launcherSh = Path.Combine(scriptsDir, "pilot");
            string launcherContent = File.Exists(pilotExe)
                ? $"#!/bin/sh\nexec \"{pilotExe}\" \"$@\"\n"
                : $"#!/bin/sh\nexec dotnet \"{pilotDll}\" \"$@\"\n";
            File.WriteAllText(launcherSh, launcherContent);
        }
    }
}
