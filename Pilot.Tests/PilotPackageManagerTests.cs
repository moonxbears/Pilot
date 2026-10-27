#nullable disable
using System;
using System.IO;
using Pilot.Runtime.Packaging;
using Xunit;

namespace Pilot.Tests;

public class PilotPackageManagerTests : IDisposable
{
    private readonly string _tempDir;
    private readonly PilotPackageManager _pm = new();

    public PilotPackageManagerTests()
    {
        _tempDir = Path.Combine(Path.GetTempPath(), "pilot_pkg_tests_" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(_tempDir);
    }

    public void Dispose()
    {
        try
        {
            if (Directory.Exists(_tempDir))
            {
                Directory.Delete(_tempDir, true);
            }
        }
        catch { }
    }

    [Fact]
    public void Test_Init_Creates_Manifest_And_Main()
    {
        var manifest = _pm.Init(_tempDir, "test-service");
        Assert.NotNull(manifest);
        Assert.Equal("test-service", manifest.Name);
        Assert.Equal("0.1.0", manifest.Version);
        Assert.True(File.Exists(Path.Combine(_tempDir, "pilot.json")));
        Assert.True(File.Exists(Path.Combine(_tempDir, "main.pilot")));
    }

    [Fact]
    public void Test_Add_Native_Pilot_Dependency()
    {
        _pm.Init(_tempDir, "app");
        bool ok = _pm.Add(_tempDir, "pilot-http@2.1.0");
        Assert.True(ok);

        var manifest = PilotPackageManifest.Load(Path.Combine(_tempDir, "pilot.json"));
        Assert.True(manifest.Dependencies.ContainsKey("pilot-http"));
        Assert.Equal("2.1.0", manifest.Dependencies["pilot-http"]);

        // Verify lockfile was generated
        string lockPath = Path.Combine(_tempDir, "pilot.lock.json");
        Assert.True(File.Exists(lockPath));
        var lockFile = PilotPackageLock.Load(lockPath);
        Assert.True(lockFile.Packages.ContainsKey("pilot:pilot-http"));
    }

    [Fact]
    public void Test_Add_Adopted_Interop_Dependencies()
    {
        _pm.Init(_tempDir, "polyglot-app");

        // Adopt packages from NuGet, npm, pip, and cargo
        Assert.True(_pm.Add(_tempDir, "nuget:Newtonsoft.Json@13.0.3"));
        Assert.True(_pm.Add(_tempDir, "npm:lodash@4.17.21"));
        Assert.True(_pm.Add(_tempDir, "pip:requests@2.31.0"));
        Assert.True(_pm.Add(_tempDir, "cargo:serde@1.0"));

        var manifest = PilotPackageManifest.Load(Path.Combine(_tempDir, "pilot.json"));

        Assert.Equal("13.0.3", manifest.InteropDependencies["nuget"]["Newtonsoft.Json"]);
        Assert.Equal("4.17.21", manifest.InteropDependencies["npm"]["lodash"]);
        Assert.Equal("2.31.0", manifest.InteropDependencies["pip"]["requests"]);
        Assert.Equal("1.0", manifest.InteropDependencies["cargo"]["serde"]);

        // Verify lockfile reflects all adopted ecosystems
        string lockPath = Path.Combine(_tempDir, "pilot.lock.json");
        Assert.True(File.Exists(lockPath));
        var lockFile = PilotPackageLock.Load(lockPath);

        Assert.True(lockFile.Packages.ContainsKey("nuget:Newtonsoft.Json"));
        Assert.True(lockFile.Packages.ContainsKey("npm:lodash"));
        Assert.True(lockFile.Packages.ContainsKey("pip:requests"));
        Assert.True(lockFile.Packages.ContainsKey("cargo:serde"));
    }

    [Fact]
    public void Test_Remove_Dependency()
    {
        _pm.Init(_tempDir, "demo");
        _pm.Add(_tempDir, "my-lib@1.0.0");
        _pm.Add(_tempDir, "npm:chalk@5.0.0");

        Assert.True(_pm.Remove(_tempDir, "my-lib"));
        Assert.True(_pm.Remove(_tempDir, "npm:chalk"));

        var manifest = PilotPackageManifest.Load(Path.Combine(_tempDir, "pilot.json"));
        Assert.False(manifest.Dependencies.ContainsKey("my-lib"));
        Assert.False(manifest.InteropDependencies["npm"].ContainsKey("chalk"));
    }

    [Fact]
    public void Test_Pack_Generates_PilPkg_Archive()
    {
        _pm.Init(_tempDir, "dist-module");
        File.WriteAllText(Path.Combine(_tempDir, "math.pilot"), "def add(a, b) { return a + b; }");

        string archivePath = _pm.Pack(_tempDir);
        Assert.True(File.Exists(archivePath));
        Assert.EndsWith(".pilpkg", archivePath);
        Assert.True(new FileInfo(archivePath).Length > 0);
    }
}
