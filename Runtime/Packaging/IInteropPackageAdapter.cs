using System;
using System.Collections.Generic;

namespace Pilot.Runtime.Packaging;

/// <summary>
/// Adapter contract enabling the Pilot package manager to adopt external package ecosystems
/// (NuGet, npm, pip, cargo, maven, luarocks) directly into Pilot projects.
/// </summary>
public interface IInteropPackageAdapter
{
    /// <summary>
    /// Target ecosystem identifier: nuget, npm, pip, cargo, maven, luarocks.
    /// </summary>
    string Ecosystem { get; }

    /// <summary>
    /// Installs a single package into the project's .pilot_modules directory.
    /// </summary>
    bool Install(string packageName, string version, string projectDirectory, Action<string>? log = null);

    /// <summary>
    /// Restores all declared dependencies for this ecosystem.
    /// </summary>
    bool Restore(IDictionary<string, string> dependencies, string projectDirectory, Action<string>? log = null);

    /// <summary>
    /// Resolves file system paths where the package's modules or binaries are located.
    /// </summary>
    IReadOnlyList<string> ResolveModulePaths(string packageName, string projectDirectory);
}
