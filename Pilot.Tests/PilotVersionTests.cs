#nullable disable
using System;
using System.IO;
using System.Text.Json.Nodes;
using Pilot.Compiler;
using Pilot.Diagnostics;
using Pilot.LanguageServer.Lsp;
using Pilot.Runtime.Venv;
using Pilot.Scripting;
using Xunit;

namespace Pilot.Tests;

public class PilotVersionTests
{
    [Fact]
    public void Test_CommitBound_Version_Metadata_Is_Populated()
    {
        Assert.StartsWith("1.1.", PilotVersion.SemVer);
        Assert.True(PilotVersion.CommitCount > 0);
        Assert.Equal($"{PilotVersion.BaseVersion}.{PilotVersion.CommitCount}", PilotVersion.SemVer);
        Assert.False(string.IsNullOrWhiteSpace(PilotVersion.ShortCommitSha));
        Assert.Equal($"{PilotVersion.SemVer}+{PilotVersion.ShortCommitSha}", PilotVersion.FullVersion);
        Assert.Contains(PilotVersion.FullVersion, PilotVersion.DisplayBanner);
    }

    [Fact]
    public void Test_Version_Compatibility_Detects_Commit_Mismatch()
    {
        Assert.True(PilotVersion.IsExactMatch(PilotVersion.FullVersion));
        Assert.True(PilotVersion.IsExactMatch("v" + PilotVersion.FullVersion));
        Assert.True(PilotVersion.IsExactMatch(PilotVersion.SemVer));

        Assert.False(PilotVersion.IsExactMatch("1.1.0+0000000"));
        Assert.False(PilotVersion.IsExactMatch($"{PilotVersion.SemVer}+0000000"));

        bool ok = PilotVersion.CheckCompatibility("1.1.1+deadbee", "compiled binary 'stale.pilc'", out string mismatch);
        Assert.False(ok);
        Assert.NotNull(mismatch);
        Assert.Contains("1.1.1+deadbee", mismatch);
        Assert.Contains(PilotVersion.FullVersion, mismatch);
    }

    [Fact]
    public void Test_CompiledProgram_Pilc_Serializes_And_Verifies_CompilerVersion()
    {
        var comp = PilotCompiler.Default.Compile("let x = 42\n");
        Assert.True(comp.Success);
        var prog = Assert.IsType<PilotCompiledProgram>(comp.Program);
        Assert.Equal(PilotVersion.FullVersion, prog.CompilerVersion);
        Assert.True(prog.IsVersionMatched);

        byte[] bytes = prog.ToBytes();
        var loaded = PilotCompiledProgram.FromBytes(bytes, "test.pilc");
        Assert.Equal(PilotVersion.FullVersion, loaded.CompilerVersion);
        Assert.True(loaded.IsVersionMatched);
        Assert.True(loaded.CheckVersionCompatibility(out string noMismatch));
        Assert.Null(noMismatch);

        var mismatchedProg = new PilotCompiledProgram(
            loaded.Source,
            loaded.ProgramTree,
            "old.pilc",
            loaded.CompiledAt,
            loaded.DefinedSymbols,
            loaded.BytecodeChunk,
            compilerVersion: "1.1.1+0000000");

        Assert.False(mismatchedProg.IsVersionMatched);
        Assert.False(mismatchedProg.CheckVersionCompatibility(out string mismatchMsg));
        Assert.Contains("1.1.1+0000000", mismatchMsg);
        Assert.Contains(PilotVersion.FullVersion, mismatchMsg);
    }

    [Fact]
    public void Test_VirtualEnvironment_Records_And_Verifies_PilotVersion()
    {
        string tempDir = Path.Combine(Path.GetTempPath(), "pilot_venv_ver_" + Guid.NewGuid().ToString("N"));
        try
        {
            var cfg = PilotVirtualEnvironment.Create(tempDir);
            Assert.Equal(PilotVersion.SemVer, cfg.Version);
            Assert.Equal(PilotVersion.FullVersion, cfg.PilotVersion);
            Assert.True(cfg.IsVersionMatched);

            PilotVirtualEnvironment.Activate(tempDir);
            Assert.Null(PilotVirtualEnvironment.LastVersionMismatchWarning);
            PilotVirtualEnvironment.Deactivate();

            // Simulate a venv created by an older commit
            string cfgPath = Path.Combine(tempDir, PilotVirtualEnvironment.ConfigFileName);
            cfg.PilotVersion = "1.1.2+oldc0de";
            cfg.Save(cfgPath);

            PilotVirtualEnvironment.Activate(tempDir);
            Assert.NotNull(PilotVirtualEnvironment.LastVersionMismatchWarning);
            Assert.Contains("1.1.2+oldc0de", PilotVirtualEnvironment.LastVersionMismatchWarning);
        }
        finally
        {
            PilotVirtualEnvironment.Deactivate();
            if (Directory.Exists(tempDir))
            {
                try { Directory.Delete(tempDir, true); } catch { }
            }
        }
    }

    [Fact]
    public void Test_Builtin_PilotVersion_Functions_In_Script()
    {
        string code = @"
let full = pilot_version()
let semver = pilot_version(""semver"")
let sha = pilot_version(""sha"")
let count = pilot_version(""count"")
let info = pilot_version_info()
assert full == info[""version""]
assert semver == info[""semver""]
assert sha == info[""commit_sha""]
assert count == info[""commit_count""]
";
        var res = PilotScript.Run(code);
        Assert.True(res.Success, string.Join("; ", res.Diagnostics));
    }

    [Fact]
    public void Test_LspServer_Initialize_Returns_CommitBound_Version()
    {
        var server = new PilotLspServer();
        var responses = server.ProcessMessage(@"{""jsonrpc"":""2.0"",""id"":1,""method"":""initialize"",""params"":{}}");
        Assert.Single(responses);

        var root = JsonNode.Parse(responses[0]);
        var serverInfo = root?["result"]?["serverInfo"];
        Assert.NotNull(serverInfo);
        Assert.Equal(PilotVersion.FullVersion, serverInfo!["version"]?.GetValue<string>());
        Assert.Equal(PilotVersion.SemVer, serverInfo["semver"]?.GetValue<string>());
        Assert.Equal(PilotVersion.CommitCount, serverInfo["commitCount"]?.GetValue<int>());
        Assert.Equal(PilotVersion.ShortCommitSha, serverInfo["commitSha"]?.GetValue<string>());
    }
}
