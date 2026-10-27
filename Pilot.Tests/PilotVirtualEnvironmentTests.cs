#nullable disable
using System;
using System.IO;
using Pilot.Runtime.Packaging;
using Pilot.Runtime.Venv;
using Xunit;

namespace Pilot.Tests;

public class PilotVirtualEnvironmentTests : IDisposable
{
    private readonly string _tempDir;

    public PilotVirtualEnvironmentTests()
    {
        _tempDir = Path.Combine(Path.GetTempPath(), "pilot_venv_tests_" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(_tempDir);
    }

    public void Dispose()
    {
        PilotVirtualEnvironment.Deactivate();
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
    public void Test_Create_Generates_Complete_Venv_Structure()
    {
        string venvPath = Path.Combine(_tempDir, "myenv");
        var config = PilotVirtualEnvironment.Create(venvPath, new PilotVenvOptions
        {
            Prompt = "custom-env",
            IncludeSystemPackages = true,
            ExtraConfig = { ["custom_key"] = "custom_val" }
        });

        Assert.NotNull(config);
        Assert.Equal("custom-env", config.Prompt);
        Assert.True(config.IncludeSystemPackages);
        Assert.Equal("custom_val", config.Properties["custom_key"]);

        // Config file
        string cfgFile = Path.Combine(venvPath, PilotVirtualEnvironment.ConfigFileName);
        Assert.True(File.Exists(cfgFile));

        // Subdirectories
        Assert.True(Directory.Exists(Path.Combine(venvPath, "lib", ".pilot_modules")));
        Assert.True(Directory.Exists(Path.Combine(venvPath, "include")));

        string scriptsDir = Path.Combine(venvPath, OperatingSystem.IsWindows() ? "Scripts" : "bin");
        Assert.True(Directory.Exists(scriptsDir));

        // Activation scripts
        Assert.True(File.Exists(Path.Combine(scriptsDir, "activate.ps1")));
        Assert.True(File.Exists(Path.Combine(scriptsDir, "activate.bat")));
        Assert.True(File.Exists(Path.Combine(scriptsDir, "deactivate.bat")));
        Assert.True(File.Exists(Path.Combine(scriptsDir, "activate")));

        // Launcher wrapper
        if (OperatingSystem.IsWindows())
        {
            Assert.True(File.Exists(Path.Combine(scriptsDir, "pilot.cmd")));
        }
        else
        {
            Assert.True(File.Exists(Path.Combine(scriptsDir, "pilot")));
        }
    }

    [Fact]
    public void Test_Detect_Finds_Virtual_Environment_Hierarchy()
    {
        string venvPath = Path.Combine(_tempDir, "isolated_env");
        PilotVirtualEnvironment.Create(venvPath);

        // Direct detection
        string detected = PilotVirtualEnvironment.Detect(venvPath);
        Assert.NotNull(detected);
        Assert.Equal(Path.GetFullPath(venvPath), detected);

        // Subdirectory detection
        string childDir = Path.Combine(venvPath, "lib", ".pilot_modules");
        string detectedFromChild = PilotVirtualEnvironment.Detect(childDir);
        Assert.NotNull(detectedFromChild);
        Assert.Equal(Path.GetFullPath(venvPath), detectedFromChild);
    }

    [Fact]
    public void Test_Activate_And_Deactivate_Lifecycle()
    {
        string venvPath = Path.Combine(_tempDir, "test_env");
        PilotVirtualEnvironment.Create(venvPath);

        Assert.False(PilotVirtualEnvironment.IsActive);

        PilotVirtualEnvironment.Activate(venvPath);

        Assert.True(PilotVirtualEnvironment.IsActive);
        Assert.Equal(Path.GetFullPath(venvPath), PilotVirtualEnvironment.ActiveVenvPath);

        // Verify package manager routes modules to venv lib/.pilot_modules
        string resolvedModules = PilotPackageManager.GetEffectiveModulesDirectory(_tempDir);
        string expectedModules = Path.Combine(Path.GetFullPath(venvPath), "lib", ".pilot_modules");
        Assert.Equal(expectedModules, resolvedModules);

        PilotVirtualEnvironment.Deactivate();

        Assert.False(PilotVirtualEnvironment.IsActive);
        Assert.Null(PilotVirtualEnvironment.ActiveVenvPath);
    }
}
