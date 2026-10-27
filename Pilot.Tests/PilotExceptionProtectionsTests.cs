#nullable disable
using System;
using System.Collections.Generic;
using Pilot.Runtime.ControlFlow;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

public class PilotExceptionProtectionsTests
{
    private readonly PilotEngine _engine = new();

    [Fact]
    public void Test_Try_Except_Catches_Generic_User_Error()
    {
        string code = @"
            var caught_msg = """";
            try {
                raise ""Critical calculation failure"";
            } except err {
                caught_msg = str(err);
            }
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var caughtMsg = result.Context.Environment.CurrentScope.Resolve("caught_msg");
        Assert.Contains("Critical calculation failure", caughtMsg.AsString());
    }

    [Fact]
    public void Test_Try_Except_Catches_DivideByZeroError()
    {
        string code = @"
            var handled = false;
            var result_val = -1;

            try {
                let a = 100 / 0;
            } except DivideByZeroError as ex {
                handled = true;
                result_val = 0;
            }
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var handled = result.Context.Environment.CurrentScope.Resolve("handled");
        var resultVal = result.Context.Environment.CurrentScope.Resolve("result_val");

        Assert.True(handled.IsTrue);
        Assert.Equal(0, resultVal.AsInt());
    }

    [Fact]
    public void Test_Try_Except_Catches_NullReferenceError()
    {
        string code = @"
            var handled = false;
            let non_null_safe_val = null;

            // Safe nav should not throw
            let safe_check = non_null_safe_val?.length;

            // Dot access without safe nav should throw NullReferenceError
            try {
                let bad = non_null_safe_val.member;
            } except NullReferenceError as ex {
                handled = true;
            }
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var safeCheck = result.Context.Environment.CurrentScope.Resolve("safe_check");
        var handled = result.Context.Environment.CurrentScope.Resolve("handled");

        Assert.True(safeCheck.IsUndefined);
        Assert.True(handled.IsTrue);
    }

    [Fact]
    public void Test_Try_Except_Catches_TypeError()
    {
        string code = @"
            var handled = false;
            let not_a_func = 42;

            try {
                not_a_func();
            } except TypeError as ex {
                handled = true;
            }
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var handled = result.Context.Environment.CurrentScope.Resolve("handled");
        Assert.True(handled.IsTrue);
    }

    [Fact]
    public void Test_Try_Else_Finally_Full_Lifecycle_No_Exception()
    {
        string code = @"
            let steps = [];

            try {
                steps.append(""try"");
            } except {
                steps.append(""except"");
            } else {
                steps.append(""else"");
            } finally {
                steps.append(""finally"");
            }
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var steps = result.Context.Environment.CurrentScope.Resolve("steps").AsList();
        Assert.Equal(3, steps.Count);
        Assert.Equal("try", steps[0].AsString());
        Assert.Equal("else", steps[1].AsString());
        Assert.Equal("finally", steps[2].AsString());
    }

    [Fact]
    public void Test_Try_Except_Finally_Lifecycle_With_Exception()
    {
        string code = @"
            let steps = [];

            try {
                steps.append(""try"");
                raise ""Error trigger"";
            } except as err {
                steps.append(""except"");
            } else {
                steps.append(""else"");
            } finally {
                steps.append(""finally"");
            }
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var steps = result.Context.Environment.CurrentScope.Resolve("steps").AsList();
        Assert.Equal(3, steps.Count);
        Assert.Equal("try", steps[0].AsString());
        Assert.Equal("except", steps[1].AsString());
        Assert.Equal("finally", steps[2].AsString());
    }

    [Fact]
    public void Test_ControlFlow_Return_Not_Swallowed_By_Except()
    {
        string code = @"
            def test_func() {
                try {
                    return 777;
                } except {
                    return 999;
                }
            }

            let ret = test_func();
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var ret = result.Context.Environment.CurrentScope.Resolve("ret");
        Assert.Equal(777, ret.AsInt());
    }

    [Fact]
    public void Test_Assertion_Throws_AssertionError_Catchable()
    {
        string code = @"
            var assert_caught = false;

            try {
                assert 5 > 10, ""Five is not greater than ten"";
            } except AssertionError as e {
                assert_caught = true;
            }
        ";
        var result = _engine.Execute(code);
        Assert.True(result.Success, string.Join("; ", result.Diagnostics));

        var assertCaught = result.Context.Environment.CurrentScope.Resolve("assert_caught");
        Assert.True(assertCaught.IsTrue);
    }
}
