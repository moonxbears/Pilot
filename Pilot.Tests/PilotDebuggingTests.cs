#nullable disable
using System;
using System.Collections.Generic;
using Pilot.Compiler;
using Pilot.Debugging;
using Pilot.Runtime.Execution;
using Pilot.Runtime.Values;
using Xunit;

namespace Pilot.Tests;

public class PilotDebuggingTests
{
    [Fact]
    public void Test_Breakpoint_Hit_And_Locals_Inspection()
    {
        string code = @"
            def compute(x, y) {
                let sum = x + y;
                let product = x * y;
                return product - sum;
            }

            let result = compute(10, 20);
        ";

        var debugger = new PilotDebugger();
        bool breakpointHit = false;
        IReadOnlyDictionary<string, PilotValue> capturedLocals = null;
        PilotValue evaluatedExpr = PilotValue.Undefined;

        // Set breakpoint on 'let product = x * y;' (line 4)
        debugger.SetBreakpoint("", 4);

        debugger.Paused += (sender, e) =>
        {
            if (e.Reason == DebuggerPauseReason.Breakpoint)
            {
                breakpointHit = true;
                capturedLocals = debugger.GetLocals();
                evaluatedExpr = debugger.Evaluate("x + y + sum");
                debugger.Continue();
            }
        };

        var options = new PilotExecutionOptions().WithDebugger(debugger);
        var engine = new PilotEngine();
        var execResult = engine.Execute(code, options);

        Assert.True(execResult.Success, string.Join("; ", execResult.Diagnostics));
        Assert.True(breakpointHit);
        Assert.NotNull(capturedLocals);
        Assert.Equal(10, capturedLocals["x"].AsInt());
        Assert.Equal(20, capturedLocals["y"].AsInt());
        Assert.Equal(30, capturedLocals["sum"].AsInt());
        Assert.Equal(60, evaluatedExpr.AsInt()); // 10 + 20 + 30
    }

    [Fact]
    public void Test_CallStack_Unwinding_MultiFrame()
    {
        string code = @"
            def inner_fn(val) {
                let inner_var = val * 2;
                return inner_var;
            }

            def middle_fn(a, b) {
                let mid_sum = a + b;
                return inner_fn(mid_sum);
            }

            def outer_fn() {
                let start = 5;
                return middle_fn(start, 15);
            }

            let final_ans = outer_fn();
        ";

        var debugger = new PilotDebugger();
        IReadOnlyList<PilotStackFrame> capturedStack = null;

        // Set breakpoint in inner_fn at line 3: let inner_var = val * 2;
        debugger.SetBreakpoint("", 3);

        debugger.Paused += (sender, e) =>
        {
            capturedStack = debugger.CallStack.Frames;
            debugger.Continue();
        };

        var options = new PilotExecutionOptions().WithDebugger(debugger);
        var execResult = new PilotEngine().Execute(code, options);

        Assert.True(execResult.Success, string.Join("; ", execResult.Diagnostics));
        Assert.NotNull(capturedStack);
        Assert.Equal(4, capturedStack.Count);

        // Frame 0: inner_fn
        Assert.Equal("inner_fn", capturedStack[0].FunctionName);
        Assert.Equal(20, capturedStack[0].GetLocals()["val"].AsInt());

        // Frame 1: middle_fn
        Assert.Equal("middle_fn", capturedStack[1].FunctionName);
        Assert.Equal(5, capturedStack[1].GetLocals()["a"].AsInt());
        Assert.Equal(15, capturedStack[1].GetLocals()["b"].AsInt());

        // Frame 2: outer_fn
        Assert.Equal("outer_fn", capturedStack[2].FunctionName);
        Assert.Equal(5, capturedStack[2].GetLocals()["start"].AsInt());

        // Frame 3: <global>
        Assert.Equal("<global>", capturedStack[3].FunctionName);
    }

    [Fact]
    public void Test_Conditional_Breakpoint()
    {
        string code = @"
            let total = 0;
            for i in [1, 2, 3, 4, 5] {
                total = total + i;
            }
        ";

        var debugger = new PilotDebugger();
        int breakHitCount = 0;
        long capturedI = -1;

        // Break on line 4 only when i == 4
        debugger.SetBreakpoint("", 4, condition: "i == 4");

        debugger.Paused += (sender, e) =>
        {
            breakHitCount++;
            capturedI = debugger.GetLocals()["i"].AsInt();
            debugger.Continue();
        };

        var options = new PilotExecutionOptions().WithDebugger(debugger);
        var execResult = new PilotEngine().Execute(code, options);

        Assert.True(execResult.Success, string.Join("; ", execResult.Diagnostics));
        Assert.Equal(1, breakHitCount);
        Assert.Equal(4, capturedI);
    }

    [Fact]
    public void Test_HitCount_Threshold_Breakpoint()
    {
        string code = @"
            let count = 0;
            for x in [10, 20, 30, 40, 50] {
                count = count + 1;
            }
        ";

        var debugger = new PilotDebugger();
        int breakHitCount = 0;
        long firstCapturedCount = -1;
        long lastCapturedCount = -1;

        // Break on line 4 only after it has been reached 3 times
        debugger.SetBreakpoint("", 4, hitCountThreshold: 3);

        debugger.Paused += (sender, e) =>
        {
            breakHitCount++;
            if (firstCapturedCount == -1)
            {
                firstCapturedCount = debugger.GetLocals()["count"].AsInt();
            }
            lastCapturedCount = debugger.GetLocals()["count"].AsInt();
            debugger.Continue();
        };

        var options = new PilotExecutionOptions().WithDebugger(debugger);
        var execResult = new PilotEngine().Execute(code, options);

        Assert.True(execResult.Success, string.Join("; ", execResult.Diagnostics));
        // HitCountThreshold: hits at iteration 3, 4, 5 -> 3 times
        Assert.Equal(3, breakHitCount);
        Assert.Equal(2, firstCapturedCount); // On first break (iteration 3), count was 2
        Assert.Equal(4, lastCapturedCount);  // On last break (iteration 5), count was 4
    }

    [Fact]
    public void Test_SetVariable_During_Debug_Pause()
    {
        string code = @"
            let x = 10;
            let y = 20;
            let result = x + y;
        ";

        var debugger = new PilotDebugger();

        // Break on line 4 (let result = x + y;) and modify x to 100
        debugger.SetBreakpoint("", 4);

        debugger.Paused += (sender, e) =>
        {
            bool modified = debugger.SetVariable("x", PilotValue.FromInt(100));
            Assert.True(modified);
            debugger.Continue();
        };

        var options = new PilotExecutionOptions().WithDebugger(debugger);
        var execResult = new PilotEngine().Execute(code, options);

        Assert.True(execResult.Success, string.Join("; ", execResult.Diagnostics));

        var resultVal = execResult.Context.Environment.CurrentScope.Resolve("result");
        Assert.Equal(120, resultVal.AsInt()); // 100 + 20 = 120
    }

    [Fact]
    public void Test_StepInto_StepOver_Execution()
    {
        string code = @"
            def add(a, b) {
                return a + b;
            }

            let n1 = 10;
            let n2 = 20;
            let sum = add(n1, n2);
            let final_val = sum * 2;
        ";

        var debugger = new PilotDebugger();
        var visitedLines = new List<int>();

        // Start in StepInto mode
        debugger.StepInto();

        debugger.Paused += (sender, e) =>
        {
            visitedLines.Add(e.Frame.Line);

            // Once we reach line 8 (let sum = add(n1, n2)), test StepOver
            if (e.Frame.Line == 8)
            {
                debugger.StepOver();
            }
            else
            {
                debugger.StepInto();
            }
        };

        var options = new PilotExecutionOptions().WithDebugger(debugger);
        var execResult = new PilotEngine().Execute(code, options);

        Assert.True(execResult.Success, string.Join("; ", execResult.Diagnostics));
        Assert.NotEmpty(visitedLines);

        // Lines 6, 7, 8 should have been visited
        Assert.Contains(6, visitedLines);
        Assert.Contains(7, visitedLines);
        Assert.Contains(8, visitedLines);
        Assert.Contains(9, visitedLines);
    }
}
