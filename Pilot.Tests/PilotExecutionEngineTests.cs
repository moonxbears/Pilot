using System;
using System.IO;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

public class PilotExecutionEngineTests
{
    private readonly PilotEngine _engine = new();

    [Fact]
    public void Test_Arithmetic_And_Expressions_Evaluation()
    {
        string code = @"
            let a = 10 + 20 * 2;
            let b = a / 5;
            let c = ""Hello, "" + ""Pilot!"";
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var val = _engine.EvaluateExpression("10 + 5 * 2");
        Assert.Equal(20, val.AsInt());

        var strVal = _engine.EvaluateExpression(@"""Pilot"" + "" "" + ""Rocks""");
        Assert.Equal("Pilot Rocks", strVal.AsString());
    }

    [Fact]
    public void Test_Function_Declaration_And_Recursion()
    {
        string code = @"
            def factorial(n) {
                if n <= 1 {
                    return 1;
                }
                return n * factorial(n - 1);
            }

            let result = factorial(5);
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var res = _engine.Execute(@"
            def fib(n) {
                if n <= 0 { return 0; }
                if n == 1 { return 1; }
                return fib(n - 1) + fib(n - 2);
            }
            let res = fib(7);
        ");
        Assert.True(res.Success);
    }

    [Fact]
    public void Test_Guard_And_Defer_Execution()
    {
        string code = @"
            let logs = [];
            
            def process(val) {
                defer {
                    logs.append(""defer:cleanup"");
                }

                guard val > 0 else {
                    logs.append(""guard:failed"");
                    return -1;
                }

                logs.append(""processed"");
                return val * 2;
            }

            let r1 = process(-5);
            let r2 = process(10);
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
    }

    [Fact]
    public void Test_Cascade_Chaining_And_Return_Interruption()
    {
        string code = @"
            class Builder {
                def __init__(self) {
                    self.items = [];
                    self.last_added = """";
                }

                def add(self, item) {
                    self.items.append(item);
                    self.last_added = item;
                    return item;
                }

                def count(self) {
                    return len(self.items);
                }
            }

            let b = Builder();
            b..add(""first"")
             ..let captured = add(""second"")
             ..add(""third"");
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
    }

    [Fact]
    public void Test_Pattern_Matching_Collection_Slices()
    {
        string code = @"
            let items = [10, 20, 30, 40, 50];
            var category = """";

            match items {
                case [first, ..mid, last] => {
                    category = ""sliced"";
                }
                case _ => {
                    category = ""other"";
                }
            }
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
    }

    [Fact]
    public void Test_Pattern_Matching_Relational_And_Type_Guards()
    {
        string code = @"
            let score = 85;
            var grade = """";

            match score {
                case >= 90 => { grade = ""A""; }
                case >= 80 => { grade = ""B""; }
                case >= 70 => { grade = ""C""; }
                case _     => { grade = ""F""; }
            }
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
    }

    [Fact]
    public void Test_Root_Object_Methods_On_All_Values()
    {
        string code = @"
            let num = 42;
            let str_val = num.to_string();
            let type_str = num.type_of();
            let hash_val = num.hash();

            let list_val = [1, 2, 3];
            let list_clone = list_val.clone();
            let list_type = list_val.type_of();
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
    }

    [Fact]
    public void Test_Matrix_Runtime_Operations()
    {
        var m1 = PilotMatrix.Identity(2);
        var m2 = PilotMatrix.Identity(2);
        var sum = m1.Add(m2);

        Assert.Equal(2.0, sum[0, 0]);
        Assert.Equal(2.0, sum[1, 1]);

        string code = @"
            let m = eye(3);
            let s = zeros(2, 2);
            let o = ones(2, 2);
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
    }

    [Fact]
    public void Test_Polyglot_Interop_Execution()
    {
        string code = @"
            let py_res = python {
                import math
                x = math.sin(3.14)
            };

            let cs_res = csharp {
                var g = Guid.NewGuid();
            };

            let rs_res = rust {
                let v = vec![1, 2, 3];
            };
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
    }

    [Fact]
    public void Test_Lisp_Metaprogramming_Builtins()
    {
        string code = @"
            let s1 = gensym(""tmp"");
            let s2 = gensym(""tmp"");
            let is_diff = s1 != s2;

            let my_list = cons(1, cons(2, []));
            let h = car(my_list);
            let t = cdr(my_list);
            let is_at = atom(42);
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
    }

    [Fact]
    public void Test_Execution_Options_Output_Redirection()
    {
        using var sw = new StringWriter();
        var options = new PilotExecutionOptions().WithStandardOutput(sw);

        string code = @"
            println(""Pilot Standard Output Test"");
            print(""Line without newline"");
        ";
        var result = _engine.Execute(code, options);
        Assert.True(result.Success);
        Assert.Contains("Pilot Standard Output Test", sw.ToString());
        Assert.Contains("Line without newline", sw.ToString());
    }

    [Fact]
    public void Test_Custom_Pluggable_Builtin_Function()
    {
        var options = new PilotExecutionOptions();
        options.Builtins.Register("double_it", (_, args) =>
        {
            long n = args.Count > 0 ? args[0].AsInt() : 0;
            return PilotValue.FromInt(n * 2);
        }, "n");

        string code = @"
            let val = double_it(21);
            assert(val == 42, ""Custom builtin failed"");
        ";
        var result = _engine.Execute(code, options);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));
    }
}
