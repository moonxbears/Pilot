using System;
using System.IO;
using System.Text;
using Pilot.Cli;
using Xunit;

namespace Pilot.Tests;

public class PilotReplTests
{
    private (int Result, string Output) RunReplWithInput(string input)
    {
        var originalIn = Console.In;
        var originalOut = Console.Out;
        var originalError = Console.Error;

        try
        {
            using var sw = new StringWriter();
            using var sr = new StringReader(input);

            Console.SetIn(sr);
            Console.SetOut(sw);
            Console.SetError(sw);

            var repl = new PilotRepl();
            int result = repl.Run();

            return (result, sw.ToString());
        }
        finally
        {
            Console.SetIn(originalIn);
            Console.SetOut(originalOut);
            Console.SetError(originalError);
        }
    }

    [Fact]
    public void Run_ExitCommand_ExitsSuccessfully()
    {
        var (result, output) = RunReplWithInput("exit\n");

        Assert.Equal(0, result);
        Assert.Contains("Goodbye!", output);
    }

    [Fact]
    public void Run_HelpCommand_PrintsHelpMenu()
    {
        var (result, output) = RunReplWithInput(":help\nexit\n");

        Assert.Equal(0, result);
        Assert.Contains("Available REPL commands:", output);
        Assert.Contains(":help", output);
        Assert.Contains(":scope", output);
        Assert.Contains(":reset", output);
        Assert.Contains(":exit", output);
    }

    [Fact]
    public void Run_ScopeCommand_ListsVariables()
    {
        var (result, output) = RunReplWithInput("let test_var = 42;\n:scope\nexit\n");

        Assert.Equal(0, result);
        Assert.Contains("Variables in active REPL scope:", output);
        Assert.Contains("test_var", output);
        Assert.Contains("42", output);
    }

    [Fact]
    public void Run_ExpressionEvaluation_PrintsResult()
    {
        var (result, output) = RunReplWithInput("10 + 32\nexit\n");

        Assert.Equal(0, result);
        Assert.Contains("=> 42", output);
    }

    [Fact]
    public void Run_MultiLineInput_EvaluatesCorrectly()
    {
        var input = @"def add(a, b) {
    return a + b;
}
add(10, 20)
exit
";
        var (result, output) = RunReplWithInput(input);

        Assert.Equal(0, result);
        Assert.Contains("=> 30", output);
    }

    [Fact]
    public void Run_ResetCommand_ClearsScope()
    {
        var input = @"let my_val = 99;
:reset
:scope
exit
";
        var (result, output) = RunReplWithInput(input);

        Assert.Equal(0, result);
        Assert.Contains("REPL session reset.", output);
        Assert.DoesNotContain("my_val", output.Substring(output.IndexOf("REPL session reset.")));
    }
}
