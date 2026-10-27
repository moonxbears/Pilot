#nullable disable
using System;
using System.Collections.Generic;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

public class PilotPolyglotExtendedTests
{
    private readonly PilotEngine _engine = new();

    [Fact]
    public void Test_Postgresql_Inline_Block_Query_And_Row_Iteration()
    {
        string code = @"
            let psqlVal = postgresql {
                select id, name, count(*)
                from mytable tb
                where tb.id in (1, 2, 3, 4, 5, 6);
            }

            let names = [];
            let ids = [];
            for val in psqlVal {
                names.append(val.name);
                ids.append(val.id);
            }
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var namesVal = result.Context.Environment.CurrentScope.Resolve("names");
        var idsVal = result.Context.Environment.CurrentScope.Resolve("ids");

        Assert.Equal(6, namesVal.AsList().Count);
        Assert.Equal(6, idsVal.AsList().Count);
        Assert.Equal("item_1", namesVal.AsList()[0].AsString());
        Assert.Equal("item_6", namesVal.AsList()[5].AsString());
        Assert.Equal(1, idsVal.AsList()[0].AsInt());
        Assert.Equal(6, idsVal.AsList()[5].AsInt());
    }

    [Fact]
    public void Test_Mysql_Inline_Block_Query()
    {
        string code = @"
            let myRows = mysql {
                select id, name from products where tb.id in (10, 20);
            }

            let firstId = myRows[0].id;
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var firstId = result.Context.Environment.CurrentScope.Resolve("firstId");
        Assert.Equal(10, firstId.AsInt());
    }

    [Fact]
    public void Test_Pilot_Variable_Injection_Into_Foreign_Block()
    {
        string code = @"
            let myPilotVariable: str = ""Hello, Python!"";
            var captured = """";

            python {
                captured = myPilotVariable;
            }
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var captured = result.Context.Environment.CurrentScope.Resolve("captured");
        Assert.Equal("Hello, Python!", captured.AsString());
    }

    [Fact]
    public void Test_Callable_Foreign_Block_With_Yields_Coroutine()
    {
        string code = @"
            let outputs = [];

            def recorder(val) {
                outputs.append(val);
                return val;
            }

            let myCustomFunc = python {
                recorder(""1"")
                yield recorder
                recorder(""3"")
                yield recorder(""4"")
            }

            myCustomFunc(""2"");
            myCustomFunc();
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var outputs = result.Context.Environment.CurrentScope.Resolve("outputs");
        var list = outputs.AsList();

        Assert.Equal(4, list.Count);
        Assert.Equal("1", list[0].AsString());
        Assert.Equal("2", list[1].AsString());
        Assert.Equal("3", list[2].AsString());
        Assert.Equal("4", list[3].AsString());
    }

    [Fact]
    public void Test_Javascript_Inline_Block()
    {
        string code = @"
            let jsResult = js {
                return 42;
            }
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var jsRes = result.Context.Environment.CurrentScope.Resolve("jsResult");
        Assert.Equal(42, jsRes.AsInt());
    }

    [Fact]
    public void Test_Lua_Inline_Block()
    {
        string code = @"
            let luaResult = lua {
                return 100;
            }
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var luaRes = result.Context.Environment.CurrentScope.Resolve("luaResult");
        Assert.Equal(100, luaRes.AsInt());
    }

    [Fact]
    public void Test_Java_Inline_Block()
    {
        string code = @"
            let javaResult = java {
                return 200;
            }
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var javaRes = result.Context.Environment.CurrentScope.Resolve("javaResult");
        Assert.Equal(200, javaRes.AsInt());
    }

    [Fact]
    public void Test_C_And_Cpp_Inline_Blocks()
    {
        string code = @"
            let cResult = c {
                return 300;
            }

            let cppResult = cpp {
                return 400;
            }
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var cRes = result.Context.Environment.CurrentScope.Resolve("cResult");
        var cppRes = result.Context.Environment.CurrentScope.Resolve("cppResult");

        Assert.Equal(300, cRes.AsInt());
        Assert.Equal(400, cppRes.AsInt());
    }

    [Fact]
    public void Test_Assembly_Inline_Block_Execution()
    {
        string code = @"
            let asmResult = asm {
                mov eax, 150;
                add eax, 350;
                ret;
            }
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var asmRes = result.Context.Environment.CurrentScope.Resolve("asmResult");
        Assert.Equal(500, asmRes.AsInt());
    }
}
