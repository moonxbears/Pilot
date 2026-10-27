using System;
using System.Collections.Generic;
using Pilot.CodeGen.Engine;
using Pilot.Compiler;
using Pilot.Runtime.Values;
using Pilot.Scripting;
using Xunit;

namespace Pilot.Tests;

public class PilotSourceGeneratorTests
{
    [Fact]
    public void Test_FastGate_ZeroCost_WhenUnused()
    {
        string plainCode = @"
        let a = 10
        let b = 20
        let c = a + b
        ";

        Assert.False(PilotCodeGeneratorPipeline.ShouldRun(plainCode));

        string annotatedCode = @"
        @generate(""json"")
        class User {
            var id: int
            var name: str
        }
        ";

        Assert.True(PilotCodeGeneratorPipeline.ShouldRun(annotatedCode));
    }

    [Fact]
    public void Test_JsonGenerator_ProducesJsonSerializer()
    {
        string code = @"
        @generate(""json"")
        class User {
            var id: int = 42
            var name: str = ""Alice""
        }

        let u = User()
        let json_text = u.to_json()
        let map_data = u.to_map()
        ";

        var program = PilotScript.Compile(code);
        var result = program.Run();
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var jsonText = result.Context!.Environment.CurrentScope.Resolve("json_text").AsString();
        Assert.Contains("\"id\": 42", jsonText);
        Assert.Contains("\"name\": \"Alice\"", jsonText);

        var mapData = result.Context!.Environment.CurrentScope.Resolve("map_data");
        Assert.Equal(PilotValueKind.Map, mapData.Kind);
    }

    [Fact]
    public void Test_BuilderGenerator_ProducesFluentChaining()
    {
        string code = @"
        @generate(""builder"")
        class Config {
            var host: str = ""localhost""
            var port: int = 8080
            var timeout: float = 30.0
        }

        let cfg = Config().with_host(""api.example.com"").with_port(443).with_timeout(5.0)
        let final_host = cfg.host
        let final_port = cfg.port
        let final_timeout = cfg.timeout
        ";

        var program = PilotScript.Compile(code);
        var result = program.Run();
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        Assert.Equal("api.example.com", result.Context!.Environment.CurrentScope.Resolve("final_host").AsString());
        Assert.Equal(443, result.Context!.Environment.CurrentScope.Resolve("final_port").AsInt());
        Assert.Equal(5.0, result.Context!.Environment.CurrentScope.Resolve("final_timeout").AsFloat());
    }

    [Fact]
    public void Test_ReprGenerator_ProducesCleanString()
    {
        string code = @"
        @generate(""repr"")
        class Point {
            var x: int = 10
            var y: int = 20
        }

        let p = Point()
        let str_rep = p.to_string()
        ";

        var program = PilotScript.Compile(code);
        var result = program.Run();
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        string repr = result.Context!.Environment.CurrentScope.Resolve("str_rep").AsString();
        Assert.Equal("Point(x=10, y=20)", repr);
    }

    [Fact]
    public void Test_ObservableGenerator_NotifiesSubscribersOnMutation()
    {
        string code = @"
        @generate(""observable"")
        class Account {
            var balance: int = 100
        }

        let acc = Account()
        var observed_prop: str = """"
        var observed_old: int = 0
        var observed_new: int = 0

        def on_change(sender, prop, old_val, new_val) {
            observed_prop = prop
            observed_old = old_val
            observed_new = new_val
        }

        acc.subscribe(on_change)

        acc.set_balance(250)
        ";

        var program = PilotScript.Compile(code);
        var result = program.Run();
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        Assert.Equal(250, result.Context!.Environment.CurrentScope.Resolve("acc").AsObject().GetMember("balance").AsInt());
        Assert.Equal("balance", result.Context!.Environment.CurrentScope.Resolve("observed_prop").AsString());
        Assert.Equal(100, result.Context!.Environment.CurrentScope.Resolve("observed_old").AsInt());
        Assert.Equal(250, result.Context!.Environment.CurrentScope.Resolve("observed_new").AsInt());
    }

    [Fact]
    public void Test_EqualityGenerator_ProducesEqualsAndHash()
    {
        string code = @"
        @generate(""equality"")
        class Coord {
            var lat: int = 34
            var lon: int = -118
        }

        let c1 = Coord()
        let c2 = Coord()
        let eq_self = c1.equals(c2)
        let hash_val = c1.hash()
        ";

        var program = PilotScript.Compile(code);
        var result = program.Run();
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        Assert.True(result.Context!.Environment.CurrentScope.Resolve("eq_self").IsTrue);
        Assert.True(result.Context!.Environment.CurrentScope.Resolve("hash_val").Kind == PilotValueKind.Int);
    }

    [Fact]
    public void Test_MetadataGenerator_ExposesCustomMetaDictionary()
    {
        string code = @"
        @meta(author=""John"", version=""2.1"", audited=""true"")
        class SecureDocument {
            var title: str = ""Secret""
        }

        let doc = SecureDocument()
        let meta_map = doc.get_meta()
        let author = meta_map[""author""]
        let version = meta_map[""version""]
        ";

        var program = PilotScript.Compile(code);
        var result = program.Run();
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        Assert.Equal("John", result.Context!.Environment.CurrentScope.Resolve("author").AsString());
        Assert.Equal("2.1", result.Context!.Environment.CurrentScope.Resolve("version").AsString());
    }

    [Fact]
    public void Test_ScriptModel_SelfModification_And_CompanionScripts()
    {
        string code = @"
        let greeting = ""REPLACE_ME""
        ";

        var model = PilotCodeGeneratorPipeline.ExtractScriptModel(code);
        model.ReplaceCode("\"REPLACE_ME\"", "\"Hello from Generator!\"");
        model.AddScript("generated_utils.pilot", "def get_codegen_version() -> str { return \"v2.0\" }");

        var genResult = PilotCodeGeneratorPipeline.Run(model);

        Assert.Contains("Hello from Generator!", genResult.AugmentedSource);
        Assert.True(genResult.HasModifications);
    }
}
