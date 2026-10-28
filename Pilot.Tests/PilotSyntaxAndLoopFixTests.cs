#nullable disable
using System;
using Pilot.Scripting;
using Pilot.Compiler;
using Xunit;

namespace Pilot.Tests;

/// <summary>
/// Unit tests verifying for loop complete iteration and Pythonic colon-indentation syntax.
/// </summary>
public class PilotSyntaxAndLoopFixTests
{
    [Fact]
    public void Test_ForLoop_Over_Range_Iterates_All_Items()
    {
        string code = @"
var sum = 0
for x in 15..20:
    sum += x
";
        var res = PilotScript.Run(code);
        Assert.True(res.Success);
        Assert.NotNull(res.Context);
        // 15 + 16 + 17 + 18 + 19 + 20 = 105 (inclusive ..)
        Assert.Equal(105, res.Context.Environment.CurrentScope.Resolve("sum").AsInt());
    }

    [Fact]
    public void Test_ForLoop_Over_List_Iterates_All_Items()
    {
        string code = @"
var sum = 0
for j in [1, 2, 3, 4, 5]{
    sum += j
}
";
        var res = PilotScript.Run(code);
        Assert.True(res.Success);
        Assert.NotNull(res.Context);
        Assert.Equal(15, res.Context.Environment.CurrentScope.Resolve("sum").AsInt());
    }

    [Fact]
    public void Test_Pythonic_While_Colon_With_Nested_If()
    {
        string code = @"
var myvar : int = 0
while (true):    
    if (myvar >= 10){
        break;
    }
    myvar += 1
";
        var res = PilotScript.Run(code);
        Assert.True(res.Success);
        Assert.NotNull(res.Context);
        Assert.Equal(10, res.Context.Environment.CurrentScope.Resolve("myvar").AsInt());
    }

    [Fact]
    public void Test_Pythonic_Colon_Nested_If_Colon()
    {
        string code = @"
var myvar : int = 0
while (true):    
    if (myvar >= 5):
        break
    myvar += 1
";
        var res = PilotScript.Run(code);
        Assert.True(res.Success);
        Assert.NotNull(res.Context);
        Assert.Equal(5, res.Context.Environment.CurrentScope.Resolve("myvar").AsInt());
    }

    [Fact]
    public void Test_User_Script_Execution()
    {
        string code = @"
var count = 0
for x in 15..20:
    count += 1

var listCount = 0
for j in [1, 2, 3, 4, 5]{
    listCount += 1
}

var myvar : int = 0
while (true):    
    if (myvar >= 20){
        break;
    }
    myvar += 1

while (true):    
    if (myvar >= 20){
        break;
    }
    myvar += 1
";
        var res = PilotScript.Run(code);
        Assert.True(res.Success);
        Assert.NotNull(res.Context);
        Assert.Equal(6, res.Context.Environment.CurrentScope.Resolve("count").AsInt());
        Assert.Equal(5, res.Context.Environment.CurrentScope.Resolve("listCount").AsInt());
        Assert.Equal(20, res.Context.Environment.CurrentScope.Resolve("myvar").AsInt());
    }
}
