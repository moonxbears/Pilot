#nullable disable
using System;
using System.Collections.Generic;
using Pilot.Cli;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

public class PilotAdvancedFeaturesTests
{
    private readonly PilotEngine _engine = new();

    [Fact]
    public void Test_Partial_Classes_Merge_Methods()
    {
        string code = @"
            partial class Calculator {
                def add(self, a, b) {
                    return a + b;
                }
            }

            partial class Calculator {
                def multiply(self, a, b) {
                    return a * b;
                }
            }

            let c = Calculator();
            let sum = c.add(10, 20);
            let prod = c.multiply(5, 6);
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var sumVal = result.Context.Environment.CurrentScope.Resolve("sum");
        var prodVal = result.Context.Environment.CurrentScope.Resolve("prod");

        Assert.Equal(30, sumVal.AsInt());
        Assert.Equal(30, prodVal.AsInt());
    }

    [Fact]
    public void Test_Generators_Yield_Next_And_For()
    {
        string code = @"
            def count_to(limit) {
                var i = 1;
                while i <= limit {
                    yield i;
                    i = i + 1;
                }
            }

            let g = count_to(3);
            let a = g.next();
            let b = g.next();
            let c = g.next();
            let d = g.next();
            let is_done = g.is_done;
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var a = result.Context.Environment.CurrentScope.Resolve("a");
        var b = result.Context.Environment.CurrentScope.Resolve("b");
        var c = result.Context.Environment.CurrentScope.Resolve("c");
        var d = result.Context.Environment.CurrentScope.Resolve("d");
        var isDone = result.Context.Environment.CurrentScope.Resolve("is_done");

        Assert.Equal(1, a.AsInt());
        Assert.Equal(2, b.AsInt());
        Assert.Equal(3, c.AsInt());
        Assert.True(d.IsUndefined);
        Assert.True(isDone.IsTrue);
    }

    [Fact]
    public void Test_Generators_In_For_Loop()
    {
        string code = @"
            def fib_gen(n) {
                var a = 0;
                var b = 1;
                var count = 0;
                while count < n {
                    yield a;
                    let next = a + b;
                    a = b;
                    b = next;
                    count = count + 1;
                }
            }

            let collected = [];
            for num in fib_gen(5) {
                collected.append(num);
            }
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var collected = result.Context.Environment.CurrentScope.Resolve("collected");
        var list = collected.AsList();
        Assert.Equal(5, list.Count);
        Assert.Equal(0, list[0].AsInt());
        Assert.Equal(1, list[1].AsInt());
        Assert.Equal(1, list[2].AsInt());
        Assert.Equal(2, list[3].AsInt());
        Assert.Equal(3, list[4].AsInt());
    }

    [Fact]
    public void Test_Multiprocessing_Spawn_And_ParallelMap()
    {
        string code = @"
            def double_num(x) {
                return x * 2;
            }

            let future = spawn double_num(21);
            let awaited = await future;

            let nums = [1, 2, 3, 4, 5];
            let mapped = parallel_map(double_num, nums);
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var awaited = result.Context.Environment.CurrentScope.Resolve("awaited");
        Assert.Equal(42, awaited.AsInt());

        var mapped = result.Context.Environment.CurrentScope.Resolve("mapped");
        var list = mapped.AsList();
        Assert.Equal(5, list.Count);
        Assert.Equal(2, list[0].AsInt());
        Assert.Equal(4, list[1].AsInt());
        Assert.Equal(6, list[2].AsInt());
        Assert.Equal(8, list[3].AsInt());
        Assert.Equal(10, list[4].AsInt());
    }

    [Fact]
    public void Test_Lazy_Evaluation()
    {
        string code = @"
            var eval_count = 0;
            def compute() {
                eval_count = eval_count + 1;
                return 42 * 2;
            }

            let lazy_val = lazy(compute);
            let count_before = eval_count;
            let first_eval = lazy_val.value;
            let second_eval = lazy_val.value;
            let count_after = eval_count;
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var countBefore = result.Context.Environment.CurrentScope.Resolve("count_before");
        var firstEval = result.Context.Environment.CurrentScope.Resolve("first_eval");
        var secondEval = result.Context.Environment.CurrentScope.Resolve("second_eval");
        var countAfter = result.Context.Environment.CurrentScope.Resolve("count_after");

        Assert.Equal(0, countBefore.AsInt());
        Assert.Equal(84, firstEval.AsInt());
        Assert.Equal(84, secondEval.AsInt());
        Assert.Equal(1, countAfter.AsInt());
    }

    [Fact]
    public void Test_Json_Serialization_And_Deserialization()
    {
        string code = @"
            let original = { ""title"": ""Pilot Language"", ""stars"": 999 };
            let serialized = json.dumps(original);
            let restored = json.loads(serialized);
            let title = restored[""title""];
            let stars = restored[""stars""];
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var title = result.Context.Environment.CurrentScope.Resolve("title");
        var stars = result.Context.Environment.CurrentScope.Resolve("stars");

        Assert.Equal("Pilot Language", title.AsString());
        Assert.Equal(999, stars.AsInt());
    }

    [Fact]
    public void Test_Json_Object_Root_Method()
    {
        string code = @"
            class Product {
                def __init__(self, name, price) {
                    self.name = name;
                    self.price = price;
                }
            }

            let prod = Product(""Widget"", 19);
            let json_str = prod.to_json();
            let parsed = json.loads(json_str);
            let p_name = parsed[""name""];
            let p_price = parsed[""price""];
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var pName = result.Context.Environment.CurrentScope.Resolve("p_name");
        var pPrice = result.Context.Environment.CurrentScope.Resolve("p_price");

        Assert.Equal("Widget", pName.AsString());
        Assert.Equal(19, pPrice.AsInt());
    }

    [Fact]
    public void Test_Collection_Expressions_Look_Spread_Inline()
    {
        string code = @"
            let a = [2, 3];
            let b = [5, 6];
            let combined = [1, ..a, 4, ..b, 7];

            def numbers_gen() {
                yield 100;
                yield 200;
            }
            let from_gen = [0, ..numbers_gen(), 300];

            let m1 = { ""a"": 1, ""b"": 2 };
            let m2 = { ..m1, ""c"": 3 };
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var combined = result.Context!.Environment.CurrentScope.Resolve("combined").AsList();
        Assert.Equal(7, combined.Count);
        Assert.Equal(new[] { 1, 2, 3, 4, 5, 6, 7 }, combined.ConvertAll(v => (int)v.AsInt()));

        var fromGen = result.Context.Environment.CurrentScope.Resolve("from_gen").AsList();
        Assert.Equal(4, fromGen.Count);
        Assert.Equal(new[] { 0, 100, 200, 300 }, fromGen.ConvertAll(v => (int)v.AsInt()));

        var m2 = result.Context.Environment.CurrentScope.Resolve("m2").AsMap();
        Assert.Equal(3, m2.Count);
        Assert.Equal(1, m2[PilotValue.FromString("a")].AsInt());
        Assert.Equal(2, m2[PilotValue.FromString("b")].AsInt());
        Assert.Equal(3, m2[PilotValue.FromString("c")].AsInt());
    }

    [Fact]
    public void Test_REPL_Can_Instantiate()
    {
        var repl = new PilotRepl();
        Assert.NotNull(repl);
    }
}
