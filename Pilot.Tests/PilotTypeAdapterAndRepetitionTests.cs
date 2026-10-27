#nullable disable
using System;
using System.Collections.Generic;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

public class PilotTypeAdapterAndRepetitionTests
{
    private readonly PilotEngine _engine = new();

    #region 1. Collection-to-Scalar Repeated Call (append example)

    [Fact]
    public void Test_Append_With_Each_Modifier()
    {
        string code = @"
            let myarr = [];
            let myitems : list = [1, 2, 3, 4, 5];

            myarr.append(each myitems);
            let finalCount = myarr.length;
            let firstItem = myarr[0];
            let lastItem = myarr[4];
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var scope = result.Context.Environment.CurrentScope;
        Assert.Equal(5, scope.Resolve("finalCount").AsInt());
        Assert.Equal(1, scope.Resolve("firstItem").AsInt());
        Assert.Equal(5, scope.Resolve("lastItem").AsInt());
    }

    [Fact]
    public void Test_Append_With_Spread_DotDot_Modifier()
    {
        string code = @"
            let myarr = [];
            let myitems : list = [10, 20, 30];

            myarr.append(..myitems);
            let len = myarr.length;
            let item1 = myarr[0];
            let item2 = myarr[1];
            let item3 = myarr[2];
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var scope = result.Context.Environment.CurrentScope;
        Assert.Equal(3, scope.Resolve("len").AsInt());
        Assert.Equal(10, scope.Resolve("item1").AsInt());
        Assert.Equal(20, scope.Resolve("item2").AsInt());
        Assert.Equal(30, scope.Resolve("item3").AsInt());
    }

    [Fact]
    public void Test_Append_With_Repeat_Modifier_And_Broadcast()
    {
        string code = @"
            let arr1 = [];
            let arr2 = [];
            let myitems = [1, 2, 3];

            arr1.append(repeat myitems);
            arr2.append*(myitems);

            let len1 = arr1.length;
            let len2 = arr2.length;
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var scope = result.Context.Environment.CurrentScope;
        Assert.Equal(3, scope.Resolve("len1").AsInt());
        Assert.Equal(3, scope.Resolve("len2").AsInt());
    }

    [Fact]
    public void Test_Append_Member_Each_And_Flag()
    {
        string code = @"
            let arr1 = [];
            let arr2 = [];
            let myitems = [1, 2, 3];

            arr1.append.each(myitems);
            arr2.append(myitems, true);

            let len1 = arr1.length;
            let len2 = arr2.length;
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var scope = result.Context.Environment.CurrentScope;
        Assert.Equal(3, scope.Resolve("len1").AsInt());
        Assert.Equal(3, scope.Resolve("len2").AsInt());
    }

    [Fact]
    public void Test_Scoped_Auto_Repeat_Context()
    {
        string code = @"
            let myarr = [];
            let myitems : list = [1, 2, 3, 4, 5];

            with auto_repeat {
                myarr.append(myitems);
            }

            let len = myarr.length;
            let item3 = myarr[2];
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var scope = result.Context.Environment.CurrentScope;
        Assert.Equal(5, scope.Resolve("len").AsInt());
        Assert.Equal(3, scope.Resolve("item3").AsInt());
    }

    #endregion

    #region 2. Function Vectorization & Repeated Return Values

    [Fact]
    public void Test_Function_Repeated_Call_Returns_Vector()
    {
        string code = @"
            def double_val(x: int) {
                return x * 2;
            }

            let nums = [1, 2, 3, 4];
            let doubledEach = double_val(each nums);
            let doubledSpread = double_val(..nums);
            let doubledBroadcast = double_val*(nums);
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var scope = result.Context.Environment.CurrentScope;
        var r1 = scope.Resolve("doubledEach").AsList();
        var r2 = scope.Resolve("doubledSpread").AsList();
        var r3 = scope.Resolve("doubledBroadcast").AsList();

        Assert.Equal(new long[] { 2, 4, 6, 8 }, r1.ConvertAll(v => v.AsInt()));
        Assert.Equal(new long[] { 2, 4, 6, 8 }, r2.ConvertAll(v => v.AsInt()));
        Assert.Equal(new long[] { 2, 4, 6, 8 }, r3.ConvertAll(v => v.AsInt()));
    }

    [Fact]
    public void Test_Parameter_Each_Declaration()
    {
        string code = @"
            def triple_items(each item: int) {
                return item * 3;
            }

            let res = triple_items([10, 20, 30]);
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var resList = result.Context.Environment.CurrentScope.Resolve("res").AsList();
        Assert.Equal(new long[] { 30, 60, 90 }, resList.ConvertAll(v => v.AsInt()));
    }

    [Fact]
    public void Test_Method_Group_Auto_Lifting_When_No_List_Overload()
    {
        string code = @"
            def transform(x: int) {
                return x + 100;
            }

            # Method group transform only has (int) overload, not (list)
            # Passing list automatically lifts and routes each item
            let res = transform([1, 2, 3]);
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var resList = result.Context.Environment.CurrentScope.Resolve("res").AsList();
        Assert.Equal(new long[] { 101, 102, 103 }, resList.ConvertAll(v => v.AsInt()));
    }

    #endregion

    #region 3. Argument Type Adapters & Conversions for Signatures

    [Fact]
    public void Test_Builtin_Signature_Type_Adapters()
    {
        string code = @"
            def calculate_tax(amount: int, rate: float) {
                return amount * rate;
            }

            # Pass string representations; adapters convert to int and float
            let tax = calculate_tax(""100"", ""0.15"");
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var taxVal = result.Context.Environment.CurrentScope.Resolve("tax");
        Assert.Equal(15.0, taxVal.AsFloat(), 3);
    }

    [Fact]
    public void Test_Structural_Map_To_Class_Adapter()
    {
        string code = @"
            class UserRecord {
                let name = """";
                let role = """";
            }

            def format_user(u: UserRecord) {
                return u.name + "" ("" + u.role + "")"";
            }

            # Pass dict/map; adapted into UserRecord instance
            let summary = format_user({ ""name"": ""Isaac"", ""role"": ""Architect"" });
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var summaryVal = result.Context.Environment.CurrentScope.Resolve("summary");
        Assert.Equal("Isaac (Architect)", summaryVal.AsString());
    }

    [Fact]
    public void Test_Language_Level_Adapter_Definition()
    {
        string code = @"
            class Meters {
                let value = 0;
            }

            adapter int -> Meters {
                let m = Meters();
                m.value = self;
                return m;
            }

            def describe_distance(m: Meters) {
                return str(m.value) + ""m"";
            }

            # Passing raw int adapts to Meters
            let dist = describe_distance(42);
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var distVal = result.Context.Environment.CurrentScope.Resolve("dist");
        Assert.Equal("42m", distVal.AsString());
    }

    [Fact]
    public void Test_Programmatic_TypeAdapter_Register()
    {
        string code = @"
            class Price {
                let amount = 0;
            }

            def convert_to_price(n) {
                let p = Price();
                p.amount = n;
                return p;
            }

            TypeAdapter.register(""int"", ""Price"", convert_to_price);

            def print_price(p: Price) {
                return ""$"" + str(p.amount);
            }

            let res = print_price(250);
        ";

        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var resVal = result.Context.Environment.CurrentScope.Resolve("res");
        Assert.Equal("$250", resVal.AsString());
    }

    #endregion
}
