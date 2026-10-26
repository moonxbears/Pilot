#nullable disable
using System;
using System.Collections.Generic;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

public class PilotMethodGroupTests
{
    private readonly PilotEngine _engine = new();

    [Fact]
    public void Test_Switchboard_AutoRouting_Heterogeneous_Data()
    {
        string code = @"
            def process(data: int) {
                return data * 2;
            }

            def process(data: str) {
                return data.to_upper();
            }

            def process(data: list) {
                return data.length;
            }

            let mixed_data = [42, ""hello"", [1, 2, 3], 99];
            let results = mixed_data.map(process);
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var resultsVal = result.Context.Environment.CurrentScope.Resolve("results");
        Assert.Equal(PilotValueKind.List, resultsVal.Kind);
        var list = resultsVal.AsList();
        Assert.Equal(4, list.Count);
        Assert.Equal(84, list[0].AsInt());
        Assert.Equal("HELLO", list[1].AsString());
        Assert.Equal(3, list[2].AsInt());
        Assert.Equal(198, list[3].AsInt());
    }

    [Fact]
    public void Test_Method_Group_Currying_Signature_Sieve()
    {
        string code = @"
            class Circle {
                let radius = 0.0;
            }

            class Square {
                let side = 0.0;
            }

            class Triangle {
                let angle = 0.0;
            }

            class Color {
                let RED = ""red"";
                let BLUE = ""blue"";
            }

            def draw(shape: Circle, color: str) {
                return ""Circle in "" + color;
            }

            def draw(shape: Square, color: str) {
                return ""Square in "" + color;
            }

            def draw(shape: Triangle, angle: float) {
                return ""Triangle at angle"";
            }

            # Providing only color filters out Triangle (Signature Sieve)
            let draw_red = draw(color=Color.RED);

            let c = Circle();
            let s = Square();
            let res_circle = draw_red(c);
            let res_square = draw_red(s);
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var resCircle = result.Context.Environment.CurrentScope.Resolve("res_circle");
        var resSquare = result.Context.Environment.CurrentScope.Resolve("res_square");

        Assert.Equal("Circle in red", resCircle.AsString());
        Assert.Equal("Square in red", resSquare.AsString());

        // Triangle should fail with No matching overload in group
        string invalidCallCode = @"
            let t = Triangle();
            let res_tri = draw_red(t);
        ";
        var invalidResult = _engine.Execute(invalidCallCode, result.Context.Environment.CurrentScope);
        Assert.False(invalidResult.Success);
        Assert.Contains("No matching overload", string.Join(" ", invalidResult.Diagnostics));
    }

    [Fact]
    public void Test_Method_Group_Union_And_Fallback()
    {
        string code = @"
            class User {
                let name = """";
            }

            class Document {
                let title = """";
            }

            class Json {
                def serialize(data: User) {
                    return ""JSON:"" + data.name;
                }
            }

            class Xml {
                def serialize(data: Document) {
                    return ""XML:"" + data.title;
                }
            }

            # Merge method groups using bitwise OR (|)
            let serialize_any = Json.serialize | Xml.serialize;

            let u = User();
            u.name = ""Isaac"";

            let d = Document();
            d.title = ""Manifesto"";

            let jsonRes = serialize_any(u);
            let xmlRes = serialize_any(d);
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var jsonRes = result.Context.Environment.CurrentScope.Resolve("jsonRes");
        var xmlRes = result.Context.Environment.CurrentScope.Resolve("xmlRes");

        Assert.Equal("JSON:Isaac", jsonRes.AsString());
        Assert.Equal("XML:Manifesto", xmlRes.AsString());
    }

    [Fact]
    public void Test_CompileTime_AST_Overload_Introspection()
    {
        string code = @"
            def transform(x: int, factor: float) {
                return x * factor;
            }

            def transform(text: str) {
                return text;
            }

            let overloads_list = transform.overloads;
            let group_name = transform.name;
            let count = overloads_list.length;

            let first_sig = overloads_list[0].signature;
            let first_params = overloads_list[0].parameters;
            let first_param_name = first_params[0][""name""];
            let first_param_type = first_params[0][""type""];

            let second_sig = overloads_list[1].signature;
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var groupName = result.Context.Environment.CurrentScope.Resolve("group_name");
        var count = result.Context.Environment.CurrentScope.Resolve("count");
        var firstSig = result.Context.Environment.CurrentScope.Resolve("first_sig");
        var firstParamName = result.Context.Environment.CurrentScope.Resolve("first_param_name");
        var firstParamType = result.Context.Environment.CurrentScope.Resolve("first_param_type");
        var secondSig = result.Context.Environment.CurrentScope.Resolve("second_sig");

        Assert.Equal("transform", groupName.AsString());
        Assert.Equal(2, count.AsInt());
        Assert.Equal("(x: int, factor: float) -> any", firstSig.AsString());
        Assert.Equal("x", firstParamName.AsString());
        Assert.Equal("int", firstParamType.AsString());
        Assert.Equal("(text: str) -> any", secondSig.AsString());
    }

    [Fact]
    public void Test_MultiCast_Broadcast_Execution()
    {
        string code = @"
            class EntitySpawned {
                let id = 101;
            }

            class PlayerJoined(EntitySpawned) {
                let name = ""Isaac"";
            }

            let events_fired = [];

            def on_event(e: PlayerJoined) {
                events_fired.append(""player:"" + e.name);
                return ""player_done"";
            }

            def on_event(e: EntitySpawned) {
                events_fired.append(""entity:"" + e.id);
                return ""entity_done"";
            }

            let player = PlayerJoined();
            # Broadcast operator (*) invokes BOTH matching overloads in hierarchy
            let broadcast_results = on_event*(player);
            let count = broadcast_results.length;
            let log_count = events_fired.length;
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var count = result.Context.Environment.CurrentScope.Resolve("count");
        var logCount = result.Context.Environment.CurrentScope.Resolve("log_count");
        var broadcastResults = result.Context.Environment.CurrentScope.Resolve("broadcast_results");
        var eventsFired = result.Context.Environment.CurrentScope.Resolve("events_fired");

        Assert.Equal(2, count.AsInt());
        Assert.Equal(2, logCount.AsInt());

        var resList = broadcastResults.AsList();
        Assert.Equal("player_done", resList[0].AsString());
        Assert.Equal("entity_done", resList[1].AsString());

        var eventList = eventsFired.AsList();
        Assert.Equal("player:Isaac", eventList[0].AsString());
        Assert.Equal("entity:101", eventList[1].AsString());
    }
}
