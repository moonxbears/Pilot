#nullable disable
using System;
using System.IO;
using Pilot.Compiler;
using Pilot.Runtime.Values;
using Pilot.Scripting;
using Xunit;

namespace Pilot.Tests;

public class PilotCompilationAndScriptingTests
{
    [Fact]
    public void Test_Compile_Source_And_Execute_Multiple_Times()
    {
        string code = @"
            let x = 10;
            let y = 20;
            x + y * 2;
        ";

        var compileResult = PilotCompiler.Default.Compile(code);
        Assert.True(compileResult.Success, string.Join("; ", compileResult.Diagnostics));
        Assert.NotNull(compileResult.Program);

        var program = compileResult.Program;

        // Execute multiple times with identical results and no re-parsing
        for (int i = 0; i < 3; i++)
        {
            var runResult = program.Run();
            Assert.True(runResult.Success, string.Join("; ", runResult.Diagnostics));
            Assert.Equal(50, runResult.Value.AsInt());
        }
    }

    [Fact]
    public void Test_Compile_To_Pilc_Binary_And_Reload()
    {
        string code = @"
            def square(n) {
                return n * n;
            }
            square(12);
        ";

        var compileResult = PilotCompiler.Default.Compile(code);
        Assert.True(compileResult.Success);
        var program = compileResult.Program;

        // Serialize to bytes
        byte[] pilcBytes = program.ToBytes();
        Assert.NotEmpty(pilcBytes);

        // Deserialize and run
        var loadedProgram = PilotCompiler.Default.LoadCompiled(pilcBytes);
        Assert.NotNull(loadedProgram);

        var result = loadedProgram.Run();
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Equal(144, result.Value.AsInt());
    }

    [Fact]
    public void Test_Scripting_Args_And_Argv()
    {
        string code = @"
            let first = args[0];
            let second = argv[1];
            let count = args.length;
            first + ""_"" + second + "":"" + str(count);
        ";

        var result = PilotScript.Run(code, "alpha", "beta", "gamma");
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Equal("alpha_beta:3", result.Value.AsString());
    }

    [Fact]
    public void Test_Scripting_Environment_And_Cwd()
    {
        string code = @"
            let current_dir = cwd();
            let has_env = env.length > 0;
            let valid = current_dir.length > 0 && has_env;
            valid;
        ";

        var result = PilotScript.Run(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.True(result.Value.IsTrue);
    }

    [Fact]
    public void Test_Scripting_Exit_Code()
    {
        string code = @"
            let value = 100;
            if (value > 50) {
                exit(42);
            }
            0;
        ";

        var result = PilotScript.Run(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Equal(42, result.Value.AsInt());
    }

    [Fact]
    public void Test_Scripting_File_Utilities()
    {
        string tempFile = Path.Combine(Path.GetTempPath(), $"pilot_test_{Guid.NewGuid():N}.txt");
        try
        {
            string code = $@"
                let path = ""{tempFile.Replace("\\", "/")}"";
                let written = write_text(path, ""Pilot scripting engine rocks!"");
                let exists = file_exists(path);
                let content = read_text(path);
                written && exists && content == ""Pilot scripting engine rocks!"";
            ";

            var result = PilotScript.Run(code);
            Assert.True(result.Success, string.Join("; ", result.Diagnostics));
            Assert.True(result.Value.IsTrue);
        }
        finally
        {
            if (File.Exists(tempFile)) File.Delete(tempFile);
        }
    }

    [Fact]
    public void Test_Shebang_Ignored_Cleanly()
    {
        string code = @"#!/usr/bin/env pilot
            let a = 5;
            let b = 15;
            a + b;
        ";

        var result = PilotScript.Run(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
        Assert.Equal(20, result.Value.AsInt());
    }

    [Fact]
    public void Test_PilotScript_Evaluate_Generic()
    {
        int intVal = PilotScript.Evaluate<int>("10 + 20 * 2");
        Assert.Equal(50, intVal);

        string strVal = PilotScript.Evaluate<string>("'Hello ' + 'World'");
        Assert.Equal("Hello World", strVal);

        bool boolVal = PilotScript.Evaluate<bool>("100 > 50");
        Assert.True(boolVal);
    }

    [Fact]
    public void Test_Compiler_In_Memory_Cache()
    {
        string code = @"let answer = 42; answer;";

        var options = new PilotCompilerOptions { EnableCaching = true };
        var res1 = PilotCompiler.Default.Compile(code, options);
        var res2 = PilotCompiler.Default.Compile(code, options);

        Assert.True(res1.Success);
        Assert.True(res2.Success);
        Assert.Same(res1.Program, res2.Program);
    }
}
